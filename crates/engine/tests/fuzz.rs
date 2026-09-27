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
use zvtes::crypt::{CryptCard, CRYPT, PRECONS};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::{Engine, Step, TURN_LIMIT};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::rng::Rng;
use zvtes::state::{find_card, find_library, CardId, CardInPlay, UncontrolledSlot, HAND_SIZE};

const SEEDS: u64 = 300;
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
/// plays are keyed by the card's printed name.
fn kind(e: &Engine, seat: SeatId, c: &Choice) -> String {
    match c {
        Choice::Play { card, .. } => {
            let id = e.playable_card(seat, *card);
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
    }
}

/// Opening position: the rulebook start, 4 seats, random precon crypts and
/// libraries.
fn opening(seed: u64) -> Engine {
    Engine::new(seed, 4)
}

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
fn table(seed: u64, slice: &[&str], extra: &[&str]) -> Engine {
    let mut e = Engine::new(seed, 4);
    let mut rng = Rng::new(seed ^ 0x5eed);
    let shared = find_card("Alice Chen");
    let city = [find_card("Horst von Brühl"), find_card("Aline Gädeke")];
    let slice: Vec<u16> = slice.iter().map(|n| find_library(n)).collect();
    let mut used = vec![shared, city[0], city[1]];
    // The titled vampires every seat gets: never picked for Discipline
    // cover too.
    let titled: Vec<u16> = TITLED
        .iter()
        .flat_map(|t| [t.0, t.1, t.2])
        .map(find_card)
        .collect();
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
        let excluded: Vec<u16> = used.iter().chain(&titled).copied().collect();
        for card in cover(&discs, &excluded) {
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
            effect_only: false,
            against: Default::default(),
            entered: 0,
            chosen: Vec::new(),
        });
    }
    e
}

fn play(seed: u64, start: &dyn Fn(u64) -> Engine) -> (Engine, Vec<u32>, HashSet<String>) {
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
                if std::env::var("FUZZ_TRACE").is_ok() {
                    eprintln!(
                        "{:?} {:?} -> {:?} {}",
                        req.seat,
                        req.window,
                        req.options[idx],
                        kind(&e, req.seat, &req.options[idx])
                    );
                }
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

/// Plays and replays every seed: the decision kinds taken, and whether a
/// game ended before the time limit with an oust and a full VP awarded.
fn fuzz(start: &dyn Fn(u64) -> Engine) -> (HashSet<String>, bool) {
    let (mut seen, mut ousted) = (HashSet::new(), false);
    // `SEED_FROM=n` starts the sweep at seed n (debugging one failure).
    let from: u64 = std::env::var("SEED_FROM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    for seed in from..SEEDS {
        let (played, vps, chosen) = play(seed, start);
        seen.extend(chosen);
        ousted |= played.state.seats.iter().any(|s| s.ousted) && vps.iter().any(|&v| v >= 2);
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
    (seen, ousted)
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
}

const BASE: Fixture = Fixture {
    slice: &[],
    extra: &[],
    plays: 0,
    covers: &[],
    untwin: false,
    setup: |_, _| {},
    ousts: false,
};

fn run(f: &Fixture) {
    let (seen, ousted) = fuzz(&|seed| {
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
        "no oust in {SEEDS} mid-game random games"
    );
}

/// `table` may give a seat's torpid vampire the name of a later seat's
/// titled one, both uncontested: no game reaches that, and Graverobbing
/// would put both under one Methuselah. The torpid copy leaves.
fn untwin(e: &mut Engine) {
    let st = &mut e.state;
    for i in 0..st.minions.len() {
        let (name, owner) = (st.minions[i].name(), st.minions[i].owner);
        let twin = st
            .minions
            .iter()
            .any(|o| o.in_play && !o.in_torpor && o.owner != owner && o.name() == name);
        if twin && st.minions[i].in_torpor {
            st.minions[i].in_play = false;
        }
    }
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
            run(&$fixture);
        }
    )*};
}

#[test]
fn random_games_from_the_opening_terminate_hold_invariants_and_replay() {
    let (seen, _) = fuzz(&opening);
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
        ousts: true,
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
                |c| c.clan == "Salubri" && c.sect == zvtes::crypt::Sect::Independent,
                3,
            );
            let valerius = find_card("Valerius");
            if absent(e, valerius) {
                e.add_minion(SeatId((seed % 4) as u8), valerius, 4, false);
            }
        },
        untwin: true,
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
            "ability:Asylum Hunting Ground",
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
            "ability:Ritual Goblet",
        ],
        untwin: true,
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
        ..BASE
    };
}
