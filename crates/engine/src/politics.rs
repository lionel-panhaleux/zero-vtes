//! Default politics for the trained network's seats (owner, 2026-10-04:
//! "the bot is too bad at setting the terms"; DESIGN §11.2): some terms by
//! rule (`terms_choice`), the rest and the votes bounded (`mask`). Temporary
//! guidance, to be relaxed later in training. Agent behaviour, not an
//! engine mask (rules-qa 17.4 untouched); reads only public state (the
//! terms are announced).

use crate::bots::does;
use crate::cards::{CardFilter, RefEffect, Term};
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::Engine;
use crate::ids::{MinionId, SeatId};
use crate::referendum::RefStep;
use crate::state::CardId;

/// How much the caller wants a Methuselah to lose pool: the caller never,
/// the prey most, then the predator, then the others. Gains are the
/// negation.
fn harm(e: &Engine, me: SeatId, s: SeatId) -> i32 {
    let st = &e.state;
    if s == me {
        -10
    } else if s == st.prey(me) {
        3
    } else if s == st.predator(me) {
        2
    } else {
        1
    }
}

/// Whether allocated points burn (true) or give (false) pool, for the
/// referendums whose points this rule allocates.
fn points_burn(effect: RefEffect) -> Option<bool> {
    match effect {
        RefEffect::BurnAllocated | RefEffect::BurnAllocatedChosenGains(_) => Some(true),
        RefEffect::MoveAllocatedFromChosen => Some(false),
        _ => None,
    }
}

/// Vampires of `clan` a Methuselah controls (Consanguineous Boon's gains).
fn clan_count(e: &Engine, seat: SeatId, clan: &str) -> i32 {
    e.state
        .minions
        .iter()
        .filter(|m| m.present() && m.owner == seat && m.clan() == Some(clan))
        .count() as i32
}

fn owner(e: &Engine, m: MinionId) -> SeatId {
    e.state.minions[m.index()].owner
}

/// The caller's terms choice, by rule, for the terms this rule sets;
/// `None` elsewhere (the network chooses, within `mask`).
pub fn terms_choice(e: &Engine, req: &DecisionRequest) -> Option<usize> {
    if req.window != WindowLabel::ReferendumTerms {
        return None;
    }
    let r = e.referendum()?;
    let def = r.def?;
    let me = req.seat;
    // Ancient Influence, Reins of Power: one's biggest ready vampire (one's
    // own gain, or the prey's burn).
    if r.step == RefStep::Choose {
        return biggest_vampire(e, req);
    }
    let w = |s: SeatId| -> Option<i32> {
        Some(if points_burn(def.effect)? {
            harm(e, me, s)
        } else {
            -harm(e, me, s)
        })
    };
    let score = |o: &Choice| -> Option<i32> {
        match *o {
            // Points not given to this seat go to the best of those after
            // it: the remainder is the same for every option, so
            // n × (w − best later).
            Choice::AllocateTo { seat, n } => {
                let mut later: Option<i32> = None;
                for &s in r.terms.pending.iter().skip(1) {
                    let x = w(s)?;
                    later = Some(later.map_or(x, |l| l.max(x)));
                }
                Some(n as i32 * (w(seat)? - later.unwrap_or(0)))
            }
            Choice::ChooseSeat(s) => match (r.current_term()?, def.effect) {
                // The chosen Methuselah gains (Camarilla's Iron Fist) or
                // takes the location (Disputed Territory).
                (
                    Term::ChooseSeat(_),
                    RefEffect::BurnAllocatedChosenGains(_) | RefEffect::ChosenTakesChosenCard,
                ) => Some(-harm(e, me, s)),
                // The chosen Methuselah loses the pool (Parity Shift).
                (Term::ChooseSeat(_), RefEffect::MoveAllocatedFromChosen) => Some(harm(e, me, s)),
                _ => None,
            },
            // A title: one of the caller's own vampires, the biggest first.
            Choice::ChooseMinion(m) if def.effect == RefEffect::PutOnChosen => {
                Some(if owner(e, m) == me {
                    1000 + e.state.capacity_of(m).unwrap_or(0) as i32
                } else {
                    -100 * harm(e, me, owner(e, m))
                })
            }
            Choice::Pass if def.effect == RefEffect::PutOnChosen => Some(0),
            // The clan with the most of the caller's vampires, the fewest
            // of the others'.
            Choice::ChooseClan(c) if def.effect == RefEffect::GainPerClanVampire => {
                let others: i32 = e
                    .state
                    .clockwise_from(me)
                    .into_iter()
                    .filter(|&s| s != me)
                    .map(|s| clan_count(e, s, c))
                    .sum();
                Some(10 * clan_count(e, me, c) - others)
            }
            _ => None,
        }
    };
    let scores: Vec<i32> = req.options.iter().map(score).collect::<Option<Vec<_>>>()?;
    // The first best: among equals, the order the engine offers.
    let best = *scores.iter().max()?;
    scores.iter().position(|&s| s == best)
}

/// The option choosing the biggest vampire (capacity; the first of
/// equals), if any.
pub(crate) fn biggest_vampire(e: &Engine, req: &DecisionRequest) -> Option<usize> {
    let cap = |o: &Choice| match *o {
        Choice::ChooseMinion(m) => e.state.capacity_of(m).map(i32::from),
        _ => None,
    };
    let best = req.options.iter().filter_map(cap).max()?;
    req.options.iter().position(|o| cap(o) == Some(best))
}

/// The biggest capacity among the ready vampires `s` controls (0: none).
fn biggest_ready(e: &Engine, s: SeatId) -> i32 {
    let st = &e.state;
    st.minions_of(s)
        .filter(|&m| st.minions[m.index()].is_vampire() && st.minions[m.index()].ready())
        .filter_map(|m| st.capacity_of(m))
        .max()
        .map_or(0, i32::from)
}

/// The terms a network caller may choose where the rule bounds them
/// without choosing (`None` elsewhere).
fn terms_mask(e: &Engine, req: &DecisionRequest) -> Option<Vec<bool>> {
    if req.window != WindowLabel::ReferendumTerms {
        return None;
    }
    let r = e.referendum()?;
    let def = r.def?;
    let st = &e.state;
    let me = req.seat;
    let neighbour = |s: SeatId| s != me && (s == st.prey(me) || s == st.predator(me));
    // The fewest points the engine offers this recipient (what it must
    // take).
    let least = req
        .options
        .iter()
        .filter_map(|o| match *o {
            Choice::AllocateTo { n, .. } | Choice::AllocateToCard { n, .. } => Some(n),
            _ => None,
        })
        .min()
        .unwrap_or(0);
    let mask: Vec<bool> = match def.effect {
        // A neighbour's minion, else another opponent's, never one's own
        // unless nothing else is offered (the fallback below).
        RefEffect::Banish | RefEffect::Expel => {
            let near = req.options.iter().any(|o| match *o {
                Choice::ChooseMinion(m) => neighbour(owner(e, m)),
                _ => false,
            });
            req.options
                .iter()
                .map(|o| match *o {
                    Choice::ChooseMinion(m) => {
                        let s = owner(e, m);
                        s != me && (neighbour(s) || !near)
                    }
                    _ => true,
                })
                .collect()
        }
        // A location among the others'.
        RefEffect::ChosenTakesChosenCard => req
            .options
            .iter()
            .map(|o| match *o {
                Choice::ChooseCard(c) => st.card_controller(CardId(c)) != me,
                _ => true,
            })
            .collect(),
        // Pool burned on the prey; the others' locations burned, one point
        // each; nothing on oneself or one's cards, nor elsewhere, beyond
        // what the engine forces.
        RefEffect::LockChosenBurnAllocated => req
            .options
            .iter()
            .map(|o| match *o {
                Choice::AllocateTo { seat, n } => n == least || seat == st.prey(me),
                Choice::AllocateToCard { card, n } => {
                    let theirs = st.card_controller(CardId(card)) != me;
                    let location = CardFilter::Location.matches(st.cards[card as usize].card);
                    n == least || (theirs && location && n <= 1)
                }
                _ => true,
            })
            .collect(),
        _ => return None,
    };
    mask.iter().any(|&m| m).then_some(mask)
}

/// What the referendum's terms do to `me`, if this rule reads its effect:
/// negative a loss (pool burned or taken, a minion banished or expelled, a
/// location lost or burned, an opponent's vampire titled), positive a
/// gain, 0 untouched.
fn stake(e: &Engine, me: SeatId) -> Option<i32> {
    let r = e.referendum()?;
    let def = r.def?;
    let st = &e.state;
    let alloc: i32 = r
        .terms
        .alloc
        .iter()
        .filter(|(s, _)| *s == me)
        .map(|(_, n)| *n as i32)
        .sum();
    let total: i32 = r.terms.alloc.iter().map(|(_, n)| *n as i32).sum();
    let chosen = r.terms.chosen == Some(me);
    let mine = r
        .terms
        .minions
        .iter()
        .filter(|&&m| owner(e, m) == me)
        .count() as i32;
    match def.effect {
        RefEffect::BurnAllocated => Some(-alloc),
        RefEffect::BurnAllocatedChosenGains(n) => Some(-alloc + if chosen { n as i32 } else { 0 }),
        RefEffect::MoveAllocatedFromChosen => Some(alloc - if chosen { total } else { 0 }),
        RefEffect::Banish | RefEffect::Expel | RefEffect::RemoveChosen => Some(-mine),
        // Each one's biggest ready vampire chosen (the default politics).
        RefEffect::GainChosenCapacityBurn(n) => Some(biggest_ready(e, me) - n as i32),
        RefEffect::GainBurnPredatorsChosenCapacity(n) => {
            Some(n as i32 - biggest_ready(e, st.predator(me)))
        }
        RefEffect::GainPerVampireCapacityAtLeast { cap, n } => Some(
            n as i32
                * st.minions_of(me)
                    .filter(|&m| {
                        st.minions[m.index()].is_vampire()
                            && st.capacity_of(m).is_some_and(|c| c >= cap)
                    })
                    .count() as i32,
        ),
        RefEffect::GainPerMinion(n) => Some(n as i32 * st.minions_of(me).count() as i32),
        // Kindred Segregation: one's allies, burned or paid for.
        RefEffect::BurnAlliesUnlessKept => Some(
            -(st.minions_of(me)
                .filter(|&m| st.minions[m.index()].is_ally())
                .count() as i32),
        ),
        // Every Methuselah opposes an opponent's title (owner, 2026-10-04).
        RefEffect::PutOnChosen if !r.terms.minions.is_empty() => {
            Some(if mine > 0 { 1 } else { -1 })
        }
        RefEffect::GainPerClanVampire => Some(clan_count(e, me, r.terms.clan?)),
        RefEffect::ChosenTakesChosenCard => {
            let lost = r.terms.cards.iter().any(|&c| st.card_controller(c) == me);
            Some(match (chosen, lost) {
                (true, false) => 1,
                (false, true) => -1,
                _ => 0,
            })
        }
        // Points on one's pool or cards (one's Anarchs locked: the caller's
        // cost, and the caller votes for).
        RefEffect::LockChosenBurnAllocated => {
            let cards = r
                .terms
                .card_alloc
                .iter()
                .filter(|&&(c, n)| n > 0 && st.card_controller(c) == me)
                .count() as i32;
            Some(-alloc - cards)
        }
        _ => None,
    }
}

/// The options a network seat may take while votes are cast: `None` when
/// the rule has no opinion (a referendum it does not read, a seat the
/// terms leave alone).
fn vote_mask(e: &Engine, req: &DecisionRequest) -> Option<Vec<bool>> {
    if req.window != WindowLabel::ReferendumPolling {
        return None;
    }
    let r = e.referendum()?;
    let me = req.seat;
    let favor = if r.caller == me {
        true
    } else {
        match stake(e, me)? {
            0 => return None,
            s => s > 0,
        }
    };
    let free = |o: &Choice| matches!(*o, Choice::CastVotes { favor: f, .. } | Choice::CastCardVote { favor: f } if f == favor);
    let has_free = req.options.iter().any(free);
    let mask: Vec<bool> = req
        .options
        .iter()
        .map(|o| match *o {
            Choice::CastVotes { favor: f, .. }
            | Choice::CastCardVote { favor: f }
            | Choice::CastEdge { favor: f }
            | Choice::BurnForVote { favor: f, .. } => f == favor,
            Choice::Play { card, mode, .. } => {
                let d = does(e.playable_card(me, card), mode);
                // A card adding votes only on the wrong side.
                !(if favor {
                    d.votes_against && !d.votes_for
                } else {
                    d.votes_for && !d.votes_against
                })
            }
            Choice::Pass => !has_free,
            _ => true,
        })
        .collect();
    mask.iter().any(|&m| m).then_some(mask)
}

/// The options a network seat may take in a referendum's terms or votes:
/// `None` when the rule has no opinion (any other decision).
pub fn mask(e: &Engine, req: &DecisionRequest) -> Option<Vec<bool>> {
    terms_mask(e, req).or_else(|| vote_mask(e, req))
}
