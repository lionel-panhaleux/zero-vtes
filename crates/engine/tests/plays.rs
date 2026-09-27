//! Slice 1 of the effects framework: the master phase as an offer window,
//! terms at announcement, the as-played cancel window, cost layers, trifle
//! and out-of-turn accounting, the discard phase. Cards: Villein, Sudden
//! Reversal.

use zvtes::cards::{self, Target, Window};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::state::{find_card, find_library};

fn request(e: &mut Engine) -> DecisionRequest {
    match e.run() {
        Step::Decision(req) => req,
        Step::Over(_) => panic!("game over while expecting a decision"),
    }
}

fn expect(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    let req = request(e);
    assert_eq!((req.seat, req.window), (seat, window), "{:?}", req.options);
    req
}

fn pick(e: &mut Engine, want: Choice) {
    let req = request(e);
    let idx = req
        .options
        .iter()
        .position(|o| *o == want)
        .unwrap_or_else(|| panic!("option {want:?} not in {:?}", req.options));
    e.choose(idx);
}

fn play(card: u8, target: MinionId) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: None,
        target: Target::Minion(target),
    }
}

/// 3 seats at seat 0's master phase on turn 4. Seat 0 has two ready
/// vampires (0: blood 5, 1: blood 1); seat 1 has one (2).
fn table() -> Engine {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), find_card("Abraham DuSable"), 5, false);
    e.add_minion(SeatId(0), find_card("Abraham Mellon"), 1, false);
    e.add_minion(SeatId(1), find_card("Abaddon"), 5, false);
    e.state.turn = 4;
    e.at_master_phase();
    e
}

#[test]
fn implemented_definitions_cover_the_slice() {
    let implemented: Vec<&str> = LIBRARY
        .iter()
        .enumerate()
        .filter(|(i, _)| cards::lookup(*i as u16).is_some())
        .map(|(_, c)| c.name)
        .collect();
    assert_eq!(
        implemented,
        vec![
            ".44 Magnum",
            "47th Street Royals",
            "AK-47",
            "Academic Hunting Ground",
            "Aggressive Corpse",
            "Aid from Bats",
            "Aire of Elation",
            "Alamut",
            "Amici Noctis",
            "Anarch Free Press, The",
            "Anarch Railroad",
            "Anarch Revolt",
            "Anarch Salon",
            "Anarch Troublemaker",
            "Anarchist Uprising",
            "Ancilla Empowerment",
            "Angel's Gift",
            "Anticipation",
            "Apportation",
            "Aranthebes, The Immortal",
            "Arcane Library",
            "Archon",
            "Archon Investigation",
            "Arms of Ahriman",
            "Army of Rats",
            "Art Museum",
            "Assault Rifle",
            "Asylum Hunting Ground",
            "Backways",
            "Bait and Switch",
            "Banishment",
            "Barrens, The",
            "Bear's Skin",
            "Beast Meld",
            "Bewitching Oration",
            "Biotech Company Hunting Ground",
            "Biothaumaturgic Experiment",
            "Black Market Cache",
            "Black Throne, The",
            "Blood Doll",
            "Blood Fury",
            "Blood Rage",
            "Blur",
            "Body Flare",
            "Bonding",
            "Bone Shambler",
            "Bowl of Convergence",
            "Break the Bonds",
            "British Museum, London, The",
            "Brujah Debate",
            "Camarilla's Iron Fist",
            "Camera Phone",
            "Cappadocian Crypt",
            "Carfax Abbey",
            "Carrion Crows",
            "Carver's Meat Packing and Storage",
            "Cats' Guidance",
            "Cave of Apples",
            "Celerity",
            "Change of Target",
            "Channel 10",
            "Chantry",
            "Charisma",
            "Childe of the Revolution",
            "City Star Taxi",
            "Claws of the Dead",
            "Cloak the Gathering",
            "Closed Session",
            "Club Illusion",
            "Codex of the Edenic Groundskeepers",
            "Conceal",
            "Conditioning",
            "Consanguineous Boon",
            "Conservative Agitation",
            "Consign to Oblivion",
            "Constant Revolution",
            "Contraband",
            "Coven, The",
            "Creeping Sabotage",
            "Creepshow Casino",
            "Crypt's Sons",
            "Dabbler",
            "Daring the Dawn",
            "Dawn Operation",
            "Dead Pool",
            "Deep Ecology",
            "Deep Song",
            "Deflection",
            "Delaying Tactics",
            "Direct Intervention",
            "Disarm",
            "Disputed Territory",
            "Diversion",
            "Dog Pack",
            "Dogged Pursuit",
            "Domain Challenge",
            "Dominate",
            "Dominate Kine",
            "Double Deuce",
            "Dreams of the Sphinx",
            "Dummy Corporation",
            "Dust Up",
            "ECTU Operative",
            "Eagle's Sight",
            "Earth Control",
            "Earth Meld",
            "Ecoterrorists",
            "Elder Impersonation",
            "Elder Intervention",
            "Elder Library",
            "Elysian Fields",
            "Elysium: The Palace of Versailles",
            "Enchant Kindred",
            "Enchanting Gaze",
            "Enhanced Coagulant",
            "Enhanced Senses",
            "Entrancement",
            "Expulsion",
            "Eyes of Argus",
            "Eyes of the Wild",
            "Faceless Night",
            "Fame",
            "Family Gathering",
            "Far Mastery",
            "Feast of the Soul's Secrets",
            "Fee Stake: Boston",
            "Fee Stake: New York",
            "Fee Stake: Seattle",
            "Femur of Toomler",
            "Feral Hound",
            "Ferraille",
            "Fifth Tradition: Hospitality",
            "Flamethrower",
            "Flaming Candle",
            "Flash",
            "Fleetness",
            "Flesh of Marble",
            "Fleshforge Chamber",
            "Forced Awakening",
            "Forced Confessional",
            "Forced Vigilance",
            "Foreshadowing Destruction",
            "Forgotten Labyrinth",
            "Form of Mist",
            "Form of the Bat",
            "Form of the Cobra",
            "Form of the Wolf",
            "Fortune Teller Shop",
            "Fourth Tradition: The Accounting",
            "Fragment of the Book of Nod",
            "Freak Drive",
            "Frontal Assault",
            "Gangrel Revel",
            "Garibaldi-Meucci Museum",
            "Gate of Acheron, The",
            "Giant's Blood",
            "Go-getter",
            "Golconda: Inner Peace",
            "Govern the Unaligned",
            "Graverobbing",
            "Grooming the Protégé",
            "Guard Dogs",
            "Guardian Angel",
            "Haqim's Law: Retribution",
            "Haven Uncovered",
            "Heart of Nizchetus",
            "Heart of the City",
            "Heroic Might",
            "Hidden Strength",
            "Hide the Mind",
            "Homunculus",
            "Hunger of Marduk",
            "Hunter's Mark",
            "Hunting the Beast",
            "Immortal Grapple",
            "Indomitability",
            "Information Highway",
            "Instinctive Reaction",
            "Intimidation",
            "Invigorate",
            "Ivory Bow",
            "Judgment: Camarilla Segregation",
            "KRCG News Radio",
            "Kali's Fang",
            "Kevlar Vest",
            "Khabar: Community, The",
            "Khabar: Glory",
            "Kine Resources Contested",
            "Kumpania",
            "Labyrinth, The",
            "Laptop Computer",
            "Libertas",
            "Library Hunting Ground",
            "Life in the City",
            "Lightning Reflexes",
            "Line Brawl",
            "Line, The",
            "Living Manse",
            "Lost in Crowds",
            "Magic of the Smith",
            "Majesty",
            "Make the Misere",
            "Malkavian Justicar",
            "Market Square",
            "Martyr's Resilience",
            "Meditative Grove",
            "Melange",
            "Michael Harris, the Incinerator",
            "Mind Numb",
            "Minion Tap",
            "Mirror Walk",
            "Misdirection",
            "Mithraeum, London, The",
            "Mithraic Cultist",
            "Mob Connections",
            "Mobile HQ, Operation Antigen",
            "Monkey Wrench",
            "Monster",
            "Monstrous Form",
            "Mr. Winthrop",
            "Murder of Crows",
            "My Enemy's Enemy",
            "Neonate Breach",
            "New Carthage",
            "Night Terrors",
            "No Trace",
            "Obedient Flesh",
            "Obfuscate",
            "Oblivion",
            "Old Friends",
            "Ominous Chorus",
            "On the Qui Vive",
            "One With the Land",
            "Open War",
            "Opium Den",
            "Organized Resistance",
            "Owl Companion",
            "Oxford University, England",
            "Pack Alpha",
            "Papillon",
            "Parity Shift",
            "Park Hunting Ground",
            "Parthenon, The",
            "Party Out Of Bounds",
            "Pass Through Shadow",
            "Paths in Two Worlds",
            "Pentex(TM) Subversion",
            "Perfect Paragon",
            "Perfectionist",
            "Phantasmagoria",
            "Piper",
            "Pit of Contemplation",
            "Platinum Protocol, The",
            "Political Ally",
            "Political Hunting Ground",
            "Potence",
            "Power Structure",
            "Powerbase: Los Angeles",
            "Powerbase: Montreal",
            "Powerbase: Munich",
            "Precognition",
            "Preternatural Strength",
            "Priority Contract",
            "Propaganda",
            "Protean",
            "Protected District",
            "Protection Racket",
            "Psychophagia",
            "Public Trust",
            "Puppet Master",
            "Pursuit",
            "Quickness",
            "Rack, The",
            "Raising the Portcullis",
            "Rapid Change",
            "Raptor",
            "Rat's Warning",
            "Raven Spy",
            "Ravnos Cache",
            "Ravnos Carnival",
            "Rebel",
            "Reckless Agitation",
            "Redirection",
            "Rego Motum",
            "Resist Earth's Grasp",
            "Restoration",
            "Revelation of the Serpent",
            "Revelations",
            "Revenant",
            "Revolutionary Council",
            "Rewilding",
            "Righteous Blade",
            "Ritual Goblet",
            "Rolling with the Punches",
            "Rotting Behemoth",
            "Roundhouse",
            "Rumor Mill, Tabloid Newspaper, The",
            "Rutor's Hand",
            "Saulot's Avenging Fist",
            "Saulot's Guiding Wisdom",
            "Saulot's Healing Touch",
            "Scalpel Tongue",
            "Scorn of Adonis",
            "Scouting Mission",
            "Screamer",
            "Sculpt the Flesh",
            "Second Tradition: Domain",
            "Secure Haven",
            "Seduction",
            "Sense the Savage Way",
            "Shadow Cast",
            "Shadow Cloak",
            "Shadow Sentinel",
            "Shadow Shift",
            "Shard, London, The",
            "Shilmulo Tarot",
            "Show of Force",
            "Shroud of Decay",
            "Side Strike",
            "Sight Beyond Sight",
            "Slam",
            "Slaughtering the Herd",
            "Sleeping Mind, The",
            "Slum Hunting Ground",
            "Smiling Jack, The Anarch",
            "Sniper Rifle",
            "Soak",
            "Social Charm",
            "Society Hunting Ground",
            "Soul Burn",
            "Spectral Servitor",
            "Spirit's Touch",
            "Split the Veil",
            "Sport Bike",
            "Spying Mission",
            "Stolen Police Cruiser",
            "Stygian Shroud",
            "Sudden Reversal",
            "Swallowed by the Night",
            "Sword of the Archangel",
            "Szlachta Assistant",
            "Szlachta Bodyguard",
            "Taste of Vitae",
            "Telepathic Counter",
            "Telepathic Misdirection",
            "Telepathic Tracking",
            "Telepathic Vote Counting",
            "Temple Hunting Ground",
            "Tension in the Ranks",
            "Terror Frenzy",
            "Theft of Vitae",
            "Thing",
            "Threats",
            "Tier of Souls",
            "Toreador Grand Ball",
            "Toreador Justicar",
            "Torn Signpost",
            "Touch of Oblivion",
            "Touch of Valeren",
            "Tranquility Shield",
            "Treasured Samadji",
            "True Love's Face",
            "Truth in Darkness",
            "Twilight Camp",
            "Umbrous Clutch",
            "Underbridge Stray",
            "Underworld Hunting Ground",
            "Unleashing the Bestial Soul",
            "Unlicensed Taxicab",
            "Uptown Hunting Ground",
            "Vagabond Mystic",
            "Valerie Diaz, Antigen Commander",
            "Vast Wealth",
            "Veil the Legions",
            "Ventrue Headquarters",
            "Vessel",
            "Villein",
            "Visions of Gehenna",
            "Visions of Zapathasura",
            "Visit from the Capuchin",
            "Voracious Vermin",
            "Voter Captivation",
            "Vozhd of Gravesend, The",
            "Vozhd of Juiz de Fora, The",
            "Vozhd of Sofia, The",
            "Vozhd of Szczecin, The",
            "WMRH Talk Radio",
            "Wake with Evening's Freshness",
            "Walk of Flame",
            "War Ghoul",
            "Warrens, The",
            "Warsaw Station",
            "Warzone Hunting Ground",
            "Wash",
            "Wasserschloss Anif, Austria",
            "Waters of Duat",
            "Weather Control",
            "Week of Nightmares",
            "Weighted Walking Stick",
            "Where the Veil Thins",
            "Wider View",
            "Wind Dance",
            "Wolf Claws",
            "Yoruba Shrine",
            "Zoo Hunting Ground",
        ]
    );
    // Definition modes line up with the generated per-mode static half:
    // each maps to a library mode (`cards::library_mode`: the same index
    // unless a printed choice is split), every library mode is covered, and
    // the windows of the definition modes on one library mode are exactly
    // the reviewed matrix's (the matrix is authoritative).
    for (i, c) in LIBRARY.iter().enumerate() {
        if let Some(def) = cards::lookup(i as u16) {
            let mapped: Vec<usize> = (0..def.modes.len())
                .map(|mi| cards::library_mode(i as u16, mi))
                .collect();
            for lm in 0..c.modes.len() {
                assert!(mapped.contains(&lm), "{} library mode {lm}", c.name);
            }
            for (lmi, lm) in c.modes.iter().enumerate() {
                let mut want: Vec<Window> = lm
                    .windows
                    .iter()
                    .map(|w| Window::from_name(w).unwrap())
                    .collect();
                let mut have: Vec<Window> = def
                    .modes
                    .iter()
                    .zip(&mapped)
                    .filter(|(_, &m)| m == lmi)
                    .flat_map(|(dm, _)| dm.windows.iter().copied())
                    .collect();
                want.sort_by_key(|w| *w as u8);
                have.sort_by_key(|w| *w as u8);
                have.dedup();
                assert_eq!(have, want, "{} mode {lmi}", c.name);
            }
        }
    }
}

#[test]
fn villein_targets_terms_blood_move_layers_and_trifle() {
    let mut e = table();
    let villein = find_library("Villein");
    e.state.seats[0].hand.push(villein);
    // Targets: seat 0's vampires with any blood; not seat 1's.
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(
        req.options,
        vec![Choice::Pass, play(0, MinionId(0)), play(0, MinionId(1))]
    );
    pick(&mut e, play(0, MinionId(0)));
    // Terms are announced at play: 2 to 5 (rules-qa 3.5).
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(
        req.options,
        vec![
            Choice::SetX(2),
            Choice::SetX(3),
            Choice::SetX(4),
            Choice::SetX(5)
        ]
    );
    pick(&mut e, Choice::SetX(4));
    // Nobody holds a cancel: the as-played window elides; the trifle refunds
    // the master phase action, but the hand is empty, so the master phase
    // elides too and the minion phase opens.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[0].blood, 1);
    assert_eq!(e.state.seats[0].pool, 34);
    assert_eq!(e.state.seats[0].mpa, 1);
    assert!(e.state.seats[0].trifle_used);
    assert_eq!(e.state.cards.len(), 1);
    assert_eq!(e.state.cards[0].host, Some(MinionId(0)));
    assert_eq!(e.state.layers.len(), 2);
    assert!(e.state.seats[0].ash.is_empty());
}

// Rulings: [LSJ 20020620]; [ANK 20210714].
#[test]
fn villein_costs_one_more_on_a_vampire_that_already_has_one() {
    let mut e = table();
    let villein = find_library("Villein");
    e.state.seats[0].hand.extend([villein, villein]);
    pick(&mut e, play(0, MinionId(0)));
    pick(&mut e, Choice::SetX(2));
    // Trifle refunded: a second Villein this master phase. With 0 pool the
    // copy on the same vampire (cost 1) is masked out; another vampire is
    // free.
    expect(&mut e, SeatId(0), WindowLabel::MasterPhase); // first play settled
    e.state.seats[0].pool = 0;
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(req.options, vec![Choice::Pass, play(0, MinionId(1))]);
    e.state.seats[0].pool = 1;
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(req.options.contains(&play(0, MinionId(0))));
    pick(&mut e, play(0, MinionId(0)));
    pick(&mut e, Choice::SetX(3));
    // Cost 1 paid, the 3 blood left taken (pool 1 - 1 + 3): the second
    // trifle gives nothing back this master phase.
    // (Drained to 0, the vampire's mandatory hunt is elided at once.)
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 3);
    assert!(e.state.minions[0].blood == 1 && e.state.minions[0].locked);
    assert_eq!(e.state.seats[0].mpa, 0);
}

// Rulings: [LSJ 20020620]; [LSJ 20100216].
#[test]
fn minion_tap_moves_any_amount_paying_its_villein_surcharge_as_pool_is_gained() {
    let mut e = table();
    let (villein, tap) = (find_library("Villein"), find_library("Minion Tap"));
    e.state.seats[0].hand.extend([villein, tap]);
    pick(&mut e, play(0, MinionId(1)));
    pick(&mut e, Choice::SetX(2));
    // Villein took the 1 blood it found; with 1 pool left, Minion Tap costs
    // 1 (Villein's surcharge) and is still offered: the cost is paid as the
    // pool is gained.
    expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.state.seats[0].pool = 1;
    pick(&mut e, play(0, MinionId(0)));
    // Any amount, 0 included, up to the 5 blood there.
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(req.options, (0..=5).map(Choice::SetX).collect::<Vec<_>>());
    pick(&mut e, Choice::SetX(4));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 4);
    assert_eq!(e.state.minions[0].blood, 1);
    assert_eq!(e.state.seats[0].ash, vec![tap]);
    // A vampire in torpor is one "you control" too.
    let mut e = table();
    let t = e.add_minion(SeatId(0), find_card("Alice Chen"), 2, true);
    e.state.seats[0].hand.push(tap);
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(req.options.contains(&play(0, t)));
}

// Rulings: [LSJ 20100216].
#[test]
fn minion_tap_takes_what_is_left_at_resolution() {
    let mut e = table();
    let tap = find_library("Minion Tap");
    e.state.seats[0].hand.push(tap);
    e.state.seats[1].hand.push(find_library("Sudden Reversal"));
    pick(&mut e, play(0, MinionId(0)));
    pick(&mut e, Choice::SetX(5));
    // In the as-played window the vampire loses blood: 3 are left to take.
    expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    e.state.minions[0].blood = 3;
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 33);
    // Drained to 0, its mandatory hunt ran at once.
    assert!(e.state.minions[0].blood == 1 && e.state.minions[0].locked);
}

// Rulings: [LSJ 20090626]; [ANK 20190104] [LSJ 19980212].
#[test]
fn sudden_reversal_cancels_a_master_and_its_cost_and_trifle_refund() {
    let mut e = table();
    e.state.seats[0].hand.push(find_library("Villein"));
    e.state.seats[1].hand.push(find_library("Sudden Reversal"));
    pick(&mut e, play(0, MinionId(0)));
    pick(&mut e, Choice::SetX(5));
    // As-played window: seat 1 may cancel (out of turn).
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::Play {
                card: 0,
                mode: 0,
                actor: None,
                target: Target::None
            }
        ]
    );
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::None,
        },
    );
    // The cancel's own as-played window has no takers; both settle.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[0].blood, 5);
    assert_eq!(e.state.seats[0].pool, 30);
    assert_eq!(e.state.seats[0].ash, vec![find_library("Villein")]);
    assert_eq!(e.state.seats[1].ash, vec![find_library("Sudden Reversal")]);
    assert!(e.state.cards.is_empty() && e.state.layers.is_empty());
    // A canceled trifle refunds nothing; the master phase action is spent.
    assert_eq!(e.state.seats[0].mpa, 0);
    assert!(e.state.seats[1].out_of_turn.is_some());
}

#[test]
fn discard_phase_replaces_from_the_library() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), find_card("Abraham DuSable"), 5, false);
    e.add_minion(SeatId(0), find_card("Abraham Mellon"), 5, false);
    let a = find_library(".44 Magnum");
    let b = find_library("AK-47");
    e.state.seats[0].hand.push(a);
    e.state.seats[0].library.push(b);
    e.state.turn = 4;
    pick(&mut e, Choice::Pass); // minion phase; influence elides
    let req = expect(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert_eq!(req.options, vec![Choice::Pass, Choice::Discard(0)]);
    pick(&mut e, Choice::Discard(0));
    assert_eq!(e.state.seats[0].ash, vec![a]);
    assert_eq!(e.state.seats[0].hand, vec![b]);
    // One discard phase action: the phase elides and the turn passes; the
    // two empty seats elide their whole turns.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.turn, 7);
}

#[test]
fn cards_without_a_definition_are_not_playable() {
    let mut e = table();
    e.state.seats[0].hand.push(find_library(".44 Magnum"));
    // Master phase with only an equipment card: Pass alone, elided.
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&Choice::TakeAction {
        minion: MinionId(0),
        action: ActionKind::Bleed,
        target: None
    }));
}
