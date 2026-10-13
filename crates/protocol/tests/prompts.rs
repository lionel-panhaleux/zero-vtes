//! What a choice is for, said with the choice: a play's two vampires
//! (Toreador Grand Ball), a referendum's terms (Camarilla's Iron Fist,
//! Parity Shift).

use zvtes::cards::Target;
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::SeatId;
use zvtes::library::LIBRARY;
use zvtes_protocol::describe::Describer;
use zvtes_protocol::track::Tracker;
use zvtes_protocol::view::table_view;

fn library(name: &str) -> u16 {
    LIBRARY.iter().position(|c| c.name == name).expect(name) as u16
}

/// A plain (no printed text implemented) vampire.
fn plain(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, n: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| f(c) && zvtes::cards::vampire(*i as u16).is_none())
        .nth(n)
        .map(|(i, _)| i as u16)
        .expect("a plain vampire")
}

fn next(e: &mut Engine) -> DecisionRequest {
    match e.run() {
        Step::Decision(req) => req,
        Step::Over(_) => panic!("over"),
    }
}

/// The options' texts and the prompt, as the deciding seat reads them.
fn read(e: &Engine, req: &DecisionRequest, about: Option<&str>) -> (Vec<String>, Option<String>) {
    let view = table_view(e, Some(req.seat), &mut Tracker::default());
    let d = Describer {
        e,
        seat: req.seat,
        window: req.window,
        view: &view,
        names: &[],
        narrate: false,
        offered: &req.options,
        about: about.map(str::to_string),
    };
    let texts = req
        .options
        .iter()
        .enumerate()
        .map(|(i, c)| d.describe(i, c).text)
        .collect();
    (texts, d.prompt())
}

fn pick(e: &mut Engine, req: &DecisionRequest, c: &Choice) {
    let i = req.options.iter().position(|o| o == c).expect("offered");
    e.choose(i);
}

#[test]
fn toreador_grand_ball_names_the_one_locked_and_the_one_unblockable() {
    let mut e = Engine::bare(3);
    let toreador = |n| plain(|c| c.clan == "Toreador" && c.title == Title::None, n);
    let a = e.add_minion(SeatId(0), toreador(0), 4, false);
    let b = e.add_minion(SeatId(0), toreador(1), 4, false);
    e.add_minion(SeatId(0), toreador(2), 4, false);
    e.state.seats[0].hand.push(library("Toreador Grand Ball"));
    e.at_master_phase();
    let req = next(&mut e);
    let play = req
        .options
        .iter()
        .find(|c| matches!(c, Choice::Play { target: Target::Minion(m), .. } if *m == a))
        .copied()
        .expect("the ball on a");
    let (texts, _) = read(&e, &req, None);
    let i = req.options.iter().position(|c| *c == play).unwrap();
    let name = CRYPT[toreador(0) as usize].name;
    assert!(
        texts[i].contains(&format!(": lock {name}"))
            && texts[i]
                .contains("then choose the Toreador whose non-bleed actions cannot be blocked"),
        "{}",
        texts[i]
    );
    pick(&mut e, &req, &play);
    let req = next(&mut e);
    assert_eq!(req.window, WindowLabel::PlayTerms);
    let (texts, prompt) = read(&e, &req, Some("Toreador Grand Ball"));
    assert_eq!(
        prompt.as_deref(),
        Some("Toreador Grand Ball: choose the Toreador whose non-bleed actions cannot be blocked")
    );
    let i = req
        .options
        .iter()
        .position(|c| *c == Choice::ChooseMinion(b))
        .unwrap();
    assert!(
        texts[i].ends_with("(non-bleed actions cannot be blocked)"),
        "{}",
        texts[i]
    );
}

/// The caller's political action, to the terms (a prince calls).
fn to_terms(card: &str) -> (Engine, DecisionRequest) {
    let mut e = Engine::bare(4);
    let prince = plain(
        |c| c.sect == Sect::Camarilla && matches!(c.title, Title::Prince(_)),
        0,
    );
    e.add_minion(SeatId(0), prince, 8, false);
    for s in 1..4 {
        e.add_minion(
            SeatId(s),
            plain(|c| c.title == Title::None, s as usize),
            4,
            false,
        );
    }
    e.state.turn = 4;
    for seat in &mut e.state.seats {
        seat.pool = 10;
    }
    e.state.seats[1].pool = 20;
    e.state.seats[2].pool = 20;
    e.state.seats[0].hand.push(library(card));
    let req = next(&mut e);
    let call = *req
        .options
        .iter()
        .find(|c| matches!(c, Choice::Play { .. }))
        .unwrap_or_else(|| panic!("the call: {:?} {:?}", req.window, req.options));
    pick(&mut e, &req, &call);
    for _ in 0..200 {
        let req = next(&mut e);
        if req.window == WindowLabel::ReferendumTerms {
            return (e, req);
        }
        // Nobody blocks.
        let i = req
            .options
            .iter()
            .position(|c| *c == Choice::Pass)
            .unwrap_or(0);
        e.choose(i);
    }
    panic!("no terms");
}

#[test]
fn iron_fist_and_parity_shift_say_who_gains_and_who_pays() {
    let (mut e, req) = to_terms("Camarilla's Iron Fist");
    let (texts, prompt) = read(&e, &req, Some("Camarilla's Iron Fist"));
    assert_eq!(
        prompt.as_deref(),
        Some("Camarilla's Iron Fist: choose the Methuselah who gains 1 pool")
    );
    assert!(
        texts.iter().all(|t| t.ends_with("(gains 1 pool)")),
        "{texts:?}"
    );
    pick(&mut e, &req, &Choice::ChooseSeat(SeatId(0)));
    let req = next(&mut e);
    let (texts, prompt) = read(&e, &req, Some("Camarilla's Iron Fist"));
    assert_eq!(
        prompt.as_deref(),
        Some("Camarilla's Iron Fist: allocate the points: each Methuselah burns 1 pool per point")
    );
    assert!(
        texts.iter().all(|t| t.contains(" pool)"))
            && texts.iter().any(|t| t.contains("(burns 2 pool)")),
        "{texts:?}"
    );

    let (mut e, req) = to_terms("Parity Shift");
    let (texts, prompt) = read(&e, &req, Some("Parity Shift"));
    assert_eq!(
        prompt.as_deref(),
        Some("Parity Shift: choose the Methuselah whose pool is given away")
    );
    assert!(
        texts.iter().all(|t| t.ends_with("(gives the pool)")),
        "{texts:?}"
    );
    pick(&mut e, &req, &Choice::ChooseSeat(SeatId(1)));
    let req = next(&mut e);
    let (texts, prompt) = read(&e, &req, Some("Parity Shift"));
    let p = prompt.expect("a prompt");
    assert!(
        p.starts_with("Parity Shift: allocate ")
            && p.ends_with("each Methuselah gains 1 pool per point, from Seat 2"),
        "{p}"
    );
    assert!(
        texts.iter().all(|t| t.contains(" pool from Seat 2)")),
        "{texts:?}"
    );
}

/// A card's abilities, each said by what it does — not "(ability 2)"
/// (owner, 2026-10-04: Wider View, Dreams of the Sphinx).
#[test]
fn several_abilities_say_what_each_does() {
    use zvtes::cards::AbilitySource;
    use zvtes::state::{CardId, CardInPlay};
    let mut e = Engine::bare(3);
    let read = |e: &Engine, card: CardId, ability: u8| {
        let view = table_view(e, Some(SeatId(0)), &mut Tracker::default());
        let d = Describer {
            e,
            seat: SeatId(0),
            window: WindowLabel::MasterPhase,
            view: &view,
            names: &[],
            narrate: false,
            offered: &[],
            about: None,
        };
        let c = Choice::UseAbility {
            source: AbilitySource::Card(card),
            ability,
            actor: None,
            target: Target::None,
        };
        d.describe(0, &c).text
    };
    let mut put = |name: &str| {
        e.state
            .cards
            .push(CardInPlay::new(library(name), SeatId(0), None, 0));
        CardId(e.state.cards.len() as u32 - 1)
    };
    let wider = put("Wider View");
    let sphinx = put("Dreams of the Sphinx");
    assert_eq!(
        read(&e, wider, 0),
        "Use Wider View: 1 transfer, crypt draw, then remove one"
    );
    assert_eq!(
        read(&e, wider, 1),
        "Use Wider View: 4 transfers, burn it, gain 2 pool"
    );
    assert_eq!(
        read(&e, sphinx, 0),
        "Use Dreams of the Sphinx: +2 hand size this turn"
    );
    assert_eq!(read(&e, sphinx, 1), "Use Dreams of the Sphinx: gain 1 pool");
}

/// The blocker chooses its strike knowing the acting minion's: the table
/// shows it (owner, 2026-10-04: "the acting strike is not displayed
/// anywhere").
#[test]
fn the_acting_strike_is_shown_before_the_blocker_chooses() {
    use zvtes::decision::ActionKind;
    use zvtes::state::CardInPlay;
    let mut e = Engine::bare(3);
    let plain_of = |n| plain(|c| c.title == Title::None, n);
    let a = e.add_minion(SeatId(0), plain_of(0), 4, false);
    e.add_minion(SeatId(0), plain_of(1), 4, false);
    let z = e.add_minion(SeatId(1), plain_of(2), 4, false);
    e.state.cards.push(CardInPlay::new(
        library(".44 Magnum"),
        SeatId(1),
        Some(z),
        0,
    ));
    let req = next(&mut e);
    pick(
        &mut e,
        &req,
        &Choice::TakeAction {
            minion: a,
            action: ActionKind::Bleed,
            target: None,
        },
    );
    for _ in 0..50 {
        let req = next(&mut e);
        if req.window == WindowLabel::CombatStrike && req.seat == SeatId(1) {
            let view = table_view(&e, Some(SeatId(1)), &mut Tracker::default());
            let c = view.combat.expect("a combat");
            assert_eq!(c.sides[0].minion, a.0);
            assert_eq!(c.sides[0].strike.as_deref(), Some("hand strike"));
            assert_eq!(c.sides[1].strike, None);
            return;
        }
        let i = req
            .options
            .iter()
            .position(|c| *c == Choice::DeclareBlock { minion: z })
            .or_else(|| req.options.iter().position(|c| *c == Choice::Pass))
            .unwrap_or(0);
        e.choose(i);
    }
    panic!("no strike asked of the blocker");
}

/// A pass that does something says it (owner play feedback: Anarch
/// Convert's "gain 1 pool" read "Pass"): keeping him, then the pool.
#[test]
fn anarch_converts_passes_say_what_they_do() {
    let mut e = Engine::bare(2);
    let camarilla = |n| plain(|c| c.sect == Sect::Camarilla && c.title == Title::None, n);
    let v = e.add_minion(SeatId(0), camarilla(0), 4, false);
    e.add_minion(SeatId(1), camarilla(1), 4, false);
    let convert = CRYPT
        .iter()
        .position(|c| c.name == "Anarch Convert")
        .unwrap() as u16;
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: convert,
            copy: Default::default(),
            counters: 1,
            banished: None,
        });
    e.at_master_phase();
    let mut req = next(&mut e);
    for _ in 0..20 {
        match req.window {
            WindowLabel::EntryEffect => break,
            WindowLabel::InfluenceTransfers => {
                pick(&mut e, &req, &Choice::InfluenceOut { slot: 0 })
            }
            _ => pick(&mut e, &req, &Choice::Pass),
        }
        req = next(&mut e);
    }
    assert_eq!(req.window, WindowLabel::EntryEffect);
    let pass = req.options.iter().position(|c| *c == Choice::Pass).unwrap();
    assert_eq!(read(&e, &req, None).0[pass], "Keep him in play");
    pick(&mut e, &req, &Choice::ChooseMinion(v));
    let req = next(&mut e);
    let (texts, _) = read(&e, &req, None);
    let pass = req.options.iter().position(|c| *c == Choice::Pass).unwrap();
    assert_eq!(texts[pass], "Gain 1 pool");
    assert!(req.options.contains(&Choice::DrawCrypt));
}

// An ally's own ability names the ally as the table shows it: its minion's
// copy, as it has no place among the cards in play (Screamer's burn).
#[test]
fn an_allys_ability_names_its_copy_on_the_table() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain(|_| true, 0), 1, false);
    e.add_minion(SeatId(0), plain(|_| true, 1), 4, false);
    e.add_minion(SeatId(1), plain(|_| true, 2), 4, false);
    let screamer = e.add_ally(SeatId(1), library("Screamer"));
    let req = next(&mut e);
    pick(
        &mut e,
        &req,
        &Choice::TakeAction {
            minion: a,
            action: zvtes::decision::ActionKind::Bleed,
            target: None,
        },
    );
    let req = next(&mut e);
    assert_eq!(req.window, WindowLabel::ActionBlock);
    let view = table_view(&e, Some(req.seat), &mut Tracker::default());
    let d = Describer {
        e: &e,
        seat: req.seat,
        window: req.window,
        view: &view,
        names: &[],
        narrate: false,
        offered: &req.options,
        about: None,
    };
    let i = req
        .options
        .iter()
        .position(|c| matches!(c, Choice::UseAbility { .. }))
        .expect("Screamer's burn");
    let copy = view.seats[1]
        .ready
        .iter()
        .find(|m| m.minion == screamer.0)
        .map(|m| m.card.copy);
    let card = d.describe(i, &req.options[i]).card.expect("a card");
    assert!(copy.is_some());
    assert_eq!(card.copy, copy);
}

/// Answers Pass (or the first option) until `want` is offered.
fn until_offered(e: &mut Engine, want: &Choice) -> DecisionRequest {
    for _ in 0..100 {
        let req = next(e);
        if req.options.contains(want) {
            return req;
        }
        let i = req.options.iter().position(|c| *c == Choice::Pass);
        e.choose(i.unwrap_or(0));
    }
    panic!("{want:?} never offered");
}

fn describe_one(
    e: &Engine,
    req: &DecisionRequest,
    c: &Choice,
) -> zvtes_protocol::describe::OptionView {
    let view = table_view(e, Some(req.seat), &mut Tracker::default());
    let d = Describer {
        e,
        seat: req.seat,
        window: req.window,
        view: &view,
        names: &[],
        narrate: false,
        offered: &req.options,
        about: None,
    };
    let i = req.options.iter().position(|o| o == c).expect("offered");
    d.describe(i, c)
}

fn crypt(name: &str) -> u16 {
    CRYPT.iter().position(|c| c.name == name).expect(name) as u16
}

// Disciple of Lazarus searches the crypt: its pick names the crypt, not the
// ash heap. Solomon Batanea's pick is in his prey's uncontrolled region:
// it names the prey's card there.
#[test]
fn searches_and_picks_name_the_zone_they_take_from() {
    let mut e = Engine::bare(2);
    e.state.turn = 4;
    let erlik = e.add_minion(SeatId(0), crypt("Erlik"), 6, false);
    e.add_minion(SeatId(0), plain(|c| c.clan == "Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain(|c| c.clan == "Ventrue", 0), 4, false);
    let young = crypt("Nicomedes");
    e.state.seats[0].crypt = vec![young].into();
    e.state.seats[0].hand.push(library("Disciple of Lazarus"));
    let play = Choice::Play {
        card: 0,
        mode: 1,
        actor: Some(erlik),
        target: Target::None,
    };
    let req = until_offered(&mut e, &play);
    pick(&mut e, &req, &play);
    let want = Choice::AshVampire(young);
    let req = until_offered(&mut e, &want);
    let o = describe_one(&e, &req, &want);
    assert_eq!(o.zone.expect("a zone").zone, "crypt");

    let mut e = Engine::bare(2);
    e.state.turn = 4;
    let s = e.add_minion(SeatId(0), crypt("Solomon Batanea"), 4, false);
    e.add_minion(SeatId(0), plain(|c| c.clan == "Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain(|c| c.clan == "Ventrue", 0), 4, false);
    let slot = |card, counters| zvtes::state::UncontrolledSlot {
        card,
        copy: Default::default(),
        counters,
        banished: None,
    };
    e.state.seats[0].uncontrolled = vec![slot(plain(|c| c.clan == "Brujah", 1), 0)];
    e.state.seats[1].uncontrolled = vec![
        slot(plain(|c| c.clan == "Ventrue", 1), 3),
        slot(plain(|c| c.clan == "Ventrue", 2), 3),
    ];
    let solomon = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(s),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let req = until_offered(&mut e, &solomon);
    pick(&mut e, &req, &solomon);
    let want = Choice::Slot(1);
    let req = until_offered(&mut e, &want);
    let o = describe_one(&e, &req, &want);
    let view = table_view(&e, Some(req.seat), &mut Tracker::default());
    let copy = view.seats[1].uncontrolled[1].card.copy;
    assert_eq!(o.card.expect("a card").copy, Some(copy));
}
