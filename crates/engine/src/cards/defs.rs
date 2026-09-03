//! Card behavior tables — hand-authored data, one entry per implemented
//! card, keyed by printed name against the generated static half
//! (`library::LIBRARY`). Windows are the reviewed matrix's; effects are the
//! primitives of `cards`. Slice 1 (master phase): Villein, Sudden Reversal.
//! Slice 2 (the bleed): Conditioning, Consign to Oblivion, Form of the Cobra
//! [PRO] modifier, Eyes of Argus, Deflection, Archon Investigation,
//! Homunculus.

use super::{
    Actor, CardDef, Effect, ModeDef, Modifier, Requirement, TargetKind, TargetSpec, Terms, Window,
};
use crate::decision::ActionKind;

const NO_TARGET: Option<TargetSpec> = None;

/// "Trifle. Put this card on a vampire you control who has any amount of
/// blood and move 2 to 5 blood from that vampire to your pool. Cards named
/// Minion Tap cost you +1 pool to play. Villein costs +1 pool to play on this
/// vampire."
pub static VILLEIN: CardDef = CardDef {
    name: "Villein",
    trifle: true,
    out_of_turn: false,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[Requirement::Controlled, Requirement::BloodAtLeast(1)],
        }),
        terms: Some(Terms::BloodRange { min: 2, max: 5 }),
        waives_needed: false,
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
    abilities: &[],
};

/// "Out-of-turn. Cancel a master card as it is played by another Methuselah,
/// and its cost is not paid."
pub static SUDDEN_REVERSAL: CardDef = CardDef {
    name: "Sudden Reversal",
    trifle: false,
    out_of_turn: true,
    modes: &[ModeDef {
        windows: &[Window::CardAsPlayed],
        actor: Actor::Methuselah,
        requires: &[
            Requirement::NotOwnTurn,
            Requirement::PlayedCardIsMaster,
            Requirement::PlayedByOtherSeat,
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        effects: &[Effect::CancelPlayed { refund: true }],
    }],
    abilities: &[],
};

/// "Only usable during a bleed action. [dom] +2 bleed (limited). [DOM] +3
/// bleed (limited)."
pub static CONDITIONING: CardDef = CardDef {
    name: "Conditioning",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::ActionDuring],
            actor: Actor::ActingMinion,
            requires: &[Requirement::IsBleed],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            effects: &[Effect::BleedMod {
                n: 2,
                limited: true,
            }],
        },
        ModeDef {
            windows: &[Window::ActionDuring],
            actor: Actor::ActingMinion,
            requires: &[Requirement::IsBleed],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            effects: &[Effect::BleedMod {
                n: 3,
                limited: true,
            }],
        },
    ],
    abilities: &[],
};

/// "Only usable as the action is announced. [obl] Reaction cards cost +1
/// blood or life. [OBL] As above, and those cards are not replaced until the
/// end of the action."
pub static CONSIGN_TO_OBLIVION: CardDef = CardDef {
    name: "Consign to Oblivion",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::ActionAsAnnounced],
            actor: Actor::ActingMinion,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            effects: &[Effect::ActionLayer(Modifier::ReactionBloodCost(1))],
        },
        ModeDef {
            windows: &[Window::ActionAsAnnounced],
            actor: Actor::ActingMinion,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            effects: &[
                Effect::ActionLayer(Modifier::ReactionBloodCost(1)),
                Effect::DeferReactionReplacement,
            ],
        },
    ],
    abilities: &[],
};

/// "[PRO] [ACTION MODIFIER] Only usable as the action is announced. +1
/// stealth, even if stealth is not yet needed." (The [pro] combat strike
/// mode waits for the combat slice.)
pub static FORM_OF_THE_COBRA: CardDef = CardDef {
    name: "Form of the Cobra",
    trifle: false,
    out_of_turn: false,
    modes: &[
        // [pro] [COMBAT] Strike: steal 1 blood — combat slice.
        ModeDef {
            windows: &[],
            actor: Actor::ActingMinion,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            effects: &[],
        },
        ModeDef {
            windows: &[Window::ActionAsAnnounced],
            actor: Actor::ActingMinion,
            requires: &[Requirement::StealthNeeded],
            target: NO_TARGET,
            terms: None,
            waives_needed: true,
            effects: &[Effect::AddStealth(1)],
        },
    ],
    abilities: &[],
};

/// "[aus] Only usable during an action directed at you (or a card you
/// control). +2 intercept. [AUS] Only usable by a locked vampire. This
/// vampire wakes."
pub static EYES_OF_ARGUS: CardDef = CardDef {
    name: "Eyes of Argus",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::ActionStealthIntercept],
            actor: Actor::ReactingMinion,
            requires: &[Requirement::DirectedAtYou, Requirement::InterceptNeeded],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            effects: &[Effect::AddIntercept(2)],
        },
        ModeDef {
            windows: &[Window::Wake, Window::CardAsPlayed],
            actor: Actor::ReactingMinion,
            requires: &[Requirement::LockedActor],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            effects: &[Effect::Wake],
        },
    ],
    abilities: &[],
};

/// "Only usable if a minion is bleeding you, after blocks are declined.
/// [dom] Lock this reacting vampire. Change the target of the bleed to
/// another Methuselah other than the acting minion's controller (that
/// Methuselah can attempt to block). [DOM] As above, but do not lock this
/// vampire."
pub static DEFLECTION: CardDef = CardDef {
    name: "Deflection",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::ActionBlocksDeclined],
            actor: Actor::ReactingMinion,
            requires: &[
                Requirement::DirectedAtYou,
                Requirement::IsBleed,
                Requirement::BlocksDeclined,
            ],
            target: Some(TargetSpec {
                kind: TargetKind::Seat,
                filter: &[Requirement::NotActingController, Requirement::NotSelf],
            }),
            terms: None,
            waives_needed: false,
            effects: &[Effect::LockActor, Effect::RedirectBleed],
        },
        ModeDef {
            windows: &[Window::ActionBlocksDeclined],
            actor: Actor::ReactingMinion,
            requires: &[
                Requirement::DirectedAtYou,
                Requirement::IsBleed,
                Requirement::BlocksDeclined,
            ],
            target: Some(TargetSpec {
                kind: TargetKind::Seat,
                filter: &[Requirement::NotActingController, Requirement::NotSelf],
            }),
            terms: None,
            waives_needed: false,
            effects: &[Effect::RedirectBleed],
        },
    ],
    abilities: &[],
};

/// "Out-of-turn. Only usable if a minion is bleeding you and the bleed
/// amount is 4 or more, after blocks are declined. Burn the acting minion.
/// (The action is not successful.)"
pub static ARCHON_INVESTIGATION: CardDef = CardDef {
    name: "Archon Investigation",
    trifle: false,
    out_of_turn: true,
    modes: &[ModeDef {
        windows: &[Window::ActionBlocksDeclined],
        actor: Actor::Methuselah,
        requires: &[
            Requirement::NotOwnTurn,
            Requirement::DirectedAtYou,
            Requirement::IsBleed,
            Requirement::BleedAmountAtLeast(4),
            Requirement::BlocksDeclined,
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        effects: &[Effect::BurnActingMinion, Effect::ActionFails],
    }],
    abilities: &[],
};

/// "Retainer with 1 life. [pro] During any Methuselah's unlock phase, the
/// employer can burn 1 blood to unlock. [PRO] As above, but Homunculus has 2
/// life."
pub static HOMUNCULUS: CardDef = CardDef {
    name: "Homunculus",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::ActionDeclareEmploy],
            actor: Actor::ActingCandidate,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            effects: &[
                Effect::DeclareAction(ActionKind::Employ),
                Effect::EmployRetainer { life: 1 },
            ],
        },
        ModeDef {
            windows: &[Window::ActionDeclareEmploy],
            actor: Actor::ActingCandidate,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            effects: &[
                Effect::DeclareAction(ActionKind::Employ),
                Effect::EmployRetainer { life: 2 },
            ],
        },
    ],
    abilities: &[ModeDef {
        windows: &[Window::UnlockPhaseAny],
        actor: Actor::Host,
        requires: &[Requirement::HostLocked, Requirement::HostBloodAtLeast(1)],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        effects: &[Effect::HostBurnBloodToUnlock],
    }],
};

/// "Only usable during a bleed action. [dom] +1 bleed (limited). [DOM] +1
/// stealth and +1 bleed (limited)."
pub static BONDING: CardDef = CardDef {
    name: "Bonding",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::ActionDuring],
            actor: Actor::ActingMinion,
            requires: &[Requirement::IsBleed],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            effects: &[Effect::BleedMod {
                n: 1,
                limited: true,
            }],
        },
        ModeDef {
            windows: &[Window::ActionDuring, Window::ActionStealthIntercept],
            actor: Actor::ActingMinion,
            requires: &[Requirement::IsBleed, Requirement::StealthNeeded],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            effects: &[
                Effect::AddStealth(1),
                Effect::BleedMod {
                    n: 1,
                    limited: true,
                },
            ],
        },
    ],
    abilities: &[],
};

pub static DEFS: [&CardDef; 10] = [
    &BONDING,
    &VILLEIN,
    &SUDDEN_REVERSAL,
    &CONDITIONING,
    &CONSIGN_TO_OBLIVION,
    &FORM_OF_THE_COBRA,
    &EYES_OF_ARGUS,
    &DEFLECTION,
    &ARCHON_INVESTIGATION,
    &HOMUNCULUS,
];
