//! A play's target told in its option's effects (docs/protocol.md §7):
//! a card put on it, the clan it must be of, from positions built by
//! hand.

use zvtes::cards::{AbilitySource, Target};
use zvtes::crypt::CRYPT;
use zvtes::decision::{Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::state::{find_library, CardInPlay};
use zvtes_api::ids::Zone;
use zvtes_api::option::{Effect, EffectKind, On};

use crate::terms_told::told;

/// The `nth` vampire of `clan` with no printed-text definition.
fn of_clan(clan: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| c.clan == clan && zvtes::cards::vampire(*i as u16).is_none())
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a vampire of the clan")
}

/// `card` in seat 0's hand: the effects told of playing it on `target`,
/// passing until it is offered (seat 0's master phase).
fn played_on(mut e: Engine, card: &str, target: MinionId) -> Vec<Effect> {
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    e.state.seats[0].hand.push(find_library(card));
    zvtes::copies::number_unnumbered(&mut e);
    for _ in 0..100 {
        let req: DecisionRequest = match e.run() {
            Step::Decision(r) => r,
            Step::Over(_) => panic!("over before the play"),
        };
        let on = |o: &Choice| matches!(*o, Choice::Play { target: Target::Minion(t), .. } if t == target);
        if let Some(i) = req.options.iter().position(on) {
            assert_eq!(req.window, WindowLabel::MasterPhase);
            return told(&e, &req).options[i].effects.clone();
        }
        let pass = req.options.iter().position(|o| *o == Choice::Pass);
        e.choose(pass.unwrap_or(0));
    }
    panic!("the play never offered");
}

/// Toreador Grand Ball: "Lock a ready Toreador you control": the lock on
/// the target says the clan it must be.
#[test]
fn a_target_s_clan_is_told_on_its_effects() {
    let mut e = Engine::bare(3);
    let t = e.add_minion(SeatId(0), of_clan("Toreador", 0), 5, false);
    e.add_minion(SeatId(0), of_clan("Toreador", 1), 5, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 0), 5, false);
    let effects = played_on(e, "Toreador Grand Ball", t);
    let lock = effects
        .iter()
        .find(|f| f.kind == EffectKind::Lock && f.on == Some(On::Target))
        .expect("the lock on the target");
    assert_eq!(lock.clan.as_deref(), Some("toreador"), "{effects:?}");
}

/// Priority Contract: put on a minion the prey controls: a move of the
/// card played onto the target.
#[test]
fn a_card_put_on_its_target_is_told_moving_there() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), of_clan("Banu Haqim", 0), 5, false);
    e.add_minion(SeatId(0), of_clan("Banu Haqim", 1), 5, false);
    let t = e.add_minion(SeatId(1), of_clan("Ventrue", 0), 5, false);
    let effects = played_on(e, "Priority Contract", t);
    let id = LIBRARY[find_library("Priority Contract") as usize].id;
    assert!(
        effects.iter().any(|f| f.kind == EffectKind::Move
            && f.on == Some(On::Target)
            && f.to == Some(Zone::InPlay)
            && f.card == Some(id)
            && f.copy.is_some()),
        "{effects:?}"
    );
}

/// Saulot's Avenging Fist: "[ACTION] +1 stealth. Enter combat with a ready
/// Tremere": an ability's effects on its target say the clan too.
#[test]
fn an_ability_s_target_clan_is_told() {
    let mut e = Engine::bare(3);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    let salubri = e.add_minion(SeatId(0), of_clan("Salubri", 0), 5, false);
    e.add_minion(SeatId(0), of_clan("Salubri", 1), 5, false);
    let tremere = e.add_minion(SeatId(1), of_clan("Tremere", 0), 5, false);
    let fist = find_library("Saulot's Avenging Fist");
    e.state
        .cards
        .push(CardInPlay::new(fist, SeatId(0), Some(salubri), 0));
    let id = zvtes::state::CardId(e.state.cards.len() as u32 - 1);
    zvtes::copies::number_unnumbered(&mut e);
    let req = match e.run() {
        Step::Decision(r) => r,
        Step::Over(_) => panic!("a decision"),
    };
    let i = req
        .options
        .iter()
        .position(|o| {
            matches!(*o, Choice::UseAbility { source: AbilitySource::Card(c), target: Target::Minion(t), .. }
                if c == id && t == tremere)
        })
        .expect("the action on the Tremere offered");
    let effects = told(&e, &req).options[i].effects.clone();
    let enter = effects
        .iter()
        .find(|f| f.kind == EffectKind::EnterCombat && f.on == Some(On::Target))
        .expect("entering combat with the target");
    assert_eq!(enter.clan.as_deref(), Some("tremere"), "{effects:?}");
}
