//! Random-legal fuzzing with invariants and replay determinism (DESIGN §7):
//! every game terminates within a decision budget, invariants hold at every
//! decision point, and (seed, decision log) resimulation (every 4th seed)
//! reproduces the exact final state hash. Starting positions: the rulebook
//! opening, and seeded mid-game tables that reach the torpor / referendum /
//! contest / oust / card-play paths random play from the opening rarely or
//! never gets to. Every game is pinned in `tests/golden/<test>.txt` (golden replays,
//! DESIGN §7.3; `GOLDEN_BLESS=1` rewrites them).

use std::collections::{HashMap, HashSet};

mod common;

use common::tables::{table, untwin, ALLY_SLICE, PRECON_SLICE, SLICE};
use zvtes::agent::RandomLegal;
use zvtes::cards::AbilitySource;
use zvtes::copies::{census, CopyId};
use zvtes::crypt::{CryptCard, CRYPT};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step, TURN_LIMIT};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::state::{find_card, CardId};

/// The opening's seeds; a fixture sets its own (`Fixture::seeds`): about
/// twice the seed by which its last coverage key is first taken, in
/// hundreds (at most 300: the midgame's last key comes at seed 163). `FUZZ_SEEDS=n` plays n seeds everywhere instead (a deeper
/// sweep; the golden check covers the seeds it pins).
const SEEDS: u64 = 300;
/// Every `REPLAY_EVERY`th seed is replayed from its decision log alone (the
/// goldens pin the rest across runs).
const REPLAY_EVERY: u64 = 4;
const DECISION_BUDGET: u64 = 200_000;

/// Library-card conservation baseline per seat (library + hand + ash +
/// cards in play), fixed at the start of a game.
fn zone_totals(e: &Engine) -> Vec<usize> {
    // Cards out of play on a banished vampire (Banishment: they stay with
    // it), gathered once.
    let banished: Vec<CardId> = e
        .state
        .seats
        .iter()
        .flat_map(|s| &s.uncontrolled)
        .filter_map(|u| u.banished.as_ref())
        .flat_map(|b| b.cards.iter().copied())
        .collect();
    let banished = |id: usize| banished.contains(&CardId(id as u32));
    e.state
        .seats
        .iter()
        .enumerate()
        .map(|(i, s)| {
            // A card in play with the library cards stored on it, or out of
            // play on a banished vampire.
            let in_play: usize = e
                .state
                .cards
                .iter()
                .enumerate()
                // An effect-only card is a lingering effect; its card is in
                // the ash heap.
                .filter(|(ci, c)| {
                    c.owner.index() == i && !c.effect_only && (c.in_play || banished(*ci))
                })
                .map(|(_, c)| c)
                .map(|c| 1 + c.stored.len())
                .sum();
            // An Absimiliard's Army ghoul: its card, in play as the minion.
            let tokens = e
                .state
                .minions
                .iter()
                .filter(|m| {
                    m.in_play
                        && matches!(m.kind, zvtes::state::MinionKind::Token { .. })
                        && m.origin.or(m.temp_from).unwrap_or(m.owner).index() == i
                })
                .count();
            s.library.len() + s.hand.len() + s.ash.len() + s.removed.len() + in_play + tokens
        })
        .collect()
}

/// The dealt copies (`zvtes::copies`): each copy's card and the
/// Methuselah whose deck it came from, at the start of a game.
fn dealt_copies(e: &Engine) -> HashMap<CopyId, (u16, usize)> {
    use zvtes::copies::Place;
    census(e)
        .into_iter()
        .map(|(copy, card, place)| {
            let seat = match place {
                Place::Zone(s, _) | Place::Uncontrolled(s) => s as usize,
                Place::Card { id, .. } => e.state.cards[id as usize].owner.index(),
                Place::Minion { id, .. } => {
                    let m = &e.state.minions[id as usize];
                    m.origin.or(m.temp_from).unwrap_or(m.owner).index()
                }
                Place::InFlight(s) | Place::Banished { seat: s, .. } | Place::Training(s) => {
                    s as usize
                }
            };
            (copy, (card, seat))
        })
        .collect()
}

/// Copies are conserved: each in one place, still its card, and every
/// dealt copy somewhere until its Methuselah is ousted — or the game ends
/// (`over`: a play under way is dropped).
fn copies_conserved(
    e: &Engine,
    dealt: &HashMap<CopyId, (u16, usize)>,
    last: &mut HashMap<CopyId, zvtes::copies::Place>,
    over: bool,
    seed: u64,
) {
    let mut seen = HashMap::new();
    for (copy, card, place) in census(e) {
        // Every card was numbered at the start (`number_unnumbered`): a
        // card without a copy was put in place by the engine without one.
        assert!(
            copy != CopyId::NONE,
            "card {card} at {place:?} has no copy (seed {seed}, decision {})",
            e.log.len()
        );
        if let Some(before) = seen.insert(copy, place) {
            panic!(
                "copy {copy:?} twice: {before:?} and {place:?} (seed {seed}, decision {})",
                e.log.len()
            );
        }
        if let Some(&(dealt, _)) = dealt.get(&copy) {
            assert_eq!(
                card,
                dealt,
                "copy {copy:?} changed card at {place:?} (seed {seed}, decision {})",
                e.log.len()
            );
        }
    }
    // An ousted Methuselah's cards leave the game with them (rulebook
    // "Ending the Game"), theirs and those they controlled.
    let ousted_had = |place: Option<&zvtes::copies::Place>| {
        use zvtes::copies::Place;
        let seat = match place {
            Some(
                Place::Zone(s, _)
                | Place::Uncontrolled(s)
                | Place::InFlight(s)
                | Place::Banished { seat: s, .. }
                | Place::Training(s),
            ) => *s as usize,
            Some(Place::Minion { id, .. }) => e.state.minions[*id as usize].owner.index(),
            Some(Place::Card { id, .. }) => e.state.card_controller(CardId(*id)).index(),
            _ => return false,
        };
        e.state.seats[seat].ousted
    };
    for (copy, &(card, seat)) in dealt {
        assert!(
            over || seen.contains_key(copy) || e.state.seats[seat].ousted || ousted_had(last.get(copy)),
            "copy {copy:?} (card {card} of seat {seat}, last at {:?}) lost (seed {seed}, decision {}); \
             in play as {:?}",
            last.get(copy),
            e.log.len(),
            e.state
                .cards
                .iter()
                .enumerate()
                .filter(|(_, c)| c.card == card)
                .map(|(i, c)| (i, c.in_play, c.effect_only, c.copy.0, c.body, c.host, c.training))
                .collect::<Vec<_>>()
        );
    }
    // The copies gone with an ousted Methuselah keep their last place.
    for (copy, place) in last.drain() {
        seen.entry(copy).or_insert(place);
    }
    *last = seen;
}

fn invariants(e: &Engine, totals: &[usize], seed: u64, window: WindowLabel) {
    let st = &e.state;
    assert!(
        !st.seats[st.active.index()].ousted,
        "active seat is ousted (seed {seed})"
    );
    let mut edges = 0;
    for (i, s) in st.seats.iter().enumerate() {
        // Below 0 only while about to be ousted, a Life Boon pending (what
        // it owes, RTR 19970630).
        assert!(
            s.pool >= 0 || s.oust_pending,
            "negative pool at seat {i} (seed {seed})"
        );
        // At 0 only while ousted or about to be: the last pool paid ousts
        // at once, so a seat left at 0 shows a payment no oust followed.
        assert!(
            s.pool > 0 || s.ousted || s.oust_pending,
            "a living seat at 0 pool: seat {i} (seed {seed}, decision {}, {window:?})",
            e.log.len()
        );
        if s.ousted {
            assert_eq!(s.pool, 0, "ousted seat {i} kept pool (seed {seed})");
            assert!(!s.has_edge, "ousted seat {i} kept the Edge (seed {seed})");
        }
        if s.has_edge {
            edges += 1;
        }
        // Never over hand size — the cards not replaced yet counting
        // against it — except while discarding down (rulebook "Drawing
        // Cards": at once) or while Heart of Nizchetus draws "without
        // discarding" and moves as many back: over size, a draw choice can
        // only be one of its draws (a draw-up or a replacement is asked
        // below size).
        let size = st.hand_size(zvtes::ids::SeatId(i as u8));
        assert!(
            s.ousted
                || s.hand.len() + s.held.total() <= size
                || window == WindowLabel::DiscardDown
                || window == WindowLabel::Draw
                // Goblinism [MYT]: the haven on the vampire "instead" of
                // the hand, chosen before the discard.
                || matches!(e.interrupt(), Some(zvtes::draws::Interrupt::PutFoundOn { .. }))
                || window == WindowLabel::BottomCards,
            "hand over size at seat {i} (seed {seed}): {} cards + {} held, size {size}, at {window:?}",
            s.hand.len(),
            s.held.total(),
        );
        // Never under hand size between plays with a library to draw from
        // (rulebook "Drawing Cards": a hand-size change draws up at once).
        let quiet = e.action().is_none()
            && matches!(
                window,
                WindowLabel::MasterPhase
                    | WindowLabel::MinionPhase
                    | WindowLabel::InfluenceTransfers
                    | WindowLabel::DiscardPhase
            );
        assert!(
            s.ousted || !quiet || s.library.is_empty() || st.hand_load(zvtes::ids::SeatId(i as u8)) >= size,
            "hand under size at seat {i} (seed {seed}): {} cards + {} held, size {size}, at {window:?}",
            s.hand.len(),
            s.held.total(),
        );
    }
    // An equipment bar lives with its combat (rules-qa 76.16).
    assert!(
        e.combat().is_some() || st.equipment_barred.is_empty(),
        "equipment bar outside combat (seed {seed})"
    );
    // Every card in play is controlled by a living Methuselah.
    for ci in 0..st.cards.len() {
        if st.cards[ci].in_play {
            assert!(
                !st.seats[st.card_controller(CardId(ci as u32)).index()].ousted,
                "card in play controlled by an ousted seat (seed {seed}): {:?}",
                st.cards[ci]
            );
        }
    }
    let now = zone_totals(e);
    for i in 0..now.len() {
        if st.seats[i].ousted {
            // An oust removes what the Methuselah controlled (spec §6.1.2):
            // their cards another Methuselah controls (a minion card on
            // that Methuselah's minion, The Coven taken) stay in play.
            let stray = st.cards.iter().enumerate().any(|(ci, c)| {
                c.owner.index() == i
                    && c.in_play
                    && st.seats[st.card_controller(CardId(ci as u32)).index()].ousted
            });
            assert!(!stray, "ousted seat {i} kept library cards (seed {seed})");
            assert!(
                st.seats[i].library.is_empty() && st.seats[i].hand.is_empty(),
                "ousted seat {i} kept a library or hand (seed {seed})"
            );
            continue;
        }
    }
    // Pending plays are out of every zone until they settle: allow a
    // couple of cards in flight per seat (a play and its cancel). Counted
    // over the living seats together: a card stored on another
    // Methuselah's equipment and drawn from it (Shilmulo Tarot taken by
    // another Ravnos) moves to the drawer's zones — a hand does not track
    // its cards' owners (a known gap).
    let living: Vec<usize> = (0..now.len()).filter(|&i| !st.seats[i].ousted).collect();
    let a: usize = living.iter().map(|&i| now[i]).sum();
    let b: usize = living.iter().map(|&i| totals[i]).sum();
    // Cards taken from another's ash heap (The Erciyes Fragments) may
    // leave an ousted seat counted in a living one's zones.
    assert!(
        a <= b + st.taken_across as usize && a + 2 * living.len() >= b,
        "library cards not conserved: {a} vs {b} (seed {seed}): {now:?} vs {totals:?}"
    );
    assert!(edges <= 1, "multiple Edges (seed {seed})");
    for (i, m) in st.minions.iter().enumerate() {
        // In play: out of it, cards on it give nothing, and the blood they
        // let it hold stays (advanced rules 5.1.1).
        assert!(
            !m.in_play
                || st
                    .capacity_of(MinionId(i as u32))
                    .is_none_or(|c| m.blood <= c),
            "blood over capacity (seed {seed}): {:?} {:?}",
            m,
            st.capacity_of(MinionId(i as u32))
        );
        if m.in_play {
            assert!(
                !st.seats[m.owner.index()].ousted,
                "minion of an ousted seat in play (seed {seed})"
            );
            // Uniqueness: a seat never has two copies of a vampire, or of a
            // unique ally, in play (a created vampire or an Absimiliard's
            // Army ghoul is non-unique).
            let created = matches!(
                m.kind,
                zvtes::state::MinionKind::Created { .. } | zvtes::state::MinionKind::Token { .. }
            );
            let unique = !created
                && m.kind.ally_card().is_none_or(|c| {
                    zvtes::library::LIBRARY[c as usize]
                        .labels
                        .contains(&zvtes::library::Label::Unique)
                });
            let dup = unique
                && st.minions.iter().enumerate().any(|(j, o)| {
                    j != i && o.owner == m.owner && o.in_play && o.name() == m.name()
                });
            assert!(!dup, "seat holds two copies of {} (seed {seed})", m.name());
        }
        // A contest is over a title still claimed: a lost printed title
        // is not contested unless a title card or a stolen title replaced
        // it (out of play, a vampire's title card is out of play too).
        if m.in_play && m.title_contested {
            assert!(
                st.claimed_title(MinionId(i as u32)) != zvtes::crypt::Title::None,
                "lost title still contested (seed {seed}): {}",
                m.name()
            );
        }
    }
    // 1 VP per oust, +1 for the last survivor, or 0.5 per survivor at time:
    // never more than one VP per seat, in half-points.
    let half_vps: u32 = st.seats.iter().map(|s| s.half_vps).sum();
    assert!(
        half_vps <= 2 * st.seats.len() as u32,
        "more VPs than seats (seed {seed})"
    );
}

/// Coverage key of a chosen option: which kind of decision it was; card
/// plays are keyed by the card's printed name (`play`, `Some(Name)`), the rest
/// by kind alone (`pass`, `None`). Allocation-free: taken at every decision.
type Key = (&'static str, Option<&'static str>);

fn kind(e: &Engine, seat: SeatId, c: &Choice) -> Key {
    match c {
        Choice::Play { card, .. } => {
            let id = e.playable_card(seat, *card);
            ("play", Some(LIBRARY[id as usize].name))
        }
        Choice::UseAbility { source, .. } => match source {
            AbilitySource::Card(c) => {
                let id = e.state.cards[c.index()].card;
                ("ability", Some(LIBRARY[id as usize].name))
            }
            AbilitySource::Vampire(m) => ("ability", Some(e.state.minions[m.index()].name())),
        },
        Choice::ResolveRider { card } => {
            let id = e.state.cards[*card as usize].card;
            ("rider", Some(LIBRARY[id as usize].name))
        }
        _ => (kind_of(c), None),
    }
}

/// A key as the fixtures spell it: `play:Name`, or the kind alone.
fn spell((kind, name): Key) -> String {
    match name {
        Some(name) => format!("{kind}:{name}"),
        None => kind.to_string(),
    }
}

fn kind_of(c: &Choice) -> &'static str {
    match c {
        Choice::Pass => "pass",
        Choice::BurnEdge => "burn_edge",
        Choice::WithdrawFromGame => "withdraw_from_game",
        Choice::FollowPath { .. } => "follow_path",
        Choice::PayContest(_) => "pay_contest",
        Choice::YieldContest(_) => "yield_contest",
        Choice::Play { .. } => "play",
        Choice::UseAbility { .. } => "use_ability",
        Choice::ResolvePrinted { .. } => "resolve_printed",
        Choice::ResolveRider { .. } => "resolve_rider",
        Choice::BurnInPlay { .. } => "burn_in_play",
        Choice::BurnPool => "burn_pool",
        Choice::BurnBloodFrom { .. } => "burn_blood_from",
        Choice::SetX(_) => "set_x",
        Choice::Discard(_) => "discard",
        Choice::TakeAction { action, .. } => match action {
            ActionKind::Bleed => "bleed",
            ActionKind::Hunt => "hunt",
            ActionKind::LeaveTorpor => "leave_torpor",
            ActionKind::Rescue => "rescue",
            ActionKind::Diablerie => "diablerie",
            ActionKind::Employ => "employ",
            ActionKind::Recruit => "recruit",
            ActionKind::Political => "political",
            ActionKind::Card => "card_action",
            ActionKind::BurnCard => "burn_card",
            ActionKind::StealCard => "steal_card",
            ActionKind::Equip => "equip",
            ActionKind::EquipFrom => "equip_from",
            ActionKind::EquipFound => "equip_found",
            ActionKind::Ability => "ability_action",
            ActionKind::BecomeAnarch => "become_anarch",
            ActionKind::RedListHunt => "red_list_hunt",
            ActionKind::Power => "power",
            ActionKind::BurnImbued => "burn_imbued",
        },
        Choice::FindCard(_) => "find_card",
        Choice::TakeFrom { .. } => "take_from",
        Choice::Slot(_) => "slot",
        Choice::AshVampire(_) => "ash_vampire",
        Choice::DrawLibrary => "draw_library",
        Choice::DrawStored { .. } => "draw_stored",
        Choice::Bottom(_) => "bottom",
        Choice::TakeEquipment { .. } => "take_equipment",
        Choice::DeclareBlock { .. } => "declare_block",
        Choice::PayRescue { .. } => "pay_rescue",
        Choice::CastVotes { .. } => "cast_votes",
        Choice::CastEdge { .. } => "cast_edge",
        Choice::CastCardVote { .. } => "cast_card_vote",
        Choice::BurnForVote { .. } => "burn_for_vote",
        Choice::AllocateTo { .. } => "allocate_to",
        Choice::ChooseSeat(_) => "choose_seat",
        Choice::ChooseMinion(_) => "choose_minion",
        Choice::ChooseClan(_) => "choose_clan",
        Choice::ChooseCard(_) => "choose_card",
        Choice::AllocateToCard { .. } => "allocate_to_card",
        Choice::Transfer { .. } => "transfer",
        Choice::Withdraw { .. } => "withdraw",
        Choice::DrawCrypt => "draw_crypt",
        Choice::InfluenceOut { .. } => "influence_out",
        Choice::Merge { .. } => "merge",
        Choice::HandStrike => "hand_strike",
        Choice::UseManeuver => "use_maneuver",
        Choice::UsePrevent => "use_prevent",
        Choice::UnlockForBlood { .. } => "unlock_for_blood",
        Choice::BurnBloodForIntercept { .. } => "burn_blood_for_intercept",
        Choice::StrikeCombatEnds => "strike_combat_ends",
        Choice::RoundStrike { .. } => "round_strike",
        Choice::CommittedStrike => "committed_strike",
        Choice::TakeDamageAfterResolution => "take_damage_after_resolution",
        Choice::BurnWeaponStrike { .. } => "burn_weapon_strike",
        Choice::EmployFromHand { .. } => "employ_from_hand",
        Choice::PayBlockToll => "pay_block_toll",
        Choice::PayToCancel => "pay_to_cancel",
        Choice::UsePress => "use_press",
        Choice::ContinueAction => "continue_action",
        Choice::BurnCorruptionToFail { .. } => "burn_corruption_to_fail",
        Choice::GainStrength => "gain_strength",
        Choice::BurnBloodToAvoid => "burn_blood_to_avoid",
        Choice::BurnCorruptionToUnlock { .. } => "burn_corruption_to_unlock",
        Choice::Diablerise => "diablerise_blocked",
        Choice::CancelCombat => "cancel_combat",
        Choice::UnlockInfernal { .. } => "unlock_infernal",
        Choice::ChooseCircle(_) => "choose_circle",
        Choice::ChooseDiscipline(_) => "choose_discipline",
    }
}

/// Opening position: the rulebook start, 4 seats, random precon crypts and
/// libraries.
fn opening(seed: u64) -> Engine {
    Engine::new(seed, 4)
}

/// One played game: the final engine, its VPs, the decision kinds taken,
/// and its golden checkpoints (`Trail`).
struct Played {
    e: Engine,
    vps: Vec<u32>,
    chosen: HashSet<Key>,
    trail: Trail,
}

fn play(seed: u64, start: &dyn Fn(u64) -> Engine) -> Played {
    let mut e = start(seed);
    // Fixture tables put cards in place with no copy: numbered here, so
    // every game follows its copies.
    zvtes::copies::number_unnumbered(&mut e);
    let totals = zone_totals(&e);
    let dealt = dealt_copies(&e);
    let mut last = HashMap::new();
    let mut agent = RandomLegal::new(seed ^ 0x00c0_ffee);
    let mut chosen = HashSet::new();
    let mut trail = Trail::default();
    let trace = std::env::var("FUZZ_TRACE").is_ok();
    loop {
        assert!(
            e.decisions < DECISION_BUDGET,
            "decision budget exhausted (seed {seed})"
        );
        match e.run() {
            Step::Decision(req) => {
                // A decision's phase is the turn's, never the game's end.
                assert_ne!(
                    e.phase(),
                    zvtes::game::Phase::Over,
                    "a decision in no phase (seed {seed}, {:?})",
                    req.window
                );
                assert!(
                    !e.state.seats[req.seat.index()].ousted,
                    "decision requested from an ousted seat (seed {seed}, {:?})",
                    req.window
                );
                invariants(&e, &totals, seed, req.window);
                // At every decision of the replayed seeds, every 16th of the
                // others (the census is the fuzz's dearest check).
                if seed.is_multiple_of(REPLAY_EVERY) || e.decisions.is_multiple_of(16) {
                    copies_conserved(&e, &dealt, &mut last, false, seed);
                }
                for o in &req.options {
                    if let Choice::UseAbility {
                        source: AbilitySource::Card(card),
                        ..
                    } = o
                    {
                        // Spec §1.13.3: nothing printed on a contested card
                        // applies (its suspended layers are the scenario
                        // tests' claim, tests/contests.rs).
                        assert!(
                            !e.state.cards[card.index()].contested,
                            "ability offered from a contested card (seed {seed})"
                        );
                    }
                    if let Choice::UseAbility {
                        source: AbilitySource::Vampire(m),
                        ..
                    } = o
                    {
                        assert!(
                            !e.state.minions[m.index()].contested,
                            "ability offered from a contested vampire (seed {seed})"
                        );
                    }
                }
                // A withdrawal (offered at most unlock phases once a
                // fixture's library is empty) is taken on one seed in eight,
                // a path (any legacy Sabbat vampire's) on another, with no
                // draw; otherwise the draw is among the rest as before they
                // existed, a lone option without a draw (elided then).
                let withdraw = req
                    .options
                    .iter()
                    .position(|o| *o == Choice::WithdrawFromGame);
                let path = req
                    .options
                    .iter()
                    .position(|o| matches!(o, Choice::FollowPath { .. }));
                let late =
                    |o: &Choice| matches!(o, Choice::WithdrawFromGame | Choice::FollowPath { .. });
                let rest: Vec<usize> = (0..req.options.len())
                    .filter(|&i| !late(&req.options[i]))
                    .collect();
                let idx = match (seed % 8, withdraw, path, rest.len()) {
                    (7, Some(w), _, _) => w,
                    (6, _, Some(p), _) => p,
                    (.., n) if n == req.options.len() => agent.choose(&req),
                    (.., 1) => rest[0],
                    (.., n) => rest[agent.rng.below(n)],
                };
                if trace {
                    eprintln!(
                        "#{} {:?} {:?} -> {:?} {}",
                        e.log.len() + 1,
                        req.seat,
                        req.window,
                        req.options[idx],
                        spell(kind(&e, req.seat, &req.options[idx]))
                    );
                }
                chosen.insert(kind(&e, req.seat, &req.options[idx]));
                trail.step(&e, &req, idx);
                e.choose(idx);
            }
            Step::Over(vps) => {
                // A game ends for a legal reason only: one Methuselah (or
                // none) left, or the time limit (a flow left on the
                // game-over placeholder would end it silently).
                assert!(
                    e.state.living() <= 1 || e.state.turn >= TURN_LIMIT,
                    "game over with {} Methuselahs at turn {} (seed {seed})",
                    e.state.living(),
                    e.state.turn
                );
                copies_conserved(&e, &dealt, &mut last, true, seed);
                return Played {
                    e,
                    vps,
                    chosen,
                    trail,
                };
            }
        }
    }
}

/// Plays `seeds` seeds (replaying every `REPLAY_EVERY`th), then checks the
/// games against the golden file of `name`: the decision kinds taken, and
/// whether a game ended before the time limit with an oust and a full VP
/// awarded.
fn fuzz(name: &str, seeds: u64, start: &dyn Fn(u64) -> Engine) -> (HashSet<String>, bool) {
    let (mut seen, mut ousted) = (HashSet::new(), false);
    // `SEED_FROM=n` starts the sweep at seed n (debugging one failure).
    let from = std::env::var("SEED_FROM").ok().and_then(|v| v.parse().ok());
    let mut lines = Vec::new();
    let sweep = std::env::var("FUZZ_SEEDS")
        .ok()
        .and_then(|v| v.parse().ok());
    let seeds = sweep.unwrap_or(seeds);
    for seed in from.unwrap_or(0)..seeds {
        let Played {
            e: played,
            vps,
            chosen,
            trail,
        } = play(seed, start);
        seen.extend(chosen.into_iter().map(spell));
        lines.push(trail.line(seed, &played, &vps));
        ousted |= played.state.seats.iter().any(|s| s.ousted) && vps.iter().any(|&v| v >= 2);
        let hash = played.state.hash();
        if seed % REPLAY_EVERY != 0 {
            continue;
        }

        // Replay: the recorded decision log reproduces the identical game.
        let mut replayed = start(seed);
        let mut cursor = 0;
        let replayed_vps = loop {
            match replayed.run() {
                Step::Decision(_) => {
                    replayed.choose(played.log[cursor]);
                    cursor += 1;
                }
                Step::Over(v) => break v,
            }
        };
        assert_eq!(
            cursor,
            played.log.len(),
            "log length mismatch (seed {seed})"
        );
        assert_eq!(replayed_vps, vps, "VP mismatch on replay (seed {seed})");
        assert_eq!(
            replayed.state.hash(),
            hash,
            "state hash mismatch on replay (seed {seed})"
        );
    }
    if from.is_none() {
        golden(name, &lines, sweep.is_none());
    }
    (seen, ousted)
}

// ---- Golden replays (DESIGN §7.3) ---------------------------------------------

/// Real decisions between two golden checkpoints.
const CHECKPOINT: usize = 64;

/// A game's golden trail: an FNV-1a fold of every real decision (seat,
/// window, option count, chosen index) and, every `CHECKPOINT` decisions,
/// of the state hash; the fold's low 32 bits are kept at each checkpoint to
/// localize a divergence.
struct Trail {
    fold: u64,
    checkpoints: Vec<u32>,
}

impl Default for Trail {
    fn default() -> Self {
        Trail {
            fold: 0xcbf2_9ce4_8422_2325,
            checkpoints: Vec::new(),
        }
    }
}

impl std::hash::Hasher for Trail {
    fn finish(&self) -> u64 {
        self.fold
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.fold = (self.fold ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

impl Trail {
    /// Before `e` takes option `idx` of `req`. The window by name: a label
    /// added elsewhere in the enum leaves the trail as it was.
    fn step(&mut self, e: &Engine, req: &DecisionRequest, idx: usize) {
        use std::hash::Hasher;
        // The label's `Debug` name, streamed (no allocation).
        struct Name<'a>(&'a mut Trail);
        impl std::fmt::Write for Name<'_> {
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                self.0.write(s.as_bytes());
                Ok(())
            }
        }
        std::fmt::Write::write_fmt(&mut Name(self), format_args!("{:?}", req.window)).unwrap();
        self.write_u64(req.seat.index() as u64);
        self.write_u64(req.options.len() as u64);
        self.write_u64(idx as u64);
        if (e.log.len() + 1).is_multiple_of(CHECKPOINT) {
            self.write_u64(e.state.hash());
            self.checkpoints.push(self.fold as u32);
        }
    }

    /// `seed decisions turn vps final-hash : checkpoints…` (VPs in half
    /// points).
    fn line(&self, seed: u64, e: &Engine, vps: &[u32]) -> String {
        let vps: Vec<String> = vps.iter().map(u32::to_string).collect();
        let mut line = format!(
            "{seed} {} {} {} {:016x} :",
            e.log.len(),
            e.state.turn,
            vps.join(","),
            e.state.hash()
        );
        for c in &self.checkpoints {
            line.push_str(&format!(" {c:08x}"));
        }
        line
    }
}

/// Compare a sweep's lines with `tests/golden/<name>.txt` (the seeds both
/// have; all of them unless `FUZZ_SEEDS` changed the count, `full`), or
/// rewrite it under `GOLDEN_BLESS=1`.
fn golden(name: &str, lines: &[String], full: bool) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden");
    let path = format!("{dir}/{name}.txt");
    let header =
        format!("# seed decisions turn vps final-hash : a checkpoint every {CHECKPOINT} decisions");
    let bless = "GOLDEN_BLESS=1 cargo test --profile ci --test fuzz";
    if std::env::var("GOLDEN_BLESS").is_ok() {
        assert!(
            full,
            "{name}: FUZZ_SEEDS set; bless the fixture's own seeds"
        );
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(&path, format!("{header}\n{}\n", lines.join("\n"))).unwrap();
        return;
    }
    let Ok(text) = std::fs::read_to_string(&path) else {
        panic!(
            "{name}: no golden file {path}; if the games are meant to change, re-bless: {bless}"
        );
    };
    let mut want = text.lines();
    assert_eq!(
        want.next(),
        Some(header.as_str()),
        "{name}: golden header differs (checkpoint interval changed?); re-bless: {bless}"
    );
    let want: Vec<&str> = want.collect();
    let differ = lines.iter().zip(&want).filter(|(g, w)| g != w).count();
    if let Some((got, want)) = lines.iter().zip(&want).find(|(g, w)| g != w) {
        let seed = got.split(' ').next().unwrap();
        let cps = |l: &str| -> Vec<String> {
            let tail = l.split(" :").nth(1).unwrap_or("");
            tail.split_whitespace().map(String::from).collect()
        };
        let (g, w) = (cps(got), cps(want));
        let first = g.iter().zip(&w).take_while(|(a, b)| a == b).count();
        let range = if first < g.len().min(w.len()) {
            format!("{}..={}", first * CHECKPOINT + 1, (first + 1) * CHECKPOINT)
        } else {
            format!("{}.. (after the last checkpoint)", first * CHECKPOINT + 1)
        };
        panic!(
            "{name}: golden replay diverges at seed {seed} ({differ} of {} seeds differ), \
             first in decisions {range}\n  \
             golden: {}\n  now:    {}\n\
             inspect: FUZZ_TRACE=1 SEED_FROM={seed} cargo test --profile ci --test fuzz {name}\n\
             if the change is intended, re-bless: {bless}",
            lines.len(),
            want.split(" :").next().unwrap(),
            got.split(" :").next().unwrap(),
        );
    }
    assert!(
        !full || lines.len() == want.len(),
        "{name}: {} games, golden has {}; re-bless: {bless}",
        lines.len(),
        want.len()
    );
}

/// Coverage: random play must actually take every listed kind of decision,
/// or the invariants are not being exercised there.
fn covers(seen: &HashSet<String>, kinds: &[&str]) {
    let missing: Vec<&&str> = kinds.iter().filter(|k| !seen.contains(**k)).collect();
    let mut taken: Vec<&String> = seen.iter().collect();
    taken.sort();
    assert!(
        missing.is_empty(),
        "decision kinds {missing:?} never taken by random play; taken: {taken:?}"
    );
}

/// A random table: `table` over `slice` (its vampires covering `extra`
/// Disciplines too), then `setup`; random play must take the first `plays`
/// slice cards' plays and every `covers` kind.
struct Fixture {
    slice: &'static [&'static str],
    extra: &'static [&'static str],
    plays: usize,
    covers: &'static [&'static str],
    /// Remove torpid twins (`untwin`).
    untwin: bool,
    setup: fn(&mut Engine, u64),
    /// At least one game must end before the time limit with an oust and
    /// a full VP awarded.
    ousts: bool,
    /// Seeds played (`SEEDS`).
    seeds: u64,
}

const BASE: Fixture = Fixture {
    slice: &[],
    extra: &[],
    plays: 0,
    covers: &[],
    untwin: false,
    setup: |_, _| {},
    ousts: false,
    seeds: 100,
};

fn run(name: &str, f: &Fixture) {
    let (seen, ousted) = fuzz(name, f.seeds, &|seed| {
        let mut e = table(seed, f.slice, f.extra);
        if f.untwin {
            untwin(&mut e);
        }
        (f.setup)(&mut e, seed);
        e
    });
    let plays: Vec<String> = f.slice[..f.plays]
        .iter()
        .map(|n| format!("play:{n}"))
        .collect();
    let kinds: Vec<&str> = plays
        .iter()
        .map(String::as_str)
        .chain(f.covers.iter().copied())
        .collect();
    covers(&seen, &kinds);
    assert!(
        !f.ousts || ousted,
        "no oust in {} mid-game random games",
        f.seeds
    );
}

/// Whether a vampire is nowhere on the table yet (no contest to stage).
fn absent(e: &Engine, card: u16) -> bool {
    !e.state.minions.iter().any(|m| m.crypt_card() == Some(card))
        && !e
            .state
            .seats
            .iter()
            .any(|s| s.crypt.contains(&card) || s.uncontrolled.iter().any(|u| u.card == card))
}

/// One vampire per seat among the absent ones `of`, at up to `blood`.
fn per_seat(e: &mut Engine, of: impl Fn(&CryptCard) -> bool, blood: u8) {
    let cards: Vec<u16> = CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| of(c) && absent(e, *i as u16))
        .map(|(i, _)| i as u16)
        .take(4)
        .collect();
    for (s, &card) in cards.iter().enumerate() {
        let blood = CRYPT[card as usize].capacity.min(blood);
        e.add_minion(SeatId(s as u8), card, blood, false);
    }
}

/// The named vampires, where absent, dealt around the table at 4 blood.
/// Appolonius, base (`false`) or advanced (`true`): his CRYPT index.
fn appolonius(adv: bool) -> u16 {
    (0..zvtes::crypt::CRYPT.len() as u16)
        .find(|&c| {
            let v = &zvtes::crypt::CRYPT[c as usize];
            v.name == "Appolonius" && v.adv == adv
        })
        .expect("Appolonius")
}

fn named(e: &mut Engine, names: &[&str]) {
    for (s, name) in names.iter().enumerate() {
        let card = find_card(name);
        if absent(e, card) {
            e.add_minion(SeatId((s % 4) as u8), card, 4, false);
        }
    }
}

/// One #[test] per fixture (run in parallel). The attribute shares its line
/// with the `fn`: scripts/rulings_v5.py reads a lone `#[test]` line as a
/// test function at column 0.
macro_rules! fuzz_tests {
    ($($(#[$doc:meta])* $name:ident: $fixture:expr;)*) => {$(
        $(#[$doc])*
        #[test] fn $name() {
            run(stringify!($name), &$fixture);
        }
    )*};
}

#[test]
fn random_games_from_the_opening_terminate_hold_invariants_and_replay() {
    let (seen, _) = fuzz(
        "random_games_from_the_opening_terminate_hold_invariants_and_replay",
        SEEDS,
        &opening,
    );
    covers(
        &seen,
        &[
            "pass",
            "burn_edge",
            "bleed",
            "hunt",
            "declare_block",
            "transfer",
            "withdraw",
            "draw_crypt",
            "influence_out",
            "discard",
        ],
    );
}

/// Two seats: VEKN's two-player variant (`Format::TwoPlayer`) from the
/// opening, the precons dealt.
#[test]
fn random_two_player_games_from_the_opening_terminate_hold_invariants_and_replay() {
    let (seen, _) = fuzz(
        "random_two_player_games_from_the_opening_terminate_hold_invariants_and_replay",
        SEEDS,
        &|seed| Engine::new(seed, 2),
    );
    covers(
        &seen,
        &[
            "pass",
            "bleed",
            "hunt",
            "transfer",
            "influence_out",
            "pay_contest",
        ],
    );
}

fuzz_tests! {
    /// The mid-game position (`table`): torpor, referendums, contests, card
    /// plays and ousts.
    random_games_from_midgame_reach_torpor_referendums_contests_plays_and_ousts: Fixture {
        slice: SLICE,
        covers: &[
            "leave_torpor",
            "rescue",
            "pay_rescue",
            "diablerie",
            "cast_votes",
            "cast_edge",
            "pay_contest",
            "yield_contest",
            "set_x",
            "play:Villein",
            "play:Sudden Reversal",
            "play:Bonding",
            "play:Earth Control",
            "play:Conditioning",
            "play:Consign to Oblivion",
            "play:Form of the Cobra",
            "hand_strike",
            "use_maneuver",
            "use_press",
            "continue_action",
            "play:Flash",
            "play:Torn Signpost",
            "play:Roundhouse",
            "play:Body Flare",
            "play:Theft of Vitae",
            "play:Majesty",
            "play:Side Strike",
            "play:Blur",
            "play:Immortal Grapple",
            "play:Hidden Strength",
            "play:Soak",
            "play:Indomitability",
            "play:Claws of the Dead",
            "play:Taste of Vitae",
            "play:Telepathic Tracking",
            "play:Cats' Guidance",
            "play:Form of Mist",
            "play:Eyes of Argus",
            "play:Deflection",
            // ("play:Archon Investigation" needs a bleed of 4 against a seat
            // with 3 pool: taken at 400 seeds only; scripted in tests/bleed.rs.)
            "play:Homunculus",
            "ability:Homunculus",
            "allocate_to",
            "choose_seat",
            "cast_card_vote",
            "burn_for_vote",
            "play:Kine Resources Contested",
            "play:Conservative Agitation",
            "play:Ancilla Empowerment",
            "play:Anarchist Uprising",
            "play:Domain Challenge",
            "play:Neonate Breach",
            "play:Bewitching Oration",
            "play:Scalpel Tongue",
            "choose_minion",
            "choose_clan",
            "play:Reckless Agitation",
            "play:Camarilla's Iron Fist",
            "play:Parity Shift",
            "play:Consanguineous Boon",
            "play:Anarch Salon",
            "play:Banishment",
            "play:Expulsion",
            "play:Ominous Chorus",
            "play:Perfect Paragon",
            "play:Old Friends",
            "play:Party Out Of Bounds",
            "play:Protected District",
            "play:Scorn of Adonis",
            "play:Telepathic Vote Counting",
            "play:Delaying Tactics",
            // (Closed Session needs a prince or justicar in the polling
            // step before anyone votes — random play votes early, and the
            // play comes and goes as the slice reshuffles hands: scripted in
            // tests/politics.rs.)
            "play:Voter Captivation",
            "play:Anarch Revolt",
            "ability:Anarch Revolt",
            "play:Guardian Angel",
            "ability:Guardian Angel",
            "ability:Saku Pihlajamäki",
            "ability:Aisha az-Zahra",
            "ability:Alexa Draper",
            // (Yewon Ong's vote needs a card requiring Oblivion in hand at a
            // polling step: it comes and goes with the fixture; scripted in
            // tests/vampires.rs.)
            "ability:Larissa Moreira",
            "ability:Kalinda",
            "ability:Alicia Cortez",
            "ability:Aline Gädeke",
            // (Ilonka needs a successful bleed at her Methuselah while she
            // is ready: taken at 600 seeds only since slice 12b reshuffled
            // the fixture; scripted in tests/vampires.rs.)
            "play:Creeping Sabotage",
            "ability:Creeping Sabotage",
            "play:Blood Doll",
            "ability:Blood Doll",
            "play:Vessel",
            "ability:Vessel",
            "burn_in_play",
            "play:Papillon",
            "ability:Papillon",
            "play:Carfax Abbey",
            "ability:Carfax Abbey",
            "play:Fame",
            "play:Tension in the Ranks",
            "ability:Tension in the Ranks",
            "play:Smiling Jack, The Anarch",
            "ability:Smiling Jack, The Anarch",
            "play:Dreams of the Sphinx",
            "ability:Dreams of the Sphinx",
            "play:Barrens, The",
            "ability:Barrens, The",
            "play:Coven, The",
            "ability:Coven, The",
            "play:Constant Revolution",
            "ability:Constant Revolution",
            "play:Elder Library",
            "play:Visit from the Capuchin",
            "play:Wake with Evening's Freshness",
            "play:Mirror Walk",
            "play:Sport Bike",
            "play:Unlicensed Taxicab",
            // (Treasured Samadji needs a Ravnos to equip and its dodge a
            // Ravnos bearer in a combat — rare in random hands: scripted in
            // tests/equipment.rs.)
            "play:Codex of the Edenic Groundskeepers",
            "ability:Codex of the Edenic Groundskeepers",
            "play:Stolen Police Cruiser",
            "ability:Stolen Police Cruiser",
            "play:Bowl of Convergence",
            "play:.44 Magnum",
            "ability:.44 Magnum",
            "play:AK-47",
            "ability:AK-47",
            "play:Assault Rifle",
            // (Assault Rifle's strike comes and goes as the slices reshuffle
            // hands and combats: no key.)
            "play:Flamethrower",
            // (Flamethrower strikes and the melee weapons' — Femur of
            // Toomler, Kali's Fang and Righteous Blade need their clan to
            // equip — are rare in random combats and come and go as the
            // slice reshuffles hands; Ivory Bow takes the aggravated ranged
            // kind, and the melee strikes are scripted in tests/weapons.rs.)
            "play:Ivory Bow",
            // (ability:Ivory Bow: starved once political cards alike in hand
            // became one vote choice; scripted in tests/vampires.rs.)
            "play:Femur of Toomler",
            "play:Kali's Fang",
            "play:Sniper Rifle",
            // (ability:Sniper Rifle: starved once copies alike in hand
            // became one choice; scripted in tests/weapons.rs.)
            "play:Kevlar Vest",
            "ability:Kevlar Vest",
            "play:Righteous Blade",
            "play:Magic of the Smith",
            "find_card",
            "play:Vast Wealth",
            "ability:Vast Wealth",
            // play:Black Market Cache: starved once 9c-ii vampires joined the
            // random stream; scripted in tests/draws.rs.
            // (Shilmulo Tarot needs a Ravnos to equip, and its unlock move a
            // Ravnos bearer living until its unlock phase — rare at random
            // tables, the play coming and going as printed vampire text
            // shifts the games: scripted in tests/draws.rs.)
            // draw_stored: past 300 seeds since the block changes of
            // rules-qa 48.5/48.6 (reached within 600); tests/draws.rs covers
            // it.
            "play:Heart of Nizchetus",
            "ability:Heart of Nizchetus",
            "bottom",
            "equip_from",
            "take_equipment",
            // (Smiling Jack's pool unit elides when it is the only source and
            // is seldom picked against several vampires: no key.)
            "burn_blood_from",
            // Slice 9d: printed actions. (Sakhar and Dominica, one copy
            // each in the Salubri crypt, are seldom in play at random
            // tables; Elen Kamjian's mandatory bleed is her only option and
            // elides: scripted in tests/vampires.rs.)
            // (ability:Seraphina: starved once the rescue's split moved to
            // its announcement; scripted in tests/allies.rs and
            // tests/vampires.rs.)
            // Lenelle's ability: no longer reached within 600 seeds since the
            // reshuffle of rules-qa 47.2; vampires.rs covers it.
            "ability:Theo Bell",
            // (Anarch Revolt's and Fame's riders are decisions only when
            // ordered against another unlock effect; alone they elide, so
            // no key.)
        ],
        ousts: true,
        seeds: 300,
        ..BASE
    };
    /// Slice 10: random tables whose libraries are the allies and retainers.
    random_games_with_allies_and_retainers_hold_invariants_and_replay: Fixture {
        slice: ALLY_SLICE,
        covers: &[
            // (Feral Hound needs a Ravnos to employ it, City Star Taxi's
            // ability a Ravnos acting beside it, Double Deuce's rider a
            // hurt Double Deuce, The Vozhd of Gravesend's cancel an
            // opposing strike card in its combat — rare at these tables:
            // scripted in tests/allies.rs.)
            "play:47th Street Royals",
            "ability:47th Street Royals",
            "play:City Star Taxi",
            "play:Dog Pack",
            "play:Double Deuce",
            "play:Murder of Crows",
            "play:Political Ally",
            "play:Raptor",
            "play:Raven Spy",
            "play:Screamer",
            "ability:Screamer",
            "play:Szlachta Bodyguard",
            "ability:Szlachta Bodyguard",
            "play:Szlachta Assistant",
            "ability:Szlachta Assistant",
            "play:Vozhd of Sofia, The",
            "ability:Vozhd of Sofia, The",
            "play:Vozhd of Szczecin, The",
            "ability:Vozhd of Szczecin, The",
            "play:War Ghoul",
            "ability:War Ghoul",
            "burn_in_play",
            "play:Bone Shambler",
            "ability:Bone Shambler",
            "play:Crypt's Sons",
            "ability:Crypt's Sons",
            "play:Underbridge Stray",
            "ability:Underbridge Stray",
            "play:Vagabond Mystic",
            "ability:Vagabond Mystic",
            "play:Aggressive Corpse",
            "ability:Aggressive Corpse",
            "play:Entrancement",
            "play:Far Mastery",
            "play:Invigorate",
            "play:Libertas",
            "play:Slaughtering the Herd",
            "ability:Slaughtering the Herd",
            "play:Visions of Gehenna",
            "play:Rotting Behemoth",
            "play:Vozhd of Gravesend, The",
            "play:Vozhd of Juiz de Fora, The",
            "play:Spectral Servitor",
            "rider:Spectral Servitor",
            "play:Owl Companion",
        ],
        // City Star Taxi's play first comes between seeds 400 and 500 since
        // the Become Anarch action (slice 16k).
        seeds: 500,
        ..BASE
    };
    /// Slice 11a: random tables whose libraries are the pre-built decks' cards
    /// built from the existing vocabulary.
    random_games_with_precon_cards_hold_invariants_and_replay: Fixture {
        slice: PRECON_SLICE,
        extra: &["OBF"],
        covers: &[
            "play:Govern the Unaligned",
            "play:Enchant Kindred",
            "play:Intimidation",
            "play:Heart of the City",
            "play:Lost in Crowds",
            "play:Swallowed by the Night",
            "play:Resist Earth's Grasp",
            "play:Aire of Elation",
            "play:Foreshadowing Destruction",
            "play:On the Qui Vive",
            "play:Shadow Sentinel",
            "play:Guard Dogs",
            "play:Spirit's Touch",
            "play:Instinctive Reaction",
            "play:Precognition",
            "use_prevent",
            "play:Second Tradition: Domain",
            "play:Deep Ecology",
            "play:One With the Land",
            "play:Form of the Bat",
            "play:Truth in Darkness",
            "unlock_for_blood",
            "play:Organized Resistance",
            "play:Eyes of the Wild",
            // (Eyes of the Wild's burn needs its Gangrel attempting against
            // more stealth with blood left — rare at these tables:
            // scripted in tests/precon.rs.)
            "play:Night Terrors",
            "play:Stygian Shroud",
            "play:Forced Confessional",
            "play:True Love's Face",
            "play:Where the Veil Thins",
            "play:Faceless Night",
            "pay_to_cancel",
            "pay_block_toll",
            "play:Direct Intervention",
            "play:Wash",
            "play:Telepathic Misdirection",
            "play:My Enemy's Enemy",
            "play:Enhanced Senses",
            "play:Earth Meld",
            "play:Pursuit",
            "play:Arms of Ahriman",
            "play:Quickness",
            "play:Rego Motum",
            "play:Life in the City",
            "play:Misdirection",
            "play:Information Highway",
            // (Thing needs a Gangrel acting and one in its uncontrolled
            // region — rare at these tables: scripted in tests/precon.rs.)
            "play:Monkey Wrench",
            "play:Bait and Switch",
            "play:Protection Racket",
            "play:Warrens, The",
            "play:Apportation",
            "play:Walk of Flame",
            "play:Labyrinth, The",
            "ability:Labyrinth, The",
            "play:Elysian Fields",
            "ability:Elysian Fields",
            // (Backways competes with the other masters for the master
            // phase action — starved at these tables since 11d; the same
            // macro as The Labyrinth, scripted in tests/precon.rs.)
            // (Backways' lock needs its controller's acting Gangrel under a
            // block attempt — rare at these tables; the same macro as The
            // Labyrinth, scripted in tests/precon.rs.)
            "play:Opium Den",
            "ability:Opium Den",
            "play:Market Square",
            "ability:Market Square",
            "set_x",
        ],
        seeds: 200,
        ..BASE
    };
    /// Slices 11e and 11ab: title cards, vote locations and archetypes, the
    /// pool's last political actions, with political actions to vote on and
    /// a few strikes to fight with; an Independent
    /// Salubri per seat for Saulot's Guiding Wisdom and Valerius to steal
    /// princes' titles.
    random_games_with_titles_and_political_actions_hold_invariants_and_replay: Fixture {
        slice: &[
            "Toreador Justicar",
            "Saulot's Guiding Wisdom",
            "Rebel",
            "Perfectionist",
            "Saulot's Avenging Fist",
            "Saulot's Healing Touch",
            "Amici Noctis",
            "Elysium: The Palace of Versailles",
            "Power Structure",
            "Ventrue Headquarters",
            "Anarchist Uprising",
            "Kine Resources Contested",
            "Consanguineous Boon",
            "Bewitching Oration",
            "Roundhouse",
            "Soak",
            "Archon",
            "Disputed Territory",
            "Malkavian Justicar",
            "Raising the Portcullis",
            "Revolutionary Council",
            "Mobile HQ, Operation Antigen",
            "Parthenon, The",
            "Living Manse",
            ".44 Magnum",
            "Villein",
        ],
        covers: &[
            "play:Toreador Justicar",
            "play:Saulot's Guiding Wisdom",
            "ability:Saulot's Guiding Wisdom",
            "play:Rebel",
            "play:Elysium: The Palace of Versailles",
            "ability:Elysium: The Palace of Versailles",
            "play:Power Structure",
            "ability:Power Structure",
            "play:Ventrue Headquarters",
            "ability:Ventrue Headquarters",
            "ability:Valerius",
            "play:Perfectionist",
            "ability:Perfectionist",
            "play:Saulot's Avenging Fist",
            "ability:Saulot's Avenging Fist",
            "play:Saulot's Healing Touch",
            "ability:Saulot's Healing Touch",
            "play:Amici Noctis",
            "choose_minion",
            "pay_contest",
            "yield_contest",
            "play:Archon",
            "play:Disputed Territory",
            "play:Malkavian Justicar",
            "play:Raising the Portcullis",
            "play:Revolutionary Council",
            "ability:Archon",
            "ability:Raising the Portcullis",
            "choose_card",
            "allocate_to_card",
        ],
        setup: |e, seed| {
            per_seat(
                e,
                |c| c.clan == "Salubri" && c.sect == Some(zvtes::crypt::Sect::Independent),
                3,
            );
            let valerius = find_card("Valerius");
            if absent(e, valerius) {
                e.add_minion(SeatId((seed % 4) as u8), valerius, 4, false);
            }
        },
        untwin: true,
        seeds: 300,
        ..BASE
    };
    /// Slice 11f: the combat staples, with weapons, dodges, additional strikes
    /// and Fortitude prevention to meet them; a Banu Haqim of Blood Sorcery per
    /// seat for Hunger of Marduk.
    random_games_with_combat_staples_hold_invariants_and_replay: Fixture {
        slice: &[
            "Blood Rage",
            "Soul Burn",
            "Carrion Crows",
            "Hunger of Marduk",
            "Bear's Skin",
            "Dust Up",
            "Form of the Wolf",
            "Rolling with the Punches",
            "Sculpt the Flesh",
            "Touch of Oblivion",
            "Pass Through Shadow",
            "Slam",
            "Aid from Bats",
            "Pack Alpha",
            "Raven Spy",
            "Voracious Vermin",
            "Weighted Walking Stick",
            ".44 Magnum",
            "Side Strike",
            "Quickness",
            "Majesty",
            "Soak",
            "Hidden Strength",
            "Roundhouse",
        ],
        covers: &[
            "play:Blood Rage",
            "play:Soul Burn",
            "play:Carrion Crows",
            "play:Hunger of Marduk",
            "round_strike",
            "play:Bear's Skin",
            "play:Dust Up",
            "play:Form of the Wolf",
            "play:Rolling with the Punches",
            "play:Sculpt the Flesh",
            "play:Touch of Oblivion",
            "play:Pass Through Shadow",
            "play:Slam",
            "play:Aid from Bats",
            "play:Pack Alpha",
            "play:Voracious Vermin",
            "play:Weighted Walking Stick",
            // (The committed strike is the lone option at its strike
            // choice, elided: scripted in tests/precon.rs.)
            "employ_from_hand",
        ],
        setup: |e, _| {
            per_seat(
                e,
                |c| {
                    c.clan == "Banu Haqim"
                        && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("tha"))
                },
                4,
            )
        },
        ..BASE
    };
    /// Slices 11k, 11l and 11n: action modifiers and reactions, the cards
    /// remembering a Methuselah and Eagle's Sight, with blocks to modify and
    /// stealth past, bleeds to react to and redirect, actions directed at
    /// others and cards to cancel; a Salubri of Dominate per seat for
    /// Unleashing the Bestial Soul.
    random_games_with_action_modifiers_hold_invariants_and_replay: Fixture {
        slice: &[
            "Cloak the Gathering",
            "Veil the Legions",
            "Daring the Dawn",
            "Freak Drive",
            "Seduction",
            "Sleeping Mind, The",
            "Unleashing the Bestial Soul",
            "Visions of Zapathasura",
            "Change of Target",
            "Shadow Cloak",
            "Spying Mission",
            "Shadow Cast",
            "Eagle's Sight",
            "Form of Mist",
            "Deflection",
            "Enhanced Senses",
            "Direct Intervention",
            "Roundhouse",
        ],
        extra: &["OBF", "OBL"],
        plays: 14,
        covers: &[
            "ability:Shadow Cast",
        ],
        setup: |e, _| {
            per_seat(
                e,
                |c| {
                    c.clan == "Salubri"
                        && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("dom"))
                },
                4,
            )
        },
        ..BASE
    };
    /// Slice 11o: action cards — bleeds with riders, combat-entering actions,
    /// cards put on the acting vampire.
    random_games_with_action_cards_hold_invariants_and_replay: Fixture {
        slice: &[
            "Feast of the Soul's Secrets",
            "Fleetness",
            "Hunter's Mark",
            "Line Brawl",
            "Show of Force",
            "Shroud of Decay",
            "Rewilding",
            "Preternatural Strength",
            "Heroic Might",
            "Rutor's Hand",
            "Monstrous Form",
            "Touch of Valeren",
            "Biothaumaturgic Experiment",
            "Revelations",
            "Raven Spy",
            "Elysian Fields",
        ],
        extra: &["OBL"],
        covers: &[
            "play:Fleetness",
            "play:Hunter's Mark",
            "play:Show of Force",
            "play:Shroud of Decay",
            "play:Preternatural Strength",
            "play:Heroic Might",
            "play:Rutor's Hand",
            "play:Monstrous Form",
            "play:Touch of Valeren",
            "play:Revelations",
        ],
        ..BASE
    };
    /// Slice 11m: corruption counters and the multi-Discipline Anarch bleeds;
    /// an Anarch of Obfuscate, Presence and Protean per seat (the bleeds'
    /// requirement), Faruq Abd al-Qadir and Aniel.
    random_games_with_corruption_hold_invariants_and_replay: Fixture {
        slice: &[
            "Platinum Protocol, The",
            "Break the Bonds",
            "Enchanting Gaze",
            "Revelation of the Serpent",
            "Enhanced Senses",
            "Majesty",
        ],
        extra: &["OBF"],
        plays: 4,
        covers: &["ability:Aniel", "burn_corruption_to_fail"],
        setup: |e, _| {
            named(
                e,
                &[
                    "Jenny Silver",
                    "Crystal Lynn",
                    "Nonu Dis",
                    "Hesha Ruhadze",
                    "Faruq Abd al-Qadir",
                    "Aniel",
                ],
            )
        },
        seeds: 400,
        ..BASE
    };
    /// Slice 11p and the hunting grounds: the second wave of action cards,
    /// the clans' hunting grounds and the legacy equipment and retainers; a
    /// Hecata per seat (The Gate of Acheron, Pit of Contemplation).
    random_games_with_action_cards_wave2_and_hunting_grounds_hold_invariants_and_replay: Fixture {
        slice: &[
            "Obedient Flesh",
            "Gate of Acheron, The",
            "Pit of Contemplation",
            "Deep Song",
            "Puppet Master",
            "Academic Hunting Ground",
            "Asylum Hunting Ground",
            "Biotech Company Hunting Ground",
            "Library Hunting Ground",
            "Park Hunting Ground",
            "Political Hunting Ground",
            "Slum Hunting Ground",
            "Society Hunting Ground",
            "Temple Hunting Ground",
            "Underworld Hunting Ground",
            "Uptown Hunting Ground",
            "Warzone Hunting Ground",
            "Zoo Hunting Ground",
            "Camera Phone",
            "Laptop Computer",
            "Mr. Winthrop",
            "Revenant",
            "On the Qui Vive",
        ],
        plays: 22,
        covers: &[
            "ability:Puppet Master",
            "ability:Gate of Acheron, The",
            "ability:Pit of Contemplation",
            "ability:Academic Hunting Ground",
            // (ability:Asylum Hunting Ground: starved once the rescue's
            // split moved to its announcement; the same hunting_ground!
            // definition as Academic Hunting Ground's, scripted in
            // tests/allies.rs.)
            "ability:Biotech Company Hunting Ground",
            "ability:Library Hunting Ground",
            // (Park Hunting Ground needs a Ravnos to put it in play and
            // seldom stays in play to an unlock phase; the same macro as
            // the others: no ability key.)
            "ability:Political Hunting Ground",
            "ability:Slum Hunting Ground",
            "ability:Society Hunting Ground",
            "ability:Temple Hunting Ground",
            "ability:Underworld Hunting Ground",
            "ability:Uptown Hunting Ground",
            "ability:Warzone Hunting Ground",
            "ability:Zoo Hunting Ground",
            "ability:Camera Phone",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Gebeyehu Abdu",
                    "Monica Giovanni",
                    "Hiromitsu Asano",
                    "Alek König",
                ],
            )
        },
        seeds: 300,
        ..BASE
    };
    /// Slice 11q: the precon masters (and Childe of the Revolution, which
    /// fetches Discipline masters).
    random_games_with_masters_hold_invariants_and_replay: Fixture {
        slice: &[
            "Celerity",
            "Dominate",
            "Obfuscate",
            "Oblivion",
            "Potence",
            "Protean",
            "Childe of the Revolution",
            "Arcane Library",
            "Art Museum",
            "Ecoterrorists",
            "Wasserschloss Anif, Austria",
            "Wider View",
            "Family Gathering",
            "Powerbase: Montreal",
            "Rack, The",
            "Anarch Troublemaker",
            "Powerbase: Los Angeles",
            "Garibaldi-Meucci Museum",
            "Anarch Free Press, The",
            "Channel 10",
            "Creepshow Casino",
            "Club Illusion",
            "Haqim's Law: Retribution",
            "Pentex(TM) Subversion",
            "Haven Uncovered",
            "Chantry",
            "Toreador Grand Ball",
            "Priority Contract",
            "Frontal Assault",
            "Piper",
            "Crypt's Sons",
            "Raven Spy",
            "47th Street Royals",
            "Warsaw Station",
            "Anarch Railroad",
            "Fortune Teller Shop",
            "KRCG News Radio",
            "Carver's Meat Packing and Storage",
            "Meditative Grove",
            "Week of Nightmares",
            "Living Manse",
            "Sword of the Archangel",
        ],
        covers: &[
            "play:Oblivion",
            "play:Potence",
            "play:Childe of the Revolution",
            "take_from",
            "play:Rack, The",
            "ability:Rack, The",
            "ability:Powerbase: Montreal",
            "ability:Anarch Troublemaker",
            "ability:Wider View",
            "ability:Wasserschloss Anif, Austria",
            "ability:Powerbase: Los Angeles",
            "play:Toreador Grand Ball",
            "play:Priority Contract",
            "play:Piper",
            "ability:Frontal Assault",
            "ability:Haven Uncovered",
            "ability:Pentex(TM) Subversion",
            "ability:Club Illusion",
            "ability:Warsaw Station",
            "ability:Anarch Railroad",
            "ability:KRCG News Radio",
            "play:Carver's Meat Packing and Storage",
        ],
        ..BASE
    };
    /// Slice 11s: the second wave of combat cards.
    random_games_with_combat_cards_wave_2_hold_invariants_and_replay: Fixture {
        slice: &[
            "Blood Fury",
            "No Trace",
            "Shadow Shift",
            "Wolf Claws",
            "Rapid Change",
            "Diversion",
            "Lightning Reflexes",
            "Wind Dance",
            "Anticipation",
            "Terror Frenzy",
            "Tranquility Shield",
            "Flesh of Marble",
            "Weather Control",
            "Disarm",
            "Hide the Mind",
            "Angel's Gift",
            "Contraband",
            "Martyr's Resilience",
            "Righteous Blade",
            ".44 Magnum",
            "Owl Companion",
            "Pursuit",
            "Quickness",
            "Slam",
            "Telepathic Misdirection",
            "Meditative Grove",
        ],
        extra: &["OBF"],
        plays: 18,
        covers: &[
            "ability:Disarm",
        ],
        ..BASE
    };
    /// Slice 11t: the second wave of reactions and action modifiers.
    random_games_with_reactions_wave_2_hold_invariants_and_replay: Fixture {
        slice: &[
            "Threats",
            "Forgotten Labyrinth",
            "Go-getter",
            "Elder Impersonation",
            "Beast Meld",
            "Paths in Two Worlds",
            "Dawn Operation",
            "Dogged Pursuit",
            "Elder Intervention",
            "Forced Awakening",
            "Forced Vigilance",
            "Melange",
            "Rat's Warning",
            "Redirection",
            "Sense the Savage Way",
            "Telepathic Counter",
            "Bone Shambler",
            ".44 Magnum",
            "Lost in Crowds",
            "Precognition",
            "Form of Mist",
        ],
        extra: &["OBF"],
        plays: 16,
        covers: &[
            "withdraw_from_game",
            "ability:Dogged Pursuit",
            "ability:Melange",
        ],
        ..BASE
    };
    /// Slice 11u: the pool's actions, wave 1.
    random_games_with_pool_actions_wave_1_hold_invariants_and_replay: Fixture {
        slice: &[
            "Scouting Mission",
            "Social Charm",
            "Public Trust",
            "Restoration",
            "Fifth Tradition: Hospitality",
            "Fourth Tradition: The Accounting",
            "Umbrous Clutch",
            "Hunting the Beast",
            "Make the Misere",
            "Mind Numb",
            "Propaganda",
            "Conceal",
            "Dominate Kine",
            "Graverobbing",
            ".44 Magnum",
            "Elysian Fields",
            "Telepathic Counter",
            "Lost in Crowds",
        ],
        extra: &["OBF"],
        plays: 14,
        covers: &[
        ],
        untwin: true,
        ..BASE
    };
    /// Slice 11v: the pool's actions, wave 2.
    random_games_with_pool_actions_wave_2_hold_invariants_and_replay: Fixture {
        slice: &[
            "Army of Rats",
            "Judgment: Camarilla Segregation",
            "Tier of Souls",
            "Phantasmagoria",
            "Aranthebes, The Immortal",
            "Fee Stake: Boston",
            "Fee Stake: New York",
            "Fee Stake: Seattle",
            "Waters of Duat",
            "Split the Veil",
            "Psychophagia",
            "Khabar: Glory",
            "Open War",
            "Screamer",
            "Bone Shambler",
            "Elysian Fields",
            "Graverobbing",
        ],
        extra: &["OBF"],
        plays: 13,
        covers: &[
            "ability:Army of Rats",
            "ability:Judgment: Camarilla Segregation",
            "ability:Tier of Souls",
            "ability:Phantasmagoria",
            "ability:Aranthebes, The Immortal",
            "ability:Fee Stake: Boston",
            "ability:Split the Veil",
            "ability:Open War",
            "rider:Army of Rats",
            "rider:Judgment: Camarilla Segregation",
            "rider:Khabar: Glory",
        ],
        untwin: true,
        seeds: 250,
        ..BASE
    };
    /// Slice 11w: the pool's masters, wave 1.
    random_games_with_pool_masters_wave_1_hold_invariants_and_replay: Fixture {
        slice: &[
            "Parthenon, The",
            "Dummy Corporation",
            "Ferraille",
            "Oxford University, England",
            "Black Throne, The",
            "Kumpania",
            "Sight Beyond Sight",
            "Charisma",
            "Giant's Blood",
            "Golconda: Inner Peace",
            "Grooming the Protégé",
            "Gangrel Revel",
            "Khabar: Community, The",
            "New Carthage",
            "Mob Connections",
            "Fragment of the Book of Nod",
            "Brujah Debate",
            "Priority Contract",
            "Anarchist Uprising",
            "Double Deuce",
            "Political Ally",
        ],
        extra: &["OBF"],
        plays: 17,
        covers: &[
            "ability:Black Throne, The",
            "ability:Brujah Debate",
            "ability:Dummy Corporation",
            "ability:Ferraille",
            "ability:Fragment of the Book of Nod",
            "ability:Gangrel Revel",
            "ability:Khabar: Community, The",
            "ability:Kumpania",
            "ability:Mob Connections",
            "ability:Oxford University, England",
            "ability:Parthenon, The",
            "rider:Black Throne, The",
        ],
        untwin: true,
        seeds: 200,
        ..BASE
    };
    /// Slice 11x: the pool's masters, wave 2.
    random_games_with_pool_masters_wave_2_hold_invariants_and_replay: Fixture {
        slice: &[
            "WMRH Talk Radio",
            "Rumor Mill, Tabloid Newspaper, The",
            "Monster",
            "Dabbler",
            "Dead Pool",
            "Alamut",
            "Cappadocian Crypt",
            "Powerbase: Munich",
            "Twilight Camp",
            "Mithraeum, London, The",
            "Mithraic Cultist",
            "Anarchist Uprising",
            "Make the Misere",
            "Umbrous Clutch",
            "Lost in Crowds",
        ],
        extra: &["OBF", "OBL"],
        plays: 11,
        covers: &[
            "ability:Alamut",
            "ability:Cappadocian Crypt",
            "ability:Dead Pool",
            "ability:Mithraeum, London, The",
            "ability:Monster",
            "ability:Powerbase: Munich",
            "ability:Rumor Mill, Tabloid Newspaper, The",
            "ability:Twilight Camp",
            "ability:WMRH Talk Radio",
        ],
        untwin: true,
        seeds: 300,
        ..BASE
    };
    /// Slice 11y: the pool's masters, wave 3.
    random_games_with_pool_masters_wave_3_hold_invariants_and_replay: Fixture {
        slice: &[
            "Secure Haven",
            "Yoruba Shrine",
            "Cave of Apples",
            "British Museum, London, The",
            "Banishment",
            "Anarchist Uprising",
            "Umbrous Clutch",
            "Open War",
            "Living Manse",
            "Kevlar Vest",
            "Giant's Blood",
            "Villein",
            "Fleshforge Chamber",
            "Szlachta Assistant",
            "War Ghoul",
        ],
        extra: &["OBF", "OBL"],
        plays: 4,
        covers: &[
            "play:Fleshforge Chamber",
            "ability:Fleshforge Chamber",
            "ability:British Museum, London, The",
            "ability:Cave of Apples",
            "ability:Yoruba Shrine",
        ],
        untwin: true,
        ..BASE
    };
    /// Slice 11z: the cost-reduction masters.
    random_games_with_cost_reductions_hold_invariants_and_replay: Fixture {
        slice: &[
            "Line, The",
            "Shard, London, The",
            "Ravnos Cache",
            "Ravnos Carnival",
            "Propaganda",
            "Public Trust",
            ".44 Magnum",
            "Kevlar Vest",
            "Parthenon, The",
            "Direct Intervention",
            "Wash",
            "Political Ally",
        ],
        extra: &["OBF"],
        plays: 4,
        covers: &[
            "ability:Line, The",
            "ability:Shard, London, The",
            "ability:Ravnos Cache",
            "rider:Line, The",
            "rider:Shard, London, The",
            "rider:Ravnos Cache",
            "rider:Ravnos Carnival",
        ],
        untwin: true,
        // Ravnos Cache's rider first comes near seed 120.
        seeds: 240,
        ..BASE
    };
    /// Slice 11aa: the pool's last allies and equipment, Mobile HQ and Minion
    /// Tap.
    random_games_with_allies_and_equipment_hold_invariants_and_replay: Fixture {
        slice: &[
            "ECTU Operative",
            "Michael Harris, the Incinerator",
            "Valerie Diaz, Antigen Commander",
            "Mobile HQ, Operation Antigen",
            "Enhanced Coagulant",
            "Flaming Candle",
            "Ritual Goblet",
            "Minion Tap",
            "Villein",
            "Line Brawl",
            ".44 Magnum",
            "Soak",
            "Side Strike",
            "Conditioning",
        ],
        extra: &["OBF"],
        plays: 8,
        covers: &[
            "ability:ECTU Operative",
            "ability:Michael Harris, the Incinerator",
            "ability:Valerie Diaz, Antigen Commander",
            "ability:Mobile HQ, Operation Antigen",
            "ability:Enhanced Coagulant",
            "ability:Flaming Candle",
            // Ritual Goblet's unlock starves since a blocker left for
            // another is not offered again (rules-qa 48.6);
            // tests/precon.rs ritual_goblet_discounts_a_rescue_and_unlocks_once_a_turn
            // scripts it.
        ],
        untwin: true,
        seeds: 300,
        ..BASE
    };
    /// Slices 11ac–11ae: the pool's events.
    random_games_with_events_hold_invariants_and_replay: Fixture {
        slice: &[
            "Narrow Minds",
            "Torpid Blood",
            "Blood Weakens",
            "Thirst",
            "Fueled by Heart's Blood",
            "Blood Trade",
            "Nightmares upon Nightmares",
            "Restricted Vitae",
            "FBI Special Affairs Division",
            "Uncoiling, The",
            "Slow Withering, The",
            "Wormwood",
            "New Inquisition, The",
            "Rise of the Nephtali",
            "Absimiliard's Army",
            "Conquest of Humanity",
            "Recalled to the Founder",
            "Fall of the Camarilla",
            "Fall of the Sabbat",
            "Fourth Cycle, The",
            "Parthenon, The",
            "Telepathic Misdirection",
            "Conditioning",
        ],
        extra: &["OBF"],
        plays: 20,
        covers: &[
            "ability:Blood Trade",
            "ability:Absimiliard's Army",
            "ability:New Inquisition, The",
            "ability:Restricted Vitae",
            "ability:Uncoiling, The",
            "ability:Rise of the Nephtali",
            "ability:Conquest of Humanity",
        ],
        untwin: true,
        seeds: 200,
        ..BASE
    };
    /// Slice 12a: the last vampires' printed text, all of them dealt at the
    /// table, with the cards their text reads (Gangrel and Hecata actions,
    /// animals, Tzimisce and Hecata allies, locations, Toreador cards,
    /// Oblivion cards, a referendum, corruption; a directed action card, a
    /// dodge, a Protean strike).
    random_games_with_the_last_vampires_hold_invariants_and_replay: Fixture {
        slice: &[
            "Rewilding",
            "Dog Pack",
            "Raven Spy",
            "Aggressive Corpse",
            "Screamer",
            "War Ghoul",
            ".44 Magnum",
            "Elder Library",
            "Living Manse",
            "Toreador Grand Ball",
            "Ancilla Empowerment",
            "Scorn of Adonis",
            "Shadow Sentinel",
            "Vagabond Mystic",
            "Platinum Protocol, The",
            "Piper",
            "Revelations",
            "Side Strike",
            "Body Flare",
        ],
        extra: &["OBF"],
        plays: 16,
        // (The passive texts — the cost reductions, Regina's blood, the
        // rescue discounts, Alexander's, Sergio's and Mithras's tolls,
        // Daria's and Djeneba's surcharges, Abraham's and Lady Scarlett's
        // clans — take no decision of their own: covered by the invariants,
        // scripted in tests/vampires.rs.)
        covers: &[
            "ability:\"Mother\" Anja Giovanni",
            "ability:Guardian, The",
            "ability:Hel-Blá",
            "ability:Keegan",
            "ability:Luciano Carvalho",
            "ability:Roger de Camden",
            "ability:Sri Sansa",
            "ability:Tommaso Sforza",
            "resolve_printed",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Abraham Mellon",
                    "Adelino Da Silva",
                    "Alexander Silverson",
                    "Anne-Marie Bourgeois, Inconnu Recruiter",
                    "Doc Martina",
                    "Gnaeus Aemilius Augustinus",
                    "Hel-Blá",
                    "Keegan",
                    "Kuyén",
                    "Lady Scarlett Churchill",
                    "Luciano Carvalho",
                    "Máddji, Mistress of Szlachtas",
                    "Regina Blake",
                    "Roger de Camden",
                    "Sergio Bueno",
                    "Sri Sansa",
                    "Tommaso Sforza",
                    "\"Mother\" Anja Giovanni",
                    "Guardian, The",
                    "Daria",
                    "Djeneba",
                    "Marchesa Liliana",
                    "Mithras",
                    "Ragnar Nordstrom",
                ],
            )
        },
        untwin: true,
        seeds: 300,
        ..BASE
    };
    /// Slices 16a and 16b: the legacy additions for tournament decks, Life
    /// Boon's decision point before an oust reached from bleeds, costs,
    /// referendums and combat (pool from 2 to 9 per seat).
    random_games_with_legacy_additions_hold_invariants_and_replay: Fixture {
        slice: &[
            "Life Boon",
            "Life Boon",
            "Rötschreck",
            "Donnybrook",
            "Hell-for-Leather",
            "Ashur Tablets",
            "Jake Washington",
            "Ennoia's Theater",
            "Powerbase: Savannah",
            "Murmur of the False Will",
            "Saatet-ta",
            "Carlton Van Wyk",
            "Kine Resources Contested",
            "Conditioning",
            "Ivory Bow",
        ],
        covers: &[
            "play:Life Boon",
            "play:Rötschreck",
            "play:Ashur Tablets",
            "play:Jake Washington",
        ],
        ousts: true,
        seeds: 200,
        ..BASE
    };
    /// Reign of Stanislava (25th Anniversary): the deck's new cards, its
    /// five vampires dealt around the table (Inner Circle, archbishop,
    /// justicar, princes), a werewolf and a retainer to meet them.
    random_games_with_reign_of_stanislava_hold_invariants_and_replay: Fixture {
        slice: &[
            "Ancient Influence",
            "Enkil Cog",
            "Enkil Cog",
            "Fear of Mekhet",
            "Forced March",
            "Instantaneous Transformation",
            "Monastery of Shadows",
            "Permanent Vacation",
            "Political Stranglehold",
            "Reins of Power",
            "Zillah's Valley",
            "Kine Resources Contested",
            "Double Deuce",
            "Raptor",
        ],
        covers: &[
            "play:Enkil Cog",
            "ability:Enkil Cog",
            "play:Ancient Influence",
            "play:Reins of Power",
            "play:Fear of Mekhet",
            "ability:Monastery of Shadows",
            "play:Forced March",
            "play:Instantaneous Transformation",
            "play:Permanent Vacation",
            "ability:Hartmut Stover",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Stanislava",
                    "Hartmut Stover",
                    "Ingrid Rossler",
                    "Mark Decker",
                    "Xaviar",
                ],
            )
        },
        seeds: 100,
        ..BASE
    };
    /// The Endless Dance (30th Anniversary): the deck's new cards, its
    /// Toreador dealt around the table (a justicar, princes of Paris, Bogota
    /// and Gary), an equipment to confiscate, a reaction and a grapple to
    /// cancel.
    random_games_with_the_endless_dance_hold_invariants_and_replay: Fixture {
        slice: &[
            "Awe",
            "Business Pressure",
            "Camarilla Conclave",
            "Charismatic Aura",
            "Confiscation",
            "Loup",
            "Reinforcements",
            "Louvre, Paris, The",
            "Political Stranglehold",
            "Kine Resources Contested",
            "Delaying Tactics",
            "Immortal Grapple",
            ".44 Magnum",
            "Majesty",
        ],
        covers: &[
            "play:Awe",
            "play:Business Pressure",
            "play:Camarilla Conclave",
            "play:Confiscation",
            "play:Loup",
            "play:Reinforcements",
            "ability:Louvre, Paris, The",
            "ability:Camarilla Conclave",
            "ability:François Villon",
            "ability:Modius",
            "ability:Silvian Popescu",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Diana Iadanza",
                    "François Villon",
                    "Marcos Belegrad",
                    "Modius",
                    "Silvian Popescu",
                ],
            )
        },
        seeds: 100,
        ..BASE
    };
    /// Sabbat V5, the Path of Power and the Inner Voice: the deck's new
    /// cards, its titled vampires dealt around the table (cardinal, priscus,
    /// archbishops, bishops) and Lika on the path, with a political action to
    /// call. Unholy Sacrament wants a titled Sabbat vampire uncontrolled: not
    /// dealt here, its own test covers it.
    random_games_with_the_path_of_power_hold_invariants_and_replay: Fixture {
        slice: &[
            "Absolute Tyranny",
            "Black Forest Base",
            "Cardinal Benediction",
            "Catatonic Fear",
            "Cold War",
            "Día de los Muertos",
            "Dominant Personality",
            "Empires Fall",
            "Iron Glare",
            "Magnetic Authority",
            "Powerbase: Madrid",
            "Private Audience",
            "Privileged Position",
            "Regent",
            "Relentlessness",
            "Under Siege",
            "Unholy Sacrament",
            "Unthinkable Humiliation",
            "War of Ages",
            "Political Stranglehold",
        ],
        covers: &[
            "follow_path",
            "play:Absolute Tyranny",
            "play:Black Forest Base",
            "play:Cardinal Benediction",
            "play:Catatonic Fear",
            "play:Cold War",
            "play:Día de los Muertos",
            "play:Dominant Personality",
            "play:Empires Fall",
            "play:Iron Glare",
            "play:Magnetic Authority",
            "play:Powerbase: Madrid",
            "play:Private Audience",
            "play:Privileged Position",
            "play:Regent",
            "play:Relentlessness",
            "play:Under Siege",
            "play:Unthinkable Humiliation",
            "play:War of Ages",
            "ability:Black Forest Base",
            "ability:Powerbase: Madrid",
            "ability:Privileged Position",
            "ability:Regent",
            "ability:Under Siege",
            "ability:War of Ages",
            "ability:Aaradhya, The Callous Tyrant",
            "ability:Věnceslava, The Implacable",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Aaradhya, The Callous Tyrant",
                    "Damian",
                    "Khin Aye",
                    "Rexton \"Savage\" Abernathy",
                    "Üresség",
                    "Věnceslava, The Implacable",
                    "Concordia",
                    "Lika, The Purger",
                ],
            )
        },
        seeds: 100,
        ..BASE
    };
    /// Sabbat V5, the Path of Cathari: the deck's new cards, its vampires
    /// dealt around the table (a cardinal, archbishops, a bishop), with an
    /// action-ending card and a strike that unlocks against Kiss of
    /// Cathari's stun.
    random_games_with_the_path_of_cathari_hold_invariants_and_replay: Fixture {
        slice: &[
            "Enthrall",
            "Fever Pitch",
            "Forward Momentum",
            "Hedonism",
            "Hungry Coyote, The",
            "Kiss of Cathari",
            "Seeds of Terror",
            "Subversion",
            "Swift Cover",
            "Terrifying Visage",
            "Change of Target",
            "Majesty",
        ],
        covers: &[
            "play:Enthrall",
            "play:Fever Pitch",
            "play:Forward Momentum",
            "play:Hedonism",
            "play:Hungry Coyote, The",
            "play:Kiss of Cathari",
            "play:Seeds of Terror",
            "play:Subversion",
            "play:Swift Cover",
            "play:Terrifying Visage",
            "ability:Fever Pitch",
            "ability:Forward Momentum",
            "ability:Agnieszka, Tempter of Legions",
            "ability:Gostoso",
            "draw_crypt",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Aelswith, The Irresistible",
                    "Agnieszka, Tempter of Legions",
                    "Devorah",
                    "Fotini Katsikaris",
                    "Gostoso",
                    "Jürgen, The Libertine",
                    "Osvaldo Kühnemann",
                    "Verrix, Naughty Boy",
                    "Glissando",
                    "Ximena",
                ],
            )
        },
        seeds: 100,
        ..BASE
    };
    /// Sabbat V5, the Path of Caine: the deck's new cards, its bishops and
    /// warriors dealt around the table, with political actions for Yawp
    /// Court to fight and a would-end card to race Hunting the Quarry.
    random_games_with_the_path_of_caine_hold_invariants_and_replay: Fixture {
        slice: &[
            "Abbot",
            "Abuse of Power",
            "Augury of Doom",
            "Dance with the Devil",
            "Depravity",
            "Dread Mastiff",
            "Ensnare a Beast",
            "Fiendish Tongue",
            "Hunting the Quarry",
            "Pillars Fall",
            "Rage of Apedemak",
            "Sentry Signal",
            "Soulgrinder",
            "Twisted Bloodhound",
            "Wall of Filth",
            "Yawp Court",
            "Political Stranglehold",
            "Kine Resources Contested",
        ],
        covers: &[
            "play:Abbot",
            "play:Abuse of Power",
            "play:Augury of Doom",
            "play:Dance with the Devil",
            "play:Depravity",
            "play:Dread Mastiff",
            "play:Ensnare a Beast",
            "play:Fiendish Tongue",
            "play:Hunting the Quarry",
            "play:Pillars Fall",
            "play:Rage of Apedemak",
            "play:Sentry Signal",
            "play:Soulgrinder",
            "play:Twisted Bloodhound",
            "play:Wall of Filth",
            "play:Yawp Court",
            "ability:Yawp Court",
            "ability:Hunting the Quarry",
            "ability:Twisted Bloodhound",
            "ability:Wall of Filth",
            "ability:Crossbreaker",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Algirdas, The Solar Prophet",
                    "Anousha, The Maniac",
                    "Crossbreaker",
                    "Kalyani, Agent of Doom",
                    "Serhat Gunde, Combat Junkie",
                    "Przemysław, War Beast",
                    "Hekla \"Bonecrusher\" Jónasdóttir",
                    "Lawbringer",
                    "Berenguela, The Vicious",
                    "Loyiso, The Ruthless",
                ],
            )
        },
        seeds: 100,
        ..BASE
    };
    /// Sabbat V5, the Path of Death and the Soul: the deck's new cards, its
    /// zombie-raising vampires dealt around the table, and allies for
    /// Heartrender to burn.
    random_games_with_the_path_of_death_hold_invariants_and_replay: Fixture {
        slice: &[
            "Burial Site Hunting Ground",
            "Cursed Abattoir",
            "Dance of the Dead",
            "Fiorella, Empty One",
            "Freakish Conglomeration",
            "Gifts From Hereafter",
            "Glancing Blow",
            "Gravebound Drone",
            "Heartrender",
            "Morbidity",
            "Obedience",
            "Putrescent Sustenance",
            "Target Vitals",
            "Wall Street Night, Financial Newspaper",
        ],
        covers: &[
            "play:Burial Site Hunting Ground",
            "play:Cursed Abattoir",
            "play:Dance of the Dead",
            "play:Fiorella, Empty One",
            "play:Freakish Conglomeration",
            "play:Gifts From Hereafter",
            "play:Glancing Blow",
            "play:Gravebound Drone",
            "play:Heartrender",
            "play:Morbidity",
            "play:Obedience",
            "play:Putrescent Sustenance",
            "play:Target Vitals",
            "play:Wall Street Night, Financial Newspaper",
            "ability:Burial Site Hunting Ground",
            "ability:Cursed Abattoir",
            "ability:Dance of the Dead",
            "ability:Fiorella, Empty One",
            "ability:Freakish Conglomeration",
            "ability:Gravebound Drone",
            "ability:Heartrender",
            "ability:Wall Street Night, Financial Newspaper",
            "ability:Abderrahim, Death's Hand",
            "ability:Eulogio Sánchez de los Reyes",
            "ability:Huldu, The Desecrator",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Abderrahim, Death's Hand",
                    "Aitana, The Unholy",
                    "Anxo Vilela",
                    "Ashur-uballit, Son of the Abyss",
                    "El Monseñor",
                    "Eulogio Sánchez de los Reyes",
                    "Frau Schädel",
                    "Huldu, The Desecrator",
                    "Nadezhda",
                    "Sakura, The Merciless",
                    "Yurena, The Boor",
                    "Zbigniew, Hunter of Mekhet",
                ],
            );
            // Allies for Heartrender to burn.
            for s in 1..4 {
                e.add_ally(SeatId(s), zvtes::state::find_library("Double Deuce"));
            }
        },
        seeds: 200,
        ..BASE
    };
    /// Legacy cards for TWDA decks, staples and steps 1-3 (slice 16j): the
    /// cards, their vampires dealt around the table, and an Anarch Convert
    /// ready to move out in every uncontrolled region.
    random_games_with_twda_staples_hold_invariants_and_replay: Fixture {
        slice: &[
            "Bum's Rush",
            "Dragonbound",
            "Heidelberg Castle, Germany",
            "J. S. Simmons, Esq.",
            "Lam Into",
            "Mylan Horseed",
            "Scourge of the Enochians",
            "Tasha Morgan",
            // A political action card for Maila to discard.
            "Kine Resources Contested",
        ],
        covers: &[
            "play:Bum's Rush",
            "play:Dragonbound",
            "play:Heidelberg Castle, Germany",
            "play:J. S. Simmons, Esq.",
            "play:Lam Into",
            "play:Mylan Horseed",
            "play:Scourge of the Enochians",
            "play:Tasha Morgan",
            "ability:Heidelberg Castle, Germany",
            "ability:Mylan Horseed",
            "ability:Scourge of the Enochians",
            "ability:Maila",
            "ability:Gabriel Tremblay",
        ],
        setup: |e, _| {
            named(e, &["Alonso Petrodon", "Maila", "Gabriel Tremblay"]);
            let convert = find_card("Anarch Convert");
            for s in 0..4 {
                e.state.seats[s].uncontrolled.push(zvtes::state::UncontrolledSlot {
                    card: convert,
                    copy: Default::default(),
                    counters: 1,
                    banished: None,
                });
            }
        },
        seeds: 20,
        ..BASE
    };
    /// Legacy cards for TWDA decks, steps 4-9 (slice 16k): the cards and
    /// their vampires dealt around the table — the Become Anarch action,
    /// an infernal unlock, first strike, Mask of a Thousand Faces.
    random_games_with_twda_steps_4_to_9_hold_invariants_and_replay: Fixture {
        slice: &[
            "Disguised Weapon",
            "Hackerspace",
            "Mask of a Thousand Faces",
            "Patsy",
            "SchreckNET",
            "Unflinching Persistence",
            // A weapon to disguise, a referendum for the votes.
            ".44 Magnum",
            "Kine Resources Contested",
        ],
        covers: &[
            "play:Disguised Weapon",
            "play:Hackerspace",
            "play:Mask of a Thousand Faces",
            "play:Patsy",
            "play:SchreckNET",
            "play:Unflinching Persistence",
            "ability:Arishat",
            "ability:Danielle Diron",
            "ability:Scout Youngwood",
            "ability:Medic, The",
            "ability:Benjamin Rose",
            "ability:Dmitra Ilyanova",
            "become_anarch",
            "unlock_infernal",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Elimelech the Twice-Damned",
                    "Benjamin Rose",
                    "Arishat",
                    "Scout Youngwood",
                    "Dmitra Ilyanova",
                    "Karen Suadela",
                    "Reginald Moore",
                    "Brunhilde",
                    "Danielle Diron",
                    "Valkyrie",
                    "Medic, The",
                    "Jacques Rouge",
                ],
            );
        },
        seeds: 20,
        ..BASE
    };
    /// Legacy cards for TWDA decks, steps 10-15 (slice 16l): the cards and
    /// their vampires — Vampiric Disease, Trap, Conquer the Beast, the
    /// Society of Leopold, Ephor, Justicar Retribution.
    random_games_with_twda_steps_10_to_15_hold_invariants_and_replay: Fixture {
        slice: &[
            "Auspex",
            "Conquer the Beast",
            "Ephor",
            "Justicar Retribution",
            "Quick Meld",
            "Signet of King Saul, The",
            "Society of Leopold",
            "Trap",
            "Vampiric Disease",
            // A referendum and a Gangrel ally.
            "Kine Resources Contested",
            "Double Deuce",
        ],
        covers: &[
            "follow_path",
            "play:Auspex",
            "play:Conquer the Beast",
            "play:Ephor",
            "play:Justicar Retribution",
            "play:Quick Meld",
            "play:Signet of King Saul, The",
            "play:Society of Leopold",
            "play:Trap",
            "play:Vampiric Disease",
            "ability:Giovanni del Georgio",
            "ability:Mary Anne Blaire",
            "ability:Aksinya Daclau",
            "ability:Randall",
            "ability:Lodin (Olaf Holte)",
            "ability:Fakir al Sidi",
            "ability:T.J.",
            "ability:Society of Leopold",
            "burn_blood_to_avoid",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Giovanni del Georgio",
                    "Mary Anne Blaire",
                    "Karl Schrekt",
                    "Aksinya Daclau",
                    "Randall",
                    "Lodin (Olaf Holte)",
                    "Fakir al Sidi",
                    "T.J.",
                    "Matasuntha",
                    "Gerald Windham",
                    "Kevin Jackson",
                    "Inyanga",
                ],
            );
        },
        seeds: 100,
        ..BASE
    };
    /// Legacy cards for TWDA decks, steps 16-20 (slice 16m): the cards and
    /// their vampires — the Great Beast, Unleash Hell's Fury's blocker, the
    /// Erciyes Fragments, the Soul Gem, a second Jimmy Dunn to come out.
    random_games_with_twda_steps_16_to_20_hold_invariants_and_replay: Fixture {
        slice: &[
            "Call the Great Beast",
            "Changeling",
            "Condemnation: Mute",
            "Crocodile's Tongue",
            "Erciyes Fragments, The",
            "Erebus Mask",
            "I am Legion",
            "My Kin Against the World",
            "Plasmic Form",
            "Sargon Fragment, The",
            "Soul Gem of Etrius",
            "Street Cred",
            "Under My Skin",
            "Unleash Hell's Fury",
        ],
        covers: &[
            "follow_path",
            "play:Call the Great Beast",
            "play:Condemnation: Mute",
            "play:Erciyes Fragments, The",
            "play:I am Legion",
            "play:Soul Gem of Etrius",
            "play:Street Cred",
            "play:Under My Skin",
            "play:Unleash Hell's Fury",
            "ability:Lord Tremere",
            "ability:Malgorzata",
            "ability:Mistress Fanchon",
            "ability:Beast, The Leatherface of Detroit",
            "ability:Erciyes Fragments, The",
            "ability:Call the Great Beast",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Nergal",
                    "Lord Tremere",
                    "Beast, The Leatherface of Detroit",
                    "Capuchin, The",
                    "Juliet Parr",
                    "Malgorzata",
                    "Mistress Fanchon",
                    "Jimmy Dunn",
                    "Arishat",
                ],
            );
            let theo = CRYPT
                .iter()
                .position(|c| c.name == "Theo Bell" && c.group == 2)
                .expect("Theo Bell G2") as u16;
            // Not beside a G6 Theo already there: one vampire.
            if !e.state.minions.iter().any(|m| m.in_play && m.name() == "Theo Bell") {
                e.add_minion(SeatId(1), theo, 4, false);
            }
            // A second Jimmy ready to come out.
            e.state.seats[2]
                .uncontrolled
                .push(zvtes::state::UncontrolledSlot {
                    card: find_card("Jimmy Dunn"),
                    copy: Default::default(),
                    counters: 4,
                    banished: None,
                });
        },
        seeds: 300,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 2 (slice 16n): Nephandus and
    /// Tupdog, The unnamed, Lutz's Inner Circle, Goratrix, the Toreador
    /// wall, the Ventrue grinder — their cards and vampires.
    random_games_with_twda_round_2_hold_invariants_and_replay: Fixture {
        slice: &[
            "Canine Horde",
            "Contagion",
            "Corporate Hunting Ground",
            "CrimethInc.",
            "Liquidation",
            "Memory Rift",
            "Nephandus",
            "Powerbase: Barranquilla",
            "Psychic Assault",
            "Ruins of Ceoris",
            "Sense the Sin",
            "Unmasking, The",
            "Veneficti",
            "Herald of Topheth",
            // A referendum for Lutz and Marcellus.
            "Kine Resources Contested",
        ],
        covers: &[
            "play:Canine Horde",
            "play:Contagion",
            "play:CrimethInc.",
            "play:Memory Rift",
            "play:Nephandus",
            "play:Psychic Assault",
            "play:Unmasking, The",
            "ability:Goratrix",
            "ability:Alexandra",
            "ability:Owain Evans, The Wanderer",
            "ability:Tupdog",
            "ability:Victoria Ash",
            "ability:Nephandus",
            "ability:Omaya",
            "ability:Keith Moody",
            "ability:Antonio d'Erlette",
            "ability:unnamed, The",
            "ability:Contagion",
            "ability:CrimethInc.",
            "ability:Ruins of Ceoris",
            "ability:Powerbase: Barranquilla",
            "ability:Corporate Hunting Ground",
            "play:Liquidation",
            "play:Sense the Sin",
            "play:Veneficti",
            "play:Herald of Topheth",
            "rider:Contagion",
        ],
        setup: |e, seed| {
            named(
                e,
                &[
                    "Goratrix",
                    "Omaya",
                    "Alexandra",
                    "Rafael de Corazon",
                    "Lutz von Hohenzollern",
                    "Unmada",
                    "Anson",
                    "Anneke",
                    "Marcellus",
                    "Owain Evans, The Wanderer",
                    "Keith Moody",
                    // Tupdog beside Antonio (seat 3): a ready Tremere
                    // antitribu for his rush.
                    "Antonio d'Erlette",
                    "Blackhorse Tanner",
                    "Saiz",
                    "unnamed, The",
                    "Tupdog",
                    "Victoria Ash",
                    "Adana de Sforza",
                    "Neighbor John",
                    "Elisha Tucker",
                ],
            );
            // On even seeds, a [DAI] Contagion already on The unnamed: its
            // strike (one played from hand is rarely the superior one).
            if let Some(i) = (seed % 2 == 0).then_some(()).and_then(|_| {
                e
                .state
                .minions
                .iter()
                .position(|m| m.in_play && m.name() == "unnamed, The")
            }) {
                let owner = e.state.minions[i].owner;
                e.state.cards.push(zvtes::state::CardInPlay::new(
                    zvtes::state::find_library("Contagion"),
                    owner,
                    Some(MinionId(i as u32)),
                    1,
                ));
            }
        },
        seeds: 100,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 3 (slice 16o): the owner's list —
    /// Abactor and the hunters, the Celerity and Obfuscate combat cards,
    /// Concealed Weapon, Learjet, Baltimore Purge, Renewed Vigor and the
    /// blood-cursed Banu Haqim.
    random_games_with_twda_round_3_hold_invariants_and_replay: Fixture {
        slice: &[
            "Aaron's Feeding Razor",
            "Abactor",
            "Abombwe",
            "Alpha Glint",
            "Baltimore Purge",
            "Behind You!",
            "Concealed Weapon",
            "Confusion of the Eye",
            "Desert Eagle",
            "Focus the Blood",
            "Gregory Winter",
            "Hospital Food",
            "Illusory Resources",
            "Infernal Pursuit",
            "Learjet",
            "Preternatural Evasion",
            "Renegade Garou",
            "Renewed Vigor",
            "Staredown",
            "Torrent",
        ],
        covers: &[
            "play:Aaron's Feeding Razor",
            "play:Abactor",
            "play:Abombwe",
            "play:Alpha Glint",
            "play:Baltimore Purge",
            "play:Behind You!",
            "play:Concealed Weapon",
            "play:Confusion of the Eye",
            "play:Desert Eagle",
            "play:Focus the Blood",
            "play:Gregory Winter",
            "play:Hospital Food",
            "play:Illusory Resources",
            "play:Infernal Pursuit",
            "play:Learjet",
            "play:Preternatural Evasion",
            "play:Renegade Garou",
            "play:Renewed Vigor",
            "play:Staredown",
            "play:Torrent",
            "ability:Gregory Winter",
            "ability:Renegade Garou",
            "ability:Illusory Resources",
            "ability:Hospital Food",
            "ability:Renewed Vigor",
            "rider:Baltimore Purge",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Lisette Vizquel",
                    "Wise Frog",
                    "Alu",
                    "Cameron",
                    "Hester Reed",
                    "Lord Tremere",
                    "Danielle Diron",
                    "Elisha Tucker",
                    "Casey Snyder",
                    "Doc Martina",
                    "Kalinda",
                    "Crow",
                    "Ariadne",
                    "Gunnar",
                    "Jacob Fermor",
                ],
            );
        },
        seeds: 240,
        ..BASE
    };
    /// Legacy cards for TWDA decks, rounds 4–5 (slices 16p–16q): the Gangrel
    /// decks' missing cards and vampires.
    random_games_with_twda_round_5_hold_invariants_and_replay: Fixture {
        slice: &[
            "Anachronism",
            "Bone Spur",
            "Kiss of Ra, The",
            "Mind of the Wilds",
            "Ossian",
            "Path of Lilith, The",
            "Predator's Transformation",
            "Read the Winds",
            "Zip Gun",
            // Ranged weapons for Anachronism, a frenzy for the reflex.
            ".44 Magnum",
            "Terror Frenzy",
            "Neighborhood Watch Commander",
            "Psyche!",
        ],
        covers: &[
            "play:Anachronism",
            "play:Bone Spur",
            "play:Kiss of Ra, The",
            "play:Mind of the Wilds",
            "play:Ossian",
            "play:Path of Lilith, The",
            "play:Predator's Transformation",
            "play:Read the Winds",
            "play:Zip Gun",
            "ability:Ossian",
            "ability:Zip Gun",
            "rider:Path of Lilith, The",
            "play:Neighborhood Watch Commander",
            "play:Psyche!",
            "ability:Neighborhood Watch Commander",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Ariadne",
                    "Gunnar",
                    "Jacob Fermor",
                    "Lillian",
                    "Robert Price",
                    "Xaviar",
                    "Fakir al Sidi",
                    "Vivian VI",
                    "Tabitha Fisk",
                    "Tatiana Romanov",
                    "Delilah Easton",
                    "Isabel de Leon",
                ],
            );
        },
        seeds: 40,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 7 (slice 16s), steps 1–5: the
    /// Antti, Erlik, Catalana, Sabbat Arising and Esqueci decks' cards.
    random_games_with_twda_round_7a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Aura Reading",
            "Banshee Ironwail",
            "Poacher's Hunting Ground",
            "Smear Campaign",
            "Breath of Thanatos",
            "Disciple of Lazarus",
            "Emerald Legionnaire",
            "Trochomancy",
            "Coma",
            "Confusion",
            "Eyes of Chaos",
            "Kindred Spirits",
            "Mind Tricks",
            // Hunting grounds for Poacher's; a master card on minions for
            // Smear Campaign; a retainer for Marcus Vitel.
            "Academic Hunting Ground",
            "Rebel",
            "Owl Companion",
        ],
        covers: &[
            "play:Aura Reading",
            "play:Banshee Ironwail",
            "play:Poacher's Hunting Ground",
            "play:Smear Campaign",
            "play:Breath of Thanatos",
            "play:Disciple of Lazarus",
            "play:Emerald Legionnaire",
            "play:Trochomancy",
            "play:Coma",
            "play:Confusion",
            "play:Eyes of Chaos",
            "play:Kindred Spirits",
            "play:Mind Tricks",
            "ability:Poacher's Hunting Ground",
            "ability:Smear Campaign",
            "ability:Emerald Legionnaire",
            "ability:Adonai",
            "ability:Babalawo Alafin",
            "ability:Mina Grotius",
            "ability:Nicomedes",
            "ability:Solomon Batanea",
            "ability:Le Dinh Tho",
            "ability:Lucinde, Alastor",
            "ability:Marcus Vitel",
            "ability:Baron, The",
            "ability:Rurik Rakoczy",
            "ability:Osric Vladislav",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Adonai",
                    "Cornelius Ottavio",
                    "Ozmo",
                    "Quira, The Bitch Queen",
                    "Sheila Mezarin",
                    "Babalawo Alafin",
                    "Erebus",
                    "Erlik",
                    "Mina Grotius",
                    "Nicomedes",
                    "Solomon Batanea",
                    "Arika",
                    "Le Dinh Tho",
                    "Lucinde, Alastor",
                    "Marcus Vitel",
                    "Baron, The",
                    "Céleste Lamontagne",
                    "Ludmijla Rakoczy",
                    "Petra",
                    "Rurik Rakoczy",
                    "Osric Vladislav",
                    "Persephone Tar-Anis",
                ],
            );
        },
        seeds: 152,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 7 (slice 16s), steps 6–11: the
    /// Giovanni toolbox, Beauty and the Beast, Helena, True Osebo and Army
    /// of Rats decks' cards; the Red List.
    random_games_with_twda_round_7b_hold_invariants_and_replay: Fixture {
        slice: &[
            "Acquired Ventrue Assets",
            "Call of the Hungry Dead",
            "Fake Out",
            "Powerbase: Cape Verde",
            "Spectral Divination",
            "Spiritual Guidance",
            "Sudario Refraction",
            "Thrown Sewer Lid",
            "Bleeding the Vine",
            "Jackie Therman",
            "Shadow Boxing",
            "Blood Puppy",
            "Millicent Smith, Puritan Vampire Hunter",
            "Quicken Sight",
            "Sideslip",
            "Base Hunting Ground",
            "Mbare Market, Harare",
            "Dementation",
            "Deny",
            "Effective Management",
            "Bitter and Sweet Story, The",
        ],
        covers: &[
            "play:Acquired Ventrue Assets",
            "play:Call of the Hungry Dead",
            "play:Fake Out",
            "play:Powerbase: Cape Verde",
            "play:Spectral Divination",
            "play:Spiritual Guidance",
            "play:Sudario Refraction",
            "play:Thrown Sewer Lid",
            "play:Bleeding the Vine",
            "play:Jackie Therman",
            "play:Shadow Boxing",
            "play:Blood Puppy",
            "play:Millicent Smith, Puritan Vampire Hunter",
            "play:Quicken Sight",
            "play:Sideslip",
            "play:Base Hunting Ground",
            "play:Mbare Market, Harare",
            "play:Dementation",
            "play:Deny",
            "play:Effective Management",
            "play:Bitter and Sweet Story, The",
            "ability:Acquired Ventrue Assets",
            "ability:Powerbase: Cape Verde",
            "ability:Blood Puppy",
            "ability:Jackie Therman",
            "ability:Diego Giovanni",
            "ability:Donatello Giovanni",
            "ability:Thomasso Ghiberti",
            "ability:Andre LeRoux",
            "ability:Black Annis",
            "ability:Enkidu, The Noah",
            "ability:Eugene",
            "ability:Helena",
            "ability:Joaquin Murietta",
            "ability:Khalu",
            "ability:Cesewayo",
            "ability:Massassi",
            "ability:Persia, The Beautiful Statue",
            "red_list_hunt",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Diego Giovanni",
                    "Don Michael Antonio Giovanni",
                    "Donatello Giovanni",
                    "Thomasso Ghiberti",
                    "Primo Giovanni",
                    "Andre LeRoux",
                    "Black Annis",
                    "Enkidu, The Noah",
                    "New Blood",
                    "Eugene",
                    "Helena",
                    "Ira Rivers",
                    "Joaquin Murietta",
                    "Khalu",
                    "Abiku",
                    "Cesewayo",
                    "Massassi",
                    "Tatu Sawosa",
                    "Eddie Gaines",
                    "Marta",
                    "Midget",
                    "Persia, The Beautiful Statue",
                    "Fish",
                ],
            );
        },
        seeds: 190,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 8 (slice 16t), steps 1–5: Blood
    /// Brother circles — the Mexico City, Torrance and Kiev circles' cards,
    /// New Blood's choice.
    random_games_with_twda_round_8a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Blood Buff",
            "Camaraderie",
            "Claiming the Body",
            "Hive Mind",
            "Shell Game",
            "Slake the Thirst",
            "Walk of Caine",
            "Zillah's Tears",
            "Octopod",
            "Oppugnant Night",
            "Three's a Crowd",
            "Coagulated Entity",
            "Coordinate Attacks",
            "Unwholesome Bond",
            "Ambush",
            "Gestalt",
            "London Evening Star, Tabloid Newspaper",
            "Sanguine Instruction",
            "Dark Influences",
            "Loki's Gift",
            "Procurer",
        ],
        covers: &[
            "play:Blood Buff",
            "play:Camaraderie",
            "play:Hive Mind",
            "play:Shell Game",
            "play:Walk of Caine",
            "play:Zillah's Tears",
            "play:Octopod",
            "play:Oppugnant Night",
            "play:Coagulated Entity",
            "play:Unwholesome Bond",
            "play:Gestalt",
            "play:London Evening Star, Tabloid Newspaper",
            "play:Sanguine Instruction",
            "play:Loki's Gift",
            "play:Procurer",
            "ability:Oppugnant Night",
            "ability:Danylo",
            "ability:Pavlo",
            "choose_circle",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Hermana Hambrienta Mayor",
                    "Hermana Hambrienta Menor",
                    "Ilse",
                    "Jack",
                    "Karl",
                    "Mark",
                    "Truman",
                    "Danylo",
                    "Gavrylo",
                    "Kyrylo",
                    "Pavlo",
                    "Volo",
                ],
            );
            // New Blood uncontrolled, to be influenced out.
            let nb = find_card("New Blood");
            for s in &mut e.state.seats {
                s.uncontrolled.push(zvtes::state::UncontrolledSlot {
                    card: nb,
                    copy: Default::default(),
                    counters: 0,
                    banished: None,
                });
            }
        },
        seeds: 8,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 8 (slice 16t), steps 6–10: the
    /// Chicago and Budapest circles' cards, Major Boon, the Tzimisce
    /// toolbox.
    random_games_with_twda_round_8b_hold_invariants_and_replay: Fixture {
        slice: &[
            "Major Boon",
            "Rampage",
            "Skin of Rock",
            "Monocle of Clarity",
            "Redistribution",
            "Robert Carter",
            "Breath of the Dragon",
            "Chiropteran Marauder",
            "Tribute to the Master",
            "Blood Tears of Kephran",
            "Concoction of Vitality",
            "NRA PAC",
            "Fortitude",
            "Horrid Form",
            "Inner Essence",
            "Skin of Steel",
            "Vicissitude",
            // A location for Rampage.
            "Channel 10",
        ],
        covers: &[
            "play:Major Boon",
            "play:Rampage",
            "play:Skin of Rock",
            "play:Redistribution",
            "play:Robert Carter",
            "play:Breath of the Dragon",
            "play:Chiropteran Marauder",
            "play:Tribute to the Master",
            "play:Blood Tears of Kephran",
            "play:Concoction of Vitality",
            "play:NRA PAC",
            "play:Fortitude",
            "play:Horrid Form",
            "play:Inner Essence",
            "play:Skin of Steel",
            "play:Vicissitude",
            "ability:Jerry",
            "ability:Terry",
            "ability:Dorka",
            "ability:Zizi",
        ],
        setup: |e, _| {
            // Zizi and Viri on one seat (`named` deals the names round
            // the table): "in your ready region" (rules-qa 73.7).
            named(
                e,
                &[
                    "Barry",
                    "Jerry",
                    "Larry",
                    "Zizi",
                    "Tom",
                    "Dorka",
                    "Gabi",
                    "Viri",
                    "Terry",
                    "Ági",
                ],
            );
        },
        seeds: 8,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 9 (slice 16u): Shadow Play and
    /// the Lasombra decks, The Embrace and the Toreador antitribu and
    /// Ravnos decks.
    random_games_with_twda_round_9a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Shadow Play",
            "Shroud of Absence",
            "Shroud of Night",
            "Tenebrous Form",
            "Blanket of Night",
            "Oubliette",
            "Shadow Body",
            "Metro Underground",
            "Gurchon Hall",
            "Embrace, The",
            "Web of Knives Recruit",
            "Art Scam",
            "Fetish Club Hunting Ground",
            "Palla Grande",
            "Gird Minions",
            "Mozambique Allure",
            "Sunset Strip, Hollywood",
        ],
        covers: &[
            "play:Shadow Play",
            "play:Tenebrous Form",
            "play:Oubliette",
            "play:Metro Underground",
            "play:Embrace, The",
            "play:Web of Knives Recruit",
            "play:Palla Grande",
            "play:Gird Minions",
            "play:Mozambique Allure",
            "play:Sunset Strip, Hollywood",
            "ability:Francisco Domingo de Polonia",
            "ability:Montano",
            "ability:Anka, Priestess of Thorns",
            "play:Shroud of Absence",
            "play:Shroud of Night",
            "play:Blanket of Night",
            "play:Shadow Body",
            "play:Gurchon Hall",
            "play:Art Scam",
            "play:Fetish Club Hunting Ground",
            "ability:Gurchon Hall",
            "ability:Sunset Strip, Hollywood",
            "ability:Ayelea, The Manipulator",
            "ability:Lázár Dobrescu",
            "ability:Marciana Giovanni, Investigator",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Alvaro, The Scion of Angelica",
                    "Ambrosio Luis Monçada, Plenipotentiary",
                    "Antonio Delgado",
                    "Francisco Domingo de Polonia",
                    "Gratiano",
                    "Angelica, The Canonicus",
                    "Ayelea, The Manipulator",
                    // A Tremere antitribu on the next seat, for Ayelea.
                    "Ember Wright",
                    "Marciana Giovanni, Investigator",
                    "Montano",
                    "Royce",
                    "Lolita",
                    "Jost Werner",
                    "Zöe",
                    "Anka, Priestess of Thorns",
                    "Lázár Dobrescu",
                    "Spleen, Georgio Gutterpunk",
                    "Khalil Ravana",
                    "Ignacio, The Black Priest",
                ],
            );
        },
        seeds: 24,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 9 (slice 16u): Popó's Appolonius
    /// merge, Bollix and Flames of Insurrection, the Ravnos referendums.
    random_games_with_twda_round_9b_hold_invariants_and_replay: Fixture {
        slice: &[
            "Eldest Are Kholo, The",
            "Cryptic Rider",
            "Autarkis Persecution",
            "Poison Pill",
            "Bollix",
            "Flames of Insurrection",
            "Legacy",
        ],
        covers: &[
            "play:Autarkis Persecution",
            "play:Bollix",
            "play:Flames of Insurrection",
            "play:Legacy",
            "merge",
            "play:Cryptic Rider",
            "play:Poison Pill",
            "play:Eldest Are Kholo, The",
            "ability:Appolonius",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Ashlesha",
                    "Sasha Miklos",
                    "Andrei Puxon",
                    "Richter, The Templar of Du Mont",
                ],
            );
            // Appolonius: the base for seat 0, its advanced card
            // uncontrolled beside it (a merge); the advanced card for seat
            // 1, its base in the crypt (Legacy).
            let (base, adv) = (appolonius(false), appolonius(true));
            if absent(e, base) && absent(e, adv) {
                e.add_minion(SeatId(0), base, 4, false);
                e.state.seats[0].uncontrolled.push(zvtes::state::UncontrolledSlot {
                    card: adv,
                    copy: Default::default(),
                    counters: 2,
                    banished: None,
                });
                let m = e.add_minion(SeatId(1), adv, 4, false);
                e.state.minions[m.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
                e.state.seats[1].crypt.push(base);
            }
        },
        seeds: 8,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 10 (slice 16v): the Anarch
    /// modifiers, combat cards, Chameleon, Progeny, the locations and the
    /// round's vampires.
    random_games_with_twda_round_10a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Power of One",
            "Adhocracy",
            "Ni Dieu ni Maître",
            "Marked Path",
            "Horrific Countenance",
            "Fast Hands",
            "Orgy of Blood",
            "Drawing Out the Beast",
            "Approximation of Loyalty",
            "Chameleon",
            "Third Tradition: Progeny",
            "Zaire River Ferry",
            "Ankara Citadel, Turkey, The",
            "Infernal Servitor",
            // A weapon for Fast Hands, an ally for Orgy of Blood.
            ".44 Magnum",
            "Political Ally",
        ],
        covers: &[
            "play:Power of One",
            "play:Adhocracy",
            "play:Ni Dieu ni Maître",
            "play:Marked Path",
            "play:Fast Hands",
            "play:Drawing Out the Beast",
            "play:Approximation of Loyalty",
            "play:Chameleon",
            "play:Third Tradition: Progeny",
            "play:Zaire River Ferry",
            "play:Ankara Citadel, Turkey, The",
            "play:Infernal Servitor",
            "ability:Zaire River Ferry",
            "ability:Marked Path",
            "ability:Ian Carfax",
            "ability:Malabranca",
            "ability:Victoria Ash",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Annabelle Triabell",
                    "Dark Selina",
                    "Drozodny",
                    "Hardestadt",
                    "Malabranca",
                    "Ian Carfax",
                    "Genevieve",
                    "Stavros",
                    "Arishat",
                ],
            );
            // Victoria Ash (G7) beside a Banu Haqim.
            let v = CRYPT
                .iter()
                .position(|c| c.name == "Victoria Ash" && c.group == 7)
                .expect("Victoria Ash G7") as u16;
            if absent(e, v) {
                e.add_minion(SeatId(1), v, 4, false);
            }
            per_seat(e, |c| c.clan == "Banu Haqim", 4);
        },
        seeds: 186,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 10 (slice 16v): the referendums
    /// and masters — Rumors of Gehenna, Eat the Rich, Anathema, The
    /// Meddling of Semsith, Kaymakli Nightmares.
    random_games_with_twda_round_10b_hold_invariants_and_replay: Fixture {
        slice: &[
            "Rumors of Gehenna",
            "Eat the Rich",
            "Anathema",
            "Meddling of Semsith, The",
            "Camarilla Threat",
            "Protected Resources",
            "Antediluvian Awakening",
            "Kaymakli Nightmares",
            "Redeem the Lost Soul",
            "Vulnerability",
        ],
        covers: &[
            "play:Rumors of Gehenna",
            "play:Eat the Rich",
            "play:Anathema",
            "play:Meddling of Semsith, The",
            "play:Camarilla Threat",
            "play:Protected Resources",
            "play:Antediluvian Awakening",
            "play:Kaymakli Nightmares",
            "play:Redeem the Lost Soul",
            "play:Vulnerability",
            "ability:Drozodny",
        ],
        setup: |e, _| {
            named(
                e,
                &["Genevieve", "Dark Selina", "Hardestadt", "Annabelle Triabell", "Drozodny"],
            );
        },
        seeds: 16,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 11 (slice 16w): the minion cards
    /// — Groundfighting, The Status Perfectus, Kaymakli Fragment, Taunt the
    /// Caged Beast, Perfect Clarity, Touch of Clarity — and the round's
    /// vampires.
    random_games_with_twda_round_11a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Mesu Bedshet",
            "Groundfighting",
            "Deer Rifle",
            "Raking Talons",
            "Brick by Brick",
            "Nose of the Hound",
            "Taunt the Caged Beast",
            "Big Game",
            "Mental Maze",
            "Perfect Clarity",
            "Legal Manipulations",
            "Touch of Clarity",
            "Anarch Manifesto, An",
            "Kaymakli Fragment",
            "Petra Resonance",
            "Status Perfectus, The",
            // Something for Groundfighting to cancel.
            "Immortal Grapple",
        ],
        covers: &[
            "play:Mesu Bedshet",
            "play:Groundfighting",
            "play:Deer Rifle",
            "play:Raking Talons",
            "play:Brick by Brick",
            "play:Nose of the Hound",
            "play:Taunt the Caged Beast",
            "play:Big Game",
            "play:Mental Maze",
            "play:Perfect Clarity",
            "play:Legal Manipulations",
            "play:Touch of Clarity",
            "play:Anarch Manifesto, An",
            "play:Kaymakli Fragment",
            "play:Petra Resonance",
            "play:Status Perfectus, The",
            "ability:Deer Rifle",
            "ability:Kaymakli Fragment",
            "ability:Status Perfectus, The",
            "ability:Gustaphe Brunnelle",
            "ability:Venere Carboni",
            "ability:Josef von Bauren",
            "ability:Gabrielle di Righetti",
            "ability:Giotto Verducci",
            "ability:Nergal",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Gustaphe Brunnelle",
                    "Josef von Bauren",
                    "Gabrielle di Righetti",
                    "Harold Tanner",
                    "Giotto Verducci",
                    "Leandro",
                    "Venere Carboni",
                    "Alabástrom",
                    "Midget",
                    "Enkidu, The Noah",
                    "Nichodemus",
                    "Lachlan, Noddist",
                ],
            );
            let adv = CRYPT
                .iter()
                .position(|c| c.name == "Nergal" && c.adv)
                .expect("Nergal (ADV)") as u16;
            if absent(e, adv) {
                e.add_minion(SeatId(2), adv, 4, false);
            }
            per_seat(e, |c| c.sect == Some(zvtes::crypt::Sect::Anarch), 4);
            // Venere Carboni in torpor, to leave it in his unlock phase.
            let venere = find_card("Venere Carboni");
            for m in &mut e.state.minions {
                if m.crypt_card() == Some(venere) {
                    m.in_torpor = true;
                }
            }
        },
        seeds: 16,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 11 (slice 16w): the masters, the
    /// referendums and the event — Kindred Segregation, Free States Rant,
    /// Legendary Vampire, Propaganda of the Deed, Waiting Game, Maabara.
    random_games_with_twda_round_11b_hold_invariants_and_replay: Fixture {
        slice: &[
            "Kindred Segregation",
            "Free States Rant",
            "Legendary Vampire",
            "Propaganda of the Deed",
            "Waiting Game",
            "Maabara",
            "Fortschritt Library",
            "Art of Pain, The",
            // Allies for Kindred Segregation and Waiting Game, a Gehenna
            // event for Fortschritt Library.
            "Political Ally",
            "Gravebound Drone",
            "Blood Weakens",
        ],
        covers: &[
            "play:Kindred Segregation",
            "play:Free States Rant",
            "play:Legendary Vampire",
            "play:Propaganda of the Deed",
            "play:Waiting Game",
            "play:Maabara",
            "play:Fortschritt Library",
            "play:Art of Pain, The",
            "ability:Maabara",
            "ability:Propaganda of the Deed",
        ],
        setup: |e, _| {
            named(e, &["Leandro", "Venere Carboni", "Jessica", "Giotto Verducci"]);
            per_seat(e, |c| c.sect == Some(zvtes::crypt::Sect::Anarch), 4);
            per_seat(e, |c| c.clan == "Toreador antitribu", 4);
            // Maabara is a Harbinger of Skulls card.
            per_seat(e, |c| c.clan == "Harbinger of Skulls", 4);
        },
        seeds: 40,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 12 (slice 16x): the actions,
    /// equipment, master and combat cards — Clandestine Contract, Harass,
    /// Loss, Dive into Madness, Gear Up, Divine Sign, Arson, The Book of
    /// Going Forth by Night, Palatial Estate, Flak Jacket, Sawed-Off
    /// Shotgun, Island of Yiaros, Pulled Fangs, Lucky Blow, Anesthetic
    /// Touch.
    random_games_with_twda_round_12a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Clandestine Contract",
            "Harass",
            "Loss",
            "Dive into Madness",
            "Gear Up",
            "Divine Sign",
            "Arson",
            "Book of Going Forth by Night, The",
            "Palatial Estate",
            "Flak Jacket",
            "Sawed-Off Shotgun",
            "Island of Yiaros",
            "Pulled Fangs",
            "Lucky Blow",
            "Anesthetic Touch",
        ],
        covers: &[
            "play:Clandestine Contract",
            "play:Harass",
            "play:Loss",
            "play:Dive into Madness",
            "play:Gear Up",
            "play:Divine Sign",
            "play:Arson",
            "play:Book of Going Forth by Night, The",
            "play:Palatial Estate",
            "play:Flak Jacket",
            "play:Sawed-Off Shotgun",
            "play:Island of Yiaros",
            "play:Lucky Blow",
            "play:Anesthetic Touch",
            "play:Pulled Fangs",
            "ability:Sawed-Off Shotgun",
            "ability:Flak Jacket",
            "ability:Island of Yiaros",
            "ability:Clandestine Contract",
        ],
        setup: |e, _| {
            let has = |c: &CryptCard, d: &str| c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d));
            per_seat(e, |c| c.clan == "Banu Haqim" && c.capacity >= 5, 4);
            per_seat(e, |c| has(c, "qui"), 4);
            per_seat(e, |c| has(c, "dem"), 4);
            per_seat(e, |c| has(c, "nec") && c.sect == Some(zvtes::crypt::Sect::Anarch), 4);
            per_seat(e, |c| has(c, "nec"), 4);
            per_seat(e, |c| has(c, "obe") || has(c, "aus"), 4);
            per_seat(e, |c| c.clan == "Ventrue", 4);
        },
        seeds: 14,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 12 (slice 16x): the modifiers and
    /// reactions — Deed the Heart's Desire, Guardian Vigil, Acheron Vortex,
    /// Eluding the Arms of Morpheus, Repulsion, Tainted Spring — and the
    /// round's vampires.
    random_games_with_twda_round_12b_hold_invariants_and_replay: Fixture {
        slice: &[
            "Deed the Heart's Desire",
            "Guardian Vigil",
            "Acheron Vortex",
            "Eluding the Arms of Morpheus",
            "Repulsion",
            "Tainted Spring",
            // An animal for Tainted Spring.
            "Murder of Crows",
        ],
        covers: &[
            "play:Deed the Heart's Desire",
            "play:Guardian Vigil",
            "play:Acheron Vortex",
            "play:Eluding the Arms of Morpheus",
            "play:Repulsion",
            "play:Tainted Spring",
            "ability:Laika",
            "ability:Xipe Totec",
            "ability:Luc",
            "ability:Thucimia",
            "ability:Mateusz Kazimirski, Biker of Blood",
            "ability:Saulot, The Wanderer",
            "ability:Lucinde, Alastor",
            "ability:Leila Monroe",
            // Asguresh's cancel needs a combat card played against him: not
            // reached by random play in 60 seeds (tested in twda_round12).
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Asguresh",
                    "Omer",
                    "Mateusz Kazimirski, Biker of Blood",
                    "Luc",
                    "Thucimia",
                    "Alessandro Garcia",
                    "Paulo de Castille",
                    "Ermenegildo, The Rake",
                    "Leila Monroe",
                    "Saulot, The Wanderer",
                    "Tyler McGill",
                    "Laika",
                    "Xipe Totec",
                    "Adhiambo",
                ],
            );
            let lucinde = CRYPT
                .iter()
                .position(|c| c.name == "Lucinde, Alastor" && c.group == 7)
                .expect("Lucinde G7") as u16;
            if absent(e, lucinde) {
                e.add_minion(SeatId(3), lucinde, 4, false);
            }
            let has = |c: &CryptCard, d: &str| c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d));
            per_seat(e, |c| c.sect == Some(zvtes::crypt::Sect::Anarch) && has(c, "for"), 4);
            per_seat(e, |c| c.clan == "Harbinger of Skulls", 4);
            per_seat(e, |c| has(c, "ani"), 4);
            per_seat(e, |c| has(c, "obe") || has(c, "pre"), 4);
            per_seat(e, |c| c.sect == Some(zvtes::crypt::Sect::Camarilla), 4);
        },
        seeds: 10,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 13 (slice 16y): the library cards.
    /// Form of Corruption needs [ser], which no pool vampire has, and Two
    /// Wrongs a bleed at its player from beyond its predator (a redirect or
    /// "bleed anyone"): not played (both tested in twda_round13).
    random_games_with_twda_round_13a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Animalism",
            "University Hunting Ground",
            "Computer Hacking",
            "Aching Beauty",
            "Banu Haqim Justicar",
            "Lasombra Justicar",
            "Thadius Zho",
            "Black Metamorphosis",
            "Foundation Exhibit",
            "Entombment",
            "Hand of Conrad",
            "Khobar Towers, Al-Khubar",
            "Camarilla Vitae Slave",
            "Talbot's Chainsaw",
            "Molotov Cocktail",
            "Charnas the Imp",
            "Two Wrongs",
            "Game of Malkav",
            "Alastor",
            // Equipment for Alastor's search.
            "Flak Jacket",
        ],
        covers: &[
            "play:Animalism",
            "play:University Hunting Ground",
            "play:Computer Hacking",
            "play:Aching Beauty",
            "play:Banu Haqim Justicar",
            "play:Lasombra Justicar",
            "play:Thadius Zho",
            "play:Black Metamorphosis",
            "play:Foundation Exhibit",
            "play:Entombment",
            "play:Hand of Conrad",
            "play:Khobar Towers, Al-Khubar",
            "play:Camarilla Vitae Slave",
            "play:Talbot's Chainsaw",
            "play:Molotov Cocktail",
            "play:Charnas the Imp",
            "play:Game of Malkav",
            "play:Alastor",
            "ability:University Hunting Ground",
            "ability:Hand of Conrad",
            "ability:Talbot's Chainsaw",
            "ability:Molotov Cocktail",
            // Camarilla Vitae Slave's choice: not reached by random play
            // (tested in twda_round13).
            "ability:Khobar Towers, Al-Khubar",
            "ability:Alastor",
            "ability:Thadius Zho",
        ],
        setup: |e, _| {
            let has = |c: &CryptCard, d: &str| c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d));
            let camarilla = |c: &CryptCard| c.sect == Some(zvtes::crypt::Sect::Camarilla);
            per_seat(e, |c| c.clan == "Toreador", 4);
            per_seat(e, |c| c.clan == "Tremere", 4);
            per_seat(e, |c| c.clan == "Tremere antitribu", 4);
            per_seat(e, |c| c.clan == "Toreador antitribu", 4);
            per_seat(e, |c| c.clan == "Ministry", 4);
            per_seat(e, |c| c.clan == "Malkavian", 4);
            per_seat(e, |c| c.clan == "Lasombra", 4);
            per_seat(e, |c| has(c, "obt"), 4);
            per_seat(e, |c| c.clan == "Banu Haqim" && camarilla(c), 4);
            per_seat(e, |c| c.clan == "Lasombra" && camarilla(c), 4);
            per_seat(
                e,
                |c| matches!(c.title, zvtes::crypt::Title::Justicar(_) | zvtes::crypt::Title::InnerCircle(_)),
                4,
            );
        },
        seeds: 170,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 13 (slice 16y): the vampires.
    random_games_with_twda_round_13b_hold_invariants_and_replay: Fixture {
        // Action cards for Epikasta to recycle.
        slice: &["Kine Resources Contested", "Aid from Bats", "Computer Hacking"],
        covers: &[
            "ability:Calebros, The Martyr",
            "ability:Dr. Morrow, The Skindoctor",
            "ability:Zane",
            "ability:Mustafa, The Heir",
            "ability:Kateline Nadasdy",
            "ability:Dónal O'Connor",
            "ability:Emily Carson",
            "ability:Lady Vadislava",
            "ability:Lord Leopold Valdemar",
            "ability:Luca Italicus",
            "ability:Epikasta Rigatos",
            "ability:Antón de Concepción",
            // Aidan's and Charles's presses are printed stats, not
            // abilities; Nikolaus needs an uncontrolled Nosferatu: tested in
            // twda_round13.
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Calebros, The Martyr",
                    "Dr. Morrow, The Skindoctor",
                    "Zane",
                    "Mustafa, The Heir",
                    "Kateline Nadasdy",
                    "Nikolaus Vermeulen",
                    "Dónal O'Connor",
                    "Emily Carson",
                    "Lady Vadislava",
                    "Lord Leopold Valdemar",
                    "Luca Italicus",
                    "Epikasta Rigatos",
                    "Antón de Concepción",
                    "Aidan Lyle",
                    "Charles Delmare",
                    "Bela",
                    "Troius",
                    "Molly MacDonald",
                    "Carolina Vález",
                    "Rutor",
                    "Vasily",
                    "João Bilé",
                    "Thomas De Lutrius",
                    "Selma the Repugnant",
                    "Una",
                    "Grimgroth",
                ],
            );
            per_seat(e, |c| c.clan == "Toreador" && c.capacity >= 7, 4);
            per_seat(e, |c| c.sect == Some(zvtes::crypt::Sect::Camarilla) && c.capacity >= 6, 4);
            per_seat(e, |c| c.clan == "Nosferatu", 4);
            // Allies for Dr. Morrow's grafts.
            for s in 0..4 {
                e.add_ally(SeatId(s), zvtes::state::find_library("Aggressive Corpse"));
            }
        },
        seeds: 120,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 14 (slice 16z): the library cards.
    random_games_with_twda_round_14a_hold_invariants_and_replay: Fixture {
        slice: &[
            "419 Operation",
            "Changeling Skin Mask",
            "Dark Steel",
            "Demonstration",
            "Esprit de Corps",
            "Festivo dello Estinto",
            "Inside Dirt",
            "Marked Territory",
            "Osebo Kholo",
            "Powerbase: Luanda",
            "Public Vilification",
            "Recruitment Exercise",
            "Rewind Time",
            "Rise of the Fallen",
            "Sanguine Entrapment",
            "Shakar: the Hunt",
            "Spoils of War",
            "Summon History",
            "Superior Mettle",
            "Tabriz Assembly",
            "Tangle Atropos' Hand",
            "Taste of Death",
            // For Summon History's search.
            "Kevlar Vest",
        ],
        covers: &[
            "play:419 Operation",
            "play:Changeling Skin Mask",
            "play:Dark Steel",
            "play:Demonstration",
            "play:Esprit de Corps",
            "play:Festivo dello Estinto",
            "play:Inside Dirt",
            "play:Marked Territory",
            "play:Osebo Kholo",
            "play:Powerbase: Luanda",
            "play:Public Vilification",
            "play:Recruitment Exercise",
            "play:Rewind Time",
            "play:Rise of the Fallen",
            "play:Sanguine Entrapment",
            "play:Shakar: the Hunt",
            "play:Spoils of War",
            "play:Summon History",
            "play:Superior Mettle",
            "play:Tabriz Assembly",
            "play:Tangle Atropos' Hand",
            "play:Taste of Death",
            "ability:419 Operation",
            "ability:Changeling Skin Mask",
            "ability:Demonstration",
            "ability:Marked Territory",
            "ability:Powerbase: Luanda",
            "ability:Shakar: the Hunt",
            "ability:Tabriz Assembly",
        ],
        setup: |e, _| {
            let has = |c: &CryptCard, d: &str| c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d));
            let sabbat = |c: &CryptCard| c.sect == Some(zvtes::crypt::Sect::Sabbat);
            per_seat(e, |c| c.clan == "Osebo", 4);
            per_seat(e, |c| c.clan == "Nosferatu antitribu", 4);
            per_seat(e, |c| c.clan == "Gangrel antitribu", 4);
            per_seat(e, |c| c.clan == "Brujah antitribu", 4);
            per_seat(e, |c| c.clan == "Ventrue antitribu", 4);
            per_seat(e, |c| c.clan == "True Brujah", 4);
            per_seat(e, |c| c.black_hand, 4);
            per_seat(e, |c| c.infernal, 4);
            per_seat(e, |c| sabbat(c) && c.title != zvtes::crypt::Title::None, 4);
            per_seat(e, |c| has(c, "cel") && has(c, "qui"), 4);
            per_seat(e, |c| has(c, "obt") && has(c, "pot"), 4);
            per_seat(e, |c| has(c, "pot") && has(c, "pre"), 4);
            per_seat(e, |c| has(c, "tem"), 4);
            per_seat(e, |c| has(c, "for"), 4);
        },
        seeds: 300,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 14 (slice 16z): the vampires.
    random_games_with_twda_round_14b_hold_invariants_and_replay: Fixture {
        // Political actions for Bruce de Guy, Quentin, Masika's discards;
        // allies and retainers for Baba Yaga and Carna.
        slice: &[
            "Kine Resources Contested",
            "ECTU Operative",
            "Raven Spy",
            "Laptop Computer",
            "Giant's Blood",
            // A [tha] card for Claudio Severino's discard.
            "Theft of Vitae",
        ],
        covers: &[
            "ability:Alex Camille",
            "ability:Carna, The Princess Witch",
            "ability:Claudio Severino",
            "ability:Dan Murdock",
            // Gold Pan Dan's unlock needs a combat leaving his opponent out
            // of the ready region: rare (tested in twda_round14).
            "ability:Kyle Strathcona, Cardinal of Canada",
            "ability:Masika",
            "ability:Quentin",
            "ability:Mélusine d'Anjou",
            "ability:Karif al Numair",
            "ability:Gerald FitzGerald",
            "ability:Paul Cordwood",
            "ability:Bruce de Guy",
            "ability:Nehemiah",
            "ability:Baba Yaga, the Iron Hag",
            "ability:Synner-G",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Alex Camille",
                    "Carna, The Princess Witch",
                    "Claudio Severino",
                    "Dan Murdock",
                    "Gold Pan Dan",
                    "Kyle Strathcona, Cardinal of Canada",
                    "Masika",
                    "Quentin",
                    "Mélusine d'Anjou",
                    "Karif al Numair",
                    "Gerald FitzGerald",
                    "Paul Cordwood",
                    "Bruce de Guy",
                    "Nehemiah",
                    "Baba Yaga, the Iron Hag",
                    "Synner-G",
                    "Vlad Tepes",
                    "Andrew Stuart",
                    "Orlando Oriundus",
                    "Mimir",
                    "Strohmann",
                    "De Sade",
                    "Dracon, The",
                    "Selena",
                    "Suhailah",
                    "Righteous Endeavor",
                    "Maldavis",
                    "Bulscu",
                    "Badr, Shadow of Granada",
                    "Aimery Methuin",
                    "Dr. Julius Sutphen",
                    "Martin Franckel",
                ],
            );
            per_seat(e, |c| c.sect == Some(zvtes::crypt::Sect::Sabbat) && c.capacity >= 4, 4);
            per_seat(e, |c| c.clan == "Tremere", 4);
            // Younger untitled Sabbat vampires beside Gerald FitzGerald.
            per_seat(
                e,
                |c| {
                    c.sect == Some(zvtes::crypt::Sect::Sabbat)
                        && c.title == zvtes::crypt::Title::None
                        && c.capacity <= 5
                },
                4,
            );
            // A retainer on Carna, for her to burn.
            let carna = find_card("Carna, The Princess Witch");
            if let Some(m) = (0..e.state.minions.len())
                .map(|i| zvtes::ids::MinionId(i as u32))
                .find(|m| e.state.minions[m.index()].crypt_card() == Some(carna))
            {
                let owner = e.state.minions[m.index()].owner;
                e.state.cards.push(zvtes::state::CardInPlay::new(
                    zvtes::state::find_library("Raven Spy"),
                    owner,
                    Some(m),
                    0,
                ));
            }
        },
        seeds: 120,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 15 (slice 16aa): the library cards.
    random_games_with_twda_round_15a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Arcanum Chapterhouse, Alexandria",
            "Burnt Offerings",
            "Corporal Reservoir",
            "Curse of Nitocris",
            "Derange",
            "Descent into Darkness",
            "Diabolic Lure",
            "Domain of Evernight",
            "Dual Form",
            "Engling Fury",
            "Eye of Hazimel",
            "Flurry of Action",
            "Hag's Wrinkles",
            "Into Thin Air",
            "Madness Network",
            "Malkavian Dementia",
            "Mantle of the Bestial Majesty",
            "Mythic Form",
            "Outside the Hourglass",
            "Pocket Out of Time",
            "Stunt Cycle",
            "Taking the Skin: Minion",
            "Tattoo Signal",
            "Thrown Gate",
            "Tye Cooper",
            "Typhonic Beast",
            // For Hag's Wrinkles' equip actions and Arcanum Chapterhouse's
            // hunting grounds.
            "Kevlar Vest",
            ".44 Magnum",
            "Flak Jacket",
            "Base Hunting Ground",
        ],
        covers: &[
            "play:Arcanum Chapterhouse, Alexandria",
            "play:Burnt Offerings",
            "play:Corporal Reservoir",
            "play:Curse of Nitocris",
            "play:Derange",
            "play:Descent into Darkness",
            "play:Domain of Evernight",
            "play:Dual Form",
            "play:Engling Fury",
            "play:Eye of Hazimel",
            "play:Flurry of Action",
            "play:Hag's Wrinkles",
            "play:Into Thin Air",
            "play:Madness Network",
            "play:Malkavian Dementia",
            "play:Mantle of the Bestial Majesty",
            "play:Mythic Form",
            "play:Outside the Hourglass",
            "play:Pocket Out of Time",
            "play:Stunt Cycle",
            "play:Tattoo Signal",
            "play:Thrown Gate",
            "play:Tye Cooper",
            "play:Typhonic Beast",
            "ability:Corporal Reservoir",
            "ability:Descent into Darkness",
            "ability:Eye of Hazimel",
            "ability:Madness Network",
            "play:Diabolic Lure",
            "play:Taking the Skin: Minion",
            "ability:Dual Form",
            "ability:Derange",
            "ability:Into Thin Air",
            "ability:Pocket Out of Time",
            "ability:Tattoo Signal",
        ],
        setup: |e, _| {
            let has = |c: &CryptCard, d: &str| c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d));
            per_seat(e, |c| c.clan.starts_with("Malkavian"), 4);
            per_seat(e, |c| c.clan == "Giovanni", 4);
            per_seat(e, |c| c.seraph || c.black_hand, 4);
            per_seat(e, |c| c.infernal && has(c, "dai"), 4);
            per_seat(e, |c| has(c, "obf") && has(c, "tem"), 4);
            per_seat(e, |c| has(c, "obt") && has(c, "tem"), 4);
            per_seat(e, |c| has(c, "pro"), 4);
            per_seat(e, |c| has(c, "pot") && has(c, "ser"), 4);
            per_seat(e, |c| has(c, "cel"), 4);
            per_seat(e, |c| has(c, "aus"), 4);
            per_seat(e, |c| has(c, "abo"), 4);
            per_seat(e, |c| has(c, "for") || has(c, "spi"), 4);
            per_seat(e, |c| has(c, "thn") || has(c, "obf"), 4);
        },
        seeds: 300,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 15 (slice 16aa): the vampires.
    random_games_with_twda_round_15b_hold_invariants_and_replay: Fixture {
        // Dominate and Auspex cards for Edward Vignes and Katherine
        // Stoddard, a master for Walker Grimes, a political action for
        // Donald Cargill, Celerity for Shalmath's bar.
        slice: &[
            "Deflection",
            "Eyes of Argus",
            "Kine Resources Contested",
            "Villein",
            "Line Brawl",
            "Political Ally",
        ],
        covers: &[
            "ability:Abdelsobek",
            "ability:Ghede",
            "ability:Juniper",
            "ability:Saul Meira",
            "ability:Feo Ramos",
            "ability:Shalmath",
            "ability:Cybele",
            "ability:Edward Vignes",
            "ability:Iris Bennett",
            "ability:Katherine Stoddard",
            "ability:Bolesław Gutowski",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Abdelsobek",
                    "Ghede",
                    "Juniper",
                    "Saul Meira",
                    "Feo Ramos",
                    "Shalmath",
                    "Cybele",
                    "Edward Vignes",
                    "Iris Bennett",
                    "Katherine Stoddard",
                    "Bolesław Gutowski",
                    "César Holfield",
                    "Donald Cargill",
                    "Lubomira Hradok",
                    "Walker Grimes",
                    "Skidmark",
                    "Franciscus",
                    "Appius Claudius Corvus",
                    "Onaedo",
                    "Tegyrius, Vizier",
                ],
            );
            per_seat(e, |c| c.sect == Some(zvtes::crypt::Sect::Sabbat) && c.capacity >= 4, 4);
            per_seat(e, |c| c.title != zvtes::crypt::Title::None, 4);
        },
        seeds: 100,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 16 (slice 16ab): the library cards.
    random_games_with_twda_round_16a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Corpse Minion",
            "Dark Web Procurer",
            "Death Seeker",
            "Decapitate",
            "Fast Reaction",
            "Fee Stake: Perth",
            "Gran Madre di Dio, Italy",
            "Institution Hunting Ground",
            "IR Goggles",
            "Kindred Intelligence",
            "Light Intensifying Goggles",
            "Malkavian Rider Clause",
            "Night Moves",
            "Nosferatu Bestial",
            "Nosferatu Kingdom",
            "Patagia: Flaps Allowing Limited Flight",
            "Ponticulus",
            "Praxis Seizure: York",
            "Righteous Aura",
            "Rumble",
            "Seal of Veddartha",
            "Sense Death",
            "Smash and Grab",
            "Tremere Convocation",
            "Vaticination",
            "Vivienne Géroux",
            "Wrong and Crosswise",
            // An aim card for the Goggles' store, an Animalism retainer for
            // the Bestial, a political action for the Rider Clause and
            // Wrong and Crosswise.
            "Target Vitals",
            "Raven Spy",
            "Kine Resources Contested",
        ],
        covers: &[
            "play:Corpse Minion",
            "play:Dark Web Procurer",
            "play:Death Seeker",
            "play:Decapitate",
            "play:Fast Reaction",
            "play:Fee Stake: Perth",
            "play:Gran Madre di Dio, Italy",
            "play:Institution Hunting Ground",
            "play:IR Goggles",
            "play:Kindred Intelligence",
            "play:Light Intensifying Goggles",
            "play:Malkavian Rider Clause",
            "play:Night Moves",
            "play:Nosferatu Bestial",
            "play:Nosferatu Kingdom",
            "play:Patagia: Flaps Allowing Limited Flight",
            "play:Ponticulus",
            "play:Praxis Seizure: York",
            "play:Righteous Aura",
            "play:Rumble",
            "play:Seal of Veddartha",
            "play:Sense Death",
            "play:Smash and Grab",
            "play:Tremere Convocation",
            "play:Vaticination",
            "play:Vivienne Géroux",
            "play:Wrong and Crosswise",
            "ability:Corpse Minion",
            "ability:Fee Stake: Perth",
            "ability:Institution Hunting Ground",
            "ability:IR Goggles",
            "ability:Light Intensifying Goggles",
            "ability:Nosferatu Bestial",
            "ability:Nosferatu Kingdom",
            "ability:Patagia: Flaps Allowing Limited Flight",
            "ability:Ponticulus",
            "ability:Seal of Veddartha",
            "ability:Tremere Convocation",
            "ability:Vivienne Géroux",
        ],
        setup: |e, _| {
            let has = |c: &CryptCard, d: &str| c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d));
            use zvtes::crypt::Sect;
            per_seat(e, |c| c.clan == "Tremere", 4);
            per_seat(e, |c| c.clan == "Salubri antitribu", 4);
            per_seat(e, |c| c.clan == "Nosferatu", 4);
            per_seat(e, |c| c.clan == "Nosferatu antitribu", 4);
            per_seat(e, |c| c.clan == "Malkavian antitribu", 4);
            per_seat(e, |c| c.clan == "Malkavian", 4);
            per_seat(e, |c| c.clan == "Banu Haqim", 4);
            per_seat(e, |c| c.clan == "True Brujah", 4);
            per_seat(e, |c| c.sect == Some(Sect::Anarch) && c.capacity >= 5, 4);
            per_seat(e, |c| c.sect == Some(Sect::Camarilla) && c.capacity >= 6, 4);
            per_seat(e, |c| c.sect == Some(Sect::Sabbat), 4);
            per_seat(e, |c| has(c, "aus") && has(c, "dem"), 4);
            per_seat(e, |c| has(c, "obf"), 4);
            per_seat(e, |c| has(c, "pot"), 4);
            per_seat(e, |c| has(c, "ani"), 4);
            // A Nosferatu antitribu to influence, for Nosferatu Kingdom.
            let antitribu: Vec<u16> = CRYPT
                .iter()
                .enumerate()
                .filter(|(i, c)| c.clan == "Nosferatu antitribu" && absent(e, *i as u16))
                .map(|(i, _)| i as u16)
                .take(4)
                .collect();
            for (s, card) in antitribu.into_iter().enumerate() {
                e.state.seats[s].uncontrolled.push(zvtes::state::UncontrolledSlot {
                    card,
                    copy: Default::default(),
                    counters: 0,
                    banished: None,
                });
            }
            // A Corpse Minion on each seat's first vampire: employed in play,
            // its intercept came only on a contest with oneself (rules-qa
            // 100.1) at 300 seeds.
            for s in 0..e.state.seats.len() {
                let owner = SeatId(s as u8);
                if let Some(m) = (0..e.state.minions.len())
                    .map(|i| zvtes::ids::MinionId(i as u32))
                    .find(|m| e.state.minions[m.index()].owner == owner)
                {
                    e.state.cards.push(zvtes::state::CardInPlay::new(
                        zvtes::state::find_library("Corpse Minion"),
                        owner,
                        Some(m),
                        0,
                    ));
                }
            }
        },
        seeds: 240,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 16 (slice 16ab): the vampires.
    random_games_with_twda_round_16b_hold_invariants_and_replay: Fixture {
        // Equipment for Marie Faucigny, a location for General Perfidio
        // Díos to guard, a political action for Fiorenza Savona.
        slice: &[
            "Laptop Computer",
            ".44 Magnum",
            "Conceal",
            "Kine Resources Contested",
            "Villein",
        ],
        covers: &[
            "ability:Qawiyya el-Ghaduba",
            "ability:Okulos",
            "ability:General Perfidio Díos",
            "ability:Nefertiti",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Aisling Sturbridge",
                    "Blythe Candeleria",
                    "Dr. Solomon Grey",
                    "General Perfidio Díos",
                    "Kite",
                    "Marie Faucigny",
                    "Qawiyya el-Ghaduba",
                    "Richard Tauber, Ayelea's Puppet",
                    "Rodolfo",
                    "Fiorenza Savona",
                    "Okulos",
                    "Len Konopka",
                ],
            );
            per_seat(
                e,
                |c| c.adv && ["Karsh", "Menele", "Yazid Tamari", "Nefertiti"].contains(&c.name),
                4,
            );
            per_seat(e, |c| c.clan == "Tremere", 4);
            per_seat(e, |c| c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("obf")), 4);
            // General Perfidio Díos locked, with a location to guard.
            let g = find_card("General Perfidio Díos");
            if let Some(i) = e.state.minions.iter().position(|m| m.crypt_card() == Some(g)) {
                e.state.minions[i].locked = true;
                let owner = e.state.minions[i].owner;
                e.state.cards.push(zvtes::state::CardInPlay::new(
                    zvtes::state::find_library("Palatial Estate"),
                    owner,
                    None,
                    0,
                ));
            }
        },
        seeds: 10,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 17 (slice 16ac): the library cards.
    random_games_with_twda_round_17a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Alvusia",
            "Argent Baton",
            "As the Crow",
            "Black Sunrise",
            "Botched Move",
            "Call the Wild Hunt",
            "Centralized Background Check",
            "Cold Aura",
            "Command of the Beast",
            "Crematorium",
            "Dam the Heart's River",
            "Dis Pater",
            "Dive Bomb",
            "Earthshock",
            "Ensemble",
            "Filchware's Pawn Shop",
            "Gianna di Canneto",
            "Giulia Giovanni Abruzzina",
            "Guide and Mentor",
            "Hand Contract",
            "Increased Strength",
            "Infamous Insurgent",
            "Lapse",
            "Leadership Vacuum",
            "Legion",
            "Morgue Hunting Ground",
            "Path of Bone, The",
            "Path of the Scorched Heart, The",
            "Port Authority",
            "Promise of 1528",
            "Prophecies of Gehenna",
            "Provision of the Silsila",
            "Public Enemy",
            "Raw Recruit",
            "Retribution",
            "Rockheart",
            "Ruins of Villers Abbey, Belgium",
            "Shakar",
            "Spiritual Intervention",
            "Spontaneous Power",
            "Stonestrength",
            "Therbold Realty",
            "Thin Blood",
            "Thoughts Betrayed",
            "Waxen Poetica",
            // A Master: Discipline for Legion, an archetype for Guide and
            // Mentor, an animal for Call the Wild Hunt, equipment for
            // Gianna, Filchware's and the Background Check.
            "Abombwe",
            "Monster",
            "Dog Pack",
            ".44 Magnum",
            // A contract for Provision of the Silsila (its chain too rare
            // for a key: its tests cover it).
            "Priority Contract",
        ],
        covers: &[
            "play:Alvusia",
            "play:Argent Baton",
            "play:As the Crow",
            "play:Black Sunrise",
            "play:Botched Move",
            "play:Call the Wild Hunt",
            "play:Centralized Background Check",
            "play:Cold Aura",
            "play:Command of the Beast",
            "play:Crematorium",
            "play:Dam the Heart's River",
            "play:Dis Pater",
            "play:Dive Bomb",
            "play:Earthshock",
            "play:Ensemble",
            "play:Filchware's Pawn Shop",
            "play:Gianna di Canneto",
            "play:Giulia Giovanni Abruzzina",
            "play:Guide and Mentor",
            "play:Hand Contract",
            "play:Increased Strength",
            "play:Infamous Insurgent",
            "play:Lapse",
            "play:Leadership Vacuum",
            "play:Legion",
            "play:Morgue Hunting Ground",
            "play:Path of Bone, The",
            "play:Path of the Scorched Heart, The",
            "play:Port Authority",
            "play:Promise of 1528",
            "play:Prophecies of Gehenna",
            "play:Public Enemy",
            "play:Raw Recruit",
            "play:Retribution",
            "play:Rockheart",
            "play:Ruins of Villers Abbey, Belgium",
            "play:Shakar",
            "play:Spiritual Intervention",
            "play:Spontaneous Power",
            "play:Stonestrength",
            "play:Therbold Realty",
            "play:Thin Blood",
            "play:Thoughts Betrayed",
            "play:Waxen Poetica",
            "ability:Alvusia",
            "ability:Argent Baton",
            "ability:Crematorium",
            "ability:Dis Pater",
            "ability:Filchware's Pawn Shop",
            "ability:Gianna di Canneto",
            "ability:Giulia Giovanni Abruzzina",
            "ability:Guide and Mentor",
            "ability:Hand Contract",
            "ability:Infamous Insurgent",
            "ability:Legion",
            "ability:Morgue Hunting Ground",
            "ability:Path of Bone, The",
            "ability:Prophecies of Gehenna",
            "ability:Raw Recruit",
            "ability:Ruins of Villers Abbey, Belgium",
            "ability:Shakar",
            "ability:Waxen Poetica",
        ],
        setup: |e, _| {
            let has = |c: &CryptCard, d: &str| c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d));
            use zvtes::crypt::Sect;
            per_seat(e, |c| c.clan == "Tremere", 4);
            per_seat(e, |c| c.clan == "Gargoyle", 4);
            per_seat(e, |c| c.clan == "Giovanni", 4);
            per_seat(e, |c| c.clan == "Banu Haqim", 4);
            per_seat(e, |c| c.clan == "Daughter of Cacophony", 4);
            per_seat(e, |c| c.clan == "Harbinger of Skulls", 4);
            per_seat(e, |c| c.clan == "True Brujah", 4);
            per_seat(e, |c| c.black_hand, 4);
            per_seat(e, |c| c.sect == Some(Sect::Camarilla) && c.capacity >= 6, 4);
            per_seat(e, |c| c.sect == Some(Sect::Anarch), 4);
            per_seat(e, |c| c.title != zvtes::crypt::Title::None, 4);
            per_seat(e, |c| c.capacity >= 8, 4);
            per_seat(e, |c| has(c, "ani") && has(c, "pro"), 4);
            // Giulia beside seat 0's Giovanni: her lock needs one in combat.
            e.add_ally(SeatId(0), zvtes::state::find_library("Giulia Giovanni Abruzzina"));
            // Dis Pater in play there, seven ash heap cards to remove.
            e.state.cards.push(zvtes::state::CardInPlay::new(
                zvtes::state::find_library("Dis Pater"),
                SeatId(0),
                None,
                0,
            ));
            for _ in 0..7 {
                e.state.seats[0].ash.push(zvtes::state::find_library("Conceal"));
            }
        },
        seeds: 150,
        ..BASE
    };
    /// Legacy cards for TWDA decks, round 17 (slice 16ac): the vampires.
    random_games_with_twda_round_17b_hold_invariants_and_replay: Fixture {
        // Equipment for Martino, Baldesar, Fatima and White Lily, the Eye
        // for Lambach, a location for Guillaume and Porphyry, a political
        // action for Jessica and Reverend Adams, a werewolf for Joe, a
        // Blood Sorcery card for Claudio's discard.
        slice: &[
            ".44 Magnum",
            "Laptop Computer",
            "Eye of Hazimel",
            "Palatial Estate",
            "Kine Resources Contested",
            "Conceal",
            "Renegade Garou",
            "Apportation",
        ],
        covers: &[
            "ability:Angelo",
            "ability:Anu Diptinatpa",
            "ability:Barbaro Lucchese",
            "ability:Claudio Severino",
            "ability:Fatima al-Faqadi",
            "ability:Hannibal",
            "ability:Joe \"Boot\" Hill",
            "ability:Lambach",
            "ability:Nu, The Pillar",
            "ability:Porphyry",
            "ability:Verbruch",
            "ability:White Lily",
            "ability:Xaviar",
            "ability:Yong-Sun, Harmonist",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Angelo",
                    "Anu Diptinatpa",
                    "Baldesar Rossellini",
                    "Barbaro Lucchese",
                    "Claudio Severino",
                    "Dr. Oliver Genet",
                    "Echo",
                    "Fatima al-Faqadi",
                    "Genina, The Red Poet",
                    "Guillaume Giovanni",
                    "Joe \"Boot\" Hill",
                    "Kestrelle Hayes",
                    "Kisha Bhimji",
                    "Lucita",
                    "Martino della Passaglia",
                    "Mitchell, The Headhunter",
                    "Nu, The Pillar",
                    "Porphyry",
                    "Reverend Adams",
                    "Sascha Vykos, The Angel of Caine",
                    "Tori Longwood",
                    "Verbruch",
                    "Victor Pelletier",
                    "White Lily",
                    "Yong-Sun, Harmonist",
                ],
            );
            per_seat(e, |c| c.name == "Xaviar" && !c.adv, 4);
            // A werewolf at every seat, for Joe "Boot" Hill's rush.
            for s in 0..4 {
                e.add_ally(SeatId(s), zvtes::state::find_library("Renegade Garou"));
            }
            per_seat(
                e,
                |c| c.adv && ["Alan Sovereign", "Hannibal", "Jessica", "Lambach"].contains(&c.name),
                4,
            );
            // Hesha's groups are one vampire: G2 only if no Hesha is dealt.
            if !e.state.minions.iter().any(|m| m.name() == "Hesha Ruhadze") {
                per_seat(e, |c| c.name == "Hesha Ruhadze" && c.group == 2, 4);
            }
            per_seat(e, |c| c.clan == "Tremere", 4);
            per_seat(e, |c| c.clan == "Werewolf" || c.clan == "Gargoyle", 4);
        },
        // Seed 156: White Lily paying her Methuselah's last pool in
        // another's turn.
        seeds: 157,
        ..BASE
    };
    /// Legacy cards, round 18 (slice 16ad): the library cards.
    random_games_with_twda_round_18a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Admonitions, The",
            "Communal Haven: Cathedral",
            "Conflagration",
            "Creation Rites",
            "Crusade: Chicago",
            "Crusade: Istanbul",
            "Crusade: London",
            "Dodge",
            "Eternal Vigilance",
            "Hatch the Viper",
            "Helicopter",
            "Hidden Lurker",
            "Improvised Flamethrower",
            "Leverage",
            "Nosferatu Justicar",
            "NSA Trio",
            "Sacrificial Lamb",
            "Secret Library of Alexandria, The",
            "Shanty Town Hunting Ground",
            "Storage Annex",
            // A gun striking at long range, a Master: Discipline card for
            // Creation Rites' search.
            ".44 Magnum",
            "Abombwe",
        ],
        covers: &[
            "play:Admonitions, The",
            "play:Communal Haven: Cathedral",
            "play:Conflagration",
            "play:Creation Rites",
            "play:Crusade: Chicago",
            "play:Crusade: Istanbul",
            "play:Crusade: London",
            "play:Dodge",
            "play:Eternal Vigilance",
            "play:Hatch the Viper",
            "play:Helicopter",
            "play:Hidden Lurker",
            "play:Improvised Flamethrower",
            "play:Leverage",
            "play:Nosferatu Justicar",
            "play:NSA Trio",
            "play:Sacrificial Lamb",
            "play:Secret Library of Alexandria, The",
            "play:Shanty Town Hunting Ground",
            "play:Storage Annex",
            "ability:Admonitions, The",
            "ability:Communal Haven: Cathedral",
            "ability:Eternal Vigilance",
            "ability:Hatch the Viper",
            "ability:Helicopter",
            "ability:Improvised Flamethrower",
            "ability:Secret Library of Alexandria, The",
            "ability:Shanty Town Hunting Ground",
            "ability:Storage Annex",
        ],
        setup: |e, _| {
            use zvtes::crypt::{Sect, Title};
            let has = |c: &CryptCard, d: &str| c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d));
            per_seat(
                e,
                |c| c.sect == Some(Sect::Sabbat)
                    && matches!(c.title, Title::Archbishop(_) | Title::Priscus | Title::Cardinal | Title::Regent),
                4,
            );
            per_seat(e, |c| c.black_hand, 4);
            per_seat(e, |c| c.sect == Some(Sect::Sabbat), 4);
            per_seat(e, |c| c.clan == "Nosferatu" && c.sect == Some(Sect::Camarilla), 4);
            per_seat(e, |c| c.clan == "Nosferatu antitribu", 4);
            per_seat(e, |c| has(c, "obf"), 4);
            per_seat(e, |c| has(c, "pro") && has(c, "ser"), 4);
            per_seat(e, |c| has(c, "pot") || has(c, "dai"), 4);
        },
        seeds: 10,
        ..BASE
    };
    /// Legacy cards, round 18 (slice 16ad): the vampires.
    random_games_with_twda_round_18b_hold_invariants_and_replay: Fixture {
        // Cards with Celerity for Antoinette's surcharge, a weapon for the
        // combats.
        slice: &[".44 Magnum", "Side Strike", "Dodge"],
        covers: &[
            "ability:Eurayle Gelasia Mylonas",
            "ability:Fergus Alexander",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Antoinette DuChamp",
                    "Bobby Lemon",
                    "Chandler Hungerford",
                    "Christine Boscacci",
                    "Eurayle Gelasia Mylonas",
                    "Fergus Alexander",
                    "Greensleeves",
                    "Julius",
                    "Lukas",
                    "Mirembe Kabbada",
                    "Navar McClaren",
                    "Ohanna",
                    "Samson",
                    "Smudge the Ignored",
                ],
            );
            per_seat(e, |c| c.sect == Some(zvtes::crypt::Sect::Sabbat), 4);
            per_seat(e, |c| c.clan == "Gangrel", 4);
        },
        seeds: 8,
        ..BASE
    };    /// Legacy cards, round 19 (slice 16ae): the powers and convictions, on
    /// imbued covering every virtue.
    random_games_with_twda_round_19a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Abjure",
            "Champion",
            "Discern",
            "Donate",
            "Foresee",
            "Hide",
            "Illuminate",
            "Inspire",
            "Project",
            "Rejuvenate",
            "Respire",
            "Shame",
            "Surge",
            "Vigilance",
            "React with Conviction",
            "Second Sight",
            "Strike with Conviction",
        ],
        covers: &[
            "play:Abjure",
            "play:Champion",
            "play:Discern",
            "play:Donate",
            "play:Foresee",
            "play:Hide",
            "play:Illuminate",
            "play:Inspire",
            "play:Project",
            "play:Rejuvenate",
            "play:Respire",
            "play:Shame",
            "play:Surge",
            "play:Vigilance",
            "play:React with Conviction",
            "play:Second Sight",
            "play:Strike with Conviction",
            "ability:Abjure",
            "ability:Champion",
            "ability:Discern",
            "ability:Donate",
            "ability:Foresee",
            "ability:Hide",
            "ability:Illuminate",
            "ability:Inspire",
            "ability:Project",
            "ability:Rejuvenate",
            "ability:Respire",
            "ability:Surge",
            "ability:Vigilance",
            "ability:Second Sight",
            "ability:Strike with Conviction",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Anna \"Dictatrix11\" Suljic",
                    "Béatrice \"Oracle171\" Tremblay",
                    "Earl \"Shaka74\" Deams",
                    "Erick \"Shophet125\" Franco",
                    "François \"Warden\" Loehr",
                    "Inez \"Nurse216\" Villagrande",
                    "Jack \"Hannibal137\" Harmon",
                    "Jennie \"Cassie247\" Orne",
                    "Jennifer \"Flame61\" Vidisania",
                    "John \"Cop90\" O'Malley",
                    "Leaf \"Potter116\" Pankowski",
                    "Liz \"Ticket312\" Thornton",
                    "Lupe \"Cabbie22\" Droin",
                    "Maman Boumba",
                    "Marion \"Teacher193\" Perks",
                    "Paul \"Sixofswords29\" Moreton",
                    "Pedro Cortez",
                    "Peter \"Outback295\" Rophail",
                    "Travis \"Traveler72\" Miller",
                    "Xian \"DziDzat155\" Quan",
                ],
            );
        },
        seeds: 200,
        ..BASE
    };
    /// Legacy cards, round 19 (slice 16ae): the imbued, convictions to pay
    /// their costs.
    random_games_with_twda_round_19b_hold_invariants_and_replay: Fixture {
        slice: &["Second Sight", "Strike with Conviction", "Sport Bike"],
        covers: &[
            "play:Second Sight",
            "burn_imbued",
            "ability:Anna \"Dictatrix11\" Suljic",
            "ability:Erick \"Shophet125\" Franco",
            "ability:Jack \"Hannibal137\" Harmon",
            "ability:Jennie \"Cassie247\" Orne",
            "ability:John \"Cop90\" O'Malley",
            "ability:Leaf \"Potter116\" Pankowski",
            "ability:Lupe \"Cabbie22\" Droin",
            "ability:Maman Boumba",
            "ability:Peter \"Outback295\" Rophail",
            "ability:Travis \"Traveler72\" Miller",
            "ability:Xian \"DziDzat155\" Quan",
            "ability:Marion \"Teacher193\" Perks",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Anna \"Dictatrix11\" Suljic",
                    "Béatrice \"Oracle171\" Tremblay",
                    "Earl \"Shaka74\" Deams",
                    "Erick \"Shophet125\" Franco",
                    "François \"Warden\" Loehr",
                    "Inez \"Nurse216\" Villagrande",
                    "Jack \"Hannibal137\" Harmon",
                    "Jennie \"Cassie247\" Orne",
                    "Jennifer \"Flame61\" Vidisania",
                    "John \"Cop90\" O'Malley",
                    "Leaf \"Potter116\" Pankowski",
                    "Liz \"Ticket312\" Thornton",
                    "Lupe \"Cabbie22\" Droin",
                    "Maman Boumba",
                    "Marion \"Teacher193\" Perks",
                    "Paul \"Sixofswords29\" Moreton",
                    "Pedro Cortez",
                    "Peter \"Outback295\" Rophail",
                    "Travis \"Traveler72\" Miller",
                    "Xian \"DziDzat155\" Quan",
                ],
            );
        },
        seeds: 300,
        ..BASE
    };
    /// Legacy cards, round 20 (slice 16af): the cards players asked for,
    /// the library cards (Kiasyd's Mytherceria, the Sabbat's).
    random_games_with_twda_round_20a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Aura Absorption",
            "Catacombs",
            "Chanjelin Ward",
            "Fae Contortion",
            "Fee Stake: Corte",
            "Fee Stake: Los Angeles",
            "Goblinism",
            "Great Symposium",
            "Gremlins",
            "Guarded Rubrics",
            "Harvest Rites",
            "Humanitas",
            "Nocturn",
            "Ravager",
            "Song of Pan",
            "Stone Travel",
            "Watchtower: Four Ride Forth",
            // Targets and partners: equipment for Gremlins, a grapple for
            // Fae Contortion, havens for Goblinism, a changeling for Song
            // of Pan and Black Forest Base, Khobar Towers for Nocturn.
            "Kevlar Vest",
            "Immortal Grapple",
            "Secure Haven",
            "Therbold Realty",
            "Mylan Horseed",
            "Black Forest Base",
            "Khobar Towers, Al-Khubar",
        ],
        covers: &[
            "play:Aura Absorption",
            "play:Catacombs",
            "play:Chanjelin Ward",
            "play:Fae Contortion",
            "play:Fee Stake: Corte",
            "play:Fee Stake: Los Angeles",
            "play:Goblinism",
            "play:Great Symposium",
            "play:Gremlins",
            "play:Guarded Rubrics",
            "play:Harvest Rites",
            "play:Humanitas",
            "play:Nocturn",
            "play:Ravager",
            "play:Song of Pan",
            "play:Stone Travel",
            "play:Watchtower: Four Ride Forth",
            "ability:Chanjelin Ward",
            "ability:Guarded Rubrics",
            "ability:Watchtower: Four Ride Forth",
        ],
        setup: |e, _| {
            let has = |c: &CryptCard, d: &str| c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d));
            per_seat(e, |c| c.clan == "Kiasyd", 4);
            per_seat(e, |c| c.seraph || c.black_hand, 4);
            per_seat(e, |c| has(c, "obt"), 4);
            per_seat(e, |c| has(c, "pot"), 4);
            per_seat(e, |c| has(c, "aus"), 4);
            per_seat(
                e,
                |c| c.sect == Some(zvtes::crypt::Sect::Anarch) && c.capacity >= 5,
                4,
            );
        },
        seeds: 300,
        ..BASE
    };
    /// Legacy cards, round 20 (slice 16af): the vampires, Winterlich with
    /// a changeling to recruit.
    random_games_with_twda_round_20b_hold_invariants_and_replay: Fixture {
        slice: &["Mylan Horseed", "Kevlar Vest", "Flak Jacket", "Dodge"],
        covers: &[
            "ability:Isanwayen",
            "ability:Arcadian, The",
            "ability:Pherydima",
            "ability:Quincy, The Trapper",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Isanwayen",
                    "Arcadian, The",
                    "Pherydima",
                    "Quincy, The Trapper",
                    "Omme Enberbenight",
                    "Roderick Phillips March",
                    "Dame Hollerton",
                    "Andrew Emory",
                    "Banjoko",
                    "Randel, The Coward",
                    "Winterlich",
                ],
            );
        },
        seeds: 80,
        ..BASE
    };
    /// Legacy cards, round 20 (slice 16af): Stone Travel [MYT]'s cancel of
    /// the combat a block begins, the Kiasyd alone (rare at round 20a's
    /// table).
    random_games_with_twda_round_20c_hold_invariants_and_replay: Fixture {
        slice: &["Stone Travel"],
        covers: &["cancel_combat"],
        setup: |e, _| {
            per_seat(e, |c| c.clan == "Kiasyd", 4);
        },
        seeds: 12,
        ..BASE
    };
    /// Legacy cards, round 21 (slice 16ag): Nights of Reckoning's cards,
    /// the imbued against vampires, convictions to pay their costs, the
    /// Embrace for Blood Cult Awareness Network.
    random_games_with_twda_round_21a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Antithesis",
            "Augur",
            "Angel of Berlin",
            "Blood Cult Awareness Network",
            "Bond",
            "Break the Code",
            "Church of Vindicated Faith, The",
            "Cleave",
            "Crusader Sword, The",
            "Determine",
            "Expiate",
            "Imprison",
            "Inflict",
            "Living Wood Staff",
            "Lock",
            "Moise Kasavubu",
            "Muse of Flame",
            "Punish",
            "Rose Foundation, The",
            "Smite",
            "Unity",
            "Vigil: The Thin Line",
            // Partners: convictions to pay with, equipment for Moise and
            // Angel of Berlin, a vampire-making action for the Network.
            "React with Conviction",
            "Strike with Conviction",
            "Embrace, The",
        ],
        covers: &[
            "play:Antithesis",
            "play:Augur",
            "play:Angel of Berlin",
            "play:Blood Cult Awareness Network",
            "play:Bond",
            "play:Break the Code",
            "play:Church of Vindicated Faith, The",
            "play:Cleave",
            "play:Crusader Sword, The",
            "play:Determine",
            "play:Expiate",
            "play:Imprison",
            "play:Inflict",
            "play:Living Wood Staff",
            "play:Lock",
            "play:Moise Kasavubu",
            "play:Muse of Flame",
            "play:Punish",
            "play:Rose Foundation, The",
            "play:Smite",
            "play:Unity",
            "play:Vigil: The Thin Line",
            "ability:Crusader Sword, The",
            "ability:Living Wood Staff",
            "ability:Cleave",
            "ability:Lock",
            "ability:Moise Kasavubu",
            "ability:Unity",
            "ability:Vigil: The Thin Line",
            "ability:Rose Foundation, The",
            "ability:Church of Vindicated Faith, The",
            "ability:Break the Code",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Anna \"Dictatrix11\" Suljic",
                    "Earl \"Shaka74\" Deams",
                    "Jack \"Hannibal137\" Harmon",
                    "Jennifer \"Flame61\" Vidisania",
                    "John \"Cop90\" O'Malley",
                    "Lupe \"Cabbie22\" Droin",
                    "Paul \"Sixofswords29\" Moreton",
                    "Pedro Cortez",
                    "Inez \"Nurse216\" Villagrande",
                ],
            );
            per_seat(e, |c| !c.imbued && c.capacity >= 5, 4);
            // Convictions for Smite and Expiate; an imbued in each
            // uncontrolled region for The Church of Vindicated Faith.
            let conviction = zvtes::state::find_library("React with Conviction");
            for i in 0..e.state.minions.len() {
                let m = &e.state.minions[i];
                if m.in_play && m.is_imbued() {
                    let owner = m.owner;
                    for _ in 0..5 {
                        e.state.cards.push(zvtes::state::CardInPlay::new(
                            conviction,
                            owner,
                            Some(MinionId(i as u32)),
                            0,
                        ));
                    }
                }
            }
            // One incapacitated, for Bond.
            if let Some(i) = e.state.minions.iter().position(|m| m.in_play && m.is_imbued()) {
                e.state.minions[i].incapacitated = true;
                e.state.minions[i].blood = 0;
            }
            let spare = find_card("Peter \"Outback295\" Rophail");
            for s in 0..4 {
                e.state.seats[s].uncontrolled.push(zvtes::state::UncontrolledSlot {
                    card: spare,
                    copy: Default::default(),
                    counters: 1,
                    banished: None,
                });
            }
        },
        seeds: 300,
        ..BASE
    };
    /// Legacy cards, round 21 (slice 16ag): the Kiasyd and Faerie Wards.
    random_games_with_twda_round_21b_hold_invariants_and_replay: Fixture {
        slice: &["Faerie Wards", "Kevlar Vest", "Dodge"],
        covers: &[
            "play:Faerie Wards",
            "ability:Julia Prima",
            "ability:Kassiym Malikhair",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Bartholomew",
                    "Julia Prima",
                    "Kassiym Malikhair",
                    "Marconius",
                    "Winterlich",
                    "Isanwayen",
                ],
            );
            per_seat(e, |c| c.clan == "Pander", 4);
        },
        seeds: 100,
        ..BASE
    };
    /// Round 22 (slice 16ah): Anarch Unbound's library cards and its
    /// Anarchs' abilities, with vehicles and an Anarch's intercept.
    random_games_with_twda_round_22a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Field Training",
            "Illegalism",
            "Netwar",
            "Red Question, The",
            "Zephyr",
            // Partners: vehicles for Vulture, an Anarch's master card for
            // Salvador, an intercept the sect change reads.
            "Sport Bike",
            "Learjet",
            "Anarch Free Press, The",
            "Protection Racket",
        ],
        covers: &[
            "play:Field Training",
            "play:Illegalism",
            "play:Netwar",
            "play:Red Question, The",
            "play:Zephyr",
            "ability:Javier Montoya",
            "ability:Daliah",
            "ability:Vulture",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Daliah",
                    "Gengis",
                    "Javier Montoya",
                    "Jenna Cross",
                    "Salvador Garcia",
                    "Vulture",
                    "Marguerite Foccart",
                    "Boss Callihan",
                ],
            );
            per_seat(e, |c| c.sect == Some(zvtes::crypt::Sect::Camarilla), 4);
        },
        seeds: 8,
        ..BASE
    };
    /// Round 22 (slice 16ah): Victor Gerard and Philip van Vermeer IV
    /// playing as a Camarilla vampire, a prince and a Ventrue.
    random_games_with_twda_round_22b_hold_invariants_and_replay: Fixture {
        slice: &[
            "Justicar Retribution",
            "Second Tradition: Domain",
            "Ventrue Headquarters",
            "Political Ally",
        ],
        covers: &[
            "play:Justicar Retribution",
            "play:Ventrue Headquarters",
        ],
        setup: |e, _| {
            named(
                e,
                &["Victor Gerard", "Philip van Vermeer IV", "Boss Callihan"],
            );
            per_seat(e, |c| c.sect == Some(zvtes::crypt::Sect::Sabbat), 4);
        },
        seeds: 8,
        ..BASE
    };
    /// Round 23 (slice 16ai): The Unaligned's combat and bleed cards, Gift
    /// of Proteus and Nar-Sheptha, with strikes and stealth to meet them.
    random_games_with_twda_round_23a_hold_invariants_and_replay: Fixture {
        slice: &[
            "Hall of Mirrors",
            "Blight",
            "Serpent's Numbing Kiss",
            "Steely Tenacity",
            "Suspension of Disbelief",
            "Ophidian Gaze",
            "Gift of Proteus",
            "Nar-Sheptha",
            // Partners: strikes Hall of Mirrors cancels, Protean cards for
            // the Gift, a redirect Suspension taxes.
            "Body Flare",
            "Wolf Claws",
            "Form of Mist",
            "Deflection",
        ],
        covers: &[
            "play:Hall of Mirrors",
            "play:Blight",
            "play:Serpent's Numbing Kiss",
            "play:Steely Tenacity",
            "play:Suspension of Disbelief",
            "play:Ophidian Gaze",
            "play:Gift of Proteus",
            "play:Nar-Sheptha",
            "ability:Glycon",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Delizbieta of the Dark Eyes",
                    "Count Jocalo",
                    "Rosa Hernández",
                    "Abid bin Haji, Scholar of Mirrors",
                    "Iago Castille",
                    "Legba",
                    "Art Morgan",
                    "Madeleine Giovanni",
                    "Glycon",
                    "Karmenita Yoryari",
                ],
            );
            per_seat(e, |c| c.sect == Some(zvtes::crypt::Sect::Anarch), 4);
        },
        seeds: 8,
        ..BASE
    };
    /// Round 23 (slice 16ai): The Unaligned's politics and masters, its
    /// Giovanni and Followers of Set, Vestgeir.
    random_games_with_twda_round_23b_hold_invariants_and_replay: Fixture {
        slice: &[
            "Velvet Tongue",
            "Skaald",
            "From a Sinking Ship",
            "Conniver",
            "Weirding Stone",
            "In Memory of the Two Lands",
            "Funeral Wake",
            // Partners: a referendum to vote in, a mummy and a wraith, Auspex
            // cards for Rebekah.
            "Ancilla Empowerment",
            "Saatet-ta",
            "Screamer",
            "Eyes of Argus",
        ],
        covers: &[
            "play:Velvet Tongue",
            "play:Skaald",
            "play:From a Sinking Ship",
            "play:Conniver",
            "play:Weirding Stone",
            "play:In Memory of the Two Lands",
            "play:Funeral Wake",
            "ability:Enzo Giovanni, Pentex Board of Directors",
            "ability:Rebekah",
            "ability:Lupo Giovanni",
            "ability:Weirding Stone",
        ],
        setup: |e, _| {
            named(
                e,
                &[
                    "Legba",
                    "Elizabeth Dimitros",
                    "Enzo Giovanni, Pentex Board of Directors",
                    "Rebekah",
                    "Imanna",
                    "Vestgeir",
                    "Lupo Giovanni",
                    "Lisandro Giovanni",
                    "Rosa Hernández",
                    "Dr. Sutton Gassaway",
                    "Bez Dunsirn",
                ],
            );
        },
        seeds: 36,
        ..BASE
    };
}
