//! Each terms decision told as a client reads it: its prompt and its
//! options' text and effects, the unit counted, where from and where to
//! (docs/protocol.md §5, §7), from positions built by hand.

use zvtes::cards::defs::DEFS;
use zvtes::cards::vampires::VAMPIRES;
use zvtes::cards::{AbilitySource, Effect as E, Layer, Scope, Target, Terms};
use zvtes::copies::CopyId;
use zvtes::crypt::{CryptCard, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, DecisionRequest};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::state::{find_card, find_library, CardId, CardInPlay, UncontrolledSlot};
use zvtes_api::message::Decision;
use zvtes_api::option::{EffectKind, OptionView};

use crate::describe;
use crate::numbering::Decks;
use crate::view::Viewer;

const SEATS: [&str; 4] = ["Ann", "Bob", "Cid", "Dee"];

/// The decision `req` as the deciding seat is sent it with the text
/// setting: built as `Game::advance` builds it, its prompt and options
/// rendered as `Game::tell` renders them.
fn told(e: &Engine, req: &DecisionRequest) -> Decision {
    let names: Vec<String> = SEATS[..e.state.seats.len()]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let dealt = Decks::dealt(e);
    let mut viewer = Viewer::new(Some(req.seat));
    let view = viewer.view(e, &dealt, &names);
    let n = &viewer.numbering;
    let options = req
        .options
        .iter()
        .enumerate()
        .map(|(i, c)| OptionView {
            index: i as u32,
            key: describe::key(e, req, c, n),
            effects: describe::effects(e, req, c, n),
            abilities: describe::abilities(e, c),
            apart: describe::ability_apart(e, c),
            user: describe::ability_user(e, req, c, n),
            text: None,
        })
        .collect();
    let mut d = Decision {
        decision: 0,
        seat: req.seat.0,
        window: crate::ids::window(req.window),
        source: describe::source(e, req, n),
        purpose: e.interrupt().map(|i| crate::ids::purpose(&i)),
        about: describe::about(e, req, n),
        due: describe::dues(e, req, n),
        options,
        undo: false,
        allocation: None,
        votes: None,
        prompt: None,
        view,
    };
    let mut known = zvtes_text::Names::new(crate::cards::cards());
    known.see(&d.view);
    d.prompt = zvtes_text::prompt(&d, &known);
    let offered = d.options.clone();
    for o in &mut d.options {
        o.text = zvtes_text::option(
            o,
            d.window,
            d.purpose,
            d.about.as_ref(),
            d.seat,
            &offered,
            &known,
        );
    }
    d
}

/// Runs `e` to the first decision `stop` accepts, taking at each the first
/// option `want` accepts, else a pass (or a hand strike), else the first.
fn drive(
    e: &mut Engine,
    want: impl Fn(&Engine, &Choice) -> bool,
    stop: impl Fn(&Engine, &DecisionRequest) -> bool,
) -> DecisionRequest {
    zvtes::copies::number_unnumbered(e);
    let mut last = None;
    for _ in 0..300 {
        let Step::Decision(req) = e.run() else {
            panic!("over before the terms: {last:?}");
        };
        if stop(e, &req) {
            return req;
        }
        let i = req
            .options
            .iter()
            .position(|o| want(e, o))
            .or_else(|| {
                req.options
                    .iter()
                    .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
            })
            .unwrap_or(0);
        e.choose(i);
        last = Some(req);
    }
    panic!("the terms never asked: {last:?}");
}

/// Driven to the terms of the in-play ability of `source`.
fn to_ability_terms(
    e: &mut Engine,
    source: AbilitySource,
    want: impl Fn(&Engine, &Choice) -> bool,
) -> Decision {
    let req = drive(e, want, |e, _| {
        e.ability_terms().is_some_and(|(s, ..)| s == source)
    });
    told(e, &req)
}

/// Driven to the rider terms of `card` at `seat`.
fn to_rider_terms(e: &mut Engine, card: CardId, seat: u8) -> Decision {
    let req = drive(
        e,
        |_, _| false,
        |e, req| req.seat == SeatId(seat) && e.rider_terms().is_some_and(|(c, _)| c == card),
    );
    told(e, &req)
}

/// The use of `source`'s ability, whatever its actor and target.
fn uses(source: AbilitySource) -> impl Fn(&Engine, &Choice) -> bool {
    move |_, c| matches!(c, Choice::UseAbility { source: s, .. } if *s == source)
}

/// The play of the hand card named `name` by `seat`.
fn plays(e: &Engine, c: &Choice, seat: u8, name: &str) -> bool {
    matches!(c, Choice::Play { card, .. }
        if e.state.active == SeatId(seat)
            && e.playable_card(SeatId(seat), *card) == find_library(name))
}

/// The option choosing the amount `n`.
fn amount(d: &Decision, n: u8) -> &OptionView {
    d.options
        .iter()
        .find(|o| o.key.amount == Some(n))
        .unwrap_or_else(|| panic!("no amount {n}: {:?}", d.options))
}

fn text(o: &OptionView) -> &str {
    o.text.as_deref().unwrap_or("<none>")
}

fn detail(d: &Decision) -> Option<&str> {
    d.about.as_ref().and_then(|a| a.detail.as_deref())
}

/// The `nth` V5 vampire with no printed-text definition matching `f`.
fn vampire(f: impl Fn(&CryptCard) -> bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| f(c) && zvtes::cards::vampire(*i as u16).is_none())
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a vampire matching the filter")
}

/// The `nth` untitled vampire of `clan` of capacity 5 or more.
fn plain(clan: &str, nth: usize) -> u16 {
    vampire(
        |c| c.clan == clan && c.title == Title::None && c.capacity >= 5,
        nth,
    )
}

fn name(card: u16) -> &'static str {
    CRYPT[card as usize].name
}

fn game(seats: usize) -> Engine {
    let mut e = Engine::bare(seats);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    e
}

/// A library card already in play for `owner`, on `host` if any, with its
/// first mode's layers (no play).
fn put(e: &mut Engine, card: &str, owner: u8, host: Option<MinionId>) -> CardId {
    let card = find_library(card);
    e.state.cards.push(CardInPlay::entering(
        card,
        CopyId::NONE,
        SeatId(owner),
        host,
        0,
    ));
    let id = CardId(e.state.cards.len() as u32 - 1);
    for effect in zvtes::cards::lookup(card).expect("defined").modes[0].effects {
        if let E::Layer(modifier) = effect {
            e.state.layers.push(Layer {
                scope: Scope::InPlay,
                modifier: *modifier,
                source: id,
            });
        }
    }
    id
}

fn hand(e: &mut Engine, seat: usize, cards: &[&str]) {
    for c in cards {
        e.state.seats[seat].hand.put(find_library(c), CopyId::NONE);
    }
}

// Heart of Nizchetus: the cards drawn, then as many put back.
#[test]
fn heart_of_nizchetus_counts_cards_drawn_then_bottomed() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let heart = put(&mut e, "Heart of Nizchetus", 0, Some(m));
    for _ in 0..3 {
        e.state.seats[0]
            .library
            .put(find_library("Soak"), CopyId::NONE);
    }
    let src = AbilitySource::Card(heart);
    let d = to_ability_terms(&mut e, src, uses(src));
    assert_eq!(detail(&d), Some("count"));
    assert_eq!(
        d.prompt.as_deref(),
        Some(
            "Heart of Nizchetus: how many cards to draw, then as many from your hand to the \
             bottom of your library"
        )
    );
    let o = amount(&d, 2);
    assert_eq!(
        (o.effects[0].kind, o.effects[0].detail.as_deref()),
        (EffectKind::Draw, Some("then_bottom"))
    );
    assert_eq!(
        text(o),
        "Draw 2 card(s), then 2 to the bottom of the library"
    );
}

// Jake Washington: blood put on the vampire chosen with the burn.
#[test]
fn jake_washington_counts_blood_on_his_target() {
    let mut e = game(2);
    let card = plain("Brujah", 0);
    let empty = e.add_minion(SeatId(0), card, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Jake Washington"]);
    e.at_master_phase();
    // Emptied once its own minion phase is past (it would hunt).
    drive(
        &mut e,
        |e, c| plays(e, c, 0, "Jake Washington"),
        |_, req| req.seat == SeatId(1),
    );
    e.state.minions[empty.index()].blood = 0;
    let jake = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Jake Washington"))
        .map(|i| CardId(i as u32))
        .expect("Jake in play");
    let src = AbilitySource::Card(jake);
    let d = to_ability_terms(&mut e, src, |_, c| {
        matches!(c, Choice::UseAbility { source, target: Target::Minion(t), .. }
            if *source == src && *t == empty)
    });
    assert_eq!(detail(&d), Some("blood_on_target"));
    assert!(d.about.as_ref().unwrap().minion.is_some());
    assert_eq!(
        d.prompt.as_deref().unwrap(),
        format!("Jake Washington: how much blood to put on {}", name(card))
    );
    let o = amount(&d, 3);
    assert_eq!(o.effects[0].kind, EffectKind::Blood);
    assert_eq!(o.effects[0].n, Some(3));
    assert_eq!(o.effects[0].copy, d.about.as_ref().unwrap().minion);
    assert_eq!(text(o), format!("3 blood on {}", name(card)));
}

// Nehemiah: cards from the hand to the bottom of the library.
#[test]
fn nehemiah_counts_cards_moved_to_the_bottom() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nehemiah"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Kevlar Vest", "Sport Bike"]);
    let src = AbilitySource::Vampire(n);
    let d = to_ability_terms(&mut e, src, uses(src));
    assert_eq!(detail(&d), Some("hand_to_bottom"));
    assert_eq!(
        d.prompt.as_deref(),
        Some("Nehemiah: how many cards to move from your hand to the bottom of your library")
    );
    let o = amount(&d, 2);
    assert_eq!(
        (
            o.effects[0].kind,
            o.effects[0].detail.as_deref(),
            o.effects[0].to
        ),
        (
            EffectKind::Move,
            Some("hand_to_bottom"),
            Some(zvtes_api::ids::Zone::Library)
        )
    );
    assert_eq!(text(o), "2 card(s) to the bottom of the library");
}

// Ilonka: cards discarded at random from the bleeder's controller's hand.
#[test]
fn ilonka_counts_cards_discarded_at_random_from_the_bleeders_hand() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ilonka = e.add_minion(SeatId(1), find_card("Ilonka"), 4, false);
    hand(&mut e, 0, &["Blood Rage", "Army of Rats"]);
    let src = AbilitySource::Vampire(ilonka);
    let d = to_ability_terms(&mut e, src, move |_, c| {
        matches!(c, Choice::TakeAction { minion, action: ActionKind::Bleed, .. } if *minion == a)
            || matches!(c, Choice::UseAbility { source, .. } if *source == src)
    });
    assert_eq!(detail(&d), Some("discard_at_random"));
    assert_eq!(d.about.as_ref().unwrap().seat, Some(0));
    assert_eq!(
        d.prompt.as_deref(),
        Some("Ilonka: how many cards to discard at random from Ann's hand")
    );
    let o = amount(&d, 1);
    assert_eq!(
        (
            o.effects[0].kind,
            o.effects[0].detail.as_deref(),
            o.effects[0].seat
        ),
        (EffectKind::Discard, Some("at_random"), Some(0))
    );
    assert_eq!(text(o), "Discard 1 card(s) at random from Ann's hand");
}

// Fleshforge Chamber: the ghoul picked is put on it, not discarded.
#[test]
fn fleshforge_chamber_puts_the_card_picked_on_itself() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let ff = put(&mut e, "Fleshforge Chamber", 0, None);
    // Two ghouls: one alone would be picked with no decision.
    hand(&mut e, 0, &["Szlachta Assistant", "Szlachta Bodyguard"]);
    e.at_master_phase();
    let src = AbilitySource::Card(ff);
    let d = to_ability_terms(&mut e, src, uses(src));
    assert_eq!(detail(&d), Some("put_on_source"));
    assert_eq!(
        d.prompt.as_deref(),
        Some("Fleshforge Chamber: the card from your hand to put on it")
    );
    let o = d
        .options
        .iter()
        .find(|o| o.key.card.as_ref().and_then(|c| c.id) == Some(card_id("Szlachta Assistant")))
        .expect("the Assistant offered");
    let f = &o.effects[0];
    assert_eq!(
        (f.kind, f.detail.as_deref(), f.to, f.card),
        (
            EffectKind::Move,
            Some("on_source"),
            Some(zvtes_api::ids::Zone::InPlay),
            Some(card_id("Fleshforge Chamber"))
        )
    );
    assert!(f.copy.is_some(), "the location's copy");
    assert_eq!(
        f.copy,
        d.source.as_ref().and_then(|c| c.copy),
        "the source's"
    );
    assert_eq!(text(o), "Put Szlachta Assistant on Fleshforge Chamber");
}

fn card_id(name: &str) -> u32 {
    LIBRARY[find_library(name) as usize].id
}

// Angelica: any number of master cards, one at a time.
#[test]
fn angelica_discards_any_masters_one_at_a_time() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        vampire(|c| c.disciplines.contains(&"obt"), 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ang = e.add_minion(SeatId(1), find_card("Angelica, The Canonicus"), 4, false);
    hand(&mut e, 0, &["Shadow Play"]);
    hand(&mut e, 1, &["Metro Underground", "Gird Minions"]);
    let src = AbilitySource::Vampire(ang);
    let d = to_ability_terms(&mut e, src, move |e, c| {
        matches!(c, Choice::TakeAction { minion, action: ActionKind::Bleed, .. } if *minion == a)
            || *c == Choice::DeclareBlock { minion: ang }
            || plays(e, c, 0, "Shadow Play")
            || matches!(c, Choice::UseAbility { source, .. } if *source == src)
    });
    assert_eq!(detail(&d), Some("discard_any"));
    assert_eq!(
        d.prompt.as_deref(),
        Some("Angelica, The Canonicus: the cards to discard, one at a time")
    );
    let o = d
        .options
        .iter()
        .find(|o| o.key.card.as_ref().and_then(|c| c.id) == Some(card_id("Gird Minions")))
        .expect("Gird Minions offered");
    assert_eq!(text(o), "Discard Gird Minions");
}

/// Seat 0's card in play with `counters`, seat 1 paying at its unlock.
fn prey_pays(card: &str, counters: u8) -> (Engine, CardId) {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 1), 4, false);
    let id = put(&mut e, card, 0, None);
    e.state.cards[id.index()].counters = counters;
    (e, id)
}

// Smiling Jack: each unit 1 pool or 1 blood of a vampire.
#[test]
fn smiling_jack_tells_each_unit_paid_in_pool_or_blood() {
    let (mut e, jack) = prey_pays("Smiling Jack, The Anarch", 2);
    let d = to_rider_terms(&mut e, jack, 1);
    assert_eq!(detail(&d), Some("pool_or_blood"));
    assert_eq!(
        d.prompt.as_deref(),
        Some(
            "Smiling Jack, The Anarch: 2 still to pay, each 1 pool or 1 blood from a vampire \
             you control"
        )
    );
    assert_eq!(text(&d.options[0]), "Burn 1 pool");
    assert_eq!(
        text(&d.options[1]),
        format!("Burn 1 blood from {}", name(plain("Brujah", 0)))
    );
}

// Constant Revolution: cards burned at random from the hand, the rest pool.
#[test]
fn constant_revolution_tells_cards_burned_from_the_hand_and_pool() {
    let (mut e, cr) = prey_pays("Constant Revolution", 2);
    hand(&mut e, 1, &["Villein", "Bonding", "Flash"]);
    let d = to_rider_terms(&mut e, cr, 1);
    assert_eq!(detail(&d), Some("pool_or_hand_cards"));
    assert_eq!(
        d.prompt.as_deref(),
        Some(
            "Constant Revolution: 2 to pay: how many cards to burn at random from your hand, \
             the rest in pool"
        )
    );
    let o = amount(&d, 1);
    assert_eq!(
        (o.effects[0].kind, o.effects[0].detail.as_deref()),
        (EffectKind::Burn, Some("hand_at_random"))
    );
    assert_eq!(
        (o.effects[1].kind, o.effects[1].n, o.effects[1].cost),
        (EffectKind::Pool, Some(-1), true)
    );
    assert_eq!(
        text(o),
        "Burn 1 card(s) at random from your hand and 1 pool"
    );
}

// The Gate of Acheron: ash heap cards removed at random, the rest pool.
#[test]
fn gate_of_acheron_tells_ash_cards_removed_and_pool() {
    let (mut e, gate) = prey_pays("Gate of Acheron, The", 2);
    for _ in 0..2 {
        e.state.seats[1].ash.put(find_library("Soak"), CopyId::NONE);
    }
    let d = to_rider_terms(&mut e, gate, 1);
    assert_eq!(detail(&d), Some("pool_or_ash_cards"));
    assert_eq!(
        d.prompt.as_deref(),
        Some(
            "The Gate of Acheron: 2 to pay: how many cards at random from your ash heap to \
             remove from the game, the rest in pool"
        )
    );
    let o = amount(&d, 1);
    assert_eq!(
        (
            o.effects[0].kind,
            o.effects[0].detail.as_deref(),
            o.effects[0].to
        ),
        (
            EffectKind::Move,
            Some("ash_at_random"),
            Some(zvtes_api::ids::Zone::Removed)
        )
    );
    assert_eq!(
        text(o),
        "Remove 1 card(s) at random from your ash heap and burn 1 pool"
    );
}

// Spectral Servitor [OBL]: 1 pool, or the ally burned.
#[test]
fn spectral_servitor_tells_pool_or_burn() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Hecata", 0), 4, false);
    let sup = e.add_ally_version(SeatId(0), find_library("Spectral Servitor"), 1);
    let card = e.state.minions[sup.index()].ally_id().expect("an ally");
    let d = to_rider_terms(&mut e, card, 0);
    assert_eq!(detail(&d), Some("pool_or_burn"));
    assert_eq!(
        d.prompt.as_deref(),
        Some("Spectral Servitor: burn 1 pool, or burn it")
    );
    assert_eq!(text(&d.options[0]), "Burn 1 pool");
    assert_eq!(text(&d.options[1]), "Burn Spectral Servitor");
}

// Villein: blood from its vampire to its Methuselah's pool.
#[test]
fn villein_tells_blood_moved_from_its_vampire_to_the_pool() {
    let mut e = game(2);
    let card = plain("Ventrue", 0);
    let v = e.add_minion(SeatId(0), card, 5, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Villein"]);
    e.at_master_phase();
    let req = drive(
        &mut e,
        move |e, c| {
            plays(e, c, 0, "Villein")
                && matches!(c, Choice::Play { target: Target::Minion(t), .. } if *t == v)
        },
        |e, _| e.play_terms().is_some(),
    );
    let d = told(&e, &req);
    let about = d.about.as_ref().unwrap();
    assert_eq!(
        (about.detail.as_deref(), about.minion.is_some(), about.to),
        (Some("blood_range"), true, None)
    );
    assert_eq!(
        d.prompt.as_deref().unwrap(),
        format!(
            "Villein: how much blood to move from {} to your pool",
            name(card)
        )
    );
    let o = amount(&d, 2);
    assert_eq!(
        (o.effects[0].kind, o.effects[0].n, o.effects[0].copy),
        (EffectKind::Blood, Some(-2), about.minion)
    );
    assert_eq!(
        (o.effects[1].kind, o.effects[1].n),
        (EffectKind::Pool, Some(2)),
        "to the pool"
    );
    assert_eq!(text(o), format!("2 blood from {}", name(card)));
}

// Grooming the Protégé: blood from the actor to the uncontrolled vampire.
#[test]
fn grooming_the_protege_tells_blood_moved_from_the_actor_to_the_protege() {
    let mut e = game(3);
    let card = plain("Ventrue", 0);
    let a = e.add_minion(SeatId(0), card, 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let cap = CRYPT[card as usize].capacity;
    let young = vampire(|c| c.clan == "Ventrue" && c.capacity < cap, 0);
    e.state.seats[0].uncontrolled.push(UncontrolledSlot {
        card: young,
        copy: Default::default(),
        counters: 0,
        banished: None,
    });
    hand(&mut e, 0, &["Grooming the Protégé"]);
    e.at_master_phase();
    let req = drive(
        &mut e,
        move |e, c| {
            plays(e, c, 0, "Grooming the Protégé")
                && matches!(c, Choice::Play { actor: Some(x), .. } if *x == a)
        },
        |e, _| e.play_terms().is_some(),
    );
    let d = told(&e, &req);
    let about = d.about.as_ref().unwrap();
    assert_eq!(about.detail.as_deref(), Some("blood_range"));
    assert!(about.minion.is_some() && about.to.is_some(), "{about:?}");
    assert_eq!(
        d.prompt.as_deref().unwrap(),
        format!(
            "Grooming the Protégé: how much blood to move from {} to {}",
            name(card),
            name(young)
        )
    );
    let o = amount(&d, 3);
    assert_eq!(
        (o.effects[0].kind, o.effects[0].n, o.effects[0].copy),
        (EffectKind::Blood, Some(-3), about.minion)
    );
    assert_eq!(
        (o.effects[1].kind, o.effects[1].n, o.effects[1].copy),
        (EffectKind::Blood, Some(3), about.to),
        "to the protégé"
    );
    assert_eq!(o.effects[1].on, Some(zvtes_api::option::On::Uncontrolled));
    assert_eq!(text(o), format!("3 blood from {}", name(card)));
}

// Ravnos Carnival: its counters counted with the action card they pay.
#[test]
fn ravnos_carnival_tells_the_counters_used_for_a_play() {
    let mut e = game(3);
    let r = e.add_minion(
        SeatId(0),
        vampire(
            |c| c.clan == "Ravnos" && c.disciplines.contains(&"pre") && c.title == Title::None,
            0,
        ),
        1,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let carnival = put(&mut e, "Ravnos Carnival", 0, None);
    e.state.cards[carnival.index()].counters = 2;
    hand(&mut e, 0, &["Propaganda"]);
    let req = drive(
        &mut e,
        move |e, c| {
            plays(e, c, 0, "Propaganda")
                && matches!(c, Choice::Play { actor: Some(x), .. } if *x == r)
        },
        |e, _| e.cost_terms_source().is_some(),
    );
    let d = told(&e, &req);
    let about = d.about.as_ref().unwrap();
    assert_eq!(about.detail.as_deref(), Some("cost_counters"));
    assert_eq!(
        about.card.as_ref().and_then(|c| c.id),
        Some(card_id("Ravnos Carnival"))
    );
    assert_eq!(
        d.prompt.as_deref(),
        Some("Propaganda: how many counters from Ravnos Carnival to use")
    );
    let o = amount(&d, 2);
    assert_eq!(text(o), "2 counter(s)");
}

/// The effects `count_of` reads, and the rider effects `rider_payment`
/// reads.
fn counted(f: &E) -> bool {
    matches!(
        f,
        E::DrawThenBottom
            | E::BloodToTargetX
            | E::HandToBottomX
            | E::ActingControllerDiscardsAtRandom
    )
}

fn per_counter(f: &E) -> bool {
    matches!(
        f,
        E::PoolOrBloodPerCounter
            | E::PoolOrRandomCardsPerCounter
            | E::PoolOrAshCardsPerCounter
            | E::BurnSelfUnlessPool(_)
    )
}

// Every count of an ability is of one effect `count_of` reads (none told
// by another card's reading), and every rider paid unit by unit is read
// by `rider_payment`.
#[test]
fn every_count_and_rider_payment_is_recognised() {
    let modes = DEFS
        .iter()
        .flat_map(|d| d.modes.iter().chain(d.abilities))
        .chain(VAMPIRES.iter().flat_map(|v| v.abilities.iter()));
    let mut counts = 0;
    for m in modes {
        if !matches!(m.terms, Some(Terms::Count { .. })) {
            continue;
        }
        counts += 1;
        let found: Vec<_> = m.effects.iter().filter(|f| counted(f)).collect();
        assert_eq!(found.len(), 1, "{:?}", m.effects);
        assert_eq!(describe::count_of(m).as_ref(), Some(found[0]));
    }
    assert!(counts >= 4, "{counts}");
    for v in VAMPIRES.iter() {
        let paid = v.riders.iter().flat_map(|r| r.effects).any(per_counter);
        assert!(!paid, "{}: a vampire's rider paid unit by unit", v.name);
    }
    let mut riders = 0;
    for d in DEFS.iter() {
        let found: Vec<_> = d
            .riders
            .iter()
            .flat_map(|r| r.effects)
            .filter(|f| per_counter(f))
            .collect();
        if found.is_empty() {
            continue;
        }
        riders += 1;
        assert_eq!(found.len(), 1, "{}: {found:?}", d.name);
        let mut e = Engine::bare(2);
        e.state.cards.push(CardInPlay::entering(
            find_library(d.name),
            CopyId::NONE,
            SeatId(0),
            None,
            0,
        ));
        assert_eq!(
            describe::rider_payment(&e, CardId(0)).as_ref(),
            Some(found[0]),
            "{}",
            d.name
        );
    }
    assert!(riders >= 5, "{riders}");
}
