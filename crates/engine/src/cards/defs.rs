//! Card behavior tables — hand-authored data, one entry per implemented
//! card, keyed by printed name against the generated static half
//! (`library::LIBRARY`). Windows are the reviewed matrix's; effects are the
//! primitives of `cards`. Slice 1 (master phase): Villein, Sudden Reversal.
//! Slice 2 (the bleed): Conditioning, Consign to Oblivion, Form of the Cobra
//! [PRO] modifier, Eyes of Argus, Deflection, Archon Investigation,
//! Homunculus. Slice 3 (combat): Flash, Torn Signpost, Roundhouse, Body
//! Flare, Theft of Vitae, Majesty, Side Strike, Blur, Immortal Grapple,
//! Hidden Strength, Soak, Indomitability, Claws of the Dead, Taste of Vitae,
//! Telepathic Tracking, Cats' Guidance, Form of Mist, Form of the Cobra
//! [pro].

use super::{
    Actor, CardDef, Effect, Limit, ModeDef, Modifier, Requirement, StrikeKind, TargetKind,
    TargetSpec, Terms, Window,
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
        limit: None,
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
        limit: None,
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
            limit: None,
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
            limit: None,
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
            limit: None,
            effects: &[Effect::ActionLayer(Modifier::ReactionBloodCost(1))],
        },
        ModeDef {
            windows: &[Window::ActionAsAnnounced],
            actor: Actor::ActingMinion,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
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
        // [pro] [COMBAT] Strike: steal 1 blood or life (becoming blood).
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::StealBlood {
                n: 1,
                ranged: false,
            })],
        },
        ModeDef {
            windows: &[Window::ActionAsAnnounced],
            actor: Actor::ActingMinion,
            requires: &[Requirement::StealthNeeded],
            target: NO_TARGET,
            terms: None,
            waives_needed: true,
            limit: None,
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
            limit: None,
            effects: &[Effect::AddIntercept(2)],
        },
        ModeDef {
            windows: &[Window::Wake, Window::CardAsPlayed],
            actor: Actor::ReactingMinion,
            requires: &[Requirement::LockedActor],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
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
            limit: None,
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
            limit: None,
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
        limit: None,
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
            limit: None,
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
            limit: None,
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
        limit: None,
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
            limit: None,
            effects: &[Effect::BleedMod {
                n: 1,
                limited: true,
            }],
        },
        ModeDef {
            // "+1 stealth and +1 bleed": one effect, playable whenever its
            // bleed half applies — the stealth rides along, needed or not.
            windows: &[Window::ActionDuring, Window::ActionStealthIntercept],
            actor: Actor::ActingMinion,
            requires: &[Requirement::IsBleed],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
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

/// "[pro] +1 stealth. [PRO] +2 stealth." — a pure stealth modifier: only
/// when needed (rules-qa 3.2).
pub static EARTH_CONTROL: CardDef = CardDef {
    name: "Earth Control",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::ActionStealthIntercept],
            actor: Actor::ActingMinion,
            requires: &[Requirement::StealthNeeded],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::AddStealth(1)],
        },
        ModeDef {
            windows: &[Window::ActionStealthIntercept],
            actor: Actor::ActingMinion,
            requires: &[Requirement::StealthNeeded],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::AddStealth(2)],
        },
    ],
    abilities: &[],
};

/// "[cel] Maneuver or press. [CEL] Maneuver with 1 optional press." The
/// effect matching the step applies (a maneuver in determine range, a
/// press in the press step).
pub static FLASH: CardDef = CardDef {
    name: "Flash",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatDetermineRange, Window::CombatPress],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Maneuver, Effect::Press],
        },
        ModeDef {
            windows: &[Window::CombatDetermineRange],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Maneuver, Effect::OptionalPress],
        },
    ],
    abilities: &[],
};

/// "Only usable before range is determined. [pot] This vampire gets a
/// strength of 2 this combat. [POT] … strength of 3 this combat."
pub static TORN_SIGNPOST: CardDef = CardDef {
    name: "Torn Signpost",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatBeforeRange],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::SetStrength(2)],
        },
        ModeDef {
            windows: &[Window::CombatBeforeRange],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::SetStrength(3)],
        },
    ],
    abilities: &[],
};

/// "[pot] Strike: hand strike at +2 damage. [POT] Strike: hand strike at
/// +3 damage."
pub static ROUNDHOUSE: CardDef = CardDef {
    name: "Roundhouse",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::Hand { bonus: 2 })],
        },
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::Hand { bonus: 3 })],
        },
    ],
    abilities: &[],
};

/// "[pro] Strike: 2 aggravated damage. [PRO] Strike: 2R aggravated
/// damage." (2 blood.)
pub static BODY_FLARE: CardDef = CardDef {
    name: "Body Flare",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::Damage {
                n: 2,
                ranged: false,
                aggravated: true,
            })],
        },
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::Damage {
                n: 2,
                ranged: true,
                aggravated: true,
            })],
        },
    ],
    abilities: &[],
};

/// "[tha] Strike, ranged: steal 1 blood or life (becoming blood). [THA]
/// … steal 2 blood or life."
pub static THEFT_OF_VITAE: CardDef = CardDef {
    name: "Theft of Vitae",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::StealBlood {
                n: 1,
                ranged: true,
            })],
        },
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::StealBlood {
                n: 2,
                ranged: true,
            })],
        },
    ],
    abilities: &[],
};

/// "[pre] Strike: combat ends. [PRE] As above, and unlock this vampire
/// before combat ends." (1 blood.)
pub static MAJESTY: CardDef = CardDef {
    name: "Majesty",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::CombatEnds { unlock: false })],
        },
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::CombatEnds { unlock: true })],
        },
    ],
    abilities: &[],
};

/// "[cel] Strike: dodge. [CEL] Additional strike (limited)."
pub static SIDE_STRIKE: CardDef = CardDef {
    name: "Side Strike",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::Dodge)],
        },
        ModeDef {
            windows: &[Window::CombatAdditionalStrike],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::AdditionalStrikes {
                n: 1,
                limited: true,
            }],
        },
    ],
    abilities: &[],
};

/// "[cel] Additional strike (limited). [CEL] 2 additional strikes
/// (limited)." (1 blood.)
pub static BLUR: CardDef = CardDef {
    name: "Blur",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatAdditionalStrike],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::AdditionalStrikes {
                n: 1,
                limited: true,
            }],
        },
        ModeDef {
            windows: &[Window::CombatAdditionalStrike],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::AdditionalStrikes {
                n: 2,
                limited: true,
            }],
        },
    ],
    abilities: &[],
};

/// "Grapple. Only usable at close range before strikes are chosen. A
/// vampire can play only one Immortal Grapple each round. [pot] Strikes
/// that are not hand strikes cannot be used this round (by either
/// combatant). [POT] As above, with 1 optional press. If another round of
/// combat occurs, that round is at close range (skip the determine range
/// step for that round)."
pub static IMMORTAL_GRAPPLE: CardDef = CardDef {
    name: "Immortal Grapple",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatBeforeStrikes],
            actor: Actor::Combatant,
            requires: &[Requirement::RangeClose],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerRound),
            effects: &[Effect::OnlyHandStrikes],
        },
        ModeDef {
            windows: &[Window::CombatBeforeStrikes],
            actor: Actor::Combatant,
            requires: &[Requirement::RangeClose],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerRound),
            effects: &[
                Effect::OnlyHandStrikes,
                Effect::OptionalPress,
                Effect::NextRoundClose,
            ],
        },
    ],
    abilities: &[],
};

/// "[for] Prevent X+1 damage. [FOR] As above, with 1 optional press." X is
/// the blood cost, chosen as the card is played (rules-qa 3.5); it may
/// exceed what is needed, and the card needs damage to prevent (rulings).
pub static HIDDEN_STRENGTH: CardDef = CardDef {
    name: "Hidden Strength",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatDamagePrevention],
            actor: Actor::Combatant,
            requires: &[Requirement::DamagePending],
            target: NO_TARGET,
            terms: Some(Terms::XBlood),
            waives_needed: false,
            limit: None,
            effects: &[Effect::PreventXPlus(1)],
        },
        ModeDef {
            windows: &[Window::CombatDamagePrevention],
            actor: Actor::Combatant,
            requires: &[Requirement::DamagePending],
            target: NO_TARGET,
            terms: Some(Terms::XBlood),
            waives_needed: false,
            limit: None,
            effects: &[Effect::PreventXPlus(1), Effect::OptionalPress],
        },
    ],
    abilities: &[],
};

/// "A vampire can play only one Soak each round. [for] Prevent 2
/// non-aggravated damage. [FOR] Prevent 4 non-aggravated damage."
pub static SOAK: CardDef = CardDef {
    name: "Soak",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatDamagePrevention],
            actor: Actor::Combatant,
            requires: &[Requirement::NonAggravatedDamagePending],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerRound),
            effects: &[Effect::PreventNonAggravated(2)],
        },
        ModeDef {
            windows: &[Window::CombatDamagePrevention],
            actor: Actor::Combatant,
            requires: &[Requirement::NonAggravatedDamagePending],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerRound),
            effects: &[Effect::PreventNonAggravated(4)],
        },
    ],
    abilities: &[],
};

/// "[for] Prevent 1 damage. [FOR] Press, or prevent 1 damage with 1
/// optional press." In the press step only the press applies.
pub static INDOMITABILITY: CardDef = CardDef {
    name: "Indomitability",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatDamagePrevention],
            actor: Actor::Combatant,
            requires: &[Requirement::DamagePending],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Prevent(1)],
        },
        ModeDef {
            windows: &[Window::CombatDamagePrevention, Window::CombatPress],
            actor: Actor::Combatant,
            requires: &[Requirement::DamagePending],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Prevent(1), Effect::OptionalPress, Effect::Press],
        },
    ],
    abilities: &[],
};

/// "[pro] Damage from this vampire's hand strikes is aggravated this round.
/// [PRO] Maneuver." (1 blood.) The basic mode has no printed step: any
/// impulse before the strike resolution it affects (rules-qa 2.2).
pub static CLAWS_OF_THE_DEAD: CardDef = CardDef {
    name: "Claws of the Dead",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatUntimed],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::AggravatedHands],
        },
        ModeDef {
            windows: &[Window::CombatDetermineRange],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Maneuver],
        },
    ],
    abilities: &[],
};

/// "Only usable at the end of a round of combat. Not usable by a vampire
/// being burned or going to torpor. A vampire can play only one Taste of
/// Vitae each round. This vampire gains blood equal to the amount of blood
/// lost by the opposing vampire to damage this round."
pub static TASTE_OF_VITAE: CardDef = CardDef {
    name: "Taste of Vitae",
    trifle: false,
    out_of_turn: false,
    modes: &[ModeDef {
        windows: &[Window::CombatEndOfRound],
        actor: Actor::Combatant,
        requires: &[Requirement::ActorReady],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: Some(Limit::PerRound),
        effects: &[Effect::GainBloodLostByOpponent],
    }],
    abilities: &[],
};

/// "[aus] Press, only usable to continue combat. If another round of combat
/// occurs, this vampire gets 1 optional maneuver that round. [AUS] Only
/// usable if both combatants are still ready and combat would end.
/// Instead, start a new round." (1 blood.)
pub static TELEPATHIC_TRACKING: CardDef = CardDef {
    name: "Telepathic Tracking",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatPress],
            actor: Actor::Combatant,
            requires: &[Requirement::NoContinuePress],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Press, Effect::NextRoundManeuver],
        },
        ModeDef {
            windows: &[Window::CombatWouldEnd],
            actor: Actor::Combatant,
            requires: &[
                Requirement::BothCombatantsReady,
                Requirement::CombatWouldEnd,
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::NewRound],
        },
    ],
    abilities: &[],
};

/// "[ani] Only usable by a locked vampire who has blocked, after block
/// resolution. Unlock this vampire. [ANI] +1 intercept." The basic mode's
/// point is the after-combat window (rules-qa 2.1).
pub static CATS_GUIDANCE: CardDef = CardDef {
    name: "Cats' Guidance",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::ActionAfterCombat],
            actor: Actor::ReactingMinion,
            requires: &[Requirement::LockedActor, Requirement::ActorHasBlocked],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::UnlockActor],
        },
        ModeDef {
            windows: &[Window::ActionStealthIntercept],
            actor: Actor::ReactingMinion,
            requires: &[Requirement::InterceptNeeded],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::AddIntercept(1)],
        },
    ],
    abilities: &[],
};

/// "[pro] Strike: dodge. [PRO] Strike: combat ends. After combat ends, if
/// this vampire was blocked, they can burn 1 blood to continue the action
/// with +1 stealth as if unblocked, even if stealth is not yet needed. A
/// vampire can play only one Form of Mist at superior each action."
pub static FORM_OF_MIST: CardDef = CardDef {
    name: "Form of Mist",
    trifle: false,
    out_of_turn: false,
    modes: &[
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::Dodge)],
        },
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Combatant,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerAction),
            effects: &[
                Effect::Strike(StrikeKind::CombatEnds { unlock: false }),
                Effect::ContinueAfterCombat {
                    blood: 1,
                    stealth: 1,
                },
            ],
        },
    ],
    abilities: &[],
};

pub static DEFS: [&CardDef; 28] = [
    &EARTH_CONTROL,
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
    &FLASH,
    &TORN_SIGNPOST,
    &ROUNDHOUSE,
    &BODY_FLARE,
    &THEFT_OF_VITAE,
    &MAJESTY,
    &SIDE_STRIKE,
    &BLUR,
    &IMMORTAL_GRAPPLE,
    &HIDDEN_STRENGTH,
    &SOAK,
    &INDOMITABILITY,
    &CLAWS_OF_THE_DEAD,
    &TASTE_OF_VITAE,
    &TELEPATHIC_TRACKING,
    &CATS_GUIDANCE,
    &FORM_OF_MIST,
];
