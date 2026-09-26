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

use super::vampires::{ability, enter_combat, printed_action, MANEUVER_EACH_COMBAT};
use super::{
    Actor, Among, AsVampire, Bearer, BlockRider, CardDef, CardFilter, Effect, Holding, Hook, Limit,
    ModeDef, Modifier, On, Points, RefEffect, ReferendumDef, Requirement, Rider, RiderScope,
    SeatFilter, Stat, StrikeKind, TargetKind, TargetSpec, Term, Terms, TitleClass, Until,
    VoteGrant, Voters, Window,
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
    grants: &[],
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
    grants: &[],
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
    grants: &[],
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
    grants: &[],
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
    grants: &[],
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
    grants: &[],
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
    grants: &[],
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
    grants: &[],
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
    grants: &[],
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
    grants: &[],
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
    grants: &[],
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
    grants: &[],
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
/// passed. For each vote by which the referendum passed, distribute 1 blood
/// from the blood bank among the ready Lasombra you control and your pool.
/// A vampire cannot gain more than 1 blood this way. You cannot gain more
/// than 1 pool this way or more than 2 pool this way if this acting
/// Lasombra is titled." (Lasombra.) The distribution is chosen as it
/// resolves, pool first; what finds no room is lost. A referendum passing
/// with no polling step passes by 0 (LSJ 19980107, PIB 20150105).
pub static AMICI_NOCTIS: CardDef = CardDef {
    name: "Amici Noctis",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionAfterResolution],
        actor: Actor::ActingMinion,
        requires: &[Requirement::IsPolitical, Requirement::ReferendumPassed],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::DistributeMargin {
            clan: "Lasombra",
            pool: 1,
            titled_pool: 2,
        }],
    }],
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

pub static DEFS: [&CardDef; 292] = [
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
    &GOVERN_THE_UNALIGNED,
    &ENCHANT_KINDRED,
    &INTIMIDATION,
    &THING,
    &HEART_OF_THE_CITY,
    &LOST_IN_CROWDS,
    &SWALLOWED_BY_THE_NIGHT,
    &RESIST_EARTHS_GRASP,
    &AIRE_OF_ELATION,
    &MONKEY_WRENCH,
    &TELEPATHIC_MISDIRECTION,
    &BAIT_AND_SWITCH,
    &MY_ENEMYS_ENEMY,
    &ENHANCED_SENSES,
    &PROTECTION_RACKET,
    &THE_WARRENS,
    &EARTH_MELD,
    &PURSUIT,
    &APPORTATION,
    &ARMS_OF_AHRIMAN,
    &WALK_OF_FLAME,
    &QUICKNESS,
    &REGO_MOTUM,
    &LIFE_IN_THE_CITY,
    &MISDIRECTION,
    &INFORMATION_HIGHWAY,
    &THE_LABYRINTH,
    &ELYSIAN_FIELDS,
    &BACKWAYS,
    &OPIUM_DEN,
    &MARKET_SQUARE,
    &FORESHADOWING_DESTRUCTION,
    &ON_THE_QUI_VIVE,
    &SHADOW_SENTINEL,
    &GUARD_DOGS,
    &SPIRITS_TOUCH,
    &INSTINCTIVE_REACTION,
    &PRECOGNITION,
    &SECOND_TRADITION_DOMAIN,
    &DEEP_ECOLOGY,
    &ONE_WITH_THE_LAND,
    &FORM_OF_THE_BAT,
    &TRUTH_IN_DARKNESS,
    &ORGANIZED_RESISTANCE,
    &EYES_OF_THE_WILD,
    &NIGHT_TERRORS,
    &STYGIAN_SHROUD,
    &FORCED_CONFESSIONAL,
    &TRUE_LOVES_FACE,
    &WHERE_THE_VEIL_THINS,
    &FACELESS_NIGHT,
    &DIRECT_INTERVENTION,
    &WASH,
    &TOREADOR_JUSTICAR,
    &SAULOTS_GUIDING_WISDOM,
    &REBEL,
    &ELYSIUM_THE_PALACE_OF_VERSAILLES,
    &POWER_STRUCTURE,
    &VENTRUE_HEADQUARTERS,
    &BLOOD_RAGE,
    &SOUL_BURN,
    &CARRION_CROWS,
    &HUNGER_OF_MARDUK,
    &PERFECTIONIST,
    &SAULOTS_AVENGING_FIST,
    &SAULOTS_HEALING_TOUCH,
    &AMICI_NOCTIS,
    &BEARS_SKIN,
    &DUST_UP,
    &FORM_OF_THE_WOLF,
    &ROLLING_WITH_THE_PUNCHES,
    &SCULPT_THE_FLESH,
    &TOUCH_OF_OBLIVION,
    &PASS_THROUGH_SHADOW,
    &SLAM,
    &AID_FROM_BATS,
    &PACK_ALPHA,
    &VORACIOUS_VERMIN,
    &WEIGHTED_WALKING_STICK,
    &CLOAK_THE_GATHERING,
    &VEIL_THE_LEGIONS,
    &DARING_THE_DAWN,
    &FREAK_DRIVE,
    &SEDUCTION,
    &THE_SLEEPING_MIND,
    &UNLEASHING_THE_BESTIAL_SOUL,
    &VISIONS_OF_ZAPATHASURA,
    &CHANGE_OF_TARGET,
    &SHADOW_CLOAK,
    &SPYING_MISSION,
    &SHADOW_CAST,
    &PLATINUM_PROTOCOL,
    &BREAK_THE_BONDS,
    &ENCHANTING_GAZE,
    &REVELATION_OF_THE_SERPENT,
    &EAGLES_SIGHT,
    &FEAST_OF_THE_SOULS_SECRETS,
    &FLEETNESS,
    &HUNTERS_MARK,
    &LINE_BRAWL,
    &SHOW_OF_FORCE,
    &SHROUD_OF_DECAY,
    &REWILDING,
    &PRETERNATURAL_STRENGTH,
    &HEROIC_MIGHT,
    &RUTORS_HAND,
    &MONSTROUS_FORM,
    &TOUCH_OF_VALEREN,
    &BIOTHAUMATURGIC_EXPERIMENT,
    &REVELATIONS,
    &OBEDIENT_FLESH,
    &GATE_OF_ACHERON,
    &PIT_OF_CONTEMPLATION,
    &DEEP_SONG,
    &PUPPET_MASTER,
    &CELERITY,
    &DOMINATE,
    &OBFUSCATE,
    &OBLIVION,
    &POTENCE,
    &PROTEAN,
    &CHILDE_OF_THE_REVOLUTION,
    &ARCANE_LIBRARY,
    &ART_MUSEUM,
    &ECOTERRORISTS,
    &WASSERSCHLOSS_ANIF,
    &WIDER_VIEW,
    &FAMILY_GATHERING,
    &POWERBASE_MONTREAL,
    &THE_RACK,
    &ANARCH_TROUBLEMAKER,
    &POWERBASE_LOS_ANGELES,
    &GARIBALDI_MEUCCI_MUSEUM,
    &ANARCH_FREE_PRESS,
    &CHANNEL_10,
    &CREEPSHOW_CASINO,
    &CLUB_ILLUSION,
    &HAQIMS_LAW_RETRIBUTION,
    &PENTEX_SUBVERSION,
    &HAVEN_UNCOVERED,
    &CHANTRY,
    &TOREADOR_GRAND_BALL,
    &PRIORITY_CONTRACT,
    &FRONTAL_ASSAULT,
    &PIPER,
    &WARSAW_STATION,
    &ANARCH_RAILROAD,
    &FORTUNE_TELLER_SHOP,
    &KRCG_NEWS_RADIO,
    &CARVERS_MEAT_PACKING,
    &MEDITATIVE_GROVE,
    &WEEK_OF_NIGHTMARES,
];

// ---- Cards in play with riders and granted actions (slice 5) ------------

static BURN_THIS_CARD: ReferendumDef = ReferendumDef {
    terms: &[],
    effect: RefEffect::BurnSource,
    grants: &[],
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

// ---- Precon coverage (slice 11a) --------------------------------------------

/// "Ⓓ Bleed with +n bleed": an action card's bleed mode.
macro_rules! bleed_action {
    ($n:expr) => {
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
                    n: $n,
                    limited: false,
                },
            ],
        }
    };
}

/// "+n stealth": an action modifier of the acting minion's, when stealth
/// is needed.
const fn stealth(effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionStealthIntercept],
        actor: Actor::ActingMinion,
        requires: &[Requirement::StealthNeeded],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// "+n intercept": a reaction of a minion attempting to block, when
/// intercept is needed.
const fn intercept(requires: &'static [Requirement], effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionStealthIntercept],
        actor: Actor::ReactingMinion,
        requires,
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// A combat card's mode for the combatant playing it.
const fn combat(
    windows: &'static [Window],
    requires: &'static [Requirement],
    effects: &'static [Effect],
) -> ModeDef {
    ModeDef {
        windows,
        actor: Actor::Combatant,
        requires,
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// "+1 stealth action. Add n blood to a vampire in your uncontrolled
/// region" matching `filter`: the vampire is chosen as the card is played
/// (ANK 20240628-2) and re-checked at resolution.
macro_rules! uncontrolled_blood {
    ($filter:expr, $n:expr) => {
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Uncontrolled,
                filter: $filter,
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::BloodToUncontrolledTarget($n),
            ],
        }
    };
}

/// "[dom] Ⓓ Bleed with +2 bleed. [DOM] +1 stealth action. Add 3 blood to
/// a younger vampire in your uncontrolled region."
pub static GOVERN_THE_UNALIGNED: CardDef = CardDef {
    name: "Govern the Unaligned",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        bleed_action!(2),
        uncontrolled_blood!(&[Requirement::Younger], 3),
    ],
    abilities: &[],
    riders: &[],
};

/// "[pre] Ⓓ Bleed with +1 bleed. [PRE] +1 stealth action. Add 2 blood to
/// a younger vampire in your uncontrolled region."
pub static ENCHANT_KINDRED: CardDef = CardDef {
    name: "Enchant Kindred",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        bleed_action!(1),
        uncontrolled_blood!(&[Requirement::Younger], 2),
    ],
    abilities: &[],
    riders: &[],
};

/// "[pre] Ⓓ Bleed with +1 bleed. [PRE] Ⓓ Bleed with +2 bleed."
pub static INTIMIDATION: CardDef = CardDef {
    name: "Intimidation",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[bleed_action!(1), bleed_action!(2)],
    abilities: &[],
    riders: &[],
};

/// "+1 stealth action. Add 2 blood to a Gangrel in your uncontrolled
/// region."
pub static THING: CardDef = CardDef {
    name: "Thing",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[uncontrolled_blood!(
        &[Requirement::TargetClan("Gangrel")],
        2
    )],
    abilities: &[],
    riders: &[],
};

/// "+1 stealth action. [pre] Put this card on this vampire. This vampire
/// gets +1 bleed. A vampire can have only one Heart of the City. [PRE] As
/// above, but this vampire gets +2 bleed." (2 blood.)
pub static HEART_OF_THE_CITY: CardDef = CardDef {
    name: "Heart of the City",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[Requirement::ActorWithout(Holding::Named(
                "Heart of the City",
            ))],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::PutOnActor,
                Effect::Layer(Modifier::Bearer {
                    stat: Stat::Bleed,
                    n: 1,
                    during: &[],
                    who: Bearer::Any,
                }),
            ],
        },
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[Requirement::ActorWithout(Holding::Named(
                "Heart of the City",
            ))],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::PutOnActor,
                Effect::Layer(Modifier::Bearer {
                    stat: Stat::Bleed,
                    n: 2,
                    during: &[],
                    who: Bearer::Any,
                }),
            ],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "[obf] +1 stealth. Lost in Crowds and cards named Into Thin Air cannot
/// both be played during the same action. [OBF] As above, but for +2
/// stealth." (Into Thin Air is not in the V5 pool.)
pub static LOST_IN_CROWDS: CardDef = CardDef {
    name: "Lost in Crowds",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        stealth(&[Effect::AddStealth(2)]),
    ],
    abilities: &[],
    riders: &[],
};

/// "[obf] [ACTION MODIFIER] +1 stealth. [OBF] [COMBAT] Maneuver."
pub static SWALLOWED_BY_THE_NIGHT: CardDef = CardDef {
    name: "Swallowed by the Night",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        combat(&[Window::CombatDetermineRange], &[], &[Effect::Maneuver]),
    ],
    abilities: &[],
    riders: &[],
};

/// "[cel] [COMBAT] Press, or maneuver with 1 optional press. [CEL] [ACTION
/// MODIFIER] +1 stealth." (1 blood.) The optional press is for the round
/// it is had in (TOM 19960521).
pub static RESIST_EARTHS_GRASP: CardDef = CardDef {
    name: "Resist Earth's Grasp",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatDetermineRange, Window::CombatPress],
            &[],
            &[Effect::Maneuver, Effect::OptionalPress, Effect::Press],
        ),
        stealth(&[Effect::AddStealth(1)]),
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable during a bleed action. [pre] +1 bleed, with an additional
/// +1 bleed if this vampire is Toreador (limited). [PRE] +2 bleed, with an
/// additional +1 bleed if this vampire is Toreador (limited)." (1 blood.)
pub static AIRE_OF_ELATION: CardDef = CardDef {
    name: "Aire of Elation",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        modifier(
            &[Requirement::IsBleed],
            &[
                Effect::BleedMod {
                    n: 1,
                    limited: true,
                },
                Effect::If {
                    on: On::Actor,
                    requires: &[Requirement::ActorClan("Toreador")],
                    then: &Effect::BleedMod {
                        n: 1,
                        limited: true,
                    },
                },
            ],
        ),
        modifier(
            &[Requirement::IsBleed],
            &[
                Effect::BleedMod {
                    n: 2,
                    limited: true,
                },
                Effect::If {
                    on: On::Actor,
                    requires: &[Requirement::ActorClan("Toreador")],
                    then: &Effect::BleedMod {
                        n: 1,
                        limited: true,
                    },
                },
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable during a bleed action. [dom] +1 bleed (limited). [DOM] +3
/// bleed if the target Methuselah has 9 or fewer pool (limited)." The
/// condition is read live on the bleed's current target: played against 10
/// or more pool it lingers and does not count against the limit (PIB
/// 20110817); lost to a change of target, it no longer counts and another
/// limited increase may be played (RTR 20191031, ANK 20211019-2).
pub static FORESHADOWING_DESTRUCTION: CardDef = CardDef {
    name: "Foreshadowing Destruction",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        modifier(
            &[Requirement::IsBleed],
            &[Effect::BleedMod {
                n: 1,
                limited: true,
            }],
        ),
        modifier(
            &[Requirement::IsBleed],
            &[Effect::If {
                on: On::Acting,
                requires: &[Requirement::BleedTargetPoolAtMost(9)],
                then: &Effect::BleedMod {
                    n: 3,
                    limited: true,
                },
            }],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Requires an Anarch. Only usable during a bleed action. +X bleed
/// (limited). X must be 1, 2 or 3." (X blood.)
pub static MONKEY_WRENCH: CardDef = CardDef {
    name: "Monkey Wrench",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionDuring],
        actor: Actor::ActingMinion,
        requires: &[ANARCH, Requirement::IsBleed],
        target: NO_TARGET,
        terms: Some(Terms::XBloodIn { min: 1, max: 3 }),
        waives_needed: false,
        limit: None,
        effects: &[Effect::BleedModX { limited: true }],
    }],
    abilities: &[],
    riders: &[],
};

/// "Change the target of the bleed to another Methuselah other than the
/// acting minion's controller (that Methuselah can attempt to block)",
/// locking the reacting vampire: Deflection's shape.
const REDIRECT_TARGET: Option<TargetSpec> = Some(TargetSpec {
    kind: TargetKind::Seat,
    filter: &[Requirement::NotActingController, Requirement::NotSelf],
});

/// "[aus] +1 intercept. [AUS] Only usable if a minion is bleeding you,
/// after blocks are declined. Lock this reacting vampire. Change the target
/// of the bleed to another Methuselah other than the acting minion's
/// controller (that Methuselah can attempt to block)." (1 blood.)
pub static TELEPATHIC_MISDIRECTION: CardDef = CardDef {
    name: "Telepathic Misdirection",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(1)]),
        ModeDef {
            windows: &[Window::ActionBlocksDeclined],
            actor: Actor::ReactingMinion,
            requires: &[
                Requirement::DirectedAtYou,
                Requirement::IsBleed,
                Requirement::BlocksDeclined,
            ],
            target: REDIRECT_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockActor, Effect::RedirectBleed],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "Requires a baron. Only usable if a minion is bleeding you, after
/// blocks are declined. Lock this reacting vampire. Change the target of
/// the bleed to another Methuselah other than the acting minion's
/// controller (that Methuselah can attempt to block)."
pub static BAIT_AND_SWITCH: CardDef = CardDef {
    name: "Bait and Switch",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionBlocksDeclined],
        actor: Actor::ReactingMinion,
        requires: &[
            Requirement::ActorTitleIn(&[TitleClass::Baron]),
            Requirement::DirectedAtYou,
            Requirement::IsBleed,
            Requirement::BlocksDeclined,
        ],
        target: REDIRECT_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::LockActor, Effect::RedirectBleed],
    }],
    abilities: &[],
    riders: &[],
};

/// "[aus] +1 intercept. [AUS] Only usable if a minion controlled by your
/// predator is bleeding you, after blocks are declined. Not usable if
/// there are only two Methuselahs left. Lock this reacting vampire. Change
/// the target of the bleed to your predator's predator (that Methuselah can
/// attempt to block)."
pub static MY_ENEMYS_ENEMY: CardDef = CardDef {
    name: "My Enemy's Enemy",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(1)]),
        ModeDef {
            windows: &[Window::ActionBlocksDeclined],
            actor: Actor::ReactingMinion,
            requires: &[
                Requirement::DirectedAtYou,
                Requirement::IsBleed,
                Requirement::BlocksDeclined,
                Requirement::ActingByPredator,
                Requirement::MoreThanTwoLeft,
            ],
            target: Some(TargetSpec {
                kind: TargetKind::Seat,
                filter: &[Requirement::PredatorsPredator],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockActor, Effect::RedirectBleed],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "[aus] +1 intercept. [AUS] +2 intercept."
pub static ENHANCED_SENSES: CardDef = CardDef {
    name: "Enhanced Senses",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(1)]),
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(2)]),
    ],
    abilities: &[],
    riders: &[],
};

/// "Requires an Anarch. +1 intercept, with an additional +1 intercept if
/// the acting minion is titled." The title is read live for the rest of
/// the action (ANK 20190416).
pub static PROTECTION_RACKET: CardDef = CardDef {
    name: "Protection Racket",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[intercept(
        &[ANARCH, Requirement::InterceptNeeded],
        &[
            Effect::AddIntercept(1),
            Effect::If {
                on: On::Acting,
                requires: &[Requirement::ActorTitleIn(&[TitleClass::Any])],
                then: &Effect::AddIntercept(1),
            },
        ],
    )],
    abilities: &[],
    riders: &[],
};

/// "Only usable during an action directed at you (or a card you control).
/// +2 intercept, with an additional +1 intercept if this Nosferatu is
/// titled." (Nosferatu, 1 blood.)
pub static THE_WARRENS: CardDef = CardDef {
    name: "Warrens, The",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[intercept(
        &[Requirement::DirectedAtYou, Requirement::InterceptNeeded],
        &[
            Effect::AddIntercept(2),
            Effect::If {
                on: On::Actor,
                requires: &[Requirement::ActorTitleIn(&[TitleClass::Any])],
                then: &Effect::AddIntercept(1),
            },
        ],
    )],
    abilities: &[],
    riders: &[],
};

/// "[pro] Strike: combat ends. [PRO] As above, and unlock this vampire
/// before combat ends."
pub static EARTH_MELD: CardDef = CardDef {
    name: "Earth Meld",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[Effect::Strike(StrikeKind::CombatEnds { unlock: false })],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[Effect::Strike(StrikeKind::CombatEnds { unlock: true })],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[cel] Maneuver. [CEL] Additional strike (limited)."
pub static PURSUIT: CardDef = CardDef {
    name: "Pursuit",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(&[Window::CombatDetermineRange], &[], &[Effect::Maneuver]),
        combat(
            &[Window::CombatAdditionalStrike],
            &[],
            &[Effect::AdditionalStrikes {
                n: 1,
                limited: true,
            }],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[tha] Press, only usable to continue combat. [THA] Maneuver."
pub static APPORTATION: CardDef = CardDef {
    name: "Apportation",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatPress],
            &[Requirement::NoContinuePress],
            &[Effect::Press],
        ),
        combat(&[Window::CombatDetermineRange], &[], &[Effect::Maneuver]),
    ],
    abilities: &[],
    riders: &[],
};

/// "[obl] Additional strike (limited). [OBL] Strike: dodge, with 1
/// additional strike (limited)."
pub static ARMS_OF_AHRIMAN: CardDef = CardDef {
    name: "Arms of Ahriman",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatAdditionalStrike],
            &[],
            &[Effect::AdditionalStrikes {
                n: 1,
                limited: true,
            }],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[
                Effect::Strike(StrikeKind::Dodge),
                Effect::AdditionalStrikes {
                    n: 1,
                    limited: true,
                },
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Not usable during the first round. [tha] Strike: 1R aggravated damage.
/// [THA] Strike: 2R aggravated damage."
pub static WALK_OF_FLAME: CardDef = CardDef {
    name: "Walk of Flame",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[Requirement::NotFirstRound],
            &[Effect::Strike(StrikeKind::Damage {
                n: 1,
                ranged: true,
                aggravated: true,
            })],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[Requirement::NotFirstRound],
            &[Effect::Strike(StrikeKind::Damage {
                n: 2,
                ranged: true,
                aggravated: true,
            })],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "A vampire can play only one Quickness each round. [cel] Additional
/// strike (limited). [CEL] Additional strike that does not count against
/// the limit."
pub static QUICKNESS: CardDef = CardDef {
    name: "Quickness",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            limit: Some(Limit::PerRound),
            ..combat(
                &[Window::CombatAdditionalStrike],
                &[],
                &[Effect::AdditionalStrikes {
                    n: 1,
                    limited: true,
                }],
            )
        },
        ModeDef {
            limit: Some(Limit::PerRound),
            ..combat(
                &[Window::CombatAdditionalStrike],
                &[],
                &[Effect::AdditionalStrikes {
                    n: 1,
                    limited: false,
                }],
            )
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "A vampire can play only one Rego Motum each round. [tha] Prevent 2
/// damage from the opposing minion's strike. [THA] Prevent 4 damage from
/// the opposing minion's strike." (1 blood.)
pub static REGO_MOTUM: CardDef = CardDef {
    name: "Rego Motum",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            limit: Some(Limit::PerRound),
            ..combat(
                &[Window::CombatDamagePrevention],
                &[Requirement::DamagePending],
                &[Effect::Prevent(2)],
            )
        },
        ModeDef {
            limit: Some(Limit::PerRound),
            ..combat(
                &[Window::CombatDamagePrevention],
                &[Requirement::DamagePending],
                &[Effect::Prevent(4)],
            )
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "Trifle. Add 1 blood to a ready vampire."
pub static LIFE_IN_THE_CITY: CardDef = CardDef {
    name: "Life in the City",
    trifle: true,
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
        effects: &[Effect::BloodToTarget(1)],
    }],
    abilities: &[],
    riders: &[],
};

/// "Lock a minion." (1 pool.) Any minion in play, a locked one included:
/// playing it to no effect is legal (and cycles the card).
pub static MISDIRECTION: CardDef = CardDef {
    name: "Misdirection",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::LockTarget],
    }],
    abilities: &[],
    riders: &[],
};

/// "Unique location. +2 transfers." (Its controller's influence phase.)
pub static INFORMATION_HIGHWAY: CardDef = CardDef {
    name: "Information Highway",
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
        effects: &[Effect::PutInPlay, Effect::Layer(Modifier::Transfers(2))],
    }],
    abilities: &[],
    riders: &[],
};

/// A location put in play in the master phase.
const LOCATION: ModeDef = ModeDef {
    windows: &[Window::MasterPhase],
    actor: Actor::Methuselah,
    requires: &[],
    target: NO_TARGET,
    terms: None,
    waives_needed: false,
    limit: None,
    effects: &[Effect::PutInPlay],
};

/// "Lock this card to give a <clan> you control +1 stealth": to the acting
/// minion of that clan, when stealth is needed (City Star Taxi's shape).
macro_rules! stealth_location {
    ($name:ident, $printed:expr, $clan:expr, $doc:expr) => {
        #[doc = $doc]
        pub static $name: CardDef = CardDef {
            name: $printed,
            trifle: false,
            out_of_turn: false,
            delayed_replacement: None,
            modes: &[LOCATION],
            abilities: &[ModeDef {
                windows: &[Window::ActionDuring],
                actor: Actor::Controller,
                requires: &[
                    Requirement::CardUnlocked,
                    Requirement::OtherActingControlled,
                    Requirement::OtherActingClan($clan),
                    Requirement::StealthNeeded,
                ],
                target: NO_TARGET,
                terms: None,
                waives_needed: false,
                limit: None,
                effects: &[Effect::LockSelf, Effect::AddStealth(1)],
            }],
            riders: &[],
        };
    };
}

stealth_location!(
    THE_LABYRINTH,
    "Labyrinth, The",
    "Nosferatu",
    "\"Unique location. You can lock this card to give a Nosferatu you \
     control +1 stealth.\""
);
stealth_location!(
    ELYSIAN_FIELDS,
    "Elysian Fields",
    "Lasombra",
    "\"Unique location. You can lock this location to give a Lasombra you \
     control +1 stealth.\""
);
stealth_location!(
    BACKWAYS,
    "Backways",
    "Gangrel",
    "\"Unique location. Lock to give a Gangrel you control +1 stealth.\""
);
stealth_location!(
    OPIUM_DEN,
    "Opium Den",
    "Ministry",
    "\"Unique location. Lock to give a Follower of Set you control +1 \
     stealth.\" Follower of Set is the clan's V5 name, Ministry."
);

/// "Unique location. Lock to give an Assamite you control +1 intercept."
/// To the Banu Haqim (the clan's V5 name) of its controller attempting to
/// block, while intercept is needed (Oskar Anasov's shape).
pub static MARKET_SQUARE: CardDef = CardDef {
    name: "Market Square",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[ModeDef {
        windows: &[Window::ActionStealthIntercept],
        actor: Actor::Controller,
        requires: &[
            Requirement::CardUnlocked,
            Requirement::YourAttemptNeedsIntercept,
            Requirement::AttemptingClan("Banu Haqim"),
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::LockSelf, Effect::AttemptingIntercept(1)],
    }],
    riders: &[],
};

// ---- Wakes and "if this vampire blocks" riders (slice 11b) -----------------

/// A wake of the reacting minion: locked, once between its unlock phases.
macro_rules! wake {
    ($requires:expr, $effects:expr) => {
        ModeDef {
            windows: &[Window::Wake, Window::CardAsPlayed],
            actor: Actor::ReactingMinion,
            requires: $requires,
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: Some(Limit::BetweenUnlockPhases),
            effects: $effects,
        }
    };
}

/// "Only usable by a locked minion. A minion can play only one On the Qui
/// Vive between their unlock phases. This minion wakes (they ignore the
/// requirement to be unlocked for playing reaction cards and attempting to
/// block until the end of the action). If this minion is an ally, they do
/// not unlock as normal during their next unlock phase."
pub static ON_THE_QUI_VIVE: CardDef = CardDef {
    name: "On the Qui Vive",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[wake!(
        &[Requirement::LockedActor],
        &[Effect::Wake, Effect::AllySkipsNextUnlock]
    )],
    abilities: &[],
    riders: &[],
};

/// "A vampire can play only one Shadow Sentinel between their unlock
/// phases. [obl] Only usable by a locked vampire. This vampire wakes. [OBL]
/// Usable by a locked vampire. Choose a locked wraith or zombie ally you
/// control. The chosen ally wakes."
pub static SHADOW_SENTINEL: CardDef = CardDef {
    name: "Shadow Sentinel",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        wake!(
            &[Requirement::LockedActor, Requirement::ActorVampire],
            &[Effect::Wake]
        ),
        ModeDef {
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::Controlled,
                    Requirement::TargetLocked,
                    Requirement::TargetSubtypeIn(&[Label::Wraith, Label::Zombie]),
                ],
            }),
            ..wake!(
                &[Requirement::MayBeLocked, Requirement::ActorVampire],
                &[Effect::WakeTarget]
            )
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable by a locked vampire during a bleed action directed at you.
/// [ani] Unlock this vampire. [ANI] As above, with 1 optional maneuver
/// during the resulting combat if this vampire blocks."
pub static GUARD_DOGS: CardDef = CardDef {
    name: "Guard Dogs",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            windows: &[Window::ReactionUnlock],
            actor: Actor::ReactingMinion,
            requires: &[
                Requirement::LockedActor,
                Requirement::ActorVampire,
                Requirement::DirectedAtYou,
                Requirement::IsBleed,
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::UnlockActor],
        },
        ModeDef {
            windows: &[Window::ReactionUnlock],
            actor: Actor::ReactingMinion,
            requires: &[
                Requirement::LockedActor,
                Requirement::ActorVampire,
                Requirement::DirectedAtYou,
                Requirement::IsBleed,
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::UnlockActor, Effect::IfBlocks(BlockRider::Maneuver)],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "[aus] +1 intercept. [AUS] As above, with 1 optional maneuver during the
/// resulting combat if this vampire blocks."
pub static SPIRITS_TOUCH: CardDef = CardDef {
    name: "Spirit's Touch",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(1)]),
        intercept(
            &[Requirement::InterceptNeeded],
            &[
                Effect::AddIntercept(1),
                Effect::IfBlocks(BlockRider::Maneuver),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable if a minion controlled by your predator is acting. [ani]
/// +1 intercept. [ANI] As above, with 1 optional maneuver during the
/// resulting combat if this vampire blocks."
pub static INSTINCTIVE_REACTION: CardDef = CardDef {
    name: "Instinctive Reaction",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        intercept(
            &[Requirement::ActingByPredator, Requirement::InterceptNeeded],
            &[Effect::AddIntercept(1)],
        ),
        intercept(
            &[Requirement::ActingByPredator, Requirement::InterceptNeeded],
            &[
                Effect::AddIntercept(1),
                Effect::IfBlocks(BlockRider::Maneuver),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[aus] +1 intercept. [AUS] As above, and this vampire can prevent 1
/// damage during the first round of the resulting combat if they block."
pub static PRECOGNITION: CardDef = CardDef {
    name: "Precognition",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(1)]),
        intercept(
            &[Requirement::InterceptNeeded],
            &[
                Effect::AddIntercept(1),
                Effect::IfBlocks(BlockRider::PreventFirstRound(1)),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

// ---- Forced block attempts and more block riders (slice 11b-ii) ------------

/// "Unlocks and attempts to block, even if intercept is not yet needed":
/// a locked reacting minion that could block (engine-checked).
macro_rules! unlock_attempt {
    ($requires:expr, $effects:expr) => {
        ModeDef {
            windows: &[Window::ReactionUnlockAttempt],
            actor: Actor::ReactingMinion,
            requires: $requires,
            target: NO_TARGET,
            terms: None,
            waives_needed: true,
            limit: None,
            effects: $effects,
        }
    };
}

const PRINCE_OR_JUSTICAR: Requirement =
    Requirement::ActorTitleIn(&[TitleClass::Prince, TitleClass::Justicar]);

/// "Requires a prince or justicar. [REACTION] +2 intercept. [REACTION]
/// Only usable by a locked prince or justicar. This vampire burns 1 blood
/// to unlock and attempt to block with +2 intercept, even if intercept is
/// not yet needed." The blood is an effect, not a cost: not reduced, and
/// the card is not playable without it (ANK 20210226).
pub static SECOND_TRADITION_DOMAIN: CardDef = CardDef {
    name: "Second Tradition: Domain",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        intercept(
            &[PRINCE_OR_JUSTICAR, Requirement::InterceptNeeded],
            &[Effect::AddIntercept(2)],
        ),
        unlock_attempt!(
            &[
                Requirement::LockedActor,
                PRINCE_OR_JUSTICAR,
                Requirement::ActorBloodAtLeast(1),
                Requirement::CanAttemptBlock,
            ],
            &[
                Effect::BurnActorBlood(1),
                Effect::UnlockActor,
                Effect::AttemptBlock,
                Effect::AddIntercept(2),
            ]
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Requires an Anarch. [ani] Only usable during an action directed at you
/// (or a card you control). +2 intercept. [for] Reduce a bleed against you
/// by 2. [pro] Only usable by a locked vampire. This vampire burns 1 blood
/// to unlock."
pub static DEEP_ECOLOGY: CardDef = CardDef {
    name: "Deep Ecology",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        intercept(
            &[
                ANARCH,
                Requirement::DirectedAtYou,
                Requirement::InterceptNeeded,
            ],
            &[Effect::AddIntercept(2)],
        ),
        ModeDef {
            windows: &[Window::ReactionDuringAction],
            actor: Actor::ReactingMinion,
            requires: &[ANARCH, Requirement::DirectedAtYou, Requirement::IsBleed],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::ReduceBleed(2)],
        },
        ModeDef {
            windows: &[Window::ReactionUnlock],
            actor: Actor::ReactingMinion,
            requires: &[
                ANARCH,
                Requirement::LockedActor,
                Requirement::ActorVampire,
                Requirement::ActorBloodAtLeast(1),
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::BurnActorBlood(1), Effect::UnlockActor],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable during an action directed at you (or a card you control).
/// [pro] +2 intercept. [PRO] Only usable by a locked vampire. This vampire
/// unlocks and attempts to block. If they block, neither combatant can
/// strike during the first round of the resulting combat." (Tzimisce.)
pub static ONE_WITH_THE_LAND: CardDef = CardDef {
    name: "One With the Land",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        intercept(
            &[Requirement::DirectedAtYou, Requirement::InterceptNeeded],
            &[Effect::AddIntercept(2)],
        ),
        unlock_attempt!(
            &[
                Requirement::DirectedAtYou,
                Requirement::LockedActor,
                Requirement::ActorVampire,
                Requirement::CanAttemptBlock,
            ],
            &[
                Effect::UnlockActor,
                Effect::AttemptBlock,
                Effect::IfBlocks(BlockRider::NoStrikesFirstRound),
            ]
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[pro] [ACTION MODIFIER] Only usable if a minion attempts to block. The
/// blocking minion gets -1 intercept. If this action is blocked, this
/// vampire gets 1 optional maneuver and cannot use equipment during the
/// resulting combat. [PRO] [REACTION] +1 intercept. If this vampire
/// blocks, they get 1 optional maneuver and cannot use equipment during the
/// resulting combat."
pub static FORM_OF_THE_BAT: CardDef = CardDef {
    name: "Form of the Bat",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            windows: &[Window::ActionBlockAttempt],
            actor: Actor::ActingMinion,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::AttemptingIntercept(-1),
                Effect::IfBlocks(BlockRider::Maneuver),
                Effect::IfBlocks(BlockRider::NoEquipment),
            ],
        },
        intercept(
            &[Requirement::InterceptNeeded],
            &[
                Effect::AddIntercept(1),
                Effect::IfBlocks(BlockRider::Maneuver),
                Effect::IfBlocks(BlockRider::NoEquipment),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[obl] +1 intercept. [OBL] As above, and if this vampire blocks, they
/// can burn 1 blood to unlock after block resolution."
pub static TRUTH_IN_DARKNESS: CardDef = CardDef {
    name: "Truth in Darkness",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(1)]),
        intercept(
            &[Requirement::InterceptNeeded],
            &[
                Effect::AddIntercept(1),
                Effect::IfBlocks(BlockRider::UnlockAfterForBlood(1)),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

const BARON: Requirement = Requirement::ActorTitleIn(&[TitleClass::Baron]);

/// "Requires a baron. Usable by a locked baron. An Anarch you control gets
/// +1 intercept. Also usable to unlock a locked Anarch you control; that
/// Anarch attempts to block with +1 intercept, even if intercept is not
/// yet needed." The intercept goes to the Anarch attempting to block; the
/// forced attempt keeps the blocking restrictions (ANK 20180623).
pub static ORGANIZED_RESISTANCE: CardDef = CardDef {
    name: "Organized Resistance",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            windows: &[Window::ActionStealthIntercept],
            actor: Actor::ReactingMinion,
            requires: &[
                BARON,
                Requirement::MayBeLocked,
                Requirement::YourAttemptNeedsIntercept,
                Requirement::AttemptingSectIn(&[Sect::Anarch]),
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::AttemptingIntercept(1)],
        },
        ModeDef {
            windows: &[Window::ReactionUnlockAttempt],
            actor: Actor::ReactingMinion,
            requires: &[BARON, Requirement::MayBeLocked],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::Controlled,
                    Requirement::TargetLocked,
                    ANARCH,
                    Requirement::CanAttemptBlock,
                ],
            }),
            terms: None,
            waives_needed: true,
            limit: None,
            effects: &[Effect::TargetAttemptsBlock(1)],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable by a locked Gangrel. This Gangrel unlocks and attempts to
/// block. During this action, this Gangrel can burn 1 blood to get +1
/// intercept. Lock this Gangrel after action resolution if they did not
/// block this action."
pub static EYES_OF_THE_WILD: CardDef = CardDef {
    name: "Eyes of the Wild",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[unlock_attempt!(
        &[Requirement::LockedActor, Requirement::CanAttemptBlock,],
        &[
            Effect::UnlockActor,
            Effect::AttemptBlock,
            Effect::BurnForInterceptThisAction,
            Effect::LockIfNoBlock,
        ]
    )],
    abilities: &[],
    riders: &[],
};

/// "[obf][pre] Reduce the acting minion's stealth to 0. (The acting minion
/// can still increase their stealth.) [OBF][PRE] As above, and if this
/// vampire blocks, they can strike: combat ends during the first round of
/// the resulting combat." (1 blood.) Any time before resolution, no attempt
/// needed (LSJ 20020612); a fixed -X by ruling, X the stealth as played
/// (RTR 20030519) — the exception to live conditions.
pub static NIGHT_TERRORS: CardDef = CardDef {
    name: "Night Terrors",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            windows: &[Window::ReactionDuringAction],
            actor: Actor::ReactingMinion,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::ReduceStealthToZero],
        },
        ModeDef {
            windows: &[Window::ReactionDuringAction],
            actor: Actor::ReactingMinion,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::ReduceStealthToZero,
                Effect::IfBlocks(BlockRider::CombatEndsFirstRound),
            ],
        },
    ],
    abilities: &[],
    riders: &[],
};

// ---- Block attempts that fail or cost (slice 11c) ---------------------------

/// An action modifier during a block attempt.
macro_rules! on_attempt {
    ($requires:expr, $effects:expr) => {
        ModeDef {
            windows: &[Window::ActionBlockAttempt],
            actor: Actor::ActingMinion,
            requires: $requires,
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: $effects,
        }
    };
}

/// "Only usable if a minion attempts to block. [obl] The blocking minion
/// gets -1 intercept. [OBL] Burn 1 blood to have that attempt fail; the
/// blocking minion cannot attempt to block this action again." The blood
/// is an effect (ANK 20210226).
pub static STYGIAN_SHROUD: CardDef = CardDef {
    name: "Stygian Shroud",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        on_attempt!(&[], &[Effect::AttemptingIntercept(-1)]),
        on_attempt!(
            &[Requirement::ActorBloodAtLeast(1)],
            &[Effect::BurnActorBlood(1), Effect::AttemptFails]
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable if a minion attempts to block. [dom] The blocking minion
/// gets -1 intercept. [DOM] That attempt fails and the blocking minion
/// cannot attempt to block this action again." (Salubri, 1 blood.)
pub static FORCED_CONFESSIONAL: CardDef = CardDef {
    name: "Forced Confessional",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        on_attempt!(&[], &[Effect::AttemptingIntercept(-1)]),
        on_attempt!(&[], &[Effect::AttemptFails]),
    ],
    abilities: &[],
    riders: &[],
};

/// "[obf][pre] Only usable during a bleed action. +1 bleed (limited).
/// [OBF][PRE] Only usable if a minion attempts to block. That attempt fails
/// and the blocking minion cannot attempt to block this action again. The
/// blocking minion's controller can burn 1 pool to cancel this card as it
/// is played."
pub static TRUE_LOVES_FACE: CardDef = CardDef {
    name: "True Love's Face",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        modifier(
            &[Requirement::IsBleed],
            &[Effect::BleedMod {
                n: 1,
                limited: true,
            }],
        ),
        on_attempt!(&[], &[Effect::CancelableForPool(1), Effect::AttemptFails]),
    ],
    abilities: &[],
    riders: &[],
};

/// "[obl] +1 stealth. [OBL] As above, and minions without Oblivion must
/// burn 1 blood to attempt to block (allies cannot burn blood)." One
/// already attempting pays or resigns its attempt (LSJ 20031121-2).
pub static WHERE_THE_VEIL_THINS: CardDef = CardDef {
    name: "Where the Veil Thins",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        stealth(&[Effect::AddStealth(1), Effect::VeilThins]),
    ],
    abilities: &[],
    riders: &[],
};

/// "[obf] +1 stealth. [OBF] As above, and minions who attempt to block this
/// action and fail become locked before action resolution." The attempts
/// that fail from its play on — the one under way included (LSJ 20081202),
/// not an earlier failure (ANK 20190509) — lock at resolution (ANK
/// 20171017).
pub static FACELESS_NIGHT: CardDef = CardDef {
    name: "Faceless Night",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        stealth(&[Effect::AddStealth(1), Effect::LockFailedBlockers]),
    ],
    abilities: &[],
    riders: &[],
};

// ---- Cancellation (slice 11d) ------------------------------------------------

/// "Out-of-turn. Cancel a minion card as it is played, and its cost is not
/// paid. (If it was an action card, the acting minion does not lock. If it
/// was a strike card, the minion chooses another strike.)" (1 pool.) An
/// owner-added legacy card (2026-09-25). A canceled action card declares
/// no action: its minion may take the same action again.
pub static DIRECT_INTERVENTION: CardDef = CardDef {
    name: "Direct Intervention",
    trifle: false,
    out_of_turn: true,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::CardAsPlayed],
        actor: Actor::Methuselah,
        requires: &[Requirement::NotOwnTurn, Requirement::PlayedCardIsMinionCard],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::CancelPlayed { refund: true }],
    }],
    abilities: &[],
    riders: &[],
};

/// "Out-of-turn. Trifle. Do not replace until your unlock phase. Cancel a
/// master card played by your predator or prey as it is played, and its
/// cost is not paid. That Methuselah gets +1 master phase action
/// immediately, or at the start of their next master phase if the canceled
/// card is an out-of-turn master." An owner-added legacy card
/// (2026-09-25).
pub static WASH: CardDef = CardDef {
    name: "Wash",
    trifle: true,
    out_of_turn: true,
    delayed_replacement: Some(Until::NextUnlockPhase),
    modes: &[ModeDef {
        windows: &[Window::CardAsPlayed],
        actor: Actor::Methuselah,
        requires: &[
            Requirement::NotOwnTurn,
            Requirement::PlayedCardIsMaster,
            Requirement::PlayedByPredatorOrPrey,
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::CancelPlayed { refund: true },
            Effect::CanceledGetsMasterAction,
        ],
    }],
    abilities: &[],
    riders: &[],
};

// ---- Titles and votes (slice 11e) ---------------------------------------

/// "A vampire can have only one archetype."
const NO_ARCHETYPE: Requirement = Requirement::TargetWithout(Holding::Label(Label::Archetype));

static TOREADOR_JUSTICAR_REFERENDUM: ReferendumDef = ReferendumDef {
    // Only a Camarilla vampire can be chosen; not playable without one
    // (LSJ 20041026).
    terms: &[Term::ChooseMinions {
        filter: &[
            Requirement::ActorReady,
            Requirement::TargetClan("Toreador"),
            Requirement::ActorSectIn(&[Sect::Camarilla]),
        ],
        min: 1,
        max: 1,
    }],
    effect: RefEffect::PutOnChosen,
    grants: &[VoteGrant {
        voters: Voters::Clan("Toreador"),
        n: 1,
    }],
};

/// "Title. Choose a ready Toreador. Successful referendum means this card is
/// put on the chosen Toreador to represent the unique Camarilla title of
/// Toreador Justicar. In this referendum, each ready Toreador gets +1
/// vote." The title is the one a printed Toreador justicar holds: the two
/// contest (advanced rules §5.8.1).
pub static TOREADOR_JUSTICAR: CardDef = CardDef {
    name: "Toreador Justicar",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::ActionDeclarePolitical],
        actor: Actor::ActingCandidate,
        requires: &[],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::CallReferendum(&TOREADOR_JUSTICAR_REFERENDUM),
            Effect::RepresentsTitle(crate::crypt::Title::Justicar("Toreador")),
        ],
    }],
    abilities: &[],
    riders: &[],
};

/// "Unique archetype. Title. Put this card on an Independent Salubri you
/// control to represent a unique Independent title worth 2 votes. This
/// Salubri can lock during any referendum to force a vampire to abstain.
/// Once each turn, this Salubri can lock before range is determined to end
/// a combat involving another minion you control. A vampire can have only
/// one archetype." A second copy contests as a title, paid in blood
/// (glossary "Title"). The abstention may target a vampire that has not
/// cast yet (advanced rules §3.7.6.3).
pub static SAULOTS_GUIDING_WISDOM: CardDef = CardDef {
    name: "Saulot's Guiding Wisdom",
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
                Requirement::Controlled,
                Requirement::TargetClan("Salubri"),
                Requirement::ActorSectIn(&[Sect::Independent]),
                NO_ARCHETYPE,
            ],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::PutOnTarget,
            Effect::RepresentsTitle(crate::crypt::Title::Independent {
                name: "Saulot's Guiding Wisdom",
                votes: 2,
            }),
        ],
    }],
    abilities: &[
        ModeDef {
            windows: &[Window::Referendum],
            actor: Actor::Host,
            requires: &[
                Requirement::AnyPollingStep,
                Requirement::HostReady,
                Requirement::HostUnlocked,
            ],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::TargetVampire,
                    Requirement::ActorReady,
                    Requirement::NotAbstaining,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::LockHost,
                Effect::Abstain {
                    lock: false,
                    burn: 0,
                },
            ],
        },
        ModeDef {
            windows: &[Window::CombatBeforeRange],
            actor: Actor::ForOwnCombatant,
            requires: &[
                Requirement::HostOutOfCombat,
                Requirement::HostReady,
                Requirement::HostUnlocked,
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerTurn),
            effects: &[Effect::LockHost, Effect::EndCombat],
        },
    ],
    riders: &[],
};

/// "Archetype. Trifle. Put this card on a vampire you control. Once each
/// turn, if they block a titled vampire or a political action, this vampire
/// gains 1 blood before block resolution. A vampire can have only one
/// archetype."
pub static REBEL: CardDef = CardDef {
    name: "Rebel",
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
                Requirement::Controlled,
                Requirement::TargetVampire,
                NO_ARCHETYPE,
            ],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::PutOnTarget],
    }],
    abilities: &[],
    riders: &[Rider {
        hook: Hook::HostBlocks,
        scope: RiderScope::Every,
        requires: &[
            Requirement::OncePerTurn,
            Requirement::AnyOf(&[
                &[Requirement::ActorTitleIn(&[TitleClass::Any])],
                &[Requirement::IsPolitical],
            ]),
        ],
        effects: &[Effect::AddHostBlood(1)],
    }],
};

/// A location locked during the polling step for votes: its master phase
/// play, and one ability per `effects` list.
macro_rules! vote_location {
    ($name:ident, $printed:expr, $doc:expr, $requires:expr, [$($effects:expr),+]) => {
        #[doc = $doc]
        pub static $name: CardDef = CardDef {
            name: $printed,
            trifle: false,
            out_of_turn: false,
            delayed_replacement: None,
            modes: &[LOCATION],
            abilities: &[$(ModeDef {
                windows: &[Window::Referendum],
                actor: Actor::Methuselah,
                requires: $requires,
                target: NO_TARGET,
                terms: None,
                waives_needed: false,
                limit: None,
                effects: $effects,
            }),+],
            riders: &[],
        };
    };
}

vote_location!(
    ELYSIUM_THE_PALACE_OF_VERSAILLES,
    "Elysium: The Palace of Versailles",
    "\"Unique location. You can lock this location during the polling step \
     of a political action to give each titled Camarilla vampire you control \
     +1 vote.\" Each such vampire, read live for the rest of the \
     referendum; one that has cast adds the vote to its cast.",
    &[Requirement::PollingStep, Requirement::CardUnlocked],
    [&[
        Effect::LockSelf,
        Effect::GrantVotes(VoteGrant {
            voters: Voters::TitledSect(Sect::Camarilla),
            n: 1,
        }),
    ]]
);

vote_location!(
    POWER_STRUCTURE,
    "Power Structure",
    "\"Unique. Put this card in play. During the polling step of a political \
     action, you can lock this card to give each Lasombra you control +1 \
     vote.\" Not during a blood hunt: no political action (ANK \
     20190606-2).",
    &[Requirement::PollingStep, Requirement::CardUnlocked],
    [&[
        Effect::LockSelf,
        Effect::GrantVotes(VoteGrant {
            voters: Voters::Clan("Lasombra"),
            n: 1,
        }),
    ]]
);

vote_location!(
    VENTRUE_HEADQUARTERS,
    "Ventrue Headquarters",
    "\"Unique location. You can lock this card during the polling step of any \
     referendum to get +3 votes.\" The Methuselah's own votes, cast as the \
     card locks, all on one side.",
    &[Requirement::AnyPollingStep, Requirement::CardUnlocked],
    [
        &[
            Effect::LockSelf,
            Effect::MethuselahVotes { n: 3, favor: true }
        ],
        &[
            Effect::LockSelf,
            Effect::MethuselahVotes { n: 3, favor: false }
        ]
    ]
);

// ---- Combat staples (slice 11f) --------------------------------------------

/// "Only usable at close range. [tha] Strike: hand strike. Damage from this
/// strike cannot be prevented by cards requiring Fortitude [for]. The
/// opposing vampire's strikes with weapons inflict no damage this round.
/// [THA] As above, and this hand strike is at +1 damage." The +1 is the
/// hand strike's own damage (TOM 19960225); dodged, it does nothing to the
/// weapons (LSJ 20040928).
pub static BLOOD_RAGE: CardDef = CardDef {
    name: "Blood Rage",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[Requirement::RangeClose],
            &[
                Effect::Strike(StrikeKind::Hand { bonus: 0 }),
                Effect::FortitudeProofStrike,
                Effect::NullifyOpposingWeapons,
            ],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[Requirement::RangeClose],
            &[
                Effect::Strike(StrikeKind::Hand { bonus: 1 }),
                Effect::FortitudeProofStrike,
                Effect::NullifyOpposingWeapons,
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[tha] Strike: 1R damage. This damage cannot be prevented by cards
/// requiring Fortitude [for]. The opposing vampire's strikes with weapons
/// inflict no damage this round. [THA] As above, but for 2R damage." The
/// weapons are nullified as it resolves (ANK 20170519).
pub static SOUL_BURN: CardDef = CardDef {
    name: "Soul Burn",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[
                Effect::Strike(StrikeKind::Damage {
                    n: 1,
                    ranged: true,
                    aggravated: false,
                }),
                Effect::FortitudeProofStrike,
                Effect::NullifyOpposingWeapons,
            ],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[
                Effect::Strike(StrikeKind::Damage {
                    n: 2,
                    ranged: true,
                    aggravated: false,
                }),
                Effect::FortitudeProofStrike,
                Effect::NullifyOpposingWeapons,
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable before range is determined. A vampire can play only one
/// Carrion Crows each combat. [ani] This combat, the opposing minion takes
/// 1R environmental damage each round during normal strike resolution.
/// [ANI] As above, but for 2R environmental damage." Dealt even against a
/// dodge; a strike: combat ends comes first (RTR 19941109, TOM 19951216).
pub static CARRION_CROWS: CardDef = CardDef {
    name: "Carrion Crows",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            limit: Some(Limit::PerCombat),
            ..combat(
                &[Window::CombatBeforeRange],
                &[],
                &[Effect::EnvironmentalDamage(1)],
            )
        },
        ModeDef {
            limit: Some(Limit::PerCombat),
            ..combat(
                &[Window::CombatBeforeRange],
                &[],
                &[Effect::EnvironmentalDamage(2)],
            )
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable before range is determined. [tha] This round, this vampire
/// can strike, ranged: steal 1 blood or life (becoming blood). [THA] As
/// above, but for 2 blood or life (becoming blood)." Usable more than once
/// in the round (ANK 20220207).
pub static HUNGER_OF_MARDUK: CardDef = CardDef {
    name: "Hunger of Marduk",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatBeforeRange],
            &[],
            &[Effect::RoundStrike(StrikeKind::StealBlood {
                n: 1,
                ranged: true,
            })],
        ),
        combat(
            &[Window::CombatBeforeRange],
            &[],
            &[Effect::RoundStrike(StrikeKind::StealBlood {
                n: 2,
                ranged: true,
            })],
        ),
    ],
    abilities: &[],
    riders: &[],
};

// ---- Archetypes (slice 11g) --------------------------------------------------

/// "Put this card on a <filter> you control. … A vampire can have only one
/// archetype": the master phase play, then `effects`.
const fn archetype(filter: &'static [Requirement], effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter,
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// "Archetype. Put this card on a vampire you control. Once each turn,
/// this vampire can gain 1 blood after performing a successful action
/// during which no reaction cards were played. A vampire can have only one
/// archetype." A reaction canceled as it is played still was played (LSJ
/// 20070330); an unblocked action with no effect is successful (LSJ
/// 20070411); after-resolution cards may follow the gain (ANK 20190113).
pub static PERFECTIONIST: CardDef = CardDef {
    name: "Perfectionist",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[archetype(
        &[
            Requirement::Controlled,
            Requirement::TargetVampire,
            NO_ARCHETYPE,
        ],
        &[Effect::PutOnTarget],
    )],
    abilities: &[ModeDef {
        windows: &[Window::ActionAfterResolution],
        actor: Actor::Host,
        requires: &[
            Requirement::ActorActing,
            Requirement::ActionSuccessful,
            Requirement::NoReactionPlayed,
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: Some(Limit::PerTurn),
        effects: &[Effect::AddHostBlood(1)],
    }],
    riders: &[],
};

/// "Unique archetype. Put this card on a Salubri you control. This Salubri
/// gets +1 strength. This Salubri can enter combat with a non-Salubri
/// vampire as a Ⓓ action; if that vampire is Tremere, this is a +1 stealth
/// action. A vampire can have only one archetype."
pub static SAULOTS_AVENGING_FIST: CardDef = CardDef {
    name: "Saulot's Avenging Fist",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[archetype(
        &[
            Requirement::Controlled,
            Requirement::TargetClan("Salubri"),
            NO_ARCHETYPE,
        ],
        &[
            Effect::PutOnTarget,
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Strength,
                n: 1,
                during: &[],
                who: Bearer::Any,
            }),
        ],
    )],
    abilities: &[
        enter_combat(&[
            Requirement::NotControlled,
            Requirement::ActorReady,
            Requirement::TargetVampire,
            Requirement::TargetClanNotIn(&["Salubri", "Tremere"]),
        ]),
        printed_action(
            Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::NotControlled,
                    Requirement::ActorReady,
                    Requirement::TargetClan("Tremere"),
                ],
            }),
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::EnterCombatWithTarget,
            ],
        ),
    ],
    riders: &[],
};

/// "Unique archetype. Put this card on a Salubri you control. Rescuing a
/// non-Tremere vampire from torpor costs this Salubri -2 blood, and if the
/// action is successful, the rescued vampire gains 1 blood. This Salubri
/// can add 1 blood or life to another ready minion, not to exceed starting
/// life, as a +1 stealth action. A vampire can have only one archetype."
/// The discount applies to the action, whoever pays (PIB 20110918, LSJ
/// 20020304-2). The action is directed at the target's controller unless
/// the target is the acting Methuselah's own (Ⓓ is only a reminder:
/// rules-qa 20.1).
pub static SAULOTS_HEALING_TOUCH: CardDef = CardDef {
    name: "Saulot's Healing Touch",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[archetype(
        &[
            Requirement::Controlled,
            Requirement::TargetClan("Salubri"),
            NO_ARCHETYPE,
        ],
        &[
            Effect::PutOnTarget,
            Effect::Layer(Modifier::RescueDiscount {
                not_clan: "Tremere",
                n: 2,
                gain: 1,
            }),
        ],
    )],
    abilities: &[printed_action(
        Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[Requirement::ActorReady, Requirement::OtherThanActor],
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::BloodToTargetToStart(1),
        ],
    )],
    riders: &[],
};

// ---- Combat cards (slice 11i) ----------------------------------------------

/// "Only usable before range is determined. A vampire can play only one
/// Bear's Skin each combat. [ani][pro] This round, this vampire gets +1
/// strength and can prevent 1 damage. [ANI][PRO] This combat, this vampire
/// gets +1 strength and can prevent 1 damage each round."
pub static BEARS_SKIN: CardDef = CardDef {
    name: "Bear's Skin",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            limit: Some(Limit::PerCombat),
            ..combat(
                &[Window::CombatBeforeRange],
                &[],
                &[Effect::StrengthThisRound(1), Effect::PreventThisRound(1)],
            )
        },
        ModeDef {
            limit: Some(Limit::PerCombat),
            ..combat(
                &[Window::CombatBeforeRange],
                &[],
                &[Effect::StrengthThisCombat(1), Effect::PreventEachRound(1)],
            )
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "Requires an Anarch. [ani] Strike: hand strike at +1 damage. This strike
/// cannot be dodged. [cel] Strike: dodge, with 1 additional strike
/// (limited). [pot] Strike: hand strike at +2 damage." A dodge against the
/// [ani] strike may be played, with no effect (LSJ 20030902-2); the [cel]
/// additional strike is not optional: not playable once the round's limited
/// one is used (ANK 20220204).
pub static DUST_UP: CardDef = CardDef {
    name: "Dust Up",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[Requirement::ActorSectIn(&[Sect::Anarch])],
            &[
                Effect::Strike(StrikeKind::Hand { bonus: 1 }),
                Effect::UndodgeableStrike,
            ],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[Requirement::ActorSectIn(&[Sect::Anarch])],
            &[
                Effect::Strike(StrikeKind::Dodge),
                Effect::AdditionalStrikes {
                    n: 1,
                    limited: true,
                },
            ],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[Requirement::ActorSectIn(&[Sect::Anarch])],
            &[Effect::Strike(StrikeKind::Hand { bonus: 2 })],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable before range is determined. A vampire can play only one
/// Form of the Wolf each combat. [pro] This vampire gets +1 strength this
/// combat. [PRO] As above, and this vampire gains 1 optional press this
/// combat." The press is usable this round only (TOM 19960521).
pub static FORM_OF_THE_WOLF: CardDef = CardDef {
    name: "Form of the Wolf",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            limit: Some(Limit::PerCombat),
            ..combat(
                &[Window::CombatBeforeRange],
                &[],
                &[Effect::StrengthThisCombat(1)],
            )
        },
        ModeDef {
            limit: Some(Limit::PerCombat),
            ..combat(
                &[Window::CombatBeforeRange],
                &[],
                &[Effect::StrengthThisCombat(1), Effect::PressThisRound],
            )
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "[for] Prevent 1 damage. [FOR] Burn 1 blood to prevent all damage from
/// the opposing minion's strikes this round." The blood is the effect's,
/// not a cost (ANK 20210226): the card is not playable without it, nor
/// with no damage to prevent (LSJ 20001114).
pub static ROLLING_WITH_THE_PUNCHES: CardDef = CardDef {
    name: "Rolling with the Punches",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatDamagePrevention],
            &[],
            &[Effect::Prevent(1)],
        ),
        combat(
            &[Window::CombatDamagePrevention],
            &[
                Requirement::ActorBloodAtLeast(1),
                Requirement::StrikeDamagePending,
            ],
            &[
                Effect::ActorBurnsBlood(1),
                Effect::PreventOpposingStrikesThisRound,
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[pro] Strike: hand strike, aggravated. [PRO] Strike: hand strike at +1
/// damage and put this card on the opposing minion. During their unlock
/// phase, the attached minion burns 1 blood or life. Minions can burn this
/// card as a +1 stealth action." (Tzimisce.) The card goes on as the strike
/// lands; an ally at no life is burned.
pub static SCULPT_THE_FLESH: CardDef = CardDef {
    name: "Sculpt the Flesh",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[
                Effect::Strike(StrikeKind::Hand { bonus: 0 }),
                Effect::AggravatedStrike,
            ],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[
                Effect::Strike(StrikeKind::Hand { bonus: 1 }),
                Effect::PutOnOpponentOnHit { counters: 0 },
            ],
        ),
    ],
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
            Effect::AddStealth(1),
            Effect::BurnSource,
        ],
    }],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::HostBurnBlood(1)],
    }],
};

/// "[obl] Strike: put this card on the opposing minion with 2 counters. The
/// attached minion burns 1 counter from this card instead of unlocking as
/// normal. If this card has no counters, burn it. [OBL] Strike: send the
/// opposing vampire to torpor or burn the opposing ally." The [OBL] vampire
/// is wounded (LSJ 20090622-2).
pub static TOUCH_OF_OBLIVION: CardDef = CardDef {
    name: "Touch of Oblivion",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[
                Effect::Strike(StrikeKind::CardEffect),
                Effect::PutOnOpponentOnHit { counters: 2 },
                Effect::Layer(Modifier::CounterInsteadOfUnlock),
            ],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[Effect::Strike(StrikeKind::TorporOrBurn)],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[obl] Strike: combat ends. [OBL] Strike: combat ends. After combat
/// ends, put this card on this vampire. This vampire can burn this card to
/// get +1 stealth." Not if the combat continues or a new one begins (RTR
/// 20020501, LSJ 19980109).
pub static PASS_THROUGH_SHADOW: CardDef = CardDef {
    name: "Pass Through Shadow",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[Effect::Strike(StrikeKind::CombatEnds { unlock: false })],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[
                Effect::Strike(StrikeKind::CombatEnds { unlock: false }),
                Effect::PutOnActorAfterCombat,
            ],
        ),
    ],
    abilities: &[ModeDef {
        windows: &[Window::ActionDuring],
        actor: Actor::Host,
        requires: &[Requirement::ActorActing, Requirement::StealthNeeded],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::BurnSelf, Effect::AddStealth(1)],
    }],
    riders: &[],
};

// ---- Combat cards with their own mechanisms (slice 11j) ---------------------

/// "[pot] Strike: hand strike at +2 damage. [POT] As above, with 1 optional
/// maneuver, only usable to get to close range." (1 blood.) Played at the
/// maneuver, it commits the round's initial strike (window review; LSJ
/// 20021028: not while the strike cannot be used).
pub static SLAM: CardDef = CardDef {
    name: "Slam",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[Effect::Strike(StrikeKind::Hand { bonus: 2 })],
        ),
        combat(
            &[
                Window::CombatStrikeDeclaration,
                Window::CombatDetermineRange,
            ],
            &[Requirement::ManeuverToClose],
            &[Effect::CommittedStrike(StrikeKind::Hand { bonus: 2 })],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[ani] Strike: 1R damage, with 1 optional maneuver. [ANI] As above, with
/// 1 optional press." Played at the maneuver, it commits the round's
/// initial strike.
pub static AID_FROM_BATS: CardDef = CardDef {
    name: "Aid from Bats",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[
                Window::CombatStrikeDeclaration,
                Window::CombatDetermineRange,
            ],
            &[],
            &[Effect::CommittedStrike(StrikeKind::Damage {
                n: 1,
                ranged: true,
                aggravated: false,
            })],
        ),
        combat(
            &[
                Window::CombatStrikeDeclaration,
                Window::CombatDetermineRange,
            ],
            &[],
            &[
                Effect::CommittedStrike(StrikeKind::Damage {
                    n: 1,
                    ranged: true,
                    aggravated: false,
                }),
                Effect::PressThisRound,
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "A vampire can play only one Pack Alpha each round. [ani] Only usable
/// before range is determined. Employ an animal retainer from your hand
/// (requirements and cost apply as normal). [ANI] Burn an animal retainer
/// employed by this vampire to put this card on this vampire. This minion
/// gets +1 strength. A minion can have only one Pack Alpha." The retainer
/// is named as the card is played (LSJ 20100130); not an action, so an
/// employ action's riders do not apply (LSJ 20090115-1).
pub static PACK_ALPHA: CardDef = CardDef {
    name: "Pack Alpha",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            limit: Some(Limit::PerRound),
            terms: Some(Terms::EmployFromHand(Label::Animal)),
            ..combat(&[Window::CombatBeforeRange], &[], &[Effect::EmployNamed])
        },
        ModeDef {
            windows: &[Window::CombatUntimed],
            actor: Actor::Combatant,
            requires: &[Requirement::ActorWithout(Holding::Named("Pack Alpha"))],
            target: Some(TargetSpec {
                kind: TargetKind::Card,
                filter: &[
                    Requirement::TargetCard(CardFilter::Labeled(CardType::Retainer, Label::Animal)),
                    Requirement::TargetOnActor,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerRound),
            effects: &[
                Effect::BurnTargetCard,
                Effect::PutOnActor,
                Effect::Layer(Modifier::Bearer {
                    stat: Stat::Strength,
                    n: 1,
                    during: &[],
                    who: Bearer::Any,
                }),
            ],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "[ani] Strike: dodge. [ANI] As above, with 1 additional ranged strike:
/// burn weapon." The superior version needs a weapon to burn; its
/// additional strike burns one of the opposing minion's weapons, at any
/// range, unless dodged (owner, 2026-09-26: rules-qa 22.3).
pub static VORACIOUS_VERMIN: CardDef = CardDef {
    name: "Voracious Vermin",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[Effect::Strike(StrikeKind::Dodge)],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[Requirement::OpposingWeapon],
            &[
                Effect::Strike(StrikeKind::Dodge),
                Effect::AdditionalStrikeOnly(StrikeKind::BurnWeapon),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable before range is determined during the first round. Put this
/// card with 5 counters on it on this minion; it becomes a melee weapon
/// equipment that can strike: strength+1 damage. For each damage inflicted
/// by this strike (even if prevented), burn 1 counter from this card. Burn
/// this card if it has no counters. A minion can have only one Weighted
/// Walking Stick." Damage not inflicted (Blood Rage) burns no counter (LSJ
/// 20100519); full damage while any counter is left (ANK 20201229-2).
pub static WEIGHTED_WALKING_STICK: CardDef = CardDef {
    name: "Weighted Walking Stick",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[combat(
        &[Window::CombatBeforeRange],
        &[
            Requirement::FirstRound,
            Requirement::ActorWithout(Holding::Named("Weighted Walking Stick")),
        ],
        &[
            Effect::PutOnActor,
            Effect::EnterWithCounters(5),
            Effect::BecomesWeapon,
        ],
    )],
    abilities: &[weapon_strike(
        &[],
        &[
            Effect::Strike(StrikeKind::Melee {
                bonus: 1,
                aggravated: false,
            }),
            Effect::CountersPerDamage,
        ],
    )],
    riders: &[],
};

// ---- Action modifiers and reactions (slice 11k) ----------------------------

/// A mode of `actor` in `windows`, no target.
const fn mode_of(
    windows: &'static [Window],
    actor: Actor,
    requires: &'static [Requirement],
    effects: &'static [Effect],
) -> ModeDef {
    ModeDef {
        windows,
        actor,
        requires,
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// "[obf] +1 stealth. [OBF] Only usable by a ready vampire other than the
/// acting minion. The acting minion gets +1 stealth." The [OBF] vampire is
/// the acting minion's controller's (LSJ 19990425), locked or not (RTR
/// 19941109, LSJ 19980210); several may each play one (ANK 20200515).
pub static CLOAK_THE_GATHERING: CardDef = CardDef {
    name: "Cloak the Gathering",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::OtherOwnMinion,
            &[Requirement::ActorVampire, Requirement::StealthNeeded],
            &[Effect::AddStealth(1)],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable by a ready unlocked vampire other than the acting minion.
/// Only one Veil the Legions can be played each action. [obf] The acting
/// minion gets +1 stealth. [OBF] As above, and this vampire can burn X
/// blood to give the next X actions minions you control perform this turn
/// +1 stealth. Only one Veil the Legions can be played at superior each
/// turn." An action canceled after it is announced still counts as one of
/// the next X (LSJ 20070526).
pub static VEIL_THE_LEGIONS: CardDef = CardDef {
    name: "Veil the Legions",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            limit: Some(Limit::OnePerAction),
            ..mode_of(
                &[Window::ActionStealthIntercept],
                Actor::OtherOwnMinion,
                &[
                    Requirement::ActorVampire,
                    Requirement::ActorUnlocked,
                    Requirement::StealthNeeded,
                ],
                &[Effect::AddStealth(1)],
            )
        },
        ModeDef {
            limit: Some(Limit::OnePerAction),
            terms: Some(Terms::XBlood),
            ..mode_of(
                &[Window::ActionStealthIntercept],
                Actor::OtherOwnMinion,
                &[
                    Requirement::ActorVampire,
                    Requirement::ActorUnlocked,
                    Requirement::StealthNeeded,
                    Requirement::NoSuperiorVeilThisTurn,
                ],
                &[Effect::AddStealth(1), Effect::StealthForNextActions],
            )
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "[for] Vampires cannot block this action. This vampire takes 2
/// unpreventable environmental aggravated damage after action resolution.
/// [FOR] As above, but this vampire takes 1 …" If blocked, after combat
/// (RTR 19980623); whenever the action reaches resolution, even ended before
/// combat or with no effect (ANK 20220331); not if canceled before (ANK
/// 20220401).
pub static DARING_THE_DAWN: CardDef = CardDef {
    name: "Daring the Dawn",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        mode_of(
            &[Window::ActionDuring],
            Actor::ActingMinion,
            &[],
            &[
                Effect::VampiresCannotBlock,
                Effect::DamageAfterResolution(2),
            ],
        ),
        mode_of(
            &[Window::ActionDuring],
            Actor::ActingMinion,
            &[],
            &[
                Effect::VampiresCannotBlock,
                Effect::DamageAfterResolution(1),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable after action resolution. [for] Only usable if the action
/// was successful. Unlock this vampire. [FOR] Only usable if the action was
/// blocked. Unlock this vampire." (1 blood.) A fizzled action is successful
/// (ANK 20220218); after an ended action no card is played (RTR 20180719).
pub static FREAK_DRIVE: CardDef = CardDef {
    name: "Freak Drive",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        mode_of(
            &[Window::ActionAfterResolution],
            Actor::ActingMinion,
            &[Requirement::ActionSuccessful],
            &[Effect::UnlockActor],
        ),
        mode_of(
            &[Window::ActionAfterResolution],
            Actor::ActingMinion,
            &[Requirement::ActionBlocked],
            &[Effect::UnlockActor],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable as the action is announced. [dom] Choose a younger vampire.
/// The chosen vampire cannot block this action. [DOM] As above, but choose
/// any vampire."
pub static SEDUCTION: CardDef = CardDef {
    name: "Seduction",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[Requirement::TargetVampire, Requirement::Younger],
            }),
            ..mode_of(
                &[Window::ActionAsAnnounced],
                Actor::ActingMinion,
                &[],
                &[Effect::TargetCannotBlock],
            )
        },
        ModeDef {
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[Requirement::TargetVampire],
            }),
            ..mode_of(
                &[Window::ActionAsAnnounced],
                Actor::ActingMinion,
                &[],
                &[Effect::TargetCannotBlock],
            )
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable as the action is announced. [dom] Choose a locked vampire.
/// The chosen vampire cannot block this action. [DOM] During this action,
/// minions cannot unlock." (1 blood.) Wakes still work; unlocks after
/// resolution do not (PIB 20150703).
pub static THE_SLEEPING_MIND: CardDef = CardDef {
    name: "Sleeping Mind, The",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[Requirement::TargetVampire, Requirement::TargetLocked],
            }),
            ..mode_of(
                &[Window::ActionAsAnnounced],
                Actor::ActingMinion,
                &[],
                &[Effect::TargetCannotBlock],
            )
        },
        mode_of(
            &[Window::ActionAsAnnounced],
            Actor::ActingMinion,
            &[],
            &[Effect::NoUnlockThisAction],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable as the action is announced. [dom] Choose a minion. The
/// chosen minion cannot play reaction cards this action. [DOM] As above,
/// and the next reaction card played this action costs +1 blood or life."
/// (Salubri, 1 blood.)
pub static UNLEASHING_THE_BESTIAL_SOUL: CardDef = CardDef {
    name: "Unleashing the Bestial Soul",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[],
            }),
            ..mode_of(
                &[Window::ActionAsAnnounced],
                Actor::ActingMinion,
                &[],
                &[Effect::TargetNoReactions],
            )
        },
        ModeDef {
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[],
            }),
            ..mode_of(
                &[Window::ActionAsAnnounced],
                Actor::ActingMinion,
                &[],
                &[Effect::TargetNoReactions, Effect::NextReactionCosts(1)],
            )
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "[obf][pre] Reduce a bleed against you by 3. [OBF][PRE] Lock this
/// vampire to reduce a bleed against you to 0. (The acting minion can
/// still increase the bleed amount.)" (1 blood; a combo card.)
pub static VISIONS_OF_ZAPATHASURA: CardDef = CardDef {
    name: "Visions of Zapathasura",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        mode_of(
            &[Window::ReactionDuringAction],
            Actor::ReactingMinion,
            &[Requirement::DirectedAtYou, Requirement::IsBleed],
            &[Effect::ReduceBleed(3)],
        ),
        mode_of(
            &[Window::ReactionDuringAction],
            Actor::ReactingMinion,
            &[Requirement::DirectedAtYou, Requirement::IsBleed],
            &[Effect::LockActor, Effect::ReduceBleedToZero],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Only usable if this minion is blocked, before block resolution. Unlock
/// this minion. The action ends (unsuccessfully). This minion cannot
/// perform the same action again this turn." It has reached resolution
/// (NRA, LSJ 20070709); nothing more is played (LSJ 20010803-1); a blocked
/// leave-torpor gives no diablerie (LSJ 20050105-2).
pub static CHANGE_OF_TARGET: CardDef = CardDef {
    name: "Change of Target",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[mode_of(
        &[Window::ActionBlocked],
        Actor::ActingMinion,
        &[],
        &[Effect::EndBlockedAction],
    )],
    abilities: &[],
    riders: &[],
};

/// "[obl] +1 stealth. [OBL] Only usable after resolution of a successful
/// action. Put this card on this vampire. Minions without Auspex [aus]
/// cannot perform actions directed at this vampire. During your unlock
/// phase, burn this card."
pub static SHADOW_CLOAK: CardDef = CardDef {
    name: "Shadow Cloak",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        mode_of(
            &[Window::ActionAfterResolution],
            Actor::ActingMinion,
            &[Requirement::ActionSuccessful],
            &[
                Effect::PutOnActor,
                Effect::Layer(Modifier::DirectedOnlyBy(&["aus"])),
            ],
        ),
    ],
    abilities: &[],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::BurnSelf],
    }],
};

// ---- Cards remembering a Methuselah (slice 11l) ------------------------------

/// "[obf] +1 stealth. [OBF] Only usable if a bleed would be successful.
/// Instead, the bleed burns no pool, is unsuccessful, and this card is put
/// on this vampire. The next time this vampire is about to successfully
/// bleed the same Methuselah, burn this card and this vampire gets +2
/// bleed." [OBF] is played in the would-be-successful window, after every
/// other modifier and reaction; only cards of that timing follow it, even
/// canceled (LSJ 19980105, LSJ 20110502, LSJ 20050607); only for a bleed of
/// 1 or more (PIB 20150612). Every copy remembering the prey burns as a later bleed
/// resolves against it (ANK 20200609), after every card (LSJ 19980105).
pub static SPYING_MISSION: CardDef = CardDef {
    name: "Spying Mission",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        mode_of(
            &[Window::ActionWouldBeSuccessful],
            Actor::ActingMinion,
            &[Requirement::IsBleed, Requirement::BleedAmountAtLeast(1)],
            &[Effect::ActionFails, Effect::PutOnActorRemembering],
        ),
    ],
    abilities: &[],
    riders: &[Rider {
        hook: Hook::HostAboutToBleedRemembered,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[
            Effect::BurnSelf,
            Effect::BleedMod {
                n: 2,
                limited: false,
            },
        ],
    }],
};

/// "[obl] +1 stealth. [OBL] Only usable after resolution of a successful
/// directed action. Put this card on this vampire. During an action
/// directed at the same Methuselah or same set of Methuselahs, this vampire
/// can burn this card to get +1 stealth." Played after resolution, after
/// any combat, still during the action (PIB 20150915, LSJ 19981028, ANK
/// 20190425); the vampire's controller is not part of the remembered set
/// (LSJ 20100527).
pub static SHADOW_CAST: CardDef = CardDef {
    name: "Shadow Cast",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        mode_of(
            &[Window::ActionAfterResolution],
            Actor::ActingMinion,
            &[Requirement::ActionSuccessful, Requirement::ActionDirected],
            &[Effect::PutOnActorRemembering],
        ),
    ],
    abilities: &[ModeDef {
        windows: &[Window::ActionStealthIntercept],
        actor: Actor::Host,
        requires: &[
            Requirement::ActorActing,
            Requirement::StealthNeeded,
            Requirement::DirectedAtRemembered,
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::BurnSelf, Effect::AddStealth(1)],
    }],
    riders: &[],
};

// ---- Corruption and multi-Discipline bleeds (slice 11m) ---------------------

/// A mode of an Anarch's multi-Discipline bleed card ("Requires an Anarch.
/// More than one Discipline can be used to play this card. Ⓓ Bleed."): one
/// per set of Disciplines used (`Label::MultiDiscipline`).
const fn anarch_bleed(target: Option<TargetSpec>, effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires: &[Requirement::ActorSectIn(&[Sect::Anarch])],
        target,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

const PLUS_ONE_BLEED: Effect = Effect::BleedMod {
    n: 1,
    limited: false,
};

/// "[obf] This is a +1 stealth action. [pre] The bleed is with +1 bleed.
/// [pro] Put 1 corruption counter on a minion controlled by the target
/// Methuselah if the bleed is successful (for 1 or more)." The minion is
/// chosen at resolution (ANK 20240706, LSJ 20080608); the counter gives no
/// opportunity to steal it (ANK 20200925). Modes: [obf], [pre], [pro], then
/// each larger set of them.
pub static PLATINUM_PROTOCOL: CardDef = CardDef {
    name: "Platinum Protocol, The",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        anarch_bleed(
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::AddStealth(1),
            ],
        ),
        anarch_bleed(
            NO_TARGET,
            &[Effect::DeclareAction(ActionKind::Bleed), PLUS_ONE_BLEED],
        ),
        anarch_bleed(
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::IfBleedSuccessful(&[Effect::CorruptMinionOfTarget]),
            ],
        ),
        anarch_bleed(
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::AddStealth(1),
                PLUS_ONE_BLEED,
            ],
        ),
        anarch_bleed(
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::AddStealth(1),
                Effect::IfBleedSuccessful(&[Effect::CorruptMinionOfTarget]),
            ],
        ),
        anarch_bleed(
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                PLUS_ONE_BLEED,
                Effect::IfBleedSuccessful(&[Effect::CorruptMinionOfTarget]),
            ],
        ),
        anarch_bleed(
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::AddStealth(1),
                PLUS_ONE_BLEED,
                Effect::IfBleedSuccessful(&[Effect::CorruptMinionOfTarget]),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// The vampire in the player's uncontrolled region Break the Bonds [pre]
/// names: "there must be an uncontrolled vampire, and it is chosen upon
/// declaration" (ANK 20240706).
const UNCONTROLLED_VAMPIRE: Option<TargetSpec> = Some(TargetSpec {
    kind: TargetKind::Uncontrolled,
    filter: &[],
});

/// "[ani] The bleed is with +1 bleed. [obf] If the bleed is successful, you
/// can lock a minion controlled by the target Methuselah. [pre] If the
/// bleed is successful, add 1 blood to a vampire in your uncontrolled
/// region." The [obf] minion is chosen at resolution, the prey needing
/// none (ANK 20240706). Modes: [ani], [obf], [pre], then each larger set.
pub static BREAK_THE_BONDS: CardDef = CardDef {
    name: "Break the Bonds",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        anarch_bleed(
            NO_TARGET,
            &[Effect::DeclareAction(ActionKind::Bleed), PLUS_ONE_BLEED],
        ),
        anarch_bleed(
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::IfBleedSuccessful(&[Effect::LockMinionOfTarget]),
            ],
        ),
        anarch_bleed(
            UNCONTROLLED_VAMPIRE,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::IfBleedSuccessful(&[Effect::BloodToUncontrolledTarget(1)]),
            ],
        ),
        anarch_bleed(
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                PLUS_ONE_BLEED,
                Effect::IfBleedSuccessful(&[Effect::LockMinionOfTarget]),
            ],
        ),
        anarch_bleed(
            UNCONTROLLED_VAMPIRE,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                PLUS_ONE_BLEED,
                Effect::IfBleedSuccessful(&[Effect::BloodToUncontrolledTarget(1)]),
            ],
        ),
        anarch_bleed(
            UNCONTROLLED_VAMPIRE,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::IfBleedSuccessful(&[
                    Effect::LockMinionOfTarget,
                    Effect::BloodToUncontrolledTarget(1),
                ]),
            ],
        ),
        anarch_bleed(
            UNCONTROLLED_VAMPIRE,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                PLUS_ONE_BLEED,
                Effect::IfBleedSuccessful(&[
                    Effect::LockMinionOfTarget,
                    Effect::BloodToUncontrolledTarget(1),
                ]),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[pre][pro] +1 stealth. [PRE][PRO] As above, and once this action, you
/// can burn 1 of your corruption counters from a blocking minion to have
/// their block attempt fail, and they cannot attempt to block this action
/// again."
pub static ENCHANTING_GAZE: CardDef = CardDef {
    name: "Enchanting Gaze",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        stealth(&[Effect::AddStealth(1), Effect::FailBlockForCorruption]),
    ],
    abilities: &[],
    riders: &[],
};

const LIMITED_BLEED: Effect = Effect::BleedMod {
    n: 1,
    limited: true,
};

/// "Only usable during a bleed action. [pre][pro] +1 bleed (limited), or +1
/// stealth and +1 bleed (limited). [PRE][PRO] As above, and if the bleed is
/// successful, this vampire can burn 2 of your corruption counters from a
/// minion controlled by the target Methuselah to unlock." Each printed mode
/// splits in two (`cards::library_mode`): the bleed alone, or with the
/// stealth — only when stealth is needed. Not usable to unlock if the
/// target is ousted by the bleed (ANK 20220210).
pub static REVELATION_OF_THE_SERPENT: CardDef = CardDef {
    name: "Revelation of the Serpent",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        mode_of(
            &[Window::ActionDuring],
            Actor::ActingMinion,
            &[Requirement::IsBleed],
            &[LIMITED_BLEED],
        ),
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::ActingMinion,
            &[Requirement::IsBleed, Requirement::StealthNeeded],
            &[Effect::AddStealth(1), LIMITED_BLEED],
        ),
        mode_of(
            &[Window::ActionDuring],
            Actor::ActingMinion,
            &[Requirement::IsBleed],
            &[LIMITED_BLEED, Effect::UnlockForCorruption(2)],
        ),
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::ActingMinion,
            &[Requirement::IsBleed, Requirement::StealthNeeded],
            &[
                Effect::AddStealth(1),
                LIMITED_BLEED,
                Effect::UnlockForCorruption(2),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

// ---- Effect-widened block eligibility (slice 11n) ---------------------------

/// "[aus] +1 intercept. [AUS] This vampire attempts to block the current
/// action, ignoring the normal prey, predator or target restrictions for
/// blocking actions." Only who may block is widened: a barred blocker
/// (Seduction) or a Methuselah who declined stays out (RTR 19950413, RTR
/// 20020501); one attempt only — continued as if unblocked, the action
/// needs another (LSJ 20030227).
pub static EAGLES_SIGHT: CardDef = CardDef {
    name: "Eagle's Sight",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(1)]),
        ModeDef {
            windows: &[Window::ActionStealthIntercept],
            actor: Actor::ReactingMinion,
            requires: &[Requirement::CanAttemptBlockAnyone],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::AttemptBlock],
        },
    ],
    abilities: &[],
    riders: &[],
};

// ---- Actions (slice 11o) -----------------------------------------------------

/// An action card played by a ready unlocked minion of the acting
/// Methuselah, at a target.
const fn action_at(
    requires: &'static [Requirement],
    target: Option<TargetSpec>,
    effects: &'static [Effect],
) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires,
        target,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// "Enter combat with a minion": another Methuselah's ready minion (no
/// combat with one's own, rules-qa 17.3).
const COMBAT_TARGET: Option<TargetSpec> = Some(TargetSpec {
    kind: TargetKind::Minion,
    filter: &[Requirement::NotControlled, Requirement::ActorReady],
});

/// "[dom] Ⓓ Bleed with +2 bleed. [DOM] As above, and if the bleed is
/// successful (for 1 or more), add 2 blood to a younger vampire you control
/// or in your uncontrolled region." (1 blood; Salubri.) The vampire is
/// chosen when the card is played (ANK 20240628-2): [DOM] splits by where
/// it is (`cards::library_mode`).
pub static FEAST_OF_THE_SOULS_SECRETS: CardDef = CardDef {
    name: "Feast of the Soul's Secrets",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        action_at(
            &[],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::BleedMod {
                    n: 2,
                    limited: false,
                },
            ],
        ),
        action_at(
            &[],
            Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::Controlled,
                    Requirement::TargetVampire,
                    Requirement::Younger,
                ],
            }),
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::BleedMod {
                    n: 2,
                    limited: false,
                },
                Effect::IfBleedSuccessful(&[Effect::BloodToTarget(2)]),
            ],
        ),
        action_at(
            &[],
            Some(TargetSpec {
                kind: TargetKind::Uncontrolled,
                filter: &[Requirement::Younger],
            }),
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::BleedMod {
                    n: 2,
                    limited: false,
                },
                Effect::IfBleedSuccessful(&[Effect::BloodToUncontrolledTarget(2)]),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "+1 stealth action. [cel] Ⓓ Bleed. [CEL] Ⓓ Enter combat with a locked
/// minion. This acting vampire gets 1 optional maneuver during that
/// combat." (1 blood.) A target unlocked by resolution: the action fizzles
/// (ANK 20210124, LSJ 20070411, LSJ 20090514).
pub static FLEETNESS: CardDef = CardDef {
    name: "Fleetness",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        action_at(
            &[],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::AddStealth(1),
            ],
        ),
        action_at(
            &[],
            Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::NotControlled,
                    Requirement::ActorReady,
                    Requirement::TargetLocked,
                ],
            }),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::EnterCombatWithTarget,
                Effect::InResultingCombat(BlockRider::Maneuver),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[cel][tha] Ⓓ Enter combat with a minion. This acting vampire gets 1
/// optional press during that combat. [CEL][THA] As above, and the opposing
/// minion cannot strike: combat ends during the first round of that
/// combat."
pub static HUNTERS_MARK: CardDef = CardDef {
    name: "Hunter's Mark",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        action_at(
            &[],
            COMBAT_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::EnterCombatWithTarget,
                Effect::InResultingCombat(BlockRider::Press),
            ],
        ),
        action_at(
            &[],
            COMBAT_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::EnterCombatWithTarget,
                Effect::InResultingCombat(BlockRider::Press),
                Effect::InResultingCombat(BlockRider::OpponentNoCombatEndsFirstRound),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Requires an Anarch. [cel] Ⓓ Steal 1 pool from another Methuselah. [pot]
/// Ⓓ Enter combat with a minion. [pre] Ⓓ Bleed with +1 bleed." A choice:
/// one Discipline (owner, 2026-09-26: rules-qa 25.2).
pub static LINE_BRAWL: CardDef = CardDef {
    name: "Line Brawl",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        action_at(
            &[Requirement::ActorSectIn(&[Sect::Anarch])],
            Some(TargetSpec {
                kind: TargetKind::Seat,
                filter: &[Requirement::NotSelf],
            }),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::StealPoolFromTarget(1),
            ],
        ),
        action_at(
            &[Requirement::ActorSectIn(&[Sect::Anarch])],
            COMBAT_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::EnterCombatWithTarget,
            ],
        ),
        action_at(
            &[Requirement::ActorSectIn(&[Sect::Anarch])],
            NO_TARGET,
            &[Effect::DeclareAction(ActionKind::Bleed), PLUS_ONE_BLEED],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[pot][pre] Ⓓ Bleed with +1 bleed. If this action is blocked, before
/// range is determined during the first round of the resulting combat,
/// this vampire can gain +1 strength that combat. [POT][PRE] As above, but
/// at +2 bleed and this vampire can gain +2 strength."
pub static SHOW_OF_FORCE: CardDef = CardDef {
    name: "Show of Force",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        action_at(
            &[],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                PLUS_ONE_BLEED,
                Effect::IfBlocks(BlockRider::StrengthBeforeRange(1)),
            ],
        ),
        action_at(
            &[],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::BleedMod {
                    n: 2,
                    limited: false,
                },
                Effect::IfBlocks(BlockRider::StrengthBeforeRange(2)),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[obl] Ⓓ Bleed with +1 bleed. If the bleed is successful (for 1 or
/// more), the target Methuselah discards 2 cards of their choice. [OBL] Ⓓ
/// Remove 7 cards in your prey's ash heap from the game to burn 3 of their
/// pool." The cards removed are chosen by the player; there is no order to
/// the ash heap (LSJ 20090710).
pub static SHROUD_OF_DECAY: CardDef = CardDef {
    name: "Shroud of Decay",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        action_at(
            &[],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                PLUS_ONE_BLEED,
                Effect::IfBleedSuccessful(&[Effect::TargetMethuselahDiscards(2)]),
            ],
        ),
        action_at(
            &[],
            Some(TargetSpec {
                kind: TargetKind::Seat,
                filter: &[Requirement::TargetPrey, Requirement::TargetAshAtLeast(7)],
            }),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::RemovePreyAshToBurnPool { remove: 7, pool: 3 },
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "Ⓓ Burn a location and burn 2 pool from its controller." (Gangrel.)
/// Directed at the location's controller (LSJ 20090324); one's own is
/// undirected.
pub static REWILDING: CardDef = CardDef {
    name: "Rewilding",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[action_at(
        &[],
        Some(TargetSpec {
            kind: TargetKind::Card,
            filter: &[Requirement::TargetCard(CardFilter::Location)],
        }),
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::TargetControllerBurnsPool(2),
            Effect::BurnTargetCard,
        ],
    )],
    abilities: &[],
    riders: &[],
};

const fn strength_on_bearer(n: i8) -> Effect {
    Effect::Layer(Modifier::Bearer {
        stat: Stat::Strength,
        n,
        during: &[],
        who: Bearer::Any,
    })
}

/// "+2 stealth action. [pot] Put this card on this vampire. This vampire
/// gets +1 strength. They cannot play cards named Torn Signpost. A vampire
/// can have only one Preternatural Strength. [POT] As above, but this
/// vampire gets +2 strength." (1 blood.)
pub static PRETERNATURAL_STRENGTH: CardDef = CardDef {
    name: "Preternatural Strength",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        action_at(
            &[Requirement::ActorWithout(Holding::Named(
                "Preternatural Strength",
            ))],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(2),
                Effect::PutOnActor,
                strength_on_bearer(1),
                Effect::Layer(Modifier::BearerCannotPlay("Torn Signpost")),
            ],
        ),
        action_at(
            &[Requirement::ActorWithout(Holding::Named(
                "Preternatural Strength",
            ))],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(2),
                Effect::PutOnActor,
                strength_on_bearer(2),
                Effect::Layer(Modifier::BearerCannotPlay("Torn Signpost")),
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "+3 stealth action. [pot] Put this card on this vampire. This vampire
/// gets +1 strength. This vampire can strike: burn equipment. Burn this
/// card if this vampire is in torpor. A vampire can have only one Heroic
/// Might. [POT] As above, with an additional +1 strength, and this vampire
/// can strike: 2R damage." (3 blood.) The equipment is named with the
/// strike.
pub static HEROIC_MIGHT: CardDef = CardDef {
    name: "Heroic Might",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        action_at(
            &[Requirement::ActorWithout(Holding::Named("Heroic Might"))],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(3),
                Effect::PutOnActor,
                strength_on_bearer(1),
            ],
        ),
        action_at(
            &[Requirement::ActorWithout(Holding::Named("Heroic Might"))],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(3),
                Effect::PutOnActor,
                strength_on_bearer(2),
            ],
        ),
    ],
    abilities: &[
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Host,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Card,
                filter: &[
                    Requirement::TargetCard(CardFilter::Type(CardType::Equipment)),
                    Requirement::TargetOnOpposingCombatant,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::BurnEquipment)],
        },
        ModeDef {
            windows: &[Window::CombatStrikeDeclaration],
            actor: Actor::Host,
            requires: &[Requirement::SuperiorVersion],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::Strike(StrikeKind::Damage {
                n: 2,
                ranged: true,
                aggravated: false,
            })],
        },
    ],
    riders: &[Rider {
        hook: Hook::HostToTorpor,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::BurnSelf],
    }],
};

/// "+1 stealth action. [tha] Put this card on this vampire, locked; this
/// vampire takes 1 unpreventable environmental aggravated damage. During
/// your minion phase, this vampire can lock this card to unlock. A vampire
/// can have only one Rutor's Hand. [THA] As above, and this vampire can
/// burn 3 blood to be immune to this aggravated damage." (1 blood.) The
/// card unlocks in its controller's unlock phase like any.
pub static RUTORS_HAND: CardDef = CardDef {
    name: "Rutor's Hand",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        action_at(
            &[Requirement::ActorWithout(Holding::Named("Rutor's Hand"))],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::PutOnActor,
                Effect::LockPlayedCard,
                Effect::ActorTakesAggravated {
                    damage: 1,
                    avoid_for: 0,
                },
            ],
        ),
        action_at(
            &[Requirement::ActorWithout(Holding::Named("Rutor's Hand"))],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::PutOnActor,
                Effect::LockPlayedCard,
                Effect::ActorTakesAggravated {
                    damage: 1,
                    avoid_for: 3,
                },
            ],
        ),
    ],
    abilities: &[ModeDef {
        windows: &[Window::AnyImpulse],
        actor: Actor::Host,
        requires: &[Requirement::OwnMinionPhase, Requirement::CardUnlocked],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::LockSelf, Effect::UnlockHost],
    }],
    riders: &[],
};

/// "[dom][pro] Only usable before range is determined. This vampire gets
/// +1 strength this combat. A vampire can play only one Monstrous Form each
/// round. [DOM][PRO] +1 stealth action. Put this card on this vampire.
/// During combat, you can lock this card to give this vampire +1 strength
/// this round, or 1 maneuver or press."
pub static MONSTROUS_FORM: CardDef = CardDef {
    name: "Monstrous Form",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            windows: &[Window::CombatBeforeRange],
            actor: Actor::Combatant,
            requires: &[Requirement::ActorVampire],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerRound),
            effects: &[Effect::StrengthThisCombat(1)],
        },
        action_at(
            &[],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::PutOnActor,
            ],
        ),
    ],
    abilities: &[
        ModeDef {
            windows: &[Window::CombatUntimed],
            actor: Actor::Host,
            requires: &[Requirement::CardUnlocked],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::StrengthThisRound(1)],
        },
        ModeDef {
            windows: &[Window::CombatDetermineRange],
            actor: Actor::Host,
            requires: &[Requirement::CardUnlocked],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::Maneuver],
        },
        ModeDef {
            windows: &[Window::CombatPress],
            actor: Actor::Host,
            requires: &[Requirement::CardUnlocked],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::Press],
        },
    ],
    riders: &[],
};

/// "[for] [ACTION] +1 stealth action. Add 3 blood or life to a minion, not
/// to exceed their starting life. [for] [COMBAT] Prevent all damage from
/// the opposing minion's strike. [FOR] [COMBAT] Only usable by a ready
/// vampire not involved in the combat involving another minion you
/// control. Prevent all damage to that minion from the opposing minion's
/// strike." (1 blood; Salubri.) Not usable with no damage to prevent (LSJ
/// 20001114).
pub static TOUCH_OF_VALEREN: CardDef = CardDef {
    name: "Touch of Valeren",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        action_at(
            &[],
            Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[],
            }),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::BloodToTargetToStart(3),
            ],
        ),
        ModeDef {
            windows: &[Window::CombatDamagePrevention],
            actor: Actor::Combatant,
            requires: &[Requirement::StrikeDamagePending],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::PreventStrikeDamage],
        },
        ModeDef {
            windows: &[Window::CombatDamagePrevention],
            actor: Actor::OwnOutOfCombat,
            requires: &[Requirement::StrikeDamagePending],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::PreventStrikeDamage],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "+1 stealth action. [tha] Employ an animal retainer from your hand
/// ignoring requirements (pay cost as normal). [THA] Put this card on a
/// minion you control. The attached minion gets 1 optional maneuver each
/// combat and +1 strength." (1 pool.) The retainer is named with the play
/// (as Pack Alpha, LSJ 20100130); its version follows the vampire's
/// Disciplines (LSJ 20080803); it is not an employ action (ANK 20180817).
pub static BIOTHAUMATURGIC_EXPERIMENT: CardDef = CardDef {
    name: "Biothaumaturgic Experiment",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[],
            target: NO_TARGET,
            terms: Some(Terms::EmployFromHandIgnoring(Label::Animal)),
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::EmployNamed,
            ],
        },
        action_at(
            &[],
            Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[Requirement::Controlled],
            }),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::PutOnTarget,
                strength_on_bearer(1),
            ],
        ),
    ],
    abilities: &[super::vampires::MANEUVER_EACH_COMBAT],
    riders: &[],
};

/// "+1 stealth action. [aus] Ⓓ Look at your prey's hand and discard one
/// card of your choice from it. [AUS] Put this card in play. Your prey
/// plays with an open hand. Any minion can burn this card as a Ⓓ action."
/// (1 blood.) The replacement is not seen (LSJ 20100119, PIB 20121031).
pub static REVELATIONS: CardDef = CardDef {
    name: "Revelations",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        action_at(
            &[],
            Some(TargetSpec {
                kind: TargetKind::Seat,
                filter: &[Requirement::TargetPrey],
            }),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::DiscardFromTargetHand,
            ],
        ),
        action_at(
            &[],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::PutInPlay,
                Effect::Layer(Modifier::PreyOpenHand),
            ],
        ),
    ],
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
    riders: &[],
};

// ---- Actions, wave 2 (slice 11p) ----------------------------------------------

/// "[dom][pro] [ACTION MODIFIER] +1 stealth. Allies get -1 intercept.
/// [DOM][PRO] [COMBAT] Only usable before range is determined. This round,
/// this vampire gets +1 strength and 1 optional maneuver, and can prevent 1
/// damage. A vampire can play only one Obedient Flesh each round." (1
/// blood.)
pub static OBEDIENT_FLESH: CardDef = CardDef {
    name: "Obedient Flesh",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        stealth(&[Effect::AddStealth(1), Effect::AlliesIntercept(-1)]),
        ModeDef {
            windows: &[Window::CombatBeforeRange],
            actor: Actor::Combatant,
            requires: &[Requirement::ActorVampire],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerRound),
            effects: &[
                Effect::StrengthThisRound(1),
                Effect::OptionalManeuver,
                Effect::PreventThisRound(1),
            ],
        },
    ],
    abilities: &[],
    riders: &[],
};

/// "Vampires can burn this card as a Ⓓ action."
const VAMPIRES_BURN_THIS: ModeDef = ModeDef {
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
};

/// "+1 stealth action. Unique. Put this card in play with 1 counter. During
/// your unlock phase, add 1 counter to this card. During your prey's unlock
/// phase, for each counter on this card, they burn 1 pool or remove a
/// library card at random in their ash heap from the game. Hecata you
/// control with capacity 4 or more can add 1 counter to this card as a +1
/// stealth action. Vampires can burn this card as a Ⓓ action." (Hecata.)
/// The prey's payment is announced as a count of ash heap cards, the rest
/// pool (as Constant Revolution, LSJ 20090618). The granted action
/// requires Hecata (LSJ 20080604).
pub static GATE_OF_ACHERON: CardDef = CardDef {
    name: "Gate of Acheron, The",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[action_at(
        &[],
        NO_TARGET,
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
            Effect::EnterWithCounters(1),
        ],
    )],
    abilities: &[
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[
                Requirement::YouControlThisCard,
                Requirement::ActorOfClan("Hecata"),
                Requirement::ActorCapacityAtLeast(4),
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::AddCounter(1),
            ],
        },
        VAMPIRES_BURN_THIS,
    ],
    riders: &[
        Rider {
            hook: Hook::UnlockPhase,
            scope: RiderScope::Controller,
            requires: &[],
            effects: &[Effect::AddCounter(1)],
        },
        Rider {
            hook: Hook::UnlockPhase,
            scope: RiderScope::Prey,
            requires: &[Requirement::CountersAtLeast(1)],
            effects: &[Effect::PoolOrAshCardsPerCounter],
        },
    ],
};

/// "+1 stealth action. Unique. Put this card in play with 1 counter. Hecata
/// you control can add 1 counter to this card as a +1 stealth action.
/// During your unlock phase, you can burn this card to move a vampire
/// controlled by your predator or prey with capacity less than X to their
/// owner's uncontrolled region, where X is the number of counters on this
/// card. Vampires can burn this card as a Ⓓ action." (Hecata.) The move is
/// Banishment's: blood kept as counters, the cards on it with it.
pub static PIT_OF_CONTEMPLATION: CardDef = CardDef {
    name: "Pit of Contemplation",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[action_at(
        &[],
        NO_TARGET,
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
            Effect::EnterWithCounters(1),
        ],
    )],
    abilities: &[
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[
                Requirement::YouControlThisCard,
                Requirement::ActorOfClan("Hecata"),
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::AddCounter(1),
            ],
        },
        ModeDef {
            windows: &[Window::UnlockPhaseOwn],
            actor: Actor::Controller,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::TargetVampire,
                    Requirement::ControlledByPredatorOrPrey,
                    Requirement::TargetCapacityBelowCounters,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::TargetToUncontrolled, Effect::BurnSelf],
        },
        VAMPIRES_BURN_THIS,
    ],
    riders: &[],
};

/// "[ani] Ⓓ Bleed with +1 bleed. [ANI] Frenzy. Ⓓ Enter combat with and lock
/// a vampire. The target vampire is considered the acting minion during
/// that combat." The lock and the combat are simultaneous, both lost if the
/// action ends first (ANK 20200702); another effect changing the acting
/// minion later takes precedence (ANK 20211022). Another Methuselah's
/// ready vampire (rules-qa 17.3).
pub static DEEP_SONG: CardDef = CardDef {
    name: "Deep Song",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        action_at(
            &[],
            NO_TARGET,
            &[Effect::DeclareAction(ActionKind::Bleed), PLUS_ONE_BLEED],
        ),
        action_at(
            &[],
            Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::NotControlled,
                    Requirement::ActorReady,
                    Requirement::TargetVampire,
                ],
            }),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::Frenzy,
                Effect::EnterCombatWithTarget,
                Effect::TargetActsInCombat,
            ],
        ),
    ],
    abilities: &[],
    riders: &[],
};

/// "[dom] Ⓓ Bleed with +2 bleed. [DOM] Ⓓ Put this card on a younger vampire
/// and lock them; you still control this card. The attached vampire does
/// not unlock as normal. During your next minion phase, burn this card to
/// unlock the attached vampire and take control of them until the end of
/// your turn." (2 blood.) Playable on a locked vampire (PIB 20150412); the
/// control taken at any point of that minion phase (LSJ 20001201), which
/// cannot end before; the vampire returns at the end of the turn as it is
/// (LSJ 20010119), removed from the game if its Methuselah is ousted (ANK
/// 20181110).
pub static PUPPET_MASTER: CardDef = CardDef {
    name: "Puppet Master",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[
        action_at(
            &[],
            NO_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::BleedMod {
                    n: 2,
                    limited: false,
                },
            ],
        ),
        action_at(
            &[],
            Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[Requirement::TargetVampire, Requirement::Younger],
            }),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::StillYours,
                Effect::PutOnTarget,
                Effect::LockTarget,
                Effect::Layer(Modifier::HostDoesNotUnlock),
            ],
        ),
    ],
    abilities: &[ModeDef {
        windows: &[Window::AnyImpulse],
        actor: Actor::Controller,
        requires: &[
            Requirement::OwnMinionPhase,
            Requirement::EnteredBeforeThisTurn,
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::TakeControlOfHost, Effect::BurnSelf],
    }],
    riders: &[],
};

// ---- Masters (slice 11q) ---------------------------------------------------

/// "Discipline. Put this card on a vampire. This vampire gets +1 level of
/// <Discipline> and +1 capacity. Cannot be put on a vampire with superior
/// <Discipline>." Any Methuselah's vampire, in torpor or not; the level and
/// capacity are read live (`GameState::discipline_level`, `capacity_of`),
/// given nothing while the vampire is out of play (advanced rules 5.1.1),
/// and blood above capacity drains off as the card leaves.
macro_rules! discipline_master {
    ($name:ident, $printed:expr, $tag:expr) => {
        pub static $name: CardDef = CardDef {
            name: $printed,
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
                        Requirement::TargetNotSuperior($tag),
                    ],
                }),
                terms: None,
                waives_needed: false,
                limit: None,
                effects: &[
                    Effect::PutOnTarget,
                    Effect::Layer(Modifier::DisciplineLevel($tag)),
                ],
            }],
            abilities: &[],
            riders: &[],
        };
    };
}

discipline_master!(CELERITY, "Celerity", "cel");
discipline_master!(DOMINATE, "Dominate", "dom");
discipline_master!(OBFUSCATE, "Obfuscate", "obf");
discipline_master!(OBLIVION, "Oblivion", "obl");
discipline_master!(POTENCE, "Potence", "pot");
discipline_master!(PROTEAN, "Protean", "pro");

/// "+1 stealth action. Requires a non-sterile baron. Put this card in play.
/// It becomes a 1-capacity (non-unique) Anarch vampire of the same clan as
/// the acting vampire and must hunt this turn. You can search your library
/// (shuffle afterward), hand and/or ash heap for a Discipline master card
/// and put it on this new vampire." Requiring a baron, it requires an
/// Anarch (LSJ 20080603). No V5 vampire is sterile. The new vampire enters
/// with no blood; it is no vampire in the ash heap (LSJ 20020115) — its
/// card goes there as a library card — and, moved to the uncontrolled
/// region, it stays a vampire and can be influenced (RTR 19990712).
pub static CHILDE_OF_THE_REVOLUTION: CardDef = CardDef {
    name: "Childe of the Revolution",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[action_at(
        &[BARON],
        NO_TARGET,
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
            Effect::BecomesVampire,
        ],
    )],
    abilities: &[],
    riders: &[],
};

/// "Unique location. [You can] lock during your influence phase to add 1
/// blood to a <clan> in your uncontrolled region." (Its clan icon: played
/// with a ready vampire of the clan.)
macro_rules! influence_location {
    ($name:ident, $printed:expr, $clan:expr, $doc:expr) => {
        #[doc = $doc]
        pub static $name: CardDef = CardDef {
            name: $printed,
            trifle: false,
            out_of_turn: false,
            delayed_replacement: None,
            modes: &[LOCATION],
            abilities: &[ModeDef {
                windows: &[Window::InfluencePhaseOwn],
                actor: Actor::Controller,
                requires: &[Requirement::CardUnlocked],
                target: Some(TargetSpec {
                    kind: TargetKind::Uncontrolled,
                    filter: &[Requirement::TargetClan($clan)],
                }),
                terms: None,
                waives_needed: false,
                limit: None,
                effects: &[Effect::LockSelf, Effect::BloodToUncontrolledTarget(1)],
            }],
            riders: &[],
        };
    };
}

influence_location!(
    ARCANE_LIBRARY,
    "Arcane Library",
    "Tremere",
    "\"Unique location. You can lock this card during your influence phase \
     to add 1 blood to a Tremere in your uncontrolled region.\""
);
influence_location!(
    ART_MUSEUM,
    "Art Museum",
    "Toreador",
    "\"Unique location. You can lock this location during your influence \
     phase to add 1 blood to a Toreador in your uncontrolled region.\""
);
influence_location!(
    ECOTERRORISTS,
    "Ecoterrorists",
    "Gangrel",
    "\"Unique location. Lock during your influence phase to add 1 blood to \
     a Gangrel in your uncontrolled region.\""
);

/// "Unique location. During your master phase, a Tremere you control can
/// move 1 blood from themselves to this card. Lock during your influence
/// phase to move all counters on this card to a Tremere in your
/// uncontrolled region. Any minion can burn this card as a Ⓓ action;
/// Malkavians get +1 stealth during that action." Blood from one Tremere a
/// turn (RTR 19960221: once per master phase per copy); a ready one — a
/// vampire in torpor uses no such ability. The counters go up to the
/// vampire's capacity there, the rest to the blood bank.
pub static WASSERSCHLOSS_ANIF: CardDef = CardDef {
    name: "Wasserschloss Anif, Austria",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            windows: &[Window::MasterPhase],
            actor: Actor::Controller,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::Controlled,
                    Requirement::ActorReady,
                    Requirement::TargetClan("Tremere"),
                    Requirement::BloodAtLeast(1),
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::BloodToCard(1)],
        },
        ModeDef {
            windows: &[Window::InfluencePhaseOwn],
            actor: Actor::Controller,
            requires: &[Requirement::CardUnlocked],
            target: Some(TargetSpec {
                kind: TargetKind::Uncontrolled,
                filter: &[Requirement::TargetClan("Tremere")],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::CountersToUncontrolledTarget],
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
                Effect::DeclareAction(ActionKind::BurnCard),
                Effect::BurnSource,
                Effect::StealthForClan {
                    clan: "Malkavian",
                    n: 1,
                },
            ],
        },
    ],
    riders: &[],
};

/// "Trifle. Put this card in play. You can use 1 transfer to draw 1 card
/// from your crypt and then remove a crypt card in your uncontrolled region
/// from the game. You can use 4 transfers to burn this card and gain 2
/// pool." As often as the transfers last; the draw needs a card in the
/// crypt. Removed cards are public (LSJ 20080314).
pub static WIDER_VIEW: CardDef = CardDef {
    name: "Wider View",
    trifle: true,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            windows: &[Window::Transfers],
            actor: Actor::Controller,
            requires: &[Requirement::CryptNotEmpty],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::SpendTransfers(1), Effect::DrawCryptThenRemove],
        },
        ModeDef {
            windows: &[Window::Transfers],
            actor: Actor::Controller,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::SpendTransfers(4),
                Effect::BurnSelf,
                Effect::GainPool(2),
            ],
        },
    ],
    riders: &[],
};

/// "Trifle. Reveal the top card of your crypt. If it is a Hecata, draw it
/// and add 1 blood to it; otherwise, move it to the bottom of your crypt."
/// (Hecata: played with a ready Hecata.) Not with an empty crypt (RTR
/// 20000501, LSJ 20070411-2).
pub static FAMILY_GATHERING: CardDef = CardDef {
    name: "Family Gathering",
    trifle: true,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[Requirement::CryptNotEmpty],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::RevealCryptTop("Hecata")],
    }],
    abilities: &[],
    riders: &[],
};

/// "<Minions> can steal this location as a Ⓓ action", for a card another
/// Methuselah controls.
const fn steal_this(requires: &'static [Requirement]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires,
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::DeclareAction(ActionKind::StealCard)],
    }
}

/// "Unique location. During your influence phase, you can add 1 blood to a
/// vampire in your uncontrolled region. Vampires can steal this location
/// as a Ⓓ action." Not locked: once each influence phase. Usable with no
/// transfer taken (LSJ 20100527).
pub static POWERBASE_MONTREAL: CardDef = CardDef {
    name: "Powerbase: Montreal",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            windows: &[Window::InfluencePhaseOwn],
            actor: Actor::Controller,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Uncontrolled,
                filter: &[],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::BloodToUncontrolledTarget(1)],
        },
        steal_this(&[
            Requirement::ActorVampire,
            Requirement::OthersControlThisCard,
        ]),
    ],
    riders: &[],
};

/// "Unique location. As this location is played or its controller
/// changes, its controller chooses a ready vampire they control. During
/// this location's controller's unlock phase, the chosen vampire can gain
/// 2 blood. Vampires controlled by another Methuselah can steal this
/// location as a Ⓓ action." The choice holds through a contest or a
/// banishment, and gives blood again on the vampire's return (LSJ
/// 20010809-2/-3) — never to a new copy of a burned one (TOM 19960122) —
/// in torpor too, not while uncontrolled (PIB 20150522); a stolen vampire's
/// new controller decides (ANK 20200130-2). Falling out of a contest makes
/// no new choice (RTR 19960112).
pub static THE_RACK: CardDef = CardDef {
    name: "Rack, The",
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
                Requirement::Controlled,
                Requirement::TargetVampire,
                Requirement::ActorReady,
            ],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::PutInPlay,
            Effect::ChooseTarget,
            Effect::RechoosesOnControlChange,
        ],
    }],
    abilities: &[
        ModeDef {
            windows: &[Window::UnlockPhaseAny],
            actor: Actor::ChosenController,
            requires: &[Requirement::CardControllersTurn],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::BloodToChosen(2)],
        },
        steal_this(&[
            Requirement::ActorVampire,
            Requirement::OthersControlThisCard,
        ]),
    ],
    riders: &[],
};

/// "Unique. Put this card in play. During your unlock phase, you can give
/// your prey control of the Anarch Troublemaker and either lock up to two
/// vampires they control or burn an equipment on one of their minions."
/// A locked vampire is a legal choice (PIB 20121031).
pub static ANARCH_TROUBLEMAKER: CardDef = CardDef {
    name: "Anarch Troublemaker",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            windows: &[Window::UnlockPhaseOwn],
            actor: Actor::Controller,
            requires: &[],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::GiveToPrey, Effect::LockPreyVampires(2)],
        },
        ModeDef {
            windows: &[Window::UnlockPhaseOwn],
            actor: Actor::Controller,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Card,
                filter: &[
                    Requirement::TargetCard(CardFilter::Type(CardType::Equipment)),
                    Requirement::TargetOnPreyMinion,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::GiveToPrey, Effect::BurnTargetCard],
        },
    ],
    riders: &[],
};

/// "Unique location. During your discard phase, you can lock this location
/// to get +1 discard phase action. If you use that discard phase action to
/// discard a card requiring an Anarch or making a vampire Anarch, you can
/// unlock a ready Anarch. Anarchs controlled by other Methuselahs can steal
/// this location as a Ⓓ action." A card requiring a baron requires an
/// Anarch (LSJ 20050128, 20080603); the steal is an action requiring an
/// Anarch (LSJ 20080604).
pub static POWERBASE_LOS_ANGELES: CardDef = CardDef {
    name: "Powerbase: Los Angeles",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            windows: &[Window::DiscardPhase],
            actor: Actor::Controller,
            requires: &[Requirement::CardUnlocked],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::GrantDiscardAction],
        },
        steal_this(&[
            Requirement::ActorSectIn(&[Sect::Anarch]),
            Requirement::OthersControlThisCard,
        ]),
    ],
    riders: &[],
};

/// "Unique location. You can lock this location and burn 1 pool during
/// your unlock phase to exchange one card from your hand for one card in
/// your ash heap requiring an Anarch. You can lock this location before
/// range is determined to end a combat involving an Anarch you control and
/// another Anarch." Only cards whose requirements name Anarch or baron
/// (ANK 20250320), both named as it is used (LSJ 20000222); end-of-combat
/// cards can still be played (RTR 20001020).
pub static GARIBALDI_MEUCCI_MUSEUM: CardDef = CardDef {
    name: "Garibaldi-Meucci Museum",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            windows: &[Window::UnlockPhaseOwn],
            actor: Actor::Controller,
            requires: &[
                Requirement::CardUnlocked,
                Requirement::PoolAtLeast(1),
                Requirement::AnarchCardInAshAndHand,
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::LockSelf,
                Effect::PayPool(1),
                Effect::ExchangeForAnarchCard,
            ],
        },
        ModeDef {
            windows: &[Window::CombatBeforeRange],
            actor: Actor::Controller,
            requires: &[
                Requirement::CardUnlocked,
                Requirement::AnarchCombatWithYours,
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::EndCombat],
        },
    ],
    riders: &[],
};

/// A master card put in play by its player, requiring nothing.
const fn master_in_play(requires: &'static [Requirement], effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires,
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// "Unique. Requires a ready Anarch. Put this card in play. You can lock
/// this card to give an Anarch you control +1 intercept. You can lock this
/// card after an Anarch successfully hunts to add 1 blood to that Anarch."
/// A hunt gaining no blood is not successful (LSJ 20050720-2); the bonus
/// does not make the hunt one requiring an Anarch (ANK 20220317).
pub static ANARCH_FREE_PRESS: CardDef = CardDef {
    name: "Anarch Free Press, The",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[master_in_play(
        &[Requirement::ControlsReadySect(&[Sect::Anarch])],
        &[Effect::PutInPlay],
    )],
    abilities: &[
        ModeDef {
            windows: &[Window::ActionStealthIntercept],
            actor: Actor::Controller,
            requires: &[
                Requirement::CardUnlocked,
                Requirement::YourAttemptNeedsIntercept,
                Requirement::AttemptingSectIn(&[Sect::Anarch]),
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::AttemptingIntercept(1)],
        },
        ModeDef {
            windows: &[Window::ActionAfterResolution],
            actor: Actor::Controller,
            requires: &[Requirement::CardUnlocked, Requirement::AnarchHuntedYours],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::BloodToActing(1)],
        },
    ],
    riders: &[],
};

/// "Unique location. You can lock this card to give a minion you control
/// +2 intercept. Not usable during the first action in a minion phase."
/// An action canceled as its card was played is not the first action; one
/// canceled after it was played is (LSJ 20100505, ANK 20210124).
pub static CHANNEL_10: CardDef = CardDef {
    name: "Channel 10",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[ModeDef {
        windows: &[Window::ActionStealthIntercept],
        actor: Actor::Controller,
        requires: &[
            Requirement::CardUnlocked,
            Requirement::YourAttemptNeedsIntercept,
            Requirement::NotFirstActionOfPhase,
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::LockSelf, Effect::AttemptingIntercept(2)],
    }],
    riders: &[],
};

/// "Unique location. You can lock this card as a vampire you control
/// announces an undirected action to give that vampire +1 stealth, even if
/// stealth is not yet needed."
pub static CREEPSHOW_CASINO: CardDef = CardDef {
    name: "Creepshow Casino",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[ModeDef {
        windows: &[Window::ActionAsAnnounced],
        actor: Actor::Controller,
        requires: &[
            Requirement::CardUnlocked,
            Requirement::OtherActingControlled,
            Requirement::ActingVampire,
            Requirement::ActionUndirected,
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: true,
        limit: None,
        effects: &[Effect::LockSelf, Effect::AddStealth(1)],
    }],
    riders: &[],
};

/// "Unique location. An Anarch can burn 1 blood once as they announce a
/// bleed action to get +1 bleed during that action." Any Anarch, whoever
/// controls the location; not an action requiring an Anarch (ANK
/// 20220317).
pub static CLUB_ILLUSION: CardDef = CardDef {
    name: "Club Illusion",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[ModeDef {
        windows: &[Window::ActionAsAnnounced],
        actor: Actor::ActingMinion,
        requires: &[
            Requirement::IsBleed,
            Requirement::ActorSectIn(&[Sect::Anarch]),
            Requirement::ActorBloodAtLeast(1),
        ],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: Some(Limit::PerAction),
        effects: &[
            Effect::HostBurnBlood(1),
            Effect::BleedMod {
                n: 1,
                limited: false,
            },
        ],
    }],
    riders: &[],
};

/// "Put this card in play. During a bleed action, an Assamite you control
/// can discard a combat card to get +1 bleed." (Banu Haqim.) Once per
/// action per copy (ANK 20221102-2), by the bleeding minion (ANK
/// 20220502), before resolution (ANK 20231124, LSJ 19970718).
pub static HAQIMS_LAW_RETRIBUTION: CardDef = CardDef {
    name: "Haqim's Law: Retribution",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[master_in_play(&[], &[Effect::PutInPlay])],
    abilities: &[ModeDef {
        windows: &[Window::ActionAsAnnounced, Window::ActionDuring],
        actor: Actor::ActingMinion,
        requires: &[
            Requirement::IsBleed,
            Requirement::YouControlThisCard,
            Requirement::ActorClan("Banu Haqim"),
            Requirement::HandHas(CardFilter::Type(CardType::Combat), 1),
        ],
        target: NO_TARGET,
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Type(CardType::Combat),
        }),
        waives_needed: false,
        limit: Some(Limit::PerAction),
        effects: &[Effect::BleedMod {
            n: 1,
            limited: false,
        }],
    }],
    riders: &[],
};

/// "Unique. Put this card on a ready minion. This minion cannot block.
/// Other minions can burn this card as a Ⓓ action."
pub static PENTEX_SUBVERSION: CardDef = CardDef {
    name: "Pentex(TM) Subversion",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
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
        effects: &[
            Effect::PutOnTarget,
            Effect::Layer(Modifier::HostCannotBlock),
        ],
    }],
    abilities: &[ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires: &[Requirement::NotHost],
        target: NO_TARGET,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::BurnSource,
        ],
    }],
    riders: &[],
};

/// "Put this card on a ready vampire. Minions can enter combat with this
/// vampire as a +1 stealth Ⓓ action. This vampire can burn this card as a
/// +1 stealth Ⓓ action." Not with a minion of one's own (rules-qa 17.3).
pub static HAVEN_UNCOVERED: CardDef = CardDef {
    name: "Haven Uncovered",
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
    abilities: &[
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::TargetIsHost,
                    Requirement::NotControlled,
                    Requirement::ActorReady,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::EnterCombatWithTarget,
            ],
        },
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[Requirement::ActorIsHost],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::BurnCard),
                Effect::AddStealth(1),
                Effect::BurnSource,
            ],
        },
    ],
    riders: &[],
};

/// "Unique location. You can lock this card and burn 1 pool or 1 blood from
/// a ready Tremere you control during your master phase to move a Tremere
/// from torpor to their controller's ready region." (Tremere.) Any
/// Methuselah's Tremere; once each master phase.
pub static CHANTRY: CardDef = CardDef {
    name: "Chantry",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            windows: &[Window::MasterPhase],
            actor: Actor::Controller,
            requires: &[Requirement::CardUnlocked, Requirement::PoolAtLeast(1)],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::TargetClan("Tremere"),
                    Requirement::TargetInTorpor,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::LockSelf,
                Effect::PayPool(1),
                Effect::TargetLeavesTorpor,
            ],
        },
        ModeDef {
            windows: &[Window::MasterPhase],
            actor: Actor::Controller,
            requires: &[
                Requirement::CardUnlocked,
                Requirement::OwnReadyClanWithBlood("Tremere"),
            ],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::TargetClan("Tremere"),
                    Requirement::TargetInTorpor,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::LockSelf,
                Effect::OwnClanBurnsBlood("Tremere"),
                Effect::TargetLeavesTorpor,
            ],
        },
    ],
    riders: &[],
};

/// "Choose two ready Toreador you control, put this card in play, and lock
/// one of the two. The locked Toreador does not unlock as normal. The other
/// Toreador's non-bleed actions cannot be blocked. Minions can burn this
/// card as a Ⓓ action; Nosferatu get -1 stealth during that action."
/// (Toreador.) The one to lock is the target; the other is chosen as it
/// resolves — it may be the second Toreador of other Grand Balls (TOM
/// 19960528), and stays unblockable on non-bleed actions after a bleed (LSJ
/// 19970814).
pub static TOREADOR_GRAND_BALL: CardDef = CardDef {
    name: "Toreador Grand Ball",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[Requirement::ControlsReadyClan("Toreador", 2)],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[
                Requirement::Controlled,
                Requirement::ActorReady,
                Requirement::TargetClan("Toreador"),
            ],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::PutInPlay,
            Effect::ChooseTarget,
            Effect::LockTarget,
            Effect::ChooseSecondOfClan("Toreador"),
            Effect::Layer(Modifier::FirstChosenDoesNotUnlock),
            Effect::Layer(Modifier::SecondChosenUnblockable),
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
            Effect::StealthForClan {
                clan: "Nosferatu",
                n: -1,
            },
        ],
    }],
    riders: &[],
};

/// "Unique contract. Trifle. Choose a ready Assamite you control and put
/// this card on a minion controlled by your prey. The chosen Assamite can
/// enter combat with the attached minion as a +1 stealth Ⓓ action. If the
/// attached minion is about to leave the ready region, you can burn this
/// card to gain 3 pool." (Banu Haqim.) The Assamite is chosen as it
/// resolves; the trifle's master phase action comes even if it contests
/// (RBK contested-cards); the pool comes as its controller is ousted too
/// (ANK 20180129-2), and if the card goes with the minion (PIB 20150512).
pub static PRIORITY_CONTRACT: CardDef = CardDef {
    name: "Priority Contract",
    trifle: true,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires: &[Requirement::ControlsReadyClan("Banu Haqim", 1)],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[Requirement::ControlledByPrey],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::PutOnTarget,
            Effect::ChooseOwnOfClan("Banu Haqim"),
            Effect::PoolWhenHostLeaves(3),
        ],
    }],
    abilities: &[ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires: &[Requirement::ActorIsChosen],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[
                Requirement::TargetIsHost,
                Requirement::NotControlled,
                Requirement::ActorReady,
            ],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::EnterCombatWithTarget,
        ],
    }],
    riders: &[],
};

/// "Put this card in play. Each ready minion you control can enter combat
/// with a minion controlled by your prey as a Ⓓ action. You gain 1 pool
/// after a ready minion controlled by your prey is burned or sent to
/// torpor. During your influence phase, burn this card and burn 1 pool for
/// each ready minion controlled by your prey." Copies stack (LSJ
/// 20061018).
pub static FRONTAL_ASSAULT: CardDef = CardDef {
    name: "Frontal Assault",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[master_in_play(
        &[],
        &[Effect::PutInPlay, Effect::BurnAtInfluencePerPreyReady],
    )],
    abilities: &[ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires: &[Requirement::YouControlThisCard],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[Requirement::ControlledByPrey, Requirement::ActorReady],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::EnterCombatWithTarget,
        ],
    }],
    riders: &[Rider {
        hook: Hook::ReadyMinionBurnedOrTorpor,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::ControllerGainsPoolIfPrey(1)],
    }],
};

/// "Requires a ready Anarch. Lock a ready unlocked Anarch you control. That
/// Anarch recruits or employs an ally or retainer from your hand
/// (requirements and cost apply as normal). This is not an action and
/// cannot be blocked." Named as it is played; the ally cannot act this turn
/// (RBK recruit-ally); the Anarch's Disciplines decide the version (LSJ
/// 20080803); its requirements are not the action's (LSJ 20100725); the
/// same vampire may still employ or recruit normally (LSJ 20080815); no
/// cost reduction keyed to an action (LSJ 20090115-1); the card is not
/// played by the minion (ANK 20201229), nor from hand as an action card
/// Direct Intervention could cancel (RTR 20001020).
pub static PIPER: CardDef = CardDef {
    name: "Piper",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::MasterForMinion,
        requires: &[Requirement::ActorSectIn(&[Sect::Anarch])],
        target: NO_TARGET,
        terms: Some(Terms::RecruitOrEmployFromHand),
        waives_needed: false,
        limit: None,
        effects: &[Effect::LockActor, Effect::EmployNamed],
    }],
    abilities: &[],
    riders: &[],
};

/// "Unique location. You can lock this card as a Nosferatu announces an
/// undirected action; after action resolution, if that action was
/// successful, unlock the acting Nosferatu. You can burn this card (even
/// if it is locked) to move a Nosferatu in torpor to the ready region."
/// (Nosferatu.) Any Methuselah's Nosferatu. The unlock comes after the
/// resolution and any referendum, if the vampire is still Nosferatu (LSJ
/// 20090216, ANK 20180922), with or without the card (ANK 20250120). A
/// vampire leaving torpor mid-action ends what it was the object of: a
/// diablerie or leave-torpor action fizzles (ANK 20220218).
pub static WARSAW_STATION: CardDef = CardDef {
    name: "Warsaw Station",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            windows: &[Window::ActionAsAnnounced, Window::ReactionAsAnnounced],
            actor: Actor::Controller,
            requires: &[
                Requirement::CardUnlocked,
                Requirement::ActingClan("Nosferatu"),
                Requirement::ActionUndirected,
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::LockSelf,
                Effect::UnlockActingIfSuccessful("Nosferatu"),
            ],
        },
        ModeDef {
            windows: &[Window::AnyImpulse],
            actor: Actor::Controller,
            requires: &[],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::TargetClan("Nosferatu"),
                    Requirement::TargetInTorpor,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::BurnSelf, Effect::TargetLeavesTorpor],
        },
    ],
    riders: &[],
};

/// "You can lock this card to give <any minion of the kind> +1 stealth":
/// the acting minion of any Methuselah, while stealth is needed.
macro_rules! any_stealth_location {
    ($name:ident, $printed:expr, $requires:expr, $doc:expr) => {
        #[doc = $doc]
        pub static $name: CardDef = CardDef {
            name: $printed,
            trifle: false,
            out_of_turn: false,
            delayed_replacement: None,
            modes: &[LOCATION],
            abilities: &[ModeDef {
                windows: &[Window::ActionDuring, Window::ReactionDuringAction],
                actor: Actor::Controller,
                requires: $requires,
                target: NO_TARGET,
                terms: None,
                waives_needed: false,
                limit: None,
                effects: &[Effect::LockSelf, Effect::AddStealth(1)],
            }],
            riders: &[],
        };
    };
}

any_stealth_location!(
    ANARCH_RAILROAD,
    "Anarch Railroad",
    &[
        Requirement::CardUnlocked,
        Requirement::ActingSectIn(&[Sect::Anarch]),
        Requirement::StealthNeeded,
    ],
    "\"Unique location. You can lock this card to give an Anarch +1 \
     stealth.\" Any Methuselah's Anarch."
);
any_stealth_location!(
    FORTUNE_TELLER_SHOP,
    "Fortune Teller Shop",
    &[
        Requirement::CardUnlocked,
        Requirement::ActingClan("Ravnos"),
        Requirement::StealthNeeded,
    ],
    "\"Unique location. You can lock this card to give a Ravnos +1 \
     stealth.\" Any Methuselah's Ravnos."
);

/// "Unique location. You can lock this card to give a minion you control
/// +1 intercept. You can lock this card and burn 1 pool to give a minion
/// controlled by another Methuselah +1 intercept." Used holding the impulse,
/// by sequencing: another Methuselah's attempt first, then the impulse
/// passes (ANK 20200607); no additional cost for a card in play (ANK
/// 20221120).
pub static KRCG_NEWS_RADIO: CardDef = CardDef {
    name: "KRCG News Radio",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            windows: &[Window::ActionStealthIntercept],
            actor: Actor::Controller,
            requires: &[
                Requirement::CardUnlocked,
                Requirement::YourAttemptNeedsIntercept,
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::AttemptingIntercept(1)],
        },
        ModeDef {
            windows: &[Window::ActionStealthIntercept],
            actor: Actor::Controller,
            requires: &[
                Requirement::CardUnlocked,
                Requirement::OthersAttemptNeedsIntercept,
                // Not burning a last pool this way (as True Love's Face).
                Requirement::PoolAtLeast(2),
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::LockSelf,
                Effect::PayPool(1),
                Effect::AttemptingIntercept(1),
            ],
        },
    ],
    riders: &[],
};

/// "Unique location. After a vampire with capacity 3 or less goes to
/// torpor, put 1 hostage counter on them. Vampires with any hostage
/// counters cannot be moved to the ready region or be diablerized. Lock
/// during your master phase to add X blood to a ready vampire you control,
/// where X is the number of vampires with any hostage counters. During any
/// unlock phase, any ready vampire can burn 2 blood to burn any vampire's
/// hostage counters. After this card leaves play, burn all the hostage
/// counters." The burn once each turn (ANK 20200729), table-wide; a
/// hostage is no target for leaving torpor, rescue or diablerie.
pub static CARVERS_MEAT_PACKING: CardDef = CardDef {
    name: "Carver's Meat Packing and Storage",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            windows: &[Window::MasterPhase],
            actor: Actor::Controller,
            requires: &[Requirement::CardUnlocked],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::Controlled,
                    Requirement::ActorReady,
                    Requirement::TargetVampire,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::BloodPerHostage],
        },
        ModeDef {
            windows: &[Window::UnlockPhaseAny],
            actor: Actor::AnyReadyVampire,
            requires: &[Requirement::ActorBloodAtLeast(2)],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[Requirement::TargetHostage],
            }),
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerTurn),
            effects: &[Effect::HostBurnBlood(2), Effect::BurnTargetHostages],
        },
    ],
    riders: &[Rider {
        hook: Hook::ReadyMinionBurnedOrTorpor,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::HostageOnTorpor(3)],
    }],
};

/// "Unique location. Hunting ground. During your unlock phase, a ready
/// Salubri you control can gain 1 blood. You can lock this card to cancel a
/// frenzy card as it is played on a Salubri you control (cost is still
/// paid). A vampire can gain blood from only one hunting ground each
/// turn." (Salubri.) The cancel follows the cancel rules: in the as-played
/// window, where nothing but wakes may be used (LSJ 20061207); the card is
/// still played (ANK 20190104) and replaced normally (LSJ 20080630); a
/// limited effect canceled leaves the limit untouched (LSJ 20030224).
pub static MEDITATIVE_GROVE: CardDef = CardDef {
    name: "Meditative Grove",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[LOCATION],
    abilities: &[
        hunting_ground_pick!(
            1,
            &[
                Requirement::TargetVampire,
                Requirement::Controlled,
                Requirement::ActorReady,
                Requirement::TargetClan("Salubri"),
                Requirement::NotHuntingGroundFed,
            ]
        ),
        ModeDef {
            windows: &[Window::CardAsPlayed],
            actor: Actor::Controller,
            requires: &[
                Requirement::CardUnlocked,
                Requirement::PlayedFrenzyOnYour("Salubri"),
            ],
            target: NO_TARGET,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::LockSelf, Effect::CancelPlayed { refund: false }],
        },
    ],
    riders: &[],
};

/// "Only one Week of Nightmares can be played in a game. Put this card in
/// play with 10 nightmare counters. Ravnos get +1 bleed and +1 strength and
/// do not hunt as normal. Any Ravnos can steal 1 blood from another Ravnos
/// as a +1 stealth hunt action. During each Methuselah's unlock phase, that
/// Methuselah can move 1 nightmare counter from this card to a Ravnos. If
/// this card has no counters, each Ravnos burns 1 blood for each nightmare
/// counter on them or is burned, then burn this card and the nightmare
/// counters." (Ravnos.) The special hunt: once a turn by a minion (RBK
/// action-card-or-card-in-play), at another Ravnos even with no blood,
/// never without one (LSJ 19970224); successful only if it steals blood
/// (LSJ 20050720-2), what the thief cannot hold going to the blood bank
/// (LSJ 20010809-1); it requires a Ravnos (LSJ 20080604); other special
/// hunts stay allowed (LSJ 20090606). A Ravnos that must hunt takes it if it
/// can. Paying its nightmare blood when it can, a Ravnos is burned only if
/// it cannot.
pub static WEEK_OF_NIGHTMARES: CardDef = CardDef {
    name: "Week of Nightmares",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[master_in_play(
        &[Requirement::NotPlayedThisGame],
        &[
            Effect::PutInPlay,
            Effect::EnterWithCounters(10),
            Effect::Layer(Modifier::ClanStat {
                clan: "Ravnos",
                stat: Stat::Bleed,
                n: 1,
            }),
            Effect::Layer(Modifier::ClanStat {
                clan: "Ravnos",
                stat: Stat::Strength,
                n: 1,
            }),
            Effect::Layer(Modifier::ClanCannotHunt("Ravnos")),
        ],
    )],
    abilities: &[
        ModeDef {
            windows: &[Window::ActionDeclare],
            actor: Actor::ActingCandidate,
            requires: &[Requirement::ActorClan("Ravnos")],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::TargetVampire,
                    Requirement::TargetClan("Ravnos"),
                    Requirement::OtherThanActor,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::HuntAction,
                Effect::StealBloodFromTarget(1),
            ],
        },
        ModeDef {
            windows: &[Window::UnlockPhaseOwn],
            actor: Actor::Methuselah,
            requires: &[Requirement::CountersAtLeast(1)],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    Requirement::TargetVampire,
                    Requirement::TargetClan("Ravnos"),
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::CounterToTarget],
        },
    ],
    riders: &[],
};
