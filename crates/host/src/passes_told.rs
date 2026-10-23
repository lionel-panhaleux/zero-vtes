//! What a pass does, told in its effects (docs/protocol.md §7), from
//! positions built by hand: the block window's, whose English reads them
//! alone.

use zvtes::copies::CopyId;
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library};
use zvtes_api::option::{Effect, EffectKind};

use crate::terms_told::told;

fn next(e: &mut Engine) -> DecisionRequest {
    match e.run() {
        Step::Decision(req) => req,
        Step::Over(_) => panic!("a decision"),
    }
}

/// The pass of `req` as its seat is told it: its effects and English.
fn pass(e: &Engine, req: &DecisionRequest) -> (Vec<Effect>, String) {
    let i = req.options.iter().position(|o| *o == Choice::Pass);
    let o = &told(e, req).options[i.expect("a pass")];
    (o.effects.clone(), o.text.clone().expect("told"))
}

/// Madison takes `action` (a bleed: 0 stealth; a hunt: +1) against Donny
/// Kowalczyk's 0 intercept, his seat the prey, holding Enhanced Senses
/// (the block window not elided while he is short): the minion phase's pass as she could
/// have ended it, then Donny's seat in the block window.
fn block_window(action: ActionKind) -> (Engine, MinionId, (Vec<Effect>, String)) {
    let mut e = Engine::bare(3);
    let m = e.add_minion(SeatId(0), find_card("Madison"), 4, false);
    e.add_minion(SeatId(0), find_card("Crystal Lynn"), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Donny Kowalczyk"), 4, false);
    let senses = find_library("Enhanced Senses");
    e.state.seats[1].hand.put(senses, CopyId::NONE);
    e.state.seats[1].library.put(senses, CopyId::NONE);
    let req = next(&mut e);
    let phase = pass(&e, &req);
    let take = req
        .options
        .iter()
        .position(|o| matches!(*o, Choice::TakeAction { minion, action: a, .. } if minion == m && a == action))
        .expect("the action offered");
    e.choose(take);
    (e, z, phase)
}

/// Pass until `seat` decides in the block window.
fn to_block(e: &mut Engine, seat: u8) -> DecisionRequest {
    for _ in 0..20 {
        let req = next(e);
        if req.window == WindowLabel::ActionBlock && req.seat.0 == seat {
            return req;
        }
        let i = req.options.iter().position(|o| *o == Choice::Pass);
        e.choose(i.expect("a pass"));
    }
    panic!("seat {seat} never in the block window");
}

fn block(copy: Option<u32>, detail: Option<&str>) -> Vec<Effect> {
    vec![Effect {
        copy,
        detail: detail.map(Into::into),
        ..Effect::new(EffectKind::Block)
    }]
}

#[test]
fn the_minion_phase_pass_ends_the_phase() {
    let (_, _, (effects, text)) = block_window(ActionKind::Bleed);
    assert_eq!(
        effects,
        vec![Effect {
            detail: Some("minion".into()),
            ..Effect::new(EffectKind::EndPhase)
        }]
    );
    assert_eq!(text, "End the minion phase");
}

/// rules-qa 1.3: no attempt pending, the prey's pass is its final
/// decline.
#[test]
fn the_prey_s_pass_with_no_attempt_declines() {
    let (mut e, _, _) = block_window(ActionKind::Bleed);
    let req = to_block(&mut e, 1);
    assert_eq!(
        pass(&e, &req),
        (block(None, Some("declines")), "Decline to block".into())
    );
}

/// rules-qa 82.1: its minion's attempt short of the stealth, the prey's
/// pass is the decline (an attempt meeting it, the sweep's
/// `every_option_and_choice_is_told`).
#[test]
fn a_pass_on_an_attempt_short_of_the_stealth_is_the_decline() {
    let (mut e, z, _) = block_window(ActionKind::Hunt);
    let req = to_block(&mut e, 1);
    let attempt = Choice::DeclareBlock { minion: z };
    let i = req.options.iter().position(|o| *o == attempt);
    e.choose(i.expect("the attempt offered"));
    let req = to_block(&mut e, 1);
    let copy = told(&e, &req).view.action.and_then(|a| a.blocker);
    assert!(copy.is_some(), "the attempt in the view");
    assert_eq!(
        pass(&e, &req),
        (block(copy, Some("declines")), "Decline to block".into())
    );
}
