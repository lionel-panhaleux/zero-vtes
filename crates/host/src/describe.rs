//! Options as data (docs/protocol.md §7): each engine `Choice` as its
//! `ChoiceKey` — what identifies it among the decision's options, by
//! card ids and copies, never by a position in a hand or a region.
//!
//! One derivation, two namings: a viewer's wire copies (the options a
//! seat is shown) or the engine's own (the record's keys).

use zvtes::cards::{AbilitySource, ModeDef, Target as T};
use zvtes::copies::CopyId;
use zvtes::crypt::CRYPT;
use zvtes::decision::{Choice, DecisionRequest, Zone as Z};
use zvtes::draws::Interrupt;
use zvtes::game::{Contest, Engine};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::state::{CardId, MinionKind, UncontrolledSlot, CREATED};
use zvtes_api::ids::{DueKind, OptionKind, Role, Zone};
use zvtes_api::message::{About, Due};
use zvtes_api::option::{CardRef, ChoiceKey, Effect, EffectKind, On, User, ZoneRef};
use zvtes_api::view::Target;

use crate::effects::{curated, Whom};
use crate::view::{action_kind, minion_copy, mode, snake};

/// How copies are named: a viewer's numbers, or the engine's.
pub trait Names {
    fn copy(&self, c: CopyId) -> Option<u32>;
}

/// The engine's own numbers (records).
pub struct EngineNames;

impl Names for EngineNames {
    fn copy(&self, c: CopyId) -> Option<u32> {
        (c != CopyId::NONE).then_some(c.0 as u32)
    }
}

impl Names for crate::numbering::Numbering {
    fn copy(&self, c: CopyId) -> Option<u32> {
        self.get(c).map(|(w, _)| w)
    }
}

/// Copies not named (out of the namer's sight): a sentinel, never a
/// number given.
const UNSEEN: u32 = u32::MAX;

struct Ctx<'a, N: Names> {
    e: &'a Engine,
    seat: SeatId,
    names: &'a N,
}

impl<N: Names> Ctx<'_, N> {
    fn minion(&self, m: MinionId) -> u32 {
        self.names
            .copy(minion_copy(&self.e.state, m.index()))
            .unwrap_or(UNSEEN)
    }

    fn card(&self, c: CardId) -> u32 {
        self.names
            .copy(*self.e.state.cards[c.index()].copy)
            .unwrap_or(UNSEEN)
    }

    /// A card in play: its KRCG id and copy.
    fn in_play(&self, c: CardId) -> CardRef {
        CardRef {
            id: Some(LIBRARY[self.e.state.cards[c.index()].card as usize].id),
            copy: Some(self.card(c)),
            from: None,
        }
    }

    /// The region an uncontrolled-slot index points into: the
    /// interrupt's seat's (a vampire of another Methuselah named), else
    /// the deciding seat's.
    fn region(&self) -> SeatId {
        match self.e.interrupt() {
            Some(
                Interrupt::NameUncontrolled { seat, .. }
                | Interrupt::UncontrolledBlood { seat, .. },
            ) => seat,
            Some(
                Interrupt::PreySlotToPool { of, .. }
                | Interrupt::NameSlotPair { of, from: None, .. },
            ) => of,
            _ => self.seat,
        }
    }

    /// An uncontrolled vampire, by its slot: its card and copy.
    fn slot(&self, i: u8) -> CardRef {
        let region = &self.e.state.seats[self.region().index()].uncontrolled;
        let u: &UncontrolledSlot = &region[i as usize];
        CardRef {
            id: Some(slot_id(self.e, u)),
            copy: Some(self.names.copy(*u.copy).unwrap_or(UNSEEN)),
            from: None,
        }
    }

    fn target(&self, t: T) -> Option<Target> {
        Some(match t {
            T::None => return None,
            T::Seat(s) => Target::Seat { seat: s.0 },
            T::Minion(m) => Target::Card {
                copy: self.minion(m),
            },
            T::Card(c) => Target::Card { copy: self.card(c) },
            T::Uncontrolled(i) => Target::Card {
                copy: self.slot(i).copy.unwrap_or(UNSEEN),
            },
        })
    }

    /// A card in the deciding seat's hand (copies alike are one option):
    /// its KRCG id, and the card it is stored on when played from there.
    fn hand(&self, i: u8) -> CardRef {
        let (c, stored) = self.e.virtual_hand(self.seat)[i as usize];
        CardRef {
            id: Some(LIBRARY[c as usize].id),
            copy: None,
            from: stored.map(|s| self.card(s)),
        }
    }
}

/// A vampire's card in a slot: its crypt card, or a banished created
/// vampire's library card.
fn slot_id(e: &Engine, u: &UncontrolledSlot) -> u32 {
    if u.card != CREATED {
        return CRYPT[u.card as usize].id;
    }
    u.banished
        .as_ref()
        .and_then(|b| match e.state.minions[b.minion.index()].kind {
            MinionKind::Created { card, .. } => Some(LIBRARY[card as usize].id),
            _ => None,
        })
        .unwrap_or(0)
}

fn library(c: u16) -> CardRef {
    CardRef {
        id: Some(LIBRARY[c as usize].id),
        copy: None,
        from: None,
    }
}

/// What taking `choice` does (docs/protocol.md §7): an ability's
/// effects, a play's mode's, each naming whom it applies to by copy; a
/// play choosing a second vampire after its target says its role.
pub fn effects(
    e: &Engine,
    req: &DecisionRequest,
    choice: &Choice,
    names: &impl Names,
) -> Vec<Effect> {
    let x = Ctx {
        e,
        seat: req.seat,
        names,
    };
    let st = &e.state;
    match *choice {
        Choice::Play {
            card,
            mode,
            actor,
            target,
        } => {
            let played = e.virtual_hand(req.seat)[card as usize].0;
            let Some(m) = zvtes::cards::lookup(played).and_then(|d| d.modes.get(mode as usize))
            else {
                return Vec::new();
            };
            let whom = Whom {
                target: x.target(target).and_then(|t| match t {
                    Target::Card { copy } => Some(copy),
                    Target::Seat { .. } => None,
                }),
                actor: actor.map(|a| x.minion(a)),
                ..Whom::default()
            };
            let mut out = curated(m.effects, whom);
            out.extend(second_chosen(m));
            out
        }
        // The second vampire a play chooses (Toreador Grand Ball's).
        Choice::ChooseMinion(_) if req.window == zvtes::decision::WindowLabel::PlayTerms => e
            .play_terms()
            .and_then(|p| zvtes::cards::lookup(p.card)?.modes.get(p.mode as usize))
            .and_then(second_chosen)
            .map(|r| Effect {
                on: Some(On::Chosen),
                ..r
            })
            .into_iter()
            .chain(cost_payer(e, req))
            .collect(),
        Choice::UseAbility {
            source,
            ability,
            actor,
            target,
        } => {
            let (abilities, host, card) = match source {
                AbilitySource::Card(c) => {
                    let k = &st.cards[c.index()];
                    let abilities = zvtes::cards::lookup(k.card).map_or(&[][..], |d| d.abilities);
                    (abilities, k.host.map(|m| x.minion(m)), Some(x.card(c)))
                }
                AbilitySource::Vampire(m) => {
                    let v = &st.minions[m.index()];
                    let abilities = zvtes::cards::vampire_of(v).map_or(&[][..], |d| d.abilities);
                    (abilities, Some(x.minion(m)), None)
                }
            };
            let Some(a) = abilities.get(ability as usize) else {
                return Vec::new();
            };
            let whom = Whom {
                target: x.target(target).and_then(|t| match t {
                    Target::Card { copy } => Some(copy),
                    Target::Seat { .. } => None,
                }),
                host,
                actor: actor.map(|a| x.minion(a)),
                card,
            };
            curated(a.effects, whom)
        }
        Choice::SetX(n) => amount(&x, req, n),
        // The strike a card granted for the round (Hunger of Marduk).
        Choice::RoundStrike { at: None } => match e.combat().map(|c| (c, c.step)) {
            Some((c, zvtes::combat::Step::ChooseStrike(i))) => c.sides[i]
                .round_strike
                .map(|k| Effect {
                    strike: Some(crate::view::strike(k, None)),
                    ..Effect::new(EffectKind::Strike)
                })
                .into_iter()
                .collect(),
            _ => Vec::new(),
        },
        // The votes a minion casts, with its prisci ballots (rules-qa 52.9).
        Choice::CastVotes { minion, favor } => {
            let mut out: Vec<Effect> = e
                .referendum()
                .map(|r| Effect {
                    n: Some(r.votes_cast(st, minion, favor) as i32),
                    favor: Some(favor),
                    ..Effect::new(EffectKind::Votes)
                })
                .into_iter()
                .collect();
            if let b @ 1.. = st.ballots(minion) {
                out.push(Effect {
                    n: Some(b as i32),
                    favor: Some(favor),
                    detail: Some("prisci_ballot".into()),
                    ..Effect::new(EffectKind::Votes)
                });
            }
            out
        }
        // The political action card's vote; Business Pressure's round:
        // the pool burned times the rate.
        Choice::CastCardVote { favor } => {
            let (n, on) = match e.referendum().and_then(|r| r.pressure.as_ref()) {
                Some(p) => ((p.amount.unwrap_or(0) * p.rate(req.seat)) as i32, None),
                None => (1, Some(On::Card)),
            };
            vec![Effect {
                n: Some(n),
                on,
                favor: Some(favor),
                ..Effect::new(EffectKind::Votes)
            }]
        }
        // A cost's terms: a card's counters, a vampire's own (Nergal, Jack)
        // reducing it.
        Choice::ResolveRider { .. } | Choice::ResolvePrinted { .. }
            if req.window == zvtes::decision::WindowLabel::PlayTerms
                && e.cost_terms().is_some() =>
        {
            vec![Effect::new(EffectKind::Cost)]
        }
        _ if req.window == zvtes::decision::WindowLabel::ReferendumTerms => {
            referendum_terms(&x, choice)
        }
        _ => e
            .interrupt()
            .map_or_else(Vec::new, |i| asked(&x, i, choice)),
    }
}

/// What choosing does in a referendum's terms, when they choose several
/// things of a kind: the pool a Methuselah burns, gains or gives, what a
/// chosen minion or card gets.
fn referendum_terms(x: &Ctx<impl Names>, choice: &Choice) -> Vec<Effect> {
    use zvtes::cards::{RefEffect as R, Term};
    let Some(r) = x.e.referendum() else {
        return Vec::new();
    };
    let Some(def) = r.def else {
        return Vec::new();
    };
    let Some(&term) = def.terms.get(r.terms.term).or(def.terms.last()) else {
        return Vec::new();
    };
    let chosen = |kind| Effect {
        on: Some(On::Chosen),
        ..Effect::new(kind)
    };
    let n = |n: i32, e: Effect| Effect { n: Some(n), ..e };
    match (*choice, term, def.effect) {
        (Choice::ChooseSeat(_), Term::ChooseSeat(_), R::BurnAllocatedChosenGains(g)) => {
            vec![n(g as i32, chosen(EffectKind::Pool))]
        }
        // The chosen gives the pool allocated to the others.
        (Choice::ChooseSeat(_), Term::ChooseSeat(_), R::MoveAllocatedFromChosen) => {
            vec![Effect {
                detail: Some("allocated".into()),
                ..chosen(EffectKind::Pool)
            }]
        }
        (Choice::ChooseSeat(_), Term::ChooseSeat(_), R::ChosenTakesChosenCard) => {
            vec![chosen(EffectKind::Steal)]
        }
        (Choice::ChooseCard(_), _, R::ChosenTakesChosenCard) => vec![Effect {
            on: Some(On::Card),
            ..Effect::new(EffectKind::Steal)
        }],
        (Choice::ChooseSeat(_), Term::ChooseSeats { .. }, R::BurnChosen { base, .. }) => {
            vec![n(-(base as i32), chosen(EffectKind::Pool))]
        }
        (
            Choice::AllocateTo { n: a, .. },
            _,
            R::BurnAllocated | R::BurnAllocatedChosenGains(_) | R::LockChosenBurnAllocated,
        ) => vec![n(-(a as i32), Effect::new(EffectKind::Pool))],
        // Pool moved from the chosen Methuselah to the one allocated to.
        (Choice::AllocateTo { n: a, .. }, _, R::MoveAllocatedFromChosen) => vec![
            n(a as i32, Effect::new(EffectKind::Pool)),
            Effect {
                seat: r.terms.chosen.map(|s| s.0),
                ..n(-(a as i32), chosen(EffectKind::Pool))
            },
        ],
        (Choice::AllocateToCard { n: a, .. }, _, R::LockChosenBurnAllocated) if a > 0 => {
            vec![Effect {
                on: Some(On::Target),
                ..Effect::new(EffectKind::Burn)
            }]
        }
        (Choice::ChooseMinion(_), _, R::Banish) => vec![Effect {
            to: Some(Zone::Removed),
            ..chosen(EffectKind::Move)
        }],
        (Choice::ChooseMinion(_), _, R::Expel) => vec![Effect {
            role: Some(Role::Expelled),
            ..chosen(EffectKind::Role)
        }],
        // The political action card put on the chosen.
        (Choice::ChooseMinion(_), _, R::PutOnChosen { .. }) => vec![Effect {
            card: x
                .e
                .action()
                .and_then(|a| a.card.as_ref())
                .map(|p| LIBRARY[p.card as usize].id),
            ..chosen(EffectKind::Move)
        }],
        (Choice::ChooseMinion(_), _, R::LockChosenBurnAllocated) => {
            vec![chosen(EffectKind::Lock)]
        }
        _ => Vec::new(),
    }
}

/// The second vampire a play chooses, of its clan (Toreador Grand Ball):
/// what it becomes.
fn second_chosen(m: &ModeDef) -> Option<Effect> {
    use zvtes::cards::{Effect as E, Modifier, Terms};
    let Some(Terms::OwnOfClan { clan, .. }) = m.terms else {
        return None;
    };
    let unblockable = m
        .effects
        .contains(&E::Layer(Modifier::SecondChosenUnblockable));
    Some(Effect {
        role: Some(if unblockable {
            Role::UnblockableNonBleed
        } else {
            Role::Chosen
        }),
        clan: Some(snake(clan)),
        ..Effect::new(EffectKind::Role)
    })
}

/// A cost's terms: a vampire paying 1 of the cost in blood.
fn cost_payer(e: &Engine, req: &DecisionRequest) -> Option<Effect> {
    (req.window == zvtes::decision::WindowLabel::PlayTerms && e.cost_terms().is_some()).then(|| {
        Effect {
            n: Some(-1),
            on: Some(On::Chosen),
            cost: true,
            ..Effect::new(EffectKind::Blood)
        }
    })
}

/// What an option does for the choice an effect asks (`Decision::purpose`),
/// where the purpose alone does not say: what the chosen minion gets, where
/// a card goes, what a pass keeps or gains.
fn asked(x: &Ctx<impl Names>, i: Interrupt, choice: &Choice) -> Vec<Effect> {
    use zvtes::draws::MinionPurpose as P;
    let chosen = |kind| Effect {
        on: Some(On::Chosen),
        ..Effect::new(kind)
    };
    let n = |n: i32, e: Effect| Effect { n: Some(n), ..e };
    let library = |c: u16| Some(LIBRARY[c as usize].id);
    match (i, *choice) {
        (Interrupt::MinionOf { purpose, .. }, Choice::ChooseMinion(_)) => vec![match purpose {
            P::Lock | P::LockVampire => chosen(EffectKind::Lock),
            P::LockOldest(_) => Effect {
                detail: Some("oldest".into()),
                ..chosen(EffectKind::Lock)
            },
            P::Corrupt => n(1, chosen(EffectKind::Corruption)),
            P::UnlockAnarch => chosen(EffectKind::Unlock),
            P::BurnBloodOfClan(_) => n(-1, chosen(EffectKind::Blood)),
            P::HostCard(card) => Effect {
                copy: Some(x.card(card)),
                card: Some(LIBRARY[x.e.state.cards[card.index()].card as usize].id),
                ..chosen(EffectKind::Move)
            },
            P::BurnOfCrowdedClan => chosen(EffectKind::Burn),
            P::OldestHuntingGround(_) => n(1, chosen(EffectKind::Blood)),
            P::Damage(d) => n(d as i32, chosen(EffectKind::Damage)),
        }],
        // Ashur Tablets: to the hand, then into the library.
        (Interrupt::AshurPicks { to_library, .. }, Choice::FindCard(c)) => vec![Effect {
            card: library(c),
            to: Some(if to_library {
                Zone::Library
            } else {
                Zone::Hand
            }),
            ..Effect::new(EffectKind::Move)
        }],
        (Interrupt::ExchangeAnarch { ash: Some(c), .. }, Choice::Discard(_)) => vec![Effect {
            card: library(c),
            to: Some(Zone::Hand),
            ..Effect::new(EffectKind::Move)
        }],
        (Interrupt::BloodOrBurned { minion, n: b, .. }, Choice::BurnBloodToAvoid) => {
            vec![Effect {
                copy: Some(x.minion(minion)),
                cost: true,
                ..n(-(b as i32), Effect::new(EffectKind::Blood))
            }]
        }
        (Interrupt::BloodOrBurned { minion, card, .. }, Choice::Pass) => vec![match card {
            Some(c) => Effect {
                copy: Some(x.card(c)),
                card: Some(LIBRARY[x.e.state.cards[c.index()].card as usize].id),
                ..Effect::new(EffectKind::Burn)
            },
            None => Effect {
                copy: Some(x.minion(minion)),
                ..Effect::new(EffectKind::Burn)
            },
        }],
        // What the card does with its chosen vampire (Toreador Grand
        // Ball's two, Priority Contract's).
        (
            Interrupt::ChooseForCard {
                card, clan, second, ..
            },
            Choice::ChooseMinion(_),
        ) => {
            use zvtes::cards::{Effect as E, Modifier};
            let effects = zvtes::cards::lookup(x.e.state.cards[card.index()].card)
                .and_then(|d| d.modes.first())
                .map_or(&[][..], |m| m.effects);
            let has = |m: Modifier| effects.contains(&E::Layer(m));
            let role = if second && has(Modifier::SecondChosenUnblockable) {
                Role::UnblockableNonBleed
            } else if !second && has(Modifier::FirstChosenDoesNotUnlock) {
                Role::DoesNotUnlock
            } else {
                Role::Chosen
            };
            vec![Effect {
                role: Some(role),
                clan: clan.map(snake),
                ..chosen(EffectKind::Role)
            }]
        }
        // A card burned unless its pool is paid.
        (Interrupt::PayOrBurn { card, .. }, Choice::Pass) => vec![Effect {
            copy: Some(x.card(card)),
            ..Effect::new(EffectKind::Burn)
        }],
        (Interrupt::PayOrBurn { card, pool, .. }, Choice::PayToCancel) => vec![
            Effect {
                cost: true,
                ..n(-(pool as i32), Effect::new(EffectKind::Pool))
            },
            Effect {
                copy: Some(x.card(card)),
                ..Effect::new(EffectKind::Keep)
            },
        ],
        (Interrupt::DistributeFrom { .. }, Choice::Slot(_)) => {
            vec![n(1, chosen(EffectKind::Blood))]
        }
        // Gilbert Duane: the chosen becomes Anarch; once chosen, a pass
        // gains 1 pool.
        (Interrupt::AnarchConvert { chosen: None, .. }, Choice::ChooseMinion(_)) => {
            vec![Effect {
                role: Some(Role::Anarch),
                ..chosen(EffectKind::Role)
            }]
        }
        (
            Interrupt::AnarchConvert {
                chosen: Some(_), ..
            },
            Choice::Pass,
        ) => {
            vec![n(1, Effect::new(EffectKind::Pool))]
        }
        _ => Vec::new(),
    }
}

/// What an amount chosen does (`set_x`), by the terms asking it: blood
/// paid or moved, pool, counters, cards; `x` where the card calls it X.
fn amount(x: &Ctx<impl Names>, req: &DecisionRequest, n: u8) -> Vec<Effect> {
    use zvtes::cards::{Effect as E, Terms};
    use zvtes::decision::WindowLabel as W;
    let e = x.e;
    let n = n as i32;
    let of = |kind, n| Effect {
        n: Some(n),
        ..Effect::new(kind)
    };
    let per_x = |kind, n| Effect {
        x: true,
        ..of(kind, n)
    };
    match req.window {
        W::PlayTerms => {
            if let Some(p) = e.play_terms() {
                let Some(terms) = zvtes::cards::lookup(p.card)
                    .and_then(|d| d.modes.get(p.mode as usize))
                    .and_then(|m| m.terms)
                else {
                    return Vec::new();
                };
                return match terms {
                    Terms::BloodRange { .. } | Terms::EquipFromHand { .. } => {
                        vec![of(EffectKind::Blood, -n)]
                    }
                    // "Any amount of blood" of the target vampire.
                    Terms::UpToTargetBlood => vec![Effect {
                        on: Some(On::Target),
                        copy: Some(x.minion(p.target.minion())),
                        ..of(EffectKind::Blood, -n)
                    }],
                    Terms::XBlood | Terms::XBloodIn { .. } => vec![per_x(EffectKind::Blood, -1)],
                    // The referendum's margin, as blood to the pool.
                    Terms::UpToMargin(_) => vec![Effect {
                        detail: Some("from_blood".into()),
                        ..of(EffectKind::Pool, n)
                    }],
                    Terms::BoonPool => vec![Effect {
                        on: Some(On::Target),
                        ..of(EffectKind::Pool, -n)
                    }],
                    _ => Vec::new(),
                };
            }
            // A cost reduced by burning counters.
            if e.cost_terms().is_some() {
                return vec![Effect {
                    cost: true,
                    ..of(EffectKind::Counters, -n)
                }];
            }
            let Some((_, mode, _)) = e.ability_terms() else {
                return Vec::new();
            };
            match mode.terms {
                Some(Terms::Count { .. }) if mode.effects.contains(&E::DrawThenBottom) => {
                    vec![Effect {
                        detail: Some("then_bottom".into()),
                        ..of(EffectKind::Draw, n)
                    }]
                }
                // Jake Washington: blood put on the target.
                Some(Terms::Count { .. }) => vec![Effect {
                    on: Some(On::Target),
                    ..of(EffectKind::Blood, n)
                }],
                Some(Terms::XPool) => vec![per_x(EffectKind::Pool, -1)],
                Some(Terms::XCounters) => vec![per_x(EffectKind::Counters, 1)],
                // The Vozhd: blood from the target to the host.
                Some(Terms::LifeFromTarget) => vec![Effect {
                    on: Some(On::Host),
                    ..of(EffectKind::Blood, n)
                }],
                _ => Vec::new(),
            }
        }
        // Smiling Jack: cards discarded, the rest of the units in pool.
        W::RiderTerms => {
            let left = e.rider_terms().map_or(0, |(_, l)| l as i32);
            vec![
                of(EffectKind::Discard, n),
                Effect {
                    cost: true,
                    ..of(EffectKind::Pool, -(left - n))
                },
            ]
        }
        // Business Pressure: pool burned for votes.
        W::ReferendumPolling => match e.referendum().and_then(|r| r.pressure.as_ref()) {
            Some(p) => vec![
                Effect {
                    cost: true,
                    ..of(EffectKind::Pool, -n)
                },
                of(EffectKind::Votes, n * p.rate(x.seat) as i32),
            ],
            None => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// An ability's source's abilities, and which of them it is.
fn ability_defs(e: &Engine, choice: &Choice) -> Option<(&'static [ModeDef], usize)> {
    let Choice::UseAbility {
        source, ability, ..
    } = *choice
    else {
        return None;
    };
    let st = &e.state;
    let defs = match source {
        AbilitySource::Card(c) => {
            zvtes::cards::lookup(st.cards[c.index()].card).map_or(&[][..], |d| d.abilities)
        }
        AbilitySource::Vampire(m) => {
            zvtes::cards::vampire_of(&st.minions[m.index()]).map_or(&[][..], |d| d.abilities)
        }
    };
    Some((defs, ability as usize))
}

/// An ability's: how many its source has.
pub fn abilities(e: &Engine, choice: &Choice) -> Option<u8> {
    ability_defs(e, choice).map(|(defs, _)| defs.len() as u8)
}

/// An ability's: what sets it apart from its source's other abilities,
/// offered or not (`OptionView::apart`).
pub fn ability_apart(e: &Engine, choice: &Choice) -> Vec<Effect> {
    let Some((defs, i)) = ability_defs(e, choice) else {
        return Vec::new();
    };
    let Some(own) = defs.get(i) else {
        return Vec::new();
    };
    let siblings = defs.iter().enumerate().filter(|&(j, _)| j != i);
    crate::view::apart_from(own, siblings.map(|(_, d)| d))
}

/// What a minion phase waits for before it may end (`Decision::due`).
pub fn dues(e: &Engine, req: &DecisionRequest, names: &impl Names) -> Vec<Due> {
    use zvtes::actions::Due as D;
    let x = Ctx {
        e,
        seat: req.seat,
        names,
    };
    e.dues(req)
        .into_iter()
        .map(|d| match d {
            D::Hunt(m) => Due {
                kind: DueKind::Hunt,
                copy: x.minion(m),
            },
            D::Action(m) => Due {
                kind: DueKind::Action,
                copy: x.minion(m),
            },
            D::TakeControl(c) => Due {
                kind: DueKind::TakeControl,
                copy: x.card(c),
            },
        })
        .collect()
}

/// An ability's: who uses it (`OptionView::user`).
pub fn ability_user(
    e: &Engine,
    req: &DecisionRequest,
    choice: &Choice,
    names: &impl Names,
) -> Option<User> {
    let (seat, minion) = e.ability_used_by(req.seat, choice)?;
    let x = Ctx {
        e,
        seat: req.seat,
        names,
    };
    Some(match minion {
        Some(m) => User::Minion { copy: x.minion(m) },
        None => User::Methuselah { seat: seat.0 },
    })
}

/// The card whose terms a terms decision chooses (`Decision::source`):
/// the action's card or what grants it, the card being played.
pub fn source(e: &Engine, req: &DecisionRequest, names: &impl Names) -> Option<CardRef> {
    use zvtes::decision::WindowLabel as W;
    let x = Ctx {
        e,
        seat: req.seat,
        names,
    };
    let crypt = |m: MinionId| {
        let st = &e.state;
        let (card, crypt) = crate::view::minion_face(&st.minions[m.index()]);
        CardRef {
            id: Some(if crypt {
                CRYPT[card as usize].id
            } else {
                LIBRARY[card as usize].id
            }),
            copy: Some(x.minion(m)),
            from: None,
        }
    };
    match req.window {
        W::ReferendumTerms | W::ActionTerms => {
            let a = e.action()?;
            match (&a.card, a.source) {
                (Some(p), _) => Some(CardRef {
                    id: Some(LIBRARY[p.card as usize].id),
                    copy: names.copy(*p.copy),
                    from: None,
                }),
                (None, Some(AbilitySource::Card(c))) => Some(x.in_play(c)),
                (None, Some(AbilitySource::Vampire(m))) => Some(crypt(m)),
                (None, None) => None,
            }
        }
        // The terms being chosen: a play's, a cost's, an ability's.
        W::PlayTerms if e.play_terms().or(e.cost_terms()).is_some() => {
            let p = e.play_terms().or(e.cost_terms())?;
            Some(CardRef {
                id: Some(LIBRARY[p.card as usize].id),
                copy: names.copy(*p.copy),
                from: None,
            })
        }
        W::PlayTerms if e.ability_terms().is_some() => match e.ability_terms()?.0 {
            AbilitySource::Card(c) => Some(x.in_play(c)),
            AbilitySource::Vampire(m) => Some(crypt(m)),
        },
        W::RiderTerms => e.rider_terms().map(|(c, _)| x.in_play(c)),
        W::PlayTerms => e
            .in_flight()
            .into_iter()
            .rev()
            .find(|p| p.seat == req.seat)
            .map(|p| CardRef {
                id: Some(LIBRARY[p.card as usize].id),
                copy: names.copy(*p.copy),
                from: None,
            }),
        _ => None,
    }
}

/// What a choice is about (`Decision::about`): the particulars its
/// prompt names, by the effect asking it or the terms chosen.
pub fn about(e: &Engine, req: &DecisionRequest, names: &impl Names) -> Option<About> {
    use zvtes::cards::{Effect as E, Modifier, RefEffect as R, Term, Terms};
    use zvtes::decision::WindowLabel as W;
    use zvtes::draws::MinionPurpose as P;
    use Interrupt as I;
    let x = Ctx {
        e,
        seat: req.seat,
        names,
    };
    let st = &e.state;
    let m = |m: MinionId| Some(x.minion(m));
    let card = |c: CardId| Some(x.in_play(c));
    let named = |c: u16| {
        Some(CardRef {
            id: Some(LIBRARY[c as usize].id),
            copy: None,
            from: None,
        })
    };
    let by_name = |name: &str| {
        LIBRARY
            .iter()
            .position(|c| c.name == name)
            .and_then(|i| named(i as u16))
    };
    let detail = |d: &str| Some(d.to_string());
    let a = About::default;
    if let Some(i) = e.interrupt() {
        return Some(match i {
            I::MoveCardToTitled { card: c, .. } => About {
                card: card(c),
                ..a()
            },
            I::MoveOffBurnedBearer { card: c, from, .. } => About {
                card: card(c),
                minion: m(from),
                ..a()
            },
            I::GiveBlood { from, .. } => About {
                minion: m(from),
                ..a()
            },
            I::DiablerieEquipment {
                diablerist, victim, ..
            } => About {
                minion: m(victim),
                to: m(diablerist),
                ..a()
            },
            I::MoveBetween {
                from,
                to,
                blood_done,
                ..
            } => About {
                minion: m(from),
                to: to.and_then(m),
                detail: match (to, blood_done) {
                    (None, _) => None,
                    (Some(_), false) => detail("blood"),
                    (Some(_), true) => detail("cards"),
                },
                ..a()
            },
            I::PayOrBurn { card: c, pool, .. } | I::LockOrPay { card: c, pool, .. } => About {
                card: card(c),
                n: Some(pool),
                ..a()
            },
            I::LibraryToHand { .. } => a(),
            I::BloodOrCamarilla { minion, .. } => About {
                minion: m(minion),
                ..a()
            },
            I::PoolOrNoTransfers { pool, .. } => About {
                n: Some(pool),
                ..a()
            },
            I::PoolOrVampire {
                card: c, pool, cap, ..
            } => About {
                card: card(c),
                n: Some(pool),
                cap: Some(cap),
                ..a()
            },
            I::BloodOrBurned {
                minion, n, card: c, ..
            } => About {
                minion: m(minion),
                n: Some(n),
                card: c.and_then(card),
                ..a()
            },
            I::StoreSearch { card: c, left, .. } => About {
                card: card(c),
                n: Some(left),
                ..a()
            },
            I::LockOrDiscard { minion, .. }
            | I::BlockToll { minion, .. }
            | I::CancelAttempt { minion, .. }
            | I::CorruptionOwner { minion, .. }
            | I::DisciplineSearch { minion, .. }
            | I::AllySearch { minion, .. }
            | I::StealForCorruption { minion, .. }
            | I::ChooseCircle { minion, .. }
            | I::BaseSearch { minion, .. }
            | I::MayTorture { minion, .. }
            | I::EntryBurn { ally: minion, .. }
            | I::EntryRemoveOrBurn { ally: minion, .. } => About {
                minion: m(minion),
                ..a()
            },
            I::Reorder { total, placed, .. } => About {
                n: Some(total),
                m: Some(placed),
                ..a()
            },
            I::MinionOf { of, purpose, .. } => {
                let (d, clan, c, n) = match purpose {
                    P::Lock => ("lock", None, None, None),
                    P::Corrupt => ("corrupt", None, None, None),
                    P::LockVampire => ("lock_vampires", None, None, None),
                    P::UnlockAnarch => ("unlock_anarch", None, None, None),
                    P::BurnBloodOfClan(k) => ("burn_blood", Some(k), None, None),
                    P::HostCard(h) => ("host_card", None, card(h), None),
                    P::LockOldest(k) => ("lock_oldest", Some(k), None, None),
                    P::OldestHuntingGround(_) => ("oldest_gain_blood", None, None, None),
                    P::BurnOfCrowdedClan => ("burn_crowded_clan", None, None, None),
                    P::Damage(d) => ("damage", None, None, Some(d)),
                };
                About {
                    seat: Some(of.0),
                    detail: detail(d),
                    clan: clan.map(snake),
                    card: c,
                    n,
                    ..a()
                }
            }
            I::AvoidDamage {
                minion,
                blood,
                damage,
                ..
            } => About {
                minion: m(minion),
                n: Some(blood),
                m: Some(damage),
                ..a()
            },
            I::DiscardFromHandOf { of, .. }
            | I::PreySlotToPool { of, .. }
            | I::NameAshPair { of, .. } => About {
                seat: Some(of.0),
                ..a()
            },
            I::NameSlotPair { of, from, .. } => About {
                seat: Some(of.0),
                detail: from.map(|_| "named".to_string()),
                ..a()
            },
            I::DiscardChosen {
                left, combat_only, ..
            } => About {
                n: Some(left),
                detail: combat_only.then(|| "combat".to_string()),
                ..a()
            },
            I::RemoveFromAsh { of, left, .. } => About {
                seat: Some(of.0),
                n: Some(left),
                ..a()
            },
            I::MayDrawExtra { .. } => About {
                card: by_name("Learjet"),
                ..a()
            },
            I::PursuitDraw { .. } => About {
                card: by_name("Infernal Pursuit"),
                ..a()
            },
            I::PursuitDiscard { left, .. } => About {
                card: by_name("Infernal Pursuit"),
                n: Some(left),
                ..a()
            },
            I::PurgePick { of, host, .. } => About {
                seat: Some(of.0),
                minion: m(host),
                ..a()
            },
            I::StealUpTo { from, to, max, .. } | I::MoveBloodUpTo { from, to, max, .. } => About {
                minion: m(from),
                to: m(to),
                n: Some(max),
                ..a()
            },
            I::Redraw {
                minion, card: c, ..
            } => About {
                minion: m(minion),
                card: named(c),
                ..a()
            },
            I::ChooseForCard {
                card: c,
                clan,
                second,
                ..
            } => {
                let effects = zvtes::cards::lookup(st.cards[c.index()].card)
                    .and_then(|d| d.modes.first())
                    .map_or(&[][..], |m| m.effects);
                let has = |m: Modifier| effects.contains(&E::Layer(m));
                About {
                    card: card(c),
                    clan: clan.map(snake),
                    detail: detail(if second && has(Modifier::SecondChosenUnblockable) {
                        "unblockable_non_bleed"
                    } else if !second && has(Modifier::FirstChosenDoesNotUnlock) {
                        "does_not_unlock"
                    } else {
                        "chosen"
                    }),
                    ..a()
                }
            }
            I::ExchangeAnarch { ash, .. } => About {
                card: ash.and_then(named),
                ..a()
            },
            I::PoolForCard { card: c, n, .. } | I::LockForPool { card: c, n, .. } => About {
                card: card(c),
                n: Some(n),
                ..a()
            },
            I::MakeAnarch {
                card: c, minion, ..
            } => About {
                card: card(c),
                minion: m(minion),
                ..a()
            },
            I::AshurPicks {
                left, to_library, ..
            } => About {
                n: Some(left),
                detail: to_library.then(|| "to_library".to_string()),
                ..a()
            },
            I::AnarchConvert {
                convert, chosen, ..
            } => About {
                minion: m(convert),
                detail: chosen.map(|_| "chosen".to_string()),
                ..a()
            },
            I::UncontrolledBlood { clan, n, .. } => About {
                clan: clan.map(snake),
                n: Some(n),
                ..a()
            },
            I::CryptSearch { clan, .. } => About {
                clan: Some(snake(clan)),
                ..a()
            },
            I::DistributeFrom {
                from, clan, left, ..
            } => About {
                minion: m(from),
                clan: Some(snake(clan)),
                n: Some(left),
                ..a()
            },
            I::CircleBlood {
                circle,
                from,
                to,
                moving,
                ..
            } => About {
                clan: Some(snake(circle)),
                minion: from.and_then(m),
                to: to.and_then(m),
                card: moving.and_then(card),
                ..a()
            },
            I::PoolToVampires { to, .. } => About {
                to: to.and_then(m),
                ..a()
            },
            I::MayRecycleCanceled { card: c, .. } => About {
                card: named(c),
                ..a()
            },
            I::MayPutOn { held, .. } => About {
                card: named(held.card),
                minion: m(held.minion),
                ..a()
            },
            I::MayAddCounter { card: c, .. }
            | I::StorePreyAsh { card: c, .. }
            | I::StoreOwnAsh { card: c, .. }
            | I::StoredToLibraryTop { card: c, .. } => About {
                card: card(c),
                ..a()
            },
            I::MayGainPool { minion, n, .. } => About {
                minion: m(minion),
                n: Some(n),
                ..a()
            },
            I::MoveForBlood {
                payer, minion, n, ..
            } => About {
                minion: m(payer),
                to: m(minion),
                n: Some(n),
                ..a()
            },
            I::Replacement { .. }
            | I::DrawUp { .. }
            | I::Draws { .. }
            | I::Bottom { .. }
            | I::MarginPool { .. }
            | I::MarginBlood { .. }
            | I::RemoveUncontrolled { .. }
            | I::BecomesAnarch { .. }
            | I::AllyFromAsh { .. }
            | I::NameUncontrolled { .. }
            | I::BeastDisciplines { .. }
            | I::NameOwnAshCard { .. }
            | I::NameAshVampire { .. }
            | I::SecretNumber { .. } => return None,
            I::SearchEquipFor { minion, .. } => About {
                minion: m(minion),
                ..a()
            },
            I::PutInPlaySearch { minion, x, .. } => About {
                minion: m(minion),
                n: Some(x),
                ..a()
            },
            I::MoveCardToOldest { card: c, .. } => About {
                card: card(c),
                ..a()
            },
            I::MayPreventForBlood { minion, n, .. } => About {
                minion: m(minion),
                n: Some(n),
                ..a()
            },
            I::ShortfallPicks { from, .. } => About {
                minion: m(from),
                ..a()
            },
            I::MayUnlock { minion, .. } => About {
                minion: m(minion),
                ..a()
            },
            I::GiveCardTo { card: c, .. } => About {
                card: card(c),
                ..a()
            },
        });
    }
    match req.window {
        W::PlayTerms => {
            if let Some(p) = e.play_terms() {
                let mode = zvtes::cards::lookup(p.card)?.modes.get(p.mode as usize)?;
                // A second vampire chosen (Toreador Grand Ball).
                if let Some(Terms::OwnOfClan { clan, .. }) = mode.terms {
                    let unblockable = mode
                        .effects
                        .contains(&E::Layer(Modifier::SecondChosenUnblockable));
                    return Some(About {
                        clan: Some(snake(clan)),
                        detail: detail(if unblockable {
                            "second_unblockable_non_bleed"
                        } else {
                            "second_chosen"
                        }),
                        ..a()
                    });
                }
                return Some(match mode.terms? {
                    Terms::BloodRange { .. } => About {
                        detail: detail("blood_range"),
                        ..a()
                    },
                    Terms::UpToTargetBlood => About {
                        detail: detail("up_to_target_blood"),
                        minion: m(p.target.minion()),
                        ..a()
                    },
                    Terms::XBlood | Terms::XBloodIn { .. } => About {
                        detail: detail("x_blood"),
                        ..a()
                    },
                    Terms::UpToMargin(_) => About {
                        detail: detail("up_to_margin"),
                        ..a()
                    },
                    Terms::BoonPool => About {
                        detail: detail("boon_pool"),
                        ..a()
                    },
                    Terms::BurnNamedInPlay(name) => About {
                        detail: detail("burn_named"),
                        card: by_name(name),
                        ..a()
                    },
                    Terms::AnyAshAlly => About {
                        detail: detail(match p.ash {
                            None => "ash_heap_ally",
                            Some(_) => "ally",
                        }),
                        ..a()
                    },
                    Terms::EquipFromHand { .. } => match p.named {
                        None => About {
                            detail: detail("equipment"),
                            ..a()
                        },
                        Some(_) => About {
                            detail: detail("equipment_cost"),
                            minion: p.actor.and_then(m),
                            ..a()
                        },
                    },
                    Terms::EmployFromHand(_)
                    | Terms::EmployFromHandIgnoring(_)
                    | Terms::RecruitOrEmployFromHand => About {
                        detail: detail("put_in_play"),
                        ..a()
                    },
                    _ => return None,
                });
            }
            if e.cost_terms().is_some() {
                return Some(About {
                    detail: detail("cost_reductions"),
                    ..a()
                });
            }
            let (_, mode, target) = e.ability_terms()?;
            Some(match mode.terms? {
                Terms::Discard { n, .. } => About {
                    detail: detail("discard"),
                    n: Some(n),
                    ..a()
                },
                Terms::Count { .. } => About {
                    detail: detail("count"),
                    ..a()
                },
                Terms::XPool => About {
                    detail: detail("x_pool"),
                    ..a()
                },
                Terms::XCounters => About {
                    detail: detail("x_counters"),
                    ..a()
                },
                Terms::LifeFromTarget => About {
                    detail: detail("life_from_target"),
                    minion: m(target.minion()),
                    ..a()
                },
                Terms::AshCard => About {
                    detail: detail("ash_card"),
                    ..a()
                },
                Terms::DirectedHandCard => About {
                    detail: detail("hand_card"),
                    ..a()
                },
                Terms::VampireToHoldTarget => About {
                    detail: detail("hold_target"),
                    card: match target {
                        zvtes::cards::Target::Card(c) => card(c),
                        _ => None,
                    },
                    ..a()
                },
                _ => return None,
            })
        }
        W::RiderTerms => {
            let (_, left) = e.rider_terms()?;
            Some(About {
                n: Some(left),
                ..a()
            })
        }
        W::ReferendumTerms => {
            let r = e.referendum()?;
            let def = r.def?;
            let term = *def.terms.get(r.terms.term).or(def.terms.last())?;
            let chosen = r.terms.chosen.map(|s| s.0);
            let mut cap = None;
            let (d, n, mm) = match (term, def.effect) {
                (Term::ChooseSeat(_), R::BurnAllocatedChosenGains(g)) => {
                    ("gains_pool", Some(g), None)
                }
                (Term::ChooseSeat(_), R::MoveAllocatedFromChosen) => ("gives_pool", None, None),
                (Term::ChooseSeat(_), R::ChosenTakesChosenCard) => ("takes_card", None, None),
                (Term::ChooseCard(_), R::ChosenTakesChosenCard) => {
                    ("card_changes_control", None, None)
                }
                (
                    Term::ChooseSeats { .. },
                    R::BurnChosen {
                        base,
                        young_bonus,
                        young_cap,
                    },
                ) => {
                    cap = Some(young_cap);
                    ("burn_chosen", Some(base), Some(base + young_bonus))
                }
                (Term::Allocate { .. }, R::BurnAllocated | R::BurnAllocatedChosenGains(_)) => {
                    ("burn_allocated", None, None)
                }
                (Term::Allocate { .. }, R::MoveAllocatedFromChosen) => {
                    ("move_allocated", None, None)
                }
                (Term::AllocatePerChosenWithCards { .. }, R::LockChosenBurnAllocated) => {
                    ("lock_burn_allocated", None, None)
                }
                (Term::ChooseMinions { .. }, R::Banish) => ("banish", None, None),
                (Term::ChooseMinions { max, .. }, R::Expel) => ("expel", Some(max), None),
                (Term::ChooseMinions { .. }, R::PutOnChosen { .. }) => {
                    ("put_on_chosen", None, None)
                }
                (Term::ChooseMinions { .. }, R::LockChosenBurnAllocated) => {
                    ("lock_chosen", None, None)
                }
                (Term::ChooseClan, R::GainPerClanVampire) => ("gain_per_clan", None, None),
                _ => return None,
            };
            Some(About {
                detail: detail(d),
                n,
                m: mm,
                cap,
                seat: chosen,
                ..a()
            })
        }
        _ => None,
    }
}

/// The key of `choice`, offered to `req.seat`.
pub fn key(e: &Engine, req: &DecisionRequest, choice: &Choice, names: &impl Names) -> ChoiceKey {
    let x = Ctx {
        e,
        seat: req.seat,
        names,
    };
    let mut k = ChoiceKey {
        kind: kind(choice),
        card: None,
        actor: None,
        minion: None,
        target: None,
        action: None,
        mode: None,
        amount: None,
        favor: None,
        blood: None,
        zone: None,
        value: None,
        ability: None,
    };
    match *choice {
        Choice::Pass
        | Choice::BurnEdge
        | Choice::BurnPool
        | Choice::DrawLibrary
        | Choice::DrawCrypt
        | Choice::HandStrike
        | Choice::UseManeuver
        | Choice::UsePress
        | Choice::UsePrevent
        | Choice::PayBlockToll
        | Choice::PayToCancel
        | Choice::StrikeCombatEnds
        | Choice::TakeDamageAfterResolution
        | Choice::CommittedStrike
        | Choice::ContinueAction
        | Choice::GainStrength
        | Choice::BurnBloodToAvoid
        | Choice::Diablerise => {}
        Choice::PayContest(c) | Choice::YieldContest(c) => match c {
            Contest::Card(m) => k.minion = Some(x.minion(m)),
            Contest::Title(m) => {
                k.minion = Some(x.minion(m));
                k.value = Some("title".into());
            }
            Contest::Library(c) => k.card = Some(x.in_play(c)),
        },
        Choice::Play {
            card,
            mode: m,
            actor,
            target,
        } => {
            let r = x.hand(card);
            k.card = Some(r);
            k.mode = Some(mode(e.virtual_hand(req.seat)[card as usize].0, m));
            k.actor = actor.map(|a| x.minion(a));
            k.target = x.target(target);
        }
        Choice::UseAbility {
            source,
            ability,
            actor,
            target,
        } => {
            match source {
                // The card, and the minion it is on (its lasting effect's,
                // the card in the ash heap).
                AbilitySource::Card(c) => {
                    k.card = Some(x.in_play(c));
                    k.minion = e.state.cards[c.index()].host.map(|m| x.minion(m));
                }
                AbilitySource::Vampire(m) => k.minion = Some(x.minion(m)),
            }
            k.ability = Some(ability);
            k.actor = actor.map(|a| x.minion(a));
            k.target = x.target(target);
        }
        Choice::ResolveRider { card }
        | Choice::BurnInPlay { card }
        | Choice::TakeEquipment { card }
        | Choice::ChooseCard(card)
        | Choice::BurnWeaponStrike { weapon: card } => k.card = Some(x.in_play(CardId(card))),
        Choice::RoundStrike { at } => k.card = at.map(|c| x.in_play(c)),
        Choice::ResolvePrinted { minion }
        | Choice::BurnBloodFrom { minion }
        | Choice::DeclareBlock { minion }
        | Choice::ChooseMinion(minion)
        | Choice::BurnBloodForIntercept { minion }
        | Choice::UnlockForBlood { minion }
        | Choice::UnlockInfernal { minion }
        | Choice::BurnCorruptionToFail { minion }
        | Choice::BurnCorruptionToUnlock { minion } => k.minion = Some(x.minion(minion)),
        Choice::SetX(n) => k.amount = Some(n),
        Choice::Discard(i) | Choice::Bottom(i) => k.card = Some(x.hand(i)),
        Choice::BurnForVote { card, favor } => {
            k.card = Some(x.hand(card));
            k.favor = Some(favor);
        }
        Choice::FindCard(c) => k.card = Some(library(c)),
        Choice::TakeFrom { zone, card } => {
            k.card = Some(library(card));
            k.zone = Some(ZoneRef {
                seat: req.seat.0,
                zone: match zone {
                    Z::Library => Zone::Library,
                    Z::Hand => Zone::Hand,
                    Z::AshHeap => Zone::AshHeap,
                },
            });
        }
        // The Great Beast's Disciplines: a slot of the Discipline list.
        // The Great Beast's Disciplines: the one this slot of the list is.
        Choice::Slot(i) if matches!(e.interrupt(), Some(Interrupt::BeastDisciplines { .. })) => {
            k.value = zvtes::cards::GREAT_BEAST_DISCIPLINES
                .get(i as usize)
                .map(|d| d.to_lowercase())
        }
        Choice::Slot(i) => k.card = Some(x.slot(i)),
        Choice::Transfer { slot }
        | Choice::Withdraw { slot }
        | Choice::InfluenceOut { slot }
        | Choice::Merge { slot } => {
            k.target = Some(Target::Card {
                copy: x.slot(slot).copy.unwrap_or(UNSEEN),
            });
        }
        Choice::AshVampire(c) => {
            k.card = Some(CardRef {
                id: Some(CRYPT[c as usize].id),
                copy: None,
                from: None,
            });
            // A crypt search takes it from the crypt; Legacy's merge from
            // the crypt if there.
            let crypt = match e.interrupt() {
                Some(Interrupt::CryptSearch { .. }) => true,
                Some(
                    Interrupt::BaseSearch { seat, .. } | Interrupt::PutInPlaySearch { seat, .. },
                ) => e.state.seats[seat.index()].crypt.contains(&c),
                _ => false,
            };
            k.zone = Some(ZoneRef {
                seat: req.seat.0,
                zone: if crypt { Zone::Crypt } else { Zone::AshHeap },
            });
        }
        Choice::DrawStored { card, stored } => {
            k.card = Some(CardRef {
                from: Some(x.card(CardId(card))),
                ..library(stored)
            })
        }
        Choice::TakeAction {
            minion,
            action,
            target,
        } => {
            k.actor = Some(x.minion(minion));
            k.action = Some(action_kind(action));
            k.target = target.map(|t| Target::Card { copy: x.minion(t) });
        }
        Choice::PayRescue { from_acting } => k.amount = Some(from_acting),
        Choice::CastVotes { minion, favor } => {
            k.minion = Some(x.minion(minion));
            k.favor = Some(favor);
        }
        Choice::CastEdge { favor } | Choice::CastCardVote { favor } => k.favor = Some(favor),
        Choice::AllocateTo { seat, n } => {
            k.target = Some(Target::Seat { seat: seat.0 });
            k.amount = Some(n);
        }
        Choice::AllocateToCard { card, n } => {
            k.target = Some(Target::Card {
                copy: x.card(CardId(card)),
            });
            k.amount = Some(n);
        }
        Choice::ChooseSeat(s) => k.target = Some(Target::Seat { seat: s.0 }),
        Choice::ChooseClan(v) | Choice::ChooseCircle(v) | Choice::ChooseDiscipline(v) => {
            k.value = Some(snake(v))
        }
        Choice::EmployFromHand { card, mode: m } => {
            k.card = Some(library(card));
            k.mode = Some(mode(card, m));
        }
    }
    k
}

pub fn kind(c: &Choice) -> OptionKind {
    use OptionKind as K;
    match c {
        Choice::Pass => K::Pass,
        Choice::BurnEdge => K::EdgePool,
        Choice::PayContest(_) => K::PayContest,
        Choice::YieldContest(_) => K::YieldContest,
        Choice::Play { .. } => K::Play,
        Choice::UseAbility { .. } => K::UseAbility,
        Choice::ResolveRider { .. } => K::ResolveRider,
        Choice::ResolvePrinted { .. } => K::ResolvePrinted,
        Choice::BurnInPlay { .. } => K::BurnInPlay,
        Choice::BurnPool => K::BurnPool,
        Choice::BurnBloodFrom { .. } => K::BurnBloodFrom,
        Choice::SetX(_) => K::SetX,
        Choice::Discard(_) => K::Discard,
        Choice::FindCard(_) => K::FindCard,
        Choice::TakeFrom { .. } => K::TakeFrom,
        Choice::Slot(_) => K::Slot,
        Choice::AshVampire(_) => K::AshVampire,
        Choice::DrawLibrary => K::DrawLibrary,
        Choice::DrawStored { .. } => K::DrawStored,
        Choice::Bottom(_) => K::Bottom,
        Choice::TakeEquipment { .. } => K::TakeEquipment,
        Choice::TakeAction { .. } => K::TakeAction,
        Choice::DeclareBlock { .. } => K::DeclareBlock,
        Choice::PayRescue { .. } => K::PayRescue,
        Choice::CastVotes { .. } => K::CastVotes,
        Choice::CastEdge { .. } => K::CastEdge,
        Choice::CastCardVote { .. } => K::CastCardVote,
        Choice::BurnForVote { .. } => K::BurnForVote,
        Choice::AllocateTo { .. } => K::AllocateTo,
        Choice::ChooseSeat(_) => K::ChooseSeat,
        Choice::ChooseMinion(_) => K::ChooseMinion,
        Choice::ChooseClan(_) => K::ChooseClan,
        Choice::ChooseCircle(_) => K::ChooseCircle,
        Choice::ChooseDiscipline(_) => K::ChooseDiscipline,
        Choice::ChooseCard(_) => K::ChooseCard,
        Choice::AllocateToCard { .. } => K::AllocateToCard,
        Choice::Transfer { .. } => K::Transfer,
        Choice::Withdraw { .. } => K::Withdraw,
        Choice::DrawCrypt => K::DrawCrypt,
        Choice::InfluenceOut { .. } => K::InfluenceOut,
        Choice::Merge { .. } => K::Merge,
        Choice::HandStrike => K::HandStrike,
        Choice::UseManeuver => K::UseManeuver,
        Choice::UsePress => K::UsePress,
        Choice::UsePrevent => K::UsePrevent,
        Choice::PayBlockToll => K::PayBlockToll,
        Choice::PayToCancel => K::PayToCancel,
        Choice::StrikeCombatEnds => K::StrikeCombatEnds,
        Choice::RoundStrike { .. } => K::RoundStrike,
        Choice::TakeDamageAfterResolution => K::TakeDamageAfterResolution,
        Choice::CommittedStrike => K::CommittedStrike,
        Choice::BurnWeaponStrike { .. } => K::BurnWeaponStrike,
        Choice::EmployFromHand { .. } => K::EmployFromHand,
        Choice::BurnBloodForIntercept { .. } => K::BurnBloodForIntercept,
        Choice::UnlockForBlood { .. } => K::UnlockForBlood,
        Choice::UnlockInfernal { .. } => K::UnlockInfernal,
        Choice::ContinueAction => K::ContinueAction,
        Choice::GainStrength => K::GainStrength,
        Choice::BurnBloodToAvoid => K::BurnBloodToAvoid,
        Choice::BurnCorruptionToFail { .. } => K::BurnCorruptionToFail,
        Choice::BurnCorruptionToUnlock { .. } => K::BurnCorruptionToUnlock,
        Choice::Diablerise => K::Diablerise,
    }
}
