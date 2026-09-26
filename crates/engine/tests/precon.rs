//! Slice 11: the pre-built decks' cards. 11a: the cards built from the
//! engine's existing vocabulary — bleed and uncontrolled-region actions,
//! stealth and bleed modifiers, intercept and bleed-redirect reactions,
//! combat staples, and simple masters and locations. Windows with a single
//! option are elided, so the scripts only stop where a real decision
//! exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Stat, Target};
use zvtes::crypt::{Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId, CardInPlay, UncontrolledSlot};

fn request(e: &mut Engine) -> DecisionRequest {
    match e.run() {
        Step::Decision(req) => {
            if std::env::var("TRACE").is_ok() {
                eprintln!("REQ {:?} {:?} {:?}", req.seat, req.window, req.options);
            }
            req
        }
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
        .unwrap_or_else(|| {
            panic!(
                "option {want:?} not in {:?} ({:?})",
                req.options, req.window
            )
        });
    e.choose(idx);
}

/// The first option of a request that is a pass (or a hand strike).
fn fallback(req: &DecisionRequest) -> usize {
    req.options
        .iter()
        .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
        .unwrap_or(0)
}

/// Pass (or hand strike) through every decision until one offers `want`,
/// then take it.
fn pick_when(e: &mut Engine, want: Choice) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(|o| *o == want) {
            e.choose(i);
            return;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    panic!("{want:?} never offered");
}

/// Pass (or hand strike) through every decision until `seat`'s `window`.
fn until(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, window) {
            return req;
        }
        let i = fallback(&req);
        e.choose(i);
    }
}

fn hand(e: &mut Engine, seat: u8, cards: &[&str]) {
    for c in cards {
        e.state.seats[seat as usize].hand.push(find_library(c));
    }
}

fn play(card: u8, mode: u8, actor: MinionId) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target: Target::None,
    }
}

fn play_at(card: u8, mode: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor,
        target,
    }
}

fn action(minion: MinionId, action: ActionKind) -> Choice {
    Choice::TakeAction {
        minion,
        action,
        target: None,
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

fn blood(e: &Engine, m: MinionId) -> u8 {
    e.state.minions[m.index()].blood
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
    e.state.cards.push(CardInPlay {
        card,
        owner: SeatId(seat),
        controller: SeatId(seat),
        host: None,
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
        effect_only: false,
    });
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
    let g_card = vampire(
        |c| c.disciplines.contains(&"DOM") && c.title == Title::None && c.capacity >= 7,
        0,
    );
    let g = e.add_minion(SeatId(0), g_card, 4, false);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let cap = CRYPT[g_card as usize].capacity;
    let older = uncontrolled(&mut e, 0, vampire(|c| c.capacity >= cap, 0));
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

// Govern the Unaligned [dom]: "Ⓓ Bleed with +2 bleed."
#[test]
fn govern_the_unaligned_bleeds_with_plus_2_bleed() {
    let mut e = Engine::bare(3);
    let g = e.add_minion(SeatId(0), plain("", "dom", 0), 4, false);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    hand(&mut e, 0, &["Govern the Unaligned"]);
    let before = pool(&e, 1);
    pick(&mut e, play(0, 0, g));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), before - 3);
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

// Intimidation [PRE]: "Ⓓ Bleed with +2 bleed."
#[test]
fn intimidation_bleeds_with_plus_1_or_plus_2_bleed() {
    for (mode, tag, n) in [(0, "pre", 1), (1, "PRE", 2)] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("", tag, 0), 4, false);
        e.add_minion(SeatId(0), plain("", "", 0), 4, false);
        hand(&mut e, 0, &["Intimidation"]);
        let before = pool(&e, 1);
        pick(&mut e, play(0, mode, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e, 1), before - 1 - n, "{tag}");
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
    for (clan, n) in [("Toreador", 2), ("Ventrue", 1)] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain(clan, "pre", 0), 4, false);
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
    loop {
        let req = request(e);
        if let Some(i) = req.options.iter().position(|o| *o == play(0, 1, a)) {
            e.choose(i);
            return true;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            return false;
        }
        let i = fallback(&req);
        e.choose(i);
    }
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
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 0, a)), "{req:?}");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
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
    loop {
        let req = request(&mut e);
        if req.options.contains(&redirect(b)) {
            assert!(!req.options.contains(&redirect(other)));
            e.choose(req.options.iter().position(|o| *o == redirect(b)).unwrap());
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
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
    loop {
        let req = request(&mut e);
        if req.options.contains(&to(3)) {
            assert!(!req.options.contains(&to(2)));
            e.choose(req.options.iter().position(|o| *o == to(3)).unwrap());
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    let before = pool(&e, 3);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 3), before - 1);

    let (mut e, a, t) = bleed_at(2, plain("", "AUS", 0), "My Enemy's Enemy");
    pick(&mut e, action(a, ActionKind::Bleed));
    loop {
        let req = request(&mut e);
        assert!(
            !req.options.iter().any(|o| matches!(
                o,
                Choice::Play { mode: 1, actor: Some(m), .. } if *m == t
            )),
            "two Methuselahs left: {req:?}"
        );
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
}

/// Seat 0's `acting` bleeds with +2 stealth (Lost in Crowds [OBF]) against
/// seat 1's `blocker` holding `card`: was the bleed blocked?
fn blocked_with(acting: u16, blocker: u16, card: &str, extra_stealth: bool) -> bool {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), acting, 4, false);
    e.add_minion(SeatId(0), plain("", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), blocker, 4, false);
    hand(&mut e, 0, &["Lost in Crowds"]);
    if extra_stealth {
        hand(&mut e, 0, &["Lost in Crowds"]);
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

// Lost in Crowds, Swallowed by the Night [obf] and Resist Earth's Grasp
// [CEL]: "+1 stealth" when needed; the attempt fails.
#[test]
fn stealth_modifiers_beat_a_block_attempt() {
    for (tag, card, mode) in [
        ("obf", "Lost in Crowds", 0),
        ("obf", "Swallowed by the Night", 0),
        ("CEL", "Resist Earth's Grasp", 1),
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
    uncontrolled(&mut e, 0, vampire(|c| c.capacity >= 8, 0));
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

// Market Square: "Lock to give an Assamite you control +1 intercept" — the
// Banu Haqim attempting to block.
#[test]
fn market_square_locks_to_give_a_blocking_banu_haqim_intercept() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Banu Haqim", "", 0), 4, false);
    let square = location(&mut e, 1, "Market Square");
    hand(&mut e, 0, &["Lost in Crowds"]);
    let before = pool(&e, 1);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick(&mut e, play(0, 0, a)); // +1 stealth
    pick_when(&mut e, ability(square));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), before, "blocked");
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
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 0, z)), "{req:?}");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
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
    let z = e.add_minion(SeatId(1), vampire(|c| has(c, &["ANI", "aus"]), 0), 5, false);
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
    loop {
        let req = request(&mut e);
        if req.window == WindowLabel::ActionBlocksDeclined && req.seat == SeatId(1) {
            assert!(req.options.contains(&play(0, 0, z)), "{req:?}");
            e.choose(
                req.options
                    .iter()
                    .position(|o| *o == play(0, 0, z))
                    .unwrap(),
            );
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
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
        let mut maneuver = false;
        loop {
            let req = request(&mut e);
            maneuver |= req.options.contains(&Choice::UseManeuver);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            let i = fallback(&req);
            e.choose(i);
        }
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
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 1, p)), "{req:?}");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
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
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 1, across)), "{req:?}");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }

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
    let z = e.add_minion(SeatId(1), plain("", "", 1), 5, false);
    let magnum = equip(&mut e, ".44 Magnum", a);
    hand(&mut e, 0, &["Form of the Bat"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick_when(&mut e, play(0, 0, a)); // the attempt gets -1 intercept
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

/// The `nth` baron with no printed-text definition.
fn a_baron(nth: usize) -> u16 {
    vampire(|c| matches!(c.title, Title::Baron(_)), nth)
}

// Organized Resistance: "Requires a baron. Usable by a locked baron. An
// Anarch you control gets +1 intercept."
#[test]
fn organized_resistance_gives_an_attempting_anarch_intercept_from_a_locked_baron() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let b = e.add_minion(SeatId(1), a_baron(0), 4, false);
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

// Organized Resistance: "Also usable to unlock a locked Anarch you
// control; that Anarch attempts to block with +1 intercept, even if
// intercept is not yet needed." Not a locked non-Anarch.
#[test]
fn organized_resistance_unlocks_an_anarch_into_a_block_attempt() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    let b = e.add_minion(SeatId(1), a_baron(0), 4, false);
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
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            let i = fallback(&req);
            e.choose(i);
        }
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
    loop {
        let req = request(e);
        assert!(!req.options.contains(bad), "{bad:?} offered: {req:?}");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
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
// be used": Conditioning canceled by Direct Intervention, another
// Conditioning is still playable.
#[test]
fn a_canceled_limited_bleed_leaves_the_limit_unused() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("", "dom", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", "", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", "", 0), 4, false);
    hand(&mut e, 0, &["Conditioning", "Conditioning"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick_when(&mut e, play(0, 0, a));
    pick_for(&mut e, SeatId(1), master(0)); // canceled
    pick_when(&mut e, play(0, 0, a)); // the second: the limit is unused
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 30 - 1 - 1 - 2);
}

// ---- Titles and votes (slice 11e) -------------------------------------------

fn crypt(name: &str) -> u16 {
    zvtes::state::find_card(name)
}

fn votes(minion: MinionId, favor: bool) -> Choice {
    Choice::CastVotes { minion, favor }
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

/// Everyone passes the polling step until it closes.
fn close_polling(e: &mut Engine) {
    loop {
        match e.run() {
            Step::Decision(req)
                if matches!(
                    req.window,
                    WindowLabel::ReferendumPolling | WindowLabel::BloodHuntPolling
                ) =>
            {
                let i = fallback(&req);
                e.choose(i);
            }
            _ => return,
        }
    }
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
// the base damage": the +1 is no more preventable.
#[test]
fn blood_rage_nullifies_the_opposing_weapons_and_fortitude_prevention() {
    let modes: [(u8, &'static [&'static str], u8); 2] = [(0, &["tha"], 1), (1, &["THA"], 2)];
    for (mode, tags, damage) in modes {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), with(tags, 0), 3, false);
        e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        let b = e.add_minion(SeatId(1), with(&["for"], 0), 5, false);
        let gun = equip(&mut e, ".44 Magnum", b);
        hand(&mut e, 0, &["Blood Rage"]);
        hand(&mut e, 1, &["Soak"]);
        bleed_blocked(&mut e, a, b);
        pick_when(&mut e, play(0, mode, a));
        pick_when(&mut e, weapon_strike(gun));
        never_offered(&mut e, &play(0, 0, b));
        assert_eq!(blood(&e, a), 3, "the gun inflicted no damage");
        assert_eq!(blood(&e, b), 5 - damage, "the hand strike, unprevented");
    }
}

// [LSJ 20040928] Blood Rage, Soul Burn: "If the opponent dodges this
// strike, his weapons are unaffected and do damage this round." A dodge,
// then an additional strike with the gun.
#[test]
fn a_dodged_blood_rage_or_soul_burn_leaves_the_weapons_working() {
    for card in ["Blood Rage", "Soul Burn"] {
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
        let cost = if card == "Soul Burn" { 1 } else { 0 };
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
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    let gun = equip(&mut e, ".44 Magnum", b);
    hand(&mut e, 0, &["Soul Burn"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, weapon_strike(gun));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "Soul Burn's own blood cost only");
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
        let b = e.add_minion(SeatId(1), with(&["cel", "pre"], 0), 5, false);
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
    let a = e.add_minion(SeatId(0), with(&["ani", "cel"], 0), 5, false);
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
#[test]
fn touch_of_oblivion_keeps_the_minion_locked_two_unlock_phases() {
    let touch = find_library("Touch of Oblivion");
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with(&["obl"], 0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", "", 0), 5, false);
    e.add_minion(SeatId(1), plain("Gangrel", "", 0), 5, false);
    hand(&mut e, 0, &["Touch of Oblivion"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, b), vec![(touch, 2)]);
    assert_eq!(blood(&e, a), 2, "2 blood, and the hand strike");
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
// burn this card to get +1 stealth." A non-action card goes to the ash heap
// as it resolves and stays there; its delayed effect applies all the same
// (owner, 2026-09-26: rules-qa 21.1).
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
    // The card itself went to the ash heap as it resolved and stays there;
    // the effect is in play.
    assert_eq!(e.state.seats[0].ash, vec![shadow]);
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
    assert_eq!(e.state.seats[0].ash, vec![shadow], "still one card");
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
    assert!(req.options.contains(&play(0, 1, a)));
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 2, "a hand strike at +2, at close range");
    assert_eq!(blood(&e, a), 2, "Slam's blood, and the gun's 2");
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
    let req = until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    assert!(req.options.contains(&play(0, 0, a)));
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
// (eg. Blood Fury)" burns no counter — Blood Rage nullifies the weapon.
#[test]
fn weighted_walking_stick_loses_counters_per_damage_inflicted() {
    let stick = find_library("Weighted Walking Stick");
    for rage in [false, true] {
        let mut e = Engine::bare(3);
        let a = e.add_minion(SeatId(0), plain("Ventrue", "", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", "", 1), 5, false);
        let b = e.add_minion(SeatId(1), with(&["tha"], 0), 3, false);
        hand(&mut e, 0, &["Weighted Walking Stick"]);
        hand(&mut e, 1, &["Blood Rage"]);
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
        if rage {
            pick_when(&mut e, play(0, 0, b));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let want = if rage { 5 } else { 3 };
        assert_eq!(on(&e, a), vec![(stick, want)], "Blood Rage: {rage}");
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
    let a = e.add_minion(
        SeatId(0),
        vampire(
            |c| has(c, &["dom"]) && c.capacity >= 8 && c.title == Title::None,
            0,
        ),
        5,
        false,
    );
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
    let z = e.add_minion(SeatId(1), with(&["dom"], 0), 5, false);
    let y = e.add_minion(SeatId(1), with(&["dom"], 1), 5, false);
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
        hand(&mut e, 0, &["Daring the Dawn", "Change of Target"]);
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
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[a.index()].in_torpor, "ended: {ended}");
    }
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
