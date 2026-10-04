//! Slice 11: the pre-built decks' cards. 11a: the cards built from the
//! engine's existing vocabulary — bleed and uncontrolled-region actions,
//! stealth and bleed modifiers, intercept and bleed-redirect reactions,
//! combat staples, and simple masters and locations. Windows with a single
//! option are elided, so the scripts only stop where a real decision
//! exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Stat, Target};
use zvtes::crypt::{Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId, CardInPlay, UncontrolledSlot};

fn play_at(card: u8, mode: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor,
        target,
    }
}

fn ability(source: CardId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(source),
        ability: 0,
        actor: None,
        target: Target::None,
    }
}

/// The `nth` V5 vampire with no printed-text definition matching `f`.
fn vampire(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| f(c) && cards::vampire(*i as u16).is_none())
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a vampire matching the filter")
}

/// The `nth` untitled vampire of capacity 5 or more, of `clan` (any if
/// empty), printing the Discipline `tag` exactly (`dom` basic, `DOM`
/// superior; any if empty).
fn plain(clan: &str, tag: &str, nth: usize) -> u16 {
    vampire(
        |c| {
            (clan.is_empty() || c.clan == clan)
                && (tag.is_empty() || c.disciplines.contains(&tag))
                && c.title == Title::None
                && c.capacity >= 5
        },
        nth,
    )
}

fn pool(e: &Engine, seat: u8) -> i32 {
    e.state.seats[seat as usize].pool
}

/// Put `card` (a CRYPT index) in `seat`'s uncontrolled region with no
/// counters; its slot index.
fn uncontrolled(e: &mut Engine, seat: u8, card: u16) -> u8 {
    let region = &mut e.state.seats[seat as usize].uncontrolled;
    region.push(UncontrolledSlot {
        card,
        counters: 0,
        banished: None,
    });
    region.len() as u8 - 1
}

/// A card already in play for `seat`, not on a minion (a location), with
/// its layers (no play): its index.
fn location(e: &mut Engine, seat: u8, name: &str) -> CardId {
    let card = find_library(name);
    e.state
        .cards
        .push(CardInPlay::new(card, SeatId(seat), None, 0));
    let id = CardId(e.state.cards.len() as u32 - 1);
    for effect in cards::lookup(card).expect("definition").modes[0].effects {
        if let Effect::Layer(modifier) = effect {
            e.state.layers.push(Layer {
                scope: Scope::InPlay,
                modifier: *modifier,
                source: id,
            });
        }
    }
    id
}

// ---- Actions ---------------------------------------------------------------

// Govern the Unaligned [DOM]: "+1 stealth action. Add 3 blood to a younger
// vampire in your uncontrolled region" — chosen as the card is played
// (ANK 20240628-2, a ruling on the same wording), an older one never
// offered; undirected, so the prey cannot block without intercept.
#[test]
fn govern_the_unaligned_adds_3_blood_to_a_younger_vampire_in_your_uncontrolled_region() {
    let mut e = Engine::bare(3);
    // Abraham's text (+1 hand size, Tremere cards) is inert here.
    let g_card = crypt("Abraham Mellon");
    let g = e.add_minion(SeatId(0), g_card, 4, false);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    // Anja's text (unlocking another Hecata after its action) is inert here.
    let older = uncontrolled(&mut e, 0, crypt("\"Mother\" Anja Giovanni"));
    let younger = uncontrolled(&mut e, 0, vampire(|c| c.capacity == 4, 0));
    hand(&mut e, 0, &["Govern the Unaligned"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let govern = |slot| play_at(0, 1, Some(g), Target::Uncontrolled(slot));
    assert!(
        !req.options.contains(&govern(older)),
        "not a younger vampire"
    );
    pick(&mut e, govern(younger));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[younger as usize].counters, 3);
    assert_eq!(e.state.seats[0].uncontrolled[older as usize].counters, 0);
    assert!(e.state.minions[g.index()].locked);
}

// Thing (Gangrel): "+1 stealth action. Add 2 blood to a Gangrel in your
// uncontrolled region." A vampire of another clan is not offered.
#[test]
fn thing_adds_2_blood_to_a_gangrel_in_your_uncontrolled_region() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Gangrel", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let other = uncontrolled(&mut e, 0, plain("Ventrue", "", 0));
    let gangrel = uncontrolled(&mut e, 0, plain("Gangrel", "", 1));
    hand(&mut e, 0, &["Thing"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let thing = |slot| play_at(0, 0, Some(a), Target::Uncontrolled(slot));
    assert!(!req.options.contains(&thing(other)));
    pick(&mut e, thing(gangrel));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[gangrel as usize].counters, 2);
}

// Intimidation [pre]/[PRE]: "Ⓓ Bleed with +1 bleed." / "+2 bleed."
// Govern the Unaligned [dom]: "Ⓓ Bleed with +2 bleed."
#[test]
fn bleed_cards_bleed_with_their_printed_bonus() {
    for (card, mode, tag, n) in [
        ("Intimidation", 0, "pre", 1),
        ("Intimidation", 1, "PRE", 2),
        ("Govern the Unaligned", 0, "dom", 2),
    ] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", tag, 0), 4, false);
        e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        hand(&mut e, 0, &[card]);
        let before = pool(&e, 1);
        pick(&mut e, play(0, mode, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 1), before - 1 - n, "{card} [{tag}]");
    }
}

// Heart of the City: "+1 stealth action. [pre] Put this card on this
// vampire. This vampire gets +1 bleed. A vampire can have only one Heart of
// the City." (2 blood, paid at resolution.)
#[test]
fn heart_of_the_city_goes_on_the_vampire_for_plus_1_bleed_one_per_vampire() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "pre", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    hand(&mut e, 0, &["Heart of the City", "Heart of the City"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let heart = find_library("Heart of the City");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == heart && c.in_play && c.host == Some(a)));
    assert_eq!(blood(&e, a), 4 - 2);
    assert_eq!(e.state.bearer_mod(a, Stat::Bleed, ActionKind::Bleed), 1);
    e.state.minions[a.index()].locked = false;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !req.options.iter().any(|o| matches!(
            o,
            Choice::Play { actor: Some(m), .. } if *m == a
        )),
        "only one Heart of the City: {:?}",
        req.options
    );
}

// ---- Action modifiers --------------------------------------------------------

// Aire of Elation [pre]: "+1 bleed, with an additional +1 bleed if this
// vampire is Toreador (limited)."
#[test]
fn aire_of_elation_gives_a_toreador_an_additional_bleed() {
    // No Toreador with basic Presence lacks printed text: Anne-Marie's
    // (equipment costs her 1 pool less) does not come into play here.
    for (a, n) in [
        (crypt("Anne-Marie Bourgeois, Inconnu Recruiter"), 2),
        (plain("Ventrue", "pre", 0), 1),
    ] {
        let mut e = Engine::bare(3);
        let clan = CRYPT[a as usize].clan;
        let a = e.add_minion(SeatId(0), a, 4, false);
        e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        hand(&mut e, 0, &["Aire of Elation"]);
        let before = pool(&e, 1);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 1), before - 1 - n, "{clan}");
    }
}

/// Seat 0's superior-Dominate vampire bleeds seat 1 (`pool1`) with
/// Foreshadowing Destruction [DOM] and Conditioning in hand; seat 1's
/// superior-Auspex vampire holds Telepathic Misdirection; seat 2 has
/// `pool2`.
fn foreshadowing(pool1: i32, pool2: i32) -> (Engine, MinionId, MinionId) {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "DOM", 0), 5, false);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("", "AUS", 0), 4, false);
    e.state.seats[1].pool = pool1;
    e.state.seats[2].pool = pool2;
    hand(&mut e, 0, &["Foreshadowing Destruction", "Conditioning"]);
    hand(&mut e, 1, &["Telepathic Misdirection"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, a)); // Foreshadowing Destruction [DOM]
    (e, a, t)
}

/// Is Conditioning (now the acting seat's card 0) offered to `a` before
/// the action ends? Takes it if so.
fn conditioning_offered(e: &mut Engine, a: MinionId) -> bool {
    offered_before(e, play(0, 1, a), SeatId(0), WindowLabel::MinionPhase)
}

// Foreshadowing Destruction [DOM]: "+3 bleed if the target Methuselah has 9
// or fewer pool (limited)" — counting against the limit.
#[test]
fn foreshadowing_destruction_adds_3_against_9_or_fewer_pool_and_is_limited() {
    let (mut e, a, _) = foreshadowing(9, 30);
    assert!(!conditioning_offered(&mut e, a), "the limit is used");
    assert_eq!(pool(&e, 1), 9 - 1 - 3);
}

// [PIB 20110817] [RTR 20191031] Foreshadowing Destruction [DOM]: "Can be
// played against a target with 10 or more pool and does not count against
// the limit in this case: it will only have a lingering effect (+3 bleed
// if the target happens to have 9 or less)."
#[test]
fn foreshadowing_destruction_against_10_pool_lingers_outside_the_limit() {
    let (mut e, a, _) = foreshadowing(10, 30);
    assert!(conditioning_offered(&mut e, a), "not counting yet");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 10 - 1 - 3, "Conditioning [DOM]'s +3 only");

    // Bounced onto 9 or fewer pool, the +3 applies.
    let (mut e, _, t) = foreshadowing(12, 8);
    pick_when(&mut e, play_at(0, 1, Some(t), Target::Seat(SeatId(2))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((pool(&e, 1), pool(&e, 2)), (12, 8 - 1 - 3));

    // Conditioning played meanwhile: bounced onto 9 or fewer, both apply —
    // the lingering effect is not undone by the limit (owner, 2026-09-25).
    let (mut e, a, t) = foreshadowing(12, 9);
    assert!(conditioning_offered(&mut e, a));
    pick_when(&mut e, play_at(0, 1, Some(t), Target::Seat(SeatId(2))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((pool(&e, 1), pool(&e, 2)), (12, 9 - 1 - 3 - 3));
}

// [RTR 20191031] [ANK 20211019-2] Foreshadowing Destruction [DOM]: "If the
// bleed bonus is lost during the action (eg. Deflection to another
// target), it does not count against the limit anymore and an additional
// bleed modifier can be used."
#[test]
fn foreshadowing_destruction_bounced_to_10_pool_frees_the_limit() {
    let (mut e, a, t) = foreshadowing(8, 20);
    pick_when(&mut e, play_at(0, 1, Some(t), Target::Seat(SeatId(2))));
    assert!(conditioning_offered(&mut e, a), "the bonus is lost");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((pool(&e, 1), pool(&e, 2)), (8, 20 - 1 - 3));
}

// Monkey Wrench: "Requires an Anarch. Only usable during a bleed action. +X
// bleed (limited). X must be 1, 2 or 3." X is its blood cost: never above
// the Anarch's blood.
#[test]
fn monkey_wrench_bleeds_for_x_paid_in_blood() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(
        SeatId(0),
        vampire(
            |c| c.sect == zvtes::crypt::Sect::Anarch && c.title == Title::None,
            0,
        ),
        2,
        false,
    );
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    hand(&mut e, 0, &["Monkey Wrench"]);
    let before = pool(&e, 1);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(req.options, vec![Choice::SetX(1), Choice::SetX(2)]);
    pick(&mut e, Choice::SetX(2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), before - 1 - 2);
    assert_eq!(blood(&e, a), 0);

    // With no blood left during its bleed (a cost paid, say), X cannot be
    // 1: not offered.
    let mut e = Engine::bare(3);
    let a = e.add_minion(
        SeatId(0),
        vampire(
            |c| c.sect == zvtes::crypt::Sect::Anarch && c.title == Title::None,
            0,
        ),
        2,
        false,
    );
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    hand(&mut e, 0, &["Monkey Wrench"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    e.state.minions[a.index()].blood = 0;
    never_offered(&mut e, &play(0, 0, a));
}

// ---- Reactions ---------------------------------------------------------------

/// Seat 0's plain Brujah bleeds seat 1, whose `reactor` holds `card`.
fn bleed_at(n_seats: usize, reactor: u16, card: &str) -> (Engine, MinionId, MinionId) {
    let mut e = Engine::bare(n_seats);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let r = e.add_minion(SeatId(1), reactor, 4, false);
    hand(&mut e, 1, &[card]);
    (e, a, r)
}

// Telepathic Misdirection [AUS]: after blocks are declined, lock this
// vampire and change the target of the bleed; the new target may block.
#[test]
fn telepathic_misdirection_locks_and_redirects_the_bleed() {
    let (mut e, a, t) = bleed_at(3, plain("", "AUS", 0), "Telepathic Misdirection");
    let before = (pool(&e, 1), pool(&e, 2));
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play_at(0, 1, Some(t), Target::Seat(SeatId(2))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((pool(&e, 1), pool(&e, 2)), (before.0, before.1 - 1));
    assert!(e.state.minions[t.index()].locked);
    assert_eq!(blood(&e, t), 4 - 1);
}

/// The `nth` baron with no printed-text definition.
fn baron(nth: usize) -> u16 {
    vampire(|c| matches!(c.title, Title::Baron(_)), nth)
}

// Bait and Switch: "Requires a baron." Only the baron is offered it.
#[test]
fn bait_and_switch_is_a_barons_redirect() {
    let (mut e, a, b) = bleed_at(3, baron(0), "Bait and Switch");
    let other = e.add_minion(SeatId(1), plain("", "", 0), 4, false);
    pick(&mut e, action(a, ActionKind::Bleed));
    let redirect = |m| play_at(0, 0, Some(m), Target::Seat(SeatId(2)));
    let req = offering(&mut e, redirect(b));
    assert!(!req.options.contains(&redirect(other)));
    pick(&mut e, redirect(b));
    let before = pool(&e, 2);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 2), before - 1);
    assert!(e.state.minions[b.index()].locked);
}

// My Enemy's Enemy [AUS]: the bleed of a minion of your predator goes to
// your predator's predator, and no one else; not usable with only two
// Methuselahs left.
#[test]
fn my_enemys_enemy_redirects_to_the_predators_predator_only() {
    let (mut e, a, t) = bleed_at(4, plain("", "AUS", 0), "My Enemy's Enemy");
    pick(&mut e, action(a, ActionKind::Bleed));
    let to = |s| play_at(0, 1, Some(t), Target::Seat(SeatId(s)));
    let req = offering(&mut e, to(3));
    assert!(!req.options.contains(&to(2)));
    pick(&mut e, to(3));
    let before = pool(&e, 3);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 3), before - 1);

    let (mut e, a, t) = bleed_at(2, plain("", "AUS", 0), "My Enemy's Enemy");
    pick(&mut e, action(a, ActionKind::Bleed));
    // Two Methuselahs left: no predator's predator.
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { mode: 1, actor: Some(m), .. } if *m == t),
    );
}

/// Seat 0's `acting` bleeds with +2 stealth (Lost in Crowds [OBF]; +1 more
/// with Swallowed by the Night [obf]) against seat 1's `blocker` holding
/// `card`: was the bleed blocked?
fn blocked_with(acting: u16, blocker: u16, card: &str, extra_stealth: bool) -> bool {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), acting, 4, false);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), blocker, 4, false);
    hand(&mut e, 0, &["Lost in Crowds"]);
    if extra_stealth {
        hand(&mut e, 0, &["Swallowed by the Night"]);
    }
    hand(&mut e, 1, &[card]);
    let before = pool(&e, 1);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick(&mut e, play(0, 1, a)); // +2 stealth
    pick_when(&mut e, play(0, 0, z));
    if extra_stealth {
        pick_when(&mut e, play(0, 0, a)); // +1 stealth
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pool(&e, 1) == before
}

/// An acting vampire with superior Obfuscate: titled or not.
fn obfuscator(titled: bool) -> u16 {
    vampire(
        |c| c.disciplines.contains(&"OBF") && (c.title != Title::None) == titled,
        0,
    )
}

// Protection Racket: "Requires an Anarch. +1 intercept, with an additional
// +1 intercept if the acting minion is titled."
#[test]
fn protection_racket_gives_an_additional_intercept_against_a_titled_acting_minion() {
    let anarch = vampire(
        |c| c.sect == zvtes::crypt::Sect::Anarch && c.title == Title::None,
        0,
    );
    assert!(blocked_with(
        obfuscator(true),
        anarch,
        "Protection Racket",
        false
    ));
    assert!(!blocked_with(
        obfuscator(false),
        anarch,
        "Protection Racket",
        false
    ));
}

// [ANK 20190416] Protection Racket: "The additional intercept is gained or
// lost if the acting vampire changes sect during the action" — read live:
// the acting prince losing the title after the card was played loses it.
#[test]
fn protection_racket_reads_the_acting_minions_title_live() {
    let anarch = vampire(
        |c| c.sect == zvtes::crypt::Sect::Anarch && c.title == Title::None,
        0,
    );
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), obfuscator(true), 4, false);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), anarch, 4, false);
    hand(&mut e, 0, &["Lost in Crowds"]);
    hand(&mut e, 1, &["Protection Racket"]);
    let before = pool(&e, 1);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick(&mut e, play(0, 1, a)); // +2 stealth
    pick_when(&mut e, play(0, 0, z)); // +1, +1 against a titled vampire
    e.state.minions[a.index()].title_lost = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), before - 1, "1 intercept against 2 stealth");
}

// The Warrens: "+2 intercept, with an additional +1 intercept if this
// Nosferatu is titled" — during an action directed at you.
#[test]
fn the_warrens_gives_a_titled_nosferatu_an_additional_intercept() {
    let titled = vampire(|c| c.clan == "Nosferatu" && c.title != Title::None, 0);
    let untitled = plain("Nosferatu", "", 0);
    assert!(blocked_with(
        obfuscator(false),
        titled,
        "Warrens, The",
        true
    ));
    assert!(!blocked_with(
        obfuscator(false),
        untitled,
        "Warrens, The",
        true
    ));
}

// ---- Combat ------------------------------------------------------------------

/// Seat 0's `a_card` bleeds with `a_hand`; seat 1's `z_card` with `z_hand`
/// blocks.
fn fight(
    a_card: u16,
    a_hand: &[&str],
    z_card: u16,
    z_hand: &[&str],
) -> (Engine, MinionId, MinionId) {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), a_card, 5, false);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), z_card, 5, false);
    hand(&mut e, 0, a_hand);
    hand(&mut e, 1, z_hand);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    (e, a, z)
}

// Earth Meld [PRO]: "Strike: combat ends … and unlock this vampire before
// combat ends."
#[test]
fn earth_meld_ends_combat_and_unlocks_at_superior() {
    let (mut e, a, z) = fight(plain("", "", 1), &[], plain("", "PRO", 0), &["Earth Meld"]);
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].locked);
    assert_eq!((blood(&e, a), blood(&e, z)), (5, 5), "no damage");
}

// [LSJ 20001206] [ANK 20211124] Quickness [CEL]: "A limited additional
// strike can be played before or after." One Quickness each round.
#[test]
fn quickness_superior_does_not_count_against_the_limit_one_per_round() {
    for quickness_first in [true, false] {
        let (mut e, _, z) = fight(
            plain("", "", 1),
            &[],
            plain("", "CEL", 0),
            &["Quickness", "Quickness", "Blur"],
        );
        // Hand [Quickness, Quickness, Blur]: indices shift as cards leave.
        let (first, second) = if quickness_first {
            (play(0, 1, z), play(1, 0, z)) // then Blur
        } else {
            (play(2, 0, z), play(0, 1, z)) // then Quickness
        };
        pick_when(&mut e, first);
        let req = request(&mut e);
        assert!(req.options.contains(&second), "{quickness_first}: {req:?}");
        if quickness_first {
            assert!(
                !req.options.contains(&play(0, 1, z)) && !req.options.contains(&play(0, 0, z)),
                "one Quickness each round: {req:?}"
            );
        }
    }
}

// Rego Motum [THA]: "Prevent 4 damage from the opposing minion's strike."
#[test]
fn rego_motum_prevents_damage_from_the_opposing_strike() {
    let (mut e, _, z) = fight(
        plain("", "POT", 0),
        &[],
        plain("", "THA", 0),
        &["Rego Motum"],
    );
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 5 - 1, "the card's blood only");
}

// Arms of Ahriman [OBL]: "Strike: dodge, with 1 additional strike
// (limited)."
#[test]
fn arms_of_ahriman_dodges_with_an_additional_strike() {
    let (mut e, a, z) = fight(
        plain("", "", 1),
        &[],
        plain("", "OBL", 0),
        &["Arms of Ahriman"],
    );
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // The hand strike against Z is dodged; Z's additional hand strike hits.
    assert_eq!((blood(&e, a), blood(&e, z)), (5 - 1, 5));
}

// Apportation [tha]: "Press, only usable to continue combat." Walk of
// Flame: "Not usable during the first round. [tha] Strike: 1R aggravated
// damage."
#[test]
fn apportation_presses_into_a_round_where_walk_of_flame_strikes() {
    let (mut e, a, z) = fight(
        plain("", "", 1),
        &[],
        plain("", "tha", 0),
        &["Walk of Flame", "Apportation"],
    );
    // Round 1's strike window is elided: the hand strike is Z's only one.
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 0, z)), "first round");
        if let Some(i) = req.options.iter().position(|o| *o == play(1, 0, z)) {
            e.choose(i); // press to continue
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    pick_when(&mut e, play(0, 0, z)); // round 2: 1R aggravated
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Round 1: hand strikes both ways; round 2: A's hand strike and the
    // 1R aggravated, which A cannot heal: torpor.
    assert_eq!(blood(&e, a), 5 - 1);
    assert!(e.state.minions[a.index()].in_torpor);
    assert_eq!(blood(&e, z), 5 - 1 - 1);
}

// Pursuit [cel]: "Maneuver."; Swallowed by the Night [OBF] "[COMBAT]
// Maneuver." — to long range, where hand strikes miss.
#[test]
fn pursuit_and_swallowed_by_the_night_maneuver() {
    for (tag, card, mode) in [("cel", "Pursuit", 0), ("OBF", "Swallowed by the Night", 1)] {
        let (mut e, a, z) = fight(plain("", "", 1), &[], plain("", tag, 0), &[card]);
        pick_when(&mut e, play(0, mode, z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!((blood(&e, a), blood(&e, z)), (5, 5), "{card}: long range");
    }
}

// Lost in Crowds, Swallowed by the Night [obf], Resist Earth's Grasp [CEL]
// and Rapid Change [pro]: "+1 stealth" when needed; the attempt fails.
#[test]
fn stealth_modifiers_beat_a_block_attempt() {
    for (tag, card, mode) in [
        ("obf", "Lost in Crowds", 0),
        ("obf", "Swallowed by the Night", 0),
        ("CEL", "Resist Earth's Grasp", 1),
        ("pro", "Rapid Change", 0),
    ] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", tag, 0), 4, false);
        e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("", "", 1), 4, false);
        hand(&mut e, 0, &[card]);
        let before = pool(&e, 1);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick(&mut e, Choice::DeclareBlock { minion: z });
        pick(&mut e, play(0, mode, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 1), before - 1, "{card}");
    }
}

// ---- Masters -----------------------------------------------------------------

// Life in the City: "Trifle. Add 1 blood to a ready vampire." Misdirection:
// "Lock a minion." Any Methuselah's, a locked one too.
#[test]
fn life_in_the_city_and_misdirection_target_any_methuselahs_minions() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 3, false);
    let torpid = e.add_minion(SeatId(0), plain("", "", 1), 1, true);
    let x = e.add_minion(SeatId(1), plain("", "", 2), 3, false);
    hand(&mut e, 0, &["Life in the City", "Misdirection"]);
    e.at_master_phase();
    e.state.minions[x.index()].locked = true;
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    // A locked minion stays a target of Misdirection (to no effect).
    assert!(req
        .options
        .contains(&play_at(1, 0, None, Target::Minion(x))));
    e.state.minions[x.index()].locked = false;
    let life = |m| play_at(0, 0, None, Target::Minion(m));
    assert!(req.options.contains(&life(x)));
    assert!(!req.options.contains(&life(torpid)), "not a ready vampire");
    pick(&mut e, life(a));
    // A trifle: the master phase action is still there.
    pick(&mut e, play_at(0, 0, None, Target::Minion(x)));
    request(&mut e);
    assert_eq!(blood(&e, a), 4);
    assert!(e.state.minions[x.index()].locked);
}

// Information Highway: "Unique location. +2 transfers."
#[test]
fn information_highway_gives_2_more_transfers() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.state.turn = 1;
    // Anja's text (unlocking another Hecata after its action) is inert here.
    uncontrolled(&mut e, 0, crypt("\"Mother\" Anja Giovanni"));
    location(&mut e, 0, "Information Highway");
    let mut req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let mut transfers = 0;
    while req.window == WindowLabel::InfluenceTransfers {
        let Some(i) = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Transfer { .. }))
        else {
            break;
        };
        transfers += 1;
        e.choose(i);
        req = request(&mut e);
    }
    assert_eq!(transfers, 1 + 2, "turn 1's single transfer and 2 more");
}

// The Labyrinth: "You can lock this card to give a Nosferatu you control
// +1 stealth" — the acting Nosferatu, when stealth is needed.
#[test]
fn the_labyrinth_locks_to_give_the_acting_nosferatu_stealth() {
    let mut e = Engine::bare(3);
    let n = e.add_minion(SeatId(0), plain("Nosferatu", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let labyrinth = location(&mut e, 0, "Labyrinth, The");
    let before = pool(&e, 1);
    pick(&mut e, action(n, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, ability(labyrinth));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[labyrinth.index()].locked);
    assert_eq!(pool(&e, 1), before - 1, "the attempt failed");
}

// Market Square: "Lock to give an Assamite you control +1 intercept"; The
// Anarch Free Press: "You can lock this card to give an Anarch you control
// +1 intercept." — the Banu Haqim, the Anarch attempting to block.
#[test]
fn market_square_and_the_anarch_free_press_lock_to_give_a_blocker_intercept() {
    for (card, blocker, blood) in [
        ("Market Square", plain("Banu Haqim", "", 0), 4),
        ("Anarch Free Press, The", anarch_with(&[], &[], 0), 3),
    ] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "obf", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
        let z = e.add_minion(SeatId(1), blocker, blood, false);
        let loc = location(&mut e, 1, card);
        hand(&mut e, 0, &["Lost in Crowds"]);
        let before = pool(&e, 1);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick(&mut e, Choice::DeclareBlock { minion: z });
        pick(&mut e, play(0, 0, a)); // +1 stealth
        pick_when(&mut e, ability(loc));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 1), before, "{card}: blocked");
    }
}

// ---- Wakes and "if this vampire blocks" riders (slice 11b) ----------------

fn has(c: &zvtes::crypt::CryptCard, tags: &[&str]) -> bool {
    tags.iter().all(|t| c.disciplines.contains(t))
}

// On the Qui Vive: "Only usable by a locked minion. A minion can play only
// one On the Qui Vive between their unlock phases. This minion wakes."
#[test]
fn on_the_qui_vive_wakes_a_locked_minion_once_between_its_unlock_phases() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 1, &["On the Qui Vive", "On the Qui Vive"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 0, z)); // wakes
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 30, "blocked");
    // The second copy: not before Z's next unlock phase.
    pick(&mut e, action(b, ActionKind::Bleed));
    never_offered(&mut e, &play(0, 0, z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].played_since_unlock.is_empty());
}

// On the Qui Vive: "If this minion is an ally, they do not unlock as normal
// during their next unlock phase."
#[test]
fn on_the_qui_vive_keeps_an_ally_locked_through_its_next_unlock_phase() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.state.minions[ally.index()].locked = true;
    // A vampire beside it, so seat 1's minion phase is a decision.
    e.add_minion(SeatId(1), plain("", "", 0), 4, false);
    hand(&mut e, 1, &["On the Qui Vive"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 0, ally));
    pick_when(&mut e, Choice::DeclareBlock { minion: ally });
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[ally.index()].locked);
}

// Shadow Sentinel [OBL]: "Usable by a locked vampire. Choose a locked
// wraith or zombie ally you control. The chosen ally wakes." — played by
// an unlocked vampire too.
#[test]
fn shadow_sentinel_superior_wakes_a_locked_wraith_or_zombie_ally() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let s = e.add_minion(SeatId(1), plain("", "OBL", 0), 4, false);
    let zombie = e.add_ally(SeatId(1), find_library("Bone Shambler"));
    e.state.minions[zombie.index()].locked = true;
    hand(&mut e, 1, &["Shadow Sentinel"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play_at(0, 1, Some(s), Target::Minion(zombie)));
    pick_when(&mut e, Choice::DeclareBlock { minion: zombie });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 30, "blocked by the woken zombie");
}

// [LSJ 20010814-2] Guard Dogs [ANI]: "The maneuver is provided again if a
// second block happens on the same action (eg. after [PRO] Form of Mist)."
// Guard Dogs unlocks Z during a bleed at its Methuselah; Z blocks and has
// the maneuver; Form of Mist [PRO] continues the action; Z wakes (On the
// Qui Vive), blocks again and has it again.
#[test]
fn guard_dogs_maneuver_is_given_again_to_a_second_block() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "PRO", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    // Keegan's text (unlock after his own Gangrel action) is inert here.
    let z = e.add_minion(SeatId(1), crypt("Keegan"), 5, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Form of Mist"]);
    hand(
        &mut e,
        1,
        &["Guard Dogs", "On the Qui Vive", "Enhanced Senses"],
    );
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, z)); // Guard Dogs [ANI]: unlock
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    let req = until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver), "first block");
    e.choose(0); // pass
    pick_when(&mut e, play(0, 1, a)); // Form of Mist [PRO]: combat ends
    pick_when(&mut e, Choice::ContinueAction);
    pick_when(&mut e, play(0, 0, z)); // On the Qui Vive: Z wakes
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, z)); // Enhanced Senses: 1 against 1 stealth
    let req = until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver), "second block");
}

// Guard Dogs: "Unlock this vampire" — in any reaction window of the bleed,
// after blocks are declined too (rules-qa 1.5 `REACTION_UNLOCK`; before
// Faceless Night's lock at resolution, ANK 20171017).
#[test]
fn guard_dogs_unlocks_after_blocks_are_declined_too() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "ani", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 1, &["Guard Dogs"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    // The block window passes with no attempt: blocks are declined.
    until(&mut e, SeatId(1), WindowLabel::ActionBlocksDeclined);
    pick(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].locked);
}

// [LSJ 20010813] [LSJ 20010819-2] Spirit's Touch [AUS], Instinctive
// Reaction [ANI]: "The maneuver can only be used if the block is
// successful, and only in the resulting (block-induced) combat." Spirit's
// Touch [aus] gives none.
#[test]
fn if_this_vampire_blocks_maneuvers_come_with_the_resulting_combat() {
    for (tags, card, mode, rider) in [
        (&["AUS"][..], "Spirit's Touch", 1, true),
        (&["ANI"][..], "Instinctive Reaction", 1, true),
        (&["aus"][..], "Spirit's Touch", 0, false),
    ] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "obf", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), vampire(|c| has(c, tags), 0), 4, false);
        hand(&mut e, 0, &["Lost in Crowds"]);
        hand(&mut e, 1, &[card]);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick(&mut e, Choice::DeclareBlock { minion: z });
        pick(&mut e, play(0, 0, a)); // +1 stealth
        pick_when(&mut e, play(0, mode, z)); // +1 intercept: blocked
                                             // Without a maneuver the range window is elided.
        let maneuver = offered_before(
            &mut e,
            Choice::UseManeuver,
            SeatId(0),
            WindowLabel::MinionPhase,
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(maneuver, rider, "{card} {mode}");
    }
}

// Instinctive Reaction: "Only usable if a minion controlled by your
// predator is acting." A hunt, undirected: the prey (whose predator acts)
// may play it, the acting minion's predator may not.
#[test]
fn instinctive_reaction_needs_a_minion_of_your_predator_acting() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let prey = e.add_minion(SeatId(1), plain("", "ani", 0), 4, false);
    let predator = e.add_minion(SeatId(2), plain("", "ani", 1), 4, false);
    hand(&mut e, 1, &["Instinctive Reaction"]);
    hand(&mut e, 2, &["Instinctive Reaction"]);
    pick(&mut e, action(a, ActionKind::Hunt)); // +1 stealth
    let mut seen = (false, false);
    let mut attempted = Vec::new();
    loop {
        let req = request(&mut e);
        if req.options.contains(&play(0, 0, predator)) {
            seen.1 = true;
        }
        if req.options.contains(&play(0, 0, prey)) {
            seen.0 = true;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        // Each non-acting seat attempts with its vampire once.
        let block = req.options.iter().position(
            |o| matches!(o, Choice::DeclareBlock { minion } if !attempted.contains(minion)),
        );
        if let Some(i) = block {
            if let Choice::DeclareBlock { minion } = req.options[i] {
                attempted.push(minion);
            }
        }
        let i = block.unwrap_or_else(|| fallback(&req));
        e.choose(i);
    }
    assert_eq!(seen, (true, false));
}

// [LSJ 20010813] Precognition [AUS]: "this vampire can prevent 1 damage
// during the first round of the resulting combat if they block" — only if
// the block is successful, only in the resulting combat.
#[test]
fn precognition_prevents_1_damage_in_the_first_round_if_it_blocks() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "AUS", 0), 4, false);
    hand(&mut e, 0, &["Lost in Crowds"]);
    hand(&mut e, 1, &["Precognition"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, z));
    pick_when(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, a), blood(&e, z)),
        (4 - 1, 4),
        "A's strike prevented"
    );
}

// ---- Forced block attempts and more block riders (slice 11b-ii) ----------

/// A card put on `host` (equipment), with its layers: its index.
fn equip(e: &mut Engine, name: &str, host: MinionId) -> CardId {
    let id = location(e, e.state.minions[host.index()].owner.index() as u8, name);
    e.state.cards[id.index()].host = Some(host);
    id
}

/// The `nth` prince with no printed-text definition.
fn prince(nth: usize) -> u16 {
    vampire(|c| matches!(c.title, Title::Prince(_)), nth)
}

// Second Tradition: Domain: "Only usable by a locked prince or justicar.
// This vampire burns 1 blood to unlock and attempt to block with +2
// intercept, even if intercept is not yet needed."
#[test]
fn second_tradition_unlocks_and_attempts_to_block() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let p = e.add_minion(SeatId(1), prince(0), 3, false);
    e.state.minions[p.index()].locked = true;
    hand(&mut e, 1, &["Second Tradition: Domain"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, p));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 30, "blocked");
    // 1 blood burned, 1 damage from A's hand strike.
    assert_eq!(blood(&e, p), 3 - 1 - 1);
}

// [ANK 20210226] Second Tradition: Domain: "The card cannot be played if
// the minion cannot afford to burn the blood."
#[test]
fn second_tradition_needs_the_blood_it_burns() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let p = e.add_minion(SeatId(1), prince(0), 0, false);
    e.state.minions[p.index()].locked = true;
    hand(&mut e, 1, &["Second Tradition: Domain"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    never_offered(&mut e, &play(0, 1, p));
}

// Second Tradition's attempt still pays the acting vampire's block toll:
// against Mithras ("younger vampires must burn 1 blood to attempt to
// block"), a prince left with no blood after burning 1 to unlock cannot
// pay it — unlocked, it makes no attempt (its +2 intercept is its own,
// unused) and the bleed goes through.
#[test]
fn second_tradition_attempt_fails_on_an_unpayable_block_toll() {
    let mut e = Engine::bare(3);
    let m = e.add_minion(SeatId(0), crypt("Mithras"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let p = e.add_minion(SeatId(1), prince(0), 1, false);
    assert!(CRYPT[prince(0) as usize].capacity < CRYPT[crypt("Mithras") as usize].capacity);
    e.state.minions[p.index()].locked = true;
    hand(&mut e, 1, &["Second Tradition: Domain"]);
    pick(&mut e, action(m, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, p));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[p.index()];
    assert_eq!((v.blood, v.locked), (0, false), "1 blood burned, unlocked");
    assert_eq!(pool(&e, 1), 30 - 3, "Mithras' +2 bleed, unblocked");
    assert_eq!(e.state.seats[1].ash.len(), 1, "the card played");
}

// [ANK 20180623] "Unlocks and attempts to block" "does not ignore the
// normal prey, predator or target restrictions": a hunt is blockable by
// the prey and the predator only; the seat across the table cannot use
// it. [ANK 20181122-2] "Can be used by a vampire who is already attempting
// to block" — a locked prince woken by On the Qui Vive, attempting.
#[test]
fn forced_attempts_keep_the_blocking_restrictions_and_an_ongoing_attempt() {
    let mut e = Engine::bare(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let across = e.add_minion(SeatId(2), prince(1), 3, false);
    e.state.minions[across.index()].locked = true;
    hand(&mut e, 2, &["Second Tradition: Domain"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    never_offered(&mut e, &play(0, 1, across));

    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let p = e.add_minion(SeatId(1), prince(0), 3, false);
    e.state.minions[p.index()].locked = true;
    hand(&mut e, 1, &["On the Qui Vive", "Second Tradition: Domain"]);
    pick(&mut e, action(a, ActionKind::Hunt)); // +1 stealth
    pick_when(&mut e, play(0, 0, p)); // wakes
    pick_when(&mut e, Choice::DeclareBlock { minion: p });
    pick_when(&mut e, play(0, 1, p)); // already attempting: +2 intercept
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Blocked: hand strikes both ways; the prince burned 1 blood.
    assert_eq!((blood(&e, a), blood(&e, p)), (4 - 1, 3 - 1 - 1));
}

// Deep Ecology [for]: "Reduce a bleed against you by 2." [pro]: "Only
// usable by a locked vampire. This vampire burns 1 blood to unlock."
#[test]
fn deep_ecology_reduces_a_bleed_and_unlocks_for_blood() {
    let anarch_with = |tag: &'static str| {
        vampire(
            move |c| {
                c.sect == zvtes::crypt::Sect::Anarch
                    && c.title == Title::None
                    && c.disciplines.contains(&tag)
            },
            0,
        )
    };
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "PRE", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let f = e.add_minion(SeatId(1), anarch_with("for"), 4, false);
    let p = e.add_minion(SeatId(1), anarch_with("pro"), 4, false);
    e.state.minions[p.index()].locked = true;
    hand(&mut e, 0, &["Intimidation"]);
    hand(&mut e, 1, &["Deep Ecology", "Deep Ecology"]);
    pick(&mut e, play(0, 1, a)); // bleed 3
    pick_when(&mut e, play(0, 2, p)); // unlock for 1 blood
    pick_when(&mut e, play(0, 1, f)); // the bleed reduced by 2
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 30 - 1);
    assert!(!e.state.minions[p.index()].locked);
    assert_eq!(blood(&e, p), 3);
}

// One With the Land [PRO]: "This vampire unlocks and attempts to block. If
// they block, neither combatant can strike during the first round of the
// resulting combat" — only in the combat of a successful block ([LSJ
// 20010813] [LSJ 20010819-2]). [LSJ 20001126] "Minions that cannot strike, cannot
// play or use the maneuver from a strike" (.44 Magnum). [LSJ 19970814-2]
// "Minions can (uselessly) gain additional strikes even if they cannot
// strike."
#[test]
fn one_with_the_land_blocks_with_no_strikes_in_the_first_round() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "cel", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Tzimisce", "PRO", 0), 5, false);
    e.state.minions[z.index()].locked = true;
    let magnum = equip(&mut e, ".44 Magnum", a);
    hand(&mut e, 0, &["Blur"]);
    hand(&mut e, 1, &["One With the Land"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, z));
    let mut blur = false;
    loop {
        let req = request(&mut e);
        assert!(
            !req.options.iter().any(|o| matches!(
                o,
                Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == magnum
            )),
            "no strike, nor its maneuver: {req:?}"
        );
        if let Some(i) = req.options.iter().position(|o| *o == play(0, 0, a)) {
            blur = true;
            e.choose(i);
            continue;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    assert!(blur, "an additional strike is still gained");
    assert_eq!(pool(&e, 1), 30, "blocked");
    assert_eq!((blood(&e, a), blood(&e, z)), (5 - 1, 5), "Blur's cost only");
}

// [LSJ 20010813] [LSJ 20010819-2] Form of the Bat [pro]: "The maneuver,
// and equipment restriction, apply only in the combat resulting from a
// successful block" — the acting vampire's, if its action is blocked: a
// maneuver, and its .44 Magnum unusable.
#[test]
fn form_of_the_bat_gives_a_maneuver_and_bars_equipment_if_blocked() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "pro", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "aus", 1), 5, false);
    let magnum = equip(&mut e, ".44 Magnum", a);
    hand(&mut e, 0, &["Form of the Bat"]);
    hand(&mut e, 1, &["Enhanced Senses"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, a)); // the attempt gets -1 intercept
    pick_when(&mut e, play(0, 0, z)); // +1: back to 0 against 0 stealth
    let mut maneuver = false;
    loop {
        let req = request(&mut e);
        assert!(
            !req.options.iter().any(|o| matches!(
                o,
                Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == magnum
            )),
            "no equipment: {req:?}"
        );
        maneuver |= req.seat == SeatId(0) && req.options.contains(&Choice::UseManeuver);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    assert!(maneuver);
    assert_eq!(pool(&e, 1), 30, "blocked at 0 against 0");
}

// Form of the Bat [pro]'s -1 intercept is not floored at 0 (rules-qa
// 48.1): a 0-intercept blocker of a 0-stealth action is at -1, so its
// attempt fails and the bleed goes through.
#[test]
fn form_of_the_bat_fails_a_zero_intercept_block_of_a_zero_stealth_action() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "pro", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    hand(&mut e, 0, &["Form of the Bat"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, a));
    // The attempt fails at -1 against 0; no re-attempt can reach 0: the
    // rest of the action elides.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 29, "not blocked: the bleed for 1 succeeds");
    assert_eq!(blood(&e, a), 5, "no combat");
}

// An attempt is offered whenever the blocker holds an intercept card and
// its Methuselah has a library (rules-qa 48.5): Enhanced Senses' +1 is
// short of a Taxicab hunt's 2 stealth, but its replacement may be another.
// With no library to draw from, the attempt stays pruned.
#[test]
fn a_block_is_offered_on_the_chance_of_drawing_more_intercept() {
    for library in [true, false] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let x = e.add_minion(SeatId(1), plain("", "aus", 1), 4, false);
        equip(&mut e, "Unlicensed Taxicab", a);
        hand(&mut e, 1, &["Enhanced Senses"]);
        e.state.seats[1].library = if library {
            vec![find_library("Enhanced Senses")]
        } else {
            vec![]
        };
        pick(&mut e, action(a, ActionKind::Hunt));
        let stealth = e.action().map(|q| q.stealth(&e.state));
        assert_eq!(stealth, Some(2), "hunt and Taxicab");
        let offered = request(&mut e)
            .options
            .contains(&Choice::DeclareBlock { minion: x });
        assert_eq!(offered, library, "library: {library}");
    }
}

// No identical re-attempt (rules-qa 48.6): X's attempt fails with nothing
// played; in the same situation it is not offered again — another minion
// still is.
#[test]
fn a_failed_attempt_is_not_offered_again_in_the_same_situation() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("", "aus", 1), 4, false);
    let y = e.add_minion(SeatId(1), plain("", "aus", 2), 4, false);
    equip(&mut e, "Unlicensed Taxicab", a);
    hand(&mut e, 1, &["Enhanced Senses"]);
    e.state.seats[1].library = vec![find_library("Enhanced Senses")];
    pick(&mut e, action(a, ActionKind::Hunt)); // 2 stealth
    pick(&mut e, Choice::DeclareBlock { minion: x });
    pick(&mut e, Choice::Pass); // nothing played: the attempt fails
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(
        !req.options.contains(&Choice::DeclareBlock { minion: x }),
        "{req:?}"
    );
    assert!(
        req.options.contains(&Choice::DeclareBlock { minion: y }),
        "{req:?}"
    );
}

// Truth in Darkness [OBL]: "if this vampire blocks, they can burn 1 blood
// to unlock after block resolution."
#[test]
fn truth_in_darkness_unlocks_the_blocker_for_blood_after_combat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "OBL", 0), 4, false);
    hand(&mut e, 0, &["Lost in Crowds"]);
    hand(&mut e, 1, &["Truth in Darkness"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, z));
    pick_when(&mut e, Choice::UnlockForBlood { minion: z });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].locked);
    assert_eq!(blood(&e, z), 4 - 1 - 1, "a hand strike, then the unlock");
}

/// The `nth` untitled Anarch vampire with no printed-text definition.
fn anarch(nth: usize) -> u16 {
    vampire(
        |c| c.sect == zvtes::crypt::Sect::Anarch && c.title == Title::None,
        nth,
    )
}

// Organized Resistance: "Requires a baron. Usable by a locked baron. An
// Anarch you control gets +1 intercept."
#[test]
fn organized_resistance_gives_an_attempting_anarch_intercept_from_a_locked_baron() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let b = e.add_minion(SeatId(1), baron(0), 4, false);
    e.state.minions[b.index()].locked = true;
    let z = e.add_minion(SeatId(1), anarch(0), 4, false);
    hand(&mut e, 0, &["Lost in Crowds"]);
    hand(&mut e, 1, &["Organized Resistance"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick(&mut e, play(0, 0, a)); // +1 stealth
    pick_when(&mut e, play(0, 0, b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 30, "blocked");
}

// Organized Resistance from an unlocked baron for its own attempt: a hunt
// (+1 stealth) is blockable by a 0-intercept baron holding it — the
// attempt is offered (its intercept reachable), then the card.
#[test]
fn organized_resistance_lets_an_unlocked_baron_block_a_hunt() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let b = e.add_minion(SeatId(1), baron(0), 4, false);
    hand(&mut e, 1, &["Organized Resistance"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick(&mut e, Choice::DeclareBlock { minion: b });
    pick_when(&mut e, play(0, 0, b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
}

// A change of blocker (rules-qa 48.3): X attempts, the acting minion's
// stealth leaves it short; its Methuselah attempts with Y instead (X's
// attempt fails then), and Y blocks with Enhanced Senses.
#[test]
fn a_blocker_can_be_changed_during_its_attempt() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let y = e.add_minion(SeatId(1), plain("", "aus", 1), 4, false);
    hand(&mut e, 0, &["Lost in Crowds"]);
    hand(&mut e, 1, &["Enhanced Senses"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: x });
    pick_when(&mut e, play(0, 0, a)); // +1 stealth: X is short
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(
        req.options.contains(&Choice::DeclareBlock { minion: y })
            && !req.options.contains(&Choice::DeclareBlock { minion: x }),
        "{req:?}"
    );
    pick(&mut e, Choice::DeclareBlock { minion: y });
    pick_when(&mut e, play(0, 0, y)); // +1 intercept
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert_eq!(pool(&e, 1), 30);
}

// Organized Resistance during another of its Methuselah's attempts: the
// unlocked Anarch must be the one blocking — X's attempt fails, C attempts
// with +1 intercept and blocks (rules-qa 48.3).
#[test]
fn organized_resistance_switches_the_blocker_to_the_unlocked_anarch() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let b = e.add_minion(SeatId(1), baron(0), 4, false);
    let c = e.add_minion(SeatId(1), anarch(0), 4, false);
    e.state.minions[c.index()].locked = true;
    hand(&mut e, 0, &["Lost in Crowds"]);
    hand(&mut e, 1, &["Organized Resistance"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    let c0 = blood(&e, c); // its capacity
    pick(&mut e, Choice::DeclareBlock { minion: x });
    pick_when(&mut e, play(0, 0, a)); // +1 stealth: X is short
    let unlock = Choice::Play {
        card: 0,
        mode: 1,
        actor: Some(b),
        target: Target::Minion(c),
    };
    pick_when(&mut e, unlock);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert_eq!((blood(&e, x), blood(&e, c)), (4, c0 - 1), "C fought, not X");
    assert_eq!(pool(&e, 1), 30);
}

// Organized Resistance: "Also usable to unlock a locked Anarch you
// control; that Anarch attempts to block with +1 intercept, even if
// intercept is not yet needed." Not a locked non-Anarch.
#[test]
fn organized_resistance_unlocks_an_anarch_into_a_block_attempt() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let b = e.add_minion(SeatId(1), baron(0), 4, false);
    let z = e.add_minion(SeatId(1), anarch(0), 4, false);
    let other = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    e.state.minions[other.index()].locked = true;
    hand(&mut e, 0, &["Lost in Crowds"]);
    hand(&mut e, 1, &["Organized Resistance"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    let at = |m| play_at(0, 1, Some(b), Target::Minion(m));
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&at(other)), "not an Anarch");
        if let Some(i) = req.options.iter().position(|o| *o == at(z)) {
            e.choose(i);
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    // Z attempts with 1 intercept: Lost in Crowds (+1) only ties it.
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 30, "blocked");
    assert!(e.state.minions[z.index()].locked, "a blocker locks");
}

// Eyes of the Wild: "This Gangrel unlocks and attempts to block. During
// this action, this Gangrel can burn 1 blood to get +1 intercept. Lock
// this Gangrel after action resolution if they did not block this action."
#[test]
fn eyes_of_the_wild_burns_blood_for_intercept_and_relocks_without_a_block() {
    for (burns, blocked) in [(2, true), (1, false)] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "OBF", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let g = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
        e.state.minions[g.index()].locked = true;
        hand(&mut e, 0, &["Lost in Crowds"]);
        hand(&mut e, 1, &["Eyes of the Wild"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick_when(&mut e, play(0, 0, g)); // unlock and attempt
        pick_when(&mut e, play(0, 1, a)); // +2 stealth
        for _ in 0..burns {
            pick_when(&mut e, Choice::BurnBloodForIntercept { minion: g });
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 1) == 30, blocked, "{burns} blood");
        assert!(e.state.minions[g.index()].locked, "blocked, or relocked");
        if !blocked {
            assert_eq!(blood(&e, g), 4 - 1);
        }
    }
}

/// The `nth` Ravnos with no printed-text definition printing both
/// Disciplines (`obf`/`OBF` and `pre`/`PRE`): Night Terrors is a Ravnos
/// card.
fn with_both(a: &'static str, b: &'static str, nth: usize) -> u16 {
    vampire(move |c| c.clan == "Ravnos" && has(c, &[a, b]), nth)
}

// [ANK 20200607] Night Terrors, a reaction targeting another player:
// "Other Methuselahs must declare block attempts (or not) and pass the
// impulse to you as normal." A hunt (+1 stealth) across a 4-seat table:
// the prey's vampire, no intercept in hand, is offered the attempt only
// because a third party holds Night Terrors (rules-qa 1.2) — and blocks
// once it is played.
#[test]
fn night_terrors_from_a_third_party_strips_stealth_for_the_attempt() {
    for third_party in [false, true] {
        let mut e = Engine::bare(4);
        let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
        let t = e.add_minion(SeatId(2), with_both("obf", "pre", 0), 4, false);
        if third_party {
            hand(&mut e, 2, &["Night Terrors"]);
        }
        pick(&mut e, action(a, ActionKind::Hunt));
        let block = Choice::DeclareBlock { minion: z };
        let req = request(&mut e);
        assert_eq!(req.options.contains(&block), third_party, "{req:?}");
        if !third_party {
            // Nothing to decide: the action went through.
            assert_eq!(
                (req.seat, req.window),
                (SeatId(0), WindowLabel::MinionPhase)
            );
            continue;
        }
        e.choose(req.options.iter().position(|o| *o == block).unwrap());
        pick_when(&mut e, play(0, 0, t));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), 4 - 1, "blocked: hand strikes, no hunt");
        assert_eq!(blood(&e, t), 4 - 1, "Night Terrors' blood");
    }
}

// [LSJ 20020612] Night Terrors "can be played/used any time during the
// action before resolution; a block attempt is not required." [RTR
// 20030519] It "effectively gives a -X stealth modifier to the action,
// where X is the amount of stealth that was reduced when this card was
// played": stealth added afterwards stands. [LSJ 20021211] Played at zero
// stealth, it leaves the stealth unchanged.
#[test]
fn night_terrors_is_a_fixed_reduction_of_the_stealth_as_played() {
    for hunt in [true, false] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "obf", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), with_both("obf", "pre", 0), 4, false);
        hand(&mut e, 0, &["Lost in Crowds"]);
        hand(&mut e, 1, &["Night Terrors"]);
        let kind = if hunt {
            ActionKind::Hunt
        } else {
            ActionKind::Bleed
        };
        pick(&mut e, action(a, kind));
        pick(&mut e, play(0, 0, z)); // before any attempt: stealth to 0
        pick_when(&mut e, Choice::DeclareBlock { minion: z });
        pick_when(&mut e, play(0, 0, a)); // +1 stealth: 1 against 0
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if hunt {
            assert_eq!(blood(&e, a), 4 + 1, "the hunt succeeded");
        } else {
            assert_eq!(pool(&e, 1), 30 - 1, "the bleed succeeded");
        }
    }
}

// [LSJ 20010813] [LSJ 20010819-2] Night Terrors [OBF][PRE]: "The strike:
// combat ends can only be used in the combat resulting from a successful
// block."
#[test]
fn night_terrors_superior_gives_the_blocker_a_combat_ends_strike() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_both("OBF", "PRE", 0), 4, false);
    hand(&mut e, 1, &["Night Terrors"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick(&mut e, play(0, 1, z)); // stealth to 0, and the rider
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, Choice::StrikeCombatEnds);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (4, 4 - 1), "no damage");
}

// ---- Block attempts that fail or cost (slice 11c) -------------------------

/// Pass through the action until seat 0's minion phase, asserting `bad`
/// is never offered.
fn never_offered(e: &mut Engine, bad: &Choice) {
    never_offered_until(e, SeatId(0), WindowLabel::MinionPhase, *bad);
}

// Forced Confessional [DOM]: "That attempt fails and the blocking minion
// cannot attempt to block this action again."
#[test]
fn forced_confessional_fails_the_attempt_for_the_rest_of_the_action() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Salubri", "DOM", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    hand(&mut e, 0, &["Forced Confessional"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 1, a));
    never_offered(&mut e, &Choice::DeclareBlock { minion: z });
    assert_eq!(pool(&e, 1), 30 - 1);
}

// [ANK 20210226] Stygian Shroud [OBL]: "Burn 1 blood to have that attempt
// fail" — "the card cannot be played if the minion cannot afford to burn
// the blood."
#[test]
fn stygian_shroud_burns_blood_to_fail_the_attempt() {
    for left in [1, 0] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "OBL", 0), left, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
        hand(&mut e, 0, &["Stygian Shroud"]);
        // A 0-blood vampire must hunt: the bleed is declared by hand.
        if left == 0 {
            e.state.minions[a.index()].blood = 1;
        }
        pick(&mut e, action(a, ActionKind::Bleed));
        e.state.minions[a.index()].blood = left;
        pick(&mut e, Choice::DeclareBlock { minion: z });
        if left == 0 {
            never_offered(&mut e, &play(0, 1, a));
            assert_eq!(pool(&e, 1), 30, "blocked");
        } else {
            pick_when(&mut e, play(0, 1, a));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(pool(&e, 1), 30 - 1);
            assert_eq!(blood(&e, a), 0);
        }
    }
}

// True Love's Face [OBF][PRE]: "That attempt fails … The blocking minion's
// controller can burn 1 pool to cancel this card as it is played."
#[test]
fn true_loves_face_fails_the_attempt_unless_its_blocker_pays_to_cancel_it() {
    for cancel in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), vampire(|c| has(c, &["OBF", "PRE"]), 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
        hand(&mut e, 0, &["True Love's Face"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick_when(&mut e, Choice::DeclareBlock { minion: z });
        pick_when(&mut e, play(0, 1, a));
        if cancel {
            pick(&mut e, Choice::PayToCancel);
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if cancel {
            assert_eq!(pool(&e, 1), 30 - 1, "the cancel's pool");
            assert!(e.state.minions[z.index()].locked, "blocked");
        } else {
            assert_eq!(pool(&e, 1), 30 - 1, "the bleed");
            assert!(!e.state.minions[z.index()].locked, "no block");
        }
    }
}

// [LSJ 20031121-2] Where the Veil Thins [OBL]: "minions without Oblivion
// must burn 1 blood to attempt to block" — "A minion previously attempting
// to block can resign his attempt and pay no blood." A new attempt pays.
#[test]
fn where_the_veil_thins_makes_an_attempt_pay_or_resign() {
    for pay in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "OBL", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("", "aus", 0), 4, false);
        hand(&mut e, 0, &["Where the Veil Thins"]);
        // An intercept card keeps a new attempt reachable.
        hand(&mut e, 1, &["Enhanced Senses"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick(&mut e, Choice::DeclareBlock { minion: z });
        pick(&mut e, play(0, 1, a));
        let req = expect(&mut e, SeatId(1), WindowLabel::BlockToll);
        assert_eq!(req.options, vec![Choice::PayBlockToll, Choice::Pass]);
        e.choose(if pay { 0 } else { 1 });
        assert_eq!(blood(&e, z), if pay { 4 - 1 } else { 4 });
        if !pay {
            // Resigned: a new attempt pays.
            pick_when(&mut e, Choice::DeclareBlock { minion: z });
            request(&mut e);
            assert_eq!(blood(&e, z), 4 - 1);
        }
    }
}

// Where the Veil Thins [OBL]: "(allies cannot burn blood)" — no attempt.
#[test]
fn where_the_veil_thins_bars_allies_from_blocking() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "OBL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    hand(&mut e, 0, &["Where the Veil Thins"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick(&mut e, play(0, 1, a));
    pick(&mut e, Choice::Pass); // resign
    never_offered(&mut e, &Choice::DeclareBlock { minion: ally });
}

// [LSJ 20081202] Faceless Night [OBF]: "A minion attempting to block before
// it is played cannot back out." [ANK 20190509] [LSJ 20020101] "A minion
// that has attempted to block but failed before it was played is not
// affected." [ANK 20171017] [ANK 20230305] "The aspirant blocker is locked
// at resolution. He can play Deflection before that."
#[test]
fn faceless_night_locks_the_blockers_failing_after_it_at_resolution() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "OBF", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let early = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    let late = e.add_minion(SeatId(1), vampire(|c| has(c, &["DOM", "aus"]), 0), 4, false);
    hand(&mut e, 0, &["Lost in Crowds", "Faceless Night"]);
    hand(&mut e, 1, &["Deflection", "Enhanced Senses"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: early });
    pick(&mut e, play(0, 0, a)); // +1 stealth: the early attempt fails
    pick_when(&mut e, Choice::DeclareBlock { minion: late });
    pick_when(&mut e, play(1, 0, late)); // Enhanced Senses: 1 against 1
    pick_when(&mut e, play(0, 1, a)); // Faceless Night [OBF]: +1 stealth
                                      // The late attempt fails; before resolution its minion, unlocked
                                      // still, deflects the bleed.
    pick_when(&mut e, play_at(0, 1, Some(late), Target::Seat(SeatId(2))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 2), 30 - 1, "deflected");
    assert!(e.state.minions[late.index()].locked, "locked at resolution");
    assert!(!e.state.minions[early.index()].locked, "failed before");
}

/// Seat 0's Bone Shambler (a zombie ally) bleeds; seat 0 also controls
/// Parijat, the Dark Oracle, ready.
fn zombie_bleed(library: &[&str]) -> (Engine, MinionId) {
    let mut e = Engine::bare(3);
    let shambler = e.add_ally(SeatId(0), find_library("Bone Shambler"));
    e.add_minion(
        SeatId(0),
        zvtes::state::find_card("Parijat, the Dark Oracle"),
        4,
        false,
    );
    e.state.seats[1].library = library.iter().map(|c| find_library(c)).collect();
    pick(&mut e, action(shambler, ActionKind::Bleed));
    (e, shambler)
}

// [ANK 20260317] Parijat: "If you cannot burn a card from the top of your
// library (e.g., your library is empty), you cannot attempt to block
// wraith or zombie allies while Parijat is ready."
#[test]
fn parijat_tolls_a_block_of_a_zombie_with_the_top_library_card() {
    let (mut e, _) = zombie_bleed(&[]);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    never_offered(&mut e, &Choice::DeclareBlock { minion: z });

    let (mut e, _) = zombie_bleed(&["Deflection", "Conditioning"]);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    pick(&mut e, Choice::DeclareBlock { minion: z });
    request(&mut e);
    assert_eq!(e.state.seats[1].ash, vec![find_library("Conditioning")]);
}

// [ANK 20260317] Parijat: "If a card forces a block attempt (eg, Second
// Tradition: Domain), burning the top of their library is part of the cost
// of playing the card. The cost is paid if the card is not cancelled and
// after the replacement card has been drawn."
#[test]
fn parijat_tolls_a_forced_attempt_after_its_card_is_replaced() {
    let (mut e, _) = zombie_bleed(&["Deflection", "Conditioning"]);
    let p = e.add_minion(SeatId(1), prince(0), 3, false);
    e.state.minions[p.index()].locked = true;
    // A full hand: the replacement draws one card.
    hand(&mut e, 1, &["Second Tradition: Domain"]);
    for _ in 0..6 {
        hand(&mut e, 1, &["Soak"]);
    }
    pick_when(&mut e, play(0, 1, p));
    request(&mut e);
    // Conditioning (the top) replaced the card; Deflection paid the toll.
    assert!(e.state.seats[1]
        .hand
        .contains(&find_library("Conditioning")));
    assert!(e.state.seats[1].ash.contains(&find_library("Deflection")));
}

// ---- Cancellation (slice 11d) ----------------------------------------------

/// Pass through every decision until `seat` is offered `want`; take it.
fn pick_for(e: &mut Engine, seat: SeatId, want: Choice) {
    for _ in 0..200 {
        let req = request(e);
        if req.seat == seat {
            if let Some(i) = req.options.iter().position(|o| *o == want) {
                e.choose(i);
                return;
            }
        }
        let i = fallback(&req);
        e.choose(i);
    }
    panic!("{want:?} never offered to {seat:?}");
}

/// A Methuselah's play of card `card` (no actor, no target).
fn master(card: u8) -> Choice {
    play_at(card, 0, None, Target::None)
}

// Direct Intervention: "Cancel a minion card as it is played, and its cost
// is not paid. (If it was an action card, the acting minion does not
// lock.)" "If an action is canceled, it is not performed and the minion is
// free to attempt the same action again." [RBK cancel-a-card] [ANK
// 20200813-2] It "can be used to cancel all action cards".
#[test]
fn direct_intervention_cancels_an_action_card_the_minion_stays_unlocked() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "PRE", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    hand(&mut e, 0, &["Intimidation"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, play(0, 1, a));
    pick_when(&mut e, master(0));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked);
    assert!(
        req.options.contains(&action(a, ActionKind::Bleed)),
        "free to act"
    );
    assert_eq!(pool(&e, 1), 30 - 1, "Direct Intervention's pool");
    assert!(e.state.seats[0].ash.contains(&find_library("Intimidation")));
    assert_eq!(blood(&e, a), 4, "the action card's cost not paid");
}

// Direct Intervention: "If it was a strike card, the minion chooses another
// strike."
#[test]
fn direct_intervention_cancels_a_strike_card_and_the_minion_strikes_again() {
    let (mut e, a, z) = fight(
        plain("", "pro", 0),
        &["Earth Meld"],
        plain("Ventrue", "", 0),
        &[],
    );
    hand(&mut e, 1, &["Direct Intervention"]);
    pick_when(&mut e, play(0, 0, a)); // Earth Meld: combat ends
    pick_when(&mut e, master(0)); // canceled
                                  // A's hand strike is its only strike left (elided): the round is
                                  // fought.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (5 - 1, 5 - 1));
    assert!(e.state.seats[0].ash.contains(&find_library("Earth Meld")));
}

// Wash: "Cancel a master card played by your predator or prey as it is
// played, and its cost is not paid. That Methuselah gets +1 master phase
// action immediately." [ANK 20170124] [LSJ 20090626] Not a trifle's bonus.
#[test]
fn wash_cancels_a_master_and_its_player_gets_a_master_phase_action_back() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    hand(&mut e, 0, &["Misdirection", "Misdirection"]);
    hand(&mut e, 1, &["Wash"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(x)));
    pick_when(&mut e, master(0)); // Wash, by seat 0's prey
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    // The action back: the second Misdirection is playable.
    assert!(req
        .options
        .contains(&play_at(0, 0, None, Target::Minion(x))));
    assert!(!e.state.minions[x.index()].locked);
    assert_eq!(pool(&e, 0), 30, "Misdirection's pool not paid");
    // Wash is not replaced until its player's unlock phase.
    assert!(e.state.seats[1].hand.is_empty());
    let _ = a;
}

// [LSJ 20070309-2] Wash: "If used to cancel an out-of-turn master, that
// Methuselah still cannot play another out-of-turn master against the same
// master phase" — the master phase action comes at the start of their next
// master phase.
#[test]
fn wash_on_an_out_of_turn_master_gives_the_action_at_the_next_master_phase() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    e.add_minion(SeatId(2), plain("Tremere", "", 0), 4, false);
    hand(&mut e, 0, &["Misdirection"]);
    // Seat 2, seat 0's predator, plays Sudden Reversal out of turn; seat 1
    // (seat 2's prey) washes it.
    hand(&mut e, 2, &["Sudden Reversal", "Sudden Reversal"]);
    hand(&mut e, 1, &["Wash"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(x)));
    pick_for(&mut e, SeatId(2), master(0)); // Sudden Reversal
    pick_for(&mut e, SeatId(1), master(0)); // Wash cancels it
    request(&mut e);
    assert!(e.state.minions[x.index()].locked, "Misdirection resolved");
    assert_eq!(e.state.seats[2].mpa_next, 1);
    assert!(e.state.seats[2].out_of_turn.is_some(), "still spent");
}

// [ANK 20260317] Parijat: a forced attempt's toll "is paid if the card is
// not cancelled" (owner, 2026-09-25: over ANK 20260216) — Second
// Tradition: Domain canceled by Direct Intervention costs no library card.
#[test]
fn a_canceled_forced_attempt_pays_no_parijat_toll() {
    let (mut e, _) = zombie_bleed(&["Deflection", "Conditioning"]);
    let p = e.add_minion(SeatId(1), prince(0), 3, false);
    e.state.minions[p.index()].locked = true;
    e.add_minion(SeatId(2), plain("Tremere", "", 0), 4, false);
    hand(&mut e, 1, &["Second Tradition: Domain"]);
    for _ in 0..6 {
        hand(&mut e, 1, &["Soak"]);
    }
    hand(&mut e, 2, &["Direct Intervention"]);
    pick_when(&mut e, play(0, 1, p));
    pick_for(&mut e, SeatId(2), master(0));
    request(&mut e);
    assert!(e.state.minions[p.index()].locked, "no attempt");
    assert_eq!(
        e.state.seats[1].ash,
        vec![find_library("Second Tradition: Domain")],
        "no toll"
    );
    assert_eq!(e.state.seats[1].library, vec![find_library("Deflection")]);
}

// [LSJ 20080630] [LSJ 20011023] "If the canceled card had a ""do not
// replace until"" clause …, that clause is canceled as well and the card
// is replaced normally": Wash, canceled by Sudden Reversal, is replaced at
// once.
#[test]
fn a_canceled_wash_is_replaced_at_once() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    e.add_minion(SeatId(2), plain("Tremere", "", 0), 4, false);
    hand(&mut e, 0, &["Misdirection"]);
    hand(&mut e, 1, &["Wash"]);
    hand(&mut e, 2, &["Sudden Reversal"]);
    e.state.seats[1].library = vec![find_library("Soak")];
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(x)));
    pick_for(&mut e, SeatId(1), master(0)); // Wash on Misdirection
    pick_for(&mut e, SeatId(2), master(0)); // Sudden Reversal on Wash
    request(&mut e);
    assert!(e.state.minions[x.index()].locked, "Misdirection resolved");
    assert_eq!(e.state.seats[1].hand, vec![find_library("Soak")]);
}

// [LSJ 20030224] "If a limited effect is canceled (bleed, additional
// strike), then the limit is not triggered and another limited effect can
// be used": Conditioning canceled by Direct Intervention, Bonding [dom]
// (+1 bleed, limited) is still playable — not another Conditioning: the
// canceled one was played (rules-qa 46.1).
#[test]
fn a_canceled_limited_bleed_leaves_the_limit_unused() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "dom", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    hand(&mut e, 0, &["Conditioning", "Bonding"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 0, a));
    pick_for(&mut e, SeatId(1), master(0)); // canceled
    pick_when(&mut e, play(0, 0, a)); // Bonding: the limit is unused
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 30 - 1 - 1 - 1);
}

// ---- Titles and votes (slice 11e) -------------------------------------------

fn crypt(name: &str) -> u16 {
    zvtes::state::find_card(name)
}

/// An untitled Camarilla Toreador with no printed-text definition.
fn camarilla_toreador(nth: usize) -> u16 {
    vampire(
        |c| {
            c.clan == "Toreador"
                && c.sect == zvtes::crypt::Sect::Camarilla
                && c.title == Title::None
                && c.capacity >= 5
        },
        nth,
    )
}

// [LSJ 20041026] Toreador Justicar: "Only a Camarilla vampire can be
// chosen." The card's text: "Choose a ready Toreador. Successful referendum
// means this card is put on the chosen Toreador to represent the unique
// Camarilla title of Toreador Justicar. In this referendum, each ready
// Toreador gets +1 vote" — an Anarch Toreador included.
#[test]
fn toreador_justicar_titles_a_ready_camarilla_toreador() {
    let mut e = Engine::bare(3);
    let t = e.add_minion(SeatId(0), camarilla_toreador(0), 5, false);
    let aisha = e.add_minion(SeatId(0), crypt("Aisha az-Zahra"), 5, false);
    let other = e.add_minion(SeatId(1), camarilla_toreador(1), 5, false);
    e.add_minion(SeatId(2), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Toreador Justicar"]);
    pick(&mut e, play(0, 0, t));
    let req = until(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert_eq!(
        req.options,
        vec![Choice::ChooseMinion(t), Choice::ChooseMinion(other)],
        "an Anarch Toreador cannot be chosen"
    );
    pick(&mut e, Choice::ChooseMinion(t));
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    // Untitled, each ready Toreador has the card's +1 vote.
    assert!(req.options.contains(&votes(t, true)));
    assert!(req.options.contains(&votes(aisha, true)));
    pick(&mut e, votes(t, true));
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(t), Title::Justicar("Toreador"));
    assert_eq!(e.state.title_votes_of(t), 3);
    let (card, _) = e.state.title_card_on(t).expect("the title card on him");
    assert_eq!(
        e.state.cards[card.index()].card,
        find_library("Toreador Justicar")
    );
    assert!(e.state.seats[0].ash.is_empty(), "in play, not discarded");
}

// [LSJ 20041026] [LSJ 20100129] [ANK 20191228] Toreador Justicar "cannot be
// played without a valid target": no ready Camarilla Toreador, no play.
#[test]
fn toreador_justicar_is_not_offered_without_a_ready_camarilla_toreador() {
    let mut e = Engine::bare(3);
    let aisha = e.add_minion(SeatId(0), crypt("Aisha az-Zahra"), 5, false);
    e.add_minion(SeatId(0), camarilla_toreador(0), 3, true);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Toreador Justicar"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, aisha)));
    e.state.minions[1].in_torpor = false;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&play(0, 0, aisha)));
}

// Rulebook "Vampiric Sects": "a vampire cannot have more than one title. If
// a vampire with a title gains another, they lose the first title"; the
// glossary's title card "is a placeholder for a title. If the title is
// yielded or lost, the card is burned." A prince given the justicar card is
// no prince any more, even once the card is gone.
#[test]
fn a_title_card_replaces_a_printed_title_for_good() {
    let mut e = Engine::bare(3);
    let m = e.add_minion(SeatId(0), crypt("Mkhokheli"), 5, false);
    e.add_minion(SeatId(0), camarilla_toreador(0), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Toreador Justicar"]);
    pick(&mut e, play(0, 0, m));
    pick_when(&mut e, Choice::ChooseMinion(m));
    pick_when(&mut e, votes(m, true));
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(m), Title::Justicar("Toreador"));
    assert!(e.state.minions[m.index()].title_lost);
    let (card, _) = e.state.title_card_on(m).expect("the title card");
    e.state.cards[card.index()].in_play = false;
    assert_eq!(e.state.title_of(m), Title::None, "Cape Town stays lost");
}

// Saulot's Guiding Wisdom is a unique card: a second copy contests as a
// card first, paid in pool (owner, 2026-09-25: rules-qa 19.1). Face down,
// neither copy's title is claimed, so there is no title contest.
#[test]
fn saulots_guiding_wisdom_contests_as_a_unique_card_not_as_a_title() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), crypt("Castellan"), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), crypt("Malachi"), 5, false);
    let theirs = equip(&mut e, "Saulot's Guiding Wisdom", b);
    assert_eq!(
        e.state.title_of(b),
        Title::Independent {
            name: "Saulot's Guiding Wisdom",
            votes: 2
        }
    );
    hand(&mut e, 0, &["Saulot's Guiding Wisdom"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(a)));
    request(&mut e);
    let mine = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[mine.index()].host, Some(a));
    for (m, c) in [(a, mine), (b, theirs)] {
        assert!(e.state.cards[c.index()].contested);
        assert!(!e.state.minions[m.index()].title_contested);
        assert_eq!(e.state.title_of(m), Title::None);
    }
    assert_eq!(pool(&e, 0), 29, "its 1 pool cost");
    // Seat 1's upkeep in their unlock phase: the card, in pool.
    let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    let contest = zvtes::game::Contest::Library(theirs);
    assert!(
        req.options.contains(&Choice::PayContest(contest)),
        "{:?}",
        req.options
    );
    let before = blood(&e, b);
    pick(&mut e, Choice::PayContest(contest));
    assert_eq!(blood(&e, b), before);
    assert_eq!(pool(&e, 1), 29);
}

// Saulot's Guiding Wisdom: "This Salubri can lock during any referendum to
// force a vampire to abstain" — one that has not cast yet included
// (advanced rules §3.7.6.3).
#[test]
fn saulots_guiding_wisdom_locks_to_force_an_abstention() {
    let mut e = Engine::bare(3);
    let p = e.add_minion(SeatId(0), prince(0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let s = e.add_minion(SeatId(1), crypt("Malachi"), 5, false);
    let sgw = equip(&mut e, "Saulot's Guiding Wisdom", s);
    e.add_minion(SeatId(2), plain("Brujah", "", 0), 5, false);
    hand(&mut e, 0, &["Anarchist Uprising"]);
    pick(&mut e, play(0, 0, p));
    let force = Choice::UseAbility {
        source: AbilitySource::Card(sgw),
        ability: 0,
        actor: None,
        target: Target::Minion(p),
    };
    pick_for(&mut e, SeatId(1), force);
    assert!(e.state.minions[s.index()].locked);
    let req = until(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert!(!req.options.contains(&votes(p, true)), "abstaining");
    // Locked, the Salubri still votes with its title's 2 votes.
    let req = until(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert!(req.options.contains(&votes(s, false)));
}

// Saulot's Guiding Wisdom: "Once each turn, this Salubri can lock before
// range is determined to end a combat involving another minion you
// control." No strike resolves (advanced rules §4.7.2).
#[test]
fn saulots_guiding_wisdom_ends_a_combat_of_another_minion() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let s = e.add_minion(SeatId(1), crypt("Malachi"), 5, false);
    let sgw = equip(&mut e, "Saulot's Guiding Wisdom", s);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    let end = Choice::UseAbility {
        source: AbilitySource::Card(sgw),
        ability: 1,
        actor: None,
        target: Target::None,
    };
    pick_for(&mut e, SeatId(1), end);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[s.index()].locked);
    assert_eq!((blood(&e, a), blood(&e, z)), (5, 5), "no strike resolved");
    assert_eq!(pool(&e, 1), 30, "blocked");
    // Once each turn: unlocked again, the Salubri cannot end the next one.
    e.state.minions[s.index()].locked = false;
    let z2 = e.add_minion(SeatId(1), plain("Gangrel", "", 1), 5, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", "", 1), 5, false);
    pick(&mut e, action(b, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z2 });
    never_offered(&mut e, &end);
}

// Valerius: "If Valerius is Camarilla and not a prince, he can steal the
// title held by a prince as a Ⓓ action." The prince loses the title,
// Valerius holds it — and, a prince, cannot steal again. (A card
// representing the title would be burned: rules-qa 19.6.)
#[test]
fn valerius_steals_a_princes_title() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), crypt("Valerius"), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let p = e.add_minion(SeatId(1), crypt("Donny Kowalczyk"), 5, false);
    let steal = Choice::UseAbility {
        source: AbilitySource::Vampire(v),
        ability: 0,
        actor: Some(v),
        target: Target::Minion(p),
    };
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let offered: Vec<&Choice> = req
        .options
        .iter()
        .filter(|o| matches!(o, Choice::UseAbility { .. }))
        .collect();
    assert_eq!(offered, vec![&steal], "only a prince");
    pick(&mut e, steal);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(v), Title::Prince("Pittsburgh"));
    assert_eq!(e.state.title_of(p), Title::None);
    assert!(e.state.minions[p.index()].title_lost);
    e.state.minions[v.index()].locked = false;
    e.state.minions[v.index()].acted.clear();
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::UseAbility { .. })),
        "a prince now"
    );
}

// Valerius's stolen title is his without a card: contested by the Baron of
// the same city (a prince and a baron of one city contest, rulebook
// "Titles") and yielded, it is gone — no card to burn, no printed title
// back — and the baron holds his title uncontested.
#[test]
fn valerius_yields_a_stolen_title_and_keeps_none() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), crypt("Valerius"), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let p = e.add_minion(SeatId(1), crypt("Donny Kowalczyk"), 5, false);
    let b = e.add_minion(SeatId(2), crypt("Elisha Tucker"), 5, false);
    // Added as fixtures, Donny and Elisha do not contest Pittsburgh yet.
    assert!(!e.state.minions[b.index()].title_contested);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(v),
            ability: 0,
            actor: Some(v),
            target: Target::Minion(p),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[v.index()].granted_title,
        Some(Title::Prince("Pittsburgh"))
    );
    assert!(e.state.minions[v.index()].title_contested);
    assert!(e.state.minions[b.index()].title_contested);
    let yield_it = Choice::YieldContest(zvtes::game::Contest::Title(v));
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&yield_it), "{:?}", req.options);
    pick(&mut e, yield_it);
    let m = &e.state.minions[v.index()];
    assert_eq!(m.granted_title, None);
    assert!(m.title_lost && !m.title_contested && m.in_play);
    assert_eq!(e.state.title_of(v), Title::None);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert!(!e.state.minions[b.index()].title_contested);
    assert_eq!(e.state.title_of(b), Title::Baron("Pittsburgh"));
}

// Rebel: "Once each turn, if they block a titled vampire or a political
// action, this vampire gains 1 blood before block resolution." Blocking an
// untitled vampire's bleed gains nothing: the same combat (hand strikes
// only, elided) leaves the titled case's blocker 1 blood ahead.
#[test]
fn rebel_gains_blood_blocking_a_titled_vampire() {
    let after = |titled: bool| {
        let mut e = Engine::bare(3);
        let a = if titled {
            e.add_minion(SeatId(0), prince(0), 5, false)
        } else {
            e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false)
        };
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let z = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 3, false);
        equip(&mut e, "Rebel", z);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick(&mut e, Choice::DeclareBlock { minion: z });
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        blood(&e, z)
    };
    assert_eq!(after(true), after(false) + 1);
}

// Rebel: "… if they block a titled vampire or a political action …" — an
// untitled vampire's political action gains it the blood too.
#[test]
fn rebel_gains_blood_blocking_a_political_action() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("", "aus", 0), 3, false);
    equip(&mut e, "Rebel", z);
    hand(&mut e, 0, &["Anarchist Uprising"]);
    // Intercept against the political action's +1 stealth.
    hand(&mut e, 1, &["Enhanced Senses"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, z)); // Enhanced Senses
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // +1 before block resolution, -1 from the acting Brujah's hand strike.
    assert_eq!(blood(&e, z), 3);
    assert!(e.state.minions[a.index()].political_this_turn);
}

// Ventrue Headquarters: "You can lock this card during the polling step of
// any referendum to get +3 votes." The Methuselah's votes, all one way.
#[test]
fn ventrue_headquarters_locks_for_3_votes() {
    let mut e = Engine::bare(3);
    let p = e.add_minion(SeatId(0), prince(0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    let hq = location(&mut e, 1, "Ventrue Headquarters");
    hand(&mut e, 0, &["Anarchist Uprising"]);
    pick(&mut e, play(0, 0, p));
    pick_when(&mut e, votes(p, true));
    let against = Choice::UseAbility {
        source: AbilitySource::Card(hq),
        ability: 1,
        actor: None,
        target: Target::None,
    };
    pick_for(&mut e, SeatId(1), against);
    assert!(e.state.cards[hq.index()].locked);
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // 2 for (the prince) against 3: failed, no pool burned.
    assert_eq!((pool(&e, 0), pool(&e, 1)), (30, 30));
}

// Elysium: The Palace of Versailles: "lock this location during the polling
// step of a political action to give each titled Camarilla vampire you
// control +1 vote"; Power Structure: "… each Lasombra you control +1
// vote". A vampire that has cast adds the vote to its cast; one casting
// later has it.
#[test]
fn vote_locations_give_each_vampire_they_name_a_vote() {
    let mut e = Engine::bare(3);
    let p = e.add_minion(SeatId(0), prince(0), 5, false);
    let q = e.add_minion(SeatId(0), prince(1), 5, false);
    let l = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Lasombra" && c.title == Title::None, 0),
        5,
        false,
    );
    e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    let elysium = location(&mut e, 0, "Elysium: The Palace of Versailles");
    let ps = location(&mut e, 0, "Power Structure");
    hand(&mut e, 0, &["Anarchist Uprising"]);
    pick(&mut e, play(0, 0, p));
    pick_when(&mut e, votes(p, true));
    pick_for(&mut e, SeatId(0), ability(elysium));
    pick_for(&mut e, SeatId(0), ability(ps));
    let req = until(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert!(req.options.contains(&votes(l, true)), "the Lasombra's vote");
    pick(&mut e, votes(q, true));
    let rec = |m: MinionId| {
        e.referendum()
            .expect("the referendum")
            .records
            .iter()
            .find(|r| r.source == zvtes::referendum::Source::Vampire(m))
            .map(|r| r.n)
    };
    assert_eq!(rec(p), Some(3), "cast before: joins the cast");
    assert_eq!(rec(q), Some(3), "cast after: has it");
}

// ---- Combat staples (slice 11f) ----------------------------------------------

/// The `nth` untitled vampire (no printed-text definition) printing all of
/// `tags` exactly.
fn with(tags: &'static [&'static str], nth: usize) -> u16 {
    vampire(move |c| has(c, tags) && c.title == Title::None, nth)
}

/// The strike of the weapon `gun` in play.
fn weapon_strike(gun: CardId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(gun),
        ability: 0,
        actor: None,
        target: Target::None,
    }
}

/// Seat 0's `a` bleeds, seat 1's `b` blocks: into the combat.
fn bleed_blocked(e: &mut Engine, a: MinionId, b: MinionId) {
    pick(e, action(a, ActionKind::Bleed));
    pick(e, Choice::DeclareBlock { minion: b });
}

// Blood Rage [tha]: "Strike: hand strike. Damage from this strike cannot be
// prevented by cards requiring Fortitude [for]. The opposing vampire's
// strikes with weapons inflict no damage this round." [LSJ 20050628]: "The
// opponent cannot play prevention cards of that discipline (eg. for
// cycling)." The gun strike resolving with it deals nothing. [TOM
// 19960225] [THA]: "Additional damage inherits all of the properties of
// the base damage": the +1 is no more preventable. Blood Fury likewise, at
// +1 and +2 damage, for 1 blood.
#[test]
fn blood_rage_nullifies_the_opposing_weapons_and_fortitude_prevention() {
    type Mode = (&'static str, u8, u8, &'static [&'static str], u8);
    let modes: [Mode; 4] = [
        ("Blood Rage", 0, 0, &["tha"], 1),
        ("Blood Rage", 0, 1, &["THA"], 2),
        ("Blood Fury", 1, 0, &["tha"], 2),
        ("Blood Fury", 1, 1, &["THA"], 3),
    ];
    for (card, cost, mode, tags, damage) in modes {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(tags, 0), 3, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let b = e.add_minion(SeatId(1), with(&["for"], 0), 5, false);
        let gun = equip(&mut e, ".44 Magnum", b);
        hand(&mut e, 0, &[card]);
        hand(&mut e, 1, &["Soak"]);
        bleed_blocked(&mut e, a, b);
        pick_when(&mut e, play(0, mode, a));
        pick_when(&mut e, weapon_strike(gun));
        never_offered(&mut e, &play(0, 0, b));
        assert_eq!(
            blood(&e, a),
            3 - cost,
            "{card}: the gun inflicted no damage"
        );
        assert_eq!(
            blood(&e, b),
            5 - damage,
            "{card}: the hand strike, unprevented"
        );
    }
}

// [LSJ 20040928] Blood Rage, Soul Burn: "If the opponent dodges this
// strike, his weapons are unaffected and do damage this round." A dodge,
// then an additional strike with the gun.
#[test]
fn a_dodged_blood_rage_or_soul_burn_leaves_the_weapons_working() {
    for card in ["Blood Rage", "Soul Burn", "Blood Fury"] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["tha"], 0), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let b = e.add_minion(SeatId(1), with(&["cel"], 0), 5, false);
        let gun = equip(&mut e, ".44 Magnum", b);
        hand(&mut e, 0, &[card]);
        hand(&mut e, 1, &["Side Strike", "Quickness"]);
        bleed_blocked(&mut e, a, b);
        pick_when(&mut e, play(0, 0, a));
        pick_when(&mut e, play(0, 0, b)); // Side Strike: dodge
        pick_when(&mut e, play(0, 0, b)); // Quickness: additional strike
        pick_when(&mut e, weapon_strike(gun));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let cost = if card == "Blood Rage" { 0 } else { 1 };
        assert_eq!(blood(&e, a), 3 - cost, "{card}: the gun's 2 damage");
        assert_eq!(blood(&e, b), 5, "{card}: dodged");
    }
}

// [ANK 20170519] Soul Burn: "Only nullify the weapon damage when it
// resolves" — a ranged strike: at long range too, the gun resolving with
// it deals nothing; the Soul Burn's own 2R lands.
#[test]
fn soul_burn_nullifies_weapon_damage_as_it_resolves() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["THA"], 0), 5, false);
    let start = e.state.capacity_of(a).unwrap().min(5);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let gun = equip(&mut e, ".44 Magnum", b);
    hand(&mut e, 0, &["Soul Burn"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, weapon_strike(gun));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), start - 1, "Soul Burn's own blood cost only");
    assert_eq!(blood(&e, b), 3);
}

// Carrion Crows: "This combat, the opposing minion takes 1R environmental
// damage each round during normal strike resolution." [RTR 19941109] [TOM
// 19951216] [PIB 20140324] [ANK 20200422]: "Damage is inflicted even
// against a dodge. A 'strike: combat ends' ends combat before damage is
// applied." "A vampire can play only one Carrion Crows each combat."
#[test]
fn carrion_crows_damages_through_a_dodge_but_not_a_combat_ends() {
    // Majesty costs 1 blood: the 1 lost is its cost, not the crows'.
    for (strike, want) in [("Side Strike", 4), ("Majesty", 4)] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["ani"], 0), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        // Adelino's text (cheaper rescues) is inert here.
        let b = e.add_minion(SeatId(1), crypt("Adelino Da Silva"), 5, false);
        hand(&mut e, 0, &["Carrion Crows", "Carrion Crows"]);
        hand(&mut e, 1, &[strike]);
        bleed_blocked(&mut e, a, b);
        pick_when(&mut e, play(0, 0, a));
        let req = request(&mut e);
        assert!(
            !req.options.contains(&play(0, 0, a)),
            "one Carrion Crows each combat"
        );
        pick_when(&mut e, play(0, 0, b));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, b), want, "{strike}");
    }
}

// Hunger of Marduk [tha]: "This round, this vampire can strike, ranged:
// steal 1 blood or life (becoming blood)." [ANK 20220207]: "The provided
// strike can be chosen and used more than once during the round."
#[test]
fn hunger_of_marduk_grants_a_steal_usable_again_in_the_round() {
    let mut e = Engine::bare(3);
    // Hunger of Marduk requires a Banu Haqim.
    let a = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Banu Haqim" && has(c, &["cel", "tha"]), 0),
        2,
        false,
    );
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    hand(&mut e, 0, &["Hunger of Marduk", "Quickness"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::RoundStrike { at: None });
    pick_when(&mut e, play(0, 0, a)); // Quickness
    pick_when(&mut e, Choice::RoundStrike { at: None });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // 2 stolen; the Gangrel's hand strike (1) in the first pair.
    assert_eq!(blood(&e, b), 3);
    assert_eq!(blood(&e, a), 3);
}

// ---- Archetypes (slice 11g) --------------------------------------------------

/// Use ability `ability` of card `source` for its host, no target.
fn card_ability(source: CardId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(source),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// The granted action `ability` of card `source`, by `actor` at `target`.
fn granted(source: CardId, ability: u8, actor: MinionId, target: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(source),
        ability,
        actor: Some(actor),
        target: Target::Minion(target),
    }
}

// Perfectionist: "Once each turn, this vampire can gain 1 blood after
// performing a successful action during which no reaction cards were
// played." [ANK 20210309-3]: "The vampire's controller chooses to use the
// effect of the card or not." [ANK 20190113]: "Cards and effects that can
// be played 'after resolution' can be played after the gain blood effect"
// — Alicia Cortez's own after-resolution ability is still offered after
// it. [ANK 20180719-4]: "'Once each turn' means once during each player's
// turn."
#[test]
fn perfectionist_gains_blood_once_each_turn_after_a_successful_action() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), crypt("Alicia Cortez"), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let y = e.add_minion(SeatId(1), vampire(|c| c.capacity <= 5, 0), 3, false);
    let perf = equip(&mut e, "Perfectionist", a);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(req.options.contains(&Choice::Pass), "optional");
    pick(&mut e, card_ability(perf, 0));
    assert_eq!(blood(&e, a), 5);
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(
        req.options.contains(&Choice::UseAbility {
            source: AbilitySource::Vampire(a),
            ability: 0,
            actor: None,
            target: Target::Minion(y),
        }),
        "after-resolution effects follow the gain: {:?}",
        req.options
    );
    // Once each turn: a second successful action this turn gains nothing.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[a.index()].locked = false;
    pick(&mut e, action(a, ActionKind::Hunt));
    never_offered(&mut e, &card_ability(perf, 0));
}

// [LSJ 20070330] Perfectionist: "No blood can be gained if a reaction card
// is played and canceled as it is played: it has still been played."
// Deflection canceled by a third party's Direct Intervention.
#[test]
fn perfectionist_gains_nothing_after_a_canceled_reaction() {
    let mut e = Engine::bare(3);
    let p = e.add_minion(SeatId(0), plain("Brujah", "", 0), 3, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("", "dom", 0), 5, false);
    e.add_minion(SeatId(2), plain("Toreador", "", 0), 5, false);
    let perf = equip(&mut e, "Perfectionist", p);
    hand(&mut e, 1, &["Deflection"]);
    hand(&mut e, 2, &["Direct Intervention"]);
    pick(&mut e, action(p, ActionKind::Bleed));
    pick_when(&mut e, play_at(0, 0, Some(z), Target::Seat(SeatId(2))));
    pick_for(&mut e, SeatId(2), master(0));
    never_offered(&mut e, &card_ability(perf, 0));
    assert_eq!(pool(&e, 1), 29, "the bleed went through, unredirected");
    assert_eq!(blood(&e, p), 3);
}

// [LSJ 20070411] Perfectionist: "The blood can be gained if the action is
// unblocked even if it has no effect" — a hunt at full capacity.
#[test]
fn perfectionist_gains_blood_after_an_action_with_no_effect() {
    let mut e = Engine::bare(3);
    let card = plain("Brujah", "", 0);
    let cap = CRYPT[card as usize].capacity;
    let p = e.add_minion(SeatId(0), card, cap - 1, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let perf = equip(&mut e, "Perfectionist", p);
    pick(&mut e, action(p, ActionKind::Hunt)); // to capacity
    pick_when(&mut e, card_ability(perf, 0));
    assert_eq!(blood(&e, p), cap, "gained, but capped");
    let _ = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
}

// Saulot's Avenging Fist: "This Salubri gets +1 strength. This Salubri can
// enter combat with a non-Salubri vampire as a Ⓓ action; if that vampire is
// Tremere, this is a +1 stealth action."
#[test]
fn saulots_avenging_fist_enters_combat_with_a_non_salubri_vampire() {
    let mut e = Engine::bare(3);
    let s = e.add_minion(SeatId(0), crypt("Castellan"), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", "", 1), 5, false);
    let t = e.add_minion(SeatId(1), plain("Tremere", "", 0), 5, false);
    let x = e.add_minion(SeatId(1), crypt("Malachi"), 5, false);
    let fist = equip(&mut e, "Saulot's Avenging Fist", s);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let offered: Vec<Choice> = req
        .options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::UseAbility { .. }))
        .collect();
    assert_eq!(
        offered,
        vec![granted(fist, 0, s, v), granted(fist, 1, s, t)]
    );
    assert!(!offered.contains(&granted(fist, 0, s, x)), "not a Salubri");
    pick(&mut e, granted(fist, 0, s, v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 3, "a hand strike at strength 2");
}

// Saulot's Healing Touch: "Rescuing a non-Tremere vampire from torpor costs
// this Salubri -2 blood, and if the action is successful, the rescued
// vampire gains 1 blood." [PIB 20110918] [LSJ 20020304-2]: "The discount
// applies to the action, regardless of who pays." A Tremere pays full.
#[test]
fn saulots_healing_touch_rescues_a_non_tremere_for_free() {
    for (clan, free) in [("Ventrue", true), ("Tremere", false)] {
        let mut e = Engine::bare(3);
        let s = e.add_minion(SeatId(0), crypt("Castellan"), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
        // 1 blood between them: only the discount makes it payable.
        let t = e.add_minion(SeatId(0), plain(clan, "", 0), 0, true);
        e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
        equip(&mut e, "Saulot's Healing Touch", s);
        let rescue = Choice::TakeAction {
            minion: s,
            action: ActionKind::Rescue,
            target: Some(t),
        };
        let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(req.options.contains(&rescue), free, "{clan}: payable?");
        if free {
            pick(&mut e, rescue);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(!e.state.minions[t.index()].in_torpor);
            assert_eq!((blood(&e, s), blood(&e, t)), (1, 1), "free, +1 blood");
        }
    }
}

// Saulot's Healing Touch: "This Salubri can add 1 blood or life to another
// ready minion, not to exceed starting life, as a +1 stealth action."
// Directed at another Methuselah's minion's controller, though printed
// without Ⓓ: undirected only at one's own (owner, 2026-09-25: rules-qa
// 20.1).
#[test]
fn saulots_healing_touch_adds_blood_to_another_ready_minion() {
    let mut e = Engine::bare(3);
    let s = e.add_minion(SeatId(0), crypt("Castellan"), 5, false);
    let o = e.add_minion(SeatId(0), plain("Brujah", "", 0), 2, false);
    let y = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 2, false);
    let touch = equip(&mut e, "Saulot's Healing Touch", s);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !req.options.contains(&granted(touch, 0, s, s)),
        "another minion"
    );
    assert!(req.options.contains(&granted(touch, 0, s, y)));
    pick(&mut e, granted(touch, 0, s, o));
    let a = e.action().expect("the action");
    assert_eq!(a.directed_seat(&e.state), None, "at one's own minion");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, o), 3);
    assert!(e.state.minions[s.index()].locked);
    // At another Methuselah's minion: directed at them.
    e.state.minions[s.index()].locked = false;
    e.state.minions[s.index()].acted.clear();
    pick(&mut e, granted(touch, 0, s, y));
    let a = e.action().expect("the action");
    assert_eq!(a.directed_seat(&e.state), Some(SeatId(1)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, y), 3);
}

// ---- Amici Noctis (slice 11h) ------------------------------------------------

/// The `nth` untitled Lasombra with no printed-text definition.
fn lasombra(nth: usize) -> u16 {
    vampire(|c| c.clan == "Lasombra" && c.title == Title::None, nth)
}

// [PIB 20150915] [LSJ 19981028] [ANK 20190425] Amici Noctis "is played
// after resolution": "For each vote by which the referendum passed,
// distribute 1 blood from the blood bank among the ready Lasombra you
// control and your pool. A vampire cannot gain more than 1 blood this way.
// You cannot gain more than 1 pool this way or more than 2 pool this way if
// this acting Lasombra is titled." A prince's 2 votes and the card's: 3.
#[test]
fn amici_noctis_distributes_the_margin_between_pool_and_ready_lasombra() {
    let mut e = Engine::bare(3);
    let p = e.add_minion(
        SeatId(0),
        vampire(
            |c| c.clan == "Lasombra" && matches!(c.title, Title::Prince(_)),
            0,
        ),
        3,
        false,
    );
    let l1 = e.add_minion(SeatId(0), lasombra(0), 1, false);
    let l2 = e.add_minion(SeatId(0), lasombra(1), 1, false);
    e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    e.add_minion(SeatId(2), plain("Gangrel", "", 0), 5, false);
    hand(&mut e, 0, &["Anarchist Uprising", "Amici Noctis"]);
    pick(&mut e, play(0, 0, p));
    pick_when(&mut e, votes(p, true));
    pick(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    let before = pool(&e, 0);
    pick(&mut e, play(0, 0, p));
    // Titled: up to 2 to the pool; the three Lasombra can take the rest.
    let req = expect(&mut e, SeatId(0), WindowLabel::Distribution);
    let to_pool = |n| Choice::AllocateTo { seat: SeatId(0), n };
    assert_eq!(req.options, vec![to_pool(0), to_pool(1), to_pool(2)]);
    pick(&mut e, to_pool(2));
    // One blood left: the prince first, skipped; then the first Lasombra.
    let req = expect(&mut e, SeatId(0), WindowLabel::Distribution);
    assert_eq!(req.options, vec![Choice::ChooseMinion(p), Choice::Pass]);
    pick(&mut e, Choice::Pass);
    pick(&mut e, Choice::ChooseMinion(l1));
    request(&mut e);
    assert_eq!(pool(&e, 0), before + 2);
    assert_eq!((blood(&e, p), blood(&e, l1), blood(&e, l2)), (3, 2, 1));
}

// [RTR 19951110] Amici Noctis "is played after the referendum is resolved.
// If the effect of the referendum ousts the acting Methuselah, there is no
// time to play it to prevent the ousting."
#[test]
fn amici_noctis_has_no_window_when_the_referendum_ousts_the_caller() {
    let mut e = Engine::bare(3);
    let p = e.add_minion(
        SeatId(0),
        vampire(
            |c| c.clan == "Lasombra" && matches!(c.title, Title::Prince(_)),
            0,
        ),
        3,
        false,
    );
    e.add_minion(SeatId(0), lasombra(0), 1, false);
    e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    e.add_minion(SeatId(2), plain("Gangrel", "", 0), 5, false);
    e.state.seats[0].pool = 2;
    hand(&mut e, 0, &["Ancilla Empowerment", "Amici Noctis"]);
    pick(&mut e, play(0, 0, p));
    pick_when(&mut e, votes(p, true));
    close_polling(&mut e);
    let req = request(&mut e);
    assert!(e.state.seats[0].ousted);
    assert_ne!(req.seat, SeatId(0));
}

// ---- Combat cards (slice 11i) ------------------------------------------------

/// The cards in play on minion `m`, by LIBRARY index, with their counters.
fn on(e: &Engine, m: MinionId) -> Vec<(u16, u8)> {
    e.state
        .cards
        .iter()
        .filter(|c| c.in_play && c.host == Some(m))
        .map(|c| (c.card, c.counters))
        .collect()
}

// Bear's Skin [ani][pro]: "This round, this vampire gets +1 strength and can
// prevent 1 damage." "A vampire can play only one Bear's Skin each
// combat."
#[test]
fn bears_skin_gives_strength_and_a_prevention_this_round() {
    let mut e = Engine::bare(3);
    // A combo card: Animalism and Protean both.
    let a = e.add_minion(SeatId(0), with(&["ani", "pro"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    hand(&mut e, 0, &["Bear's Skin", "Bear's Skin"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)), "one each combat");
    pick_when(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 5, "prevented");
    assert_eq!(blood(&e, b), 3, "strength 2");
}

// [LSJ 20030902-2] [LSJ 20060808-1] Dust Up [ani]: "This strike cannot be
// dodged" — "Does not prevent the opponent from dodging, the dodge just has
// no effect." [TOM 19960225]: the +1 is part of the hand strike.
#[test]
fn dust_up_strikes_through_a_dodge() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(
        SeatId(0),
        vampire(
            |c| c.sect == zvtes::crypt::Sect::Anarch && has(c, &["ani"]),
            0,
        ),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), with(&["cel"], 0), 5, false);
    hand(&mut e, 0, &["Dust Up"]);
    hand(&mut e, 1, &["Side Strike"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, b)); // the dodge, offered
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 3, "2 damage through the dodge");
    assert_eq!(blood(&e, a), 5, "the dodge was its strike");
}

// [ANK 20220204] Dust Up [cel]: "The additional strike is not optional: you
// cannot play it for the dodge only if you already played it for a
// (limited) additional strike this round." After Quickness, no [cel] Dust
// Up in the additional pair; the [pot] strike is still offered. [LSJ
// 20011204-3] [PIB 20130704]: "When played, a multi-Disciplines card counts
// as requiring the Discipline(s) being used" — each mode by its own.
#[test]
fn dust_up_dodge_is_not_playable_once_the_limited_strike_is_used() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(
        SeatId(0),
        vampire(
            |c| c.sect == zvtes::crypt::Sect::Anarch && has(c, &["cel"]) && has(c, &["pot"]),
            0,
        ),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    hand(&mut e, 0, &["Quickness", "Dust Up"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, Choice::HandStrike);
    pick_when(&mut e, play(0, 0, a)); // Quickness: the limited strike
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(
        !req.options.contains(&play(0, 1, a)),
        "the dodge's strike is limited"
    );
    assert!(req.options.contains(&play(0, 2, a)));
}

// Form of the Wolf [PRO]: "This vampire gets +1 strength this combat …
// and gains 1 optional press this combat." [TOM 19960521]: "The optional
// press can only be used during the current round."
#[test]
fn form_of_the_wolf_gives_strength_and_a_press_this_round() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRO"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    hand(&mut e, 0, &["Form of the Wolf"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::UsePress);
    // A second round at strength 2 again, with no press left.
    never_offered(&mut e, &Choice::UsePress);
    assert_eq!(blood(&e, b), 1, "two rounds at strength 2");
}

// Rolling with the Punches [FOR]: "Burn 1 blood to prevent all damage from
// the opposing minion's strikes this round." [ANK 20210226]: "The card
// cannot be played if the minion cannot afford to burn the blood." [LSJ
// 20001114]: "Cannot be used if there is no damage to prevent" — not for
// environmental damage alone.
#[test]
fn rolling_with_the_punches_prevents_the_strikes_for_1_blood() {
    // The strike's damage, prevented for 1 blood.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), with(&["FOR"], 0), 4, false);
    hand(&mut e, 1, &["Rolling with the Punches"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 1, b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 3, "the blood burned, no damage");
    // No blood to burn: not playable.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), with(&["FOR"], 0), 0, false);
    hand(&mut e, 1, &["Rolling with the Punches"]);
    bleed_blocked(&mut e, a, b);
    never_offered(&mut e, &play(0, 1, b));
    // Environmental damage only: nothing from the strikes to prevent.
    let mut e = Engine::bare(3);
    // Animalism and Celerity: Keegan's text (unlock after his own Gangrel
    // action) is inert here.
    let a = e.add_minion(SeatId(0), crypt("Keegan"), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), with(&["FOR"], 0), 4, false);
    hand(&mut e, 0, &["Carrion Crows", "Side Strike"]);
    hand(&mut e, 1, &["Rolling with the Punches"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 0, a)); // Carrion Crows
    pick_when(&mut e, play(0, 0, a)); // Side Strike: no strike damage
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert!(req.options.contains(&play(0, 0, b)));
    assert!(!req.options.contains(&play(0, 1, b)));
}

// Sculpt the Flesh [pro]: "Strike: hand strike, aggravated": one point of
// aggravated damage sends a vampire to torpor.
#[test]
fn sculpt_the_flesh_hand_strike_is_aggravated() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Tzimisce" && has(c, &["pro"]), 0),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    hand(&mut e, 0, &["Sculpt the Flesh"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[b.index()].in_torpor);
}

// Sculpt the Flesh [PRO]: "Strike: hand strike at +1 damage and put this
// card on the opposing minion. During their unlock phase, the attached
// minion burns 1 blood or life. Minions can burn this card as a +1 stealth
// action." Dodged, the card is discarded.
#[test]
fn sculpt_the_flesh_superior_stays_on_the_opposing_minion() {
    let sculpt = find_library("Sculpt the Flesh");
    for dodge in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(
            SeatId(0),
            vampire(|c| c.clan == "Tzimisce" && has(c, &["PRO"]), 0),
            5,
            false,
        );
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let b = e.add_minion(SeatId(1), with(&["cel"], 0), 5, false);
        e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
        hand(&mut e, 0, &["Sculpt the Flesh"]);
        hand(&mut e, 1, &["Side Strike"]);
        bleed_blocked(&mut e, a, b);
        pick_when(&mut e, play(0, 1, a));
        if dodge {
            pick_when(&mut e, play(0, 0, b));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if dodge {
            assert!(on(&e, b).is_empty());
            assert_eq!(e.state.seats[0].ash, vec![sculpt]);
            continue;
        }
        assert_eq!(on(&e, b), vec![(sculpt, 0)]);
        assert_eq!(blood(&e, b), 3);
        // Seat 1's unlock phase: 1 blood burned.
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, b), 2);
        // Burned as a +1 stealth action.
        let (id, _) = e
            .state
            .cards
            .iter()
            .enumerate()
            .find(|(_, c)| c.in_play && c.card == sculpt)
            .expect("in play");
        let burn = Choice::UseAbility {
            source: AbilitySource::Card(CardId(id as u32)),
            ability: 0,
            actor: Some(b),
            target: Target::None,
        };
        pick(&mut e, burn);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(on(&e, b).is_empty());
    }
}

// Touch of Oblivion [obl]: "Strike: put this card on the opposing minion
// with 2 counters. The attached minion burns 1 counter from this card
// instead of unlocking as normal. If this card has no counters, burn it."
// The card itself goes on the minion, then burns to the ash heap (owner,
// 2026-10-01: rules-qa 47.1).
#[test]
fn touch_of_oblivion_keeps_the_minion_locked_two_unlock_phases() {
    let touch = find_library("Touch of Oblivion");
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["obl"], 0), 5, false);
    let start = e.state.capacity_of(a).unwrap().min(5);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    hand(&mut e, 0, &["Touch of Oblivion"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, b), vec![(touch, 2)]);
    assert!(e.state.seats[0].ash.is_empty(), "on the minion");
    assert!(!e
        .state
        .cards
        .iter()
        .any(|c| c.card == touch && c.effect_only));
    assert_eq!(blood(&e, a), start - 3, "2 blood, and the hand strike");
    for left in [1, 0] {
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(e.state.minions[b.index()].locked, "{left} left");
        let want = if left > 0 {
            vec![(touch, left)]
        } else {
            vec![]
        };
        assert_eq!(on(&e, b), want);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    }
    assert_eq!(e.state.seats[0].ash, vec![touch], "burned to the ash heap");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[b.index()].locked);
}

// [LSJ 20090622-2] Touch of Oblivion [OBL]: "Strike: send the opposing
// vampire to torpor or burn the opposing ally" — the vampire is wounded.
#[test]
fn touch_of_oblivion_superior_sends_the_vampire_to_torpor() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["OBL"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    hand(&mut e, 0, &["Touch of Oblivion"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[b.index()].in_torpor);
    assert_eq!(blood(&e, b), 5, "no damage: sent to torpor");
}

// [RTR 20020501] [LSJ 19980109] Pass Through Shadow [OBL]: "Strike: combat
// ends. After combat ends, put this card on this vampire. This vampire can
// burn this card to get +1 stealth." The card itself goes on the vampire
// after combat, from the ash heap it went to as it resolved, and burns back
// there (owner, 2026-09-30: rules-qa 46.4, revising 21.1 for this card).
#[test]
fn pass_through_shadow_goes_on_the_vampire_after_combat() {
    let shadow = find_library("Pass Through Shadow");
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["OBL"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), with(&["aus"], 0), 5, false);
    hand(&mut e, 0, &["Pass Through Shadow"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, a), vec![(shadow, 0)]);
    // The card itself is on the vampire, no longer in the ash heap.
    assert!(e.state.seats[0].ash.is_empty());
    assert!(!e
        .state
        .cards
        .iter()
        .any(|c| c.card == shadow && c.effect_only));
    assert_eq!(blood(&e, b), 5, "combat ended");
    // Acting again, under a block attempt: the card burns for +1 stealth,
    // and the attempt fails.
    e.state.minions[a.index()].locked = false;
    e.state.minions[a.index()].acted.clear();
    e.state.minions[a.index()].bled_this_turn = false;
    e.state.minions[b.index()].locked = false;
    let id = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == shadow)
        .expect("in play");
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: b });
    pick_when(&mut e, card_ability(CardId(id as u32), 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(on(&e, a).is_empty(), "burned");
    assert_eq!(e.state.seats[0].ash, vec![shadow], "burned to the ash heap");
    assert_eq!(pool(&e, 1), 29, "the bleed went through");
}

// [RTR 20020501] [LSJ 19980109] Pass Through Shadow [OBL]: "If the combat
// continues or a new combat begins, the after combat effect does not
// happen" — Telepathic Tracking [AUS] starts a new round instead: the card
// is discarded.
#[test]
fn pass_through_shadow_is_lost_if_the_combat_continues() {
    let shadow = find_library("Pass Through Shadow");
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["OBL"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), with(&["AUS"], 0), 5, false);
    hand(&mut e, 0, &["Pass Through Shadow"]);
    hand(&mut e, 1, &["Telepathic Tracking"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, b)); // a new round instead
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(on(&e, a).is_empty());
    assert!(e.state.seats[0].ash.contains(&shadow));
}

// Touch of Oblivion's strikes are not ranged: at long range they do
// nothing, and the [obl] card is discarded (owner, 2026-09-26: a strike
// works at close range unless it says so; rules-qa 21.1).
#[test]
fn touch_of_oblivion_does_nothing_at_long_range() {
    let touch = find_library("Touch of Oblivion");
    for mode in [0u8, 1] {
        let mut e = Engine::bare(3);
        let tag: &'static [&'static str] = if mode == 0 { &["obl"] } else { &["OBL"] };
        let a = e.add_minion(SeatId(0), with(tag, 0), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
        let gun = equip(&mut e, ".44 Magnum", b);
        hand(&mut e, 0, &["Touch of Oblivion"]);
        bleed_blocked(&mut e, a, b);
        pick_when(&mut e, card_ability(gun, 1)); // the gun's maneuver: long
        pick_when(&mut e, play(0, mode, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(on(&e, b).iter().all(|&(c, _)| c != touch), "mode {mode}");
        assert!(!e.state.minions[b.index()].in_torpor, "mode {mode}");
        assert!(e.state.seats[0].ash.contains(&touch));
    }
}

// ---- Combat cards with their own mechanisms (slice 11j) --------------------

// Slam [POT]: "Strike: hand strike at +2 damage, with 1 optional maneuver,
// only usable to get to close range." Played at the maneuver once the gun
// has taken the range long, it closes it and commits the strike. [TOM
// 19960225]: the +2 is the hand strike's own damage.
#[test]
fn slam_played_at_the_maneuver_closes_the_range_and_commits_the_strike() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["POT"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    let gun = equip(&mut e, ".44 Magnum", b);
    hand(&mut e, 0, &["Slam"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, card_ability(gun, 1)); // the gun's maneuver: long
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::CombatDetermineRange)
    );
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 2, "a hand strike at +2, at close range");
    assert_eq!(blood(&e, a), 2, "Slam's blood, and the gun's 2");
}

// Slam [POT] only at the maneuver (rules-qa 48.4): at the strike, [pot]
// alone — the same strike without paying for an unusable maneuver.
#[test]
fn slam_at_the_strike_is_offered_at_inferior_only() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["POT"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    hand(&mut e, 0, &["Slam"]);
    bleed_blocked(&mut e, a, b);
    let req = offering(&mut e, Choice::HandStrike);
    assert_eq!(req.window, WindowLabel::CombatStrike);
    assert!(req.options.contains(&play(0, 0, a)), "{req:?}");
    assert!(!req.options.contains(&play(0, 1, a)), "{req:?}");
}

// [LSJ 20021028] Slam, Aid from Bats: "The optional maneuver cannot be used
// if the strike cannot be used" — no strike in the first round (One With the
// Land [PRO]): no Aid from Bats at the maneuver.
#[test]
fn a_strike_with_a_maneuver_is_not_played_while_strikes_are_barred() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ani"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Tzimisce", "PRO", 0), 5, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Aid from Bats"]);
    hand(&mut e, 1, &["One With the Land"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, z));
    never_offered(&mut e, &play(0, 0, a));
}

// Aid from Bats [ANI]: "Strike: 1R damage, with 1 optional maneuver … with
// 1 optional press." At the maneuver it takes the range long and commits
// the strike; [TOM 19960521] the press is usable this round only.
#[test]
fn aid_from_bats_maneuvers_either_way_and_commits_its_strike() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ANI"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let gun = equip(&mut e, ".44 Magnum", b);
    hand(&mut e, 0, &["Aid from Bats"]);
    bleed_blocked(&mut e, a, b);
    // At close range, played at the maneuver: long.
    pick_when(&mut e, play(0, 1, a));
    // The gun maneuvers back to close; the committed 1R stays A's strike.
    pick_when(&mut e, card_ability(gun, 1));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(req.options.contains(&Choice::UsePress));
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 4, "the committed 1R");
    assert_eq!(blood(&e, a), 3, "the gun's 2R");
}

// Pack Alpha [ani]: "Employ an animal retainer from your hand (requirements
// and cost apply as normal)." [LSJ 20100130] [ANK 20180925-2] [LSJ
// 19991207]: "The equipment or retainer must be announced by name, and in
// hand, when the card or effect is declared." [LSJ 20100725] [LSJ 20100303]
// [ANK 20180817]: "Requirements apply" — Raven Spy's superior version needs
// [ANI]. [LSJ 20080815]: "This does not count as performing the action to
// employ … this retainer." [LSJ 20090115-1] [ANK 20170309]: "This is not an
// action, so cost reductions referring to an action do not apply."
#[test]
fn pack_alpha_employs_an_animal_retainer_from_hand() {
    let raven = find_library("Raven Spy");
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ani"], 0), 3, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    hand(&mut e, 0, &["Pack Alpha", "Raven Spy"]);
    bleed_blocked(&mut e, a, b);
    until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    pick(&mut e, play(0, 0, a));
    // Only the basic Raven Spy: its [ANI] version needs superior.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, a), vec![(raven, 1)]);
    assert!(e.state.seats[0].hand.is_empty());
    assert_eq!(pool(&e, 0), 30, "no pool");
    assert_eq!(blood(&e, a), 1, "Raven Spy's blood, and the hand strike");
    assert!(
        !e.state.minions[a.index()]
            .acted
            .contains(&zvtes::state::ActionId::Card(raven)),
        "not an employ action"
    );
}

// Pack Alpha [ANI]: "Burn an animal retainer employed by this vampire to
// put this card on this vampire. This minion gets +1 strength. A minion can
// have only one Pack Alpha."
#[test]
fn pack_alpha_superior_burns_a_retainer_for_strength() {
    let pack = find_library("Pack Alpha");
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ANI"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    let spy = equip(&mut e, "Raven Spy", a);
    e.state.cards[spy.index()].counters = 1;
    hand(&mut e, 0, &["Pack Alpha", "Pack Alpha"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play_at(0, 1, Some(a), Target::Card(spy)));
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { mode: 1, .. })),
        "one Pack Alpha a round, and a minion has only one"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, a), vec![(pack, 0)]);
    assert_eq!(blood(&e, b), 3, "strength 2");
}

// Voracious Vermin [ANI]: "Strike: dodge … with 1 additional ranged strike:
// burn weapon." With no weapon to burn the superior version cannot be
// chosen; with one, the additional strike burns it (owner, 2026-09-26:
// rules-qa 22.3).
#[test]
fn voracious_vermin_dodges_then_burns_a_weapon() {
    for armed in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["ANI"], 0), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
        hand(&mut e, 0, &["Voracious Vermin"]);
        if !armed {
            bleed_blocked(&mut e, a, b);
            let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
            assert!(req.options.contains(&play(0, 0, a)));
            assert!(!req.options.contains(&play(0, 1, a)), "no weapon to burn");
            continue;
        }
        let gun = equip(&mut e, ".44 Magnum", b);
        bleed_blocked(&mut e, a, b);
        pick_when(&mut e, play(0, 1, a));
        pick_when(&mut e, weapon_strike(gun));
        // The additional strike: the lone weapon, burned.
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.cards[gun.index()].in_play, "burned");
        assert_eq!(blood(&e, a), 5, "the gun was dodged");
    }
}

// Weighted Walking Stick: "Put this card with 5 counters on it on this
// minion; it becomes a melee weapon equipment that can strike: strength+1
// damage. For each damage inflicted by this strike (even if prevented),
// burn 1 counter from this card." [LSJ 20100519]: damage "not inflicted
// (eg. Blood Fury)" burns no counter — Blood Rage and Blood Fury nullify
// the weapon.
#[test]
fn weighted_walking_stick_loses_counters_per_damage_inflicted() {
    let stick = find_library("Weighted Walking Stick");
    for rage in [None, Some("Blood Rage"), Some("Blood Fury")] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 5, false);
        let b = e.add_minion(SeatId(1), with(&["tha"], 0), 3, false);
        hand(&mut e, 0, &["Weighted Walking Stick"]);
        hand(&mut e, 1, &[rage.unwrap_or("Blood Rage")]);
        bleed_blocked(&mut e, a, b);
        pick_when(&mut e, play(0, 0, a));
        request(&mut e);
        let (id, _) = e
            .state
            .cards
            .iter()
            .enumerate()
            .find(|(_, c)| c.in_play && c.card == stick)
            .expect("in play");
        let id = CardId(id as u32);
        assert!(e.state.equipment_on(a).contains(&id), "equipment");
        pick_when(&mut e, weapon_strike(id));
        if rage.is_some() {
            pick_when(&mut e, play(0, 0, b));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let want = if rage.is_some() { 5 } else { 3 };
        assert_eq!(on(&e, a), vec![(stick, want)], "{rage:?}");
    }
}

// [ANK 20201229-2] Weighted Walking Stick: "Full damage is applied if there
// are any counter left, even just 1" — and the card is burned.
#[test]
fn weighted_walking_stick_deals_full_damage_on_its_last_counter() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 5, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    let stick = equip(&mut e, "Weighted Walking Stick", a);
    e.state.cards[stick.index()].counters = 1;
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, weapon_strike(stick));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 3, "strength+1: 2 damage");
    assert!(!e.state.cards[stick.index()].in_play, "burned at 0");
}

// ---- Action modifiers and reactions (slice 11k) ----------------------------

// Cloak the Gathering [OBF]: "Only usable by a ready vampire other than the
// acting minion. The acting minion gets +1 stealth." [LSJ 19990425]: by the
// acting minion's controller; [RTR 19941109] [LSJ 19980210]: a locked
// vampire can play it; [ANK 20200515]: "Multiple copies can be played by
// different minions on the same action."
#[test]
fn cloak_the_gathering_superior_is_played_by_another_vampire() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let o = e.add_minion(SeatId(0), with(&["OBF"], 0), 5, false);
    let o2 = e.add_minion(SeatId(0), with(&["OBF"], 1), 5, false);
    e.state.minions[o.index()].locked = true;
    let z = e.add_minion(SeatId(1), with(&["aus"], 0), 5, false);
    hand(&mut e, 0, &["Cloak the Gathering", "Cloak the Gathering"]);
    hand(&mut e, 1, &["Enhanced Senses"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 1, o));
    pick_when(&mut e, play(0, 0, z)); // +1 intercept: 1 against 1
    pick_when(&mut e, play(0, 1, o2));
    assert!(e.state.seats[0].hand.is_empty());
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 29, "the block failed");
}

// Veil the Legions [OBF]: "this vampire can burn X blood to give the next X
// actions minions you control perform this turn +1 stealth." [LSJ
// 20070526]: counted as the action is announced. "Only one Veil the
// Legions can be played each action." [LSJ 19990425]: "Can only be played
// by the controller of the acting minion."
#[test]
fn veil_the_legions_superior_gives_the_next_actions_stealth() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let o = e.add_minion(SeatId(0), with(&["OBF"], 0), 5, false);
    let c = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    hand(&mut e, 0, &["Veil the Legions", "Veil the Legions"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 1, o));
    pick(&mut e, Choice::SetX(1));
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|x| matches!(x, Choice::Play { card: 0, .. })),
        "one Veil each action"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, o), 4);
    // The next action: +1 stealth, beyond the reach of an attempt with no
    // intercept (pruned).
    e.state.minions[z.index()].locked = false;
    pick(&mut e, action(c, ActionKind::Bleed));
    never_offered(&mut e, &Choice::DeclareBlock { minion: z });
    assert_eq!(pool(&e, 1), 28, "both bleeds went through");
}

// Daring the Dawn [for]: "Vampires cannot block this action. This vampire
// takes 2 unpreventable environmental aggravated damage after action
// resolution." One aggravated point sends it to torpor.
#[test]
fn daring_the_dawn_bars_vampire_blockers_and_burns_after_resolution() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["for"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    hand(&mut e, 0, &["Daring the Dawn"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 0, a));
    never_offered(&mut e, &Choice::DeclareBlock { minion: z });
    assert_eq!(pool(&e, 1), 29);
    assert!(
        e.state.minions[a.index()].in_torpor,
        "wounded by aggravated damage"
    );
}

// Freak Drive [for]: "Only usable if the action was successful. Unlock this
// vampire." [PIB 20150915] [LSJ 19981028] [ANK 20190425]: played after
// resolution, still during the action. [PIB 20150703] The Sleeping Mind [DOM]: "During this action,
// minions cannot unlock" — no unlock after resolution either.
#[test]
fn freak_drive_unlocks_unless_the_sleeping_mind_bars_it() {
    for sleeping in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["for", "DOM"], 0), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
        hand(&mut e, 0, &["Freak Drive", "Sleeping Mind, The"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        if sleeping {
            pick_when(&mut e, play(1, 1, a));
        }
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[a.index()].locked, sleeping);
    }
}

// Seduction [dom]: "Only usable as the action is announced … Choose a
// younger vampire. The chosen vampire cannot block this action." [ANK
// 20190425]: as-announced modifiers come before the others — the
// as-announced window opens first.
#[test]
fn seduction_bars_a_younger_vampire_from_blocking() {
    let mut e = Engine::bare(3);
    // Anja's text (unlocking another Hecata after its action) is inert here.
    let a = e.add_minion(SeatId(0), crypt("\"Mother\" Anja Giovanni"), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    hand(&mut e, 0, &["Seduction"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(z)));
    never_offered(&mut e, &Choice::DeclareBlock { minion: z });
    assert_eq!(pool(&e, 1), 29);
}

// Unleashing the Bestial Soul [DOM]: "Choose a minion. The chosen minion
// cannot play reaction cards this action … and the next reaction card
// played this action costs +1 blood or life."
#[test]
fn unleashing_the_bestial_soul_bars_and_taxes_reactions() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Salubri" && has(c, &["DOM"]), 0),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    // Anja's text (unlocking another Hecata after its action) is inert here.
    let z = e.add_minion(SeatId(1), crypt("\"Mother\" Anja Giovanni"), 5, false);
    let y = e.add_minion(SeatId(1), crypt("Amelia Locke"), 5, false);
    hand(&mut e, 0, &["Unleashing the Bestial Soul"]);
    hand(&mut e, 1, &["Deflection", "Deflection"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play_at(0, 1, Some(a), Target::Minion(z)));
    let deflect = |m| play_at(0, 0, Some(m), Target::Seat(SeatId(2)));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlocksDeclined);
    assert!(!req.options.contains(&deflect(z)), "barred");
    pick(&mut e, deflect(y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, y), 3, "Deflection's 1 blood, +1");
    assert!(e.state.minions[y.index()].locked);
}

// Visions of Zapathasura [OBF][PRE]: "Lock this vampire to reduce a bleed
// against you to 0."
#[test]
fn visions_of_zapathasura_superior_reduces_the_bleed_to_zero() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["pre"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), with(&["OBF", "PRE"], 0), 5, false);
    hand(&mut e, 1, &["Visions of Zapathasura"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 30);
    assert!(e.state.minions[z.index()].locked);
    assert_eq!(blood(&e, z), 4, "1 blood");
}

// Change of Target: "Only usable if this minion is blocked, before block
// resolution. Unlock this minion. The action ends (unsuccessfully)."
// [LSJ 19980224] [RTR 19991206]: played "before the blocker is locked and
// combat begins"; [RTR 19991206]: "Effects triggered when the action is
// blocked are still applied" (Unlicensed Taxicab's burn, blocked by a
// prince); [LSJ 20010803-1]
// [RTR 20180719]: "No other modifier or reaction can be played
// afterwards" — nor Freak Drive [FOR] (RTR 20180719: not if "ended");
// [LSJ 20070709]: it reached resolution, subject to NRA.
#[test]
fn change_of_target_ends_a_blocked_action_before_combat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["FOR"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), prince(0), 5, false);
    let taxi = equip(&mut e, "Unlicensed Taxicab", a);
    hand(&mut e, 0, &["Change of Target", "Freak Drive"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.cards[taxi.index()].in_play,
        "the block's rider applied"
    );
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    assert!(
        !e.state.minions[z.index()].locked,
        "the blocker never locked"
    );
    assert_eq!((blood(&e, a), blood(&e, z)), (5, 5), "no combat");
    assert_eq!(pool(&e, 1), 30);
    assert_eq!(e.state.seats[0].hand, vec![find_library("Freak Drive")]);
    assert!(
        !req.options.contains(&action(a, ActionKind::Bleed)),
        "not the same action again this turn"
    );
}

// [RTR 19950509] [LSJ 20080710] [ANK 20200502] Change of Target: "Actions
// without card (provided by the rulebook) can be repeated if they have
// different targets" — a rescue of another vampire stays open. [LSJ
// 20050105-2]: "If used while leaving torpor, the blocker does not get an
// opportunity for diablerie."
#[test]
fn change_of_target_forbids_the_same_rulebook_action_only() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let t1 = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 2, true);
    let t2 = e.add_minion(SeatId(0), plain("Ventrue", "", 1), 2, true);
    let z = e.add_minion(SeatId(1), with(&["aus"], 0), 5, false);
    hand(&mut e, 0, &["Change of Target"]);
    hand(&mut e, 1, &["Enhanced Senses", "Enhanced Senses"]);
    let rescue = |t| Choice::TakeAction {
        minion: a,
        action: ActionKind::Rescue,
        target: Some(t),
    };
    pick(&mut e, rescue(t1));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, z)); // +1 intercept against +1 stealth
    pick_when(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&rescue(t1)));
    assert!(req.options.contains(&rescue(t2)));
    // Leaving torpor, blocked: Change of Target, and no diablerie.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 3, true);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), with(&["aus"], 0), 5, false);
    hand(&mut e, 0, &["Change of Target"]);
    hand(&mut e, 1, &["Enhanced Senses"]);
    pick(&mut e, action(a, ActionKind::LeaveTorpor));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 0, a));
    never_offered(&mut e, &Choice::Diablerise);
    assert!(e.state.minions[a.index()].in_play);
}

// [RTR 19980623] Daring the Dawn: "If the action is blocked, damage is
// applied after combat" — by an ally, vampires being barred. [ANK
// 20220331] [ANK 20210124] [LSJ 20020927] [LSJ 20021115] [ANK
// 20221011-2]: "The damage is applied if the action reaches resolution,
// even if it ends before combat (eg. Change of Target)". [LSJ
// 20010714-2]: a card forcing an attempt "cannot be used if the vampire
// cannot attempt to block the action" (Second Tradition: Domain).
#[test]
fn daring_the_dawn_burns_after_combat_or_an_ended_action() {
    for ended in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["FOR"], 0), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let ally = e.add_ally(SeatId(1), find_library("47th Street Royals"));
        let p = e.add_minion(SeatId(1), prince(0), 5, false);
        e.state.minions[p.index()].locked = true;
        hand(
            &mut e,
            0,
            &["Daring the Dawn", "Change of Target", "Freak Drive"],
        );
        hand(&mut e, 1, &["Second Tradition: Domain"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick_when(&mut e, play(0, 1, a));
        let req = request(&mut e);
        assert!(!req
            .options
            .iter()
            .any(|o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == p)));
        pick_when(&mut e, Choice::DeclareBlock { minion: ally });
        if ended {
            pick_when(&mut e, play(0, 0, a));
        } else {
            // After the block's combat: the damage among the
            // after-resolution effects, with Freak Drive [FOR].
            let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
            assert!(req.options.contains(&Choice::TakeDamageAfterResolution));
            assert!(req.options.contains(&play(1, 1, a)), "Freak Drive [FOR]");
            pick(&mut e, Choice::TakeDamageAfterResolution);
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[a.index()].in_torpor, "ended: {ended}");
    }
}
// Freak Drive [FOR]: "Only usable if the action was blocked. Unlock this
// vampire." [PIB 20150915] [LSJ 19981028] [ANK 20190425]: after
// resolution, after all combats — a blocked action has its after-resolution
// window after the block's combat (owner, 2026-09-26).
#[test]
fn freak_drive_superior_unlocks_after_the_blocked_actions_combat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["FOR"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    hand(&mut e, 0, &["Freak Drive"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    let plays: Vec<_> = req
        .options
        .iter()
        .filter(|o| matches!(o, Choice::Play { .. }))
        .collect();
    assert_eq!(plays, vec![&play(0, 1, a)], "[FOR] only: blocked");
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked);
    assert_eq!(blood(&e, a), 3, "a hand strike, then Freak Drive's blood");
}

// [PIB 20150915] [LSJ 19981028] [ANK 20190425] Shadow Cloak [OBL]: played
// after resolution, still during the action. "Put this card on this vampire. Minions without
// Auspex [aus] cannot perform actions directed at this vampire. During
// your unlock phase, burn this card."
#[test]
fn shadow_cloak_shields_its_vampire_until_the_next_unlock_phase() {
    let cloak = find_library("Shadow Cloak");
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["OBL"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let seer = e.add_minion(SeatId(1), with(&["aus"], 0), 5, false);
    hand(&mut e, 0, &["Shadow Cloak"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, a), vec![(cloak, 0)]);
    assert!(!e.state.may_direct_at(z, a), "no Auspex");
    assert!(e.state.may_direct_at(seer, a));
    // Seat 0's next unlock phase: burned.
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(on(&e, a), vec![(cloak, 0)]);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(on(&e, a).is_empty());
}

// [LSJ 20010715] Cards usable after action resolution "can be played before
// or after the damage is taken" (Daring the Dawn): the acting Methuselah
// orders the damage among the after-resolution effects, and takes it
// before passing (owner, 2026-09-26: rules-qa 23.3).
#[test]
fn daring_the_dawn_damage_is_ordered_among_after_resolution_effects() {
    for damage_first in [true, false] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["for"], 0), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
        hand(&mut e, 0, &["Daring the Dawn", "Freak Drive"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick_when(&mut e, play(0, 0, a));
        let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
        assert!(
            !req.options.contains(&Choice::Pass),
            "the damage first, or a card"
        );
        assert!(req.options.contains(&Choice::TakeDamageAfterResolution));
        assert!(req.options.contains(&play(0, 0, a)), "Freak Drive");
        if damage_first {
            pick(&mut e, Choice::TakeDamageAfterResolution);
        } else {
            // Freak Drive, then the damage (a lone option, elided).
            pick(&mut e, play(0, 0, a));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(
            e.state.minions[a.index()].in_torpor,
            "damage first: {damage_first}"
        );
    }
}

// ---- Cards remembering a Methuselah (slice 11l) ------------------------------

use zvtes::actions::Outcome;
use zvtes::ids::SeatSet;

mod common;
use common::*;

/// Pass through the other Methuselahs' turns to seat 0's next minion phase.
fn next_own_turn(e: &mut Engine) {
    until(e, SeatId(1), WindowLabel::MinionPhase);
    until(e, SeatId(0), WindowLabel::MinionPhase);
}

/// The copies of `name` in play, and what each remembers.
fn remembering(e: &Engine, name: &str) -> Vec<(CardId, SeatSet)> {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .enumerate()
        .filter(|(_, c)| c.in_play && c.card == card)
        .map(|(i, c)| (CardId(i as u32), c.against))
        .collect()
}

// Spying Mission [OBF]: "Only usable if a bleed would be successful.
// Instead, the bleed burns no pool, is unsuccessful, and this card is put on
// this vampire. The next time this vampire is about to successfully bleed
// the same Methuselah, burn this card and this vampire gets +2 bleed." [LSJ
// 19980105] [LSJ 20110502] played when the bleed is about to be successful,
// after every other modifier and reaction (Deflection offered before, not
// after). [ANK 20200609] several on one minion against the same Methuselah
// burn at once, +2 bleed each. [LSJ 19980105] burned only after every card
// is played: nothing can follow them (no window after the burn).
#[test]
fn spying_mission_superior_replaces_a_bleed_then_boosts_the_next_ones() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "OBF", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), with(&["dom"], 0), 4, false);
    hand(&mut e, 0, &["Spying Mission", "Spying Mission"]);
    hand(&mut e, 1, &["Deflection", "Archon Investigation"]);
    // Turn 1: the redirect comes first, then the bleed would succeed.
    pick(&mut e, action(a, ActionKind::Bleed));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlocksDeclined);
    assert!(req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == z)));
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::ActionWouldBeSuccessful);
    assert!(req.options.contains(&play(0, 1, a)));
    assert!(!req.options.contains(&play(0, 0, a)), "[obf] is stealth");
    pick(&mut e, play(0, 1, a));
    // The bleed is no longer successful: the other copy is not playable.
    never_offered(&mut e, &play(0, 1, a));
    assert_eq!(e.last_outcome, Some(Outcome::Failed), "unsuccessful");
    assert_eq!(pool(&e, 1), 30, "no pool burned");
    assert!(!e.state.seats[0].has_edge);
    assert_eq!(
        remembering(&e, "Spying Mission")
            .iter()
            .map(|&(_, s)| s)
            .collect::<Vec<_>>(),
        vec![SeatSet::single(SeatId(1))]
    );
    // Turn 2: a second one, played before the first could burn.
    next_own_turn(&mut e);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 30);
    assert_eq!(remembering(&e, "Spying Mission").len(), 2);
    // Turn 3: both burn as the bleed resolves: 1 + 2 + 2 — too late for
    // Archon Investigation (a bleed of 4 or more).
    next_own_turn(&mut e);
    pick(&mut e, action(a, ActionKind::Bleed));
    let archon = Choice::Play {
        card: 1,
        mode: 0,
        actor: None,
        target: Target::None,
    };
    never_offered(&mut e, &archon);
    assert_eq!(pool(&e, 1), 25);
    assert!(remembering(&e, "Spying Mission").is_empty());
    assert_eq!(
        e.state.seats[0].ash,
        vec![find_library("Spying Mission"); 2]
    );
    assert!(e.state.seats[0].has_edge);
}

// [PIB 20150612] A bleed reduced to 0 "is no longer successful and Spying
// Mission cannot be used afterwards" — nor does one in play burn, and no
// Edge is taken (rulebook "Bleed": successful for 1 or more).
#[test]
fn spying_mission_needs_a_bleed_of_one_or_more() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "OBF", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let royals = e.add_ally(SeatId(1), find_library("47th Street Royals"));
    let id = e.state.minions[royals.index()].ally_id().unwrap();
    hand(&mut e, 0, &["Spying Mission"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, ability(id)); // reduce by 3: 0
    never_offered(&mut e, &play(0, 1, a));
    assert_eq!(pool(&e, 1), 30);
    assert!(!e.state.seats[0].has_edge, "a bleed of 0 is not successful");
}

// [LSJ 20050607] [LSJ 20050608] Spying Mission [OBF] canceled (Direct
// Intervention): "it is too late for the target to play reactions" — the
// bleed resolves. "If the cost of the cancellation ousts a player, handle
// the oust before resolving the bleed": the prey ousted, the bleed has no
// one to burn pool; another player ousted, it burns the prey's.
#[test]
fn spying_mission_canceled_lets_the_bleed_resolve_after_the_oust() {
    for canceller in [1u8, 2] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "OBF", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
        e.add_minion(SeatId(2), plain("Ventrue", "", 1), 4, false);
        e.state.seats[canceller as usize].pool = 1;
        hand(&mut e, 0, &["Spying Mission"]);
        hand(&mut e, canceller, &["Direct Intervention", "Deflection"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick_when(&mut e, play(0, 1, a));
        let di = Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::None,
        };
        pick_when(&mut e, di);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.seats[canceller as usize].ousted);
        assert_eq!(e.state.seats[0].ash, vec![find_library("Spying Mission")]);
        assert!(remembering(&e, "Spying Mission").is_empty());
        if canceller == 1 {
            assert_eq!(e.last_outcome, Some(Outcome::Fizzled));
            assert!(!e.state.seats[0].has_edge);
        } else {
            assert_eq!(e.last_outcome, Some(Outcome::Successful));
            assert_eq!(pool(&e, 1), 30 + 6 - 1, "the oust pays its predator first");
            assert!(e.state.seats[0].has_edge);
        }
    }
}

// Shadow Cast [OBL]: "Only usable after resolution of a successful directed
// action. Put this card on this vampire. During an action directed at the
// same Methuselah or same set of Methuselahs, this vampire can burn this
// card to get +1 stealth." [PIB 20150915] [LSJ 19981028] [ANK 20190425]
// played after resolution, still during the action.
#[test]
fn shadow_cast_superior_gives_stealth_against_the_same_methuselah() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "OBL", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Shadow Cast"]);
    // Turn 1: a successful bleed, then [OBL].
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let cast = remembering(&e, "Shadow Cast");
    assert_eq!(cast.len(), 1);
    let (id, against) = cast[0];
    assert_eq!(against, SeatSet::single(SeatId(1)));
    assert_eq!(on(&e, a), vec![(find_library("Shadow Cast"), 0)]);
    // Turn 2: an undirected hunt: not usable.
    next_own_turn(&mut e);
    pick(&mut e, action(a, ActionKind::Hunt));
    never_offered(&mut e, &ability(id));
    assert_eq!(remembering(&e, "Shadow Cast").len(), 1);
    // Turn 3: a bleed of the same Methuselah: burned for +1 stealth, the
    // block attempt fails.
    next_own_turn(&mut e);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, ability(id));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(remembering(&e, "Shadow Cast").is_empty());
    assert_eq!(e.state.seats[0].ash, vec![find_library("Shadow Cast")]);
    assert_eq!(pool(&e, 1), 28, "the second bleed went through");
}

// Shadow Cast [OBL] is not usable after an unsuccessful or undirected
// action: a blocked bleed, or a hunt.
#[test]
fn shadow_cast_needs_a_successful_directed_action() {
    for hunt in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "OBL", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
        hand(&mut e, 0, &["Shadow Cast"]);
        if hunt {
            pick(&mut e, action(a, ActionKind::Hunt));
        } else {
            pick(&mut e, action(a, ActionKind::Bleed));
            pick_when(&mut e, Choice::DeclareBlock { minion: z });
        }
        never_offered(&mut e, &play(0, 1, a));
    }
}

// Mirror Walk [THA] ends the action only "if this action is blocked": an
// unblocked one keeps its after-resolution window's cards (Freak Drive
// [for]); a blocked one takes none (RTR 20180719).
#[test]
fn mirror_walk_superior_leaves_an_unblocked_actions_after_resolution_open() {
    let mut e = Engine::bare(3);
    // Tommaso's text (ending a wraith's or zombie's combat) is inert here.
    let a = e.add_minion(SeatId(0), crypt("Tommaso Sforza"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Mirror Walk", "Freak Drive"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 1, a)); // [THA]: +1 stealth, the attempt fails
    until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    pick(&mut e, play(0, 0, a)); // Freak Drive [for]
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 29);
    assert!(!e.state.minions[a.index()].locked);
}

// [LSJ 20100527] Shadow Cast [OBL]: "If the set of Methuselahs includes the
// controller of the vampire this card is on, the controller of the vampire
// is not considered as part of that set." Owner (2026-09-26, rules-qa
// 24.5): usable when the recorded set is included in the set the current
// action is directed at.
#[test]
fn shadow_cast_does_not_count_its_vampires_controller_in_the_set() {
    for (recorded, usable) in [(0b011u8, true), (0b110, false)] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "OBL", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
        e.state.cards.push(CardInPlay {
            by: Some(a),
            against: SeatSet(recorded),
            ..CardInPlay::new(find_library("Shadow Cast"), SeatId(0), Some(a), 1)
        });
        let id = CardId(e.state.cards.len() as u32 - 1);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick_when(&mut e, Choice::DeclareBlock { minion: z });
        if usable {
            pick_when(&mut e, ability(id));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(pool(&e, 1), 29, "stealth past the block");
        } else {
            // Seat 2 is not directed at: {1, 2} is not included in {1}.
            never_offered(&mut e, &ability(id));
        }
    }
}

// ---- Corruption and multi-Discipline bleeds (slice 11m) ---------------------

/// Has each Discipline, at either level.
fn has_level(c: &zvtes::crypt::CryptCard, tags: &[&str]) -> bool {
    tags.iter()
        .all(|t| c.disciplines.contains(t) || c.disciplines.contains(&t.to_uppercase().as_str()))
}

/// The `nth` untitled Anarch with these Disciplines (either level) and
/// none of `without`.
fn anarch_with(tags: &[&str], without: &[&str], nth: usize) -> u16 {
    vampire(
        |c| {
            c.sect == zvtes::crypt::Sect::Anarch
                && c.title == Title::None
                && has_level(c, tags)
                && !without.iter().any(|w| has_level(c, &[w]))
        },
        nth,
    )
}

fn corruption(e: &Engine, m: MinionId, seat: u8) -> u8 {
    e.state.minions[m.index()].corruption_of(SeatId(seat))
}

// The Platinum Protocol: "Requires an Anarch. More than one Discipline can
// be used to play this card. Ⓓ Bleed. [obf] This is a +1 stealth action.
// [pre] The bleed is with +1 bleed. [pro] Put 1 corruption counter on a
// minion controlled by the target Methuselah if the bleed is successful
// (for 1 or more)." [LSJ 20011204-3] [PIB 20130704] played, it requires the
// Disciplines used: any set of those the vampire has. [ANK 20240706] [LSJ
// 20080608] the minion is chosen at resolution.
#[test]
fn platinum_protocol_uses_any_set_of_its_disciplines_and_corrupts() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(
        SeatId(0),
        anarch_with(&["pre", "pro"], &["obf"], 0),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", "", 1), 5, true);
    hand(&mut e, 0, &["Platinum Protocol, The"]);
    let req = request(&mut e);
    let modes: Vec<u8> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::Play {
                mode,
                actor: Some(m),
                ..
            } if *m == a => Some(*mode),
            _ => None,
        })
        .collect();
    // [pre], [pro], [pre][pro]: never a set with [obf].
    assert_eq!(modes, vec![1, 2, 5]);
    pick(&mut e, play(0, 5, a));
    // Resolved for 2: a minion of the bled Methuselah, chosen now — the one
    // in torpor too.
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::ChooseMinion(z), Choice::ChooseMinion(y)]
    );
    pick(&mut e, Choice::ChooseMinion(y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 28);
    assert_eq!((corruption(&e, z, 0), corruption(&e, y, 0)), (0, 1));
}

// The Platinum Protocol "requires an Anarch"; a non-Anarch cannot play it.
// A prey ousted by the bleed has no minion left for the counter.
#[test]
fn platinum_protocol_needs_an_anarch_and_a_minion_to_corrupt() {
    let mut e = Engine::bare(3);
    // Gnaeus's text (unlock-phase damage to his prey) is not reached here.
    let b = e.add_minion(SeatId(0), crypt("Gnaeus Aemilius Augustinus"), 5, false);
    let a = e.add_minion(SeatId(0), anarch_with(&["pro"], &[], 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    e.state.seats[1].pool = 1;
    hand(&mut e, 0, &["Platinum Protocol, The"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == b)));
    pick(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert_eq!(corruption(&e, z, 0), 0);
}

// Break the Bonds: "[ani] The bleed is with +1 bleed. [obf] If the bleed is
// successful, you can lock a minion controlled by the target Methuselah.
// [pre] If the bleed is successful, add 1 blood to a vampire in your
// uncontrolled region." [ANK 20240706] "[pre] There must be an uncontrolled
// vampire, and it is chosen upon declaration"; "[obf] The prey need not
// control a minion, the target is chosen upon resolution." [LSJ
// 20011204-3] [PIB 20130704] played with the Disciplines used: any set of
// those the vampire has.
#[test]
fn break_the_bonds_locks_and_feeds_the_uncontrolled_region() {
    let v = anarch_with(&["obf", "pre"], &["ani"], 0);
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), v, 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Break the Bonds"]);
    // No uncontrolled vampire: no [pre] set.
    let req = request(&mut e);
    let offered = |req: &DecisionRequest| -> Vec<u8> {
        req.options
            .iter()
            .filter_map(|o| match o {
                Choice::Play {
                    mode,
                    actor: Some(m),
                    ..
                } if *m == a => Some(*mode),
                _ => None,
            })
            .collect()
    };
    assert_eq!(offered(&req), vec![1]);
    let slot = uncontrolled(&mut e, 0, plain("Toreador", "", 0));
    let req = request(&mut e);
    assert_eq!(offered(&req), vec![1, 2, 2 + 3]);
    // [obf][pre] at the uncontrolled vampire.
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 5,
            actor: Some(a),
            target: Target::Uncontrolled(slot),
        },
    );
    // The lock is optional; its minion chosen now.
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, Choice::ChooseMinion(z)]);
    pick(&mut e, Choice::ChooseMinion(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].locked);
    assert_eq!(e.state.seats[0].uncontrolled[slot as usize].counters, 1);
    assert_eq!(pool(&e, 1), 29);
}

// Enchanting Gaze [PRE][PRO]: "+1 stealth, and once this action, you can
// burn 1 of your corruption counters from a blocking minion to have their
// block attempt fail, and they cannot attempt to block this action again."
#[test]
fn enchanting_gaze_superior_burns_a_counter_to_fail_a_block() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRE", "PRO"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), with(&["aus"], 0), 5, false);
    e.state.minions[z.index()].add_corruption(SeatId(0), 2);
    hand(&mut e, 0, &["Enchanting Gaze"]);
    hand(&mut e, 1, &["Enhanced Senses"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick(&mut e, play(0, 1, a)); // +1 stealth: 1 against 0
    pick_when(&mut e, play(0, 0, z)); // Enhanced Senses: 1 against 1
    let fail = Choice::BurnCorruptionToFail { minion: z };
    pick_when(&mut e, fail);
    never_offered(&mut e, &Choice::DeclareBlock { minion: z });
    assert_eq!(corruption(&e, z, 0), 1, "one burned");
    assert_eq!(pool(&e, 1), 29);
}

// Revelation of the Serpent: "Only usable during a bleed action. [pre][pro]
// +1 bleed (limited), or +1 stealth and +1 bleed (limited). [PRE][PRO] As
// above, and if the bleed is successful, this vampire can burn 2 of your
// corruption counters from a minion controlled by the target Methuselah to
// unlock." The stealth option only when stealth is needed. [ANK 20220210]
// "Cannot be used to unlock if the target is ousted by the bleed."
#[test]
fn revelation_of_the_serpent_superior_unlocks_for_two_counters() {
    for ousted in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["PRE", "PRO"], 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
        e.state.minions[z.index()].add_corruption(SeatId(0), 2);
        if ousted {
            e.state.seats[1].pool = 2;
        }
        hand(&mut e, 0, &["Revelation of the Serpent"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        let req = until(&mut e, SeatId(0), WindowLabel::ActionBlock);
        assert!(req.options.contains(&play(0, 2, a)), "the bleed alone");
        assert!(!req.options.contains(&play(0, 3, a)), "no stealth needed");
        pick(&mut e, play(0, 2, a));
        let unlock = Choice::BurnCorruptionToUnlock { minion: z };
        if ousted {
            never_offered(&mut e, &unlock);
            assert!(e.state.seats[1].ousted);
            assert!(e.state.minions[a.index()].locked);
        } else {
            // An unlocked vampire can unlock too, to no effect (owner,
            // 2026-09-26: rules-qa 25.5).
            let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
            assert!(req.options.contains(&unlock));
            e.state.minions[a.index()].locked = false;
            let req = request(&mut e);
            assert!(req.options.contains(&unlock), "unlocked");
            e.state.minions[a.index()].locked = true;
            pick_when(&mut e, unlock);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(!e.state.minions[a.index()].locked);
            assert_eq!(corruption(&e, z, 0), 0);
            assert_eq!(pool(&e, 1), 28);
        }
    }
}

// Faruq Abd al-Qadir: "Minions with any of your corruption counters in
// combat with Faruq cannot strike: combat ends."
#[test]
fn faruq_bars_combat_ends_from_minions_he_corrupted() {
    for corrupted in [false, true] {
        let mut e = Engine::bare(3);
        let f = e.add_minion(
            SeatId(0),
            zvtes::state::find_card("Faruq Abd al-Qadir"),
            6,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), with(&["pre"], 0), 5, false);
        if corrupted {
            e.state.minions[z.index()].add_corruption(SeatId(0), 1);
        }
        hand(&mut e, 1, &["Majesty"]);
        pick(&mut e, action(f, ActionKind::Bleed));
        pick_when(&mut e, Choice::DeclareBlock { minion: z });
        if corrupted {
            // A lone hand strike: elided.
            never_offered(&mut e, &play(0, 0, z));
        } else {
            let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
            assert!(req.options.contains(&play(0, 0, z)), "Majesty");
        }
    }
}

// Aniel: "Aniel can burn 1 corruption counter or a card requiring a
// Discipline from another ready minion as a +1 stealth Ⓓ action that costs
// 1 blood." One printed action: after burning a counter she cannot burn a
// card the same turn (NRA). Several Methuselahs' counters: whose is chosen.
#[test]
fn aniel_burns_a_corruption_counter_as_one_printed_action() {
    let mut e = Engine::bare(3);
    let aniel = e.add_minion(SeatId(0), zvtes::state::find_card("Aniel"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    e.state.minions[z.index()].add_corruption(SeatId(1), 1);
    e.state.minions[z.index()].add_corruption(SeatId(2), 2);
    // A card requiring a Discipline on the same minion: the other half.
    e.state.cards.push(CardInPlay {
        by: Some(z),
        against: SeatSet::EMPTY,
        ..CardInPlay::new(find_library("Shadow Cloak"), SeatId(1), Some(z), 1)
    });
    let cloak = CardId(e.state.cards.len() as u32 - 1);
    let aniel_uses = |req: &DecisionRequest| -> Vec<u8> {
        req.options
            .iter()
            .filter_map(|o| match o {
                Choice::UseAbility {
                    source: AbilitySource::Vampire(m),
                    ability,
                    ..
                } if *m == aniel => Some(*ability),
                _ => None,
            })
            .collect()
    };
    let req = request(&mut e);
    assert_eq!(aniel_uses(&req), vec![0, 1]);
    let burn = Choice::UseAbility {
        source: AbilitySource::Vampire(aniel),
        ability: 1,
        actor: Some(aniel),
        target: Target::Minion(z),
    };
    pick(&mut e, burn);
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::ChooseSeat(SeatId(1)), Choice::ChooseSeat(SeatId(2))]
    );
    pick(&mut e, Choice::ChooseSeat(SeatId(2)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((corruption(&e, z, 1), corruption(&e, z, 2)), (1, 1));
    assert_eq!(blood(&e, aniel), 5);
    assert!(aniel_uses(&req).is_empty(), "locked");
    // Unlocked again: neither half of the printed action this turn.
    e.state.minions[aniel.index()].locked = false;
    let req = request(&mut e);
    assert!(aniel_uses(&req).is_empty(), "the same action");
    assert!(e.state.cards[cloak.index()].in_play);
}

// [ANK 20240706] Break the Bonds [obf]: "The prey need not control a minion,
// the target is chosen upon resolution."
#[test]
fn break_the_bonds_needs_no_minion_of_the_prey() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch_with(&["obf"], &[], 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    hand(&mut e, 0, &["Break the Bonds"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 29);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
}

// The Platinum Protocol [pro] with one minion to choose: the lone choice
// is elided, the counter put on it.
#[test]
fn platinum_protocol_corrupts_the_only_minion_without_asking() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch_with(&["pro"], &[], 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Platinum Protocol, The"]);
    pick(&mut e, play(0, 2, a));
    never_offered(&mut e, &Choice::ChooseMinion(z));
    assert_eq!(corruption(&e, z, 0), 1);
}

// ---- Effect-widened block eligibility (slice 11n) ---------------------------

// Eagle's Sight [AUS]: "This vampire attempts to block the current action,
// ignoring the normal prey, predator or target restrictions for blocking
// actions": a third Methuselah blocks a bleed directed at another.
#[test]
fn eagles_sight_superior_blocks_an_action_directed_at_another() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(2), with(&["AUS"], 0), 5, false);
    hand(&mut e, 2, &["Eagle's Sight"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    // Never an ordinary attempt for seat 2.
    let req = until(&mut e, SeatId(2), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&Choice::DeclareBlock { minion: z }));
    assert!(!req.options.contains(&play(0, 0, z)), "[aus]: no attempt");
    pick(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert!(e.state.minions[z.index()].locked);
    assert_eq!(pool(&e, 1), 30);
}

// [RTR 19950413] [RTR 20020501] Eagle's Sight [AUS]: "Only affects the rule
// restricting who gets to block an action. The vampire must still meet all
// other requirements to block. In particular, Blood Bond, Day Operation,
// Seduction and prior decision not to block are not circumvented."
#[test]
fn eagles_sight_superior_does_not_undo_a_decline_or_seduction() {
    for seduced in [false, true] {
        let mut e = Engine::bare(3);
        // Anja's text (unlocking another Hecata after its action) is inert here.
        let a = e.add_minion(SeatId(0), crypt("\"Mother\" Anja Giovanni"), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        // Younger than the acting vampire, for Seduction.
        let young = vampire(
            |c| has(c, &["AUS"]) && c.capacity <= 6 && c.title == Title::None,
            0,
        );
        let z = e.add_minion(SeatId(1), young, 5, false);
        hand(&mut e, 0, &["Seduction"]);
        hand(&mut e, 1, &["Eagle's Sight"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        if seduced {
            pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(z)));
        } else {
            // Seat 1 declines first.
            let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
            assert!(req.options.contains(&play(0, 1, z)));
            pick(&mut e, Choice::Pass);
        }
        never_offered(&mut e, &play(0, 1, z));
        assert_eq!(pool(&e, 1), 29, "seduced: {seduced}");
    }
}

// [LSJ 20030227] Eagle's Sight [AUS]: "Only applies to one block attempt. If
// the action later continues as if unblocked (eg. Form of Mist), another one
// is needed."
#[test]
fn eagles_sight_superior_covers_one_attempt() {
    let mut e = Engine::bare(3);
    let p = e.add_minion(SeatId(0), zvtes::state::find_card("Crystal Lynn"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(2), with(&["AUS"], 0), 5, false);
    let y = e.add_minion(SeatId(2), with(&["AUS"], 1), 5, false);
    hand(&mut e, 0, &["Form of Mist"]);
    hand(&mut e, 2, &["Eagle's Sight", "Eagle's Sight"]);
    pick(&mut e, action(p, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, z));
    // Combat: Form of Mist [PRO], then the continuation.
    pick_when(&mut e, play(0, 1, p));
    pick_when(&mut e, Choice::ContinueAction);
    // Back in the block loop: seat 2 still has no ordinary attempt; its
    // other vampire needs the other Eagle's Sight.
    let req = until(&mut e, SeatId(2), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&Choice::DeclareBlock { minion: y }));
    assert!(req.options.contains(&play(0, 1, y)));
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
}

// ---- Actions (slice 11o) -----------------------------------------------------

/// The Play choices offered for `m`, by (card, mode).
fn plays_by(req: &DecisionRequest, m: MinionId) -> Vec<(u8, u8)> {
    req.options
        .iter()
        .filter_map(|o| match o {
            Choice::Play {
                card,
                mode,
                actor: Some(a),
                ..
            } if *a == m => Some((*card, *mode)),
            _ => None,
        })
        .collect()
}

// Fleetness [CEL]: "+1 stealth action. Ⓓ Enter combat with a locked
// minion. This acting vampire gets 1 optional maneuver during that combat."
// (1 blood.) [ANK 20210124] [LSJ 20070411] [LSJ 20090514] "If the action is
// unblocked when it resolves and the target is unlocked, the action
// 'fizzles' (cost, if any, is paid, and it is considered successful, but
// has no effect)."
#[test]
fn fleetness_superior_enters_combat_with_a_locked_minion_or_fizzles() {
    for unlocked in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["CEL"], 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
        e.state.minions[z.index()].locked = true;
        hand(&mut e, 0, &["Fleetness"]);
        pick(&mut e, play_at(0, 1, Some(a), Target::Minion(z)));
        if unlocked {
            e.state.minions[z.index()].locked = false;
            never_offered(&mut e, &Choice::UseManeuver);
            assert_eq!(e.last_outcome, Some(Outcome::Fizzled));
            assert_eq!((blood(&e, a), blood(&e, z)), (4, 5), "paid, no combat");
        } else {
            let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
            assert!(req.options.contains(&Choice::UseManeuver));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.last_outcome, Some(Outcome::Successful));
            assert_eq!(blood(&e, z), 4, "a hand strike");
        }
    }
}

// Hunter's Mark: "[cel][tha] Ⓓ Enter combat with a minion. This acting
// vampire gets 1 optional press during that combat. [CEL][THA] As above,
// and the opposing minion cannot strike: combat ends during the first round
// of that combat."
#[test]
fn hunters_mark_gives_a_press_and_bars_combat_ends_in_the_first_round() {
    for superior in [false, true] {
        let mut e = Engine::bare(3);
        let name = if superior {
            "Khadija Al-Kindi"
        } else {
            "Bijou"
        };
        let a = e.add_minion(SeatId(0), zvtes::state::find_card(name), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), with(&["pre"], 0), 6, false);
        hand(&mut e, 0, &["Hunter's Mark"]);
        hand(&mut e, 1, &["Majesty", "Majesty"]);
        pick(
            &mut e,
            play_at(0, u8::from(superior), Some(a), Target::Minion(z)),
        );
        // Round 1: Majesty only without the superior bar.
        let mut majesty_in_round_1 = false;
        loop {
            let req = request(&mut e);
            if req.seat == SeatId(1) && req.options.contains(&play(0, 0, z)) {
                majesty_in_round_1 = true;
            }
            if (req.seat, req.window) == (SeatId(0), WindowLabel::CombatPress) {
                assert!(req.options.contains(&Choice::UsePress));
                break;
            }
            e.choose(fallback(&req));
        }
        assert_eq!(majesty_in_round_1, !superior);
        // The press continues: in round 2 Majesty is back.
        pick(&mut e, Choice::UsePress);
        pick_when(&mut e, play(0, 0, z));
    }
}

// Line Brawl: "Requires an Anarch. [cel] Ⓓ Steal 1 pool from another
// Methuselah." [LSJ 20011204-3] [PIB 20130704] a choice: one Discipline.
#[test]
fn line_brawl_steals_a_pool_from_any_other_methuselah() {
    let mut e = Engine::bare(3);
    // Its text (unlock after its own Gangrel action) is inert here.
    let a = e.add_minion(SeatId(0), crypt("Keegan"), 5, false);
    let b = e.add_minion(
        SeatId(0),
        vampire(
            |c| c.sect != zvtes::crypt::Sect::Anarch && has_level(c, &["cel"]),
            0,
        ),
        5,
        false,
    );
    e.add_minion(SeatId(2), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Line Brawl"]);
    let req = request(&mut e);
    assert!(plays_by(&req, b).is_empty(), "requires an Anarch");
    assert_eq!(plays_by(&req, a), vec![(0, 0), (0, 0)], "seats 1 and 2");
    pick(&mut e, play_at(0, 0, Some(a), Target::Seat(SeatId(2))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((pool(&e, 0), pool(&e, 2)), (31, 29));
}

// Show of Force [POT][PRE]: "Ⓓ Bleed with +2 bleed. If this action is
// blocked, before range is determined during the first round of the
// resulting combat, this vampire can gain +2 strength that combat."
#[test]
fn show_of_force_superior_gains_strength_when_blocked() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["POT", "PRE"], 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 6, false);
    hand(&mut e, 0, &["Show of Force"]);
    pick(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    pick(&mut e, Choice::GainStrength);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 6 - 3, "strength 3");
}

// Feast of the Soul's Secrets [DOM]: "Ⓓ Bleed with +2 bleed, and if the
// bleed is successful (for 1 or more), add 2 blood to a younger vampire you
// control or in your uncontrolled region." [ANK 20240628-2] "The target
// vampire is chosen when the card is played."
#[test]
fn feast_of_the_souls_secrets_superior_feeds_a_younger_vampire() {
    for uncontrolled_slot in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), zvtes::state::find_card("Djeneba"), 5, false);
        let young = vampire(|c| c.capacity <= 4 && c.title == Title::None, 0);
        let y = e.add_minion(SeatId(0), young, 1, false);
        hand(&mut e, 0, &["Feast of the Soul's Secrets"]);
        if uncontrolled_slot {
            let slot = uncontrolled(&mut e, 0, vampire(|c| c.capacity <= 4, 1));
            pick(&mut e, play_at(0, 2, Some(a), Target::Uncontrolled(slot)));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[0].uncontrolled[slot as usize].counters, 2);
        } else {
            pick(&mut e, play_at(0, 1, Some(a), Target::Minion(y)));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, y), 3);
        }
        assert_eq!(pool(&e, 1), 27);
        assert_eq!(blood(&e, a), 4, "1 blood");
    }
}

// Shroud of Decay [obl]: "Ⓓ Bleed with +1 bleed. If the bleed is successful
// (for 1 or more), the target Methuselah discards 2 cards of their choice."
#[test]
fn shroud_of_decay_makes_the_bled_methuselah_discard_two() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["obl"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Shroud of Decay"]);
    hand(&mut e, 1, &["Deflection", "Majesty", "Soak"]);
    hand(&mut e, 1, &["Blur"; 4]);
    let villein = find_library("Villein");
    e.state.seats[1].library = vec![villein; 3];
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert_eq!(req.options.len(), 4, "one choice per card: the Blurs alike");
    pick(&mut e, Choice::Discard(2)); // Soak
    pick(&mut e, Choice::Discard(0)); // Deflection
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Both discards made, then replaced (advanced rules 6.8.3).
    let hand1 = &e.state.seats[1].hand;
    assert_eq!(hand1.len(), 7);
    assert!(!hand1.contains(&find_library("Soak")) && !hand1.contains(&find_library("Deflection")));
    assert_eq!(hand1.iter().filter(|&&c| c == villein).count(), 2);
    assert_eq!(pool(&e, 1), 28);
}

// [LSJ 20090710] Shroud of Decay [OBL]: "Ⓓ Remove 7 cards in your prey's
// ash heap from the game to burn 3 of their pool." "The cards to remove are
// chosen by the player playing it. There is no order to the ash heap."
#[test]
fn shroud_of_decay_superior_removes_seven_ash_cards_for_three_pool() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["OBL"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Shroud of Decay"]);
    for _ in 0..6 {
        e.state.seats[1].ash.push(find_library("Soak"));
    }
    let req = request(&mut e);
    assert!(
        !req.options
            .contains(&play_at(0, 1, Some(a), Target::Seat(SeatId(1)))),
        "6 cards"
    );
    e.state.seats[1].ash.push(find_library("Majesty"));
    e.state.seats[1].ash.push(find_library("Deflection"));
    pick(&mut e, play_at(0, 1, Some(a), Target::Seat(SeatId(1))));
    // The player picks: Majesty and six Soaks, Deflection stays.
    let majesty = find_library("Majesty");
    pick_when(&mut e, Choice::FindCard(majesty));
    for _ in 0..6 {
        pick(&mut e, Choice::FindCard(find_library("Soak")));
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].ash, vec![find_library("Deflection")]);
    assert_eq!(e.state.seats[1].removed.len(), 7);
    assert_eq!(pool(&e, 1), 27);
}

// Rewilding: "Ⓓ Burn a location and burn 2 pool from its controller."
// [LSJ 20090324] Directed at the location's controller.
#[test]
fn rewilding_burns_a_location_and_two_pool_from_its_controller() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Gangrel", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(2), plain("Ventrue", "", 0), 5, false);
    let place = location(&mut e, 2, "Elysian Fields");
    hand(&mut e, 0, &["Rewilding"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Card(place)));
    // Directed at seat 2: its vampire may block.
    let req = until(&mut e, SeatId(2), WindowLabel::ActionBlock);
    assert!(req.options.contains(&Choice::DeclareBlock { minion: z }));
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[place.index()].in_play);
    assert_eq!(pool(&e, 2), 28);
}

/// `name` in play on `host` as if played in version `mode`, with that
/// mode's layers.
fn attach(e: &mut Engine, name: &str, host: MinionId, mode: u8) -> CardId {
    let card = find_library(name);
    let seat = e.state.minions[host.index()].owner;
    e.state.cards.push(CardInPlay {
        by: Some(host),
        against: SeatSet::EMPTY,
        ..CardInPlay::new(card, seat, Some(host), mode)
    });
    let id = CardId(e.state.cards.len() as u32 - 1);
    for effect in cards::lookup(card).expect("definition").modes[mode as usize].effects {
        if let Effect::Layer(modifier) = effect {
            e.state.layers.push(Layer {
                scope: Scope::InPlay,
                modifier: *modifier,
                source: id,
            });
        }
    }
    id
}

// Preternatural Strength [POT]: "+2 stealth action. Put this card on this
// vampire. This vampire gets +2 strength. They cannot play cards named Torn
// Signpost. A vampire can have only one Preternatural Strength."
#[test]
fn preternatural_strength_superior_adds_strength_and_bars_torn_signpost() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["POT"], 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 6, false);
    hand(
        &mut e,
        0,
        &["Preternatural Strength", "Preternatural Strength"],
    );
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, a), vec![(find_library("Preternatural Strength"), 0)]);
    assert_eq!(blood(&e, a), 5, "1 blood");
    assert!(e.state.bars_playing(a, find_library("Torn Signpost")));
    // Next turn: no second one; a combat at strength 3, no Torn Signpost.
    next_own_turn(&mut e);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 1, a)), "only one");
    hand(&mut e, 0, &["Torn Signpost"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    never_offered(&mut e, &play(1, 0, a));
    assert_eq!(blood(&e, z), 6 - 3);
}

// Heroic Might: "This vampire gets +1 strength. This vampire can strike:
// burn equipment. Burn this card if this vampire is in torpor. [POT] As
// above, with an additional +1 strength, and this vampire can strike: 2R
// damage."
#[test]
fn heroic_might_strikes_to_burn_equipment_and_burns_in_torpor() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["POT"], 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 6, false);
    let might = attach(&mut e, "Heroic Might", a, 1);
    let vest = equip(&mut e, "Kevlar Vest", z);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    // No blood left: the opposing hand strike sends it to torpor.
    e.state.minions[a.index()].blood = 0;
    let burn = Choice::UseAbility {
        source: AbilitySource::Card(might),
        ability: 0,
        actor: None,
        target: Target::Card(vest),
    };
    assert!(req.options.contains(&burn), "{:?}", req.options);
    let ranged = Choice::UseAbility {
        source: AbilitySource::Card(might),
        ability: 1,
        actor: None,
        target: Target::None,
    };
    assert!(req.options.contains(&ranged), "[POT]: 2R");
    pick(&mut e, burn);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.cards[vest.index()].in_play,
        "burned at close range"
    );
    assert!(e.state.minions[a.index()].in_torpor);
    assert!(!e.state.cards[might.index()].in_play, "burned in torpor");
}

// Rutor's Hand: "+1 stealth action. [tha] Put this card on this vampire,
// locked; this vampire takes 1 unpreventable environmental aggravated
// damage. During your minion phase, this vampire can lock this card to
// unlock. [THA] As above, and this vampire can burn 3 blood to be immune to
// this aggravated damage." (1 blood.)
#[test]
fn rutors_hand_wounds_or_costs_blood_then_unlocks_its_vampire() {
    for immune in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["THA"], 0), 6, false);
        let start = e.state.capacity_of(a).unwrap().min(6);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", "", 0), 6, false);
        hand(&mut e, 0, &["Rutor's Hand"]);
        pick(&mut e, play(0, 1, a));
        let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        assert_eq!(req.options, vec![Choice::Pass, Choice::BurnBloodToAvoid]);
        pick(
            &mut e,
            if immune {
                Choice::BurnBloodToAvoid
            } else {
                Choice::Pass
            },
        );
        let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let hand_id = remembering(&e, "Rutor's Hand")[0].0;
        assert!(e.state.cards[hand_id.index()].locked, "locked");
        if !immune {
            assert!(e.state.minions[a.index()].in_torpor, "aggravated damage");
            continue;
        }
        assert_eq!(blood(&e, a), start - 1 - 3);
        // Locked itself: not usable this turn.
        assert!(!req.options.contains(&ability(hand_id)));
        // Next turn: the card unlocks; the vampire must hunt — its only
        // option, the Hand not offered while it is unlocked (rules-qa 48.7)
        // — then unlocks.
        next_own_turn(&mut e);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[a.index()].locked);
        pick(&mut e, ability(hand_id));
        let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[a.index()].locked);
        assert!(e.state.cards[hand_id.index()].locked);
        assert!(!req.options.contains(&ability(hand_id)));
    }
}

// Monstrous Form [dom][pro]: "Only usable before range is determined. This
// vampire gets +1 strength this combat. A vampire can play only one
// Monstrous Form each round." [DOM][PRO]: "Put this card on this vampire.
// During combat, you can lock this card to give this vampire +1 strength
// this round, or 1 maneuver or press."
#[test]
fn monstrous_form_adds_strength_and_its_card_locks_for_a_bonus() {
    let mut e = Engine::bare(3);
    // Máddji's text (cheaper Tzimisce allies and retainers) is inert here.
    let a = e.add_minion(SeatId(0), crypt("Máddji, Mistress of Szlachtas"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    let form = attach(&mut e, "Monstrous Form", a, 1);
    hand(&mut e, 0, &["Monstrous Form", "Monstrous Form"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, a));
    // One each round: the other copy is not offered again.
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)), "{:?}", req.options);
    // The card in play: +1 strength this round.
    let strength = Choice::UseAbility {
        source: AbilitySource::Card(form),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, strength);
    assert!(e.state.cards[form.index()].locked);
    // Round 1: 1 + 1 (combat) + 1 (round): 3 damage; then combat ends.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 5 - 3);
}

// Touch of Valeren: "[for] [ACTION] +1 stealth action. Add 3 blood or life
// to a minion, not to exceed their starting life. [for] [COMBAT] Prevent
// all damage from the opposing minion's strike. [FOR] [COMBAT] Only usable
// by a ready vampire not involved in the combat involving another minion
// you control. Prevent all damage to that minion from the opposing
// minion's strike." (1 blood; Salubri.) [LSJ 20001114] "Cannot be used if
// there is no damage to prevent."
#[test]
fn touch_of_valeren_heals_and_prevents_a_strike_even_from_outside() {
    // [ACTION]: up to the starting life (capacity).
    let mut e = Engine::bare(3);
    let s = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Salubri" && has(c, &["FOR"]), 0),
        5,
        false,
    );
    let cap = e.state.capacity_of(s).unwrap();
    let y = e.add_minion(SeatId(0), plain("Brujah", "", 1), 3, false);
    let ycap = e.state.capacity_of(y).unwrap();
    e.state.minions[y.index()].blood = ycap - 1;
    hand(&mut e, 0, &["Touch of Valeren", "Touch of Valeren"]);
    pick(&mut e, play_at(0, 0, Some(s), Target::Minion(y)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, y), ycap, "not above its capacity");
    assert_eq!(blood(&e, s), 4, "1 blood");
    let _ = cap;
    // [FOR] [COMBAT]: another vampire of seat 0 fights; the Salubri, ready
    // and out of the combat, prevents the strike's damage.
    let mut e = Engine::bare(3);
    let s = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Salubri" && has(c, &["FOR"]), 0),
        5,
        false,
    );
    let f = e.add_minion(SeatId(0), plain("Brujah", "", 1), 5, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Touch of Valeren"]);
    pick(&mut e, action(f, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPrevention);
    assert!(req.options.contains(&play(0, 2, s)), "{:?}", req.options);
    assert!(
        !req.options.contains(&play(0, 1, f)),
        "[for]: a Salubri's card"
    );
    pick(&mut e, play(0, 2, s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, f), blood(&e, s), blood(&e, z)), (5, 4, 4));
    // [for] [COMBAT]: the Salubri itself in combat.
    let mut e = Engine::bare(3);
    let s = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Salubri" && has(c, &["FOR"]), 0),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 5, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Touch of Valeren"]);
    pick(&mut e, action(s, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 1, s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, s), blood(&e, z)),
        (4, 4),
        "1 blood paid, no damage taken"
    );
}

// Rewilding on one's own location, at 2 pool: the acting Methuselah is
// ousted by it, and the turn passes.
#[test]
fn rewilding_ousting_its_own_player_passes_the_turn() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Gangrel", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(2), plain("Ventrue", "", 1), 5, false);
    e.state.seats[0].pool = 2;
    let place = location(&mut e, 0, "Elysian Fields");
    hand(&mut e, 0, &["Rewilding"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Card(place)));
    // Undirected: prey and predator may block; both decline.
    pick(&mut e, Choice::Pass);
    pick(&mut e, Choice::Pass);
    let req = request(&mut e);
    assert!(e.state.seats[0].ousted);
    assert_eq!(req.seat, SeatId(1), "{req:?}");
    assert_eq!(e.state.active, SeatId(1));
}

// Biothaumaturgic Experiment [tha]: "+1 stealth action. Employ an animal
// retainer from your hand ignoring requirements (pay cost as normal)." (1
// pool.) [LSJ 20080803] a version depending on a Discipline level follows
// the vampire's Disciplines: without Animalism, the basic one. [ANK
// 20180817] Not an employ action. [LSJ 20100725] [LSJ 20100303] [ANK
// 20180817] Its requirements do not become the action's.
#[test]
fn biothaumaturgic_experiment_employs_an_animal_ignoring_requirements() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(
        SeatId(0),
        vampire(
            |c| has(c, &["tha"]) && !has_level(c, &["ani"]) && c.title == Title::None,
            0,
        ),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    hand(&mut e, 0, &["Biothaumaturgic Experiment", "Raven Spy"]);
    let spy = find_library("Raven Spy");
    // Raven Spy alone: its Animalism is required.
    let req = request(&mut e);
    assert!(!plays_by(&req, a).contains(&(1, 0)));
    // The retainer named with the play: a lone choice, elided.
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, a), vec![(spy, 1)], "the retainer, basic, 1 life");
    assert_eq!(blood(&e, a), 4, "Raven Spy's blood");
    assert_eq!(pool(&e, 0), 29, "the card's pool");
}

// Biothaumaturgic Experiment [THA]: "Put this card on a minion you control.
// The attached minion gets 1 optional maneuver each combat and +1
// strength."
#[test]
fn biothaumaturgic_experiment_superior_strengthens_a_minion() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["THA"], 0), 5, false);
    let y = e.add_minion(SeatId(0), plain("Brujah", "", 1), 5, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Biothaumaturgic Experiment"]);
    pick(&mut e, play_at(0, 1, Some(a), Target::Minion(y)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let bio = find_library("Biothaumaturgic Experiment");
    assert_eq!(on(&e, y), vec![(bio, 0)]);
    pick(&mut e, action(y, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.iter().any(|o| matches!(
        o,
        Choice::UseAbility {
            source: AbilitySource::Card(_),
            ..
        }
    )));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 5 - 2, "strength 2");
}

// Revelations [aus]: "+1 stealth action. Ⓓ Look at your prey's hand and
// discard one card of your choice from it." (1 blood.) [LSJ 20100119] [PIB
// 20121031] "You do not get to see the card drawn as replacement."
#[test]
fn revelations_discards_a_card_of_the_preys_hand() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["aus"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Revelations"]);
    hand(&mut e, 1, &["Soak", "Majesty", "Soak"]);
    let villein = find_library("Villein");
    e.state.seats[1].library = vec![villein; 5];
    pick(&mut e, play_at(0, 0, Some(a), Target::Seat(SeatId(1))));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    let majesty = find_library("Majesty");
    let soak = find_library("Soak");
    let mut want = vec![Choice::FindCard(soak), Choice::FindCard(majesty)];
    want.sort_by_key(|c| match c {
        Choice::FindCard(x) => *x,
        _ => 0,
    });
    assert_eq!(req.options, want);
    pick(&mut e, Choice::FindCard(majesty));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.seats[1].hand.contains(&majesty));
    assert!(e.state.seats[1].hand.contains(&villein), "replaced");
    assert!(!e.state.hand_visible_to(SeatId(1), SeatId(0)), "unseen");
}

// Revelations [AUS]: "Put this card in play. Your prey plays with an open
// hand. Any minion can burn this card as a Ⓓ action."
#[test]
fn revelations_superior_opens_the_preys_hand_until_burned() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["AUS"], 0), 5, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Revelations"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.hand_visible_to(SeatId(1), SeatId(0)));
    assert!(e.state.hand_visible_to(SeatId(1), SeatId(2)), "open to all");
    assert!(!e.state.hand_visible_to(SeatId(2), SeatId(0)));
    let rev = remembering(&e, "Revelations")[0].0;
    let burn = Choice::UseAbility {
        source: AbilitySource::Card(rev),
        ability: 0,
        actor: Some(b),
        target: Target::None,
    };
    pick(&mut e, burn);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[rev.index()].in_play);
    assert!(!e.state.hand_visible_to(SeatId(1), SeatId(0)));
}

// Rutor's Hand: "During your minion phase, this vampire can lock this card
// to unlock" — at any time of it, during another vampire's action too
// (owner, 2026-09-26: rules-qa 26.5), e.g. to then play Veil the Legions.
#[test]
fn rutors_hand_unlocks_during_another_vampires_action() {
    let mut e = Engine::bare(3);
    let r = e.add_minion(SeatId(0), with(&["OBF"], 0), 5, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    e.state.minions[r.index()].locked = true;
    let rutor = attach(&mut e, "Rutor's Hand", r, 0);
    pick(&mut e, action(b, ActionKind::Bleed));
    pick_when(&mut e, ability(rutor));
    assert!(!e.state.minions[r.index()].locked);
    assert!(e.state.cards[rutor.index()].locked);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 29);
}

// Not offered while the vampire is unlocked: locking the card would buy
// nothing (owner, 2026-10-04: rules-qa 48.7).
#[test]
fn rutors_hand_is_not_offered_to_an_unlocked_vampire() {
    for locked in [false, true] {
        let mut e = Engine::bare(3);
        let r = e.add_minion(SeatId(0), with(&["OBF"], 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
        e.state.minions[r.index()].locked = locked;
        let rutor = attach(&mut e, "Rutor's Hand", r, 0);
        let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(req.options.contains(&ability(rutor)), locked);
    }
}

// ---- Actions, wave 2 (slice 11p) ----------------------------------------------

// Obedient Flesh [dom][pro]: "+1 stealth. Allies get -1 intercept."
// [DOM][PRO] [COMBAT]: "Only usable before range is determined. This round,
// this vampire gets +1 strength and 1 optional maneuver, and can prevent 1
// damage."
#[test]
fn obedient_flesh_hinders_allies_and_arms_a_round() {
    let mut e = Engine::bare(3);
    // Máddji's text (cheaper Tzimisce allies and retainers) is inert here.
    let a = e.add_minion(SeatId(0), crypt("Máddji, Mistress of Szlachtas"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    let z = e.add_minion(SeatId(1), with(&["aus"], 0), 5, false);
    hand(&mut e, 0, &["Obedient Flesh", "Obedient Flesh"]);
    hand(&mut e, 1, &["Enhanced Senses"]);
    // The ally attempts at 0 against 0: +1 stealth and -1 intercept.
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: ally });
    pick(&mut e, play(0, 0, a));
    // Seat 1 again: its vampire could still reach 1 with Enhanced Senses.
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    let i = e.state.layers.iter().any(|l| {
        matches!(
            l.modifier,
            cards::Modifier::InterceptMod {
                younger_than: 0,
                n: -1
            }
        )
    });
    assert!(i, "allies get -1 intercept");
    // Then the vampire blocks with Enhanced Senses; the combat card.
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver));
    pick_when(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 5 - 2, "strength 2");
    assert_eq!(
        blood(&e, a),
        5 - 1 - 1,
        "1 blood each card, the hand strike prevented"
    );
}

// The Gate of Acheron: "Put this card in play with 1 counter. During your
// unlock phase, add 1 counter to this card. During your prey's unlock
// phase, for each counter on this card, they burn 1 pool or remove a
// library card at random in their ash heap from the game. Hecata you
// control with capacity 4 or more can add 1 counter to this card as a +1
// stealth action." [LSJ 20080604] the action requires Hecata.
#[test]
fn gate_of_acheron_grows_and_drains_the_prey() {
    let mut e = Engine::bare(3);
    let h = e.add_minion(SeatId(0), plain("Hecata", "", 0), 5, false);
    let young = e.add_minion(SeatId(0), zvtes::state::find_card("Alek König"), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Gate of Acheron, The"]);
    pick(&mut e, play(0, 0, h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let gate = remembering(&e, "Gate of Acheron, The")[0].0;
    assert_eq!(e.state.cards[gate.index()].counters, 1);
    // The action to add a counter: a Hecata of capacity 4 or more only.
    let add = |m| Choice::UseAbility {
        source: AbilitySource::Card(gate),
        ability: 0,
        actor: Some(m),
        target: Target::None,
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&add(young)), "capacity 3");
    assert!(!req.options.contains(&add(h)), "locked");
    e.state.minions[h.index()].locked = false;
    pick(&mut e, add(h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[gate.index()].counters, 2);
    // The prey's unlock phase: 2 counters, 1 ash card and 1 pool.
    e.state.seats[1].ash = vec![find_library("Soak")];
    let req = until(&mut e, SeatId(1), WindowLabel::RiderTerms);
    assert_eq!(req.options, vec![Choice::SetX(0), Choice::SetX(1)]);
    pick(&mut e, Choice::SetX(1));
    assert_eq!(pool(&e, 1), 29);
    assert!(e.state.seats[1].ash.is_empty());
    assert_eq!(e.state.seats[1].removed, vec![find_library("Soak")]);
    // Seat 0's unlock phase: one more counter.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[gate.index()].counters, 3);
}

// Pit of Contemplation: "During your unlock phase, you can burn this card
// to move a vampire controlled by your predator or prey with capacity
// less than X to their owner's uncontrolled region, where X is the number
// of counters on this card."
#[test]
fn pit_of_contemplation_moves_a_young_vampire_to_the_uncontrolled_region() {
    let mut e = Engine::bare(3);
    let h = e.add_minion(SeatId(0), plain("Hecata", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let young = e.add_minion(SeatId(1), zvtes::state::find_card("Alek König"), 2, false);
    let old = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Pit of Contemplation"]);
    pick(&mut e, play(0, 0, h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let pit = remembering(&e, "Pit of Contemplation")[0].0;
    e.state.cards[pit.index()].counters = 4;
    // Seat 0's next unlock phase: the Pit's burn, at capacity 3 only.
    let burn = |t| Choice::UseAbility {
        source: AbilitySource::Card(pit),
        ability: 1,
        actor: None,
        target: Target::Minion(t),
    };
    let req = loop {
        let req = request(&mut e);
        if req.seat == SeatId(0) && req.options.contains(&burn(young)) {
            break req;
        }
        e.choose(fallback(&req));
    };
    assert!(!req.options.contains(&burn(old)), "capacity 4 or more");
    pick(&mut e, burn(young));
    assert!(!e.state.cards[pit.index()].in_play);
    let slot = e.state.seats[1].uncontrolled.last().expect("moved");
    assert_eq!(
        (slot.card, slot.counters),
        (zvtes::state::find_card("Alek König"), 2)
    );
}

// Deep Song [ANI]: "Frenzy. Ⓓ Enter combat with and lock a vampire. The
// target vampire is considered the acting minion during that combat." [ANK
// 20200702] "The lock and enter combat effects are simultaneous": the
// target, locked, has the acting minion's side — its Methuselah first in
// the combat.
#[test]
fn deep_song_superior_locks_and_fights_a_vampire_as_the_acting_minion() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ANI"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Deep Song"]);
    hand(&mut e, 1, &["Majesty"]);
    pick(&mut e, play_at(0, 1, Some(a), Target::Minion(z)));
    // The combat, Majesty offered at its strike: the target on the acting
    // minion's side (its Methuselah first to act in each step).
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert_eq!(e.state.combatants, Some([z, a]), "the target acts");
    assert!(
        e.state.minions[z.index()].locked,
        "locked as the combat begins"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
}

// A blocked employ action goes to the ash heap even when the acting vampire
// already has a retainer of that name (the card never entered play).
#[test]
fn a_blocked_second_raven_spy_is_discarded() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ani"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), with(&["aus"], 0), 5, false);
    let spy = find_library("Raven Spy");
    equip(&mut e, "Raven Spy", a);
    hand(&mut e, 0, &["Raven Spy"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick(&mut e, play(0, 0, z)); // +1 intercept against the employ's 1
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert_eq!(e.state.seats[0].ash, vec![spy]);
}

/// Seat 0's Djeneba (capacity 9, [DOM]) puts Puppet Master [DOM] on seat
/// 1's younger vampire `z`, locked already (PIB 20150412); the turn passes
/// to seat 0's next minion phase. Returns (engine, puppeteer, second
/// vampire of seat 0, z, the card).
fn puppet_master_next_turn() -> (Engine, MinionId, MinionId, MinionId, CardId) {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), zvtes::state::find_card("Djeneba"), 6, false);
    let b = e.add_minion(SeatId(0), with(&["CEL"], 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 1), 5, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Puppet Master"]);
    pick(&mut e, play_at(0, 1, Some(a), Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let pm = remembering(&e, "Puppet Master")[0].0;
    assert_eq!(e.state.cards[pm.index()].host, Some(z));
    assert_eq!(
        e.state.card_controller(pm),
        SeatId(0),
        "you still control this card"
    );
    assert_eq!(blood(&e, a), 4, "2 blood");
    next_own_turn(&mut e);
    assert!(
        e.state.minions[z.index()].locked,
        "does not unlock as normal"
    );
    (e, a, b, z, pm)
}

// Puppet Master [DOM]: "Ⓓ Put this card on a younger vampire and lock them;
// you still control this card. The attached vampire does not unlock as
// normal. During your next minion phase, burn this card to unlock the
// attached vampire and take control of them until the end of your turn."
// [PIB 20150412] "Can be played on a locked vampire." [LSJ 20001201] "Can
// be used … at any point during the appropriate minion phase. It does not
// have to be done before non-mandatory actions."
#[test]
fn puppet_master_superior_takes_a_vampire_for_the_next_turn() {
    let (mut e, a, _, z, pm) = puppet_master_next_turn();
    // Due: the minion phase cannot end, but other actions come first.
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::Pass));
    pick(&mut e, action(a, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, ability(pm));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[pm.index()].in_play, "burned");
    let v = &e.state.minions[z.index()];
    assert_eq!((v.owner, v.locked), (SeatId(0), false));
    // It hunts for seat 0, then returns as the turn ends.
    pick(&mut e, action(z, ActionKind::Hunt));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert_eq!(v.owner, SeatId(1));
    assert_eq!(v.temp_from, None);
}

// [LSJ 20080326] Stealing "a minion attempting to block, if the block
// attempt becomes illegal, the block attempt fails." [LSJ 20020618] "If used
// to steal a vampire in combat, combat ends. Any damage yet to resolve still
// resolves before the change of control."
#[test]
fn puppet_master_steals_a_blocker_or_a_combatant() {
    for in_combat in [false, true] {
        let (mut e, _, b, z, pm) = puppet_master_next_turn();
        hand(&mut e, 1, &["On the Qui Vive"]);
        pick(&mut e, action(b, ActionKind::Bleed));
        // Seat 1 wakes the locked vampire and blocks with it.
        pick_when(&mut e, play(0, 0, z));
        pick_when(&mut e, Choice::DeclareBlock { minion: z });
        if in_combat {
            until(&mut e, SeatId(0), WindowLabel::CombatStrike);
            // A hand strike each, then the steal in the prevention step.
            pick(&mut e, Choice::HandStrike);
            pick_when(&mut e, ability(pm));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.last_outcome, Some(Outcome::Blocked));
        } else {
            pick_when(&mut e, ability(pm));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(pool(&e, 1), 29, "the attempt failed");
        }
        assert_eq!(e.state.minions[z.index()].owner, SeatId(0));
    }
}

// [LSJ 20020725] "If used to steal a minion targeted by an action, the
// action continues with the same target … If the target is not legal
// anymore, the action fizzles (ends successfully with no effect)."
#[test]
fn puppet_master_stealing_the_target_fizzles_the_action() {
    let (mut e, _, b, z, pm) = puppet_master_next_turn();
    hand(&mut e, 0, &["Fleetness"]);
    // Fleetness [CEL] at the locked vampire, then the steal.
    pick(&mut e, play_at(0, 1, Some(b), Target::Minion(z)));
    pick_when(&mut e, ability(pm));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Fizzled));
    assert_eq!(e.state.minions[z.index()].owner, SeatId(0));
}

// [ANK 20181110] [LSJ 20081105] "If one is ousted while having temporary
// control of a minion, the minion is removed from the game." [ANK
// 20181110] "The stolen minion is removed from the game when he should
// return to a Methuselah who has been ousted." [LSJ 20010119] "The stolen
// minion returns to his previous controller even if he is in torpor."
#[test]
fn puppet_master_temporary_control_and_ousts() {
    // Its controller ousted by its bleed: removed at the end of the turn.
    let (mut e, _, _, z, pm) = puppet_master_next_turn();
    pick(&mut e, ability(pm));
    e.state.seats[1].pool = 1;
    pick_when(&mut e, action(z, ActionKind::Bleed));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert!(
        e.state.minions[z.index()].in_play,
        "until the end of the turn"
    );
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_play, "removed, not returned");
    // The taker ousted: removed with its minions.
    let (mut e, a, _, z, pm) = puppet_master_next_turn();
    pick(&mut e, ability(pm));
    e.state.seats[0].pool = 2;
    let place = location(&mut e, 0, "Elysian Fields");
    hand(&mut e, 0, &["Rewilding"]);
    e.state.minions[a.index()].locked = true;
    e.state.minions[z.index()].locked = true;
    let g = e.add_minion(SeatId(0), plain("Gangrel", "", 0), 5, false);
    pick(&mut e, play_at(0, 0, Some(g), Target::Card(place)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].ousted);
    assert!(!e.state.minions[z.index()].in_play);
    // In torpor at the end of the turn: it returns, in torpor.
    let (mut e, _, _, z, pm) = puppet_master_next_turn();
    pick(&mut e, ability(pm));
    e.state.minions[z.index()].in_torpor = true;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert!(v.in_play && v.in_torpor);
    assert_eq!(v.owner, SeatId(1));
}

// [LSJ 20031106] Rutor's Hand: "If the vampire is stolen before 'your next
// turn', the Hand will not allow the vampire an extra unlock until a
// Methuselah begins a turn with the vampire with the Hand under her
// control": the card, locked as it entered, unlocks only in its
// controller's unlock phase.
#[test]
fn rutors_hand_on_a_stolen_vampire_stays_locked() {
    let (mut e, _, _, z, pm) = puppet_master_next_turn();
    let hand_id = attach(&mut e, "Rutor's Hand", z, 0);
    e.state.cards[hand_id.index()].locked = true;
    pick(&mut e, ability(pm));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[z.index()].owner, SeatId(0));
    assert!(!req.options.contains(&ability(hand_id)));
}

// ---- Masters (slice 11q) ---------------------------------------------------

// Oblivion (a Discipline master card): "Put this card on a vampire. This
// vampire gets +1 level of Oblivion [obl] and +1 capacity. Cannot be put on
// a vampire with superior Oblivion [OBL]." Any Methuselah's vampire; the
// level and capacity are read live.
#[test]
fn oblivion_gives_a_level_of_oblivion_and_one_capacity() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch_with(&[], &["obl", "OBL"], 0), 3, false);
    let basic = e.add_minion(SeatId(1), with(&["obl"], 0), 3, false);
    let superior = e.add_minion(SeatId(1), with(&["OBL"], 0), 3, false);
    let torpid = e.add_minion(SeatId(2), plain("", "", 2), 0, true);
    hand(&mut e, 0, &["Oblivion", "Oblivion", "Shroud of Decay"]);
    e.at_master_phase();
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let on = |m| play_at(0, 0, None, Target::Minion(m));
    assert!(!req.options.contains(&on(superior)), "superior Oblivion");
    assert!(req.options.contains(&on(basic)) && req.options.contains(&on(torpid)));
    let cap = e.state.capacity_of(a).unwrap();
    assert_eq!(e.state.discipline_level(a, "obl"), 0);
    pick(&mut e, on(a));
    request(&mut e);
    assert_eq!(e.state.capacity_of(a), Some(cap + 1));
    assert_eq!(e.state.discipline_level(a, "obl"), 1);
    // Now it can play a card requiring Oblivion, and gain blood to its new
    // capacity.
    e.state.minions[a.index()].blood = cap;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let req = request(&mut e);
    let shroud = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Shroud of Decay"))
        .unwrap() as u8;
    assert!(
        plays_by(&req, a).contains(&(shroud, 0)),
        "[obl] Shroud of Decay"
    );
    assert!(!plays_by(&req, a).contains(&(shroud, 1)), "not [OBL]");
    pick(&mut e, action(a, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), cap + 1);
}

// A second level on an inferior vampire makes it superior; on the same
// vampire a second copy stacks the same way (+1 capacity each).
#[test]
fn a_discipline_master_on_an_inferior_vampire_gives_superior() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["pot"], 0), 3, false);
    e.add_minion(SeatId(0), plain("", "", 1), 3, false);
    e.add_minion(SeatId(1), plain("", "", 2), 3, false);
    e.add_minion(SeatId(2), plain("", "", 3), 3, false);
    hand(&mut e, 0, &["Potence", "Potence"]);
    e.at_master_phase();
    let cap = e.state.capacity_of(a).unwrap();
    pick(&mut e, play_at(0, 0, None, Target::Minion(a)));
    request(&mut e);
    assert_eq!(e.state.discipline_level(a, "pot"), 2);
    assert_eq!(e.state.capacity_of(a), Some(cap + 1));
    // Next master phase: now superior, it cannot take another.
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(!req
        .options
        .contains(&play_at(0, 0, None, Target::Minion(a))));
}

// Advanced rules 5.1.1: cards on an uncontrolled vampire are out of play, so
// its capacity there is printed — blood held for the raised capacity does not
// drain off, and returns with it.
#[test]
fn a_banished_vampire_keeps_the_blood_its_discipline_card_held() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch_with(&[], &["obl", "OBL"], 0), 3, false);
    e.add_minion(SeatId(0), plain("", "", 1), 3, false);
    let obl = attach(&mut e, "Oblivion", a, 0);
    let cap = e.state.capacity_of(a).unwrap();
    e.state.minions[a.index()].blood = cap;
    // Banished (Banishment's move, by hand): out of play with its card.
    let card = e.state.minions[a.index()].crypt_card().unwrap();
    e.state.minions[a.index()].in_play = false;
    e.state.minions[a.index()].blood = 0;
    e.state.cards[obl.index()].in_play = false;
    e.state.seats[0].uncontrolled.push(UncontrolledSlot {
        card,
        counters: cap,
        banished: Some(zvtes::state::Banished {
            minion: a,
            cards: vec![obl],
        }),
    });
    let slot = e.state.seats[0].uncontrolled.len() as u8 - 1;
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    // Printed capacity there: already enough to influence out — more can
    // still go on (rules-qa 28.3).
    assert!(req.options.contains(&Choice::Transfer { slot }));
    pick(&mut e, Choice::InfluenceOut { slot });
    request(&mut e);
    assert!(e.state.minions[a.index()].in_play);
    assert_eq!(e.state.capacity_of(a), Some(cap));
    assert_eq!(blood(&e, a), cap);
    assert_eq!(e.state.discipline_level(a, "obl"), 1);
}

// Childe of the Revolution: "+1 stealth action. Requires a non-sterile
// baron. Put this card in play. It becomes a 1-capacity (non-unique) Anarch
// vampire of the same clan as the acting vampire and must hunt this turn.
// You can search your library (shuffle afterward), hand and/or ash heap for
// a Discipline master card and put it on this new vampire."
#[test]
fn childe_of_the_revolution_becomes_a_vampire_that_must_hunt() {
    let mut e = Engine::bare(3);
    let b = e.add_minion(SeatId(0), baron(0), 5, false);
    let other = e.add_minion(SeatId(0), plain("", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    hand(&mut e, 0, &["Childe of the Revolution", "Oblivion"]);
    hand(&mut e, 0, &["Blur"; 5]);
    // The top card replaces Childe of the Revolution as it is played.
    e.state.seats[0].library = vec![
        find_library("Potence"),
        find_library("Soak"),
        find_library("Soak"),
    ];
    e.state.seats[0].ash = vec![find_library("Celerity")];
    let req = request(&mut e);
    assert!(!plays_by(&req, other).contains(&(0, 0)), "requires a baron");
    let before = e.state.minions.len();
    pick(&mut e, play(0, 0, b));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    use zvtes::decision::Zone;
    let take = |zone, name| Choice::TakeFrom {
        zone,
        card: find_library(name),
    };
    assert!(req.options.contains(&take(Zone::Hand, "Oblivion")));
    assert!(req.options.contains(&take(Zone::AshHeap, "Celerity")));
    assert!(!req.options.contains(&take(Zone::Library, "Soak")));
    pick(&mut e, take(Zone::Library, "Potence"));
    let childe = MinionId(before as u32);
    let v = &e.state.minions[childe.index()];
    assert_eq!(v.clan(), e.state.minions[b.index()].clan());
    assert_eq!(e.state.sect_of(childe), Some(zvtes::crypt::Sect::Anarch));
    assert_eq!((v.blood, v.locked), (0, false));
    assert_eq!(e.state.capacity_of(childe), Some(2), "1 + Potence");
    assert_eq!(e.state.discipline_level(childe, "pot"), 1);
    assert!(!e.state.seats[0].library.contains(&find_library("Potence")));
    // It must hunt, even with blood: the only action on offer, taken at
    // once (a single option), before any other minion may act.
    e.state.minions[childe.index()].blood = 1;
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, childe), 2);
    assert!(e.state.minions[childe.index()].locked);
    assert!(!e.state.minions[other.index()].locked);
    assert!(req.options.contains(&Choice::Pass));
}

// [LSJ 20020115] "Is not a vampire in the ash heap". Diablerized, its card
// goes to the ash heap as the library card it is; two of them never contest
// (non-unique).
#[test]
fn a_burned_childe_is_a_library_card_in_the_ash_heap() {
    let mut e = Engine::bare(3);
    let b = e.add_minion(SeatId(0), baron(0), 5, false);
    e.add_minion(SeatId(0), plain("", "", 0), 5, false);
    let d = e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    e.add_minion(SeatId(1), plain("", "", 2), 5, false);
    e.add_minion(SeatId(2), plain("", "", 3), 5, false);
    hand(&mut e, 0, &["Childe of the Revolution"]);
    let before = e.state.minions.len();
    pick(&mut e, play(0, 0, b));
    request(&mut e);
    let childe = MinionId(before as u32);
    assert!(e.state.card_rivals(childe).is_empty());
    // Sent to torpor; the next Methuselah's vampire diablerizes it.
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[childe.index()].in_torpor = true;
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(childe),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[childe.index()].in_play);
    assert!(
        e.state.seats[0].ash_crypt.is_empty(),
        "no vampire in the ash heap"
    );
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Childe of the Revolution")));
}

// [RTR 19990712] Moved to the uncontrolled region, it stays a vampire (its
// capacity 1) and can be influenced out again.
#[test]
fn a_banished_childe_can_be_influenced_again() {
    let mut e = Engine::bare(3);
    let b = e.add_minion(SeatId(0), baron(0), 5, false);
    e.add_minion(SeatId(0), plain("", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    hand(&mut e, 0, &["Childe of the Revolution"]);
    let before = e.state.minions.len();
    pick(&mut e, play(0, 0, b));
    request(&mut e);
    let childe = MinionId(before as u32);
    let card = e
        .state
        .cards
        .iter()
        .position(|c| c.body == Some(childe))
        .unwrap();
    // Banished (Banishment's move, by hand), with 1 blood as a counter.
    e.state.minions[childe.index()].in_play = false;
    e.state.minions[childe.index()].blood = 0;
    e.state.cards[card].in_play = false;
    e.state.seats[0].uncontrolled.push(UncontrolledSlot {
        card: zvtes::state::CREATED,
        counters: 1,
        banished: Some(zvtes::state::Banished {
            minion: childe,
            cards: vec![CardId(card as u32)],
        }),
    });
    let slot = e.state.seats[0].uncontrolled.len() as u8 - 1;
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot });
    request(&mut e);
    let v = &e.state.minions[childe.index()];
    assert!(v.in_play && v.is_vampire());
    assert_eq!((v.blood, e.state.capacity_of(childe)), (1, Some(1)));
    assert!(e.state.cards[card].in_play);
}

/// An ability of the card in play `source`, with its target.
fn use_on(source: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(source),
        ability,
        actor,
        target,
    }
}

// Arcane Library: "Unique location. You can lock this card during your
// influence phase to add 1 blood to a Tremere in your uncontrolled region."
#[test]
fn arcane_library_adds_blood_to_an_uncontrolled_tremere_once() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.state.turn = 4;
    let tremere = uncontrolled(&mut e, 0, vampire(|c| c.clan == "Tremere", 0));
    let other = uncontrolled(&mut e, 0, vampire(|c| c.clan == "Brujah", 0));
    let lib = location(&mut e, 0, "Arcane Library");
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let add = |slot| use_on(lib, 0, None, Target::Uncontrolled(slot));
    assert!(!req.options.contains(&add(other)), "not a Tremere");
    pick(&mut e, add(tremere));
    let req = request(&mut e);
    assert_eq!(e.state.seats[0].uncontrolled[tremere as usize].counters, 1);
    assert!(e.state.cards[lib.index()].locked);
    assert!(!req.options.contains(&add(tremere)), "locked");
    // The transfers are all still there.
    let transfers = req
        .options
        .iter()
        .filter(|o| matches!(o, Choice::Transfer { .. }))
        .count();
    assert_eq!(transfers, 2);
}

// Wasserschloss Anif, Austria: "During your master phase, a Tremere you
// control can move 1 blood from themselves to this card. Lock during your
// influence phase to move all counters on this card to a Tremere in your
// uncontrolled region." [RTR 19960221] "Can only receive blood from one
// Tremere on any given turn."
#[test]
fn wasserschloss_anif_banks_one_tremeres_blood_for_the_uncontrolled_region() {
    let mut e = Engine::bare(3);
    let t1 = e.add_minion(SeatId(0), plain("Tremere", "", 0), 4, false);
    // In torpor: "a Tremere you control", not a ready one (rules-qa 28.6).
    let t2 = e.add_minion(SeatId(0), plain("Tremere", "", 1), 4, true);
    e.add_minion(SeatId(0), plain("", "", 2), 4, false);
    e.state.turn = 4;
    let slot = uncontrolled(
        &mut e,
        0,
        vampire(|c| c.clan == "Tremere" && c.capacity >= 4, 2),
    );
    let w = location(&mut e, 0, "Wasserschloss Anif, Austria");
    e.state.cards[w.index()].counters = 2;
    e.at_master_phase();
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let give = |m| use_on(w, 0, None, Target::Minion(m));
    assert!(req.options.contains(&give(t2)));
    pick(&mut e, give(t1));
    let req = request(&mut e);
    assert_eq!((blood(&e, t1), e.state.cards[w.index()].counters), (3, 3));
    assert!(!req.options.contains(&give(t2)), "one Tremere a turn");
    // All the counters, over capacity too (rules-qa 28.3).
    let cap = e.state.slot_vampire(SeatId(0), slot as usize).0;
    e.state.cards[w.index()].counters = cap + 2;
    pick_when(&mut e, use_on(w, 1, None, Target::Uncontrolled(slot)));
    request(&mut e);
    assert_eq!(
        e.state.seats[0].uncontrolled[slot as usize].counters,
        cap + 2
    );
    assert_eq!(e.state.cards[w.index()].counters, 0);
}

// "Any minion can burn this card as a Ⓓ action; Malkavians get +1 stealth
// during that action."
#[test]
fn a_malkavian_burns_wasserschloss_anif_with_one_stealth() {
    for (clan, stealth) in [("Malkavian", 1), ("Brujah", 0)] {
        let mut e = Engine::bare(3);
        let m = e.add_minion(SeatId(0), plain(clan, "", 0), 4, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
        e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
        let w = location(&mut e, 1, "Wasserschloss Anif, Austria");
        pick(&mut e, use_on(w, 2, Some(m), Target::None));
        // As announced, before any window runs.
        let a = e.action().expect("the burn action");
        assert_eq!(a.stealth(&e.state), stealth, "{clan}");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.cards[w.index()].in_play, "{clan}");
    }
}

// Wider View: "Trifle. Put this card in play. You can use 1 transfer to
// draw 1 card from your crypt and then remove a crypt card in your
// uncontrolled region from the game. You can use 4 transfers to burn this
// card and gain 2 pool." [LSJ 20080314] "Cards removed from the game are
// public."
#[test]
fn wider_view_trades_transfers_for_crypt_cards_or_pool() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    e.add_minion(SeatId(2), plain("", "", 2), 4, false);
    e.state.turn = 4;
    let keep = vampire(|c| c.clan == "Brujah", 0);
    let drawn = vampire(|c| c.clan == "Ventrue", 0);
    uncontrolled(&mut e, 0, keep);
    e.state.seats[0].crypt = vec![drawn];
    let w = location(&mut e, 0, "Wider View");
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, use_on(w, 0, None, Target::None));
    // Two crypt cards now: which one goes.
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Slot(0), Choice::Slot(1)]);
    pick(&mut e, Choice::Slot(0));
    let req = request(&mut e);
    assert_eq!(e.state.seats[0].removed_crypt, vec![keep]);
    assert_eq!(e.state.seats[0].uncontrolled[0].card, drawn);
    // An empty crypt: no draw. 3 transfers left: not 4.
    assert!(!req.options.contains(&use_on(w, 0, None, Target::None)));
    assert!(!req.options.contains(&use_on(w, 1, None, Target::None)));
    // Next turn, all 4 of them: burn it for 2 pool.
    let pool0 = pool(&e, 0);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, use_on(w, 1, None, Target::None));
    request(&mut e);
    assert!(!e.state.cards[w.index()].in_play);
    assert_eq!(pool(&e, 0), pool0 + 2);
}

// Family Gathering: "Trifle. Reveal the top card of your crypt. If it is a
// Hecata, draw it and add 1 blood to it; otherwise, move it to the bottom
// of your crypt." [RTR 20000501] [LSJ 20070411-2] Not playable on an empty
// crypt.
#[test]
fn family_gathering_draws_a_hecata_or_bottoms_the_card() {
    let hecata = vampire(|c| c.clan == "Hecata", 0);
    let other = vampire(|c| c.clan == "Brujah", 0);
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Hecata", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    hand(&mut e, 0, &["Family Gathering", "Family Gathering"]);
    e.state.seats[0].crypt = vec![other, hecata];
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::None));
    request(&mut e);
    let slot = e.state.seats[0].uncontrolled.last().unwrap();
    assert_eq!((slot.card, slot.counters), (hecata, 1));
    // A trifle: the second one, with the master phase action.
    pick(&mut e, play_at(0, 0, None, Target::None));
    request(&mut e);
    assert_eq!(e.state.seats[0].crypt, vec![other], "to the bottom");
    // An empty crypt: not playable.
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Hecata", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    hand(&mut e, 0, &["Family Gathering", "Misdirection"]);
    e.at_master_phase();
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(!req.options.contains(&play_at(0, 0, None, Target::None)));
}

// Powerbase: Montreal: "Unique location. During your influence phase, you
// can add 1 blood to a vampire in your uncontrolled region. Vampires can
// steal this location as a Ⓓ action."
#[test]
fn powerbase_montreal_feeds_the_uncontrolled_region_and_can_be_stolen() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(1), plain("", "", 3), 4, false);
    e.add_minion(SeatId(2), plain("", "", 4), 4, false);
    e.state.turn = 4;
    let slot = uncontrolled(&mut e, 0, vampire(|c| c.capacity >= 4, 0));
    let pb = location(&mut e, 0, "Powerbase: Montreal");
    // Not by its controller's own vampire.
    let req = request(&mut e);
    assert!(!req.options.contains(&use_on(pb, 1, Some(a), Target::None)));
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let add = use_on(pb, 0, None, Target::Uncontrolled(slot));
    pick(&mut e, add);
    let req = request(&mut e);
    assert_eq!(e.state.seats[0].uncontrolled[slot as usize].counters, 1);
    assert!(!req.options.contains(&add), "once each influence phase");
    // Seat 1's vampire steals it.
    pick_when(&mut e, use_on(pb, 1, Some(x), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.card_controller(pb), SeatId(1));
}

// The Rack: "As this location is played or its controller changes, its
// controller chooses a ready vampire they control. During this location's
// controller's unlock phase, the chosen vampire can gain 2 blood. Vampires
// controlled by another Methuselah can steal this location as a Ⓓ action."
// [PIB 20150522] [LSJ 20010623] In torpor it gains blood; uncontrolled
// (banished) it does not. [ANK 20200130-2] Stolen, its new controller
// decides.
#[test]
fn the_rack_feeds_its_chosen_vampire_and_rechooses_when_stolen() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 2, false);
    let b = e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let x = e.add_minion(SeatId(1), plain("", "", 2), 2, false);
    let y = e.add_minion(SeatId(1), plain("", "", 3), 2, false);
    e.add_minion(SeatId(2), plain("", "", 4), 4, false);
    hand(&mut e, 0, &["Rack, The"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(a)));
    request(&mut e);
    let rack = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[rack.index()].chosen, vec![a]);
    // Seat 0's next unlock phase: a, in torpor, gains 2.
    e.state.minions[a.index()].in_torpor = true;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let feed = use_on(rack, 0, None, Target::None);
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick(&mut e, feed);
    request(&mut e);
    assert_eq!(blood(&e, a), 4);
    // Seat 1 steals it and chooses between its two vampires.
    e.state.minions[a.index()].in_torpor = false;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, use_on(rack, 1, Some(x), Target::None));
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    // Ready, locked or not: the stealing vampire too.
    assert_eq!(
        req.options,
        vec![Choice::ChooseMinion(x), Choice::ChooseMinion(y)]
    );
    pick(&mut e, Choice::ChooseMinion(y));
    request(&mut e);
    assert_eq!(e.state.card_controller(rack), SeatId(1));
    assert_eq!(e.state.cards[rack.index()].chosen, vec![y]);
    let _ = b;
}

// [LSJ 20010809-2] [LSJ 20010809-3] The Rack "tracks the target if it
// leaves the game by contesting or Banishment: if the target gets back into
// play, The Rack gives it blood again." [TOM 19960122] "Will not target a
// new copy of the same vampire if the original is burned." [RTR 19960112]
// No new target for falling out of contention. [PIB 20150522] [LSJ 20010623] No blood
// while it is uncontrolled.
#[test]
fn the_rack_tracks_its_vampire_out_of_play_and_back() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 2, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    hand(&mut e, 0, &["Rack, The"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(a)));
    request(&mut e);
    let rack = CardId(e.state.cards.len() as u32 - 1);
    let feed = use_on(rack, 0, None, Target::None);
    // Banished: out of play, no blood; the choice stays.
    let card = e.state.minions[a.index()].crypt_card().unwrap();
    e.state.minions[a.index()].in_play = false;
    e.state.seats[0].uncontrolled.push(UncontrolledSlot {
        card,
        counters: 2,
        banished: Some(zvtes::state::Banished {
            minion: a,
            cards: vec![],
        }),
    });
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, feed);
    assert_eq!(e.state.cards[rack.index()].chosen, vec![a]);
    // Back in play: blood again at the next unlock phase.
    e.state.minions[a.index()].in_play = true;
    e.state.minions[a.index()].blood = 2;
    e.state.seats[0].uncontrolled.pop();
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, feed);
    assert_eq!(blood(&e, a), 4);
    // Contested, then out of contention: still its choice, no new one.
    e.state.minions[a.index()].contested = true;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].contested);
    assert_eq!(e.state.cards[rack.index()].chosen, vec![a]);
    // Burned: gone for good — a new copy is a new vampire.
    e.state.minions[a.index()].in_play = false;
    let copy = e.add_minion(SeatId(0), card, 2, false);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, feed);
    assert_eq!(blood(&e, copy), 2);
}

// Anarch Troublemaker: "Unique. Put this card in play. During your unlock
// phase, you can give your prey control of the Anarch Troublemaker and
// either lock up to two vampires they control or burn an equipment on one
// of their minions." [PIB 20121031] "Can lock a vampire that is already
// locked."
#[test]
fn anarch_troublemaker_passes_to_the_prey_locking_or_burning() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let y = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 3), 4, false);
    e.add_minion(SeatId(2), plain("", "", 4), 4, false);
    let gun = equip(&mut e, ".44 Magnum", z);
    let t = location(&mut e, 0, "Anarch Troublemaker");
    e.state.minions[x.index()].locked = true;
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&use_on(t, 1, None, Target::Card(gun))));
    pick(&mut e, use_on(t, 0, None, Target::None));
    // Up to two, a locked one too.
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&Choice::ChooseMinion(x)));
    assert!(req.options.contains(&Choice::Pass));
    pick(&mut e, Choice::ChooseMinion(x));
    pick(&mut e, Choice::ChooseMinion(y));
    request(&mut e);
    assert!(e.state.minions[y.index()].locked && !e.state.minions[z.index()].locked);
    assert_eq!(e.state.card_controller(t), SeatId(1));
    assert!(e.state.cards[gun.index()].in_play);
    // The other half, from seat 1 to seat 2 next: burn an equipment.
    let w = e.add_minion(SeatId(2), plain("", "", 5), 4, false);
    let bike = equip(&mut e, "Sport Bike", w);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick_when(&mut e, use_on(t, 1, None, Target::Card(bike)));
    request(&mut e);
    assert!(!e.state.cards[bike.index()].in_play);
    assert_eq!(e.state.card_controller(t), SeatId(2));
}

// [ANK 20200130-2] The Rack: "If the target is stolen, the new controller
// chooses if the vampire gains blood during The Rack controller's unlock
// phase."
#[test]
fn the_racks_stolen_vampire_is_fed_at_its_new_controllers_choice() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 2, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    hand(&mut e, 0, &["Rack, The"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(a)));
    request(&mut e);
    let rack = CardId(e.state.cards.len() as u32 - 1);
    let feed = use_on(rack, 0, None, Target::None);
    // Seat 1 has taken control of it.
    e.state.minions[a.index()].owner = SeatId(1);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&feed));
    assert_eq!(
        e.state.active,
        SeatId(0),
        "during the Rack controller's unlock phase"
    );
    pick(&mut e, feed);
    request(&mut e);
    assert_eq!(blood(&e, a), 4);
}

// Powerbase: Los Angeles: "During your discard phase, you can lock this
// location to get +1 discard phase action. If you use that discard phase
// action to discard a card requiring an Anarch or making a vampire Anarch,
// you can unlock a ready Anarch. Anarchs controlled by other Methuselahs
// can steal this location as a Ⓓ action." [LSJ 20050128] [LSJ 20080603] A
// card requiring a baron is a card requiring an Anarch ([LSJ 20080603]
// [LSJ 20050128]: Bait and Switch, Organized Resistance, Childe of the
// Revolution). [LSJ 20080604] The steal is an action requiring an Anarch.
#[test]
fn powerbase_los_angeles_unlocks_an_anarch_for_an_anarch_discard() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch_with(&[], &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let x = e.add_minion(SeatId(1), anarch_with(&[], &[], 1), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", "", 1), 4, false);
    e.add_minion(SeatId(2), plain("", "", 2), 4, false);
    hand(&mut e, 0, &["Soak", "Bait and Switch"]);
    let la = location(&mut e, 0, "Powerbase: Los Angeles");
    e.state.minions[a.index()].locked = true;
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, use_on(la, 0, None, Target::None));
    assert_eq!(e.state.seats[0].dpa, 2);
    // A Soak first: the normal action.
    pick(&mut e, Choice::Discard(0));
    // Then the baron card with Los Angeles's: unlock a ready Anarch.
    let k = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Bait and Switch"))
        .unwrap() as u8;
    pick(&mut e, Choice::Discard(k));
    let req2 = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req2.options, vec![Choice::Pass, Choice::ChooseMinion(a)]);
    pick(&mut e, Choice::ChooseMinion(a));
    request(&mut e);
    assert!(!e.state.minions[a.index()].locked);
    let _ = req;
    // Only Anarchs of other Methuselahs steal it: [LSJ 20080604] an action
    // requiring an Anarch.
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&use_on(la, 1, Some(x), Target::None)));
    assert!(!req.options.contains(&use_on(la, 1, Some(y), Target::None)));
}

// Garibaldi-Meucci Museum: "You can lock this location and burn 1 pool
// during your unlock phase to exchange one card from your hand for one card
// in your ash heap requiring an Anarch." [ANK 20250320] [LSJ 20080603]
// [LSJ 20050128] Only cards whose requirements name Anarch or baron: a
// card requiring a baron (Open War) is one requiring an Anarch ([LSJ
// 20050128] [LSJ 20080603]).
// [LSJ 20000222] [LSJ 20011023] Both cards are named.
#[test]
fn garibaldi_meucci_museum_trades_a_hand_card_for_an_anarch_card() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), anarch_with(&[], &[], 0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    e.add_minion(SeatId(2), plain("", "", 2), 4, false);
    hand(&mut e, 0, &["Soak", "Blur"]);
    e.state.seats[0].ash = vec![
        find_library("Open War"),
        find_library("Majesty"),
        find_library("Line Brawl"),
        find_library("Reckless Agitation"),
    ];
    let g = location(&mut e, 0, "Garibaldi-Meucci Museum");
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    let p0 = pool(&e, 0);
    pick_when(&mut e, use_on(g, 0, None, Target::None));
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    let mut want = vec![
        Choice::FindCard(find_library("Open War")),
        Choice::FindCard(find_library("Line Brawl")),
        Choice::FindCard(find_library("Reckless Agitation")),
    ];
    want.sort_by_key(|c| match c {
        Choice::FindCard(k) => *k,
        _ => 0,
    });
    assert_eq!(req.options, want, "not Majesty");
    pick(&mut e, Choice::FindCard(find_library("Open War")));
    pick(&mut e, Choice::Discard(0));
    request(&mut e);
    assert!(e.state.seats[0].hand.contains(&find_library("Open War")));
    assert!(e.state.seats[0].ash.contains(&find_library("Soak")));
    assert_eq!(pool(&e, 0), p0 - 1);
    assert!(e.state.cards[g.index()].locked);
}

// Garibaldi-Meucci Museum: "You can lock this location before range is
// determined to end a combat involving an Anarch you control and another
// Anarch." [RTR 20001020] [LSJ 20001024] End-of-combat cards can still be
// played.
#[test]
fn garibaldi_meucci_museum_ends_a_combat_between_anarchs() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch_with(&[], &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let b = e.add_minion(SeatId(1), anarch_with(&[], &[], 1), 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let g = location(&mut e, 0, "Garibaldi-Meucci Museum");
    let start = |m| e.state.capacity_of(m).unwrap().min(4);
    let (sa, sb) = (start(a), start(b));
    hand(&mut e, 0, &["Taste of Vitae"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, use_on(g, 1, None, Target::None));
    // The end of the round still comes: Taste of Vitae is playable (it
    // gains nothing, no blood lost).
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Taste of Vitae")));
    assert_eq!((blood(&e, a), blood(&e, b)), (sa, sb), "no strike");
    assert!(e.state.cards[g.index()].locked);
}

// "You can lock this card after an Anarch successfully hunts to add 1 blood
// to that Anarch." [LSJ 20050720-2] A hunt gaining no blood is not
// successful. [ANK 20220317] Not a hunt requiring an Anarch.
#[test]
fn the_anarch_free_press_adds_blood_after_a_successful_hunt_only() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch_with(&[], &[], 0), 1, false);
    let full = e.add_minion(SeatId(0), anarch_with(&[], &[], 1), 3, false);
    e.state.minions[full.index()].blood = e.state.capacity_of(full).unwrap();
    e.add_minion(SeatId(1), plain("", "", 0), 4, false);
    let press = location(&mut e, 0, "Anarch Free Press, The");
    let add = use_on(press, 1, None, Target::None);
    pick(&mut e, action(full, ActionKind::Hunt));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, add);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, add);
    request(&mut e);
    assert_eq!(blood(&e, a), 3, "1, +1 hunting, +1");
}

// Channel 10: "You can lock this card to give a minion you control +2
// intercept. Not usable during the first action in a minion phase."
// [LSJ 20100505] [ANK 20210124] An action canceled "as played" (Direct
// Intervention) was never announced: the next one is the first.
#[test]
fn channel_10_waits_for_the_second_action_of_a_minion_phase() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), baron(0), 4, false);
    let b = e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let ch = location(&mut e, 1, "Channel 10");
    hand(
        &mut e,
        0,
        &[
            "Childe of the Revolution",
            "Lost in Crowds",
            "Lost in Crowds",
        ],
    );
    hand(&mut e, 1, &["Direct Intervention"]);
    e.state.seats[1].pool = 10;
    // Childe of the Revolution, canceled as it is played.
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play_at(0, 0, None, Target::None));
    request(&mut e);
    assert_eq!(e.state.phase_actions, 0, "no action announced");
    let p1 = pool(&e, 1);
    // The first action: b bleeds at +1 stealth; Channel 10 is not usable.
    pick_when(&mut e, action(b, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    let lic = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Lost in Crowds"))
        .unwrap() as u8;
    pick(&mut e, play(lic, 0, b));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, ability(ch));
    assert_eq!(pool(&e, 1), p1 - 1, "not blocked");
    // The second: a bleeds; now it is.
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    let lic = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Lost in Crowds"))
        .unwrap() as u8;
    pick(&mut e, play(lic, 0, a));
    pick_when(&mut e, ability(ch));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), p1 - 1, "blocked");
}

// Creepshow Casino: "You can lock this card as a vampire you control
// announces an undirected action to give that vampire +1 stealth, even if
// stealth is not yet needed."
#[test]
fn creepshow_casino_gives_stealth_to_an_undirected_action_as_announced() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 2, false);
    let b = e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let casino = location(&mut e, 0, "Creepshow Casino");
    // A bleed is directed: not offered.
    pick(&mut e, action(b, ActionKind::Bleed));
    let req = request(&mut e);
    assert!(!req.options.contains(&ability(casino)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, action(a, ActionKind::Hunt));
    let base = e.action().unwrap().stealth(&e.state);
    pick(&mut e, ability(casino));
    assert_eq!(e.action().unwrap().stealth(&e.state), base + 1);
}

// Club Illusion: "An Anarch can burn 1 blood once as they announce a bleed
// action to get +1 bleed during that action." [ANK 20220317] Not an action
// requiring an Anarch. Any Anarch: the location is another Methuselah's.
#[test]
fn club_illusion_lets_any_anarch_burn_blood_for_one_bleed_once() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch_with(&[], &[], 0), 3, false);
    let b = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let club = location(&mut e, 2, "Club Illusion");
    let burn = use_on(club, 0, Some(a), Target::None);
    pick(&mut e, action(b, ActionKind::Bleed));
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::UseAbility { .. })),
        "not an Anarch"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let p1 = pool(&e, 1);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, burn);
    let req = request(&mut e);
    assert!(!req.options.contains(&burn), "once");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), p1 - 2);
    assert_eq!(blood(&e, a), e.state.capacity_of(a).unwrap().min(3) - 1);
}

// Haqim's Law: Retribution: "During a bleed action, an Assamite you
// control can discard a combat card to get +1 bleed." [ANK 20221102-2] Once
// per action for each copy. [ANK 20220502] Only by the bleeding minion.
// [ANK 20231124] [LSJ 19970718] Before action resolution.
#[test]
fn haqims_law_retribution_trades_a_combat_card_for_one_bleed_per_copy() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Banu Haqim", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let law1 = location(&mut e, 0, "Haqim's Law: Retribution");
    let law2 = location(&mut e, 0, "Haqim's Law: Retribution");
    hand(&mut e, 0, &["Lost in Crowds", "Side Strike", "Roundhouse"]);
    let p1 = pool(&e, 1);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, use_on(law1, 0, Some(a), Target::None));
    // The combat card to discard (Lost in Crowds is not one).
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::Discard(0)));
    pick(&mut e, Choice::Discard(1));
    let req = request(&mut e);
    assert!(
        !req.options
            .contains(&use_on(law1, 0, Some(a), Target::None)),
        "once"
    );
    pick(&mut e, use_on(law2, 0, Some(a), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), p1 - 3);
}

// Haqim's Law: Retribution: "an Assamite you control can discard a combat
// card to get +1 bleed" — read live on the bleeding minion: a Brujah is
// not offered it (a regression: a clan requirement of a mode was once
// taken as always met).
#[test]
fn haqims_law_retribution_is_not_offered_to_another_clan() {
    for (clan, offered) in [("Brujah", false), ("Banu Haqim", true)] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain(clan, "", 0), 4, false);
        e.add_minion(SeatId(0), plain("", "", 1), 4, false);
        e.add_minion(SeatId(1), plain("", "", 2), 4, false);
        let law = location(&mut e, 0, "Haqim's Law: Retribution");
        hand(&mut e, 0, &["Lost in Crowds", "Side Strike", "Roundhouse"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        let used = offered_before(
            &mut e,
            use_on(law, 0, Some(a), Target::None),
            SeatId(0),
            WindowLabel::MinionPhase,
        );
        assert_eq!(used, offered, "{clan}");
    }
}

// Pentex™ Subversion: "Put this card on a ready minion. This minion cannot
// block. Other minions can burn this card as a Ⓓ action."
#[test]
fn pentex_subversion_bars_blocking_until_another_minion_burns_it() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let pentex = equip(&mut e, "Pentex(TM) Subversion", z);
    e.state.cards[pentex.index()].owner = SeatId(0);
    e.state.cards[pentex.index()].controller = SeatId(0);
    for effect in cards::lookup(find_library("Pentex(TM) Subversion"))
        .unwrap()
        .modes[0]
        .effects
    {
        if let Effect::Layer(modifier) = effect {
            e.state.layers.push(Layer {
                scope: Scope::InPlay,
                modifier: *modifier,
                source: pentex,
            });
        }
    }
    let p1 = pool(&e, 1);
    pick(&mut e, action(a, ActionKind::Bleed));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::DeclareBlock { minion: z },
    );
    assert_eq!(pool(&e, 1), p1 - 1);
    // Seat 1's turn: z itself cannot burn it; another minion can.
    let y = e.add_minion(SeatId(1), plain("", "", 3), 4, false);
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!req
        .options
        .contains(&use_on(pentex, 0, Some(z), Target::None)));
    pick(&mut e, use_on(pentex, 0, Some(y), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[pentex.index()].in_play);
}

/// `name`, a master card of `seat`'s, in play on `host` with its layers.
fn master_on(e: &mut Engine, seat: u8, name: &str, host: MinionId) -> CardId {
    let id = location(e, seat, name);
    e.state.cards[id.index()].host = Some(host);
    id
}

// Haven Uncovered: "Put this card on a ready vampire. Minions can enter
// combat with this vampire as a +1 stealth Ⓓ action. This vampire can burn
// this card as a +1 stealth Ⓓ action."
#[test]
fn haven_uncovered_opens_its_vampire_to_combat_until_it_burns_it() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let a2 = e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(1), plain("", "", 3), 4, false);
    let haven = master_on(&mut e, 0, "Haven Uncovered", z);
    let rush = |m| use_on(haven, 0, Some(m), Target::Minion(z));
    pick(&mut e, rush(a));
    assert_eq!(e.action().unwrap().stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].blood < 4, "the combat happened");
    let _ = a2;
    // z's turn: it burns the card.
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req
        .options
        .contains(&use_on(haven, 1, Some(z), Target::None)));
    pick(&mut e, use_on(haven, 1, Some(z), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[haven.index()].in_play);
}

// Chantry: "You can lock this card and burn 1 pool or 1 blood from a ready
// Tremere you control during your master phase to move a Tremere from
// torpor to their controller's ready region."
#[test]
fn chantry_brings_a_tremere_out_of_torpor_for_pool_or_blood() {
    for blood_cost in [false, true] {
        let mut e = Engine::bare(3);
        let t = e.add_minion(SeatId(0), plain("Tremere", "", 0), 3, false);
        e.add_minion(SeatId(1), plain("", "", 0), 4, false);
        let torpid = e.add_minion(SeatId(1), plain("Tremere", "", 1), 1, true);
        let ch = location(&mut e, 0, "Chantry");
        let p0 = pool(&e, 0);
        e.at_master_phase();
        pick(
            &mut e,
            use_on(ch, u8::from(blood_cost), None, Target::Minion(torpid)),
        );
        request(&mut e);
        assert!(
            !e.state.minions[torpid.index()].in_torpor,
            "another's Tremere too"
        );
        if blood_cost {
            assert_eq!((blood(&e, t), pool(&e, 0)), (2, p0));
        } else {
            assert_eq!((blood(&e, t), pool(&e, 0)), (3, p0 - 1));
        }
    }
}

// Toreador Grand Ball: "Choose two ready Toreador you control, put this
// card in play, and lock one of the two. The locked Toreador does not
// unlock as normal. The other Toreador's non-bleed actions cannot be
// blocked." [LSJ 19970814] Still after it bled. [TOM 19960528] The same
// vampire can be the second Toreador of several Grand Balls.
#[test]
fn toreador_grand_ball_locks_one_toreador_and_frees_the_others_actions() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Toreador", "", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Toreador", "", 1), 4, false);
    // Only two untitled Toreador of capacity 5+ lack printed text:
    // Anne-Marie's (equipment costs her 1 pool less) is inert here.
    let c = e.add_minion(
        SeatId(0),
        crypt("Anne-Marie Bourgeois, Inconnu Recruiter"),
        4,
        false,
    );
    let z = e.add_minion(SeatId(1), plain("", "", 0), 4, false);
    e.add_minion(SeatId(2), plain("", "", 1), 4, false);
    hand(&mut e, 0, &["Toreador Grand Ball"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(a)));
    // Both chosen as it is played (rules-qa 28.5).
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(
        req.options,
        vec![Choice::ChooseMinion(b), Choice::ChooseMinion(c)]
    );
    pick(&mut e, Choice::ChooseMinion(b));
    request(&mut e);
    let ball = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[ball.index()].chosen, vec![a, b]);
    assert!(e.state.minions[a.index()].locked);
    // b bleeds: blockable; then hunts: not.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, action(b, ActionKind::Bleed));
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::DeclareBlock { minion: z }));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[b.index()].locked = false;
    e.state.minions[b.index()].acted.clear();
    pick(&mut e, action(b, ActionKind::Hunt));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::DeclareBlock { minion: z },
    );
    // a stays locked through seat 0's next unlock phase.
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].locked);
}

// "Minions can burn this card as a Ⓓ action; Nosferatu get -1 stealth
// during that action."
#[test]
fn a_nosferatu_burns_toreador_grand_ball_at_minus_one_stealth() {
    // At 0 stealth against 0 intercept the block succeeds; at 1 it fails.
    for (clan, burned) in [("Nosferatu", false), ("Malkavian", true)] {
        let mut e = Engine::bare(3);
        let n = e.add_minion(SeatId(0), plain(clan, "obf", 0), 4, false);
        e.add_minion(SeatId(0), plain("", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
        let ball = location(&mut e, 1, "Toreador Grand Ball");
        hand(&mut e, 0, &["Lost in Crowds"]);
        pick(&mut e, use_on(ball, 0, Some(n), Target::None));
        pick(&mut e, Choice::DeclareBlock { minion: z });
        pick(&mut e, play(0, 0, n)); // +1 stealth
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(!e.state.cards[ball.index()].in_play, burned, "{clan}");
    }
}

// Priority Contract: [LSJ 20080604] its action requires the Assamite. "Choose
// a ready Assamite you control and put this card on a minion controlled by
// your prey. The chosen Assamite can enter
// combat with the attached minion as a +1 stealth Ⓓ action. If the attached
// minion is about to leave the ready region, you can burn this card to gain
// 3 pool." [PIB 20150512] Even when the card goes with it.
#[test]
fn priority_contract_marks_a_prey_minion_for_the_chosen_assamite() {
    let mut e = Engine::bare(3);
    let haqim = |n| vampire(|c| c.clan == "Banu Haqim" && c.title == Title::None, n);
    let h1 = e.add_minion(SeatId(0), haqim(0), 3, false);
    let h2 = e.add_minion(SeatId(0), haqim(1), 3, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    hand(&mut e, 0, &["Priority Contract"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(z)));
    // Chosen as it is played (rules-qa 28.5).
    expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    pick(&mut e, Choice::ChooseMinion(h2));
    request(&mut e);
    let pc = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[pc.index()].chosen, vec![h2]);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let rush = |m| use_on(pc, 0, Some(m), Target::Minion(z));
    assert!(!req.options.contains(&rush(h1)), "the chosen one only");
    pick(&mut e, rush(h2));
    assert_eq!(e.action().unwrap().stealth(&e.state), 1);
    // z goes to torpor in the combat: the contract pays.
    e.state.minions[z.index()].blood = 0;
    let p0 = pool(&e, 0);
    pick_when(&mut e, Choice::ResolveRider { card: pc.0 });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(pool(&e, 0), p0 + 3);
    assert!(!e.state.cards[pc.index()].in_play);
}

// [ANK 20180129-2] Priority Contract: "Can be burned to get 3 pool when a
// minion leaves the ready region because his controller is ousted."
#[test]
fn priority_contract_pays_when_the_prey_is_ousted() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Banu Haqim", "", 0), 4, false);
    let a = e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    let pc = master_on(&mut e, 0, "Priority Contract", z);
    e.state.seats[1].pool = 1;
    let p0 = pool(&e, 0);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, Choice::ResolveRider { card: pc.0 });
    request(&mut e);
    assert!(e.state.seats[1].ousted);
    assert_eq!(pool(&e, 0), p0 + 6 + 3, "the oust's 6 and the contract's 3");
}

// Frontal Assault: "Each ready minion you control can enter combat with a
// minion controlled by your prey as a Ⓓ action. You gain 1 pool after a
// ready minion controlled by your prey is burned or sent to torpor. During
// your influence phase, burn this card and burn 1 pool for each ready
// minion controlled by your prey." [LSJ 20061018] Copies stack.
#[test]
fn frontal_assault_rushes_the_prey_pays_for_losses_and_costs_at_influence() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 0, false);
    e.add_minion(SeatId(1), plain("", "", 3), 4, false);
    e.add_minion(SeatId(1), plain("", "", 4), 4, false);
    e.add_minion(SeatId(2), plain("", "", 5), 4, false);
    let f1 = location(&mut e, 0, "Frontal Assault");
    let f2 = location(&mut e, 0, "Frontal Assault");
    let p0 = pool(&e, 0);
    pick(&mut e, use_on(f1, 0, Some(a), Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(pool(&e, 0), p0 + 2, "1 for each copy");
    // The influence phase: both burn, 2 ready prey minions each — when the
    // player chooses, transfers first if they like, never passing before
    // (rules-qa 28.9).
    e.state.seats[0].uncontrolled.push(UncontrolledSlot {
        card: e.state.minions[a.index()].crypt_card().unwrap(),
        counters: 0,
        banished: None,
    });
    pick(&mut e, Choice::Pass);
    let req = expect(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert!(!req.options.contains(&Choice::Pass));
    assert!(req.options.contains(&Choice::ResolveRider { card: f2.0 }));
    // A transfer first.
    pick(&mut e, Choice::Transfer { slot: 0 });
    // The other one, then, is the only option left (elided).
    pick(&mut e, Choice::ResolveRider { card: f1.0 });
    request(&mut e);
    assert!(!e.state.cards[f1.index()].in_play && !e.state.cards[f2.index()].in_play);
    assert_eq!(pool(&e, 0), p0 + 2 - 1 - 4);
}

// Piper: "Requires a ready Anarch. Lock a ready unlocked Anarch you
// control. That Anarch recruits or employs an ally or retainer from your
// hand (requirements and cost apply as normal). This is not an action and
// cannot be blocked." [RBK recruit-ally] The ally cannot act this turn.
// [LSJ 20100725] [LSJ 20100303] [ANK 20180817] The requirements apply.
#[test]
fn piper_has_an_anarch_recruit_an_ally_outside_an_action() {
    let mut e = Engine::bare(3);
    let brujah = vampire(
        |c| c.sect == zvtes::crypt::Sect::Anarch && c.clan == "Brujah" && c.title == Title::None,
        0,
    );
    let a = e.add_minion(SeatId(0), brujah, 3, false);
    let other = e.add_minion(
        SeatId(0),
        vampire(|c| c.sect != zvtes::crypt::Sect::Anarch, 0),
        3,
        false,
    );
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    hand(
        &mut e,
        0,
        &[
            "Piper",
            "47th Street Royals",
            "Political Ally",
            "Crypt's Sons",
        ],
    );
    e.at_master_phase();
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(
        !req.options.contains(&play(0, 0, other)),
        "requires an Anarch"
    );
    pick(&mut e, play(0, 0, a));
    // The Ventrue ally is not his to recruit.
    let royals = find_library("47th Street Royals");
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(req.options.contains(&Choice::EmployFromHand {
        card: royals,
        mode: 0
    }));
    let sons = find_library("Crypt's Sons");
    assert!(req.options.contains(&Choice::EmployFromHand {
        card: sons,
        mode: 0
    }));
    assert!(!req.options.contains(&Choice::EmployFromHand {
        card: find_library("Political Ally"),
        mode: 0
    }));
    pick(
        &mut e,
        Choice::EmployFromHand {
            card: royals,
            mode: 0,
        },
    );
    request(&mut e);
    assert!(e.state.minions[a.index()].locked);
    let ally = MinionId(e.state.minions.len() as u32 - 1);
    assert!(e.state.minions[ally.index()].is_ally());
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !req.options.contains(&action(ally, ActionKind::Bleed)),
        "recruited this turn"
    );
}

// [LSJ 20080803] Piper: "When recruiting an ally or retainer whose effect
// depend on Discipline level, the chosen vampire decides in respect to his
// disciplines the normal way." [LSJ 20080815] "This does not count as
// performing the action to employ … The same vampire can play the
// respective card in a normal fashion during their turn, if unlocked."
// [RTR 20001020] Direct Intervention cannot cancel the retainer: it is not
// played from the hand as an action card.
#[test]
fn piper_employs_by_the_anarchs_disciplines_and_is_no_action() {
    let mut e = Engine::bare(3);
    let superior_ani = vampire(
        |c| {
            c.sect == zvtes::crypt::Sect::Anarch
                && c.title == Title::None
                && c.disciplines.contains(&"ANI")
        },
        0,
    );
    let a = e.add_minion(SeatId(0), superior_ani, 4, false);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    hand(&mut e, 0, &["Piper", "Raven Spy", "Raven Spy"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    e.state.minions[a.index()].blood = e.state.capacity_of(a).unwrap().min(4);
    let has_ani = e.state.discipline_level(a, "ani") == 2;
    assert!(has_ani);
    e.at_master_phase();
    pick(&mut e, play(0, 0, a));
    let spy = find_library("Raven Spy");
    // Its superior version, by its Animalism.
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    let superior = Choice::EmployFromHand { card: spy, mode: 1 };
    assert_eq!(req.options.contains(&superior), has_ani);
    pick(
        &mut e,
        if has_ani {
            superior
        } else {
            Choice::EmployFromHand { card: spy, mode: 0 }
        },
    );
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play_at(0, 0, None, Target::None),
    );
    let spy_id = e
        .state
        .cards
        .iter()
        .position(|c| c.card == spy && c.host == Some(a))
        .unwrap();
    assert_eq!(e.state.cards[spy_id].counters, if has_ani { 2 } else { 1 });
    // Unlocked, it may still employ one normally.
    e.state.minions[a.index()].locked = false;
    let k = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == spy)
        .unwrap() as u8;
    let req = request(&mut e);
    assert!(plays_by(&req, a).iter().any(|&(c, _)| c == k));
}

// [TOM 19960528] Toreador Grand Ball: "The same vampire can serve as the
// 'second Toreador' for multiple Toreador Grand Balls."
#[test]
fn a_toreador_can_be_the_second_of_two_grand_balls() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Toreador", "", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Toreador", "", 1), 4, false);
    // Only two untitled Toreador of capacity 5+ lack printed text:
    // Anne-Marie's (equipment costs her 1 pool less) is inert here.
    let c = e.add_minion(
        SeatId(0),
        crypt("Anne-Marie Bourgeois, Inconnu Recruiter"),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("", "", 0), 4, false);
    let first = location(&mut e, 0, "Toreador Grand Ball");
    e.state.cards[first.index()].chosen = vec![a, b];
    hand(&mut e, 0, &["Toreador Grand Ball"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(c)));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(req.options.contains(&Choice::ChooseMinion(b)));
}

// [RBK contested-cards] Priority Contract: "You still gain the trifle master
// phase action, even if this card contests when played."
#[test]
fn a_contested_priority_contract_still_gives_the_trifle_action() {
    let mut e = Engine::bare(3);
    e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Banu Haqim" && c.title == Title::None, 0),
        3,
        false,
    );
    let z = e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let y = e.add_minion(SeatId(2), plain("", "", 2), 4, false);
    // Seat 2's copy already in play, on seat 0's... any minion.
    master_on(&mut e, 2, "Priority Contract", y);
    hand(&mut e, 0, &["Priority Contract", "Misdirection"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(z)));
    let req = request(&mut e);
    assert!(
        req.options.iter().any(|o| matches!(o, Choice::Play { .. })),
        "a master phase action left"
    );
}

// Warsaw Station: "You can lock this card as a Nosferatu announces an
// undirected action; after action resolution, if that action was
// successful, unlock the acting Nosferatu. You can burn this card (even if
// it is locked) to move a Nosferatu in torpor to the ready region."
// [ANK 20250120] The unlock applies even if Warsaw Station is not in play
// when the action resolves.
#[test]
fn warsaw_station_unlocks_a_successful_nosferatu_even_once_burned() {
    let mut e = Engine::bare(3);
    let n = e.add_minion(SeatId(0), plain("Nosferatu", "", 0), 2, false);
    let torpid = e.add_minion(SeatId(0), plain("Nosferatu", "", 1), 2, true);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let w = location(&mut e, 0, "Warsaw Station");
    pick(&mut e, action(n, ActionKind::Hunt));
    pick(&mut e, use_on(w, 0, None, Target::None));
    // Burned, locked as it is, to raise the other Nosferatu.
    pick_when(&mut e, use_on(w, 1, None, Target::Minion(torpid)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[w.index()].in_play);
    assert!(!e.state.minions[torpid.index()].in_torpor);
    assert!(
        !e.state.minions[n.index()].locked,
        "unlocked after the hunt"
    );
    assert_eq!(blood(&e, n), 3);
}

// [ANK 20220218] [ANK 20181107] Warsaw Station "can be burned after the
// announcement of a diablerie action to bring the target Nosferatu out of
// torpor. The action will still be successful if unblocked, but will have
// no effect: no diablerie happen."
#[test]
fn warsaw_station_saves_a_nosferatu_from_a_diablerie() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let victim = e.add_minion(SeatId(1), plain("Nosferatu", "", 0), 1, true);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let w = location(&mut e, 1, "Warsaw Station");
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    pick_when(&mut e, use_on(w, 1, None, Target::Minion(victim)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[victim.index()];
    assert!(v.in_play && !v.in_torpor, "no diablerie");
    assert!(e.state.minions[a.index()].locked);
}

// [LSJ 20090216] [ANK 20180922] Warsaw Station: "The unlock effect is
// applied after the action is resolved and the referendum (if any)
// concluded."
#[test]
fn warsaw_station_unlocks_after_the_referendum() {
    let mut e = Engine::bare(3);
    let n = e.add_minion(SeatId(0), plain("Nosferatu", "", 0), 3, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    e.add_minion(SeatId(2), plain("", "", 2), 4, false);
    let w = location(&mut e, 0, "Warsaw Station");
    hand(&mut e, 0, &["Anarchist Uprising"]);
    pick(&mut e, play(0, 0, n));
    pick(&mut e, use_on(w, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.action().is_none());
    assert!(!e.state.minions[n.index()].locked);
}

// [ANK 20220218] "When the acting vampire leaves torpor before the action
// resolves (e.g. Warsaw Station) the action immediately fizzles (cost, if
// any, is paid, and it is considered successful, but has no effect)."
#[test]
fn a_leave_torpor_action_fizzles_once_warsaw_station_raised_its_vampire() {
    let mut e = Engine::bare(3);
    let n = e.add_minion(SeatId(0), plain("Nosferatu", "", 0), 3, true);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let w = location(&mut e, 0, "Warsaw Station");
    pick(&mut e, action(n, ActionKind::LeaveTorpor));
    pick_when(&mut e, use_on(w, 1, None, Target::Minion(n)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[n.index()];
    assert!(!v.in_torpor);
    assert_eq!(v.blood, 1, "its cost paid");
}

// Rules-qa 28.7 (owner, 2026-09-26): a rescue whose vampire left torpor
// before it resolves fizzles as the leave-torpor action does — its cost
// paid (ANK 20220218), no effect.
#[test]
fn a_rescue_fizzles_paid_once_warsaw_station_raised_its_vampire() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    // Another minion to act: the phase stops right after the rescue.
    e.add_minion(SeatId(0), plain("", "", 3), 4, false);
    let n = e.add_minion(SeatId(0), plain("Nosferatu", "", 1), 1, true);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let w = location(&mut e, 0, "Warsaw Station");
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Rescue,
            target: Some(n),
        },
    );
    // The rescuer pays it all, chosen as it is announced (rules-qa 48.8):
    // the rescued keeps its blood (no hunt).
    pick(&mut e, Choice::PayRescue { from_acting: 2 });
    pick_when(&mut e, use_on(w, 1, None, Target::Minion(n)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert!(!e.state.minions[n.index()].in_torpor);
    assert_eq!((blood(&e, a), blood(&e, n)), (2, 1), "its cost paid");
}

// Rules-qa 28.7, ANK 20220218: a rescue whose vampire is no longer in play
// as it resolves fizzles, its cost paid by the rescuer. Here the vampire's
// Methuselah pays their last pool for Direct Intervention (canceling The
// Sleeping Mind) and is ousted mid-action.
#[test]
fn a_rescue_fizzles_paid_once_its_vampire_left_play() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "dom", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 3), 4, false);
    let n = e.add_minion(SeatId(1), plain("", "", 1), 1, true);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.state.minions[z.index()].locked = true;
    e.add_minion(SeatId(2), plain("", "", 4), 4, false);
    e.state.seats[1].pool = 1;
    hand(&mut e, 0, &["Sleeping Mind, The"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Rescue,
            target: Some(n),
        },
    );
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(z)));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    let intervene = play_at(0, 0, None, Target::None);
    assert!(req.options.contains(&intervene), "{:?}", req.options);
    pick(&mut e, intervene);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert!(!e.state.minions[n.index()].in_play);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(blood(&e, a), 2, "its cost paid");
}

// Anarch Railroad: "You can lock this card to give an Anarch +1 stealth."
// Any Methuselah's Anarch: a third party's location helps the acting one,
// holding the impulse in turn.
#[test]
fn anarch_railroad_gives_any_acting_anarch_stealth() {
    let mut e = Engine::bare(4);
    let a = e.add_minion(SeatId(0), anarch_with(&[], &[], 0), 3, false);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    e.add_minion(SeatId(2), plain("", "", 2), 4, false);
    e.add_minion(SeatId(3), plain("", "", 3), 4, false);
    let rr = location(&mut e, 2, "Anarch Railroad");
    let p1 = pool(&e, 1);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, ability(rr));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), p1 - 1, "not blocked");
    assert!(e.state.cards[rr.index()].locked);
}

// Fortune Teller Shop: "You can lock this card to give a Ravnos +1
// stealth."
#[test]
fn fortune_teller_shop_gives_a_ravnos_stealth_only() {
    for (clan, offered) in [("Ravnos", true), ("Brujah", false)] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(
            SeatId(0),
            vampire(|c| c.clan == clan && c.title == Title::None, 0),
            3,
            false,
        );
        e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("", "", 1), 4, false);
        let shop = location(&mut e, 0, "Fortune Teller Shop");
        pick(&mut e, action(a, ActionKind::Bleed));
        pick(&mut e, Choice::DeclareBlock { minion: z });
        if offered {
            pick_when(&mut e, ability(shop));
        } else {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, ability(shop));
        }
    }
}

// KRCG News Radio: "You can lock this card to give a minion you control +1
// intercept. You can lock this card and burn 1 pool to give a minion
// controlled by another Methuselah +1 intercept." [ANK 20200607]
// Sequencing: the other Methuselahs declare their attempts, then the
// impulse comes. The attempt it could make succeed is offered, not pruned.
#[test]
fn krcg_news_radio_pays_for_another_methuselahs_intercept() {
    for with_radio in [false, true] {
        let mut e = Engine::bare(4);
        let a = e.add_minion(SeatId(0), plain("", "", 0), 3, false);
        e.add_minion(SeatId(0), plain("", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
        e.add_minion(SeatId(2), plain("", "", 3), 4, false);
        e.add_minion(SeatId(3), plain("", "", 4), 4, false);
        if with_radio {
            location(&mut e, 2, "KRCG News Radio");
        }
        hand(&mut e, 0, &["Anarchist Uprising"]);
        pick(&mut e, play(0, 0, a)); // a +1 stealth political action
        let block = Choice::DeclareBlock { minion: z };
        if !with_radio {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block);
            continue;
        }
        let radio = CardId(0);
        let p2 = pool(&e, 2);
        pick_when(&mut e, block);
        pick_when(&mut e, use_on(radio, 1, None, Target::None));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 2), p2 - 1);
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    }
}

// Carver's Meat Packing and Storage: "After a vampire with capacity 3 or
// less goes to torpor, put 1 hostage counter on them. Vampires with any
// hostage counters cannot be moved to the ready region or be diablerized.
// Lock during your master phase to add X blood to a ready vampire you
// control, where X is the number of vampires with any hostage counters.
// During any unlock phase, any ready vampire can burn 2 blood to burn any
// vampire's hostage counters. After this card leaves play, burn all the
// hostage counters." [ANK 20200729] The burn once each turn.
#[test]
fn carvers_meat_packing_holds_young_vampires_hostage() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 2, false);
    let b = e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let young = vampire(|c| c.capacity <= 3 && c.title == Title::None, 0);
    let z = e.add_minion(SeatId(1), young, 0, false);
    let y = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let w = e.add_minion(
        SeatId(2),
        vampire(|c| c.capacity <= 3 && c.title == Title::None, 1),
        0,
        true,
    );
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    e.state.minions[w.index()].hostage = 1;
    let carver = location(&mut e, 0, "Carver's Meat Packing and Storage");
    bleed_blocked(&mut e, a, z);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(e.state.minions[z.index()].hostage, 1);
    // No rescue, no diablerie of it.
    let req = request(&mut e);
    let rescue = Choice::TakeAction {
        minion: b,
        action: ActionKind::Rescue,
        target: Some(z),
    };
    let diablerie = Choice::TakeAction {
        minion: b,
        action: ActionKind::Diablerie,
        target: Some(z),
    };
    assert!(!req.options.contains(&rescue) && !req.options.contains(&diablerie));
    // Seat 1's unlock phase: y burns 2 blood to free it; then no second
    // burn this turn.
    until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    let free = use_on(carver, 1, Some(y), Target::Minion(z));
    pick(&mut e, free);
    assert_eq!(e.state.minions[z.index()].hostage, 0);
    assert_eq!(blood(&e, y), e.state.capacity_of(y).unwrap().min(4) - 2);
    let again = use_on(carver, 1, Some(y), Target::Minion(w));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, again);
}

// "Lock during your master phase to add X blood to a ready vampire you
// control"; "After this card leaves play, burn all the hostage counters."
#[test]
fn carvers_meat_packing_feeds_per_hostage_and_frees_them_as_it_goes() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 1, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let h1 = e.add_minion(SeatId(1), vampire(|c| c.capacity <= 3, 0), 0, true);
    let h2 = e.add_minion(SeatId(2), vampire(|c| c.capacity <= 3, 1), 0, true);
    e.state.minions[h1.index()].hostage = 1;
    e.state.minions[h2.index()].hostage = 1;
    let carver = location(&mut e, 0, "Carver's Meat Packing and Storage");
    e.at_master_phase();
    pick(&mut e, use_on(carver, 0, None, Target::Minion(a)));
    request(&mut e);
    assert_eq!(blood(&e, a), 3);
    // Gone (contested, burned, out of play): the counters go.
    e.state.cards[carver.index()].in_play = false;
    request(&mut e);
    assert_eq!(
        e.state.minions[h1.index()].hostage + e.state.minions[h2.index()].hostage,
        0
    );
}

// Meditative Grove: "During your unlock phase, a ready Salubri you control
// can gain 1 blood. You can lock this card to cancel a frenzy card as it is
// played on a Salubri you control (cost is still paid)." [ANK 20260216]
// [RBK cancel-a-card] The cost of a canceled card is paid. [LSJ 20061207]
// [LSJ 20061013] Played at once in the as-played window. [ANK 20200813-2]
// Action cards too.
#[test]
fn meditative_grove_cancels_a_frenzy_card_played_on_its_salubri() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ANI"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let salubri = vampire(|c| c.clan == "Salubri" && c.title == Title::None, 0);
    let z = e.add_minion(SeatId(1), salubri, 3, false);
    let grove = location(&mut e, 1, "Meditative Grove");
    hand(&mut e, 0, &["Deep Song"]);
    let before = blood(&e, a);
    pick(&mut e, play_at(0, 1, Some(a), Target::Minion(z)));
    pick_when(&mut e, use_on(grove, 1, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.action().is_none() && e.state.combatants.is_none());
    assert!(!e.state.minions[z.index()].locked, "no combat, no lock");
    // [RBK cancel-a-card] Not performed: its minion unlocked, free to act.
    assert!(!e.state.minions[a.index()].locked);
    assert_eq!(blood(&e, a), before);
    assert!(e.state.cards[grove.index()].locked);
}

// The hunting ground: a Salubri only.
#[test]
fn meditative_grove_feeds_a_salubri() {
    let mut e = Engine::bare(3);
    let salubri = vampire(|c| c.clan == "Salubri" && c.title == Title::None, 0);
    let s = e.add_minion(SeatId(0), salubri, 1, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", "", 0), 1, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let grove = location(&mut e, 0, "Meditative Grove");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(!req
        .options
        .contains(&use_on(grove, 0, None, Target::Minion(other))));
    pick(&mut e, use_on(grove, 0, None, Target::Minion(s)));
    request(&mut e);
    assert_eq!(blood(&e, s), 2);
}

fn ravnos(nth: usize) -> u16 {
    vampire(|c| c.clan == "Ravnos" && c.title == Title::None, nth)
}

// Week of Nightmares: "Put this card in play with 10 nightmare counters.
// Ravnos get +1 bleed and +1 strength and do not hunt as normal. Any Ravnos
// can steal 1 blood from another Ravnos as a +1 stealth hunt action."
// [LSJ 20050720-2] [RTR 20010711] [LSJ 20100915] It can target a vampire
// with no blood; stealing none, the hunt is unsuccessful. [LSJ 20010809-1]
// Blood it cannot hold goes to the blood bank. [LSJ 19970224] Directed at a
// vampire. [LSJ 20080604] An action requiring a Ravnos.
#[test]
fn week_of_nightmares_replaces_the_ravnos_hunt_with_a_theft() {
    let mut e = Engine::bare(3);
    let r = e.add_minion(SeatId(0), ravnos(0), 1, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let victim = e.add_minion(SeatId(1), ravnos(1), 2, false);
    let empty = e.add_minion(SeatId(1), ravnos(2), 0, false);
    let week = location(&mut e, 0, "Week of Nightmares");
    e.state.cards[week.index()].counters = 10;
    let req = request(&mut e);
    assert!(
        !req.options.contains(&action(r, ActionKind::Hunt)),
        "no hunt as normal"
    );
    assert!(req.options.contains(&action(other, ActionKind::Hunt)));
    let steal = |t| use_on(week, 0, Some(r), Target::Minion(t));
    assert!(
        req.options.contains(&steal(empty)),
        "a vampire with no blood too"
    );
    assert!(!req.options.contains(&steal(r)), "another Ravnos");
    // At capacity, the stolen blood goes to the blood bank.
    let cap = e.state.capacity_of(r).unwrap();
    e.state.minions[r.index()].blood = cap;
    pick(&mut e, steal(victim));
    assert_eq!(e.action().unwrap().stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, r), blood(&e, victim)), (cap, 1));
    // +1 bleed for a Ravnos.
    let p1 = pool(&e, 1);
    e.state.minions[r.index()].locked = false;
    e.state.minions[r.index()].acted.clear();
    pick(&mut e, action(r, ActionKind::Bleed));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), p1 - 2);
}

// [RBK action-card-or-card-in-play] "A minion can only perform this special
// hunt once per turn." A Ravnos with no blood must take it if it can.
#[test]
fn a_starving_ravnos_must_steal_once() {
    let mut e = Engine::bare(3);
    let r = e.add_minion(SeatId(0), ravnos(0), 0, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let v1 = e.add_minion(SeatId(1), ravnos(1), 3, false);
    let week = location(&mut e, 0, "Week of Nightmares");
    e.state.cards[week.index()].counters = 10;
    // Its one theft is its only option: taken at once, before anything
    // else.
    let req = request(&mut e);
    assert_eq!((blood(&e, r), blood(&e, v1)), (1, 2));
    assert!(e.state.minions[b.index()].acted.is_empty());
    assert!(req.options.contains(&Choice::Pass));
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::UseAbility { actor: Some(m), .. } if *m == r)));
}

// Rules-qa 28.8 (owner, 2026-09-26): a Ravnos that must hunt but has no
// target for the special hunt is frozen — no action at all; the others act.
#[test]
fn a_starving_ravnos_without_a_target_is_frozen() {
    let mut e = Engine::bare(3);
    let r = e.add_minion(SeatId(0), ravnos(0), 0, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let week = location(&mut e, 0, "Week of Nightmares");
    e.state.cards[week.index()].counters = 10;
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&Choice::Pass));
    assert!(req.options.contains(&action(b, ActionKind::Bleed)));
    assert!(!req.options.iter().any(|o| match *o {
        Choice::TakeAction { minion, .. } => minion == r,
        Choice::UseAbility { actor, .. } | Choice::Play { actor, .. } => actor == Some(r),
        _ => false,
    }));
}

// "During each Methuselah's unlock phase, that Methuselah can move 1
// nightmare counter from this card to a Ravnos. If this card has no
// counters, each Ravnos burns 1 blood for each nightmare counter on them or
// is burned, then burn this card and the nightmare counters." "Only one
// Week of Nightmares can be played in a game."
#[test]
fn week_of_nightmares_ends_in_nightmares_and_never_comes_back() {
    let mut e = Engine::bare(3);
    let r1 = e.add_minion(SeatId(0), ravnos(0), 3, false);
    let r2 = e.add_minion(SeatId(1), ravnos(1), 1, false);
    e.add_minion(SeatId(2), plain("", "", 0), 4, false);
    hand(&mut e, 0, &["Week of Nightmares", "Week of Nightmares"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::None));
    request(&mut e);
    let week = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[week.index()].counters, 10);
    // Down to one counter; the nightmares already spread.
    e.state.cards[week.index()].counters = 1;
    e.state.minions[r1.index()].nightmare = 2;
    e.state.minions[r2.index()].nightmare = 3;
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    let b1 = blood(&e, r1);
    assert!(b1 >= 3);
    // Seat 0's unlock phase: the last counter goes to r1.
    pick_when(&mut e, use_on(week, 1, None, Target::Minion(r1)));
    assert!(!e.state.cards[week.index()].in_play);
    assert_eq!(blood(&e, r1), b1 - 3, "3 nightmares, 3 blood");
    assert!(
        !e.state.minions[r2.index()].in_play,
        "burned: 3 counters, 1 blood"
    );
    // Never a second one this game.
    assert!(e
        .state
        .once_played
        .contains(&find_library("Week of Nightmares")));
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play_at(0, 0, None, Target::None),
    );
}

// ---- The pre-built decks' last equipment (slice 11r) ------------------------

// Living Manse: "Location. While in play, this card does not count as
// equipment. This vampire gets +1 bleed. They can burn this card before
// range is determined to end combat. A vampire can have only one Living
// Manse." (Tzimisce; 1 blood.)
#[test]
fn living_manse_bleeds_and_ends_a_combat_one_per_vampire() {
    let mut e = Engine::bare(3);
    let t = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Tzimisce" && c.title == Title::None, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    hand(&mut e, 0, &["Living Manse", "Living Manse"]);
    let b0 = blood(&e, t);
    pick(&mut e, play(0, 0, t));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let manse = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[manse.index()].host, Some(t));
    assert_eq!(blood(&e, t), b0 - 1);
    assert_eq!(e.state.bearer_mod(t, Stat::Bleed, ActionKind::Bleed), 1);
    // One per vampire.
    e.state.minions[t.index()].locked = false;
    e.state.minions[t.index()].acted.clear();
    let req = request(&mut e);
    assert!(!plays_by(&req, t)
        .iter()
        .any(|&(c, _)| e.state.seats[0].hand[c as usize] == find_library("Living Manse")));
    // +1 bleed; blocked, it burns the Manse before range: no strike.
    let p1 = pool(&e, 1);
    pick(&mut e, action(t, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    let (bt, bz) = (blood(&e, t), blood(&e, z));
    pick_when(&mut e, use_on(manse, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[manse.index()].in_play);
    assert_eq!((blood(&e, t), blood(&e, z), pool(&e, 1)), (bt, bz, p1));
}

// [LSJ 20090324] Living Manse: "An action targeting it (eg. Conceal) is
// directed at the Methuselah controlling it, not the minion it is on." In
// play it is a location, not an equipment (rules-qa 17.4): Rewilding burns
// it, and its controller — the vampire's — pays.
#[test]
fn living_manse_in_play_is_a_location_not_an_equipment() {
    let mut e = Engine::bare(3);
    let g = e.add_minion(SeatId(0), plain("Gangrel", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let z = e.add_minion(
        SeatId(1),
        vampire(|c| c.clan == "Tzimisce" && c.title == Title::None, 0),
        4,
        false,
    );
    e.add_minion(SeatId(2), plain("", "", 1), 4, false);
    let manse = equip(&mut e, "Living Manse", z);
    assert!(!cards::is_equipment_in_play(find_library("Living Manse")));
    hand(&mut e, 0, &["Rewilding"]);
    let p1 = pool(&e, 1);
    pick(&mut e, play_at(0, 0, Some(g), Target::Card(manse)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[manse.index()].in_play);
    assert_eq!(pool(&e, 1), p1 - 2);
}

fn salubri(nth: usize) -> u16 {
    vampire(|c| c.clan == "Salubri" && c.title == Title::None, nth)
}

// Sword of the Archangel: "Strike: strength+1 aggravated damage. … Once each
// turn, if the opposing vampire is burned during this weapon's strike
// resolution and the bearer remains ready, the bearer can unlock at the end
// of combat."
#[test]
fn sword_of_the_archangel_unlocks_its_bearer_after_burning_a_vampire() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), salubri(0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 0, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let sword = equip(&mut e, "Sword of the Archangel", a);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, weapon_strike(sword));
    pick_when(&mut e, use_on(sword, 2, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[z.index()].in_play,
        "burned: 2 aggravated, no blood"
    );
    assert!(!e.state.minions[a.index()].locked);
}

// A hand strike burning the vampire is not the weapon's: no unlock.
#[test]
fn sword_of_the_archangel_unlocks_only_after_its_own_strike() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), salubri(0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 0, true);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.state.minions[z.index()].in_torpor = false;
    let sword = equip(&mut e, "Sword of the Archangel", a);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, Choice::HandStrike);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_on(sword, 2, None, Target::None),
    );
    assert!(
        e.state.minions[z.index()].in_torpor,
        "a hand strike: torpor"
    );
    assert!(e.state.minions[a.index()].locked);
}

// "Once each combat, this Salubri can burn 1 blood to cancel a grapple or
// aim card as it is played by the opposing minion, and its cost is not
// paid."
#[test]
fn sword_of_the_archangel_cancels_an_opposing_grapple_for_one_blood() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), salubri(0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), with(&["pot"], 0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let sword = equip(&mut e, "Sword of the Archangel", a);
    hand(&mut e, 1, &["Immortal Grapple"]);
    let b0 = blood(&e, a);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, use_on(sword, 1, None, Target::None));
    // Canceled: the weapon strike is still there.
    pick_when(&mut e, weapon_strike(sword));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Immortal Grapple")));
    // 1 blood for the cancel, 1 to z's hand strike.
    assert_eq!(blood(&e, a), b0 - 2);
}

// rules-qa 17.4: Living Manse is a location once in play, so Form of the
// Bat's "cannot use equipment during the resulting combat" leaves its
// combat-ending burn usable.
#[test]
fn living_manse_ends_a_combat_under_form_of_the_bat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "pro", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "aus", 1), 5, false);
    let manse = equip(&mut e, "Living Manse", a);
    hand(&mut e, 0, &["Form of the Bat"]);
    hand(&mut e, 1, &["Enhanced Senses"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, a)); // -1 intercept
    pick_when(&mut e, play(0, 0, z)); // +1: blocked at 0 against 0
    pick_when(&mut e, use_on(manse, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[manse.index()].in_play);
    assert_eq!(blood(&e, z), 5, "no strike");
}

// ---- Combat cards, wave 2 (slice 11s) --------------------------------------

// No Trace: "[obf] Only usable at long range. Strike: combat ends. [OBF]
// Strike: combat ends."
#[test]
fn no_trace_ends_combat_at_long_range_or_at_superior() {
    let (mut e, _, z) = fight(plain("", "", 1), &[], plain("", "obf", 0), &["No Trace"]);
    never_offered(&mut e, &play(0, 0, z));
    let (mut e, a, z) = fight(plain("", "", 1), &[], plain("", "OBF", 0), &["No Trace"]);
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (5, 5 - 1), "its blood only");
}

// Shadow Shift: "[obl] Additional strike (limited). [OBL] Maneuver."
#[test]
fn shadow_shift_strikes_again_or_maneuvers_away() {
    let (mut e, a, z) = fight(plain("", "", 1), &[], with(&["OBL"], 0), &["Shadow Shift"]);
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (5 - 2, 5 - 1));
    let (mut e, a, z) = fight(plain("", "", 1), &[], with(&["OBL"], 0), &["Shadow Shift"]);
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (5, 5), "long range");
}

// Wolf Claws: "[pro] Damage from this vampire's hand strikes is aggravated
// this round." (1 blood.) [RTR 19960112] "The ability to make the damage
// from the vampire's strikes aggravated that round must be used before
// strike resolution and damage prevention in order to affect the current
// strike. Can be played after both strikes have been chosen."
#[test]
fn wolf_claws_after_both_strikes_are_chosen_aggravates_the_hand_strike() {
    let (mut e, a, z) = fight(plain("", "pro", 0), &["Wolf Claws"], plain("", "", 1), &[]);
    until(&mut e, SeatId(0), WindowLabel::CombatBeforeResolution);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert!(v.in_torpor, "1 aggravated damage: wounded");
    assert_eq!(v.blood, 5);
    assert_eq!(blood(&e, a), 5 - 1 - 1, "its blood and the hand strike");
}

// Diversion: "Requires an Anarch. [cel] Additional strike (limited). [for]
// Prevent 2 damage. [tha] Strike, ranged: steal 1 blood or life (becoming
// blood), with 1 optional maneuver." [RTR 20010711] "[tha] can target a
// vampire with no blood." [RTR 20041202] "If fewer points of (preventable)
// damage are being resolved, then the effect prevents all of those points."
// [ANK 20200318] [LSJ 20001114] Unused points do not carry over to the next
// strike. [LSJ 20001114] "Cannot be used if there is no damage to prevent."
// [LSJ 20011204-3] [PIB 20130704] Played, it requires the Discipline used:
// an Anarch without Fortitude never plays the [for] prevention.
#[test]
fn diversion_is_an_anarchs_steal_maneuver_or_prevention() {
    let (mut e, a, _) = fight(
        anarch_with(&["tha"], &["for"], 0),
        &["Diversion"],
        plain("", "", 1),
        &[],
    );
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&play(0, 2, a)));
    never_offered(&mut e, &play(0, 1, a));
    // Not an Anarch: none of it.
    let not_anarch = vampire(
        |c| {
            c.sect != zvtes::crypt::Sect::Anarch && c.title == Title::None && has_level(c, &["tha"])
        },
        0,
    );
    let (mut e, a, _) = fight(not_anarch, &["Diversion"], plain("", "", 1), &[]);
    never_offered(&mut e, &play(0, 2, a));
    // At the maneuver: to long range, the steal committed — from a vampire
    // with no blood, so nothing; no damage, no prevention offered.
    let (mut e, a, z) = fight(
        anarch_with(&["tha", "for"], &[], 0),
        &["Diversion", "Diversion"],
        plain("", "", 1),
        &[],
    );
    e.state.minions[z.index()].blood = 0;
    let start = capped(&e, a);
    until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    pick(&mut e, play(0, 2, a));
    never_offered(&mut e, &play(0, 1, a));
    assert_eq!((blood(&e, a), blood(&e, z)), (start, 0));
    assert!(
        e.state.minions[z.index()].ready(),
        "long range: no hand strike"
    );
    // [for]: 1 damage pending, all of it prevented; the unused point does
    // not reach the additional strike.
    let (mut e, a, z) = fight(
        anarch_with(&["for"], &[], 0),
        &["Diversion"],
        with(&["cel"], 0),
        &["Quickness"],
    );
    let start = capped(&e, a);
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), start - 1, "the additional strike's point");
}

// Lightning Reflexes: "[CEL] Burn X blood to get X additional strikes
// (limited)." (1 blood.) [ANK 20210226] "The burn blood effect is not
// reduced by effects that reduce the cost of the card": an effect, X up to
// the blood left after the card's cost.
#[test]
fn lightning_reflexes_burns_x_blood_for_x_additional_strikes() {
    let (mut e, a, z) = fight(
        with(&["CEL"], 0),
        &["Lightning Reflexes"],
        plain("", "", 1),
        &[],
    );
    let start = capped(&e, a);
    pick_when(&mut e, play(0, 1, a));
    // Played after the first pair (rules-qa 47.2): Z's hand strike landed.
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(req.options.last(), Some(&Choice::SetX(start - 2)));
    pick(&mut e, Choice::SetX(2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), start - 1 - 2 - 1, "cost, X, one hand strike");
    assert_eq!(blood(&e, z), 5 - 3, "three hand strikes");
}

// Wind Dance: "[THA] Strike: dodge, with 1 additional strike: dodge
// (limited)." Both of the opposing minion's strikes are dodged, and no other
// limited additional strike follows.
#[test]
fn wind_dance_dodges_twice() {
    let (mut e, a, z) = fight(
        with(&["cel"], 0),
        &["Quickness"],
        plain("", "THA", 0),
        &["Wind Dance", "Quickness"],
    );
    pick_when(&mut e, play(0, 1, z)); // Wind Dance [THA]
    pick_when(&mut e, play(0, 0, a)); // Quickness, after the pair: a second strike
    never_offered(&mut e, &play(0, 0, z));
    assert_eq!((blood(&e, a), blood(&e, z)), (5, 5), "both dodged");
}

// Anticipation: "[aus] Strike: hand strike or use a melee weapon strike.
// This strike is at +2 damage."
#[test]
fn anticipation_strikes_with_the_hand_or_a_melee_weapon_at_plus_two() {
    for weapon in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), salubri(0), 5, false);
        let start = capped(&e, a);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let z = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
        let zs = capped(&e, z);
        let blade = equip(&mut e, "Righteous Blade", a);
        hand(&mut e, 0, &["Anticipation"]);
        bleed_blocked(&mut e, a, z);
        let choice = if weapon {
            play_at(0, 1, Some(a), Target::Card(blade))
        } else {
            play(0, 0, a)
        };
        pick_when(&mut e, choice);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let damage = if weapon { 1 + 1 + 2 } else { 1 + 2 };
        assert_eq!(blood(&e, z), zs - damage, "weapon: {weapon}");
        assert_eq!(blood(&e, a), start - 1);
    }
}

// Anticipation: "[AUS] Burn 1 blood to cancel the opposing minion's strike
// card as it is played, and its cost is not paid (the minion chooses a
// strike again)."
#[test]
fn anticipation_superior_cancels_a_strike_card_unpaid() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Salubri" && has(c, &["AUS"]), 0),
        5,
        false,
    );
    let start = capped(&e, a);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), with(&["tha"], 0), 5, false);
    let zs = capped(&e, z);
    hand(&mut e, 0, &["Anticipation"]);
    hand(&mut e, 1, &["Blood Fury"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, z));
    pick(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), zs - 1, "unpaid; a's hand strike");
    assert_eq!(blood(&e, a), start - 1 - 1, "its blood; z's hand strike");
    assert!(e.state.seats[1].ash.contains(&find_library("Blood Fury")));
}

// Terror Frenzy: "Frenzy. Only usable before range is determined. [ani]
// Only usable during the first round. This combat, the opposing minion
// cannot maneuver to close range, press to continue combat or use
// equipment." Pursuit, Apportation's press and the gun are never offered.
#[test]
fn terror_frenzy_keeps_the_opponent_away_from_its_equipment_and_close_range() {
    let mut e = Engine::bare(3);
    // Animalism and Celerity: Keegan's text (unlock after his own Gangrel
    // action) is inert here.
    let a = e.add_minion(SeatId(0), crypt("Keegan"), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(
        SeatId(1),
        vampire(
            |c| has_level(c, &["cel", "tha"]) && c.title == Title::None,
            0,
        ),
        5,
        false,
    );
    let gun = equip(&mut e, ".44 Magnum", z);
    hand(&mut e, 0, &["Terror Frenzy", "Pursuit"]);
    hand(&mut e, 1, &["Pursuit", "Apportation"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    // To long range: z's Pursuit would bring it back to close range.
    pick_when(&mut e, play(0, 0, a));
    // No maneuver to close, no equipment, no press to continue.
    let barred = [play(0, 0, z), weapon_strike(gun), play(1, 0, z)];
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        barred.contains(o)
    });
}

// Terror Frenzy: "[ANI] This combat, combat cards cost the opposing vampire
// +1 blood. A vampire can play only one Terror Frenzy at superior each
// combat." [ANK 20180910-3] "Also affects the cost of combat cards played
// at the end of a round or combat."
#[test]
fn terror_frenzy_superior_raises_the_opponents_combat_cards_once_each_combat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ANI"], 0), 5, false);
    let start = capped(&e, a);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), with(&["tha"], 0), 5, false);
    let zs = capped(&e, z);
    hand(&mut e, 0, &["Terror Frenzy", "Terror Frenzy"]);
    hand(&mut e, 1, &["Blood Rage"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 1, a)), "one at superior");
    assert!(req.options.contains(&play(0, 0, a)), "the basic one still");
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, z),
        zs - 1 - 1,
        "Blood Rage at 1 blood; a's hand strike"
    );
    assert_eq!(
        blood(&e, a),
        start - 1 - 1,
        "its blood; Blood Rage's strike"
    );
}

// Rules-qa 28.11 (owner, 2026-09-26): a frenzy card is played on the minion
// it affects — Terror Frenzy on the opposing minion (LSJ 20051113, ANK
// 20220409): Meditative Grove cancels it for its Salubri. [ANK 20260216]
// "The cost for a canceled non-action-card is still paid": Terror
// Frenzy's blood.
#[test]
fn meditative_grove_cancels_a_terror_frenzy_played_on_its_salubri() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ani"], 0), 5, false);
    let start = capped(&e, a);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), salubri(0), 3, false);
    let grove = location(&mut e, 1, "Meditative Grove");
    let gun = equip(&mut e, ".44 Magnum", z);
    hand(&mut e, 0, &["Terror Frenzy"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    pick(&mut e, use_on(grove, 1, None, Target::None));
    request(&mut e);
    assert_eq!(blood(&e, a), start - 1, "its cost still paid");
    pick_when(&mut e, weapon_strike(gun));
}

// Tranquility Shield: "[for] This combat, this vampire can prevent 1 damage
// each round. This combat, frenzy cards cannot be used on this vampire;
// cancel the effects of frenzy cards that have already been used on this
// vampire this combat."
#[test]
fn tranquility_shield_cancels_and_bars_frenzy_cards() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ani"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(
        SeatId(1),
        vampire(|c| c.clan == "Salubri" && has_level(c, &["for"]), 0),
        5,
        false,
    );
    let zs = capped(&e, z);
    let gun = equip(&mut e, ".44 Magnum", z);
    hand(&mut e, 0, &["Terror Frenzy", "Terror Frenzy"]);
    hand(&mut e, 1, &["Tranquility Shield"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, z));
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)), "no frenzy card on z");
    // Its gun back; a's hand strike prevented.
    pick_when(&mut e, weapon_strike(gun));
    pick_when(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), zs - 1, "its blood only");
}

// Flesh of Marble: "[pro] This combat, if a damage is successfully
// inflicted on this vampire in a given round, any additional damage
// inflicted on this vampire in the same round is automatically prevented."
#[test]
fn flesh_of_marble_lets_one_damage_through_each_round() {
    let modes: [(&'static [&'static str], u8); 2] = [(&["pro"], 0), (&["PRO"], 1)];
    for (tag, mode) in modes {
        let (mut e, a, z) = fight(
            with(tag, 0),
            &["Flesh of Marble"],
            with(&["cel"], 1),
            &["Quickness"],
        );
        let start = capped(&e, a);
        pick_when(&mut e, play(0, mode, a));
        pick_when(&mut e, play(0, 0, z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            blood(&e, a),
            start - 1,
            "{tag:?}: the second hand strike prevented"
        );
    }
}

// Weather Control: "[tha] Both combatants and each retainer on them take 1
// unpreventable environmental damage before range is determined each round
// this combat. [THA] As above, but the amount of damage inflicted is
// increased by 1 in each subsequent round." [RTR 19970630] "The damage is
// environmental": unpreventable, Soak is not offered. [ANK 20170930] Dealt
// once both minions have played their before-range effects. [LSJ 19971110]
// [ANK 20190725] "The damage is applied even if the combat ends
// prematurely": Living Manse ends it before range is determined.
#[test]
fn weather_control_damages_both_combatants_and_their_retainers() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["tha"], 0), 5, false);
    let start = capped(&e, a);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    let zs = capped(&e, z);
    let owl = equip(&mut e, "Owl Companion", z);
    e.state.cards[owl.index()].counters = 2;
    hand(&mut e, 0, &["Weather Control"]);
    hand(&mut e, 1, &["Soak"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    never_offered(&mut e, &play(0, 0, z));
    assert_eq!(blood(&e, a), start - 1 - 1, "weather, z's hand strike");
    assert_eq!(blood(&e, z), zs - 1 - 1, "weather, a's hand strike");
    assert_eq!(e.state.cards[owl.index()].counters, 1);
    // Ended before range: the weather still strikes.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["tha"], 0), 5, false);
    let start = capped(&e, a);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    let zs = capped(&e, z);
    let manse = equip(&mut e, "Living Manse", a);
    hand(&mut e, 0, &["Weather Control"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, use_on(manse, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (start - 1, zs - 1));
}

// Weather Control [THA]: 1 damage in the first round, 2 in the second.
#[test]
fn weather_control_superior_grows_each_round() {
    let mut e = Engine::bare(3);
    // Tommaso's text (ending a wraith's or zombie's combat) is inert here.
    let a = e.add_minion(SeatId(0), crypt("Tommaso Sforza"), 8, false);
    let start = capped(&e, a);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    // Anja's text (unlocking another Hecata after its action) is inert here.
    let z = e.add_minion(SeatId(1), crypt("\"Mother\" Anja Giovanni"), 8, false);
    let zs = capped(&e, z);
    hand(&mut e, 0, &["Weather Control", "Apportation"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, a)); // Apportation: press to continue
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, a),
        start - 3 - 2,
        "1 + 2 weather, two hand strikes"
    );
    assert_eq!(blood(&e, z), zs - 3 - 2);
}

// Disarm: "Only usable at close range at the end of a round during which
// this vampire successfully inflicted more damage than the opposing
// vampire. [pot] Put this card on the opposing vampire and send them to
// torpor. The attached vampire gets -1 strength. They can burn 3 blood to
// burn this card." [RTR 20030519] "Must be played after the presses are
// handled": at the end of the round. [LSJ 20010812] "Environmental damages
// (eg. Carrion Crows) do not count."
#[test]
fn disarm_sends_the_weaker_striker_to_torpor_with_less_strength() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["pot"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    hand(&mut e, 0, &["Slam", "Disarm"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a)); // Slam: hand strike at +2
    pick_when(&mut e, play(0, 0, a)); // Disarm
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let disarm = find_library("Disarm");
    assert!(e.state.minions[z.index()].in_torpor);
    assert!(on(&e, z).iter().any(|&(c, _)| c == disarm));
    assert_eq!(e.state.bearer_mod(z, Stat::Strength, ActionKind::Bleed), -1);
    // Burned for 3 blood.
    let d = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == disarm)
            .unwrap() as u32,
    );
    e.state.minions[z.index()].blood = 3;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, use_on(d, 0, None, Target::None));
    request(&mut e);
    assert!(!e.state.cards[d.index()].in_play);
    assert_eq!(blood(&e, z), 0);
    // Environmental damage does not count.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ani", "pot"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    hand(&mut e, 0, &["Carrion Crows", "Disarm"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    never_offered(&mut e, &play(0, 0, a));
}

/// A vampire's blood once any excess over capacity has drained (at the
/// first `run`).
fn capped(e: &Engine, m: MinionId) -> u8 {
    blood(e, m).min(e.state.capacity_of(m).unwrap_or(u8::MAX))
}

// Hide the Mind: "[obf] [COMBAT] Cancel a combat card requiring Auspex [aus]
// as it is played, and its cost is not paid."
#[test]
fn hide_the_mind_cancels_an_auspex_combat_card() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["obf"], 0), 5, false);
    let start = capped(&e, a);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), salubri(0), 5, false);
    hand(&mut e, 0, &["Hide the Mind"]);
    hand(&mut e, 1, &["Anticipation"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, z));
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), start - 1, "z chose a hand strike again");
}

// Hide the Mind: "[OBF] [ACTION MODIFIER] Cancel a reaction card requiring
// Auspex as it is played, and its cost is not paid." [LSJ 20061211] A bleed
// redirect canceled as it is played: "No blocks can be attempted … (as
// blocks have been declined)."
#[test]
fn hide_the_mind_superior_cancels_a_redirect_and_no_block_follows() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["OBF"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("", "AUS", 0), 4, false);
    let t2 = e.add_minion(SeatId(2), plain("", "", 2), 4, false);
    hand(&mut e, 0, &["Hide the Mind"]);
    hand(&mut e, 1, &["Telepathic Misdirection"]);
    let before = (pool(&e, 1), pool(&e, 2));
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play_at(0, 1, Some(t), Target::Seat(SeatId(2))));
    pick(&mut e, play(0, 1, a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::DeclareBlock { minion: t2 },
    );
    assert_eq!((pool(&e, 1), pool(&e, 2)), (before.0 - 1, before.1));
    assert!(!e.state.minions[t.index()].locked, "unpaid, not locked");
}

// Angel's Gift: "[COMBAT] Equip this vampire with a melee weapon from your
// hand (requirements and cost apply as normal)." Used at once.
#[test]
fn angels_gift_equips_a_melee_weapon_for_this_combat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), salubri(0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    let zs = capped(&e, z);
    hand(
        &mut e,
        0,
        &["Angel's Gift", "Righteous Blade", ".44 Magnum"],
    );
    bleed_blocked(&mut e, a, z);
    // The blade is the only melee weapon in hand: named at once (elided).
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    let id = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(
        e.state.cards[id.index()].card,
        find_library("Righteous Blade")
    );
    assert_eq!(e.state.cards[id.index()].host, Some(a));
    pick_when(&mut e, weapon_strike(id));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), zs - 2, "strength+1");
}

// Angel's Gift: "[COMBAT] This round, this vampire gets 1 optional maneuver,
// only usable to get to close range."
#[test]
fn angels_gift_maneuvers_only_back_to_close_range() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), salubri(0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), with(&["cel"], 0), 5, false);
    let zs = capped(&e, z);
    hand(&mut e, 0, &["Angel's Gift"]);
    hand(&mut e, 1, &["Pursuit"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    // At close range, no use for it: a's maneuver window elides.
    pick_when(&mut e, play(0, 0, z)); // Pursuit: to long range
    pick(&mut e, Choice::UseManeuver);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), zs - 1, "back at close range");
}

// Contraband: "[OBF] As above, and this vampire can pay up to half the cost
// rounded down of that equipment with their blood." A non-unique one only.
// [LSJ 20100725] [LSJ 20100303] [ANK 20180817] "Requirements apply": not a
// Salubri's Righteous Blade. [LSJ 20100130] [ANK 20180925-2] [LSJ 19991207]
// Named as the card is played. [LSJ 20080815] Not an action: the vampire
// has not acted.
#[test]
fn contraband_superior_equips_paying_half_in_blood() {
    let mut e = Engine::bare(3);
    let ravnos = vampire(|c| c.clan == "Ravnos" && has(c, &["OBF"]), 0);
    let a = e.add_minion(SeatId(0), ravnos, 5, false);
    let start = capped(&e, a);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    hand(
        &mut e,
        0,
        &["Contraband", ".44 Magnum", "Ivory Bow", "Righteous Blade"],
    );
    let p0 = pool(&e, 0);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    let magnum = find_library(".44 Magnum");
    // The Magnum named at once (neither the unique bow nor the Salubri
    // blade is offered), then up
    // to 1 of its 2 pool in blood.
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(req.options, vec![Choice::SetX(0), Choice::SetX(1)]);
    pick(&mut e, Choice::SetX(1));
    request(&mut e);
    assert_eq!((pool(&e, 0), capped(&e, a)), (p0 - 1, start - 1));
    assert!(e
        .state
        .equipment_on(a)
        .iter()
        .any(|&c| e.state.cards[c.index()].card == magnum));
    assert!(e.state.minions[a.index()].acted.is_empty(), "no action");
}

// Martyr's Resilience: "Only usable by an unlocked vampire not involved in
// the combat. [aus][for] Prevent 1 damage to a minion or retainer in
// combat." [ANK 20181101] "Can be played by a minion of a Methuselah not
// involved in the current combat." [LSJ 20020416] "Can be used by a vampire
// in torpor." [LSJ 20001114] "Cannot be used if there is no damage to
// prevent": both strikes dodged or missed, never offered.
#[test]
fn martyrs_resilience_prevents_from_outside_the_combat() {
    {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 1), 5, false);
        let z = e.add_minion(SeatId(1), with(&["cel"], 0), 5, false);
        let m = e.add_minion(
            SeatId(2),
            vampire(
                |c| has_level(c, &["aus", "for"]) && c.title == Title::None,
                0,
            ),
            3,
            false,
        );
        hand(&mut e, 1, &["Pursuit"]);
        hand(&mut e, 2, &["Martyr's Resilience"]);
        bleed_blocked(&mut e, a, z);
        pick_when(&mut e, play(0, 0, z)); // to long range
        for t in [a, z] {
            never_offered(&mut e, &play_at(0, 0, Some(m), Target::Minion(t)));
        }
    }
    for torpid in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let start = capped(&e, a);
        e.add_minion(SeatId(0), plain("Ventrue", "", 1), 5, false);
        let z = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
        let zs = capped(&e, z);
        let m = e.add_minion(
            SeatId(2),
            vampire(
                |c| has_level(c, &["aus", "for"]) && c.title == Title::None,
                0,
            ),
            3,
            torpid,
        );
        e.add_minion(SeatId(2), plain("Ventrue", "", 2), 5, false);
        hand(&mut e, 2, &["Martyr's Resilience"]);
        bleed_blocked(&mut e, a, z);
        pick_when(&mut e, play_at(0, 0, Some(m), Target::Minion(z)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), zs, "torpid: {torpid}");
        assert_eq!(blood(&e, a), start - 1, "torpid: {torpid}");
    }
}

// Martyr's Resilience: "[AUS][FOR] Burn X blood to prevent X+1 damage to a
// minion or retainer in combat." A retainer hit by a ranged strike at long
// range. [ANK 20210226] The X blood is an effect, up to what the vampire
// has.
#[test]
fn martyrs_resilience_superior_saves_a_retainer() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 1), 5, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    let owl = equip(&mut e, "Owl Companion", z);
    e.state.cards[owl.index()].counters = 2;
    let gun = equip(&mut e, ".44 Magnum", a);
    let m = e.add_minion(
        SeatId(2),
        vampire(|c| has(c, &["AUS", "FOR"]) && c.title == Title::None, 0),
        5,
        false,
    );
    let ms = capped(&e, m);
    hand(&mut e, 2, &["Martyr's Resilience"]);
    bleed_blocked(&mut e, a, z);
    // The Magnum's maneuver to long range, then its 2R at the owl.
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(gun),
            ability: 1,
            actor: None,
            target: Target::None,
        },
    );
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(gun),
            ability: 0,
            actor: None,
            target: Target::Card(owl),
        },
    );
    pick_when(&mut e, play_at(0, 3, Some(m), Target::Card(owl)));
    pick(&mut e, Choice::SetX(1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[owl.index()].counters, 2, "2 prevented");
    assert_eq!(blood(&e, m), ms - 1);
}

// Disarm: [LSJ 20021113] [ANK 20191219] [ANK 20180910-1] "Can be played
// before or after effects that are played at the end of the round or 'when
// the combat would end' (eg. [AUS] Telepathic Tracking)." After a new round
// is started, still offered; played first, Telepathic Tracking has no
// combatant pair left to continue.
#[test]
fn disarm_and_telepathic_tracking_share_the_end_of_round() {
    for tracking_first in [true, false] {
        let mut e = Engine::bare(3);
        // Anja's text (unlocking another Hecata after its action) is inert here.
        let a = e.add_minion(SeatId(0), crypt("\"Mother\" Anja Giovanni"), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let z = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
        hand(&mut e, 0, &["Slam", "Disarm", "Telepathic Tracking"]);
        bleed_blocked(&mut e, a, z);
        pick_when(&mut e, play(0, 0, a)); // Slam
        let (disarm, tracking) = (play(0, 0, a), play(1, 1, a));
        let req = until(&mut e, SeatId(0), WindowLabel::CombatEndOfRound);
        assert!(req.options.contains(&disarm) && req.options.contains(&tracking));
        if tracking_first {
            pick(&mut e, tracking);
            let req = request(&mut e);
            assert!(req.options.contains(&disarm), "still the round's end");
        } else {
            pick(&mut e, disarm);
            let req = request(&mut e);
            assert!(!req.options.contains(&play(0, 1, a)), "z no longer ready");
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(e.state.minions[z.index()].in_torpor);
        }
    }
}

// Terror Frenzy [ANI]: [ANK 20180910-3] "Also affects the cost of combat
// cards played at the end of a round or combat (eg. Disarm, Telepathic
// Tracking, Psyche!, Taste of Vitae)."
#[test]
fn terror_frenzy_surcharges_an_end_of_round_disarm() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ANI"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), with(&["pot"], 0), 5, false);
    let zs = capped(&e, z);
    hand(&mut e, 0, &["Terror Frenzy"]);
    hand(&mut e, 1, &["Slam", "Disarm"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, z)); // Slam: 1 + 1 blood
    pick_when(&mut e, play(0, 0, z)); // Disarm: 0 + 1 blood
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].in_torpor);
    assert_eq!(
        blood(&e, z),
        zs - 2 - 1 - 1,
        "Slam, Disarm, a's hand strike"
    );
}

// Flesh of Marble: [LSJ 20040812-2] [PIB 20130327] "[pro] Cannot be used to
// prevent aggravated damage." [LSJ 20021001] "[PRO] The vampire preventing the
// damage can choose which one of the damage he does not prevent, normal or
// aggravated." An aggravated hand strike (Wolf Claws) and environmental
// damage (Carrion Crows) in the same resolution.
#[test]
fn flesh_of_marble_leaves_aggravated_damage_unless_superior() {
    for superior in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["PRO"], 0), 5, false);
        let start = capped(&e, a);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let z = e.add_minion(
            SeatId(1),
            vampire(
                |c| has_level(c, &["ani", "pro"]) && c.title == Title::None,
                0,
            ),
            5,
            false,
        );
        hand(&mut e, 0, &["Flesh of Marble"]);
        hand(&mut e, 1, &["Carrion Crows", "Wolf Claws"]);
        bleed_blocked(&mut e, a, z);
        pick_when(&mut e, play(0, superior as u8, a));
        pick_when(&mut e, play(0, 0, z)); // Carrion Crows
        pick_when(&mut e, play(0, 0, z)); // Wolf Claws
        if superior {
            // Prevent 1 of 1 normal + 1 aggravated: cover the aggravated
            // one, the normal point lands.
            let req = until(&mut e, SeatId(0), WindowLabel::CombatPreventionAllocation);
            assert_eq!(req.options, vec![Choice::SetX(0), Choice::SetX(1)]);
            pick(&mut e, Choice::SetX(1));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, a), start - 1);
            assert!(!e.state.minions[a.index()].in_torpor);
        } else {
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, a), start, "the normal point prevented");
            assert!(
                e.state.minions[a.index()].in_torpor,
                "the aggravated one lands"
            );
        }
    }
}

// Rules-qa 29.3 (owner, 2026-09-26): the player preventing chooses what they
// prevent — strike or environmental damage — and Disarm counts the strike
// damage that lands. a's Slam (3) and Carrion Crows (1) on z; z's Hidden
// Strength prevents 2 of them; z's hand strike deals a 1.
#[test]
fn disarm_counts_the_strike_damage_the_preventer_let_through() {
    for (strike_points, offered) in [(1, true), (2, false)] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["ani", "pot"], 0), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let z = e.add_minion(SeatId(1), with(&["for"], 1), 5, false);
        hand(&mut e, 0, &["Carrion Crows", "Slam", "Disarm"]);
        hand(&mut e, 1, &["Hidden Strength"]);
        bleed_blocked(&mut e, a, z);
        pick_when(&mut e, play(0, 0, a)); // Carrion Crows
        pick_when(&mut e, play(0, 0, a)); // Slam
        pick_when(&mut e, play(0, 0, z)); // Hidden Strength
        pick(&mut e, Choice::SetX(1)); // prevent 2
        let req = expect(&mut e, SeatId(1), WindowLabel::CombatPreventionAllocation);
        assert_eq!(req.options, vec![Choice::SetX(1), Choice::SetX(2)]);
        pick(&mut e, Choice::SetX(strike_points));
        let disarm = play(0, 0, a);
        if offered {
            pick_when(&mut e, disarm);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(e.state.minions[z.index()].in_torpor);
        } else {
            never_offered(&mut e, &disarm);
        }
    }
}

// Rules-qa 29.8 (owner, 2026-09-26): Lightning Reflexes [CEL] with X = 0
// gets no additional strike and uses no limit.
#[test]
fn lightning_reflexes_for_no_blood_leaves_the_limit() {
    let (mut e, a, _) = fight(
        with(&["CEL"], 0),
        &["Lightning Reflexes", "Quickness"],
        plain("", "", 1),
        &[],
    );
    pick_when(&mut e, play(0, 1, a));
    pick(&mut e, Choice::SetX(0));
    let req = request(&mut e);
    assert!(
        req.options.contains(&play(0, 0, a)),
        "Quickness still offered"
    );
}

// ---- Reactions and action modifiers, wave 2 (slice 11t) --------------------

/// The `nth` untitled vampire of `clan` with these Disciplines (either
/// level).
fn clan_with(clan: &'static str, tags: &'static [&'static str], nth: usize) -> u16 {
    vampire(
        move |c| c.clan == clan && has_level(c, tags) && c.title == Title::None,
        nth,
    )
}

// Threats: "Only usable during a bleed action. [DOM] +2 bleed (limited)."
#[test]
fn threats_adds_two_to_a_bleed_only() {
    let mut e2 = Engine::bare(3);
    let a2 = e2.add_minion(SeatId(0), with(&["DOM"], 0), 4, false);
    e2.add_minion(SeatId(0), plain("", "", 1), 4, false);
    e2.add_minion(SeatId(1), plain("", "", 2), 4, false);
    hand(&mut e2, 0, &["Threats"]);
    let before = pool(&e2, 1);
    pick(&mut e2, action(a2, ActionKind::Bleed));
    pick_when(&mut e2, play(0, 1, a2));
    until(&mut e2, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e2, 1), before - 3);
    // Not on a hunt.
    let mut e3 = Engine::bare(3);
    let a3 = e3.add_minion(SeatId(0), with(&["DOM"], 0), 1, false);
    e3.add_minion(SeatId(0), plain("", "", 1), 4, false);
    e3.add_minion(SeatId(1), plain("", "", 2), 4, false);
    hand(&mut e3, 0, &["Threats"]);
    pick(&mut e3, action(a3, ActionKind::Hunt));
    never_offered(&mut e3, &play(0, 1, a3));
}

// Forgotten Labyrinth: "Not usable during a bleed action. [OBF] +3
// stealth." Go-getter [obf]: "+1 stealth", not during a bleed either. The
// other action: equipping with a .44 Magnum (+1 stealth), z blocking with
// Precognition's +1 intercept.
#[test]
fn forgotten_labyrinth_and_go_getter_hide_non_bleed_actions() {
    for (card, clan, mode) in [("Forgotten Labyrinth", "", 1), ("Go-getter", "Ravnos", 0)] {
        for bleed in [true, false] {
            let mut e = Engine::bare(3);
            let who = if clan.is_empty() {
                with(&["OBF"], 0)
            } else {
                clan_with(clan, &["obf"], 0)
            };
            let a = e.add_minion(SeatId(0), who, 4, false);
            e.add_minion(SeatId(0), plain("", "", 1), 4, false);
            let z = e.add_minion(SeatId(1), plain("", "aus", 2), 4, false);
            e.add_minion(SeatId(2), plain("", "", 3), 4, false);
            hand(&mut e, 0, &[card, ".44 Magnum"]);
            hand(&mut e, 1, &["Precognition"]);
            if bleed {
                pick(&mut e, action(a, ActionKind::Bleed));
            } else {
                pick(&mut e, play(1, 0, a));
            }
            pick_when(&mut e, Choice::DeclareBlock { minion: z });
            if bleed {
                never_offered(&mut e, &play(0, mode, a));
            } else {
                pick_when(&mut e, play(0, 0, z)); // +1 intercept
                pick_when(&mut e, play(0, mode, a));
                until(&mut e, SeatId(0), WindowLabel::MinionPhase);
                assert!(e.state.combatants.is_none());
                assert!(!e.state.minions[z.index()].locked, "{card}: not blocked");
            }
        }
    }
}

// Go-getter: "[OBF] Only usable after resolution of a blocked action. This
// vampire burns 1 blood to continue the action as if unblocked." (Ravnos.)
// An equip action, blocked, then continued: the gun is equipped.
#[test]
fn go_getter_superior_continues_a_blocked_action() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), clan_with("Ravnos", &["OBF"], 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "aus", 2), 4, false);
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    hand(&mut e, 0, &["Go-getter", ".44 Magnum"]);
    hand(&mut e, 1, &["Precognition"]);
    pick(&mut e, play(1, 0, a));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, z)); // +1 intercept: blocked
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // A hand strike (-1), the card (-1).
    assert_eq!(blood(&e, a), 4 - 1 - 1);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
    assert_eq!(e.state.equipment_on(a).len(), 1);
}

// Elder Impersonation: "[OBF] Only usable if a minion attempts to block.
// That attempt fails and the blocking minion cannot attempt to block this
// action again." (1 blood.)
#[test]
fn elder_impersonation_superior_fails_the_attempt_for_good() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["OBF"], 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    hand(&mut e, 0, &["Elder Impersonation"]);
    let before = pool(&e, 1);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 1, a));
    never_offered(&mut e, &Choice::DeclareBlock { minion: z });
    assert_eq!(pool(&e, 1), before - 1);
}

// Beast Meld: "[ani][pro] +1 stealth. If this vampire is blocked, they can
// prevent 1 damage during the resulting combat." (2 blood.) [LSJ 20010813]
// [LSJ 20010819-2] Only in the block's combat. [LSJ 20010814-2] Again for a
// second block of the same action.
#[test]
fn beast_meld_prevents_a_damage_once_blocked() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ani", "pro"], 0), 5, false);
    let start = capped(&e, a);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "AUS", 0), 4, false);
    hand(&mut e, 0, &["Beast Meld"]);
    hand(&mut e, 1, &["Precognition"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick(&mut e, play(0, 0, a)); // +1 stealth
    pick_when(&mut e, play(0, 1, z)); // +1 intercept: blocked
    pick_when(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), start - 2, "its cost; z's strike prevented");
}

// Beast Meld: "[ANI][PRO] Only usable as a non-bleed action is announced.
// Vampires cannot block this action." (2 blood.) [ANK 20190425] Played as
// announced, before the other modifiers. [ANK 20181208] The action card is
// replaced first.
#[test]
fn beast_meld_superior_keeps_vampires_from_blocking_a_non_bleed() {
    for kind in [ActionKind::Hunt, ActionKind::Bleed] {
        let mut e = Engine::bare(3);
        // Keegan's text (unlock after his own Gangrel action) is inert here.
        let a = e.add_minion(SeatId(0), crypt("Keegan"), 5, false);
        e.add_minion(SeatId(0), plain("", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
        e.add_minion(SeatId(2), plain("", "", 3), 4, false);
        hand(&mut e, 0, &["Beast Meld"]);
        pick(&mut e, action(a, kind));
        if kind == ActionKind::Bleed {
            never_offered(&mut e, &play(0, 1, a));
        } else {
            expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
            pick(&mut e, play(0, 1, a));
            never_offered(&mut e, &Choice::DeclareBlock { minion: z });
        }
    }
}

// Paths in Two Worlds: "Only usable as an action to recruit or employ a
// wraith or zombie is announced. [OBL] +1 stealth, even if stealth is not
// yet needed, and if the action is successful, this vampire can burn 1
// blood to unlock after action resolution." [ANK 20190425] [ANK 20181208]
// Played as announced.
#[test]
fn paths_in_two_worlds_hides_a_recruit_and_unlocks_after() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), clan_with("Hecata", &["OBL"], 0), 5, false);
    let start = capped(&e, a);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    hand(&mut e, 0, &["Bone Shambler", "Paths in Two Worlds"]);
    pick(&mut e, play(0, 0, a));
    expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    pick(&mut e, play(0, 1, a));
    assert_eq!(e.action().unwrap().stealth(&e.state), 1);
    pick_when(&mut e, Choice::UnlockForBlood { minion: a });
    request(&mut e);
    assert!(!e.state.minions[a.index()].locked);
    assert_eq!(
        blood(&e, a),
        start - 1 - 1,
        "Bone Shambler's blood, the unlock"
    );
}

// Dawn Operation: "[FOR] If this action is blocked, all damage inflicted on
// vampires during the resulting combat is aggravated." [PIB 20150603-1]
// "Applies to environmental and retainer damage too": Carrion Crows.
#[test]
fn dawn_operation_aggravates_the_combat_damage() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["FOR", "ani"], 0), 5, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", "", 2), 5, false);
    let zs = capped(&e, z);
    hand(&mut e, 0, &["Dawn Operation", "Carrion Crows"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, a)); // Carrion Crows
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // 2 aggravated: a wound, then a blood burned.
    let v = &e.state.minions[z.index()];
    assert!(v.in_torpor);
    assert_eq!(v.blood, zs - 1);
    assert!(e.state.minions[a.index()].in_torpor, "the hand strike too");
}

// Dawn Operation: "[for] … If a vampire is currently attempting to block,
// they can cancel their block attempt."
#[test]
fn dawn_operation_lets_the_attempting_vampire_cancel() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["for"], 0), 5, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", "", 2), 5, false);
    hand(&mut e, 0, &["Dawn Operation"]);
    let before = pool(&e, 1);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, Choice::ChooseMinion(z)]);
    pick(&mut e, Choice::ChooseMinion(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.combatants.is_none() && !e.state.minions[z.index()].locked);
    assert_eq!(pool(&e, 1), before - 1);
}

// Dogged Pursuit: "Only usable by a locked vampire. [ANI] This vampire
// unlocks and attempts to block. If this vampire does not block this
// action, lock them after action resolution, and put this card on them.
// This vampire can burn this card to get +1 intercept." (Ravnos.)
#[test]
fn dogged_pursuit_follows_a_missed_block_with_intercept_for_later() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["OBF"], 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    // A third minion: the phase stops after b's hunt.
    e.add_minion(SeatId(0), plain("", "", 3), 4, false);
    let z = e.add_minion(SeatId(1), clan_with("Ravnos", &["ANI"], 0), 4, true);
    e.state.minions[z.index()].in_torpor = false;
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Lost in Crowds"]);
    hand(&mut e, 1, &["Dogged Pursuit"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, z)); // unlocks, attempts
    pick_when(&mut e, play(0, 1, a)); // +2 stealth: the attempt fails
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].locked);
    let pursuit = find_library("Dogged Pursuit");
    let d = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == pursuit && c.host == Some(z))
            .expect("on z") as u32,
    );
    // Unlocked again, z burns it for +1 intercept on b's hunt.
    e.state.minions[z.index()].locked = false;
    pick(&mut e, action(b, ActionKind::Hunt));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, use_on(d, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[d.index()].in_play);
    assert!(e.state.combatants.is_none());
    assert!(
        e.state.minions[b.index()].locked && blood(&e, b) < 4,
        "blocked"
    );
}

// Elder Intervention: "Only usable during a bleed action directed at you. Do
// not replace until after this action. +2 intercept." [RTR 19960530] "Can
// be used during any bleed attempt made against you, not just ones
// declared against you": a bleed redirected to seat 2.
#[test]
fn elder_intervention_blocks_a_bleed_redirected_at_you_and_waits_to_be_replaced() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["obf"], 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("", "AUS", 0), 4, false);
    let w = e.add_minion(SeatId(2), plain("", "", 2), 4, false);
    hand(&mut e, 0, &["Lost in Crowds"]);
    hand(&mut e, 1, &["Telepathic Misdirection"]);
    hand(&mut e, 2, &["Elder Intervention"]);
    e.state.seats[2].library = vec![find_library("Elder Intervention"); 3];
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play_at(0, 1, Some(t), Target::Seat(SeatId(2))));
    pick_when(&mut e, Choice::DeclareBlock { minion: w });
    pick_when(&mut e, play(0, 0, a)); // +1 stealth
    pick_when(&mut e, play(0, 0, w)); // +2 intercept
    assert!(e.state.seats[2].hand.is_empty(), "not replaced yet");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[w.index()].locked, "blocked");
    assert!(
        !e.state.seats[2].hand.is_empty(),
        "replaced after the action"
    );
}

// Forced Awakening: "Only usable by a locked vampire. This vampire wakes …
// If they do not block this action, they burn 1 blood before action
// resolution." [LSJ 19990421] As the action begins to resolve. [TOM
// 19951129] The block attempt is only an option. [LSJ 20070417] No blood
// after a block of theirs.
// Rules-qa 30.2 (owner, 2026-09-26): another minion's block is the action's
// resolution — the blood is burned.
#[test]
fn forced_awakening_costs_a_blood_unless_it_blocks() {
    for blocker in [None, Some(2), Some(3)] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        e.add_minion(SeatId(0), plain("", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
        let w = e.add_minion(SeatId(1), plain("", "", 3), 4, false);
        e.state.minions[z.index()].locked = true;
        hand(&mut e, 1, &["Forced Awakening"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick_when(&mut e, play(0, 0, z));
        match blocker {
            Some(2) => pick_when(&mut e, Choice::DeclareBlock { minion: z }),
            Some(_) => pick_when(&mut e, Choice::DeclareBlock { minion: w }),
            None => {}
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        // Unblocked or w blocking: the card's blood; z blocking: a's hand
        // strike, and no blood for the card.
        assert_eq!(blood(&e, z), 4 - 1, "blocker {blocker:?}");
        if blocker == Some(3) {
            assert_eq!(blood(&e, w), 4 - 1, "w's combat");
        }
    }
}

// Forced Vigilance: "[for] Only usable by a locked vampire who has blocked,
// after block resolution. Unlock this vampire. [FOR] Only usable by a
// locked vampire during an action directed at you. Unlock this vampire."
// (1 blood.)
#[test]
fn forced_vigilance_unlocks_after_blocking_or_when_targeted() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), with(&["for"], 0), 5, false);
    hand(&mut e, 1, &["Forced Vigilance"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].locked);
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), with(&["FOR"], 0), 5, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 1, &["Forced Vigilance"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, z));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
}

// Melange: "[AUS] +1 intercept, and if this vampire blocks, put this card on
// the acting minion; you still control this card. During a bleed action
// against the controller of the attached minion, this reacting vampire can
// burn this card to get +1 bleed." [ANK 20180531] [ANK 20231124] [LSJ
// 19970718] Burned during the bleed, before it resolves. [ANK 20220805]
// The card went to the ash heap as played: its effect lands as z blocks.
#[test]
fn melange_superior_marks_the_acting_minion_for_a_later_bleed() {
    let mut e = Engine::bare(2);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), with(&["AUS"], 0), 5, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    hand(&mut e, 0, &["Lost in Crowds"]);
    hand(&mut e, 1, &["Melange"]);
    pick(&mut e, action(a, ActionKind::Hunt)); // +1 stealth
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 1, z)); // +1 intercept: blocks
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let melange = find_library("Melange");
    let m = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == melange && c.host == Some(a))
            .expect("on a") as u32,
    );
    assert_eq!(e.state.card_controller(m), SeatId(1), "still z's");
    let before = pool(&e, 0);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, action(z, ActionKind::Bleed));
    pick_when(&mut e, use_on(m, 0, Some(z), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 0), before - 2);
    assert!(!e.state.cards[m.index()].in_play);
}

// Rat's Warning: "Only usable by a locked vampire during a bleed action
// against you. [ANI] Unlock this vampire, with 1 optional press during the
// resulting combat if this vampire blocks."
#[test]
fn rats_warning_unlocks_to_block_with_a_press() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), with(&["ANI"], 0), 5, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 1, &["Rat's Warning"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, z));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, Choice::UsePress);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 5 - 2, "two rounds of hand strikes");
}

// Redirection: "[dom] Only usable if a younger vampire is bleeding you,
// after blocks are declined. Lock this reacting vampire. Change the target
// of the bleed … [DOM] As above, but the acting vampire can be the same age
// or older." [ANK 20211003] [LSJ 19980105] [PIB 20130711] [LSJ 20061213]
// [LSJ 20061211] As the other redirects.
#[test]
fn redirection_redirects_a_younger_bleeder_or_any_at_superior() {
    for (tag, young, offered) in [
        ("dom", true, true),
        ("dom", false, false),
        ("DOM", false, true),
    ] {
        let mut e = Engine::bare(3);
        // No untitled vampire of capacity 9+ lacks printed text: Hel-Blá's
        // (a printed action of its own) is inert as it bleeds.
        let acting = if young {
            vampire(|c| c.capacity <= 4 && c.title == Title::None, 0)
        } else {
            crypt("Hel-Blá")
        };
        let a = e.add_minion(SeatId(0), acting, 3, false);
        e.add_minion(SeatId(0), plain("", "", 1), 4, false);
        let r = e.add_minion(
            SeatId(1),
            vampire(|c| has(c, &[tag]) && (5..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        e.add_minion(SeatId(2), plain("", "", 2), 4, false);
        hand(&mut e, 1, &["Redirection"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        let mode = if tag == "dom" { 0 } else { 1 };
        let redirect = play_at(0, mode, Some(r), Target::Seat(SeatId(2)));
        if offered {
            let before = pool(&e, 2);
            pick_when(&mut e, redirect);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(pool(&e, 2), before - 1, "{tag}, young {young}");
            assert!(e.state.minions[r.index()].locked);
        } else {
            never_offered(&mut e, &redirect);
        }
    }
}

// Sense the Savage Way: "Requires a vampire with capacity 7 or more. [ANI]
// Only usable by a locked vampire. This vampire unlocks and attempts to
// block." [ANK 20181122-2] [LSJ 20010714-2] [ANK 20180623] [LSJ 20090514]
// As the other forced attempts: a vampire that could block, the usual
// limits applying.
#[test]
fn sense_the_savage_way_unlocks_an_elder_to_block() {
    for cap in [6, 8] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        e.add_minion(SeatId(0), plain("", "", 1), 4, false);
        // No untitled capacity-8 vampire with superior Animalism lacks
        // printed text: Keegan's (unlock after his own Gangrel action) is
        // inert as he blocks.
        let z = if cap == 8 {
            crypt("Keegan")
        } else {
            vampire(
                |c| has(c, &["ANI"]) && c.capacity == cap && c.title == Title::None,
                0,
            )
        };
        let z = e.add_minion(SeatId(1), z, 4, false);
        e.state.minions[z.index()].locked = true;
        hand(&mut e, 1, &["Sense the Savage Way"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        let sense = play(0, 1, z);
        if cap >= 7 {
            pick_when(&mut e, sense);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(e.state.minions[z.index()].locked, "blocked");
            assert!(blood(&e, a) < 4);
        } else {
            never_offered(&mut e, &sense);
        }
    }
}

// Telepathic Counter: "[AUS] Reduce a bleed against you by 2." [TOM
// 19960303] [LSJ 20061212] Before the bleed is successful, before or after
// block attempts.
#[test]
fn telepathic_counter_reduces_a_bleed_against_you() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["DOM"], 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), with(&["AUS"], 0), 4, false);
    hand(&mut e, 0, &["Threats"]);
    hand(&mut e, 1, &["Telepathic Counter"]);
    let before = pool(&e, 1);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, a)); // bleed 3
    pick_when(&mut e, play(0, 1, z)); // -2
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), before - 1);
}

// Forced Awakening: [LSJ 20070417] "The vampire needs not pay a blood if he
// successfully blocks, even if the action continues as if unblocked after
// the block (eg. Form of Mist)." [ANK 20230319] "Does not consider blocks
// made before the card was played." [ANK 20210817] Playable in the
// as-played window of a card during an action.
#[test]
fn forced_awakening_counts_only_blocks_after_it() {
    for played_first in [true, false] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["PRO"], 0), 5, false);
        e.add_minion(SeatId(0), plain("", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
        hand(&mut e, 0, &["Form of Mist"]);
        hand(&mut e, 1, &["Forced Awakening"]);
        if played_first {
            e.state.minions[z.index()].locked = true;
        }
        pick(&mut e, action(a, ActionKind::Bleed));
        if played_first {
            pick_when(&mut e, play(0, 0, z));
        }
        pick_when(&mut e, Choice::DeclareBlock { minion: z });
        pick_when(&mut e, play(0, 1, a)); // Form of Mist: combat ends
        pick_when(&mut e, Choice::ContinueAction);
        if !played_first {
            // Locked by its block: the wake, in the continued action.
            pick_when(&mut e, play(0, 0, z));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let want = if played_first { 4 } else { 4 - 1 };
        assert_eq!(blood(&e, z), want, "played first: {played_first}");
    }
    // In the as-played window of a's Lost in Crowds.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["obf"], 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.state.minions[z.index()].locked = true;
    e.add_minion(SeatId(1), plain("", "", 3), 4, false);
    hand(&mut e, 0, &["Lost in Crowds"]);
    hand(&mut e, 1, &["Forced Awakening"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(
        &mut e,
        Choice::DeclareBlock {
            minion: MinionId(3),
        },
    );
    pick_when(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert!(req.options.contains(&play(0, 0, z)));
}

// ---- Pool actions, wave 1 (slice 11u) ---------------------------------------

// Social Charm [PRE]: "Ⓓ Bleed with +1 bleed, and gain 1 pool if the bleed
// is successful (for 1 or more)."
#[test]
fn social_charm_superior_gains_a_pool_on_a_successful_bleed() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRE"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    hand(&mut e, 0, &["Social Charm"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((pool(&e, 0), pool(&e, 1)), (31, 28));
}

// Scouting Mission [DOM]: "+1 stealth action. Add 2 blood to a younger
// vampire in your uncontrolled region."
#[test]
fn scouting_mission_superior_feeds_a_younger_uncontrolled_vampire() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["DOM"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let cap = CRYPT[with(&["DOM"], 0) as usize].capacity;
    let young = uncontrolled(&mut e, 0, vampire(|c| c.capacity < cap, 0));
    hand(&mut e, 0, &["Scouting Mission"]);
    pick(&mut e, play_at(0, 1, Some(a), Target::Uncontrolled(young)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[young as usize].counters, 2);
}

// [ANK 20240706] Public Trust [PRE]: "Ⓓ Bleed with +2 bleed, and if the
// bleed is successful (for 1 or more), add 1 blood to a vampire in your
// uncontrolled region." "The target uncontrolled vampire is chosen upon
// declaration."
#[test]
fn public_trust_superior_feeds_the_uncontrolled_vampire_named_as_played() {
    for reduced in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["PRE"], 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let slot = uncontrolled(&mut e, 0, plain("", "", 2));
        hand(&mut e, 0, &["Public Trust"]);
        if reduced {
            // One Telepathic Counter each: a minion plays the same
            // reaction once in an action (rules-qa 46.1).
            e.add_minion(SeatId(1), with(&["AUS"], 0), 4, false);
            e.add_minion(SeatId(1), with(&["AUS"], 1), 4, false);
            hand(&mut e, 1, &["Telepathic Counter", "Telepathic Counter"]);
        }
        pick(&mut e, play_at(0, 1, Some(a), Target::Uncontrolled(slot)));
        if reduced {
            pick_when(&mut e, play(0, 1, MinionId(2)));
            pick_when(&mut e, play(0, 1, MinionId(3)));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let fed = e.state.seats[0].uncontrolled[slot as usize].counters;
        assert_eq!(fed, u8::from(!reduced), "only if the bleed is successful");
        assert_eq!(blood(&e, a), 3, "1 blood");
    }
}

// Restoration: "+1 stealth action. [for] This vampire gains 2 blood. [FOR]
// This vampire gains 3 blood."
#[test]
fn restoration_gains_blood_up_to_capacity() {
    let card = with(&["FOR"], 0);
    let cap = CRYPT[card as usize].capacity;
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), card, cap - 2, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    hand(&mut e, 0, &["Restoration"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), cap, "3 gained, capped");
}

// Fifth Tradition: Hospitality: "+1 stealth action. Requires a prince or
// justicar. Add 4 blood to another vampire." Any Methuselah's: at one
// across the table, directed at them, who may block (rules-qa 20.1: Ⓓ is
// only indicative); an untitled vampire cannot play it.
#[test]
fn fifth_tradition_feeds_another_vampire() {
    let mut e = Engine::bare(4);
    let p = e.add_minion(SeatId(0), prince(0), 4, false);
    let u = e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z_card = with(&["aus"], 0);
    let z = e.add_minion(SeatId(2), z_card, 1, false);
    hand(&mut e, 2, &["Enhanced Senses"]);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    hand(&mut e, 0, &["Fifth Tradition: Hospitality"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&play_at(0, 0, Some(u), Target::Minion(z))));
    assert!(!req
        .options
        .contains(&play_at(0, 0, Some(p), Target::Minion(p))));
    pick(&mut e, play_at(0, 0, Some(p), Target::Minion(z)));
    let req = until(&mut e, SeatId(2), WindowLabel::ActionBlock);
    assert!(req.options.contains(&Choice::DeclareBlock { minion: z }));
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let cap = CRYPT[z_card as usize].capacity;
    assert_eq!(blood(&e, z), 5.min(cap));
    assert_eq!(blood(&e, p), 3, "1 blood");
}

// Fourth Tradition: The Accounting: "+1 stealth action. Requires a prince or
// justicar. Add 3 blood to a younger vampire in your uncontrolled region."
#[test]
fn fourth_tradition_feeds_a_younger_uncontrolled_vampire() {
    let mut e = Engine::bare(3);
    let p = e.add_minion(SeatId(0), prince(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let cap = CRYPT[prince(0) as usize].capacity;
    let young = uncontrolled(&mut e, 0, vampire(|c| c.capacity < cap, 0));
    hand(&mut e, 0, &["Fourth Tradition: The Accounting"]);
    pick(&mut e, play_at(0, 0, Some(p), Target::Uncontrolled(young)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[young as usize].counters, 3);
}

// Umbrous Clutch [OBL]: "+1 stealth action. Ⓓ Enter combat with a minion,
// and this acting vampire gets 1 optional maneuver during that combat."
#[test]
fn umbrous_clutch_superior_enters_combat_with_a_maneuver() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["OBL"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Umbrous Clutch"]);
    pick(&mut e, play_at(0, 1, Some(a), Target::Minion(z)));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4, "a hand strike");
}

// Hunting the Beast [AUS]: "+1 stealth action. Ⓓ Enter combat with a
// vampire, and at the end of that combat, if this vampire is ready and the
// opposing vampire is not, add 2 blood to a Salubri in your uncontrolled
// region." The Salubri is chosen then; none if the opposing vampire is
// still ready.
#[test]
fn hunting_the_beast_superior_feeds_an_uncontrolled_salubri_after_winning() {
    for won in [false, true] {
        let mut e = Engine::bare(3);
        let card = vampire(|c| c.clan == "Salubri" && has(c, &["AUS"]), 0);
        let a = e.add_minion(SeatId(0), card, 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(
            SeatId(1),
            plain("Ventrue", "", 0),
            u8::from(!won) * 4,
            false,
        );
        let other = uncontrolled(&mut e, 0, plain("Ventrue", "", 1));
        // Any Salubri crypt cards: their texts do not matter uncontrolled.
        let mut salubris =
            (0..CRYPT.len() as u16).filter(|&i| CRYPT[i as usize].clan == "Salubri" && i != card);
        let s1 = uncontrolled(&mut e, 0, salubris.next().expect("a Salubri"));
        let s2 = uncontrolled(&mut e, 0, salubris.next().expect("a Salubri"));
        hand(&mut e, 0, &["Hunting the Beast"]);
        pick(&mut e, play_at(0, 1, Some(a), Target::Minion(z)));
        if won {
            let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
            assert_eq!(req.options, vec![Choice::Slot(s1), Choice::Slot(s2)]);
            pick(&mut e, Choice::Slot(s2));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[z.index()].in_torpor == won);
        let u = &e.state.seats[0].uncontrolled;
        assert_eq!(
            (
                u[other as usize].counters,
                u[s1 as usize].counters,
                u[s2 as usize].counters
            ),
            (0, 0, if won { 2 } else { 0 })
        );
    }
}

// Make the Misere: "Requires an Anarch. More than one Discipline can be
// used to play this card. Ⓓ Enter combat with a locked minion. [cel] This
// Anarch gets 1 optional maneuver during that combat. [obf] This is a +1
// stealth action. [pot] This Anarch gets +1 strength during that combat."
// [LSJ 20011204-3] [PIB 20130704] "When played, a multi-Disciplines card
// counts as requiring the Discipline(s) being used": the modes, each set
// of Disciplines the Anarch has.
#[test]
fn make_the_misere_gives_what_each_discipline_used_adds() {
    let card = vampire(
        |c| {
            c.sect == zvtes::crypt::Sect::Anarch
                && has(c, &["pot"])
                && !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("cel"))
        },
        0,
    );
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), card, 5, false);
    let other = e.add_minion(
        SeatId(0),
        vampire(|c| c.sect != zvtes::crypt::Sect::Anarch, 0),
        4,
        false,
    );
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    let free = e.add_minion(SeatId(1), plain("Ventrue", "", 1), 5, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Make the Misere"]);
    let req = request(&mut e);
    let misere = |mode, by, t| play_at(0, mode, Some(by), Target::Minion(t));
    assert!(
        !req.options.contains(&misere(2, other, z)),
        "an Anarch only"
    );
    assert!(
        !req.options.contains(&misere(2, a, free)),
        "a locked minion"
    );
    assert!(!req.options.contains(&misere(0, a, z)), "no Celerity");
    pick(&mut e, misere(2, a, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 3, "a hand strike at +1 strength");
}

// Mind Numb [PRE]: "Ⓓ Stun an unlocked vampire; +1 stealth action." Stunned:
// locked, and it does not unlock during its controller's next unlock
// phase.
#[test]
fn mind_numb_stuns_through_the_next_unlock_phase() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRE"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Mind Numb"]);
    pick(&mut e, play_at(0, 1, Some(a), Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].locked);
    assert_eq!(blood(&e, a), 3, "1 blood");
    // Seat 1's turn passes with nothing to do; back to seat 0, z still
    // locked; it unlocks as normal the turn after.
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&action(a, ActionKind::Bleed)));
    assert!(
        e.state.minions[z.index()].locked,
        "stunned through its unlock phase"
    );
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].locked);
}

// Propaganda: "[pre] Ⓓ Bleed with +1 bleed. Titled vampires cannot block
// this action. [PRE] As above, and the target Methuselah locks a ready
// unlocked minion they control" — the one the player names as the card is
// played (rules-qa 31.2).
#[test]
fn propaganda_bars_titled_blockers_and_locks_a_prey_minion() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRE"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let p = e.add_minion(SeatId(1), prince(0), 4, false);
    let u = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    hand(&mut e, 0, &["Propaganda"]);
    let torpid = e.add_minion(SeatId(1), plain("Ventrue", "", 1), 4, true);
    let req = request(&mut e);
    let named = |m| play_at(0, 1, Some(a), Target::Minion(m));
    assert!(req.options.contains(&named(u)));
    assert!(!req.options.contains(&named(torpid)), "ready ones only");
    pick(&mut e, named(p));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&Choice::DeclareBlock { minion: u }));
    assert!(!req.options.contains(&Choice::DeclareBlock { minion: p }));
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[p.index()].locked && !e.state.minions[u.index()].locked);
    assert_eq!((pool(&e, 1), blood(&e, a)), (28, 2));
}

// Conceal: "[obf] Ⓓ Burn an equipment. [OBF] Ⓓ Burn a location." [LSJ
// 20090324] Directed at the Methuselah controlling it, not the minion it is
// on. Living Manse in play is a location, not an equipment.
#[test]
fn conceal_burns_an_equipment_or_a_location() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["OBF"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    let t = e.add_minion(
        SeatId(2),
        vampire(|c| c.clan == "Tzimisce" && c.title == Title::None, 0),
        4,
        false,
    );
    let gun = equip(&mut e, ".44 Magnum", z);
    let manse = equip(&mut e, "Living Manse", t);
    hand(&mut e, 0, &["Conceal", "Conceal"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&play_at(0, 0, Some(a), Target::Card(manse))));
    assert!(req
        .options
        .contains(&play_at(0, 1, Some(a), Target::Card(manse))));
    assert!(!req
        .options
        .contains(&play_at(0, 1, Some(a), Target::Card(gun))));
    pick(&mut e, play_at(0, 0, Some(a), Target::Card(gun)));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&Choice::DeclareBlock { minion: z }));
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[gun.index()].in_play);
}

// Dominate Kine [DOM]: "Ⓓ Steal a location." [RTR 19980623] Its controller
// changes and nothing else. [RTR 19960112] One on a minion (Living Manse)
// moves onto a minion of the new controller's able to hold it.
#[test]
fn dominate_kine_superior_steals_a_location() {
    for manse in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["DOM"], 0), 4, false);
        let tz = vampire(|c| c.clan == "Tzimisce" && c.title == Title::None, 0);
        let t = e.add_minion(SeatId(0), tz, 4, false);
        let z = e.add_minion(
            SeatId(1),
            vampire(|c| c.clan == "Tzimisce" && c.title == Title::None, 1),
            4,
            false,
        );
        let place = if manse {
            equip(&mut e, "Living Manse", z)
        } else {
            location(&mut e, 1, "Elysian Fields")
        };
        hand(&mut e, 0, &["Dominate Kine"]);
        pick(&mut e, play_at(0, 1, Some(a), Target::Card(place)));
        if manse {
            let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
            assert_eq!(
                req.options,
                vec![Choice::ChooseMinion(a), Choice::ChooseMinion(t)]
            );
            pick(&mut e, Choice::ChooseMinion(t));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.card_controller(place), SeatId(0));
        assert!(e.state.cards[place.index()].in_play);
        if manse {
            assert_eq!(e.state.cards[place.index()].host, Some(t), "the Tzimisce");
        }
        assert_eq!(blood(&e, a), 2, "2 blood");
    }
}

// Graverobbing: "[dom] Ⓓ Steal a vampire in torpor controlled by another
// Methuselah. [DOM] As above, and this acting vampire can burn 2 blood to
// move the stolen vampire to your ready region." [ANK 20210124] [LSJ
// 20070411] [LSJ 20090514] Out of torpor by resolution: the action
// fizzles. Stolen for good, its cards go
// with it; burned, it goes to its owner's ash heap.
#[test]
fn graverobbing_superior_steals_a_torpid_vampire_and_may_rescue_it() {
    for (pay, fizzle) in [(false, false), (true, false), (false, true)] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["DOM"], 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 1, true);
        let gun = equip(&mut e, ".44 Magnum", z);
        hand(&mut e, 0, &["Graverobbing"]);
        pick(&mut e, play_at(0, 1, Some(a), Target::Minion(z)));
        if fizzle {
            e.state.minions[z.index()].in_torpor = false;
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.minions[z.index()].owner, SeatId(1));
            assert_eq!(e.last_outcome, Some(Outcome::Fizzled));
            continue;
        }
        let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        assert_eq!(req.options, vec![Choice::Pass, Choice::ChooseMinion(z)]);
        pick(
            &mut e,
            if pay {
                Choice::ChooseMinion(z)
            } else {
                Choice::Pass
            },
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[z.index()];
        assert_eq!(
            (v.owner, v.origin, v.in_torpor),
            (SeatId(0), Some(SeatId(1)), !pay)
        );
        assert_eq!(e.state.card_controller(gun), SeatId(0));
        assert_eq!(blood(&e, a), if pay { 2 } else { 4 });
    }
}

// [ANK 20210124] [LSJ 20070411] [LSJ 20090514] "If the action is unblocked
// when it resolves and the target is locked (Mind Numb) / unlocked (Make
// the Misere), the action fizzles (cost, if any, is paid, and it is
// considered successful, but has no effect)."
#[test]
fn mind_numb_and_make_the_misere_fizzle_on_a_target_no_longer_eligible() {
    for misere in [false, true] {
        let mut e = Engine::bare(3);
        let card = if misere {
            vampire(
                |c| c.sect == zvtes::crypt::Sect::Anarch && has(c, &["pot"]),
                0,
            )
        } else {
            with(&["PRE"], 0)
        };
        let a = e.add_minion(SeatId(0), card, 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
        e.state.minions[z.index()].locked = misere;
        if misere {
            hand(&mut e, 0, &["Make the Misere"]);
            pick(&mut e, play_at(0, 2, Some(a), Target::Minion(z)));
        } else {
            hand(&mut e, 0, &["Mind Numb"]);
            pick(&mut e, play_at(0, 0, Some(a), Target::Minion(z)));
        }
        e.state.minions[z.index()].locked = !misere;
        never_offered(&mut e, &Choice::HandStrike);
        assert_eq!(e.last_outcome, Some(Outcome::Fizzled));
        let v = &e.state.minions[z.index()];
        assert_eq!((v.blood, v.skip_unlock), (5, false));
        assert_eq!(blood(&e, a), if misere { 4 } else { 3 }, "paid");
    }
}

// Piper's ally paid down to 0 pool ousts its Methuselah once, as its cost
// is paid: one victory point for the predator (fuzz: the card's own
// resolution ousted them a second time).
#[test]
fn piper_paid_down_to_zero_ousts_once() {
    let mut e = Engine::bare(3);
    let brujah = vampire(
        |c| c.sect == zvtes::crypt::Sect::Anarch && c.clan == "Brujah" && c.title == Title::None,
        0,
    );
    let a = e.add_minion(SeatId(0), brujah, 3, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    hand(&mut e, 0, &["Piper", "Crypt's Sons"]);
    e.state.seats[0].pool = 1;
    e.at_master_phase();
    // The only retainer to employ: the terms are elided.
    pick(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(1), "the turn passed");
    assert!(e.state.seats[0].ousted);
    assert_eq!(e.state.seats[2].half_vps, 2);
    assert_eq!(e.state.seats[2].pool, 30 + 6);
}

// ---- Pool actions, wave 2 (slice 11v) ---------------------------------------

/// A copy of `name` in play for `seat` with `counters`, no host.
fn in_play_with(e: &mut Engine, seat: u8, name: &str, counters: u8) -> CardId {
    let id = location(e, seat, name);
    e.state.cards[id.index()].counters = counters;
    id
}

/// Pass every decision (resolving mandatory riders) until `seat`'s
/// minion phase.
fn to_minion_phase(e: &mut Engine, seat: u8) -> DecisionRequest {
    until(e, SeatId(seat), WindowLabel::MinionPhase)
}

// Army of Rats [ani]: "+1 stealth action. Put this card in play. During
// your unlock phase, your prey burns 1 pool. You can burn only 1 pool each
// turn with Army of Rats cards."
#[test]
fn army_of_rats_burns_one_prey_pool_a_turn_whatever_the_copies() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ani"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    hand(&mut e, 0, &["Army of Rats"]);
    pick(&mut e, play(0, 0, a));
    let req = to_minion_phase(&mut e, 0);
    let rats = find_library("Army of Rats");
    assert!(e.state.cards.iter().any(|c| c.in_play && c.card == rats));
    in_play_with(&mut e, 0, "Army of Rats", 0);
    let p1 = pool(&e, 1);
    let _ = req;
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, 0);
    assert_eq!(pool(&e, 1), p1 - 1, "one pool for two copies");
}

// Judgment: Camarilla Segregation: "Each Methuselah controlling a
// non-Camarilla vampire burns 1 pool during their unlock phase.
// Methuselahs can burn a non-Camarilla vampire they control during their
// master phase to burn this card." [ANK 20200703-4] Copies stack. [ANK
// 20181016] The condition is read as the burn resolves.
#[test]
fn judgment_camarilla_segregation_taxes_non_camarilla_methuselahs() {
    let mut e = Engine::bare(3);
    let p = e.add_minion(SeatId(0), prince(0), 4, false);
    let cam = |c: &zvtes::crypt::CryptCard| c.sect == zvtes::crypt::Sect::Camarilla;
    e.add_minion(
        SeatId(0),
        vampire(|c| cam(c) && c.title == Title::None, 0),
        4,
        false,
    );
    let x = e.add_minion(SeatId(1), anarch(0), 4, false);
    e.add_minion(
        SeatId(1),
        vampire(|c| cam(c) && c.title == Title::None, 1),
        4,
        false,
    );
    e.add_minion(
        SeatId(2),
        vampire(|c| cam(c) && c.title == Title::None, 2),
        4,
        false,
    );
    hand(&mut e, 0, &["Judgment: Camarilla Segregation"]);
    pick(&mut e, play(0, 0, p));
    to_minion_phase(&mut e, 0);
    let j2 = in_play_with(&mut e, 0, "Judgment: Camarilla Segregation", 0);
    let (p0, p1, p2) = (pool(&e, 0), pool(&e, 1), pool(&e, 2));
    pick(&mut e, Choice::Pass);
    // Seat 1's unlock phase: 1 pool per copy; then its master phase, where
    // it burns its Anarch to burn one copy.
    until(&mut e, SeatId(1), WindowLabel::MasterPhase);
    assert_eq!(pool(&e, 1), p1 - 2, "two copies");
    let burn = use_on(j2, 0, None, Target::Minion(x));
    pick(&mut e, burn);
    to_minion_phase(&mut e, 0);
    assert!(!e.state.minions[x.index()].in_play && !e.state.cards[j2.index()].in_play);
    assert_eq!(
        (pool(&e, 0), pool(&e, 2)),
        (p0, p2),
        "only Camarilla vampires"
    );
}

// Tier of Souls [ANI]: "Ⓓ Steal 1 blood or life from a minion controlled
// by your prey, and put this card on this vampire. This vampire gets +1
// bleed against your prey." An ally left with no life is burned.
#[test]
fn tier_of_souls_superior_steals_blood_and_bleeds_the_prey_harder() {
    for ally in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["ANI"], 0), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = if ally {
            e.add_ally(SeatId(1), find_library("Screamer"))
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false)
        };
        hand(&mut e, 0, &["Tier of Souls"]);
        pick(&mut e, play_at(0, 1, Some(a), Target::Minion(z)));
        to_minion_phase(&mut e, 0);
        assert_eq!(blood(&e, a), 4);
        if ally {
            assert!(!e.state.minions[z.index()].in_play, "no life left");
        } else {
            assert_eq!(blood(&e, z), 3);
        }
        assert_eq!(e.state.bleed_against_mod(a, SeatId(1)), 1);
        assert_eq!(e.state.bleed_against_mod(a, SeatId(2)), 0);
        // Its next bleed of the prey: 2.
        let p1 = pool(&e, 1);
        pick(&mut e, Choice::Pass);
        to_minion_phase(&mut e, 0);
        pick(&mut e, action(a, ActionKind::Bleed));
        to_minion_phase(&mut e, 0);
        assert_eq!(pool(&e, 1), p1 - 2);
    }
}

// Phantasmagoria [PRE]: "+1 stealth action. Ⓓ Put this card on a minion;
// you still control this card. The attached minion gets -1 stealth. …
// and if the attached minion is blocked, they burn 1 blood or life before
// block resolution." [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR
// 19991206] [LSJ 20091125] Applied as the block succeeds, before any other
// card or effect.
#[test]
fn phantasmagoria_superior_costs_the_blocked_minion_a_blood() {
    let mut e = Engine::bare(3);
    let card = vampire(|c| c.clan == "Ravnos" && has(c, &["PRE"]), 0);
    let a = e.add_minion(SeatId(0), card, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    let w = e.add_minion(SeatId(2), plain("Ventrue", "", 1), 4, false);
    hand(&mut e, 0, &["Phantasmagoria"]);
    pick(&mut e, play_at(0, 1, Some(a), Target::Minion(z)));
    to_minion_phase(&mut e, 0);
    let ph = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[ph.index()].host, Some(z));
    assert_eq!(e.state.card_controller(ph), SeatId(0), "still yours");
    assert_eq!(blood(&e, a), 3, "1 blood");
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, 1);
    bleed_blocked(&mut e, z, w);
    request(&mut e);
    // The combat's hand strikes: 1 each; z burned 1 more as it was blocked.
    assert_eq!((blood(&e, z), blood(&e, w)), (2, 3));
}

// Aranthebes, The Immortal: "+1 stealth action. Unique. Put this card in
// play. You can lock Aranthebes to give a minion controlled by your
// predator -1 stealth. While Aranthebes is unlocked, vampires with
// capacity 4 or less get -1 bleed against you. Vampires with capacity 5 or
// more can shuffle Aranthebes into your library as a Ⓓ action." (1 pool.)
// [LSJ 20080604] Its action requires capacity 5 or more.
#[test]
fn aranthebes_shields_against_young_bleeds_and_shuffles_away() {
    let mut e = Engine::bare(3);
    let big = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let young = e.add_minion(SeatId(0), vampire(|c| c.capacity <= 4, 0), 3, false);
    let y2 = e.add_minion(SeatId(2), vampire(|c| c.capacity <= 4, 1), 3, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    hand(&mut e, 0, &["Aranthebes, The Immortal"]);
    pick(&mut e, play(0, 0, young));
    let req = to_minion_phase(&mut e, 0);
    let id = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(pool(&e, 0), 29, "1 pool");
    assert_eq!(e.state.bleed_against_mod(y2, SeatId(0)), -1);
    assert_eq!(e.state.bleed_against_mod(big, SeatId(1)), 0);
    assert!(req
        .options
        .contains(&use_on(id, 1, Some(big), Target::None)));
    assert!(!req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { source: AbilitySource::Card(c), ability: 1, actor: Some(m), .. }
            if *c == id && *m != big)));
    e.state.cards[id.index()].locked = true;
    assert_eq!(e.state.bleed_against_mod(y2, SeatId(0)), 0, "locked");
    e.state.cards[id.index()].locked = false;
    pick(&mut e, use_on(id, 1, Some(big), Target::None));
    to_minion_phase(&mut e, 1);
    assert!(!e.state.cards[id.index()].in_play);
    assert_eq!(
        e.state.seats[0].library,
        vec![find_library("Aranthebes, The Immortal")]
    );
}

// [LSJ 20020612] Aranthebes's lock: "Can be played/used any time during the
// action before resolution; a block attempt is not required" — on the
// acting minion of the predator.
#[test]
fn aranthebes_locks_for_minus_one_stealth_on_the_predators_action() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let id = in_play_with(&mut e, 0, "Aranthebes, The Immortal", 0);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(2), with(&["FOR"], 0), 2, false);
    hand(&mut e, 2, &["Restoration"]);
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, 2);
    // A +1 stealth action of the predator's: the lock takes the stealth.
    pick(&mut e, play(0, 1, z));
    let lock = use_on(id, 0, None, Target::None);
    offering(&mut e, lock);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    pick(&mut e, lock);
    assert!(e.state.cards[id.index()].locked);
    assert_eq!(e.action().expect("action").stealth(&e.state), 0);
}

// Fee Stake: New York: "+1 stealth action. Requires an Anarch with capacity
// 5 or more. Title. Put this card on this Anarch to represent the unique
// Anarch title of Baron of New York. While this Anarch is Brujah or Brujah
// antitribu, they get +1 vote during referendums they call.
// Vampires can call a referendum to burn this card as a +1 stealth
// political action; during that referendum, non-Anarch titles are worth -1
// vote."
#[test]
fn fee_stake_represents_a_baron_and_its_burn_weakens_other_titles() {
    let mut e = Engine::bare(3);
    let t = e.add_minion(
        SeatId(0),
        vampire(
            |c| {
                c.clan == "Brujah"
                    && c.sect == zvtes::crypt::Sect::Anarch
                    && c.capacity >= 5
                    && c.title == Title::None
            },
            0,
        ),
        4,
        false,
    );
    let p = e.add_minion(SeatId(1), prince(0), 4, false);
    let b = e.add_minion(SeatId(1), baron(0), 4, false);
    e.add_minion(SeatId(2), plain("", "", 2), 4, false);
    hand(&mut e, 0, &["Fee Stake: New York"]);
    pick(&mut e, play(0, 0, t));
    to_minion_phase(&mut e, 0);
    assert_eq!(e.state.title_of(t), Title::Baron("New York"));
    let fee = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.calling_votes(t), 1, "a Brujah");
    // Seat 1's prince calls the burn: its title counts one less.
    pick(&mut e, Choice::Pass);
    let burn = use_on(fee, 0, Some(p), Target::None);
    pick_when(&mut e, burn);
    to_referendum(&mut e);
    let r = e.referendum().expect("a referendum");
    assert_eq!(r.votes_of(&e.state, p), 1, "a prince, -1");
    assert_eq!(r.votes_of(&e.state, b), 2, "a baron: an Anarch title");
    assert_eq!(r.votes_of(&e.state, t), 2, "not its caller");
}

// [LSJ 20070808-2] Fee Stake: "If the title is contested, the card is
// turned face down out of play and the provided action cannot be used."
#[test]
fn a_contested_fee_stake_provides_no_action() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch(0), 4, false);
    let b = e.add_minion(SeatId(1), anarch(1), 4, false);
    e.add_minion(SeatId(2), plain("", "", 2), 4, false);
    let f1 = attach(&mut e, "Fee Stake: Boston", a, 0);
    attach(&mut e, "Fee Stake: Boston", b, 0);
    e.stage_contests(a);
    assert!(e.state.cards[f1.index()].contested);
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == f1)));
}

// Waters of Duat: "+1 stealth action. Requires a non-sterile Follower of
// Set with capacity 5 or more. Put this card in play. It becomes a
// 1-capacity (non-unique) Follower of Set and must hunt this turn." No sect
// printed: its sire's (rulebook "Sect"; rules-qa 32.6).
#[test]
fn waters_of_duat_creates_a_follower_of_set_that_must_hunt() {
    let mut e = Engine::bare(3);
    let m = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Ministry" && c.capacity >= 5, 0),
        4,
        false,
    );
    let small = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Ministry" && c.capacity < 5, 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    hand(&mut e, 0, &["Waters of Duat"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play(0, 0, small)),
        "capacity 5 or more"
    );
    pick(&mut e, play(0, 0, m));
    let req = to_minion_phase(&mut e, 0);
    let v = MinionId(e.state.minions.len() as u32 - 1);
    let new = &e.state.minions[v.index()];
    assert_eq!(
        (new.clan(), e.state.sect_of(v), e.state.capacity_of(v)),
        (
            Some("Ministry"),
            Some(
                zvtes::crypt::CRYPT[e.state.minions[m.index()].crypt_card().unwrap() as usize].sect
            ),
            Some(1)
        )
    );
    // Its hunt is its only option then: taken at once.
    assert_eq!(blood(&e, v), 1, "it hunted");
    assert!(!req.options.contains(&action(v, ActionKind::Hunt)));
    assert_eq!(blood(&e, m), 3, "1 blood");
}

// Split the Veil: "Put this card in play with 2 counters. During your
// unlock phase, you can burn 1 counter from this card. You can burn
// counters from no more than two Split the Veil cards each unlock phase.
// If this card has no counters, burn it and move a wraith or zombie ally
// from your ash heap to your ready region with life equal to its starting
// life." [LSJ 20100204] [LSJ 20040518-2] [LSJ 20100302-1] Requirements do
// not apply; the ally can act this turn. [RBK equip] [RBK recruit-ally]
// [RBK employ-retainer] It comes in play at its inferior version.
#[test]
fn split_the_veil_raises_a_wraith_or_zombie_from_the_ash_heap() {
    let mut e = Engine::bare(3);
    let h = e.add_minion(SeatId(0), vampire(|c| c.clan == "Hecata", 0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    hand(&mut e, 0, &["Split the Veil"]);
    pick(&mut e, play(0, 0, h));
    to_minion_phase(&mut e, 0);
    let fresh = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[fresh.index()].counters, 2);
    let s1 = in_play_with(&mut e, 0, "Split the Veil", 1);
    let s2 = in_play_with(&mut e, 0, "Split the Veil", 1);
    e.state.seats[0].ash = vec![
        find_library("Bone Shambler"),
        find_library("Screamer"),
        find_library("Political Ally"),
    ];
    pick(&mut e, Choice::Pass);
    let tick = |c| use_on(c, 0, None, Target::None);
    pick_when(&mut e, tick(s1));
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![
            Choice::FindCard(find_library("Bone Shambler")),
            Choice::FindCard(find_library("Screamer")),
        ]
    );
    pick(&mut e, Choice::FindCard(find_library("Screamer")));
    request(&mut e);
    assert!(!e.state.cards[s1.index()].in_play);
    pick(&mut e, tick(s2));
    let req = request(&mut e);
    assert!(!req.options.contains(&tick(fresh)), "two copies this phase");
    let req = to_minion_phase(&mut e, 0);
    let raised: Vec<MinionId> = (0..e.state.minions.len() as u32)
        .map(MinionId)
        .filter(|&m| e.state.minions[m.index()].is_ally() && e.state.minions[m.index()].in_play)
        .collect();
    assert_eq!(raised.len(), 2);
    for &m in &raised {
        let id = e.state.minions[m.index()]
            .ally_id()
            .expect("an ally's card");
        assert_eq!(e.state.cards[id.index()].mode, 0, "inferior version");
    }
    for m in raised {
        assert!(
            req.options.iter().any(|o| matches!(o,
            Choice::TakeAction { minion, .. } if *minion == m)),
            "acts this turn"
        );
    }
    assert_eq!(e.state.cards[fresh.index()].counters, 2);
}

// Psychophagia: "+1 stealth hunt action. Remove an ally in any Methuselah's
// ash heap from the game to gain 3 blood or to gain 2 blood and unlock."
// [LSJ 20010627] [ANK 20180202] Undirected, whoever's ash heap. [ANK
// 20200420-3] Not playable with no ally in any ash heap. [RBK
// action-card-or-card-in-play] A minion's one special hunt a turn.
#[test]
fn psychophagia_eats_an_ash_heap_ally() {
    let mut e = Engine::bare(3);
    let h = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Hecata" && c.capacity >= 6, 0),
        1,
        false,
    );
    let h2 = e.add_minion(SeatId(0), vampire(|c| c.clan == "Hecata", 1), 1, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    hand(&mut e, 0, &["Psychophagia", "Psychophagia"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play(0, 1, h)),
        "no ally in any ash heap"
    );
    e.state.seats[2].ash.push(find_library("Screamer"));
    pick(&mut e, play(0, 1, h));
    // Undirected: no one else may block without intercept; the only ally
    // is taken without a choice.
    to_minion_phase(&mut e, 0);
    assert_eq!(blood(&e, h), 3);
    assert!(!e.state.minions[h.index()].locked, "unlocked");
    assert!(e.state.seats[2].removed.contains(&find_library("Screamer")));
    e.state.seats[1].ash.push(find_library("Screamer"));
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, h2)));
    assert!(!req.options.contains(&play(0, 0, h)), "once a turn");
}

// Khabar: Glory: "+1 stealth action. Unique. Not usable if any
// non-mandatory actions have been performed this turn. Put this card on
// this Assamite and unlock them. This Assamite gets +1 bleed. If your prey
// is ousted, you gain 4 additional pool. Burn this card during your unlock
// phase."
#[test]
fn khabar_glory_is_a_first_action_that_pays_for_an_oust() {
    for late in [false, true] {
        let mut e = Engine::bare(3);
        let k = e.add_minion(
            SeatId(0),
            vampire(|c| c.clan == "Banu Haqim" && c.title == Title::None, 0),
            4,
            false,
        );
        let o = e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        e.add_minion(SeatId(1), plain("", "", 2), 4, false);
        e.add_minion(SeatId(2), plain("", "", 3), 4, false);
        hand(&mut e, 0, &["Khabar: Glory"]);
        if late {
            pick(&mut e, action(o, ActionKind::Hunt));
            let req = to_minion_phase(&mut e, 0);
            assert!(!req.options.contains(&play(0, 0, k)), "an action performed");
            continue;
        }
        pick(&mut e, play(0, 0, k));
        let req = to_minion_phase(&mut e, 0);
        assert!(!e.state.minions[k.index()].locked, "unlocked");
        let kg = CardId(e.state.cards.len() as u32 - 1);
        assert_eq!(e.state.cards[kg.index()].host, Some(k));
        e.state.seats[1].pool = 2;
        let _ = req;
        pick(&mut e, action(k, ActionKind::Bleed));
        to_minion_phase(&mut e, 0);
        assert!(e.state.seats[1].ousted);
        assert_eq!(pool(&e, 0), 30 + 6 + 4);
        pick(&mut e, Choice::Pass);
        to_minion_phase(&mut e, 0);
        assert!(
            !e.state.cards[kg.index()].in_play,
            "burned in the unlock phase"
        );
    }
}

// Open War: "Requires a baron. Only one Open War can be played in a game.
// Put this card in play. Anarchs can enter combat with a minion as a Ⓓ
// action. Anarchs can burn a location as a Ⓓ action that costs 2 pool.
// Methuselahs can use a master phase action to move 1 counter from their
// pool to this card. If this card has 4 counters, burn it and gain 4
// pool." [LSJ 20100909] The two actions are not cross-restrictive. [LSJ
// 20080604] Each requires an Anarch. [LSJ 20090324] The location burn is
// directed at its controller.
#[test]
fn open_war_grants_two_anarch_actions_and_pays_out_at_four_counters() {
    let mut e = Engine::bare(3);
    let b = e.add_minion(SeatId(0), baron(0), 4, false);
    let a = e.add_minion(SeatId(0), anarch(0), 4, false);
    let c = e.add_minion(
        SeatId(0),
        vampire(
            |c| c.sect == zvtes::crypt::Sect::Camarilla && c.title == Title::None,
            0,
        ),
        4,
        false,
    );
    let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    let place = location(&mut e, 1, "Elysian Fields");
    hand(&mut e, 0, &["Open War", "Open War"]);
    pick(&mut e, play(0, 0, b));
    let req = to_minion_phase(&mut e, 0);
    assert!(!req.options.contains(&play(1, 0, a)) && !req.options.contains(&play(0, 0, a)));
    let war = CardId(e.state.cards.len() as u32 - 1);
    let rush = use_on(war, 0, Some(a), Target::Minion(z));
    let raze = use_on(war, 1, Some(a), Target::Card(place));
    assert!(req.options.contains(&rush) && req.options.contains(&raze));
    assert!(
        !req.options
            .contains(&use_on(war, 0, Some(c), Target::Minion(z))),
        "an Anarch"
    );
    pick(&mut e, raze);
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(
        req.options.contains(&Choice::DeclareBlock { minion: z }),
        "directed"
    );
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, 0);
    assert!(!e.state.cards[place.index()].in_play);
    assert_eq!(pool(&e, 0), 28, "2 pool");
    e.state.minions[a.index()].locked = false;
    let req = request(&mut e);
    assert!(req.options.contains(&rush), "not cross-restrictive");
    // Three counters already: the fourth, moved by seat 0, pays it 4.
    e.state.cards[war.index()].counters = 3;
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(1), WindowLabel::MasterPhase);
    let feed = use_on(war, 2, None, Target::None);
    assert!(req.options.contains(&feed), "any Methuselah");
    let (p0, p1) = (pool(&e, 0), pool(&e, 1));
    pick(&mut e, feed);
    request(&mut e);
    assert!(!e.state.cards[war.index()].in_play);
    // Its controller gains the pool (rules-qa 32.7).
    assert_eq!((pool(&e, 0), pool(&e, 1)), (p0 + 4, p1 - 1));
}

// [ANK 20181016] Judgment: Camarilla Segregation: "The effect must be
// applied during the unlock phase, but other effects can be applied
// before. If an effect applied before burns the last non-Camarilla
// vampire, no pool is lost."
#[test]
fn judgment_reads_the_non_camarilla_vampire_as_it_resolves() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), prince(0), 4, false);
    let x = e.add_minion(SeatId(1), anarch(0), 4, false);
    e.add_minion(SeatId(2), prince(1), 4, false);
    in_play_with(&mut e, 0, "Judgment: Camarilla Segregation", 0);
    in_play_with(&mut e, 0, "Judgment: Camarilla Segregation", 0);
    let p1 = pool(&e, 1);
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    assert_eq!(req.options.len(), 2, "two burns due");
    // Another effect burns the Anarch first.
    e.state.minions[x.index()].in_play = false;
    to_minion_phase(&mut e, 2);
    assert_eq!(pool(&e, 1), p1);
}

// Fee Stake: New York: "While this Anarch is Brujah or Brujah antitribu,
// they get +1 vote during referendums they call" — the burn of its own
// card included; a baron's title is an Anarch one, not reduced.
#[test]
fn fee_stake_gives_its_brujah_a_vote_in_the_referendums_it_calls() {
    let mut e = Engine::bare(3);
    let t = e.add_minion(
        SeatId(0),
        vampire(
            |c| {
                c.clan == "Brujah"
                    && c.sect == zvtes::crypt::Sect::Anarch
                    && c.capacity >= 5
                    && c.title == Title::None
            },
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    e.add_minion(SeatId(2), plain("", "", 2), 4, false);
    let fee = attach(&mut e, "Fee Stake: New York", t, 0);
    e.stage_contests(t);
    assert_eq!(e.state.title_of(t), Title::Baron("New York"));
    pick_when(&mut e, use_on(fee, 0, Some(t), Target::None));
    to_referendum(&mut e);
    let r = e.referendum().expect("a referendum");
    assert_eq!(r.votes_of(&e.state, t), 3, "2 for a baron, +1 calling");
}

// Psychophagia: the ally is named as the card is played — the ash heap,
// then the ally (rules-qa 32.12); a vampire that must hunt may hunt with it
// (rules-qa 32.13).
#[test]
fn psychophagia_names_its_ally_at_play_and_serves_a_forced_hunt() {
    let mut e = Engine::bare(3);
    let h = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Hecata" && c.capacity >= 6, 0),
        0,
        false,
    );
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    hand(&mut e, 0, &["Psychophagia"]);
    e.state.seats[1].ash = vec![find_library("Screamer")];
    e.state.seats[2].ash = vec![find_library("Screamer"), find_library("Bone Shambler")];
    // No blood: the vampire must hunt — the rulebook hunt or this one.
    let req = request(&mut e);
    assert!(req.options.contains(&action(h, ActionKind::Hunt)));
    assert!(req.options.contains(&play(0, 0, h)));
    assert!(!req.options.contains(&Choice::Pass));
    pick(&mut e, play(0, 0, h));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(
        req.options,
        vec![Choice::ChooseSeat(SeatId(1)), Choice::ChooseSeat(SeatId(2))]
    );
    pick(&mut e, Choice::ChooseSeat(SeatId(2)));
    let bone = find_library("Bone Shambler");
    pick(&mut e, Choice::FindCard(bone));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[2].removed, vec![bone]);
    assert_eq!(e.state.seats[1].ash.len(), 1);
    assert_eq!(blood(&e, h), 3);
}

// ---- Pool masters, wave 1 (slice 11w) ---------------------------------------

/// Seat 0's first minion calls Anarchist Uprising; decisions pass until
/// `want` is offered, which is taken; the referendum's records then.
fn in_referendum_take(e: &mut Engine, caller: MinionId, want: Choice) -> Vec<(SeatId, bool, u8)> {
    hand(e, 0, &["Anarchist Uprising"]);
    pick(e, play(e.state.seats[0].hand.len() as u8 - 1, 0, caller));
    pick_when(e, want);
    let r = e.referendum().expect("referendum");
    r.records
        .iter()
        .filter(|v| matches!(v.source, zvtes::referendum::Source::InPlay(_)))
        .map(|v| (v.seat, v.favor, v.n))
        .collect()
}

// The Parthenon: "During your master phase, you can lock this card to get
// +1 master phase action."
#[test]
fn the_parthenon_locks_for_a_master_phase_action() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let p = location(&mut e, 0, "Parthenon, The");
    e.at_master_phase();
    pick(&mut e, use_on(p, 0, None, Target::None));
    assert_eq!(e.state.seats[0].mpa, 2);
    assert!(e.state.cards[p.index()].locked);
}

// Dummy Corporation: "You can burn this card if a minion is bleeding you to
// reduce the bleed amount by 2."
#[test]
fn dummy_corporation_burns_to_reduce_a_bleed_by_two() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRE"], 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let d = location(&mut e, 1, "Dummy Corporation");
    hand(&mut e, 0, &["Intimidation"]);
    pick(&mut e, play(0, 1, a));
    pick_when(&mut e, use_on(d, 0, None, Target::None));
    to_minion_phase(&mut e, 0);
    assert_eq!(pool(&e, 1), 29, "3 - 2");
    assert!(!e.state.cards[d.index()].in_play);
}

// Ferraille: "Once each turn, you can burn 1 pool to get +3 votes during
// the polling step of any referendum." [ANK 20180719-4] Each player's turn.
// [LSJ 20081105] No votes if the cost ousts the player.
#[test]
fn ferraille_buys_three_votes_once_a_turn() {
    for ousted in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), anarch(0), 4, false);
        e.add_minion(SeatId(1), vampire(|c| c.clan == "Ministry", 0), 4, false);
        e.add_minion(SeatId(2), plain("", "", 2), 4, false);
        let f = location(&mut e, 1, "Ferraille");
        if ousted {
            e.state.seats[1].pool = 1;
        }
        let votes = in_referendum_take(&mut e, a, use_on(f, 0, None, Target::None));
        if ousted {
            assert!(e.state.seats[1].ousted);
            assert!(votes.is_empty(), "no votes");
        } else {
            assert_eq!(votes, vec![(SeatId(1), true, 3)]);
            assert_eq!(pool(&e, 1), 29);
            let req = request(&mut e);
            assert!(
                !req.options.contains(&use_on(f, 1, None, Target::None)),
                "once a turn"
            );
        }
    }
}

// Oxford University, England: "You can lock this card and burn X pool during
// the polling step of a political action to get +2X votes."
#[test]
fn oxford_university_buys_two_votes_a_pool() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch(0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(2), plain("", "", 2), 4, false);
    let o = location(&mut e, 1, "Oxford University, England");
    hand(&mut e, 0, &["Anarchist Uprising"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, use_on(o, 1, None, Target::None));
    let req = expect(&mut e, SeatId(1), WindowLabel::PlayTerms);
    assert_eq!(req.options.len(), 30, "X up to the pool");
    pick(&mut e, Choice::SetX(3));
    request(&mut e);
    let r = e.referendum().expect("referendum");
    assert!(r
        .records
        .iter()
        .any(|v| v.seat == SeatId(1) && !v.favor && v.n == 6));
    assert_eq!(pool(&e, 1), 27);
}

// The Black Throne: "You can lock this card during the polling step of any
// referendum to get +2 votes. You can lock this card after a minion with a
// contract for which an Assamite you control is chosen leaves the ready
// region to gain 1 pool." [PIB 20150512] Even when the contract burns with
// its minion. [ANK 20180129-2] Even when its controller is ousted.
#[test]
fn the_black_throne_pays_when_a_contracted_minion_leaves() {
    for oust in [false, true] {
        let mut e = Engine::bare(3);
        let h = e.add_minion(
            SeatId(0),
            vampire(|c| c.clan == "Banu Haqim" && c.title == Title::None, 0),
            4,
            false,
        );
        let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
        e.add_minion(SeatId(2), plain("", "", 3), 4, false);
        let pc = master_on(&mut e, 0, "Priority Contract", z);
        e.state.cards[pc.index()].chosen = vec![h];
        let bt = location(&mut e, 0, "Black Throne, The");
        let p0 = pool(&e, 0);
        if oust {
            e.state.seats[1].pool = 1;
            pick(&mut e, action(h, ActionKind::Bleed));
        } else {
            // The contract's rush sends z to torpor.
            e.state.minions[z.index()].blood = 0;
            pick(&mut e, use_on(pc, 0, Some(h), Target::Minion(z)));
        }
        pick_when(&mut e, Choice::ResolveRider { card: pc.0 });
        pick_when(&mut e, Choice::ResolveRider { card: bt.0 });
        request(&mut e);
        assert!(e.state.cards[bt.index()].locked);
        let extra = if oust { 6 } else { 0 };
        assert_eq!(pool(&e, 0), p0 + extra + 3 + 1);
    }
}

// Kumpania: "You can lock this card to give a Ravnos with capacity 5 or more
// you control +1 intercept." Sight Beyond Sight: "Put this card on a Salubri
// you control. This Salubri gets +1 intercept."
#[test]
fn kumpania_and_sight_beyond_sight_give_intercept() {
    for old_enough in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        e.add_minion(SeatId(0), plain("", "", 1), 4, false);
        let r = e.add_minion(
            SeatId(1),
            vampire(
                |c| c.clan == "Ravnos" && c.capacity >= 5 && c.title == Title::None,
                0,
            ),
            4,
            false,
        );
        let small = e.add_minion(
            SeatId(1),
            vampire(|c| c.clan == "Ravnos" && c.capacity < 5, 0),
            4,
            false,
        );
        let k = location(&mut e, 1, "Kumpania");
        let s = e.add_minion(SeatId(2), salubri(0), 4, false);
        master_on(&mut e, 2, "Sight Beyond Sight", s);
        assert_eq!(e.state.bearer_mod(s, Stat::Intercept, ActionKind::Bleed), 1);
        // A hunt: +1 stealth, intercept needed.
        pick(&mut e, action(a, ActionKind::Hunt));
        let lock = use_on(k, 0, None, Target::None);
        if old_enough {
            pick(&mut e, Choice::DeclareBlock { minion: r });
            pick_when(&mut e, lock);
            assert!(e.state.cards[k.index()].locked);
        } else {
            // Kumpania cannot reach the young Ravnos: its attempt, needing
            // intercept it cannot get, is not offered.
            let req = request(&mut e);
            assert!(req.options.contains(&Choice::DeclareBlock { minion: r }));
            assert!(!req
                .options
                .contains(&Choice::DeclareBlock { minion: small }));
        }
    }
}

// Charisma: "Put this card on a ready vampire. Recruit ally actions cost
// this vampire -1 blood or pool." [LSJ 20090115-1] [ANK 20170309] Not a
// recruit outside an action (Piper): "This is not an action, so cost
// reductions referring to an action do not apply".
#[test]
fn charisma_discounts_the_recruit_ally_action() {
    for piper in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(
            SeatId(0),
            vampire(
                |c| {
                    c.clan == "Gangrel"
                        && c.sect == zvtes::crypt::Sect::Anarch
                        && c.title == Title::None
                },
                0,
            ),
            4,
            false,
        );
        e.add_minion(SeatId(1), plain("", "", 1), 4, false);
        master_on(&mut e, 0, "Charisma", a);
        let p0 = pool(&e, 0);
        if piper {
            hand(&mut e, 0, &["Piper", "Double Deuce"]);
            e.at_master_phase();
            pick(&mut e, play(0, 0, a));
            request(&mut e);
            assert_eq!(pool(&e, 0), p0 - 2, "not an action: no discount");
        } else {
            hand(&mut e, 0, &["Double Deuce"]);
            pick(&mut e, play(0, 0, a));
            to_minion_phase(&mut e, 0);
            assert_eq!(pool(&e, 0), p0 - 1);
        }
    }
}

// Giant's Blood: "Only one Giant's Blood can be played in a game. Choose a
// vampire. The chosen vampire gains enough blood to reach full capacity."
// [LSJ 20061109] Not an uncontrolled one.
#[test]
fn giants_blood_fills_a_vampire_once_a_game() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 1, false);
    hand(&mut e, 0, &["Giant's Blood", "Giant's Blood"]);
    e.at_master_phase();
    e.state.seats[0].mpa = 2;
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| matches!(
        o,
        Choice::Play {
            target: Target::Uncontrolled(_),
            ..
        }
    )));
    pick(&mut e, play_at(0, 0, None, Target::Minion(z)));
    let req = request(&mut e);
    assert_eq!(blood(&e, z), e.state.capacity_of(z).unwrap());
    assert!(
        !req.options.iter().any(|o| matches!(o, Choice::Play { .. })),
        "once a game"
    );
}

// Golconda: Inner Peace: "Remove a vampire with capacity 8 or more from the
// game. Their controller gains pool equal to the vampire's capacity. Their
// controller can burn 2 pool to cancel this card as it is played."
#[test]
fn golconda_removes_an_elder_for_its_capacity_in_pool() {
    for cancel in [false, true] {
        let mut e = Engine::bare(3);
        e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        // Anja's text (unlocking another Hecata after its action) is inert here.
        let big = crypt("\"Mother\" Anja Giovanni");
        let z = e.add_minion(SeatId(1), big, 4, false);
        let small = e.add_minion(SeatId(1), vampire(|c| c.capacity < 8, 0), 4, false);
        // A card on it is burned as it leaves play (advanced rules
        // §1.12.1), not lost.
        let on = attach(&mut e, "Monster", z, 0);
        hand(&mut e, 0, &["Golconda: Inner Peace"]);
        e.at_master_phase();
        let req = request(&mut e);
        assert!(!req
            .options
            .contains(&play_at(0, 0, None, Target::Minion(small))));
        pick(&mut e, play_at(0, 0, None, Target::Minion(z)));
        let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
        assert!(req.options.contains(&Choice::PayToCancel));
        let cap = CRYPT[big as usize].capacity as i32;
        pick(
            &mut e,
            if cancel {
                Choice::PayToCancel
            } else {
                Choice::Pass
            },
        );
        request(&mut e);
        if cancel {
            assert!(e.state.minions[z.index()].in_play);
            assert_eq!(pool(&e, 1), 28);
        } else {
            assert!(!e.state.minions[z.index()].in_play);
            assert_eq!(e.state.seats[1].removed_crypt, vec![big]);
            assert_eq!(pool(&e, 1), 30 + cap);
            assert!(!e.state.cards[on.index()].in_play);
            assert_eq!(e.state.seats[1].ash, vec![find_library("Monster")]);
        }
    }
}

// Grooming the Protégé: "Move up to 3 blood from a ready vampire you control
// to a younger vampire of the same clan in your uncontrolled region."
#[test]
fn grooming_the_protege_feeds_a_younger_clanmate() {
    let mut e = Engine::bare(3);
    let card = plain("Ventrue", "", 0);
    let a = e.add_minion(SeatId(0), card, 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let cap = CRYPT[card as usize].capacity;
    let young = uncontrolled(
        &mut e,
        0,
        vampire(|c| c.clan == "Ventrue" && c.capacity < cap, 0),
    );
    let other = uncontrolled(
        &mut e,
        0,
        vampire(|c| c.clan == "Brujah" && c.capacity < cap, 0),
    );
    hand(&mut e, 0, &["Grooming the Protégé"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&play_at(0, 0, Some(a), Target::Uncontrolled(other))));
    pick(&mut e, play_at(0, 0, Some(a), Target::Uncontrolled(young)));
    pick(&mut e, Choice::SetX(3));
    request(&mut e);
    assert_eq!(blood(&e, a), 1);
    assert_eq!(e.state.seats[0].uncontrolled[young as usize].counters, 3);
    assert!(!e.state.minions[a.index()].locked, "no action");
}

// Gangrel Revel: "Gangrel you control get +1 strength. Non-Ravnos minions
// can burn this card as a Ⓓ action." The Khabar: Community: "Assamites get
// +1 stealth when bleeding." New Carthage: "Titled Brujah get +1 bleed and
// +1 vote. Ventrue get -1 vote." [ANK 20251125] [LSJ 20050221-2] Net
// negative votes count as none.
#[test]
fn clan_wide_masters_modify_their_clans() {
    let mut e = Engine::bare(3);
    let g0 = e.add_minion(SeatId(0), plain("Gangrel", "", 0), 4, false);
    let g1 = e.add_minion(SeatId(1), plain("Gangrel", "", 1), 4, false);
    let r = e.add_minion(SeatId(1), plain("Ravnos", "", 0), 4, false);
    let h = e.add_minion(SeatId(2), plain("Banu Haqim", "", 0), 4, false);
    let b = e.add_minion(SeatId(2), baron(0), 4, false);
    let v = e.add_minion(SeatId(2), plain("Ventrue", "", 0), 4, false);
    let revel = location(&mut e, 0, "Gangrel Revel");
    location(&mut e, 1, "Khabar: Community, The");
    location(&mut e, 1, "New Carthage");
    let str_of = |e: &Engine, m| e.state.bearer_mod(m, Stat::Strength, ActionKind::Bleed);
    assert_eq!((str_of(&e, g0), str_of(&e, g1)), (1, 0), "you control");
    assert_eq!(e.state.bearer_mod(h, Stat::Stealth, ActionKind::Bleed), 1);
    assert_eq!(e.state.bearer_mod(h, Stat::Stealth, ActionKind::Hunt), 0);
    let brujah_baron =
        CRYPT[e.state.minions[b.index()].crypt_card().unwrap() as usize].clan == "Brujah";
    assert_eq!(e.state.clan_votes(v), -1);
    if brujah_baron {
        assert_eq!(e.state.clan_votes(b), 1);
        assert_eq!(e.state.bearer_mod(b, Stat::Bleed, ActionKind::Bleed), 1);
    }
    // Seat 1's Ravnos cannot burn the Revel; its Gangrel can.
    pick(&mut e, Choice::Pass);
    let req = to_minion_phase(&mut e, 1);
    assert!(req
        .options
        .contains(&use_on(revel, 0, Some(g1), Target::None)));
    assert!(!req
        .options
        .contains(&use_on(revel, 0, Some(r), Target::None)));
}

// [ANK 20251125] [LSJ 20050221-2] New Carthage: "A vampire can have net negative votes, but
// net negative votes do not subtract from the total when votes are
// tallied."
#[test]
fn new_carthage_leaves_an_untitled_ventrue_no_votes() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch(0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let v = e.add_minion(SeatId(2), plain("Ventrue", "", 0), 4, false);
    location(&mut e, 1, "New Carthage");
    hand(&mut e, 0, &["Anarchist Uprising"]);
    pick(&mut e, play(0, 0, a));
    to_referendum(&mut e);
    let r = e.referendum().expect("a referendum");
    assert_eq!(r.votes_of(&e.state, v), 0);
}

// Mob Connections: "You can lock this card to give a minion you control 1
// press, only usable to continue combat."
#[test]
fn mob_connections_gives_a_press_to_continue() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 6, false);
    let mob = location(&mut e, 1, "Mob Connections");
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, use_on(mob, 0, None, Target::None));
    assert!(e.state.cards[mob.index()].locked);
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::UsePress), "{:?}", req.options);
}

// Fragment of the Book of Nod: "You can lock this card to draw 2 cards
// (discard down afterward)." [LSJ 20060620] From an empty library, draw what
// there is. [ANK 20200616] During an action too, when its controller has
// the impulse.
#[test]
fn fragment_of_the_book_of_nod_draws_two() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let f = location(&mut e, 1, "Fragment of the Book of Nod");
    e.state.seats[1].library = vec![find_library("Soak")];
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, use_on(f, 0, None, Target::None));
    assert_eq!(e.state.seats[1].hand, vec![find_library("Soak")]);
    assert!(e.state.cards[f.index()].locked);
}

// Brujah Debate: "During each Methuselah's master phase, that Methuselah
// locks one of the oldest Brujah they control (if any). Brujah get +1
// strength and 1 optional maneuver each combat." [LSJ 19980224] Their pick
// among the oldest. [LSJ 20010508] A locked one may be picked.
#[test]
fn brujah_debate_locks_an_oldest_brujah_each_master_phase() {
    let mut e = Engine::bare(3);
    let cap = |m: &MinionId, e: &Engine| e.state.capacity_of(*m).unwrap();
    let pick_cap = CRYPT[plain("Brujah", "", 0) as usize].capacity;
    let b1 = e.add_minion(
        SeatId(0),
        vampire(
            |c| c.clan == "Brujah" && c.capacity == pick_cap && c.title == Title::None,
            0,
        ),
        4,
        false,
    );
    let b2 = e.add_minion(
        SeatId(0),
        vampire(
            |c| c.clan == "Brujah" && c.capacity == pick_cap && c.title == Title::None,
            1,
        ),
        4,
        false,
    );
    let young = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Brujah" && c.capacity < pick_cap, 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    location(&mut e, 1, "Brujah Debate");
    assert_eq!(cap(&b1, &e), cap(&b2, &e));
    e.state.minions[b2.index()].locked = true;
    e.at_master_phase();
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::ChooseMinion(b1), Choice::ChooseMinion(b2)]
    );
    pick(&mut e, Choice::ChooseMinion(b2));
    request(&mut e);
    assert!(!e.state.minions[b1.index()].locked && !e.state.minions[young.index()].locked);
    assert_eq!(e.state.bearer_mod(b1, Stat::Strength, ActionKind::Bleed), 1);
    assert_eq!(e.state.clan_combat_maneuvers(young), 1);
}

// [RTR 20040501] Fragment of the Book of Nod: "Cannot be used when a card
// is played to draw a card that would cancel it 'as it is played'."
#[test]
fn fragment_of_the_book_of_nod_is_no_as_played_answer() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRE"], 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let f = location(&mut e, 1, "Fragment of the Book of Nod");
    e.state.seats[1].library = vec![find_library("Soak"); 3];
    hand(&mut e, 0, &["Intimidation"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, play(0, 1, a));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert!(!req.options.contains(&use_on(f, 0, None, Target::None)));
}

// Rulebook (advanced) "Burn option": "a Methuselah who does not control a
// minion who meets the requirements of this card … may discard it during
// ANY Methuselah's unlock phase and replace it. Each Methuselah is limited
// to one such discard each unlock phase." [LSJ 20100203-2] Sight Beyond
// Sight: "The fact it is Unique has no bearing on the burn option."
#[test]
fn a_burn_option_card_is_discarded_without_a_minion_to_play_it() {
    // A Salubri in torpor does not meet it (owner, 2026-09-26: rules-qa
    // 33.8).
    for (salubri_too, torpid) in [(false, false), (true, false), (true, true)] {
        let mut e = Engine::bare(3);
        e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", "", 1), 4, false);
        if salubri_too {
            e.add_minion(SeatId(1), salubri(0), 4, torpid);
        }
        let salubri_too = salubri_too && !torpid;
        e.add_minion(SeatId(2), plain("", "", 2), 4, false);
        let sight = find_library("Sight Beyond Sight");
        assert!(zvtes::library::LIBRARY[sight as usize].burn_option);
        e.state.seats[1].hand = vec![sight, sight];
        e.state.seats[1].library = vec![find_library("Soak"); 2];
        pick(&mut e, Choice::Pass);
        let found = loop {
            let req = request(&mut e);
            if req.window == WindowLabel::UnlockAbilities && req.seat == SeatId(1) {
                break Some(req);
            }
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break None;
            }
            e.choose(fallback(&req));
        };
        if salubri_too {
            assert!(found.is_none_or(|r| !r.options.contains(&Choice::Discard(0))));
            continue;
        }
        found.expect("the unlock window offers the discard");
        pick(&mut e, Choice::Discard(0));
        let hand1 = &e.state.seats[1].hand;
        assert_eq!(hand1.iter().filter(|&&c| c == sight).count(), 1);
        assert!(hand1.contains(&find_library("Soak")), "replaced");
        assert_eq!(e.state.seats[1].ash, vec![sight]);
        let req = request(&mut e);
        assert!(
            !req.options.contains(&Choice::Discard(0)),
            "one each unlock phase"
        );
    }
}

// ---- Pool masters, wave 2 (slice 11x) ---------------------------------------

// WMRH Talk Radio: "You can lock this card to give a minion +1 intercept. If
// that minion does not block the action, burn 1 pool after action
// resolution." [ANK 20200607] Sequencing as usual: its controller uses it
// when they hold the impulse. [ANK 20230319] Blocks before the lock do not
// count.
#[test]
fn wmrh_talk_radio_charges_a_pool_for_a_failed_block() {
    for blocked in [true, false] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["OBF"], 0), 4, false);
        e.add_minion(SeatId(0), plain("", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
        e.add_minion(SeatId(2), plain("", "", 3), 4, false);
        let radio = location(&mut e, 2, "WMRH Talk Radio");
        if !blocked {
            hand(&mut e, 0, &["Lost in Crowds"]);
        }
        pick(&mut e, action(a, ActionKind::Hunt));
        pick(&mut e, Choice::DeclareBlock { minion: z });
        pick_when(&mut e, use_on(radio, 0, None, Target::None));
        if !blocked {
            // The acting Methuselah answers with stealth: the attempt fails.
            pick_when(&mut e, play(0, 0, a));
        }
        to_minion_phase(&mut e, 1);
        assert_eq!(pool(&e, 2), if blocked { 30 } else { 29 });
    }
}

// The Rumor Mill, Tabloid Newspaper: "You can lock this card during an action
// to choose a vampire; the chosen vampire can burn 1 blood to get +1
// intercept during that action." [ANK 20200607] Sequencing as usual.
#[test]
fn the_rumor_mill_lets_a_chosen_vampire_buy_one_intercept() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    let mill = location(&mut e, 1, "Rumor Mill, Tabloid Newspaper, The");
    pick(&mut e, action(a, ActionKind::Hunt));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, use_on(mill, 0, None, Target::Minion(z)));
    pick_when(&mut e, Choice::BurnBloodForIntercept { minion: z });
    assert_eq!(blood(&e, z), 3);
    to_minion_phase(&mut e, 0);
    assert!(e.state.minions[a.index()].locked);
    assert_eq!(blood(&e, a), 3, "blocked: a hand strike, no hunt");
}

// Monster: "Once each turn after a combat involving this vampire ends, this
// vampire can burn 1 blood to unlock if the opposing minion is not ready."
// [ANK 20230314] After the combat, among the after-combat effects. [ANK
// 20180719-4] "Each turn": each player's. [ANK 20210309-3] Its controller
// chooses.
#[test]
fn monster_unlocks_its_vampire_after_a_combat_it_won() {
    let mut e = Engine::bare(3);
    let h = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 0, false);
    master_on(&mut e, 0, "Monster", h);
    let monster = CardId(e.state.cards.len() as u32 - 1);
    bleed_blocked(&mut e, h, z);
    pick_when(&mut e, use_on(monster, 0, None, Target::None));
    assert!(!e.state.minions[h.index()].locked);
    assert_eq!(blood(&e, h), 2, "a hand strike and the blood burned");
    assert!(e.state.minions[z.index()].in_torpor);
}

// Dabbler: "Once each turn, after an action (successful or not) is resolved
// during which this vampire used 3 or more Disciplines to play cards, this
// vampire can either burn 1 blood to unlock or gain 1 blood." [ANK
// 20220411] All the cards played count; the action itself must not be
// canceled. [ANK 20180719-4] "Once each turn" is each player's turn.
// [ANK 20210309-3] Its controller chooses to use it.
#[test]
fn dabbler_rewards_three_disciplines_in_one_action() {
    let mut e = Engine::bare(3);
    let at = e.add_minion(SeatId(0), zvtes::state::find_card("Atiena"), 3, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 5, false);
    e.state.minions[z.index()].locked = true;
    let d = master_on(&mut e, 0, "Dabbler", at);
    hand(&mut e, 0, &["Make the Misere"]);
    // [cel][obf][pot]: three Disciplines in one card.
    pick(&mut e, play_at(0, 6, Some(at), Target::Minion(z)));
    pick_when(&mut e, use_on(d, 1, None, Target::None));
    assert_eq!(blood(&e, at), 3 - 1 + 1, "struck once, then gained 1");
}

// Dead Pool: "After a vampire in combat with a Lasombra you control leaves
// the ready region, you can add 1 counter to this card. During a bleed
// action …, a Lasombra you control can burn 1 counter from this card to get
// +2 bleed." [ANK 20231124] [LSJ 19970718] During the bleed, before its
// resolution.
#[test]
fn dead_pool_counts_victims_and_pays_them_as_bleed() {
    let mut e = Engine::bare(3);
    let l = e.add_minion(SeatId(0), plain("Lasombra", "", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 0, false);
    let w = e.add_minion(SeatId(1), plain("", "", 3), 4, false);
    let dp = location(&mut e, 0, "Dead Pool");
    bleed_blocked(&mut e, l, z);
    to_minion_phase(&mut e, 0);
    assert_eq!(e.state.cards[dp.index()].counters, 1);
    let _ = w;
    e.state.minions[l.index()].locked = false;
    e.state.minions[l.index()].bled_this_turn = false;
    let p1 = pool(&e, 1);
    pick(&mut e, action(l, ActionKind::Bleed));
    pick_when(&mut e, use_on(dp, 0, Some(l), Target::None));
    to_minion_phase(&mut e, 0);
    assert_eq!(pool(&e, 1), p1 - 3);
    assert_eq!(e.state.cards[dp.index()].counters, 0);
}

// Alamut: "After an Assamite you control successfully bleeds a Methuselah,
// put the pool lost by that Methuselah on this card. During the polling step
// of a political action, you can burn X counters from this card to give a
// vampire you control +X votes."
#[test]
fn alamut_banks_bled_pool_for_votes() {
    let mut e = Engine::bare(3);
    let h = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Banu Haqim" && c.title == Title::None, 0),
        4,
        false,
    );
    let an = e.add_minion(SeatId(0), anarch(0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let al = location(&mut e, 0, "Alamut");
    pick(&mut e, action(h, ActionKind::Bleed));
    to_minion_phase(&mut e, 0);
    assert_eq!(e.state.cards[al.index()].counters, 1);
    hand(&mut e, 0, &["Anarchist Uprising"]);
    pick(&mut e, play(0, 0, an));
    // One counter: X = 1 without asking.
    pick_when(&mut e, use_on(al, 0, Some(h), Target::None));
    request(&mut e);
    assert_eq!(e.state.cards[al.index()].counters, 0);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, h), 1);
}

// Cappadocian Crypt: "You can lock this location after resolution of a
// successful action requiring Hecata or Oblivion [obl] to add 1 blood to a
// Hecata you control."
#[test]
fn cappadocian_crypt_feeds_a_hecata_after_an_oblivion_action() {
    let mut e = Engine::bare(3);
    let o = e.add_minion(SeatId(0), with(&["OBL"], 0), 4, false);
    let h = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Hecata" && c.title == Title::None, 0),
        2,
        false,
    );
    let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    let crypt = location(&mut e, 0, "Cappadocian Crypt");
    hand(&mut e, 0, &["Umbrous Clutch"]);
    pick(&mut e, play_at(0, 0, Some(o), Target::Minion(z)));
    pick_when(&mut e, use_on(crypt, 0, None, Target::Minion(h)));
    assert_eq!(blood(&e, h), 3);
    assert!(e.state.cards[crypt.index()].locked);
}

// Powerbase: Munich: "During your master phase, you can lock this location
// to move 1 blood from a vampire with Oblivion [obl] you control to your
// pool or from your pool to a ready vampire with Oblivion [obl] you
// control. Independent vampires can burn this location as a Ⓓ action that
// costs 1 blood." [LSJ 20080604] That action requires an Independent.
#[test]
fn powerbase_munich_shuttles_blood_and_burns_for_independents() {
    let mut e = Engine::bare(3);
    let o = e.add_minion(SeatId(0), with(&["obl"], 0), 1, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let ind = e.add_minion(
        SeatId(1),
        vampire(
            |c| c.sect == zvtes::crypt::Sect::Independent && c.title == Title::None,
            0,
        ),
        3,
        false,
    );
    let pm = location(&mut e, 0, "Powerbase: Munich");
    e.at_master_phase();
    pick(&mut e, use_on(pm, 1, None, Target::Minion(o)));
    assert_eq!((blood(&e, o), pool(&e, 0)), (2, 29));
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, 1);
    let burn = use_on(pm, 2, Some(ind), Target::None);
    pick(&mut e, burn);
    to_minion_phase(&mut e, 1);
    assert!(!e.state.cards[pm.index()].in_play);
    assert_eq!(blood(&e, ind), 2, "1 blood");
}

// Twilight Camp: "Put this card in play with 4 blood. After you put a
// non-titled non-Anarch vampire in play from your uncontrolled region, you
// can burn 1 blood from this card to make that vampire Anarch. You can use a
// master phase action to move 1 blood from this card to an Anarch."
#[test]
fn twilight_camp_makes_anarchs_and_feeds_them() {
    let mut e = Engine::bare(3);
    let an = e.add_minion(SeatId(0), anarch(0), 1, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let camp = in_play_with(&mut e, 0, "Twilight Camp", 4);
    e.at_master_phase();
    pick(&mut e, use_on(camp, 0, None, Target::Minion(an)));
    assert_eq!(
        (blood(&e, an), e.state.cards[camp.index()].counters),
        (2, 3)
    );
    let cam = vampire(
        |c| c.sect == zvtes::crypt::Sect::Camarilla && c.title == Title::None,
        7,
    );
    let slot = uncontrolled(&mut e, 0, cam);
    e.state.seats[0].uncontrolled[slot as usize].counters = CRYPT[cam as usize].capacity;
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, Choice::InfluenceOut { slot });
    pick(&mut e, Choice::ResolveRider { card: camp.0 });
    let v = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(e.state.sect_of(v), Some(zvtes::crypt::Sect::Anarch));
    assert_eq!(e.state.cards[camp.index()].counters, 2);
}

// The Mithraeum, London: "If you do not control /Mithras/ during your
// influence phase, you can search your crypt for 1 copy of him, reveal it,
// and move it to your uncontrolled region (shuffle afterward). During your
// unlock phase, you can add 1 blood to /Mithras/ or a Mithraic Mysteries
// member (the target must be controlled)." Mithraic Cultist: "This vampire
// is a Mithraic Mysteries member. This vampire gets +1 vote while you
// control /Mithras/. This vampire gets +1 intercept during actions directed
// at /Mithras/ or a Mithraic Mysteries card or member."
#[test]
fn the_mithraeum_finds_mithras_and_feeds_his_cult() {
    let mut e = Engine::bare(3);
    let c = e.add_minion(SeatId(0), plain("", "", 0), 2, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let mithras = zvtes::state::find_card("Mithras");
    e.state.seats[0].crypt = vec![plain("", "", 2), mithras];
    let m = location(&mut e, 0, "Mithraeum, London, The");
    master_on(&mut e, 0, "Mithraic Cultist", c);
    assert!(e.state.mithraic_member(c));
    assert_eq!(e.state.host_votes(c), 0, "no Mithras controlled");
    // Influence phase: Mithras moves to the uncontrolled region.
    pick_when(&mut e, use_on(m, 0, None, Target::None));
    assert!(e.state.seats[0]
        .uncontrolled
        .iter()
        .any(|s| s.card == mithras));
    assert!(!e.state.seats[0].crypt.contains(&mithras));
    // Next unlock phase: 1 blood to the member.
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, use_on(m, 1, None, Target::Minion(c)));
    assert_eq!(blood(&e, c), 3);
    let mm = e.add_minion(SeatId(0), mithras, 3, false);
    let _ = mm;
    assert_eq!(e.state.host_votes(c), 1, "Mithras controlled");
}

// ---- Pool masters, wave 3 (slice 11y) ---------------------------------------

// Secure Haven: "This minion cannot be the target of other Methuselahs'
// actions." [TOM 19951208-1] [LSJ 19980302-1] "Actions that target
// retainers, equipment, or other cards on the minion are allowed." [LSJ
// 20031010] [LSJ 20090319] [ANK 20180907] "Actions that choose the vampire
// cannot be played against him (eg. Propaganda)." [ANK 20221019-3]
// Banishment is not directed: no protection.
#[test]
fn secure_haven_shields_its_minion_from_others_actions() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRE"], 0), 4, false);
    let o = e.add_minion(SeatId(0), with(&["OBF"], 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let w = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    master_on(&mut e, 1, "Secure Haven", z);
    let gun = equip(&mut e, ".44 Magnum", z);
    hand(&mut e, 0, &["Propaganda", "Conceal", "Umbrous Clutch"]);
    let req = request(&mut e);
    let named = |m| play_at(0, 1, Some(a), Target::Minion(m));
    assert!(!req.options.contains(&named(z)), "a chosen minion");
    assert!(req.options.contains(&named(w)));
    assert!(
        req.options
            .contains(&play_at(1, 0, Some(o), Target::Card(gun))),
        "its equipment"
    );
    let ob = e.add_minion(SeatId(0), with(&["OBL"], 0), 4, false);
    let req = request(&mut e);
    let clutch = |m| play_at(2, 0, Some(ob), Target::Minion(m));
    assert!(!req.options.contains(&clutch(z)), "an action card's combat");
    assert!(req.options.contains(&clutch(w)));
    // A granted action too (Open War's rush).
    let war = location(&mut e, 0, "Open War");
    let an = e.add_minion(SeatId(0), anarch(0), 4, false);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&use_on(war, 0, Some(an), Target::Minion(z))));
    assert!(req
        .options
        .contains(&use_on(war, 0, Some(an), Target::Minion(w))));
}

// Secure Haven: [TOM 19951208-1] "All Methuselahs, including the target
// vampire's controller, must pay one extra pool when playing Master cards on
// the target minion." [LSJ 20020620] Paid as the pool comes (Villein): not
// ousted in between. [LSJ 20010610] Playable on a vampire in torpor. [LSJ
// 20031010] [LSJ 20090319] [ANK 20180907] Choosing is targeting (Piper).
#[test]
fn secure_haven_taxes_master_cards_on_its_minion() {
    let mut e = Engine::bare(3);
    let torpid = e.add_minion(SeatId(0), plain("", "", 0), 4, true);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 1, false);
    hand(&mut e, 0, &["Secure Haven", "Giant's Blood"]);
    e.at_master_phase();
    e.state.seats[0].mpa = 3;
    pick(&mut e, play_at(0, 0, None, Target::Minion(torpid)));
    request(&mut e);
    assert_eq!(pool(&e, 0), 29, "1 pool, on a vampire in torpor");
    master_on(&mut e, 1, "Secure Haven", z);
    pick(&mut e, play_at(0, 0, None, Target::Minion(z)));
    request(&mut e);
    assert_eq!(pool(&e, 0), 28, "Giant's Blood costs 1");
    // Its controller pays too: Villein on it with 1 pool, the blood coming
    // at once.
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    master_on(&mut e, 0, "Secure Haven", v);
    hand(&mut e, 0, &["Villein"]);
    e.state.seats[0].pool = 1;
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::Minion(v)));
    pick(&mut e, Choice::SetX(2));
    request(&mut e);
    assert!(!e.state.seats[0].ousted);
    assert_eq!(pool(&e, 0), 2, "1 - 1 + 2");
    // Piper for a vampire with a haven: 1 more.
    let mut e = Engine::bare(3);
    let an = e.add_minion(
        SeatId(0),
        vampire(
            |c| {
                c.clan == "Gangrel"
                    && c.sect == zvtes::crypt::Sect::Anarch
                    && c.title == Title::None
            },
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    master_on(&mut e, 0, "Secure Haven", an);
    hand(&mut e, 0, &["Piper", "Double Deuce"]);
    e.at_master_phase();
    pick(&mut e, play(0, 0, an));
    request(&mut e);
    assert_eq!(pool(&e, 0), 30 - 2 - 1);
}

// Secure Haven: "Burn this card after this minion goes to torpor." [RTR
// 19960124] Contested, it is not burned. [ANK 20221120] "There is no
// additional cost for using the effects of cards already in play (eg. KRCG
// News Radio)."
#[test]
fn secure_haven_burns_with_its_minion_unless_contested() {
    for contested in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        e.add_minion(SeatId(0), plain("", "", 3), 4, false);
        let z = e.add_minion(SeatId(1), plain("", "", 1), 0, false);
        let sh = master_on(&mut e, 1, "Secure Haven", z);
        e.state.cards[sh.index()].contested = contested;
        bleed_blocked(&mut e, a, z);
        to_minion_phase(&mut e, 0);
        assert!(e.state.minions[z.index()].in_torpor);
        assert_eq!(e.state.cards[sh.index()].in_play, contested);
    }
    // KRCG News Radio's intercept for a haven's minion: no extra pool.
    let mut e = Engine::bare(4);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 3, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 2), 4, false);
    e.add_minion(SeatId(2), plain("", "", 3), 4, false);
    e.add_minion(SeatId(3), plain("", "", 4), 4, false);
    let radio = location(&mut e, 2, "KRCG News Radio");
    master_on(&mut e, 1, "Secure Haven", z);
    hand(&mut e, 0, &["Anarchist Uprising"]);
    pick(&mut e, play(0, 0, a));
    let p2 = pool(&e, 2);
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, use_on(radio, 1, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 2), p2 - 1, "its own cost only");
}

// Yoruba Shrine: "If a ready Assamite you control is the target of a
// directed action …, you can lock this location to unlock the acting minion
// and have the action … fail. Only usable as the directed action is
// announced." [LSJ 20081105] After the acting player's as-announced
// effects. [ANK 20190425] Before other modifiers and reactions. [ANK
// 20211015] [RTR 20180719] The failed action reached resolution: NRA.
// [LSJ 20070219] It was not blocked.
#[test]
fn yoruba_shrine_fails_an_action_aimed_at_an_assamite() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["OBL"], 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let h = e.add_minion(
        SeatId(1),
        vampire(|c| c.clan == "Banu Haqim" && c.title == Title::None, 0),
        4,
        false,
    );
    let shrine = location(&mut e, 1, "Yoruba Shrine");
    hand(&mut e, 0, &["Umbrous Clutch", "Umbrous Clutch"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Minion(h)));
    until(&mut e, SeatId(1), WindowLabel::ActionAsAnnounced);
    let fail = use_on(shrine, 0, None, Target::None);
    pick(&mut e, fail);
    let req = to_minion_phase(&mut e, 0);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    assert!(e.state.cards[shrine.index()].locked);
    assert!(
        !req.options
            .contains(&play_at(0, 0, Some(a), Target::Minion(h))),
        "the same action again: NRA"
    );
}

// Cave of Apples: "Followers of Set you control can put 1 corruption counter
// on an ally or younger vampire controlled by your prey as a Ⓓ action. If the
// action is successful and the number of your corruption counters on the
// minion equals or exceeds their capacity or cost, you can burn those
// counters to steal that minion." [LSJ 20080604] The action requires a
// Follower of Set. [ANK 20200925] [ANK 20211105] A counter put otherwise
// (The Platinum Protocol [pro]) gives no opportunity to steal.
#[test]
fn cave_of_apples_corrupts_and_steals_at_capacity() {
    let mut e = Engine::bare(3);
    let m = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Ministry" && c.capacity >= 7, 0),
        4,
        false,
    );
    let other = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire(|c| c.capacity == 2, 0), 2, false);
    let cave = location(&mut e, 0, "Cave of Apples");
    e.state.minions[z.index()].add_corruption(SeatId(0), 1);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&use_on(cave, 0, Some(other), Target::Minion(z))));
    pick(&mut e, use_on(cave, 0, Some(m), Target::Minion(z)));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, Choice::ChooseMinion(z)]);
    pick(&mut e, Choice::ChooseMinion(z));
    request(&mut e);
    let v = &e.state.minions[z.index()];
    assert_eq!((v.owner, v.corruption_of(SeatId(0))), (SeatId(0), 0));
}

// British Museum: "+1 hand size. Minions you control can unlock after
// resolution of a successful action to equip with a non-electronic,
// non-vehicle, non-weapon equipment card from your hand … Non-electronic,
// non-vehicle, non-weapon equipment cards cost minions you control -1 blood
// or pool to equip." [ANK 20220824] The type read in hand: a location
// equipment too. [ANK 20221026] The reduction always.
#[test]
fn british_museum_discounts_plain_equipment_and_unlocks_once() {
    let mut e = Engine::bare(3);
    let t = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Tzimisce" && c.title == Title::None, 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let bm = location(&mut e, 0, "British Museum, London, The");
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
    hand(&mut e, 0, &["Living Manse"]);
    let cost = match zvtes::library::LIBRARY[find_library("Living Manse") as usize].pool_cost {
        zvtes::library::Cost::Fixed(n) => n as i32,
        _ => 0,
    };
    let p0 = pool(&e, 0);
    pick(&mut e, play(0, 0, t));
    pick_when(&mut e, use_on(bm, 0, None, Target::None));
    assert!(!e.state.minions[t.index()].locked);
    assert_eq!(pool(&e, 0), p0 - (cost - 1).max(0));
}

// Yoruba Shrine: "If a ready Assamite you control … is chosen by the acting
// Methuselah in the terms of a referendum, you can lock this location to
// unlock the acting minion and have the … referendum fail. Only usable …
// during the polling step of the referendum before votes and ballots are
// cast."
#[test]
fn yoruba_shrine_fails_a_referendum_naming_an_assamite() {
    let mut e = Engine::bare(3);
    let alexa = e.add_minion(SeatId(0), zvtes::state::find_card("Alexa Draper"), 5, false);
    e.add_minion(SeatId(0), plain("", "", 1), 4, false);
    let h = e.add_minion(
        SeatId(1),
        vampire(|c| c.clan == "Banu Haqim" && c.title == Title::None, 0),
        4,
        false,
    );
    e.add_minion(SeatId(2), plain("", "", 2), 4, false);
    let shrine = location(&mut e, 1, "Yoruba Shrine");
    hand(&mut e, 0, &["Banishment"]);
    pick(&mut e, play(0, 0, alexa));
    pick_when(&mut e, Choice::ChooseMinion(h));
    let fail = use_on(shrine, 1, None, Target::None);
    pick_when(&mut e, fail);
    to_minion_phase(&mut e, 0);
    assert!(e.state.minions[h.index()].in_play, "not banished");
    assert!(!e.state.minions[alexa.index()].locked, "unlocked");
    assert!(e.state.cards[shrine.index()].locked);
}

// ---- Pool masters, wave 4: cost reductions (slice 11z) ----------------------

// The Line: "You can lock this location to reduce the cost of an action card
// a vampire you control plays by 1 blood (this location is not locked if
// that card is canceled as it is played)." [ANK 20181007] Blood, not pool.
// [ANK 20230620] [ANK 20170605] [LSJ 20090705] From the announcement to
// just before the cost is paid; canceled as played, not locked. [LSJ
// 20090705] [LSJ 20030917] [ANK 20230721] It lets a card otherwise
// unaffordable be played, announced with it.
#[test]
fn the_line_takes_a_blood_off_an_action_card() {
    // Unaffordable without it: announced with the card, no Pass.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRE"], 0), 1, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let line = location(&mut e, 0, "Line, The");
    hand(&mut e, 0, &["Propaganda"]);
    // Its only choice: The Line (elided).
    pick(&mut e, play(0, 0, a));
    to_minion_phase(&mut e, 1);
    assert!(e.state.cards[line.index()].locked);
    assert_eq!((blood(&e, a), pool(&e, 1)), (0, 28), "2 blood - 1");
    // Canceled as played: not locked.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRE"], 0), 1, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let line = location(&mut e, 0, "Line, The");
    hand(&mut e, 0, &["Propaganda"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play_at(0, 0, None, Target::None));
    request(&mut e);
    assert!(!e.state.cards[line.index()].locked);
    // During the action, before the cost is paid.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRE"], 0), 3, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let line = location(&mut e, 0, "Line, The");
    hand(&mut e, 0, &["Public Trust"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, use_on(line, 0, None, Target::None));
    to_minion_phase(&mut e, 1);
    assert_eq!(blood(&e, a), 3, "1 blood - 1");
}

// The Shard, London: "If you have the Edge, you can lock this card to reduce
// the cost of a card you play by 2 pool or blood." [ANK 20221102] Minion
// cards too. Master cards: announced with the card, locked as it resolves
// ([ANK 20230620] [ANK 20170605] [LSJ 20090705]: not if canceled as played;
// [LSJ 20090705] [LSJ 20030917] [ANK 20230721]: announced when otherwise
// unaffordable).
#[test]
fn the_shard_takes_two_off_a_card_with_the_edge() {
    for edge in [false, true] {
        let mut e = Engine::bare(3);
        e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        e.add_minion(SeatId(1), plain("", "", 1), 4, false);
        let shard = location(&mut e, 0, "Shard, London, The");
        e.state.seats[0].has_edge = edge;
        hand(&mut e, 0, &["Parthenon, The"]);
        e.at_master_phase();
        pick(&mut e, play_at(0, 0, None, Target::None));
        if edge {
            let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
            assert!(req.options.contains(&Choice::Pass));
            pick(&mut e, Choice::ResolveRider { card: shard.0 });
        }
        request(&mut e);
        assert_eq!(pool(&e, 0), if edge { 30 } else { 28 });
        assert_eq!(e.state.cards[shard.index()].locked, edge);
    }
    // Unaffordable without it: no Pass. Canceled as played (Wash): not
    // locked.
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 1), 4, false);
    let shard = location(&mut e, 0, "Shard, London, The");
    e.state.seats[0].has_edge = true;
    e.state.seats[0].pool = 1;
    hand(&mut e, 0, &["Parthenon, The"]);
    hand(&mut e, 1, &["Wash"]);
    e.at_master_phase();
    // The Shard its only choice: elided.
    pick(&mut e, play_at(0, 0, None, Target::None));
    pick_when(&mut e, play_at(0, 0, None, Target::None));
    request(&mut e);
    assert!(!e.state.cards[shard.index()].locked);
    assert_eq!(pool(&e, 0), 1);
}

// Ravnos Cache: "During your master phase, you can move 1 counter from your
// pool to this location and add 1 counter (from the blood bank) to it.
// Minions you control can lock this location to use those counters to pay
// some or all of the blood or pool cost of equipment they equip." [RTR
// 19960112] In place of pool or blood, as the cost is.
#[test]
fn ravnos_cache_banks_pool_to_pay_for_equipment() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let cache = location(&mut e, 0, "Ravnos Cache");
    e.at_master_phase();
    pick(&mut e, use_on(cache, 0, None, Target::None));
    assert_eq!(
        (pool(&e, 0), e.state.cards[cache.index()].counters),
        (29, 2)
    );
    // Once a master phase: the phase ends with nothing else to do.
    hand(&mut e, 0, &[".44 Magnum"]);
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::MinionPhase);
    pick_when(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ResolveRider { card: cache.0 });
    pick(&mut e, Choice::SetX(2));
    to_minion_phase(&mut e, 1);
    assert_eq!(pool(&e, 0), 29, "the Magnum's 2 pool from the counters");
    assert!(e.state.cards[cache.index()].locked);
    assert_eq!(e.state.cards[cache.index()].counters, 0);
}

// Ravnos Carnival: "This location comes into play with 1 counter for each
// Ravnos you control. Ravnos you control can use those counters to pay some
// or all of the blood cost of action cards they play. If this location has
// no counters, burn it." [LSJ 20050408] An action otherwise unaffordable
// may be attempted. [RTR 20111202] The counters are announced with the
// card. [ANK 20180109] [LSJ 20070224] Not a cardless action, nor one
// provided by a card in play. [LSJ 20091021] Not a political action card's
// cost.
#[test]
fn ravnos_carnival_pays_a_ravnos_action_card() {
    let mut e = Engine::bare(3);
    let r = e.add_minion(
        SeatId(0),
        vampire(
            |c| c.clan == "Ravnos" && has(c, &["pre"]) && c.title == Title::None,
            0,
        ),
        1,
        false,
    );
    e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Ravnos" && c.title == Title::None, 1),
        3,
        false,
    );
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    hand(&mut e, 0, &["Ravnos Carnival"]);
    e.at_master_phase();
    pick(&mut e, play_at(0, 0, None, Target::None));
    request(&mut e);
    let carnival = CardId(e.state.cards.len() as u32 - 1);
    assert_eq!(e.state.cards[carnival.index()].counters, 2);
    hand(&mut e, 0, &["Propaganda"]);
    // 1 blood for a 2-blood card: the counters pay (announced with it).
    // The Carnival its only choice (unaffordable alone): how many counters.
    pick(&mut e, play(0, 0, r));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(req.options, vec![Choice::SetX(1), Choice::SetX(2)]);
    pick(&mut e, Choice::SetX(2));
    to_minion_phase(&mut e, 0);
    assert_eq!((pool(&e, 1), blood(&e, r)), (28, 1));
    assert!(!e.state.cards[carnival.index()].in_play, "no counters left");
}

// Fleshforge Chamber: "During your master phase, you can put a ghoul (ally
// or retainer) from your hand on this location, face up and out of play.
// Tzimisce you control can play cards from this location as if from your
// hand (requirements and cost apply as normal)." Put there, the card is
// replaced; played from there, it is not (owner, 2026-09-26: rules-qa
// 37.1).
#[test]
fn fleshforge_chamber_keeps_ghouls_for_the_tzimisce() {
    let mut e = Engine::bare(3);
    let t = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Tzimisce" && c.title == Title::None, 0),
        4,
        false,
    );
    let v = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let ff = location(&mut e, 0, "Fleshforge Chamber");
    let ghoul = find_library("Szlachta Assistant");
    hand(&mut e, 0, &["Szlachta Assistant", "Soak"]);
    e.state.seats[0].library = vec![find_library("Soak"); 3];
    e.at_master_phase();
    pick(&mut e, use_on(ff, 0, None, Target::None));
    let req = request(&mut e);
    assert_eq!(e.state.cards[ff.index()].stored, vec![ghoul]);
    // Replaced (rules-qa 37.1).
    let library = e.state.seats[0].library.len();
    assert!(library < 3, "replaced");
    // The minion phase: the index past the hand is the card on the chamber.
    let _ = req;
    let req = to_minion_phase(&mut e, 0);
    let k = e.state.seats[0].hand.len() as u8;
    assert!(req.options.contains(&play(k, 0, t)));
    assert!(!req.options.contains(&play(k, 0, v)), "a Tzimisce only");
    pick(&mut e, play(k, 0, t));
    to_minion_phase(&mut e, 1);
    assert!(e.state.cards[ff.index()].stored.is_empty());
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == ghoul && c.host == Some(t)));
    assert_eq!(
        e.state.seats[0].library.len(),
        library,
        "played from it: not replaced"
    );
}

// Rules-qa 36.1 (owner, 2026-09-26): The Line and The Shard can be locked
// as the action resolves too.
#[test]
fn the_line_can_be_locked_as_the_action_resolves() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRE"], 0), 3, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let line = location(&mut e, 0, "Line, The");
    hand(&mut e, 0, &["Propaganda"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::Pass); // nothing announced with the card
    let lock = use_on(line, 0, None, Target::None);
    until(&mut e, SeatId(0), WindowLabel::ActionWouldBeSuccessful);
    pick(&mut e, lock);
    to_minion_phase(&mut e, 1);
    assert_eq!(blood(&e, a), 2, "2 blood - 1");
}

// [ANK 20230824] [RTR 20070707] "Can reduce the cost of an action card even
// if it is not played as an action (e.g. a weapon played with Concealed
// Weapon or an Ally with Piper)." "Cost reductions that are not referring to
// an action apply." (owner, 2026-09-26: rules-qa 36.5): The Shard for
// Piper's ally, Ravnos Cache for Contraband's equipment.
#[test]
fn reductions_reach_cards_played_by_piper_and_contraband() {
    let mut e = Engine::bare(3);
    let an = e.add_minion(
        SeatId(0),
        vampire(
            |c| {
                c.clan == "Gangrel"
                    && c.sect == zvtes::crypt::Sect::Anarch
                    && c.title == Title::None
            },
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let shard = location(&mut e, 0, "Shard, London, The");
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Piper", "Double Deuce"]);
    e.at_master_phase();
    pick(&mut e, play(0, 0, an));
    // The ally named (its only choice), then The Shard for it.
    pick_when(&mut e, Choice::ResolveRider { card: shard.0 });
    request(&mut e);
    assert_eq!(pool(&e, 0), 30, "Double Deuce's 2 pool taken off");
    assert!(e.state.cards[shard.index()].locked);
    // Contraband's Magnum paid with Ravnos Cache counters.
    let mut e = Engine::bare(3);
    let ravnos = vampire(|c| c.clan == "Ravnos" && has(c, &["OBF"]), 0);
    let a = e.add_minion(SeatId(0), ravnos, 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    let cache = in_play_with(&mut e, 0, "Ravnos Cache", 2);
    hand(&mut e, 0, &["Contraband", ".44 Magnum"]);
    let p0 = pool(&e, 0);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::SetX(0)); // no blood share
    pick_when(&mut e, Choice::ResolveRider { card: cache.0 });
    pick(&mut e, Choice::SetX(2));
    request(&mut e);
    assert_eq!(pool(&e, 0), p0, "the counters paid");
    assert_eq!(e.state.cards[cache.index()].counters, 0);
}

// ---- Allies and equipment (slice 11aa) ---------------------------------------

// Ritual Goblet: "Unique. This vampire gets +1 level of Blood Sorcery [tha].
// Rescuing a vampire from torpor costs this vampire -1 blood. Once each
// turn, this vampire can unlock after rescuing a vampire from torpor." [PIB
// 20110918] [LSJ 20020304-2]: "The discount applies to the action,
// regardless of who pays."
#[test]
fn ritual_goblet_discounts_a_rescue_and_unlocks_once_a_turn() {
    let mut e = Engine::bare(3);
    let s = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 2, false);
    let t = e.add_minion(SeatId(0), plain("Brujah", "", 0), 0, true);
    let u = e.add_minion(SeatId(0), plain("Brujah", "", 1), 0, true);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let cap = e.state.capacity_of(s);
    let goblet = equip(&mut e, "Ritual Goblet", s);
    assert_eq!(e.state.discipline_level(s, "tha"), 1);
    assert_eq!(e.state.capacity_of(s), cap, "no capacity with it");
    let rescue = |t| Choice::TakeAction {
        minion: s,
        action: ActionKind::Rescue,
        target: Some(t),
    };
    pick(&mut e, rescue(t));
    pick_when(&mut e, card_ability(goblet, 0));
    assert!(!e.state.minions[t.index()].in_torpor);
    assert_eq!(blood(&e, s), 1, "1 blood paid");
    assert!(!e.state.minions[s.index()].locked, "unlocked");
    // A second rescue this turn: no second unlock.
    pick(&mut e, rescue(u));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[u.index()].in_torpor);
    assert!(e.state.minions[s.index()].locked);
    assert!(!req.options.contains(&card_ability(goblet, 0)));
}

// Flaming Candle: "Only one Flaming Candle can be played or equipped in a
// game. This vampire can burn 1 blood and the Flaming Candle as they
// announce an action to make that action unblockable by vampires." [ANK
// 20221103]: "Once in play, it can be moved, or equipped by another minion
// from the minion who has it."
#[test]
fn flaming_candle_is_equipped_once_a_game_and_burns_for_an_unblockable_action() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
    hand(&mut e, 0, &["Flaming Candle", "Flaming Candle"]);
    let candle = find_library("Flaming Candle");
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let id = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == candle)
        .map(|i| CardId(i as u32))
        .expect("equipped");
    assert_eq!(e.state.cards[id.index()].host, Some(a));
    // The other copy is no longer offered; taking it from `a` is.
    assert!(!req.options.contains(&play(0, 0, b)));
    let take = Choice::TakeAction {
        minion: b,
        action: ActionKind::EquipFrom,
        target: Some(a),
    };
    pick(&mut e, take);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[id.index()].host, Some(b));
    // `b`'s bleed, burning it.
    e.state.minions[b.index()].locked = false;
    e.state.minions[b.index()].acted.clear();
    pick(&mut e, action(b, ActionKind::Bleed));
    pick(&mut e, card_ability(id, 0));
    assert_eq!(blood(&e, b), 3);
    assert!(!e.state.cards[id.index()].in_play);
    assert!(e.state.seats[0].ash.contains(&candle));
    // Unblockable by vampires: the prey's vampire is never offered a block.
    let p1 = pool(&e, 1);
    never_offered(&mut e, &Choice::DeclareBlock { minion: z });
    assert_eq!(pool(&e, 1), p1 - 1, "bled");
}

// Enhanced Coagulant: "Weapon. Strike: 3 unpreventable damage and put this
// card on the opposing vampire; it becomes a card with \"Cards requiring a
// Discipline cost this vampire +1 blood. After this vampire successfully
// hunts, burn this card\"." [ANK 20220818] [ANK 20221102]: "Cannot be used
// against an ally." "Once used, it becomes a typeless card and cannot be
// used as a weapon anymore." Soak cannot prevent it.
#[test]
fn enhanced_coagulant_strikes_unpreventably_and_moves_onto_the_vampire() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), with(&["for"], 0), 5, false);
    let coag = equip(&mut e, "Enhanced Coagulant", a);
    hand(&mut e, 1, &["Soak"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, weapon_strike(coag));
    never_offered(&mut e, &play(0, 0, b));
    assert_eq!(blood(&e, b), 2, "3 unpreventable damage");
    let c = &e.state.cards[coag.index()];
    assert!(c.in_play && c.host == Some(b));
    assert!(cards::became(c.card, c.mode));
    assert!(e.state.equipment_on(b).is_empty(), "no equipment any more");
    assert!(e
        .state
        .layers
        .iter()
        .any(|l| l.source == coag && l.modifier == cards::Modifier::HostDisciplineCardsCost(1)));
}

// [ANK 20220923] Enhanced Coagulant: "The card stays there if the strike is
// dodged."
#[test]
fn a_dodged_enhanced_coagulant_stays_equipped() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), with(&["cel"], 0), 5, false);
    let coag = equip(&mut e, "Enhanced Coagulant", a);
    hand(&mut e, 1, &["Side Strike"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, weapon_strike(coag));
    pick_when(&mut e, play(0, 0, b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 5);
    let c = &e.state.cards[coag.index()];
    assert!(c.in_play && c.host == Some(a) && !cards::became(c.card, c.mode));
}

// Enhanced Coagulant, become: "Cards requiring a Discipline cost this
// vampire +1 blood. After this vampire successfully hunts, burn this card."
#[test]
fn enhanced_coagulant_become_costs_discipline_cards_one_more_until_a_hunt() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), with(&["dom"], 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let coag = equip(&mut e, "Enhanced Coagulant", v);
    e.state.cards[coag.index()].owner = SeatId(1);
    e.state.cards[coag.index()].controller = SeatId(1);
    e.state.cards[coag.index()].mode = 1;
    e.state.layers.push(Layer {
        scope: Scope::InPlay,
        modifier: cards::Modifier::HostDisciplineCardsCost(1),
        source: coag,
    });
    hand(&mut e, 0, &["Conditioning"]);
    // Conditioning costs it 2 blood: not with its 1.
    pick(&mut e, action(v, ActionKind::Bleed));
    never_offered(&mut e, &play(0, 0, v));
    let m = &mut e.state.minions[v.index()];
    m.locked = false;
    m.acted.clear();
    pick(&mut e, action(v, ActionKind::Hunt));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[coag.index()].in_play, "burned by the hunt");
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Enhanced Coagulant")));
    assert!(!e.state.layers.iter().any(|l| l.source == coag));
}

/// The ally `name` for `seat`, able to act; its card in play.
fn antigen(e: &mut Engine, seat: u8, name: &str) -> (MinionId, CardId) {
    let m = e.add_ally(SeatId(seat), find_library(name));
    (m, e.state.minions[m.index()].ally_id().expect("ally card"))
}

// ECTU Operative: "He can burn a vampire in torpor as a Ⓓ action." Any
// vampire in torpor: another Methuselah's, directed at them, or one's own,
// undirected (rules-qa 20.1, 38.2); not a ready one.
#[test]
fn ectu_operative_burns_a_vampire_in_torpor() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let own = e.add_minion(SeatId(0), plain("Brujah", "", 0), 0, true);
    let t = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 2, true);
    let y = e.add_minion(SeatId(1), plain("Gangrel", "", 1), 5, false);
    let (ectu, card) = antigen(&mut e, 0, "ECTU Operative");
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&granted(card, 2, ectu, own)));
    assert!(!req.options.contains(&granted(card, 2, ectu, y)), "ready");
    pick(&mut e, granted(card, 2, ectu, t));
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_play, "burned");
    assert!(e.state.minions[ectu.index()].locked);
}

// ECTU Operative: "ECTU Operative can strike: 1R damage. He gets 1 optional
// maneuver each combat." Blocking a bleed: the maneuver once, the 1R strike
// at long range.
#[test]
fn ectu_operative_strikes_at_range_with_its_maneuver() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let (ectu, card) = antigen(&mut e, 1, "ECTU Operative");
    bleed_blocked(&mut e, a, ectu);
    pick_when(&mut e, card_ability(card, 1));
    pick_when(&mut e, card_ability(card, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "1R at long range; the hand strike missed");
    assert_eq!(blood(&e, ectu), 3);
}

// Michael Harris: "Michael can search your library for an equipment card
// and equip with it as a +1 stealth action (requirements and cost apply as
// normal; shuffle afterward)." [TOM 19960130] "Their special is an equip
// action." [ANK 20180817] [LSJ 20100303] [LSJ 20100725] "Does not prevent
// the acting ally from playing the equipment directly this turn if he
// unlocks (not the same action)." "Requirements apply": no Salubri weapon;
// nor a Flaming Candle already equipped this game.
#[test]
fn michael_harris_searches_the_library_for_equipment_to_equip() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let (mh, card) = antigen(&mut e, 0, "Michael Harris, the Incinerator");
    let (magnum, candle) = (find_library(".44 Magnum"), find_library("Flaming Candle"));
    let blade = find_library("Righteous Blade");
    e.state.seats[0].library = vec![candle, magnum, blade];
    e.state.once_played.push(candle);
    hand(&mut e, 0, &[".44 Magnum"]);
    let search = Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability: 0,
        actor: Some(mh),
        target: Target::None,
    };
    pick(&mut e, search);
    assert_eq!(e.action().expect("action").base_stealth, 1);
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    assert!(req.options.contains(&Choice::FindCard(magnum)));
    assert!(
        !req.options.contains(&Choice::FindCard(blade)),
        "Salubri only"
    );
    assert!(
        !req.options.contains(&Choice::FindCard(candle)),
        "once a game"
    );
    let p0 = pool(&e, 0);
    pick(&mut e, Choice::FindCard(magnum));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 0), p0 - 2);
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == magnum && c.host == Some(mh)));
    // Unlocked, he may equip a Magnum from hand: another action.
    e.state.minions[mh.index()].locked = false;
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, mh)));
    assert!(!req.options.contains(&search), "the search once a turn");
}

// Michael Harris: "If he is in combat with a vampire, you can burn 1 pool
// once each round before range is determined to set the range for the
// round." [RTR 19970630] [ANK 20180720]: "Once the range is set, no other
// effect can be used to reset the range that round … Skip the Determine
// Range step for that round."
#[test]
fn michael_harris_sets_the_range_once_a_round_for_a_pool() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let (mh, card) = antigen(&mut e, 1, "Michael Harris, the Incinerator");
    bleed_blocked(&mut e, a, mh);
    let p1 = pool(&e, 1);
    pick_when(&mut e, card_ability(card, 2));
    assert_eq!(pool(&e, 1), p1 - 1);
    // No second setting, and no determine range step this round.
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&card_ability(card, 1)));
        assert!(!req.options.contains(&card_ability(card, 2)));
        assert_ne!(req.window, WindowLabel::CombatDetermineRange);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(
        (blood(&e, a), blood(&e, mh)),
        (5, 3),
        "long range: no hand strikes"
    );
}

// Valerie Diaz: "If Valerie is in combat with a vampire, you can burn 1
// pool once each round to make her immune to all non-aggravated damage
// from a strike or damaging effect. Once each turn after combat involving
// Valerie ends, she can unlock if the opposing vampire is not ready."
#[test]
fn valerie_diaz_is_immune_to_a_strike_and_unlocks_after_torporing_a_vampire() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let (vd, card) = antigen(&mut e, 1, "Valerie Diaz, Antigen Commander");
    let gun = equip(&mut e, ".44 Magnum", vd);
    bleed_blocked(&mut e, a, vd);
    pick_when(&mut e, weapon_strike(gun));
    let p1 = pool(&e, 1);
    pick_when(&mut e, card_ability(card, 0));
    assert_eq!(pool(&e, 1), p1 - 1);
    // The Magnum's 2 damage sends `a` to torpor; Valerie took none.
    pick_when(&mut e, card_ability(card, 2));
    assert!(e.state.minions[a.index()].in_torpor);
    assert_eq!(blood(&e, vd), 3, "immune");
    assert!(!e.state.minions[vd.index()].locked, "unlocked");
}

// Mobile HQ, Operation Antigen: "An Operation Antigen ally you control can
// lock this location to attempt to enter combat with a vampire as a +1
// stealth Ⓓ action." [ANK 20221102]: "The card is locked if used to enter
// combat, even if that action fails." Not another ally, nor one recruited
// this turn.
#[test]
fn mobile_hq_sends_an_antigen_ally_into_combat_locking_as_declared() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let other = e.add_ally(SeatId(0), find_library("Underbridge Stray"));
    let (ectu, _) = antigen(&mut e, 0, "ECTU Operative");
    let y = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let hq = location(&mut e, 0, "Mobile HQ, Operation Antigen");
    e.state.minions[ectu.index()].recruited = true;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&granted(hq, 0, ectu, y)), "recruited");
    assert!(
        !req.options.contains(&granted(hq, 0, other, y)),
        "no Antigen"
    );
    e.state.minions[ectu.index()].recruited = false;
    pick(&mut e, granted(hq, 0, ectu, y));
    assert!(e.state.cards[hq.index()].locked, "locked as declared");
    assert_eq!(e.action().expect("action").base_stealth, 1);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(blood(&e, y) < 5, "the combat took place");
    // Its target gone to torpor before resolution, the action fails: the
    // location stays locked.
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let (ectu, _) = antigen(&mut e, 0, "ECTU Operative");
    let y = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let hq = location(&mut e, 0, "Mobile HQ, Operation Antigen");
    pick(&mut e, granted(hq, 0, ectu, y));
    e.state.minions[y.index()].in_torpor = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, y), 5, "no combat");
    assert!(e.state.cards[hq.index()].locked);
}

// Mobile HQ: "You can lock this location to add enough life to an
// Operation Antigen ally you control to reach their starting life."
#[test]
fn mobile_hq_restores_an_antigen_ally_to_its_starting_life() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let (vd, _) = antigen(&mut e, 0, "Valerie Diaz, Antigen Commander");
    let stray = e.add_ally(SeatId(0), find_library("Underbridge Stray"));
    e.state.minions[vd.index()].blood = 1;
    e.state.minions[stray.index()].blood = 0;
    let hq = location(&mut e, 0, "Mobile HQ, Operation Antigen");
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let heal = |m| use_on(hq, 1, None, Target::Minion(m));
    assert!(!req.options.contains(&heal(stray)), "not an Antigen ally");
    pick(&mut e, heal(vd));
    assert_eq!(blood(&e, vd), 3);
    assert!(e.state.cards[hq.index()].locked);
}

// Mobile HQ: "You can lock this location and burn 2 pool to cancel an
// action card that targets an Operation Antigen card you control as it is
// played; its cost is not paid." [LSJ 20061207] [ANK 20190104] [RBK
// cancel-a-card]: in the as-played window; the canceled card has still
// been played; "if an action is canceled, it is not performed and the
// minion is free to attempt the same action again." [ANK 20200813-2]:
// "Can be used to cancel all action cards."
#[test]
fn mobile_hq_cancels_an_action_card_at_an_antigen_ally_as_it_is_played() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch_with(&["pot"], &[], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let (ectu, _) = antigen(&mut e, 1, "ECTU Operative");
    let hq = location(&mut e, 1, "Mobile HQ, Operation Antigen");
    hand(&mut e, 0, &["Line Brawl", "Line Brawl"]);
    let brawl = play_at(0, 1, Some(a), Target::Minion(ectu));
    pick(&mut e, brawl);
    let cancel = use_on(hq, 2, None, Target::None);
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert!(req.options.contains(&cancel));
    let p1 = pool(&e, 1);
    pick(&mut e, cancel);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), p1 - 2);
    assert!(e.state.cards[hq.index()].locked);
    assert!(!e.state.minions[a.index()].locked, "not performed");
    assert!(e.state.seats[0].ash.contains(&find_library("Line Brawl")));
    assert!(req.options.contains(&brawl), "free to attempt it again");
}

// [ANK 20220818] [ANK 20221102] Enhanced Coagulant: "Cannot be used against
// an ally."
#[test]
fn enhanced_coagulant_cannot_strike_an_ally() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let (ectu, _) = antigen(&mut e, 1, "ECTU Operative");
    let coag = equip(&mut e, "Enhanced Coagulant", a);
    bleed_blocked(&mut e, a, ectu);
    never_offered(&mut e, &weapon_strike(coag));
}

// [LSJ 20060106] "A card played \"as if from hand\" can be canceled \"as
// played\"": Direct Intervention on a ghoul played from Fleshforge Chamber;
// played from there, it is not replaced (rules-qa 37.1).
#[test]
fn direct_intervention_cancels_a_card_played_from_fleshforge_chamber() {
    let mut e = Engine::bare(3);
    let t = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Tzimisce" && c.title == Title::None, 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    let ff = location(&mut e, 0, "Fleshforge Chamber");
    let ghoul = find_library("Szlachta Assistant");
    e.state.cards[ff.index()].stored = vec![ghoul];
    e.state.seats[0].library = vec![find_library("Soak"); 3];
    hand(&mut e, 1, &["Direct Intervention"]);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, t));
    pick_when(&mut e, master(0));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].locked, "no action");
    assert!(e.state.cards[ff.index()].stored.is_empty());
    assert!(e.state.seats[0].ash.contains(&ghoul));
    assert!(e.state.seats[0].hand.is_empty(), "not replaced");
    assert!(!req.options.contains(&play(0, 0, t)));
}

// [PIB 20120214] Michael Harris: "If another effect was already setting that
// round's range (e.g. Neutral Guard), that effect has priority." A Sniper
// Rifle's blocker sets it first: Michael cannot set it again.
#[test]
fn michael_harris_yields_to_a_range_already_set() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let (mh, card) = antigen(&mut e, 0, "Michael Harris, the Incinerator");
    let b = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let rifle = equip(&mut e, "Sniper Rifle", b);
    pick(&mut e, action(mh, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: b });
    let req = request(&mut e);
    assert!(
        req.options.contains(&card_ability(card, 1)),
        "offered first"
    );
    e.choose(fallback(&req));
    pick_when(&mut e, card_ability(rifle, 1));
    never_offered(&mut e, &card_ability(card, 1));
}

// [LSJ 20011204-3] [PIB 20130704] "When played, a multi-Disciplines card
// counts as requiring the Discipline(s) being used. In the hand, library,
// or ash heap, the card is considered to require any and/or all
// Disciplines listed on it." Party Out Of Bounds played with Obfuscate
// beside Libertas' Anarch costs no blood (it lists Presence); in the hand,
// it is a card requiring Protean for The Vozhd of Szczecin to discard.
// Rules-qa 38.4 (owner, 2026-09-26): "Cards requiring a Discipline are those
// played using a Discipline."
#[test]
fn party_out_of_bounds_requires_the_discipline_used_or_any_listed_in_hand() {
    // As played: Obfuscate only — offered at 0 blood, and nothing paid at 1.
    for b in [0, 1] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), anarch_with(&[], &[], 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), anarch_with(&["obf"], &[], 0), b, false);
        let lib = location(&mut e, 0, "Libertas");
        e.state.cards[lib.index()].host = Some(a);
        hand(&mut e, 1, &["Party Out Of Bounds"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick_when(&mut e, play(0, 0, z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 1), 30, "the bleed of 1 reduced by 2 ({b} blood)");
        assert_eq!(blood(&e, z), b, "no Libertas surcharge");
    }
    // In hand: listing Protean, it may be discarded to prevent.
    for (card, offered) in [("Party Out Of Bounds", true), ("Soak", false)] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let t = e.add_minion(SeatId(1), plain("Tzimisce", "", 0), 4, false);
        equip(&mut e, "Revenant", t);
        let (vozhd, id) = antigen(&mut e, 1, "Vozhd of Szczecin, The");
        request(&mut e); // its entry burns the Revenant
        hand(&mut e, 1, &[card]);
        bleed_blocked(&mut e, a, vozhd);
        let discard = offered_before(
            &mut e,
            use_on(id, 1, None, Target::None),
            SeatId(0),
            WindowLabel::MinionPhase,
        );
        assert_eq!(discard, offered, "{card}");
    }
}

// Rules-qa 38.5 (owner, 2026-09-26): Ravnos Cache "should be usable here:
// reductions and replacement payment offered after it is chosen." Michael
// Harris finds a .44 Magnum he can only pay with the Cache's counters.
#[test]
fn a_searched_equipment_takes_ravnos_cache_counters_once_chosen() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let (mh, card) = antigen(&mut e, 0, "Michael Harris, the Incinerator");
    let cache = in_play_with(&mut e, 0, "Ravnos Cache", 2);
    let magnum = find_library(".44 Magnum");
    e.state.seats[0].library = vec![magnum];
    e.state.seats[0].pool = 1;
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(card),
            ability: 0,
            actor: Some(mh),
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    // Payable with the Cache.
    pick(&mut e, Choice::FindCard(magnum));
    // Unaffordable without it, the Cache is the only choice (elided): its
    // counters next.
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(req.options, vec![Choice::SetX(1), Choice::SetX(2)]);
    pick(&mut e, Choice::SetX(2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 0), 1, "paid by the counters");
    assert_eq!(e.state.cards[cache.index()].counters, 0);
    assert!(e.state.cards[cache.index()].locked);
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == magnum && c.host == Some(mh)));
}

// ---- Political actions (slice 11ab) -------------------------------------------

/// An untitled vampire of `clan` and `sect`, no printed text.
fn of_sect(clan: &str, sect: zvtes::crypt::Sect, nth: usize) -> u16 {
    vampire(
        |c| c.clan == clan && c.sect == sect && c.title == Title::None,
        nth,
    )
}

/// The referendum passes: the calling Methuselah votes for with its vampire,
/// its political action card or the Edge; the others pass.
fn pass_referendum(e: &mut Engine, caller: MinionId) {
    e.state.seats[e.state.minions[caller.index()].owner.index()].has_edge = true;
    for _ in 0..200 {
        let req = request(e);
        let yes = [
            votes(caller, true),
            Choice::CastCardVote { favor: true },
            Choice::CastEdge { favor: true },
        ];
        if let Some(i) = req.options.iter().position(|o| yes.contains(o)) {
            e.choose(i);
            close_polling(e);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("no vote for the referendum");
}

// Malkavian Justicar: "Title. Choose a ready Camarilla Malkavian. Successful
// referendum means this card is put on the chosen Malkavian to represent
// the unique Camarilla title of Malkavian Justicar. In this referendum,
// each Malkavian gets +1 vote." [LSJ 20100129] [ANK 20191228]: not playable
// if the terms cannot be met.
#[test]
fn malkavian_justicar_titles_a_ready_camarilla_malkavian() {
    use zvtes::crypt::Sect;
    let mut e = Engine::bare(3);
    let m = e.add_minion(
        SeatId(0),
        of_sect("Malkavian", Sect::Camarilla, 0),
        5,
        false,
    );
    // Made Anarch (as by Twilight Camp): no longer Camarilla.
    let a = e.add_minion(
        SeatId(0),
        of_sect("Malkavian", Sect::Camarilla, 1),
        5,
        false,
    );
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    hand(&mut e, 0, &["Malkavian Justicar"]);
    e.state.minions[m.index()].in_torpor = true;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !req.options.contains(&play(0, 0, a)),
        "no ready Camarilla Malkavian"
    );
    e.state.minions[m.index()].in_torpor = false;
    pick(&mut e, play(0, 0, a));
    // The only choice (not the Anarch) is elided: the polling opens.
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert_eq!(e.referendum().expect("referendum").terms.minions, vec![m]);
    assert!(
        req.options.contains(&votes(m, true)),
        "+1 vote each Malkavian"
    );
    pass_referendum(&mut e, m);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(m), Title::Justicar("Malkavian"));
}

// Archon: "Requires a prince or justicar. Choose a Camarilla vampire.
// Successful referendum means this card is put on the chosen vampire. The
// attached vampire can enter combat with a vampire as a +1 stealth Ⓓ
// action. Vampires attempting to block the attached vampire burn 1 blood."
// [LSJ 19970707] "A vampire with no blood can still block, they burn what
// they can." [LSJ 19980225] "Burn blood effects are cumulative."
#[test]
fn archon_puts_on_a_camarilla_vampire_its_combat_action_and_blocking_toll() {
    use zvtes::crypt::Sect;
    let mut e = Engine::bare(3);
    let p = e.add_minion(SeatId(0), prince(0), 5, false);
    let c = e.add_minion(SeatId(0), of_sect("Ventrue", Sect::Camarilla, 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 0, false);
    let y = e.add_minion(SeatId(1), plain("Gangrel", "", 1), 3, false);
    // Keeps seat 0's minion phase open.
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    hand(&mut e, 0, &["Archon"]);
    pick(&mut e, play(0, 0, p));
    pick_when(&mut e, Choice::ChooseMinion(c));
    pass_referendum(&mut e, p);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let archon = e
        .state
        .cards
        .iter()
        .position(|k| k.in_play && k.card == find_library("Archon"))
        .map(|i| CardId(i as u32))
        .expect("in play");
    assert_eq!(e.state.cards[archon.index()].host, Some(c));
    assert!(req.options.contains(&granted(archon, 0, c, y)));
    assert!(
        !req.options.contains(&granted(archon, 0, p, y)),
        "the attached only"
    );
    // A second Archon on it: blockers burn 2 (LSJ 19980225).
    let second = location(&mut e, 0, "Archon");
    e.state.cards[second.index()].host = Some(c);
    pick(&mut e, action(c, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: y });
    assert_eq!(blood(&e, y), 1, "2 burned");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // With no blood, `z` burns none and still blocks (LSJ 19970707).
    let m = &mut e.state.minions[c.index()];
    m.locked = false;
    m.acted.clear();
    m.bled_this_turn = false;
    let p1 = pool(&e, 1);
    pick(&mut e, action(c, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    assert_eq!(blood(&e, z), 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), p1, "blocked");
}

// Archon: "Blood hunts cannot be called on the attached vampire."
#[test]
fn archon_bars_a_blood_hunt_on_its_vampire() {
    use zvtes::crypt::Sect;
    let mut e = Engine::bare(3);
    let c = e.add_minion(SeatId(0), of_sect("Ventrue", Sect::Camarilla, 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let t = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 2, true);
    e.add_minion(SeatId(1), plain("Gangrel", "", 1), 5, false);
    let archon = location(&mut e, 0, "Archon");
    e.state.cards[archon.index()].host = Some(c);
    // The Edge: a vote to cast, so a blood hunt would show its polling.
    e.state.seats[1].has_edge = true;
    pick(
        &mut e,
        Choice::TakeAction {
            minion: c,
            action: ActionKind::Diablerie,
            target: Some(t),
        },
    );
    loop {
        let req = request(&mut e);
        assert_ne!(req.window, WindowLabel::BloodHuntPolling);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(!e.state.minions[t.index()].in_play, "diablerised");
    assert!(e.state.minions[c.index()].in_play, "no blood hunt");
}

// Archon: "Camarilla vampires can call a referendum to burn this card as a
// +1 stealth political action." [RTR 20030519] "Each copy of this card
// provides its own action." [LSJ 20100909] "The two actions provided are not
// cross-restrictive."
#[test]
fn archon_can_be_burned_by_a_camarilla_referendum() {
    use zvtes::crypt::Sect;
    let mut e = Engine::bare(3);
    let c = e.add_minion(SeatId(0), of_sect("Ventrue", Sect::Camarilla, 0), 5, false);
    let n = e.add_minion(SeatId(0), of_sect("Brujah", Sect::Anarch, 0), 5, false);
    let y = e.add_minion(SeatId(1), plain("Gangrel", "", 1), 5, false);
    let archon = location(&mut e, 0, "Archon");
    e.state.cards[archon.index()].host = Some(c);
    let burn = |m| Choice::UseAbility {
        source: AbilitySource::Card(archon),
        ability: 1,
        actor: Some(m),
        target: Target::None,
    };
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&burn(c)));
    assert!(!req.options.contains(&burn(n)), "not Camarilla");
    // A second copy on it: its own action.
    let second = location(&mut e, 0, "Archon");
    e.state.cards[second.index()].host = Some(c);
    // Its combat action first; the referendum is another action.
    pick(&mut e, granted(archon, 0, c, y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let m = &mut e.state.minions[c.index()];
    m.locked = false;
    let req = request(&mut e);
    assert!(req.options.contains(&burn(c)), "not cross-restrictive");
    assert!(!req.options.contains(&granted(archon, 0, c, y)), "used");
    assert!(req.options.contains(&granted(second, 0, c, y)), "per copy");
    pick(&mut e, burn(c));
    assert_eq!(e.action().expect("action").base_stealth, 1);
    pass_referendum(&mut e, c);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[archon.index()].in_play);
}

// Raising the Portcullis: "Successful referendum means this card is put in
// play. While your prey controls a vampire in torpor, vampires you control
// get +1 bleed. … A Methuselah can have only one Raising the Portcullis."
// [LSJ 20100129] [ANK 20191228] Not playable when its terms are prohibited
// by card text: a second copy.
#[test]
fn raising_the_portcullis_bleeds_more_while_the_prey_has_a_vampire_in_torpor() {
    let mut e = Engine::bare(3);
    let l = e.add_minion(SeatId(0), plain("Lasombra", "", 0), 5, false);
    let v = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let t = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 2, false);
    hand(
        &mut e,
        0,
        &["Raising the Portcullis", "Raising the Portcullis"],
    );
    pick(&mut e, play(0, 0, l));
    pass_referendum(&mut e, l);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = find_library("Raising the Portcullis");
    assert!(e.state.cards.iter().any(|c| c.in_play && c.card == card));
    // The second copy is not playable.
    let m = &mut e.state.minions[l.index()];
    m.locked = false;
    m.political_this_turn = false;
    let _ = req;
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, l)));
    // No vampire in torpor: a plain bleed; then one in torpor: +1.
    let p1 = pool(&e, 1);
    pick(&mut e, action(v, ActionKind::Bleed));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), p1 - 1);
    e.state.minions[t.index()].in_torpor = true;
    e.add_minion(SeatId(1), plain("Gangrel", "", 1), 5, false);
    let m = &mut e.state.minions[v.index()];
    m.locked = false;
    m.acted.clear();
    m.bled_this_turn = false;
    pick(&mut e, action(v, ActionKind::Bleed));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), p1 - 3, "+1 bleed");
}

// Disputed Territory: "Choose a location and a Methuselah. Successful
// referendum means the chosen Methuselah takes control of the chosen
// location." [TOM 19960226-1] "Requirements do not apply when taking
// control of a card already in play." [LSJ 20100129] [ANK 20191228] Not
// playable with no location in play. Rules-qa 38.6: choosing Mobile HQ is
// no targeting — no cancel offered.
#[test]
fn disputed_territory_moves_a_location_to_the_chosen_methuselah() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    e.add_minion(SeatId(2), plain("Brujah", "", 0), 5, false);
    hand(&mut e, 0, &["Disputed Territory"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, a)), "no location in play");
    let hq = location(&mut e, 1, "Mobile HQ, Operation Antigen");
    let tremere = location(&mut e, 1, "Academic Hunting Ground");
    e.state.seats[1].pool = 30;
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert_eq!(
        req.options,
        vec![Choice::ChooseCard(hq.0), Choice::ChooseCard(tremere.0)]
    );
    assert_eq!(pool(&e, 1), 30, "no Mobile HQ cancel");
    pick(&mut e, Choice::ChooseCard(tremere.0));
    pick(&mut e, Choice::ChooseSeat(SeatId(0)));
    pass_referendum(&mut e, a);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.card_controller(tremere),
        SeatId(0),
        "no Tremere needed"
    );
}

// [RTR 19960112] [RTR 19980623] Disputed Territory on an equipment
// representing a location: "it can be placed on any minion controlled by
// the new controller"; [LSJ 19971002] given to its own Methuselah, it
// cannot move to another of their minions. [RTR 19960112] Living Manse:
// "If the new controller has no minions, it is burned" (seat 2).
#[test]
fn disputed_territory_moves_a_location_on_a_minion_to_one_of_the_new_controllers() {
    for to in [SeatId(0), SeatId(1), SeatId(2)] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let b = e.add_minion(SeatId(0), plain("Ventrue", "", 1), 5, false);
        let y = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
        let manse = equip(&mut e, "Living Manse", y);
        hand(&mut e, 0, &["Disputed Territory"]);
        pick(&mut e, play(0, 0, a));
        // The Manse, the only location, is chosen (elided).
        pick(&mut e, Choice::ChooseSeat(to));
        assert_eq!(e.referendum().expect("referendum").terms.cards, vec![manse]);
        pass_referendum(&mut e, a);
        if to == SeatId(0) {
            let req = request(&mut e);
            assert!(req.options.contains(&Choice::ChooseMinion(b)));
            e.choose(
                req.options
                    .iter()
                    .position(|o| *o == Choice::ChooseMinion(b))
                    .unwrap(),
            );
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.cards[manse.index()].host, Some(b));
        } else if to == SeatId(2) {
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(!e.state.cards[manse.index()].in_play, "no minion: burned");
        } else {
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.cards[manse.index()].host, Some(y), "stays");
        }
    }
}

// Revolutionary Council: "Requires a baron. Choose X ready unlocked Anarchs
// you control and allocate 2X points among one or more Methuselahs,
// locations, and equipment. Successful referendum means each chosen Anarch
// is locked, each Methuselah burns 1 pool for each point allocated, and
// each location or equipment allocated a point is burned." [LSJ 20081203]
// "Unlocked Anarchs are chosen when choosing terms. They still count if they
// get locked during the vote." [LSJ 20020819] "Can allocate more points
// than a Methuselah has pool."
#[test]
fn revolutionary_council_locks_its_anarchs_to_burn_pool_and_cards() {
    let mut e = Engine::bare(3);
    let b = e.add_minion(SeatId(0), baron(0), 5, false);
    let x = e.add_minion(SeatId(0), anarch_with(&[], &[], 0), 5, false);
    let w = e.add_minion(SeatId(0), anarch_with(&[], &[], 1), 5, false);
    let y = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    // A non-Anarch: never chosen; keeps the minion phase open.
    let v = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let gun = equip(&mut e, ".44 Magnum", y);
    e.state.seats[1].pool = 2;
    hand(&mut e, 0, &["Revolutionary Council"]);
    pick(&mut e, play(0, 0, b));
    let req = until(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert!(
        !req.options.contains(&Choice::ChooseMinion(b)),
        "locked, acting"
    );
    pick(&mut e, Choice::ChooseMinion(x));
    // No other Anarch to choose: the Pass is elided.
    pick(&mut e, Choice::ChooseMinion(w));
    // 4 points: seat 0 first, then seats 1, 2, then the Magnum.
    pick(
        &mut e,
        Choice::AllocateTo {
            seat: SeatId(0),
            n: 0,
        },
    );
    pick(
        &mut e,
        Choice::AllocateTo {
            seat: SeatId(1),
            n: 3,
        },
    );
    pick(
        &mut e,
        Choice::AllocateTo {
            seat: SeatId(2),
            n: 0,
        },
    );
    // The Magnum's 1 point is the only choice left (elided).
    // `x` gets locked during the vote: still counted.
    e.state.minions[x.index()].locked = true;
    pass_referendum(&mut e, b);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&action(v, ActionKind::Bleed)));
    assert!(e.state.minions[w.index()].locked && e.state.minions[x.index()].locked);
    assert!(!e.state.cards[gun.index()].in_play, "burned");
    assert!(e.state.seats[1].ousted, "3 points on 2 pool");
}

// `Engine::allocation` over cards: Revolutionary Council's 2X points go to
// the Methuselahs, then to the locations and equipment in play (one or
// more recipients).
#[test]
fn revolutionary_council_allocation_is_read_whole() {
    use zvtes::referendum::{Allocation, Recipient};
    let mut e = Engine::bare(3);
    let b = e.add_minion(SeatId(0), baron(0), 5, false);
    let x = e.add_minion(SeatId(0), anarch_with(&[], &[], 0), 5, false);
    let w = e.add_minion(SeatId(0), anarch_with(&[], &[], 1), 5, false);
    let y = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let gun = equip(&mut e, ".44 Magnum", y);
    hand(&mut e, 0, &["Revolutionary Council"]);
    pick(&mut e, play(0, 0, b));
    until(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    pick(&mut e, Choice::ChooseMinion(x));
    pick(&mut e, Choice::ChooseMinion(w));
    // No other Anarch to choose: the Pass is elided.
    expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    let seat = |s| Recipient::Seat(SeatId(s));
    assert_eq!(
        e.allocation(),
        Some(Allocation {
            points: 4,
            recipients: vec![seat(0), seat(1), seat(2), Recipient::Card(gun)],
            caps: vec![None; 4],
            min_nonzero: 1,
        })
    );
    pick(&mut e, alloc(0, 1));
    assert_eq!(
        e.allocation().map(|a| (a.points, a.min_nonzero)),
        Some((3, 0))
    );
}

// [LSJ 20080603] [LSJ 20050128] Revolutionary Council "counts as requiring
// an Anarch (for eg. Powerbase: Los Angeles)": discarded with Los Angeles's
// discard phase action, it unlocks a ready Anarch.
#[test]
fn revolutionary_council_is_a_card_requiring_an_anarch() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch_with(&[], &[], 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 1), 4, false);
    hand(&mut e, 0, &["Revolutionary Council"]);
    let la = location(&mut e, 0, "Powerbase: Los Angeles");
    e.state.minions[a.index()].locked = true;
    until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, use_on(la, 0, None, Target::None));
    pick(&mut e, Choice::Discard(0));
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, Choice::ChooseMinion(a)]);
}

// Rules-qa 39.1 (owner, 2026-09-27): Revolutionary Council's "X can be zero
// and the vote does nothing much. The acting baron can be chosen if
// unlocked (eg. using CrimethInc.)."
#[test]
fn revolutionary_council_takes_zero_anarchs_or_its_unlocked_baron() {
    for unlock in [false, true] {
        let mut e = Engine::bare(3);
        let b = e.add_minion(SeatId(0), baron(0), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
        hand(&mut e, 0, &["Revolutionary Council"]);
        pick(&mut e, play(0, 0, b));
        let req = until(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
        assert!(e.referendum().expect("referendum").terms.minions.is_empty());
        let _ = req;
        if !unlock {
            // X = 0: no points to allocate; it passes and does nothing.
            pass_referendum(&mut e, b);
            request(&mut e);
            assert_eq!((pool(&e, 0), pool(&e, 1)), (30, 30));
            continue;
        }
        // Unlocked again during the action (as by CrimethInc.): it may be
        // chosen. A prince able to block gives the action a window.
        let mut e = Engine::bare(3);
        let b = e.add_minion(SeatId(0), baron(0), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        e.add_minion(SeatId(1), prince(0), 5, false);
        hand(&mut e, 0, &["Revolutionary Council"]);
        hand(&mut e, 1, &["Second Tradition: Domain"]);
        pick(&mut e, play(0, 0, b));
        let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
        e.state.minions[b.index()].locked = false;
        e.choose(fallback(&req));
        let req = until(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
        assert_eq!(req.options, vec![Choice::Pass, Choice::ChooseMinion(b)]);
    }
}

// Rules-qa 38.8 (owner, 2026-09-27): Valerie Diaz's immunity is "once a
// round, for a single strike or damaging effect in that round." Immune to
// the strike, she still takes Carrion Crows' damage that round.
#[test]
fn valerie_diaz_is_immune_to_one_strike_or_effect_a_round() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ani"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let (vd, card) = antigen(&mut e, 1, "Valerie Diaz, Antigen Commander");
    hand(&mut e, 0, &["Carrion Crows"]);
    bleed_blocked(&mut e, a, vd);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, card_ability(card, 0));
    // The other immunity is not offered this round: no prevention left.
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::CombatPrevention, "once a round");
    assert_eq!(blood(&e, vd), 2, "Carrion Crows' 1 damage");
}

// Rules-qa 38.5 follow-up (owner, 2026-09-27): Vast Wealth's found
// equipment "should offer reductions" too. The Magnum found, Ravnos Cache
// pays it: its 2 pool would have ousted a Methuselah with 1.
#[test]
fn vast_wealth_takes_ravnos_cache_counters_for_its_find() {
    let mut e = Engine::bare(3);
    let t = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let wealth = location(&mut e, 0, "Vast Wealth");
    e.state.cards[wealth.index()].host = Some(t);
    let cache = in_play_with(&mut e, 0, "Ravnos Cache", 2);
    let magnum = find_library(".44 Magnum");
    e.state.seats[0].library = vec![magnum];
    e.state.seats[0].pool = 1;
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(wealth),
            ability: 0,
            actor: Some(t),
            target: Target::None,
        },
    );
    // The Cache, then its counters (a single choice elided).
    pick_when(&mut e, Choice::SetX(2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.seats[0].ousted);
    assert_eq!(pool(&e, 0), 1);
    assert_eq!(e.state.cards[cache.index()].counters, 0);
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == magnum && c.host == Some(t)));
}

// [LSJ 19990928] "Loquipment may be put on any ready minion if moved in an
// ambiguous fashion (by Disputed Territory, e.g.). If the new controller
// has no minions, the locquipment is burned." (owner, 2026-09-27: rules-qa
// 39.4, Dominate Kine alike): given to a Methuselah whose only vampire is
// in torpor, or stolen by one whose minions each hold a Living Manse
// already, it is burned.
#[test]
fn a_locquipment_with_no_ready_minion_to_hold_it_is_burned() {
    // Disputed Territory: seat 2's only vampire is in torpor.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 1), 5, false);
    let y = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    e.add_minion(SeatId(2), plain("Brujah", "", 0), 2, true);
    let manse = equip(&mut e, "Living Manse", y);
    hand(&mut e, 0, &["Disputed Territory"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ChooseSeat(SeatId(2)));
    pass_referendum(&mut e, a);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[manse.index()].in_play, "burned");
    assert!(e.state.seats[1].ash.contains(&find_library("Living Manse")));
    // Dominate Kine [DOM]: each of the thief's minions has one already.
    let mut e = Engine::bare(3);
    let tz = |n| vampire(|c| c.clan == "Tzimisce" && c.title == Title::None, n);
    let a = e.add_minion(SeatId(0), with(&["DOM"], 0), 4, false);
    let t = e.add_minion(SeatId(0), tz(0), 4, false);
    let z = e.add_minion(SeatId(1), tz(1), 4, false);
    equip(&mut e, "Living Manse", a);
    equip(&mut e, "Living Manse", t);
    let stolen = equip(&mut e, "Living Manse", z);
    hand(&mut e, 0, &["Dominate Kine"]);
    pick(&mut e, play_at(0, 1, Some(a), Target::Card(stolen)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[stolen.index()].in_play, "burned");
}

// ---- Events (slice 11ac) -------------------------------------------------------

/// Put the event `name` in play for `seat`, with `counters`.
fn event_in_play(e: &mut Engine, seat: u8, name: &str, counters: u8) -> CardId {
    in_play_with(e, seat, name, counters)
}

// Rulebook "Event Cards": "Once each discard phase, a Methuselah may use a
// discard phase action to put an event card in play. Each event can only
// be played once in a game. An event card is controlled by the Methuselah
// who played it." An event is no minion card: Direct Intervention cannot
// cancel it. Narrow Minds: "Do not replace until your next unlock
// phase." [ANK 20200629-2] [LSJ 20091208] [RTR 19951017]: "Is replaced
// before unlocking cards."
#[test]
fn an_event_takes_a_discard_phase_action_once_a_phase_and_once_a_game() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    e.add_minion(SeatId(2), plain("Brujah", "", 0), 5, false);
    hand(&mut e, 0, &["Narrow Minds", "Torpid Blood"]);
    // Direct Intervention cancels minion cards: no event.
    hand(&mut e, 1, &["Narrow Minds", "Direct Intervention"]);
    for s in 0..3 {
        e.state.seats[s].library = vec![find_library("Soak"); 5];
    }
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(req.options.contains(&master(0)) && req.options.contains(&master(1)));
    // A second discard phase action: still one event this phase.
    e.state.seats[0].dpa = 2;
    pick(&mut e, master(0));
    let req = expect(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(!req.options.contains(&master(0)), "one event a phase");
    assert!(req.options.contains(&Choice::Discard(0)), "a discard left");
    let minds = find_library("Narrow Minds");
    let id = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == minds)
        .map(|i| CardId(i as u32))
        .expect("in play");
    assert_eq!(e.state.card_controller(id), SeatId(0));
    assert_eq!(e.state.seats[0].held.unlock, 1, "not replaced yet");
    // Seat 1's copy: never offered.
    let req = until(&mut e, SeatId(1), WindowLabel::DiscardPhase);
    assert!(!req.options.contains(&master(0)), "once a game");
    // Seat 0's next unlock phase: replaced first.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].held.unlock, 0);
}

// Narrow Minds: "Minion cards that change the target of a bleed action cost
// +1 blood or life."
#[test]
fn narrow_minds_taxes_bleed_redirection() {
    for blood in [1, 2] {
        let (mut e, a, t) = bleed_at(3, plain("", "AUS", 0), "Telepathic Misdirection");
        e.state.minions[t.index()].blood = blood;
        event_in_play(&mut e, 2, "Narrow Minds", 0);
        pick(&mut e, action(a, ActionKind::Bleed));
        let redirect = play_at(0, 1, Some(t), Target::Seat(SeatId(2)));
        if blood == 1 {
            never_offered(&mut e, &redirect);
        } else {
            pick_when(&mut e, redirect);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.minions[t.index()].blood, 0, "2 blood");
        }
    }
}

// Torpid Blood: "Do not replace until a vampire moves from torpor to the
// ready region. Actions performed by vampires in torpor cost +1 blood.
// Rescuing an older vampire from torpor costs +1 blood." [LSJ 20080805]
// "Is not replaced until the condition is met, even if it is burned."
#[test]
fn torpid_blood_taxes_torpor_and_waits_for_a_vampire_to_leave_it() {
    let mut e = Engine::bare(3);
    let t = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 2, true);
    let r = e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    e.state.seats[0].library = vec![find_library("Soak"); 5];
    let torpid = event_in_play(&mut e, 0, "Torpid Blood", 0);
    e.state.seats[0].held.leaves_torpor = 1;
    // Leave torpor costs 3: not with 2 blood.
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&action(t, ActionKind::LeaveTorpor)));
    // Burned, its replacement still waits.
    e.burn_card(torpid);
    assert_eq!(e.state.seats[0].held.leaves_torpor, 1);
    let torpid = event_in_play(&mut e, 0, "Torpid Blood", 0);
    let _ = torpid;
    // Rescuing it costs 2, or 3 if it is older than the rescuer.
    let older = e.state.younger_than(r, t);
    let rescue = Choice::TakeAction {
        minion: r,
        action: ActionKind::Rescue,
        target: Some(t),
    };
    pick(&mut e, rescue);
    // The rescuer pays it all: chosen as it is announced, paid as it
    // resolves (rules-qa 48.8).
    let cost = 2 + u8::from(older);
    pick(&mut e, Choice::PayRescue { from_acting: cost });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, r), 5 - cost);
    assert!(!e.state.minions[t.index()].in_torpor);
    assert_eq!(e.state.seats[0].held.leaves_torpor, 0, "replaced");
}

// Rescue: who pays the 2 blood is chosen as the action is announced, the
// blood paid as it resolves (owner, 2026-10-04: rules-qa 48.8); a share its
// payer can no longer meet falls to the other.
#[test]
fn rescue_cost_split_is_chosen_as_announced_and_paid_at_resolution() {
    for (late_blood, paid) in [(4, (2, 3)), (1, (0, 2))] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        e.add_minion(SeatId(0), plain("", "", 3), 4, false);
        let n = e.add_minion(SeatId(0), plain("Nosferatu", "", 1), 3, true);
        e.add_minion(SeatId(1), plain("", "", 2), 4, false);
        pick(
            &mut e,
            Choice::TakeAction {
                minion: a,
                action: ActionKind::Rescue,
                target: Some(n),
            },
        );
        // Asked now, not at resolution.
        let req = expect(&mut e, SeatId(0), WindowLabel::RescueCost);
        assert_eq!(req.options.len(), 3);
        pick(&mut e, Choice::PayRescue { from_acting: 2 });
        assert!(e.state.minions[n.index()].in_torpor, "not paid yet");
        assert_eq!((blood(&e, a), blood(&e, n)), (4, 3));
        // The rescuer's blood by resolution.
        e.state.minions[a.index()].blood = late_blood;
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[n.index()].in_torpor);
        assert_eq!((blood(&e, a), blood(&e, n)), paid);
    }
}

// The Slow Withering: "Cards requiring 1 or more Disciplines at the
// superior level cost +1 blood. Vampires who commit diablerie ignore this
// effect until a Gehenna event is played." [ANK 20190625] "The additional
// cost is paid at the same time the normal cost is paid."
#[test]
fn the_slow_withering_taxes_superior_cards_but_not_a_diablerist() {
    for shield in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["DOM"], 0), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
        e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
        event_in_play(&mut e, 1, "Slow Withering, The", 0);
        e.state.minions[a.index()].diablerie_shield = shield;
        hand(&mut e, 0, &["Conditioning"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        let req = request(&mut e);
        assert!(req.options.contains(&play(0, 0, a)), "[dom]: 1 blood");
        assert_eq!(req.options.contains(&play(0, 1, a)), shield, "[DOM]: 2");
    }
}

// Blood Weakens: "During an action, cards minions play requiring 1 or more
// Disciplines are not replaced until the end of that action or until a
// Methuselah's hand is empty (whichever comes first)." [ANK 20201015] "You
// do not replace cards that require a discipline during any action."
#[test]
fn blood_weakens_holds_discipline_card_replacements_until_the_action_ends() {
    for spare in [true, false] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["dom"], 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
        e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
        event_in_play(&mut e, 1, "Blood Weakens", 0);
        e.state.seats[0].library = vec![find_library("Soak"); 5];
        hand(&mut e, 0, &["Conditioning"]);
        if spare {
            hand(&mut e, 0, &["Soak"]);
        }
        pick(&mut e, action(a, ActionKind::Bleed));
        pick(&mut e, play(0, 0, a));
        request(&mut e);
        // Held, it counts against the hand size: no draw-up (hand 1); with
        // the hand emptied, drawn at once (and up to the hand size).
        let hand_now = e.state.seats[0].hand.len();
        if spare {
            assert_eq!(hand_now, 1);
        } else {
            assert!(hand_now > 1, "drawn up");
        }
        assert_eq!(e.state.seats[0].held.weakens, u8::from(spare));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.seats[0].held.weakens, 0,
            "replaced as the action ends"
        );
        assert!(e.state.seats[0].hand.len() > 1);
    }
}

/// The `nth` untitled vampire of capacity `cap`, no printed text.
fn of_capacity(cap: u8, nth: usize) -> u16 {
    vampire(|c| c.capacity == cap && c.title == Title::None, nth)
}

// Thirst: "After each Methuselah's minion phase ends, each ready vampire
// controlled by that Methuselah with capacity less than the number of
// Gehenna events in play who did not hunt during that minion phase burns 1
// blood." [LSJ 20050727] "The hunt need not be successful."
#[test]
fn thirst_burns_a_blood_from_small_vampires_that_did_not_hunt() {
    let mut e = Engine::bare(3);
    let idle = e.add_minion(SeatId(0), of_capacity(3, 0), 3, false);
    let hunter = e.add_minion(SeatId(0), of_capacity(3, 1), 3, false);
    let big = e.add_minion(SeatId(0), of_capacity(5, 0), 3, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    for name in ["Thirst", "Blood Weakens", "Torpid Blood", "Narrow Minds"] {
        event_in_play(&mut e, 1, name, 0);
    }
    // Narrow Minds is no Gehenna event: 3 in play, capacity 3 unaffected.
    pick(&mut e, action(hunter, ActionKind::Hunt));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, idle), 3, "3 is not less than 3");
    // A fourth Gehenna event: now it is.
    let mut e2 = Engine::bare(3);
    let idle = e2.add_minion(SeatId(0), of_capacity(3, 0), 3, false);
    let hunter = e2.add_minion(SeatId(0), of_capacity(3, 1), 3, false);
    let big = {
        let _ = big;
        e2.add_minion(SeatId(0), of_capacity(5, 0), 3, false)
    };
    e2.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    for name in [
        "Thirst",
        "Blood Weakens",
        "Torpid Blood",
        "Slow Withering, The",
    ] {
        event_in_play(&mut e2, 1, name, 0);
    }
    // A hunt blocked or not, gaining blood or not: it hunted.
    e2.state.minions[hunter.index()].blood = 3;
    pick(&mut e2, action(hunter, ActionKind::Hunt));
    until(&mut e2, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e2, idle), 2, "did not hunt");
    assert_eq!(blood(&e2, hunter), 3, "hunted, to capacity");
    assert_eq!(blood(&e2, big), 3, "capacity 5");
}

// Wormwood: "This card comes into play with 10 counters. After another
// Gehenna event is played, burn 1 counter from this card. Vampires with
// capacity greater than X are considered vampires with capacity X
// (minimum of 1)." [LSJ 20051205] It ceils the capacity as changed by
// other effects. The Slow Withering needs another Gehenna event in play:
// [PIB 20121031] "The \"other Gehenna cards in play\" requirement is only
// checked when playing the card."
#[test]
fn wormwood_ceils_capacity_by_its_counters() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), of_capacity(5, 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    e.state.seats[0].library = vec![find_library("Soak"); 5];
    hand(&mut e, 0, &["Slow Withering, The"]);
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(!req.options.contains(&master(0)), "no other Gehenna event");
    let ww = event_in_play(&mut e, 1, "Wormwood", 2);
    assert_eq!(e.state.capacity_of(v), Some(2));
    pick(&mut e, master(0));
    request(&mut e);
    assert_eq!(e.state.cards[ww.index()].counters, 1);
    assert_eq!(e.state.capacity_of(v), Some(1));
    assert_eq!(blood(&e, v), 1, "drained");
    // Burned: its capacity back.
    e.burn_card(ww);
    assert_eq!(e.state.capacity_of(v), Some(5));
}

// Fueled by Heart's Blood: "Blood hunts cannot be called on vampires with
// capacity greater than the number of counters on this card who diablerize
// a younger vampire."
#[test]
fn fueled_by_hearts_blood_bars_a_blood_hunt_on_a_big_diablerist() {
    for counters in [10, 3] {
        let mut e = Engine::bare(3);
        // Anja's text (unlocking another Hecata after its action) is inert here.
        let d = e.add_minion(SeatId(0), crypt("\"Mother\" Anja Giovanni"), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
        let t = e.add_minion(SeatId(1), of_capacity(3, 0), 1, true);
        e.add_minion(SeatId(1), plain("Gangrel", "", 1), 5, false);
        event_in_play(&mut e, 1, "Fueled by Heart's Blood", counters);
        // The Edge: a vote to cast, so a blood hunt shows its polling.
        e.state.seats[1].has_edge = true;
        assert!(e.state.younger_than(t, d));
        pick(
            &mut e,
            Choice::TakeAction {
                minion: d,
                action: ActionKind::Diablerie,
                target: Some(t),
            },
        );
        let mut hunt = false;
        loop {
            let req = request(&mut e);
            hunt |= req.window == WindowLabel::BloodHuntPolling;
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            e.choose(fallback(&req));
        }
        assert_eq!(hunt, counters == 10, "{counters} counters");
    }
}

// The Fourth Cycle: "Requires 2 or more Gehenna events in play. After this
// card enters play, burn all Gehenna events and conviction cards, then send
// all vampires with capacity 3 or less to torpor. Gehenna events cannot be
// played."
#[test]
fn the_fourth_cycle_ends_the_gehenna_events_and_the_young() {
    let mut e = Engine::bare(3);
    let small = e.add_minion(SeatId(0), of_capacity(3, 0), 2, false);
    let big = e.add_minion(SeatId(0), of_capacity(5, 0), 2, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    e.state.seats[0].library = vec![find_library("Soak"); 5];
    let thirst = event_in_play(&mut e, 1, "Thirst", 0);
    let torpid = event_in_play(&mut e, 1, "Torpid Blood", 0);
    hand(&mut e, 0, &["Fourth Cycle, The", "Blood Weakens"]);
    until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, master(0));
    request(&mut e);
    assert!(!e.state.cards[thirst.index()].in_play && !e.state.cards[torpid.index()].in_play);
    assert!(e.state.minions[small.index()].in_torpor);
    assert!(!e.state.minions[big.index()].in_torpor);
    assert!(e.state.gehenna_barred(), "Gehenna events cannot be played");
}

// The Uncoiling: "During your discard phase, burn another event at random.
// Other Methuselahs can use a discard phase action and burn X pool to burn
// this card, where X is the number of other events in play." [LSJ
// 20080805] Torpid Blood burned: its replacement still waits. The burn is
// mandatory, ordered by its player among the phase's effects (owner,
// 2026-09-27: rules-qa 40.5).
#[test]
fn the_uncoiling_burns_an_event_each_discard_phase_and_can_be_bought_off() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    e.add_minion(SeatId(2), plain("Brujah", "", 0), 5, false);
    for s in 0..3 {
        e.state.seats[s].library = vec![find_library("Soak"); 5];
    }
    // A card in hand: seat 0's discard phase is not elided.
    hand(&mut e, 0, &["Soak"]);
    let u = event_in_play(&mut e, 0, "Uncoiling, The", 0);
    let torpid = event_in_play(&mut e, 1, "Torpid Blood", 0);
    e.state.seats[1].held.leaves_torpor = 1;
    let minds = event_in_play(&mut e, 2, "Narrow Minds", 0);
    // Due in seat 0's discard phase: ordered among its other effects, no
    // passing before it (rules-qa 40.5).
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(req.options.contains(&Choice::ResolveRider { card: u.0 }));
    assert!(!req.options.contains(&Choice::Pass));
    assert!(req.options.contains(&Choice::Discard(0)), "orderable");
    pick(&mut e, Choice::ResolveRider { card: u.0 });
    let burned = [torpid, minds]
        .iter()
        .filter(|c| !e.state.cards[c.index()].in_play)
        .count();
    assert_eq!(burned, 1, "one of them, at random");
    assert_eq!(e.state.seats[1].held.leaves_torpor, 1, "still waiting");
    // Seat 1's discard phase: 1 pool (one other event) and its action.
    let buy = Choice::UseAbility {
        source: AbilitySource::Card(u),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let req = until(&mut e, SeatId(1), WindowLabel::DiscardPhase);
    assert!(req.options.contains(&buy));
    let p1 = pool(&e, 1);
    pick(&mut e, buy);
    assert_eq!(pool(&e, 1), p1 - 1);
    assert!(!e.state.cards[u.index()].in_play);
}

// [LSJ 20100902] Wormwood: "When a vampire is influenced, the excess blood
// drains off before entering contest (if any) and before playing \"as
// played/as he enters play\" effects." A capacity 5 vampire with 5 counters
// comes out with 2 under a 2-counter Wormwood.
#[test]
fn wormwood_drains_an_influenced_vampire_to_its_ceiling() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    event_in_play(&mut e, 1, "Wormwood", 2);
    let card = of_capacity(5, 0);
    e.state.seats[0].uncontrolled.push(UncontrolledSlot {
        card,
        counters: 5,
        banished: None,
    });
    let slot = e.state.seats[0].uncontrolled.len() as u8 - 1;
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot });
    request(&mut e);
    let m = e
        .state
        .minions
        .iter()
        .position(|v| v.in_play && v.crypt_card() == Some(card))
        .map(|i| MinionId(i as u32))
        .expect("influenced");
    assert_eq!(blood(&e, m), 2);
}

// ---- Events, wave 2 (slice 11ad) ------------------------------------------------

// Blood Trade: "During each Methuselah's unlock phase, that Methuselah can
// move 1 blood from a vampire they control to a vampire controlled by
// another Methuselah." [LSJ 20040531-2] "Does not prevent Consanguineous
// Boon to be played."
#[test]
fn blood_trade_moves_a_blood_to_another_methuselahs_vampire() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 2, false);
    let z = e.add_minion(SeatId(2), plain("Brujah", "", 0), 2, false);
    hand(&mut e, 1, &["Consanguineous Boon"]);
    let trade = event_in_play(&mut e, 2, "Blood Trade", 0);
    let give = |m| Choice::UseAbility {
        source: AbilitySource::Card(trade),
        ability: 0,
        actor: None,
        target: Target::Minion(m),
    };
    let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&give(y)), "the active Methuselah's");
    assert!(!req.options.contains(&give(a)));
    pick(&mut e, give(y));
    let req = expect(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::ChooseMinion(a), Choice::ChooseMinion(z)]
    );
    pick(&mut e, Choice::ChooseMinion(z));
    assert_eq!((blood(&e, y), blood(&e, z)), (1, 3));
    // Consanguineous Boon still playable.
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&play(0, 0, y)));
}

// Conquest of Humanity: "During each Methuselah's unlock phase, that
// Methuselah can choose a location controlled by their prey. The chosen
// location is burned unless its controller burns 2 pool."
#[test]
fn conquest_of_humanity_burns_a_prey_location_unless_paid_for() {
    for pay in [false, true] {
        let mut e = Engine::bare(3);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
        e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
        e.add_minion(SeatId(2), plain("Brujah", "", 0), 4, false);
        let conquest = event_in_play(&mut e, 0, "Conquest of Humanity", 0);
        let place = location(&mut e, 2, "Parthenon, The");
        let own = location(&mut e, 0, "Academic Hunting Ground");
        let choose = |c: CardId| Choice::UseAbility {
            source: AbilitySource::Card(conquest),
            ability: 0,
            actor: None,
            target: Target::Card(c),
        };
        // Seat 1's prey is seat 2.
        let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
        assert!(req.options.contains(&choose(place)));
        assert!(!req.options.contains(&choose(own)), "not their prey's");
        pick(&mut e, choose(place));
        let p2 = pool(&e, 2);
        let req = expect(&mut e, SeatId(2), WindowLabel::ResolutionChoice);
        e.choose(
            req.options
                .iter()
                .position(|o| {
                    *o == if pay {
                        Choice::PayToCancel
                    } else {
                        Choice::Pass
                    }
                })
                .unwrap(),
        );
        assert_eq!(e.state.cards[place.index()].in_play, pay);
        assert_eq!(pool(&e, 2), if pay { p2 - 2 } else { p2 });
    }
}

// The New Inquisition: "During each Methuselah's unlock phase, that
// Methuselah can choose a ready vampire controlled by their prey. The
// chosen vampire takes 1 unpreventable environmental damage. Titled
// vampires can call a referendum to burn this card as a +1 stealth
// political action." "Do not replace until a titled vampire goes to
// torpor." [LSJ 20100527] "When a vampire goes to torpor, the card is
// replaced before any other effect." [LSJ 20080604] The granted action
// requires a titled vampire.
#[test]
fn the_new_inquisition_wounds_a_prey_vampire_and_is_burned_by_the_titled() {
    let mut e = Engine::bare(3);
    let p = e.add_minion(SeatId(0), prince(0), 4, false);
    let u = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 1, false);
    e.add_minion(SeatId(2), plain("Brujah", "", 0), 4, false);
    let inq = event_in_play(&mut e, 2, "New Inquisition, The", 0);
    e.state.seats[2].held.titled_torpor = 1;
    let burn = |m| Choice::UseAbility {
        source: AbilitySource::Card(inq),
        ability: 1,
        actor: Some(m),
        target: Target::None,
    };
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&burn(p)));
    assert!(!req.options.contains(&burn(u)), "untitled");
    // Seat 0's next unlock phase: 1 damage to the prey's ready vampire.
    let wound = Choice::UseAbility {
        source: AbilitySource::Card(inq),
        ability: 0,
        actor: None,
        target: Target::Minion(y),
    };
    pick_when(&mut e, wound);
    assert_eq!(blood(&e, y), 0, "1 damage, 1 blood burned");
    assert_eq!(e.state.seats[2].held.titled_torpor, 1);
    // Seat 2's unlock phase: the prince (no blood) is sent to torpor, a
    // titled vampire: the held replacement is drawn.
    e.state.seats[2].library = vec![find_library("Soak"); 3];
    until(&mut e, SeatId(2), WindowLabel::UnlockAbilities);
    e.state.minions[p.index()].blood = 0;
    let wound = Choice::UseAbility {
        source: AbilitySource::Card(inq),
        ability: 0,
        actor: None,
        target: Target::Minion(p),
    };
    pick(&mut e, wound);
    assert!(e.state.minions[p.index()].in_torpor);
    assert_eq!(e.state.seats[2].held.titled_torpor, 0, "replaced");
}

// Nightmares upon Nightmares: "During each Methuselah's unlock phase, for
// each minion they control, they lock that minion unless they discard a
// card; cards discarded this way are not replaced until the master phase.
// Mortal allies and vampires with capacity greater than the number of
// Gehenna events in play ignore this effect." [LSJ 20040415] Mortal allies
// and bigger vampires are not affected. [ANK 20250121] Once each unlock
// phase. [ANK 20210322-2] A locked minion may be "locked" (the Pass).
#[test]
fn nightmares_upon_nightmares_locks_small_minions_unless_their_player_discards() {
    let mut e = Engine::bare(3);
    let small = e.add_minion(SeatId(0), of_capacity(3, 0), 3, false);
    let other = e.add_minion(SeatId(0), of_capacity(3, 1), 3, false);
    let big = e.add_minion(SeatId(0), of_capacity(5, 0), 3, false);
    let mortal = e.add_ally(SeatId(0), find_library("ECTU Operative"));
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
    let nightmares = event_in_play(&mut e, 1, "Nightmares upon Nightmares", 0);
    for name in ["Thirst", "Torpid Blood"] {
        event_in_play(&mut e, 1, name, 0);
    }
    hand(&mut e, 0, &["Soak", "Soak"]);
    e.state.seats[0].library = vec![find_library("Conditioning"); 5];
    // `other` stays locked through its unlock phase.
    e.state.minions[other.index()].locked = true;
    e.state.minions[other.index()].skip_unlock = true;
    // Seat 0's next unlock phase: the rider (its only effect, elided), then
    // two capacity-3 vampires (3 Gehenna events in play): one choice each.
    let _ = nightmares;
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options[..2], [Choice::Pass, Choice::Discard(0)]);
    let before = e.state.seats[0].hand.len();
    pick(&mut e, Choice::Discard(0));
    // The second one, still locked: asked too, and "locked" again.
    pick(&mut e, Choice::Pass);
    let locked: Vec<bool> = [small, other, big, mortal]
        .iter()
        .map(|m| e.state.minions[m.index()].locked)
        .collect();
    assert_eq!(locked, vec![false, true, false, false]);
    assert_eq!(e.state.seats[0].hand.len(), before - 1, "not replaced yet");
    assert_eq!(e.state.seats[0].held.master, 1);
    // The master phase: replaced.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].held.master, 0);
}

// Recalled to the Founder: "During each Methuselah's unlock phase, if that
// Methuselah controls 3 or more vampires of the same clan, they burn one of
// those vampires. If that vampire's capacity is 6 or more, that Methuselah
// ignores this effect until the end of the game."
#[test]
fn recalled_to_the_founder_burns_one_of_three_and_spares_after_an_elder() {
    let brujah = |n| vampire(|c| c.clan == "Brujah" && c.title == Title::None, n);
    let mut e = Engine::bare(3);
    let bs: Vec<MinionId> = (0..3)
        .map(|n| e.add_minion(SeatId(0), brujah(n), 3, false))
        .collect();
    let v = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 3, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
    let recalled = event_in_play(&mut e, 1, "Recalled to the Founder", 0);
    // Seat 0's next unlock phase: the rider (elided), then the choice.
    let _ = recalled;
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    let want: Vec<Choice> = bs.iter().map(|&m| Choice::ChooseMinion(m)).collect();
    assert_eq!(
        req.options, want,
        "one of the three, no Pass, not the Ventrue"
    );
    let elder = *bs.iter().max_by_key(|&&m| e.state.capacity_of(m)).unwrap();
    let big = e.state.capacity_of(elder).unwrap() >= 6;
    pick(&mut e, Choice::ChooseMinion(elder));
    assert!(!e.state.minions[elder.index()].in_play);
    assert!(e.state.minions[v.index()].in_play);
    assert_eq!(e.state.seats[0].recalled_ignored, big);
}

// Restricted Vitae: "Vampires cannot hunt unless they must hunt. Vampires
// who must hunt can steal 1 blood from a younger vampire as a Ⓓ hunt
// action." "Do not replace until a vampire successfully hunts." [RBK
// action-card-or-card-in-play] "A minion can only perform this special
// hunt once per turn."
#[test]
fn restricted_vitae_leaves_only_the_starving_hunting() {
    let mut e = Engine::bare(3);
    // Anja's text (unlocking another Hecata after its action) is inert here.
    let fed = e.add_minion(SeatId(0), crypt("\"Mother\" Anja Giovanni"), 3, false);
    // Abraham's text (+1 hand size, Tremere cards) is inert here.
    let hungry = e.add_minion(SeatId(0), crypt("Abraham Mellon"), 0, false);
    let young = e.add_minion(SeatId(1), of_capacity(3, 0), 2, false);
    let rv = event_in_play(&mut e, 1, "Restricted Vitae", 0);
    e.state.seats[1].held.hunt = 1;
    e.state.seats[1].library = vec![find_library("Soak"); 3];
    let steal = Choice::UseAbility {
        source: AbilitySource::Card(rv),
        ability: 0,
        actor: Some(hungry),
        target: Target::Minion(young),
    };
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !req.options.contains(&action(fed, ActionKind::Hunt)),
        "not starving"
    );
    assert!(req.options.contains(&action(hungry, ActionKind::Hunt)));
    // The special hunt, a forced hunt too.
    pick(&mut e, steal);
    let a = e.action().expect("action");
    assert!(a.is_hunt);
    assert_eq!(a.directed_seat(&e.state), Some(SeatId(1)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, hungry), blood(&e, young)), (1, 1));
    assert_eq!(e.state.seats[1].held.hunt, 0, "a successful hunt: replaced");
    // Once a turn: unlocked, starving again, it cannot take it twice.
    let m = &mut e.state.minions[hungry.index()];
    m.locked = false;
    m.blood = 0;
    let req = request(&mut e);
    assert!(!req.options.contains(&steal));
}

// FBI Special Affairs Division: "If an ally is burned in combat with an
// acting vampire, add 1 counter to this card and inflict 2 unpreventable
// environmental damage on the acting vampire after the combat ends. If
// this card has 4 counters, burn it."
#[test]
fn fbi_special_affairs_division_wounds_the_vampire_that_burned_an_ally() {
    let mut e = Engine::bare(3);
    // Anja's text (unlocking another Hecata after its action) is inert here.
    let a = e.add_minion(SeatId(0), crypt("\"Mother\" Anja Giovanni"), 5, false);
    // Adelino's text (cheaper rescues) is inert here.
    e.add_minion(SeatId(0), crypt("Adelino Da Silva"), 5, false);
    let ally = e.add_ally(SeatId(1), find_library("Underbridge Stray"));
    let fbi = event_in_play(&mut e, 2, "FBI Special Affairs Division", 3);
    bleed_blocked(&mut e, a, ally);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play, "burned in combat");
    assert_eq!(blood(&e, a), 5 - 2, "2 after the combat");
    assert!(!e.state.cards[fbi.index()].in_play, "4 counters: burned");
}

// ---- Events, wave 3 (slice 11ae) -------------------------------------------------

// Fall of the Camarilla: "There is no Camarilla. Instead, Camarilla
// vampires are considered Independent." [LSJ 20080510] [LSJ 20100811] [ANK
// 20190619] "Is a temporary effect that overrides the underlying sect …
// If the Fall leaves play, the vampire retrieves his underlying sect."
// [LSJ 20060904] "Vampires with a Camarilla title lose the benefit from it
// until they become Camarilla again." [LSJ 20040519] A card making a
// vampire Camarilla still does: Independent while the Fall is in play.
#[test]
fn fall_of_the_camarilla_makes_camarilla_vampires_independent() {
    use zvtes::crypt::Sect;
    let mut e = Engine::bare(3);
    let p = e.add_minion(SeatId(0), prince(0), 4, false);
    let n = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    e.state.minions[n.index()].sect_override = Some(Sect::Camarilla);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
    assert_eq!(e.state.sect_of(p), Some(Sect::Camarilla));
    let votes = e.state.title_votes_of(p);
    assert!(votes > 0);
    let fall = event_in_play(&mut e, 1, "Fall of the Camarilla", 0);
    assert_eq!(e.state.sect_of(p), Some(Sect::Independent));
    assert_eq!(
        e.state.sect_of(n),
        Some(Sect::Independent),
        "made Camarilla"
    );
    assert_eq!(e.state.title_votes_of(p), 0, "no benefit from the title");
    e.burn_card(fall);
    assert_eq!(e.state.sect_of(p), Some(Sect::Camarilla));
    assert_eq!(e.state.title_votes_of(p), votes);
}

// Fall of the Sabbat: "Requires 3 or more other Gehenna events in play.
// There is no Sabbat. Instead, Sabbat vampires are considered Independent."
// [PIB 20121031] The requirement is read as it is played only.
#[test]
fn fall_of_the_sabbat_needs_three_gehenna_events_and_ends_the_sabbat() {
    use zvtes::crypt::Sect;
    let mut e = Engine::bare(3);
    let s = e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    e.state.minions[s.index()].sect_override = Some(Sect::Sabbat);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
    e.state.seats[0].library = vec![find_library("Soak"); 5];
    hand(&mut e, 0, &["Fall of the Sabbat"]);
    let t = event_in_play(&mut e, 1, "Thirst", 0);
    event_in_play(&mut e, 1, "Torpid Blood", 0);
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(!req.options.contains(&master(0)), "two Gehenna events");
    event_in_play(&mut e, 1, "Blood Weakens", 0);
    pick(&mut e, master(0));
    request(&mut e);
    assert_eq!(e.state.sect_of(s), Some(Sect::Independent));
    // One of the others burned: the Fall stays.
    e.burn_card(t);
    assert_eq!(e.state.sect_of(s), Some(Sect::Independent));
}

// Rise of the Nephtali: "Independent vampires can receive the unique
// Independent title of Liaison (worth 4 votes) as an action. If this title
// would be contested with a younger vampire, the younger vampire
// immediately yields instead of contesting." "Do not replace until a
// vampire becomes a liaison." [LSJ 20070125] "The title stays in play when
// the event is removed (eg The Uncoiling) but can't be claimed any more
// with the action."
#[test]
fn rise_of_the_nephtali_makes_an_independent_liaison_the_younger_yielding() {
    use zvtes::crypt::Sect;
    let mut e = Engine::bare(3);
    let young = e.add_minion(SeatId(0), of_capacity(3, 0), 3, false);
    // Anja's text (unlocking another Hecata after its action) is inert here.
    let old = e.add_minion(SeatId(0), crypt("\"Mother\" Anja Giovanni"), 3, false);
    let cam = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 3, false);
    for m in [young, old] {
        e.state.minions[m.index()].sect_override = Some(Sect::Independent);
    }
    e.state.minions[cam.index()].sect_override = Some(Sect::Camarilla);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
    let rise = event_in_play(&mut e, 1, "Rise of the Nephtali", 0);
    e.state.seats[1].held.liaison = 1;
    e.state.seats[1].library = vec![find_library("Soak"); 3];
    let claim = |m| Choice::UseAbility {
        source: AbilitySource::Card(rise),
        ability: 0,
        actor: Some(m),
        target: Target::None,
    };
    let liaison = Title::Independent {
        name: "Liaison",
        votes: 4,
    };
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&claim(cam)), "not Independent");
    pick(&mut e, claim(young));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(young), liaison);
    assert_eq!(e.state.title_votes_of(young), 4);
    assert_eq!(e.state.seats[1].held.liaison, 0, "replaced");
    // The older one claims it: the younger yields at once.
    pick(&mut e, claim(old));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(old), liaison);
    assert_eq!(e.state.title_of(young), Title::None);
    // The event gone: the title stays, no more claims.
    e.burn_card(rise);
    assert_eq!(e.state.title_of(old), liaison);
    let m = &mut e.state.minions[young.index()];
    m.locked = false;
    let req = request(&mut e);
    assert!(!req.options.contains(&claim(young)));
}

// Absimiliard's Army: "During each Methuselah's unlock phase, that
// Methuselah can put the top card of their library in play (face up); it
// becomes a nameless, textless ghoul ally with 2 life, 2 strength and 1
// bleed. After that ally is burned or this event leaves play, remove that
// ally from the game. After a Methuselah is ousted, burn this card." [LSJ
// 20040531] Not playing the card. [LSJ 20040623] [LSJ 20071003] Its name is
// ignored; they do not contest. [LSJ 20090920] A burned ghoul is removed,
// but burned; the event removed, they are removed without burning. [ANK
// 20200203-2] [LSJ 20060515] Ghouls are monsters, not mortal.
#[test]
fn absimiliards_army_raises_textless_ghouls_from_the_library() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", "", 0), 4, false);
    let army = event_in_play(&mut e, 1, "Absimiliard's Army", 0);
    // Seat 0's library top: a unique ally with text, and a copy in play.
    let stray = find_library("Michael Harris, the Incinerator");
    e.add_ally(SeatId(1), stray);
    let raise = Choice::UseAbility {
        source: AbilitySource::Card(army),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&raise));
    e.state.seats[0].library = vec![find_library("Soak"), stray, stray];
    let before = e.state.minions.len();
    pick(&mut e, raise);
    let g = MinionId(before as u32);
    let v = &e.state.minions[g.index()];
    assert!(v.in_play && v.is_ally() && !v.contested, "no contest");
    assert_eq!(v.name(), "", "nameless");
    assert_eq!(
        e.state.ally_body(g).map(|b| (b.life, b.strength, b.bleed)),
        Some((2, 2, 1))
    );
    assert!(
        v.ally_id().is_none() && v.kind.ally_card().is_none(),
        "textless"
    );
    assert_eq!(
        e.state.seats[0].library.len(),
        2,
        "not a play: no replacement"
    );
    // Burned: removed from the game.
    e.burn(g);
    assert!(e.state.seats[0].removed.contains(&stray));
    // A second one, removed with the event.
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&raise));
    e.state.seats[0].library = vec![stray];
    let before = e.state.minions.len();
    pick(&mut e, raise);
    let g2 = MinionId(before as u32);
    e.burn_card(army);
    assert!(!e.state.minions[g2.index()].in_play);
    assert_eq!(e.state.seats[0].removed.len(), 2);
}

// Absimiliard's Army: "Do not replace until a Methuselah is ousted. … After
// a Methuselah is ousted, burn this card."
#[test]
fn absimiliards_army_burns_as_a_methuselah_is_ousted() {
    let mut e = Engine::bare(4);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", "", 0), 4, false);
    e.add_minion(SeatId(3), plain("Brujah", "", 1), 4, false);
    let army = event_in_play(&mut e, 2, "Absimiliard's Army", 0);
    e.state.seats[2].held.ousted = 1;
    e.state.seats[2].library = vec![find_library("Soak"); 3];
    e.state.seats[1].pool = 1;
    pick(&mut e, action(a, ActionKind::Bleed));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert!(!e.state.cards[army.index()].in_play, "burned");
    assert_eq!(e.state.seats[2].held.ousted, 0, "replaced");
}

// Absimiliard's Army: "After … this event leaves play, remove that ally
// from the game" — its controller ousted, the Army leaves with them, and
// another Methuselah's ghoul goes too.
#[test]
fn absimiliards_army_takes_its_ghouls_with_its_ousted_controller() {
    let mut e = Engine::bare(4);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", "", 0), 4, false);
    e.add_minion(SeatId(3), plain("Brujah", "", 1), 4, false);
    let army = event_in_play(&mut e, 1, "Absimiliard's Army", 0);
    let soak = find_library("Soak");
    e.state.minions.push(zvtes::state::Minion::new(
        SeatId(2),
        zvtes::state::MinionKind::Token {
            card: soak,
            body: zvtes::library::Body {
                life: 2,
                strength: 2,
                bleed: 1,
            },
            source: army,
        },
        2,
    ));
    let g = MinionId(e.state.minions.len() as u32 - 1);
    e.state.seats[1].pool = 1;
    pick(&mut e, action(a, ActionKind::Bleed));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert!(!e.state.minions[g.index()].in_play, "removed with the Army");
    assert!(e.state.seats[2].removed.contains(&soak));
}

// Conquest of Humanity: "burned unless its controller burns 2 pool" — with
// 1 pool, it cannot be paid: burned.
#[test]
fn conquest_of_humanity_burns_a_location_its_controller_cannot_pay_for() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", "", 0), 4, false);
    let conquest = event_in_play(&mut e, 0, "Conquest of Humanity", 0);
    let place = location(&mut e, 2, "Parthenon, The");
    let choose = Choice::UseAbility {
        source: AbilitySource::Card(conquest),
        ability: 0,
        actor: None,
        target: Target::Card(place),
    };
    until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    e.state.seats[2].pool = 1;
    pick(&mut e, choose);
    // Only "let it burn": elided.
    request(&mut e);
    assert!(!e.state.cards[place.index()].in_play);
    assert_eq!(pool(&e, 2), 1);
}

// Rules-qa 40.4 (owner, 2026-09-27): under Blood Weakens, an action card
// played using a Discipline is not replaced until its action ends either
// ([LSJ 20080809], Gear Up).
#[test]
fn blood_weakens_holds_an_action_card_played_with_a_discipline() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "PRE", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    event_in_play(&mut e, 1, "Blood Weakens", 0);
    e.state.seats[0].library = vec![find_library("Soak"); 5];
    hand(&mut e, 0, &["Intimidation", "Soak"]);
    pick(&mut e, play(0, 1, a));
    request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), 1, "not replaced yet");
    assert_eq!(e.state.seats[0].held.weakens, 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].held.weakens, 0, "replaced as it ends");
}

// ---- Rulings re-triage: riders of a second block --------------------------

/// Seat 0's `a` (superior Protean, with any Obfuscate) bleeds with Form of
/// Mist and Lost in Crowds in hand; seat 1's `z`, the crypt card `blocker`.
fn second_block_table(blocker: u16) -> (Engine, MinionId, MinionId) {
    let mut e = Engine::bare(3);
    let a = e.add_minion(
        SeatId(0),
        vampire(
            |c| has(c, &["PRO"]) && has_level(c, &["obf"]) && c.title == Title::None,
            0,
        ),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), blocker, 5, false);
    hand(&mut e, 0, &["Form of Mist", "Lost in Crowds"]);
    (e, a, z)
}

/// In the block's combat: Form of Mist [PRO] ends it and the action
/// continues; `z` wakes (On the Qui Vive, first in its hand) and blocks
/// again.
fn form_of_mist_then_second_block(e: &mut Engine, a: MinionId, z: MinionId) {
    pick_when(e, play(0, 1, a)); // Form of Mist [PRO]: combat ends
    pick_when(e, Choice::ContinueAction);
    pick_when(e, play(0, 0, z)); // On the Qui Vive: z wakes
    pick_when(e, Choice::DeclareBlock { minion: z });
}

// [LSJ 20010814-2] Instinctive Reaction [ANI], Spirit's Touch [AUS]: "The
// maneuver is provided again if a second block happens on the same action
// (eg. after [PRO] Form of Mist)." Precognition [AUS]: "The prevention is
// provided again if a second block happens on the same action." Played on
// the first attempt; after Form of Mist the woken blocker blocks again
// (Enhanced Senses against its +1 stealth) and has the rider again.
#[test]
fn intercept_riders_come_again_with_a_second_block_after_form_of_mist() {
    // Blockers with the card's Discipline; their texts (unlock after
    // Keegan's own Gangrel action, after another Hecata's action for Anja)
    // are inert here.
    let anja = crypt("\"Mother\" Anja Giovanni");
    for (blocker, card) in [
        (crypt("Keegan"), "Instinctive Reaction"),
        (anja, "Spirit's Touch"),
        (anja, "Precognition"),
    ] {
        let (mut e, a, z) = second_block_table(blocker);
        hand(&mut e, 1, &[card, "On the Qui Vive", "Enhanced Senses"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick(&mut e, Choice::DeclareBlock { minion: z });
        pick(&mut e, play(1, 0, a)); // Lost in Crowds: +1 stealth
        pick_when(&mut e, play(0, 1, z)); // +1 intercept: blocked
        form_of_mist_then_second_block(&mut e, a, z);
        pick_when(&mut e, play(0, 0, z)); // Enhanced Senses: +1 intercept
        let rider = if card == "Precognition" {
            Choice::UsePrevent
        } else {
            Choice::UseManeuver
        };
        let again = offered_before(&mut e, rider, SeatId(0), WindowLabel::MinionPhase);
        assert!(again, "{card}: the second block's combat");
    }
}

// [LSJ 20010814-2] Form of the Bat [pro]: "The maneuver, and equipment
// restriction, apply again if a second block happens on the same action
// (eg. after [PRO] Form of Mist)." Blocked through the -1 intercept, the
// acting vampire continues with Form of Mist; blocked again, it has the
// maneuver again and its .44 Magnum is still unusable.
#[test]
fn form_of_the_bat_applies_again_to_a_second_block_after_form_of_mist() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRO"], 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), with(&["aus"], 0), 5, false);
    equip(&mut e, "Raven Spy", z); // +1 intercept
    let magnum = equip(&mut e, ".44 Magnum", a);
    hand(&mut e, 0, &["Form of Mist", "Form of the Bat"]);
    hand(&mut e, 1, &["On the Qui Vive", "Enhanced Senses"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(1, 0, a)); // Form of the Bat: -1 intercept
    form_of_mist_then_second_block(&mut e, a, z);
    pick_when(&mut e, play(0, 0, z)); // Enhanced Senses: +1 intercept
    let mut maneuver = false;
    loop {
        let req = request(&mut e);
        assert!(
            !req.options.iter().any(|o| matches!(
                o,
                Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == magnum
            )),
            "no equipment: {req:?}"
        );
        maneuver |= req.seat == SeatId(0) && req.options.contains(&Choice::UseManeuver);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    assert!(maneuver, "the second block's combat");
    assert_eq!(pool(&e, 1), 30, "blocked");
}

// [LSJ 20010814-2] Night Terrors [OBF][PRE]: "The strike: combat ends is
// provided again if a second block happens on the same action (eg. after
// [PRO] Form of Mist)."
#[test]
fn night_terrors_strike_comes_again_with_a_second_block_after_form_of_mist() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRO"], 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_both("OBF", "PRE", 0), 5, false);
    equip(&mut e, "Raven Spy", z); // +1 intercept
    hand(&mut e, 0, &["Form of Mist"]);
    hand(&mut e, 1, &["Night Terrors", "On the Qui Vive"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, z)); // stealth to 0, and the rider
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 1, a)); // Form of Mist [PRO]: combat ends
    let first = offering(&mut e, Choice::StrikeCombatEnds);
    assert_eq!(first.seat, SeatId(1), "the first block's combat");
    pick_when(&mut e, Choice::ContinueAction);
    pick_when(&mut e, play(0, 0, z)); // On the Qui Vive: z wakes
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    let again = offered_before(
        &mut e,
        Choice::StrikeCombatEnds,
        SeatId(0),
        WindowLabel::MinionPhase,
    );
    assert!(again, "the second block's combat");
}

// [LSJ 20010814-2] One With the Land [PRO]: "The strike restriction applies
// again if a second block happens on the same action (e.g. after [PRO] Form
// of Mist)." No strike in the first round; Wolf Claws [PRO] presses into a
// second, where Form of Mist ends the combat and continues the action;
// blocked again, the first round of that combat has no strike either.
#[test]
fn one_with_the_land_bars_strikes_again_after_a_second_block() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRO"], 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Tzimisce", "PRO", 0), 5, false);
    e.state.minions[z.index()].locked = true;
    equip(&mut e, "Raven Spy", z); // +1 intercept
    hand(&mut e, 0, &["Form of Mist", "Wolf Claws"]);
    hand(&mut e, 1, &["One With the Land", "On the Qui Vive"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, z)); // unlocks and blocks
    pick_when(&mut e, play(1, 1, a)); // Wolf Claws [PRO]: press
    pick_when(&mut e, play(0, 1, a)); // Form of Mist [PRO], round 2
    pick_when(&mut e, Choice::ContinueAction);
    pick_when(&mut e, play(0, 0, z)); // On the Qui Vive: z wakes
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    let before = (blood(&e, a), blood(&e, z));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::HandStrike)
    });
    assert_eq!((blood(&e, a), blood(&e, z)), before, "no strike");
    assert_eq!(pool(&e, 1), 30, "blocked");
}

// [LSJ 20010714-2] Sense the Savage Way, Eyes of the Wild, Organized
// Resistance, One With the Land: a card forcing a block attempt "cannot be
// used if the vampire cannot attempt to block the action (eg. Daring the
// Dawn)." None is offered once Daring the Dawn [FOR] bars vampires from
// blocking.
#[test]
fn forced_block_attempts_are_not_offered_under_daring_the_dawn() {
    for case in 0..4 {
        let mut e = Engine::bare(3);
        // Anja's text (unlocking another Hecata after its action) is inert here.
        let a = e.add_minion(SeatId(0), crypt("\"Mother\" Anja Giovanni"), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        hand(&mut e, 0, &["Daring the Dawn"]);
        let (card, z) = match case {
            0 => {
                // Its text (unlock after its own Gangrel action) is inert
                // here.
                let elder = crypt("Keegan");
                (
                    "Sense the Savage Way",
                    e.add_minion(SeatId(1), elder, 4, false),
                )
            }
            1 => (
                "Eyes of the Wild",
                e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false),
            ),
            2 => {
                e.add_minion(SeatId(1), baron(0), 4, false);
                (
                    "Organized Resistance",
                    e.add_minion(SeatId(1), anarch(0), 4, false),
                )
            }
            _ => (
                "One With the Land",
                e.add_minion(SeatId(1), plain("Tzimisce", "PRO", 0), 4, false),
            ),
        };
        e.state.minions[z.index()].locked = true;
        hand(&mut e, 1, &[card]);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick_when(&mut e, play(0, 1, a)); // Daring the Dawn [FOR]
        never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
            matches!(o, Choice::Play { .. })
                && !matches!(o, Choice::Play { actor: Some(m), .. } if *m == a)
        });
        assert!(e.state.minions[z.index()].locked, "{card}: no attempt");
    }
}

// [LSJ 20021028] .44 Magnum, AK-47, Assault Rifle: "The optional maneuver
// cannot be used if the strike cannot be used (eg. Hidden Lurker)." No
// strike in the first round (One With the Land [PRO]): neither the gun's
// strike nor its maneuver is offered.
#[test]
fn a_guns_maneuver_is_not_used_while_its_strike_is_barred() {
    for gun in [".44 Magnum", "AK-47", "Assault Rifle"] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Tzimisce", "PRO", 0), 5, false);
        e.state.minions[z.index()].locked = true;
        let weapon = equip(&mut e, gun, a);
        hand(&mut e, 1, &["One With the Land"]);
        pick(&mut e, action(a, ActionKind::Bleed));
        pick_when(&mut e, play(0, 1, z));
        never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
            matches!(o, Choice::UseManeuver)
                || matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == weapon)
        });
        assert_eq!(pool(&e, 1), 30, "{gun}: blocked");
    }
}

// [ANK 20220331] [ANK 20210124] [LSJ 20020927] [LSJ 20021115] [ANK
// 20221011-2] Daring the Dawn: "The damage is applied if the action reaches
// resolution, even if it ends before combat (eg. Mirror Walk, Change of
// Target, Red Herring)." Mirror Walk [THA] locks the blocking ally and ends
// the action before block resolution; the acting vampire still takes its 2
// aggravated damage.
#[test]
fn daring_the_dawn_burns_after_mirror_walk_ends_the_action() {
    let mut e = Engine::bare(3);
    // Tommaso's text (ending a wraith's or zombie's combat) is inert here.
    let a = e.add_minion(SeatId(0), crypt("Tommaso Sforza"), 1, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let ally = e.add_ally(SeatId(1), find_library("47th Street Royals"));
    equip(&mut e, "Raven Spy", ally); // +1 intercept
    hand(&mut e, 0, &["Daring the Dawn", "Mirror Walk"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 0, a)); // Daring the Dawn [for]
    pick_when(&mut e, Choice::DeclareBlock { minion: ally });
    pick_when(&mut e, play(0, 1, a)); // Mirror Walk [THA]: +1 stealth
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.combatants.is_none());
    assert!(e.state.minions[ally.index()].locked, "the blocker locked");
    assert_eq!(pool(&e, 1), 30, "no bleed");
    assert!(e.state.minions[a.index()].in_torpor, "2 aggravated damage");
}

// [RTR 20180719] [ANK 20200207] [ANK 20210627] Mirror Walk [THA]: "If the
// action is blocked, it ends unsuccessfully immediately, no other action
// modifier or reaction can be played afterwards (eg. Freak Drive, slave
// rule, Sowing Dissension)." Freak Drive [FOR], usable after a blocked
// action, is not offered (Abraham DuSable: FOR, THA).
#[test]
fn freak_drive_is_not_played_after_mirror_walk_ends_the_action() {
    let mut e = Engine::bare(3);
    let abraham = crypt("Abraham DuSable");
    let a = e.add_minion(SeatId(0), abraham, 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    equip(&mut e, "Raven Spy", z); // +1 intercept
    hand(&mut e, 0, &["Mirror Walk", "Freak Drive"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 1, a)); // Mirror Walk [THA]
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a),
    );
    assert!(e.state.minions[z.index()].locked, "the blocker locked");
    assert!(e.state.minions[a.index()].locked, "no unlock");
}

// [RTR 20180719] Freak Drive [FOR]: "Cannot be played if the action
// 'failed' unless there was a successful block attempt." [LSJ 20070219]
// "If an action fails, it was not blocked. Cards/effects depending on a
// block (e.g. Mirror Image, [FOR] Freak Drive) cannot be played/used." A
// Szlachta Bodyguard fails a Frontal Assault rush; the acting vampire stays
// locked, and neither Freak Drive is offered.
#[test]
fn freak_drive_is_not_played_after_a_failed_action() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["FOR"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    let guard = equip(&mut e, "Szlachta Bodyguard", ally);
    let assault = location(&mut e, 0, "Frontal Assault");
    hand(&mut e, 0, &["Freak Drive"]);
    pick(&mut e, use_on(assault, 0, Some(a), Target::Minion(ally)));
    pick_when(&mut e, use_on(guard, 1, None, Target::None));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a),
    );
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
    assert!(e.state.minions[a.index()].locked);
}

// [ANK 20220218] Freak Drive: "The after action resolution effect can be
// used if the action 'fizzles' (is unblocked at resolution, but has no
// effect because some requirement cannot be met or cost can't be paid in
// full)." A Raptor employed for 2 blood by a vampire down to 1 blood at
// resolution: the action fizzles, and Freak Drive [for] unlocks — the
// vampire, now with no blood, must then hunt (locking it again).
#[test]
fn freak_drive_unlocks_after_a_fizzled_action() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["for", "ANI"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    // The prey may attempt to block (a Raven Spy's intercept): a window.
    let z = e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    equip(&mut e, "Raven Spy", z);
    hand(&mut e, 0, &["Raptor", "Freak Drive"]);
    pick(&mut e, play(0, 0, a));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.state.minions[a.index()].blood = 1;
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert_eq!(
        e.action().and_then(|x| x.outcome),
        Some(zvtes::actions::Outcome::Fizzled)
    );
    assert!(req.options.contains(&play(0, 0, a)), "Freak Drive [for]");
    pick(&mut e, play(0, 0, a));
    // Unlocked with no blood left, it must hunt (a forced hunt, elided).
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].locked, "locked by the hunt");
    assert_eq!(blood(&e, a), 1, "Freak Drive's blood; no Raptor paid; fed");
    assert!(!e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == find_library("Raptor")));
}

// ---- Rulings re-triage: Change of Target, cancels ---------------------------

// [LSJ 20080725] Change of Target: "Actions provided by different cards in
// play are not the same, even if the cards have the same name." The rush
// granted by one of two Frontal Assaults, ended by Change of Target: the
// other copy's rush is still offered, not the ended copy's.
#[test]
fn change_of_target_bars_only_the_copy_whose_action_it_ended() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    let y = e.add_minion(SeatId(1), plain("", "", 2), 5, false);
    let first = location(&mut e, 0, "Frontal Assault");
    let second = location(&mut e, 0, "Frontal Assault");
    hand(&mut e, 0, &["Change of Target"]);
    let rush = |c| use_on(c, 0, Some(a), Target::Minion(z));
    pick(&mut e, rush(first));
    pick_when(&mut e, Choice::DeclareBlock { minion: y });
    pick_when(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    assert!(!req.options.contains(&rush(first)), "the same action");
    assert!(req.options.contains(&rush(second)), "another card's");
}

// [LSJ 19980112] Change of Target: "If the card prevents the acting minion
// from performing a mandatory action, he is stuck and cannot act." A
// vampire with no blood must hunt; its blocked hunt ended by Change of
// Target, it cannot hunt again this turn: stuck, it takes no action, and
// the other vampire still may.
#[test]
fn change_of_target_leaves_a_vampire_that_must_hunt_stuck() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 0, false);
    let b = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    equip(&mut e, "Raven Spy", z); // +1 intercept: may block the hunt
    hand(&mut e, 0, &["Change of Target"]);
    // The hunt is its only option: declared at once (elided).
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    assert!(
        !req.options.iter().any(|o| matches!(
            o,
            Choice::TakeAction { minion, .. } if *minion == a
        )),
        "stuck: {:?}",
        req.options
    );
    assert!(req.options.contains(&action(b, ActionKind::Bleed)));
}

// [LSJ 20010702] [RTR 19941006] [LSJ 19991025] [LSJ 20100527] "If a minion
// who is attempting to block is locked, then that block will fail, unless
// they woke earlier during the action." Attempting to block, the vampire
// locks itself for Visions of Zapathasura [OBF][PRE]: the attempt fails,
// no combat, the bleed reduced to 0.
#[test]
fn locking_for_visions_of_zapathasura_fails_the_own_block_attempt() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "", 0), 5, false);
    let start = capped(&e, a);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), with_both("OBF", "PRE", 0), 5, false);
    let zs = capped(&e, z);
    hand(&mut e, 1, &["Visions of Zapathasura"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 1, z)); // [OBF][PRE]: lock to reduce to 0
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_ne!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert!(e.state.minions[z.index()].locked);
    assert_eq!((blood(&e, a), blood(&e, z)), (start, zs - 1), "no combat");
    assert_eq!(pool(&e, 1), 30, "reduced to 0");
}

// [ANK 20200525-2] Immortal Grapple: "Is still considered played if it is
// canceled as it is played (eg. Charismatic Aura). A second one cannot be
// played in the same round as its card text forbids it." Direct
// Intervention cancels the first: the second is not offered that round,
// though the window is polled again (Z's grapple).
#[test]
fn immortal_grapple_canceled_as_played_still_counts_for_the_round() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["pot"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    // Z holds a grapple too: the window keeps being polled after the
    // cancel.
    let z = e.add_minion(SeatId(1), with(&["pot"], 1), 5, false);
    hand(&mut e, 0, &["Immortal Grapple", "Immortal Grapple"]);
    hand(&mut e, 1, &["Direct Intervention", "Immortal Grapple"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a)); // Immortal Grapple [pot]
    pick_when(&mut e, master(0)); // Direct Intervention: canceled
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::CombatBeforeStrikes)
    );
    // Z's Immortal Grapple: the impulse back to A, whose second one is
    // not offered (its window elided).
    pick(&mut e, play(0, 0, z));
    never_offered(&mut e, &play(0, 0, a));
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Immortal Grapple")));
    assert_eq!(e.state.seats[0].hand.len(), 1, "the second one kept");
}

// ---- Rulings re-triage: events -------------------------------------------------

// [LSJ 20090920] Absimiliard's Army: "A burned ghoul is removed from the
// game, but it is still burned (e.g. it triggers Tension in the Ranks).
// However, when Absimillard's Army is removed from the game, the ghouls are
// removed without burning (will not trigger Tension in the Ranks)."
#[test]
fn tension_in_the_ranks_counts_a_burned_ghoul_of_absimiliards_army_only() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", "", 0), 4, false);
    let army = event_in_play(&mut e, 1, "Absimiliard's Army", 0);
    location(&mut e, 2, "Tension in the Ranks");
    let raise = Choice::UseAbility {
        source: AbilitySource::Card(army),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    e.state.seats[0].library = vec![find_library("Soak"); 3];
    let before = e.state.minions.len();
    pick(&mut e, raise);
    let g = MinionId(before as u32);
    let p0 = pool(&e, 0);
    e.burn(g);
    assert_eq!(pool(&e, 0), p0 - 1, "burned: Tension in the Ranks");
    assert_eq!(e.state.seats[0].removed.len(), 1, "and removed");
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    e.state.seats[0].library = vec![find_library("Soak"); 3];
    let before = e.state.minions.len();
    pick(&mut e, raise);
    let g2 = MinionId(before as u32);
    let p0 = pool(&e, 0);
    e.burn_card(army);
    assert!(!e.state.minions[g2.index()].in_play);
    assert_eq!(pool(&e, 0), p0, "removed without burning");
}

// [LSJ 20080106] Anarch Revolt: "If a Methuselah controls no ready Anarch by
// the end of the unlock phase, they must burn 1 pool, even if they had a
// ready Anarch at some point during the unlock phase." Recalled to the
// Founder, ordered first, burns the Methuselah's only Anarch (one of three
// Toreador): Anarch Revolt then burns a pool.
#[test]
fn anarch_revolt_burns_after_the_only_anarch_left_during_the_unlock_phase() {
    let mut e = Engine::bare(3);
    let aisha = crypt("Aisha az-Zahra"); // Anarch Toreador
    let anarch = e.add_minion(SeatId(0), aisha, 3, false);
    for n in 0..2 {
        e.add_minion(SeatId(0), camarilla_toreador(n), 3, false);
    }
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
    let recalled = event_in_play(&mut e, 1, "Recalled to the Founder", 0);
    let revolt = location(&mut e, 1, "Anarch Revolt");
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req
        .options
        .contains(&Choice::ResolveRider { card: revolt.0 }));
    pick(&mut e, Choice::ResolveRider { card: recalled.0 });
    let p0 = pool(&e, 0);
    pick(&mut e, Choice::ChooseMinion(anarch));
    assert!(!e.state.minions[anarch.index()].in_play);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 0), p0 - 1, "no ready Anarch left");
}

// [ANK 20200203-2] [LSJ 20060515] Underbridge Stray (an animal), War Ghoul
// and The Vozhd of Sofia (ghouls): "Is neither a mortal nor a monster";
// "Ghouls are monsters, not mortal." Nightmares upon Nightmares, which
// mortal allies ignore, asks for each of them; the mortal Political Ally
// is spared.
#[test]
fn nightmares_upon_nightmares_reaches_animal_and_ghoul_allies() {
    for name in [
        "Underbridge Stray",
        "War Ghoul",
        "Vozhd of Sofia, The",
        "Political Ally",
    ] {
        let mut e = Engine::bare(3);
        // Hel-Blá's text (a printed action) is inert here.
        let v = e.add_minion(SeatId(0), crypt("Hel-Blá"), 3, false);
        // A retainer for a ghoul's entry to burn.
        let spy = equip(&mut e, "Raven Spy", v);
        let ally = e.add_ally(SeatId(0), find_library(name));
        e.add_minion(SeatId(1), plain("Gangrel", "", 0), 4, false);
        event_in_play(&mut e, 1, "Nightmares upon Nightmares", 0);
        hand(&mut e, 0, &["Soak"]);
        let mortal = name == "Political Ally";
        let req = request(&mut e);
        if req.window == WindowLabel::EntryEffect {
            pick(&mut e, Choice::BurnInPlay { card: spy.0 });
        }
        pick(&mut e, Choice::Pass); // this turn's minion phase
        let mut asked = false;
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            if (req.seat, req.window) == (SeatId(0), WindowLabel::ResolutionChoice) {
                asked = true;
                pick(&mut e, Choice::Pass); // "locked"
                continue;
            }
            e.choose(fallback(&req));
        }
        assert_eq!(asked, !mortal, "{name}");
        assert_eq!(e.state.minions[ally.index()].locked, !mortal, "{name}");
    }
}

// ---- Rulings re-triage: cost reductions --------------------------------------

/// `line`'s reduction (its ability 0, or announced with a card).
fn uses_line(o: &Choice, line: CardId) -> bool {
    matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 0, .. } if *c == line)
        || matches!(o, Choice::ResolveRider { card } if *card == line.0)
}

// [RTR 20070707] [ANK 20230824] Pack Alpha: "Cost reductions apply." [ANK
// 20230824] [RTR 20070707] "Cost reductions that are not referring to an
// action apply." The Line: "Can reduce the cost of an action card even if
// it is not played as an action (e.g. a weapon played with Concealed Weapon
// or an Ally with Piper)." The Line takes the blood off the Raven Spy Pack
// Alpha employs, and off the Underbridge Stray Piper recruits.
#[test]
fn the_line_reduces_a_card_pack_alpha_or_piper_brings_in() {
    // Pack Alpha's Raven Spy: 1 blood, 0 with The Line.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["ani"], 0), 3, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    let line = location(&mut e, 0, "Line, The");
    hand(&mut e, 0, &["Pack Alpha", "Raven Spy"]);
    bleed_blocked(&mut e, a, b);
    until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ResolveRider { card: line.0 });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, a), vec![(find_library("Raven Spy"), 1)]);
    assert!(e.state.cards[line.index()].locked);
    assert_eq!(blood(&e, a), 3 - 1, "the hand strike only");
    // Piper's Underbridge Stray: 1 blood, 0 with The Line.
    let mut e = Engine::bare(3);
    let an = e.add_minion(SeatId(0), anarch_with(&["ani"], &[], 0), 3, false);
    e.add_minion(SeatId(1), plain("", "", 1), 4, false);
    let line = location(&mut e, 0, "Line, The");
    hand(&mut e, 0, &["Piper", "Underbridge Stray"]);
    e.at_master_phase();
    pick(&mut e, play(0, 0, an));
    pick_when(&mut e, Choice::ResolveRider { card: line.0 });
    request(&mut e);
    assert!(e.state.cards[line.index()].locked);
    assert_eq!(blood(&e, an), 3, "no blood paid");
    let stray = MinionId(e.state.minions.len() as u32 - 1);
    assert!(e.state.minions[stray.index()].is_ally());
}

// [ANK 20200605] The Line: "Cannot be used by an ally acting 'as a
// vampire'." A vampire employing a Raven Spy may use it; The Vozhd of Juiz
// de Fora, employing one as a vampire, may not, and pays the blood in life.
#[test]
fn the_line_is_not_for_an_ally_playing_as_a_vampire() {
    for ally in [false, true] {
        let mut e = Engine::bare(3);
        let t = e.add_minion(SeatId(0), clan_with("Tzimisce", &["ani"], 0), 4, false);
        equip(&mut e, "Revenant", t);
        let actor = if ally {
            let v = e.add_ally(SeatId(0), find_library("Vozhd of Juiz de Fora, The"));
            request(&mut e); // its entry burns the Revenant
            v
        } else {
            t
        };
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 4, false);
        e.add_minion(SeatId(1), plain("", "", 1), 4, false);
        let line = location(&mut e, 0, "Line, The");
        hand(&mut e, 0, &["Raven Spy"]);
        let before = blood(&e, actor);
        pick(&mut e, play(0, 0, actor));
        let mut used = false;
        loop {
            let req = request(&mut e);
            if let Some(i) = req.options.iter().position(|o| uses_line(o, line)) {
                used = true;
                e.choose(i);
                continue;
            }
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            e.choose(fallback(&req));
        }
        assert_eq!(used, !ally, "ally: {ally}");
        assert_eq!(e.state.cards[line.index()].locked, !ally);
        assert_eq!(
            blood(&e, actor),
            before - if ally { 1 } else { 0 },
            "ally: {ally}"
        );
    }
}

// [ANK 20210226] Form of Mist: "The burn blood effect is not reduced by
// effects that reduce the cost of the card (eg. Adana de Sforza)." With the
// Edge, The Shard is not offered for Form of Mist [PRO] (no cost to
// reduce), and the blood to continue is burned.
#[test]
fn form_of_mists_blood_to_continue_is_not_a_cost_the_shard_reduces() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRO"], 0), 5, false);
    let start = capped(&e, a);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    let shard = location(&mut e, 0, "Shard, London, The");
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Form of Mist"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a)); // Form of Mist [PRO]
    let shard_use = |o: &Choice| {
        matches!(o, Choice::ResolveRider { card } if *card == shard.0)
            || matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 0, .. } if *c == shard)
    };
    loop {
        let req = request(&mut e);
        assert!(!req.options.iter().any(shard_use), "{req:?}");
        if req.options.contains(&Choice::ContinueAction) {
            pick(&mut e, Choice::ContinueAction);
            break;
        }
        e.choose(fallback(&req));
    }
    request(&mut e);
    assert_eq!(blood(&e, a), start - 1, "the blood to continue");
    assert!(!e.state.cards[shard.index()].locked);
}

// [LSJ 20090529] "If an opponent then plays an effect that grants a strike
// (e.g, Aid from Bats), they pay the additional cost." Terror Frenzy [ANI]:
// "combat cards cost the opposing vampire +1 blood" — Aid from Bats,
// played after it, costs its player 1 blood more than without it.
#[test]
fn aid_from_bats_pays_terror_frenzys_surcharge() {
    let mut spent = Vec::new();
    for frenzy in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["ANI"], 0), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let z = e.add_minion(SeatId(1), with(&["ani"], 0), 5, false);
        let zs = capped(&e, z);
        hand(&mut e, 0, &["Terror Frenzy"]);
        hand(&mut e, 1, &["Aid from Bats"]);
        bleed_blocked(&mut e, a, z);
        if frenzy {
            pick_when(&mut e, play(0, 1, a)); // Terror Frenzy [ANI]
        }
        pick_when(&mut e, play(0, 0, z)); // Aid from Bats [ani]
        request(&mut e);
        spent.push(zs - blood(&e, z));
    }
    assert_eq!(spent, vec![0, 1]);
}

// ---- Rulings re-triage: Sniper Rifle ------------------------------------------

// [PIB 20120214] Sniper Rifle: "If another effect was already setting that
// round's range (e.g. Neutral Guard), that effect has priority." [RTR
// 20020501] "The bearer is not commited to use it if he or she uses another
// effect to set the range." Michael Harris, bearing a Sniper Rifle and a .44
// Magnum, blocks and sets the range to long himself: the Rifle's setting
// is no longer offered, and his strike may be the Magnum's.
#[test]
fn sniper_rifle_yields_to_its_bearers_other_range_setting() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let (mh, card) = antigen(&mut e, 1, "Michael Harris, the Incinerator");
    let rifle = equip(&mut e, "Sniper Rifle", mh);
    let magnum = equip(&mut e, ".44 Magnum", mh);
    bleed_blocked(&mut e, a, mh);
    pick_when(&mut e, card_ability(card, 2)); // long, for a pool
    let req = loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&card_ability(rifle, 1)), "{req:?}");
        if (req.seat, req.window) == (SeatId(1), WindowLabel::CombatStrike) {
            break req;
        }
        e.choose(fallback(&req));
    };
    assert!(
        req.options.contains(&weapon_strike(magnum)),
        "not committed"
    );
    assert!(req.options.contains(&weapon_strike(rifle)));
}

// [LSJ 20021028] Sniper Rifle: "The ability to set the range can only be
// used if the weapon was equipped when blocking (eg. not when using
// Disguised Weapon after the block)." A blocking Ravnos equips a Sniper
// Rifle with Contraband before range is determined: it does not set the
// range.
#[test]
fn a_sniper_rifle_equipped_after_the_block_does_not_set_the_range() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), ravnos(0), 5, false);
    hand(&mut e, 1, &["Contraband", "Sniper Rifle"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, z)); // Contraband [obf]
    request(&mut e);
    let rifle = find_library("Sniper Rifle");
    let id = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == rifle && c.host == Some(z))
            .expect("the rifle equipped") as u32,
    );
    never_offered(&mut e, &card_ability(id, 1));
}

// ---- Rulings re-triage: combat ------------------------------------------------

// [LSJ 20031201-2] Taste of Vitae: "Can be played if the opponent was
// burned." Torn Signpost [POT] and Wolf Claws [pro]: 3 aggravated damage
// burn a ready vampire with no blood (wounded by the first, destroyed by
// the rest); Taste of Vitae is still offered at the end of the round.
#[test]
fn taste_of_vitae_is_played_when_the_opponent_was_burned() {
    let mut e = Engine::bare(3);
    // Gnaeus's text (unlock-phase damage to his prey) is not reached here.
    let a = e.add_minion(SeatId(0), crypt("Gnaeus Aemilius Augustinus"), 6, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 0, false);
    hand(
        &mut e,
        0,
        &["Torn Signpost", "Wolf Claws", "Taste of Vitae"],
    );
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 1, a)); // Torn Signpost [POT]: strength 3
    pick_when(&mut e, play(0, 0, a)); // Wolf Claws [pro]: aggravated hands
    let taste = offered_before(&mut e, play(0, 0, a), SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_play, "burned");
    assert!(taste, "Taste of Vitae offered");
}

// [PIB 20130303] Torn Signpost: "Sets base strength." Heroic Might's +1
// strength still adds to it: 2 + 1.
#[test]
fn torn_signpost_sets_the_base_strength_modifiers_still_apply() {
    let mut dealt = Vec::new();
    for might in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(&["pot"], 0), 5, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let z = e.add_minion(SeatId(1), plain("", "", 1), 5, false);
        let zs = capped(&e, z);
        if might {
            equip(&mut e, "Heroic Might", a);
        }
        hand(&mut e, 0, &["Torn Signpost"]);
        bleed_blocked(&mut e, a, z);
        pick_when(&mut e, play(0, 0, a)); // Torn Signpost [pot]: strength 2
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        dealt.push(zs - blood(&e, z));
    }
    assert_eq!(dealt, vec![2, 3]);
}

// ---- Rulings re-triage: actions -----------------------------------------------

// [LSJ 20070526] [ANK 20210124] Veil the Legions [OBF]: "If an action is
// cancelled after it has been played (e.g. Tangle Atropos' Hand but not
// Direct Intervention) it still counts as 'one of the next X actions'." An
// action card canceled as played by Direct Intervention is no action: the
// next one still gets the +1 stealth.
#[test]
fn veil_the_legions_skips_an_action_card_canceled_as_played() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let o = e.add_minion(SeatId(0), with(&["OBF"], 0), 5, false);
    let c = e.add_minion(SeatId(0), plain("", "PRE", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    hand(&mut e, 0, &["Veil the Legions", "Intimidation"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 1, o)); // [OBF]: X = 1
    pick(&mut e, Choice::SetX(1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[z.index()].locked = false;
    pick(&mut e, play(0, 1, c)); // Intimidation
    pick_when(&mut e, master(0)); // Direct Intervention: canceled
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[c.index()].locked, "no action");
    pick(&mut e, action(c, ActionKind::Bleed));
    never_offered(&mut e, &Choice::DeclareBlock { minion: z });
    assert_eq!(pool(&e, 1), 30 - 1 - 1 - 1, "two bleeds, and its pool");
}

// [RTR 19951017] Vast Wealth: "Vast Wealth's ability is not usable if the
// vampire does not share the same controller as Vast Wealth." Far Mastery
// [DOM] steals the ally Vast Wealth is on: its new controller is offered
// no equip action with it.
#[test]
fn vast_wealth_is_not_usable_once_its_minion_is_stolen() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(1), with(&["DOM"], 0), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 5, false);
    let ally = e.add_ally(SeatId(0), find_library("Political Ally"));
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 5, false);
    let wealth = master_on(&mut e, 0, "Vast Wealth", ally);
    let equip_found = use_on(wealth, 0, Some(ally), Target::None);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&equip_found), "its controller's");
    hand(&mut e, 1, &["Far Mastery"]);
    to_minion_phase(&mut e, 1);
    pick(&mut e, play_at(0, 1, Some(a), Target::Minion(ally)));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[ally.index()].owner, SeatId(1), "stolen");
    assert_eq!(e.state.card_controller(wealth), SeatId(0));
    e.state.minions[ally.index()].locked = false;
    let _ = req;
    let req = request(&mut e);
    assert!(
        req.options.contains(&action(ally, ActionKind::Bleed)),
        "it may act"
    );
    assert!(
        !req.options.iter().any(|o| matches!(
            o,
            Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == wealth
        )),
        "{:?}",
        req.options
    );
}

// [PIB 20150703] The Sleeping Mind [DOM]: "Does not prevent wake effects to
// be played (eg. Wake with Evening's Freshness), but does prevent unlock
// after action resolution (eg. Freak Drive or [ani] Cats' Guidance)." A
// locked vampire wakes and blocks; after the combat, Cats' Guidance [ani]
// does not unlock it.
#[test]
fn the_sleeping_mind_lets_a_vampire_wake_but_not_unlock() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["DOM"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), with(&["ani"], 0), 5, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Sleeping Mind, The"]);
    hand(
        &mut e,
        1,
        &["Wake with Evening's Freshness", "Cats' Guidance"],
    );
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 1, a)); // [DOM]: minions cannot unlock
    pick_when(&mut e, play(0, 0, z)); // Wake with Evening's Freshness
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    // Cats' Guidance, if offered at all, is played: no unlock either way.
    if offered_before(&mut e, play(0, 0, z), SeatId(0), WindowLabel::MinionPhase) {
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    }
    assert_eq!(pool(&e, 1), 30, "blocked");
    assert!(e.state.minions[z.index()].locked, "no unlock");
    // Without The Sleeping Mind, Cats' Guidance unlocks it.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["DOM"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), with(&["ani"], 0), 5, false);
    hand(&mut e, 1, &["Cats' Guidance"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].locked);
}

// [LSJ 20101021] "The card is replaced after all 'after resolution' cards
// and effects (e.g. Freak Drive). No more cards can be played on that
// action; the action has ended." Elder Intervention ("Do not replace until
// after this action") is still unreplaced while Freak Drive [FOR] is
// played after the blocked action's resolution, and replaced once the
// action has ended.
#[test]
fn a_card_held_until_after_the_action_is_replaced_after_freak_drive() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(
        SeatId(0),
        vampire(|c| has(c, &["FOR"]) && has_level(c, &["obf"]), 0),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    hand(&mut e, 0, &["Freak Drive", "Lost in Crowds"]);
    hand(&mut e, 1, &["Elder Intervention"]);
    e.state.seats[1].library = vec![find_library("Soak"); 3];
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(1, 0, a)); // Lost in Crowds: +1 stealth
    pick_when(&mut e, play(0, 0, z)); // Elder Intervention: +2 intercept
    let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert!(e.state.seats[1].hand.is_empty(), "not replaced yet");
    assert!(req.options.contains(&play(0, 1, a)));
    pick(&mut e, play(0, 1, a)); // Freak Drive [FOR]
    assert!(e.state.seats[1].hand.is_empty(), "not replaced yet");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked);
    assert!(
        !e.state.seats[1].hand.is_empty(),
        "replaced after the action"
    );
}

// [RTR 19970630] Form of Mist: "An action cannot continue 'as if unblocked'
// after a combat resulting from a successful action (e.g. Bum's Rush)."
// Form of Mist [PRO] in the combat of a Frontal Assault rush ends it, and
// the action is not continued.
#[test]
fn form_of_mist_does_not_continue_a_rush_after_its_combat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["PRO"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    let assault = location(&mut e, 0, "Frontal Assault");
    hand(&mut e, 0, &["Form of Mist"]);
    pick(&mut e, use_on(assault, 0, Some(a), Target::Minion(z)));
    let played = offered_before(&mut e, play(0, 1, a), SeatId(0), WindowLabel::MinionPhase);
    never_offered(&mut e, &Choice::ContinueAction);
    assert!(played, "Form of Mist [PRO] in the rush's combat");
    assert!(e.state.minions[a.index()].locked);
}

// [ANK 20180817] Magic of the Smith: "Does not prevent the acting vampire
// from playing the equipment/retainer/ally directly this turn if he
// unlocks (not the same action)." Having equipped a .44 Magnum from the
// library, the unlocked vampire may equip another from hand.
#[test]
fn magic_of_the_smith_does_not_bar_equipping_from_hand_after() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["tha"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let magnum = find_library(".44 Magnum");
    let soak = find_library("Soak");
    e.state.seats[0].library = vec![soak, magnum, soak];
    // A full hand: only the card's replacement is drawn.
    hand(&mut e, 0, &["Magic of the Smith", ".44 Magnum"]);
    hand(&mut e, 0, &["Deflection"; 5]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::FindCard(magnum));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == magnum && c.host == Some(a)));
    e.state.minions[a.index()].locked = false;
    let req = request(&mut e);
    let from_hand = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == magnum)
        .expect("the Magnum in hand") as u8;
    assert!(
        req.options.contains(&play(from_hand, 0, a)),
        "{:?}",
        req.options
    );
}

// [ANK 20231028] Unleashing the Bestial Soul: "An opponent can play a
// reaction to wake as the card is played, before its effects apply." The
// chosen locked vampire plays Wake with Evening's Freshness in the card's
// as-played window; woken, it blocks, but plays no other reaction card.
#[test]
fn a_wake_is_played_as_unleashing_the_bestial_soul_is_played() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(
        SeatId(0),
        vampire(|c| c.clan == "Salubri" && has(c, &["DOM"]), 0),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), with(&["aus"], 0), 5, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Unleashing the Bestial Soul"]);
    hand(
        &mut e,
        1,
        &["Wake with Evening's Freshness", "Enhanced Senses"],
    );
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(z)));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert!(req.options.contains(&play(0, 0, z)), "the wake");
    pick(&mut e, play(0, 0, z));
    let no_reaction = |o: &Choice| matches!(o, Choice::Play { actor: Some(m), .. } if *m == z);
    loop {
        let req = request(&mut e);
        assert!(!req.options.iter().any(no_reaction), "{req:?}");
        if req.options.contains(&Choice::DeclareBlock { minion: z }) {
            pick(&mut e, Choice::DeclareBlock { minion: z });
            break;
        }
        e.choose(fallback(&req));
    }
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, no_reaction);
    assert_eq!(pool(&e, 1), 30, "blocked by the woken vampire");
}

// [ANK 20230319] WMRH Talk Radio: "Does not consider blocks made before the
// effect was used (if the action has been continued with eg. Form of
// Mist)." The first block's combat ends with Form of Mist [PRO]; the
// woken blocker, given +1 intercept by the radio, fails the second
// attempt: its controller burns the pool.
#[test]
fn wmrh_talk_radio_ignores_a_block_before_form_of_mist_continued_the_action() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(
        SeatId(0),
        vampire(
            |c| has(c, &["PRO"]) && has_level(c, &["obf"]) && c.title == Title::None,
            0,
        ),
        6,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    let radio = location(&mut e, 1, "WMRH Talk Radio");
    hand(&mut e, 0, &["Form of Mist", "Lost in Crowds"]);
    hand(&mut e, 1, &["On the Qui Vive"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    form_of_mist_then_second_block(&mut e, a, z);
    pick_when(&mut e, use_on(radio, 0, None, Target::None)); // +1 intercept
    pick_when(&mut e, play(0, 0, a)); // Lost in Crowds: +1 stealth, 2 to 1
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[radio.index()].locked);
    assert_eq!(pool(&e, 1), 30 - 1 - 1, "the bleed, and the radio's pool");
}

// [ANK 20220218] [LSJ 20090325-2] Warsaw Station: "Can be burned during an
// action to bring back a vampire that was put into torpor during that
// action, but the action ends and cannot be continued (eg. with
// Ambulance)." A hunting Nosferatu sent to torpor by its blocker is
// brought back while the action is under way; Go-getter [OBF] no longer
// continues it.
#[test]
fn warsaw_station_brings_back_a_nosferatu_but_its_action_has_ended() {
    let mut e = Engine::bare(3);
    let nosferatu = vampire(|c| c.clan == "Nosferatu" && has(c, &["OBF"]), 0);
    let n = e.add_minion(SeatId(0), nosferatu, 0, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let z = e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    equip(&mut e, "Raven Spy", z); // +1 intercept: may block the hunt
    let w = location(&mut e, 0, "Warsaw Station");
    hand(&mut e, 0, &["Go-getter"]);
    // No blood: the hunt is its only option (elided).
    pick_when(&mut e, Choice::DeclareBlock { minion: z });
    let back = use_on(w, 1, None, Target::Minion(n));
    let mut used = false;
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 1, n)), "{req:?}");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        if let Some(i) = req.options.iter().position(|o| *o == back) {
            assert!(e.action().is_some(), "during the action");
            used = true;
            e.choose(i);
            continue;
        }
        e.choose(fallback(&req));
    }
    assert!(used, "Warsaw Station offered during the action");
    assert!(!e.state.minions[n.index()].in_torpor);
    assert!(!e.state.cards[w.index()].in_play);
}

// Second Tradition: Domain against a hunt (undirected: the prey may block):
// the locked prince unlocks, blocks with +2 intercept against the hunt's
// +1 stealth, and the block leads to combat.
#[test]
fn second_tradition_blocks_a_hunt_into_combat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", "", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let p = e.add_minion(SeatId(1), prince(0), 3, false);
    e.state.minions[p.index()].locked = true;
    hand(&mut e, 1, &["Second Tradition: Domain"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, play(0, 1, p));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    // 1 blood burned, 1 damage from A's hand strike; no blood hunted.
    assert_eq!(blood(&e, p), 3 - 1 - 1);
    assert_eq!(blood(&e, a), 2 - 1, "P's hand strike");
}

// Martina Srnankova: "+1 intercept during bleed actions" — not during
// Govern the Unaligned [DOM]'s action (+1 stealth, blood to an
// uncontrolled vampire: a card action, not a bleed): with no intercept
// against its stealth she is not offered a block; during its [dom] bleed
// she gets +1.
#[test]
fn martina_gets_no_intercept_against_a_superior_govern() {
    for bleed in [false, true] {
        let mut e = Engine::bare(3);
        let g = e.add_minion(SeatId(0), crypt("Abraham Mellon"), 4, false);
        e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        let martina = e.add_minion(SeatId(1), crypt("Martina Srnankova"), 4, false);
        let younger = uncontrolled(&mut e, 0, vampire(|c| c.capacity == 4, 0));
        hand(&mut e, 0, &["Govern the Unaligned"]);
        let block = Choice::DeclareBlock { minion: martina };
        if bleed {
            pick_when(&mut e, play(0, 0, g));
            offering(&mut e, block);
            let a = e.action().expect("the bleed");
            assert_eq!(a.intercept_of(&e.state, martina), 1);
        } else {
            pick_when(
                &mut e,
                play_at(0, 1, Some(g), Target::Uncontrolled(younger)),
            );
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, block);
            assert_eq!(e.state.seats[0].uncontrolled[younger as usize].counters, 3);
        }
    }
}
