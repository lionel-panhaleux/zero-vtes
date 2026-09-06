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
    Actor, Among, CardDef, Effect, Hook, Limit, ModeDef, Modifier, Points, RefEffect,
    ReferendumDef, Requirement, Rider, RiderScope, SeatFilter, StrikeKind, TargetKind, TargetSpec,
    Term, Terms, TitleClass, Window,
};
use crate::crypt::Sect;
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
    delayed_replacement: false,
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
    riders: &[],
};

/// "Out-of-turn. Cancel a master card as it is played by another Methuselah,
/// and its cost is not paid."
pub static SUDDEN_REVERSAL: CardDef = CardDef {
    name: "Sudden Reversal",
    trifle: false,
    out_of_turn: true,
    delayed_replacement: false,
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
    riders: &[],
};

/// "Only usable during a bleed action. [dom] +2 bleed (limited). [DOM] +3
/// bleed (limited)."
pub static CONDITIONING: CardDef = CardDef {
    name: "Conditioning",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "Only usable as the action is announced. [obl] Reaction cards cost +1
/// blood or life. [OBL] As above, and those cards are not replaced until the
/// end of the action."
pub static CONSIGN_TO_OBLIVION: CardDef = CardDef {
    name: "Consign to Oblivion",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[PRO] [ACTION MODIFIER] Only usable as the action is announced. +1
/// stealth, even if stealth is not yet needed." (The [pro] combat strike
/// mode waits for the combat slice.)
pub static FORM_OF_THE_COBRA: CardDef = CardDef {
    name: "Form of the Cobra",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[aus] Only usable during an action directed at you (or a card you
/// control). +2 intercept. [AUS] Only usable by a locked vampire. This
/// vampire wakes."
pub static EYES_OF_ARGUS: CardDef = CardDef {
    name: "Eyes of Argus",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
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
    delayed_replacement: false,
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
    riders: &[],
};

/// "Out-of-turn. Only usable if a minion is bleeding you and the bleed
/// amount is 4 or more, after blocks are declined. Burn the acting minion.
/// (The action is not successful.)"
pub static ARCHON_INVESTIGATION: CardDef = CardDef {
    name: "Archon Investigation",
    trifle: false,
    out_of_turn: true,
    delayed_replacement: false,
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
    riders: &[],
};

/// "Retainer with 1 life. [pro] During any Methuselah's unlock phase, the
/// employer can burn 1 blood to unlock. [PRO] As above, but Homunculus has 2
/// life."
pub static HOMUNCULUS: CardDef = CardDef {
    name: "Homunculus",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "Only usable during a bleed action. [dom] +1 bleed (limited). [DOM] +1
/// stealth and +1 bleed (limited)."
pub static BONDING: CardDef = CardDef {
    name: "Bonding",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[pro] +1 stealth. [PRO] +2 stealth." — a pure stealth modifier: only
/// when needed (rules-qa 3.2).
pub static EARTH_CONTROL: CardDef = CardDef {
    name: "Earth Control",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[cel] Maneuver or press. [CEL] Maneuver with 1 optional press." The
/// effect matching the step applies (a maneuver in determine range, a
/// press in the press step).
pub static FLASH: CardDef = CardDef {
    name: "Flash",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "Only usable before range is determined. [pot] This vampire gets a
/// strength of 2 this combat. [POT] … strength of 3 this combat."
pub static TORN_SIGNPOST: CardDef = CardDef {
    name: "Torn Signpost",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[pot] Strike: hand strike at +2 damage. [POT] Strike: hand strike at
/// +3 damage."
pub static ROUNDHOUSE: CardDef = CardDef {
    name: "Roundhouse",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[pro] Strike: 2 aggravated damage. [PRO] Strike: 2R aggravated
/// damage." (2 blood.)
pub static BODY_FLARE: CardDef = CardDef {
    name: "Body Flare",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[tha] Strike, ranged: steal 1 blood or life (becoming blood). [THA]
/// … steal 2 blood or life."
pub static THEFT_OF_VITAE: CardDef = CardDef {
    name: "Theft of Vitae",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[pre] Strike: combat ends. [PRE] As above, and unlock this vampire
/// before combat ends." (1 blood.)
pub static MAJESTY: CardDef = CardDef {
    name: "Majesty",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[cel] Strike: dodge. [CEL] Additional strike (limited)."
pub static SIDE_STRIKE: CardDef = CardDef {
    name: "Side Strike",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[cel] Additional strike (limited). [CEL] 2 additional strikes
/// (limited)." (1 blood.)
pub static BLUR: CardDef = CardDef {
    name: "Blur",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
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
    delayed_replacement: false,
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
    riders: &[],
};

/// "[for] Prevent X+1 damage. [FOR] As above, with 1 optional press." X is
/// the blood cost, chosen as the card is played (rules-qa 3.5); it may
/// exceed what is needed, and the card needs damage to prevent (rulings).
pub static HIDDEN_STRENGTH: CardDef = CardDef {
    name: "Hidden Strength",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "A vampire can play only one Soak each round. [for] Prevent 2
/// non-aggravated damage. [FOR] Prevent 4 non-aggravated damage."
pub static SOAK: CardDef = CardDef {
    name: "Soak",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[for] Prevent 1 damage. [FOR] Press, or prevent 1 damage with 1
/// optional press." In the press step only the press applies.
pub static INDOMITABILITY: CardDef = CardDef {
    name: "Indomitability",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[pro] Damage from this vampire's hand strikes is aggravated this round.
/// [PRO] Maneuver." (1 blood.) The basic mode has no printed step: any
/// impulse before the strike resolution it affects (rules-qa 2.2).
pub static CLAWS_OF_THE_DEAD: CardDef = CardDef {
    name: "Claws of the Dead",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "Only usable at the end of a round of combat. Not usable by a vampire
/// being burned or going to torpor. A vampire can play only one Taste of
/// Vitae each round. This vampire gains blood equal to the amount of blood
/// lost by the opposing vampire to damage this round."
pub static TASTE_OF_VITAE: CardDef = CardDef {
    name: "Taste of Vitae",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[aus] Press, only usable to continue combat. If another round of combat
/// occurs, this vampire gets 1 optional maneuver that round. [AUS] Only
/// usable if both combatants are still ready and combat would end.
/// Instead, start a new round." (1 blood.)
pub static TELEPATHIC_TRACKING: CardDef = CardDef {
    name: "Telepathic Tracking",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[ani] Only usable by a locked vampire who has blocked, after block
/// resolution. Unlock this vampire. [ANI] +1 intercept." The basic mode's
/// point is the after-combat window (rules-qa 2.1).
pub static CATS_GUIDANCE: CardDef = CardDef {
    name: "Cats' Guidance",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

/// "[pro] Strike: dodge. [PRO] Strike: combat ends. After combat ends, if
/// this vampire was blocked, they can burn 1 blood to continue the action
/// with +1 stealth as if unblocked, even if stealth is not yet needed. A
/// vampire can play only one Form of Mist at superior each action."
pub static FORM_OF_MIST: CardDef = CardDef {
    name: "Form of Mist",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
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
    riders: &[],
};

// ---- Politics (slice 4) -------------------------------------------------

/// A political action card's single mode: declare the political action and
/// call `r` on success.
macro_rules! political {
    ($name:ident, $printed:expr, $doc:expr, $requires:expr, $r:expr) => {
        #[doc = $doc]
        pub static $name: CardDef = CardDef {
            name: $printed,
            trifle: false,
            out_of_turn: false,
            delayed_replacement: false,
            modes: &[ModeDef {
                windows: &[Window::ActionDeclarePolitical],
                actor: Actor::ActingCandidate,
                requires: $requires,
                target: NO_TARGET,
                terms: None,
                waives_needed: false,
                limit: None,
                effects: &[
                    Effect::DeclareAction(ActionKind::Political),
                    Effect::CallReferendum(&$r),
                ],
            }],
            abilities: &[],
            riders: &[],
        };
    };
}

static KRC_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::Allocate {
        points: Points::Fixed(4),
        among: Among::Any,
        min_seats: 2,
    }],
    effect: RefEffect::BurnAllocated,
};
political!(
    KINE_RESOURCES_CONTESTED,
    "Kine Resources Contested",
    "\"Allocate 4 points among two or more Methuselahs. Successful \
     referendum means each Methuselah burns 1 pool for each point \
     allocated.\" Points may go to yourself and may exceed a pool (LSJ \
     20000622, 20020819).",
    &[],
    KRC_REFERENDUM
);

static CONSERVATIVE_AGITATION_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::Allocate {
        points: Points::Methuselahs,
        among: Among::Any,
        min_seats: 2,
    }],
    effect: RefEffect::BurnAllocated,
};
political!(
    CONSERVATIVE_AGITATION,
    "Conservative Agitation",
    "\"Allocate X points among two or more Methuselahs, where X is the \
     number of Methuselahs in the game. Successful referendum means each \
     Methuselah burns 1 pool for each point allocated.\"",
    &[],
    CONSERVATIVE_AGITATION_REFERENDUM
);

static PER_MINION_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[],
    effect: RefEffect::BurnPerMinion { locked_only: false },
};
political!(
    ANCILLA_EMPOWERMENT,
    "Ancilla Empowerment",
    "\"Successful referendum means each Methuselah burns 1 pool for each \
     minion they control.\"",
    &[],
    PER_MINION_REFERENDUM
);
political!(
    ANARCHIST_UPRISING,
    "Anarchist Uprising",
    "\"Successful referendum means each Methuselah burns 1 pool for each \
     minion they control.\"",
    &[],
    PER_MINION_REFERENDUM
);

static PER_LOCKED_MINION_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[],
    effect: RefEffect::BurnPerMinion { locked_only: true },
};
political!(
    DOMAIN_CHALLENGE,
    "Domain Challenge",
    "\"Successful referendum means each Methuselah burns 1 pool for each \
     locked minion they control.\" Locked minions are counted after the \
     referendum is completed (RTR 19941109).",
    &[],
    PER_LOCKED_MINION_REFERENDUM
);

static NEONATE_BREACH_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::ChooseSeats {
        min: 1,
        max: u8::MAX,
    }],
    effect: RefEffect::BurnChosen {
        base: 1,
        young_bonus: 3,
        young_cap: 4,
    },
};
political!(
    NEONATE_BREACH,
    "Neonate Breach",
    "\"Choose one or more Methuselahs. Successful referendum means each \
     chosen Methuselah burns 1 pool, plus 3 additional pool if they control \
     a ready vampire with capacity 4 or less.\"",
    &[],
    NEONATE_BREACH_REFERENDUM
);

/// "Only usable during the polling step of a political action. [pre] This
/// vampire gets +2 votes. [PRE] This vampire gets +4 votes."
pub static BEWITCHING_ORATION: CardDef = CardDef {
    name: "Bewitching Oration",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[
        ModeDef {
            windows: &[Window::Referendum],
            actor: Actor::ActingMinion,
            requires: &[Requirement::PollingStep],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::AddVotes(2)],
        },
        ModeDef {
            windows: &[Window::Referendum],
            actor: Actor::ActingMinion,
            requires: &[Requirement::PollingStep],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::AddVotes(4)],
        },
    ],
    abilities: &[],
    riders: &[],
};

const HAS_CAST_VOTES: Option<TargetSpec> = Some(TargetSpec {
    kind: TargetKind::Minion,
    filter: &[Requirement::HasCastVotes],
});

/// "Only usable during the polling step of a political action. [cel][pre]
/// Choose a vampire who has cast votes or ballots in this referendum. The
/// chosen vampire is locked and abstains (this cancels the chosen vampire's
/// votes and ballots). [CEL][PRE] As above, and the chosen vampire burns 1
/// blood." Action Modifier/Reaction: the acting minion, or a reacting one.
pub static SCALPEL_TONGUE: CardDef = CardDef {
    name: "Scalpel Tongue",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[
        ModeDef {
            windows: &[Window::Referendum],
            actor: Actor::ActingOrReactingMinion,
            requires: &[Requirement::PollingStep],
            target: HAS_CAST_VOTES,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Abstain {
                lock: true,
                burn: 0,
            }],
        },
        ModeDef {
            windows: &[Window::Referendum],
            actor: Actor::ActingOrReactingMinion,
            requires: &[Requirement::PollingStep],
            target: HAS_CAST_VOTES,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Abstain {
                lock: true,
                burn: 1,
            }],
        },
    ],
    abilities: &[],
    riders: &[],
};

static RECKLESS_AGITATION_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::Allocate {
        points: Points::Fixed(6),
        among: Among::Others,
        min_seats: 2,
    }],
    effect: RefEffect::BurnAllocated,
};
political!(
    RECKLESS_AGITATION,
    "Reckless Agitation",
    "\"Requires an Independent or Anarch vampire with capacity 5 or more. \
     Allocate 6 points among two or more other Methuselahs. Successful \
     referendum means each Methuselah burns 1 pool for each point \
     allocated.\" Unplayable with a single opponent (LSJ 20100129).",
    &[
        Requirement::ActorSectIn(&[Sect::Independent, Sect::Anarch]),
        Requirement::ActorCapacityAtLeast(5),
    ],
    RECKLESS_AGITATION_REFERENDUM
);

static IRON_FIST_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[
        Term::ChooseSeat(SeatFilter::Any),
        Term::Allocate {
            points: Points::Fixed(5),
            among: Among::OthersNotChosen,
            min_seats: 2,
        },
    ],
    effect: RefEffect::BurnAllocatedChosenGains(1),
};
political!(
    CAMARILLAS_IRON_FIST,
    "Camarilla's Iron Fist",
    "\"Requires a titled Camarilla vampire. Choose a Methuselah and allocate \
     5 points among two or more other Methuselahs. Successful referendum \
     means each Methuselah burns 1 pool for each point allocated and the \
     chosen Methuselah gains 1 pool.\" No point to the chosen Methuselah \
     (ANK 20241106).",
    &[
        Requirement::ActorSectIn(&[Sect::Camarilla]),
        Requirement::ActorTitleIn(&[TitleClass::Any]),
    ],
    IRON_FIST_REFERENDUM
);

static PARITY_SHIFT_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[
        Term::ChooseSeat(SeatFilter::MorePoolThanCaller),
        Term::Allocate {
            points: Points::PoolOfChosen(3),
            among: Among::AnyNotChosen,
            min_seats: 1,
        },
    ],
    effect: RefEffect::MoveAllocatedFromChosen,
};
political!(
    PARITY_SHIFT,
    "Parity Shift",
    "\"Requires a prince or justicar. Choose a Methuselah who has more pool \
     than you do and allocate 3 of their pool among 1 or more other \
     Methuselahs (including you). Successful referendum means the pool is \
     distributed as you chose.\" A shortfall at resolution is re-allocated \
     by the caller within the terms (LSJ 20010606).",
    &[Requirement::ActorTitleIn(&[
        TitleClass::Prince,
        TitleClass::Justicar,
    ])],
    PARITY_SHIFT_REFERENDUM
);

static CONSANGUINEOUS_BOON_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::ChooseClan],
    effect: RefEffect::GainPerClanVampire,
};
political!(
    CONSANGUINEOUS_BOON,
    "Consanguineous Boon",
    "\"Boon. Choose a clan. Successful referendum means each Methuselah \
     gains 1 pool for each vampire of the chosen clan they control.\"",
    &[],
    CONSANGUINEOUS_BOON_REFERENDUM
);

static ANARCH_SALON_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[],
    effect: RefEffect::AnarchSalon,
};
political!(
    ANARCH_SALON,
    "Anarch Salon",
    "\"Requires an Anarch. Successful referendum means each ready Anarch \
     gains 1 blood and each Methuselah controlling an Anarch gains 1 \
     pool.\"",
    &[Requirement::ActorSectIn(&[Sect::Anarch])],
    ANARCH_SALON_REFERENDUM
);

static BANISHMENT_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::ChooseMinions {
        filter: &[Requirement::ActorReady, Requirement::Younger],
        min: 1,
        max: 1,
    }],
    effect: RefEffect::Banish,
};
political!(
    BANISHMENT,
    "Banishment",
    "\"Choose a ready younger vampire. Successful referendum means the \
     chosen vampire is moved to the uncontrolled region (breaking any \
     temporary control effects); any cards and counters on the chosen \
     vampire stay with them but are out of play as long as the chosen \
     vampire remains uncontrolled.\" Undirected (ANK 20221019-3).",
    &[],
    BANISHMENT_REFERENDUM
);

static EXPULSION_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::ChooseMinions {
        filter: &[],
        min: 0,
        max: 2,
    }],
    effect: RefEffect::Expel,
};
political!(
    EXPULSION,
    "Expulsion",
    "\"Requires a primogen or prince. Choose up to two minions. Successful \
     referendum means the chosen minions cannot play reaction cards, block \
     or cast votes or ballots this turn.\"",
    &[Requirement::ActorTitleIn(&[
        TitleClass::Primogen,
        TitleClass::Prince,
    ])],
    EXPULSION_REFERENDUM
);

/// A polling-step vote modifier: one mode, `actor`, +`n` votes.
macro_rules! polling_mode {
    ($actor:expr, $requires:expr, $effects:expr) => {
        ModeDef {
            windows: &[Window::Referendum],
            actor: $actor,
            requires: $requires,
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: $effects,
        }
    };
}

/// "Only usable during the polling step of a political action. This
/// Lasombra gets +3 votes." Action Modifier/Reaction.
pub static OMINOUS_CHORUS: CardDef = CardDef {
    name: "Ominous Chorus",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[polling_mode!(
        Actor::ActingOrReactingMinion,
        &[Requirement::PollingStep],
        &[Effect::AddVotes(3)]
    )],
    abilities: &[],
    riders: &[],
};

/// "[pre] Only usable during the polling step of a political action. This
/// vampire gets +3 votes. [PRE] Allies and younger vampires get -1
/// intercept." The superior half is playable any time during the action
/// before resolution (LSJ 20020612).
pub static PERFECT_PARAGON: CardDef = CardDef {
    name: "Perfect Paragon",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[
        polling_mode!(
            Actor::ActingMinion,
            &[Requirement::PollingStep],
            &[Effect::AddVotes(3)]
        ),
        ModeDef {
            windows: &[Window::ActionDuring],
            actor: Actor::ActingMinion,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::YoungerIntercept(-1)],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "Do not replace until your next unlock phase. [obf] Only usable during a
/// bleed action. +1 bleed (limited). [OBF] Only usable during the polling
/// step of a political action. This vampire gets +2 votes."
pub static OLD_FRIENDS: CardDef = CardDef {
    name: "Old Friends",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: true,
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
        polling_mode!(
            Actor::ActingMinion,
            &[Requirement::PollingStep],
            &[Effect::AddVotes(2)]
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Requires an Anarch. [obf] Reduce a bleed against you by 2. [pre] Only
/// usable during the polling step of a political action. This vampire gets
/// +2 votes. [pro] +1 intercept."
pub static PARTY_OUT_OF_BOUNDS: CardDef = CardDef {
    name: "Party Out Of Bounds",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[
        ModeDef {
            windows: &[Window::ReactionDuringAction],
            actor: Actor::ReactingMinion,
            requires: &[
                Requirement::ActorSectIn(&[Sect::Anarch]),
                Requirement::DirectedAtYou,
                Requirement::IsBleed,
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::ReduceBleed(2)],
        },
        polling_mode!(
            Actor::ReactingMinion,
            &[
                Requirement::ActorSectIn(&[Sect::Anarch]),
                Requirement::PollingStep
            ],
            &[Effect::AddVotes(2)]
        ),
        ModeDef {
            windows: &[Window::ActionStealthIntercept],
            actor: Actor::ReactingMinion,
            requires: &[
                Requirement::ActorSectIn(&[Sect::Anarch]),
                Requirement::InterceptNeeded,
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::AddIntercept(1)],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "Requires a primogen. Only usable during a bleed action against you or
/// during the polling step of a political action. Reduce the bleed amount
/// by 3 or this vampire gains +3 votes against the referendum." The votes
/// force the vampire's cast against; unusable by one voting in favor (LSJ
/// 20031115, 20031119).
pub static PROTECTED_DISTRICT: CardDef = CardDef {
    name: "Protected District",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[ModeDef {
        windows: &[Window::Referendum, Window::ReactionDuringAction],
        actor: Actor::ReactingMinion,
        requires: &[
            Requirement::ActorTitleIn(&[TitleClass::Primogen]),
            Requirement::AnyOf(&[
                &[Requirement::DirectedAtYou, Requirement::IsBleed],
                &[Requirement::PollingStep, Requirement::ActorNotVotingFor],
            ]),
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::ReduceBleed(3), Effect::AddVotesAgainst(3)],
    }],
    abilities: &[],
    riders: &[],
};

/// "Only usable during the polling step of a political action. Methuselahs
/// casting (including controlling a minion casting) votes or ballots
/// against the referendum burn 1 pool once results are tallied." Reaches
/// votes cast before it was played; 1 pool per Scorn per Methuselah (RTR
/// 19951110).
pub static SCORN_OF_ADONIS: CardDef = CardDef {
    name: "Scorn of Adonis",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[polling_mode!(
        Actor::ActingMinion,
        &[Requirement::PollingStep],
        &[Effect::TallyPenaltyAgainst(1)]
    )],
    abilities: &[],
    riders: &[],
};

const ANY_VAMPIRE: Option<TargetSpec> = Some(TargetSpec {
    kind: TargetKind::Minion,
    filter: &[],
});

/// "Only usable during the polling step of a political action. [aus] Cancel
/// the referendum. If you played a political action card to call this
/// referendum, return it to its owner's hand (discard down afterward).
/// [AUS] Force a vampire to abstain (this can cancel their votes and
/// ballots)."
pub static TELEPATHIC_VOTE_COUNTING: CardDef = CardDef {
    name: "Telepathic Vote Counting",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[
        polling_mode!(
            Actor::ActingMinion,
            &[Requirement::PollingStep],
            &[Effect::CancelReferendum {
                unlock_acting: false,
                forbid_repeat: false,
            }]
        ),
        ModeDef {
            windows: &[Window::Referendum],
            actor: Actor::ActingMinion,
            requires: &[Requirement::PollingStep],
            target: ANY_VAMPIRE,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Abstain {
                lock: false,
                burn: 0,
            }],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable during the polling step of a political action. Cancel the
/// referendum. Unlock the acting vampire. If a political action card was
/// played to call this referendum, return it to its owner's hand (discard
/// down afterward). Minions controlled by the acting Methuselah cannot
/// perform the same political action again this turn." The acting minion
/// has still taken its political action (RTR 19970630).
pub static DELAYING_TACTICS: CardDef = CardDef {
    name: "Delaying Tactics",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[polling_mode!(
        Actor::ReactingMinion,
        &[Requirement::PollingStep],
        &[Effect::CancelReferendum {
            unlock_acting: true,
            forbid_repeat: true,
        }]
    )],
    abilities: &[],
    riders: &[],
};

/// "Requires a prince, justicar or Inner Circle member. Only usable during
/// the polling step of a political action before votes and ballots are
/// cast. Non-Camarilla vampires cannot cast votes or ballots during this
/// referendum."
pub static CLOSED_SESSION: CardDef = CardDef {
    name: "Closed Session",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[polling_mode!(
        Actor::ActingMinion,
        &[
            Requirement::ActorTitleIn(&[TitleClass::Prince, TitleClass::Justicar]),
            Requirement::PollingStep,
            Requirement::NoVotesCast
        ],
        &[Effect::BarNonCamarillaVotes]
    )],
    abilities: &[],
    riders: &[],
};

/// "Only usable after resolution of a political action whose referendum
/// passed. [pre] This vampire gains 1 blood for each vote by which the
/// referendum passed. [PRE] As above, but move up to 2 of those blood to
/// your pool instead of this vampire."
pub static VOTER_CAPTIVATION: CardDef = CardDef {
    name: "Voter Captivation",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[
        ModeDef {
            windows: &[Window::ActionAfterResolution],
            actor: Actor::ActingMinion,
            requires: &[Requirement::IsPolitical, Requirement::ReferendumPassed],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::GainBloodPerMargin],
        },
        ModeDef {
            windows: &[Window::ActionAfterResolution],
            actor: Actor::ActingMinion,
            requires: &[Requirement::IsPolitical, Requirement::ReferendumPassed],
            target: NO_TARGET,
            terms: Some(Terms::UpToMargin(2)),
            waives_needed: false,
            limit: None,
            effects: &[Effect::GainBloodPerMargin],
        },
    ],
    abilities: &[],
    riders: &[],
};

pub static DEFS: [&CardDef; 75] = [
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
    &KINE_RESOURCES_CONTESTED,
    &CONSERVATIVE_AGITATION,
    &ANCILLA_EMPOWERMENT,
    &ANARCHIST_UPRISING,
    &DOMAIN_CHALLENGE,
    &NEONATE_BREACH,
    &BEWITCHING_ORATION,
    &SCALPEL_TONGUE,
    &RECKLESS_AGITATION,
    &CAMARILLAS_IRON_FIST,
    &PARITY_SHIFT,
    &CONSANGUINEOUS_BOON,
    &ANARCH_SALON,
    &BANISHMENT,
    &EXPULSION,
    &OMINOUS_CHORUS,
    &PERFECT_PARAGON,
    &OLD_FRIENDS,
    &PARTY_OUT_OF_BOUNDS,
    &PROTECTED_DISTRICT,
    &SCORN_OF_ADONIS,
    &TELEPATHIC_VOTE_COUNTING,
    &DELAYING_TACTICS,
    &CLOSED_SESSION,
    &VOTER_CAPTIVATION,
    &ANARCH_REVOLT,
    &GUARDIAN_ANGEL,
    &FAME,
    &TENSION_IN_THE_RANKS,
    &CREEPING_SABOTAGE,
    &BLOOD_DOLL,
    &VESSEL,
    &ACADEMIC_HUNTING_GROUND,
    &ASYLUM_HUNTING_GROUND,
    &BIOTECH_COMPANY_HUNTING_GROUND,
    &LIBRARY_HUNTING_GROUND,
    &PARK_HUNTING_GROUND,
    &POLITICAL_HUNTING_GROUND,
    &SLUM_HUNTING_GROUND,
    &SOCIETY_HUNTING_GROUND,
    &TEMPLE_HUNTING_GROUND,
    &UNDERWORLD_HUNTING_GROUND,
    &UPTOWN_HUNTING_GROUND,
    &WARZONE_HUNTING_GROUND,
    &ZOO_HUNTING_GROUND,
    &PAPILLON,
    &CARFAX_ABBEY,
];

// ---- Cards in play with riders and granted actions (slice 5) ------------

static BURN_THIS_CARD: ReferendumDef = ReferendumDef {
    terms: &[],
    effect: RefEffect::BurnSource,
};

/// "Put this card in play. A Methuselah not controlling a ready Anarch burns
/// 1 pool during their unlock phase. Vampires can call a referendum to burn
/// this card as a +1 stealth political action." The burn is a mandatory
/// unlock-phase effect the active Methuselah orders among their own and
/// resolves before passing the impulse (rules-qa 6.1; LSJ 20080106); copies
/// burn cumulatively (LSJ 19980225). Any Methuselah's vampire may call the
/// referendum: a political action (one per minion per turn, NRA per copy)
/// whose +1 stealth is the card's own text, with no calling card to vote
/// with (rules-qa 6.2).
pub static ANARCH_REVOLT: CardDef = CardDef {
    name: "Anarch Revolt",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::PutInPlay],
    }],
    abilities: &[ModeDef {
        windows: &[Window::ActionDeclarePolitical],
        actor: Actor::ActingCandidate,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::AddStealth(1),
            Effect::CallReferendum(&BURN_THIS_CARD),
        ],
    }],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::UnlockBurnPoolUnlessReady {
            sect: Sect::Anarch,
            pool: 1,
        }],
    }],
};

/// "Put this card on a ready vampire you control. This vampire gets +1
/// intercept during bleed actions directed at you. This vampire can prevent
/// 1 damage each combat. If this vampire is in torpor, burn this card." The
/// prevention is an in-play ability of the host (once per combat per copy,
/// no as-played window); the burn fires as the host goes to torpor.
pub static GUARDIAN_ANGEL: CardDef = CardDef {
    name: "Guardian Angel",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[Requirement::Controlled, Requirement::ActorReady],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::PutOnTarget,
            Effect::Layer(Modifier::HostInterceptOnBleedAtYou(1)),
        ],
    }],
    abilities: &[ModeDef {
        windows: &[Window::CombatDamagePrevention],
        actor: Actor::Host,
        requires: &[Requirement::DamagePending],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: Some(Limit::PerCombat),
        effects: &[Effect::Prevent(1)],
    }],
    riders: &[Rider {
        hook: Hook::HostToTorpor,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::BurnSelf],
    }],
};

/// "Unique. Put this card on a ready vampire. After this vampire goes to
/// torpor, their controller burns 3 pool. During each Methuselah's unlock
/// phase, if this vampire is in torpor, that Methuselah burns 1 pool." Any
/// ready vampire, any controller; the 3 pool burn fires where the torpor
/// happens (rules-qa 6.4) and may oust its controller mid-combat; the
/// unlock rider is every Methuselah's own, read at their unlock phase.
pub static FAME: CardDef = CardDef {
    name: "Fame",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[Requirement::ActorReady],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::PutOnTarget],
    }],
    abilities: &[],
    riders: &[
        Rider {
            hook: Hook::HostToTorpor,
            scope: RiderScope::Every,
            requires: &[],
            effects: &[Effect::TheirControllerBurnsPool(3)],
        },
        Rider {
            hook: Hook::UnlockPhase,
            scope: RiderScope::Every,
            requires: &[Requirement::HostInTorpor],
            effects: &[Effect::BurnPool(1)],
        },
    ],
};

/// "Unique. Put this card in play. After a ready minion is burned or sent
/// to torpor, their controller burns 1 pool. Methuselahs can use a master
/// phase action and discard two master cards to burn this card." The
/// rider fires where the burn or torpor happens (rules-qa 6.4), any
/// controller's minion; the burn is an ability of any Methuselah in their
/// own master phase, its two discards announced with the use.
pub static TENSION_IN_THE_RANKS: CardDef = CardDef {
    name: "Tension in the Ranks",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::PutInPlay],
    }],
    abilities: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[
            Requirement::MasterPhaseActionLeft,
            Requirement::HandMasters(2),
        ],
        target: NO_TARGET,
        terms: Some(Terms::DiscardMasters(2)),
        waives_needed: false,
        limit: None,
        effects: &[Effect::SpendMasterPhaseAction, Effect::BurnSelf],
    }],
    riders: &[Rider {
        hook: Hook::ReadyMinionBurnedOrTorpor,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::TheirControllerBurnsPool(1)],
    }],
};

/// Nosferatu. "+1 stealth action. X is the number of copies of this card
/// you control. Put this card in play. During your unlock phase, your prey
/// burns 1 pool. Minions can burn this card as a Ⓓ action." X is read when the cost is
/// paid (copies already in play); the unlock-phase burn is the controller's
/// mandatory effect (rules-qa 6.1); the burn action is directed at the
/// controller (undirected, +1 stealth, for their own minions).
pub static CREEPING_SABOTAGE: CardDef = CardDef {
    name: "Creeping Sabotage",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires: &[],
        target: NO_TARGET,
        terms: Some(Terms::CopiesControlled),
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
        ],
    }],
    abilities: &[ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::BurnSource,
        ],
    }],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::PreyBurnsPool(1)],
    }],
};

/// The two directions of "move 1 blood from this vampire to their pool or
/// from their pool to this vampire" as a host ability in `window` — once
/// per that phase per copy ("during X", rules-qa 6.6); usable with the host
/// in torpor (PIB 20150522).
macro_rules! blood_shuttle {
    ($window:expr) => {
        &[
            ModeDef {
                windows: &[$window],
                actor: Actor::Host,
                requires: &[Requirement::HostBloodAtLeast(1)],
                target: NO_TARGET,
                terms: None,
                waives_needed: false,
                limit: None,
                effects: &[Effect::HostBloodToPool(1)],
            },
            ModeDef {
                windows: &[$window],
                actor: Actor::Host,
                requires: &[
                    Requirement::HostBelowCapacity,
                    Requirement::HostControllerPoolAtLeast(1),
                ],
                target: NO_TARGET,
                terms: None,
                waives_needed: false,
                limit: None,
                effects: &[Effect::PoolToHostBlood(1)],
            },
        ]
    };
}

/// "Put this card on a vampire you control. During their master phase, this
/// vampire's controller can move 1 blood from this vampire to their pool or
/// from their pool to this vampire."
pub static BLOOD_DOLL: CardDef = CardDef {
    name: "Blood Doll",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[Requirement::Controlled],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::PutOnTarget],
    }],
    abilities: blood_shuttle!(Window::MasterPhase),
    riders: &[],
};

/// "Trifle. Put this card on a vampire and you can burn a card named Blood
/// Doll (if any). During their unlock phase, this vampire's controller can
/// move 1 blood from this vampire to their pool or from their pool to this
/// vampire." Any vampire in play; the Blood Doll may be any in play (rules-qa
/// 6.7), announced with the play like the target and burned at
/// resolution; the ability is the host's controller's.
pub static VESSEL: CardDef = CardDef {
    name: "Vessel",
    trifle: true,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[],
        }),
        terms: Some(Terms::BurnNamedInPlay("Blood Doll")),
        waives_needed: false,
        limit: None,
        effects: &[Effect::PutOnTarget, Effect::BurnChosenInPlay],
    }],
    abilities: blood_shuttle!(Window::UnlockPhaseOwn),
    riders: &[],
};

// ---- Hunting grounds (slice 5c: unique locations, library contests) -----

/// A hunting ground's unlock-phase pick: "a ready vampire you control [of
/// `$filter`] can gain `$n` blood. A vampire can gain blood from only one
/// hunting ground each turn." The controller picks the vampire as the
/// ability is used (its target), once per unlock phase per copy (rules-qa
/// 6.6); a vampire at capacity is still a legal pick (DESIGN §4.5 mask
/// policy) and counts as fed (rules-qa 6.5).
macro_rules! hunting_ground_pick {
    ($n:expr, $filter:expr) => {
        ModeDef {
            windows: &[Window::UnlockPhaseOwn],
            actor: Actor::Controller,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: $filter,
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::HuntingGroundBlood($n)],
        }
    };
}

/// "Unique location. Hunting ground. During your unlock phase, a ready
/// vampire you control can gain 1 blood. A vampire can gain blood from only
/// one hunting ground each turn." Put in play (no host); unique: contests
/// a copy under another Methuselah.
macro_rules! hunting_ground {
    ($name:ident, $printed:expr) => {
        pub static $name: CardDef = CardDef {
            name: $printed,
            trifle: false,
            out_of_turn: false,
            delayed_replacement: false,
            modes: &[ModeDef {
                windows: &[Window::MasterPhase],
                actor: Actor::Methuselah,
                requires: &[],
                target: NO_TARGET,
                terms: None,
                waives_needed: false,
                limit: None,
                effects: &[Effect::PutInPlay],
            }],
            abilities: &[hunting_ground_pick!(
                1,
                &[
                    Requirement::Controlled,
                    Requirement::ActorReady,
                    Requirement::NotHuntingGroundFed,
                ]
            )],
            riders: &[],
        };
    };
}

hunting_ground!(ACADEMIC_HUNTING_GROUND, "Academic Hunting Ground");
hunting_ground!(ASYLUM_HUNTING_GROUND, "Asylum Hunting Ground");
hunting_ground!(
    BIOTECH_COMPANY_HUNTING_GROUND,
    "Biotech Company Hunting Ground"
);
hunting_ground!(LIBRARY_HUNTING_GROUND, "Library Hunting Ground");
hunting_ground!(PARK_HUNTING_GROUND, "Park Hunting Ground");
hunting_ground!(POLITICAL_HUNTING_GROUND, "Political Hunting Ground");
hunting_ground!(SLUM_HUNTING_GROUND, "Slum Hunting Ground");
hunting_ground!(SOCIETY_HUNTING_GROUND, "Society Hunting Ground");
hunting_ground!(TEMPLE_HUNTING_GROUND, "Temple Hunting Ground");
hunting_ground!(UNDERWORLD_HUNTING_GROUND, "Underworld Hunting Ground");
hunting_ground!(UPTOWN_HUNTING_GROUND, "Uptown Hunting Ground");
hunting_ground!(WARZONE_HUNTING_GROUND, "Warzone Hunting Ground");
hunting_ground!(ZOO_HUNTING_GROUND, "Zoo Hunting Ground");

/// "Unique location. Hunting ground. Requires a ready vampire with a city
/// title. During your unlock phase, a ready titled vampire you control can
/// gain 2 blood. A vampire can gain blood from only one hunting ground each
/// turn." The play needs a ready prince or baron; the pick, any title.
pub static PAPILLON: CardDef = CardDef {
    name: "Papillon",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[Requirement::ControlsReadyTitled(&[
            TitleClass::Prince,
            TitleClass::Baron,
        ])],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::PutInPlay],
    }],
    abilities: &[hunting_ground_pick!(
        2,
        &[
            Requirement::Controlled,
            Requirement::ActorReady,
            Requirement::ActorTitleIn(&[TitleClass::Any]),
            Requirement::NotHuntingGroundFed,
        ]
    )],
    riders: &[],
};

/// "Requires a ready Anarch. Unique location. Hunting ground. During your
/// unlock phase, a ready Anarch you control can gain 1 blood, and, if you
/// control a ready baron, another ready Anarch you control can gain 1 blood
/// as well. A vampire can gain blood from only one hunting ground each
/// turn." Two picks, each once per unlock phase; the second needs a ready
/// baron and cannot be the first's vampire (fed this turn).
pub static CARFAX_ABBEY: CardDef = CardDef {
    name: "Carfax Abbey",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: false,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[Requirement::ControlsReadySect(&[Sect::Anarch])],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::PutInPlay],
    }],
    abilities: &[
        ModeDef {
            windows: &[Window::UnlockPhaseOwn],
            actor: Actor::Controller,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::Controlled,
                    Requirement::ActorReady,
                    Requirement::ActorSectIn(&[Sect::Anarch]),
                    Requirement::NotHuntingGroundFed,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerPhaseEach),
            effects: &[Effect::HuntingGroundBlood(1)],
        },
        ModeDef {
            windows: &[Window::UnlockPhaseOwn],
            actor: Actor::Controller,
            requires: &[Requirement::ControlsReadyTitled(&[TitleClass::Baron])],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::Controlled,
                    Requirement::ActorReady,
                    Requirement::ActorSectIn(&[Sect::Anarch]),
                    Requirement::NotHuntingGroundFed,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerPhaseEach),
            effects: &[Effect::HuntingGroundBlood(1)],
        },
    ],
    riders: &[],
};
