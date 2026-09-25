//! Random-legal fuzzing with invariants and replay determinism (DESIGN §7):
//! every game terminates within a decision budget, invariants hold at every
//! decision point, and (seed, decision log) resimulation reproduces the exact
//! final state hash. Two starting positions: the rulebook opening, and a
//! seeded mid-game position that reaches the torpor / referendum / contest /
//! oust / card-play paths random play from the opening rarely or never gets
//! to.

use std::collections::HashSet;

use zvtes::agent::RandomLegal;
use zvtes::cards::AbilitySource;
use zvtes::crypt::{CRYPT, PRECONS};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::{Engine, Step, TURN_LIMIT};
use zvtes::ids::{MinionId, SeatId};
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
            // A card in play with the library cards stored on it, or out of
            // play on a banished vampire (Banishment: they stay with it).
            let banished = |id: usize| {
                e.state.seats.iter().any(|s| {
                    s.uncontrolled.iter().any(|u| {
                        u.banished
                            .as_ref()
                            .is_some_and(|b| b.cards.contains(&CardId(id as u32)))
                    })
                })
            };
            let in_play: usize = e
                .state
                .cards
                .iter()
                .enumerate()
                .filter(|(ci, c)| c.owner.index() == i && (c.in_play || banished(*ci)))
                .map(|(_, c)| c)
                .map(|c| 1 + c.stored.len())
                .sum();
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
                || window == WindowLabel::BottomCards,
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
        assert!(
            m.capacity().is_none_or(|c| m.blood <= c),
            "blood over capacity (seed {seed})"
        );
        if m.in_play {
            assert!(
                !st.seats[m.owner.index()].ousted,
                "minion of an ousted seat in play (seed {seed})"
            );
            // Uniqueness: a seat never has two copies of a vampire, or of a
            // unique ally, in play.
            let unique = m.kind.ally_card().is_none_or(|c| {
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
/// plays are keyed by the card's printed name.
fn kind(e: &Engine, seat: SeatId, c: &Choice) -> String {
    match c {
        Choice::Play { card, .. } => {
            let id = e.state.seats[seat.index()].hand[*card as usize];
            format!("play:{}", LIBRARY[id as usize].name)
        }
        Choice::UseAbility { source, .. } => match source {
            AbilitySource::Card(c) => {
                let id = e.state.cards[c.index()].card;
                format!("ability:{}", LIBRARY[id as usize].name)
            }
            AbilitySource::Vampire(m) => format!("ability:{}", e.state.minions[m.index()].name()),
        },
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
            ActionKind::Recruit => "recruit",
            ActionKind::Political => "political",
            ActionKind::Card => "card_action",
            ActionKind::BurnCard => "burn_card",
            ActionKind::Equip => "equip",
            ActionKind::EquipFrom => "equip_from",
            ActionKind::EquipFound => "equip_found",
            ActionKind::Ability => "ability_action",
        },
        Choice::FindCard(_) => "find_card",
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
        Choice::Transfer { .. } => "transfer",
        Choice::Withdraw { .. } => "withdraw",
        Choice::DrawCrypt => "draw_crypt",
        Choice::InfluenceOut { .. } => "influence_out",
        Choice::HandStrike => "hand_strike",
        Choice::UseManeuver => "use_maneuver",
        Choice::UsePrevent => "use_prevent",
        Choice::UnlockForBlood { .. } => "unlock_for_blood",
        Choice::BurnBloodForIntercept { .. } => "burn_blood_for_intercept",
        Choice::StrikeCombatEnds => "strike_combat_ends",
        Choice::RoundStrike { .. } => "round_strike",
        Choice::PayBlockToll => "pay_block_toll",
        Choice::PayToCancel => "pay_to_cancel",
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
    ("Catalina Vega", "Saku Pihlajamäki", "Larissa Moreira"),
    ("Flávio Gonçalves", "Sergio Bueno", "Dowager, The"),
    ("Alicia Cortez", "Valeriya Zinovieva", "Lenny Burkhead"),
    ("Mkhokheli", "Vivian VI", "Kalinda"),
];

/// The midgame tables' library: the defined cards of slices 1–9.
const SLICE: &[&str] = &[
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
    "Sport Bike",
    "Unlicensed Taxicab",
    "Treasured Samadji",
    "Codex of the Edenic Groundskeepers",
    "Stolen Police Cruiser",
    "Bowl of Convergence",
    ".44 Magnum",
    "AK-47",
    "Assault Rifle",
    "Flamethrower",
    "Ivory Bow",
    "Femur of Toomler",
    "Kali's Fang",
    "Sniper Rifle",
    "Kevlar Vest",
    "Righteous Blade",
    "Magic of the Smith",
    "Vast Wealth",
    "Black Market Cache",
    "Shilmulo Tarot",
    "Heart of Nizchetus",
];

/// The ally tables' library (slice 10): the defined allies and retainers,
/// with strikes, prevention and Protean cards to fight and pay with.
const ALLY_SLICE: &[&str] = &[
    "Political Ally",
    "47th Street Royals",
    "City Star Taxi",
    "Dog Pack",
    "Double Deuce",
    "Feral Hound",
    "Murder of Crows",
    "Raven Spy",
    "Szlachta Bodyguard",
    "Vozhd of Sofia, The",
    "War Ghoul",
    "Bone Shambler",
    "Crypt's Sons",
    "Szlachta Assistant",
    "Underbridge Stray",
    "Vagabond Mystic",
    "Raptor",
    "Screamer",
    "Vozhd of Szczecin, The",
    "Aggressive Corpse",
    "Entrancement",
    "Far Mastery",
    "Invigorate",
    "Libertas",
    "Slaughtering the Herd",
    "Visions of Gehenna",
    "Rotting Behemoth",
    "Vozhd of Gravesend, The",
    "Vozhd of Juiz de Fora, The",
    "Spectral Servitor",
    "Consign to Oblivion",
    "Owl Companion",
    "Homunculus",
    "Roundhouse",
    "Side Strike",
    "Majesty",
    "Theft of Vitae",
    "Hidden Strength",
    "Soak",
    "Form of Mist",
    "Sport Bike",
    "Unlicensed Taxicab",
];

/// The slice 11a tables' library: the pre-built decks' cards built from
/// the existing vocabulary, with a few strikes to fight with.
const PRECON_SLICE: &[&str] = &[
    "Govern the Unaligned",
    "Enchant Kindred",
    "Intimidation",
    "Thing",
    "Heart of the City",
    "Lost in Crowds",
    "Swallowed by the Night",
    "Resist Earth's Grasp",
    "Aire of Elation",
    "Monkey Wrench",
    "Foreshadowing Destruction",
    "Telepathic Misdirection",
    "Bait and Switch",
    "My Enemy's Enemy",
    "Enhanced Senses",
    "Protection Racket",
    "Warrens, The",
    "Earth Meld",
    "Pursuit",
    "Apportation",
    "Arms of Ahriman",
    "Walk of Flame",
    "Quickness",
    "Rego Motum",
    "Life in the City",
    "Misdirection",
    "Information Highway",
    "Labyrinth, The",
    "Elysian Fields",
    "Backways",
    "Opium Den",
    "Market Square",
    "On the Qui Vive",
    "Shadow Sentinel",
    "Guard Dogs",
    "Spirit's Touch",
    "Instinctive Reaction",
    "Precognition",
    "Second Tradition: Domain",
    "Deep Ecology",
    "One With the Land",
    "Form of the Bat",
    "Truth in Darkness",
    "Organized Resistance",
    "Eyes of the Wild",
    "Night Terrors",
    "Stygian Shroud",
    "Forced Confessional",
    "True Love's Face",
    "Where the Veil Thins",
    "Faceless Night",
    "Direct Intervention",
    "Wash",
    "Roundhouse",
    "Side Strike",
    "Soak",
];

fn midgame(seed: u64) -> Engine {
    table(seed, SLICE, &[])
}

/// As `midgame`, the ally tables.
fn allies_midgame(seed: u64) -> Engine {
    table(seed, ALLY_SLICE, &[])
}

/// As `midgame`, the slice 11a tables (Obfuscate covered too).
fn precon_midgame(seed: u64) -> Engine {
    table(seed, PRECON_SLICE, &["OBF"])
}

fn table(seed: u64, slice: &[&str], extra: &[&str]) -> Engine {
    let mut e = Engine::new(seed, 4);
    let mut rng = Rng::new(seed ^ 0x5eed);
    let shared = find_card("Alice Chen");
    let city = [find_card("Horst von Brühl"), find_card("Aline Gädeke")];
    let slice: Vec<u16> = slice.iter().map(|n| find_library(n)).collect();
    let mut used = vec![shared, city[0], city[1]];
    for s in 0..4u8 {
        let seat = SeatId(s);
        // Vampires covering (at superior) every discipline the slice cards
        // need, picked greedily so the table stays small; distinct across
        // seats.
        e.state.seats[s as usize].pool = 2 + rng.below(8) as i32;
        let discs: Vec<&str> = [
            "DOM", "AUS", "PRO", "OBL", "CEL", "FOR", "POT", "PRE", "THA", "ANI",
        ]
        .iter()
        .chain(extra)
        .copied()
        .collect();
        for card in cover(&discs, &used) {
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
            body: None,
            mode: 0,
            by: None,
            in_play: true,
            contested: false,
            locked: false,
            counters: 2,
            stored: Vec::new(),
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
            "ability:Ilonka",
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
            "ability:Ivory Bow",
            "play:Femur of Toomler",
            "play:Kali's Fang",
            "play:Sniper Rifle",
            "ability:Sniper Rifle",
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
            "draw_stored",
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
            "ability:Seraphina",
            "ability:Lenelle, Mambo of Birmingham",
            "ability:Theo Bell",
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

/// Slice 10: random tables whose libraries are the allies and retainers.
#[test]
fn random_games_with_allies_and_retainers_hold_invariants_and_replay() {
    let seen = fuzz(allies_midgame);
    covers(
        &seen,
        &[
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
    );
}

/// Slice 11a: random tables whose libraries are the pre-built decks' cards
/// built from the existing vocabulary.
#[test]
fn random_games_with_precon_cards_hold_invariants_and_replay() {
    let seen = fuzz(precon_midgame);
    covers(
        &seen,
        &[
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
    );
}

/// The slice 11e tables' library: title cards, vote locations and archetypes,
/// with political actions to vote on and a few strikes to fight with.
const TITLES_SLICE: &[&str] = &[
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
];

/// As `midgame`, the slice 11e tables, with an Independent Salubri per
/// seat for Saulot's Guiding Wisdom and Valerius to steal princes' titles.
fn titles_midgame(seed: u64) -> Engine {
    let mut e = table(seed, TITLES_SLICE, &[]);
    // Vampires nowhere on the table yet (no contest to stage).
    let absent =
        |e: &Engine, card: u16| {
            !e.state.minions.iter().any(|m| m.crypt_card() == Some(card))
                && !e.state.seats.iter().any(|s| {
                    s.crypt.contains(&card) || s.uncontrolled.iter().any(|u| u.card == card)
                })
        };
    let salubri: Vec<u16> = CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.clan == "Salubri"
                && c.sect == zvtes::crypt::Sect::Independent
                && absent(&e, *i as u16)
        })
        .map(|(i, _)| i as u16)
        .take(4)
        .collect();
    for (s, &card) in salubri.iter().enumerate() {
        e.add_minion(SeatId(s as u8), card, 3, false);
    }
    let valerius = find_card("Valerius");
    if absent(&e, valerius) {
        e.add_minion(SeatId((seed % 4) as u8), valerius, 4, false);
    }
    e
}

/// Slice 11e: random tables with title cards and vote locations.
#[test]
fn random_games_with_titles_and_votes_hold_invariants_and_replay() {
    let seen = fuzz(titles_midgame);
    covers(
        &seen,
        &[
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
        ],
    );
}

/// The slice 11f tables' library: the combat staples, with weapons, dodges,
/// additional strikes and Fortitude prevention to meet them.
const STAPLES_SLICE: &[&str] = &[
    "Blood Rage",
    "Soul Burn",
    "Carrion Crows",
    "Hunger of Marduk",
    ".44 Magnum",
    "Side Strike",
    "Quickness",
    "Majesty",
    "Soak",
    "Hidden Strength",
    "Roundhouse",
];

/// As `midgame`, the slice 11f tables, with a Banu Haqim of Blood Sorcery
/// per seat for Hunger of Marduk.
fn staples_midgame(seed: u64) -> Engine {
    let mut e = table(seed, STAPLES_SLICE, &[]);
    let absent =
        |e: &Engine, card: u16| {
            !e.state.minions.iter().any(|m| m.crypt_card() == Some(card))
                && !e.state.seats.iter().any(|s| {
                    s.crypt.contains(&card) || s.uncontrolled.iter().any(|u| u.card == card)
                })
        };
    let assassins: Vec<u16> = CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.clan == "Banu Haqim"
                && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("tha"))
                && absent(&e, *i as u16)
        })
        .map(|(i, _)| i as u16)
        .take(4)
        .collect();
    for (s, &card) in assassins.iter().enumerate() {
        e.add_minion(
            SeatId(s as u8),
            card,
            CRYPT[card as usize].capacity.min(4),
            false,
        );
    }
    e
}

/// Slice 11f: random tables with the combat staples.
#[test]
fn random_games_with_combat_staples_hold_invariants_and_replay() {
    let seen = fuzz(staples_midgame);
    covers(
        &seen,
        &[
            "play:Blood Rage",
            "play:Soul Burn",
            "play:Carrion Crows",
            "play:Hunger of Marduk",
            "round_strike",
        ],
    );
}
