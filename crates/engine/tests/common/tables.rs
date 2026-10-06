//! The random tables the fuzz fixtures start from (DESIGN §7.2), shared by
//! the fuzz and elision binaries: `table` builds a seeded mid-game position
//! over a library slice.

use zvtes::crypt::{CRYPT, PRECONS};
use zvtes::game::Engine;
use zvtes::ids::SeatId;
use zvtes::rng::Rng;
use zvtes::state::{find_card, find_library, CardInPlay, UncontrolledSlot, HAND_SIZE};

/// A small set of unused V5 vampires whose superior disciplines cover
/// `discs`: each pick is the vampire covering the most still-missing ones.
/// The vampires the fixture tables pick from: the V5 crypt and the
/// slice-16a additions. Later batches (Reign of Stanislava) stay out, so
/// the tables — and the golden games — hold; they have fixtures of their
/// own.
pub const FIXTURE_CRYPT: usize = 201;

pub fn cover(discs: &[&str], used: &[u16]) -> Vec<u16> {
    let mut missing: Vec<&str> = discs.to_vec();
    let mut picked = Vec::new();
    while !missing.is_empty() {
        let best = CRYPT
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                *i < FIXTURE_CRYPT && !used.contains(&(*i as u16)) && !picked.contains(&(*i as u16))
            })
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
pub const TITLED: [(&str, &str, &str); 4] = [
    ("Catalina Vega", "Saku Pihlajamäki", "Larissa Moreira"),
    ("Flávio Gonçalves", "Sergio Bueno", "Dowager, The"),
    ("Alicia Cortez", "Valeriya Zinovieva", "Lenny Burkhead"),
    ("Mkhokheli", "Vivian VI", "Kalinda"),
];

/// The midgame tables' library: the defined cards of slices 1–9.
pub const SLICE: &[&str] = &[
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
pub const ALLY_SLICE: &[&str] = &[
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
pub const PRECON_SLICE: &[&str] = &[
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
pub fn table(seed: u64, slice: &[&str], extra: &[&str]) -> Engine {
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

/// `table` may give a seat's torpid vampire the name of a later seat's
/// titled one, both uncontested: no game reaches that, and Graverobbing
/// would put both under one Methuselah. The torpid copy leaves.
pub fn untwin(e: &mut Engine) {
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
