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

use super::vampires::{ability, enter_combat, MANEUVER_EACH_COMBAT};
use super::{
    Actor, Among, AsVampire, Bearer, CardDef, CardFilter, Effect, Holding, Hook, Limit, ModeDef,
    Modifier, Points, RefEffect, ReferendumDef, Requirement, Rider, RiderScope, SeatFilter, Stat,
    StrikeKind, TargetKind, TargetSpec, Term, Terms, TitleClass, Until, Window,
};
use crate::crypt::Sect;
use crate::decision::ActionKind;
use crate::library::{CardType, Label};

const NO_TARGET: Option<TargetSpec> = None;

/// "Trifle. Put this card on a vampire you control who has any amount of
/// blood and move 2 to 5 blood from that vampire to your pool. Cards named
/// Minion Tap cost you +1 pool to play. Villein costs +1 pool to play on this
/// vampire."
pub static VILLEIN: CardDef = CardDef {
    name: "Villein",
    trifle: true,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[
                Requirement::TargetVampire,
                Requirement::Controlled,
                Requirement::BloodAtLeast(1),
            ],
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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

/// A recruit ally mode: the ally enters play in the acting Methuselah's
/// ready region with its printed life, its layers on itself.
const fn recruit(effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionDeclareRecruit],
        actor: Actor::ActingCandidate,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// "Unique mortal with 2 life. 1 strength, 0 bleed. You can burn 47th
/// Street Royals to reduce a bleed against you by 3." Its controller's,
/// during the bleed, locked or not.
pub static FORTY_SEVENTH_STREET_ROYALS: CardDef = CardDef {
    name: "47th Street Royals",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[recruit(&[Effect::DeclareAction(ActionKind::Recruit)])],
    abilities: &[ModeDef {
        windows: &[Window::ReactionDuringAction],
        actor: Actor::Controller,
        requires: &[Requirement::IsBleed, Requirement::DirectedAtYou],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::BurnSelf, Effect::ReduceBleed(3)],
    }],
    riders: &[],
};

/// "Mortal with 2 life. 0 strength, 0 bleed. This ally can strike: 1R
/// damage. This ally gets 1 optional maneuver each combat. This ally can
/// lock to give a Ravnos you control +1 stealth." The lock gives the
/// stealth to the acting Ravnos, when needed; usable the turn it is
/// recruited (ANK 20180517).
pub static CITY_STAR_TAXI: CardDef = CardDef {
    name: "City Star Taxi",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[recruit(&[Effect::DeclareAction(ActionKind::Recruit)])],
    abilities: &[
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Host,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::Damage {
                n: 1,
                ranged: true,
                aggravated: false,
            })],
        },
        MANEUVER_EACH_COMBAT,
        ModeDef {
            windows: &[Window::ActionDuring],
            actor: Actor::Host,
            requires: &[
                Requirement::CardUnlocked,
                Requirement::OtherActingControlled,
                Requirement::OtherActingClan("Ravnos"),
                Requirement::StealthNeeded,
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::AddStealth(1)],
        },
    ],
    riders: &[],
};

/// "Unique werewolf with 3 life. 1 strength, 1 bleed. Double Deuce has +1
/// stealth. If Double Deuce has 2 or fewer life during your unlock phase,
/// he gains 1 life." The gain is an unlock-phase rider its controller
/// orders among their own (rules-qa 6.1).
pub static DOUBLE_DEUCE: CardDef = CardDef {
    name: "Double Deuce",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[recruit(&[
        Effect::DeclareAction(ActionKind::Recruit),
        Effect::Layer(Modifier::Bearer {
            stat: Stat::Stealth,
            n: 1,
            during: &[],
            who: Bearer::Any,
        }),
    ])],
    abilities: &[],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Controller,
        requires: &[Requirement::HostBloodAtMost(2)],
        effects: &[Effect::AddHostBlood(1)],
    }],
};

/// "Animal with 1 life. Minions in combat with the employer cannot
/// strike: combat ends." Put on after the opposing strike was chosen, that
/// strike has no effect (ANK 20190412).
pub static DOG_PACK: CardDef = CardDef {
    name: "Dog Pack",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[employ(&[
        Effect::DeclareAction(ActionKind::Employ),
        Effect::EmployRetainer { life: 1 },
        Effect::Layer(Modifier::NoCombatEndsAgainstHost),
    ])],
    abilities: &[],
    riders: &[],
};

/// "Animal with 1 life. [ani] If the action to employ this retainer is
/// successful, unlock this vampire during the next discard phase. You can
/// lock this retainer to give the employer +1 intercept. [ANI] As above,
/// but unlock this vampire after resolution of this action instead." The
/// intercept goes to the employer's block attempt, when needed.
pub static FERAL_HOUND: CardDef = CardDef {
    name: "Feral Hound",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        employ(&[
            Effect::DeclareAction(ActionKind::Employ),
            Effect::EmployRetainer { life: 1 },
            Effect::UnlockActorAtDiscard,
        ]),
        employ(&[
            Effect::DeclareAction(ActionKind::Employ),
            Effect::EmployRetainer { life: 1 },
            Effect::UnlockActor,
        ]),
    ],
    abilities: &[ModeDef {
        windows: &[Window::ActionStealthIntercept],
        actor: Actor::Host,
        requires: &[Requirement::CardUnlocked, Requirement::InterceptNeeded],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::LockSelf, Effect::AddIntercept(1)],
    }],
    riders: &[],
};

/// "Ghoul with 2 life. The employer can lock this retainer to prevent 1
/// damage in combat. You can burn this retainer to have an action directed
/// at a minion you control fail." The failure is not usable while a block
/// is attempted or after one succeeded (ANK 20260316, LSJ 20041022); the
/// failed action reached resolution (NRA) and was not blocked (RTR
/// 20180719, LSJ 20070219).
pub static SZLACHTA_BODYGUARD: CardDef = CardDef {
    name: "Szlachta Bodyguard",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[employ(&[
        Effect::DeclareAction(ActionKind::Employ),
        Effect::EmployRetainer { life: 2 },
    ])],
    abilities: &[
        ModeDef {
            windows: &[Window::CombatDamagePrevention],
            actor: Actor::Host,
            requires: &[Requirement::CardUnlocked, Requirement::DamagePending],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::Prevent(1)],
        },
        ModeDef {
            windows: &[Window::ReactionDuringAction],
            actor: Actor::Controller,
            requires: &[
                Requirement::DirectedAtYourMinion,
                Requirement::NoBlockAttempt,
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::BurnSelf, Effect::ActionFails],
        },
    ],
    riders: &[],
};

/// "Ghoul with 5 life. 4 strength, 0 bleed. After this ally enters play,
/// burn an ally or retainer you control. It can prevent 1 damage each round
/// of combat. It can enter combat with a vampire as a Ⓓ action. It can lock
/// and burn itself to burn a location, except during combat." The entry
/// burn applies however it enters (ANK 20170309-2), itself if there is no
/// other (RTR 19961113); the location burn may answer an action stealing
/// that location (PIB 20121031).
pub static WAR_GHOUL: CardDef = CardDef {
    name: "War Ghoul",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[recruit(&[
        Effect::DeclareAction(ActionKind::Recruit),
        Effect::EntryBurnAllyOrRetainer,
    ])],
    abilities: &[
        ability(
            &[Window::CombatDamagePrevention],
            Actor::Host,
            &[Requirement::DamagePending],
            Limit::PerRound,
            &[Effect::Prevent(1)],
        ),
        enter_combat(&[
            Requirement::NotControlled,
            Requirement::ActorReady,
            Requirement::TargetVampire,
        ]),
        ModeDef {
            windows: &[Window::AnyImpulse],
            actor: Actor::Host,
            requires: &[Requirement::CardUnlocked, Requirement::NotInCombat],
            target: Some(TargetSpec {
                kind: TargetKind::Card,
                filter: &[Requirement::TargetCard(CardFilter::Location)],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::BurnTargetCard, Effect::BurnSelf],
        },
    ],
    riders: &[],
};

/// "Unique ghoul with 5 life. 3 strength, 0 bleed. After this ally enters
/// play, burn an ally or retainer you control. It can strike: 3R damage. It
/// can enter combat with a minion as a Ⓓ action. During your unlock phase,
/// you can discard an ally or retainer card to add 2 life to this ally."
/// The unlock discard is once per phase (rules-qa 6.6).
pub static VOZHD_OF_SOFIA: CardDef = CardDef {
    name: "Vozhd of Sofia, The",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[recruit(&[
        Effect::DeclareAction(ActionKind::Recruit),
        Effect::EntryBurnAllyOrRetainer,
    ])],
    abilities: &[
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Host,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::Damage {
                n: 3,
                ranged: true,
                aggravated: false,
            })],
        },
        enter_combat(&[Requirement::NotControlled, Requirement::ActorReady]),
        ModeDef {
            windows: &[Window::UnlockPhaseOwn],
            actor: Actor::Host,
            requires: &[Requirement::HandHas(CardFilter::AllyOrRetainer, 1)],
            target: NO_TARGET,
            terms: Some(Terms::Discard {
                n: 1,
                of: CardFilter::AllyOrRetainer,
            }),
            waives_needed: false,
            limit: None,
            effects: &[Effect::AddHostBlood(2)],
        },
    ],
    riders: &[],
};

/// "Zombie with 1 life. 1 strength, 1 bleed. [obl] This ally cannot have or
/// use equipment or retainers. During a bleed action, you can lock another
/// copy of this ally you control to give this ally +1 bleed. [OBL] As
/// above, and during a bleed action, a Hecata you control can burn 1 blood
/// to give this ally +1 bleed." During its bleed, before resolution (ANK
/// 20231124); a copy recruited this turn may be locked (ANK 20180517).
pub static BONE_SHAMBLER: CardDef = CardDef {
    name: "Bone Shambler",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        recruit(&[
            Effect::DeclareAction(ActionKind::Recruit),
            Effect::Layer(Modifier::NoEquipmentOrRetainers),
        ]),
        recruit(&[
            Effect::DeclareAction(ActionKind::Recruit),
            Effect::Layer(Modifier::NoEquipmentOrRetainers),
        ]),
    ],
    abilities: &[
        ModeDef {
            windows: &[Window::ActionDuring],
            actor: Actor::Host,
            requires: &[Requirement::ActorActing, Requirement::IsBleed],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::Controlled,
                    Requirement::TargetOtherCopy,
                    Requirement::TargetUnlocked,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::LockTarget,
                Effect::BleedMod {
                    n: 1,
                    limited: false,
                },
            ],
        },
        ModeDef {
            windows: &[Window::ActionDuring],
            actor: Actor::Host,
            requires: &[
                Requirement::SuperiorVersion,
                Requirement::ActorActing,
                Requirement::IsBleed,
            ],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::Controlled,
                    Requirement::ActorClan("Hecata"),
                    Requirement::BloodAtLeast(1),
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::TargetBurnsBlood(1),
                Effect::BleedMod {
                    n: 1,
                    limited: false,
                },
            ],
        },
    ],
    riders: &[],
};

/// "Unique mortal with 3 life. Requires an Anarch. If this Anarch is
/// blocked, they can burn 1 life from this retainer before block resolution
/// to lock the blocking minion and continue the action as if unblocked.
/// This retainer inflicts 1R damage on the opposing minion each round of
/// combat during normal strike resolution." The blocker is locked already
/// (LSJ 20080611); modifiers and reactions stand and the declines too (RTR
/// 19941109, ANK 20190116).
pub static CRYPTS_SONS: CardDef = CardDef {
    name: "Crypt's Sons",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionDeclareEmploy],
        actor: Actor::ActingCandidate,
        requires: &[Requirement::ActorSectIn(&[Sect::Anarch])],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Employ),
            Effect::EmployRetainer { life: 3 },
            Effect::Layer(Modifier::RetainerDamage(1)),
        ],
    }],
    abilities: &[ModeDef {
        windows: &[Window::ActionBlocked],
        actor: Actor::Host,
        requires: &[
            Requirement::ActorActing,
            Requirement::NotContinued,
            Requirement::LifeAtLeast(1),
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::BurnOwnLife(1), Effect::ContinueUnblocked],
    }],
    riders: &[],
};

/// "Ghoul with 2 life. The employer can burn this retainer to reduce the
/// cost of a ghoul ally requiring a Tzimisce they recruit by 2 blood or
/// pool." During that recruit action, the cost paid at resolution.
pub static SZLACHTA_ASSISTANT: CardDef = CardDef {
    name: "Szlachta Assistant",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[employ(&[
        Effect::DeclareAction(ActionKind::Employ),
        Effect::EmployRetainer { life: 2 },
    ])],
    abilities: &[ModeDef {
        windows: &[Window::ActionDuring],
        actor: Actor::Host,
        requires: &[
            Requirement::ActorActing,
            Requirement::RecruitingTzimisceGhoul,
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::BurnSelf, Effect::ReduceActionCost(2)],
    }],
    riders: &[],
};

/// "Animal with 1 life. 0 strength, 0 bleed. [ani] This ally can burn 1
/// life to give a minion you control 1 press. During an action directed at
/// you (or a card you control), you can burn this ally if it is not
/// blocking to unlock a ready minion you control. [ANI] As above, but this
/// ally has 2 life and 1 strength." The press is usable the turn it is
/// recruited (ANK 20180517).
pub static UNDERBRIDGE_STRAY: CardDef = CardDef {
    name: "Underbridge Stray",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        recruit(&[Effect::DeclareAction(ActionKind::Recruit)]),
        recruit(&[
            Effect::DeclareAction(ActionKind::Recruit),
            Effect::Version(crate::library::Body {
                life: 2,
                strength: 1,
                bleed: 0,
            }),
        ]),
    ],
    abilities: &[
        ModeDef {
            windows: &[Window::CombatPress],
            actor: Actor::ForOwnCombatant,
            requires: &[Requirement::LifeAtLeast(1)],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::BurnOwnLife(1), Effect::Press],
        },
        ModeDef {
            windows: &[Window::ReactionDuringAction],
            actor: Actor::Host,
            requires: &[Requirement::DirectedAtYou, Requirement::ActorNotBlocking],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[Requirement::Controlled, Requirement::ActorReady],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::BurnSelf, Effect::UnlockTarget],
        },
    ],
    riders: &[],
};

/// "Unique mortal with 2 life. 0 strength, 0 bleed. Vagabond Mystic can lock
/// to add 1 life to an ally you control who has fewer life than its
/// starting life. Vagabond Mystic cannot block vampires." Not in the middle
/// of a damage resolution step (LSJ 20001111); starting life is the
/// entering version's (advanced rules 5.5.2).
pub static VAGABOND_MYSTIC: CardDef = CardDef {
    name: "Vagabond Mystic",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[recruit(&[
        Effect::DeclareAction(ActionKind::Recruit),
        Effect::Layer(Modifier::CannotBlockVampires),
    ])],
    abilities: &[ModeDef {
        windows: &[Window::AnyImpulse],
        actor: Actor::Host,
        requires: &[
            Requirement::CardUnlocked,
            Requirement::NotInDamageResolution,
        ],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[Requirement::Controlled, Requirement::TargetAllyHurt],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::LockSelf, Effect::BloodToTarget(1)],
    }],
    riders: &[],
};

/// "Animal with 1 life. [ani] The employer gets +1 intercept. [ANI] As
/// above, and while the employer is in combat, the opposing minion's
/// controller gets -1 hand size." The opponent discards down at once and
/// draws back up as the combat ends — again for each new combat (LSJ
/// 20030530); cards not replaced count against the hand size (ANK
/// 20180318).
pub static RAPTOR: CardDef = CardDef {
    name: "Raptor",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        employ(&[
            Effect::DeclareAction(ActionKind::Employ),
            Effect::EmployRetainer { life: 1 },
            EMPLOYER_INTERCEPT,
        ]),
        employ(&[
            Effect::DeclareAction(ActionKind::Employ),
            Effect::EmployRetainer { life: 1 },
            EMPLOYER_INTERCEPT,
            Effect::Layer(Modifier::OpponentHandSizeInCombat(1)),
        ]),
    ],
    abilities: &[],
    riders: &[],
};

/// "Wraith with 1 life. 1 strength, 1 bleed. You can burn this ally to give
/// a minion controlled by your predator or prey -1 stealth. You can burn
/// this ally as an action directed at an ally you control is announced to
/// have it fail." The stealth goes off the acting minion; the failure comes
/// after the acting Methuselah's own as-announced effects (LSJ 20081105)
/// and before any other (ANK 20190425); the failed action reached
/// resolution (NRA) and was not blocked.
pub static SCREAMER: CardDef = CardDef {
    name: "Screamer",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[recruit(&[Effect::DeclareAction(ActionKind::Recruit)])],
    abilities: &[
        ModeDef {
            windows: &[Window::ReactionAsAnnounced, Window::ReactionDuringAction],
            actor: Actor::Controller,
            requires: &[Requirement::ActingByPredatorOrPrey],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::BurnSelf, Effect::ReduceStealth(1)],
        },
        ModeDef {
            windows: &[Window::ReactionAsAnnounced],
            actor: Actor::Controller,
            requires: &[Requirement::DirectedAtYourAlly],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::BurnSelf, Effect::ActionFails],
        },
    ],
    riders: &[],
};

/// "Unique ghoul with 5 life. 4 strength, 0 bleed. After this ally enters
/// play, burn an ally or retainer you control. This ally can enter combat
/// with a vampire as a Ⓓ action. Once each combat, this ally can discard a
/// card requiring Protean [pro] to prevent 2 damage. During any other
/// Methuselah's minion phase, a Tzimisce you control can burn 1 blood to
/// unlock this ally." The unlock is offered while the ally is locked.
pub static VOZHD_OF_SZCZECIN: CardDef = CardDef {
    name: "Vozhd of Szczecin, The",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[recruit(&[
        Effect::DeclareAction(ActionKind::Recruit),
        Effect::EntryBurnAllyOrRetainer,
    ])],
    abilities: &[
        enter_combat(&[
            Requirement::NotControlled,
            Requirement::ActorReady,
            Requirement::TargetVampire,
        ]),
        ModeDef {
            windows: &[Window::CombatDamagePrevention],
            actor: Actor::Host,
            requires: &[
                Requirement::DamagePending,
                Requirement::HandHas(CardFilter::Discipline("pro"), 1),
            ],
            target: NO_TARGET,
            terms: Some(Terms::Discard {
                n: 1,
                of: CardFilter::Discipline("pro"),
            }),
            waives_needed: false,
            limit: Some(Limit::PerCombat),
            effects: &[Effect::Prevent(2)],
        },
        ModeDef {
            windows: &[Window::AnyImpulse],
            actor: Actor::Controller,
            requires: &[Requirement::OthersMinionPhase, Requirement::CardLocked],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::Controlled,
                    Requirement::ActorClan("Tzimisce"),
                    Requirement::BloodAtLeast(1),
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::TargetBurnsBlood(1), Effect::UnlockSelf],
        },
    ],
    riders: &[],
};

/// "Zombie with 3 life. 2 strength, 0 bleed. This ally can enter combat
/// with a minion as a Ⓓ action. This ally cannot be the target of directed
/// actions requiring Dominate [dom] or Presence [pre]. In combat, strikes
/// made by this ally cannot be dodged. This ally cannot gain life
/// (instead, any life it would gain goes to the blood bank)." A dodge has
/// no effect on its strikes, but may still be played (LSJ 20030902-2).
pub static AGGRESSIVE_CORPSE: CardDef = CardDef {
    name: "Aggressive Corpse",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[recruit(&[
        Effect::DeclareAction(ActionKind::Recruit),
        Effect::Layer(Modifier::ImmuneToDirectedActions(&["dom", "pre"])),
        Effect::Layer(Modifier::UndodgeableStrikes),
        Effect::CannotGainLife,
    ])],
    abilities: &[enter_combat(&[
        Requirement::NotControlled,
        Requirement::ActorReady,
    ])],
    riders: &[],
};

/// "[pre] Ⓓ Bleed with +1 bleed. [PRE] +1 stealth action. Ⓓ Steal an ally
/// controlled by another Methuselah." The ally changes controller with the
/// cards on it; if it left play before resolution, the action is
/// successful with no effect (PIB 20121031).
pub static ENTRANCEMENT: CardDef = CardDef {
    name: "Entrancement",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::BleedMod {
                    n: 1,
                    limited: false,
                },
            ],
        },
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[Requirement::NotControlled, Requirement::TargetAlly],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::StealTargetAlly,
            ],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "+1 stealth action. [dom] Ⓓ Steal a retainer controlled by another
/// vampire. [DOM] Ⓓ Steal an ally controlled by another Methuselah." (1
/// blood.) Requirements do not apply to a card taken in play (TOM
/// 19960226-1); a retainer on one of the acting Methuselah's other
/// vampires makes the action undirected (rules-qa 17.4).
pub static FAR_MASTERY: CardDef = CardDef {
    name: "Far Mastery",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Card,
                filter: &[
                    Requirement::TargetCard(CardFilter::Type(CardType::Retainer)),
                    Requirement::TargetOnVampire,
                    Requirement::NotHost,
                    Requirement::ActorMayHoldTarget,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::StealTargetCard,
            ],
        },
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[Requirement::NotControlled, Requirement::TargetAlly],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::StealTargetAlly,
            ],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// An action modifier of the acting minion's.
const fn modifier(requires: &'static [Requirement], effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionDuring],
        actor: Actor::ActingMinion,
        requires,
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

const ANARCH: Requirement = Requirement::ActorSectIn(&[Sect::Anarch]);

/// "Requires an Anarch. [ani] This Anarch gets +1 strength this action.
/// [dom] This Anarch cannot be blocked by allies this action. [pro] This
/// Anarch's hand strikes are aggravated this action." Played, it requires
/// the Discipline used (LSJ 20011204-3).
pub static INVIGORATE: CardDef = CardDef {
    name: "Invigorate",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        modifier(&[ANARCH], &[Effect::StrengthThisAction(1)]),
        modifier(&[ANARCH], &[Effect::AlliesCannotBlock]),
        modifier(&[ANARCH], &[Effect::AggravatedHandsThisAction]),
    ],
    abilities: &[],
    riders: &[],
};

/// "Requires an Anarch. Put this card on an Anarch. Allies cannot block
/// this Anarch. Cards requiring Dominate [dom] or Presence [pre] cost other
/// minions +1 blood while this Anarch is acting, attempting to block or in
/// combat." A Methuselah's card requiring a sect wants a ready vampire of
/// it; the surcharge no longer applies once its block attempt failed, so
/// not to an action card paid at resolution (LSJ 20080818).
pub static LIBERTAS: CardDef = CardDef {
    name: "Libertas",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[Requirement::ControlsReadySect(&[Sect::Anarch])],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[Requirement::TargetVampire, ANARCH],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::PutOnTarget,
            Effect::Layer(Modifier::AlliesCannotBlockBearer),
            Effect::Layer(Modifier::DisciplineSurchargeAround(&["dom", "pre"], 1)),
        ],
    }],
    abilities: &[],
    riders: &[],
};

/// "[pre] Allies cannot block this action. [PRE] Only usable during a bleed
/// action. As above, with +2 bleed (limited)." (Requires a Ravnos.)
pub static VISIONS_OF_GEHENNA: CardDef = CardDef {
    name: "Visions of Gehenna",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        modifier(&[], &[Effect::AlliesCannotBlock]),
        modifier(
            &[Requirement::IsBleed],
            &[
                Effect::AlliesCannotBlock,
                Effect::BleedMod {
                    n: 2,
                    limited: true,
                },
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[dom] Ⓓ Bleed with +2 bleed. Allies cannot block this action. [DOM] Ⓓ
/// Put this card on a vampire controlled by your predator. Each time the
/// attached vampire announces an action, they move 1 blood from themselves
/// to this acting vampire. The attached vampire can burn 4 blood during
/// their minion phase to burn this card. Burn this card after this acting
/// vampire leaves the ready region." (2 blood.) The card remembers the
/// vampire that played it (`CardInPlay::by`); a target no longer the
/// predator's by resolution makes the action fizzle (LSJ 20020725).
pub static SLAUGHTERING_THE_HERD: CardDef = CardDef {
    name: "Slaughtering the Herd",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::BleedMod {
                    n: 2,
                    limited: false,
                },
                Effect::AlliesCannotBlock,
            ],
        },
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::TargetVampire,
                    Requirement::ControlledByPredator,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::DeclareAction(ActionKind::Card), Effect::PutOnTarget],
        },
    ],
    abilities: &[ModeDef {
        windows: &[Window::AnyImpulse],
        actor: Actor::Host,
        requires: &[
            Requirement::OwnMinionPhase,
            Requirement::HostBloodAtLeast(4),
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::HostBurnBlood(4), Effect::BurnSelf],
    }],
    riders: &[
        Rider {
            hook: Hook::HostAnnouncesAction,
            scope: RiderScope::Controller,
            requires: &[],
            effects: &[Effect::HostBloodToBy(1)],
        },
        Rider {
            hook: Hook::ByLeavesReady,
            scope: RiderScope::Controller,
            requires: &[],
            effects: &[Effect::BurnSelf],
        },
    ],
};

/// "Zombie with 6 life. 3 strength, 0 bleed. [obl] After this ally enters
/// play, burn it unless you remove an ally or vampire in your ash heap from
/// the game. This ally cannot gain life. This ally can enter combat with a
/// minion as a Ⓓ action that costs 1 life. [OBL] As above, and during the
/// first round of each combat, this ally can burn 1 life to get 1 press."
/// Its controller may decline the removal (LSJ 20030701-2); paying its last
/// life for the action, the action succeeds and the combat ends at once
/// (LSJ 20080512).
pub static ROTTING_BEHEMOTH: CardDef = CardDef {
    name: "Rotting Behemoth",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        recruit(&[
            Effect::DeclareAction(ActionKind::Recruit),
            Effect::EntryRemoveFromAshOrBurn,
            Effect::CannotGainLife,
        ]),
        recruit(&[
            Effect::DeclareAction(ActionKind::Recruit),
            Effect::EntryRemoveFromAshOrBurn,
            Effect::CannotGainLife,
        ]),
    ],
    abilities: &[
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::Host,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[Requirement::NotControlled, Requirement::ActorReady],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::PayLife(1),
                Effect::EnterCombatWithTarget,
            ],
        },
        ModeDef {
            windows: &[Window::CombatPress],
            actor: Actor::Host,
            requires: &[
                Requirement::SuperiorVersion,
                Requirement::FirstRound,
                Requirement::LifeAtLeast(1),
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::BurnOwnLife(1), Effect::Press],
        },
    ],
    riders: &[],
};

/// "Unique ghoul with 5 life. 4 strength, 0 bleed. After this ally enters
/// play, burn an ally or retainer you control. This ally can enter combat
/// with a vampire as a Ⓓ action. This ally can prevent 1 damage each round
/// of combat. During combat, this ally can burn 1 life to cancel a strike
/// card as it is played by the opposing minion, and its cost is not paid
/// (the minion chooses a strike again)." The canceled card is discarded
/// and replaced (spec §1.8.3).
pub static VOZHD_OF_GRAVESEND: CardDef = CardDef {
    name: "Vozhd of Gravesend, The",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[recruit(&[
        Effect::DeclareAction(ActionKind::Recruit),
        Effect::EntryBurnAllyOrRetainer,
    ])],
    abilities: &[
        enter_combat(&[
            Requirement::NotControlled,
            Requirement::ActorReady,
            Requirement::TargetVampire,
        ]),
        ability(
            &[Window::CombatDamagePrevention],
            Actor::Host,
            &[Requirement::DamagePending],
            Limit::PerRound,
            &[Effect::Prevent(1)],
        ),
        ModeDef {
            windows: &[Window::CardAsPlayed],
            actor: Actor::Host,
            requires: &[
                Requirement::PlayedStrikeByOpponent,
                Requirement::LifeAtLeast(1),
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::BurnOwnLife(1),
                Effect::CancelPlayed { refund: true },
            ],
        },
    ],
    riders: &[],
};

/// "Unique ghoul with 5 life. 4 strength, 0 bleed. After this ally enters
/// play, burn an ally or retainer you control. This ally can enter combat
/// with a vampire as a Ⓓ action. This ally can play cards requiring basic
/// Animalism [ani] as a vampire with capacity 5. During your unlock phase,
/// this ally can steal any amount of blood (becoming life) from a Tzimisce
/// you control, not to exceed its starting life." Played as a vampire, its
/// life pays, its last life burns it (ANK 20180928-1, LSJ 20080512); it
/// meets one Discipline requirement, at basic level only (LSJ 20011217,
/// ANK 20180111), and does not have the Discipline (LSJ 19980303).
pub static VOZHD_OF_JUIZ_DE_FORA: CardDef = CardDef {
    name: "Vozhd of Juiz de Fora, The",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[recruit(&[
        Effect::DeclareAction(ActionKind::Recruit),
        Effect::EntryBurnAllyOrRetainer,
        Effect::Layer(Modifier::PlaysAsVampire(AsVampire {
            disciplines: &["ani"],
            capacity: 5,
            non_action: false,
        })),
    ])],
    abilities: &[
        enter_combat(&[
            Requirement::NotControlled,
            Requirement::ActorReady,
            Requirement::TargetVampire,
        ]),
        ModeDef {
            windows: &[Window::UnlockPhaseOwn],
            actor: Actor::Host,
            requires: &[Requirement::HostBelowStartingLife],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::Controlled,
                    Requirement::ActorClan("Tzimisce"),
                    Requirement::BloodAtLeast(1),
                ],
            }),
            terms: Some(Terms::LifeFromTarget),
            waives_needed: false,
            limit: None,
            effects: &[Effect::StealBloodToHostX],
        },
    ],
    riders: &[],
};

/// "Wraith with 1 life. 1 strength, 1 bleed. [obl] This ally can perform
/// actions the turn it is recruited. Unlock this vampire if this is their
/// first successful recruit ally action this turn. This ally can play
/// non-action cards requiring basic Oblivion [obl] as a vampire. During
/// your unlock phase, burn this ally. [OBL] As above, but during your
/// unlock phase, you can burn 1 pool instead of burning this ally." It acts
/// only in its own Methuselah's minion phase (ANK 20200930); played as a
/// vampire, it counts as independent (LSJ 20020503).
pub static SPECTRAL_SERVITOR: CardDef = CardDef {
    name: "Spectral Servitor",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        recruit(&[
            Effect::DeclareAction(ActionKind::Recruit),
            Effect::ActsWhenRecruited,
            Effect::UnlockRecruiterOnFirst,
            Effect::Layer(Modifier::PlaysAsVampire(AsVampire {
                disciplines: &["obl"],
                capacity: 1,
                non_action: true,
            })),
        ]),
        recruit(&[
            Effect::DeclareAction(ActionKind::Recruit),
            Effect::ActsWhenRecruited,
            Effect::UnlockRecruiterOnFirst,
            Effect::Layer(Modifier::PlaysAsVampire(AsVampire {
                disciplines: &["obl"],
                capacity: 1,
                non_action: true,
            })),
        ]),
    ],
    abilities: &[],
    riders: &[
        Rider {
            hook: Hook::UnlockPhase,
            scope: RiderScope::Controller,
            requires: &[Requirement::BasicVersion],
            effects: &[Effect::BurnSelf],
        },
        Rider {
            hook: Hook::UnlockPhase,
            scope: RiderScope::Controller,
            requires: &[Requirement::SuperiorVersion],
            effects: &[Effect::BurnSelfUnlessPool(1)],
        },
    ],
};

/// "Animal with 1 life. [ani] While the employer is in combat, the opposing
/// minion's controller plays with an open hand. [ANI] As above, but Owl
/// Companion has 2 life." An open hand is information: the legal view reads
/// it (`GameState::hand_visible_to`).
pub static OWL_COMPANION: CardDef = CardDef {
    name: "Owl Companion",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        employ(&[
            Effect::DeclareAction(ActionKind::Employ),
            Effect::EmployRetainer { life: 1 },
            Effect::Layer(Modifier::OpponentOpenHandInCombat),
        ]),
        employ(&[
            Effect::DeclareAction(ActionKind::Employ),
            Effect::EmployRetainer { life: 2 },
            Effect::Layer(Modifier::OpponentOpenHandInCombat),
        ]),
    ],
    abilities: &[],
    riders: &[],
};

/// An employ retainer mode: the retainer enters play on the acting minion
/// with the life of its `EmployRetainer`, its layers giving the employer
/// what its text says.
const fn employ(effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionDeclareEmploy],
        actor: Actor::ActingCandidate,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// "The employer gets +1 intercept."
const EMPLOYER_INTERCEPT: Effect = Effect::Layer(Modifier::Bearer {
    stat: Stat::Intercept,
    n: 1,
    during: &[],
    who: Bearer::Any,
});

/// "Unique mortal with 1 life. The employer gets +1 intercept."
pub static MR_WINTHROP: CardDef = CardDef {
    name: "Mr. Winthrop",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[employ(&[
        Effect::DeclareAction(ActionKind::Employ),
        Effect::EmployRetainer { life: 1 },
        EMPLOYER_INTERCEPT,
    ])],
    abilities: &[],
    riders: &[],
};

/// "Animal with 1 life. [ani] Murder of Crows inflicts 1R damage on the
/// opposing minion each round of combat during normal strike resolution.
/// [ANI] As above, but Murder of Crows has 2 life." Environmental damage,
/// the retainer its source (ANK 20180612): inflicted against a dodge, not
/// once a combat ends strike ended the combat (RTR 19941109, TOM
/// 19951216), and not again for additional strikes (RTR 19941109).
pub static MURDER_OF_CROWS: CardDef = CardDef {
    name: "Murder of Crows",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        employ(&[
            Effect::DeclareAction(ActionKind::Employ),
            Effect::EmployRetainer { life: 1 },
            Effect::Layer(Modifier::RetainerDamage(1)),
        ]),
        employ(&[
            Effect::DeclareAction(ActionKind::Employ),
            Effect::EmployRetainer { life: 2 },
            Effect::Layer(Modifier::RetainerDamage(1)),
        ]),
    ],
    abilities: &[],
    riders: &[],
};

/// "Animal with 1 life. [ani] The employer gets +1 intercept. [ANI] As
/// above, but Raven Spy has 2 life." The Discipline level used to employ it
/// decides its version, for as long as it is in play (RTR 19941109, ANK
/// 20180612).
pub static RAVEN_SPY: CardDef = CardDef {
    name: "Raven Spy",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        employ(&[
            Effect::DeclareAction(ActionKind::Employ),
            Effect::EmployRetainer { life: 1 },
            EMPLOYER_INTERCEPT,
        ]),
        employ(&[
            Effect::DeclareAction(ActionKind::Employ),
            Effect::EmployRetainer { life: 2 },
            EMPLOYER_INTERCEPT,
        ]),
    ],
    abilities: &[],
    riders: &[],
};

/// "Ghoul with 2 life. The employer gets +1 intercept." Employed by a
/// Tzimisce (the card's clan requirement).
pub static REVENANT: CardDef = CardDef {
    name: "Revenant",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[employ(&[
        Effect::DeclareAction(ActionKind::Employ),
        Effect::EmployRetainer { life: 2 },
        EMPLOYER_INTERCEPT,
    ])],
    abilities: &[],
    riders: &[],
};

/// "Unique mortal with 1 life. 0 strength, 3 bleed." Recruited by a Ventrue
/// (the card's clan requirement); its type line is all its text.
pub static POLITICAL_ALLY: CardDef = CardDef {
    name: "Political Ally",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[recruit(&[Effect::DeclareAction(ActionKind::Recruit)])],
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
            // "+1 stealth and +1 bleed": the stealth is the effect's, so
            // it is playable only while stealth is needed — a block attempt
            // whose intercept matches the current stealth (TOM 19951109,
            // rules-qa 8.1).
            windows: &[Window::ActionDuring, Window::ActionStealthIntercept],
            actor: Actor::ActingMinion,
            requires: &[Requirement::IsBleed, Requirement::StealthNeeded],
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::CombatEndOfRound],
        actor: Actor::Combatant,
        requires: &[
            Requirement::ActorVampire,
            Requirement::OpponentVampire,
            Requirement::ActorReady,
        ],
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
            delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: Some(Until::NextUnlockPhase),
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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

pub static DEFS: [&CardDef; 140] = [
    &MAGIC_OF_THE_SMITH,
    &VAST_WEALTH,
    &BLACK_MARKET_CACHE,
    &SHILMULO_TAROT,
    &HEART_OF_NIZCHETUS,
    &KEVLAR_VEST,
    &RIGHTEOUS_BLADE,
    &MAGNUM_44,
    &AK_47,
    &ASSAULT_RIFLE,
    &FLAMETHROWER,
    &IVORY_BOW,
    &FEMUR_OF_TOOMLER,
    &KALIS_FANG,
    &SNIPER_RIFLE,
    &BOWL_OF_CONVERGENCE,
    &SPORT_BIKE,
    &CAMERA_PHONE,
    &LAPTOP_COMPUTER,
    &UNLICENSED_TAXICAB,
    &TREASURED_SAMADJI,
    &CODEX_OF_THE_EDENIC_GROUNDSKEEPERS,
    &STOLEN_POLICE_CRUISER,
    &ELDER_LIBRARY,
    &VISIT_FROM_THE_CAPUCHIN,
    &WAKE_WITH_EVENINGS_FRESHNESS,
    &MIRROR_WALK,
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
    &POLITICAL_ALLY,
    &FORTY_SEVENTH_STREET_ROYALS,
    &CITY_STAR_TAXI,
    &DOG_PACK,
    &DOUBLE_DEUCE,
    &FERAL_HOUND,
    &SZLACHTA_BODYGUARD,
    &VOZHD_OF_SOFIA,
    &WAR_GHOUL,
    &BONE_SHAMBLER,
    &CRYPTS_SONS,
    &SZLACHTA_ASSISTANT,
    &UNDERBRIDGE_STRAY,
    &VAGABOND_MYSTIC,
    &RAPTOR,
    &SCREAMER,
    &VOZHD_OF_SZCZECIN,
    &AGGRESSIVE_CORPSE,
    &ENTRANCEMENT,
    &FAR_MASTERY,
    &INVIGORATE,
    &LIBERTAS,
    &VISIONS_OF_GEHENNA,
    &SLAUGHTERING_THE_HERD,
    &ROTTING_BEHEMOTH,
    &VOZHD_OF_GRAVESEND,
    &VOZHD_OF_JUIZ_DE_FORA,
    &SPECTRAL_SERVITOR,
    &OWL_COMPANION,
    &MR_WINTHROP,
    &MURDER_OF_CROWS,
    &RAVEN_SPY,
    &REVENANT,
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
    &SMILING_JACK,
    &DREAMS_OF_THE_SPHINX,
    &THE_BARRENS,
    &THE_COVEN,
    &CONSTANT_REVOLUTION,
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
    delayed_replacement: None,
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
        requires: &[Requirement::ActorVampire],
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
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[
                Requirement::TargetVampire,
                Requirement::Controlled,
                Requirement::ActorReady,
            ],
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
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[Requirement::TargetVampire, Requirement::ActorReady],
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
    delayed_replacement: None,
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
            Requirement::HandHas(CardFilter::Type(CardType::Master), 2),
        ],
        target: NO_TARGET,
        terms: Some(Terms::Discard {
            n: 2,
            of: CardFilter::Type(CardType::Master),
        }),
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

/// "Unique. Put this card in play. During your unlock phase, move 1
/// counter from your pool to this card. During each other Methuselah's
/// unlock phase, for each counter on this card, that Methuselah burns 1
/// pool or burns 1 blood from a vampire they control. Vampires can burn
/// this card as a Ⓓ action." Two unlock riders (the controller's, the
/// others'); the payment is chosen unit by unit as the rider resolves
/// (`Flow::RiderTerms`); the burn is a granted directed action.
pub static SMILING_JACK: CardDef = CardDef {
    name: "Smiling Jack, The Anarch",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
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
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires: &[Requirement::ActorVampire],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::BurnSource,
        ],
    }],
    riders: &[
        Rider {
            hook: Hook::UnlockPhase,
            scope: RiderScope::Controller,
            requires: &[],
            effects: &[Effect::PoolToCounter(1)],
        },
        Rider {
            hook: Hook::UnlockPhase,
            scope: RiderScope::Others,
            requires: &[Requirement::CountersAtLeast(1)],
            effects: &[Effect::PoolOrBloodPerCounter],
        },
    ],
};

/// "Unique. Put this card in play. Add 1 counter to this card each time
/// you lock it. You can lock this card to get +2 hand size until the end
/// of the turn. You can lock this card during your unlock phase to gain 1
/// pool if you have the Edge. You can lock this card to add 1 blood to a
/// vampire in your uncontrolled region. Burn this card if it has 3
/// counters." Three lock abilities of the controller: two untimed (any
/// impulse but the as-played window, ANK 20200616), one in their own
/// unlock phase; each lock adds a counter, the third burns the card as
/// its effect resolves.
pub static DREAMS_OF_THE_SPHINX: CardDef = CardDef {
    name: "Dreams of the Sphinx",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
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
    abilities: &[
        ModeDef {
            windows: &[Window::AnyImpulse],
            actor: Actor::Controller,
            requires: &[Requirement::CardUnlocked],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::LockSelf,
                Effect::AddCounter(1),
                Effect::HandSizeBonusUntilEndOfTurn(2),
                Effect::BurnSelfAtCounters(3),
            ],
        },
        ModeDef {
            windows: &[Window::UnlockPhaseOwn],
            actor: Actor::Controller,
            requires: &[Requirement::CardUnlocked, Requirement::HasEdge],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::LockSelf,
                Effect::AddCounter(1),
                Effect::GainPool(1),
                Effect::BurnSelfAtCounters(3),
            ],
        },
        ModeDef {
            windows: &[Window::AnyImpulse],
            actor: Actor::Controller,
            requires: &[Requirement::CardUnlocked],
            target: Some(TargetSpec {
                kind: TargetKind::Uncontrolled,
                filter: &[],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::LockSelf,
                Effect::AddCounter(1),
                Effect::BloodToUncontrolledTarget(1),
                Effect::BurnSelfAtCounters(3),
            ],
        },
    ],
    riders: &[],
};

/// "Unique location. You can lock this card to discard a card (draw up
/// afterward)." An untimed lock ability of the controller (any impulse
/// but the as-played window: RTR 20040501, ANK 20200616); the card to
/// discard is announced with the use and replaced.
pub static THE_BARRENS: CardDef = CardDef {
    name: "Barrens, The",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
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
        windows: &[Window::AnyImpulse],
        actor: Actor::Controller,
        requires: &[Requirement::CardUnlocked, Requirement::HandNotEmpty],
        target: NO_TARGET,
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Any,
        }),
        waives_needed: false,
        limit: None,
        effects: &[Effect::LockSelf],
    }],
    riders: &[],
};

/// "Unique. Trifle. Put this card in play. You can lock this card to add 2
/// blood to a ready vampire you control. During your discard phase, your
/// predator takes control of The Coven." An untimed lock ability of the
/// controller (usable before a forced hunt, and in the prevention window
/// before damage lands — PIB 20121031: not midway through it, which the
/// engine's atomic damage never offers); the control change fires as the
/// controller passes out of their discard phase (LSJ 20100723), to the
/// predator of the controller at that time.
pub static THE_COVEN: CardDef = CardDef {
    name: "Coven, The",
    trifle: true,
    out_of_turn: false,
    delayed_replacement: None,
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
        windows: &[Window::AnyImpulse],
        actor: Actor::Controller,
        requires: &[Requirement::CardUnlocked],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[
                Requirement::TargetVampire,
                Requirement::Controlled,
                Requirement::ActorReady,
            ],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::LockSelf, Effect::BloodToTarget(2)],
    }],
    riders: &[Rider {
        hook: Hook::DiscardPhase,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::PredatorTakesControl],
    }],
};

/// "+1 stealth action. Requires an Anarch. Unique. Put this card in play
/// with 1 counter. During your unlock phase, add 1 counter to this card.
/// During each other Methuselah's unlock phase, that Methuselah burns X
/// pool and/or cards at random from their hand, where X is the number of
/// counters on this card. Vampires can burn this card as a Ⓓ action that
/// costs 1 pool." The others' rider is announced as a card count (the
/// rest pool; LSJ 20090618); the burn's pool is paid at resolution.
pub static CONSTANT_REVOLUTION: CardDef = CardDef {
    name: "Constant Revolution",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires: &[Requirement::ActorSectIn(&[Sect::Anarch])],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
            Effect::EnterWithCounters(1),
        ],
    }],
    abilities: &[ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires: &[Requirement::ActorVampire, Requirement::PoolAtLeast(1)],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::PayPool(1),
            Effect::BurnSource,
        ],
    }],
    riders: &[
        Rider {
            hook: Hook::UnlockPhase,
            scope: RiderScope::Controller,
            requires: &[],
            effects: &[Effect::AddCounter(1)],
        },
        Rider {
            hook: Hook::UnlockPhase,
            scope: RiderScope::Others,
            requires: &[Requirement::CountersAtLeast(1)],
            effects: &[Effect::PoolOrRandomCardsPerCounter],
        },
    ],
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
    delayed_replacement: None,
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
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[Requirement::TargetVampire, Requirement::Controlled],
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
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[Requirement::TargetVampire],
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
            delayed_replacement: None,
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
                    Requirement::TargetVampire,
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
    delayed_replacement: None,
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
    delayed_replacement: None,
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

// ---- Replacement and hand size (slice 7) ----------------------------------

/// "Unique location. +1 hand size." A hand-size layer of its controller,
/// suspended while the card is contested (ANK 20221028: a second copy
/// enters contest without modifying the hand size).
pub static ELDER_LIBRARY: CardDef = CardDef {
    name: "Elder Library",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::PutInPlay, Effect::Layer(Modifier::HandSize(1))],
    }],
    abilities: &[],
    riders: &[],
};

/// "Unique. Put this card in play with 4 counters. You get +1 hand size for
/// each counter on this card. Each time you would replace a card other
/// than this card, instead burn 1 counter from this card. Burn this card if
/// it has no counters." Its own replacement is drawn normally, up to the
/// new hand size; every later replacement — whatever took the card out of
/// hand (spec §6.8.3) — burns a counter instead, the hand size shrinking
/// with it (`Hook::WouldReplace`). A second copy of one's own is masked
/// (PIB 20121031: the self-contest rule, rules-qa 7.1).
pub static VISIT_FROM_THE_CAPUCHIN: CardDef = CardDef {
    name: "Visit from the Capuchin",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::PutInPlay,
            Effect::EnterWithCounters(4),
            Effect::Layer(Modifier::HandSizePerCounter),
        ],
    }],
    abilities: &[],
    riders: &[Rider {
        hook: Hook::WouldReplace,
        scope: RiderScope::Controller,
        requires: &[Requirement::CountersAtLeast(1)],
        effects: &[Effect::RemoveCounter(1), Effect::BurnSelfWithoutCounters],
    }],
};

/// "Only usable by a locked vampire. Do not replace until your next unlock
/// phase. This vampire wakes (they ignore the requirement to be unlocked
/// for playing reaction cards and attempting to block until the end of the
/// action)." The wake of Eyes of Argus [AUS], held back until the player's
/// next unlock phase.
pub static WAKE_WITH_EVENINGS_FRESHNESS: CardDef = CardDef {
    name: "Wake with Evening's Freshness",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: Some(Until::NextUnlockPhase),
    modes: &[ModeDef {
        windows: &[Window::Wake, Window::CardAsPlayed],
        actor: Actor::ReactingMinion,
        requires: &[Requirement::ActorVampire, Requirement::LockedActor],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::Wake],
    }],
    abilities: &[],
    riders: &[],
};

/// "Do not replace until your next discard phase. [tha] +1 stealth. [THA]
/// As above, and if this action is blocked, lock the blocking minion and
/// end the action before block resolution." The stealth only when needed
/// (rules-qa 3.2); the [THA] end is read where the block resolves: the
/// blocker locks, no combat, no diablerie opportunity on a blocked
/// leave-torpor (PIB 20121216), nothing more is played on the action (RTR
/// 20180719).
pub static MIRROR_WALK: CardDef = CardDef {
    name: "Mirror Walk",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: Some(Until::NextDiscardPhase),
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
            effects: &[Effect::AddStealth(1), Effect::EndActionIfBlocked],
        },
    ],
    abilities: &[],
    riders: &[],
};

// ---- Equipment (slice 8a) ---------------------------------------------------

/// The one vehicle a minion can have (the vehicles' shared clause).
const ONE_VEHICLE: &[Requirement] = &[Requirement::ActorWithout(Holding::Label(Label::Vehicle))];

/// "Vehicle. This minion gets +1 intercept. A minion can have only one
/// vehicle."
pub static SPORT_BIKE: CardDef = CardDef {
    name: "Sport Bike",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionDeclareEquip],
        actor: Actor::ActingCandidate,
        requires: ONE_VEHICLE,
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Intercept,
                n: 1,
                during: &[],
                who: Bearer::Any,
            }),
        ],
    }],
    abilities: &[],
    riders: &[],
};

/// "Vehicle. The action to equip with this vehicle is with an additional +1
/// stealth. This minion gets +1 stealth during hunt, recruit and employ
/// actions. If this minion is blocked by a prince or an archbishop (during
/// any action), burn this vehicle. A minion can have only one vehicle." The
/// equip stealth counts when it is taken from another minion too (LSJ
/// 20060222); the burn applies as the block resolves, before anything else
/// (ANK 20220116). No V5 vampire is an archbishop yet
/// (`TitleClass::Archbishop`).
pub static UNLICENSED_TAXICAB: CardDef = CardDef {
    name: "Unlicensed Taxicab",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionDeclareEquip],
        actor: Actor::ActingCandidate,
        requires: ONE_VEHICLE,
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::AddStealth(1),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Stealth,
                n: 1,
                during: &[ActionKind::Hunt, ActionKind::Recruit, ActionKind::Employ],
                who: Bearer::Any,
            }),
        ],
    }],
    abilities: &[],
    riders: &[Rider {
        hook: Hook::HostBlocked,
        scope: RiderScope::Every,
        requires: &[Requirement::BlockerTitleIn(&[
            TitleClass::Prince,
            TitleClass::Archbishop,
        ])],
        effects: &[Effect::BurnSelf],
    }],
};

/// "Unique. This Ravnos gets +1 bleed. Once each combat, this Ravnos can
/// strike: dodge." A minion that is not Ravnos may take it from another
/// minion (ANK 20180719-3) and gets nothing from it (rules-qa 9.1).
pub static TREASURED_SAMADJI: CardDef = CardDef {
    name: "Treasured Samadji",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionDeclareEquip],
        actor: Actor::ActingCandidate,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Bleed,
                n: 1,
                during: &[],
                who: Bearer::Clan("Ravnos"),
            }),
        ],
    }],
    abilities: &[ModeDef {
        windows: &[Window::CombatStrikeDeclaration],
        actor: Actor::Host,
        requires: &[Requirement::HostClan("Ravnos")],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: Some(Limit::PerCombat),
        effects: &[Effect::Strike(StrikeKind::Dodge)],
    }],
    riders: &[],
};

/// "Unique. The bearer gets -2 stealth during bleed actions. This vampire
/// can bleed as a Ⓓ action that costs 1 blood; this action gets +3 bleed if
/// the target Methuselah controls no ready unlocked minions." The granted
/// bleed is a bleed (one a turn) and this copy's action; its cost and the
/// +3 are read at resolution, and it fizzles if the Codex is no longer on
/// the acting vampire then (RTR 19960221).
pub static CODEX_OF_THE_EDENIC_GROUNDSKEEPERS: CardDef = CardDef {
    name: "Codex of the Edenic Groundskeepers",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionDeclareEquip],
        actor: Actor::ActingCandidate,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Stealth,
                n: -2,
                during: &[ActionKind::Bleed],
                who: Bearer::Any,
            }),
        ],
    }],
    abilities: &[ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::Host,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Bleed),
            Effect::PayBlood(1),
            Effect::BleedIfTargetHasNoReadyUnlocked(3),
        ],
    }],
    riders: &[],
};

/// "Electronic equipment. The minion with this equipment may bleed at +1
/// bleed as a Ⓓ action. No other equipment can increase the bleed amount
/// of this action." A legacy card the owner added to the pool (2026-09-25).
/// The granted bleed is a bleed (one a turn) and this copy's action; it
/// fizzles if the phone is no longer on the acting minion (RTR 19960221).
pub static CAMERA_PHONE: CardDef = CardDef {
    name: "Camera Phone",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionDeclareEquip],
        actor: Actor::ActingCandidate,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::DeclareAction(ActionKind::Equip)],
    }],
    abilities: &[ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::Host,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Bleed),
            Effect::BleedMod {
                n: 1,
                limited: false,
            },
            Effect::NoOtherEquipmentBleed,
        ],
    }],
    riders: &[],
};

/// "Electronic. The bearer gets +1 bleed. A minion can have only one Laptop
/// Computer." A legacy card the owner added to the pool (2026-09-25).
pub static LAPTOP_COMPUTER: CardDef = CardDef {
    name: "Laptop Computer",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionDeclareEquip],
        actor: Actor::ActingCandidate,
        requires: &[Requirement::ActorWithout(Holding::Named("Laptop Computer"))],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Bleed,
                n: 1,
                during: &[],
                who: Bearer::Any,
            }),
        ],
    }],
    abilities: &[],
    riders: &[],
};

/// "Vehicle. This Anarch gets +1 bleed. Allies and younger vampires get -1
/// intercept against this Anarch. Vampires can burn this card as a Ⓓ action
/// that costs 1 pool; if that action is successful, this Anarch is locked
/// and does not unlock as normal during their next unlock phase. A minion
/// can have only one vehicle." The Anarch clauses read the current bearer
/// (rules-qa 9.1); the burn is directed at the card's controller.
pub static STOLEN_POLICE_CRUISER: CardDef = CardDef {
    name: "Stolen Police Cruiser",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionDeclareEquip],
        actor: Actor::ActingCandidate,
        requires: ONE_VEHICLE,
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Bleed,
                n: 1,
                during: &[],
                who: Bearer::Sect(Sect::Anarch),
            }),
            Effect::Layer(Modifier::YoungerInterceptAgainstBearer {
                n: -1,
                who: Bearer::Sect(Sect::Anarch),
            }),
        ],
    }],
    abilities: &[ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires: &[Requirement::ActorVampire, Requirement::PoolAtLeast(1)],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::PayPool(1),
            Effect::BurnSource,
            Effect::LockHostSkipNextUnlock(Bearer::Sect(Sect::Anarch)),
        ],
    }],
    riders: &[],
};

/// "Unique. The bearer with Auspex [aus] gets +1 intercept. The bearer with
/// superior Auspex [AUS] can burn 1 blood during an action to get an
/// additional +1 intercept." The burn follows the "during X" template:
/// once per action (rules-qa 9.6), only while the bearer attempts to block
/// and the intercept is needed (rules-qa 3.2, 9.6).
pub static BOWL_OF_CONVERGENCE: CardDef = CardDef {
    name: "Bowl of Convergence",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionDeclareEquip],
        actor: Actor::ActingCandidate,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Intercept,
                n: 1,
                during: &[],
                who: Bearer::Discipline("aus"),
            }),
        ],
    }],
    abilities: &[ModeDef {
        windows: &[Window::ActionStealthIntercept],
        actor: Actor::Host,
        requires: &[
            Requirement::HostDiscipline("AUS"),
            Requirement::HostBloodAtLeast(1),
            Requirement::InterceptNeeded,
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: Some(Limit::PerAction),
        effects: &[Effect::HostBurnBlood(1), Effect::AddIntercept(1)],
    }],
    riders: &[],
};

// ---- Weapons (slice 8b) -----------------------------------------------------

/// The equip action of an equipment with nothing else on its mode.
const EQUIP: ModeDef = ModeDef {
    windows: &[Window::ActionDeclareEquip],
    actor: Actor::ActingCandidate,
    requires: &[],
    target: NO_TARGET,
    terms: None,
    waives_needed: false,
    limit: None,
    effects: &[Effect::DeclareAction(ActionKind::Equip)],
};

/// A weapon's strike, chosen by its bearer at the strike step.
const fn weapon_strike(requires: &'static [Requirement], effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::CombatStrikeDeclaration],
        actor: Actor::Host,
        requires,
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// "With 1 optional maneuver each combat": the allowance tracks the weapon
/// (LSJ 19980302-2), and using it chooses the bearer's initial strike
/// (rulebook "Determine Range"; LSJ 20090528-2-2).
const GUN_MANEUVER: ModeDef = ModeDef {
    windows: &[Window::CombatDetermineRange],
    actor: Actor::Host,
    requires: &[],
    target: NO_TARGET,
    terms: None,
    waives_needed: false,
    limit: Some(Limit::PerCombat),
    effects: &[Effect::Maneuver, Effect::CommitStrike],
};

/// "Weapon: gun. Strike: 2R damage, with 1 optional maneuver each combat."
pub static MAGNUM_44: CardDef = CardDef {
    name: ".44 Magnum",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[EQUIP],
    abilities: &[
        weapon_strike(
            &[],
            &[Effect::Strike(StrikeKind::Damage {
                n: 2,
                ranged: true,
                aggravated: false,
            })],
        ),
        GUN_MANEUVER,
    ],
    riders: &[],
};

/// "Weapon: gun. Strike: 2R damage, with 1 optional maneuver each combat.
/// After the bearer strikes with this gun, they get 1 optional additional
/// strike (limited), only usable to strike with this gun, this round."
pub static AK_47: CardDef = CardDef {
    name: "AK-47",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[EQUIP],
    abilities: &[
        weapon_strike(
            &[],
            &[
                Effect::Strike(StrikeKind::Damage {
                    n: 2,
                    ranged: true,
                    aggravated: false,
                }),
                Effect::AdditionalStrikeWithSource,
            ],
        ),
        GUN_MANEUVER,
    ],
    riders: &[],
};

/// "Weapon: gun. Strike: 4R damage, with 1 optional maneuver each combat."
pub static ASSAULT_RIFLE: CardDef = CardDef {
    name: "Assault Rifle",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[EQUIP],
    abilities: &[
        weapon_strike(
            &[],
            &[Effect::Strike(StrikeKind::Damage {
                n: 4,
                ranged: true,
                aggravated: false,
            })],
        ),
        GUN_MANEUVER,
    ],
    riders: &[],
};

/// "Weapon. Strike: 2R aggravated damage."
pub static FLAMETHROWER: CardDef = CardDef {
    name: "Flamethrower",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[EQUIP],
    abilities: &[weapon_strike(
        &[],
        &[Effect::Strike(StrikeKind::Damage {
            n: 2,
            ranged: true,
            aggravated: true,
        })],
    )],
    riders: &[],
};

/// "Unique weapon. Strike: 1R aggravated damage."
pub static IVORY_BOW: CardDef = CardDef {
    name: "Ivory Bow",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[EQUIP],
    abilities: &[weapon_strike(
        &[],
        &[Effect::Strike(StrikeKind::Damage {
            n: 1,
            ranged: true,
            aggravated: true,
        })],
    )],
    riders: &[],
};

/// "Unique melee weapon. Strike: strength+1 aggravated damage." (Tzimisce.)
/// Equipping from hand needs a Tzimisce; taken from another minion it
/// needs none (ANK 20180719-3), and the strike names no clan.
pub static FEMUR_OF_TOOMLER: CardDef = CardDef {
    name: "Femur of Toomler",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[EQUIP],
    abilities: &[weapon_strike(
        &[],
        &[Effect::Strike(StrikeKind::Melee {
            bonus: 1,
            aggravated: true,
        })],
    )],
    riders: &[],
};

/// "Unique melee weapon. Strike: strength+1 aggravated damage." (Banu
/// Haqim; as Femur of Toomler.)
pub static KALIS_FANG: CardDef = CardDef {
    name: "Kali's Fang",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[EQUIP],
    abilities: &[weapon_strike(
        &[],
        &[Effect::Strike(StrikeKind::Melee {
            bonus: 1,
            aggravated: true,
        })],
    )],
    riders: &[],
};

/// "Weapon: gun. Strike: 2R damage, only usable at long range. If the
/// bearer blocks, they can, before range is determined, set the range for
/// the first round of the resulting combat to long, and their initial
/// strike that round must be with this weapon." The range set skips the
/// determine range step and no other effect resets it (RTR 19970630, ANK
/// 20180720); every v1 combat arises from its block, so the blocker's
/// first round is the one.
pub static SNIPER_RIFLE: CardDef = CardDef {
    name: "Sniper Rifle",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[EQUIP],
    abilities: &[
        weapon_strike(
            &[Requirement::RangeLong],
            &[Effect::Strike(StrikeKind::Damage {
                n: 2,
                ranged: true,
                aggravated: false,
            })],
        ),
        ModeDef {
            windows: &[Window::CombatBeforeRange],
            actor: Actor::Host,
            requires: &[
                Requirement::ActorHasBlocked,
                Requirement::FirstRound,
                Requirement::SourceHeldWhenBlocking,
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::SetRangeLong, Effect::CommitStrike],
        },
    ],
    riders: &[],
};

/// "Melee weapon. Strike: strength+1 damage, with 1 optional press, only
/// usable to continue combat, each combat." (Salubri.) The press is had as
/// the strike is chosen, kept though the strike is dodged or does not
/// resolve, once each combat per Blade (rules-qa 10.4).
pub static RIGHTEOUS_BLADE: CardDef = CardDef {
    name: "Righteous Blade",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[EQUIP],
    abilities: &[weapon_strike(
        &[],
        &[
            Effect::Strike(StrikeKind::Melee {
                bonus: 1,
                aggravated: false,
            }),
            Effect::ContinuePressEachCombat,
        ],
    )],
    riders: &[],
};

/// "Once each combat, the bearer can prevent 2 damage from gun strikes or 1
/// damage from any other source. A minion can have only one Kevlar Vest."
/// The branch is chosen once each combat; the gun points may be spent over
/// several strikes of the combat, only against gun-strike damage; a gun's
/// damage other than its strike is not "from gun strikes" (rules-qa 10.3;
/// LSJ 20081210).
pub static KEVLAR_VEST: CardDef = CardDef {
    name: "Kevlar Vest",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionDeclareEquip],
        actor: Actor::ActingCandidate,
        requires: &[Requirement::ActorWithout(Holding::Named("Kevlar Vest"))],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::DeclareAction(ActionKind::Equip)],
    }],
    abilities: &[
        ModeDef {
            windows: &[Window::CombatDamagePrevention],
            actor: Actor::Host,
            requires: &[Requirement::GunStrikeDamagePending],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerCombat),
            effects: &[Effect::PreventGunStrikeAllowance(2)],
        },
        ModeDef {
            windows: &[Window::CombatDamagePrevention],
            actor: Actor::Host,
            requires: &[Requirement::NonGunDamagePending],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerCombat),
            effects: &[Effect::PreventNonGun(1)],
        },
    ],
    riders: &[],
};

// ---- Library search (slice 8c) ----------------------------------------------

/// "+1 stealth action. [tha] Search your library for an equipment card and
/// equip this vampire with it (requirements and cost apply as normal;
/// shuffle afterward). [THA] As above, but this is a +3 stealth action."
/// (1 blood.) The equipment is chosen at resolution ([RBK search]); one
/// whose blood cost the vampire cannot pay is not offered (rules-qa 9.3).
pub static MAGIC_OF_THE_SMITH: CardDef = CardDef {
    name: "Magic of the Smith",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::SearchEquip,
            ],
        },
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(3),
                Effect::SearchEquip,
            ],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "Put this card on a ready minion you control. If you control this
/// minion, they can equip with the first equipment you find in your library
/// as a +1 stealth equip action (working down from the top; requirements
/// and cost apply as normal; shuffle afterward)."
pub static VAST_WEALTH: CardDef = CardDef {
    name: "Vast Wealth",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
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
        effects: &[Effect::PutOnTarget],
    }],
    abilities: &[ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::Host,
        requires: &[Requirement::ControllerControlsHost],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::EquipFound),
            Effect::AddStealth(1),
            Effect::EquipFirstFound,
        ],
    }],
    riders: &[],
};

// ---- Stored cards and draw alternatives (slice 8c-ii) ----------------------

/// "Unique location. Search your library (shuffle afterward) for up to
/// three non-unique equipment cards and put them on this location, face up
/// and out of play. If you would draw a card from your library, you can
/// draw one of those cards instead. If this location has no cards on it,
/// burn it." (Ravnos.) The cards are chosen one at a time as it resolves,
/// once its replacement is drawn (rules-qa 12.4), before it enters contest
/// (spec §1.13.2); it is burned at once holding
/// none, and as its last card is drawn (rules-qa 11.3). Each card drawn
/// from the library may be one of them instead (rules-qa 9.4).
pub static BLACK_MARKET_CACHE: CardDef = CardDef {
    name: "Black Market Cache",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::PutInPlay, Effect::SearchStore(3)],
    }],
    abilities: &[],
    riders: &[Rider {
        hook: Hook::WouldDraw,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::BurnSelfWithoutStored],
    }],
};

/// "Unique. Move the top 2 cards of your library to this equipment, face
/// down and out of play (you can look at the cards at any time). While this
/// Ravnos is ready, if you would draw a card from your library, you can
/// draw one of these cards instead. During your unlock phase, you can move
/// the top card of your library to this equipment." (Ravnos.) The two cards
/// come as it is equipped — not when it is taken by another minion — and
/// before a second copy enters contest (ANK 20221028); it stores any number
/// (PIB 20110712-2). A minion that is not Ravnos may take it from another
/// minion (ANK 20180719-3) and draws nothing from it: "this Ravnos" is read
/// on the current bearer, as Treasured Samadji's (rules-qa 9.1). The unlock
/// move is once per unlock phase, the bearer ready or not (rules-qa 6.6,
/// 11.2).
pub static SHILMULO_TAROT: CardDef = CardDef {
    name: "Shilmulo Tarot",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        effects: &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::StoreTopCards(2),
        ],
        ..EQUIP
    }],
    abilities: &[ModeDef {
        windows: &[Window::UnlockPhaseOwn],
        actor: Actor::Controller,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::StoreTopCards(1)],
    }],
    riders: &[Rider {
        hook: Hook::WouldDraw,
        scope: RiderScope::Controller,
        requires: &[Requirement::HostClan("Ravnos"), Requirement::HostReady],
        effects: &[],
    }],
};

/// "Unique. If the bearer is ready during your unlock phase, you can draw
/// up to 3 cards without discarding and then move the same number of cards
/// from your hand to the bottom of your library." The number is picked as
/// the ability is used and capped by the library as it resolves; the cards
/// moved back are chosen one at a time, the last one chosen the bottom
/// card (rules-qa 9.5, 11.1).
pub static HEART_OF_NIZCHETUS: CardDef = CardDef {
    name: "Heart of Nizchetus",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[EQUIP],
    abilities: &[ModeDef {
        windows: &[Window::UnlockPhaseOwn],
        actor: Actor::Host,
        requires: &[Requirement::HostReady],
        target: NO_TARGET,
        terms: Some(Terms::Count { min: 1, max: 3 }),
        waives_needed: false,
        limit: None,
        effects: &[Effect::DrawThenBottom],
    }],
    riders: &[],
};
