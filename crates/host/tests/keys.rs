//! Every option of every decision has its own key (docs/protocol.md §7),
//! and no choice is offered twice (one card reachable from two windows):
//! what lets a record read by choice on a later engine. Random games on
//! the shipped decks, every seat answered at random, keys by the engine's
//! copies (as records name them).

use std::collections::HashMap;

use zvtes::agent::RandomLegal;
use zvtes::game::{Engine, Step};
use zvtes_host::describe::{key, EngineNames};

fn engine(seed: u64) -> Engine {
    let decks = zvtes_host::catalog();
    let resolved: Vec<_> = (0..4)
        .map(|i| {
            zvtes_host::deck::resolve(0, &decks[(seed as usize + i * 3) % decks.len()].1)
                .expect("a shipped deck")
        })
        .collect();
    let lists: Vec<_> = resolved
        .iter()
        .map(|d| (d.crypt.as_slice(), d.library.as_slice()))
        .collect();
    Engine::with_lists(seed, &lists)
}

#[test]
fn two_options_of_a_decision_never_share_a_key() {
    let seeds: u64 = std::env::var("HOST_SEEDS").map_or(60, |s| s.parse().unwrap());
    let mut decisions = 0usize;
    let mut by_kind: HashMap<String, usize> = HashMap::new();
    for seed in 1..=seeds {
        let mut e = engine(seed);
        let mut agent = RandomLegal::new(seed);
        for _ in 0..20_000 {
            match e.run() {
                Step::Over(_) => break,
                Step::Decision(req) => {
                    decisions += 1;
                    let keys: Vec<_> = req
                        .options
                        .iter()
                        .map(|c| key(&e, &req, c, &EngineNames))
                        .collect();
                    for (i, a) in keys.iter().enumerate() {
                        *by_kind.entry(format!("{:?}", a.kind)).or_default() += 1;
                        for (j, b) in keys.iter().enumerate().skip(i + 1) {
                            assert!(
                                req.options[i] != req.options[j],
                                "seed {seed}, {:?}: {:?} offered twice",
                                req.window,
                                req.options[i]
                            );
                            assert!(
                                a != b,
                                "seed {seed}, {:?}: options {i} {:?} and {j} {:?} share {a:?}",
                                req.window,
                                req.options[i],
                                req.options[j]
                            );
                        }
                    }
                    let pick = agent.choose(&req);
                    e.choose(pick);
                }
            }
        }
    }
    assert!(decisions > 10_000, "{decisions} decisions");
    eprintln!("{decisions} decisions, {} option kinds", by_kind.len());
}

/// Every option's key names what its kind is about (docs/protocol.md §7),
/// so a log reads it without its English: a play its card and mode, an
/// action its minion, kind and (a bleed) the seat bled, a choice among
/// minions its minion, an uncontrolled vampire its slot, a combatant's
/// choice its minion, an amount its amount. The few with nothing of their
/// own (a pass, the Edge's pool, a toll paid) read from the decision's
/// `source` and `purpose`.
#[test]
fn every_option_s_key_names_what_it_is_about() {
    use zvtes::decision::{ActionKind, Choice as C};
    let seeds: u64 = std::env::var("HOST_SEEDS").map_or(60, |s| s.parse().unwrap());
    let mut checked = 0usize;
    for seed in 1..=seeds {
        let mut e = engine(seed);
        let mut agent = RandomLegal::new(seed);
        for _ in 0..20_000 {
            match e.run() {
                Step::Over(_) => break,
                Step::Decision(req) => {
                    for c in &req.options {
                        let k = key(&e, &req, c, &EngineNames);
                        let complete = match *c {
                            C::Play { .. } => k.card.is_some() && k.mode.is_some(),
                            C::UseAbility { .. } => {
                                (k.card.is_some() || k.minion.is_some()) && k.ability.is_some()
                            }
                            C::TakeAction { action, .. } => {
                                k.actor.is_some()
                                    && k.action.is_some()
                                    && (action != ActionKind::Bleed || k.target.is_some())
                            }
                            C::ResolvePrinted { .. }
                            | C::BurnBloodFrom { .. }
                            | C::DeclareBlock { .. }
                            | C::ChooseMinion(_)
                            | C::BurnBloodForIntercept { .. }
                            | C::UnlockForBlood { .. }
                            | C::UnlockInfernal { .. }
                            | C::BurnCorruptionToFail { .. }
                            | C::BurnCorruptionToUnlock { .. }
                            | C::CastVotes { .. } => k.minion.is_some(),
                            C::Transfer { .. }
                            | C::Withdraw { .. }
                            | C::InfluenceOut { .. }
                            | C::Merge { .. } => k.slot.is_some(),
                            C::Slot(_) => k.slot.is_some() || k.value.is_some(),
                            C::HandStrike
                            | C::UseManeuver
                            | C::UsePress
                            | C::UsePrevent
                            | C::StrikeCombatEnds
                            | C::CommittedStrike
                            | C::GainStrength
                            | C::CancelCombat
                            | C::RoundStrike { .. }
                            | C::ContinueAction
                            | C::TakeDamageAfterResolution => k.actor.is_some(),
                            C::Diablerise => k.actor.is_some() && k.minion.is_some(),
                            C::Discard(_)
                            | C::Bottom(_)
                            | C::BurnForVote { .. }
                            | C::FindCard(_)
                            | C::TakeFrom { .. }
                            | C::ResolveRider { .. }
                            | C::BurnInPlay { .. }
                            | C::TakeEquipment { .. }
                            | C::ChooseCard(_)
                            | C::BurnWeaponStrike { .. }
                            | C::DrawStored { .. }
                            | C::AshVampire(_)
                            | C::EmployFromHand { .. } => k.card.is_some(),
                            C::SetX(_) | C::PayRescue { .. } => k.amount.is_some(),
                            C::AllocateTo { .. } | C::AllocateToCard { .. } => {
                                k.target.is_some() && k.amount.is_some()
                            }
                            C::ChooseSeat(_) => k.target.is_some(),
                            C::FollowPath { .. } => k.minion.is_some() && k.value.is_some(),
                            C::ChooseClan(_) | C::ChooseCircle(_) | C::ChooseDiscipline(_) => {
                                k.value.is_some()
                            }
                            C::CastEdge { .. } | C::CastCardVote { .. } => k.favor.is_some(),
                            C::PayContest(_) | C::YieldContest(_) => {
                                k.minion.is_some() || k.card.is_some()
                            }
                            C::Pass
                            | C::BurnEdge
                            | C::WithdrawFromGame
                            | C::BurnPool
                            | C::DrawLibrary
                            | C::DrawCrypt
                            | C::PayBlockToll
                            | C::PayToCancel
                            | C::BurnBloodToAvoid => true,
                        };
                        assert!(
                            complete,
                            "seed {seed}, {:?}: {c:?} keyed as {k:?}",
                            req.window
                        );
                        checked += 1;
                    }
                    let pick = agent.choose(&req);
                    e.choose(pick);
                }
            }
        }
    }
    assert!(checked > 10_000, "{checked} options");
}

/// A card found by name says where it is (docs/protocol.md §7 `zone`):
/// the library, hand or ash heap of the seat it is taken from, the card
/// in it; none only for a card named without taking it (Master:
/// Discipline) or one stored on a card. A library search, the seat's own
/// ash heap and another seat's (Trochomancy) are each met.
#[test]
fn a_found_card_names_the_zone_it_is_in() {
    use zvtes::decision::Choice;
    use zvtes::draws::Interrupt;
    use zvtes_api::ids::Zone;
    let seeds: u64 = std::env::var("HOST_SEEDS").map_or(60, |s| s.parse().unwrap());
    // Another seat's ash heap: Trochomancy's, added to the deck playing it.
    let mut troch = zvtes_host::catalog()
        .into_iter()
        .find(|(_, d)| d.contains_key("102026"))
        .expect("a deck with Trochomancy")
        .1;
    *troch.entry("102026".into()).or_default() += 8;
    let troch = zvtes_host::deck::resolve(0, &troch).expect("Trochomancy's deck");
    let lists = vec![(troch.crypt.as_slice(), troch.library.as_slice()); 4];
    let mut met: HashMap<(Zone, bool), usize> = HashMap::new();
    for seed in 1..=seeds + 12 {
        let mut e = if seed <= seeds {
            engine(seed)
        } else {
            Engine::with_lists(seed, &lists)
        };
        let mut agent = RandomLegal::new(seed);
        for _ in 0..20_000 {
            match e.run() {
                Step::Over(_) => break,
                Step::Decision(req) => {
                    for c in &req.options {
                        let Choice::FindCard(card) = *c else {
                            continue;
                        };
                        let k = key(&e, &req, c, &EngineNames);
                        let named = matches!(
                            e.interrupt(),
                            Some(
                                Interrupt::NameMasterDiscipline { .. }
                                    | Interrupt::StoredToLibraryTop { .. }
                            )
                        );
                        let Some(z) = k.zone else {
                            assert!(
                                named,
                                "seed {seed}, {:?}: {c:?} without its zone",
                                req.window
                            );
                            continue;
                        };
                        let s = &e.state.seats[z.seat as usize];
                        let cards = match z.zone {
                            Zone::Library => &s.library,
                            Zone::Hand => &s.hand,
                            Zone::AshHeap => &s.ash,
                            other => panic!("seed {seed}: {c:?} found in {other:?}"),
                        };
                        assert!(
                            cards.contains(&card),
                            "seed {seed}, {:?}: {c:?} not in {z:?}",
                            req.window
                        );
                        *met.entry((z.zone, z.seat == req.seat.0)).or_default() += 1;
                    }
                    let pick = agent.choose(&req);
                    e.choose(pick);
                }
            }
        }
    }
    eprintln!("found cards by zone and own seat: {met:?}");
    for want in [
        (Zone::Library, true),
        (Zone::AshHeap, true),
        (Zone::AshHeap, false),
    ] {
        assert!(met.contains_key(&want), "{want:?} never met: {met:?}");
    }
}

/// A pass on the seat's own block attempt says so (docs/protocol.md §7):
/// `block` with its minion's copy, the decline (`declines`) while short
/// of the stealth (rules-qa 82.1); every seat's, at random.
#[test]
fn a_pass_on_its_own_attempt_tells_it() {
    use zvtes::actions::ActionState;
    use zvtes::decision::{Choice, WindowLabel};
    use zvtes_api::option::EffectKind;
    use zvtes_host::describe::effects;
    let seeds: u64 = std::env::var("HOST_SEEDS").map_or(60, |s| s.parse().unwrap());
    let (mut short, mut meets) = (0usize, 0usize);
    for seed in 1..=seeds {
        let mut e = engine(seed);
        let mut agent = RandomLegal::new(seed);
        for _ in 0..20_000 {
            match e.run() {
                Step::Over(_) => break,
                Step::Decision(req) => {
                    let a = e
                        .action()
                        .filter(|_| req.window == WindowLabel::ActionBlock);
                    if let Some((a, ActionState::Attempt(m))) = a.map(|a| (a, a.state)) {
                        if e.state.minions[m.index()].owner == req.seat
                            && req.options.contains(&Choice::Pass)
                        {
                            let declines = a.intercept_of(&e.state, m) < a.stealth(&e.state);
                            let told = effects(&e, &req, &Choice::Pass, &EngineNames);
                            let block = told.iter().find(|f| f.kind == EffectKind::Block);
                            assert!(
                                block.is_some_and(|f| f.copy.is_some()
                                    && (f.detail.as_deref() == Some("declines")) == declines),
                                "seed {seed}: the pass on its own attempt told {told:?}"
                            );
                            *if declines { &mut short } else { &mut meets } += 1;
                        }
                    }
                    let pick = agent.choose(&req);
                    e.choose(pick);
                }
            }
        }
    }
    eprintln!("passes on an own attempt: {short} short, {meets} meeting");
    assert!(short > 0 && meets > 0, "{short} short, {meets} meeting");
}

/// An ash heap's vampire names the ash heap it is in (docs/protocol.md §7):
/// superior Shroud of Decay removing cards from the prey's ash heap, its
/// vampires there offered as `ash_vampire` of the prey's seat (the owner's
/// playtest, 2026-10-25).
#[test]
fn an_ash_vampire_names_the_ash_heap_it_is_in() {
    use zvtes::cards::Target;
    use zvtes::crypt::CRYPT;
    use zvtes::decision::Choice;
    use zvtes::ids::SeatId;
    use zvtes::state::find_library;
    use zvtes_api::ids::Zone;
    let obl = CRYPT
        .iter()
        .position(|c| c.disciplines.contains(&"OBL") && !c.imbued)
        .expect("a superior Oblivion vampire") as u16;
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), obl, 5, false);
    e.add_minion(SeatId(0), 0, 4, false);
    e.add_minion(SeatId(1), 1, 5, false);
    e.state.seats[0].hand.push(find_library("Shroud of Decay"));
    for _ in 0..7 {
        e.state.seats[1].ash.push(find_library("Soak"));
    }
    e.state.seats[1].ash_crypt.push(2);
    let shroud = Choice::Play {
        card: 0,
        mode: 1,
        actor: Some(a),
        target: Target::Seat(SeatId(1)),
    };
    let mut met = 0;
    for _ in 0..40 {
        let Step::Decision(req) = e.run() else { break };
        for c in &req.options {
            if let Choice::AshVampire(_) = c {
                let z = key(&e, &req, c, &EngineNames).zone.expect("its zone");
                assert_eq!((z.seat, z.zone), (1, Zone::AshHeap), "{c:?}");
                met += 1;
            }
        }
        let i = req.options.iter().position(|o| *o == shroud).unwrap_or(0);
        e.choose(i);
        if met > 0 {
            break;
        }
    }
    assert!(met > 0, "the prey's ash heap vampire offered");
}

/// Vestgeir's play from another Methuselah's ash heap names that heap,
/// not his player's (docs/protocol.md §8's `zone`).
#[test]
fn a_play_from_another_ash_heap_names_whose() {
    use zvtes::decision::Choice;
    use zvtes::ids::SeatId;
    use zvtes::state::{find_card, find_library};
    use zvtes_api::ids::Zone;
    let mut e = Engine::bare(2);
    e.add_minion(SeatId(0), find_card("Vestgeir"), 10, false);
    e.add_minion(SeatId(0), 0, 4, false);
    e.add_minion(SeatId(1), 1, 4, false);
    e.state.seats[1]
        .ash
        .push(find_library("Ancilla Empowerment"));
    let Step::Decision(req) = e.run() else {
        panic!("a decision")
    };
    let from_ash = req
        .options
        .iter()
        .find(|c| matches!(c, Choice::Play { .. }))
        .expect("the play from seat 1's ash heap");
    let z = key(&e, &req, from_ash, &EngineNames)
        .zone
        .expect("its zone");
    assert_eq!((z.seat, z.zone), (1, Zone::AshHeap));
}
