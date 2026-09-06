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
                .filter(|(_, c)| c.owner == s && c.in_play && c.contested)
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
                self.state.minions[m.index()].contested = false;
                self.enter_title_contest(m);
            }
            Contest::Title(m) => self.state.minions[m.index()].title_contested = false,
            // "Unlocked and turned face up during your next unlock phase".
            Contest::Library(c) => {
                let card = &mut self.state.cards[c.index()];
                card.contested = false;
                card.locked = false;
            }
        }
    }

    /// Pay or yield one contest. Returns true when the active Methuselah
    /// paid themselves to 0: ousted, the turn has passed.
    pub(crate) fn settle_contest(&mut self, item: Contest, pay: bool) -> bool {
        let s = self.state.active;
        match item {
            Contest::Card(m) => {
                if pay {
                    self.state.seats[s.index()].pool -= 1;
                } else {
                    // Yielded: burned with everything on it (not ready:
                    // no "ready minion burned" rider).
                    self.burn_minion(m);
                }
            }
            Contest::Title(m) => {
                let v = &mut self.state.minions[m.index()];
                if pay {
                    v.blood -= 1;
                } else {
                    v.title_contested = false;
                    v.title_lost = true;
                }
            }
            Contest::Library(c) => {
                if pay {
                    self.state.seats[s.index()].pool -= 1;
                } else {
                    // Yielded: burned, counters and all.
                    self.burn_card_in_play(c);
                }
            }
        }
        if self.state.seats[s.index()].pool == 0 {
            self.oust_and_pass(s);
            return true;
        }
        false
    }

    /// A copy of a unique vampire entering play while another Methuselah has
    /// one: every copy is contested (face down, out of play).
    pub(crate) fn enter_card_contest(&mut self, m: MinionId) {
        let rivals = self.state.card_rivals(m);
        if !rivals.is_empty() {
            self.state.minions[m.index()].contested = true;
            for r in rivals {
                self.state.minions[r.index()].contested = true;
            }
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
        let rivals = self.state.library_rivals(id);
        if !rivals.is_empty() {
            self.state.cards[id.index()].contested = true;
            for r in rivals {
                self.state.cards[r.index()].contested = true;
            }
        }
    }

    /// A vampire (re-)entering play whose unique title is already claimed:
    /// the title is contested for every claimant.
    pub(crate) fn enter_title_contest(&mut self, m: MinionId) {
        if !self.state.minions[m.index()].present() || self.state.minions[m.index()].title_lost {
            return;
        }
        let rivals = self.state.title_rivals(m);
        if !rivals.is_empty() {
            self.state.minions[m.index()].title_contested = true;
            for r in rivals {
                self.state.minions[r.index()].title_contested = true;
            }
        }
    }
}
