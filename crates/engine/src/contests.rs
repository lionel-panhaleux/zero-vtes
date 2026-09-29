//! Card and title contests (rulebook "Contested Cards" / "Contested
//! Titles"): entry on influence-out, upkeep in the owner's unlock phase
//! (DESIGN §4.4 factoring: each Methuselah settles their own side, in arena
//! order).

use crate::decision::Choice;
use crate::game::{Contest, Engine};
use crate::ids::MinionId;
use crate::library::{Label, LIBRARY};
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
            .filter(|(_, m)| m.owner == s && m.in_play)
            .flat_map(|(i, m)| {
                let id = MinionId(i as u32);
                let card = m.contested.then_some(Contest::Card(id));
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
                        && c.body.is_none()
                        && (crate::cards::card_title(c.card).is_none()
                            || LIBRARY[c.card as usize].labels.contains(&Label::Unique))
                        && self.state.card_controller(CardId(*i as u32)) == s
                })
                .map(|(i, _)| Contest::Library(CardId(i as u32))),
        );
        items
    }

    /// The upkeep decision for one contested object: none when no rival is
    /// left (it returns for free), a forced yield for a title on a torpid
    /// or empty vampire, else pay or yield. Paying is always offered for a
    /// card: a living Methuselah has pool, and paying down to 0 is legal.
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
                if v.in_torpor || v.blood == 0 {
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
                self.set_minion_contested(m, false);
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
                self.state.seats[s.index()].pool -= 1;
            }
            (Contest::Title(m), true) => self.state.minions[m.index()].blood -= 1,
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
        let rivals = self.state.card_rivals(m);
        if !rivals.is_empty() {
            self.set_minion_contested(m, true);
            for r in rivals {
                self.set_minion_contested(r, true);
            }
        }
    }

    /// A minion's card contest state; an ally's card in play follows it
    /// (face down and out of play with it: its text does not apply).
    fn set_minion_contested(&mut self, m: MinionId, contested: bool) {
        self.state.minions[m.index()].contested = contested;
        if let Some(id) = self.state.minions[m.index()].ally_id() {
            self.state.cards[id.index()].contested = contested;
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

    /// Vampire `m` loses the title it claims (yielded, stolen, or replaced):
    /// its title card is burned, a title gained without a card is gone,
    /// or else its printed title is lost for the rest of the game.
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

    /// `thief` steals the title `from` holds (Valerius): `from` loses it
    /// — a card representing it is burned — and `thief` gains the title
    /// itself (rules-qa 19.6).
    pub(crate) fn steal_title(&mut self, thief: MinionId, from: MinionId) {
        let title = self.state.title_of(from);
        if title == crate::crypt::Title::None {
            return;
        }
        self.lose_title(from);
        self.gain_title(thief, title, None);
    }

    /// Vampire `m` gains `title` (rulebook "Vampiric Sects": a vampire
    /// has one title; one with a title that gains another loses the
    /// first, and yields a contested one): with `card`, a title card put
    /// on it; else a title gained without a card (Valerius's steal). The
    /// printed title, if still held, is lost with it. A rival claimant
    /// contests it.
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
