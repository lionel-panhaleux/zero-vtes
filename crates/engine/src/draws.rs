//! Cards stored on cards and the draws they change (slice 8c-ii): draw
//! alternatives ("if you would draw a card from your library, you can draw
//! one of those cards instead": Black Market Cache, Shilmulo Tarot), a
//! replacement's treatment beside Visit from the Capuchin, Heart of
//! Nizchetus' draws and bottom move, Black Market Cache's search.
//!
//! These choices interrupt whatever drew the cards, as a discard-down does:
//! `Engine::interrupts` is served, in order, before any other decision. A
//! Methuselah with no draw alternative and nothing queued draws at once.

use crate::cards::{self, CardFilter, Effect, Hook};
use crate::decision::{Choice, DecisionRequest, WindowLabel, Zone};
use crate::game::Engine;
use crate::ids::{MinionId, SeatId};
use crate::library::Label;
use crate::state::{distinct, CardId};

/// A pending choice about cards drawn, moved or stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interrupt {
    /// `seat` moves `card` (Fear of Mekhet) to another justicar or Inner
    /// Circle member, any Methuselah's, in torpor included (ANK 20210109;
    /// rules-qa 49.4: not locked by the move); burned if there is none.
    MoveCardToTitled { seat: SeatId, card: CardId },
    /// Blood Trade: 1 blood from `from` to a vampire of another
    /// Methuselah's, the player's choice.
    GiveBlood { seat: SeatId, from: MinionId },
    /// "Burned unless its controller burns n pool" (Conquest of Humanity):
    /// `seat` pays or lets `card` burn.
    PayOrBurn {
        seat: SeatId,
        card: CardId,
        pool: u8,
    },
    /// Nightmares upon Nightmares: `minion` is locked unless its
    /// Methuselah discards a card (not replaced until their master
    /// phase).
    LockOrDiscard { seat: SeatId, minion: MinionId },
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
    /// "After this ally enters play, burn an ally or retainer you control"
    /// (War Ghoul): the controller's pick among the others, the entering
    /// ally itself if there is none (RTR 19961113).
    EntryBurn {
        seat: SeatId,
        ally: crate::ids::MinionId,
    },
    /// "After this ally enters play, burn it unless you remove an ally or
    /// vampire in your ash heap from the game" (Rotting Behemoth): the
    /// controller may decline, and the ally burns (LSJ 20030701-2).
    EntryRemoveOrBurn {
        seat: SeatId,
        ally: crate::ids::MinionId,
    },
    /// A block attempt made costly as it goes on (Where the Veil Thins
    /// [OBL]): its minion pays, or resigns the attempt and pays nothing
    /// (LSJ 20031121-2).
    BlockToll {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Dawn Operation [for]: the vampire attempting to block may cancel its
    /// attempt (`ChooseMinion` it, or pass).
    CancelAttempt {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Amici Noctis: `left` blood to distribute; first how much to the
    /// pool (at most `cap`), the rest among the ready `clan` vampires.
    MarginPool {
        seat: SeatId,
        left: u8,
        cap: u8,
        clan: &'static str,
    },
    /// Amici Noctis: `left` blood for the ready `clan` vampires below
    /// capacity, 1 each, offered in arena order from minion `next`: give or
    /// skip, a skip only while enough of them are left.
    MarginBlood {
        seat: SeatId,
        left: u8,
        next: u32,
        clan: &'static str,
    },
    /// A minion controlled by `of`, chosen as an effect resolves (Break the
    /// Bonds [obf], The Platinum Protocol [pro]: "the target is chosen upon
    /// resolution", ANK 20240706): locked, if the player wants to, or given
    /// one of the player's corruption counters.
    MinionOf {
        seat: SeatId,
        of: SeatId,
        purpose: MinionPurpose,
    },
    /// Whose corruption counter on `minion` burns, several Methuselahs'
    /// being on it (Aniel).
    CorruptionOwner {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// `minion` burns `blood` to be immune to `damage` unpreventable
    /// aggravated damage, or takes it (Rutor's Hand [THA]).
    AvoidDamage {
        seat: SeatId,
        minion: crate::ids::MinionId,
        blood: u8,
        damage: u8,
    },
    /// `seat` looks at `of`'s hand and discards a card of their choice from
    /// it, which `of` replaces (Revelations [aus]).
    DiscardFromHandOf { seat: SeatId, of: SeatId },
    /// `seat` discards `left` more cards of their choice from hand (Shroud
    /// of Decay [obl]); all `done` are replaced once every discard is made
    /// (advanced rules §6.8.3).
    /// `seat` reorders the top `total` cards of their library (Eulogio
    /// Sánchez de los Reyes): `placed` of them chosen so far, from the top
    /// down; the rest still in their old order below them.
    Reorder { seat: SeatId, total: u8, placed: u8 },
    DiscardChosen {
        seat: SeatId,
        left: u8,
        done: u8,
        /// Combat cards only (Target Vitals' cancel: "discard two combat
        /// cards").
        combat_only: bool,
    },
    /// `seat` removes `left` more cards of their choice from `of`'s ash
    /// heap from the game; then `of` burns `pool` pool (Shroud of Decay
    /// [OBL]; LSJ 20090710: there is no order to the ash heap) — from their
    /// own, pool 0, Marchesa Liliana's cost. Burned vampires are ash heap
    /// cards too (rules-qa 42.7).
    RemoveFromAsh {
        seat: SeatId,
        of: SeatId,
        left: u8,
        pool: u8,
    },
    /// "You can search your library (shuffle afterward), hand and/or ash
    /// heap for a Discipline master card and put it on this new vampire"
    /// (Childe of the Revolution): put on, not played.
    DisciplineSearch {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// "Remove a crypt card in your uncontrolled region from the game"
    /// (Wider View): the player's pick, the counters on it to the blood
    /// bank; removed cards are public (LSJ 20080314).
    RemoveUncontrolled { seat: SeatId },
    /// "As … its controller changes, its controller chooses a ready vampire
    /// they control" (The Rack): none if they have none.
    ChooseForCard {
        seat: SeatId,
        card: CardId,
        /// Of this clan only (Toreador Grand Ball, Priority Contract).
        clan: Option<&'static str>,
        /// A second choice, other than the first (Toreador Grand Ball).
        second: bool,
    },
    /// Garibaldi-Meucci Museum: the ash heap card requiring an Anarch, then
    /// the hand card it is exchanged for.
    ExchangeAnarch { seat: SeatId, ash: Option<u16> },
    /// "If the attached minion is about to leave the ready region, you can
    /// burn this card to gain n pool" (Priority Contract).
    PoolForCard { seat: SeatId, card: CardId, n: u8 },
    /// "You can lock this card … to gain n pool" (The Black Throne).
    LockForPool { seat: SeatId, card: CardId, n: u8 },
    /// "You can burn those counters to steal that minion" (Cave of Apples).
    StealForCorruption {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// "You can burn 1 blood from this card to make that vampire Anarch"
    /// (Twilight Camp).
    MakeAnarch {
        seat: SeatId,
        card: CardId,
        minion: crate::ids::MinionId,
    },
    /// Ashur Tablets' picks from `seat`'s ash heap: first the card to hand
    /// (a pass chooses none), then up to `left` more shuffled into the
    /// library (a pass stops), `to_library` once the hand card is chosen.
    AshurPicks {
        seat: SeatId,
        left: u8,
        to_library: bool,
    },
    /// "A non-titled vampire you control becomes Anarch" (Gilbert Duane):
    /// the player's pick, none if they have none.
    BecomesAnarch { seat: SeatId },
    /// "Add n blood to a `clan` vampire in your uncontrolled region"
    /// (Hunting the Beast [AUS]): the player's pick, none if they have none.
    UncontrolledBlood {
        seat: SeatId,
        clan: &'static str,
        n: u8,
    },
    /// "Move a wraith or zombie ally from your ash heap to your ready
    /// region" (Split the Veil): the player's pick among the allies
    /// matching one of `of`; with `named`, the pick is named for the
    /// action being declared, put in play as it resolves (Hel-Blá).
    AllyFromAsh {
        seat: SeatId,
        of: &'static [crate::cards::CardFilter],
        named: bool,
    },
    /// "This acting vampire can burn n blood to move the stolen vampire to
    /// your ready region" (Graverobbing [DOM]): `ChooseMinion` it, or pass.
    MoveForBlood {
        seat: SeatId,
        payer: crate::ids::MinionId,
        minion: crate::ids::MinionId,
        n: u8,
    },
}

/// What a minion chosen at resolution is for (`Interrupt::MinionOf`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinionPurpose {
    /// "You can lock a minion": optional; a locked one is a legal choice.
    Lock,
    /// "Put 1 corruption counter on a minion": mandatory.
    Corrupt,
    /// "Lock up to two vampires they control": optional, vampires only.
    LockVampire,
    /// "You can unlock a ready Anarch" (Powerbase: Los Angeles): optional,
    /// the user's own.
    UnlockAnarch,
    /// A ready vampire of the clan the user controls, with blood, burns 1
    /// (Chantry's cost): mandatory.
    BurnBloodOfClan(&'static str),
    /// A location on a minion, stolen, moves onto one of the new
    /// controller's able to hold it (Dominate Kine [DOM]; RTR 19960112):
    /// mandatory.
    HostCard(CardId),
    /// One of the oldest vampires of the clan (Brujah Debate): mandatory,
    /// locked or not (LSJ 20010508).
    LockOldest(&'static str),
    /// A vampire of a clan the user controls 3 or more of, burned
    /// (Recalled to the Founder): mandatory.
    BurnOfCrowdedClan,
    /// A ready minion of the chooser's, which takes n unpreventable damage
    /// (Gnaeus Aemilius Augustinus): mandatory.
    Damage(u8),
}

impl Interrupt {
    fn seat(self) -> SeatId {
        match self {
            Interrupt::GiveBlood { seat, .. }
            | Interrupt::PayOrBurn { seat, .. }
            | Interrupt::LockOrDiscard { seat, .. }
            | Interrupt::Replacement { seat, .. }
            | Interrupt::DrawUp { seat }
            | Interrupt::Draws { seat, .. }
            | Interrupt::Bottom { seat, .. }
            | Interrupt::StoreSearch { seat, .. }
            | Interrupt::EntryBurn { seat, .. }
            | Interrupt::EntryRemoveOrBurn { seat, .. }
            | Interrupt::BlockToll { seat, .. }
            | Interrupt::CancelAttempt { seat, .. }
            | Interrupt::MarginPool { seat, .. }
            | Interrupt::MarginBlood { seat, .. }
            | Interrupt::MinionOf { seat, .. }
            | Interrupt::CorruptionOwner { seat, .. }
            | Interrupt::DiscardChosen { seat, .. }
            | Interrupt::Reorder { seat, .. }
            | Interrupt::AvoidDamage { seat, .. }
            | Interrupt::DiscardFromHandOf { seat, .. }
            | Interrupt::RemoveFromAsh { seat, .. }
            | Interrupt::DisciplineSearch { seat, .. }
            | Interrupt::RemoveUncontrolled { seat }
            | Interrupt::ChooseForCard { seat, .. }
            | Interrupt::ExchangeAnarch { seat, .. }
            | Interrupt::PoolForCard { seat, .. }
            | Interrupt::LockForPool { seat, .. }
            | Interrupt::MakeAnarch { seat, .. }
            | Interrupt::StealForCorruption { seat, .. }
            | Interrupt::UncontrolledBlood { seat, .. }
            | Interrupt::BecomesAnarch { seat }
            | Interrupt::AshurPicks { seat, .. }
            | Interrupt::AllyFromAsh { seat, .. }
            | Interrupt::MoveForBlood { seat, .. }
            | Interrupt::MoveCardToTitled { seat, .. } => seat,
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
                    r.hook == Hook::WouldDraw && r.scope.applies(st, s, s) && st.rider_holds(r, id)
                })
            });
            if !applies {
                continue;
            }
            out.extend(distinct(&c.stored, |_| true).into_iter().map(|stored| {
                Choice::DrawStored {
                    card: i as u32,
                    stored,
                }
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
                            && r.scope.applies(&self.state, s, s)
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
            let attempting = |m: MinionId| {
                self.action
                    .as_ref()
                    .is_some_and(|a| a.state == crate::actions::ActionState::Attempt(m))
            };
            let present = |m: MinionId| self.state.minions[m.index()].present();
            // Waiting on a decision; if not, the arms that settle with an
            // effect do it and go on, the others are dropped below.
            let wait = match front {
                Interrupt::Replacement { except, .. } => {
                    // With an empty library nothing is replaced, so nothing
                    // is replaced instead (rules-qa 8.3).
                    match self.replacement_rider(s, except).filter(|_| !empty) {
                        None => false,
                        Some(_) if choice => true,
                        Some(id) => {
                            self.interrupts.remove(0);
                            self.apply_replacement_rider(id);
                            continue;
                        }
                    }
                }
                Interrupt::DrawUp { .. } => {
                    let seat = &self.state.seats[s.index()];
                    if empty || seat.hand.len() + seat.held.total() >= self.state.hand_size(s) {
                        false
                    } else if choice {
                        true
                    } else {
                        self.draw_top(s);
                        continue;
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
                    continue;
                }
                Interrupt::Bottom { left, .. } => {
                    left > 0 && !self.state.seats[s.index()].hand.is_empty()
                }
                Interrupt::StoreSearch { card, left, .. } => {
                    if left > 0 {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.finish_store_search(s, card);
                    continue;
                }
                Interrupt::EntryRemoveOrBurn { ally, .. } => {
                    self.state.minions[ally.index()].in_play
                }
                // Nothing to cancel once the attempt is over.
                Interrupt::CancelAttempt { minion, .. } => attempting(minion),
                Interrupt::BlockToll { minion, .. } => {
                    let affordable = self
                        .action
                        .as_ref()
                        .is_some_and(|a| self.block_toll(a, minion).is_some());
                    if attempting(minion) && !affordable {
                        self.interrupts.remove(0);
                        self.resign_attempt();
                        continue;
                    }
                    attempting(minion)
                }
                Interrupt::MarginPool { .. } => {
                    let range = self.margin_pool_range(front);
                    if range.len() > 1 {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.margin_to_pool(front, range[0]);
                    continue;
                }
                Interrupt::MarginBlood { left, .. } => {
                    let open = self.margin_recipients(front);
                    if left > 0 && open.len() > left as usize {
                        return;
                    }
                    self.interrupts.remove(0);
                    for m in open.into_iter().take(left as usize) {
                        self.state.gain_blood(m, 1);
                    }
                    continue;
                }
                Interrupt::DiscardFromHandOf { of, .. } => {
                    !self.state.seats[of.index()].ousted
                        && !self.state.seats[of.index()].hand.is_empty()
                }
                Interrupt::AvoidDamage { minion, .. } => present(minion),
                // One card or one kind left to place: no choice.
                Interrupt::Reorder {
                    seat,
                    total,
                    placed,
                } => {
                    let lib = &self.state.seats[seat.index()].library;
                    let n = lib.len();
                    let unplaced = &lib[n - total as usize..n - placed as usize];
                    let kinds = distinct(unplaced, |_| true).len();
                    if kinds > 1 {
                        return;
                    }
                    self.interrupts.remove(0);
                    continue;
                }
                Interrupt::DiscardChosen {
                    seat,
                    left,
                    done,
                    combat_only,
                } => {
                    let fits = |c: u16| {
                        !combat_only
                            || crate::library::LIBRARY[c as usize]
                                .types
                                .contains(&crate::library::CardType::Combat)
                    };
                    let held = self.state.seats[seat.index()]
                        .hand
                        .iter()
                        .filter(|&&c| fits(c))
                        .count();
                    if left > 0 && held > left as usize {
                        return;
                    }
                    // As many as it holds, or fewer: all of them — then
                    // the replacements.
                    self.interrupts.remove(0);
                    let st = &mut self.state.seats[seat.index()];
                    let k = if left > 0 { held } else { 0 };
                    let (cards, keep): (Vec<u16>, Vec<u16>) =
                        st.hand.iter().partition(|&&c| fits(c) && k > 0);
                    st.hand = keep;
                    st.ash.extend(cards);
                    self.replace(seat, done as usize + k, None);
                    continue;
                }
                Interrupt::RemoveFromAsh { of, left, pool, .. } => {
                    // Library cards and burned vampires alike (rules-qa
                    // 42.7).
                    let st = &self.state.seats[of.index()];
                    let kinds =
                        distinct(&st.ash, |_| true).len() + distinct(&st.ash_crypt, |_| true).len();
                    if left > 0 && !st.ousted && kinds > 1 {
                        return;
                    }
                    // One kind of card left, or none: no choice.
                    self.interrupts.remove(0);
                    let seat = &mut self.state.seats[of.index()];
                    let n = (left as usize).min(seat.ash.len());
                    let removed: Vec<u16> = seat.ash.drain(..n).collect();
                    seat.removed.extend(removed);
                    let k = (left as usize - n).min(seat.ash_crypt.len());
                    let removed: Vec<u16> = seat.ash_crypt.drain(..k).collect();
                    seat.removed_crypt.extend(removed);
                    let mut loss = vec![0; self.state.seats.len()];
                    loss[of.index()] = pool as i32;
                    self.burn_pool_batch(&loss);
                    continue;
                }
                Interrupt::StealForCorruption { minion, .. } => present(minion),
                Interrupt::MakeAnarch { card, minion, .. } => {
                    let c = &self.state.cards[card.index()];
                    c.present() && c.counters > 0 && present(minion)
                }
                Interrupt::LockForPool { card, .. } => {
                    let c = &self.state.cards[card.index()];
                    c.present() && !c.locked
                }
                Interrupt::PoolForCard { .. } => true,
                Interrupt::ExchangeAnarch { seat, .. } => {
                    !self.state.seats[seat.index()].hand.is_empty()
                }
                Interrupt::ChooseForCard { card, .. } => {
                    let ready = self.choosable(front);
                    if ready.len() > 1 && self.state.cards[card.index()].present() {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.set_chosen(front, ready.first().copied());
                    continue;
                }
                Interrupt::RemoveUncontrolled { seat } => {
                    match self.state.seats[seat.index()].uncontrolled.len() {
                        0 => false,
                        1 => {
                            self.interrupts.remove(0);
                            self.remove_uncontrolled(seat, 0);
                            continue;
                        }
                        _ => true,
                    }
                }
                Interrupt::DisciplineSearch { minion, .. } => {
                    if present(minion) && !self.discipline_masters(s).is_empty() {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.rng.shuffle(&mut self.state.seats[s.index()].library);
                    continue;
                }
                Interrupt::CorruptionOwner { minion, .. } => {
                    if self.state.minions[minion.index()].corruption.len() > 1 {
                        return;
                    }
                    self.interrupts.remove(0);
                    let v = &mut self.state.minions[minion.index()];
                    if let [(owner, _)] = v.corruption[..] {
                        v.burn_corruption(owner, 1);
                    }
                    continue;
                }
                Interrupt::MinionOf { of, purpose, .. } => {
                    !self.minion_of_options(of, purpose).is_empty()
                }
                Interrupt::AllyFromAsh { seat, of, .. } => !self.ash_allies(seat, of).is_empty(),
                Interrupt::UncontrolledBlood { seat, clan, .. } => {
                    !self.uncontrolled_of_clan(seat, clan).is_empty()
                }
                Interrupt::BecomesAnarch { seat } => !self.untitled_vampires(seat).is_empty(),
                // Nothing left to choose: the chosen cards are shuffled in.
                Interrupt::AshurPicks {
                    seat,
                    left,
                    to_library,
                } => {
                    if left > 0 && !self.state.seats[seat.index()].ash.is_empty() {
                        return;
                    }
                    self.interrupts.remove(0);
                    if to_library {
                        let library = &mut self.state.seats[seat.index()].library;
                        self.rng.shuffle(library);
                    }
                    continue;
                }
                Interrupt::MoveCardToTitled { card, .. } => {
                    if !self.state.cards[card.index()].present() {
                        false
                    } else if self.titled_hosts(card).is_empty() {
                        self.interrupts.remove(0);
                        self.burn_card_in_play(card);
                        continue;
                    } else {
                        true
                    }
                }
                Interrupt::MoveForBlood {
                    payer, minion, n, ..
                } => {
                    let v = &self.state.minions[minion.index()];
                    let p = &self.state.minions[payer.index()];
                    v.present() && v.in_torpor && v.hostage == 0 && p.present() && p.blood >= n
                }
                Interrupt::GiveBlood { seat, from } => {
                    let v = &self.state.minions[from.index()];
                    v.present() && v.blood > 0 && !self.blood_recipients(seat).is_empty()
                }
                Interrupt::PayOrBurn { card, .. } => self.state.cards[card.index()].present(),
                Interrupt::LockOrDiscard { minion, .. } => {
                    self.state.minions[minion.index()].in_play
                }
                Interrupt::EntryBurn { ally, .. } => {
                    if self.state.minions[ally.index()].in_play
                        && self.entry_burn_options(s, ally).is_empty()
                    {
                        self.interrupts.remove(0);
                        self.burn_minion(ally);
                        continue;
                    }
                    self.state.minions[ally.index()].in_play
                }
            };
            if wait {
                return;
            }
            self.interrupts.remove(0);
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
                    .filter(|&i| crate::state::first_copy(&st.seats[s.index()].hand, i))
                    .map(|i| Choice::Bottom(i as u8))
                    .collect(),
            ),
            Interrupt::StoreSearch { .. } => {
                // Any non-unique equipment, by card: out of play, it needs
                // no definition to be drawn later.
                let found = distinct(&st.seats[s.index()].library, |c| {
                    CardFilter::NonUniqueEquipment.matches(c)
                });
                let mut options = vec![Choice::Pass];
                options.extend(found.into_iter().map(Choice::FindCard));
                (WindowLabel::LibrarySearch, options)
            }
            Interrupt::EntryBurn { ally, .. } => {
                (WindowLabel::EntryEffect, self.entry_burn_options(s, ally))
            }
            Interrupt::GiveBlood { seat, .. } => (
                WindowLabel::ResolutionChoice,
                self.blood_recipients(seat)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect(),
            ),
            // Paying is offered if the pool covers it (down to 0 at most).
            Interrupt::PayOrBurn { pool, .. } => (
                WindowLabel::ResolutionChoice,
                if st.seats[s.index()].pool >= pool as i32 {
                    vec![Choice::Pass, Choice::PayToCancel]
                } else {
                    vec![Choice::Pass]
                },
            ),
            // Lock it, or discard a card (one choice per card name).
            Interrupt::LockOrDiscard { .. } => {
                let mut options = vec![Choice::Pass];
                options.extend(self.discard_choices(s));
                (WindowLabel::ResolutionChoice, options)
            }
            // The look: every card, one choice per card name.
            Interrupt::DiscardFromHandOf { of, .. } => (
                WindowLabel::ResolutionChoice,
                distinct(&st.seats[of.index()].hand, |_| true)
                    .into_iter()
                    .map(Choice::FindCard)
                    .collect(),
            ),
            Interrupt::AvoidDamage { .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::BurnBloodToAvoid],
            ),
            Interrupt::Reorder { total, placed, .. } => {
                let lib = &st.seats[s.index()].library;
                let n = lib.len();
                (
                    WindowLabel::ResolutionChoice,
                    distinct(&lib[n - total as usize..n - placed as usize], |_| true)
                        .into_iter()
                        .map(Choice::FindCard)
                        .collect(),
                )
            }
            Interrupt::DiscardChosen { combat_only, .. } => (
                WindowLabel::ResolutionChoice,
                (0..st.seats[s.index()].hand.len())
                    .filter(|&i| crate::state::first_copy(&st.seats[s.index()].hand, i))
                    .filter(|&i| {
                        !combat_only
                            || crate::library::LIBRARY[st.seats[s.index()].hand[i] as usize]
                                .types
                                .contains(&crate::library::CardType::Combat)
                    })
                    .map(|i| Choice::Discard(i as u8))
                    .collect(),
            ),
            Interrupt::RemoveFromAsh { of, .. } => (
                WindowLabel::ResolutionChoice,
                distinct(&st.seats[of.index()].ash, |_| true)
                    .into_iter()
                    .map(Choice::FindCard)
                    .chain(
                        distinct(&st.seats[of.index()].ash_crypt, |_| true)
                            .into_iter()
                            .map(Choice::AshVampire),
                    )
                    .collect(),
            ),
            Interrupt::MoveCardToTitled { card, .. } => (
                WindowLabel::ResolutionChoice,
                self.titled_hosts(card)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect(),
            ),
            Interrupt::StealForCorruption { minion, .. }
            | Interrupt::MoveForBlood { minion, .. }
            | Interrupt::CancelAttempt { minion, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ChooseMinion(minion)],
            ),
            Interrupt::MakeAnarch { card, .. }
            | Interrupt::LockForPool { card, .. }
            | Interrupt::PoolForCard { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolveRider { card: card.0 }],
            ),
            Interrupt::ExchangeAnarch { ash: None, .. } => (
                WindowLabel::ResolutionChoice,
                distinct(&st.seats[s.index()].ash, |c| {
                    CardFilter::HasLabel(Label::RequiresAnarch).matches(c)
                })
                .into_iter()
                .map(Choice::FindCard)
                .collect(),
            ),
            Interrupt::ExchangeAnarch { ash: Some(_), .. } => {
                (WindowLabel::ResolutionChoice, self.discard_choices(s))
            }
            Interrupt::ChooseForCard { .. } => (
                WindowLabel::ResolutionChoice,
                self.choosable(front)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect(),
            ),
            Interrupt::RemoveUncontrolled { .. } => (
                WindowLabel::ResolutionChoice,
                (0..st.seats[s.index()].uncontrolled.len())
                    .map(|i| Choice::Slot(i as u8))
                    .collect(),
            ),
            Interrupt::DisciplineSearch { .. } => {
                let mut options = vec![Choice::Pass];
                options.extend(self.discipline_masters(s));
                (WindowLabel::LibrarySearch, options)
            }
            Interrupt::CorruptionOwner { minion, .. } => (
                WindowLabel::ResolutionChoice,
                st.minions[minion.index()]
                    .corruption
                    .iter()
                    .map(|&(s, _)| Choice::ChooseSeat(s))
                    .collect(),
            ),
            Interrupt::AshurPicks { seat, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        distinct(&st.seats[seat.index()].ash, |_| true)
                            .into_iter()
                            .map(Choice::FindCard),
                    )
                    .collect(),
            ),
            Interrupt::BecomesAnarch { seat } => (
                WindowLabel::ResolutionChoice,
                self.untitled_vampires(seat)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect(),
            ),
            Interrupt::UncontrolledBlood { seat, clan, .. } => (
                WindowLabel::ResolutionChoice,
                self.uncontrolled_of_clan(seat, clan)
                    .into_iter()
                    .map(|i| Choice::Slot(i as u8))
                    .collect(),
            ),
            Interrupt::AllyFromAsh { seat, of, .. } => (
                WindowLabel::ResolutionChoice,
                self.ash_allies(seat, of)
                    .into_iter()
                    .map(Choice::FindCard)
                    .collect(),
            ),
            Interrupt::MinionOf { of, purpose, .. } => {
                let mut options: Vec<Choice> = self
                    .minion_of_options(of, purpose)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect();
                if !matches!(
                    purpose,
                    MinionPurpose::Corrupt
                        | MinionPurpose::BurnBloodOfClan(_)
                        | MinionPurpose::HostCard(_)
                        | MinionPurpose::LockOldest(_)
                        | MinionPurpose::BurnOfCrowdedClan
                        | MinionPurpose::Damage(_)
                ) {
                    options.insert(0, Choice::Pass);
                }
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::BlockToll { .. } => (
                WindowLabel::BlockToll,
                vec![Choice::PayBlockToll, Choice::Pass],
            ),
            Interrupt::MarginPool { .. } => (
                WindowLabel::Distribution,
                self.margin_pool_range(front)
                    .into_iter()
                    .map(|n| Choice::AllocateTo { seat: s, n })
                    .collect(),
            ),
            Interrupt::MarginBlood { .. } => {
                let first = self.margin_recipients(front)[0];
                (
                    WindowLabel::Distribution,
                    vec![Choice::ChooseMinion(first), Choice::Pass],
                )
            }
            Interrupt::EntryRemoveOrBurn { .. } => {
                // One choice per card: copies are alike.
                let mut options = vec![Choice::Pass];
                options.extend(self.ash_allies(s, &[]).into_iter().map(Choice::FindCard));
                options.extend(
                    distinct(&st.seats[s.index()].ash_crypt, |_| true)
                        .into_iter()
                        .map(Choice::AshVampire),
                );
                (WindowLabel::EntryEffect, options)
            }
        };
        Some(DecisionRequest::new(s, window, options))
    }

    /// The ready vampires a `ChooseForCard` may pick: its seat's, of its
    /// clan, other than the first chosen for a second choice.
    /// The vampires `card` may move to (`Interrupt::MoveCardToTitled`):
    /// every other justicar or Inner Circle member in play, arena order.
    pub(crate) fn titled_hosts(&self, card: CardId) -> Vec<crate::ids::MinionId> {
        use crate::cards::TitleClass;
        let st = &self.state;
        let host = st.cards[card.index()].host;
        (0..st.minions.len())
            .map(|i| crate::ids::MinionId(i as u32))
            .filter(|&m| {
                Some(m) != host
                    && st.minions[m.index()].present()
                    && st.minions[m.index()].is_vampire()
                    && [TitleClass::Justicar, TitleClass::InnerCircle]
                        .iter()
                        .any(|c| c.matches(st.title_of(m)))
            })
            .collect()
    }

    /// The minions of `of` an `Interrupt::MinionOf` may choose.
    pub(crate) fn minion_of_options(
        &self,
        of: SeatId,
        purpose: MinionPurpose,
    ) -> Vec<crate::ids::MinionId> {
        let st = &self.state;
        st.minions_of(of)
            .filter(|m| {
                let v = &st.minions[m.index()];
                match purpose {
                    MinionPurpose::LockVampire => v.is_vampire(),
                    MinionPurpose::UnlockAnarch => {
                        v.ready() && st.sect_of(*m) == Some(crate::crypt::Sect::Anarch)
                    }
                    MinionPurpose::BurnBloodOfClan(c) => {
                        v.ready() && v.clan() == Some(c) && v.blood > 0
                    }
                    // "Any ready minion" (LSJ 19990928).
                    MinionPurpose::HostCard(c) => {
                        v.ready() && st.can_hold(*m, st.cards[c.index()].card, &[])
                    }
                    MinionPurpose::Damage(_) => v.ready(),
                    MinionPurpose::BurnOfCrowdedClan => {
                        v.is_vampire()
                            && v.clan().is_some_and(|c| {
                                st.minions_of(of)
                                    .filter(|&o| {
                                        let w = &st.minions[o.index()];
                                        w.is_vampire() && w.clan() == Some(c)
                                    })
                                    .count()
                                    >= 3
                            })
                    }
                    MinionPurpose::LockOldest(c) => {
                        v.clan() == Some(c)
                            && st.capacity_of(*m)
                                == st
                                    .minions_of(of)
                                    .filter(|&o| st.minions[o.index()].clan() == Some(c))
                                    .filter_map(|o| st.capacity_of(o))
                                    .max()
                    }
                    MinionPurpose::Lock | MinionPurpose::Corrupt => true,
                }
            })
            .collect()
    }

    /// Move `card`, an ally in `s`'s ash heap, to the ready region (locked
    /// with `locked`): put in play, not recruited — its inferior version,
    /// its starting life, able to act this turn (LSJ 20100204).
    pub(crate) fn raise_ash_ally(&mut self, s: SeatId, card: u16, locked: bool) {
        let ash = &mut self.state.seats[s.index()].ash;
        let k = ash.iter().rposition(|&c| c == card).expect("ash ally");
        ash.remove(k);
        let m = self.recruit_ally(s, card, 0);
        self.state.minions[m.index()].recruited = false;
        self.state.minions[m.index()].locked |= locked;
    }

    /// The allies in `seat`'s ash heap matching one of `of` (any, if
    /// empty), one entry per card.
    pub(crate) fn ash_allies(&self, seat: SeatId, of: &[crate::cards::CardFilter]) -> Vec<u16> {
        distinct(&self.state.seats[seat.index()].ash, |c| {
            crate::cards::ash_ally(c, of)
        })
    }

    /// The Methuselahs still in the game with an ally in their ash heap.
    pub(crate) fn ash_ally_heaps(&self) -> Vec<SeatId> {
        (0..self.state.seats.len() as u8)
            .map(SeatId)
            .filter(|&s| !self.state.seats[s.index()].ousted && !self.ash_allies(s, &[]).is_empty())
            .collect()
    }

    /// The slots of `seat`'s uncontrolled region holding a `clan` vampire.
    fn uncontrolled_of_clan(&self, seat: SeatId, clan: &'static str) -> Vec<usize> {
        (0..self.state.seats[seat.index()].uncontrolled.len())
            .filter(|&i| self.state.slot_vampire(seat, i).1 == clan)
            .collect()
    }

    /// The non-titled vampires `seat` controls, in torpor too (Gilbert
    /// Duane: an Anarch one a legal no-op pick).
    fn untitled_vampires(&self, seat: SeatId) -> Vec<crate::ids::MinionId> {
        let st = &self.state;
        st.minions_of(seat)
            .filter(|&m| {
                st.minions[m.index()].is_vampire() && st.title_of(m) == crate::crypt::Title::None
            })
            .collect()
    }

    fn choosable(&self, front: Interrupt) -> Vec<crate::ids::MinionId> {
        let Interrupt::ChooseForCard {
            seat,
            card,
            clan,
            second,
        } = front
        else {
            unreachable!("not a choice for a card");
        };
        let st = &self.state;
        let first = st.cards[card.index()].chosen.first().copied();
        st.minions_of(seat)
            .filter(|&m| {
                let v = &st.minions[m.index()];
                v.ready()
                    && v.is_vampire()
                    && clan.is_none_or(|c| v.clan() == Some(c))
                    && !(second && Some(m) == first)
            })
            .collect()
    }

    /// Record a `ChooseForCard` pick: the card's chosen vampire, or its
    /// second one; none if there was none to choose.
    fn set_chosen(&mut self, front: Interrupt, m: Option<crate::ids::MinionId>) {
        let Interrupt::ChooseForCard { card, second, .. } = front else {
            unreachable!("not a choice for a card");
        };
        let chosen = &mut self.state.cards[card.index()].chosen;
        if second {
            chosen.truncate(1);
            chosen.extend(m);
        } else {
            *chosen = m.into_iter().collect();
        }
    }

    /// Minions about to leave the ready region (burned, to torpor, out of
    /// play, removed with their ousted Methuselah): the cards on them that
    /// pay for it offer their pool (Priority Contract).
    pub(crate) fn about_to_leave_ready(&mut self, ms: &[crate::ids::MinionId]) {
        for &m in ms {
            if !self.state.minions[m.index()].ready() {
                continue;
            }
            for ci in 0..self.state.cards.len() {
                let id = CardId(ci as u32);
                let c = &self.state.cards[ci];
                if !c.present() || c.host != Some(m) {
                    continue;
                }
                let n = cards::lookup(c.card).and_then(|d| {
                    d.modes[c.mode as usize]
                        .effects
                        .iter()
                        .find_map(|e| match e {
                            crate::cards::Effect::PoolWhenHostLeaves(n) => Some(*n),
                            _ => None,
                        })
                });
                if let Some(n) = n {
                    let seat = self.state.card_controller(id);
                    self.interrupts
                        .push(Interrupt::PoolForCard { seat, card: id, n });
                    // The Black Throne of the Methuselah whose vampire the
                    // contract chose (PIB 20150512, ANK 20180129-2).
                    if let Some(&chosen) = self.state.cards[ci].chosen.first() {
                        let x = self.state.minions[chosen.index()].owner;
                        self.contract_riders(x);
                    }
                }
            }
        }
    }

    /// The Black Throne of `x`'s: unlocked, offered to lock for pool.
    fn contract_riders(&mut self, x: SeatId) {
        for (id, r) in self.state.card_riders(Hook::ContractHostLeaves) {
            let c = &self.state.cards[id.index()];
            if !c.present() || c.locked || self.state.card_controller(id) != x {
                continue;
            }

            for e in r.effects {
                if let crate::cards::Effect::GainPool(n) = *e {
                    self.interrupts.push(Interrupt::LockForPool {
                        seat: x,
                        card: id,
                        n,
                    });
                }
            }
        }
    }

    /// `seat` takes control of the card in play `card` (stolen, given): a
    /// card that chooses on a change of control has its new controller
    /// choose (The Rack).
    /// Vampires of other Methuselahs a vampire of `seat`'s may give blood
    /// to (Blood Trade).
    fn blood_recipients(&self, seat: SeatId) -> Vec<MinionId> {
        (0..self.state.minions.len())
            .map(|i| MinionId(i as u32))
            .filter(|&m| {
                let v = &self.state.minions[m.index()];
                v.present() && v.is_vampire() && v.owner != seat
            })
            .collect()
    }

    /// A location changing controller (Dominate Kine [DOM], Disputed
    /// Territory): one on a minion goes on a ready minion of the new
    /// controller, their choice — burned if none can hold it (LSJ
    /// 19990928; rules-qa 39.4).
    pub(crate) fn give_location(&mut self, card: CardId, seat: SeatId) {
        self.give_card_control(card, seat);
        if self.state.cards[card.index()].host.is_none() {
            return;
        }
        if self
            .minion_of_options(seat, MinionPurpose::HostCard(card))
            .is_empty()
        {
            self.burn_card_in_play(card);
        } else {
            self.interrupts.push(Interrupt::MinionOf {
                seat,
                of: seat,
                purpose: MinionPurpose::HostCard(card),
            });
        }
    }

    pub(crate) fn give_card_control(&mut self, card: CardId, seat: SeatId) {
        self.state.cards[card.index()].controller = seat;
        let rechooses = cards::lookup(self.state.cards[card.index()].card).is_some_and(|d| {
            d.modes[0]
                .effects
                .contains(&crate::cards::Effect::RechoosesOnControlChange)
        });
        if rechooses {
            self.interrupts.push(Interrupt::ChooseForCard {
                seat,
                card,
                clan: None,
                second: false,
            });
        }
    }

    /// Remove `seat`'s uncontrolled vampire `i` from the game: its crypt
    /// card to the removed cards, its counters to the blood bank; a
    /// banished one leaves with the cards on it, burned (a created one's
    /// own card removed with it).
    pub(crate) fn remove_uncontrolled(&mut self, seat: SeatId, i: usize) {
        let slot = self.state.seats[seat.index()].uncontrolled.remove(i);
        if let Some(b) = slot.banished {
            for c in b.cards {
                if self.state.cards[c.index()].body.is_some() {
                    let card = self.state.cards[c.index()].card;
                    self.state.seats[seat.index()].removed.push(card);
                    continue;
                }
                // Out of play on it: straight to its owner's ash heap.
                let (owner, card) = (
                    self.state.cards[c.index()].owner,
                    self.state.cards[c.index()].card,
                );
                self.state.seats[owner.index()].ash.push(card);
            }
        }
        if slot.card != crate::state::CREATED {
            self.state.seats[seat.index()].removed_crypt.push(slot.card);
        }
    }

    /// The Discipline master cards `s` could take, by zone and card: the
    /// library's by card (its order hidden), the hand's and ash heap's.
    fn discipline_masters(&self, s: SeatId) -> Vec<Choice> {
        let seat = &self.state.seats[s.index()];
        let mut out = Vec::new();
        for (zone, cards) in [
            (Zone::Library, &seat.library),
            (Zone::Hand, &seat.hand),
            (Zone::AshHeap, &seat.ash),
        ] {
            out.extend(
                distinct(cards, |c| {
                    CardFilter::HasLabel(Label::Discipline).matches(c)
                })
                .into_iter()
                .map(|card| Choice::TakeFrom { zone, card }),
            );
        }
        out
    }

    /// One discard choice per card name in `s`'s hand (its first copy).
    pub(crate) fn discard_choices(&self, s: SeatId) -> Vec<Choice> {
        let hand = &self.state.seats[s.index()].hand;
        (0..hand.len())
            .filter(|&i| !hand[..i].contains(&hand[i]))
            .map(|i| Choice::Discard(i as u8))
            .collect()
    }

    pub(crate) fn interrupt_apply(&mut self, choice: &Choice) {
        let front = self.interrupts[0];
        let s = front.seat();
        match (front, *choice) {
            (
                Interrupt::StealForCorruption { .. }
                | Interrupt::MakeAnarch { .. }
                | Interrupt::LockForPool { .. }
                | Interrupt::PoolForCard { .. }
                | Interrupt::MoveForBlood { .. }
                | Interrupt::CancelAttempt { .. }
                | Interrupt::MinionOf { .. },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
            }
            // The player's counters burned; the minion theirs for good.
            (Interrupt::StealForCorruption { minion, .. }, Choice::ChooseMinion(_)) => {
                self.interrupts.remove(0);
                let v = &mut self.state.minions[minion.index()];
                let n = v.corruption_of(s);
                v.burn_corruption(s, n);
                if v.owner != s {
                    let origin = v.origin.or(v.temp_from).unwrap_or(v.owner);
                    v.origin = (origin != s).then_some(origin);
                    v.temp_from = None;
                    v.owner = s;
                    if v.is_ally() {
                        self.enter_card_contest(minion);
                    } else {
                        self.enter_title_contest(minion);
                    }
                }
            }
            (Interrupt::MakeAnarch { card, minion, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].sect_override = Some(crate::crypt::Sect::Anarch);
                let c = &mut self.state.cards[card.index()];
                c.counters -= 1;
                if c.counters == 0 {
                    self.burn_card_in_play(card);
                }
            }
            (Interrupt::LockForPool { card, n, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.state.cards[card.index()].locked = true;
                self.state.seats[s.index()].pool += n as i32;
            }
            // Burned with its minion or not (PIB 20150512): the pool all
            // the same.
            (Interrupt::PoolForCard { card, n, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.burn_card_in_play(card);
                self.state.seats[s.index()].pool += n as i32;
            }
            (Interrupt::ExchangeAnarch { ash: None, .. }, Choice::FindCard(card)) => {
                self.interrupts[0] = Interrupt::ExchangeAnarch {
                    seat: s,
                    ash: Some(card),
                };
            }
            (
                Interrupt::ExchangeAnarch {
                    ash: Some(card), ..
                },
                Choice::Discard(i),
            ) => {
                self.interrupts.remove(0);
                let seat = &mut self.state.seats[s.index()];
                let held = seat.hand.remove(i as usize);
                let k = seat
                    .ash
                    .iter()
                    .position(|&c| c == card)
                    .expect("named ash card");
                seat.ash.remove(k);
                seat.ash.push(held);
                seat.hand.push(card);
            }
            (Interrupt::ChooseForCard { .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.set_chosen(front, Some(m));
            }
            (Interrupt::AshurPicks { to_library, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                if to_library {
                    self.rng.shuffle(&mut self.state.seats[s.index()].library);
                }
            }
            (
                Interrupt::AshurPicks {
                    seat,
                    left,
                    to_library,
                },
                Choice::FindCard(card),
            ) => {
                let st = &mut self.state.seats[seat.index()];
                let i = st
                    .ash
                    .iter()
                    .position(|&c| c == card)
                    .expect("in the ash heap");
                st.ash.remove(i);
                if to_library {
                    st.library.push(card);
                } else {
                    st.hand.push(card);
                }
                self.interrupts[0] = Interrupt::AshurPicks {
                    seat,
                    left: left - 1,
                    to_library: true,
                };
            }
            (Interrupt::BecomesAnarch { .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.state.minions[m.index()].sect_override = Some(crate::crypt::Sect::Anarch);
            }
            (Interrupt::UncontrolledBlood { n, .. }, Choice::Slot(i)) => {
                self.interrupts.remove(0);
                self.state.add_uncontrolled_blood(s, i as usize, n);
            }
            (Interrupt::AllyFromAsh { named, .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                if named {
                    self.action.as_mut().expect("action").ash_named = Some(card);
                } else {
                    self.raise_ash_ally(s, card, false);
                }
            }
            (Interrupt::MoveCardToTitled { card, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.state.cards[card.index()].host = Some(m);
            }
            (Interrupt::MoveForBlood { payer, n, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.state.minions[payer.index()].blood -= n;
                self.leave_torpor(m);
            }
            (Interrupt::RemoveUncontrolled { .. }, Choice::Slot(i)) => {
                self.interrupts.remove(0);
                self.remove_uncontrolled(s, i as usize);
            }
            (Interrupt::DisciplineSearch { minion, .. }, c) => {
                self.interrupts.remove(0);
                if let Choice::TakeFrom { zone, card } = c {
                    let seat = &mut self.state.seats[s.index()];
                    let from = match zone {
                        Zone::Library => &mut seat.library,
                        Zone::Hand => &mut seat.hand,
                        Zone::AshHeap => &mut seat.ash,
                    };
                    let k = from.iter().rposition(|&x| x == card).expect("found card");
                    from.remove(k);
                    self.put_on_minion(s, card, minion);
                    // Out of the hand: drawn back up at once (advanced rules
                    // 6.8).
                    if zone == Zone::Hand {
                        self.replace(s, 1, None);
                    }
                }
                self.rng.shuffle(&mut self.state.seats[s.index()].library);
            }
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
            (Interrupt::EntryRemoveOrBurn { ally, .. }, c) => {
                self.interrupts.remove(0);
                let seat = &mut self.state.seats[s.index()];
                match c {
                    Choice::FindCard(card) => {
                        let k = seat
                            .ash
                            .iter()
                            .position(|&x| x == card)
                            .expect("in the ash heap");
                        seat.ash.remove(k);
                        seat.removed.push(card);
                    }
                    Choice::AshVampire(card) => {
                        let k = seat
                            .ash_crypt
                            .iter()
                            .position(|&x| x == card)
                            .expect("in the ash heap");
                        seat.ash_crypt.remove(k);
                        seat.removed_crypt.push(card);
                    }
                    Choice::Pass => self.burn_minion(ally),
                    other => unreachable!("illegal entry choice {other:?}"),
                }
            }
            (Interrupt::BlockToll { minion, .. }, Choice::PayBlockToll) => {
                self.interrupts.remove(0);
                self.pay_block_toll(minion, false);
            }
            (Interrupt::CancelAttempt { .. }, Choice::ChooseMinion(_)) => {
                self.interrupts.remove(0);
                self.resign_attempt();
            }
            (Interrupt::BlockToll { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.resign_attempt();
            }
            (Interrupt::MarginPool { .. }, Choice::AllocateTo { n, .. }) => {
                self.interrupts.remove(0);
                self.margin_to_pool(front, n);
            }
            (
                Interrupt::MarginBlood {
                    seat, left, clan, ..
                },
                Choice::ChooseMinion(m),
            ) => {
                self.state.gain_blood(m, 1);
                self.interrupts[0] = Interrupt::MarginBlood {
                    seat,
                    left: left - 1,
                    next: m.0 + 1,
                    clan,
                };
            }
            (
                Interrupt::MarginBlood {
                    seat, left, clan, ..
                },
                Choice::Pass,
            ) => {
                let skipped = self.margin_recipients(front)[0];
                self.interrupts[0] = Interrupt::MarginBlood {
                    seat,
                    left,
                    next: skipped.0 + 1,
                    clan,
                };
            }
            (Interrupt::DiscardFromHandOf { of, .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                let st = &mut self.state.seats[of.index()];
                let i = st.hand.iter().position(|&c| c == card).expect("in hand");
                st.hand.remove(i);
                st.ash.push(card);
                self.replace(of, 1, None);
            }
            (Interrupt::AvoidDamage { minion, damage, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.damage_out_of_combat(minion, damage, true);
            }
            (Interrupt::AvoidDamage { minion, blood, .. }, Choice::BurnBloodToAvoid) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].blood -= blood;
            }
            // The chosen card to the next place down from the top.
            (
                Interrupt::Reorder {
                    seat,
                    total,
                    placed,
                },
                Choice::FindCard(card),
            ) => {
                let lib = &mut self.state.seats[seat.index()].library;
                let n = lib.len();
                let to = n - 1 - placed as usize;
                let from = (n - total as usize..=to)
                    .rev()
                    .find(|&k| lib[k] == card)
                    .expect("the chosen card among the top ones");
                let c = lib.remove(from);
                lib.insert(to, c);
                self.interrupts[0] = Interrupt::Reorder {
                    seat,
                    total,
                    placed: placed + 1,
                };
            }
            (
                Interrupt::DiscardChosen {
                    seat,
                    left,
                    done,
                    combat_only,
                },
                Choice::Discard(i),
            ) => {
                let st = &mut self.state.seats[seat.index()];
                let card = st.hand.remove(i as usize);
                st.ash.push(card);
                self.interrupts[0] = Interrupt::DiscardChosen {
                    seat,
                    left: left - 1,
                    done: done + 1,
                    combat_only,
                };
            }
            (
                Interrupt::RemoveFromAsh {
                    seat,
                    of,
                    left,
                    pool,
                },
                c @ (Choice::FindCard(_) | Choice::AshVampire(_)),
            ) => {
                let st = &mut self.state.seats[of.index()];
                match c {
                    Choice::FindCard(card) => {
                        let i = st
                            .ash
                            .iter()
                            .position(|&x| x == card)
                            .expect("in the ash heap");
                        st.ash.remove(i);
                        st.removed.push(card);
                    }
                    Choice::AshVampire(card) => {
                        let i = st
                            .ash_crypt
                            .iter()
                            .position(|&x| x == card)
                            .expect("in the ash heap");
                        st.ash_crypt.remove(i);
                        st.removed_crypt.push(card);
                    }
                    _ => unreachable!(),
                }
                self.interrupts[0] = Interrupt::RemoveFromAsh {
                    seat,
                    of,
                    left: left - 1,
                    pool,
                };
            }
            (Interrupt::CorruptionOwner { minion, .. }, Choice::ChooseSeat(owner)) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].burn_corruption(owner, 1);
            }
            (Interrupt::MinionOf { seat, purpose, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                let v = &mut self.state.minions[m.index()];
                match purpose {
                    MinionPurpose::Lock
                    | MinionPurpose::LockVampire
                    | MinionPurpose::LockOldest(_) => v.locked = true,
                    MinionPurpose::HostCard(c) => self.state.cards[c.index()].host = Some(m),
                    MinionPurpose::Damage(n) => self.damage_out_of_combat(m, n, false),
                    MinionPurpose::UnlockAnarch => v.locked = false,
                    MinionPurpose::BurnBloodOfClan(_) => v.blood -= 1,
                    MinionPurpose::Corrupt => v.add_corruption(seat, 1),
                    // Capacity 6 or more: that Methuselah ignores it for the
                    // rest of the game.
                    MinionPurpose::BurnOfCrowdedClan => {
                        if self.state.capacity_of(m).is_some_and(|c| c >= 6) {
                            self.state.seats[seat.index()].recalled_ignored = true;
                        }
                        self.burn_minion(m);
                    }
                }
            }
            (Interrupt::GiveBlood { from, .. }, Choice::ChooseMinion(to)) => {
                self.interrupts.remove(0);
                self.state.minions[from.index()].blood -= 1;
                self.state.gain_blood(to, 1);
            }
            // Not paid: burned.
            (Interrupt::PayOrBurn { card, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.burn_card_in_play(card);
            }
            (Interrupt::PayOrBurn { seat, pool, .. }, Choice::PayToCancel) => {
                self.interrupts.remove(0);
                self.pay_pool_for_ability(seat, pool as i32);
            }
            (Interrupt::LockOrDiscard { minion, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].locked = true;
            }
            (Interrupt::LockOrDiscard { seat, .. }, Choice::Discard(i)) => {
                self.interrupts.remove(0);
                let s = &mut self.state.seats[seat.index()];
                let c = s.hand.remove(i as usize);
                s.ash.push(c);
                s.held.master += 1;
            }
            (Interrupt::EntryBurn { .. }, Choice::BurnInPlay { card }) => {
                self.interrupts.remove(0);
                self.burn_card_in_play(CardId(card));
            }
            (front, choice) => unreachable!("illegal choice {choice:?} for {front:?}"),
        }
    }

    /// The ready `clan` vampires below capacity the Methuselah controls,
    /// from minion `next` on (Amici Noctis).
    fn margin_recipients(&self, front: Interrupt) -> Vec<crate::ids::MinionId> {
        let (s, from, clan) = match front {
            Interrupt::MarginPool { seat, clan, .. } => (seat, 0, clan),
            Interrupt::MarginBlood {
                seat, next, clan, ..
            } => (seat, next, clan),
            _ => unreachable!("not a distribution"),
        };
        let st = &self.state;
        st.minions_of(s)
            .filter(|m| {
                let v = &st.minions[m.index()];
                m.0 >= from
                    && v.ready()
                    && v.clan() == Some(clan)
                    && st.capacity_of(*m).is_some_and(|c| v.blood < c)
            })
            .collect()
    }

    /// The pool amounts Amici Noctis may give: at most its cap, and at
    /// least what the vampires cannot take (all of it distributed where it
    /// fits).
    fn margin_pool_range(&self, front: Interrupt) -> Vec<u8> {
        let Interrupt::MarginPool { left, cap, .. } = front else {
            unreachable!("not the pool step");
        };
        let room = self.margin_recipients(front).len().min(u8::MAX as usize) as u8;
        let hi = left.min(cap);
        let lo = left.saturating_sub(room).min(hi);
        (lo..=hi).collect()
    }

    /// `n` to the pool; the rest goes to the vampires.
    fn margin_to_pool(&mut self, front: Interrupt, n: u8) {
        let Interrupt::MarginPool {
            seat, left, clan, ..
        } = front
        else {
            unreachable!("not the pool step");
        };
        self.state.seats[seat.index()].pool += n as i32;
        if left > n {
            self.interrupts.insert(
                0,
                Interrupt::MarginBlood {
                    seat,
                    left: left - n,
                    next: 0,
                    clan,
                },
            );
        }
    }

    /// The allies (by card) and retainers `s` controls, other than the
    /// entering `ally`: what War Ghoul's entry may burn.
    fn entry_burn_options(&self, s: SeatId, ally: crate::ids::MinionId) -> Vec<Choice> {
        let st = &self.state;
        st.cards
            .iter()
            .enumerate()
            .filter(|&(i, c)| {
                c.present()
                    && c.body != Some(ally)
                    && st.card_controller(CardId(i as u32)) == s
                    && c.bearer().is_some_and(|m| st.minions[m.index()].present())
                    && crate::cards::CardFilter::AllyOrRetainer.matches(c.card)
            })
            .map(|(i, _)| Choice::BurnInPlay { card: i as u32 })
            .collect()
    }
}
