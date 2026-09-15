//! Cards stored on cards and the draws they change (slice 8c-ii): draw
//! alternatives ("if you would draw a card from your library, you can draw
//! one of those cards instead": Black Market Cache, Shilmulo Tarot), a
//! replacement's treatment beside Visit from the Capuchin, Heart of
//! Nizchetus' draws and bottom move, Black Market Cache's search.
//!
//! These choices interrupt whatever drew the cards, as a discard-down does:
//! `Engine::interrupts` is served, in order, before any other decision. A
//! Methuselah with no draw alternative and nothing queued draws at once.

use crate::cards::{self, Effect, Hook};
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::Engine;
use crate::ids::SeatId;
use crate::library::{CardType, Label, LIBRARY};
use crate::state::CardId;

/// A pending choice about cards drawn, moved or stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Interrupt {
    /// A replacement Visit from the Capuchin would take instead while a
    /// draw alternative applies too: one treatment or the other (ANK
    /// 20200905; rules-qa 9.4). `except`: the card being replaced.
    Replacement {
        seat: SeatId,
        except: Option<CardId>,
    },
    /// Draw up to hand size, each card from the library or a draw
    /// alternative.
    DrawUp { seat: SeatId },
    /// Heart of Nizchetus: `left` cards still to draw, `drawn` so far; the
    /// same number then goes to the bottom of the library.
    Draws { seat: SeatId, left: u8, drawn: u8 },
    /// `left` cards still to move from hand to the bottom of the library.
    Bottom { seat: SeatId, left: u8 },
    /// Black Market Cache's search: up to `left` more cards to put on
    /// `card`.
    StoreSearch {
        seat: SeatId,
        card: CardId,
        left: u8,
    },
}

impl Interrupt {
    fn seat(self) -> SeatId {
        match self {
            Interrupt::Replacement { seat, .. }
            | Interrupt::DrawUp { seat }
            | Interrupt::Draws { seat, .. }
            | Interrupt::Bottom { seat, .. }
            | Interrupt::StoreSearch { seat, .. } => seat,
        }
    }
}

impl Engine {
    /// The cards `s` could draw instead of their library's top card, by
    /// card on each card in play (rules-qa 9.4); none from an empty library
    /// (rules-qa 8.3), none from a contested card (spec §1.13.3).
    pub(crate) fn draw_alternatives(&self, s: SeatId) -> Vec<Choice> {
        let st = &self.state;
        let mut out = Vec::new();
        if st.seats[s.index()].library.is_empty() {
            return out;
        }
        for (i, c) in st.cards.iter().enumerate() {
            let id = CardId(i as u32);
            if !c.present() || c.stored.is_empty() || st.card_controller(id) != s {
                continue;
            }
            let applies = cards::lookup(c.card).is_some_and(|d| {
                d.riders.iter().any(|r| {
                    r.hook == Hook::WouldDraw && r.scope.applies(s, s) && st.rider_holds(r, id)
                })
            });
            if !applies {
                continue;
            }
            let mut stored = c.stored.clone();
            stored.sort_unstable();
            stored.dedup();
            out.extend(stored.into_iter().map(|stored| Choice::DrawStored {
                card: i as u32,
                stored,
            }));
        }
        out
    }

    /// The card whose `WouldReplace` rider takes a replacement of `s`
    /// instead (Visit from the Capuchin), `except` the card being replaced
    /// ("a card other than this card").
    pub(crate) fn replacement_rider(&self, s: SeatId, except: Option<CardId>) -> Option<CardId> {
        let st = &self.state;
        (0..st.cards.len() as u32).map(CardId).find(|&id| {
            let c = &st.cards[id.index()];
            Some(id) != except
                && c.present()
                && st.card_controller(id) == s
                && cards::lookup(c.card).is_some_and(|d| {
                    d.riders.iter().any(|r| {
                        r.hook == Hook::WouldReplace
                            && r.scope.applies(s, s)
                            && st.rider_holds(r, id)
                    })
                })
        })
    }

    /// A replacement taken by that rider instead: a counter burned, the
    /// card burned with none left.
    pub(crate) fn apply_replacement_rider(&mut self, id: CardId) {
        let def = cards::lookup(self.state.cards[id.index()].card).expect("definition");
        for r in def.riders.iter().filter(|r| r.hook == Hook::WouldReplace) {
            for e in r.effects {
                match *e {
                    Effect::RemoveCounter(k) => {
                        let c = &mut self.state.cards[id.index()];
                        c.counters = c.counters.saturating_sub(k);
                    }
                    Effect::BurnSelfWithoutCounters => {
                        if self.state.cards[id.index()].counters == 0 {
                            self.burn_card_in_play(id);
                        }
                    }
                    other => unreachable!("not a replacement rider effect: {other:?}"),
                }
            }
        }
    }

    /// Whether `s`'s draws are choices now: a draw alternative applies, or
    /// choices of theirs are still queued (their draws keep their order).
    pub(crate) fn draws_interrupt(&self, s: SeatId) -> bool {
        self.interrupts.iter().any(|i| i.seat() == s) || !self.draw_alternatives(s).is_empty()
    }

    /// Move the top `n` cards of the card's controller's library onto it,
    /// as many as there are (rules-qa 11.2).
    pub(crate) fn store_top_cards(&mut self, id: CardId, n: u8) {
        let s = self.state.card_controller(id);
        for _ in 0..n {
            let Some(top) = self.state.seats[s.index()].library.pop() else {
                break;
            };
            self.state.cards[id.index()].stored.push(top);
        }
    }

    fn draw_top(&mut self, s: SeatId) {
        let seat = &mut self.state.seats[s.index()];
        if let Some(c) = seat.library.pop() {
            seat.hand.push(c);
        }
    }

    /// A card drawn from a card in play instead; the card's draw rider
    /// follows ("if this location has no cards on it, burn it").
    fn take_stored(&mut self, s: SeatId, card: u32, stored: u16) {
        let id = CardId(card);
        let on = &mut self.state.cards[id.index()].stored;
        let k = on.iter().rposition(|&c| c == stored).expect("stored card");
        on.remove(k);
        self.state.seats[s.index()].hand.push(stored);
        let def = cards::lookup(self.state.cards[id.index()].card).expect("definition");
        for r in def.riders.iter().filter(|r| r.hook == Hook::WouldDraw) {
            for e in r.effects {
                match *e {
                    Effect::BurnSelfWithoutStored => {
                        if self.state.cards[id.index()].stored.is_empty() {
                            self.burn_card_in_play(id);
                        }
                    }
                    other => unreachable!("not a draw alternative effect: {other:?}"),
                }
            }
        }
    }

    /// The end of Black Market Cache's search: the library is shuffled;
    /// holding no card, the location is burned at once (rules-qa 11.3),
    /// else a unique copy enters contest now that its cards are on it
    /// (spec §1.13.2).
    fn finish_store_search(&mut self, s: SeatId, card: CardId) {
        self.rng.shuffle(&mut self.state.seats[s.index()].library);
        if self.state.cards[card.index()].stored.is_empty() {
            self.burn_card_in_play(card);
        } else {
            self.enter_library_contest(card);
        }
    }

    /// Settle the queued interrupts needing no decision, front first, until
    /// one needs a decision or none is left: those of an ousted Methuselah
    /// are dropped; with no draw alternative left, draws and replacements
    /// happen as usual.
    pub(crate) fn settle_interrupts(&mut self) {
        while let Some(&front) = self.interrupts.first() {
            let s = front.seat();
            if self.state.seats[s.index()].ousted {
                self.interrupts.remove(0);
                continue;
            }
            let empty = self.state.seats[s.index()].library.is_empty();
            let choice = !self.draw_alternatives(s).is_empty();
            match front {
                Interrupt::Replacement { except, .. } => {
                    // With an empty library nothing is replaced, so nothing
                    // is replaced instead (rules-qa 8.3).
                    match self.replacement_rider(s, except).filter(|_| !empty) {
                        None => {
                            self.interrupts.remove(0);
                        }
                        Some(_) if choice => return,
                        Some(id) => {
                            self.interrupts.remove(0);
                            self.apply_replacement_rider(id);
                        }
                    }
                }
                Interrupt::DrawUp { .. } => {
                    let seat = &self.state.seats[s.index()];
                    if empty || seat.hand.len() + seat.held.total() >= self.state.hand_size(s) {
                        self.interrupts.remove(0);
                    } else if choice {
                        return;
                    } else {
                        self.draw_top(s);
                    }
                }
                Interrupt::Draws { left, drawn, .. } => {
                    if left == 0 || empty {
                        self.interrupts[0] = Interrupt::Bottom {
                            seat: s,
                            left: drawn,
                        };
                    } else if choice {
                        return;
                    } else {
                        self.draw_top(s);
                        self.interrupts[0] = Interrupt::Draws {
                            seat: s,
                            left: left - 1,
                            drawn: drawn + 1,
                        };
                    }
                }
                Interrupt::Bottom { left, .. } => {
                    if left > 0 && !self.state.seats[s.index()].hand.is_empty() {
                        return;
                    }
                    self.interrupts.remove(0);
                }
                Interrupt::StoreSearch { card, left, .. } => {
                    if left > 0 {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.finish_store_search(s, card);
                }
            }
        }
    }

    /// The decision of the front interrupt, once settled.
    pub(crate) fn interrupt_request(&self) -> Option<DecisionRequest> {
        let &front = self.interrupts.first()?;
        let s = front.seat();
        let st = &self.state;
        let (window, options) = match front {
            Interrupt::Replacement { except, .. } => {
                let id = self
                    .replacement_rider(s, except)
                    .expect("settled replacement");
                let mut options = vec![Choice::ResolveRider { card: id.0 }];
                options.extend(self.draw_alternatives(s));
                (WindowLabel::Draw, options)
            }
            Interrupt::DrawUp { .. } | Interrupt::Draws { .. } => {
                let mut options = vec![Choice::DrawLibrary];
                options.extend(self.draw_alternatives(s));
                (WindowLabel::Draw, options)
            }
            Interrupt::Bottom { .. } => (
                WindowLabel::BottomCards,
                (0..st.seats[s.index()].hand.len())
                    .map(|i| Choice::Bottom(i as u8))
                    .collect(),
            ),
            Interrupt::StoreSearch { .. } => {
                // Any non-unique equipment, by card: out of play, it needs
                // no definition to be drawn later.
                let mut found: Vec<u16> = st.seats[s.index()]
                    .library
                    .iter()
                    .copied()
                    .filter(|&c| {
                        let lib = &LIBRARY[c as usize];
                        lib.types.contains(&CardType::Equipment)
                            && !lib.labels.contains(&Label::Unique)
                    })
                    .collect();
                found.sort_unstable();
                found.dedup();
                let mut options = vec![Choice::Pass];
                options.extend(found.into_iter().map(Choice::FindCard));
                (WindowLabel::LibrarySearch, options)
            }
        };
        Some(DecisionRequest {
            seat: s,
            window,
            options,
        })
    }

    pub(crate) fn interrupt_apply(&mut self, choice: &Choice) {
        let front = self.interrupts[0];
        let s = front.seat();
        match (front, *choice) {
            (Interrupt::Replacement { .. }, Choice::ResolveRider { card }) => {
                self.interrupts.remove(0);
                self.apply_replacement_rider(CardId(card));
            }
            (Interrupt::Replacement { .. }, Choice::DrawStored { card, stored }) => {
                self.interrupts.remove(0);
                self.take_stored(s, card, stored);
            }
            (Interrupt::DrawUp { .. }, Choice::DrawLibrary) => self.draw_top(s),
            (Interrupt::DrawUp { .. }, Choice::DrawStored { card, stored }) => {
                self.take_stored(s, card, stored)
            }
            (Interrupt::Draws { left, drawn, .. }, c) => {
                match c {
                    Choice::DrawLibrary => self.draw_top(s),
                    Choice::DrawStored { card, stored } => self.take_stored(s, card, stored),
                    other => unreachable!("illegal draw choice {other:?}"),
                }
                self.interrupts[0] = Interrupt::Draws {
                    seat: s,
                    left: left - 1,
                    drawn: drawn + 1,
                };
            }
            (Interrupt::Bottom { left, .. }, Choice::Bottom(i)) => {
                let seat = &mut self.state.seats[s.index()];
                let card = seat.hand.remove(i as usize);
                // Under the cards moved before it: the last one chosen is
                // the bottom card (rules-qa 9.5).
                seat.library.insert(0, card);
                self.interrupts[0] = Interrupt::Bottom {
                    seat: s,
                    left: left - 1,
                };
            }
            (Interrupt::StoreSearch { card, left, .. }, Choice::FindCard(found)) => {
                let library = &mut self.state.seats[s.index()].library;
                let k = library
                    .iter()
                    .rposition(|&c| c == found)
                    .expect("found card in the library");
                library.remove(k);
                self.state.cards[card.index()].stored.push(found);
                self.interrupts[0] = Interrupt::StoreSearch {
                    seat: s,
                    card,
                    left: left - 1,
                };
            }
            (Interrupt::StoreSearch { card, .. }, Choice::Pass) => {
                self.interrupts[0] = Interrupt::StoreSearch {
                    seat: s,
                    card,
                    left: 0,
                };
            }
            (front, choice) => unreachable!("illegal choice {choice:?} for {front:?}"),
        }
    }
}
