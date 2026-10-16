//! Legacy cards for TWDA decks, round 14 (`ADDED_BATCHES`, slice 16z): the
//! 2018 Humble Bundle, 2015 Storyline Rewards and Danse Macabre completed,
//! and the TWDA greedy's steps 1-8. Readings in rules-qa Batch 77. Windows
//! with a single option are elided, so the scripts only stop where a real
//! decision exists.

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::state::{find_card, find_library, CardId, CardInPlay, PrintedCtx};

mod common;
use common::*;

// ---- Helpers ----------------------------------------------------------------

/// `n` seats at seat 0's minion phase, turn 4, 10 pool each.
fn game(n: usize) -> Engine {
    let mut e = Engine::bare(n);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    e
}

/// A vampire's own ability `ability` used by itself.
fn printed(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: Some(m),
        target,
    }
}

/// A card already in play for `seat`, on `host` if given, with its first
/// mode's layers (no play): its id.
fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    let card = find_library(name);
    e.state
        .cards
        .push(CardInPlay::new(card, SeatId(seat), host, 0));
    let id = CardId(e.state.cards.len() as u32 - 1);
    for effect in zvtes::cards::lookup(card).expect("definition").modes[0].effects {
        if let zvtes::cards::Effect::Layer(modifier) = effect {
            e.state.layers.push(zvtes::cards::Layer {
                scope: zvtes::cards::Scope::InPlay,
                modifier: *modifier,
                source: id,
            });
        }
    }
    id
}

/// `m` gets `disc` at superior level (the fixture's Great Beast slot).
fn superior(e: &mut Engine, m: MinionId, disc: &'static str) {
    let slots = &mut e.state.minions[m.index()].beast_disciplines;
    let free = slots.iter().position(Option::is_none).expect("a free slot");
    slots[free] = Some(disc);
}

/// A card play by `actor` at `target`.
fn play_at(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

fn stat(e: &Engine, m: MinionId, s: Stat) -> i32 {
    e.state.printed_mod(m, s, PrintedCtx::default())
}

/// The `nth` vampire with no printed text matching `f`.
fn textless(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| f(c) && zvtes::cards::vampire(*i as u16).is_none())
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a textless vampire")
}

fn pool(e: &Engine, seat: usize) -> i32 {
    e.state.seats[seat].pool
}

/// A master card played from hand at `target`.
fn master(card: u8, mode: u8, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: None,
        target,
    }
}

/// Through seat 0's next turn, taking every option `want` matches: how many.
fn next_turn_taking(e: &mut Engine, mut want: impl FnMut(&Choice) -> bool) -> usize {
    let start = e.state.turn;
    let mut n = 0;
    loop {
        let req = request(e);
        if req.seat == SeatId(0) && req.window == WindowLabel::MinionPhase && e.state.turn > start {
            return n;
        }
        // Every option seen by `want`, the first wanted taken.
        let wanted: Vec<bool> = req.options.iter().map(&mut want).collect();
        if let Some(i) = wanted.iter().position(|&w| w) {
            e.choose(i);
            n += 1;
            continue;
        }
        e.choose(fallback(&req));
    }
}

fn crypt_group(name: &str, group: u8) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.group == group)
        .unwrap_or_else(|| panic!("{name} G{group}")) as u16
}

/// The option using vampire `m`'s printed ability `ability`, whoever acts.
fn vamp_ability(req: &DecisionRequest, m: MinionId, ability: u8) -> Option<Choice> {
    req.options.iter().copied().find(|o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability: a, .. }
            if *v == m && *a == ability)
    })
}

/// Pass (or hand strike) until `m`'s ability `ability` is offered: take it.
fn use_vamp(e: &mut Engine, m: MinionId, ability: u8) {
    for _ in 0..300 {
        let req = request(e);
        if let Some(c) = vamp_ability(&req, m, ability) {
            pick(e, c);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("{m:?}'s ability {ability} never offered");
}

/// The `nth` textless untitled vampire of `sect` with capacity in `caps`.
fn of_sect(sect: Sect, caps: std::ops::RangeInclusive<u8>, nth: usize) -> u16 {
    textless(
        |c| c.sect == sect && c.title == Title::None && caps.contains(&c.capacity),
        nth,
    )
}

/// The first library card whose first mode requires `disc`.
fn requiring(disc: &str, kind: zvtes::library::CardType) -> u16 {
    LIBRARY
        .iter()
        .position(|c| {
            c.types.contains(&kind) && c.modes.first().is_some_and(|m| m.tags.contains(&disc))
        })
        .unwrap_or_else(|| panic!("no {kind:?} requiring {disc}")) as u16
}

fn ash_has(e: &Engine, seat: usize, name: &str) -> bool {
    e.state.seats[seat].ash.contains(&find_library(name))
}

// ---- Chunk A: vampires on existing primitives ---------------------------------

// The vampires printing only a sect, a title, Black Hand or Laibon: no
// definition, their traits generated crypt data.
#[test]
fn the_round_14_text_only_vampires_carry_their_printed_traits() {
    use Sect::{Camarilla, Laibon, Sabbat};
    let cases: [(&str, Sect, Title, bool); 12] = [
        ("Amber", Sabbat, Title::None, false),
        ("Claudia", Sabbat, Title::None, true),
        ("Dimple", Camarilla, Title::None, false),
        ("Eugenio Estevez", Camarilla, Title::None, false),
        ("Idrissa", Laibon, Title::None, false),
        (
            "Johannes Worringen",
            Sabbat,
            Title::Archbishop("Amsterdam"),
            false,
        ),
        (
            "Ladislas Toth, The Torch",
            Sabbat,
            Title::Archbishop("Frankfurt"),
            false,
        ),
        ("Pearl", Sabbat, Title::None, false),
        (
            "Polly Kay Fisher",
            Sabbat,
            Title::Archbishop("New York"),
            false,
        ),
        ("Skulk", Sabbat, Title::None, false),
        ("Titi Camara", Laibon, Title::None, false),
        ("Valois Sang, The Watcher", Camarilla, Title::None, false),
    ];
    let mut e = game(2);
    for (name, sect, title, black_hand) in cases {
        let card = find_card(name);
        assert!(
            zvtes::cards::vampire(card).is_none(),
            "{name}: no text to define"
        );
        let m = e.add_minion(SeatId(0), card, 1, false);
        assert_eq!(e.state.sect_of(m), Some(sect), "{name}");
        assert_eq!(e.state.title_of(m), title, "{name}");
        assert_eq!(e.state.is_black_hand(m), black_hand, "{name}");
    }
}

// Lazverinus, Thrall of Lambach: "+2 strength."
#[test]
fn lazverinus_strikes_for_three() {
    let mut e = game(2);
    let l = e.add_minion(
        SeatId(0),
        find_card("Lazverinus, Thrall of Lambach"),
        4,
        false,
    );
    assert_eq!(stat(&e, l, Stat::Strength), 2);
}

// Aimery Methuin: "Younger Camarilla vampires get -1 intercept against
// Aimery." A younger Camarilla blocker misses his 0-stealth bleed; an older
// Camarilla or a younger Sabbat vampire blocks it.
#[test]
fn aimery_slips_past_younger_camarilla_vampires() {
    let cases = [
        (of_sect(Sect::Camarilla, 4..=6, 0), false),
        (find_card("Masika"), true),
        (of_sect(Sect::Sabbat, 4..=6, 0), true),
    ];
    for (card, blocked) in cases {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Aimery Methuin"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), card, 4, false);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 1), if blocked { 10 } else { 9 }, "{card}");
    }
}

// Dr. Julius Sutphen: "Younger vampires must burn 1 blood to attempt to
// block Julius." A dry younger vampire cannot attempt; an older one pays
// nothing.
#[test]
fn dr_julius_sutphen_tolls_younger_blockers() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Dr. Julius Sutphen"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let dry = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=4, 0), 0, false);
    let younger = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=4, 1), 2, false);
    let older = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 6..=11, 0), 2, false);
    pick(&mut e, bleed(j));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&block(dry)));
    assert!(req.options.contains(&block(older)));
    pick(&mut e, block(younger));
    assert_eq!(blood(&e, younger), 1);
}

// Alex Camille: "During an action, Alex can burn 1 blood to get +1
// intercept." Against a +1 stealth hunt, her block succeeds.
#[test]
fn alex_camille_buys_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let alex = e.add_minion(SeatId(1), find_card("Alex Camille"), 3, false);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(alex));
    use_vamp(&mut e, alex, 0);
    assert_eq!(blood(&e, alex), 2);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

// Badr, Shadow of Granada: "Cards requiring Obtenebration [obt] cost Badr 1
// fewer blood." The mode played, not another card [rules-qa 38.4].
#[test]
fn badr_plays_obtenebration_for_less() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Badr, Shadow of Granada"), 4, false);
    let off = |card: u16| {
        e.state.printed_mod(
            b,
            Stat::BloodCostOff,
            PrintedCtx {
                card: Some((card, 0)),
                ..PrintedCtx::default()
            },
        )
    };
    use zvtes::library::CardType::Combat;
    assert_eq!(off(requiring("obt", Combat)), 1);
    assert_eq!(off(requiring("pot", Combat)), 0);
}

// Carna, The Princess Witch: "During your unlock phase, Carna can burn one
// of her retainers to gain 2 blood. +1 intercept."
#[test]
fn carna_burns_her_retainer_for_blood() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Carna, The Princess Witch"), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let raven = in_play(&mut e, 0, "Raven Spy", Some(c));
    let theirs = in_play(&mut e, 1, "Raven Spy", Some(z));
    assert_eq!(stat(&e, c, Stat::Intercept), 1);
    let mut seen_theirs = false;
    let used = next_turn_taking(&mut e, |o| {
        seen_theirs |=
            matches!(o, Choice::UseAbility { target: Target::Card(t), .. } if *t == theirs);
        matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), target: Target::Card(t), .. }
            if *v == c && *t == raven)
    });
    assert!(!seen_theirs, "her own retainer only");
    assert_eq!((used, blood(&e, c)), (1, 4));
    assert!(ash_has(&e, 0, "Raven Spy"));
}

// Claudio Severino: "If Claudio is ready, you can use a discard phase action
// to discard a card requiring Blood Sorcery [tha] to gain 1 pool."
#[test]
fn claudio_severino_discards_blood_sorcery_for_pool() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Claudio Severino"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let tha = requiring("tha", zvtes::library::CardType::Combat);
    e.state.seats[0].hand.push(tha);
    // The only [tha] card in hand: the discard is elided.
    use_vamp(&mut e, c, 0);
    request(&mut e);
    assert_eq!(pool(&e, 0), 11);
    assert!(e.state.seats[0].ash.contains(&tha));
}

// Dan Murdock: "Dan may take an action to allow you to go though your
// library, find Giant's Blood, and take it into your hand."
#[test]
fn dan_murdock_finds_giants_blood() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Dan Murdock"), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let gb = find_library("Giant's Blood");
    e.state.seats[0].library = vec![find_library("Blood Doll"), gb].into();
    fill_hand(&mut e, 0);
    e.reveal_notes = Some(Vec::new());
    pick(&mut e, printed(d, 0, Target::None));
    let a = e.action().expect("action");
    assert_eq!((a.kind, a.stealth(&e.state)), (ActionKind::Ability, 0));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    assert_eq!(req.options, vec![Choice::Pass, Choice::FindCard(gb)]);
    pick(&mut e, Choice::FindCard(gb));
    assert!(e.state.seats[0].hand.contains(&gb));
    // His text says no reveal: the search stays his.
    assert_eq!(e.reveal_notes.take(), Some(Vec::new()));
}

// De Sade: "Vampires must burn 1 blood to cast votes and ballots against
// referendums called by De Sade." Alexander Silverson's toll.
#[test]
fn de_sade_makes_votes_against_his_referendum_cost_a_blood() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("De Sade"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let dry = e.add_minion(SeatId(1), find_card("Alice Chen"), 0, false);
    let prince = e.add_minion(SeatId(1), find_card("Andi Liu"), 2, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, d));
    pick(&mut e, alloc(0, 3));
    let req = until(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert!(!req.options.contains(&votes(dry, false)));
    assert!(req.options.contains(&votes(dry, true)));
    pick(&mut e, votes(prince, false));
    assert_eq!(blood(&e, prince), 1);
}

// Gold Pan Dan: "Once each turn after combat involving Dan ends, Dan can
// burn 1 blood to unlock if the opposing minion is not ready." Monster's.
#[test]
fn gold_pan_dan_unlocks_after_a_combat_his_opponent_left() {
    let mut e = game(2);
    let a = e.add_ally(SeatId(0), find_library("Political Ally"));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let dan = e.add_minion(SeatId(1), find_card("Gold Pan Dan"), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(dan));
    use_vamp(&mut e, dan, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_play, "burned");
    assert!(!e.state.minions[dan.index()].locked);
    assert_eq!(blood(&e, dan), 3);
}

// Kyle Strathcona: "During your unlock phase, you may move 1 blood from
// Kyle to any other controlled Sabbat vampire." Any Methuselah's.
#[test]
fn kyle_strathcona_feeds_a_sabbat_vampire() {
    let mut e = game(2);
    let k = e.add_minion(
        SeatId(0),
        find_card("Kyle Strathcona, Cardinal of Canada"),
        4,
        false,
    );
    let sab = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 4..=11, 0), 1, true);
    let cam = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=11, 0), 1, false);
    let theirs = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=11, 1), 4, false);
    let feed = |t: MinionId| {
        move |o: &Choice| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), target: Target::Minion(x), .. }
                if *v == k && *x == t)
        }
    };
    let (mut seen_cam, mut seen_theirs) = (false, false);
    let used = next_turn_taking(&mut e, |o| {
        seen_cam |= feed(cam)(o);
        seen_theirs |= feed(theirs)(o);
        feed(sab)(o)
    });
    assert!(!seen_cam, "a Sabbat vampire only");
    assert!(seen_theirs, "another Methuselah's too");
    assert_eq!(
        (used, blood(&e, k), blood(&e, sab)),
        (1, 3, 2),
        "torpid too"
    );
}

// Masika: "Once each action, Masika can discard a political action card to
// give a minion you control +1 intercept." Her blocker meets a +1 stealth
// hunt.
#[test]
fn masika_discards_a_political_action_for_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let m = e.add_minion(SeatId(1), find_card("Masika"), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Kine Resources Contested"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(z));
    let give = |e: &mut Engine| {
        for _ in 0..100 {
            let req = request(e);
            if let Some(c) = req.options.iter().copied().find(|o| {
                matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == m)
            }) {
                pick(e, c);
                return;
            }
            e.choose(fallback(&req));
        }
        panic!("Masika's grant never offered");
    };
    // The only political action card in hand: the discard is elided.
    give(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Kine Resources Contested"));
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

// Masika and London Evening Star: intercept only once it is needed (D15,
// rules-qa 77.20): a bleed at 0 stealth meets the blocker; neither offered.
#[test]
fn masika_and_london_evening_star_wait_for_a_need() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let m = e.add_minion(SeatId(1), find_card("Masika"), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let star = in_play(&mut e, 1, "London Evening Star, Tabloid Newspaper", None);
    hand(&mut e, 1, &["Kine Resources Contested"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == m)
            || matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == star)
    });
}

// Quentin: "Once each political action, Quentin may burn 1 blood to force a
// vampire to abstain."
#[test]
fn quentin_forces_a_vampire_to_abstain() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), find_card("Karen Suadela"), 4, false);
    let q = e.add_minion(SeatId(1), find_card("Quentin"), 3, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    pick(&mut e, alloc(0, 3));
    let force = |e: &mut Engine| {
        for _ in 0..100 {
            let req = request(e);
            if let Some(c) = req.options.iter().copied().find(|o| {
                matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), target: Target::Minion(t), .. }
                    if *v == q && *t == caller)
            }) {
                pick(e, c);
                return;
            }
            e.choose(fallback(&req));
        }
        panic!("Quentin's force never offered");
    };
    force(&mut e);
    assert_eq!(blood(&e, q), 2);
    let r = e.referendum().expect("referendum");
    assert!(r.abstaining.contains(&caller));
}

// Strohmann: "While Strohmann is acting, reaction cards requiring Auspex
// [aus] cost 1 additional blood. +1 bleed." Telepathic Counter [aus] (free)
// costs 1 against his bleed.
#[test]
fn strohmann_makes_auspex_reactions_cost_more() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Strohmann"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    hand(&mut e, 1, &["Telepathic Counter"]);
    assert_eq!(stat(&e, s, Stat::Bleed), 1);
    pick(&mut e, bleed(s));
    pick_when(&mut e, play(0, 0, r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, r), 3);
}

/// Pass (or hand strike) until card `c`'s ability `ability` is offered,
/// with `target` if given: take it.
fn use_card(e: &mut Engine, c: CardId, ability: u8, target: Option<Target>) {
    for _ in 0..300 {
        let req = request(e);
        if let Some(o) = req.options.iter().copied().find(|o| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Card(s), ability: a, target: t, .. }
                if *s == c && *a == ability && target.is_none_or(|w| w == *t))
        }) {
            pick(e, o);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("{c:?}'s ability {ability} never offered");
}

fn in_ash(e: &Engine, c: CardId) -> bool {
    !e.state.cards[c.index()].present()
}

// ---- Chunk A2: library cards and vampires on existing primitives -------------

// Changeling Skin Mask: "The vampire with this equipment has superior
// Obfuscate [OBF]. The vampire with this equipment may burn it to get +2
// intercept for the current action." The superior Obfuscate is not
// optional [RTR 19980707] [LSJ 19980722].
#[test]
fn changeling_skin_mask_gives_obfuscate_and_burns_for_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let mask = in_play(&mut e, 1, "Changeling Skin Mask", Some(z));
    assert_eq!(e.state.discipline_level(z, "obf"), 2);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(z));
    use_card(&mut e, mask, 0, None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert!(in_ash(&e, mask));
}

// "The vampire with this equipment may burn it": an ally bearer cannot.
#[test]
fn changeling_skin_mask_gives_an_ally_no_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    let mask = in_play(&mut e, 1, "Changeling Skin Mask", Some(ally));
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(ally));
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == mask),
    );
}

// Dark Steel: "The opposing minion takes 1 environmental damage each round
// of combat during normal strike resolution at close range." At close
// range, 1 hand strike and 1 environmental damage.
#[test]
fn dark_steel_hurts_at_close_range() {
    for steel in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
        superior(&mut e, a, "OBT");
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Dark Steel"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if steel {
            pick_when(&mut e, play(0, 0, a));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let hit = stat(&e, a, Stat::Strength) as u8 + 1;
        assert_eq!(blood(&e, z), 4 - hit - steel as u8, "steel {steel}");
    }
}

// Dark Steel at long range: its damage is "at close range" only; the combat
// moved to long range (Swallowed by the Night [obf]), none.
#[test]
fn dark_steel_spares_a_long_range_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    superior(&mut e, a, "OBT");
    superior(&mut e, a, "OBF");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Dark Steel", "Swallowed by the Night"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    // Swallowed by the Night, now first in hand: its maneuver to long.
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // No hand strike at long range either: untouched.
    assert_eq!(blood(&e, z), 4, "no environmental damage at long range");
}

// Demonstration: "Lock to cancel the votes and ballots of any vampire
// during the referendum of a political action."
#[test]
fn demonstration_makes_a_vampire_abstain() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), find_card("Karen Suadela"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let d = in_play(&mut e, 1, "Demonstration", None);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    pick(&mut e, alloc(0, 3));
    use_card(&mut e, d, 0, Some(Target::Minion(caller)));
    assert!(e.state.cards[d.index()].locked);
    assert!(e
        .referendum()
        .expect("referendum")
        .abstaining
        .contains(&caller));
}

// Inside Dirt: "Ⓓ Burn the Edge to burn 3 blood from a vampire controlled
// by another Methuselah, or to burn 3 pool from your prey." Not without the
// Edge.
#[test]
fn inside_dirt_burns_the_edge_for_blood_or_pool() {
    for mode in [0u8, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Nosferatu antitribu", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
        hand(&mut e, 0, &["Inside Dirt"]);
        let target = if mode == 0 {
            Target::Minion(z)
        } else {
            Target::Seat(SeatId(1))
        };
        let req = request(&mut e);
        assert!(
            !req.options.contains(&play_at(0, mode, a, target)),
            "no Edge"
        );
        e.state.seats[0].has_edge = true;
        pick(&mut e, play_at(0, mode, a, target));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.seats[0].has_edge);
        let expect = if mode == 0 { (1, 10) } else { (4, 7) };
        assert_eq!((blood(&e, z), pool(&e, 1)), expect, "mode {mode}");
    }
}

// Inside Dirt: the Edge lost before the action resolves, nothing happens
// (Patsy's reading, rules-qa 58.11).
#[test]
fn inside_dirt_does_nothing_once_the_edge_is_lost() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Nosferatu antitribu", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Inside Dirt"]);
    e.state.seats[0].has_edge = true;
    pick(&mut e, play_at(0, 1, a, Target::Seat(SeatId(1))));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(1), "the prey's block decision");
    e.state.seats[0].has_edge = false;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 10);
}

// Marked Territory: "+2 stealth action. Put this card on this vampire and
// unlock them. You can burn this card to give this vampire +2 intercept
// during an action directed at you." The card itself is put on the vampire.
#[test]
fn marked_territory_is_put_on_and_burns_for_intercept() {
    let mut e = game(2);
    // Mimir, whose text reads no action of his.
    let g = e.add_minion(SeatId(0), find_card("Mimir"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Marked Territory"]);
    pick(&mut e, play(0, 0, g));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let mt = find_library("Marked Territory");
    let on = e
        .state
        .cards
        .iter()
        .position(|c| c.card == mt && c.host == Some(g) && c.present())
        .expect("on the vampire");
    assert!(!e.state.cards[on].effect_only);
    assert!(!ash_has(&e, 0, "Marked Territory"));
    assert!(!e.state.minions[g.index()].locked);

    // Burned against a +1 stealth bleed at its controller.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let mt = in_play(&mut e, 1, "Marked Territory", Some(z));
    hand(&mut e, 0, &["Cloak the Gathering"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    use_card(&mut e, mt, 0, None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

// Rewind Time [tem]: "Cancel an action card as it is played (the acting
// minion does not lock)." Any action card, a political action included
// [ANK 20200813-2]. Nehemiah, the pool's one Temporis vampire, is lent it.
#[test]
fn rewind_time_cancels_an_action_card() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), find_card("Karen Suadela"), 4, false);
    let r = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    superior(&mut e, r, "TEM");
    hand(&mut e, 0, &["Kine Resources Contested"]);
    hand(&mut e, 1, &["Rewind Time"]);
    pick(&mut e, play(0, 0, caller));
    pick_when(&mut e, play(0, 1, r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.referendum().is_none());
    assert!(
        !e.state.minions[caller.index()].locked,
        "the actor does not lock"
    );
    assert!(ash_has(&e, 0, "Kine Resources Contested"));
    assert_eq!(blood(&e, r), 2);
}

// Rewind Time [TEM]: "Usable even though there is no action. Cancel a
// non-out-of-turn master card as it is played by another Methuselah, and
// its cost is not paid. That Methuselah gets +1 master phase action." The
// allowance does not extend to wakes [PIB 20120808]: Forced Awakening is
// not offered.
#[test]
fn rewind_time_cancels_a_master_card_outside_an_action() {
    let mut e = game(2);
    e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Ventrue antitribu", 0),
        4,
        false,
    );
    let r = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    superior(&mut e, r, "TEM");
    hand(&mut e, 0, &["Demonstration"]);
    hand(&mut e, 1, &["Rewind Time", "Forced Awakening"]);
    e.state.minions[r.index()].locked = true;
    let s = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    superior(&mut e, s, "TEM");
    e.at_master_phase();
    let mpa = e.state.mpa_left(SeatId(0));
    pick(&mut e, master(0, 0, Target::None));
    let req = offering(&mut e, play(0, 2, s));
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 1, .. })),
        "no wake"
    );
    pick(&mut e, play(0, 2, s));
    request(&mut e);
    assert_eq!(pool(&e, 0), 10, "its cost is not paid");
    assert!(ash_has(&e, 0, "Demonstration"));
    assert_eq!(e.state.mpa_left(SeatId(0)), mpa, "the action given back");
    assert_eq!(blood(&e, s), 2, "Rewind Time's 2 blood");
}

// Spoils of War: "Only usable after resolution of a successful directed
// action. This vampire gains 1 blood and you gain 1 pool."
#[test]
fn spoils_of_war_pays_after_a_successful_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Gold Pan Dan"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    hand(&mut e, 0, &["Spoils of War"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), pool(&e, 0), pool(&e, 1)), (4, 11, 9));
}

// Taste of Death: "Only usable at long range. [qui] Strike: 1R aggravated
// damage." Not at close range; at long range after a maneuver.
#[test]
fn taste_of_death_strikes_at_long_range_only() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "QUI");
    superior(&mut e, a, "OBF");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Taste of Death", "Swallowed by the Night"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    // At close range, not offered.
    let mut close = e.clone();
    never_offered_until(
        &mut close,
        SeatId(0),
        WindowLabel::MinionPhase,
        play(0, 0, a),
    );
    // The maneuver to long range is its only use: elided.
    pick_when(&mut e, play(1, 1, a));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert_eq!(
        e.combat().expect("combat").range,
        zvtes::combat::Range::Long
    );
    assert!(req.options.contains(&play(0, 0, a)));
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor, "1 aggravated damage");
}

// Selena: "Damage inflicted on Selena by a werewolf is aggravated." A
// werewolf ally's hand strike sends her to torpor; another ally's does not.
#[test]
fn selena_takes_aggravated_damage_from_werewolves() {
    for (ally, torpor) in [("Renegade Garou", true), ("Political Ally", false)] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), find_card("Selena"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let w = e.add_ally(SeatId(1), find_library(ally));
        pick(&mut e, bleed(s));
        pick_when(&mut e, block(w));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[s.index()].in_torpor, torpor, "{ally}");
    }
}

// Martin Franckel: "Martin gets -1 bleed when bleeding a Methuselah who
// controls neither a ready Tremere nor a ready Tremere antitribu." A
// torpid Tremere is not ready.
#[test]
fn martin_franckel_bleeds_less_without_tremere() {
    for (clan, torpid, lost) in [
        ("Ventrue", false, 0),
        ("Tremere", false, 1),
        ("Tremere antitribu", false, 1),
        ("Tremere", true, 0),
    ] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Martin Franckel"), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let t = e.add_minion(SeatId(1), plain(clan, 0), 4, true);
        e.state.minions[t.index()].in_torpor = torpid;
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, true);
        pick(&mut e, bleed(m));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 1), 10 - lost, "{clan} torpid {torpid}");
    }
}

// Mélusine d'Anjou: "During an action she performs, Mélusine can burn 1
// blood to give a younger blocking vampire -2 intercept." Not an ally's
// attempt.
#[test]
fn melusine_blinds_a_younger_blocker() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mélusine d'Anjou"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=7, 0), 4, false);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(z));
    use_vamp(&mut e, m, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, m), pool(&e, 1)), (3, 9), "the block failed");

    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mélusine d'Anjou"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let w = e.add_ally(SeatId(1), find_library("Political Ally"));
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(w));
    let req = request(&mut e);
    assert!(vamp_ability(&req, m, 0).is_none(), "not an ally");
}

// Mithras (G3): "Once each action he performs, he can burn 2 blood to
// cancel a reaction card as it is played by a younger vampire, and its cost
// is not paid. +2 bleed." Telepathic Counter canceled: his bleed lands for
// 3.
#[test]
fn mithras_g3_cancels_a_younger_vampires_reaction() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), crypt_group("Mithras", 3), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    hand(&mut e, 1, &["Telepathic Counter"]);
    assert_eq!(stat(&e, m, Stat::Votes), 2);
    pick(&mut e, bleed(m));
    pick_when(&mut e, play(0, 0, r));
    use_vamp(&mut e, m, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, m), pool(&e, 1)), (4, 7));
    assert!(ash_has(&e, 1, "Telepathic Counter"));
}

// The Dracon: "He inflicts +1 damage or steals 1 additional blood or life
// with ranged strikes (even at close range). +1 bleed. +2 strength."
// Theft of Vitae [tha] steals 2 at close range.
#[test]
fn the_dracon_steals_more_with_ranged_strikes() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Dracon, The"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert_eq!(
        (stat(&e, d, Stat::Bleed), stat(&e, d, Stat::Strength)),
        (1, 2)
    );
    hand(&mut e, 0, &["Theft of Vitae"]);
    pick(&mut e, bleed(d));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, d));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // 2 stolen, not 1; his opponent's hand strike mended with 1.
    assert_eq!((blood(&e, d), blood(&e, z)), (5, 2));
}

// ---- Chunk C: new mechanisms ---------------------------------------------------

// Vlad Tepes: "Vlad uses and can play cards requiring a title or a sect as
// if he had that required title or were of that required sect." Abbot
// (requires a Sabbat vampire) and Archon (a prince or justicar) are his to
// play [PIB 20150306]; Archon's terms can choose him as a Camarilla vampire,
// the sect under the title he met [ANK 20201028] [ANK 20190228] [LSJ
// 20101013] [LSJ 20050128], no other Independent; he has no title votes
// [ANK 20231221], and Powerbase: Madrid sees no titled Sabbat vampire in him
// [PIB 20150304]; a title card's title off its sect is dormant [ANK
// 20190211].
#[test]
fn vlad_tepes_plays_cards_requiring_a_sect_or_title() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Vlad Tepes"), 6, false);
    let other = e.add_minion(SeatId(0), of_sect(Sect::Independent, 4..=11, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Abbot", "Archon"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, v)), "Abbot");
    assert!(!req.options.contains(&play(0, 0, other)));
    assert!(req.options.contains(&play(1, 0, v)), "Archon");
    let madrid = in_play(&mut e, 0, "Powerbase: Madrid", None);
    e.state.cards[madrid.index()].counters = 2;
    pick(&mut e, play(1, 0, v));
    let req = until(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert!(req.options.contains(&Choice::ChooseMinion(v)));
    assert!(!req.options.contains(&Choice::ChooseMinion(other)));
    pick(&mut e, Choice::ChooseMinion(v));
    assert_eq!(e.state.title_votes_of(v), 0, "no title votes");
    // A title card's title off its sect, dormant: untitled [ANK 20190211].
    in_play(&mut e, 0, "Banu Haqim Justicar", Some(v));
    assert_eq!(e.state.title_of(v), Title::None);
    let req = until(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert!(!req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { source: AbilitySource::Card(c), target: Target::Minion(t), .. }
            if *c == madrid && *t == v)));
}

// Summon History [tem]: "Search your library and/or ash heap for an ally,
// retainer or equipment card costing X or fewer blood or pool and put that
// card in play, on this vampire." X = 2, chosen as it is played [LSJ
// 20100314]: a 2-pool equipment from the ash heap is put on the actor, its
// inferior version, not equipped [RBK equip], a 3-cost one is not offered;
// the library is shuffled.
#[test]
fn summon_history_puts_a_cheap_card_in_play() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    superior(&mut e, a, "tem");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let vest = find_library("Kevlar Vest");
    let costly = LIBRARY
        .iter()
        .position(|c| {
            c.types.contains(&zvtes::library::CardType::Equipment)
                && c.pool_cost == zvtes::library::Cost::Fixed(3)
        })
        .expect("a 3-pool equipment") as u16;
    e.state.seats[0].ash.put(vest, Default::default());
    e.state.seats[0].library = vec![costly, find_library("Blood Doll")].into();
    hand(&mut e, 0, &["Summon History"]);
    fill_hand(&mut e, 0);
    let card = 0;
    pick(&mut e, play(card, 0, a));
    pick_when(&mut e, set_x(2));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    let take = Choice::TakeFrom {
        zone: zvtes::decision::Zone::AshHeap,
        card: vest,
    };
    assert!(req.options.contains(&take));
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::TakeFrom { card, .. } if *card == costly)));
    pick(&mut e, take);
    request(&mut e);
    let on = e
        .state
        .cards
        .iter()
        .any(|c| c.card == vest && c.host == Some(a) && c.present());
    assert!(on, "on the actor");
    assert_eq!(blood(&e, a), 3, "X paid");
}

// Summon History [TEM]: "Search your crypt and/or ash heap for a non-scarce
// vampire card with capacity X or less and put that vampire in play (with
// no blood)." No requirement, and it may act this turn [LSJ 20100204] [LSJ
// 20040518-2] [LSJ 20100302-1].
#[test]
fn summon_history_puts_a_vampire_in_play_with_no_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    superior(&mut e, a, "TEM");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let small = textless(|c| c.capacity == 3 && !c.scarce, 0);
    let big = textless(|c| c.capacity == 9 && !c.scarce, 0);
    e.state.seats[0].crypt = vec![big, small].into();
    hand(&mut e, 0, &["Summon History"]);
    fill_hand(&mut e, 0);
    let card = 0;
    pick(&mut e, play(card, 1, a));
    pick_when(&mut e, set_x(3));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    assert_eq!(req.options, vec![Choice::Pass, Choice::AshVampire(small)]);
    pick(&mut e, Choice::AshVampire(small));
    let v = e
        .state
        .minions_of(SeatId(0))
        .find(|&m| e.state.minions[m.index()].crypt_card() == Some(small))
        .expect("in play");
    assert_eq!(blood(&e, v), 0);
    // With no blood it must hunt, and may this very turn: the hunt, its
    // only option, is taken at once.
    request(&mut e);
    assert!(e.state.minions[v.index()].locked);
    assert_eq!(blood(&e, v), 1);
}

// Osebo Kholo: "Put this card on this Laibon Osebo to represent the unique
// Laibon title of Osebo Kholo (worth 2 votes). Not usable if there are any
// older ready untitled Laibon Osebo. Whenever this vampire is not a ready
// Laibon Osebo [...], move this card to (one of) the oldest ready untitled
// Laibon Osebo." Titi Camara (5) takes it, not Idrissa (4); in torpor, she
// passes it to Idrissa.
#[test]
fn osebo_kholo_titles_the_oldest_osebo_and_moves_on() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), find_card("Karen Suadela"), 4, false);
    let titi = e.add_minion(SeatId(1), find_card("Titi Camara"), 3, false);
    let idrissa = e.add_minion(SeatId(1), find_card("Idrissa"), 3, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    hand(&mut e, 1, &["Osebo Kholo"]);
    pick(&mut e, play(0, 0, caller));
    pick(&mut e, alloc(0, 3));
    let req = offering(&mut e, play(0, 1, titi));
    assert!(
        !req.options.contains(&play(0, 1, idrissa)),
        "an older one is untitled"
    );
    pick(&mut e, play(0, 1, titi));
    request(&mut e);
    let laibon = Title::Laibon {
        name: "Osebo Kholo",
        votes: 2,
    };
    assert_eq!(e.state.title_of(titi), laibon);
    assert_eq!(e.state.title_votes_of(titi), 2);
    let kholo = find_library("Osebo Kholo");
    let card = e
        .state
        .cards
        .iter()
        .position(|c| c.card == kholo && c.present())
        .expect("in play");
    assert!(!e.state.cards[card].effect_only);
    assert!(!e.state.seats[0].ash.contains(&kholo));
    e.state.minions[titi.index()].in_torpor = true;
    request(&mut e);
    assert_eq!(e.state.cards[card].host, Some(idrissa));
    assert_eq!(e.state.title_of(idrissa), laibon);
}

// Andrew Stuart: "As Andrew plays a card requiring Blood Sorcery [tha],
// reveal the top card of your library. If it also requires Blood Sorcery,
// the card played is canceled and Andrew burns 1 blood." Before anyone may
// play a cancel [LSJ 20100728] [LSJ 20090213]: Direct Intervention is never
// offered; a library top without [tha] lets it be.
#[test]
fn andrew_stuart_gambles_his_blood_sorcery() {
    use zvtes::library::CardType::{Action, Combat};
    let tha_action = requiring("tha", Action);
    for (top, canceled) in [
        (requiring("tha", Combat), true),
        (find_library("Blood Doll"), false),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Andrew Stuart"), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
        hand(&mut e, 1, &["Direct Intervention"]);
        e.state.seats[0].hand.push(tha_action);
        fill_hand(&mut e, 0);
        e.state.seats[0].library = vec![find_library("Blood Doll"), top].into();
        e.reveal_notes = Some(Vec::new());
        let req = request(&mut e);
        let p = req
            .options
            .iter()
            .copied()
            .find(|o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == a))
            .expect("the [tha] action offered");
        pick(&mut e, p);
        // The top card revealed to all, canceling or not.
        let revealed = e.reveal_notes.take().unwrap();
        assert_eq!(
            revealed
                .iter()
                .map(|r| (r.card, r.crypt))
                .collect::<Vec<_>>(),
            vec![(top, false)]
        );
        let req = request(&mut e);
        if canceled {
            assert!(!req
                .options
                .iter()
                .any(|o| matches!(o, Choice::Play { actor: None, .. })));
        } else {
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        }
        // Canceled as played: no action taken, its actor unlocked.
        assert_eq!(
            e.state.minions[a.index()].locked,
            !canceled,
            "canceled {canceled}"
        );
        if canceled {
            assert_eq!(blood(&e, a), 4);
            assert!(e.state.seats[0].ash.contains(&tha_action));
        }
    }
}

// Orlando Oriundus: "During a referendum, if Orlando is ready, your
// vampires' votes are doubled when results are tallied." His bishop's vote
// counts 2, read as it is tallied: 1 once he is in torpor.
#[test]
fn orlando_doubles_his_methuselahs_votes() {
    let mut e = game(2);
    let o = e.add_minion(SeatId(0), find_card("Orlando Oriundus"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, o));
    pick(&mut e, alloc(0, 3));
    pick_when(&mut e, votes(o, true));
    let r = e.referendum().expect("referendum");
    assert_eq!(r.tally(&e.state), (2, 0));
    e.state.minions[o.index()].in_torpor = true;
    let r = e.referendum().expect("referendum");
    assert_eq!(r.tally(&e.state), (1, 0));
}

// Sanguine Entrapment: "[cel][qui] The opposing minion cannot strike: dodge
// this round. [CEL][QUI] The opposing minion cannot strike: combat ends
// this round." Form of Mist's dodge [pro] or combat ends [PRO] is barred.
#[test]
fn sanguine_entrapment_bars_a_dodge_or_a_combat_end() {
    for (mode, barred) in [(0u8, 0u8), (1, 1)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        superior(&mut e, a, "CEL");
        superior(&mut e, a, "QUI");
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        superior(&mut e, z, "PRO");
        hand(&mut e, 0, &["Sanguine Entrapment"]);
        hand(&mut e, 1, &["Form of Mist"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, mode, a));
        let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
        assert!(!req.options.contains(&play(0, barred, z)), "mode {mode}");
        assert!(req.options.contains(&play(0, 1 - barred, z)), "mode {mode}");
    }
}

// Groundfighting cancels a combat card "that would restrict this anarch's
// choice of strikes this round": Sanguine Entrapment's barred dodge is one
// (LSJ 20050221's exclusions are range, maneuvers, undodgeable strikes).
#[test]
fn groundfighting_cancels_sanguine_entrapment() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "CEL");
    superior(&mut e, a, "QUI");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), of_sect(Sect::Anarch, 4..=11, 0), 4, false);
    superior(&mut e, z, "PRO");
    hand(&mut e, 0, &["Sanguine Entrapment"]);
    hand(&mut e, 1, &["Form of Mist", "Groundfighting"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, play(1, 1, z)));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(req.options.contains(&play(0, 0, z)), "the dodge again");
    assert!(ash_has(&e, 0, "Sanguine Entrapment"));
}

// Superior Mettle [FOR]: "each round of this combat, this vampire may burn
// 1 blood to prevent all damage from the opposing minion's initial strike
// for that round." Aidan Lyle presses to a second round: the burn is
// offered, the hand strike prevented.
#[test]
fn superior_mettle_buys_prevention_each_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Aidan Lyle"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    superior(&mut e, z, "FOR");
    hand(&mut e, 1, &["Superior Mettle"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, z));
    pick_when(&mut e, Choice::UsePress);
    let before = blood(&e, z);
    pick_when(&mut e, Choice::ResolvePrinted { minion: z });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.combat().map(|c| c.round), None);
    assert_eq!(blood(&e, z), before - 1, "the burn, no damage");
}

// Tangle Atropos' Hand [TEM]: "Cancel the action and unlock this vampire.
// [...] if an action card was played, take it back in your hand [...] and
// this vampire gets +1 stealth during their next action this turn." The
// action ends at once, nothing more played [RTR 20180719]; not resolved,
// the same action may be taken again [LSJ 20070709] [LSJ 20090220] [ANK
// 20210124], with the stealth.
#[test]
fn tangle_atropos_hand_cancels_and_returns_the_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    superior(&mut e, a, "TEM");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Computer Hacking", "Tangle Atropos' Hand"]);
    let hacking = find_library("Computer Hacking");
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(z));
    // Computer Hacking left the hand: Tangle is card 0 now.
    pick_when(&mut e, play(0, 2, a));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked);
    assert!(
        !e.state.minions[z.index()].locked,
        "the blocker does not lock"
    );
    assert!(e.state.seats[0].hand.contains(&hacking));
    assert_eq!(pool(&e, 1), 10);
    let again = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a))
        .expect("the card offered again");
    pick(&mut e, again);
    request(&mut e);
    let s = e.action().expect("action").stealth(&e.state);
    assert_eq!(s, 1, "+1 stealth");
}

// ---- Chunk B: Black Hand and the small predicates ------------------------------

// Recruitment Exercise: "Requires a Black Hand vampire. Draw 1 card from
// your crypt and reveal it. Add 1 blood from the blood bank to it if it is a
// Black Hand vampire." Claudia (Black Hand) drawn gets 1 blood; another
// none; not playable by a vampire who is not Black Hand; with an empty
// crypt, nothing drawn [LSJ 20060620].
#[test]
fn recruitment_exercise_draws_and_feeds_the_black_hand() {
    for (drawn, blood) in [(find_card("Claudia"), 1u8), (find_card("Amber"), 0)] {
        let mut e = game(2);
        let bh = e.add_minion(SeatId(0), find_card("Selena"), 4, false);
        let other = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
        e.state.seats[0].crypt = vec![drawn].into();
        hand(&mut e, 0, &["Recruitment Exercise"]);
        e.reveal_notes = Some(Vec::new());
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 0, other)), "not Black Hand");
        pick(&mut e, play(0, 0, bh));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let slot = e.state.seats[0].uncontrolled.last().expect("drawn");
        assert_eq!((slot.card, slot.counters), (drawn, blood));
        // Drawn revealed: told to all.
        let revealed = e.reveal_notes.take().unwrap();
        assert_eq!(
            revealed
                .iter()
                .map(|r| (r.card, r.crypt))
                .collect::<Vec<_>>(),
            vec![(drawn, true)]
        );
    }
    // "Draw as much as you can": played with an empty crypt, nothing drawn
    // (rules-qa 77.35).
    let mut e = game(2);
    let bh = e.add_minion(SeatId(0), find_card("Selena"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    e.state.seats[0].crypt = Vec::new().into();
    hand(&mut e, 0, &["Recruitment Exercise"]);
    pick(&mut e, play(0, 0, bh));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].uncontrolled.is_empty());
    assert!(ash_has(&e, 0, "Recruitment Exercise"));
}

// Shakar: the Hunt: "Requires a Black Hand vampire. Put this card on a ready
// minion. Any Black Hand vampire can enter combat with this minion as a +1
// stealth Ⓓ action. Burn this card if this minion is not ready."
#[test]
fn shakar_lets_the_black_hand_hunt_its_minion() {
    let mut e = game(2);
    let bh = e.add_minion(SeatId(0), find_card("Selena"), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Shakar: the Hunt"]);
    e.at_master_phase();
    pick(&mut e, master(0, 0, Target::Minion(z)));
    let req = to_minion_phase(&mut e, SeatId(0));
    let shakar = e
        .state
        .cards
        .iter()
        .position(|c| c.card == find_library("Shakar: the Hunt") && c.present())
        .expect("on the minion") as u32;
    assert_eq!(e.state.cards[shakar as usize].host, Some(z));
    assert!(!e.state.cards[shakar as usize].effect_only);
    assert!(!ash_has(&e, 0, "Shakar: the Hunt"));
    let hunt = |m: MinionId| Choice::UseAbility {
        source: AbilitySource::Card(CardId(shakar)),
        ability: 0,
        actor: Some(m),
        target: Target::Minion(z),
    };
    assert!(req.options.contains(&hunt(bh)));
    assert!(!req.options.contains(&hunt(other)));
    pick(&mut e, hunt(bh));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    // Sent to torpor by the combat (no blood to mend): the card burns.
    e.state.minions[z.index()].blood = 0;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor);
    assert!(!e.state.cards[shakar as usize].present());
}

// Karif al Numair: "During your turn, you can lock or unlock another ready
// Black Hand vampire." Selena, not a vampire who is not Black Hand; from
// torpor too [LSJ 20011022] [LSJ 20100902-2].
#[test]
fn karif_locks_or_unlocks_a_black_hand_vampire() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Karif al Numair"), 4, true);
    let s = e.add_minion(SeatId(1), find_card("Selena"), 4, false);
    let other = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert!(vamp_ability(&req, k, 0).is_some());
    let toggle = |t: MinionId| {
        req.options.iter().any(|o| {
            matches!(o,
            Choice::UseAbility { source: AbilitySource::Vampire(v), target: Target::Minion(x), .. }
                if *v == k && *x == t)
        })
    };
    assert!(toggle(s));
    assert!(!toggle(other));
    let c = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::UseAbility { target: Target::Minion(x), .. } if *x == s))
        .expect("on Selena");
    pick(&mut e, c);
    request(&mut e);
    assert!(e.state.minions[s.index()].locked);
}

// Gerald FitzGerald: "During your unlock phase, Gerald can burn 1 blood to
// make a younger non-titled Sabbat vampire you control Black Hand."
#[test]
fn gerald_makes_a_younger_sabbat_vampire_black_hand() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Gerald FitzGerald"), 3, false);
    let young = e.add_minion(SeatId(0), find_card("Amber"), 2, false);
    let titled = e.add_minion(SeatId(0), find_card("Johannes Worringen"), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let make = |t: MinionId| {
        move |o: &Choice| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), target: Target::Minion(x), .. }
                if *v == g && *x == t)
        }
    };
    let mut seen_titled = false;
    let used = next_turn_taking(&mut e, |o| {
        seen_titled |= make(titled)(o);
        make(young)(o)
    });
    assert!(!seen_titled, "not a titled one");
    assert_eq!((used, blood(&e, g)), (1, 2));
    assert!(e.state.is_black_hand(young));
}

// Righteous Endeavor: "+1 intercept against infernal minions and +1
// strength in combat with an infernal minion."
#[test]
fn righteous_endeavor_stands_against_infernal_minions() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Righteous Endeavor"), 4, false);
    let inf = e.add_minion(SeatId(1), find_card("Arishat"), 4, false);
    let other = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let against = |e: &Engine, o, s| {
        e.state.printed_mod(
            r,
            s,
            PrintedCtx {
                opponent: Some(o),
                ..PrintedCtx::default()
            },
        )
    };
    for s in [Stat::Intercept, Stat::Strength] {
        assert_eq!(
            (against(&e, inf, s), against(&e, other, s)),
            (1, 0),
            "{s:?}"
        );
    }
}

// Suhailah: "+1 stealth on each of her actions if your prey has a ready
// prince or justicar. +1 bleed." Read live.
#[test]
fn suhailah_slips_past_a_prey_with_a_prince() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Suhailah"), 4, false);
    let prince = e.add_minion(SeatId(1), find_card("Andi Liu"), 4, true);
    assert_eq!(stat(&e, s, Stat::Stealth), 0, "in torpor: not ready");
    e.state.minions[prince.index()].in_torpor = false;
    assert_eq!(
        (stat(&e, s, Stat::Stealth), stat(&e, s, Stat::Bleed)),
        (1, 1)
    );
}

// Synner-G: "Synner-G can unlock after successfully equipping with an
// electronic equipment from your hand."
#[test]
fn synner_g_unlocks_after_equipping_electronics() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Synner-G"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    hand(&mut e, 0, &["Laptop Computer"]);
    pick(&mut e, play(0, 0, g));
    use_vamp(&mut e, g, 0);
    request(&mut e);
    assert!(!e.state.minions[g.index()].locked);
}

// Paul Cordwood: "Once each action, Paul can remove the top card of your
// crypt from the game to get +1 intercept or +1 stealth." Against a +1
// stealth hunt, his block succeeds.
#[test]
fn paul_cordwood_spends_his_crypt_for_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let p = e.add_minion(SeatId(1), find_card("Paul Cordwood"), 4, false);
    let top = plain("Ventrue", 0);
    e.state.seats[1].crypt = vec![top].into();
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(p));
    use_vamp(&mut e, p, 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert!(e.state.seats[1].crypt.is_empty());
    assert!(e.state.seats[1].removed_crypt.contains(&top));
}

// Bulscu: "+1 stealth during actions directed at a location or an
// equipment." Not at a retainer, nor an undirected one.
#[test]
fn bulscu_steals_up_on_locations_and_equipment() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bulscu"), 4, false);
    let at = |e: &Engine, card: &str, directed: bool| {
        e.state.printed_mod(
            b,
            Stat::Stealth,
            PrintedCtx {
                directed: Some(directed),
                target_card: Some(find_library(card)),
                ..PrintedCtx::default()
            },
        )
    };
    assert_eq!(at(&e, "Powerbase: Madrid", true), 1);
    assert_eq!(at(&e, "Laptop Computer", true), 1);
    assert_eq!(at(&e, "Raven Spy", true), 0);
    assert_eq!(at(&e, "Laptop Computer", false), 0);
    assert_eq!(stat(&e, b, Stat::Strength), 1);
}

// Mimir: "Other Methuselahs cannot play master cards targeting Mimir."
// Giant's Blood may choose any vampire but him; his own Methuselah's may.
#[test]
fn mimir_is_no_other_methuselahs_master_target() {
    for own in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
        let m = e.add_minion(SeatId(u8::from(!own)), find_card("Mimir"), 1, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
        hand(&mut e, 0, &["Giant's Blood"]);
        e.at_master_phase();
        let req = request(&mut e);
        assert!(req.options.contains(&master(0, 0, Target::Minion(z))));
        assert_eq!(
            req.options.contains(&master(0, 0, Target::Minion(m))),
            own,
            "own {own}"
        );
    }
}

// Bruce de Guy: "Bruce may burn a blood when he announces a political action
// to prevent Camarilla vampires from casting votes or ballots in the
// resulting referendum."
#[test]
fn bruce_de_guy_silences_the_camarilla() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bruce de Guy"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let prince = e.add_minion(SeatId(1), find_card("Andi Liu"), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, b));
    use_vamp(&mut e, b, 0);
    assert_eq!(blood(&e, b), 3);
    pick_when(&mut e, alloc(0, 3));
    // The prince never casts: Bruce's Methuselah alone polls.
    while e.referendum().is_some() {
        let req = request(&mut e);
        assert!(!req
            .options
            .iter()
            .any(|o| matches!(o, Choice::CastVotes { minion, .. } if *minion == prince)));
        e.choose(fallback(&req));
    }
}

// Maldavis: "Primogen get +1 vote in referendums called by Maldavis." Any
// Methuselah's primogen; not in another's referendum.
#[test]
fn maldavis_gives_primogen_a_vote() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Maldavis"), 4, false);
    let p = e.add_minion(SeatId(0), find_card("Carna, The Princess Witch"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, m));
    pick(&mut e, alloc(0, 3));
    to_referendum(&mut e);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, p), 2, "1 primogen vote + 1");
    assert_eq!(stat(&e, p, Stat::Votes), 0, "outside his referendum");
}

// Nehemiah: "During your unlock phase, you can move up to two cards from
// your hand to the bottom of your library (draw up afterward). Nehemiah
// cannot use cards requiring Celerity [cel]." Lent Celerity, a [cel]
// combat card is still not his to play, Line Brawl's [pot] mode is [LSJ
// 20050322-2]; two cards go under the library and two are drawn.
#[test]
fn nehemiah_recycles_two_cards_and_shuns_celerity() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nehemiah"), 4, false);
    superior(&mut e, n, "CEL");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let barred = |e: &Engine, card: u16| {
        e.state.printed_mod(
            n,
            Stat::CannotPlay,
            PrintedCtx {
                card: Some((card, 0)),
                ..PrintedCtx::default()
            },
        )
    };
    use zvtes::library::CardType::Combat;
    assert_eq!(barred(&e, requiring("cel", Combat)), 1);
    assert_eq!(barred(&e, requiring("pot", Combat)), 0);
    let brawl = find_library("Line Brawl");
    let mode = |m: u8| {
        e.state.printed_mod(
            n,
            Stat::CannotPlay,
            PrintedCtx {
                card: Some((brawl, m)),
                ..PrintedCtx::default()
            },
        )
    };
    assert_eq!((mode(0), mode(1)), (1, 0), "[cel] barred, [pot] not");
    hand(&mut e, 0, &["Kevlar Vest", "Sport Bike"]);
    fill_hand(&mut e, 0);
    let doll = find_library("Blood Doll");
    e.state.seats[0].library = vec![doll, doll, doll].into();
    let before = e.state.seats[0].hand.clone();
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(n),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    pick(&mut e, Choice::SetX(2));
    pick(&mut e, Choice::Bottom(0));
    pick(&mut e, Choice::Bottom(0));
    request(&mut e);
    let s = &e.state.seats[0];
    assert_eq!(s.library.len(), 3, "two under, two drawn");
    assert_eq!(s.library[0], before[1], "the last moved, the bottom card");
    assert_eq!(s.library[1], before[0]);
    assert_eq!(s.hand.iter().filter(|&&c| c == doll).count(), 2);
}

// Tabriz Assembly: "You get +1 hand size. During your unlock phase, a True
// Brujah you control can gain 1 blood. Vampires can call a referendum to
// burn this card as a +1 stealth political action; during that referendum,
// you get +1 vote."
#[test]
fn tabriz_assembly_feeds_true_brujah_and_votes_against_its_burning() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nehemiah"), 2, false);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let base = e.state.hand_size(SeatId(0));
    let t = in_play(&mut e, 0, "Tabriz Assembly", None);
    assert_eq!(e.state.hand_size(SeatId(0)), base + 1);
    // The burn referendum, called by his own Methuselah's vampire here.
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(t),
            ability: 1,
            actor: Some(caller),
            target: Target::None,
        },
    );
    to_referendum(&mut e);
    use_card(&mut e, t, 3, None);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.tally(&e.state).1, 1, "his vote against");
    // One vote: for or against, not both (a card's per-referendum use is
    // per copy).
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 2, .. } if *c == t),
    );
    let used = next_turn_taking(&mut e, |o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 0, target: Target::Minion(m), .. }
            if *c == t && *m == n)
    });
    assert_eq!((used, blood(&e, n)), (1, 3));
}

// Festivo dello Estinto, Tabriz Assembly, Demonstration: "Put this card in
// play": each played, the card itself enters play.
#[test]
fn round_14_masters_are_put_in_play() {
    for name in ["Festivo dello Estinto", "Tabriz Assembly", "Demonstration"] {
        let mut e = game(2);
        e.add_minion(SeatId(0), find_card("Nehemiah"), 4, false);
        e.add_minion(
            SeatId(0),
            find_card("Lazverinus, Thrall of Lambach"),
            4,
            false,
        );
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &[name]);
        e.at_master_phase();
        pick(&mut e, master(0, 0, Target::None));
        request(&mut e);
        let card = find_library(name);
        assert!(
            e.state
                .cards
                .iter()
                .any(|c| c.card == card && c.present() && !c.effect_only),
            "{name}"
        );
        assert!(!ash_has(&e, 0, name), "{name}");
    }
}

// Festivo dello Estinto: "Sabbat vampires get -1 stealth during hunt
// actions. Sabbat vampires successfully hunting gain enough blood from the
// blood bank to reach full capacity. During your unlock phase, burn this
// card." Any Methuselah's Sabbat vampire; a Camarilla one hunts as usual.
#[test]
fn festivo_dello_estinto_feasts_the_sabbat() {
    for (sect, filled) in [(Sect::Sabbat, true), (Sect::Camarilla, false)] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), of_sect(sect, 5..=8, 0), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
        let f = in_play(&mut e, 1, "Festivo dello Estinto", None);
        let stealth = e
            .state
            .bearer_mod_in(v, Stat::Stealth, ActionKind::Hunt, false);
        pick(&mut e, action(v, ActionKind::Hunt));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let cap = e.state.capacity_of(v).expect("vampire");
        if filled {
            assert_eq!(stealth, -1);
            assert_eq!(blood(&e, v), cap);
        } else {
            assert_eq!((stealth, blood(&e, v)), (0, 2));
        }
        assert!(
            e.state.cards[f.index()].present(),
            "its controller's unlock"
        );
    }
}

/// A card already in play for `seat` in its `mode` (its layers ignored).
fn in_play_mode(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>, mode: u8) -> CardId {
    e.state.cards.push(CardInPlay::new(
        find_library(name),
        SeatId(seat),
        host,
        mode,
    ));
    CardId(e.state.cards.len() as u32 - 1)
}

// Esprit de Corps: "Put this card in play and unlock this vampire. Minions
// you control get +1 strength. [POT][PRE] As above, and you can burn this
// card to cancel an action card requiring Dementation [dem], Dominate [dom]
// or Presence [pre] that would target you." Intimidation [pre] bleeding its
// controller is canceled, the actor unlocked.
#[test]
fn esprit_de_corps_strengthens_and_cancels() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Esprit de Corps"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked);
    let ec = find_library("Esprit de Corps");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == ec && c.present() && c.host.is_none() && !c.effect_only));
    assert!(!ash_has(&e, 0, "Esprit de Corps"));
    let str_of = |e: &Engine, m| e.state.bearer_mod(m, Stat::Strength, ActionKind::Bleed);
    assert_eq!((str_of(&e, a), str_of(&e, b), str_of(&e, z)), (1, 1, 0));

    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 2), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let esprit = in_play_mode(&mut e, 1, "Esprit de Corps", None, 1);
    hand(&mut e, 0, &["Intimidation"]);
    pick(&mut e, play(0, 0, v));
    use_card(&mut e, esprit, 0, None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[esprit.index()].present());
    assert!(!e.state.minions[v.index()].locked, "not taken");
    assert_eq!(pool(&e, 1), 10);
}

// Public Vilification: "Choose a ready younger vampire controlled by another
// Methuselah, and allocate 3 of their blood between 1 or more other
// vampires (including this acting vampire). Successful referendum means the
// blood is distributed as you chose." Short of blood at resolution, the
// caller picks within the terms [LSJ 20010606].
#[test]
fn public_vilification_moves_three_blood() {
    for short in [false, true] {
        let mut e = game(2);
        let c = e.add_minion(SeatId(0), find_card("Johannes Worringen"), 4, false);
        let o = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
        let z = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 4..=5, 0), 4, false);
        hand(&mut e, 0, &["Public Vilification"]);
        pick(&mut e, play(0, 0, c));
        // The one younger vampire of another's: chosen at once.
        let req = until(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
        assert_eq!(
            req.options,
            vec![Choice::ChooseMinion(c), Choice::ChooseMinion(o)]
        );
        for m in [c, c, o] {
            pick(&mut e, Choice::ChooseMinion(m));
        }
        if short {
            e.state.minions[z.index()].blood = 2;
        }
        pick_when(&mut e, votes(c, true));
        close_polling(&mut e);
        if short {
            let req = request(&mut e);
            assert_eq!(
                req.options,
                vec![Choice::ChooseMinion(c), Choice::ChooseMinion(o)]
            );
            // The last blood: his allotments alone remain, given at once.
            pick(&mut e, Choice::ChooseMinion(o));
            request(&mut e);
            assert_eq!((blood(&e, z), blood(&e, c), blood(&e, o)), (0, 5, 2));
        } else {
            request(&mut e);
            assert_eq!((blood(&e, z), blood(&e, c), blood(&e, o)), (1, 6, 2));
        }
    }
}

// Powerbase: Luanda: "Lock this card during your master phase and choose a
// vampire you control. Once this turn, the chosen vampire may enter combat
// with any minion as a +1 stealth Ⓓ action." Not another of his vampires.
#[test]
fn powerbase_luanda_sends_its_chosen_vampire_to_fight() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Titi Camara"), 4, false);
    let i = e.add_minion(SeatId(0), find_card("Idrissa"), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let l = in_play(&mut e, 0, "Powerbase: Luanda", None);
    e.at_master_phase();
    use_card(&mut e, l, 0, Some(Target::Minion(t)));
    let req = to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.cards[l.index()].locked);
    let fight = |m: MinionId| Choice::UseAbility {
        source: AbilitySource::Card(l),
        ability: 1,
        actor: Some(m),
        target: Target::Minion(z),
    };
    assert!(req.options.contains(&fight(t)));
    assert!(!req.options.contains(&fight(i)));
    pick(&mut e, fight(t));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[t.index()].locked = false;
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::TakeAction {
        minion: t,
        action: ActionKind::Hunt,
        target: None,
    }));
    assert!(!req.options.contains(&fight(t)), "once this turn");
}

// Powerbase: Luanda stolen while locked: the choice is gone with it — no
// rush for the old chosen vampire in its Methuselah's next turn (rules-qa
// 77.16).
#[test]
fn powerbase_luanda_stolen_takes_its_choice() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Titi Camara"), 4, false);
    e.add_minion(SeatId(0), find_card("Idrissa"), 4, true);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let l = in_play(&mut e, 0, "Powerbase: Luanda", None);
    e.at_master_phase();
    use_card(&mut e, l, 0, Some(Target::Minion(t)));
    to_minion_phase(&mut e, SeatId(0));
    pick(&mut e, Choice::Pass);
    let steal = |o: &Choice| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 2, actor: Some(a), .. }
            if *c == l && *a == z)
    };
    to_minion_phase(&mut e, SeatId(1));
    let req = request(&mut e);
    let i = req.options.iter().position(steal).expect("the steal");
    e.choose(i);
    let req = to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.cards[l.index()].controller, SeatId(1), "stolen");
    assert!(e.state.cards[l.index()].locked);
    assert!(!req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { source: AbilitySource::Card(c), ability: 1, .. } if *c == l)));
}

// 419 Operation: "During your unlock phase, you may move 1 pool from your
// prey's pool to this card or move the pool on this card to your pool. Your
// prey can burn the Edge to move the counters on this card to his or her
// pool and burn this card."
#[test]
fn operation_419_skims_the_prey_until_the_edge_burns_it() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Titi Camara"), 4, false);
    e.add_minion(SeatId(0), find_card("Idrissa"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["419 Operation"]);
    pick(&mut e, play(0, 0, t));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = find_library("419 Operation");
    let op = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.card == card && c.present())
            .expect("in play") as u32,
    );
    assert!(!e.state.cards[op.index()].effect_only);
    assert!(!ash_has(&e, 0, "419 Operation"));
    // The prey's turn: no Edge, no burn.
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(uses(&req, op.0).is_empty());
    let used = next_turn_taking(&mut e, |o| *o == ability(op.0, 0));
    assert_eq!(used, 1);
    assert_eq!((pool(&e, 1), e.state.cards[op.index()].counters), (9, 1));
    // The prey with the Edge takes the counters back and burns it.
    e.state.seats[1].has_edge = true;
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&ability(op.0, 2)));
    pick(&mut e, ability(op.0, 2));
    request(&mut e);
    assert_eq!(pool(&e, 1), 10);
    assert!(!e.state.cards[op.index()].present());
    assert!(!e.state.seats[1].has_edge);
}

// Rise of the Fallen: "during which you have burned corruption counters":
// counters its controller burned, whoever's (owner, 2026-10-16; rules-qa
// 77.28). Aniel burns seat 1's counter on seat 1's vampire: Rise of the
// Fallen on seat 0 taxes its prey; on seat 1, nothing.
#[test]
fn rise_of_the_fallen_counts_the_counters_you_burn() {
    for (rise, pools) in [(0u8, (10, 9)), (1, (10, 10))] {
        let mut e = game(2);
        let aniel = e.add_minion(SeatId(0), find_card("Aniel"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.minions[z.index()].locked = true;
        e.state.minions[y.index()].locked = true;
        e.state.minions[z.index()].add_corruption(SeatId(1), 1);
        in_play(&mut e, rise, "Rise of the Fallen", None);
        pick(&mut e, printed(aniel, 1, Target::Minion(z)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[z.index()].corruption.is_empty());
        assert_eq!((pool(&e, 0), pool(&e, 1)), pools, "Rise on seat {rise}");
    }
}

// Rise of the Fallen: "+1 stealth action. Requires an infernal minion. Put
// this card in play." The card itself enters play.
#[test]
fn rise_of_the_fallen_is_put_in_play() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Midget"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    hand(&mut e, 0, &["Rise of the Fallen"]);
    pick(&mut e, play(0, 0, m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let rise = find_library("Rise of the Fallen");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == rise && c.present() && !c.effect_only));
    assert!(!ash_has(&e, 0, "Rise of the Fallen"));
}

// Rise of the Fallen: "Your prey burns 1 pool after resolution of each
// action during which you have burned corruption counters. Methuselahs can
// burn a vampire they control with any corruption counters during their
// unlock phase to burn this card." Enchanting Gaze [PRE][PRO] burns the
// counter on the blocker: the bleed lands, the prey burns 1 more.
#[test]
fn rise_of_the_fallen_taxes_the_prey_for_corruption() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "PRE");
    superior(&mut e, a, "PRO");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].add_corruption(SeatId(0), 1);
    let rise = in_play(&mut e, 0, "Rise of the Fallen", None);
    hand(&mut e, 0, &["Enchanting Gaze"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::BurnCorruptionToFail { minion: z });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        pool(&e, 1),
        10 - 1 - 1,
        "the bleed, then Rise of the Fallen"
    );

    // The prey's own unlock phase: their vampire with a counter burned to
    // burn the card.
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[w.index()].add_corruption(SeatId(0), 1);
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(rise),
            ability: 0,
            actor: None,
            target: Target::Minion(w),
        },
    );
    request(&mut e);
    assert!(!e.state.minions[w.index()].in_play, "burned");
    assert!(!e.state.cards[rise.index()].present());
}

// Baba Yaga: "Once each turn, Baba Yaga can unlock when she successfully
// recruits an ally or employs a retainer." Once only this turn (each
// player's turn: [ANK 20180719-4]); by Pack Alpha's employ in combat too
// [PIB 20150105-2].
#[test]
fn baba_yaga_unlocks_after_recruiting_or_employing() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Baba Yaga, the Iron Hag"), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    hand(&mut e, 0, &["ECTU Operative", "Carlton Van Wyk"]);
    pick(&mut e, play(0, 0, b));
    pick_when(&mut e, Choice::ResolvePrinted { minion: b });
    request(&mut e);
    assert!(!e.state.minions[b.index()].locked);
    pick_when(&mut e, play(0, 0, b));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::ResolvePrinted { minion: b },
    );
    assert!(e.state.minions[b.index()].locked, "once each turn");

    // Pack Alpha [ani]: an animal retainer employed in combat.
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Baba Yaga, the Iron Hag"), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Pack Alpha", "Raven Spy"]);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, b));
    pick_when(&mut e, Choice::ResolvePrinted { minion: b });
    request(&mut e);
    assert!(!e.state.minions[b.index()].locked);
}

// ---- Rulings of the round's cards ----------------------------------------------

// 419 Operation [ANK 20221011-3]: "You can burn the edge to burn the card if
// it has no counter."
#[test]
fn operation_419_burns_with_no_counter_on_it() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Titi Camara"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let op = in_play(&mut e, 0, "419 Operation", None);
    e.state.seats[1].has_edge = true;
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&ability(op.0, 2)));
    pick(&mut e, ability(op.0, 2));
    request(&mut e);
    assert!(!e.state.cards[op.index()].present());
    assert_eq!(pool(&e, 1), 10);
}

// Mimir: "Other Methuselahs cannot play master cards targeting Mimir."
// Archon Investigation targets the bleeding vampire [LSJ 20010817] [PIB
// 20131013]: not playable against his 4-pool bleed.
#[test]
fn mimir_is_no_archon_investigation_target() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mimir"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    in_play(&mut e, 0, "Laptop Computer", Some(m));
    hand(&mut e, 0, &["Computer Hacking"]);
    hand(&mut e, 1, &["Archon Investigation"]);
    pick(&mut e, play(0, 0, m));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        master(0, 0, Target::None),
    );
    assert_eq!(pool(&e, 1), 10 - 4, "a bleed of 4");
}

// Tangle Atropos' Hand and Daring the Dawn [ANK 20220401] [ANK 20211015]
// [ANK 20210124]: "The damage is not applied if the action is canceled
// before resolution." An ally's attempt, the action canceled.
#[test]
fn tangle_atropos_hand_spares_the_dawns_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    superior(&mut e, a, "FOR");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let w = e.add_ally(SeatId(1), find_library("ECTU Operative"));
    hand(&mut e, 0, &["Daring the Dawn", "Tangle Atropos' Hand"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, block(w));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_torpor);
    assert_eq!(blood(&e, a), 5 - 1, "Tangle's blood, no damage");
}

// [LSJ 20070709] [LSJ 20090220] [ANK 20210124] Tangle Atropos' Hand: "The same vampire
// can undertake the same action again, but not if it was blocked then
// continued before (eg. [PRO] Form of Mist)": Abactor blocked by seat 1,
// continued with Torrent, then Tangled against seat 2's attempt: recorded
// as taken; Tangled with no block before, not (The Kiss of Ra's shape).
#[test]
fn tangle_atropos_hand_after_a_continued_block_leaves_the_action_taken() {
    for continued in [true, false] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
        superior(&mut e, a, "cel");
        superior(&mut e, a, "pot");
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let y = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
        let z = e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
        superior(&mut e, z, "aus");
        hand(&mut e, 0, &["Abactor", "Torrent", "Tangle Atropos' Hand"]);
        hand(&mut e, 1, &["Eagle's Sight"]);
        hand(&mut e, 2, &["Eagle's Sight"]);
        pick(&mut e, play(0, 0, a));
        if continued {
            pick_when(&mut e, block(y));
            pick_when(&mut e, play(0, 0, y));
            pick_when(&mut e, play(0, 1, a));
        }
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 0, z));
        let tangle = play(if continued { 0 } else { 1 }, 0, a);
        pick_when(&mut e, tangle);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let abactor = zvtes::state::ActionId::Card(find_library("Abactor"));
        assert_eq!(
            e.state.minions[a.index()].acted.contains(&abactor),
            continued,
            "continued {continued}"
        );
    }
}

// Tangle Atropos' Hand: "The action was still taken" [ANK 20210124]: Waiting
// Game burned its counter as it was announced and keeps it burned; it was
// the phase's first action, so Channel 10 helps the next block [LSJ
// 20100505] [ANK 20210124].
#[test]
fn tangle_atropos_hand_leaves_the_action_taken() {
    let mut e = game(2);
    // A Camarilla vampire: Waiting Game counts non-Anarchs' actions.
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=11, 1), 5, false);
    superior(&mut e, a, "POT");
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let wg = in_play(&mut e, 1, "Waiting Game", None);
    e.state.cards[wg.index()].counters = 10;
    let ch = in_play(&mut e, 1, "Channel 10", None);
    hand(&mut e, 0, &["Tangle Atropos' Hand"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[wg.index()].counters, 9);
    e.state.minions[z.index()].locked = false;
    pick(&mut e, action(b, ActionKind::Hunt));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, ability(ch.0, 0));
    assert_eq!(req.seat, SeatId(1));
}

// Tangle Atropos' Hand and Veil the Legions [LSJ 20070526] [ANK 20210124]:
// "If an action is cancelled after it has been played it still counts as
// one of the next X actions." Veil [OBF] for 1: the Tangled bleed spends
// it, the next action has no stealth.
#[test]
fn tangle_atropos_hand_spends_a_veil_of_the_legions_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    let o = e.add_minion(SeatId(0), plain("Brujah", 1), 5, false);
    superior(&mut e, o, "OBF");
    let c = e.add_minion(SeatId(0), plain("Brujah", 2), 5, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    hand(&mut e, 0, &["Veil the Legions", "Tangle Atropos' Hand"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, o));
    pick(&mut e, Choice::SetX(1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[z.index()].locked = false;
    pick(&mut e, bleed(o));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, o));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(c));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").stealth(&e.state),
        0,
        "the Veil spent"
    );
}

// Vlad Tepes after Fall of the Camarilla and Fall of the Sabbat [LSJ
// 20050124] [ANK 20230503]: "An impersonator can still play cards requiring
// Camarilla or a Camarilla title" (and the Sabbat's): Archon, Abbot. A
// sect title met though the sect is gone [ANK 20230503] [LSJ 20050124].
#[test]
fn vlad_tepes_plays_sect_cards_after_the_fall() {
    for (event, card) in [
        ("Fall of the Camarilla", "Archon"),
        ("Fall of the Sabbat", "Abbot"),
    ] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), find_card("Vlad Tepes"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        in_play(&mut e, 0, event, None);
        hand(&mut e, 0, &[card]);
        let req = request(&mut e);
        assert!(req.options.contains(&play(0, 0, v)), "{card} after {event}");
    }
}

// Festivo dello Estinto on card hunts [RTR 20030519], a stolen hunt's blood
// the target's alone [RTR 20180511] [RTR 20030519]: Abactor's
// hunt at +4, then full from the bank; under Restricted Vitae a vampire who
// must hunt steals 1 from a younger vampire, the rest from the bank, not
// from it.
#[test]
fn festivo_dello_estinto_fills_card_hunts_from_the_bank() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 5..=11, 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    in_play(&mut e, 1, "Festivo dello Estinto", None);
    hand(&mut e, 0, &["Abactor"]);
    pick(&mut e, play(0, 0, v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(Some(blood(&e, v)), e.state.capacity_of(v));

    let mut e = game(2);
    let vc = of_sect(Sect::Sabbat, 5..=11, 0);
    let v = e.add_minion(SeatId(0), vc, 0, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let cap = CRYPT[vc as usize].capacity;
    let y = e.add_minion(
        SeatId(1),
        textless(|c| (2..cap).contains(&c.capacity), 0),
        3,
        false,
    );
    assert!(e.state.younger_than(y, v));
    in_play(&mut e, 1, "Festivo dello Estinto", None);
    let rv = in_play(&mut e, 1, "Restricted Vitae", None);
    let steal = Choice::UseAbility {
        source: AbilitySource::Card(rv),
        ability: 0,
        actor: Some(v),
        target: Target::Minion(y),
    };
    pick_when(&mut e, steal);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, y), 2, "1 stolen");
    assert_eq!(Some(blood(&e, v)), e.state.capacity_of(v));
}

// Superior Mettle [FOR]: "The card cannot be played if the minion cannot
// afford to burn the blood" [ANK 20210226]: with 1 blood, its [for] only;
// the burn's blood beyond the cost as paid.
#[test]
fn superior_mettle_needs_the_blood_of_its_burn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    superior(&mut e, z, "FOR");
    hand(&mut e, 1, &["Superior Mettle"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 0, z));
    assert!(!req.options.contains(&play(0, 1, z)));
    // Adana de Sforza's combat cards cost 1 fewer blood: with 1 blood, the
    // cost 0 and the burn's 1 are there.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Adana de Sforza"), 1, false);
    superior(&mut e, z, "FOR");
    hand(&mut e, 1, &["Superior Mettle"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    offering(&mut e, play(0, 1, z));
}

// A card's burn beyond its cost as paid, surcharges included [ANK
// 20210226]: under Terror Frenzy [ANI] (+1 blood), a 1-blood blocker plays
// Rolling with the Punches [for] for 1, not its [FOR] burn.
#[test]
fn a_burn_card_waits_for_its_blood_beyond_a_surcharge() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "ANI");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    superior(&mut e, z, "FOR");
    hand(&mut e, 0, &["Terror Frenzy"]);
    hand(&mut e, 1, &["Rolling with the Punches"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let req = offering(&mut e, play(0, 0, z));
    assert!(!req.options.contains(&play(0, 1, z)));
}

// De Sade's toll: "His ability applies only to the first time a vampire
// casts his votes, not if they change afterwards" [ANK 20221019]. Mustafa
// casts for, free; an older Toreador turns his cast against: no toll then.
#[test]
fn de_sade_tolls_a_first_cast_only() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("De Sade"), 4, false);
    let old0 = e.add_minion(SeatId(0), find_card("Catalina Vega"), 4, false);
    let mustafa = e.add_minion(SeatId(1), find_card("Mustafa, The Heir"), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, d));
    pick(&mut e, alloc(0, 3));
    pick_when(&mut e, votes(mustafa, true));
    assert_eq!(blood(&e, mustafa), 4);
    let turn = Choice::UseAbility {
        source: AbilitySource::Vampire(mustafa),
        ability: 1,
        actor: Some(old0),
        target: Target::Minion(mustafa),
    };
    pick_when(&mut e, turn);
    let r = e.referendum().expect("referendum");
    let against = r
        .records
        .iter()
        .any(|x| x.source == zvtes::referendum::Source::Vampire(mustafa) && !x.favor);
    assert!(against, "turned against");
    // The Toreador's blood moved to him, no toll taken back.
    assert_eq!(
        (blood(&e, old0), blood(&e, mustafa)),
        (3, 5),
        "no toll on the turned cast"
    );
}
