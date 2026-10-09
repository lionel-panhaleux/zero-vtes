//! Scripted reference bots (DESIGN §11.2 "Bots"): the evaluation ladder's
//! fixed opponents and the league's scripted members (DESIGN §5). Each
//! style scores the offered options in the windows that define it; every
//! other option scores 0 and ties are broken at random, so a bot is
//! "random with a spine": the difference from `RandomLegal` is exactly the
//! heuristics below. Deck-agnostic: a style pilots any decklist.
//!
//! A bot reads only what its seat's legal view shows (DESIGN §5): public
//! state, its own hand and uncontrolled region, and the action's public
//! fields (`seat`, `kind`, `acting`, `bleed_target`, its directed seats) —
//! never `ActionCtx::card` or `exchange`, which carry a card named from
//! hand. `tests/bots.rs` checks it by answering from the redacted state.

use crate::agent::Agent;
use crate::cards::{self, Effect, RefEffect, Term};
use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel as W};
use crate::draws::Interrupt;
use crate::game::Engine;
use crate::ids::{MinionId, SeatId};
use crate::library::{CardType, LIBRARY};
use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Bleeds its prey with every minion, pumps the bleed, blocks only
    /// what threatens it.
    AggroBleed,
    /// Keeps a blocker home, blocks and intercepts everything it can,
    /// brings out more and bigger minions.
    Wall,
}

#[derive(Clone)]
pub struct Scripted {
    pub style: Style,
    rng: Rng,
    /// The minions this bot attempted to block with during the current
    /// action, keyed by the action's public identity (turn, acting minion,
    /// kind, bleed target): a failed attempt is not repeated. A redirect
    /// changes the key, as it reopens the blocks (rulebook "Who May Attempt
    /// to Block").
    attempts: (ActionKey, Vec<MinionId>),
}

type ActionKey = Option<(u32, MinionId, ActionKind, Option<SeatId>)>;

fn action_key(e: &Engine) -> ActionKey {
    e.action()
        .map(|a| (e.state.turn, a.acting, a.kind, a.bleed_target))
}

impl Scripted {
    pub fn new(style: Style, seed: u64) -> Self {
        Self {
            style,
            rng: Rng::new(seed),
            attempts: (None, Vec::new()),
        }
    }
}

impl Agent for Scripted {
    fn decide(&mut self, e: &Engine, req: &DecisionRequest) -> usize {
        let scores: Vec<i32> = req.options.iter().map(|o| self.score(e, req, o)).collect();
        let best = *scores.iter().max().expect("a request has options");
        let top: Vec<usize> = (0..scores.len()).filter(|&i| scores[i] == best).collect();
        let i = top[self.rng.below(top.len())];
        if let Choice::DeclareBlock { minion } = req.options[i] {
            let key = action_key(e);
            if self.attempts.0 != key {
                self.attempts = (key, Vec::new());
            }
            self.attempts.1.push(minion);
        }
        i
    }
}

/// What a card's mode does, as far as the heuristics care.
#[derive(Default)]
pub(crate) struct Does {
    bleed: bool,
    stealth: bool,
    intercept: bool,
    bounce: bool,
    strike: bool,
    prevent: bool,
    /// Votes the card adds, for and against: either side for votes the
    /// player casts as they choose, one side for a card that says which.
    pub(crate) votes_for: bool,
    pub(crate) votes_against: bool,
}

pub(crate) fn does(card: u16, mode: u8) -> Does {
    let mut d = Does::default();
    let Some(m) = cards::lookup(card).and_then(|def| def.modes.get(mode as usize)) else {
        return d;
    };
    for e in m.effects {
        match e {
            Effect::BleedMod { .. } | Effect::BleedModX { .. } => d.bleed = true,
            Effect::AddStealth(_) => d.stealth = true,
            Effect::AddIntercept(_) => d.intercept = true,
            Effect::RedirectBleed | Effect::ReduceBleed(_) | Effect::ReduceBleedToZero => {
                d.bounce = true
            }
            Effect::Strike(_)
            | Effect::CommittedStrike(_)
            | Effect::RoundStrike(_)
            | Effect::AdditionalStrikes { .. }
            | Effect::AdditionalStrikesX { .. }
            | Effect::AdditionalStrikeOnly { .. } => d.strike = true,
            Effect::Prevent(_)
            | Effect::PreventXPlus(_)
            | Effect::PreventNonAggravated(_)
            | Effect::PreventThisRound(_)
            | Effect::PreventEachRound(_) => d.prevent = true,
            Effect::AddVotes(_) | Effect::AddVotesX => {
                d.votes_for = true;
                d.votes_against = true;
            }
            Effect::MethuselahVotes { favor, .. } => {
                d.votes_for |= *favor;
                d.votes_against |= !*favor;
            }
            Effect::AddVotesAgainst(_) => d.votes_against = true,
            _ => {}
        }
    }
    d
}

impl Scripted {
    fn score(&self, e: &Engine, req: &DecisionRequest, o: &Choice) -> i32 {
        let me = req.seat;
        match req.window {
            W::MinionPhase => self.minion_phase(e, me, o),
            W::ActionAsAnnounced
            | W::ActionBlock
            | W::ActionBlocksDeclined
            | W::ActionWouldBeSuccessful
            | W::ActionBlocked
            | W::ActionAfterResolution
            | W::ActionAfterCombat => self.action_window(e, me, o),
            W::ReferendumPolling => self.polling(e, me, o),
            W::ReferendumTerms => referendum_terms(e, req, o),
            W::InfluenceTransfers => self.influence(e, me, o),
            W::MasterPhase => match o {
                Choice::Play { .. } => 30,
                Choice::UseAbility { .. } => 10,
                _ => 0,
            },
            // An optional effect never worse to use (Fever Pitch put on its
            // vampire, Cursed Abattoir's counter; rules-qa 53.5, 55.6).
            W::ResolutionChoice
                if matches!(
                    e.interrupt(),
                    Some(
                        Interrupt::MayPutOn { .. }
                            | Interrupt::MayAddCounter { .. }
                            | Interrupt::MayGainPool { .. }
                    )
                ) =>
            {
                i32::from(*o != Choice::Pass) * 10
            }
            W::UnlockAbilities => match o {
                Choice::PayContest(_) => 10,
                // The Edge's pool costs nothing.
                Choice::BurnEdge => 10,
                _ => 0,
            },
            W::DiscardPhase | W::DiscardDown => match o {
                Choice::Discard(i) if !playable(e, me, e.playable_card(me, *i)) => 20,
                Choice::Discard(_) if req.window == W::DiscardPhase => -5,
                _ => 0,
            },
            W::CombatBeforeRange
            | W::CombatDetermineRange
            | W::CombatBeforeStrikes
            | W::CombatStrike
            | W::CombatBeforeResolution
            | W::CombatPrevention
            | W::CombatAdditionalStrikes
            | W::CombatPress
            | W::CombatEndOfRound => combat(e, me, o),
            _ => 0,
        }
    }

    fn minion_phase(&self, e: &Engine, me: SeatId, o: &Choice) -> i32 {
        let st = &e.state;
        // A wall keeps one ready unlocked minion home to block.
        let spare = self.style != Style::Wall || st.ready_unlocked(me).len() >= 2;
        let busy = |n: i32| if spare { n } else { -10 };
        match *o {
            Choice::TakeAction { minion, action, .. } => match action {
                ActionKind::Hunt => hunger(e, minion, self.style),
                ActionKind::Bleed => {
                    let bleed = st.base_bleed(minion) as i32;
                    match self.style {
                        Style::AggroBleed => 60 + 10 * bleed,
                        Style::Wall => busy(35),
                    }
                }
                ActionKind::LeaveTorpor => 80,
                ActionKind::Rescue => 50,
                ActionKind::Diablerie => 10,
                ActionKind::Political => self.political(spare),
                _ => busy(20),
            },
            Choice::Play { card, .. } => {
                let types = LIBRARY[e.playable_card(me, card) as usize].types;
                if types.contains(&CardType::PoliticalAction) {
                    self.political(spare)
                } else if types
                    .iter()
                    .any(|t| matches!(t, CardType::Ally | CardType::Retainer | CardType::Equipment))
                {
                    busy(60)
                } else {
                    busy(25)
                }
            }
            Choice::UseAbility { .. } => busy(20),
            _ => 0,
        }
    }

    /// A political action: for aggro-bleed, over a bleed of 1 and under a
    /// bleed of 2.
    fn political(&self, spare: bool) -> i32 {
        match (self.style, spare) {
            (Style::AggroBleed, _) => 75,
            (Style::Wall, true) => 40,
            (Style::Wall, false) => -10,
        }
    }

    fn action_window(&self, e: &Engine, me: SeatId, o: &Choice) -> i32 {
        let Some(a) = e.action() else { return 0 };
        let st = &e.state;
        let threatened = a.bleed_target == Some(me) || a.directed_seats(st).contains(me);
        match *o {
            // Another attempt is legal as often as wished (rulebook "Who May
            // Attempt to Block"): one that failed is not repeated.
            Choice::DeclareBlock { minion }
                if self.attempts.0 == action_key(e) && self.attempts.1.contains(&minion) =>
            {
                -10
            }
            Choice::DeclareBlock { .. } => match (self.style, threatened) {
                (Style::Wall, true) => 60,
                (Style::Wall, false) => 30,
                (_, true) => 40,
                (_, false) => -5,
            },
            Choice::Play { card, mode, .. } => {
                let d = does(e.playable_card(me, card), mode);
                if a.seat == me {
                    match (d.bleed, d.stealth) {
                        (true, _) if self.style == Style::AggroBleed => 50,
                        (true, _) | (_, true) => 40,
                        _ => 10,
                    }
                } else if d.bounce && threatened {
                    70
                } else if d.intercept {
                    match (self.style, threatened) {
                        (Style::Wall, _) => 50,
                        (_, true) => 30,
                        _ => 5,
                    }
                } else if threatened {
                    15
                } else {
                    -5
                }
            }
            _ => 0,
        }
    }

    fn polling(&self, e: &Engine, me: SeatId, o: &Choice) -> i32 {
        let Some(r) = e.referendum() else { return 0 };
        let mine = r.caller == me;
        match *o {
            Choice::CastVotes { favor, .. } | Choice::CastCardVote { favor } => {
                if favor == mine {
                    50
                } else {
                    -20
                }
            }
            Choice::CastEdge { favor } | Choice::BurnForVote { favor, .. } => {
                if favor && mine {
                    15
                } else {
                    -5
                }
            }
            Choice::Play { card, mode, .. } => {
                let d = does(e.playable_card(me, card), mode);
                match mine {
                    true if d.votes_for => 30,
                    false if d.votes_against => 10,
                    _ => -5,
                }
            }
            _ => 0,
        }
    }

    fn influence(&self, e: &Engine, me: SeatId, o: &Choice) -> i32 {
        let st = &e.state;
        let seat = &st.seats[me.index()];
        let (target, floor) = match self.style {
            Style::Wall => (4, 8),
            _ => (3, 6),
        };
        let vampires = st
            .minions_of(me)
            .filter(|&m| st.minions[m.index()].is_vampire())
            .count();
        let grow = vampires < target && seat.pool > floor;
        match *o {
            Choice::InfluenceOut { .. } => 100,
            Choice::Transfer { slot } if grow => {
                let (capacity, _) = st.slot_vampire(me, slot as usize);
                let need = capacity as i32 - seat.uncontrolled[slot as usize].counters as i32;
                50 - need
            }
            Choice::DrawCrypt if grow && seat.uncontrolled.len() <= 1 => 40,
            Choice::Transfer { .. } | Choice::DrawCrypt => -10,
            Choice::Withdraw { .. } => -50,
            _ => 0,
        }
    }
}

/// The Methuselah a harmful choice goes to: the prey when it may be
/// chosen, the predator otherwise.
fn victim(e: &Engine, me: SeatId, may: impl Fn(SeatId) -> bool) -> Option<SeatId> {
    let st = &e.state;
    [st.prey(me), st.predator(me)]
        .into_iter()
        .find(|&s| s != me && may(s))
}

/// The caller's referendum terms, by what the referendum does: damage
/// (pool burned, minions banished, expelled or locked, cards burned) goes to
/// the victim, gains to the caller; a minion to banish or expel is any of
/// the prey's or the predator's. Other terms stay random.
fn referendum_terms(e: &Engine, req: &DecisionRequest, o: &Choice) -> i32 {
    let (st, me) = (&e.state, req.seat);
    let Some(r) = e.referendum() else { return 0 };
    // Ancient Influence, Reins of Power: the biggest ready vampire.
    if r.step == crate::referendum::RefStep::Choose {
        return match *o {
            Choice::ChooseMinion(m) => st.capacity_of(m).map_or(0, i32::from),
            _ => -1,
        };
    }
    let (Some(def), Some(term)) = (r.def, r.current_term()) else {
        return 0;
    };
    // A Methuselah this term may name, or one already named.
    let named =
        |s: SeatId| req.options.contains(&Choice::ChooseSeat(s)) || r.terms.seats.contains(&s);
    let recipients: Vec<SeatId> = match term {
        Term::Allocate { among, .. } => cards::recipients(st, me, among, r.terms.chosen),
        _ => st.clockwise_from(me),
    };
    // Gains: Parity Shift's allocation; the chosen Methuselah of Camarilla's
    // Iron Fist and Disputed Territory.
    let gain_allocated = def.effect == RefEffect::MoveAllocatedFromChosen;
    let chosen_gains = matches!(
        def.effect,
        RefEffect::BurnAllocatedChosenGains(_) | RefEffect::ChosenTakesChosenCard
    );
    let own_minions = matches!(def.effect, RefEffect::PutOnChosen);
    let foe = |s: SeatId| s != me && [st.prey(me), st.predator(me)].contains(&s);
    match *o {
        Choice::AllocateTo { seat, n } => {
            let n = n as i32;
            if gain_allocated {
                return if seat == me { n } else { -n };
            }
            let target = victim(e, me, |s| recipients.contains(&s));
            match seat {
                s if Some(s) == target => n,
                s if s == me => -3 * n,
                _ => -n,
            }
        }
        Choice::AllocateToCard { card, n } => {
            let n = n as i32;
            if st.card_controller(crate::state::CardId(card)) == me {
                -n
            } else {
                n
            }
        }
        Choice::ChooseSeat(s) if chosen_gains => {
            if s == me {
                20
            } else {
                0
            }
        }
        Choice::ChooseSeat(s) => match victim(e, me, named) {
            Some(v) if v == s => 20,
            _ if s == me => -20,
            _ => -5,
        },
        Choice::ChooseMinion(m) => {
            let owner = st.minions[m.index()].owner;
            match (own_minions, owner == me) {
                (true, true) => 20,
                (true, false) => -20,
                (false, true) => -20,
                (false, false) if foe(owner) => 10,
                _ => 0,
            }
        }
        Choice::ChooseCard(c) => {
            let owner = st.card_controller(crate::state::CardId(c));
            match owner {
                s if s == me => -10,
                s if foe(s) => 10,
                _ => 5,
            }
        }
        Choice::ChooseClan(clan) => st
            .minions
            .iter()
            .filter(|m| m.present() && m.is_vampire() && m.clan() == Some(clan))
            .map(|m| if m.owner == me { 10 } else { -3 })
            .sum(),
        _ => 0,
    }
}

/// Hunting: always when empty, below half capacity for a wall (it wants
/// blood to block and fight), below a third otherwise.
fn hunger(e: &Engine, m: MinionId, style: Style) -> i32 {
    let st = &e.state;
    let blood = st.minions[m.index()].blood as i32;
    let capacity = st.capacity_of(m).unwrap_or(0) as i32;
    let low = match style {
        Style::Wall => 2 * blood < capacity,
        _ => 3 * blood < capacity,
    };
    match (blood, low) {
        (0, _) => 95,
        (_, true) => 70,
        _ => -5,
    }
}

/// Combat, for every style: strike and prevent with cards over the hand
/// strike, other combat cards over passing.
fn combat(e: &Engine, me: SeatId, o: &Choice) -> i32 {
    match *o {
        Choice::Play { card, mode, .. } => {
            let d = does(e.playable_card(me, card), mode);
            if d.strike || d.prevent {
                40
            } else {
                25
            }
        }
        Choice::UsePrevent => 40,
        Choice::HandStrike => 20,
        Choice::UseManeuver | Choice::UsePress => 10,
        _ => 0,
    }
}

/// Could one of the seat's minions ever play this card? Master cards
/// always; others need a matching clan and one of the mode's Disciplines
/// (all of them for a combo) on a minion the seat controls.
fn playable(e: &Engine, me: SeatId, card: u16) -> bool {
    let st = &e.state;
    let c = &LIBRARY[card as usize];
    if c.types.contains(&CardType::Master) {
        return true;
    }
    st.minions_of(me).any(|m| {
        let clan = c.clans.is_empty()
            || st.minions[m.index()]
                .clan()
                .is_some_and(|k| c.clans.contains(&k));
        clan && c.modes.iter().any(|mode| {
            mode.tags.is_empty()
                || if c.combo {
                    mode.tags.iter().all(|t| st.has_discipline(m, t))
                } else {
                    mode.tags.iter().any(|t| st.has_discipline(m, t))
                }
        })
    })
}
