//! Card behavior tables — hand-authored data, one entry per implemented
//! card, keyed by printed name against the generated static half
//! (`library::LIBRARY`). Windows are the reviewed matrix's; effects are the
//! primitives of `cards`. Slice 1 (master phase): Villein, Sudden Reversal.

use super::{
    Actor, CardDef, Effect, ModeDef, Modifier, Requirement, TargetKind, TargetSpec, Terms, Window,
};

/// "Trifle. Put this card on a vampire you control who has any amount of
/// blood and move 2 to 5 blood from that vampire to your pool. Cards named
/// Minion Tap cost you +1 pool to play. Villein costs +1 pool to play on this
/// vampire."
pub static VILLEIN: CardDef = CardDef {
    name: "Villein",
    trifle: true,
    out_of_turn: false,
    permanent: true,
    modes: &[ModeDef {
        window: Window::MasterPhase,
        actor: Actor::Methuselah,
        requires: &[],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[Requirement::Controlled, Requirement::BloodAtLeast(1)],
        }),
        terms: Some(Terms::BloodRange { min: 2, max: 5 }),
        effects: &[
            Effect::PutOnTarget,
            Effect::MoveBloodTargetToPoolX,
            Effect::Layer(Modifier::CostMod {
                name: "Minion Tap",
                on_host_only: false,
                owner_only: true,
                pool: 1,
            }),
            Effect::Layer(Modifier::CostMod {
                name: "Villein",
                on_host_only: true,
                owner_only: true,
                pool: 1,
            }),
        ],
    }],
};

/// "Out-of-turn. Cancel a master card as it is played by another Methuselah,
/// and its cost is not paid."
pub static SUDDEN_REVERSAL: CardDef = CardDef {
    name: "Sudden Reversal",
    trifle: false,
    out_of_turn: true,
    permanent: false,
    modes: &[ModeDef {
        window: Window::CardAsPlayed,
        actor: Actor::Methuselah,
        requires: &[
            Requirement::NotOwnTurn,
            Requirement::PlayedCardIsMaster,
            Requirement::PlayedByOtherSeat,
        ],
        target: None,
        terms: None,
        effects: &[Effect::CancelPlayed { refund: true }],
    }],
};

pub static DEFS: [&CardDef; 2] = [&VILLEIN, &SUDDEN_REVERSAL];
