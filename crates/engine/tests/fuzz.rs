//! Random-legal fuzzing with invariants and replay determinism (DESIGN §7):
//! every game terminates within a decision budget, invariants hold at every
//! decision point, and (seed, decision log) resimulation reproduces the exact
//! final state hash. Two starting positions: the rulebook opening, and a
//! seeded mid-game position that reaches the torpor / referendum / contest /
//! oust / card-play paths random play from the opening rarely or never gets
//! to.

use std::collections::HashSet;

use zvtes::agent::RandomLegal;
use zvtes::crypt::{CRYPT, PRECONS};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::{Engine, Step, TURN_LIMIT};
use zvtes::ids::SeatId;
use zvtes::library::LIBRARY;
use zvtes::rng::Rng;
use zvtes::state::{find_card, find_library, CardId, CardInPlay, UncontrolledSlot, HAND_SIZE};

const SEEDS: u64 = 200;
const DECISION_BUDGET: u64 = 200_000;

/// Library-card conservation baseline per seat (library + hand + ash +
/// cards in play), fixed at the start of a game.
fn zone_totals(e: &Engine) -> Vec<usize> {
    e.state
        .seats
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let in_play = e
                .state
                .cards
                .iter()
                .filter(|c| c.owner.index() == i && c.in_play)
                .count();
            s.library.len() + s.hand.len() + s.ash.len() + s.removed.len() + in_play
        })
        .collect()
}

fn invariants(e: &Engine, totals: &[usize], seed: u64, window: WindowLabel) {
    let st = &e.state;
    assert!(
        !st.seats[st.active.index()].ousted,
        "active seat is ousted (seed {seed})"
    );
    let mut edges = 0;
    for (i, s) in st.seats.iter().enumerate() {
        assert!(s.pool >= 0, "negative pool at seat {i} (seed {seed})");
        if s.ousted {
            assert_eq!(s.pool, 0, "ousted seat {i} kept pool (seed {seed})");
            assert!(!s.has_edge, "ousted seat {i} kept the Edge (seed {seed})");
        }
        if s.has_edge {
            edges += 1;
        }
        for slot in &s.uncontrolled {
            assert!(
                slot.counters <= CRYPT[slot.card as usize].capacity,
                "overfilled uncontrolled slot (seed {seed})"
            );
        }
        // Never over hand size — the cards not replaced yet counting
        // against it — except while discarding down (rulebook "Drawing
        // Cards": at once).
        let size = st.hand_size(zvtes::ids::SeatId(i as u8));
        assert!(
            s.ousted
                || s.hand.len() + s.held.total() <= size
                || window == WindowLabel::DiscardDown,
            "hand over size at seat {i} (seed {seed}): {} cards + {} held, size {size}, at {window:?}",
            s.hand.len(),
            s.held.total(),
        );
    }
    // Every card in play is controlled by a living Methuselah.
    for ci in 0..st.cards.len() {
        if st.cards[ci].in_play {
            assert!(
                !st.seats[st.card_controller(CardId(ci as u32)).index()].ousted,
                "card in play controlled by an ousted seat (seed {seed})"
            );
        }
    }
    let now = zone_totals(e);
    for (i, (a, b)) in now.iter().zip(totals).enumerate() {
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
        // Pending plays are out of every zone until they settle: allow a
        // couple of cards in flight per seat (a play and its cancel).
        assert!(
            *a <= *b && *a + 2 >= *b,
            "library cards not conserved at seat {i}: {a} vs {b} (seed {seed})"
        );
    }
    assert!(edges <= 1, "multiple Edges (seed {seed})");
    for (i, m) in st.minions.iter().enumerate() {
        assert!(m.blood <= m.capacity(), "blood over capacity (seed {seed})");
        if m.in_play {
            assert!(
                !st.seats[m.owner.index()].ousted,
                "minion of an ousted seat in play (seed {seed})"
            );
            // Uniqueness: a seat never has two copies of a vampire in play.
            let dup =
                st.minions.iter().enumerate().any(|(j, o)| {
                    j != i && o.owner == m.owner && o.in_play && o.name() == m.name()
                });
            assert!(!dup, "seat holds two copies of {} (seed {seed})", m.name());
        }
        if m.title_lost {
            assert!(
                !m.title_contested,
                "lost title still contested (seed {seed})"
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
/// plays are keyed by the card's printed name.
fn kind(e: &Engine, seat: SeatId, c: &Choice) -> String {
    match c {
        Choice::Play { card, .. } => {
            let id = e.state.seats[seat.index()].hand[*card as usize];
            format!("play:{}", LIBRARY[id as usize].name)
        }
        Choice::UseAbility { card, .. } => {
            let id = e.state.cards[*card as usize].card;
            format!("ability:{}", LIBRARY[id as usize].name)
        }
        Choice::ResolveRider { card } => {
            let id = e.state.cards[*card as usize].card;
            format!("rider:{}", LIBRARY[id as usize].name)
        }
        _ => kind_of(c).to_string(),
    }
}

fn kind_of(c: &Choice) -> &'static str {
    match c {
        Choice::Pass => "pass",
        Choice::BurnEdge => "burn_edge",
        Choice::PayContest(_) => "pay_contest",
        Choice::YieldContest(_) => "yield_contest",
        Choice::Play { .. } => "play",
        Choice::UseAbility { .. } => "use_ability",
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
            ActionKind::Political => "political",
            ActionKind::Card => "card_action",
            ActionKind::BurnCard => "burn_card",
        },
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
        Choice::Transfer { .. } => "transfer",
        Choice::Withdraw { .. } => "withdraw",
        Choice::DrawCrypt => "draw_crypt",
        Choice::InfluenceOut { .. } => "influence_out",
        Choice::HandStrike => "hand_strike",
        Choice::UseManeuver => "use_maneuver",
        Choice::UsePress => "use_press",
        Choice::ContinueAction => "continue_action",
        Choice::Diablerise => "diablerise_blocked",
    }
}

/// Opening position: the rulebook start, 4 seats, random precon crypts and
/// libraries.
fn opening(seed: u64) -> Engine {
    Engine::new(seed, 4)
}

/// Mid-game position: low pools, ready vampires at assorted blood (some at
/// 0, so they can be wounded into torpor when they block), one torpor
/// vampire per seat at assorted blood, a short crypt, fully-funded
/// uncontrolled copies of one shared vampire (card contests) and of the
/// Mannheim prince/baron pair (title contests), the Edge, and the slice-1
/// cards in every hand (master phase, terms, as-played cancels). Random
/// play from the opening never reaches torpor in the card-less pool (a
/// vampire is only ever unlocked at 0 blood on its own turn, where it must
/// hunt), nor an oust; this fixture exercises those paths. Deterministic in
/// `seed`, so replay rebuilds the same start.
/// A small set of unused V5 vampires whose superior disciplines cover
/// `discs`: each pick is the vampire covering the most still-missing ones.
fn cover(discs: &[&str], used: &[u16]) -> Vec<u16> {
    let mut missing: Vec<&str> = discs.to_vec();
    let mut picked = Vec::new();
    while !missing.is_empty() {
        let best = CRYPT
            .iter()
            .enumerate()
            .filter(|(i, _)| !used.contains(&(*i as u16)) && !picked.contains(&(*i as u16)))
            .max_by_key(|(i, c)| {
                let n = missing.iter().filter(|d| c.disciplines.contains(d)).count();
                (n, usize::MAX - *i)
            })
            .map(|(i, _)| i as u16)
            .expect("crypt");
        let discs = CRYPT[best as usize].disciplines;
        missing.retain(|d| !discs.contains(d));
        picked.push(best);
    }
    picked
}

/// Per seat: a Camarilla prince, an Anarch baron, a primogen.
const TITLED: [(&str, &str, &str); 4] = [
    ("Catalina Vega", "Saku Pihlajamäki", "Chrysanthemum"),
    ("Flávio Gonçalves", "Sergio Bueno", "Dowager, The"),
    ("Alicia Cortez", "Valeriya Zinovieva", "Lenny Burkhead"),
    ("Mkhokheli", "Vivian VI", "Rinaldo Albizzi"),
];

fn midgame(seed: u64) -> Engine {
    let mut e = Engine::new(seed, 4);
    let mut rng = Rng::new(seed ^ 0x5eed);
    let shared = find_card("Alice Chen");
    let city = [find_card("Horst von Brühl"), find_card("Aline Gädeke")];
    let slice: Vec<u16> = [
        "Villein",
        "Sudden Reversal",
        "Earth Control",
        "Bonding",
        "Conditioning",
        "Consign to Oblivion",
        "Form of the Cobra",
        "Eyes of Argus",
        "Deflection",
        "Archon Investigation",
        "Homunculus",
        "Flash",
        "Torn Signpost",
        "Roundhouse",
        "Body Flare",
        "Theft of Vitae",
        "Majesty",
        "Side Strike",
        "Blur",
        "Immortal Grapple",
        "Hidden Strength",
        "Soak",
        "Indomitability",
        "Claws of the Dead",
        "Taste of Vitae",
        "Telepathic Tracking",
        "Cats' Guidance",
        "Form of Mist",
        "Kine Resources Contested",
        "Conservative Agitation",
        "Ancilla Empowerment",
        "Anarchist Uprising",
        "Domain Challenge",
        "Neonate Breach",
        "Bewitching Oration",
        "Scalpel Tongue",
        "Reckless Agitation",
        "Camarilla's Iron Fist",
        "Parity Shift",
        "Consanguineous Boon",
        "Anarch Salon",
        "Banishment",
        "Expulsion",
        "Ominous Chorus",
        "Perfect Paragon",
        "Old Friends",
        "Party Out Of Bounds",
        "Protected District",
        "Scorn of Adonis",
        "Telepathic Vote Counting",
        "Delaying Tactics",
        "Closed Session",
        "Voter Captivation",
        "Anarch Revolt",
        "Guardian Angel",
        "Creeping Sabotage",
        "Blood Doll",
        "Vessel",
        "Papillon",
        "Fame",
        "Tension in the Ranks",
        "Smiling Jack, The Anarch",
        "Dreams of the Sphinx",
        "Barrens, The",
        "Coven, The",
        "Constant Revolution",
        "Carfax Abbey",
        "Elder Library",
        "Visit from the Capuchin",
        "Wake with Evening's Freshness",
        "Mirror Walk",
    ]
    .iter()
    .map(|n| find_library(n))
    .collect();
    let mut used = vec![shared, city[0], city[1]];
    for s in 0..4u8 {
        let seat = SeatId(s);
        // Vampires covering (at superior) every discipline the slice cards
        // need, picked greedily so the table stays small; distinct across
        // seats.
        e.state.seats[s as usize].pool = 2 + rng.below(8) as i32;
        for card in cover(
            &[
                "DOM", "AUS", "PRO", "OBL", "CEL", "FOR", "POT", "PRE", "THA", "ANI",
            ],
            &used,
        ) {
            used.push(card);
            let blood = (1 + rng.below(4) as u8).min(CRYPT[card as usize].capacity);
            e.add_minion(seat, card, blood, false);
        }
        // Vote sources and the politics cards' requirements: a Camarilla
        // prince (Toreador princes with PRE for Scorn of Adonis and Voter
        // Captivation, a Lasombra one for Ominous Chorus), an Anarch baron
        // (capacity 5+) and a primogen per seat, distinct cities.
        for name in [
            TITLED[s as usize].0,
            TITLED[s as usize].1,
            TITLED[s as usize].2,
        ] {
            let card = find_card(name);
            used.push(card);
            let blood = (1 + rng.below(4) as u8).min(CRYPT[card as usize].capacity);
            e.add_minion(seat, card, blood, false);
        }
        let (_, list) = PRECONS[(seed as usize + s as usize) % PRECONS.len()];
        let mut pool: Vec<u16> = list
            .iter()
            .map(|&(c, _)| c)
            .filter(|c| !used.contains(c))
            .collect();
        rng.shuffle(&mut pool);
        {
            let card = pool.pop().unwrap();
            used.push(card);
            let blood = (rng.below(3) as u8).min(CRYPT[card as usize].capacity);
            e.add_minion(seat, card, blood, true);
        }
        e.state.seats[s as usize].crypt = pool.split_off(pool.len() - 3);
        e.state.seats[s as usize].uncontrolled = vec![
            UncontrolledSlot {
                card: shared,
                counters: CRYPT[shared as usize].capacity,
                banished: None,
            },
            UncontrolledSlot {
                card: city[s as usize % 2],
                counters: CRYPT[city[s as usize % 2] as usize].capacity,
                banished: None,
            },
        ];
        // Seven slice cards in hand, rotating the mix by seat and seed, and
        // a library of the whole slice (shuffled) so replacements keep
        // offering defined cards all game long.
        let hand = &mut e.state.seats[s as usize].hand;
        hand.clear();
        for k in 0..HAND_SIZE {
            // A stride keeps every hand a mix of action cards and
            // modifiers (consecutive indices would group the types).
            hand.push(slice[(k * 11 + s as usize * 5 + seed as usize) % slice.len()]);
        }
        let mut library = slice.clone();
        rng.shuffle(&mut library);
        e.state.seats[s as usize].library = library;
    }
    e.state.seats[rng.below(4)].has_edge = true;
    // Half the seeds open with a Smiling Jack already fed, so the other
    // seats' per-counter payments (a mandatory rider with choices) are
    // reached without waiting for random play to bring it out and keep it.
    if seed % 2 == 1 {
        e.state.cards.push(CardInPlay {
            card: find_library("Smiling Jack, The Anarch"),
            owner: SeatId(rng.below(4) as u8),
            controller: SeatId(rng.below(4) as u8),
            host: None,
            in_play: true,
            contested: false,
            locked: false,
            counters: 2,
        });
    }
    e
}

fn play(seed: u64, start: fn(u64) -> Engine) -> (Engine, Vec<u32>, HashSet<String>) {
    let mut e = start(seed);
    let totals = zone_totals(&e);
    let mut agent = RandomLegal::new(seed ^ 0x00c0_ffee);
    let mut chosen = HashSet::new();
    loop {
        assert!(
            e.decisions < DECISION_BUDGET,
            "decision budget exhausted (seed {seed})"
        );
        match e.run() {
            Step::Decision(req) => {
                assert!(
                    !e.state.seats[req.seat.index()].ousted,
                    "decision requested from an ousted seat (seed {seed}, {:?})",
                    req.window
                );
                invariants(&e, &totals, seed, req.window);
                for o in &req.options {
                    if let Choice::UseAbility { card, .. } = o {
                        // Spec §1.13.3: nothing printed on a contested card
                        // applies (its suspended layers are the scenario
                        // tests' claim, tests/contests.rs).
                        assert!(
                            !e.state.cards[*card as usize].contested,
                            "ability offered from a contested card (seed {seed})"
                        );
                    }
                }
                let idx = agent.choose(&req);
                chosen.insert(kind(&e, req.seat, &req.options[idx]));
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
                return (e, vps, chosen);
            }
        }
    }
}

fn fuzz(start: fn(u64) -> Engine) -> HashSet<String> {
    let mut seen = HashSet::new();
    // `SEED_FROM=n` starts the sweep at seed n (debugging one failure).
    let from: u64 = std::env::var("SEED_FROM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    for seed in from..SEEDS {
        let (played, vps, chosen) = play(seed, start);
        seen.extend(chosen);
        let hash = played.state.hash();

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
    seen
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

#[test]
fn random_games_from_the_opening_terminate_hold_invariants_and_replay() {
    let seen = fuzz(opening);
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

#[test]
fn random_games_from_midgame_reach_torpor_referendums_contests_plays_and_ousts() {
    let seen = fuzz(midgame);
    covers(
        &seen,
        &[
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
            "play:Closed Session",
            "play:Voter Captivation",
            "play:Anarch Revolt",
            "ability:Anarch Revolt",
            "play:Guardian Angel",
            "ability:Guardian Angel",
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
            // (Smiling Jack's pool unit elides when it is the only source and
            // is seldom picked against several vampires: no key.)
            "burn_blood_from",
            // (Anarch Revolt's and Fame's riders are decisions only when
            // ordered against another unlock effect; alone they elide, so
            // no key.)
        ],
    );
    // Ousts and game wins happen from here: at least one game must end
    // before the time limit with a full VP awarded.
    let ousted = (0..SEEDS).any(|seed| {
        let (e, vps, _) = play(seed, midgame);
        e.state.seats.iter().any(|s| s.ousted) && vps.iter().any(|&v| v >= 2)
    });
    assert!(ousted, "no oust in {SEEDS} mid-game random games");
}
