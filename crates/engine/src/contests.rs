//! Card and title contests (rulebook "Contested Cards" / "Contested
//! Titles"): entry, and upkeep in each owner's unlock phase (DESIGN §4.4:
//! each Methuselah settles their own side, in arena order).

use crate::decision::Choice;
use crate::game::{Contest, Engine};
use crate::ids::MinionId;
use crate::library::{Label, LIBRARY};
use crate::notes::{Cause, Rule, Source};
use crate::state::CardId;

impl Engine {
    /// The active seat's contested objects, in arena order.
    pub(crate) fn collect_contests(&self) -> Vec<Contest> {
        let s = self.state.active;
        let mut items: Vec<Contest> = self
            .state
            .minions
            .iter()
            .enumerate()
            .filter(|(_, m)| m.owner == s && m.in_play && !m.out_of_play)
            .flat_map(|(i, m)| {
                let id = MinionId(i as u32);
                let card = (m.contested || m.contested_in_play).then_some(Contest::Card(id));
                let title = (m.title_contested && !m.contested).then_some(Contest::Title(id));
                card.into_iter().chain(title)
            })
            .collect();
        items.extend(
            self.state
                .cards
                .iter()
                .enumerate()
                // An ally's card is contested as its minion, a non-unique
                // title card as its vampire's title (above).
                .filter(|(i, c)| {
                    c.in_play
                        && c.contested
                        && contests_as_card(c)
                        && self.state.card_controller(CardId(*i as u32)) == s
                })
                .map(|(i, _)| Contest::Library(CardId(i as u32))),
        );
        items
    }

    /// The upkeep choices for one contested object: none without a rival,
    /// a forced yield for a title on a torpid or empty vampire, else pay or
    /// yield (a card's payment always: paying down to 0 is legal).
    pub(crate) fn contest_choices(&self, item: Contest) -> Vec<Choice> {
        let st = &self.state;
        match item {
            Contest::Card(m) if st.card_rivals(m).is_empty() => vec![],
            Contest::Title(m) if st.title_rivals(m).is_empty() => vec![],
            Contest::Library(c) if st.library_rivals(c).is_empty() => vec![],
            Contest::Card(_) | Contest::Library(_) => {
                vec![Choice::PayContest(item), Choice::YieldContest(item)]
            }
            Contest::Title(m) => {
                let v = &st.minions[m.index()];
                // Rafael de Corazon: his rivals yield.
                let forced = st.title_rivals(m).into_iter().any(|r| {
                    st.printed_mod(
                        r,
                        crate::cards::Stat::RivalsYieldTitle,
                        crate::state::PrintedCtx::default(),
                    ) > 0
                });
                if v.in_torpor || v.blood < st.title_contest_cost(m) || forced {
                    vec![Choice::YieldContest(item)]
                } else {
                    vec![Choice::PayContest(item), Choice::YieldContest(item)]
                }
            }
        }
    }

    /// A contested object with no rival left returns for free "during your
    /// next unlock phase".
    pub(crate) fn release_contest(&mut self, item: Contest) {
        match item {
            Contest::Card(m) => {
                self.set_card_contest(m, false);
                self.enter_title_contest(m);
            }
            Contest::Title(m) => self.set_title_contested(m, false),
            // "Unlocked and turned face up during your next unlock phase".
            Contest::Library(c) => {
                let card = &mut self.state.cards[c.index()];
                card.contested = false;
                card.locked = false;
                // A title card face up again: its vampire gains the title.
                if let (Some(t), Some(h)) = (crate::cards::card_title(card.card), card.host) {
                    self.gain_title(h, t, Some(c));
                }
                // Its hand size applies again: draw up at once.
                let controller = self.state.card_controller(c);
                self.draw_up(controller);
            }
        }
    }

    /// Pay or yield one contest. Returns true when the active Methuselah
    /// paid themselves to 0: ousted, the turn has passed.
    pub(crate) fn settle_contest(&mut self, item: Contest, pay: bool) -> bool {
        let s = self.state.active;
        match (item, pay) {
            (Contest::Card(_) | Contest::Library(_), true) => {
                let contest = Cause::Cost(Source::Rule(Rule::Contest));
                self.caused(contest, |e| e.state.seats[s.index()].lose_pool(1));
            }
            (Contest::Title(m), true) => {
                let cost = self.state.title_contest_cost(m);
                let contest = Cause::Cost(Source::Rule(Rule::Contest));
                self.caused(contest, |e| e.state.lose_blood(m, cost));
            }
            // Yielded: burned with everything on it (not ready: no "ready
            // minion burned" rider).
            (Contest::Card(m), false) => self.burn_minion(m),
            (Contest::Title(m), false) => {
                self.set_title_contested(m, false);
                self.lose_title(m);
            }
            // Yielded: burned, counters and all.
            (Contest::Library(c), false) => self.burn_card_in_play(c),
        }
        if self.state.seats[s.index()].pool == 0 {
            return self.oust_and_pass(s);
        }
        false
    }

    /// A copy of a unique vampire entering play while another Methuselah has
    /// one: every copy is contested (face down, out of play).
    pub(crate) fn enter_card_contest(&mut self, m: MinionId) {
        // Jimmy Dunn: the copy already in play burns instead (any
        // Methuselah's; the newcomer's controller does not burn it, LSJ
        // 19990719).
        if self.state.printed_mod(
            m,
            crate::cards::Stat::NeverContested,
            crate::state::PrintedCtx::default(),
        ) > 0
        {
            let name = self.state.minions[m.index()].name();
            let firsts: Vec<MinionId> = (0..self.state.minions.len())
                .map(|i| MinionId(i as u32))
                .filter(|&o| {
                    let v = &self.state.minions[o.index()];
                    o != m && v.in_play && !v.out_of_play && v.name() == name
                })
                .collect();
            // Burned by the rule, not by the acting minion (LSJ 19990719):
            // kept out of the action's burns.
            let burned = self.action.as_ref().map(|a| a.burned_owners.len());
            let card_burned = self.card_burns.as_ref().map(Vec::len);
            for o in firsts {
                self.burn_minion(o);
            }
            if let (Some(a), Some(n)) = (self.action.as_mut(), burned) {
                a.burned_owners.truncate(n);
            }
            if let (Some(b), Some(n)) = (self.card_burns.as_mut(), card_burned) {
                b.truncate(n);
            }
            return;
        }
        let rivals = self.state.card_rivals(m);
        if !rivals.is_empty() {
            self.set_card_contest(m, true);
            for r in rivals {
                self.set_card_contest(r, true);
            }
        }
    }

    /// `m` put out of play with its cards (Descent into Darkness): it
    /// leaves its contests until it returns (LSJ 20080816, rules-qa
    /// 78.54); a rival left alone is uncontested.
    pub(crate) fn leave_contests(&mut self, m: MinionId) {
        for r in self.state.card_rivals(m) {
            self.set_card_contest(m, false);
            if self.state.card_rivals(r).iter().all(|&o| o == m) {
                self.set_card_contest(r, false);
                self.enter_title_contest(r);
            }
        }
        if self.state.minions[m.index()].title_contested {
            self.set_title_contested(m, false);
            for r in self.state.title_rivals(m) {
                if self.state.title_rivals(r).is_empty() {
                    self.set_title_contested(r, false);
                }
            }
        }
    }

    /// A minion's card contest: a crypt card's in a two-player game stays
    /// in play and usable (`Minion::contested_in_play`), else face down
    /// and out of play.
    fn set_card_contest(&mut self, m: MinionId, contested: bool) {
        let crypt_card = matches!(
            self.state.minions[m.index()].kind,
            crate::state::MinionKind::Vampire(_) | crate::state::MinionKind::Imbued(_)
        );
        if self.state.format == crate::state::Format::TwoPlayer && crypt_card {
            self.state.minions[m.index()].contested_in_play = contested;
        } else {
            self.set_minion_contested(m, contested);
        }
    }

    /// A minion's card contest state; an ally's card in play follows it
    /// (face down and out of play with it: its text does not apply), and
    /// so do the cards on it, unless it is out of play already (rules-qa
    /// 83.1).
    fn set_minion_contested(&mut self, m: MinionId, contested: bool) {
        self.state.minions[m.index()].contested = contested;
        if let Some(id) = self.state.minions[m.index()].ally_id() {
            self.state.cards[id.index()].contested = contested;
        }
        if !self.state.minions[m.index()].out_of_play {
            if contested {
                self.cards_out_with(m, None);
            } else {
                self.cards_back_with(m);
            }
        }
    }

    /// The cards on `m` but `except` out of play with it (Descent into
    /// Darkness, a contest): each leaves its own contest until it returns,
    /// a rival left alone uncontested (advanced rules §1.13.3, rules-qa
    /// 83.1).
    pub(crate) fn cards_out_with(&mut self, m: MinionId, except: Option<CardId>) {
        let on: Vec<CardId> = (0..self.state.cards.len())
            .map(|i| CardId(i as u32))
            .filter(|&c| Some(c) != except)
            .filter(|&c| {
                let card = &self.state.cards[c.index()];
                card.in_play && card.host == Some(m)
            })
            .collect();
        for &c in &on {
            self.state.cards[c.index()].out_of_play = true;
        }
        for c in on {
            let card = &self.state.cards[c.index()];
            if !card.contested || !contests_as_card(card) {
                continue;
            }
            self.state.cards[c.index()].contested = false;
            for r in self.state.library_rivals(c) {
                if self.state.library_rivals(r).is_empty() {
                    self.state.cards[r.index()].contested = false;
                }
            }
        }
    }

    /// The cards out of play with `m` back in play, unlocked (LSJ
    /// 20060908; advanced rules §6.4.1), each in its contests again.
    pub(crate) fn cards_back_with(&mut self, m: MinionId) {
        let back: Vec<CardId> = (0..self.state.cards.len())
            .map(|i| CardId(i as u32))
            .filter(|&c| {
                let card = &self.state.cards[c.index()];
                card.in_play && card.host == Some(m) && card.out_of_play
            })
            .collect();
        for &c in &back {
            let card = &mut self.state.cards[c.index()];
            card.out_of_play = false;
            card.locked = false;
        }
        for c in back {
            self.enter_library_contest(c);
        }
    }

    /// A unique library card entering play while another Methuselah has a
    /// copy: every copy is contested (rulebook "Contested Cards"), once
    /// the card's own entry effects have resolved (spec §1.13.2).
    pub(crate) fn enter_library_contest(&mut self, id: CardId) {
        let card = self.state.cards[id.index()].card;
        if !LIBRARY[card as usize].labels.contains(&Label::Unique) {
            return;
        }
        // A unique title card contests as a card first: face down, its
        // title is not claimed, so no title contest (rules-qa 19.1).
        let rivals = self.state.library_rivals(id);
        if !rivals.is_empty() {
            self.state.cards[id.index()].contested = true;
            for r in rivals {
                self.state.cards[r.index()].contested = true;
            }
        }
    }

    /// A vampire (re-)entering play, or gaining a title, whose unique title
    /// is already claimed: the title is contested for every claimant.
    pub(crate) fn enter_title_contest(&mut self, m: MinionId) {
        if !self.state.minions[m.index()].present()
            || self.state.claimed_title(m) == crate::crypt::Title::None
        {
            return;
        }
        let rivals = self.state.title_rivals(m);
        if rivals.is_empty() {
            return;
        }
        // Victor Pelletier: "if his title would be contested, he
        // immediately yields instead of contesting".
        let yields = |e: &Self, v: MinionId| {
            e.state.printed_mod(
                v,
                crate::cards::Stat::YieldsTitleInsteadOfContest,
                crate::state::PrintedCtx::default(),
            ) > 0
        };
        if yields(self, m) {
            self.lose_title(m);
            return;
        }
        let rivals: Vec<MinionId> = rivals
            .into_iter()
            .filter(|&r| {
                let y = yields(self, r);
                if y {
                    self.lose_title(r);
                }
                !y
            })
            .collect();
        if !rivals.is_empty() {
            self.set_title_contested(m, true);
            for r in rivals {
                self.set_title_contested(r, true);
            }
        }
    }

    /// A vampire's title contest state; its title card, if any, follows it
    /// (face down and out of play: its abilities are suspended).
    fn set_title_contested(&mut self, m: MinionId, contested: bool) {
        self.state.minions[m.index()].title_contested = contested;
        if let Some((id, _)) = self.state.title_card_on(m) {
            self.state.cards[id.index()].contested = contested;
        }
    }

    /// Vampire `m` loses the title it claims: its card burned, else a
    /// granted title gone, else its printed title lost for the game.
    pub(crate) fn lose_title(&mut self, m: MinionId) {
        if let Some((id, _)) = self.state.title_card_on(m) {
            self.state.cards[id.index()].contested = false;
            self.burn_card_in_play(id);
        } else if self.state.minions[m.index()].granted_title.is_some() {
            self.state.minions[m.index()].granted_title = None;
        } else {
            self.state.minions[m.index()].title_lost = true;
        }
    }

    /// Vampire `m` yields its contested title outside the unlock phase
    /// (Ephor): as a yield in upkeep, its rivals' claims return free.
    pub(crate) fn yield_title(&mut self, m: MinionId) {
        self.set_title_contested(m, false);
        self.lose_title(m);
    }

    /// `thief` steals the title `from` holds, its card burned (Valerius,
    /// rules-qa 19.6).
    pub(crate) fn steal_title(&mut self, thief: MinionId, from: MinionId) {
        let title = self.state.title_of(from);
        if title == crate::crypt::Title::None {
            return;
        }
        self.lose_title(from);
        self.gain_title(thief, title, None);
    }

    /// Vampire `m` gains `title`, from `card` or without one (Valerius),
    /// losing any title it had (rulebook "Vampiric Sects": one title).
    pub(crate) fn gain_title(
        &mut self,
        m: MinionId,
        title: crate::crypt::Title,
        card: Option<CardId>,
    ) {
        let old = self
            .state
            .title_card_on(m)
            .map(|(id, _)| id)
            .filter(|&id| Some(id) != card);
        if self.state.minions[m.index()].title_contested {
            self.set_title_contested(m, false);
        }
        if let Some(id) = old {
            self.burn_card_in_play(id);
        }
        let v = &mut self.state.minions[m.index()];
        v.title_lost = true;
        v.granted_title = if card.is_some() { None } else { Some(title) };
        self.enter_title_contest(m);
    }
}

/// A card in play whose `contested` is its own library contest: not an
/// ally's card (contested as its minion) nor a non-unique title card
/// (contested as its vampire's title, rules-qa 19.1).
fn contests_as_card(c: &crate::state::CardInPlay) -> bool {
    c.body.is_none()
        && (crate::cards::card_title(c.card).is_none()
            || LIBRARY[c.card as usize].labels.contains(&Label::Unique))
}
