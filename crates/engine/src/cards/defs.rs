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

use super::vampires::{
    ability, enter_combat, printed_action, DODGE_EACH_COMBAT, MANEUVER_EACH_COMBAT,
};
use super::CostSource;
use super::{
    bleed, mode_of, target, Actor, Among, AsVampire, Bearer, BlockRider, CardDef, CardFilter,
    Effect, Holding, Hook, Limit, ModeDef, Modifier, On, Points, RefEffect, ReferendumDef,
    Requirement, Rider, RiderScope, SeatFilter, Stat, StrikeKind, TargetKind, TargetSpec, Term,
    Terms, TitleClass, Until, VoteGrant, Voters, Window, CARD,
};
use crate::combat::Range;
use crate::crypt::Sect;
use crate::decision::ActionKind;
use crate::library::{CardType, Label};

/// "Move any amount of blood from a vampire you control to your pool." The
/// amount is announced as it is played, from 0 to what the vampire has
/// (rules-qa 38.1), what is left taken at resolution (LSJ 20100216); its
/// cost (Villein, Secure Haven) paid as the pool is gained, no oust in
/// between (LSJ 20020620).
pub static MINION_TAP: CardDef = CardDef {
    name: "Minion Tap",
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::TargetVampire, Requirement::Controlled],
        ),
        terms: Some(Terms::UpToTargetBlood),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[Effect::MoveBloodTargetToPoolX],
        )
    }],
    ..CARD
};

/// "Trifle. Put this card on a vampire you control who has any amount of
/// blood and move 2 to 5 blood from that vampire to your pool. Cards named
/// Minion Tap cost you +1 pool to play. Villein costs +1 pool to play on this
/// vampire."
pub static VILLEIN: CardDef = CardDef {
    name: "Villein",
    trifle: true,
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::TargetVampire,
                Requirement::Controlled,
                Requirement::BloodAtLeast(1),
            ],
        ),
        terms: Some(Terms::BloodRange { min: 2, max: 5 }),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[
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
        )
    }],
    ..CARD
};

/// "Out-of-turn. Cancel a master card as it is played by another Methuselah,
/// and its cost is not paid."
pub static SUDDEN_REVERSAL: CardDef = CardDef {
    name: "Sudden Reversal",
    out_of_turn: true,
    modes: &[mode_of(
        &[Window::CardAsPlayed],
        Actor::Methuselah,
        &[
            Requirement::NotOwnTurn,
            Requirement::PlayedCardIsMaster,
            Requirement::PlayedByOtherSeat,
        ],
        &[Effect::CancelPlayed { refund: true }],
    )],
    ..CARD
};

/// "Only usable during a bleed action. [dom] +2 bleed (limited). [DOM] +3
/// bleed (limited)."
pub static CONDITIONING: CardDef = CardDef {
    name: "Conditioning",
    modes: &[
        mode_of(
            &[Window::ActionDuring],
            Actor::ActingMinion,
            &[Requirement::IsBleed],
            &[bleed(2, true)],
        ),
        mode_of(
            &[Window::ActionDuring],
            Actor::ActingMinion,
            &[Requirement::IsBleed],
            &[bleed(3, true)],
        ),
    ],
    ..CARD
};

/// "Only usable as the action is announced. [obl] Reaction cards cost +1
/// blood or life. [OBL] As above, and those cards are not replaced until the
/// end of the action."
pub static CONSIGN_TO_OBLIVION: CardDef = CardDef {
    name: "Consign to Oblivion",
    modes: &[
        mode_of(
            &[Window::ActionAsAnnounced],
            Actor::ActingMinion,
            &[],
            &[Effect::ActionLayer(Modifier::ReactionBloodCost(1))],
        ),
        mode_of(
            &[Window::ActionAsAnnounced],
            Actor::ActingMinion,
            &[],
            &[
                Effect::ActionLayer(Modifier::ReactionBloodCost(1)),
                Effect::DeferReactionReplacement,
            ],
        ),
    ],
    ..CARD
};

/// "[PRO] [ACTION MODIFIER] Only usable as the action is announced. +1
/// stealth, even if stealth is not yet needed." (The [pro] combat strike
/// mode waits for the combat slice.)
pub static FORM_OF_THE_COBRA: CardDef = CardDef {
    name: "Form of the Cobra",
    modes: &[
        // [pro] [COMBAT] Strike: steal 1 blood or life (becoming blood).
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Combatant,
            &[],
            &[Effect::Strike(StrikeKind::StealBlood {
                n: 1,
                ranged: false,
            })],
        ),
        ModeDef {
            waives_needed: true,
            ..mode_of(
                &[Window::ActionAsAnnounced],
                Actor::ActingMinion,
                &[Requirement::StealthNeeded],
                &[Effect::AddStealth(1)],
            )
        },
    ],
    ..CARD
};

/// "[aus] Only usable during an action directed at you (or a card you
/// control). +2 intercept. [AUS] Only usable by a locked vampire. This
/// vampire wakes."
pub static EYES_OF_ARGUS: CardDef = CardDef {
    name: "Eyes of Argus",
    modes: &[
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::ReactingMinion,
            &[Requirement::DirectedAtYou, Requirement::InterceptNeeded],
            &[Effect::AddIntercept(2)],
        ),
        mode_of(
            &[Window::Wake, Window::CardAsPlayed],
            Actor::ReactingMinion,
            &[Requirement::LockedActor],
            &[Effect::Wake],
        ),
    ],
    ..CARD
};

/// "Only usable if a minion is bleeding you, after blocks are declined.
/// [dom] Lock this reacting vampire. Change the target of the bleed to
/// another Methuselah other than the acting minion's controller (that
/// Methuselah can attempt to block). [DOM] As above, but do not lock this
/// vampire."
pub static DEFLECTION: CardDef = CardDef {
    name: "Deflection",
    modes: &[
        ModeDef {
            target: target(
                TargetKind::Seat,
                &[Requirement::NotActingController, Requirement::NotSelf],
            ),
            ..mode_of(
                &[Window::ActionBlocksDeclined],
                Actor::ReactingMinion,
                &[
                    Requirement::DirectedAtYou,
                    Requirement::IsBleed,
                    Requirement::BlocksDeclined,
                ],
                &[Effect::LockActor, Effect::RedirectBleed],
            )
        },
        ModeDef {
            target: target(
                TargetKind::Seat,
                &[Requirement::NotActingController, Requirement::NotSelf],
            ),
            ..mode_of(
                &[Window::ActionBlocksDeclined],
                Actor::ReactingMinion,
                &[
                    Requirement::DirectedAtYou,
                    Requirement::IsBleed,
                    Requirement::BlocksDeclined,
                ],
                &[Effect::RedirectBleed],
            )
        },
    ],
    ..CARD
};

/// "Out-of-turn. Only usable if a minion is bleeding you and the bleed
/// amount is 4 or more, after blocks are declined. Burn the acting minion.
/// (The action is not successful.)"
pub static ARCHON_INVESTIGATION: CardDef = CardDef {
    name: "Archon Investigation",
    out_of_turn: true,
    modes: &[mode_of(
        &[Window::ActionBlocksDeclined],
        Actor::Methuselah,
        &[
            Requirement::NotOwnTurn,
            Requirement::DirectedAtYou,
            Requirement::IsBleed,
            Requirement::BleedAmountAtLeast(4),
            Requirement::BlocksDeclined,
        ],
        &[Effect::BurnActingMinion, Effect::ActionFails],
    )],
    ..CARD
};

/// A recruit ally mode: the ally enters play in the acting Methuselah's
/// ready region with its printed life, its layers on itself.
const fn recruit(effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionDeclareRecruit],
        actor: Actor::ActingCandidate,
        requires: &[],
        target: None,
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
    modes: &[recruit(&[Effect::DeclareAction(ActionKind::Recruit)])],
    abilities: &[mode_of(
        &[Window::ReactionDuringAction],
        Actor::Controller,
        &[Requirement::IsBleed, Requirement::DirectedAtYou],
        &[Effect::BurnSelf, Effect::ReduceBleed(3)],
    )],
    ..CARD
};

/// "Mortal with 2 life. 0 strength, 0 bleed. This ally can strike: 1R
/// damage. This ally gets 1 optional maneuver each combat. This ally can
/// lock to give a Ravnos you control +1 stealth." The lock gives the
/// stealth to the acting Ravnos, when needed; usable the turn it is
/// recruited (ANK 20180517).
pub static CITY_STAR_TAXI: CardDef = CardDef {
    name: "City Star Taxi",
    modes: &[recruit(&[Effect::DeclareAction(ActionKind::Recruit)])],
    abilities: &[
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Host,
            &[],
            &[Effect::Strike(StrikeKind::Damage {
                n: 1,
                ranged: true,
                aggravated: false,
            })],
        ),
        MANEUVER_EACH_COMBAT,
        mode_of(
            &[Window::ActionDuring],
            Actor::Host,
            &[
                Requirement::CardUnlocked,
                Requirement::OtherActingControlled,
                Requirement::OtherActingClan("Ravnos"),
                Requirement::StealthNeeded,
            ],
            &[Effect::LockSelf, Effect::AddStealth(1)],
        ),
    ],
    ..CARD
};

/// "Unique werewolf with 3 life. 1 strength, 1 bleed. Double Deuce has +1
/// stealth. If Double Deuce has 2 or fewer life during your unlock phase,
/// he gains 1 life." The gain is an unlock-phase rider its controller
/// orders among their own (rules-qa 6.1).
pub static DOUBLE_DEUCE: CardDef = CardDef {
    name: "Double Deuce",
    modes: &[recruit(&[
        Effect::DeclareAction(ActionKind::Recruit),
        Effect::Layer(Modifier::Bearer {
            stat: Stat::Stealth,
            n: 1,
            during: &[],
            who: Bearer::Any,
        }),
    ])],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Controller,
        requires: &[Requirement::HostBloodAtMost(2)],
        effects: &[Effect::AddHostBlood(1)],
    }],
    ..CARD
};

/// "Animal with 1 life. Minions in combat with the employer cannot
/// strike: combat ends." Put on after the opposing strike was chosen, that
/// strike has no effect (ANK 20190412).
pub static DOG_PACK: CardDef = CardDef {
    name: "Dog Pack",
    modes: &[employ(&[
        Effect::DeclareAction(ActionKind::Employ),
        Effect::EmployRetainer { life: 1 },
        Effect::Layer(Modifier::NoCombatEndsAgainstHost),
    ])],
    ..CARD
};

/// "Animal with 1 life. [ani] If the action to employ this retainer is
/// successful, unlock this vampire during the next discard phase. You can
/// lock this retainer to give the employer +1 intercept. [ANI] As above,
/// but unlock this vampire after resolution of this action instead." The
/// intercept goes to the employer's block attempt, when needed.
pub static FERAL_HOUND: CardDef = CardDef {
    name: "Feral Hound",
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
    abilities: &[mode_of(
        &[Window::ActionStealthIntercept],
        Actor::Host,
        &[Requirement::CardUnlocked, Requirement::InterceptNeeded],
        &[Effect::LockSelf, Effect::AddIntercept(1)],
    )],
    ..CARD
};

/// "Ghoul with 2 life. The employer can lock this retainer to prevent 1
/// damage in combat. You can burn this retainer to have an action directed
/// at a minion you control fail." The failure is not usable while a block
/// is attempted or after one succeeded (ANK 20260316, LSJ 20041022); the
/// failed action reached resolution (NRA) and was not blocked (RTR
/// 20180719, LSJ 20070219).
pub static SZLACHTA_BODYGUARD: CardDef = CardDef {
    name: "Szlachta Bodyguard",
    modes: &[employ(&[
        Effect::DeclareAction(ActionKind::Employ),
        Effect::EmployRetainer { life: 2 },
    ])],
    abilities: &[
        mode_of(
            &[Window::CombatDamagePrevention],
            Actor::Host,
            &[Requirement::CardUnlocked, Requirement::DamagePending],
            &[Effect::LockSelf, Effect::Prevent(1)],
        ),
        mode_of(
            &[Window::ReactionDuringAction],
            Actor::Controller,
            &[
                Requirement::DirectedAtYourMinion,
                Requirement::NoBlockAttempt,
            ],
            &[Effect::BurnSelf, Effect::ActionFails],
        ),
    ],
    ..CARD
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
            target: target(
                TargetKind::Card,
                &[Requirement::TargetCard(CardFilter::Location)],
            ),
            ..mode_of(
                &[Window::AnyImpulse],
                Actor::Host,
                &[Requirement::CardUnlocked, Requirement::NotInCombat],
                &[Effect::LockSelf, Effect::BurnTargetCard, Effect::BurnSelf],
            )
        },
    ],
    ..CARD
};

/// "Unique ghoul with 5 life. 3 strength, 0 bleed. After this ally enters
/// play, burn an ally or retainer you control. It can strike: 3R damage. It
/// can enter combat with a minion as a Ⓓ action. During your unlock phase,
/// you can discard an ally or retainer card to add 2 life to this ally."
/// The unlock discard is once per phase (rules-qa 6.6).
pub static VOZHD_OF_SOFIA: CardDef = CardDef {
    name: "Vozhd of Sofia, The",
    modes: &[recruit(&[
        Effect::DeclareAction(ActionKind::Recruit),
        Effect::EntryBurnAllyOrRetainer,
    ])],
    abilities: &[
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Host,
            &[],
            &[Effect::Strike(StrikeKind::Damage {
                n: 3,
                ranged: true,
                aggravated: false,
            })],
        ),
        enter_combat(&[Requirement::NotControlled, Requirement::ActorReady]),
        ModeDef {
            terms: Some(Terms::Discard {
                n: 1,
                of: CardFilter::AllyOrRetainer,
            }),
            ..mode_of(
                &[Window::UnlockPhaseOwn],
                Actor::Host,
                &[Requirement::HandHas(CardFilter::AllyOrRetainer, 1)],
                &[Effect::AddHostBlood(2)],
            )
        },
    ],
    ..CARD
};

/// "Zombie with 1 life. 1 strength, 1 bleed. [obl] This ally cannot have or
/// use equipment or retainers. During a bleed action, you can lock another
/// copy of this ally you control to give this ally +1 bleed. [OBL] As
/// above, and during a bleed action, a Hecata you control can burn 1 blood
/// to give this ally +1 bleed." During its bleed, before resolution (ANK
/// 20231124); a copy recruited this turn may be locked (ANK 20180517).
/// Each ability once per action per copy (the "during" template, rules-qa
/// 6.6, 42.3, 42.9).
pub static BONE_SHAMBLER: CardDef = CardDef {
    name: "Bone Shambler",
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
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::TargetOtherCopy,
                    Requirement::TargetUnlocked,
                ],
            ),
            limit: Some(Limit::PerAction),
            ..mode_of(
                &[Window::ActionDuring],
                Actor::Host,
                &[Requirement::ActorActing, Requirement::IsBleed],
                &[Effect::LockTarget, bleed(1, false)],
            )
        },
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::ActorClan("Hecata"),
                    Requirement::BloodAtLeast(1),
                ],
            ),
            limit: Some(Limit::PerAction),
            ..mode_of(
                &[Window::ActionDuring],
                Actor::Host,
                &[
                    Requirement::SuperiorVersion,
                    Requirement::ActorActing,
                    Requirement::IsBleed,
                ],
                &[Effect::TargetBurnsBlood(1), bleed(1, false)],
            )
        },
    ],
    ..CARD
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
    modes: &[mode_of(
        &[Window::ActionDeclareEmploy],
        Actor::ActingCandidate,
        &[Requirement::ActorSectIn(&[Sect::Anarch])],
        &[
            Effect::DeclareAction(ActionKind::Employ),
            Effect::EmployRetainer { life: 3 },
            Effect::Layer(Modifier::RetainerDamage(1)),
        ],
    )],
    abilities: &[mode_of(
        &[Window::ActionBlocked],
        Actor::Host,
        &[
            Requirement::ActorActing,
            Requirement::NotContinued,
            Requirement::LifeAtLeast(1),
        ],
        &[Effect::BurnOwnLife(1), Effect::ContinueUnblocked],
    )],
    ..CARD
};

/// "Ghoul with 2 life. The employer can burn this retainer to reduce the
/// cost of a ghoul ally requiring a Tzimisce they recruit by 2 blood or
/// pool." During that recruit action, the cost paid at resolution.
pub static SZLACHTA_ASSISTANT: CardDef = CardDef {
    name: "Szlachta Assistant",
    modes: &[employ(&[
        Effect::DeclareAction(ActionKind::Employ),
        Effect::EmployRetainer { life: 2 },
    ])],
    abilities: &[mode_of(
        &[Window::ActionDuring],
        Actor::Host,
        &[
            Requirement::ActorActing,
            Requirement::RecruitingTzimisceGhoul,
        ],
        &[Effect::BurnSelf, Effect::ReduceActionCost(2)],
    )],
    ..CARD
};

/// "Animal with 1 life. 0 strength, 0 bleed. [ani] This ally can burn 1
/// life to give a minion you control 1 press. During an action directed at
/// you (or a card you control), you can burn this ally if it is not
/// blocking to unlock a ready minion you control. [ANI] As above, but this
/// ally has 2 life and 1 strength." The press is usable the turn it is
/// recruited (ANK 20180517).
pub static UNDERBRIDGE_STRAY: CardDef = CardDef {
    name: "Underbridge Stray",
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
        mode_of(
            &[Window::CombatPress],
            Actor::ForOwnCombatant,
            &[Requirement::LifeAtLeast(1)],
            &[Effect::BurnOwnLife(1), Effect::Press],
        ),
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[Requirement::Controlled, Requirement::ActorReady],
            ),
            ..mode_of(
                &[Window::ReactionDuringAction],
                Actor::Host,
                &[Requirement::DirectedAtYou, Requirement::ActorNotBlocking],
                &[Effect::BurnSelf, Effect::UnlockTarget],
            )
        },
    ],
    ..CARD
};

/// "Mortal with 3 life. 1 strength, 0 bleed. Operation Antigen. ECTU
/// Operative can strike: 1R damage. He gets 1 optional maneuver each combat.
/// He can burn a vampire in torpor as a Ⓓ action." (2 pool.) Any vampire in
/// torpor, directed at its controller, undirected at one's own (rules-qa
/// 20.1, 38.2); out of torpor by resolution, the action fizzles.
pub static ECTU_OPERATIVE: CardDef = CardDef {
    name: "ECTU Operative",
    modes: &[recruit(&[Effect::DeclareAction(ActionKind::Recruit)])],
    abilities: &[
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Host,
            &[],
            &[Effect::Strike(StrikeKind::Damage {
                n: 1,
                ranged: true,
                aggravated: false,
            })],
        ),
        MANEUVER_EACH_COMBAT,
        printed_action(
            target(
                TargetKind::Minion,
                &[Requirement::TargetVampire, Requirement::TargetInTorpor],
            ),
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::BurnTargetMinion,
            ],
        ),
    ],
    ..CARD
};

/// "Unique mortal with 3 life. 1 strength, 1 bleed. Operation Antigen.
/// Michael can search your library for an equipment card and equip with it
/// as a +1 stealth action (requirements and cost apply as normal; shuffle
/// afterward). If he is in combat with a vampire, you can burn 1 pool once
/// each round before range is determined to set the range for the round."
/// (3 pool.) The equipment is chosen at resolution, as Magic of the
/// Smith's; an equip action (TOM 19960130), not the same action as
/// equipping that card from hand (ANK 20180817). The range, once set, is
/// not set again that round (RTR 19970630, ANK 20180720): another effect
/// that set it first has priority (PIB 20120214) — which also makes it
/// once a round.
pub static MICHAEL_HARRIS: CardDef = CardDef {
    name: "Michael Harris, the Incinerator",
    modes: &[recruit(&[Effect::DeclareAction(ActionKind::Recruit)])],
    abilities: &[
        printed_action(
            None,
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::SearchEquip,
            ],
        ),
        mode_of(
            &[Window::CombatBeforeRange],
            Actor::Host,
            &[Requirement::OpponentVampire, Requirement::PoolAtLeast(1)],
            &[Effect::PayPool(1), Effect::SetRange(Range::Close)],
        ),
        mode_of(
            &[Window::CombatBeforeRange],
            Actor::Host,
            &[Requirement::OpponentVampire, Requirement::PoolAtLeast(1)],
            &[Effect::PayPool(1), Effect::SetRange(Range::Long)],
        ),
    ],
    ..CARD
};

/// "Unique mortal with 3 life. 1 strength, 1 bleed. Operation Antigen. If
/// Valerie is in combat with a vampire, you can burn 1 pool once each round
/// to make her immune to all non-aggravated damage from a strike or
/// damaging effect. Once each turn after combat involving Valerie ends, she
/// can unlock if the opposing vampire is not ready." (3 pool.) Used as the
/// damage is to be prevented: the opposing strike's, or the other damage
/// pending (rules-qa 38.3); the unlock ordered among the after-combat
/// effects, as Monster's.
pub static VALERIE_DIAZ: CardDef = CardDef {
    name: "Valerie Diaz, Antigen Commander",
    modes: &[recruit(&[Effect::DeclareAction(ActionKind::Recruit)])],
    abilities: &[
        ModeDef {
            limit: Some(Limit::PerRound),
            ..mode_of(
                &[Window::CombatDamagePrevention],
                Actor::Host,
                &[
                    Requirement::OpponentVampire,
                    Requirement::PoolAtLeast(1),
                    Requirement::DamagePending,
                    Requirement::StrikeNormalDamagePending,
                ],
                &[Effect::PayPool(1), Effect::ImmuneStrikeNonAggravated],
            )
        },
        ModeDef {
            limit: Some(Limit::PerRound),
            ..mode_of(
                &[Window::CombatDamagePrevention],
                Actor::Host,
                &[
                    Requirement::OpponentVampire,
                    Requirement::PoolAtLeast(1),
                    Requirement::DamagePending,
                    Requirement::EffectNormalDamagePending,
                ],
                &[Effect::PayPool(1), Effect::ImmuneEffectNonAggravated],
            )
        },
        ability(
            &[Window::ActionAfterCombat],
            Actor::Host,
            // Not while she is unlocked (rules-qa 48.7).
            &[
                Requirement::HostCombatOpposingVampireNotReady,
                Requirement::HostLocked,
            ],
            Limit::PerTurn,
            &[Effect::UnlockHost],
        ),
    ],
    ..CARD
};

/// "Unique mortal with 2 life. 0 strength, 0 bleed. Vagabond Mystic can lock
/// to add 1 life to an ally you control who has fewer life than its
/// starting life. Vagabond Mystic cannot block vampires." Not in the middle
/// of a damage resolution step (LSJ 20001111); starting life is the
/// entering version's (advanced rules 5.5.2).
pub static VAGABOND_MYSTIC: CardDef = CardDef {
    name: "Vagabond Mystic",
    modes: &[recruit(&[
        Effect::DeclareAction(ActionKind::Recruit),
        Effect::Layer(Modifier::CannotBlockVampires),
    ])],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::Controlled, Requirement::TargetAllyHurt],
        ),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[
                Requirement::CardUnlocked,
                Requirement::NotInDamageResolution,
            ],
            &[Effect::LockSelf, Effect::BloodToTarget(1)],
        )
    }],
    ..CARD
};

/// "Animal with 1 life. [ani] The employer gets +1 intercept. [ANI] As
/// above, and while the employer is in combat, the opposing minion's
/// controller gets -1 hand size." The opponent discards down at once and
/// draws back up as the combat ends — again for each new combat (LSJ
/// 20030530); cards not replaced count against the hand size (ANK
/// 20180318).
pub static RAPTOR: CardDef = CardDef {
    name: "Raptor",
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
    ..CARD
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
    modes: &[recruit(&[Effect::DeclareAction(ActionKind::Recruit)])],
    abilities: &[
        mode_of(
            &[Window::ReactionAsAnnounced, Window::ReactionDuringAction],
            Actor::Controller,
            &[Requirement::ActingByPredatorOrPrey],
            &[Effect::BurnSelf, Effect::ReduceStealth(1)],
        ),
        mode_of(
            &[Window::ReactionAsAnnounced],
            Actor::Controller,
            &[Requirement::DirectedAtYourAlly],
            &[Effect::BurnSelf, Effect::ActionFails],
        ),
    ],
    ..CARD
};

/// "Unique ghoul with 5 life. 4 strength, 0 bleed. After this ally enters
/// play, burn an ally or retainer you control. This ally can enter combat
/// with a vampire as a Ⓓ action. Once each combat, this ally can discard a
/// card requiring Protean [pro] to prevent 2 damage. During any other
/// Methuselah's minion phase, a Tzimisce you control can burn 1 blood to
/// unlock this ally." The unlock is offered while the ally is locked.
pub static VOZHD_OF_SZCZECIN: CardDef = CardDef {
    name: "Vozhd of Szczecin, The",
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
            terms: Some(Terms::Discard {
                n: 1,
                of: CardFilter::Discipline("pro"),
            }),
            limit: Some(Limit::PerCombat),
            ..mode_of(
                &[Window::CombatDamagePrevention],
                Actor::Host,
                &[
                    Requirement::DamagePending,
                    Requirement::HandHas(CardFilter::Discipline("pro"), 1),
                ],
                &[Effect::Prevent(2)],
            )
        },
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::ActorClan("Tzimisce"),
                    Requirement::BloodAtLeast(1),
                ],
            ),
            ..mode_of(
                &[Window::AnyImpulse],
                Actor::Controller,
                &[Requirement::OthersMinionPhase, Requirement::CardLocked],
                &[Effect::TargetBurnsBlood(1), Effect::UnlockSelf],
            )
        },
    ],
    ..CARD
};

/// "Zombie with 3 life. 2 strength, 0 bleed. This ally can enter combat
/// with a minion as a Ⓓ action. This ally cannot be the target of directed
/// actions requiring Dominate [dom] or Presence [pre]. In combat, strikes
/// made by this ally cannot be dodged. This ally cannot gain life
/// (instead, any life it would gain goes to the blood bank)." A dodge has
/// no effect on its strikes, but may still be played (LSJ 20030902-2).
pub static AGGRESSIVE_CORPSE: CardDef = CardDef {
    name: "Aggressive Corpse",
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
    ..CARD
};

/// "[pre] Ⓓ Bleed with +1 bleed. [PRE] +1 stealth action. Ⓓ Steal an ally
/// controlled by another Methuselah." The ally changes controller with the
/// cards on it; if it left play before resolution, the action is
/// successful with no effect (PIB 20121031).
pub static ENTRANCEMENT: CardDef = CardDef {
    name: "Entrancement",
    modes: &[
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[],
            &[Effect::DeclareAction(ActionKind::Bleed), bleed(1, false)],
        ),
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[Requirement::NotControlled, Requirement::TargetAlly],
            ),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::ActingCandidate,
                &[],
                &[
                    Effect::DeclareAction(ActionKind::Card),
                    Effect::AddStealth(1),
                    Effect::StealTargetAlly,
                ],
            )
        },
    ],
    ..CARD
};

/// "+1 stealth action. [dom] Ⓓ Steal a retainer controlled by another
/// vampire. [DOM] Ⓓ Steal an ally controlled by another Methuselah." (1
/// blood.) Requirements do not apply to a card taken in play (TOM
/// 19960226-1); a retainer on one of the acting Methuselah's other
/// vampires makes the action undirected (rules-qa 17.4).
pub static FAR_MASTERY: CardDef = CardDef {
    name: "Far Mastery",
    modes: &[
        ModeDef {
            target: target(
                TargetKind::Card,
                &[
                    Requirement::TargetCard(CardFilter::Type(CardType::Retainer)),
                    Requirement::TargetOnVampire,
                    Requirement::NotHost,
                    Requirement::ActorMayHoldTarget,
                ],
            ),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::ActingCandidate,
                &[],
                &[
                    Effect::DeclareAction(ActionKind::Card),
                    Effect::AddStealth(1),
                    Effect::StealTargetCard,
                ],
            )
        },
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[Requirement::NotControlled, Requirement::TargetAlly],
            ),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::ActingCandidate,
                &[],
                &[
                    Effect::DeclareAction(ActionKind::Card),
                    Effect::AddStealth(1),
                    Effect::StealTargetAlly,
                ],
            )
        },
    ],
    ..CARD
};

/// An action modifier of the acting minion's.
const fn modifier(requires: &'static [Requirement], effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionDuring],
        actor: Actor::ActingMinion,
        requires,
        target: None,
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
    modes: &[
        modifier(&[ANARCH], &[Effect::StrengthThisAction(1)]),
        modifier(&[ANARCH], &[Effect::AlliesCannotBlock]),
        modifier(&[ANARCH], &[Effect::AggravatedHandsThisAction]),
    ],
    ..CARD
};

/// "Requires an Anarch. Put this card on an Anarch. Allies cannot block
/// this Anarch. Cards requiring Dominate [dom] or Presence [pre] cost other
/// minions +1 blood while this Anarch is acting, attempting to block or in
/// combat." A Methuselah's card requiring a sect wants a ready vampire of
/// it; the surcharge no longer applies once its block attempt failed, so
/// not to an action card paid at resolution (LSJ 20080818).
pub static LIBERTAS: CardDef = CardDef {
    name: "Libertas",
    modes: &[ModeDef {
        target: target(TargetKind::Minion, &[Requirement::TargetVampire, ANARCH]),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[Requirement::ControlsReadySect(&[Sect::Anarch])],
            &[
                Effect::PutOnTarget,
                Effect::Layer(Modifier::AlliesCannotBlockBearer),
                Effect::Layer(Modifier::DisciplineSurchargeAround(&["dom", "pre"], 1)),
            ],
        )
    }],
    ..CARD
};

/// "[pre] Allies cannot block this action. [PRE] Only usable during a bleed
/// action. As above, with +2 bleed (limited)." (Requires a Ravnos.)
pub static VISIONS_OF_GEHENNA: CardDef = CardDef {
    name: "Visions of Gehenna",
    modes: &[
        modifier(&[], &[Effect::AlliesCannotBlock]),
        modifier(
            &[Requirement::IsBleed],
            &[Effect::AlliesCannotBlock, bleed(2, true)],
        ),
    ],
    ..CARD
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
    modes: &[
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[],
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                bleed(2, false),
                Effect::AlliesCannotBlock,
            ],
        ),
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::TargetVampire,
                    Requirement::ControlledByPredator,
                ],
            ),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::ActingCandidate,
                &[],
                &[Effect::DeclareAction(ActionKind::Card), Effect::PutOnTarget],
            )
        },
    ],
    abilities: &[mode_of(
        &[Window::AnyImpulse],
        Actor::Host,
        &[
            Requirement::OwnMinionPhase,
            Requirement::HostBloodAtLeast(4),
        ],
        &[Effect::HostBurnBlood(4), Effect::BurnSelf],
    )],
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
    ..CARD
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
            target: target(
                TargetKind::Minion,
                &[Requirement::NotControlled, Requirement::ActorReady],
            ),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::Host,
                &[],
                &[
                    Effect::DeclareAction(ActionKind::Ability),
                    Effect::PayLife(1),
                    Effect::EnterCombatWithTarget,
                ],
            )
        },
        mode_of(
            &[Window::CombatPress],
            Actor::Host,
            &[
                Requirement::SuperiorVersion,
                Requirement::FirstRound,
                Requirement::LifeAtLeast(1),
            ],
            &[Effect::BurnOwnLife(1), Effect::Press],
        ),
    ],
    ..CARD
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
        mode_of(
            &[Window::CardAsPlayed],
            Actor::Host,
            &[
                Requirement::PlayedStrikeByOpponent,
                Requirement::LifeAtLeast(1),
            ],
            &[
                Effect::BurnOwnLife(1),
                Effect::CancelPlayed { refund: true },
            ],
        ),
    ],
    ..CARD
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
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::ActorClan("Tzimisce"),
                    Requirement::BloodAtLeast(1),
                ],
            ),
            terms: Some(Terms::LifeFromTarget),
            ..mode_of(
                &[Window::UnlockPhaseOwn],
                Actor::Host,
                &[Requirement::HostBelowStartingLife],
                &[Effect::StealBloodToHostX],
            )
        },
    ],
    ..CARD
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
    ..CARD
};

/// "Animal with 1 life. [ani] While the employer is in combat, the opposing
/// minion's controller plays with an open hand. [ANI] As above, but Owl
/// Companion has 2 life." An open hand is information: the legal view reads
/// it (`GameState::hand_visible_to`).
pub static OWL_COMPANION: CardDef = CardDef {
    name: "Owl Companion",
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
    ..CARD
};

/// An employ retainer mode: the retainer enters play on the acting minion
/// with the life of its `EmployRetainer`, its layers giving the employer
/// what its text says.
const fn employ(effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionDeclareEmploy],
        actor: Actor::ActingCandidate,
        requires: &[],
        target: None,
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
    modes: &[employ(&[
        Effect::DeclareAction(ActionKind::Employ),
        Effect::EmployRetainer { life: 1 },
        EMPLOYER_INTERCEPT,
    ])],
    ..CARD
};

/// "Animal with 1 life. [ani] Murder of Crows inflicts 1R damage on the
/// opposing minion each round of combat during normal strike resolution.
/// [ANI] As above, but Murder of Crows has 2 life." Environmental damage,
/// the retainer its source (ANK 20180612): inflicted against a dodge, not
/// once a combat ends strike ended the combat (RTR 19941109, TOM
/// 19951216), and not again for additional strikes (RTR 19941109).
pub static MURDER_OF_CROWS: CardDef = CardDef {
    name: "Murder of Crows",
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
    ..CARD
};

/// "Animal with 1 life. [ani] The employer gets +1 intercept. [ANI] As
/// above, but Raven Spy has 2 life." The Discipline level used to employ it
/// decides its version, for as long as it is in play (RTR 19941109, ANK
/// 20180612).
pub static RAVEN_SPY: CardDef = CardDef {
    name: "Raven Spy",
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
    ..CARD
};

/// "Ghoul with 2 life. The employer gets +1 intercept." Employed by a
/// Tzimisce (the card's clan requirement).
pub static REVENANT: CardDef = CardDef {
    name: "Revenant",
    modes: &[employ(&[
        Effect::DeclareAction(ActionKind::Employ),
        Effect::EmployRetainer { life: 2 },
        EMPLOYER_INTERCEPT,
    ])],
    ..CARD
};

/// "Unique mortal with 1 life. 0 strength, 3 bleed." Recruited by a Ventrue
/// (the card's clan requirement); its type line is all its text.
pub static POLITICAL_ALLY: CardDef = CardDef {
    name: "Political Ally",
    modes: &[recruit(&[Effect::DeclareAction(ActionKind::Recruit)])],
    ..CARD
};

/// "Retainer with 1 life. [pro] During any Methuselah's unlock phase, the
/// employer can burn 1 blood to unlock. [PRO] As above, but Homunculus has 2
/// life."
pub static HOMUNCULUS: CardDef = CardDef {
    name: "Homunculus",
    modes: &[
        mode_of(
            &[Window::ActionDeclareEmploy],
            Actor::ActingCandidate,
            &[],
            &[
                Effect::DeclareAction(ActionKind::Employ),
                Effect::EmployRetainer { life: 1 },
            ],
        ),
        mode_of(
            &[Window::ActionDeclareEmploy],
            Actor::ActingCandidate,
            &[],
            &[
                Effect::DeclareAction(ActionKind::Employ),
                Effect::EmployRetainer { life: 2 },
            ],
        ),
    ],
    abilities: &[mode_of(
        &[Window::UnlockPhaseAny],
        Actor::Host,
        &[Requirement::HostLocked, Requirement::HostBloodAtLeast(1)],
        &[Effect::HostBurnBloodToUnlock],
    )],
    ..CARD
};

/// "Only usable during a bleed action. [dom] +1 bleed (limited). [DOM] +1
/// stealth and +1 bleed (limited)."
pub static BONDING: CardDef = CardDef {
    name: "Bonding",
    modes: &[
        mode_of(
            &[Window::ActionDuring],
            Actor::ActingMinion,
            &[Requirement::IsBleed],
            &[bleed(1, true)],
        ),
        ModeDef {
            // "+1 stealth and +1 bleed": the stealth is the effect's, so
            // it is playable only while stealth is needed — a block attempt
            // whose intercept matches the current stealth (TOM 19951109,
            // rules-qa 8.1).
            windows: &[Window::ActionDuring, Window::ActionStealthIntercept],
            actor: Actor::ActingMinion,
            requires: &[Requirement::IsBleed, Requirement::StealthNeeded],
            target: None,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::AddStealth(1), bleed(1, true)],
        },
    ],
    ..CARD
};

/// "[pro] +1 stealth. [PRO] +2 stealth." — a pure stealth modifier: only
/// when needed (rules-qa 3.2).
pub static EARTH_CONTROL: CardDef = CardDef {
    name: "Earth Control",
    modes: &[
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::ActingMinion,
            &[Requirement::StealthNeeded],
            &[Effect::AddStealth(1)],
        ),
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::ActingMinion,
            &[Requirement::StealthNeeded],
            &[Effect::AddStealth(2)],
        ),
    ],
    ..CARD
};

/// "[cel] Maneuver or press. [CEL] Maneuver with 1 optional press." The
/// effect matching the step applies (a maneuver in determine range, a
/// press in the press step).
pub static FLASH: CardDef = CardDef {
    name: "Flash",
    modes: &[
        mode_of(
            &[Window::CombatDetermineRange, Window::CombatPress],
            Actor::Combatant,
            &[],
            &[Effect::Maneuver, Effect::Press],
        ),
        mode_of(
            &[Window::CombatDetermineRange],
            Actor::Combatant,
            &[],
            &[Effect::Maneuver, Effect::OptionalPress],
        ),
    ],
    ..CARD
};

/// "Only usable before range is determined. [pot] This vampire gets a
/// strength of 2 this combat. [POT] … strength of 3 this combat."
pub static TORN_SIGNPOST: CardDef = CardDef {
    name: "Torn Signpost",
    modes: &[
        mode_of(
            &[Window::CombatBeforeRange],
            Actor::Combatant,
            &[],
            &[Effect::SetStrength(2)],
        ),
        mode_of(
            &[Window::CombatBeforeRange],
            Actor::Combatant,
            &[],
            &[Effect::SetStrength(3)],
        ),
    ],
    ..CARD
};

/// "[pot] Strike: hand strike at +2 damage. [POT] Strike: hand strike at
/// +3 damage."
pub static ROUNDHOUSE: CardDef = CardDef {
    name: "Roundhouse",
    modes: &[
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Combatant,
            &[],
            &[Effect::Strike(StrikeKind::Hand { bonus: 2 })],
        ),
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Combatant,
            &[],
            &[Effect::Strike(StrikeKind::Hand { bonus: 3 })],
        ),
    ],
    ..CARD
};

/// "[pro] Strike: 2 aggravated damage. [PRO] Strike: 2R aggravated
/// damage." (2 blood.)
pub static BODY_FLARE: CardDef = CardDef {
    name: "Body Flare",
    modes: &[
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Combatant,
            &[],
            &[Effect::Strike(StrikeKind::Damage {
                n: 2,
                ranged: false,
                aggravated: true,
            })],
        ),
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Combatant,
            &[],
            &[Effect::Strike(StrikeKind::Damage {
                n: 2,
                ranged: true,
                aggravated: true,
            })],
        ),
    ],
    ..CARD
};

/// "[tha] Strike, ranged: steal 1 blood or life (becoming blood). [THA]
/// … steal 2 blood or life."
pub static THEFT_OF_VITAE: CardDef = CardDef {
    name: "Theft of Vitae",
    modes: &[
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Combatant,
            &[],
            &[Effect::Strike(StrikeKind::StealBlood {
                n: 1,
                ranged: true,
            })],
        ),
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Combatant,
            &[],
            &[Effect::Strike(StrikeKind::StealBlood {
                n: 2,
                ranged: true,
            })],
        ),
    ],
    ..CARD
};

/// "[pre] Strike: combat ends. [PRE] As above, and unlock this vampire
/// before combat ends." (1 blood.)
pub static MAJESTY: CardDef = CardDef {
    name: "Majesty",
    modes: &[
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Combatant,
            &[],
            &[Effect::Strike(StrikeKind::CombatEnds { unlock: false })],
        ),
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Combatant,
            &[],
            &[Effect::Strike(StrikeKind::CombatEnds { unlock: true })],
        ),
    ],
    ..CARD
};

/// "[cel] Strike: dodge. [CEL] Additional strike (limited)."
pub static SIDE_STRIKE: CardDef = CardDef {
    name: "Side Strike",
    modes: &[
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Combatant,
            &[],
            &[Effect::Strike(StrikeKind::Dodge)],
        ),
        mode_of(
            &[Window::CombatAdditionalStrike],
            Actor::Combatant,
            &[],
            &[Effect::AdditionalStrikes {
                n: 1,
                limited: true,
            }],
        ),
    ],
    ..CARD
};

/// "[cel] Additional strike (limited). [CEL] 2 additional strikes
/// (limited)." (1 blood.)
pub static BLUR: CardDef = CardDef {
    name: "Blur",
    modes: &[
        mode_of(
            &[Window::CombatAdditionalStrike],
            Actor::Combatant,
            &[],
            &[Effect::AdditionalStrikes {
                n: 1,
                limited: true,
            }],
        ),
        mode_of(
            &[Window::CombatAdditionalStrike],
            Actor::Combatant,
            &[],
            &[Effect::AdditionalStrikes {
                n: 2,
                limited: true,
            }],
        ),
    ],
    ..CARD
};

/// "Grapple. Only usable at close range before strikes are chosen. A
/// vampire can play only one Immortal Grapple each round. [pot] Strikes
/// that are not hand strikes cannot be used this round (by either
/// combatant). [POT] As above, with 1 optional press. If another round of
/// combat occurs, that round is at close range (skip the determine range
/// step for that round)."
pub static IMMORTAL_GRAPPLE: CardDef = CardDef {
    name: "Immortal Grapple",
    modes: &[
        ModeDef {
            limit: Some(Limit::PerRound),
            ..mode_of(
                &[Window::CombatBeforeStrikes],
                Actor::Combatant,
                &[Requirement::RangeClose],
                &[Effect::OnlyHandStrikes],
            )
        },
        ModeDef {
            limit: Some(Limit::PerRound),
            ..mode_of(
                &[Window::CombatBeforeStrikes],
                Actor::Combatant,
                &[Requirement::RangeClose],
                &[
                    Effect::OnlyHandStrikes,
                    Effect::OptionalPress,
                    Effect::NextRoundClose,
                ],
            )
        },
    ],
    ..CARD
};

/// "[for] Prevent X+1 damage. [FOR] As above, with 1 optional press." X is
/// the blood cost, chosen as the card is played (rules-qa 3.5); it may
/// exceed what is needed, and the card needs damage to prevent (rulings).
pub static HIDDEN_STRENGTH: CardDef = CardDef {
    name: "Hidden Strength",
    modes: &[
        ModeDef {
            terms: Some(Terms::XBlood),
            ..mode_of(
                &[Window::CombatDamagePrevention],
                Actor::Combatant,
                &[Requirement::DamagePending],
                &[Effect::PreventXPlus(1)],
            )
        },
        ModeDef {
            terms: Some(Terms::XBlood),
            ..mode_of(
                &[Window::CombatDamagePrevention],
                Actor::Combatant,
                &[Requirement::DamagePending],
                &[Effect::PreventXPlus(1), Effect::OptionalPress],
            )
        },
    ],
    ..CARD
};

/// "A vampire can play only one Soak each round. [for] Prevent 2
/// non-aggravated damage. [FOR] Prevent 4 non-aggravated damage."
pub static SOAK: CardDef = CardDef {
    name: "Soak",
    modes: &[
        ModeDef {
            limit: Some(Limit::PerRound),
            ..mode_of(
                &[Window::CombatDamagePrevention],
                Actor::Combatant,
                &[Requirement::NonAggravatedDamagePending],
                &[Effect::PreventNonAggravated(2)],
            )
        },
        ModeDef {
            limit: Some(Limit::PerRound),
            ..mode_of(
                &[Window::CombatDamagePrevention],
                Actor::Combatant,
                &[Requirement::NonAggravatedDamagePending],
                &[Effect::PreventNonAggravated(4)],
            )
        },
    ],
    ..CARD
};

/// "[for] Prevent 1 damage. [FOR] Press, or prevent 1 damage with 1
/// optional press." In the press step only the press applies.
pub static INDOMITABILITY: CardDef = CardDef {
    name: "Indomitability",
    modes: &[
        mode_of(
            &[Window::CombatDamagePrevention],
            Actor::Combatant,
            &[Requirement::DamagePending],
            &[Effect::Prevent(1)],
        ),
        mode_of(
            &[Window::CombatDamagePrevention, Window::CombatPress],
            Actor::Combatant,
            &[Requirement::DamagePending],
            &[Effect::Prevent(1), Effect::OptionalPress, Effect::Press],
        ),
    ],
    ..CARD
};

/// "[pro] Damage from this vampire's hand strikes is aggravated this round.
/// [PRO] Maneuver." (1 blood.) The basic mode has no printed step: any
/// impulse before the strike resolution it affects (rules-qa 2.2).
pub static CLAWS_OF_THE_DEAD: CardDef = CardDef {
    name: "Claws of the Dead",
    modes: &[
        mode_of(
            &[Window::CombatUntimed],
            Actor::Combatant,
            &[],
            &[Effect::AggravatedHands],
        ),
        mode_of(
            &[Window::CombatDetermineRange],
            Actor::Combatant,
            &[],
            &[Effect::Maneuver],
        ),
    ],
    ..CARD
};

/// "Only usable at the end of a round of combat. Not usable by a vampire
/// being burned or going to torpor. A vampire can play only one Taste of
/// Vitae each round. This vampire gains blood equal to the amount of blood
/// lost by the opposing vampire to damage this round."
pub static TASTE_OF_VITAE: CardDef = CardDef {
    name: "Taste of Vitae",
    modes: &[ModeDef {
        limit: Some(Limit::PerRound),
        ..mode_of(
            &[Window::CombatEndOfRound],
            Actor::Combatant,
            &[
                Requirement::ActorVampire,
                Requirement::OpponentVampire,
                Requirement::ActorReady,
            ],
            &[Effect::GainBloodLostByOpponent],
        )
    }],
    ..CARD
};

/// "[aus] Press, only usable to continue combat. If another round of combat
/// occurs, this vampire gets 1 optional maneuver that round. [AUS] Only
/// usable if both combatants are still ready and combat would end.
/// Instead, start a new round." (1 blood.)
pub static TELEPATHIC_TRACKING: CardDef = CardDef {
    name: "Telepathic Tracking",
    modes: &[
        mode_of(
            &[Window::CombatPress],
            Actor::Combatant,
            &[Requirement::NoContinuePress],
            &[Effect::Press, Effect::NextRoundManeuver],
        ),
        mode_of(
            &[Window::CombatWouldEnd],
            Actor::Combatant,
            &[
                Requirement::BothCombatantsReady,
                Requirement::CombatWouldEnd,
            ],
            &[Effect::NewRound],
        ),
    ],
    ..CARD
};

/// "[ani] Only usable by a locked vampire who has blocked, after block
/// resolution. Unlock this vampire. [ANI] +1 intercept." The basic mode's
/// point is the after-combat window (rules-qa 2.1).
pub static CATS_GUIDANCE: CardDef = CardDef {
    name: "Cats' Guidance",
    modes: &[
        mode_of(
            &[Window::ActionAfterCombat],
            Actor::ReactingMinion,
            &[Requirement::LockedActor, Requirement::ActorHasBlocked],
            &[Effect::UnlockActor],
        ),
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::ReactingMinion,
            &[Requirement::InterceptNeeded],
            &[Effect::AddIntercept(1)],
        ),
    ],
    ..CARD
};

/// "[pro] Strike: dodge. [PRO] Strike: combat ends. After combat ends, if
/// this vampire was blocked, they can burn 1 blood to continue the action
/// with +1 stealth as if unblocked, even if stealth is not yet needed. A
/// vampire can play only one Form of Mist at superior each action."
pub static FORM_OF_MIST: CardDef = CardDef {
    name: "Form of Mist",
    modes: &[
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Combatant,
            &[],
            &[Effect::Strike(StrikeKind::Dodge)],
        ),
        ModeDef {
            limit: Some(Limit::PerAction),
            ..mode_of(
                &[Window::CombatStrikeDeclaration],
                Actor::Combatant,
                &[],
                &[
                    Effect::Strike(StrikeKind::CombatEnds { unlock: false }),
                    Effect::ContinueAfterCombat {
                        blood: 1,
                        stealth: 1,
                    },
                ],
            )
        },
    ],
    ..CARD
};

// ---- Politics (slice 4) -------------------------------------------------

/// A political action card's single mode: declare the political action and
/// call `r` on success.
macro_rules! political {
    ($name:ident, $printed:expr, $doc:expr, $requires:expr, $r:expr) => {
        #[doc = $doc]
        pub static $name: CardDef = CardDef {
            name: $printed,
            modes: &[mode_of(
                &[Window::ActionDeclarePolitical],
                Actor::ActingCandidate,
                $requires,
                &[
                    Effect::DeclareAction(ActionKind::Political),
                    Effect::CallReferendum(&$r),
                ],
            )],
            ..CARD
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
    non_anarch_titles_minus: false,
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
    non_anarch_titles_minus: false,
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
    non_anarch_titles_minus: false,
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
    non_anarch_titles_minus: false,
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
    non_anarch_titles_minus: false,
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
    modes: &[
        mode_of(
            &[Window::Referendum],
            Actor::ActingMinion,
            &[Requirement::PollingStep],
            &[Effect::AddVotes(2)],
        ),
        mode_of(
            &[Window::Referendum],
            Actor::ActingMinion,
            &[Requirement::PollingStep],
            &[Effect::AddVotes(4)],
        ),
    ],
    ..CARD
};

const HAS_CAST_VOTES: Option<TargetSpec> = target(TargetKind::Minion, &[Requirement::HasCastVotes]);

/// "Only usable during the polling step of a political action. [cel][pre]
/// Choose a vampire who has cast votes or ballots in this referendum. The
/// chosen vampire is locked and abstains (this cancels the chosen vampire's
/// votes and ballots). [CEL][PRE] As above, and the chosen vampire burns 1
/// blood." Action Modifier/Reaction: the acting minion, or a reacting one.
pub static SCALPEL_TONGUE: CardDef = CardDef {
    name: "Scalpel Tongue",
    modes: &[
        ModeDef {
            target: HAS_CAST_VOTES,
            ..mode_of(
                &[Window::Referendum],
                Actor::ActingOrReactingMinion,
                &[Requirement::PollingStep],
                &[Effect::Abstain {
                    lock: true,
                    burn: 0,
                }],
            )
        },
        ModeDef {
            target: HAS_CAST_VOTES,
            ..mode_of(
                &[Window::Referendum],
                Actor::ActingOrReactingMinion,
                &[Requirement::PollingStep],
                &[Effect::Abstain {
                    lock: true,
                    burn: 1,
                }],
            )
        },
    ],
    ..CARD
};

static RECKLESS_AGITATION_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::Allocate {
        points: Points::Fixed(6),
        among: Among::Others,
        min_seats: 2,
    }],
    effect: RefEffect::BurnAllocated,
    grants: &[],
    non_anarch_titles_minus: false,
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
    non_anarch_titles_minus: false,
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
    non_anarch_titles_minus: false,
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
    non_anarch_titles_minus: false,
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
    non_anarch_titles_minus: false,
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
    non_anarch_titles_minus: false,
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
    non_anarch_titles_minus: false,
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
        mode_of(&[Window::Referendum], $actor, $requires, $effects)
    };
}

/// "Only usable during the polling step of a political action. This
/// Lasombra gets +3 votes." Action Modifier/Reaction.
pub static OMINOUS_CHORUS: CardDef = CardDef {
    name: "Ominous Chorus",
    modes: &[polling_mode!(
        Actor::ActingOrReactingMinion,
        &[Requirement::PollingStep],
        &[Effect::AddVotes(3)]
    )],
    ..CARD
};

/// "[pre] Only usable during the polling step of a political action. This
/// vampire gets +3 votes. [PRE] Allies and younger vampires get -1
/// intercept." The superior half is playable any time during the action
/// before resolution (LSJ 20020612).
pub static PERFECT_PARAGON: CardDef = CardDef {
    name: "Perfect Paragon",
    modes: &[
        polling_mode!(
            Actor::ActingMinion,
            &[Requirement::PollingStep],
            &[Effect::AddVotes(3)]
        ),
        mode_of(
            &[Window::ActionDuring],
            Actor::ActingMinion,
            &[],
            &[Effect::YoungerIntercept(-1)],
        ),
    ],
    ..CARD
};

/// "Do not replace until your next unlock phase. [obf] Only usable during a
/// bleed action. +1 bleed (limited). [OBF] Only usable during the polling
/// step of a political action. This vampire gets +2 votes."
pub static OLD_FRIENDS: CardDef = CardDef {
    name: "Old Friends",
    delayed_replacement: Some(Until::NextUnlockPhase),
    modes: &[
        mode_of(
            &[Window::ActionDuring],
            Actor::ActingMinion,
            &[Requirement::IsBleed],
            &[bleed(1, true)],
        ),
        polling_mode!(
            Actor::ActingMinion,
            &[Requirement::PollingStep],
            &[Effect::AddVotes(2)]
        ),
    ],
    ..CARD
};

/// "Requires an Anarch. [obf] Reduce a bleed against you by 2. [pre] Only
/// usable during the polling step of a political action. This vampire gets
/// +2 votes. [pro] +1 intercept."
pub static PARTY_OUT_OF_BOUNDS: CardDef = CardDef {
    name: "Party Out Of Bounds",
    modes: &[
        mode_of(
            &[Window::ReactionDuringAction],
            Actor::ReactingMinion,
            &[
                Requirement::ActorSectIn(&[Sect::Anarch]),
                Requirement::DirectedAtYou,
                Requirement::IsBleed,
            ],
            &[Effect::ReduceBleed(2)],
        ),
        polling_mode!(
            Actor::ReactingMinion,
            &[
                Requirement::ActorSectIn(&[Sect::Anarch]),
                Requirement::PollingStep
            ],
            &[Effect::AddVotes(2)]
        ),
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::ReactingMinion,
            &[
                Requirement::ActorSectIn(&[Sect::Anarch]),
                Requirement::InterceptNeeded,
            ],
            &[Effect::AddIntercept(1)],
        ),
    ],
    ..CARD
};

/// "Requires a primogen. Only usable during a bleed action against you or
/// during the polling step of a political action. Reduce the bleed amount
/// by 3 or this vampire gains +3 votes against the referendum." The votes
/// force the vampire's cast against; unusable by one voting in favor (LSJ
/// 20031115, 20031119).
pub static PROTECTED_DISTRICT: CardDef = CardDef {
    name: "Protected District",
    modes: &[mode_of(
        &[Window::Referendum, Window::ReactionDuringAction],
        Actor::ReactingMinion,
        &[
            Requirement::ActorTitleIn(&[TitleClass::Primogen]),
            Requirement::AnyOf(&[
                &[Requirement::DirectedAtYou, Requirement::IsBleed],
                &[Requirement::PollingStep, Requirement::ActorNotVotingFor],
            ]),
        ],
        &[Effect::ReduceBleed(3), Effect::AddVotesAgainst(3)],
    )],
    ..CARD
};

/// "Only usable during the polling step of a political action. Methuselahs
/// casting (including controlling a minion casting) votes or ballots
/// against the referendum burn 1 pool once results are tallied." Reaches
/// votes cast before it was played; 1 pool per Scorn per Methuselah (RTR
/// 19951110).
pub static SCORN_OF_ADONIS: CardDef = CardDef {
    name: "Scorn of Adonis",
    modes: &[polling_mode!(
        Actor::ActingMinion,
        &[Requirement::PollingStep],
        &[Effect::TallyPenaltyAgainst(1)]
    )],
    ..CARD
};

const ANY_VAMPIRE: Option<TargetSpec> = target(TargetKind::Minion, &[]);

/// "Only usable during the polling step of a political action. [aus] Cancel
/// the referendum. If you played a political action card to call this
/// referendum, return it to its owner's hand (discard down afterward).
/// [AUS] Force a vampire to abstain (this can cancel their votes and
/// ballots)."
pub static TELEPATHIC_VOTE_COUNTING: CardDef = CardDef {
    name: "Telepathic Vote Counting",
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
            target: ANY_VAMPIRE,
            ..mode_of(
                &[Window::Referendum],
                Actor::ActingMinion,
                &[Requirement::PollingStep],
                &[Effect::Abstain {
                    lock: false,
                    burn: 0,
                }],
            )
        },
    ],
    ..CARD
};

/// "Only usable during the polling step of a political action. Cancel the
/// referendum. Unlock the acting vampire. If a political action card was
/// played to call this referendum, return it to its owner's hand (discard
/// down afterward). Minions controlled by the acting Methuselah cannot
/// perform the same political action again this turn." The acting minion
/// has still taken its political action (RTR 19970630).
pub static DELAYING_TACTICS: CardDef = CardDef {
    name: "Delaying Tactics",
    modes: &[polling_mode!(
        Actor::ReactingMinion,
        &[Requirement::PollingStep],
        &[Effect::CancelReferendum {
            unlock_acting: true,
            forbid_repeat: true,
        }]
    )],
    ..CARD
};

/// "Requires a prince, justicar or Inner Circle member. Only usable during
/// the polling step of a political action before votes and ballots are
/// cast. Non-Camarilla vampires cannot cast votes or ballots during this
/// referendum."
pub static CLOSED_SESSION: CardDef = CardDef {
    name: "Closed Session",
    modes: &[polling_mode!(
        Actor::ActingMinion,
        &[
            Requirement::ActorTitleIn(&[TitleClass::Prince, TitleClass::Justicar]),
            Requirement::PollingStep,
            Requirement::NoVotesCast
        ],
        &[Effect::BarNonCamarillaVotes]
    )],
    ..CARD
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
    modes: &[mode_of(
        &[Window::ActionAfterResolution],
        Actor::ActingMinion,
        &[Requirement::IsPolitical, Requirement::ReferendumPassed],
        &[Effect::DistributeMargin {
            clan: "Lasombra",
            pool: 1,
            titled_pool: 2,
        }],
    )],
    ..CARD
};

/// "Only usable after resolution of a political action whose referendum
/// passed. [pre] This vampire gains 1 blood for each vote by which the
/// referendum passed. [PRE] As above, but move up to 2 of those blood to
/// your pool instead of this vampire."
pub static VOTER_CAPTIVATION: CardDef = CardDef {
    name: "Voter Captivation",
    modes: &[
        mode_of(
            &[Window::ActionAfterResolution],
            Actor::ActingMinion,
            &[Requirement::IsPolitical, Requirement::ReferendumPassed],
            &[Effect::GainBloodPerMargin],
        ),
        ModeDef {
            terms: Some(Terms::UpToMargin(2)),
            ..mode_of(
                &[Window::ActionAfterResolution],
                Actor::ActingMinion,
                &[Requirement::IsPolitical, Requirement::ReferendumPassed],
                &[Effect::GainBloodPerMargin],
            )
        },
    ],
    ..CARD
};

pub static DEFS: [&CardDef; 454] = [
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
    &LIVING_MANSE,
    &SWORD_OF_THE_ARCHANGEL,
    &BLOOD_FURY,
    &NO_TRACE,
    &SHADOW_SHIFT,
    &WOLF_CLAWS,
    &RAPID_CHANGE,
    &DIVERSION,
    &LIGHTNING_REFLEXES,
    &WIND_DANCE,
    &ANTICIPATION,
    &TERROR_FRENZY,
    &TRANQUILITY_SHIELD,
    &FLESH_OF_MARBLE,
    &WEATHER_CONTROL,
    &DISARM,
    &HIDE_THE_MIND,
    &ANGELS_GIFT,
    &CONTRABAND,
    &MARTYRS_RESILIENCE,
    &THREATS,
    &FORGOTTEN_LABYRINTH,
    &GO_GETTER,
    &ELDER_IMPERSONATION,
    &BEAST_MELD,
    &PATHS_IN_TWO_WORLDS,
    &DAWN_OPERATION,
    &DOGGED_PURSUIT,
    &ELDER_INTERVENTION,
    &FORCED_AWAKENING,
    &FORCED_VIGILANCE,
    &MELANGE,
    &RATS_WARNING,
    &REDIRECTION,
    &SENSE_THE_SAVAGE_WAY,
    &TELEPATHIC_COUNTER,
    &SCOUTING_MISSION,
    &SOCIAL_CHARM,
    &PUBLIC_TRUST,
    &RESTORATION,
    &FIFTH_TRADITION,
    &FOURTH_TRADITION,
    &UMBROUS_CLUTCH,
    &HUNTING_THE_BEAST,
    &MAKE_THE_MISERE,
    &MIND_NUMB,
    &PROPAGANDA,
    &CONCEAL,
    &DOMINATE_KINE,
    &GRAVEROBBING,
    &ARMY_OF_RATS,
    &JUDGMENT_CAMARILLA_SEGREGATION,
    &TIER_OF_SOULS,
    &PHANTASMAGORIA,
    &ARANTHEBES,
    &FEE_STAKE_BOSTON,
    &FEE_STAKE_NEW_YORK,
    &FEE_STAKE_SEATTLE,
    &WATERS_OF_DUAT,
    &SPLIT_THE_VEIL,
    &PSYCHOPHAGIA,
    &KHABAR_GLORY,
    &OPEN_WAR,
    &THE_PARTHENON,
    &DUMMY_CORPORATION,
    &FERRAILLE,
    &OXFORD_UNIVERSITY,
    &THE_BLACK_THRONE,
    &KUMPANIA,
    &SIGHT_BEYOND_SIGHT,
    &CHARISMA,
    &GIANTS_BLOOD,
    &GOLCONDA_INNER_PEACE,
    &GROOMING_THE_PROTEGE,
    &GANGREL_REVEL,
    &KHABAR_COMMUNITY,
    &NEW_CARTHAGE,
    &MOB_CONNECTIONS,
    &FRAGMENT_OF_THE_BOOK_OF_NOD,
    &BRUJAH_DEBATE,
    &WMRH_TALK_RADIO,
    &THE_RUMOR_MILL,
    &MONSTER,
    &DABBLER,
    &DEAD_POOL,
    &ALAMUT,
    &CAPPADOCIAN_CRYPT,
    &POWERBASE_MUNICH,
    &TWILIGHT_CAMP,
    &THE_MITHRAEUM,
    &MITHRAIC_CULTIST,
    &SECURE_HAVEN,
    &YORUBA_SHRINE,
    &CAVE_OF_APPLES,
    &BRITISH_MUSEUM,
    &THE_LINE,
    &THE_SHARD,
    &RAVNOS_CACHE,
    &RAVNOS_CARNIVAL,
    &FLESHFORGE_CHAMBER,
    &MINION_TAP,
    &RITUAL_GOBLET,
    &FLAMING_CANDLE,
    &ENHANCED_COAGULANT,
    &ECTU_OPERATIVE,
    &MICHAEL_HARRIS,
    &VALERIE_DIAZ,
    &MOBILE_HQ,
    &MALKAVIAN_JUSTICAR,
    &ARCHON,
    &RAISING_THE_PORTCULLIS,
    &DISPUTED_TERRITORY,
    &REVOLUTIONARY_COUNCIL,
    &NARROW_MINDS,
    &TORPID_BLOOD,
    &SLOW_WITHERING,
    &BLOOD_WEAKENS,
    &THIRST,
    &WORMWOOD,
    &FUELED_BY_HEARTS_BLOOD,
    &FOURTH_CYCLE,
    &UNCOILING,
    &BLOOD_TRADE,
    &CONQUEST_OF_HUMANITY,
    &NEW_INQUISITION,
    &NIGHTMARES_UPON_NIGHTMARES,
    &RECALLED_TO_THE_FOUNDER,
    &RESTRICTED_VITAE,
    &FBI_SPECIAL_AFFAIRS_DIVISION,
    &FALL_OF_THE_CAMARILLA,
    &FALL_OF_THE_SABBAT,
    &RISE_OF_THE_NEPHTALI,
    &ABSIMILIARDS_ARMY,
    &ENNOIAS_THEATER,
    &DONNYBROOK,
    &HELL_FOR_LEATHER,
    &MURMUR_OF_THE_FALSE_WILL,
    &SAATET_TA,
    &CARLTON_VAN_WYK,
    &JAKE_WASHINGTON,
    &POWERBASE_SAVANNAH,
    &ASHUR_TABLETS,
    &ROTSCHRECK,
    &LIFE_BOON,
    &ZILLAHS_VALLEY,
    &POLITICAL_STRANGLEHOLD,
    &ANCIENT_INFLUENCE,
    &REINS_OF_POWER,
    &PERMANENT_VACATION,
    &MONASTERY_OF_SHADOWS,
    &FORCED_MARCH,
    &INSTANTANEOUS_TRANSFORMATION,
    &FEAR_OF_MEKHET,
    &ENKIL_COG,
    &CONFISCATION,
    &REINFORCEMENTS,
    &AWE,
    &CHARISMATIC_AURA,
    &LOUP,
    &THE_LOUVRE,
    &CAMARILLA_CONCLAVE,
    &BUSINESS_PRESSURE,
];

// ---- Cards in play with riders and granted actions (slice 5) ------------

static BURN_THIS_CARD: ReferendumDef = ReferendumDef {
    terms: &[],
    effect: RefEffect::BurnSource,
    grants: &[],
    non_anarch_titles_minus: false,
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
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[],
        &[Effect::PutInPlay],
    )],
    abilities: &[mode_of(
        &[Window::ActionDeclarePolitical],
        Actor::ActingCandidate,
        &[Requirement::ActorVampire],
        &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::AddStealth(1),
            Effect::CallReferendum(&BURN_THIS_CARD),
        ],
    )],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::UnlockBurnPoolUnlessReady {
            sect: Sect::Anarch,
            pool: 1,
        }],
    }],
    ..CARD
};

/// "Put this card on a ready vampire you control. This vampire gets +1
/// intercept during bleed actions directed at you. This vampire can prevent
/// 1 damage each combat. If this vampire is in torpor, burn this card." The
/// prevention is an in-play ability of the host (once per combat per copy,
/// no as-played window); the burn fires as the host goes to torpor.
pub static GUARDIAN_ANGEL: CardDef = CardDef {
    name: "Guardian Angel",
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::TargetVampire,
                Requirement::Controlled,
                Requirement::ActorReady,
            ],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[
                Effect::PutOnTarget,
                Effect::Layer(Modifier::HostInterceptOnBleedAtYou(1)),
            ],
        )
    }],
    abilities: &[ModeDef {
        limit: Some(Limit::PerCombat),
        ..mode_of(
            &[Window::CombatDamagePrevention],
            Actor::Host,
            &[Requirement::DamagePending],
            &[Effect::Prevent(1)],
        )
    }],
    riders: &[Rider {
        hook: Hook::HostToTorpor,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::BurnSelf],
    }],
    ..CARD
};

/// "Unique. Put this card on a ready vampire. After this vampire goes to
/// torpor, their controller burns 3 pool. During each Methuselah's unlock
/// phase, if this vampire is in torpor, that Methuselah burns 1 pool." Any
/// ready vampire, any controller; the 3 pool burn fires where the torpor
/// happens (rules-qa 6.4) and may oust its controller mid-combat; the
/// unlock rider is every Methuselah's own, read at their unlock phase.
pub static FAME: CardDef = CardDef {
    name: "Fame",
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::TargetVampire, Requirement::ActorReady],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[Effect::PutOnTarget],
        )
    }],
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
    ..CARD
};

/// "Unique. Put this card in play. After a ready minion is burned or sent
/// to torpor, their controller burns 1 pool. Methuselahs can use a master
/// phase action and discard two master cards to burn this card." The
/// rider fires where the burn or torpor happens (rules-qa 6.4), any
/// controller's minion; the burn is an ability of any Methuselah in their
/// own master phase, its two discards announced with the use.
pub static TENSION_IN_THE_RANKS: CardDef = CardDef {
    name: "Tension in the Ranks",
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[],
        &[Effect::PutInPlay],
    )],
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 2,
            of: CardFilter::Type(CardType::Master),
        }),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[
                Requirement::MasterPhaseActionLeft,
                Requirement::HandHas(CardFilter::Type(CardType::Master), 2),
            ],
            &[Effect::SpendMasterPhaseAction, Effect::BurnSelf],
        )
    }],
    riders: &[Rider {
        hook: Hook::ReadyMinionBurnedOrTorpor,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::TheirControllerBurnsPool(1)],
    }],
    ..CARD
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
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[],
        &[Effect::PutInPlay],
    )],
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::ActingCandidate,
        &[Requirement::ActorVampire],
        &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::BurnSource,
        ],
    )],
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
    ..CARD
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
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[],
        &[Effect::PutInPlay],
    )],
    abilities: &[
        mode_of(
            &[Window::AnyImpulse],
            Actor::Controller,
            &[Requirement::CardUnlocked],
            &[
                Effect::LockSelf,
                Effect::AddCounter(1),
                Effect::HandSizeBonusUntilEndOfTurn(2),
                Effect::BurnSelfAtCounters(3),
            ],
        ),
        mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Controller,
            &[Requirement::CardUnlocked, Requirement::HasEdge],
            &[
                Effect::LockSelf,
                Effect::AddCounter(1),
                Effect::GainPool(1),
                Effect::BurnSelfAtCounters(3),
            ],
        ),
        ModeDef {
            target: target(TargetKind::Uncontrolled, &[]),
            ..mode_of(
                &[Window::AnyImpulse],
                Actor::Controller,
                &[Requirement::CardUnlocked],
                &[
                    Effect::LockSelf,
                    Effect::AddCounter(1),
                    Effect::BloodToUncontrolledTarget(1),
                    Effect::BurnSelfAtCounters(3),
                ],
            )
        },
    ],
    ..CARD
};

/// "Unique location. You can lock this card to discard a card (draw up
/// afterward)." An untimed lock ability of the controller (any impulse
/// but the as-played window: RTR 20040501, ANK 20200616); the card to
/// discard is announced with the use and replaced.
pub static THE_BARRENS: CardDef = CardDef {
    name: "Barrens, The",
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[],
        &[Effect::PutInPlay],
    )],
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Any,
        }),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Controller,
            &[Requirement::CardUnlocked, Requirement::HandNotEmpty],
            &[Effect::LockSelf],
        )
    }],
    ..CARD
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
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[],
        &[Effect::PutInPlay],
    )],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::TargetVampire,
                Requirement::Controlled,
                Requirement::ActorReady,
            ],
        ),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Controller,
            &[Requirement::CardUnlocked],
            &[Effect::LockSelf, Effect::BloodToTarget(2)],
        )
    }],
    riders: &[Rider {
        hook: Hook::DiscardPhase,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::PredatorTakesControl],
    }],
    ..CARD
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
    modes: &[mode_of(
        &[Window::ActionDeclare],
        Actor::ActingCandidate,
        &[Requirement::ActorSectIn(&[Sect::Anarch])],
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
            Effect::EnterWithCounters(1),
        ],
    )],
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::ActingCandidate,
        &[Requirement::ActorVampire, Requirement::PoolAtLeast(1)],
        &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::PayPool(1),
            Effect::BurnSource,
        ],
    )],
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
    ..CARD
};

/// Nosferatu. "+1 stealth action. X is the number of copies of this card
/// you control. Put this card in play. During your unlock phase, your prey
/// burns 1 pool. Minions can burn this card as a Ⓓ action." X is read when the cost is
/// paid (copies already in play); the unlock-phase burn is the controller's
/// mandatory effect (rules-qa 6.1); the burn action is directed at the
/// controller (undirected, +1 stealth, for their own minions).
pub static CREEPING_SABOTAGE: CardDef = CardDef {
    name: "Creeping Sabotage",
    modes: &[ModeDef {
        terms: Some(Terms::CopiesControlled),
        ..mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[],
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::PutInPlay,
            ],
        )
    }],
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::ActingCandidate,
        &[],
        &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::BurnSource,
        ],
    )],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::PreyBurnsPool(1)],
    }],
    ..CARD
};

/// The two directions of "move 1 blood from this vampire to their pool or
/// from their pool to this vampire" as a host ability in `window` — once
/// per that phase per copy ("during X", rules-qa 6.6); usable with the host
/// in torpor (PIB 20150522).
macro_rules! blood_shuttle {
    ($window:expr) => {
        &[
            mode_of(
                &[$window],
                Actor::Host,
                &[Requirement::HostBloodAtLeast(1)],
                &[Effect::HostBloodToPool(1)],
            ),
            mode_of(
                &[$window],
                Actor::Host,
                &[
                    Requirement::HostBelowCapacity,
                    Requirement::HostControllerPoolAtLeast(1),
                ],
                &[Effect::PoolToHostBlood(1)],
            ),
        ]
    };
}

/// "Put this card on a vampire you control. During their master phase, this
/// vampire's controller can move 1 blood from this vampire to their pool or
/// from their pool to this vampire."
pub static BLOOD_DOLL: CardDef = CardDef {
    name: "Blood Doll",
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::TargetVampire, Requirement::Controlled],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[Effect::PutOnTarget],
        )
    }],
    abilities: blood_shuttle!(Window::MasterPhase),
    ..CARD
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
    modes: &[ModeDef {
        target: target(TargetKind::Minion, &[Requirement::TargetVampire]),
        terms: Some(Terms::BurnNamedInPlay("Blood Doll")),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[Effect::PutOnTarget, Effect::BurnChosenInPlay],
        )
    }],
    abilities: blood_shuttle!(Window::UnlockPhaseOwn),
    ..CARD
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
            target: target(TargetKind::Minion, $filter),
            ..mode_of(
                &[Window::UnlockPhaseOwn],
                Actor::Controller,
                &[],
                &[Effect::HuntingGroundBlood($n)],
            )
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
            modes: &[mode_of(
                &[Window::MasterPhase],
                Actor::Methuselah,
                &[],
                &[Effect::PutInPlay],
            )],
            abilities: &[hunting_ground_pick!(
                1,
                &[
                    Requirement::TargetVampire,
                    Requirement::Controlled,
                    Requirement::ActorReady,
                    Requirement::NotHuntingGroundFed,
                ]
            )],
            ..CARD
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
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[Requirement::ControlsReadyTitled(&[
            TitleClass::Prince,
            TitleClass::Baron,
        ])],
        &[Effect::PutInPlay],
    )],
    abilities: &[hunting_ground_pick!(
        2,
        &[
            Requirement::Controlled,
            Requirement::ActorReady,
            Requirement::ActorTitleIn(&[TitleClass::Any]),
            Requirement::NotHuntingGroundFed,
        ]
    )],
    ..CARD
};

/// "Requires a ready Anarch. Unique location. Hunting ground. During your
/// unlock phase, a ready Anarch you control can gain 1 blood, and, if you
/// control a ready baron, another ready Anarch you control can gain 1 blood
/// as well. A vampire can gain blood from only one hunting ground each
/// turn." Two picks, each once per unlock phase; the second needs a ready
/// baron and cannot be the first's vampire (fed this turn).
pub static CARFAX_ABBEY: CardDef = CardDef {
    name: "Carfax Abbey",
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[Requirement::ControlsReadySect(&[Sect::Anarch])],
        &[Effect::PutInPlay],
    )],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::ActorReady,
                    Requirement::ActorSectIn(&[Sect::Anarch]),
                    Requirement::NotHuntingGroundFed,
                ],
            ),
            limit: Some(Limit::PerPhaseEach),
            ..mode_of(
                &[Window::UnlockPhaseOwn],
                Actor::Controller,
                &[],
                &[Effect::HuntingGroundBlood(1)],
            )
        },
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::ActorReady,
                    Requirement::ActorSectIn(&[Sect::Anarch]),
                    Requirement::NotHuntingGroundFed,
                ],
            ),
            limit: Some(Limit::PerPhaseEach),
            ..mode_of(
                &[Window::UnlockPhaseOwn],
                Actor::Controller,
                &[Requirement::ControlsReadyTitled(&[TitleClass::Baron])],
                &[Effect::HuntingGroundBlood(1)],
            )
        },
    ],
    ..CARD
};

// ---- Replacement and hand size (slice 7) ----------------------------------

/// "Unique location. +1 hand size." A hand-size layer of its controller,
/// suspended while the card is contested (ANK 20221028: a second copy
/// enters contest without modifying the hand size).
pub static ELDER_LIBRARY: CardDef = CardDef {
    name: "Elder Library",
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[],
        &[Effect::PutInPlay, Effect::Layer(Modifier::HandSize(1))],
    )],
    ..CARD
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
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[],
        &[
            Effect::PutInPlay,
            Effect::EnterWithCounters(4),
            Effect::Layer(Modifier::HandSizePerCounter),
        ],
    )],
    riders: &[Rider {
        hook: Hook::WouldReplace,
        scope: RiderScope::Controller,
        requires: &[Requirement::CountersAtLeast(1)],
        effects: &[Effect::RemoveCounter(1), Effect::BurnSelfWithoutCounters],
    }],
    ..CARD
};

/// "Only usable by a locked vampire. Do not replace until your next unlock
/// phase. This vampire wakes (they ignore the requirement to be unlocked
/// for playing reaction cards and attempting to block until the end of the
/// action)." The wake of Eyes of Argus [AUS], held back until the player's
/// next unlock phase.
pub static WAKE_WITH_EVENINGS_FRESHNESS: CardDef = CardDef {
    name: "Wake with Evening's Freshness",
    delayed_replacement: Some(Until::NextUnlockPhase),
    modes: &[mode_of(
        &[Window::Wake, Window::CardAsPlayed],
        Actor::ReactingMinion,
        &[Requirement::ActorVampire, Requirement::LockedActor],
        &[Effect::Wake],
    )],
    ..CARD
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
    delayed_replacement: Some(Until::NextDiscardPhase),
    modes: &[
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::ActingMinion,
            &[Requirement::StealthNeeded],
            &[Effect::AddStealth(1)],
        ),
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::ActingMinion,
            &[Requirement::StealthNeeded],
            &[Effect::AddStealth(1), Effect::EndActionIfBlocked],
        ),
    ],
    ..CARD
};

// ---- Equipment (slice 8a) ---------------------------------------------------

/// The one vehicle a minion can have (the vehicles' shared clause).
const ONE_VEHICLE: &[Requirement] = &[Requirement::ActorWithout(Holding::Label(Label::Vehicle))];

/// "Vehicle. This minion gets +1 intercept. A minion can have only one
/// vehicle."
pub static SPORT_BIKE: CardDef = CardDef {
    name: "Sport Bike",
    modes: &[mode_of(
        &[Window::ActionDeclareEquip],
        Actor::ActingCandidate,
        ONE_VEHICLE,
        &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Intercept,
                n: 1,
                during: &[],
                who: Bearer::Any,
            }),
        ],
    )],
    ..CARD
};

/// "Vehicle. The action to equip with this vehicle is with an additional +1
/// stealth. This minion gets +1 stealth during hunt, recruit and employ
/// actions. If this minion is blocked by a prince or an archbishop (during
/// any action), burn this vehicle. A minion can have only one vehicle." The
/// equip stealth counts when it is taken from another minion too (LSJ
/// 20060222); the burn applies as the block resolves, before anything else
/// (ANK 20220116). Hartmut Stover is an archbishop (Reign of Stanislava).
pub static UNLICENSED_TAXICAB: CardDef = CardDef {
    name: "Unlicensed Taxicab",
    modes: &[mode_of(
        &[Window::ActionDeclareEquip],
        Actor::ActingCandidate,
        ONE_VEHICLE,
        &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::AddStealth(1),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Stealth,
                n: 1,
                during: &[ActionKind::Hunt, ActionKind::Recruit, ActionKind::Employ],
                who: Bearer::Any,
            }),
        ],
    )],
    riders: &[Rider {
        hook: Hook::HostBlocked,
        scope: RiderScope::Every,
        requires: &[Requirement::BlockerTitleIn(&[
            TitleClass::Prince,
            TitleClass::Archbishop,
        ])],
        effects: &[Effect::BurnSelf],
    }],
    ..CARD
};

/// "Unique. This Ravnos gets +1 bleed. Once each combat, this Ravnos can
/// strike: dodge." A minion that is not Ravnos may take it from another
/// minion (ANK 20180719-3) and gets nothing from it (rules-qa 9.1).
pub static TREASURED_SAMADJI: CardDef = CardDef {
    name: "Treasured Samadji",
    modes: &[mode_of(
        &[Window::ActionDeclareEquip],
        Actor::ActingCandidate,
        &[],
        &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Bleed,
                n: 1,
                during: &[],
                who: Bearer::Clan("Ravnos"),
            }),
        ],
    )],
    abilities: &[ModeDef {
        limit: Some(Limit::PerCombat),
        ..mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Host,
            &[Requirement::HostClan("Ravnos")],
            &[Effect::Strike(StrikeKind::Dodge)],
        )
    }],
    ..CARD
};

/// "Unique. The bearer gets -2 stealth during bleed actions. This vampire
/// can bleed as a Ⓓ action that costs 1 blood; this action gets +3 bleed if
/// the target Methuselah controls no ready unlocked minions." The granted
/// bleed is a bleed (one a turn) and this copy's action; its cost and the
/// +3 are read at resolution, and it fizzles if the Codex is no longer on
/// the acting vampire then (RTR 19960221).
pub static CODEX_OF_THE_EDENIC_GROUNDSKEEPERS: CardDef = CardDef {
    name: "Codex of the Edenic Groundskeepers",
    modes: &[mode_of(
        &[Window::ActionDeclareEquip],
        Actor::ActingCandidate,
        &[],
        &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Stealth,
                n: -2,
                during: &[ActionKind::Bleed],
                who: Bearer::Any,
            }),
        ],
    )],
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::Host,
        &[],
        &[
            Effect::DeclareAction(ActionKind::Bleed),
            Effect::PayBlood(1),
            Effect::BleedIfTargetHasNoReadyUnlocked(3),
        ],
    )],
    ..CARD
};

/// "Electronic equipment. The minion with this equipment may bleed at +1
/// bleed as a Ⓓ action. No other equipment can increase the bleed amount
/// of this action." A legacy card the owner added to the pool (2026-09-25).
/// The granted bleed is a bleed (one a turn) and this copy's action; it
/// fizzles if the phone is no longer on the acting minion (RTR 19960221).
pub static CAMERA_PHONE: CardDef = CardDef {
    name: "Camera Phone",
    modes: &[mode_of(
        &[Window::ActionDeclareEquip],
        Actor::ActingCandidate,
        &[],
        &[Effect::DeclareAction(ActionKind::Equip)],
    )],
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::Host,
        &[],
        &[
            Effect::DeclareAction(ActionKind::Bleed),
            bleed(1, false),
            Effect::NoOtherEquipmentBleed,
        ],
    )],
    ..CARD
};

/// "Electronic. The bearer gets +1 bleed. A minion can have only one Laptop
/// Computer." A legacy card the owner added to the pool (2026-09-25).
pub static LAPTOP_COMPUTER: CardDef = CardDef {
    name: "Laptop Computer",
    modes: &[mode_of(
        &[Window::ActionDeclareEquip],
        Actor::ActingCandidate,
        &[Requirement::ActorWithout(Holding::Named("Laptop Computer"))],
        &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Bleed,
                n: 1,
                during: &[],
                who: Bearer::Any,
            }),
        ],
    )],
    ..CARD
};

/// "Vehicle. This Anarch gets +1 bleed. Allies and younger vampires get -1
/// intercept against this Anarch. Vampires can burn this card as a Ⓓ action
/// that costs 1 pool; if that action is successful, this Anarch is locked
/// and does not unlock as normal during their next unlock phase. A minion
/// can have only one vehicle." The Anarch clauses read the current bearer
/// (rules-qa 9.1); the burn is directed at the card's controller.
pub static STOLEN_POLICE_CRUISER: CardDef = CardDef {
    name: "Stolen Police Cruiser",
    modes: &[mode_of(
        &[Window::ActionDeclareEquip],
        Actor::ActingCandidate,
        ONE_VEHICLE,
        &[
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
    )],
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::ActingCandidate,
        &[Requirement::ActorVampire, Requirement::PoolAtLeast(1)],
        &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::PayPool(1),
            Effect::BurnSource,
            Effect::LockHostSkipNextUnlock(Bearer::Sect(Sect::Anarch)),
        ],
    )],
    ..CARD
};

/// "Unique. The bearer with Auspex [aus] gets +1 intercept. The bearer with
/// superior Auspex [AUS] can burn 1 blood during an action to get an
/// additional +1 intercept." The burn follows the "during X" template:
/// once per action (rules-qa 9.6), only while the bearer attempts to block
/// and the intercept is needed (rules-qa 3.2, 9.6).
pub static BOWL_OF_CONVERGENCE: CardDef = CardDef {
    name: "Bowl of Convergence",
    modes: &[mode_of(
        &[Window::ActionDeclareEquip],
        Actor::ActingCandidate,
        &[],
        &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Intercept,
                n: 1,
                during: &[],
                who: Bearer::Discipline("aus"),
            }),
        ],
    )],
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionStealthIntercept],
            Actor::Host,
            &[
                Requirement::HostDiscipline("AUS"),
                Requirement::HostBloodAtLeast(1),
                Requirement::InterceptNeeded,
            ],
            &[Effect::HostBurnBlood(1), Effect::AddIntercept(1)],
        )
    }],
    ..CARD
};

// ---- Weapons (slice 8b) -----------------------------------------------------

/// The equip action of an equipment with nothing else on its mode.
const EQUIP: ModeDef = mode_of(
    &[Window::ActionDeclareEquip],
    Actor::ActingCandidate,
    &[],
    &[Effect::DeclareAction(ActionKind::Equip)],
);

/// A weapon's strike, chosen by its bearer at the strike step.
const fn weapon_strike(requires: &'static [Requirement], effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::CombatStrikeDeclaration],
        actor: Actor::Host,
        requires,
        target: None,
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
    limit: Some(Limit::PerCombat),
    ..mode_of(
        &[Window::CombatDetermineRange],
        Actor::Host,
        &[],
        &[Effect::Maneuver, Effect::CommitStrike],
    )
};

/// "Weapon: gun. Strike: 2R damage, with 1 optional maneuver each combat."
pub static MAGNUM_44: CardDef = CardDef {
    name: ".44 Magnum",
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
    ..CARD
};

/// "Weapon: gun. Strike: 2R damage, with 1 optional maneuver each combat.
/// After the bearer strikes with this gun, they get 1 optional additional
/// strike (limited), only usable to strike with this gun, this round."
pub static AK_47: CardDef = CardDef {
    name: "AK-47",
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
    ..CARD
};

/// "Weapon: gun. Strike: 4R damage, with 1 optional maneuver each combat."
pub static ASSAULT_RIFLE: CardDef = CardDef {
    name: "Assault Rifle",
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
    ..CARD
};

/// "Weapon. Strike: 2R aggravated damage."
pub static FLAMETHROWER: CardDef = CardDef {
    name: "Flamethrower",
    modes: &[EQUIP],
    abilities: &[weapon_strike(
        &[],
        &[Effect::Strike(StrikeKind::Damage {
            n: 2,
            ranged: true,
            aggravated: true,
        })],
    )],
    ..CARD
};

/// "Unique weapon. Strike: 1R aggravated damage."
pub static IVORY_BOW: CardDef = CardDef {
    name: "Ivory Bow",
    modes: &[EQUIP],
    abilities: &[weapon_strike(
        &[],
        &[Effect::Strike(StrikeKind::Damage {
            n: 1,
            ranged: true,
            aggravated: true,
        })],
    )],
    ..CARD
};

/// "Unique melee weapon. Strike: strength+1 aggravated damage." (Tzimisce.)
/// Equipping from hand needs a Tzimisce; taken from another minion it
/// needs none (ANK 20180719-3), and the strike names no clan.
pub static FEMUR_OF_TOOMLER: CardDef = CardDef {
    name: "Femur of Toomler",
    modes: &[EQUIP],
    abilities: &[weapon_strike(
        &[],
        &[Effect::Strike(StrikeKind::Melee {
            bonus: 1,
            aggravated: true,
        })],
    )],
    ..CARD
};

/// "Unique melee weapon. Strike: strength+1 aggravated damage." (Banu
/// Haqim; as Femur of Toomler.)
pub static KALIS_FANG: CardDef = CardDef {
    name: "Kali's Fang",
    modes: &[EQUIP],
    abilities: &[weapon_strike(
        &[],
        &[Effect::Strike(StrikeKind::Melee {
            bonus: 1,
            aggravated: true,
        })],
    )],
    ..CARD
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
        mode_of(
            &[Window::CombatBeforeRange],
            Actor::Host,
            &[
                Requirement::ActorHasBlocked,
                Requirement::FirstRound,
                Requirement::SourceHeldWhenBlocking,
            ],
            &[Effect::SetRange(Range::Long), Effect::CommitStrike],
        ),
    ],
    ..CARD
};

/// "Melee weapon. Strike: strength+1 damage, with 1 optional press, only
/// usable to continue combat, each combat." (Salubri.) The press is had as
/// the strike is chosen, kept though the strike is dodged or does not
/// resolve, once each combat per Blade (rules-qa 10.4).
pub static RIGHTEOUS_BLADE: CardDef = CardDef {
    name: "Righteous Blade",
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
    ..CARD
};

/// "Weapon. Strike: 3 unpreventable damage and put this card on the
/// opposing vampire; it becomes a card with \"Cards requiring a Discipline
/// cost this vampire +1 blood. After this vampire successfully hunts, burn
/// this card\"." (1 pool.) Not against an ally (ANK 20220818, ANK
/// 20221102); at close range (a strike not said ranged, rules-qa 21.1);
/// dodged, the card stays where it is (ANK 20220923); once used, a typeless
/// card, no weapon (ANK 20220818): its second mode, never played.
pub static ENHANCED_COAGULANT: CardDef = CardDef {
    name: "Enhanced Coagulant",
    modes: &[
        EQUIP,
        mode_of(
            &[],
            Actor::Host,
            &[],
            &[Effect::Layer(Modifier::HostDisciplineCardsCost(1))],
        ),
    ],
    abilities: &[weapon_strike(
        &[Requirement::OpponentVampire],
        &[
            Effect::Strike(StrikeKind::Damage {
                n: 3,
                ranged: false,
                aggravated: false,
            }),
            Effect::UnpreventableStrike,
            Effect::BecomesOnOpponent,
        ],
    )],
    riders: &[Rider {
        hook: Hook::SuccessfulHunt,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::BurnSelf],
    }],
    ..CARD
};

/// "Only one Flaming Candle can be played or equipped in a game. This
/// vampire can burn 1 blood and the Flaming Candle as they announce an
/// action to make that action unblockable by vampires." Equipped from hand
/// or from the library once a game; moved or equipped from another minion
/// afterwards (ANK 20221103).
pub static FLAMING_CANDLE: CardDef = CardDef {
    name: "Flaming Candle",
    modes: &[ModeDef {
        requires: &[Requirement::NotPlayedThisGame],
        ..EQUIP
    }],
    abilities: &[mode_of(
        &[Window::ActionAsAnnounced],
        Actor::Host,
        &[Requirement::ActorActing, Requirement::HostBloodAtLeast(1)],
        &[
            Effect::HostBurnBlood(1),
            Effect::BurnSelf,
            Effect::VampiresCannotBlock,
        ],
    )],
    ..CARD
};

/// "Unique. This vampire gets +1 level of Blood Sorcery [tha]. Rescuing a
/// vampire from torpor costs this vampire -1 blood. Once each turn, this
/// vampire can unlock after rescuing a vampire from torpor." (1 pool.) The
/// discount is the action's, whoever pays (PIB 20110918, LSJ
/// 20020304-2); the level adds no capacity.
pub static RITUAL_GOBLET: CardDef = CardDef {
    name: "Ritual Goblet",
    modes: &[ModeDef {
        effects: &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::Layer(Modifier::BearerDisciplineLevel("tha")),
            Effect::Layer(Modifier::RescueDiscount {
                not_clan: None,
                n: 1,
                gain: 0,
            }),
        ],
        ..EQUIP
    }],
    abilities: &[ability(
        &[Window::ActionAfterResolution],
        Actor::Host,
        &[Requirement::ActorActing, Requirement::RescuedFromTorpor],
        Limit::PerTurn,
        &[Effect::UnlockActing],
    )],
    ..CARD
};

/// "Once each combat, the bearer can prevent 2 damage from gun strikes or 1
/// damage from any other source. A minion can have only one Kevlar Vest."
/// The branch is chosen once each combat; the gun points may be spent over
/// several strikes of the combat, only against gun-strike damage; a gun's
/// damage other than its strike is not "from gun strikes" (rules-qa 10.3;
/// LSJ 20081210).
pub static KEVLAR_VEST: CardDef = CardDef {
    name: "Kevlar Vest",
    modes: &[mode_of(
        &[Window::ActionDeclareEquip],
        Actor::ActingCandidate,
        &[Requirement::ActorWithout(Holding::Named("Kevlar Vest"))],
        &[Effect::DeclareAction(ActionKind::Equip)],
    )],
    abilities: &[
        ModeDef {
            limit: Some(Limit::PerCombat),
            ..mode_of(
                &[Window::CombatDamagePrevention],
                Actor::Host,
                &[Requirement::GunStrikeDamagePending],
                &[Effect::PreventGunStrikeAllowance(2)],
            )
        },
        ModeDef {
            limit: Some(Limit::PerCombat),
            ..mode_of(
                &[Window::CombatDamagePrevention],
                Actor::Host,
                &[Requirement::NonGunDamagePending],
                &[Effect::PreventNonGun(1)],
            )
        },
    ],
    ..CARD
};

// ---- Library search (slice 8c) ----------------------------------------------

/// "+1 stealth action. [tha] Search your library for an equipment card and
/// equip this vampire with it (requirements and cost apply as normal;
/// shuffle afterward). [THA] As above, but this is a +3 stealth action."
/// (1 blood.) The equipment is chosen at resolution ([RBK search]); one
/// whose blood cost the vampire cannot pay is not offered (rules-qa 9.3).
pub static MAGIC_OF_THE_SMITH: CardDef = CardDef {
    name: "Magic of the Smith",
    modes: &[
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[],
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::SearchEquip,
            ],
        ),
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[],
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(3),
                Effect::SearchEquip,
            ],
        ),
    ],
    ..CARD
};

/// "Put this card on a ready minion you control. If you control this
/// minion, they can equip with the first equipment you find in your library
/// as a +1 stealth equip action (working down from the top; requirements
/// and cost apply as normal; shuffle afterward)."
pub static VAST_WEALTH: CardDef = CardDef {
    name: "Vast Wealth",
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::Controlled, Requirement::ActorReady],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[Effect::PutOnTarget],
        )
    }],
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::Host,
        &[Requirement::ControllerControlsHost],
        &[
            Effect::DeclareAction(ActionKind::EquipFound),
            Effect::AddStealth(1),
            Effect::EquipFirstFound,
        ],
    )],
    ..CARD
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
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[],
        &[Effect::PutInPlay, Effect::SearchStore(3)],
    )],
    riders: &[Rider {
        hook: Hook::WouldDraw,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::BurnSelfWithoutStored],
    }],
    ..CARD
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
    modes: &[ModeDef {
        effects: &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::StoreTopCards(2),
        ],
        ..EQUIP
    }],
    abilities: &[mode_of(
        &[Window::UnlockPhaseOwn],
        Actor::Controller,
        &[],
        &[Effect::StoreTopCards(1)],
    )],
    riders: &[Rider {
        hook: Hook::WouldDraw,
        scope: RiderScope::Controller,
        requires: &[Requirement::HostClan("Ravnos"), Requirement::HostReady],
        effects: &[],
    }],
    ..CARD
};

/// "Unique. If the bearer is ready during your unlock phase, you can draw
/// up to 3 cards without discarding and then move the same number of cards
/// from your hand to the bottom of your library." The number is picked as
/// the ability is used and capped by the library as it resolves; the cards
/// moved back are chosen one at a time, the last one chosen the bottom
/// card (rules-qa 9.5, 11.1).
pub static HEART_OF_NIZCHETUS: CardDef = CardDef {
    name: "Heart of Nizchetus",
    modes: &[EQUIP],
    abilities: &[ModeDef {
        terms: Some(Terms::Count { min: 1, max: 3 }),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[Requirement::HostReady],
            &[Effect::DrawThenBottom],
        )
    }],
    ..CARD
};

// ---- Precon coverage (slice 11a) --------------------------------------------

/// "Ⓓ Bleed with +n bleed": an action card's bleed mode.
macro_rules! bleed_action {
    ($n:expr) => {
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[],
            &[Effect::DeclareAction(ActionKind::Bleed), bleed($n, false)],
        )
    };
}

/// "+n stealth": an action modifier of the acting minion's, when stealth
/// is needed.
const fn stealth(effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionStealthIntercept],
        actor: Actor::ActingMinion,
        requires: &[Requirement::StealthNeeded],
        target: None,
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
        target: None,
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
        target: None,
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
            target: target(TargetKind::Uncontrolled, $filter),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::ActingCandidate,
                &[],
                &[
                    Effect::DeclareAction(ActionKind::Card),
                    Effect::AddStealth(1),
                    Effect::BloodToUncontrolledTarget($n),
                ],
            )
        }
    };
}

/// "[dom] Ⓓ Bleed with +2 bleed. [DOM] +1 stealth action. Add 3 blood to
/// a younger vampire in your uncontrolled region."
pub static GOVERN_THE_UNALIGNED: CardDef = CardDef {
    name: "Govern the Unaligned",
    modes: &[
        bleed_action!(2),
        uncontrolled_blood!(&[Requirement::Younger], 3),
    ],
    ..CARD
};

/// "[pre] Ⓓ Bleed with +1 bleed. [PRE] +1 stealth action. Add 2 blood to
/// a younger vampire in your uncontrolled region."
pub static ENCHANT_KINDRED: CardDef = CardDef {
    name: "Enchant Kindred",
    modes: &[
        bleed_action!(1),
        uncontrolled_blood!(&[Requirement::Younger], 2),
    ],
    ..CARD
};

/// "[pre] Ⓓ Bleed with +1 bleed. [PRE] Ⓓ Bleed with +2 bleed."
pub static INTIMIDATION: CardDef = CardDef {
    name: "Intimidation",
    modes: &[bleed_action!(1), bleed_action!(2)],
    ..CARD
};

/// "+1 stealth action. Add 2 blood to a Gangrel in your uncontrolled
/// region."
pub static THING: CardDef = CardDef {
    name: "Thing",
    modes: &[uncontrolled_blood!(
        &[Requirement::TargetClan("Gangrel")],
        2
    )],
    ..CARD
};

/// "+1 stealth action. [pre] Put this card on this vampire. This vampire
/// gets +1 bleed. A vampire can have only one Heart of the City. [PRE] As
/// above, but this vampire gets +2 bleed." (2 blood.)
pub static HEART_OF_THE_CITY: CardDef = CardDef {
    name: "Heart of the City",
    modes: &[
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[Requirement::ActorWithout(Holding::Named(
                "Heart of the City",
            ))],
            &[
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
        ),
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[Requirement::ActorWithout(Holding::Named(
                "Heart of the City",
            ))],
            &[
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
        ),
    ],
    ..CARD
};

/// "[obf] +1 stealth. Lost in Crowds and cards named Into Thin Air cannot
/// both be played during the same action. [OBF] As above, but for +2
/// stealth." (Into Thin Air is not in the V5 pool.)
pub static LOST_IN_CROWDS: CardDef = CardDef {
    name: "Lost in Crowds",
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        stealth(&[Effect::AddStealth(2)]),
    ],
    ..CARD
};

/// "[obf] [ACTION MODIFIER] +1 stealth. [OBF] [COMBAT] Maneuver."
pub static SWALLOWED_BY_THE_NIGHT: CardDef = CardDef {
    name: "Swallowed by the Night",
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        combat(&[Window::CombatDetermineRange], &[], &[Effect::Maneuver]),
    ],
    ..CARD
};

/// "[cel] [COMBAT] Press, or maneuver with 1 optional press. [CEL] [ACTION
/// MODIFIER] +1 stealth." (1 blood.) The optional press is for the round
/// it is had in (TOM 19960521).
pub static RESIST_EARTHS_GRASP: CardDef = CardDef {
    name: "Resist Earth's Grasp",
    modes: &[
        combat(
            &[Window::CombatDetermineRange, Window::CombatPress],
            &[],
            &[Effect::Maneuver, Effect::OptionalPress, Effect::Press],
        ),
        stealth(&[Effect::AddStealth(1)]),
    ],
    ..CARD
};

/// "Only usable during a bleed action. [pre] +1 bleed, with an additional
/// +1 bleed if this vampire is Toreador (limited). [PRE] +2 bleed, with an
/// additional +1 bleed if this vampire is Toreador (limited)." (1 blood.)
pub static AIRE_OF_ELATION: CardDef = CardDef {
    name: "Aire of Elation",
    modes: &[
        modifier(
            &[Requirement::IsBleed],
            &[
                bleed(1, true),
                Effect::If {
                    on: On::Actor,
                    requires: &[Requirement::ActorClan("Toreador")],
                    then: &bleed(1, true),
                },
            ],
        ),
        modifier(
            &[Requirement::IsBleed],
            &[
                bleed(2, true),
                Effect::If {
                    on: On::Actor,
                    requires: &[Requirement::ActorClan("Toreador")],
                    then: &bleed(1, true),
                },
            ],
        ),
    ],
    ..CARD
};

/// "Only usable during a bleed action. [dom] +1 bleed (limited). [DOM] +3
/// bleed if the target Methuselah has 9 or fewer pool (limited)." The
/// condition is read live on the bleed's current target: played against 10
/// or more pool it lingers and does not count against the limit (PIB
/// 20110817); lost to a change of target, it no longer counts and another
/// limited increase may be played (RTR 20191031, ANK 20211019-2).
pub static FORESHADOWING_DESTRUCTION: CardDef = CardDef {
    name: "Foreshadowing Destruction",
    modes: &[
        modifier(&[Requirement::IsBleed], &[bleed(1, true)]),
        modifier(
            &[Requirement::IsBleed],
            &[Effect::If {
                on: On::Acting,
                requires: &[Requirement::BleedTargetPoolAtMost(9)],
                then: &bleed(3, true),
            }],
        ),
    ],
    ..CARD
};

/// "Requires an Anarch. Only usable during a bleed action. +X bleed
/// (limited). X must be 1, 2 or 3." (X blood.)
pub static MONKEY_WRENCH: CardDef = CardDef {
    name: "Monkey Wrench",
    modes: &[ModeDef {
        terms: Some(Terms::XBloodIn { min: 1, max: 3 }),
        ..mode_of(
            &[Window::ActionDuring],
            Actor::ActingMinion,
            &[ANARCH, Requirement::IsBleed],
            &[Effect::BleedModX { limited: true }],
        )
    }],
    ..CARD
};

/// "Change the target of the bleed to another Methuselah other than the
/// acting minion's controller (that Methuselah can attempt to block)",
/// locking the reacting vampire: Deflection's shape.
const REDIRECT_TARGET: Option<TargetSpec> = target(
    TargetKind::Seat,
    &[Requirement::NotActingController, Requirement::NotSelf],
);

/// "[aus] +1 intercept. [AUS] Only usable if a minion is bleeding you,
/// after blocks are declined. Lock this reacting vampire. Change the target
/// of the bleed to another Methuselah other than the acting minion's
/// controller (that Methuselah can attempt to block)." (1 blood.)
pub static TELEPATHIC_MISDIRECTION: CardDef = CardDef {
    name: "Telepathic Misdirection",
    modes: &[
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(1)]),
        ModeDef {
            target: REDIRECT_TARGET,
            ..mode_of(
                &[Window::ActionBlocksDeclined],
                Actor::ReactingMinion,
                &[
                    Requirement::DirectedAtYou,
                    Requirement::IsBleed,
                    Requirement::BlocksDeclined,
                ],
                &[Effect::LockActor, Effect::RedirectBleed],
            )
        },
    ],
    ..CARD
};

/// "Requires a baron. Only usable if a minion is bleeding you, after
/// blocks are declined. Lock this reacting vampire. Change the target of
/// the bleed to another Methuselah other than the acting minion's
/// controller (that Methuselah can attempt to block)."
pub static BAIT_AND_SWITCH: CardDef = CardDef {
    name: "Bait and Switch",
    modes: &[ModeDef {
        target: REDIRECT_TARGET,
        ..mode_of(
            &[Window::ActionBlocksDeclined],
            Actor::ReactingMinion,
            &[
                Requirement::ActorTitleIn(&[TitleClass::Baron]),
                Requirement::DirectedAtYou,
                Requirement::IsBleed,
                Requirement::BlocksDeclined,
            ],
            &[Effect::LockActor, Effect::RedirectBleed],
        )
    }],
    ..CARD
};

/// "[aus] +1 intercept. [AUS] Only usable if a minion controlled by your
/// predator is bleeding you, after blocks are declined. Not usable if
/// there are only two Methuselahs left. Lock this reacting vampire. Change
/// the target of the bleed to your predator's predator (that Methuselah can
/// attempt to block)."
pub static MY_ENEMYS_ENEMY: CardDef = CardDef {
    name: "My Enemy's Enemy",
    modes: &[
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(1)]),
        ModeDef {
            target: target(TargetKind::Seat, &[Requirement::PredatorsPredator]),
            ..mode_of(
                &[Window::ActionBlocksDeclined],
                Actor::ReactingMinion,
                &[
                    Requirement::DirectedAtYou,
                    Requirement::IsBleed,
                    Requirement::BlocksDeclined,
                    Requirement::ActingByPredator,
                    Requirement::MoreThanTwoLeft,
                ],
                &[Effect::LockActor, Effect::RedirectBleed],
            )
        },
    ],
    ..CARD
};

/// "[aus] +1 intercept. [AUS] +2 intercept."
pub static ENHANCED_SENSES: CardDef = CardDef {
    name: "Enhanced Senses",
    modes: &[
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(1)]),
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(2)]),
    ],
    ..CARD
};

/// "Requires an Anarch. +1 intercept, with an additional +1 intercept if
/// the acting minion is titled." The title is read live for the rest of
/// the action (ANK 20190416).
pub static PROTECTION_RACKET: CardDef = CardDef {
    name: "Protection Racket",
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
    ..CARD
};

/// "Only usable during an action directed at you (or a card you control).
/// +2 intercept, with an additional +1 intercept if this Nosferatu is
/// titled." (Nosferatu, 1 blood.)
pub static THE_WARRENS: CardDef = CardDef {
    name: "Warrens, The",
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
    ..CARD
};

/// "[pro] Strike: combat ends. [PRO] As above, and unlock this vampire
/// before combat ends."
pub static EARTH_MELD: CardDef = CardDef {
    name: "Earth Meld",
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
    ..CARD
};

/// "[cel] Maneuver. [CEL] Additional strike (limited)."
pub static PURSUIT: CardDef = CardDef {
    name: "Pursuit",
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
    ..CARD
};

/// "[tha] Press, only usable to continue combat. [THA] Maneuver."
pub static APPORTATION: CardDef = CardDef {
    name: "Apportation",
    modes: &[
        combat(
            &[Window::CombatPress],
            &[Requirement::NoContinuePress],
            &[Effect::Press],
        ),
        combat(&[Window::CombatDetermineRange], &[], &[Effect::Maneuver]),
    ],
    ..CARD
};

/// "[obl] Additional strike (limited). [OBL] Strike: dodge, with 1
/// additional strike (limited)."
pub static ARMS_OF_AHRIMAN: CardDef = CardDef {
    name: "Arms of Ahriman",
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
    ..CARD
};

/// "Not usable during the first round. [tha] Strike: 1R aggravated damage.
/// [THA] Strike: 2R aggravated damage."
pub static WALK_OF_FLAME: CardDef = CardDef {
    name: "Walk of Flame",
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
    ..CARD
};

/// "A vampire can play only one Quickness each round. [cel] Additional
/// strike (limited). [CEL] Additional strike that does not count against
/// the limit."
pub static QUICKNESS: CardDef = CardDef {
    name: "Quickness",
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
    ..CARD
};

/// "A vampire can play only one Rego Motum each round. [tha] Prevent 2
/// damage from the opposing minion's strike. [THA] Prevent 4 damage from
/// the opposing minion's strike." (1 blood.)
pub static REGO_MOTUM: CardDef = CardDef {
    name: "Rego Motum",
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
    ..CARD
};

/// "Trifle. Add 1 blood to a ready vampire."
pub static LIFE_IN_THE_CITY: CardDef = CardDef {
    name: "Life in the City",
    trifle: true,
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::TargetVampire, Requirement::ActorReady],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[Effect::BloodToTarget(1)],
        )
    }],
    ..CARD
};

/// "Lock a minion." (1 pool.) Any minion in play, a locked one included:
/// playing it to no effect is legal (and cycles the card).
pub static MISDIRECTION: CardDef = CardDef {
    name: "Misdirection",
    modes: &[ModeDef {
        target: target(TargetKind::Minion, &[]),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[Effect::LockTarget],
        )
    }],
    ..CARD
};

/// "Unique location. +2 transfers." (Its controller's influence phase.)
pub static INFORMATION_HIGHWAY: CardDef = CardDef {
    name: "Information Highway",
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[],
        &[Effect::PutInPlay, Effect::Layer(Modifier::Transfers(2))],
    )],
    ..CARD
};

/// A location put in play in the master phase.
const LOCATION: ModeDef = mode_of(
    &[Window::MasterPhase],
    Actor::Methuselah,
    &[],
    &[Effect::PutInPlay],
);

/// "Lock this card to give a <clan> you control +1 stealth": to the acting
/// minion of that clan, when stealth is needed (City Star Taxi's shape).
macro_rules! stealth_location {
    ($name:ident, $printed:expr, $clan:expr, $doc:expr) => {
        #[doc = $doc]
        pub static $name: CardDef = CardDef {
            name: $printed,
            modes: &[LOCATION],
            abilities: &[mode_of(
                &[Window::ActionDuring],
                Actor::Controller,
                &[
                    Requirement::CardUnlocked,
                    Requirement::OtherActingControlled,
                    Requirement::OtherActingClan($clan),
                    Requirement::StealthNeeded,
                ],
                &[Effect::LockSelf, Effect::AddStealth(1)],
            )],
            ..CARD
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
    modes: &[LOCATION],
    abilities: &[mode_of(
        &[Window::ActionStealthIntercept],
        Actor::Controller,
        &[
            Requirement::CardUnlocked,
            Requirement::YourAttemptNeedsIntercept,
            Requirement::AttemptingClan("Banu Haqim"),
        ],
        &[Effect::LockSelf, Effect::AttemptingIntercept(1)],
    )],
    ..CARD
};

// ---- Wakes and "if this vampire blocks" riders (slice 11b) -----------------

/// A wake of the reacting minion: locked, once between its unlock phases.
macro_rules! wake {
    ($requires:expr, $effects:expr) => {
        ModeDef {
            limit: Some(Limit::BetweenUnlockPhases),
            ..mode_of(
                &[Window::Wake, Window::CardAsPlayed],
                Actor::ReactingMinion,
                $requires,
                $effects,
            )
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
    modes: &[wake!(
        &[Requirement::LockedActor],
        &[Effect::Wake, Effect::AllySkipsNextUnlock]
    )],
    ..CARD
};

/// "A vampire can play only one Shadow Sentinel between their unlock
/// phases. [obl] Only usable by a locked vampire. This vampire wakes. [OBL]
/// Usable by a locked vampire. Choose a locked wraith or zombie ally you
/// control. The chosen ally wakes."
pub static SHADOW_SENTINEL: CardDef = CardDef {
    name: "Shadow Sentinel",
    modes: &[
        wake!(
            &[Requirement::LockedActor, Requirement::ActorVampire],
            &[Effect::Wake]
        ),
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::TargetLocked,
                    Requirement::TargetSubtypeIn(&[Label::Wraith, Label::Zombie]),
                ],
            ),
            ..wake!(
                &[Requirement::MayBeLocked, Requirement::ActorVampire],
                &[Effect::WakeTarget]
            )
        },
    ],
    ..CARD
};

/// "Only usable by a locked vampire during a bleed action directed at you.
/// [ani] Unlock this vampire. [ANI] As above, with 1 optional maneuver
/// during the resulting combat if this vampire blocks."
pub static GUARD_DOGS: CardDef = CardDef {
    name: "Guard Dogs",
    modes: &[
        mode_of(
            &[Window::ReactionUnlock],
            Actor::ReactingMinion,
            &[
                Requirement::LockedActor,
                Requirement::ActorVampire,
                Requirement::DirectedAtYou,
                Requirement::IsBleed,
            ],
            &[Effect::UnlockActor],
        ),
        mode_of(
            &[Window::ReactionUnlock],
            Actor::ReactingMinion,
            &[
                Requirement::LockedActor,
                Requirement::ActorVampire,
                Requirement::DirectedAtYou,
                Requirement::IsBleed,
            ],
            &[Effect::UnlockActor, Effect::IfBlocks(BlockRider::Maneuver)],
        ),
    ],
    ..CARD
};

/// "[aus] +1 intercept. [AUS] As above, with 1 optional maneuver during the
/// resulting combat if this vampire blocks."
pub static SPIRITS_TOUCH: CardDef = CardDef {
    name: "Spirit's Touch",
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
    ..CARD
};

/// "Only usable if a minion controlled by your predator is acting. [ani]
/// +1 intercept. [ANI] As above, with 1 optional maneuver during the
/// resulting combat if this vampire blocks."
pub static INSTINCTIVE_REACTION: CardDef = CardDef {
    name: "Instinctive Reaction",
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
    ..CARD
};

/// "[aus] +1 intercept. [AUS] As above, and this vampire can prevent 1
/// damage during the first round of the resulting combat if they block."
pub static PRECOGNITION: CardDef = CardDef {
    name: "Precognition",
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
    ..CARD
};

// ---- Forced block attempts and more block riders (slice 11b-ii) ------------

/// "Unlocks and attempts to block, even if intercept is not yet needed":
/// a locked reacting minion that could block (engine-checked).
macro_rules! unlock_attempt {
    ($requires:expr, $effects:expr) => {
        ModeDef {
            waives_needed: true,
            ..mode_of(
                &[Window::ReactionUnlockAttempt],
                Actor::ReactingMinion,
                $requires,
                $effects,
            )
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
    ..CARD
};

/// "Requires an Anarch. [ani] Only usable during an action directed at you
/// (or a card you control). +2 intercept. [for] Reduce a bleed against you
/// by 2. [pro] Only usable by a locked vampire. This vampire burns 1 blood
/// to unlock."
pub static DEEP_ECOLOGY: CardDef = CardDef {
    name: "Deep Ecology",
    modes: &[
        intercept(
            &[
                ANARCH,
                Requirement::DirectedAtYou,
                Requirement::InterceptNeeded,
            ],
            &[Effect::AddIntercept(2)],
        ),
        mode_of(
            &[Window::ReactionDuringAction],
            Actor::ReactingMinion,
            &[ANARCH, Requirement::DirectedAtYou, Requirement::IsBleed],
            &[Effect::ReduceBleed(2)],
        ),
        mode_of(
            &[Window::ReactionUnlock],
            Actor::ReactingMinion,
            &[
                ANARCH,
                Requirement::LockedActor,
                Requirement::ActorVampire,
                Requirement::ActorBloodAtLeast(1),
            ],
            &[Effect::BurnActorBlood(1), Effect::UnlockActor],
        ),
    ],
    ..CARD
};

/// "Only usable during an action directed at you (or a card you control).
/// [pro] +2 intercept. [PRO] Only usable by a locked vampire. This vampire
/// unlocks and attempts to block. If they block, neither combatant can
/// strike during the first round of the resulting combat." (Tzimisce.)
pub static ONE_WITH_THE_LAND: CardDef = CardDef {
    name: "One With the Land",
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
    ..CARD
};

/// "[pro] [ACTION MODIFIER] Only usable if a minion attempts to block. The
/// blocking minion gets -1 intercept. If this action is blocked, this
/// vampire gets 1 optional maneuver and cannot use equipment during the
/// resulting combat. [PRO] [REACTION] +1 intercept. If this vampire
/// blocks, they get 1 optional maneuver and cannot use equipment during the
/// resulting combat."
pub static FORM_OF_THE_BAT: CardDef = CardDef {
    name: "Form of the Bat",
    modes: &[
        mode_of(
            &[Window::ActionBlockAttempt],
            Actor::ActingMinion,
            &[],
            &[
                Effect::AttemptingIntercept(-1),
                Effect::IfBlocks(BlockRider::Maneuver),
                Effect::IfBlocks(BlockRider::NoEquipment),
            ],
        ),
        intercept(
            &[Requirement::InterceptNeeded],
            &[
                Effect::AddIntercept(1),
                Effect::IfBlocks(BlockRider::Maneuver),
                Effect::IfBlocks(BlockRider::NoEquipment),
            ],
        ),
    ],
    ..CARD
};

/// "[obl] +1 intercept. [OBL] As above, and if this vampire blocks, they
/// can burn 1 blood to unlock after block resolution."
pub static TRUTH_IN_DARKNESS: CardDef = CardDef {
    name: "Truth in Darkness",
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
    ..CARD
};

const BARON: Requirement = Requirement::ActorTitleIn(&[TitleClass::Baron]);

/// "Requires a baron. Usable by a locked baron. An Anarch you control gets
/// +1 intercept. Also usable to unlock a locked Anarch you control; that
/// Anarch attempts to block with +1 intercept, even if intercept is not
/// yet needed." The intercept goes to the Anarch attempting to block; the
/// forced attempt keeps the blocking restrictions (ANK 20180623).
pub static ORGANIZED_RESISTANCE: CardDef = CardDef {
    name: "Organized Resistance",
    modes: &[
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::ReactingMinion,
            &[
                BARON,
                Requirement::MayBeLocked,
                Requirement::YourAttemptNeedsIntercept,
                Requirement::AttemptingSectIn(&[Sect::Anarch]),
            ],
            &[Effect::AttemptingIntercept(1)],
        ),
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::TargetLocked,
                    ANARCH,
                    Requirement::CanAttemptBlock,
                ],
            ),
            waives_needed: true,
            ..mode_of(
                &[Window::ReactionUnlockAttempt],
                Actor::ReactingMinion,
                &[BARON, Requirement::MayBeLocked],
                &[Effect::TargetAttemptsBlock(1)],
            )
        },
    ],
    ..CARD
};

/// "Only usable by a locked Gangrel. This Gangrel unlocks and attempts to
/// block. During this action, this Gangrel can burn 1 blood to get +1
/// intercept. Lock this Gangrel after action resolution if they did not
/// block this action."
pub static EYES_OF_THE_WILD: CardDef = CardDef {
    name: "Eyes of the Wild",
    modes: &[unlock_attempt!(
        &[Requirement::LockedActor, Requirement::CanAttemptBlock,],
        &[
            Effect::UnlockActor,
            Effect::AttemptBlock,
            Effect::BurnForInterceptThisAction,
            Effect::LockIfNoBlock,
        ]
    )],
    ..CARD
};

/// "[obf][pre] Reduce the acting minion's stealth to 0. (The acting minion
/// can still increase their stealth.) [OBF][PRE] As above, and if this
/// vampire blocks, they can strike: combat ends during the first round of
/// the resulting combat." (1 blood.) Any time before resolution, no attempt
/// needed (LSJ 20020612); a fixed -X by ruling, X the stealth as played
/// (RTR 20030519) — the exception to live conditions.
pub static NIGHT_TERRORS: CardDef = CardDef {
    name: "Night Terrors",
    modes: &[
        mode_of(
            &[Window::ReactionDuringAction],
            Actor::ReactingMinion,
            &[],
            &[Effect::ReduceStealthToZero],
        ),
        mode_of(
            &[Window::ReactionDuringAction],
            Actor::ReactingMinion,
            &[],
            &[
                Effect::ReduceStealthToZero,
                Effect::IfBlocks(BlockRider::CombatEndsFirstRound),
            ],
        ),
    ],
    ..CARD
};

// ---- Block attempts that fail or cost (slice 11c) ---------------------------

/// An action modifier during a block attempt.
macro_rules! on_attempt {
    ($requires:expr, $effects:expr) => {
        mode_of(
            &[Window::ActionBlockAttempt],
            Actor::ActingMinion,
            $requires,
            $effects,
        )
    };
}

/// "Only usable if a minion attempts to block. [obl] The blocking minion
/// gets -1 intercept. [OBL] Burn 1 blood to have that attempt fail; the
/// blocking minion cannot attempt to block this action again." The blood
/// is an effect (ANK 20210226).
pub static STYGIAN_SHROUD: CardDef = CardDef {
    name: "Stygian Shroud",
    modes: &[
        on_attempt!(&[], &[Effect::AttemptingIntercept(-1)]),
        on_attempt!(
            &[Requirement::ActorBloodAtLeast(1)],
            &[Effect::BurnActorBlood(1), Effect::AttemptFails]
        ),
    ],
    ..CARD
};

/// "Only usable if a minion attempts to block. [dom] The blocking minion
/// gets -1 intercept. [DOM] That attempt fails and the blocking minion
/// cannot attempt to block this action again." (Salubri, 1 blood.)
pub static FORCED_CONFESSIONAL: CardDef = CardDef {
    name: "Forced Confessional",
    modes: &[
        on_attempt!(&[], &[Effect::AttemptingIntercept(-1)]),
        on_attempt!(&[], &[Effect::AttemptFails]),
    ],
    ..CARD
};

/// "[obf][pre] Only usable during a bleed action. +1 bleed (limited).
/// [OBF][PRE] Only usable if a minion attempts to block. That attempt fails
/// and the blocking minion cannot attempt to block this action again. The
/// blocking minion's controller can burn 1 pool to cancel this card as it
/// is played."
pub static TRUE_LOVES_FACE: CardDef = CardDef {
    name: "True Love's Face",
    modes: &[
        modifier(&[Requirement::IsBleed], &[bleed(1, true)]),
        on_attempt!(&[], &[Effect::CancelableForPool(1), Effect::AttemptFails]),
    ],
    ..CARD
};

/// "[obl] +1 stealth. [OBL] As above, and minions without Oblivion must
/// burn 1 blood to attempt to block (allies cannot burn blood)." One
/// already attempting pays or resigns its attempt (LSJ 20031121-2).
pub static WHERE_THE_VEIL_THINS: CardDef = CardDef {
    name: "Where the Veil Thins",
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        stealth(&[Effect::AddStealth(1), Effect::VeilThins]),
    ],
    ..CARD
};

/// "[obf] +1 stealth. [OBF] As above, and minions who attempt to block this
/// action and fail become locked before action resolution." The attempts
/// that fail from its play on — the one under way included (LSJ 20081202),
/// not an earlier failure (ANK 20190509) — lock at resolution (ANK
/// 20171017).
pub static FACELESS_NIGHT: CardDef = CardDef {
    name: "Faceless Night",
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        stealth(&[Effect::AddStealth(1), Effect::LockFailedBlockers]),
    ],
    ..CARD
};

// ---- Cancellation (slice 11d) ------------------------------------------------

/// "Out-of-turn. Cancel a minion card as it is played, and its cost is not
/// paid. (If it was an action card, the acting minion does not lock. If it
/// was a strike card, the minion chooses another strike.)" (1 pool.) An
/// owner-added legacy card (2026-09-25). A canceled action card declares
/// no action: its minion may take the same action again.
pub static DIRECT_INTERVENTION: CardDef = CardDef {
    name: "Direct Intervention",
    out_of_turn: true,
    modes: &[mode_of(
        &[Window::CardAsPlayed],
        Actor::Methuselah,
        &[Requirement::NotOwnTurn, Requirement::PlayedCardIsMinionCard],
        &[Effect::CancelPlayed { refund: true }],
    )],
    ..CARD
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
    modes: &[mode_of(
        &[Window::CardAsPlayed],
        Actor::Methuselah,
        &[
            Requirement::NotOwnTurn,
            Requirement::PlayedCardIsMaster,
            Requirement::PlayedByPredatorOrPrey,
        ],
        &[
            Effect::CancelPlayed { refund: true },
            Effect::CanceledGetsMasterAction,
        ],
    )],
    ..CARD
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
    non_anarch_titles_minus: false,
};

/// "Title. Choose a ready Toreador. Successful referendum means this card is
/// put on the chosen Toreador to represent the unique Camarilla title of
/// Toreador Justicar. In this referendum, each ready Toreador gets +1
/// vote." The title is the one a printed Toreador justicar holds: the two
/// contest (advanced rules §5.8.1).
pub static TOREADOR_JUSTICAR: CardDef = CardDef {
    name: "Toreador Justicar",
    modes: &[mode_of(
        &[Window::ActionDeclarePolitical],
        Actor::ActingCandidate,
        &[],
        &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::CallReferendum(&TOREADOR_JUSTICAR_REFERENDUM),
            Effect::RepresentsTitle(crate::crypt::Title::Justicar("Toreador")),
        ],
    )],
    ..CARD
};

/// A political action card's mode, by a minion meeting `requires`
/// (`effects`: its `DeclareAction`, `CallReferendum` and layers).
const fn political(requires: &'static [Requirement], effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionDeclarePolitical],
        actor: Actor::ActingCandidate,
        requires,
        target: None,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

static MALKAVIAN_JUSTICAR_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::ChooseMinions {
        filter: &[
            Requirement::ActorReady,
            Requirement::TargetClan("Malkavian"),
            Requirement::ActorSectIn(&[Sect::Camarilla]),
        ],
        min: 1,
        max: 1,
    }],
    effect: RefEffect::PutOnChosen,
    grants: &[VoteGrant {
        voters: Voters::Clan("Malkavian"),
        n: 1,
    }],
    non_anarch_titles_minus: false,
};

/// "Title. Choose a ready Camarilla Malkavian. Successful referendum means
/// this card is put on the chosen Malkavian to represent the unique
/// Camarilla title of Malkavian Justicar. In this referendum, each Malkavian
/// gets +1 vote." Not playable without a ready Camarilla Malkavian (LSJ
/// 20100129).
pub static MALKAVIAN_JUSTICAR: CardDef = CardDef {
    name: "Malkavian Justicar",
    modes: &[political(
        &[],
        &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::CallReferendum(&MALKAVIAN_JUSTICAR_REFERENDUM),
            Effect::RepresentsTitle(crate::crypt::Title::Justicar("Malkavian")),
        ],
    )],
    ..CARD
};

static ARCHON_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::ChooseMinions {
        filter: &[
            Requirement::TargetVampire,
            Requirement::ActorSectIn(&[Sect::Camarilla]),
        ],
        min: 1,
        max: 1,
    }],
    effect: RefEffect::PutOnChosen,
    grants: &[],
    non_anarch_titles_minus: false,
};

/// "Requires a prince or justicar. Choose a Camarilla vampire. Successful
/// referendum means this card is put on the chosen vampire. The attached
/// vampire can enter combat with a vampire as a +1 stealth Ⓓ action.
/// Vampires attempting to block the attached vampire burn 1 blood. Blood
/// hunts cannot be called on the attached vampire. Camarilla vampires can
/// call a referendum to burn this card as a +1 stealth political action."
/// Each copy its own action (RTR 20030519), the two not cross-restrictive
/// (LSJ 20100909); the blood burns add up (LSJ 19980225), a blocker short
/// of blood burns what it has and still blocks (LSJ 19970707).
pub static ARCHON: CardDef = CardDef {
    name: "Archon",
    modes: &[political(
        &[PRINCE_OR_JUSTICAR],
        &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::CallReferendum(&ARCHON_REFERENDUM),
            Effect::Layer(Modifier::BlockersOfHostBurn(1)),
            Effect::Layer(Modifier::NoBloodHuntOnHost),
        ],
    )],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::NotControlled,
                    Requirement::ActorReady,
                    Requirement::TargetVampire,
                ],
            ),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::ActingCandidate,
                &[Requirement::ActorIsHost],
                &[
                    Effect::DeclareAction(ActionKind::Ability),
                    Effect::AddStealth(1),
                    Effect::EnterCombatWithTarget,
                ],
            )
        },
        mode_of(
            &[Window::ActionDeclarePolitical],
            Actor::ActingCandidate,
            &[
                Requirement::ActorVampire,
                Requirement::ActorSectIn(&[Sect::Camarilla]),
            ],
            &[
                Effect::DeclareAction(ActionKind::Political),
                Effect::AddStealth(1),
                Effect::CallReferendum(&BURN_THIS_CARD),
            ],
        ),
    ],
    ..CARD
};

static RAISING_THE_PORTCULLIS_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[],
    effect: RefEffect::PutInPlay,
    grants: &[],
    non_anarch_titles_minus: false,
};

/// "Successful referendum means this card is put in play. While your prey
/// controls a vampire in torpor, vampires you control get +1 bleed.
/// Vampires can call a referendum to burn this card as a +1 stealth
/// political action. A Methuselah can have only one Raising the
/// Portcullis." (Lasombra.) Not playable while its player has one (LSJ
/// 20100129); the bleed read live.
pub static RAISING_THE_PORTCULLIS: CardDef = CardDef {
    name: "Raising the Portcullis",
    modes: &[political(
        &[Requirement::OnePerMethuselah],
        &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::CallReferendum(&RAISING_THE_PORTCULLIS_REFERENDUM),
            Effect::Layer(Modifier::OwnBleedWhilePreyTorpor(1)),
        ],
    )],
    abilities: &[mode_of(
        &[Window::ActionDeclarePolitical],
        Actor::ActingCandidate,
        &[Requirement::ActorVampire],
        &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::AddStealth(1),
            Effect::CallReferendum(&BURN_THIS_CARD),
        ],
    )],
    ..CARD
};

static DISPUTED_TERRITORY_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[
        Term::ChooseCard(CardFilter::Location),
        Term::ChooseSeat(SeatFilter::Any),
    ],
    effect: RefEffect::ChosenTakesChosenCard,
    grants: &[],
    non_anarch_titles_minus: false,
};

/// "Choose a location and a Methuselah. Successful referendum means the
/// chosen Methuselah takes control of the chosen location." No requirement
/// checked (TOM 19960226-1); one on a minion moves onto a minion of the
/// new controller, their choice (RTR 19960112, RTR 19980623), not when it
/// stays with its Methuselah (LSJ 19971002); not playable with no location
/// in play (LSJ 20100129).
pub static DISPUTED_TERRITORY: CardDef = CardDef {
    name: "Disputed Territory",
    modes: &[political(
        &[],
        &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::CallReferendum(&DISPUTED_TERRITORY_REFERENDUM),
        ],
    )],
    ..CARD
};

static REVOLUTIONARY_COUNCIL_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[
        Term::ChooseMinions {
            filter: &[
                Requirement::Controlled,
                Requirement::ActorReady,
                Requirement::TargetUnlocked,
                Requirement::ActorSectIn(&[Sect::Anarch]),
            ],
            min: 0,
            max: u8::MAX,
        },
        Term::AllocatePerChosenWithCards { per: 2 },
    ],
    effect: RefEffect::LockChosenBurnAllocated,
    grants: &[],
    non_anarch_titles_minus: false,
};

/// "Requires a baron. Choose X ready unlocked Anarchs you control and
/// allocate 2X points among one or more Methuselahs, locations, and
/// equipment. Successful referendum means each chosen Anarch is locked,
/// each Methuselah burns 1 pool for each point allocated, and each location
/// or equipment allocated a point is burned." The Anarchs chosen with the
/// terms count though locked during the vote (LSJ 20081203); points may
/// exceed a Methuselah's pool (LSJ 20020819); X may be 0, the vote then
/// doing little, and the acting baron may be chosen if unlocked again
/// (CrimethInc.; rules-qa 39.1).
pub static REVOLUTIONARY_COUNCIL: CardDef = CardDef {
    name: "Revolutionary Council",
    modes: &[political(
        &[BARON],
        &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::CallReferendum(&REVOLUTIONARY_COUNCIL_REFERENDUM),
        ],
    )],
    ..CARD
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
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::Controlled,
                Requirement::TargetClan("Salubri"),
                Requirement::ActorSectIn(&[Sect::Independent]),
                NO_ARCHETYPE,
            ],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[
                Effect::PutOnTarget,
                Effect::RepresentsTitle(crate::crypt::Title::Independent {
                    name: "Saulot's Guiding Wisdom",
                    votes: 2,
                }),
            ],
        )
    }],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::TargetVampire,
                    Requirement::ActorReady,
                    Requirement::NotAbstaining,
                ],
            ),
            ..mode_of(
                &[Window::Referendum],
                Actor::Host,
                &[
                    Requirement::AnyPollingStep,
                    Requirement::HostReady,
                    Requirement::HostUnlocked,
                ],
                &[
                    Effect::LockHost,
                    Effect::Abstain {
                        lock: false,
                        burn: 0,
                    },
                ],
            )
        },
        ModeDef {
            limit: Some(Limit::PerTurn),
            ..mode_of(
                &[Window::CombatBeforeRange],
                Actor::ForOwnCombatant,
                &[
                    Requirement::HostOutOfCombat,
                    Requirement::HostReady,
                    Requirement::HostUnlocked,
                ],
                &[Effect::LockHost, Effect::EndCombat],
            )
        },
    ],
    ..CARD
};

/// "Archetype. Trifle. Put this card on a vampire you control. Once each
/// turn, if they block a titled vampire or a political action, this vampire
/// gains 1 blood before block resolution. A vampire can have only one
/// archetype."
pub static REBEL: CardDef = CardDef {
    name: "Rebel",
    trifle: true,
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::Controlled,
                Requirement::TargetVampire,
                NO_ARCHETYPE,
            ],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[Effect::PutOnTarget],
        )
    }],
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
    ..CARD
};

/// A location locked during the polling step for votes: its master phase
/// play, and one ability per `effects` list. "You can lock": its
/// controller's only (`Requirement::YouControlThisCard`; owner,
/// 2026-09-30: rules-qa 46.3).
macro_rules! vote_location {
    ($name:ident, $printed:expr, $doc:expr, $requires:expr, [$($effects:expr),+]) => {
        #[doc = $doc]
        pub static $name: CardDef = CardDef {
            name: $printed,
            modes: &[LOCATION],
            abilities: &[$(mode_of(&[Window::Referendum], Actor::Methuselah, $requires, $effects)),+],
..CARD
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
    &[
        Requirement::PollingStep,
        Requirement::CardUnlocked,
        Requirement::YouControlThisCard,
    ],
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
    &[
        Requirement::PollingStep,
        Requirement::CardUnlocked,
        Requirement::YouControlThisCard,
    ],
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
    &[
        Requirement::AnyPollingStep,
        Requirement::CardUnlocked,
        Requirement::YouControlThisCard,
    ],
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
    ..CARD
};

/// "[tha] Strike: 1R damage. This damage cannot be prevented by cards
/// requiring Fortitude [for]. The opposing vampire's strikes with weapons
/// inflict no damage this round. [THA] As above, but for 2R damage." The
/// weapons are nullified as it resolves (ANK 20170519).
pub static SOUL_BURN: CardDef = CardDef {
    name: "Soul Burn",
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
    ..CARD
};

/// "Only usable before range is determined. A vampire can play only one
/// Carrion Crows each combat. [ani] This combat, the opposing minion takes
/// 1R environmental damage each round during normal strike resolution.
/// [ANI] As above, but for 2R environmental damage." Dealt even against a
/// dodge; a strike: combat ends comes first (RTR 19941109, TOM 19951216).
pub static CARRION_CROWS: CardDef = CardDef {
    name: "Carrion Crows",
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
    ..CARD
};

/// "Only usable before range is determined. [tha] This round, this vampire
/// can strike, ranged: steal 1 blood or life (becoming blood). [THA] As
/// above, but for 2 blood or life (becoming blood)." Usable more than once
/// in the round (ANK 20220207).
pub static HUNGER_OF_MARDUK: CardDef = CardDef {
    name: "Hunger of Marduk",
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
    ..CARD
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
    modes: &[archetype(
        &[
            Requirement::Controlled,
            Requirement::TargetVampire,
            NO_ARCHETYPE,
        ],
        &[Effect::PutOnTarget],
    )],
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::ActionAfterResolution],
            Actor::Host,
            &[
                Requirement::ActorActing,
                Requirement::ActionSuccessful,
                Requirement::NoReactionPlayed,
            ],
            &[Effect::AddHostBlood(1)],
        )
    }],
    ..CARD
};

/// "Unique archetype. Put this card on a Salubri you control. This Salubri
/// gets +1 strength. This Salubri can enter combat with a non-Salubri
/// vampire as a Ⓓ action; if that vampire is Tremere, this is a +1 stealth
/// action. A vampire can have only one archetype."
pub static SAULOTS_AVENGING_FIST: CardDef = CardDef {
    name: "Saulot's Avenging Fist",
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
            target(
                TargetKind::Minion,
                &[
                    Requirement::NotControlled,
                    Requirement::ActorReady,
                    Requirement::TargetClan("Tremere"),
                ],
            ),
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::EnterCombatWithTarget,
            ],
        ),
    ],
    ..CARD
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
    modes: &[archetype(
        &[
            Requirement::Controlled,
            Requirement::TargetClan("Salubri"),
            NO_ARCHETYPE,
        ],
        &[
            Effect::PutOnTarget,
            Effect::Layer(Modifier::RescueDiscount {
                not_clan: Some("Tremere"),
                n: 2,
                gain: 1,
            }),
        ],
    )],
    abilities: &[printed_action(
        target(
            TargetKind::Minion,
            &[Requirement::ActorReady, Requirement::OtherThanActor],
        ),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::BloodToTargetToStart(1),
        ],
    )],
    ..CARD
};

// ---- Combat cards (slice 11i) ----------------------------------------------

/// "Only usable before range is determined. A vampire can play only one
/// Bear's Skin each combat. [ani][pro] This round, this vampire gets +1
/// strength and can prevent 1 damage. [ANI][PRO] This combat, this vampire
/// gets +1 strength and can prevent 1 damage each round."
pub static BEARS_SKIN: CardDef = CardDef {
    name: "Bear's Skin",
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
    ..CARD
};

/// "Requires an Anarch. [ani] Strike: hand strike at +1 damage. This strike
/// cannot be dodged. [cel] Strike: dodge, with 1 additional strike
/// (limited). [pot] Strike: hand strike at +2 damage." A dodge against the
/// [ani] strike may be played, with no effect (LSJ 20030902-2); the [cel]
/// additional strike is not optional: not playable once the round's limited
/// one is used (ANK 20220204).
pub static DUST_UP: CardDef = CardDef {
    name: "Dust Up",
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
    ..CARD
};

/// "Only usable before range is determined. A vampire can play only one
/// Form of the Wolf each combat. [pro] This vampire gets +1 strength this
/// combat. [PRO] As above, and this vampire gains 1 optional press this
/// combat." The press is usable this round only (TOM 19960521).
pub static FORM_OF_THE_WOLF: CardDef = CardDef {
    name: "Form of the Wolf",
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
    ..CARD
};

/// "[for] Prevent 1 damage. [FOR] Burn 1 blood to prevent all damage from
/// the opposing minion's strikes this round." The blood is the effect's,
/// not a cost (ANK 20210226): the card is not playable without it, nor
/// with no damage to prevent (LSJ 20001114).
pub static ROLLING_WITH_THE_PUNCHES: CardDef = CardDef {
    name: "Rolling with the Punches",
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
    ..CARD
};

/// "[pro] Strike: hand strike, aggravated. [PRO] Strike: hand strike at +1
/// damage and put this card on the opposing minion. During their unlock
/// phase, the attached minion burns 1 blood or life. Minions can burn this
/// card as a +1 stealth action." (Tzimisce.) The card goes on as the strike
/// lands; an ally at no life is burned.
pub static SCULPT_THE_FLESH: CardDef = CardDef {
    name: "Sculpt the Flesh",
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
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::ActingCandidate,
        &[],
        &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::AddStealth(1),
            Effect::BurnSource,
        ],
    )],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::HostBurnBlood(1)],
    }],
    ..CARD
};

/// "[obl] Strike: put this card on the opposing minion with 2 counters. The
/// attached minion burns 1 counter from this card instead of unlocking as
/// normal. If this card has no counters, burn it. [OBL] Strike: send the
/// opposing vampire to torpor or burn the opposing ally." The [OBL] vampire
/// is wounded (LSJ 20090622-2).
pub static TOUCH_OF_OBLIVION: CardDef = CardDef {
    name: "Touch of Oblivion",
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
    ..CARD
};

/// "[obl] Strike: combat ends. [OBL] Strike: combat ends. After combat
/// ends, put this card on this vampire. This vampire can burn this card to
/// get +1 stealth." Not if the combat continues or a new one begins (RTR
/// 20020501, LSJ 19980109).
pub static PASS_THROUGH_SHADOW: CardDef = CardDef {
    name: "Pass Through Shadow",
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
    abilities: &[mode_of(
        &[Window::ActionDuring],
        Actor::Host,
        &[Requirement::ActorActing, Requirement::StealthNeeded],
        &[Effect::BurnSelf, Effect::AddStealth(1)],
    )],
    ..CARD
};

// ---- Combat cards with their own mechanisms (slice 11j) ---------------------

/// "[pot] Strike: hand strike at +2 damage. [POT] As above, with 1 optional
/// maneuver, only usable to get to close range." (1 blood.) Played at the
/// maneuver, it commits the round's initial strike (window review; LSJ
/// 20021028: not while the strike cannot be used). [POT] only then: at the
/// strike it would be [pot] for 1 blood (rules-qa 48.4).
pub static SLAM: CardDef = CardDef {
    name: "Slam",
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[Effect::Strike(StrikeKind::Hand { bonus: 2 })],
        ),
        combat(
            &[Window::CombatDetermineRange],
            &[Requirement::ManeuverToClose],
            &[Effect::CommittedStrike(StrikeKind::Hand { bonus: 2 })],
        ),
    ],
    ..CARD
};

/// "[ani] Strike: 1R damage, with 1 optional maneuver. [ANI] As above, with
/// 1 optional press." Played at the maneuver, it commits the round's
/// initial strike.
pub static AID_FROM_BATS: CardDef = CardDef {
    name: "Aid from Bats",
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
    ..CARD
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
    modes: &[
        ModeDef {
            limit: Some(Limit::PerRound),
            terms: Some(Terms::EmployFromHand(Label::Animal)),
            ..combat(&[Window::CombatBeforeRange], &[], &[Effect::EmployNamed])
        },
        ModeDef {
            target: target(
                TargetKind::Card,
                &[
                    Requirement::TargetCard(CardFilter::Labeled(CardType::Retainer, Label::Animal)),
                    Requirement::TargetOnActor,
                ],
            ),
            limit: Some(Limit::PerRound),
            ..mode_of(
                &[Window::CombatUntimed],
                Actor::Combatant,
                &[Requirement::ActorWithout(Holding::Named("Pack Alpha"))],
                &[
                    Effect::BurnTargetCard,
                    Effect::PutOnActor,
                    Effect::Layer(Modifier::Bearer {
                        stat: Stat::Strength,
                        n: 1,
                        during: &[],
                        who: Bearer::Any,
                    }),
                ],
            )
        },
    ],
    ..CARD
};

/// "[ani] Strike: dodge. [ANI] As above, with 1 additional ranged strike:
/// burn weapon." The superior version needs a weapon to burn; its
/// additional strike burns one of the opposing minion's weapons, at any
/// range, unless dodged (owner, 2026-09-26: rules-qa 22.3).
pub static VORACIOUS_VERMIN: CardDef = CardDef {
    name: "Voracious Vermin",
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
                Effect::AdditionalStrikeOnly {
                    kind: StrikeKind::BurnWeapon,
                    limited: false,
                },
            ],
        ),
    ],
    ..CARD
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
    ..CARD
};

// ---- Action modifiers and reactions (slice 11k) ----------------------------

/// "[obf] +1 stealth. [OBF] Only usable by a ready vampire other than the
/// acting minion. The acting minion gets +1 stealth." The [OBF] vampire is
/// the acting minion's controller's (LSJ 19990425), locked or not (RTR
/// 19941109, LSJ 19980210); several may each play one (ANK 20200515).
pub static CLOAK_THE_GATHERING: CardDef = CardDef {
    name: "Cloak the Gathering",
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::OtherOwnMinion,
            &[Requirement::ActorVampire, Requirement::StealthNeeded],
            &[Effect::AddStealth(1)],
        ),
    ],
    ..CARD
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
    ..CARD
};

/// "[for] Vampires cannot block this action. This vampire takes 2
/// unpreventable environmental aggravated damage after action resolution.
/// [FOR] As above, but this vampire takes 1 …" If blocked, after combat
/// (RTR 19980623); whenever the action reaches resolution, even ended before
/// combat or with no effect (ANK 20220331); not if canceled before (ANK
/// 20220401).
pub static DARING_THE_DAWN: CardDef = CardDef {
    name: "Daring the Dawn",
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
    ..CARD
};

/// "Only usable after action resolution. [for] Only usable if the action
/// was successful. Unlock this vampire. [FOR] Only usable if the action was
/// blocked. Unlock this vampire." (1 blood.) A fizzled action is successful
/// (ANK 20220218); after an ended action no card is played (RTR 20180719).
pub static FREAK_DRIVE: CardDef = CardDef {
    name: "Freak Drive",
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
    ..CARD
};

/// "Only usable as the action is announced. [dom] Choose a younger vampire.
/// The chosen vampire cannot block this action. [DOM] As above, but choose
/// any vampire."
pub static SEDUCTION: CardDef = CardDef {
    name: "Seduction",
    modes: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[Requirement::TargetVampire, Requirement::Younger],
            ),
            ..mode_of(
                &[Window::ActionAsAnnounced],
                Actor::ActingMinion,
                &[],
                &[Effect::TargetCannotBlock],
            )
        },
        ModeDef {
            target: target(TargetKind::Minion, &[Requirement::TargetVampire]),
            ..mode_of(
                &[Window::ActionAsAnnounced],
                Actor::ActingMinion,
                &[],
                &[Effect::TargetCannotBlock],
            )
        },
    ],
    ..CARD
};

/// "Only usable as the action is announced. [dom] Choose a locked vampire.
/// The chosen vampire cannot block this action. [DOM] During this action,
/// minions cannot unlock." (1 blood.) Wakes still work; unlocks after
/// resolution do not (PIB 20150703).
pub static THE_SLEEPING_MIND: CardDef = CardDef {
    name: "Sleeping Mind, The",
    modes: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[Requirement::TargetVampire, Requirement::TargetLocked],
            ),
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
    ..CARD
};

/// "Only usable as the action is announced. [dom] Choose a minion. The
/// chosen minion cannot play reaction cards this action. [DOM] As above,
/// and the next reaction card played this action costs +1 blood or life."
/// (Salubri, 1 blood.)
pub static UNLEASHING_THE_BESTIAL_SOUL: CardDef = CardDef {
    name: "Unleashing the Bestial Soul",
    modes: &[
        ModeDef {
            target: target(TargetKind::Minion, &[]),
            ..mode_of(
                &[Window::ActionAsAnnounced],
                Actor::ActingMinion,
                &[],
                &[Effect::TargetNoReactions],
            )
        },
        ModeDef {
            target: target(TargetKind::Minion, &[]),
            ..mode_of(
                &[Window::ActionAsAnnounced],
                Actor::ActingMinion,
                &[],
                &[Effect::TargetNoReactions, Effect::NextReactionCosts(1)],
            )
        },
    ],
    ..CARD
};

/// "[obf][pre] Reduce a bleed against you by 3. [OBF][PRE] Lock this
/// vampire to reduce a bleed against you to 0. (The acting minion can
/// still increase the bleed amount.)" (1 blood; a combo card.)
pub static VISIONS_OF_ZAPATHASURA: CardDef = CardDef {
    name: "Visions of Zapathasura",
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
    ..CARD
};

/// "Only usable if this minion is blocked, before block resolution. Unlock
/// this minion. The action ends (unsuccessfully). This minion cannot
/// perform the same action again this turn." It has reached resolution
/// (NRA, LSJ 20070709); nothing more is played (LSJ 20010803-1); a blocked
/// leave-torpor gives no diablerie (LSJ 20050105-2).
pub static CHANGE_OF_TARGET: CardDef = CardDef {
    name: "Change of Target",
    modes: &[mode_of(
        &[Window::ActionBlocked],
        Actor::ActingMinion,
        &[],
        &[Effect::EndBlockedAction],
    )],
    ..CARD
};

/// "[obl] +1 stealth. [OBL] Only usable after resolution of a successful
/// action. Put this card on this vampire. Minions without Auspex [aus]
/// cannot perform actions directed at this vampire. During your unlock
/// phase, burn this card."
pub static SHADOW_CLOAK: CardDef = CardDef {
    name: "Shadow Cloak",
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
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::BurnSelf],
    }],
    ..CARD
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
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        mode_of(
            &[Window::ActionWouldBeSuccessful],
            Actor::ActingMinion,
            &[Requirement::IsBleed, Requirement::BleedAmountAtLeast(1)],
            &[Effect::ActionFails, Effect::PutOnActorRemembering],
        ),
    ],
    riders: &[Rider {
        hook: Hook::HostAboutToBleedRemembered,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::BurnSelf, bleed(2, false)],
    }],
    ..CARD
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
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        mode_of(
            &[Window::ActionAfterResolution],
            Actor::ActingMinion,
            &[Requirement::ActionSuccessful, Requirement::ActionDirected],
            &[Effect::PutOnActorRemembering],
        ),
    ],
    abilities: &[mode_of(
        &[Window::ActionStealthIntercept],
        Actor::Host,
        &[
            Requirement::ActorActing,
            Requirement::StealthNeeded,
            Requirement::DirectedAtRemembered,
        ],
        &[Effect::BurnSelf, Effect::AddStealth(1)],
    )],
    ..CARD
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

const PLUS_ONE_BLEED: Effect = bleed(1, false);

/// "[obf] This is a +1 stealth action. [pre] The bleed is with +1 bleed.
/// [pro] Put 1 corruption counter on a minion controlled by the target
/// Methuselah if the bleed is successful (for 1 or more)." The minion is
/// chosen at resolution (ANK 20240706, LSJ 20080608); the counter gives no
/// opportunity to steal it (ANK 20200925). Modes: [obf], [pre], [pro], then
/// each larger set of them.
pub static PLATINUM_PROTOCOL: CardDef = CardDef {
    name: "Platinum Protocol, The",
    modes: &[
        anarch_bleed(
            None,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::AddStealth(1),
            ],
        ),
        anarch_bleed(
            None,
            &[Effect::DeclareAction(ActionKind::Bleed), PLUS_ONE_BLEED],
        ),
        anarch_bleed(
            None,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::IfBleedSuccessful(&[Effect::CorruptMinionOfTarget]),
            ],
        ),
        anarch_bleed(
            None,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::AddStealth(1),
                PLUS_ONE_BLEED,
            ],
        ),
        anarch_bleed(
            None,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::AddStealth(1),
                Effect::IfBleedSuccessful(&[Effect::CorruptMinionOfTarget]),
            ],
        ),
        anarch_bleed(
            None,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                PLUS_ONE_BLEED,
                Effect::IfBleedSuccessful(&[Effect::CorruptMinionOfTarget]),
            ],
        ),
        anarch_bleed(
            None,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::AddStealth(1),
                PLUS_ONE_BLEED,
                Effect::IfBleedSuccessful(&[Effect::CorruptMinionOfTarget]),
            ],
        ),
    ],
    ..CARD
};

/// The vampire in the player's uncontrolled region Break the Bonds [pre]
/// names: "there must be an uncontrolled vampire, and it is chosen upon
/// declaration" (ANK 20240706).
const UNCONTROLLED_VAMPIRE: Option<TargetSpec> = target(TargetKind::Uncontrolled, &[]);

/// "[ani] The bleed is with +1 bleed. [obf] If the bleed is successful, you
/// can lock a minion controlled by the target Methuselah. [pre] If the
/// bleed is successful, add 1 blood to a vampire in your uncontrolled
/// region." The [obf] minion is chosen at resolution, the prey needing
/// none (ANK 20240706). Modes: [ani], [obf], [pre], then each larger set.
pub static BREAK_THE_BONDS: CardDef = CardDef {
    name: "Break the Bonds",
    modes: &[
        anarch_bleed(
            None,
            &[Effect::DeclareAction(ActionKind::Bleed), PLUS_ONE_BLEED],
        ),
        anarch_bleed(
            None,
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
            None,
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
    ..CARD
};

/// "[pre][pro] +1 stealth. [PRE][PRO] As above, and once this action, you
/// can burn 1 of your corruption counters from a blocking minion to have
/// their block attempt fail, and they cannot attempt to block this action
/// again."
pub static ENCHANTING_GAZE: CardDef = CardDef {
    name: "Enchanting Gaze",
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        stealth(&[Effect::AddStealth(1), Effect::FailBlockForCorruption]),
    ],
    ..CARD
};

const LIMITED_BLEED: Effect = bleed(1, true);

/// "Only usable during a bleed action. [pre][pro] +1 bleed (limited), or +1
/// stealth and +1 bleed (limited). [PRE][PRO] As above, and if the bleed is
/// successful, this vampire can burn 2 of your corruption counters from a
/// minion controlled by the target Methuselah to unlock." Each printed mode
/// splits in two (`cards::library_mode`): the bleed alone, or with the
/// stealth — only when stealth is needed. Not usable to unlock if the
/// target is ousted by the bleed (ANK 20220210).
pub static REVELATION_OF_THE_SERPENT: CardDef = CardDef {
    name: "Revelation of the Serpent",
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
    ..CARD
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
    modes: &[
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(1)]),
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::ReactingMinion,
            &[Requirement::CanAttemptBlockAnyone],
            &[Effect::AttemptBlock],
        ),
    ],
    ..CARD
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
const COMBAT_TARGET: Option<TargetSpec> = target(
    TargetKind::Minion,
    &[Requirement::NotControlled, Requirement::ActorReady],
);

/// "[dom] Ⓓ Bleed with +2 bleed. [DOM] As above, and if the bleed is
/// successful (for 1 or more), add 2 blood to a younger vampire you control
/// or in your uncontrolled region." (1 blood; Salubri.) The vampire is
/// chosen when the card is played (ANK 20240628-2): [DOM] splits by where
/// it is (`cards::library_mode`).
pub static FEAST_OF_THE_SOULS_SECRETS: CardDef = CardDef {
    name: "Feast of the Soul's Secrets",
    modes: &[
        action_at(
            &[],
            None,
            &[Effect::DeclareAction(ActionKind::Bleed), bleed(2, false)],
        ),
        action_at(
            &[],
            target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::TargetVampire,
                    Requirement::Younger,
                ],
            ),
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                bleed(2, false),
                Effect::IfBleedSuccessful(&[Effect::BloodToTarget(2)]),
            ],
        ),
        action_at(
            &[],
            target(TargetKind::Uncontrolled, &[Requirement::Younger]),
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                bleed(2, false),
                Effect::IfBleedSuccessful(&[Effect::BloodToUncontrolledTarget(2)]),
            ],
        ),
    ],
    ..CARD
};

/// "+1 stealth action. [cel] Ⓓ Bleed. [CEL] Ⓓ Enter combat with a locked
/// minion. This acting vampire gets 1 optional maneuver during that
/// combat." (1 blood.) A target unlocked by resolution: the action fizzles
/// (ANK 20210124, LSJ 20070411, LSJ 20090514).
pub static FLEETNESS: CardDef = CardDef {
    name: "Fleetness",
    modes: &[
        action_at(
            &[],
            None,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::AddStealth(1),
            ],
        ),
        action_at(
            &[],
            target(
                TargetKind::Minion,
                &[
                    Requirement::NotControlled,
                    Requirement::ActorReady,
                    Requirement::TargetLocked,
                ],
            ),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::EnterCombatWithTarget,
                Effect::InResultingCombat(BlockRider::Maneuver),
            ],
        ),
    ],
    ..CARD
};

/// "[cel][tha] Ⓓ Enter combat with a minion. This acting vampire gets 1
/// optional press during that combat. [CEL][THA] As above, and the opposing
/// minion cannot strike: combat ends during the first round of that
/// combat."
pub static HUNTERS_MARK: CardDef = CardDef {
    name: "Hunter's Mark",
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
    ..CARD
};

/// "Requires an Anarch. [cel] Ⓓ Steal 1 pool from another Methuselah. [pot]
/// Ⓓ Enter combat with a minion. [pre] Ⓓ Bleed with +1 bleed." A choice:
/// one Discipline (owner, 2026-09-26: rules-qa 25.2).
pub static LINE_BRAWL: CardDef = CardDef {
    name: "Line Brawl",
    modes: &[
        action_at(
            &[Requirement::ActorSectIn(&[Sect::Anarch])],
            target(TargetKind::Seat, &[Requirement::NotSelf]),
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
            None,
            &[Effect::DeclareAction(ActionKind::Bleed), PLUS_ONE_BLEED],
        ),
    ],
    ..CARD
};

/// "[pot][pre] Ⓓ Bleed with +1 bleed. If this action is blocked, before
/// range is determined during the first round of the resulting combat,
/// this vampire can gain +1 strength that combat. [POT][PRE] As above, but
/// at +2 bleed and this vampire can gain +2 strength."
pub static SHOW_OF_FORCE: CardDef = CardDef {
    name: "Show of Force",
    modes: &[
        action_at(
            &[],
            None,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                PLUS_ONE_BLEED,
                Effect::IfBlocks(BlockRider::StrengthBeforeRange(1)),
            ],
        ),
        action_at(
            &[],
            None,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                bleed(2, false),
                Effect::IfBlocks(BlockRider::StrengthBeforeRange(2)),
            ],
        ),
    ],
    ..CARD
};

/// "[obl] Ⓓ Bleed with +1 bleed. If the bleed is successful (for 1 or
/// more), the target Methuselah discards 2 cards of their choice. [OBL] Ⓓ
/// Remove 7 cards in your prey's ash heap from the game to burn 3 of their
/// pool." The cards removed are chosen by the player; there is no order to
/// the ash heap (LSJ 20090710).
pub static SHROUD_OF_DECAY: CardDef = CardDef {
    name: "Shroud of Decay",
    modes: &[
        action_at(
            &[],
            None,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                PLUS_ONE_BLEED,
                Effect::IfBleedSuccessful(&[Effect::TargetMethuselahDiscards(2)]),
            ],
        ),
        action_at(
            &[],
            target(
                TargetKind::Seat,
                &[Requirement::TargetPrey, Requirement::TargetAshAtLeast(7)],
            ),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::RemovePreyAshToBurnPool { remove: 7, pool: 3 },
            ],
        ),
    ],
    ..CARD
};

/// "Ⓓ Burn a location and burn 2 pool from its controller." (Gangrel.)
/// Directed at the location's controller (LSJ 20090324); one's own is
/// undirected.
pub static REWILDING: CardDef = CardDef {
    name: "Rewilding",
    modes: &[action_at(
        &[],
        target(
            TargetKind::Card,
            &[Requirement::TargetCard(CardFilter::Location)],
        ),
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::TargetControllerBurnsPool(2),
            Effect::BurnTargetCard,
        ],
    )],
    ..CARD
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
    modes: &[
        action_at(
            &[Requirement::ActorWithout(Holding::Named(
                "Preternatural Strength",
            ))],
            None,
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
            None,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(2),
                Effect::PutOnActor,
                strength_on_bearer(2),
                Effect::Layer(Modifier::BearerCannotPlay("Torn Signpost")),
            ],
        ),
    ],
    ..CARD
};

/// "+3 stealth action. [pot] Put this card on this vampire. This vampire
/// gets +1 strength. This vampire can strike: burn equipment. Burn this
/// card if this vampire is in torpor. A vampire can have only one Heroic
/// Might. [POT] As above, with an additional +1 strength, and this vampire
/// can strike: 2R damage." (3 blood.) The equipment is named with the
/// strike.
pub static HEROIC_MIGHT: CardDef = CardDef {
    name: "Heroic Might",
    modes: &[
        action_at(
            &[Requirement::ActorWithout(Holding::Named("Heroic Might"))],
            None,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(3),
                Effect::PutOnActor,
                strength_on_bearer(1),
            ],
        ),
        action_at(
            &[Requirement::ActorWithout(Holding::Named("Heroic Might"))],
            None,
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
            target: target(
                TargetKind::Card,
                &[
                    Requirement::TargetCard(CardFilter::Type(CardType::Equipment)),
                    Requirement::TargetOnOpposingCombatant,
                ],
            ),
            ..mode_of(
                &[Window::CombatStrikeDeclaration],
                Actor::Host,
                &[],
                &[Effect::Strike(StrikeKind::BurnEquipment)],
            )
        },
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Host,
            &[Requirement::SuperiorVersion],
            &[Effect::Strike(StrikeKind::Damage {
                n: 2,
                ranged: true,
                aggravated: false,
            })],
        ),
    ],
    riders: &[Rider {
        hook: Hook::HostToTorpor,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::BurnSelf],
    }],
    ..CARD
};

/// "+1 stealth action. [tha] Put this card on this vampire, locked; this
/// vampire takes 1 unpreventable environmental aggravated damage. During
/// your minion phase, this vampire can lock this card to unlock. A vampire
/// can have only one Rutor's Hand. [THA] As above, and this vampire can
/// burn 3 blood to be immune to this aggravated damage." (1 blood.) The
/// card unlocks in its controller's unlock phase like any.
pub static RUTORS_HAND: CardDef = CardDef {
    name: "Rutor's Hand",
    modes: &[
        action_at(
            &[Requirement::ActorWithout(Holding::Named("Rutor's Hand"))],
            None,
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
            None,
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
    abilities: &[mode_of(
        &[Window::AnyImpulse],
        Actor::Host,
        // Not while the vampire is unlocked (rules-qa 48.7).
        &[
            Requirement::OwnMinionPhase,
            Requirement::CardUnlocked,
            Requirement::HostLocked,
        ],
        &[Effect::LockSelf, Effect::UnlockHost],
    )],
    ..CARD
};

/// "[dom][pro] Only usable before range is determined. This vampire gets
/// +1 strength this combat. A vampire can play only one Monstrous Form each
/// round. [DOM][PRO] +1 stealth action. Put this card on this vampire.
/// During combat, you can lock this card to give this vampire +1 strength
/// this round, or 1 maneuver or press."
pub static MONSTROUS_FORM: CardDef = CardDef {
    name: "Monstrous Form",
    modes: &[
        ModeDef {
            limit: Some(Limit::PerRound),
            ..mode_of(
                &[Window::CombatBeforeRange],
                Actor::Combatant,
                &[Requirement::ActorVampire],
                &[Effect::StrengthThisCombat(1)],
            )
        },
        action_at(
            &[],
            None,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::PutOnActor,
            ],
        ),
    ],
    abilities: &[
        mode_of(
            &[Window::CombatUntimed],
            Actor::Host,
            &[Requirement::CardUnlocked],
            &[Effect::LockSelf, Effect::StrengthThisRound(1)],
        ),
        mode_of(
            &[Window::CombatDetermineRange],
            Actor::Host,
            &[Requirement::CardUnlocked],
            &[Effect::LockSelf, Effect::Maneuver],
        ),
        mode_of(
            &[Window::CombatPress],
            Actor::Host,
            &[Requirement::CardUnlocked],
            &[Effect::LockSelf, Effect::Press],
        ),
    ],
    ..CARD
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
    modes: &[
        action_at(
            &[],
            target(TargetKind::Minion, &[]),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::BloodToTargetToStart(3),
            ],
        ),
        mode_of(
            &[Window::CombatDamagePrevention],
            Actor::Combatant,
            &[Requirement::StrikeDamagePending],
            &[Effect::PreventStrikeDamage],
        ),
        mode_of(
            &[Window::CombatDamagePrevention],
            Actor::OwnOutOfCombat,
            &[Requirement::StrikeDamagePending],
            &[Effect::PreventStrikeDamage],
        ),
    ],
    ..CARD
};

/// "+1 stealth action. [tha] Employ an animal retainer from your hand
/// ignoring requirements (pay cost as normal). [THA] Put this card on a
/// minion you control. The attached minion gets 1 optional maneuver each
/// combat and +1 strength." (1 pool.) The retainer is named with the play
/// (as Pack Alpha, LSJ 20100130); its version follows the vampire's
/// Disciplines (LSJ 20080803); it is not an employ action (ANK 20180817).
pub static BIOTHAUMATURGIC_EXPERIMENT: CardDef = CardDef {
    name: "Biothaumaturgic Experiment",
    modes: &[
        ModeDef {
            terms: Some(Terms::EmployFromHandIgnoring(Label::Animal)),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::ActingCandidate,
                &[],
                &[
                    Effect::DeclareAction(ActionKind::Card),
                    Effect::AddStealth(1),
                    Effect::EmployNamed,
                ],
            )
        },
        action_at(
            &[],
            target(TargetKind::Minion, &[Requirement::Controlled]),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::PutOnTarget,
                strength_on_bearer(1),
            ],
        ),
    ],
    abilities: &[super::vampires::MANEUVER_EACH_COMBAT],
    ..CARD
};

/// "+1 stealth action. [aus] Ⓓ Look at your prey's hand and discard one
/// card of your choice from it. [AUS] Put this card in play. Your prey
/// plays with an open hand. Any minion can burn this card as a Ⓓ action."
/// (1 blood.) The replacement is not seen (LSJ 20100119, PIB 20121031).
pub static REVELATIONS: CardDef = CardDef {
    name: "Revelations",
    modes: &[
        action_at(
            &[],
            target(TargetKind::Seat, &[Requirement::TargetPrey]),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::DiscardFromTargetHand,
            ],
        ),
        action_at(
            &[],
            None,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::PutInPlay,
                Effect::Layer(Modifier::PreyOpenHand),
            ],
        ),
    ],
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::ActingCandidate,
        &[],
        &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::BurnSource,
        ],
    )],
    ..CARD
};

// ---- Actions, wave 2 (slice 11p) ----------------------------------------------

/// "[dom][pro] [ACTION MODIFIER] +1 stealth. Allies get -1 intercept.
/// [DOM][PRO] [COMBAT] Only usable before range is determined. This round,
/// this vampire gets +1 strength and 1 optional maneuver, and can prevent 1
/// damage. A vampire can play only one Obedient Flesh each round." (1
/// blood.)
pub static OBEDIENT_FLESH: CardDef = CardDef {
    name: "Obedient Flesh",
    modes: &[
        stealth(&[Effect::AddStealth(1), Effect::AlliesIntercept(-1)]),
        ModeDef {
            limit: Some(Limit::PerRound),
            ..mode_of(
                &[Window::CombatBeforeRange],
                Actor::Combatant,
                &[Requirement::ActorVampire],
                &[
                    Effect::StrengthThisRound(1),
                    Effect::OptionalManeuver,
                    Effect::PreventThisRound(1),
                ],
            )
        },
    ],
    ..CARD
};

/// "Vampires can burn this card as a Ⓓ action."
const VAMPIRES_BURN_THIS: ModeDef = mode_of(
    &[Window::ActionDeclare],
    Actor::ActingCandidate,
    &[Requirement::ActorVampire],
    &[
        Effect::DeclareAction(ActionKind::BurnCard),
        Effect::BurnSource,
    ],
);

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
    modes: &[action_at(
        &[],
        None,
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
            Effect::EnterWithCounters(1),
        ],
    )],
    abilities: &[
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[
                Requirement::YouControlThisCard,
                Requirement::ActorClan("Hecata"),
                Requirement::ActorCapacityAtLeast(4),
            ],
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::AddCounter(1),
            ],
        ),
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
    ..CARD
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
    modes: &[action_at(
        &[],
        None,
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
            Effect::EnterWithCounters(1),
        ],
    )],
    abilities: &[
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[
                Requirement::YouControlThisCard,
                Requirement::ActorClan("Hecata"),
            ],
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::AddCounter(1),
            ],
        ),
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::TargetVampire,
                    Requirement::ControlledByPredatorOrPrey,
                    Requirement::TargetCapacityBelowCounters,
                ],
            ),
            ..mode_of(
                &[Window::UnlockPhaseOwn],
                Actor::Controller,
                &[],
                &[Effect::TargetToUncontrolled, Effect::BurnSelf],
            )
        },
        VAMPIRES_BURN_THIS,
    ],
    ..CARD
};

/// "[ani] Ⓓ Bleed with +1 bleed. [ANI] Frenzy. Ⓓ Enter combat with and lock
/// a vampire. The target vampire is considered the acting minion during
/// that combat." The lock and the combat are simultaneous, both lost if the
/// action ends first (ANK 20200702); another effect changing the acting
/// minion later takes precedence (ANK 20211022). Another Methuselah's
/// ready vampire (rules-qa 17.3).
pub static DEEP_SONG: CardDef = CardDef {
    name: "Deep Song",
    modes: &[
        action_at(
            &[],
            None,
            &[Effect::DeclareAction(ActionKind::Bleed), PLUS_ONE_BLEED],
        ),
        action_at(
            &[],
            target(
                TargetKind::Minion,
                &[
                    Requirement::NotControlled,
                    Requirement::ActorReady,
                    Requirement::TargetVampire,
                ],
            ),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::Frenzy,
                Effect::EnterCombatWithTarget,
                Effect::TargetActsInCombat,
            ],
        ),
    ],
    ..CARD
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
    modes: &[
        action_at(
            &[],
            None,
            &[Effect::DeclareAction(ActionKind::Bleed), bleed(2, false)],
        ),
        action_at(
            &[],
            target(
                TargetKind::Minion,
                &[Requirement::TargetVampire, Requirement::Younger],
            ),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::StillYours,
                Effect::PutOnTarget,
                Effect::LockTarget,
                Effect::Layer(Modifier::HostDoesNotUnlock),
            ],
        ),
    ],
    abilities: &[mode_of(
        &[Window::AnyImpulse],
        Actor::Controller,
        &[
            Requirement::OwnMinionPhase,
            Requirement::EnteredBeforeThisTurn,
        ],
        &[Effect::TakeControlOfHost, Effect::BurnSelf],
    )],
    ..CARD
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
            modes: &[ModeDef {
                target: target(
                    TargetKind::Minion,
                    &[
                        Requirement::TargetVampire,
                        Requirement::TargetNotSuperior($tag),
                    ],
                ),
                ..mode_of(
                    &[Window::MasterPhase],
                    Actor::Methuselah,
                    &[],
                    &[
                        Effect::PutOnTarget,
                        Effect::Layer(Modifier::DisciplineLevel($tag)),
                    ],
                )
            }],
            ..CARD
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
    modes: &[action_at(
        &[BARON],
        None,
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
            Effect::BecomesVampire(Some(Sect::Anarch)),
        ],
    )],
    ..CARD
};

/// "Unique location. [You can] lock during your influence phase to add 1
/// blood to a <clan> in your uncontrolled region." (Its clan icon: played
/// with a ready vampire of the clan.)
macro_rules! influence_location {
    ($name:ident, $printed:expr, $clan:expr, $doc:expr) => {
        #[doc = $doc]
        pub static $name: CardDef = CardDef {
            name: $printed,
            modes: &[LOCATION],
            abilities: &[ModeDef {
                target: target(TargetKind::Uncontrolled, &[Requirement::TargetClan($clan)]),
                ..mode_of(
                    &[Window::InfluencePhaseOwn],
                    Actor::Controller,
                    &[Requirement::CardUnlocked],
                    &[Effect::LockSelf, Effect::BloodToUncontrolledTarget(1)],
                )
            }],
            ..CARD
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
/// turn (RTR 19960221: once per master phase per copy), in torpor too:
/// the text does not say ready (owner, 2026-09-26: rules-qa 28.6). The
/// counters go on in full, over capacity too (rules-qa 28.3).
pub static WASSERSCHLOSS_ANIF: CardDef = CardDef {
    name: "Wasserschloss Anif, Austria",
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::TargetClan("Tremere"),
                    Requirement::BloodAtLeast(1),
                ],
            ),
            ..mode_of(
                &[Window::MasterPhase],
                Actor::Controller,
                &[],
                &[Effect::BloodToCard(1)],
            )
        },
        ModeDef {
            target: target(
                TargetKind::Uncontrolled,
                &[Requirement::TargetClan("Tremere")],
            ),
            ..mode_of(
                &[Window::InfluencePhaseOwn],
                Actor::Controller,
                &[Requirement::CardUnlocked],
                &[Effect::LockSelf, Effect::CountersToUncontrolledTarget],
            )
        },
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[],
            &[
                Effect::DeclareAction(ActionKind::BurnCard),
                Effect::BurnSource,
                Effect::StealthForClan {
                    clan: "Malkavian",
                    n: 1,
                },
            ],
        ),
    ],
    ..CARD
};

/// "Trifle. Put this card in play. You can use 1 transfer to draw 1 card
/// from your crypt and then remove a crypt card in your uncontrolled region
/// from the game. You can use 4 transfers to burn this card and gain 2
/// pool." As often as the transfers last; the draw needs a card in the
/// crypt. Removed cards are public (LSJ 20080314).
pub static WIDER_VIEW: CardDef = CardDef {
    name: "Wider View",
    trifle: true,
    modes: &[LOCATION],
    abilities: &[
        mode_of(
            &[Window::Transfers],
            Actor::Controller,
            &[Requirement::CryptNotEmpty],
            &[Effect::SpendTransfers(1), Effect::DrawCryptThenRemove],
        ),
        mode_of(
            &[Window::Transfers],
            Actor::Controller,
            &[],
            &[
                Effect::SpendTransfers(4),
                Effect::BurnSelf,
                Effect::GainPool(2),
            ],
        ),
    ],
    ..CARD
};

/// "Trifle. Reveal the top card of your crypt. If it is a Hecata, draw it
/// and add 1 blood to it; otherwise, move it to the bottom of your crypt."
/// (Hecata: played with a ready Hecata.) Not with an empty crypt (RTR
/// 20000501, LSJ 20070411-2).
pub static FAMILY_GATHERING: CardDef = CardDef {
    name: "Family Gathering",
    trifle: true,
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[Requirement::CryptNotEmpty],
        &[Effect::RevealCryptTop("Hecata")],
    )],
    ..CARD
};

/// "<Minions> can steal this location as a Ⓓ action", for a card another
/// Methuselah controls.
const fn steal_this(requires: &'static [Requirement]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires,
        target: None,
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
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            target: target(TargetKind::Uncontrolled, &[]),
            ..mode_of(
                &[Window::InfluencePhaseOwn],
                Actor::Controller,
                &[],
                &[Effect::BloodToUncontrolledTarget(1)],
            )
        },
        steal_this(&[
            Requirement::ActorVampire,
            Requirement::OthersControlThisCard,
        ]),
    ],
    ..CARD
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
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::Controlled,
                Requirement::TargetVampire,
                Requirement::ActorReady,
            ],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[
                Effect::PutInPlay,
                Effect::ChooseTarget,
                Effect::RechoosesOnControlChange,
            ],
        )
    }],
    abilities: &[
        mode_of(
            &[Window::UnlockPhaseAny],
            Actor::ChosenController,
            &[Requirement::CardControllersTurn],
            &[Effect::BloodToChosen(2)],
        ),
        steal_this(&[
            Requirement::ActorVampire,
            Requirement::OthersControlThisCard,
        ]),
    ],
    ..CARD
};

/// "Unique. Put this card in play. During your unlock phase, you can give
/// your prey control of the Anarch Troublemaker and either lock up to two
/// vampires they control or burn an equipment on one of their minions."
/// A locked vampire is a legal choice (PIB 20121031).
pub static ANARCH_TROUBLEMAKER: CardDef = CardDef {
    name: "Anarch Troublemaker",
    modes: &[LOCATION],
    abilities: &[
        mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Controller,
            &[],
            &[Effect::GiveToPrey, Effect::LockPreyVampires(2)],
        ),
        ModeDef {
            target: target(
                TargetKind::Card,
                &[
                    Requirement::TargetCard(CardFilter::Type(CardType::Equipment)),
                    Requirement::TargetOnPreyMinion,
                ],
            ),
            ..mode_of(
                &[Window::UnlockPhaseOwn],
                Actor::Controller,
                &[],
                &[Effect::GiveToPrey, Effect::BurnTargetCard],
            )
        },
    ],
    ..CARD
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
    modes: &[LOCATION],
    abilities: &[
        mode_of(
            &[Window::DiscardPhase],
            Actor::Controller,
            &[Requirement::CardUnlocked],
            &[Effect::LockSelf, Effect::GrantDiscardAction],
        ),
        steal_this(&[
            Requirement::ActorSectIn(&[Sect::Anarch]),
            Requirement::OthersControlThisCard,
        ]),
    ],
    ..CARD
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
    modes: &[LOCATION],
    abilities: &[
        mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::PoolAtLeast(1),
                Requirement::AnarchCardInAshAndHand,
            ],
            &[
                Effect::LockSelf,
                Effect::PayPool(1),
                Effect::ExchangeForAnarchCard,
            ],
        ),
        mode_of(
            &[Window::CombatBeforeRange],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::AnarchCombatWithYours,
            ],
            &[Effect::LockSelf, Effect::EndCombat],
        ),
    ],
    ..CARD
};

/// A master card put in play by its player, requiring nothing.
const fn master_in_play(requires: &'static [Requirement], effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Methuselah,
        requires,
        target: None,
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
    modes: &[master_in_play(
        &[Requirement::ControlsReadySect(&[Sect::Anarch])],
        &[Effect::PutInPlay],
    )],
    abilities: &[
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::YourAttemptNeedsIntercept,
                Requirement::AttemptingSectIn(&[Sect::Anarch]),
            ],
            &[Effect::LockSelf, Effect::AttemptingIntercept(1)],
        ),
        mode_of(
            &[Window::ActionAfterResolution],
            Actor::Controller,
            &[Requirement::CardUnlocked, Requirement::AnarchHuntedYours],
            &[Effect::LockSelf, Effect::BloodToActing(1)],
        ),
    ],
    ..CARD
};

/// "Unique location. You can lock this card to give a minion you control
/// +2 intercept. Not usable during the first action in a minion phase."
/// An action canceled as its card was played is not the first action; one
/// canceled after it was played is (LSJ 20100505, ANK 20210124).
pub static CHANNEL_10: CardDef = CardDef {
    name: "Channel 10",
    modes: &[LOCATION],
    abilities: &[mode_of(
        &[Window::ActionStealthIntercept],
        Actor::Controller,
        &[
            Requirement::CardUnlocked,
            Requirement::YourAttemptNeedsIntercept,
            Requirement::NotFirstActionOfPhase,
        ],
        &[Effect::LockSelf, Effect::AttemptingIntercept(2)],
    )],
    ..CARD
};

/// "Unique location. You can lock this card as a vampire you control
/// announces an undirected action to give that vampire +1 stealth, even if
/// stealth is not yet needed."
pub static CREEPSHOW_CASINO: CardDef = CardDef {
    name: "Creepshow Casino",
    modes: &[LOCATION],
    abilities: &[ModeDef {
        waives_needed: true,
        ..mode_of(
            &[Window::ActionAsAnnounced],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::OtherActingControlled,
                Requirement::ActingVampire,
                Requirement::ActionUndirected,
            ],
            &[Effect::LockSelf, Effect::AddStealth(1)],
        )
    }],
    ..CARD
};

/// "Unique location. An Anarch can burn 1 blood once as they announce a
/// bleed action to get +1 bleed during that action." Any Anarch, whoever
/// controls the location; not an action requiring an Anarch (ANK
/// 20220317).
pub static CLUB_ILLUSION: CardDef = CardDef {
    name: "Club Illusion",
    modes: &[LOCATION],
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionAsAnnounced],
            Actor::ActingMinion,
            &[
                Requirement::IsBleed,
                Requirement::ActorSectIn(&[Sect::Anarch]),
                Requirement::ActorBloodAtLeast(1),
            ],
            &[Effect::HostBurnBlood(1), bleed(1, false)],
        )
    }],
    ..CARD
};

/// "Put this card in play. During a bleed action, an Assamite you control
/// can discard a combat card to get +1 bleed." (Banu Haqim.) Once per
/// action per copy (ANK 20221102-2), by the bleeding minion (ANK
/// 20220502), before resolution (ANK 20231124, LSJ 19970718).
pub static HAQIMS_LAW_RETRIBUTION: CardDef = CardDef {
    name: "Haqim's Law: Retribution",
    modes: &[master_in_play(&[], &[Effect::PutInPlay])],
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Type(CardType::Combat),
        }),
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionAsAnnounced, Window::ActionDuring],
            Actor::ActingMinion,
            &[
                Requirement::IsBleed,
                Requirement::YouControlThisCard,
                Requirement::ActorClan("Banu Haqim"),
                Requirement::HandHas(CardFilter::Type(CardType::Combat), 1),
            ],
            &[bleed(1, false)],
        )
    }],
    ..CARD
};

/// "Unique. Put this card on a ready minion. This minion cannot block.
/// Other minions can burn this card as a Ⓓ action."
pub static PENTEX_SUBVERSION: CardDef = CardDef {
    name: "Pentex(TM) Subversion",
    modes: &[ModeDef {
        target: target(TargetKind::Minion, &[Requirement::ActorReady]),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[
                Effect::PutOnTarget,
                Effect::Layer(Modifier::HostCannotBlock),
            ],
        )
    }],
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::ActingCandidate,
        &[Requirement::NotHost],
        &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::BurnSource,
        ],
    )],
    ..CARD
};

/// "Put this card on a ready vampire. Minions can enter combat with this
/// vampire as a +1 stealth Ⓓ action. This vampire can burn this card as a
/// +1 stealth Ⓓ action." Not with a minion of one's own (rules-qa 17.3).
pub static HAVEN_UNCOVERED: CardDef = CardDef {
    name: "Haven Uncovered",
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::TargetVampire, Requirement::ActorReady],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[Effect::PutOnTarget],
        )
    }],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::TargetIsHost,
                    Requirement::NotControlled,
                    Requirement::ActorReady,
                ],
            ),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::ActingCandidate,
                &[],
                &[
                    Effect::DeclareAction(ActionKind::Ability),
                    Effect::AddStealth(1),
                    Effect::EnterCombatWithTarget,
                ],
            )
        },
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[Requirement::ActorIsHost],
            &[
                Effect::DeclareAction(ActionKind::BurnCard),
                Effect::AddStealth(1),
                Effect::BurnSource,
            ],
        ),
    ],
    ..CARD
};

/// "Unique location. You can lock this card and burn 1 pool or 1 blood from
/// a ready Tremere you control during your master phase to move a Tremere
/// from torpor to their controller's ready region." (Tremere.) Any
/// Methuselah's Tremere; once each master phase.
pub static CHANTRY: CardDef = CardDef {
    name: "Chantry",
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::TargetClan("Tremere"),
                    Requirement::TargetInTorpor,
                ],
            ),
            ..mode_of(
                &[Window::MasterPhase],
                Actor::Controller,
                &[Requirement::CardUnlocked, Requirement::PoolAtLeast(1)],
                &[
                    Effect::LockSelf,
                    Effect::PayPool(1),
                    Effect::TargetLeavesTorpor,
                ],
            )
        },
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::TargetClan("Tremere"),
                    Requirement::TargetInTorpor,
                ],
            ),
            ..mode_of(
                &[Window::MasterPhase],
                Actor::Controller,
                &[
                    Requirement::CardUnlocked,
                    Requirement::OwnReadyClanWithBlood("Tremere"),
                ],
                &[
                    Effect::LockSelf,
                    Effect::OwnClanBurnsBlood("Tremere"),
                    Effect::TargetLeavesTorpor,
                ],
            )
        },
    ],
    ..CARD
};

/// "Choose two ready Toreador you control, put this card in play, and lock
/// one of the two. The locked Toreador does not unlock as normal. The other
/// Toreador's non-bleed actions cannot be blocked. Minions can burn this
/// card as a Ⓓ action; Nosferatu get -1 stealth during that action."
/// (Toreador.) Both chosen as it is played (owner, 2026-09-26: rules-qa
/// 28.5): the one to lock is the target, the other the terms' — it may be
/// the second Toreador of other Grand Balls (TOM 19960528), and stays
/// unblockable on non-bleed actions after a bleed (LSJ 19970814).
pub static TOREADOR_GRAND_BALL: CardDef = CardDef {
    name: "Toreador Grand Ball",
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::Controlled,
                Requirement::ActorReady,
                Requirement::TargetClan("Toreador"),
            ],
        ),
        terms: Some(Terms::OwnOfClan {
            clan: "Toreador",
            other: true,
        }),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[Requirement::ControlsReadyClan("Toreador", 2)],
            &[
                Effect::PutInPlay,
                Effect::ChooseTarget,
                Effect::LockTarget,
                Effect::ChooseSecondOfClan("Toreador"),
                Effect::Layer(Modifier::FirstChosenDoesNotUnlock),
                Effect::Layer(Modifier::SecondChosenUnblockable),
            ],
        )
    }],
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::ActingCandidate,
        &[],
        &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::BurnSource,
            Effect::StealthForClan {
                clan: "Nosferatu",
                n: -1,
            },
        ],
    )],
    ..CARD
};

/// "Unique contract. Trifle. Choose a ready Assamite you control and put
/// this card on a minion controlled by your prey. The chosen Assamite can
/// enter combat with the attached minion as a +1 stealth Ⓓ action. If the
/// attached minion is about to leave the ready region, you can burn this
/// card to gain 3 pool." (Banu Haqim.) The Assamite is chosen as it is
/// played (owner, 2026-09-26: rules-qa 28.5); the trifle's master phase action comes even if it contests
/// (RBK contested-cards); the pool comes as its controller is ousted too
/// (ANK 20180129-2), and if the card goes with the minion (PIB 20150512).
pub static PRIORITY_CONTRACT: CardDef = CardDef {
    name: "Priority Contract",
    trifle: true,
    modes: &[ModeDef {
        target: target(TargetKind::Minion, &[Requirement::ControlledByPrey]),
        terms: Some(Terms::OwnOfClan {
            clan: "Banu Haqim",
            other: false,
        }),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[Requirement::ControlsReadyClan("Banu Haqim", 1)],
            &[
                Effect::PutOnTarget,
                Effect::ChooseOwnOfClan("Banu Haqim"),
                Effect::PoolWhenHostLeaves(3),
            ],
        )
    }],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::TargetIsHost,
                Requirement::NotControlled,
                Requirement::ActorReady,
            ],
        ),
        ..mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[Requirement::ActorIsChosen],
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::EnterCombatWithTarget,
            ],
        )
    }],
    ..CARD
};

/// "Put this card in play. Each ready minion you control can enter combat
/// with a minion controlled by your prey as a Ⓓ action. You gain 1 pool
/// after a ready minion controlled by your prey is burned or sent to
/// torpor. During your influence phase, burn this card and burn 1 pool for
/// each ready minion controlled by your prey." Copies stack (LSJ
/// 20061018).
pub static FRONTAL_ASSAULT: CardDef = CardDef {
    name: "Frontal Assault",
    modes: &[master_in_play(
        &[],
        &[Effect::PutInPlay, Effect::BurnAtInfluencePerPreyReady],
    )],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::ControlledByPrey, Requirement::ActorReady],
        ),
        ..mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[Requirement::YouControlThisCard],
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::EnterCombatWithTarget,
            ],
        )
    }],
    riders: &[Rider {
        hook: Hook::ReadyMinionBurnedOrTorpor,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::ControllerGainsPoolIfPrey(1)],
    }],
    ..CARD
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
    modes: &[ModeDef {
        terms: Some(Terms::RecruitOrEmployFromHand),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::MasterForMinion,
            &[Requirement::ActorSectIn(&[Sect::Anarch])],
            &[Effect::LockActor, Effect::EmployNamed],
        )
    }],
    ..CARD
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
    modes: &[LOCATION],
    abilities: &[
        mode_of(
            &[Window::ActionAsAnnounced, Window::ReactionAsAnnounced],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::ActingClan("Nosferatu"),
                Requirement::ActionUndirected,
            ],
            &[
                Effect::LockSelf,
                Effect::UnlockActingIfSuccessful("Nosferatu"),
            ],
        ),
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::TargetClan("Nosferatu"),
                    Requirement::TargetInTorpor,
                ],
            ),
            ..mode_of(
                &[Window::AnyImpulse],
                Actor::Controller,
                &[],
                &[Effect::BurnSelf, Effect::TargetLeavesTorpor],
            )
        },
    ],
    ..CARD
};

/// "You can lock this card to give <any minion of the kind> +1 stealth":
/// the acting minion of any Methuselah, while stealth is needed.
macro_rules! any_stealth_location {
    ($name:ident, $printed:expr, $requires:expr, $doc:expr) => {
        #[doc = $doc]
        pub static $name: CardDef = CardDef {
            name: $printed,
            modes: &[LOCATION],
            abilities: &[mode_of(
                &[Window::ActionDuring, Window::ReactionDuringAction],
                Actor::Controller,
                $requires,
                &[Effect::LockSelf, Effect::AddStealth(1)],
            )],
            ..CARD
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
    modes: &[LOCATION],
    abilities: &[
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::YourAttemptNeedsIntercept,
            ],
            &[Effect::LockSelf, Effect::AttemptingIntercept(1)],
        ),
        ModeDef {
            windows: &[Window::ActionStealthIntercept],
            actor: Actor::Controller,
            requires: &[
                Requirement::CardUnlocked,
                Requirement::OthersAttemptNeedsIntercept,
                // Not burning a last pool this way (as True Love's Face).
                Requirement::PoolAtLeast(2),
            ],
            target: None,
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
    ..CARD
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
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::ActorReady,
                    Requirement::TargetVampire,
                ],
            ),
            ..mode_of(
                &[Window::MasterPhase],
                Actor::Controller,
                &[Requirement::CardUnlocked],
                &[Effect::LockSelf, Effect::BloodPerHostage],
            )
        },
        ModeDef {
            target: target(TargetKind::Minion, &[Requirement::TargetHostage]),
            limit: Some(Limit::PerTurn),
            ..mode_of(
                &[Window::UnlockPhaseAny],
                Actor::AnyReadyVampire,
                &[Requirement::ActorBloodAtLeast(2)],
                &[Effect::HostBurnBlood(2), Effect::BurnTargetHostages],
            )
        },
    ],
    riders: &[Rider {
        hook: Hook::ReadyMinionBurnedOrTorpor,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::HostageOnTorpor(3)],
    }],
    ..CARD
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
        mode_of(
            &[Window::CardAsPlayed],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::PlayedFrenzyOnYour("Salubri"),
            ],
            &[Effect::LockSelf, Effect::CancelPlayed { refund: false }],
        ),
    ],
    ..CARD
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
    modes: &[master_in_play(
        &[Requirement::NotPlayedThisGame],
        &[
            Effect::PutInPlay,
            Effect::EnterWithCounters(10),
            Effect::Layer(Modifier::ClanStat {
                clan: "Ravnos",
                stat: Stat::Bleed,
                n: 1,
                own: false,
                titled: false,
                during: &[],
            }),
            Effect::Layer(Modifier::ClanStat {
                clan: "Ravnos",
                stat: Stat::Strength,
                n: 1,
                own: false,
                titled: false,
                during: &[],
            }),
            Effect::Layer(Modifier::ClanCannotHunt("Ravnos")),
        ],
    )],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::TargetVampire,
                    Requirement::TargetClan("Ravnos"),
                    Requirement::OtherThanActor,
                ],
            ),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::ActingCandidate,
                &[Requirement::ActorClan("Ravnos")],
                &[
                    Effect::DeclareAction(ActionKind::Ability),
                    Effect::AddStealth(1),
                    Effect::HuntAction,
                    Effect::StealBloodFromTarget(1),
                ],
            )
        },
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::TargetVampire,
                    Requirement::TargetClan("Ravnos"),
                ],
            ),
            ..mode_of(
                &[Window::UnlockPhaseOwn],
                Actor::Methuselah,
                &[Requirement::CountersAtLeast(1)],
                &[Effect::CounterToTarget("Ravnos")],
            )
        },
    ],
    ..CARD
};

// ---- The pre-built decks' last equipment (slice 11r) ------------------------

/// "Location. While in play, this card does not count as equipment. This
/// vampire gets +1 bleed. They can burn this card before range is
/// determined to end combat. A vampire can have only one Living Manse."
/// (Tzimisce; 1 blood.) Equipped as equipment, then a location
/// (`cards::is_equipment_in_play`): no "cannot use equipment" bars its
/// burn (rules-qa 17.4), no equipment target reaches it, and an action at
/// it is directed at its controller (LSJ 20090324).
pub static LIVING_MANSE: CardDef = CardDef {
    name: "Living Manse",
    modes: &[ModeDef {
        requires: &[Requirement::ActorWithout(Holding::Named("Living Manse"))],
        effects: &[
            Effect::DeclareAction(ActionKind::Equip),
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Bleed,
                n: 1,
                during: &[],
                who: Bearer::Any,
            }),
        ],
        ..EQUIP
    }],
    abilities: &[mode_of(
        &[Window::CombatBeforeRange],
        Actor::Host,
        &[],
        &[Effect::BurnSelf, Effect::EndCombat],
    )],
    ..CARD
};

/// "Unique melee weapon. Strike: strength+1 aggravated damage. Once each
/// combat, this Salubri can burn 1 blood to cancel a grapple or aim card as
/// it is played by the opposing minion, and its cost is not paid. Once each
/// turn, if the opposing vampire is burned during this weapon's strike
/// resolution and the bearer remains ready, the bearer can unlock at the
/// end of combat." (Salubri; 2 pool.) The cancel follows the cancel rules
/// (in the as-played window, nothing but wakes around it: LSJ 20061207).
pub static SWORD_OF_THE_ARCHANGEL: CardDef = CardDef {
    name: "Sword of the Archangel",
    modes: &[EQUIP],
    abilities: &[
        weapon_strike(
            &[],
            &[Effect::Strike(StrikeKind::Melee {
                bonus: 1,
                aggravated: true,
            })],
        ),
        ModeDef {
            limit: Some(Limit::PerCombat),
            ..mode_of(
                &[Window::CardAsPlayed],
                Actor::Host,
                &[
                    Requirement::HostClan("Salubri"),
                    Requirement::HostBloodAtLeast(1),
                    Requirement::PlayedGrappleOrAimByOpponent,
                ],
                &[
                    Effect::HostBurnBlood(1),
                    Effect::CancelPlayed { refund: true },
                ],
            )
        },
        ModeDef {
            limit: Some(Limit::PerTurn),
            ..mode_of(
                &[Window::CombatWouldEnd],
                Actor::Host,
                // Not while the bearer is unlocked (rules-qa 48.7).
                &[
                    Requirement::HostReady,
                    Requirement::WeaponBurnedOpponent,
                    Requirement::HostLocked,
                ],
                &[Effect::UnlockHost],
            )
        },
    ],
    ..CARD
};

// ---- Combat cards, wave 2 (slice 11s) --------------------------------------

/// "Only usable at close range. [tha] Strike: hand strike at +1 damage.
/// Damage from this strike cannot be prevented by cards requiring Fortitude
/// [for]. The opposing vampire's strikes with weapons inflict no damage this
/// round. [THA] As above, but at +2 damage." (1 blood.) The +1 is the hand
/// strike's own damage (TOM 19960225); dodged, it does nothing to the
/// weapons (LSJ 20040928); a weapon's damage to its own bearer is not the
/// opponent's strike (LSJ 19980216).
pub static BLOOD_FURY: CardDef = CardDef {
    name: "Blood Fury",
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[Requirement::RangeClose],
            &[
                Effect::Strike(StrikeKind::Hand { bonus: 1 }),
                Effect::FortitudeProofStrike,
                Effect::NullifyOpposingWeapons,
            ],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[Requirement::RangeClose],
            &[
                Effect::Strike(StrikeKind::Hand { bonus: 2 }),
                Effect::FortitudeProofStrike,
                Effect::NullifyOpposingWeapons,
            ],
        ),
    ],
    ..CARD
};

/// "[obf] Only usable at long range. Strike: combat ends. [OBF] Strike:
/// combat ends." (1 blood.)
pub static NO_TRACE: CardDef = CardDef {
    name: "No Trace",
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[Requirement::RangeLong],
            &[Effect::Strike(StrikeKind::CombatEnds { unlock: false })],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[Effect::Strike(StrikeKind::CombatEnds { unlock: false })],
        ),
    ],
    ..CARD
};

/// "[obl] Additional strike (limited). [OBL] Maneuver."
pub static SHADOW_SHIFT: CardDef = CardDef {
    name: "Shadow Shift",
    modes: &[
        combat(
            &[Window::CombatAdditionalStrike],
            &[],
            &[Effect::AdditionalStrikes {
                n: 1,
                limited: true,
            }],
        ),
        combat(&[Window::CombatDetermineRange], &[], &[Effect::Maneuver]),
    ],
    ..CARD
};

/// "[pro] Damage from this vampire's hand strikes is aggravated this round.
/// [PRO] Press." (1 blood.) Played after both strikes are chosen, it still
/// reaches this round's strike, not once it has resolved (RTR 19960112).
pub static WOLF_CLAWS: CardDef = CardDef {
    name: "Wolf Claws",
    modes: &[
        combat(&[Window::CombatUntimed], &[], &[Effect::AggravatedHands]),
        combat(&[Window::CombatPress], &[], &[Effect::Press]),
    ],
    ..CARD
};

/// "[pro] [ACTION MODIFIER] +1 stealth. [PRO] [COMBAT] Strike: combat
/// ends." (1 blood.)
pub static RAPID_CHANGE: CardDef = CardDef {
    name: "Rapid Change",
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[Effect::Strike(StrikeKind::CombatEnds { unlock: false })],
        ),
    ],
    ..CARD
};

/// "Requires an Anarch. [cel] Additional strike (limited). [for] Prevent 2
/// damage. [tha] Strike, ranged: steal 1 blood or life (becoming blood),
/// with 1 optional maneuver." Played, it requires the Discipline used (LSJ
/// 20011204-3); the steal takes what is there, none from a vampire with no
/// blood (RTR 20010711); the prevention needs damage to prevent (LSJ
/// 20001114). Played at the maneuver, [tha] commits the round's strike.
pub static DIVERSION: CardDef = CardDef {
    name: "Diversion",
    modes: &[
        combat(
            &[Window::CombatAdditionalStrike],
            &[ANARCH],
            &[Effect::AdditionalStrikes {
                n: 1,
                limited: true,
            }],
        ),
        combat(
            &[Window::CombatDamagePrevention],
            &[ANARCH, Requirement::DamagePending],
            &[Effect::Prevent(2)],
        ),
        combat(
            &[
                Window::CombatStrikeDeclaration,
                Window::CombatDetermineRange,
            ],
            &[ANARCH],
            &[Effect::CommittedStrike(StrikeKind::StealBlood {
                n: 1,
                ranged: true,
            })],
        ),
    ],
    ..CARD
};

/// "[cel] Additional strike (limited). [CEL] Burn X blood to get X
/// additional strikes (limited)." (1 blood.) X is chosen as the card is
/// played, up to the blood left after the card's cost; the burn is an
/// effect no cost reduction reaches (ANK 20210226).
pub static LIGHTNING_REFLEXES: CardDef = CardDef {
    name: "Lightning Reflexes",
    modes: &[
        combat(
            &[Window::CombatAdditionalStrike],
            &[],
            &[Effect::AdditionalStrikes {
                n: 1,
                limited: true,
            }],
        ),
        ModeDef {
            terms: Some(Terms::XBlood),
            ..combat(
                &[Window::CombatAdditionalStrike],
                &[],
                &[
                    Effect::ActorBurnsBloodX,
                    Effect::AdditionalStrikesX { limited: true },
                ],
            )
        },
    ],
    ..CARD
};

/// "[tha] Strike: dodge. [THA] As above, with 1 additional strike: dodge
/// (limited)."
pub static WIND_DANCE: CardDef = CardDef {
    name: "Wind Dance",
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[Effect::Strike(StrikeKind::Dodge)],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[
                Effect::Strike(StrikeKind::Dodge),
                Effect::AdditionalStrikeOnly {
                    kind: StrikeKind::Dodge,
                    limited: true,
                },
            ],
        ),
    ],
    ..CARD
};

/// "[aus] Strike: hand strike or use a melee weapon strike. This strike is
/// at +2 damage. [AUS] Burn 1 blood to cancel the opposing minion's strike
/// card as it is played, and its cost is not paid (the minion chooses a
/// strike again)." (Salubri.) [aus] splits in two: the hand strike, or a
/// melee weapon of this vampire's (its own strike and riders). The blood of
/// [AUS] is an effect (ANK 20210226).
pub static ANTICIPATION: CardDef = CardDef {
    name: "Anticipation",
    modes: &[
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[Effect::Strike(StrikeKind::Hand { bonus: 2 })],
        ),
        ModeDef {
            target: target(
                TargetKind::Card,
                &[
                    Requirement::TargetOnActor,
                    Requirement::TargetCard(CardFilter::MeleeWeapon),
                ],
            ),
            ..combat(
                &[Window::CombatStrikeDeclaration],
                &[Requirement::ActorMayUseEquipment],
                &[Effect::WeaponStrikePlus(2)],
            )
        },
        combat(
            &[Window::CardAsPlayed],
            &[
                Requirement::PlayedStrikeByOpponent,
                Requirement::ActorBloodAtLeast(1),
            ],
            &[
                Effect::ActorBurnsBlood(1),
                Effect::CancelPlayed { refund: true },
            ],
        ),
    ],
    ..CARD
};

/// "Frenzy. Only usable before range is determined. [ani] Only usable
/// during the first round. This combat, the opposing minion cannot maneuver
/// to close range, press to continue combat or use equipment. [ANI] This
/// combat, combat cards cost the opposing vampire +1 blood. A vampire can
/// play only one Terror Frenzy at superior each combat." (1 blood.) Played
/// on the opposing minion, whom it affects (LSJ 20051113, ANK 20220409;
/// rules-qa 28.11); the surcharge reaches cards played at the end of a
/// round (ANK 20191221).
pub static TERROR_FRENZY: CardDef = CardDef {
    name: "Terror Frenzy",
    modes: &[
        combat(
            &[Window::CombatBeforeRange],
            &[Requirement::FirstRound],
            &[Effect::Frenzy, Effect::OpposingRestraint],
        ),
        ModeDef {
            limit: Some(Limit::PerCombatThisMode),
            ..combat(
                &[Window::CombatBeforeRange],
                &[],
                &[Effect::Frenzy, Effect::OpposingCombatCardsCost(1)],
            )
        },
    ],
    ..CARD
};

/// "Only usable before range is determined during the first round. [for]
/// This combat, this vampire can prevent 1 damage each round. This combat,
/// frenzy cards cannot be used on this vampire; cancel the effects of
/// frenzy cards that have already been used on this vampire this combat.
/// [FOR] As above, but this vampire can prevent 2 damage each round."
/// (Salubri; 1 blood.)
pub static TRANQUILITY_SHIELD: CardDef = CardDef {
    name: "Tranquility Shield",
    modes: &[
        combat(
            &[Window::CombatBeforeRange],
            &[Requirement::FirstRound],
            &[Effect::PreventEachRound(1), Effect::FrenzyShield],
        ),
        combat(
            &[Window::CombatBeforeRange],
            &[Requirement::FirstRound],
            &[Effect::PreventEachRound(2), Effect::FrenzyShield],
        ),
    ],
    ..CARD
};

/// "Only usable before range is determined. [pro] This combat, if a damage
/// is successfully inflicted on this vampire in a given round, any
/// additional damage inflicted on this vampire in the same round is
/// automatically prevented. Aggravated damage cannot be prevented this way.
/// [PRO] As above, but aggravated damage is prevented this way as well."
/// [pro] leaves aggravated damage alone, even treated as normal (LSJ
/// 20040812-2); [PRO]'s point let through is its controller's pick (LSJ
/// 20021001).
pub static FLESH_OF_MARBLE: CardDef = CardDef {
    name: "Flesh of Marble",
    modes: &[
        combat(
            &[Window::CombatBeforeRange],
            &[],
            &[Effect::FleshOfMarble { aggravated: false }],
        ),
        combat(
            &[Window::CombatBeforeRange],
            &[],
            &[Effect::FleshOfMarble { aggravated: true }],
        ),
    ],
    ..CARD
};

/// "Only usable before range is determined during the first round. A
/// vampire can play only one Weather Control each combat. [tha] Both
/// combatants and each retainer on them take 1 unpreventable environmental
/// damage before range is determined each round this combat. [THA] As
/// above, but the amount of damage inflicted is increased by 1 in each
/// subsequent round." Dealt as the before-range step ends, after both
/// minions' before-range effects (ANK 20170930), even if the combat ends
/// then (LSJ 19971110, ANK 20190725).
pub static WEATHER_CONTROL: CardDef = CardDef {
    name: "Weather Control",
    modes: &[
        ModeDef {
            limit: Some(Limit::PerCombat),
            ..combat(
                &[Window::CombatBeforeRange],
                &[Requirement::FirstRound],
                &[Effect::WeatherDamage { escalate: false }],
            )
        },
        ModeDef {
            limit: Some(Limit::PerCombat),
            ..combat(
                &[Window::CombatBeforeRange],
                &[Requirement::FirstRound],
                &[Effect::WeatherDamage { escalate: true }],
            )
        },
    ],
    ..CARD
};

/// "Only usable at close range at the end of a round during which this
/// vampire successfully inflicted more damage than the opposing vampire.
/// Not usable by a vampire being burned or going to torpor. [pot] Put this
/// card on the opposing vampire and send them to torpor. The attached
/// vampire gets -1 strength. They can burn 3 blood to burn this card. A
/// vampire can have only one Disarm. [POT] As above, but the attached
/// vampire gets -2 strength." Played after the presses, even when the round
/// ended early (RTR 20030519), before or after the other end-of-round
/// cards (LSJ 20021113, ANK 20180910-1); environmental damage does not
/// count (LSJ 20010812). The card itself goes on the vampire (rules-qa
/// 47.1).
pub static DISARM: CardDef = CardDef {
    name: "Disarm",
    modes: &[
        combat(
            &[Window::CombatEndOfRound],
            &DISARM_REQUIRES,
            &[
                Effect::PutOnOpponent,
                strength_on_bearer(-1),
                Effect::OpponentToTorpor,
            ],
        ),
        combat(
            &[Window::CombatEndOfRound],
            &DISARM_REQUIRES,
            &[
                Effect::PutOnOpponent,
                strength_on_bearer(-2),
                Effect::OpponentToTorpor,
            ],
        ),
    ],
    abilities: &[mode_of(
        &[Window::AnyImpulse],
        Actor::Host,
        &[Requirement::HostBloodAtLeast(3)],
        &[Effect::HostBurnBlood(3), Effect::BurnSelf],
    )],
    ..CARD
};

const DISARM_REQUIRES: [Requirement; 5] = [
    Requirement::RangeClose,
    Requirement::ActorReady,
    Requirement::OpponentVampire,
    Requirement::InflictedMoreThisRound,
    Requirement::OpponentWithout(Holding::Named("Disarm")),
];

/// "[obf] [COMBAT] Cancel a combat card requiring Auspex [aus] as it is
/// played, and its cost is not paid. [OBF] [ACTION MODIFIER] Cancel a
/// reaction card requiring Auspex as it is played, and its cost is not
/// paid." A card requiring Auspex as played: not one played with another
/// Discipline (PIB 20130704).
pub static HIDE_THE_MIND: CardDef = CardDef {
    name: "Hide the Mind",
    modes: &[
        combat(
            &[Window::CardAsPlayed],
            &[
                Requirement::PlayedCardType(CardType::Combat),
                Requirement::PlayedRequiresDiscipline("aus"),
            ],
            &[Effect::CancelPlayed { refund: true }],
        ),
        mode_of(
            &[Window::CardAsPlayed],
            Actor::ActingMinion,
            &[
                Requirement::PlayedCardType(CardType::Reaction),
                Requirement::PlayedRequiresDiscipline("aus"),
            ],
            &[Effect::CancelPlayed { refund: true }],
        ),
    ],
    ..CARD
};

/// "Only usable before range is determined. A vampire can play only one
/// Angel's Gift each round. [COMBAT] Equip this vampire with a melee weapon
/// from your hand (requirements and cost apply as normal). [COMBAT] This
/// round, this vampire gets 1 optional maneuver, only usable to get to
/// close range." (Salubri.) Two ways to play it. The weapon is named as the
/// card is played (LSJ 20100130); not an action, its requirements are the
/// weapon's own (LSJ 20100725).
pub static ANGELS_GIFT: CardDef = CardDef {
    name: "Angel's Gift",
    modes: &[
        ModeDef {
            terms: Some(Terms::EquipFromHand {
                filter: CardFilter::MeleeWeapon,
                blood_share: false,
            }),
            limit: Some(Limit::PerRound),
            ..combat(&[Window::CombatBeforeRange], &[], &[Effect::EquipNamed])
        },
        ModeDef {
            limit: Some(Limit::PerRound),
            ..combat(
                &[Window::CombatBeforeRange],
                &[],
                &[Effect::OptionalCloseManeuver],
            )
        },
    ],
    ..CARD
};

/// "Only usable before range is determined. A vampire can play only one
/// Contraband each combat. [obf] Equip this vampire with a non-unique
/// equipment from your hand (requirements and cost apply as normal). [OBF]
/// As above, and this vampire can pay up to half the cost rounded down of
/// that equipment with their blood." (Ravnos.) Named as it is played (LSJ
/// 20100130); not an action — no action's cost reduction applies (LSJ
/// 20090115-1, ANK 20170309), others do (ANK 20230824); the equipment is
/// not played by the vampire (ANK 20201229).
pub static CONTRABAND: CardDef = CardDef {
    name: "Contraband",
    modes: &[
        ModeDef {
            terms: Some(Terms::EquipFromHand {
                filter: CardFilter::NonUniqueEquipment,
                blood_share: false,
            }),
            limit: Some(Limit::PerCombat),
            ..combat(&[Window::CombatBeforeRange], &[], &[Effect::EquipNamed])
        },
        ModeDef {
            terms: Some(Terms::EquipFromHand {
                filter: CardFilter::NonUniqueEquipment,
                blood_share: true,
            }),
            limit: Some(Limit::PerCombat),
            ..combat(&[Window::CombatBeforeRange], &[], &[Effect::EquipNamed])
        },
    ],
    ..CARD
};

/// "Only usable by an unlocked vampire not involved in the combat.
/// [aus][for] Prevent 1 damage to a minion or retainer in combat.
/// [AUS][FOR] Burn X blood to prevent X+1 damage to a minion or retainer in
/// combat." Any Methuselah's vampire (ANK 20181101), in torpor too (LSJ
/// 20020416); the X blood is an effect, not a cost (ANK 20210226); no
/// damage to prevent, not playable (LSJ 20001114). Each version splits by
/// the kind of target: a minion, or a retainer.
pub static MARTYRS_RESILIENCE: CardDef = CardDef {
    name: "Martyr's Resilience",
    modes: &[
        martyr(TargetKind::Minion, None, &[Effect::PreventOnTarget(1)]),
        martyr(TargetKind::Card, None, &[Effect::PreventOnTarget(1)]),
        martyr(
            TargetKind::Minion,
            Some(Terms::XBlood),
            &[Effect::ActorBurnsBloodX, Effect::PreventOnTargetXPlus(1)],
        ),
        martyr(
            TargetKind::Card,
            Some(Terms::XBlood),
            &[Effect::ActorBurnsBloodX, Effect::PreventOnTargetXPlus(1)],
        ),
    ],
    ..CARD
};

const fn martyr(kind: TargetKind, terms: Option<Terms>, effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::CombatDamagePrevention],
        actor: Actor::UnlockedOutOfCombat,
        requires: &[],
        target: Some(TargetSpec {
            kind,
            filter: &[Requirement::TargetInCombatWithDamage],
        }),
        terms,
        waives_needed: false,
        limit: None,
        effects,
    }
}

// ---- Reactions and action modifiers, wave 2 (slice 11t) --------------------

/// "Only usable during a bleed action. [dom] +1 bleed (limited). [DOM] +2
/// bleed (limited)."
pub static THREATS: CardDef = CardDef {
    name: "Threats",
    modes: &[
        modifier(&[Requirement::IsBleed], &[bleed(1, true)]),
        modifier(&[Requirement::IsBleed], &[bleed(2, true)]),
    ],
    ..CARD
};

/// "Not usable during a bleed action. [obf] +2 stealth. [OBF] +3 stealth."
/// (1 blood.)
pub static FORGOTTEN_LABYRINTH: CardDef = CardDef {
    name: "Forgotten Labyrinth",
    modes: &[
        ModeDef {
            requires: &[Requirement::StealthNeeded, Requirement::NotBleed],
            ..stealth(&[Effect::AddStealth(2)])
        },
        ModeDef {
            requires: &[Requirement::StealthNeeded, Requirement::NotBleed],
            ..stealth(&[Effect::AddStealth(3)])
        },
    ],
    ..CARD
};

/// "Not usable during a bleed action. [obf] +1 stealth. [OBF] Only usable
/// after resolution of a blocked action. This vampire burns 1 blood to
/// continue the action as if unblocked." (Ravnos.) The blood is an effect
/// (ANK 20210226).
pub static GO_GETTER: CardDef = CardDef {
    name: "Go-getter",
    modes: &[
        ModeDef {
            requires: &[Requirement::StealthNeeded, Requirement::NotBleed],
            ..stealth(&[Effect::AddStealth(1)])
        },
        mode_of(
            &[Window::ActionAfterResolution],
            Actor::ActingMinion,
            &[
                Requirement::NotBleed,
                Requirement::ActionBlocked,
                Requirement::ActorReady,
                Requirement::ActorBloodAtLeast(1),
            ],
            &[Effect::BurnActorBlood(1), Effect::ContinueAfterResolution],
        ),
    ],
    ..CARD
};

/// "[obf] +1 stealth. [OBF] Only usable if a minion attempts to block. That
/// attempt fails and the blocking minion cannot attempt to block this
/// action again." (1 blood.)
pub static ELDER_IMPERSONATION: CardDef = CardDef {
    name: "Elder Impersonation",
    modes: &[
        stealth(&[Effect::AddStealth(1)]),
        on_attempt!(&[], &[Effect::AttemptFails]),
    ],
    ..CARD
};

/// "[ani][pro] +1 stealth. If this vampire is blocked, they can prevent 1
/// damage during the resulting combat. [ANI][PRO] Only usable as a
/// non-bleed action is announced. Vampires cannot block this action." (2
/// blood.) The prevention only in a block's combat, again for a second
/// block (LSJ 20010813, 20010819-2, 20010814-2).
pub static BEAST_MELD: CardDef = CardDef {
    name: "Beast Meld",
    modes: &[
        stealth(&[
            Effect::AddStealth(1),
            Effect::IfBlocks(BlockRider::PreventInCombat(1)),
        ]),
        mode_of(
            &[Window::ActionAsAnnounced],
            Actor::ActingMinion,
            &[Requirement::NotBleed],
            &[Effect::VampiresCannotBlock],
        ),
    ],
    ..CARD
};

/// "Only usable as an action to recruit or employ a wraith or zombie is
/// announced. [obl] +1 stealth, even if stealth is not yet needed. [OBL] As
/// above, and if the action is successful, this vampire can burn 1 blood to
/// unlock after action resolution."
pub static PATHS_IN_TWO_WORLDS: CardDef = CardDef {
    name: "Paths in Two Worlds",
    modes: &[
        ModeDef {
            waives_needed: true,
            ..mode_of(
                &[Window::ActionAsAnnounced],
                Actor::ActingMinion,
                &[WRAITH_OR_ZOMBIE],
                &[Effect::AddStealth(1)],
            )
        },
        ModeDef {
            waives_needed: true,
            ..mode_of(
                &[Window::ActionAsAnnounced],
                Actor::ActingMinion,
                &[WRAITH_OR_ZOMBIE],
                &[Effect::AddStealth(1), Effect::UnlockForBloodIfSuccessful(1)],
            )
        },
    ],
    ..CARD
};

const WRAITH_OR_ZOMBIE: Requirement =
    Requirement::RecruitingSubtype(&[Label::Wraith, Label::Zombie]);

/// "[for] If this action is blocked, all damage inflicted on vampires
/// during the resulting combat is aggravated. If a vampire is currently
/// attempting to block, they can cancel their block attempt. [FOR] As
/// above, but without the option to cancel their block attempt." Only in
/// the combat, not after it (LSJ 20020314, PIB 20130319); environmental and
/// retainer damage too (PIB 20150603-1).
pub static DAWN_OPERATION: CardDef = CardDef {
    name: "Dawn Operation",
    modes: &[
        modifier(&[], &[Effect::DawnOperation { cancel: true }]),
        modifier(&[], &[Effect::DawnOperation { cancel: false }]),
    ],
    ..CARD
};

/// "Only usable by a locked vampire. [ani] This vampire unlocks and attempts
/// to block. If this vampire does not block this action, lock them after
/// action resolution. [ANI] As above, and if this vampire does not block
/// this action, put this card on them after action resolution. This
/// vampire can burn this card to get +1 intercept. A vampire can have only
/// one Dogged Pursuit." (Ravnos.)
pub static DOGGED_PURSUIT: CardDef = CardDef {
    name: "Dogged Pursuit",
    modes: &[
        unlock_attempt!(
            &[Requirement::LockedActor, Requirement::CanAttemptBlock],
            &[
                Effect::UnlockActor,
                Effect::AttemptBlock,
                Effect::LockIfNoBlock,
            ]
        ),
        unlock_attempt!(
            &[Requirement::LockedActor, Requirement::CanAttemptBlock],
            &[
                Effect::UnlockActor,
                Effect::AttemptBlock,
                Effect::LockIfNoBlock,
                Effect::PutOnActorIfNoBlock,
            ]
        ),
    ],
    abilities: &[mode_of(
        &[Window::ActionStealthIntercept],
        Actor::Host,
        &[
            Requirement::HostAttempting,
            Requirement::YourAttemptNeedsIntercept,
        ],
        &[Effect::BurnSelf, Effect::AttemptingIntercept(1)],
    )],
    ..CARD
};

/// "Only usable during a bleed action directed at you. Do not replace until
/// after this action. +2 intercept. A vampire cannot play both Elder
/// Intervention and cards named Pack Tactics during the same action." (1
/// blood.) Any bleed against you, redirected ones included (RTR 19960530);
/// Pack Tactics is not in V5.
pub static ELDER_INTERVENTION: CardDef = CardDef {
    name: "Elder Intervention",
    delayed_replacement: Some(Until::EndOfAction),
    modes: &[intercept(
        &[
            Requirement::IsBleed,
            Requirement::DirectedAtYou,
            Requirement::InterceptNeeded,
        ],
        &[Effect::AddIntercept(2)],
    )],
    ..CARD
};

/// "Only usable by a locked vampire. This vampire wakes (they ignore the
/// requirement to be unlocked for playing reaction cards and attempting to
/// block until the end of the action). If they do not block this action,
/// they burn 1 blood before action resolution." The blood as the action
/// begins to resolve (LSJ 19990421) — another minion's block included, the
/// block being the resolution (rules-qa 30.2) — not after a block of theirs, even one
/// continued (LSJ 20070417), blocks before the card not counted (ANK
/// 20230319); playable in the as-played window of an action, not in combat
/// (ANK 20210819), after resolution too (LSJ 20091123).
pub static FORCED_AWAKENING: CardDef = CardDef {
    name: "Forced Awakening",
    modes: &[mode_of(
        &[Window::Wake, Window::CardAsPlayed],
        Actor::ReactingMinion,
        &[Requirement::LockedActor],
        &[Effect::Wake, Effect::BurnIfNoBlock(1)],
    )],
    ..CARD
};

/// "[for] Only usable by a locked vampire who has blocked, after block
/// resolution. Unlock this vampire. [FOR] Only usable by a locked vampire
/// during an action directed at you (or a card you control). Unlock this
/// vampire." (1 blood.)
pub static FORCED_VIGILANCE: CardDef = CardDef {
    name: "Forced Vigilance",
    modes: &[
        mode_of(
            &[Window::ActionAfterCombat],
            Actor::ReactingMinion,
            &[Requirement::LockedActor, Requirement::ActorHasBlocked],
            &[Effect::UnlockActor],
        ),
        mode_of(
            &[Window::ReactionUnlock],
            Actor::ReactingMinion,
            &[Requirement::LockedActor, Requirement::DirectedAtYou],
            &[Effect::UnlockActor],
        ),
    ],
    ..CARD
};

/// "[aus] +1 intercept. [AUS] As above, and if this vampire blocks, put this
/// card on the acting minion; you still control this card. During a bleed
/// action against the controller of the attached minion, this reacting
/// vampire can burn this card to get +1 bleed." The card goes to the ash
/// heap as played and on the acting minion as the block lands (ANK
/// 20220805; rules-qa 47.1); the bleed burn only before resolution (ANK
/// 20180531, 20231124).
pub static MELANGE: CardDef = CardDef {
    name: "Melange",
    modes: &[
        intercept(&[Requirement::InterceptNeeded], &[Effect::AddIntercept(1)]),
        intercept(
            &[Requirement::InterceptNeeded],
            &[
                Effect::AddIntercept(1),
                Effect::PutOnActingIfBlocks,
                Effect::StillYours,
            ],
        ),
    ],
    abilities: &[mode_of(
        &[Window::ActionDuring],
        Actor::ActingMinion,
        &[
            Requirement::YouControlThisCard,
            Requirement::ActingIsBy,
            Requirement::IsBleed,
            Requirement::BleedAgainstHostController,
        ],
        &[Effect::BurnSelf, bleed(1, false)],
    )],
    ..CARD
};

/// "Only usable by a locked vampire during a bleed action against you.
/// [ani] Unlock this vampire. [ANI] As above, with 1 optional press during
/// the resulting combat if this vampire blocks."
pub static RATS_WARNING: CardDef = CardDef {
    name: "Rat's Warning",
    modes: &[
        mode_of(
            &[Window::ReactionUnlock],
            Actor::ReactingMinion,
            &RATS_WARNING_REQUIRES,
            &[Effect::UnlockActor],
        ),
        mode_of(
            &[Window::ReactionUnlock],
            Actor::ReactingMinion,
            &RATS_WARNING_REQUIRES,
            &[Effect::UnlockActor, Effect::IfBlocks(BlockRider::Press)],
        ),
    ],
    ..CARD
};

const RATS_WARNING_REQUIRES: [Requirement; 3] = [
    Requirement::LockedActor,
    Requirement::IsBleed,
    Requirement::DirectedAtYou,
];

/// "[dom] Only usable if a younger vampire is bleeding you, after blocks are
/// declined. Lock this reacting vampire. Change the target of the bleed to
/// another Methuselah other than the acting vampire's controller (that
/// Methuselah can attempt to block). [DOM] As above, but the acting vampire
/// can be the same age or older." Blocks failed on stealth were declined
/// (ANK 20211003); before any "would be successful" card (LSJ 19980105);
/// only to a Methuselah the bleed could target (PIB 20130711).
pub static REDIRECTION: CardDef = CardDef {
    name: "Redirection",
    modes: &[
        ModeDef {
            target: REDIRECT_TARGET,
            ..mode_of(
                &[Window::ActionBlocksDeclined],
                Actor::ReactingMinion,
                &[
                    Requirement::DirectedAtYou,
                    Requirement::IsBleed,
                    Requirement::BlocksDeclined,
                    Requirement::ActingVampire,
                    Requirement::ActingYounger,
                ],
                &[Effect::LockActor, Effect::RedirectBleed],
            )
        },
        ModeDef {
            target: REDIRECT_TARGET,
            ..mode_of(
                &[Window::ActionBlocksDeclined],
                Actor::ReactingMinion,
                &[
                    Requirement::DirectedAtYou,
                    Requirement::IsBleed,
                    Requirement::BlocksDeclined,
                    Requirement::ActingVampire,
                ],
                &[Effect::LockActor, Effect::RedirectBleed],
            )
        },
    ],
    ..CARD
};

/// "Requires a vampire with capacity 7 or more. [ani] +1 intercept. [ANI]
/// Only usable by a locked vampire. This vampire unlocks and attempts to
/// block." Already attempting, it keeps its attempt (ANK 20181122-2); only
/// if it could attempt (LSJ 20010714-2), the usual prey, predator and
/// target limits applying (ANK 20180623); the attempt stands without a
/// target left (LSJ 20090514).
pub static SENSE_THE_SAVAGE_WAY: CardDef = CardDef {
    name: "Sense the Savage Way",
    modes: &[
        intercept(
            &[
                Requirement::ActorCapacityAtLeast(7),
                Requirement::InterceptNeeded,
            ],
            &[Effect::AddIntercept(1)],
        ),
        unlock_attempt!(
            &[
                Requirement::ActorCapacityAtLeast(7),
                Requirement::LockedActor,
                Requirement::CanAttemptBlock,
            ],
            &[Effect::UnlockActor, Effect::AttemptBlock]
        ),
    ],
    ..CARD
};

/// "[aus] Reduce a bleed against you by 1. [AUS] Reduce a bleed against you
/// by 2." Before the bleed is successful (TOM 19960303, LSJ 20061212),
/// before or after block attempts.
pub static TELEPATHIC_COUNTER: CardDef = CardDef {
    name: "Telepathic Counter",
    modes: &[
        mode_of(
            &[Window::ReactionDuringAction],
            Actor::ReactingMinion,
            &[Requirement::IsBleed, Requirement::DirectedAtYou],
            &[Effect::ReduceBleed(1)],
        ),
        mode_of(
            &[Window::ReactionDuringAction],
            Actor::ReactingMinion,
            &[Requirement::IsBleed, Requirement::DirectedAtYou],
            &[Effect::ReduceBleed(2)],
        ),
    ],
    ..CARD
};

// ---- Pool actions, wave 1 (slice 11u) ---------------------------------------

/// "[dom] Ⓓ Bleed with +1 bleed. [DOM] +1 stealth action. Add 2 blood to a
/// younger vampire in your uncontrolled region."
pub static SCOUTING_MISSION: CardDef = CardDef {
    name: "Scouting Mission",
    modes: &[
        bleed_action!(1),
        uncontrolled_blood!(&[Requirement::Younger], 2),
    ],
    ..CARD
};

/// "[pre] Ⓓ Bleed with +1 bleed. [PRE] As above, and gain 1 pool if the
/// bleed is successful (for 1 or more)."
pub static SOCIAL_CHARM: CardDef = CardDef {
    name: "Social Charm",
    modes: &[
        bleed_action!(1),
        action_at(
            &[],
            None,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                PLUS_ONE_BLEED,
                Effect::IfBleedSuccessful(&[Effect::GainPool(1)]),
            ],
        ),
    ],
    ..CARD
};

/// "[pre] Ⓓ Bleed with +2 bleed. [PRE] As above, and if the bleed is
/// successful (for 1 or more), add 1 blood to a vampire in your
/// uncontrolled region." (1 blood.) The vampire is chosen upon declaration
/// (ANK 20240706).
pub static PUBLIC_TRUST: CardDef = CardDef {
    name: "Public Trust",
    modes: &[
        bleed_action!(2),
        action_at(
            &[],
            target(TargetKind::Uncontrolled, &[]),
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                bleed(2, false),
                Effect::IfBleedSuccessful(&[Effect::BloodToUncontrolledTarget(1)]),
            ],
        ),
    ],
    ..CARD
};

/// "+1 stealth action. [for] This vampire gains 2 blood. [FOR] This vampire
/// gains 3 blood."
pub static RESTORATION: CardDef = CardDef {
    name: "Restoration",
    modes: &[
        action_at(
            &[],
            None,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::BloodToActing(2),
            ],
        ),
        action_at(
            &[],
            None,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::BloodToActing(3),
            ],
        ),
    ],
    ..CARD
};

/// "+1 stealth action. Requires a prince or justicar. Add 4 blood to
/// another vampire." (1 blood.) Any Methuselah's, in torpor too (rulebook:
/// torpor vampires are eligible targets); at another Methuselah's, directed
/// at them (Ⓓ is only indicative, rules-qa 20.1).
pub static FIFTH_TRADITION: CardDef = CardDef {
    name: "Fifth Tradition: Hospitality",
    modes: &[action_at(
        &[PRINCE_OR_JUSTICAR],
        target(
            TargetKind::Minion,
            &[Requirement::TargetVampire, Requirement::OtherThanActor],
        ),
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::BloodToTarget(4),
        ],
    )],
    ..CARD
};

/// "+1 stealth action. Requires a prince or justicar. Add 3 blood to a
/// younger vampire in your uncontrolled region." (1 blood.)
pub static FOURTH_TRADITION: CardDef = CardDef {
    name: "Fourth Tradition: The Accounting",
    modes: &[action_at(
        &[PRINCE_OR_JUSTICAR],
        target(TargetKind::Uncontrolled, &[Requirement::Younger]),
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::BloodToUncontrolledTarget(3),
        ],
    )],
    ..CARD
};

/// "+1 stealth action. [obl] Ⓓ Enter combat with a minion. [OBL] As above,
/// and this acting vampire gets 1 optional maneuver during that combat."
pub static UMBROUS_CLUTCH: CardDef = CardDef {
    name: "Umbrous Clutch",
    modes: &[
        action_at(
            &[],
            COMBAT_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::EnterCombatWithTarget,
            ],
        ),
        action_at(
            &[],
            COMBAT_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::EnterCombatWithTarget,
                Effect::InResultingCombat(BlockRider::Maneuver),
            ],
        ),
    ],
    ..CARD
};

/// "Enter combat with a vampire": another Methuselah's ready vampire.
const VAMPIRE_COMBAT_TARGET: Option<TargetSpec> = target(
    TargetKind::Minion,
    &[
        Requirement::NotControlled,
        Requirement::ActorReady,
        Requirement::TargetVampire,
    ],
);

/// "+1 stealth action. [aus] Ⓓ Enter combat with a vampire. [AUS] As
/// above, and at the end of that combat, if this vampire is ready and the
/// opposing vampire is not, add 2 blood to a Salubri in your uncontrolled
/// region." (Salubri.) The Salubri is chosen then.
pub static HUNTING_THE_BEAST: CardDef = CardDef {
    name: "Hunting the Beast",
    modes: &[
        action_at(
            &[],
            VAMPIRE_COMBAT_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::EnterCombatWithTarget,
            ],
        ),
        action_at(
            &[],
            VAMPIRE_COMBAT_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::EnterCombatWithTarget,
                Effect::InResultingCombat(BlockRider::UncontrolledBloodIfWon {
                    clan: "Salubri",
                    n: 2,
                }),
            ],
        ),
    ],
    ..CARD
};

/// "Enter combat with a locked minion".
const LOCKED_COMBAT_TARGET: Option<TargetSpec> = target(
    TargetKind::Minion,
    &[
        Requirement::NotControlled,
        Requirement::ActorReady,
        Requirement::TargetLocked,
    ],
);

/// Make the Misere: one mode per set of Disciplines used.
macro_rules! misere {
    ($($e:expr),*) => {
        anarch_bleed(
            LOCKED_COMBAT_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::EnterCombatWithTarget,
                $($e),*
            ],
        )
    };
}

const MISERE_CEL: Effect = Effect::InResultingCombat(BlockRider::Maneuver);
const MISERE_POT: Effect = Effect::InResultingCombat(BlockRider::Strength(1));

/// "Requires an Anarch. More than one Discipline can be used to play this
/// card. Ⓓ Enter combat with a locked minion. [cel] This Anarch gets 1
/// optional maneuver during that combat. [obf] This is a +1 stealth
/// action. [pot] This Anarch gets +1 strength during that combat." Modes:
/// [cel], [obf], [pot], then each larger set (LSJ 20011204-3, PIB
/// 20130704). A target unlocked by resolution: the action fizzles (ANK
/// 20210124).
pub static MAKE_THE_MISERE: CardDef = CardDef {
    name: "Make the Misere",
    modes: &[
        misere!(MISERE_CEL),
        misere!(Effect::AddStealth(1)),
        misere!(MISERE_POT),
        misere!(Effect::AddStealth(1), MISERE_CEL),
        misere!(MISERE_CEL, MISERE_POT),
        misere!(Effect::AddStealth(1), MISERE_POT),
        misere!(Effect::AddStealth(1), MISERE_CEL, MISERE_POT),
    ],
    ..CARD
};

/// "Ⓓ Stun an unlocked vampire".
const STUN_TARGET: Option<TargetSpec> = target(
    TargetKind::Minion,
    &[
        Requirement::NotControlled,
        Requirement::TargetVampire,
        Requirement::TargetUnlocked,
    ],
);

/// "[pre] Ⓓ Stun an unlocked vampire. [PRE] As above, and this is a +1
/// stealth action." (1 blood.) A target locked by resolution: the action
/// fizzles (ANK 20210124).
pub static MIND_NUMB: CardDef = CardDef {
    name: "Mind Numb",
    modes: &[
        action_at(
            &[],
            STUN_TARGET,
            &[Effect::DeclareAction(ActionKind::Card), Effect::Stun],
        ),
        action_at(
            &[],
            STUN_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::Stun,
            ],
        ),
    ],
    ..CARD
};

/// "[pre] Ⓓ Bleed with +1 bleed. Titled vampires cannot block this action.
/// [PRE] As above, and the target Methuselah locks a ready unlocked minion
/// they control." (2 blood.) The minion is named as the card is played
/// (rules-qa 31.2).
pub static PROPAGANDA: CardDef = CardDef {
    name: "Propaganda",
    modes: &[
        action_at(
            &[],
            None,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                PLUS_ONE_BLEED,
                Effect::TitledCannotBlock,
            ],
        ),
        action_at(
            &[],
            target(
                TargetKind::Minion,
                &[
                    Requirement::ControlledByPrey,
                    Requirement::ActorReady,
                    Requirement::TargetUnlocked,
                ],
            ),
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                PLUS_ONE_BLEED,
                Effect::TitledCannotBlock,
                Effect::TargetLocksReadyUnlocked,
            ],
        ),
    ],
    ..CARD
};

/// "[obf] Ⓓ Burn an equipment. [OBF] Ⓓ Burn a location." Directed at the
/// card's controller, not the minion it is on (LSJ 20090324); one's own is
/// undirected. A location in play is no equipment (Living Manse).
pub static CONCEAL: CardDef = CardDef {
    name: "Conceal",
    modes: &[
        action_at(
            &[],
            target(
                TargetKind::Card,
                &[Requirement::TargetCard(CardFilter::Type(
                    CardType::Equipment,
                ))],
            ),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::BurnTargetCard,
            ],
        ),
        action_at(
            &[],
            target(
                TargetKind::Card,
                &[Requirement::TargetCard(CardFilter::Location)],
            ),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::BurnTargetCard,
            ],
        ),
    ],
    ..CARD
};

/// "[dom] +1 stealth action. Ⓓ Bleed with +1 bleed. [DOM] Ⓓ Steal a
/// location." (2 blood.) Another Methuselah's location: its control changes
/// and nothing else (RTR 19980623); one on a minion moves onto a minion of
/// the player's (RTR 19960112).
pub static DOMINATE_KINE: CardDef = CardDef {
    name: "Dominate Kine",
    modes: &[
        action_at(
            &[],
            None,
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::AddStealth(1),
                PLUS_ONE_BLEED,
            ],
        ),
        action_at(
            &[],
            target(
                TargetKind::Card,
                &[
                    Requirement::TargetCard(CardFilter::Location),
                    Requirement::NotControlled,
                ],
            ),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::StealLocation,
            ],
        ),
    ],
    ..CARD
};

/// "Ⓓ Steal a vampire in torpor controlled by another Methuselah".
const GRAVEROBBING_TARGET: Option<TargetSpec> = target(
    TargetKind::Minion,
    &[
        Requirement::NotControlled,
        Requirement::TargetVampire,
        Requirement::TargetInTorpor,
    ],
);

/// "[dom] Ⓓ Steal a vampire in torpor controlled by another Methuselah.
/// [DOM] As above, and this acting vampire can burn 2 blood to move the
/// stolen vampire to your ready region." Out of torpor by resolution: the
/// action fizzles (ANK 20210124); the 2 blood is a burn, not a cost (ANK
/// 20210226).
pub static GRAVEROBBING: CardDef = CardDef {
    name: "Graverobbing",
    modes: &[
        action_at(
            &[],
            GRAVEROBBING_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::StealTargetVampire,
            ],
        ),
        action_at(
            &[],
            GRAVEROBBING_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::StealTargetVampire,
                Effect::StolenToReadyForBlood(2),
            ],
        ),
    ],
    ..CARD
};

// ---- Pool actions, wave 2 (slice 11v) ---------------------------------------

/// "+1 stealth action. [ani] Put this card in play. During your unlock
/// phase, your prey burns 1 pool. You can burn only 1 pool each turn with
/// Army of Rats cards. Minions can burn this card as a Ⓓ action." The burn
/// action is directed at the controller (undirected for their own
/// minions).
pub static ARMY_OF_RATS: CardDef = CardDef {
    name: "Army of Rats",
    modes: &[action_at(
        &[],
        None,
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
        ],
    )],
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::ActingCandidate,
        &[],
        &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::BurnSource,
        ],
    )],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Controller,
        requires: &[Requirement::OneCopyPerTurn],
        effects: &[Effect::PreyBurnsPool(1)],
    }],
    ..CARD
};

/// "+1 stealth action. Requires a prince or justicar. Put this card in
/// play. Each Methuselah controlling a non-Camarilla vampire burns 1 pool
/// during their unlock phase. Methuselahs can burn a non-Camarilla vampire
/// they control during their master phase to burn this card." (1 blood.)
/// Copies stack (ANK 20200703-4); the condition is read as the burn
/// resolves, other unlock effects first if the player wants (ANK
/// 20181016).
pub static JUDGMENT_CAMARILLA_SEGREGATION: CardDef = CardDef {
    name: "Judgment: Camarilla Segregation",
    modes: &[action_at(
        &[PRINCE_OR_JUSTICAR],
        None,
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
        ],
    )],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::Controlled,
                Requirement::TargetSectNot(Sect::Camarilla),
            ],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[Effect::BurnTargetMinion, Effect::BurnSelf],
        )
    }],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Every,
        requires: &[Requirement::ActiveControlsNonSect(Sect::Camarilla)],
        effects: &[Effect::BurnPool(1)],
    }],
    ..CARD
};

/// "Minions can burn this card as a Ⓓ action": directed at its controller
/// (undirected for their own minions).
const BURN_ACTION: ModeDef = mode_of(
    &[Window::ActionDeclare],
    Actor::ActingCandidate,
    &[],
    &[
        Effect::DeclareAction(ActionKind::BurnCard),
        Effect::BurnSource,
    ],
);

/// "Ⓓ Steal 1 blood or life from a minion controlled by your prey".
const TIER_TARGET: Option<TargetSpec> =
    target(TargetKind::Minion, &[Requirement::ControlledByPrey]);

/// "[ani] Ⓓ Steal 1 blood or life from a minion controlled by your prey.
/// [ANI] As above, and put this card on this vampire. This vampire gets +1
/// bleed against your prey. Minions can burn this card as a Ⓓ action. A
/// vampire can have only one Tier of Souls." An ally left with no life is
/// burned. "Your prey": the card's controller's, read live.
pub static TIER_OF_SOULS: CardDef = CardDef {
    name: "Tier of Souls",
    modes: &[
        action_at(
            &[],
            TIER_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::StealBloodFromTarget(1),
            ],
        ),
        action_at(
            &[Requirement::ActorWithout(Holding::Named("Tier of Souls"))],
            TIER_TARGET,
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::StealBloodFromTarget(1),
                Effect::PutOnActor,
                Effect::Layer(Modifier::HostBleedAgainstPrey(1)),
            ],
        ),
    ],
    abilities: &[BURN_ACTION],
    ..CARD
};

/// "+1 stealth action. [pre] Ⓓ Put this card on a minion; you still control
/// this card. The attached minion gets -1 stealth. Minions can burn this
/// card as a Ⓓ action. [PRE] As above, and if the attached minion is
/// blocked, they burn 1 blood or life before block resolution." (1 blood;
/// Ravnos.) Any minion: at the player's own, undirected (rules-qa 20.1).
pub static PHANTASMAGORIA: CardDef = CardDef {
    name: "Phantasmagoria",
    modes: &[
        action_at(
            &[],
            target(TargetKind::Minion, &[]),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::StillYours,
                Effect::PutOnTarget,
                Effect::Layer(Modifier::Bearer {
                    stat: Stat::Stealth,
                    n: -1,
                    during: &[],
                    who: Bearer::Any,
                }),
            ],
        ),
        action_at(
            &[],
            target(TargetKind::Minion, &[]),
            &[
                Effect::DeclareAction(ActionKind::Card),
                Effect::AddStealth(1),
                Effect::StillYours,
                Effect::PutOnTarget,
                Effect::Layer(Modifier::Bearer {
                    stat: Stat::Stealth,
                    n: -1,
                    during: &[],
                    who: Bearer::Any,
                }),
            ],
        ),
    ],
    abilities: &[BURN_ACTION],
    riders: &[Rider {
        hook: Hook::HostBlocked,
        scope: RiderScope::Every,
        requires: &[Requirement::SuperiorVersion],
        effects: &[Effect::HostBurnBlood(1)],
    }],
    ..CARD
};

/// "+1 stealth action. Unique. Put this card in play. You can lock
/// Aranthebes to give a minion controlled by your predator -1 stealth.
/// While Aranthebes is unlocked, vampires with capacity 4 or less get -1
/// bleed against you. Vampires with capacity 5 or more can shuffle
/// Aranthebes into your library as a Ⓓ action." (1 pool.) The lock: any
/// time during the action before resolution, no block attempt needed (LSJ
/// 20020612) — on the acting minion. The action it provides requires
/// capacity 5 or more (LSJ 20080604).
pub static ARANTHEBES: CardDef = CardDef {
    name: "Aranthebes, The Immortal",
    modes: &[action_at(
        &[],
        None,
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
            Effect::Layer(Modifier::YoungBleedAgainstController { max: 4, n: -1 }),
        ],
    )],
    abilities: &[
        mode_of(
            &[Window::ReactionAsAnnounced, Window::ReactionDuringAction],
            Actor::Controller,
            &[Requirement::CardUnlocked, Requirement::ActingByPredator],
            &[Effect::LockSelf, Effect::ReduceStealth(1)],
        ),
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[
                Requirement::ActorVampire,
                Requirement::ActorCapacityAtLeast(5),
            ],
            &[
                Effect::DeclareAction(ActionKind::BurnCard),
                Effect::ShuffleSourceIntoLibrary,
            ],
        ),
    ],
    ..CARD
};

/// Fee Stake's referendum: "burn this card; during that referendum,
/// non-Anarch titles are worth -1 vote" (the Priscus sub-referendum's votes
/// untouched, ANK 20180307-1: no Priscus in V5).
static FEE_STAKE_BURN: ReferendumDef = ReferendumDef {
    terms: &[],
    effect: RefEffect::BurnSource,
    grants: &[],
    non_anarch_titles_minus: true,
};

/// "+1 stealth action. Requires an Anarch with capacity 5 or more. Title.
/// Put this card on this Anarch to represent the unique Anarch title of
/// Baron of <city>. While this Anarch is <clan> or <clan> antitribu, they
/// get +1 vote during referendums they call. Vampires can call a
/// referendum to burn this card as a +1 stealth political action; during
/// that referendum, non-Anarch titles are worth -1 vote." A contested
/// title: the card face down, its action unusable (LSJ 20070808-2).
macro_rules! fee_stake {
    ($name:ident, $printed:expr, $city:expr, $clans:expr) => {
        pub static $name: CardDef = CardDef {
            name: $printed,
            modes: &[action_at(
                &[
                    Requirement::ActorSectIn(&[Sect::Anarch]),
                    Requirement::ActorCapacityAtLeast(5),
                ],
                None,
                &[
                    Effect::DeclareAction(ActionKind::Card),
                    Effect::AddStealth(1),
                    Effect::PutOnActor,
                    Effect::RepresentsTitle(crate::crypt::Title::Baron($city)),
                    Effect::Layer(Modifier::HostVoteWhenCalling($clans)),
                ],
            )],
            abilities: &[mode_of(
                &[Window::ActionDeclarePolitical],
                Actor::ActingCandidate,
                &[Requirement::ActorVampire],
                &[
                    Effect::DeclareAction(ActionKind::Political),
                    Effect::AddStealth(1),
                    Effect::CallReferendum(&FEE_STAKE_BURN),
                ],
            )],
            ..CARD
        };
    };
}

fee_stake!(
    FEE_STAKE_BOSTON,
    "Fee Stake: Boston",
    "Boston",
    &["Toreador", "Toreador antitribu"]
);
fee_stake!(
    FEE_STAKE_NEW_YORK,
    "Fee Stake: New York",
    "New York",
    &["Brujah", "Brujah antitribu"]
);
fee_stake!(
    FEE_STAKE_SEATTLE,
    "Fee Stake: Seattle",
    "Seattle",
    &["Gangrel", "Gangrel antitribu"]
);

/// "+1 stealth action. Requires a non-sterile Follower of Set with capacity
/// 5 or more. Put this card in play. It becomes a 1-capacity (non-unique)
/// Follower of Set and must hunt this turn. You can search your library
/// (shuffle afterward), hand, and/or ash heap for a Discipline master card
/// and put it on this new vampire." (1 blood; Ministry, the Followers of
/// Set's V5 name.) No V5 vampire is sterile. No sect printed: its sire's,
/// the acting vampire's (rules-qa 32.6). It is no vampire in the
/// ash heap (LSJ 20020115); moved to the uncontrolled region, it stays a
/// vampire (RTR 19990712).
pub static WATERS_OF_DUAT: CardDef = CardDef {
    name: "Waters of Duat",
    modes: &[action_at(
        &[Requirement::ActorCapacityAtLeast(5)],
        None,
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
            Effect::BecomesVampire(None),
        ],
    )],
    ..CARD
};

/// "+1 stealth action. Put this card in play with 2 counters. During your
/// unlock phase, you can burn 1 counter from this card. You can burn
/// counters from no more than two Split the Veil cards each unlock phase.
/// If this card has no counters, burn it and move a wraith or zombie ally
/// from your ash heap to your ready region with life equal to its starting
/// life." (Hecata.) The ally: its inferior version, no requirement, X = 0,
/// able to act this turn (LSJ 20100204, 20040518-2, 20100302-1).
pub static SPLIT_THE_VEIL: CardDef = CardDef {
    name: "Split the Veil",
    modes: &[action_at(
        &[],
        None,
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
            Effect::EnterWithCounters(2),
        ],
    )],
    abilities: &[mode_of(
        &[Window::UnlockPhaseOwn],
        Actor::Controller,
        &[
            Requirement::CountersAtLeast(1),
            Requirement::FewerCopiesUsedThisPhase(2),
        ],
        &[
            Effect::RemoveCounter(1),
            Effect::RaiseAllyWhenEmpty(&[
                CardFilter::HasLabel(crate::library::Label::Wraith),
                CardFilter::HasLabel(crate::library::Label::Zombie),
            ]),
        ],
    )],
    ..CARD
};

/// "+1 stealth hunt action. Remove an ally in any Methuselah's ash heap from
/// the game to gain 3 blood or to gain 2 blood and unlock." (Hecata.) The
/// two gains are two modes; the ally is named as the card is played
/// (rules-qa 32.12). A vampire that must hunt may hunt with it (rules-qa
/// 32.13).
/// Undirected, whoever's ash heap (LSJ 20010627, ANK 20180202); not
/// playable with no ally in any ash heap (ANK 20200420-3); a minion's one
/// special hunt a turn (NRA).
pub static PSYCHOPHAGIA: CardDef = CardDef {
    name: "Psychophagia",
    modes: &[
        ModeDef {
            terms: Some(Terms::AnyAshAlly),
            ..action_at(
                &[Requirement::AllyInAnyAsh],
                None,
                &[
                    Effect::DeclareAction(ActionKind::Card),
                    Effect::AddStealth(1),
                    Effect::HuntAction,
                    Effect::EatAshAlly {
                        blood: 3,
                        unlock: false,
                    },
                ],
            )
        },
        ModeDef {
            terms: Some(Terms::AnyAshAlly),
            ..action_at(
                &[Requirement::AllyInAnyAsh],
                None,
                &[
                    Effect::DeclareAction(ActionKind::Card),
                    Effect::AddStealth(1),
                    Effect::HuntAction,
                    Effect::EatAshAlly {
                        blood: 2,
                        unlock: true,
                    },
                ],
            )
        },
    ],
    ..CARD
};

/// "+1 stealth action. Unique. Not usable if any non-mandatory actions have
/// been performed this turn. Put this card on this Assamite and unlock
/// them. This Assamite gets +1 bleed. If your prey is ousted, you gain 4
/// additional pool. Burn this card during your unlock phase." (Banu Haqim,
/// the Assamites' V5 name.)
pub static KHABAR_GLORY: CardDef = CardDef {
    name: "Khabar: Glory",
    modes: &[action_at(
        &[Requirement::NoOptionalActionThisTurn],
        None,
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutOnActor,
            Effect::UnlockActor,
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Bleed,
                n: 1,
                during: &[],
                who: Bearer::Any,
            }),
        ],
    )],
    riders: &[
        Rider {
            hook: Hook::PreyOusted,
            scope: RiderScope::Controller,
            requires: &[],
            effects: &[Effect::GainPool(4)],
        },
        Rider {
            hook: Hook::UnlockPhase,
            scope: RiderScope::Controller,
            requires: &[],
            effects: &[Effect::BurnSelf],
        },
    ],
    ..CARD
};

/// "+1 stealth action. Requires a baron. Only one Open War can be played in
/// a game. Put this card in play. Anarchs can enter combat with a minion as
/// a Ⓓ action. Anarchs can burn a location as a Ⓓ action that costs 2
/// pool. Methuselahs can use a master phase action to move 1 counter from
/// their pool to this card. If this card has 4 counters, burn it and gain 4
/// pool." Its two actions are not cross-restrictive: each once a turn per
/// minion (LSJ 20100909); each requires an Anarch (LSJ 20080604); the
/// location burn is directed at its controller (LSJ 20090324). Its
/// controller gains the 4 pool, whoever moves the fourth counter (rules-qa
/// 32.7).
pub static OPEN_WAR: CardDef = CardDef {
    name: "Open War",
    modes: &[action_at(
        &[BARON, Requirement::NotPlayedThisGame],
        None,
        &[
            Effect::DeclareAction(ActionKind::Card),
            Effect::AddStealth(1),
            Effect::PutInPlay,
        ],
    )],
    abilities: &[
        ModeDef {
            target: COMBAT_TARGET,
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::ActingCandidate,
                &[Requirement::ActorSectIn(&[Sect::Anarch])],
                &[
                    Effect::DeclareAction(ActionKind::Ability),
                    Effect::EnterCombatWithTarget,
                ],
            )
        },
        ModeDef {
            target: target(
                TargetKind::Card,
                &[Requirement::TargetCard(CardFilter::Location)],
            ),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::ActingCandidate,
                &[
                    Requirement::ActorSectIn(&[Sect::Anarch]),
                    Requirement::PoolAtLeast(2),
                ],
                &[
                    Effect::DeclareAction(ActionKind::Ability),
                    Effect::PayPool(2),
                    Effect::BurnTargetCard,
                ],
            )
        },
        mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[Requirement::MasterPhaseActionLeft],
            &[
                Effect::SpendMasterPhaseAction,
                Effect::PoolToCounter(1),
                Effect::AtCountersBurnAndGain { at: 4, pool: 4 },
            ],
        ),
    ],
    ..CARD
};

// ---- Pool masters, wave 1 (slice 11w) ---------------------------------------

/// A master card put in play, then "minions can burn this card as a Ⓓ
/// action" for those `requires` names.
const fn burn_this_by(requires: &'static [Requirement]) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::ActingCandidate,
        requires,
        target: None,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::BurnSource,
        ],
    }
}

/// "Unique location. During your master phase, you can lock this card to
/// get +1 master phase action." (2 pool.)
pub static THE_PARTHENON: CardDef = CardDef {
    name: "Parthenon, The",
    modes: &[LOCATION],
    abilities: &[mode_of(
        &[Window::MasterPhase],
        Actor::Controller,
        &[Requirement::CardUnlocked],
        &[Effect::LockSelf, Effect::GainMasterPhaseAction],
    )],
    ..CARD
};

/// "Unique location. You can burn this card if a minion is bleeding you to
/// reduce the bleed amount by 2."
pub static DUMMY_CORPORATION: CardDef = CardDef {
    name: "Dummy Corporation",
    modes: &[LOCATION],
    abilities: &[mode_of(
        &[Window::ReactionDuringAction],
        Actor::Controller,
        &[Requirement::DirectedAtYou, Requirement::IsBleed],
        &[Effect::BurnSelf, Effect::ReduceBleed(2)],
    )],
    ..CARD
};

/// "Unique. Put this card in play. Once each turn, you can burn 1 pool to
/// get +3 votes during the polling step of any referendum." (Ministry.) Not
/// if the cost ousts the player (LSJ 20081105); "each turn": each player's
/// turn (ANK 20180719-4).
pub static FERRAILLE: CardDef = CardDef {
    name: "Ferraille",
    modes: &[master_in_play(&[], &[Effect::PutInPlay])],
    abilities: &[
        ModeDef {
            limit: Some(Limit::PerTurn),
            ..mode_of(
                &[Window::Referendum],
                Actor::Controller,
                &[Requirement::AnyPollingStep],
                &[
                    Effect::PayPool(1),
                    Effect::MethuselahVotes { n: 3, favor: true },
                ],
            )
        },
        ModeDef {
            limit: Some(Limit::PerTurn),
            ..mode_of(
                &[Window::Referendum],
                Actor::Controller,
                &[Requirement::AnyPollingStep],
                &[
                    Effect::PayPool(1),
                    Effect::MethuselahVotes { n: 3, favor: false },
                ],
            )
        },
    ],
    ..CARD
};

/// "Unique location. You can lock this card and burn X pool during the
/// polling step of a political action to get +2X votes." (Brujah.)
pub static OXFORD_UNIVERSITY: CardDef = CardDef {
    name: "Oxford University, England",
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            terms: Some(Terms::XPool),
            ..mode_of(
                &[Window::Referendum],
                Actor::Controller,
                &[
                    Requirement::PollingStep,
                    Requirement::CardUnlocked,
                    Requirement::YouControlThisCard,
                ],
                &[
                    Effect::LockSelf,
                    Effect::PayPoolX,
                    Effect::MethuselahVotesX {
                        per: 2,
                        favor: true,
                    },
                ],
            )
        },
        ModeDef {
            terms: Some(Terms::XPool),
            ..mode_of(
                &[Window::Referendum],
                Actor::Controller,
                &[
                    Requirement::PollingStep,
                    Requirement::CardUnlocked,
                    Requirement::YouControlThisCard,
                ],
                &[
                    Effect::LockSelf,
                    Effect::PayPoolX,
                    Effect::MethuselahVotesX {
                        per: 2,
                        favor: false,
                    },
                ],
            )
        },
    ],
    ..CARD
};

/// "Unique location. You can lock this card during the polling step of any
/// referendum to get +2 votes. You can lock this card after a minion with a
/// contract for which an Assamite you control is chosen leaves the ready
/// region to gain 1 pool." (1 pool; Banu Haqim.) The contract's host
/// leaving is enough: burned with it, or its controller ousted (PIB
/// 20150512, ANK 20180129-2).
pub static THE_BLACK_THRONE: CardDef = CardDef {
    name: "Black Throne, The",
    modes: &[LOCATION],
    abilities: &[
        mode_of(
            &[Window::Referendum],
            Actor::Controller,
            &[Requirement::AnyPollingStep, Requirement::CardUnlocked],
            &[
                Effect::LockSelf,
                Effect::MethuselahVotes { n: 2, favor: true },
            ],
        ),
        mode_of(
            &[Window::Referendum],
            Actor::Controller,
            &[Requirement::AnyPollingStep, Requirement::CardUnlocked],
            &[
                Effect::LockSelf,
                Effect::MethuselahVotes { n: 2, favor: false },
            ],
        ),
    ],
    riders: &[Rider {
        hook: Hook::ContractHostLeaves,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::GainPool(1)],
    }],
    ..CARD
};

/// "Unique. Put this card in play. You can lock this card to give a Ravnos
/// with capacity 5 or more you control +1 intercept." (1 pool; Ravnos.)
pub static KUMPANIA: CardDef = CardDef {
    name: "Kumpania",
    modes: &[master_in_play(&[], &[Effect::PutInPlay])],
    abilities: &[mode_of(
        &[Window::ActionStealthIntercept],
        Actor::Controller,
        &[
            Requirement::CardUnlocked,
            Requirement::YourAttemptNeedsIntercept,
            Requirement::AttemptingClan("Ravnos"),
            Requirement::AttemptingCapacityAtLeast(5),
        ],
        &[Effect::LockSelf, Effect::AttemptingIntercept(1)],
    )],
    ..CARD
};

/// "Unique. Put this card on a Salubri you control. This Salubri gets +1
/// intercept." (Salubri.)
pub static SIGHT_BEYOND_SIGHT: CardDef = CardDef {
    name: "Sight Beyond Sight",
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::Controlled, Requirement::TargetClan("Salubri")],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[
                Effect::PutOnTarget,
                Effect::Layer(Modifier::Bearer {
                    stat: Stat::Intercept,
                    n: 1,
                    during: &[],
                    who: Bearer::Any,
                }),
            ],
        )
    }],
    ..CARD
};

/// "Unique. Put this card on a ready vampire. Recruit ally actions cost this
/// vampire -1 blood or pool." Any Methuselah's.
pub static CHARISMA: CardDef = CardDef {
    name: "Charisma",
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::TargetVampire, Requirement::ActorReady],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[
                Effect::PutOnTarget,
                Effect::Layer(Modifier::HostRecruitDiscount(1)),
            ],
        )
    }],
    ..CARD
};

/// "Only one Giant's Blood can be played in a game. Choose a vampire. The
/// chosen vampire gains enough blood to reach full capacity." Not an
/// uncontrolled one (LSJ 20061109).
pub static GIANTS_BLOOD: CardDef = CardDef {
    name: "Giant's Blood",
    modes: &[ModeDef {
        target: target(TargetKind::Minion, &[Requirement::TargetVampire]),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[Requirement::NotPlayedThisGame],
            &[Effect::BloodToTargetFull],
        )
    }],
    ..CARD
};

/// "Remove a vampire with capacity 8 or more from the game. Their
/// controller gains pool equal to the vampire's capacity. Their controller
/// can burn 2 pool to cancel this card as it is played."
pub static GOLCONDA_INNER_PEACE: CardDef = CardDef {
    name: "Golconda: Inner Peace",
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::TargetVampire,
                Requirement::TargetCapacityAtLeast(8),
            ],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[
                Effect::CancelableForPoolByTargetController(2),
                Effect::RemoveTargetForCapacityPool,
            ],
        )
    }],
    ..CARD
};

/// "Move up to 3 blood from a ready vampire you control to a younger
/// vampire of the same clan in your uncontrolled region." The vampire
/// giving is the card's actor (not locked: no action).
pub static GROOMING_THE_PROTEGE: CardDef = CardDef {
    name: "Grooming the Protégé",
    modes: &[ModeDef {
        target: target(
            TargetKind::Uncontrolled,
            &[Requirement::Younger, Requirement::SameClanAsActor],
        ),
        terms: Some(Terms::BloodRange { min: 1, max: 3 }),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::AnyReadyVampire,
            &[Requirement::ActorBloodAtLeast(1)],
            &[Effect::MoveBloodXToUncontrolledTarget],
        )
    }],
    ..CARD
};

/// "Unique. Put this card in play. Gangrel you control get +1 strength.
/// Non-Ravnos minions can burn this card as a Ⓓ action." (1 pool; Gangrel.)
pub static GANGREL_REVEL: CardDef = CardDef {
    name: "Gangrel Revel",
    modes: &[master_in_play(
        &[],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::ClanStat {
                clan: "Gangrel",
                stat: Stat::Strength,
                n: 1,
                own: true,
                titled: false,
                during: &[],
            }),
        ],
    )],
    abilities: &[burn_this_by(&[Requirement::ActorNotClan("Ravnos")])],
    ..CARD
};

/// "Unique. Put this card in play. Assamites get +1 stealth when bleeding.
/// Minions can burn this card as a Ⓓ action; Tremere get +1 stealth during
/// that action." (2 pool; Banu Haqim.) Every Assamite, whoever controls it.
pub static KHABAR_COMMUNITY: CardDef = CardDef {
    name: "Khabar: Community, The",
    modes: &[master_in_play(
        &[],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::ClanStat {
                clan: "Banu Haqim",
                stat: Stat::Stealth,
                n: 1,
                own: false,
                titled: false,
                during: &[ActionKind::Bleed],
            }),
        ],
    )],
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::ActingCandidate,
        &[],
        &[
            Effect::DeclareAction(ActionKind::BurnCard),
            Effect::StealthForClan {
                clan: "Tremere",
                n: 1,
            },
            Effect::BurnSource,
        ],
    )],
    ..CARD
};

/// "Unique location. Titled Brujah get +1 bleed and +1 vote. Ventrue get -1
/// vote." (3 pool; Brujah.) Every such vampire; net negative votes count
/// as none (ANK 20251125, LSJ 20050221-2).
pub static NEW_CARTHAGE: CardDef = CardDef {
    name: "New Carthage",
    modes: &[master_in_play(
        &[],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::ClanStat {
                clan: "Brujah",
                stat: Stat::Bleed,
                n: 1,
                own: false,
                titled: true,
                during: &[],
            }),
            Effect::Layer(Modifier::ClanStat {
                clan: "Brujah",
                stat: Stat::Votes,
                n: 1,
                own: false,
                titled: true,
                during: &[],
            }),
            Effect::Layer(Modifier::ClanStat {
                clan: "Ventrue",
                stat: Stat::Votes,
                n: -1,
                own: false,
                titled: false,
                during: &[],
            }),
        ],
    )],
    ..CARD
};

/// "Unique. Put this card in play. You can lock this card to give a minion
/// you control 1 press, only usable to continue combat. Minions can burn
/// this card as a Ⓓ action."
pub static MOB_CONNECTIONS: CardDef = CardDef {
    name: "Mob Connections",
    modes: &[master_in_play(&[], &[Effect::PutInPlay])],
    abilities: &[
        mode_of(
            &[Window::CombatPress],
            Actor::Controller,
            &[Requirement::CardUnlocked, Requirement::OwnMinionInCombat],
            &[Effect::LockSelf, Effect::ContinuePressForOwnCombatant],
        ),
        burn_this_by(&[]),
    ],
    ..CARD
};

/// "Unique Nod fragment. Put this card in play. You can lock this card to
/// draw 2 cards (discard down afterward). Vampires can steal this card as a
/// Ⓓ action." Whenever its controller holds the impulse, during an action
/// too (ANK 20200616).
pub static FRAGMENT_OF_THE_BOOK_OF_NOD: CardDef = CardDef {
    name: "Fragment of the Book of Nod",
    modes: &[master_in_play(&[], &[Effect::PutInPlay])],
    abilities: &[
        mode_of(
            &[Window::AnyImpulse],
            Actor::Controller,
            &[Requirement::CardUnlocked],
            &[Effect::LockSelf, Effect::DrawThenDiscardDown(2)],
        ),
        steal_this(&[Requirement::ActorVampire]),
    ],
    ..CARD
};

/// "Put this card in play. During each Methuselah's master phase, that
/// Methuselah locks one of the oldest Brujah they control (if any). Brujah
/// get +1 strength and 1 optional maneuver each combat. Non-Ventrue minions
/// can burn this card as a Ⓓ action." (1 pool; Brujah.) Their pick among
/// the oldest (LSJ 19980224), a locked one too (LSJ 20010508).
pub static BRUJAH_DEBATE: CardDef = CardDef {
    name: "Brujah Debate",
    modes: &[master_in_play(
        &[],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::ClanStat {
                clan: "Brujah",
                stat: Stat::Strength,
                n: 1,
                own: false,
                titled: false,
                during: &[],
            }),
            Effect::Layer(Modifier::ClanCombatManeuver("Brujah")),
        ],
    )],
    abilities: &[burn_this_by(&[Requirement::ActorNotClan("Ventrue")])],
    riders: &[Rider {
        hook: Hook::MasterPhaseStart,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::LockOldestOfClan("Brujah")],
    }],
    ..CARD
};

// ---- Pool masters, wave 2 (slice 11x) ---------------------------------------

/// "Unique location. You can lock this card to give a minion +1 intercept.
/// If that minion does not block the action, burn 1 pool after action
/// resolution." (1 pool.) Any Methuselah's minion attempting to block;
/// blocks before the lock do not count (ANK 20230319); sequencing as
/// usual (ANK 20200607).
pub static WMRH_TALK_RADIO: CardDef = CardDef {
    name: "WMRH Talk Radio",
    modes: &[LOCATION],
    abilities: &[mode_of(
        &[Window::ActionStealthIntercept],
        Actor::Controller,
        &[
            Requirement::CardUnlocked,
            Requirement::AnyAttemptNeedsIntercept,
        ],
        &[
            Effect::LockSelf,
            Effect::AttemptingIntercept(1),
            Effect::PoolIfAttemptingDoesNotBlock,
        ],
    )],
    ..CARD
};

/// "Unique location. You can lock this card during an action to choose a
/// vampire; the chosen vampire can burn 1 blood to get +1 intercept during
/// that action." (1 pool.) Any vampire, once; sequencing as usual (ANK
/// 20200607).
pub static THE_RUMOR_MILL: CardDef = CardDef {
    name: "Rumor Mill, Tabloid Newspaper, The",
    modes: &[LOCATION],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::TargetVampire, Requirement::ActorReady],
        ),
        ..mode_of(
            &[Window::ActionDuring, Window::ReactionDuringAction],
            Actor::Controller,
            &[Requirement::CardUnlocked],
            &[Effect::LockSelf, Effect::TargetBurnsBloodForIntercept],
        )
    }],
    ..CARD
};

/// "Archetype. Put this card on a ready vampire you control. Once each turn
/// after a combat involving this vampire ends, this vampire can burn 1
/// blood to unlock if the opposing minion is not ready." Used after the
/// combat, ordered among the after-combat effects (ANK 20230314); "each
/// turn": each player's (ANK 20180719-4).
pub static MONSTER: CardDef = CardDef {
    name: "Monster",
    modes: &[archetype(
        &[
            Requirement::Controlled,
            Requirement::TargetVampire,
            Requirement::ActorReady,
            NO_ARCHETYPE,
        ],
        &[Effect::PutOnTarget],
    )],
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::ActionAfterCombat],
            Actor::Host,
            &[
                Requirement::HostCombatOpponentNotReady,
                Requirement::HostBloodAtLeast(1),
            ],
            &[Effect::HostBurnBloodToUnlock],
        )
    }],
    ..CARD
};

/// "Archetype. Trifle. Put this card on a vampire you control. Once each
/// turn, after an action (successful or not) is resolved during which this
/// vampire used 3 or more Disciplines to play cards, this vampire can
/// either burn 1 blood to unlock or gain 1 blood." Canceled cards count
/// (ANK 20220411); a canceled action has no resolution (ANK 20220411).
pub static DABBLER: CardDef = CardDef {
    name: "Dabbler",
    trifle: true,
    modes: &[archetype(
        &[
            Requirement::Controlled,
            Requirement::TargetVampire,
            NO_ARCHETYPE,
        ],
        &[Effect::PutOnTarget],
    )],
    abilities: &[
        ModeDef {
            limit: Some(Limit::PerTurn),
            ..mode_of(
                &[Window::ActionAfterResolution],
                Actor::Host,
                &[
                    Requirement::HostUsedDisciplines(3),
                    Requirement::HostBloodAtLeast(1),
                ],
                &[Effect::HostBurnBloodToUnlock],
            )
        },
        ModeDef {
            limit: Some(Limit::PerTurn),
            ..mode_of(
                &[Window::ActionAfterResolution],
                Actor::Host,
                &[Requirement::HostUsedDisciplines(3)],
                &[Effect::AddHostBlood(1)],
            )
        },
    ],
    ..CARD
};

/// "Unique. Put this card in play. After a vampire in combat with a
/// Lasombra you control leaves the ready region, you can add 1 counter to
/// this card. During a bleed action or the polling step of any referendum,
/// a Lasombra you control can burn 1 counter from this card to get +2 bleed
/// or +2 votes." (Lasombra.) The bleed only during the bleed action before
/// resolution (ANK 20231124, LSJ 19970718).
pub static DEAD_POOL: CardDef = CardDef {
    name: "Dead Pool",
    modes: &[master_in_play(&[], &[Effect::PutInPlay])],
    abilities: &[
        mode_of(
            &[Window::ActionDuring],
            Actor::AnyReadyVampire,
            &[
                Requirement::ActorActing,
                Requirement::IsBleed,
                Requirement::ActorClan("Lasombra"),
                Requirement::CountersAtLeast(1),
            ],
            &[Effect::RemoveCounter(1), bleed(2, false)],
        ),
        mode_of(
            &[Window::Referendum],
            Actor::AnyReadyVampire,
            &[
                Requirement::AnyPollingStep,
                Requirement::ActorClan("Lasombra"),
                Requirement::NotAbstaining,
                Requirement::CountersAtLeast(1),
            ],
            &[Effect::RemoveCounter(1), Effect::AddVotes(2)],
        ),
    ],
    riders: &[Rider {
        hook: Hook::CombatOpponentOfOwnClanLeft("Lasombra"),
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::AddCounter(1)],
    }],
    ..CARD
};

/// "Unique location. After an Assamite you control successfully bleeds a
/// Methuselah, put the pool lost by that Methuselah on this card. During
/// the polling step of a political action, you can burn X counters from
/// this card to give a vampire you control +X votes." (1 pool; Banu Haqim.)
pub static ALAMUT: CardDef = CardDef {
    name: "Alamut",
    modes: &[LOCATION],
    abilities: &[ModeDef {
        terms: Some(Terms::XCounters),
        ..mode_of(
            &[Window::Referendum],
            Actor::AnyReadyVampire,
            &[
                Requirement::PollingStep,
                Requirement::NotAbstaining,
                Requirement::CountersAtLeast(1),
            ],
            &[Effect::RemoveCountersX, Effect::AddVotesX],
        )
    }],
    riders: &[Rider {
        hook: Hook::OwnBleedSuccessful,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::CountersFromOwnBleed("Banu Haqim")],
    }],
    ..CARD
};

/// "Unique location. You can lock this location after resolution of a
/// successful action requiring Hecata or Oblivion [obl] to add 1 blood to a
/// Hecata you control." (Hecata.) Anyone's action; one granted by a card
/// requiring Hecata counts (LSJ 20080604).
pub static CAPPADOCIAN_CRYPT: CardDef = CardDef {
    name: "Cappadocian Crypt",
    modes: &[LOCATION],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::Controlled, Requirement::TargetClan("Hecata")],
        ),
        ..mode_of(
            &[Window::ActionAfterResolution],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::ActionSuccessful,
                Requirement::ActionRequiresClanOrDiscipline("Hecata", "obl"),
            ],
            &[Effect::LockSelf, Effect::BloodToTarget(1)],
        )
    }],
    ..CARD
};

/// "Unique location. During your master phase, you can lock this location to
/// move 1 blood from a vampire with Oblivion [obl] you control to your pool
/// or from your pool to a ready vampire with Oblivion [obl] you control.
/// Independent vampires can burn this location as a Ⓓ action that costs 1
/// blood." (Hecata.) The burn requires an Independent vampire (LSJ
/// 20080604).
pub static POWERBASE_MUNICH: CardDef = CardDef {
    name: "Powerbase: Munich",
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::TargetVampire,
                    Requirement::TargetDiscipline("obl"),
                    Requirement::BloodAtLeast(1),
                ],
            ),
            ..mode_of(
                &[Window::MasterPhase],
                Actor::Controller,
                &[Requirement::CardUnlocked],
                &[Effect::LockSelf, Effect::TargetBloodToPool(1)],
            )
        },
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::TargetVampire,
                    Requirement::ActorReady,
                    Requirement::TargetDiscipline("obl"),
                ],
            ),
            ..mode_of(
                &[Window::MasterPhase],
                Actor::Controller,
                &[Requirement::CardUnlocked],
                &[Effect::LockSelf, Effect::PoolToTargetBlood(1)],
            )
        },
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[
                Requirement::ActorSectIn(&[Sect::Independent]),
                Requirement::ActorBloodAtLeast(1),
            ],
            &[
                Effect::DeclareAction(ActionKind::BurnCard),
                Effect::PayBlood(1),
                Effect::BurnSource,
            ],
        ),
    ],
    ..CARD
};

/// "Put this card in play with 4 blood. After you put a non-titled
/// non-Anarch vampire in play from your uncontrolled region, you can burn 1
/// blood from this card to make that vampire Anarch. You can use a master
/// phase action to move 1 blood from this card to an Anarch. If this card
/// has no blood, burn it." Its blood as counters.
pub static TWILIGHT_CAMP: CardDef = CardDef {
    name: "Twilight Camp",
    modes: &[master_in_play(
        &[],
        &[Effect::PutInPlay, Effect::EnterWithCounters(4)],
    )],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::TargetVampire, Requirement::TargetAnarch],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Controller,
            &[
                Requirement::MasterPhaseActionLeft,
                Requirement::CountersAtLeast(1),
            ],
            &[
                Effect::SpendMasterPhaseAction,
                Effect::CounterToTargetBlood,
                Effect::BurnSelfWithoutCounters,
            ],
        )
    }],
    riders: &[Rider {
        hook: Hook::OwnVampireInfluenced,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[],
    }],
    ..CARD
};

/// "Unique location. Mithraic Mysteries. If you do not control /Mithras/
/// during your influence phase, you can search your crypt for 1 copy of
/// him, reveal it, and move it to your uncontrolled region (shuffle
/// afterward). During your unlock phase, you can add 1 blood to /Mithras/
/// or a Mithraic Mysteries member (the target must be controlled)." (1
/// pool.)
pub static THE_MITHRAEUM: CardDef = CardDef {
    name: "Mithraeum, London, The",
    modes: &[LOCATION],
    abilities: &[
        mode_of(
            &[Window::InfluencePhaseOwn],
            Actor::Controller,
            &[Requirement::NoVampireNamedButInCrypt("Mithras")],
            &[Effect::CryptNamedToUncontrolled("Mithras")],
        ),
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[Requirement::Controlled, Requirement::TargetMithraic],
            ),
            ..mode_of(
                &[Window::UnlockPhaseOwn],
                Actor::Controller,
                &[],
                &[Effect::BloodToTarget(1)],
            )
        },
    ],
    ..CARD
};

/// "Trifle. Put this card on a vampire you control who is not /Mithras/.
/// This vampire is a Mithraic Mysteries member. This vampire gets +1 vote
/// while you control /Mithras/. This vampire gets +1 intercept during
/// actions directed at /Mithras/ or a Mithraic Mysteries card or member. A
/// vampire can have only one Mithraic Cultist."
pub static MITHRAIC_CULTIST: CardDef = CardDef {
    name: "Mithraic Cultist",
    trifle: true,
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::Controlled,
                Requirement::TargetVampire,
                Requirement::TargetNotNamed("Mithras"),
                Requirement::TargetWithout(Holding::Named("Mithraic Cultist")),
            ],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[
                Effect::PutOnTarget,
                Effect::Layer(Modifier::MithraicMember),
                Effect::Layer(Modifier::HostVotesWhileControlling {
                    name: "Mithras",
                    n: 1,
                }),
                Effect::Layer(Modifier::HostInterceptVsMithraic(1)),
            ],
        )
    }],
    ..CARD
};

// ---- Pool masters, wave 3 (slice 11y) ---------------------------------------

/// "Unique location. Haven. Put this card on a minion you control. This
/// minion cannot be the target of other Methuselahs' actions. Master cards
/// targeting this minion cost 1 additional pool. Burn this card after this
/// minion goes to torpor. A minion can have only one haven." (1 pool.) The
/// extra pool from anyone, its controller too (TOM 19951208-1); cards on the
/// minion stay targetable (TOM 19951208-1, LSJ 19980302-1); effects of cards
/// already in play cost nothing more (ANK 20221120); playable on a vampire
/// in torpor (LSJ 20010610).
pub static SECURE_HAVEN: CardDef = CardDef {
    name: "Secure Haven",
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::Controlled,
                Requirement::TargetWithout(Holding::Named("Secure Haven")),
            ],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[Effect::PutOnTarget, Effect::Layer(Modifier::Haven)],
        )
    }],
    riders: &[Rider {
        hook: Hook::HostToTorpor,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::BurnSelf],
    }],
    ..CARD
};

/// "Unique location. If a ready Assamite you control is the target of a
/// directed action or is chosen by the acting Methuselah in the terms of a
/// referendum, you can lock this location to unlock the acting minion and
/// have the action or referendum fail. Only usable as the directed action is
/// announced or during the polling step of the referendum before votes and
/// ballots are cast." (1 pool; Banu Haqim.) After the acting Methuselah's
/// own as-announced effects (LSJ 20081105), before other modifiers and
/// reactions (ANK 20190425); the failed action reached resolution (ANK
/// 20211015, RTR 20180719) and was not blocked (LSJ 20070219).
pub static YORUBA_SHRINE: CardDef = CardDef {
    name: "Yoruba Shrine",
    modes: &[LOCATION],
    abilities: &[
        mode_of(
            &[Window::ReactionAsAnnounced],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::DirectedAtYourReadyClan("Banu Haqim"),
            ],
            &[Effect::LockSelf, Effect::UnlockActing, Effect::ActionFails],
        ),
        mode_of(
            &[Window::Referendum],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::PollingStep,
                Requirement::NoVotesCast,
                Requirement::TermsChoseYourReadyClan("Banu Haqim"),
            ],
            &[
                Effect::LockSelf,
                Effect::UnlockActing,
                Effect::FailReferendum,
            ],
        ),
    ],
    ..CARD
};

/// "Unique location. Followers of Set you control can put 1 corruption
/// counter on an ally or younger vampire controlled by your prey as a Ⓓ
/// action. If the action is successful and the number of your corruption
/// counters on the minion equals or exceeds their capacity or cost, you can
/// burn those counters to steal that minion." (3 pool.) The action requires
/// a Follower of Set (LSJ 20080604).
pub static CAVE_OF_APPLES: CardDef = CardDef {
    name: "Cave of Apples",
    modes: &[LOCATION],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[Requirement::ControlledByPrey, Requirement::AllyOrYounger],
        ),
        ..mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[Requirement::ActorClan("Ministry")],
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::CorruptTargetMayStealAtCapacity,
            ],
        )
    }],
    ..CARD
};

/// "Unique location. Operation Antigen. An Operation Antigen ally you
/// control can lock this location to attempt to enter combat with a
/// vampire as a +1 stealth Ⓓ action. You can lock this location to add
/// enough life to an Operation Antigen ally you control to reach their
/// starting life. You can lock this location and burn 2 pool to cancel an
/// action card that targets an Operation Antigen card you control as it is
/// played; its cost is not paid." (2 pool.) Locked as the action is
/// declared, even if it fails (ANK 20221102); the cancel follows the cancel
/// rules (as-played window, LSJ 20061207; still played, ANK 20190104;
/// replaced normally, LSJ 20080630), reaches employ, recruit and political
/// action cards (ANK 20200813-2), not a card played by Piper (RTR
/// 20001020).
pub static MOBILE_HQ: CardDef = CardDef {
    name: "Mobile HQ, Operation Antigen",
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::NotControlled,
                    Requirement::ActorReady,
                    Requirement::TargetVampire,
                ],
            ),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::ActingCandidate,
                &[
                    Requirement::YouControlThisCard,
                    Requirement::CardUnlocked,
                    Requirement::ActorAllyCard(OPERATION_ANTIGEN),
                ],
                &[
                    Effect::DeclareAction(ActionKind::Ability),
                    Effect::AddStealth(1),
                    Effect::LockSelf,
                    Effect::EnterCombatWithTarget,
                ],
            )
        },
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::Controlled,
                    Requirement::TargetAllyCard(OPERATION_ANTIGEN),
                    Requirement::TargetAllyHurt,
                ],
            ),
            ..mode_of(
                &[Window::AnyImpulse],
                Actor::Controller,
                &[
                    Requirement::CardUnlocked,
                    Requirement::NotInDamageResolution,
                ],
                &[Effect::LockSelf, Effect::BloodToTargetToStart(u8::MAX)],
            )
        },
        mode_of(
            &[Window::CardAsPlayed],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::PoolAtLeast(2),
                Requirement::PlayedActionCardAtYour(crate::library::Label::OperationAntigen),
            ],
            &[
                Effect::LockSelf,
                Effect::PayPool(2),
                Effect::CancelPlayed { refund: true },
            ],
        ),
    ],
    ..CARD
};

/// An Operation Antigen card.
const OPERATION_ANTIGEN: CardFilter = CardFilter::HasLabel(crate::library::Label::OperationAntigen);

/// "Unique location. +1 hand size. Minions you control can unlock after
/// resolution of a successful action to equip with a non-electronic,
/// non-vehicle, non-weapon equipment card from your hand; a minion can
/// unlock only once this way each turn. Non-electronic, non-vehicle,
/// non-weapon equipment cards cost minions you control -1 blood or pool to
/// equip." (2 pool.) The card's type read in hand (locations in play
/// included, ANK 20220824); the reduction always (ANK 20221026).
pub static BRITISH_MUSEUM: CardDef = CardDef {
    name: "British Museum, London, The",
    modes: &[master_in_play(
        &[],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::HandSize(1)),
            Effect::Layer(Modifier::PlainEquipDiscount(1)),
        ],
    )],
    abilities: &[mode_of(
        &[Window::ActionAfterResolution],
        Actor::Controller,
        &[
            Requirement::ActionSuccessful,
            Requirement::OtherActingControlled,
            Requirement::PlainEquipFromHandOncePerTurn,
        ],
        &[Effect::UnlockActing],
    )],
    ..CARD
};

// ---- Pool masters, wave 4: cost reductions (slice 11z) ----------------------

/// "Unique location. You can lock this location to reduce the cost of an
/// action card a vampire you control plays by 1 blood (this location is not
/// locked if that card is canceled as it is played). Vampires can steal this
/// location as a Ⓓ action." Any action card's blood, not its pool (ANK
/// 20181007); not for an ally playing as a vampire (ANK 20200605); from the
/// announcement to just before the cost is paid, announced with the card if
/// it is otherwise unaffordable (ANK 20230620, ANK 20170605, LSJ 20090705,
/// LSJ 20030917, ANK 20230621) — as the action resolves too (rules-qa
/// 36.1). A card played otherwise than as an action (Piper's ally) takes it
/// too (ANK 20230824, RTR 20070707; rules-qa 36.5).
pub static THE_LINE: CardDef = CardDef {
    name: "Line, The",
    modes: &[master_in_play(
        &[],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::CostSource(CostSource::Line)),
        ],
    )],
    abilities: &[
        mode_of(
            &[Window::ActionDuring, Window::ActionWouldBeSuccessful],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::OwnActionCardCosts(CostSource::Line),
            ],
            &[Effect::LockSelf, Effect::ReduceActionBlood(1)],
        ),
        steal_this(&[Requirement::ActorVampire]),
    ],
    ..CARD
};

/// "Unique location. If you have the Edge, you can lock this card to reduce
/// the cost of a card you play by 2 pool or blood. Vampires can steal this
/// location as a Ⓓ action; Ventrue get +1 stealth during that action." (2
/// pool.) Minion cards too (ANK 20221102); the same timing as The Line.
pub static THE_SHARD: CardDef = CardDef {
    name: "Shard, London, The",
    modes: &[master_in_play(
        &[],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::CostSource(CostSource::Shard)),
        ],
    )],
    abilities: &[
        mode_of(
            &[Window::ActionDuring, Window::ActionWouldBeSuccessful],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::HasEdge,
                Requirement::OwnActionCardCosts(CostSource::Shard),
            ],
            &[Effect::LockSelf, Effect::ReduceActionCost(2)],
        ),
        mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[Requirement::ActorVampire],
            &[
                Effect::DeclareAction(ActionKind::StealCard),
                Effect::StealthForClan {
                    clan: "Ventrue",
                    n: 1,
                },
            ],
        ),
    ],
    ..CARD
};

/// "Unique location. During your master phase, you can move 1 counter from
/// your pool to this location and add 1 counter (from the blood bank) to
/// it. Minions you control can lock this location to use those counters to
/// pay some or all of the blood or pool cost of equipment they equip." In
/// place of pool or blood, as the cost is (RTR 19960112).
pub static RAVNOS_CACHE: CardDef = CardDef {
    name: "Ravnos Cache",
    modes: &[master_in_play(
        &[],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::CostSource(CostSource::Cache)),
        ],
    )],
    abilities: &[
        mode_of(
            &[Window::MasterPhase],
            Actor::Controller,
            &[],
            &[Effect::PoolToCounterPlusBank { pool: 1, bank: 1 }],
        ),
        ModeDef {
            terms: Some(Terms::XCounters),
            ..mode_of(
                &[Window::ActionDuring, Window::ActionWouldBeSuccessful],
                Actor::Controller,
                &[
                    Requirement::CardUnlocked,
                    Requirement::CountersAtLeast(1),
                    Requirement::OwnActionCardCosts(CostSource::Cache),
                ],
                &[Effect::LockSelf, Effect::PayActionWithCountersX],
            )
        },
    ],
    ..CARD
};

/// "Unique location. This location comes into play with 1 counter for each
/// Ravnos you control. Ravnos you control can use those counters to pay some
/// or all of the blood cost of action cards they play. If this location has
/// no counters, burn it." (1 pool; Ravnos.) An action otherwise unaffordable
/// may be attempted (LSJ 20050408); the counters are announced with the
/// card (RTR 20111202); not a cardless action nor one granted by a card in
/// play (ANK 20180109, LSJ 20070224).
pub static RAVNOS_CARNIVAL: CardDef = CardDef {
    name: "Ravnos Carnival",
    modes: &[master_in_play(
        &[],
        &[
            Effect::PutInPlay,
            Effect::EnterWithCountersPerOwnClan("Ravnos"),
            Effect::Layer(Modifier::CostSource(CostSource::Carnival("Ravnos"))),
        ],
    )],
    ..CARD
};

/// "Unique location. During your master phase, you can put a ghoul (ally or
/// retainer) from your hand on this location, face up and out of play.
/// Tzimisce you control can play cards from this location as if from your
/// hand (requirements and cost apply as normal)." Put there, a card is not
/// replaced; played from there, neither (rules-qa 37.1).
pub static FLESHFORGE_CHAMBER: CardDef = CardDef {
    name: "Fleshforge Chamber",
    modes: &[master_in_play(
        &[],
        &[Effect::PutInPlay, Effect::PlayableFromStore("Tzimisce")],
    )],
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::HasLabel(crate::library::Label::Ghoul),
        }),
        limit: Some(Limit::PerPhaseEach),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Controller,
            &[Requirement::HandHas(
                CardFilter::HasLabel(crate::library::Label::Ghoul),
                1,
            )],
            &[Effect::StorePickedFromHand],
        )
    }],
    ..CARD
};

// ---- Events (slice 11ac) -----------------------------------------------------

/// An event put in play with a discard phase action (rulebook "Event
/// Cards"): once each discard phase, once each game (engine-checked).
const fn event(requires: &'static [Requirement], effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::DiscardPhase],
        actor: Actor::Methuselah,
        requires,
        target: None,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// "After another Gehenna event is played, burn 1 counter from this card."
const GEHENNA_COUNTDOWN: Rider = Rider {
    hook: Hook::GehennaPlayed,
    scope: RiderScope::Every,
    requires: &[],
    effects: &[Effect::RemoveCounter(1)],
};

/// "Inconnu. Do not replace until your next unlock phase. Minion cards that
/// change the target of a bleed action cost +1 blood or life." Replaced
/// before cards unlock (ANK 20200129, LSJ 20091208, RTR 19951017).
pub static NARROW_MINDS: CardDef = CardDef {
    name: "Narrow Minds",
    delayed_replacement: Some(Until::NextUnlockPhase),
    modes: &[event(
        &[],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::RedirectCardsCost(1)),
        ],
    )],
    ..CARD
};

/// "Gehenna. Do not replace until a vampire moves from torpor to the ready
/// region. Actions performed by vampires in torpor cost +1 blood. Rescuing
/// an older vampire from torpor costs +1 blood." Not replaced before the
/// condition, burned or not (LSJ 20080805).
pub static TORPID_BLOOD: CardDef = CardDef {
    name: "Torpid Blood",
    delayed_replacement: Some(Until::LeavesTorpor),
    modes: &[event(
        &[],
        &[Effect::PutInPlay, Effect::Layer(Modifier::TorpidBlood)],
    )],
    ..CARD
};

/// "Gehenna. Requires 1 or more other Gehenna events in play. Do not
/// replace until a vampire commits diablerie. Cards requiring 1 or more
/// Disciplines at the superior level cost +1 blood. Vampires who commit
/// diablerie ignore this effect until a Gehenna event is played." Paid
/// with the normal cost (ANK 20190625); the requirement read as it is
/// played only (PIB 20121031).
pub static SLOW_WITHERING: CardDef = CardDef {
    name: "Slow Withering, The",
    delayed_replacement: Some(Until::Diablerie),
    modes: &[event(
        &[Requirement::GehennaInPlay(1)],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::SuperiorCardsCost(1)),
        ],
    )],
    ..CARD
};

/// "Gehenna. Do not replace until a vampire commits diablerie. During an
/// action, cards minions play requiring 1 or more Disciplines are not
/// replaced until the end of that action or until a Methuselah's hand is
/// empty (whichever comes first). Vampires who commit diablerie ignore
/// this effect until a Gehenna event is played." During any action (ANK
/// 20201015); several held cards are drawn one at a time (ANK 20180925-1).
pub static BLOOD_WEAKENS: CardDef = CardDef {
    name: "Blood Weakens",
    delayed_replacement: Some(Until::Diablerie),
    modes: &[event(
        &[],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::HoldDisciplineReplacements),
        ],
    )],
    ..CARD
};

/// "Gehenna. After each Methuselah's minion phase ends, each ready vampire
/// controlled by that Methuselah with capacity less than the number of
/// Gehenna events in play who did not hunt during that minion phase burns
/// 1 blood." An unsuccessful hunt counts (LSJ 20050727).
pub static THIRST: CardDef = CardDef {
    name: "Thirst",
    modes: &[event(
        &[],
        &[Effect::PutInPlay, Effect::Layer(Modifier::Thirst)],
    )],
    ..CARD
};

/// "Gehenna. Requires 1 or more other Gehenna events in play. Do not
/// replace until your next discard phase. This card comes into play with
/// 10 counters. After another Gehenna event is played, burn 1 counter from
/// this card. Vampires with capacity greater than X are considered
/// vampires with capacity X (minimum of 1), where X is the number of
/// counters on this card." The capacity as changed by other effects
/// ceiled (LSJ 20051205); excess blood drains as it applies.
pub static WORMWOOD: CardDef = CardDef {
    name: "Wormwood",
    delayed_replacement: Some(Until::NextDiscardPhase),
    modes: &[event(
        &[Requirement::GehennaInPlay(1)],
        &[
            Effect::PutInPlay,
            Effect::EnterWithCounters(10),
            Effect::Layer(Modifier::CapacityCapByCounters),
        ],
    )],
    riders: &[GEHENNA_COUNTDOWN],
    ..CARD
};

/// "Gehenna. Do not replace until a vampire commits diablerie. This card
/// comes into play with 10 counters. After another Gehenna event is
/// played, burn 1 counter from this card. Blood hunts cannot be called on
/// vampires with capacity greater than the number of counters on this card
/// who diablerize a younger vampire."
pub static FUELED_BY_HEARTS_BLOOD: CardDef = CardDef {
    name: "Fueled by Heart's Blood",
    delayed_replacement: Some(Until::Diablerie),
    modes: &[event(
        &[],
        &[
            Effect::PutInPlay,
            Effect::EnterWithCounters(10),
            Effect::Layer(Modifier::NoBloodHuntAboveCounters),
        ],
    )],
    riders: &[GEHENNA_COUNTDOWN],
    ..CARD
};

/// "Requires 2 or more Gehenna events in play. After this card enters
/// play, burn all Gehenna events and conviction cards, then send all
/// vampires with capacity 3 or less to torpor. Gehenna events cannot be
/// played." No conviction card in V5.
pub static FOURTH_CYCLE: CardDef = CardDef {
    name: "Fourth Cycle, The",
    modes: &[event(
        &[Requirement::GehennaInPlay(2)],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::NoGehennaEvents),
            Effect::FourthCycle(3),
        ],
    )],
    ..CARD
};

/// "Do not replace as long as this card is in play. During your discard
/// phase, burn another event at random. Other Methuselahs can use a
/// discard phase action and burn X pool to burn this card, where X is the
/// number of other events in play." The random burn as its controller's
/// discard phase begins.
pub static UNCOILING: CardDef = CardDef {
    name: "Uncoiling, The",
    delayed_replacement: Some(Until::LeavesPlay),
    modes: &[event(
        &[],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::BurnsAnEventEachDiscard),
        ],
    )],
    abilities: &[mode_of(
        &[Window::DiscardPhaseAction],
        Actor::Methuselah,
        &[
            Requirement::OthersControlThisCard,
            Requirement::DiscardPhaseActionLeft,
        ],
        &[
            Effect::SpendDiscardPhaseAction,
            Effect::PayPoolPerOtherEvent,
            Effect::BurnSelf,
        ],
    )],
    ..CARD
};

/// "Gehenna. Burn all boons. No more boons can be put in play. During each
/// Methuselah's unlock phase, that Methuselah can move 1 blood from a
/// vampire they control to a vampire controlled by another Methuselah."
/// No boon enters play in V5 (Consanguineous Boon is a political action,
/// still playable: LSJ 20040531-2).
pub static BLOOD_TRADE: CardDef = CardDef {
    name: "Blood Trade",
    modes: &[event(&[], &[Effect::PutInPlay])],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::Controlled,
                Requirement::TargetVampire,
                Requirement::BloodAtLeast(1),
            ],
        ),
        ..mode_of(
            &[Window::UnlockPhaseAny],
            Actor::Methuselah,
            &[Requirement::OwnTurn],
            &[Effect::GiveBloodToOthers],
        )
    }],
    ..CARD
};

/// "Gehenna. Requires 2 or more other Gehenna events in play. Do not
/// replace until your next discard phase. During each Methuselah's unlock
/// phase, that Methuselah can choose a location controlled by their prey.
/// The chosen location is burned unless its controller burns 2 pool."
pub static CONQUEST_OF_HUMANITY: CardDef = CardDef {
    name: "Conquest of Humanity",
    delayed_replacement: Some(Until::NextDiscardPhase),
    modes: &[event(
        &[Requirement::GehennaInPlay(2)],
        &[Effect::PutInPlay],
    )],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Card,
            &[
                Requirement::TargetCard(CardFilter::Location),
                Requirement::CardControlledByPrey,
            ],
        ),
        ..mode_of(
            &[Window::UnlockPhaseAny],
            Actor::Methuselah,
            &[Requirement::OwnTurn],
            &[Effect::BurnUnlessControllerPays(2)],
        )
    }],
    ..CARD
};

/// "Gehenna. Requires 1 or more other Gehenna events in play. Do not
/// replace until a titled vampire goes to torpor. During each Methuselah's
/// unlock phase, that Methuselah can choose a ready vampire controlled by
/// their prey. The chosen vampire takes 1 unpreventable environmental
/// damage. Titled vampires can call a referendum to burn this card as a +1
/// stealth political action." Replaced before any torpor effect (LSJ
/// 20100527).
pub static NEW_INQUISITION: CardDef = CardDef {
    name: "New Inquisition, The",
    delayed_replacement: Some(Until::TitledToTorpor),
    modes: &[event(
        &[Requirement::GehennaInPlay(1)],
        &[Effect::PutInPlay],
    )],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::ControlledByPrey,
                    Requirement::TargetVampire,
                    Requirement::ActorReady,
                ],
            ),
            ..mode_of(
                &[Window::UnlockPhaseAny],
                Actor::Methuselah,
                &[Requirement::OwnTurn],
                &[Effect::DamageTarget(1)],
            )
        },
        mode_of(
            &[Window::ActionDeclarePolitical],
            Actor::ActingCandidate,
            &[
                Requirement::ActorVampire,
                Requirement::ActorTitleIn(&[TitleClass::Any]),
            ],
            &[
                Effect::DeclareAction(ActionKind::Political),
                Effect::AddStealth(1),
                Effect::CallReferendum(&BURN_THIS_CARD),
            ],
        ),
    ],
    ..CARD
};

/// "Gehenna. Do not replace until your next discard phase. During each
/// Methuselah's unlock phase, for each minion they control, they lock that
/// minion unless they discard a card; cards discarded this way are not
/// replaced until the master phase. Mortal allies and vampires with
/// capacity greater than the number of Gehenna events in play ignore this
/// effect." Once each unlock phase, the minions then in play (ANK
/// 20250121); a locked minion may be "locked" (ANK 20210322-2).
pub static NIGHTMARES_UPON_NIGHTMARES: CardDef = CardDef {
    name: "Nightmares upon Nightmares",
    delayed_replacement: Some(Until::NextDiscardPhase),
    modes: &[event(&[], &[Effect::PutInPlay])],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::LockUnlessDiscard],
    }],
    ..CARD
};

/// "Gehenna. Requires 2 or more other Gehenna events in play. Do not
/// replace as long as this card is in play. During each Methuselah's unlock
/// phase, if that Methuselah controls 3 or more vampires of the same clan,
/// they burn one of those vampires. If that vampire's capacity is 6 or
/// more, that Methuselah ignores this effect until the end of the game."
pub static RECALLED_TO_THE_FOUNDER: CardDef = CardDef {
    name: "Recalled to the Founder",
    delayed_replacement: Some(Until::LeavesPlay),
    modes: &[event(
        &[Requirement::GehennaInPlay(2)],
        &[Effect::PutInPlay],
    )],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::BurnOfCrowdedClan],
    }],
    ..CARD
};

/// "Gehenna. Do not replace until a vampire successfully hunts. Vampires
/// cannot hunt unless they must hunt. Vampires who must hunt can steal 1
/// blood from a younger vampire as a Ⓓ hunt action." The special hunt is
/// Week of Nightmares' (once a turn per copy; no blood stolen, no
/// successful hunt; at a vampire, none other, no action).
pub static RESTRICTED_VITAE: CardDef = CardDef {
    name: "Restricted Vitae",
    delayed_replacement: Some(Until::SuccessfulHunt),
    modes: &[event(
        &[],
        &[Effect::PutInPlay, Effect::Layer(Modifier::OnlyMustHunt)],
    )],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::TargetVampire,
                Requirement::Younger,
                Requirement::OtherThanActor,
            ],
        ),
        ..mode_of(
            &[Window::ActionDeclare],
            Actor::ActingCandidate,
            &[Requirement::ActorVampire, Requirement::ActorMustHunt],
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::HuntAction,
                Effect::StealBloodFromTarget(1),
            ],
        )
    }],
    ..CARD
};

/// "Government. If an ally is burned in combat with an acting vampire, add
/// 1 counter to this card and inflict 2 unpreventable environmental damage
/// on the acting vampire after the combat ends. If this card has 4
/// counters, burn it." Not when the acting minion is the ally (ANK
/// 20180913-2).
pub static FBI_SPECIAL_AFFAIRS_DIVISION: CardDef = CardDef {
    name: "FBI Special Affairs Division",
    modes: &[event(
        &[],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::AllyBurnedInCombat {
                damage: 2,
                burn_at: 4,
            }),
        ],
    )],
    ..CARD
};

/// "Gehenna. Requires 3 or more other Gehenna events in play. Do not
/// replace as long as this card is in play. There is no Camarilla.
/// Instead, Camarilla vampires are considered Independent." Read live, the
/// sect a card gives included (LSJ 20040519, LSJ 20080510); a Camarilla
/// title gives nothing meanwhile (LSJ 20060904).
pub static FALL_OF_THE_CAMARILLA: CardDef = CardDef {
    name: "Fall of the Camarilla",
    delayed_replacement: Some(Until::LeavesPlay),
    modes: &[event(
        &[Requirement::GehennaInPlay(3)],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::SectFallen(Sect::Camarilla)),
        ],
    )],
    ..CARD
};

/// "Gehenna. Requires 3 or more other Gehenna events in play. Not usable if
/// a ready vampire is Black Hand. Do not replace as long as this card is in
/// play. There is no Sabbat. Instead, Sabbat vampires are considered
/// Independent." No Black Hand vampire in V5.
pub static FALL_OF_THE_SABBAT: CardDef = CardDef {
    name: "Fall of the Sabbat",
    delayed_replacement: Some(Until::LeavesPlay),
    modes: &[event(
        &[Requirement::GehennaInPlay(3)],
        &[
            Effect::PutInPlay,
            Effect::Layer(Modifier::SectFallen(Sect::Sabbat)),
        ],
    )],
    ..CARD
};

/// "Gehenna. Requires 1 or more other Gehenna events in play. Do not
/// replace until a vampire becomes a liaison. Independent vampires can
/// receive the unique Independent title of Liaison (worth 4 votes) as an
/// action. If this title would be contested with a younger vampire, the
/// younger vampire immediately yields instead of contesting." The title
/// stays as the card leaves play (LSJ 20070125).
pub static RISE_OF_THE_NEPHTALI: CardDef = CardDef {
    name: "Rise of the Nephtali",
    delayed_replacement: Some(Until::Liaison),
    modes: &[event(
        &[Requirement::GehennaInPlay(1)],
        &[Effect::PutInPlay],
    )],
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::ActingCandidate,
        &[
            Requirement::ActorVampire,
            Requirement::ActorSectIn(&[Sect::Independent]),
        ],
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::GainLiaison,
        ],
    )],
    ..CARD
};

/// "Gehenna. Requires 1 or more other Gehenna events in play. Do not
/// replace until a Methuselah is ousted. During each Methuselah's unlock
/// phase, that Methuselah can put the top card of their library in play
/// (face up); it becomes a nameless, textless ghoul ally with 2 life, 2
/// strength and 1 bleed. After that ally is burned or this event leaves
/// play, remove that ally from the game. After a Methuselah is ousted, burn
/// this card." Not a play (LSJ 20040531); no contest, its name ignored
/// (LSJ 20040623, LSJ 20071003); burned, then removed; removed without
/// burning as the card leaves (LSJ 20090920); a monster, not a mortal (ANK
/// 20200203-2).
pub static ABSIMILIARDS_ARMY: CardDef = CardDef {
    name: "Absimiliard's Army",
    delayed_replacement: Some(Until::MethuselahOusted),
    modes: &[event(
        &[Requirement::GehennaInPlay(1)],
        &[Effect::PutInPlay, Effect::Layer(Modifier::GhoulArmy)],
    )],
    abilities: &[mode_of(
        &[Window::UnlockPhaseAny],
        Actor::Methuselah,
        &[Requirement::OwnTurn],
        &[Effect::GhoulFromLibraryTop(crate::library::Body {
            life: 2,
            strength: 2,
            bleed: 1,
        })],
    )],
    ..CARD
};

// ---- Legacy cards for tournament decks (slice 16a) ------------------------
//
// Owner-added legacy cards (2026-09-28) that make TWDA decks playable,
// appended to the tables (`ADDED_CARDS` in scripts/window_matrix.py).

/// "Unique location. Lock to get +1 hand size this turn. Lock during your
/// influence phase to get +1 transfer." (Gangrel or Gangrel antitribu:
/// either clan, ANK 20190203.) The hand size lock is untimed (rules-qa
/// 7.7); the transfer lock may come at any moment of the phase (RTR
/// 20180303). One lock, so one of the two a turn.
pub static ENNOIAS_THEATER: CardDef = CardDef {
    name: "Ennoia's Theater",
    modes: &[LOCATION],
    abilities: &[
        mode_of(
            &[Window::AnyImpulse],
            Actor::Controller,
            &[Requirement::CardUnlocked],
            &[Effect::LockSelf, Effect::HandSizeBonusUntilEndOfTurn(1)],
        ),
        mode_of(
            &[Window::InfluencePhaseOwn],
            Actor::Controller,
            &[Requirement::CardUnlocked],
            &[Effect::LockSelf, Effect::GainTransfers(1)],
        ),
    ],
    ..CARD
};

/// "Requires an Anarch. Only usable at close range. [obf] Strike: burn
/// equipment or retainer with first strike. [pro] Strike: hand strike,
/// aggravated. [ser] Strike: steal 2 blood or life (becoming blood)." The
/// equipment or retainer is the opposing minion's, named with the strike;
/// with first strike it burns before the opposing strike resolves, so a
/// weapon burned so loses its strike (rulebook "First Strike"). [ser] may
/// target a minion with less than 2 (RTR 20010711).
pub static DONNYBROOK: CardDef = CardDef {
    name: "Donnybrook",
    modes: &[
        ModeDef {
            target: target(
                TargetKind::Card,
                &[
                    Requirement::AnyOf(&[
                        &[Requirement::TargetCard(CardFilter::Type(
                            CardType::Equipment,
                        ))],
                        &[Requirement::TargetCard(CardFilter::Type(
                            CardType::Retainer,
                        ))],
                    ]),
                    Requirement::TargetOnOpposingCombatant,
                ],
            ),
            ..combat(
                &[Window::CombatStrikeDeclaration],
                &[ANARCH, Requirement::RangeClose],
                &[
                    Effect::Strike(StrikeKind::BurnEquipment),
                    Effect::FirstStrike,
                ],
            )
        },
        combat(
            &[Window::CombatStrikeDeclaration],
            &[ANARCH, Requirement::RangeClose],
            &[
                Effect::Strike(StrikeKind::Hand { bonus: 0 }),
                Effect::AggravatedStrike,
            ],
        ),
        combat(
            &[Window::CombatStrikeDeclaration],
            &[ANARCH, Requirement::RangeClose],
            &[Effect::Strike(StrikeKind::StealBlood {
                n: 2,
                ranged: false,
            })],
        ),
    ],
    ..CARD
};

/// "Requires an Anarch. Only one Hell-for-Leather can be played at a given
/// Discipline each combat. [ani] Strike: dodge, with 1 additional strike
/// (limited). [cel] Additional strike that does not count against the
/// limit. [obt] Only usable if this Anarch is ready and the opposing
/// minion is not. The opposing minion's controller burns 2 pool." Each
/// mode is its Discipline: once per combat each, per Anarch. [cel] may
/// come before or after a limited additional strike (LSJ 20001206). The
/// opposing minion is not ready once the round's damage sent it to torpor:
/// [obt] is played in the end-of-round window that still opens (rules-qa
/// 44.1).
pub static HELL_FOR_LEATHER: CardDef = CardDef {
    name: "Hell-for-Leather",
    modes: &[
        ModeDef {
            limit: Some(Limit::PerCombatThisMode),
            ..combat(
                &[Window::CombatStrikeDeclaration],
                &[ANARCH],
                &[
                    Effect::Strike(StrikeKind::Dodge),
                    Effect::AdditionalStrikes {
                        n: 1,
                        limited: true,
                    },
                ],
            )
        },
        ModeDef {
            limit: Some(Limit::PerCombatThisMode),
            ..combat(
                &[Window::CombatAdditionalStrike],
                &[ANARCH],
                &[Effect::AdditionalStrikes {
                    n: 1,
                    limited: false,
                }],
            )
        },
        ModeDef {
            limit: Some(Limit::PerCombatThisMode),
            ..combat(
                &[Window::CombatEndOfRound],
                &[
                    ANARCH,
                    Requirement::ActorReady,
                    Requirement::OpponentNotReady,
                ],
                &[Effect::OpposingControllerBurnsPool(2)],
            )
        },
    ],
    ..CARD
};

/// "[dom] [ACTION MODIFIER] Only usable during a bleed action. +1 bleed
/// (limited). [DOM] [REACTION] Only usable if a younger vampire is bleeding
/// you, after blocks are declined. Lock this reacting vampire. Change the
/// target of the bleed to another Methuselah other than the acting
/// vampire's controller (that Methuselah can attempt to block)." The
/// reaction is Redirection [dom]'s: blocks failed on stealth were declined
/// (ANK 20211003), before any "would be successful" card (LSJ 19980105),
/// only to a Methuselah the bleed could target (PIB 20130711).
pub static MURMUR_OF_THE_FALSE_WILL: CardDef = CardDef {
    name: "Murmur of the False Will",
    modes: &[
        mode_of(
            &[Window::ActionDuring],
            Actor::ActingMinion,
            &[Requirement::IsBleed],
            &[bleed(1, true)],
        ),
        ModeDef {
            target: REDIRECT_TARGET,
            ..mode_of(
                &[Window::ActionBlocksDeclined],
                Actor::ReactingMinion,
                &[
                    Requirement::DirectedAtYou,
                    Requirement::IsBleed,
                    Requirement::BlocksDeclined,
                    Requirement::ActingVampire,
                    Requirement::ActingYounger,
                ],
                &[Effect::LockActor, Effect::RedirectBleed],
            )
        },
    ],
    ..CARD
};

/// "Unique mummy with 2 life. 2 strength, 1 bleed. During an action,
/// Saatet-ta can lock to give a Follower of Set you control +1 stealth, +1
/// intercept, or +1 bleed. If she is burned, shuffle her into her owner's
/// library." Follower of Set is the clan's V5 name, Ministry. The stealth to
/// the acting Follower when needed, the intercept to one attempting to
/// block when needed, the bleed during a bleed action before it resolves
/// (ANK 20231124); usable the turn she is recruited (ANK 20180517).
/// Shuffled when burned, still burned (ANK 20200813); not when discarded
/// (RTR 20010711).
pub static SAATET_TA: CardDef = CardDef {
    name: "Saatet-ta",
    modes: &[recruit(&[Effect::DeclareAction(ActionKind::Recruit)])],
    abilities: &[
        mode_of(
            &[Window::ActionDuring],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::OtherActingControlled,
                Requirement::OtherActingClan("Ministry"),
                Requirement::StealthNeeded,
            ],
            &[Effect::LockSelf, Effect::AddStealth(1)],
        ),
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::YourAttemptNeedsIntercept,
                Requirement::AttemptingClan("Ministry"),
            ],
            &[Effect::LockSelf, Effect::AttemptingIntercept(1)],
        ),
        mode_of(
            &[Window::ActionDuring],
            Actor::Controller,
            &[
                Requirement::CardUnlocked,
                Requirement::IsBleed,
                Requirement::OtherActingControlled,
                Requirement::OtherActingClan("Ministry"),
            ],
            &[Effect::LockSelf, bleed(1, false)],
        ),
    ],
    riders: &[Rider {
        hook: Hook::SelfBurned,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::ShuffleSourceIntoLibrary],
    }],
    ..CARD
};

/// "Unique mortal with 2 life. 0 strength, 0 bleed. Carlton can strike: 1R
/// damage. Once each combat, he can strike: dodge. Carlton gets +1
/// intercept against vampires. During your discard phase, you can burn
/// Carlton to burn a vampire who has committed diablerie since your last
/// turn." Since the end of your last turn, your current one included (LSJ
/// 20020429), the turn he is recruited too (ANK 20180517); any vampire,
/// yours or in torpor included.
pub static CARLTON_VAN_WYK: CardDef = CardDef {
    name: "Carlton Van Wyk",
    modes: &[recruit(&[
        Effect::DeclareAction(ActionKind::Recruit),
        Effect::Layer(Modifier::HostInterceptVsVampires(1)),
    ])],
    abilities: &[
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Host,
            &[],
            &[Effect::Strike(StrikeKind::Damage {
                n: 1,
                ranged: true,
                aggravated: false,
            })],
        ),
        DODGE_EACH_COMBAT,
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    Requirement::TargetVampire,
                    Requirement::TargetDiablerieSinceYourLastTurn,
                ],
            ),
            ..mode_of(
                &[Window::DiscardPhase],
                Actor::Controller,
                &[],
                &[Effect::BurnSelf, Effect::BurnTargetMinion],
            )
        },
    ],
    ..CARD
};

/// "Unique. Put this card in play; it becomes a unique mortal ally with 1
/// life, 0 strength and 0 bleed. During your unlock phase, you can burn
/// Jake to add up to 4 blood to a ready vampire with no blood." Not an ally
/// in the ash heap (LSJ 20071014); his starting life is 1 (LSJ
/// 20030520-2). Any Methuselah's vampire; the amount chosen as the burn is
/// used (1 to 4, excess over capacity draining).
pub static JAKE_WASHINGTON: CardDef = CardDef {
    name: "Jake Washington",
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[],
        &[Effect::BecomesAlly(crate::library::Body {
            life: 1,
            strength: 0,
            bleed: 0,
        })],
    )],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::TargetVampire,
                Requirement::ActorReady,
                Requirement::BloodAtMost(0),
            ],
        ),
        terms: Some(Terms::Count { min: 1, max: 4 }),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Controller,
            &[],
            &[Effect::BurnSelf, Effect::BloodToTargetX],
        )
    }],
    ..CARD
};

/// "Master: unique location. Elysium. You may lock any other unique
/// location you control to end combat involving an acting vampire you
/// control before range is chosen. Any vampire may steal this location for
/// his or her controller as a Ⓓ action." The acting vampire is the one
/// considered acting in the combat, Deep Song's target included (ANK
/// 20230814). "Elysium" is inert: no pool card reads it.
pub static POWERBASE_SAVANNAH: CardDef = CardDef {
    name: "Powerbase: Savannah",
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Card,
                &[
                    Requirement::Controlled,
                    Requirement::TargetCard(CardFilter::Location),
                    Requirement::TargetCard(CardFilter::HasLabel(Label::Unique)),
                    Requirement::TargetUnlocked,
                    Requirement::TargetNotNamed("Powerbase: Savannah"),
                ],
            ),
            ..mode_of(
                &[Window::CombatBeforeRange],
                Actor::Controller,
                &[Requirement::ActingCombatantYourVampire],
                &[Effect::LockTargetCard, Effect::EndCombat],
            )
        },
        steal_this(&[
            Requirement::ActorVampire,
            Requirement::OthersControlThisCard,
        ]),
    ],
    ..CARD
};

/// "Only one Ashur Tablets can be played each turn. Put this card in play.
/// If you control three copies of this card, remove all copies in play
/// (even controlled by other Methuselahs) from the game to gain 3 pool and
/// choose up to thirteen library cards from your ash heap; move one of the
/// chosen cards to your hand and shuffle the others into your library."
/// Playable with an empty ash heap (PIB 20130119); the cards chosen only
/// as the third copy enters play (LSJ 20091030); the one to hand is shown
/// (LSJ 20081129) — public in the engine's observation of the move.
pub static ASHUR_TABLETS: CardDef = CardDef {
    name: "Ashur Tablets",
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[Requirement::NotPlayedThisTurn],
        &[Effect::PutInPlay, Effect::AshurTablets],
    )],
    ..CARD
};

/// "Master: out-of-turn. Frenzy. Put this card on a vampire when an
/// opposing minion attempts to inflict aggravated damage on him or her,
/// whether the damage would be successfully inflicted or not. Combat ends.
/// This vampire is locked and sent to torpor. This vampire does not unlock
/// as normal. During this vampire's next unlock phase, burn this card."
/// On the vampire facing the damage (RTR 19980623), once both strikes are
/// declared, before they resolve (LSJ 19990217); a strike's damage only
/// (ANK 20220114), effective at the range (RTR 19980928), dodged or not
/// (LSJ 20011005), aggravated at its source (ANK 20200130-1). Combat ends
/// at once: nothing resolves, no prevention, no second Rötschreck (LSJ
/// 19990217); both minions still ready as it ends (LSJ 20070907); a
/// combat that continues keeps the card on but no lock nor torpor (RTR
/// 20020501). The unlock ban ends with the card (ANK 20191025).
pub static ROTSCHRECK: CardDef = CardDef {
    name: "Rötschreck",
    out_of_turn: true,
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::Controlled,
                Requirement::TargetVampire,
                Requirement::TargetCombatant,
            ],
        ),
        ..mode_of(
            &[Window::CombatBeforeResolution],
            Actor::Methuselah,
            &[
                Requirement::NotOwnTurn,
                Requirement::OwnCombatantFacesAggravatedStrike,
            ],
            &[
                Effect::PutOnTarget,
                Effect::Layer(Modifier::HostDoesNotUnlock),
                Effect::RotschreckEndsCombat,
            ],
        )
    }],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::BurnSelf],
    }],
    ..CARD
};

/// "Out-of-turn. Boon. Give pool to a Methuselah with no pool to keep them
/// in the game, and put this card in play. During each of their unlock
/// phases, you can steal 1 pool from that Methuselah. You gain any victory
/// point gained by that Methuselah instead of them, until you are being
/// ousted or until you have gained 1 victory point from this card. Burn
/// this card after you have gained 1 victory point from this card."
/// Played in the decision point before an oust (`ousting`), on another
/// Methuselah's turn; the pool given covers what they owe (RTR 19970630);
/// several Boons reserve a series of VPs (RTR 19951017), half points going
/// to the Boon player (RTR 19991206); a prey ousted with the target by the
/// same effect scores for the Boon player, the target keeping the 6 pool
/// (LSJ 20021029); a Boon on one's predator gives nothing as one is ousted
/// (RTR 20000501).
pub static LIFE_BOON: CardDef = CardDef {
    name: "Life Boon",
    out_of_turn: true,
    modes: &[ModeDef {
        target: target(
            TargetKind::Seat,
            &[
                Requirement::NotSelf,
                Requirement::TargetSeatOusting,
                Requirement::CanSaveTarget,
            ],
        ),
        terms: Some(Terms::BoonPool),
        ..mode_of(
            &[Window::Ousting],
            Actor::Methuselah,
            &[Requirement::NotOwnTurn],
            &[Effect::PutInPlay, Effect::GiveBoonPool],
        )
    }],
    abilities: &[mode_of(
        &[Window::UnlockPhaseAny],
        Actor::Controller,
        &[Requirement::BoonTargetActive],
        &[Effect::StealPoolFromBoonTarget],
    )],
    ..CARD
};

// ---- Reign of Stanislava (25th Anniversary) -------------------------------
//
// The deck's new cards (owner, 2026-10-06), appended to the tables
// (`ADDED_BATCHES` in scripts/window_matrix.py); rulings in rules-qa Batch
// 49.

/// "Add 4 blood to a vampire in your uncontrolled region with capacity 8
/// or more."
pub static ZILLAHS_VALLEY: CardDef = CardDef {
    name: "Zillah's Valley",
    modes: &[ModeDef {
        target: target(
            TargetKind::Uncontrolled,
            &[Requirement::TargetCapacityAtLeast(8)],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[Effect::BloodToUncontrolledTarget(4)],
        )
    }],
    ..CARD
};

static POLITICAL_STRANGLEHOLD_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[],
    effect: RefEffect::GainPerVampireCapacityAtLeast { cap: 8, n: 3 },
    grants: &[],
    non_anarch_titles_minus: false,
};
political!(
    POLITICAL_STRANGLEHOLD,
    "Political Stranglehold",
    "\"Only one Political Stranglehold can be played or called in a game. \
     Successful referendum means each Methuselah gains 3 pool for each \
     vampire they control with capacity 8 or more.\" Burning it for a vote \
     is not playing it (ANK 20210309-2). Vampires in torpor count: they are \
     controlled (rules-qa 50.5).",
    &[Requirement::NotPlayedThisGame],
    POLITICAL_STRANGLEHOLD_REFERENDUM
);

static ANCIENT_INFLUENCE_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[],
    effect: RefEffect::GainChosenCapacityBurn(5),
    grants: &[],
    non_anarch_titles_minus: false,
};
political!(
    ANCIENT_INFLUENCE,
    "Ancient Influence",
    "\"Only one Ancient Influence can be played or called in a game. \
     Successful referendum means each Methuselah can choose a ready vampire \
     they control. Each Methuselah gains pool equal to their chosen \
     vampire's capacity, then burns 5 pool.\" The choices are made as it \
     passes, clockwise from the acting Methuselah, in the open; none gains \
     nothing, the 5 still burned (rules-qa 49.1). A gain then a loss: the \
     loss counts in all cases (ANK 20180813). Ousts at once: a Methuselah \
     whose prey is ousted gets the victory point though ousted too, only \
     the survivors the 6 pool (LSJ 20000309). Burning it for a vote is not \
     playing it (ANK 20210309-2).",
    &[Requirement::NotPlayedThisGame],
    ANCIENT_INFLUENCE_REFERENDUM
);

static REINS_OF_POWER_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[],
    effect: RefEffect::GainBurnPredatorsChosenCapacity(6),
    grants: &[],
    non_anarch_titles_minus: false,
};
political!(
    REINS_OF_POWER,
    "Reins of Power",
    "\"Only one Reins of Power can be played or called in a game. \
     Successful referendum means each Methuselah can choose a ready vampire \
     they control. Each Methuselah gains 6 pool, then burns pool equal to \
     the capacity of their predator's chosen vampire.\" The choices as \
     Ancient Influence's (rules-qa 49.1): a predator's no choice burns 0. A \
     gain then a loss (ANK 20180813); simultaneous ousts (LSJ 20000309); \
     burning it for a vote is not playing it (ANK 20210309-2).",
    &[Requirement::NotPlayedThisGame],
    REINS_OF_POWER_REFERENDUM
);

static PERMANENT_VACATION_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::ChooseMinions {
        filter: &[Requirement::ActorReady, Requirement::TargetAlly],
        min: 1,
        max: 1,
    }],
    effect: RefEffect::RemoveChosen,
    grants: &[],
    non_anarch_titles_minus: false,
};
political!(
    PERMANENT_VACATION,
    "Permanent Vacation",
    "\"Choose a ready ally. Successful referendum means the chosen ally is \
     removed from the game.\" Not playable without a ready ally to choose \
     (LSJ 20100129; ANK 20191228). Removed, not burned: the cards on it are \
     burned, no burn effect fires (rules-qa 50.6).",
    &[],
    PERMANENT_VACATION_REFERENDUM
);

/// "Unique location. +1 hand size. Lock to give a vampire with capacity 8
/// or more +1 stealth." Any acting vampire, another Methuselah's included;
/// "you" in card text is its controller, who locks it (rules-qa 49.5).
/// Used when stealth is needed, holding the impulse in the stealth and
/// intercept rotation (KRCG News Radio; rules-qa 50.4): not "even if
/// stealth is not yet needed" (Creepshow Casino). The hand size is suspended while contested (ANK
/// 20221028, Elder Library).
pub static MONASTERY_OF_SHADOWS: CardDef = CardDef {
    name: "Monastery of Shadows",
    modes: &[mode_of(
        &[Window::MasterPhase],
        Actor::Methuselah,
        &[],
        &[Effect::PutInPlay, Effect::Layer(Modifier::HandSize(1))],
    )],
    abilities: &[mode_of(
        &[Window::ActionStealthIntercept],
        Actor::Controller,
        &[
            Requirement::CardUnlocked,
            Requirement::ActingCapacityAtLeast(8),
            Requirement::StealthNeeded,
        ],
        &[Effect::LockSelf, Effect::AddStealth(1)],
    )],
    ..CARD
};

/// "A vampire can play only one Forced March each turn. [cel][for] Only
/// usable after resolution of a successful action. Unlock this vampire.
/// [CEL][FOR] +1 stealth. After action resolution, if the action was
/// successful, this vampire can burn 1 blood to unlock." Both modes count
/// toward the limit (`Limit::PerTurn`). After resolution, still during the
/// action (LSJ 19981028; ANK 20190425); successful though the action had no
/// effect (LSJ 20070411) or fizzled (ANK 20220218). The blood burned is an
/// effect, not a cost: no reduction applies (ANK 20210226).
pub static FORCED_MARCH: CardDef = CardDef {
    name: "Forced March",
    modes: &[
        ModeDef {
            limit: Some(Limit::PerTurn),
            ..mode_of(
                &[Window::ActionAfterResolution],
                Actor::ActingMinion,
                &[Requirement::ActionSuccessful],
                &[Effect::UnlockActor],
            )
        },
        ModeDef {
            limit: Some(Limit::PerTurn),
            ..mode_of(
                &[Window::ActionStealthIntercept],
                Actor::ActingMinion,
                &[Requirement::StealthNeeded],
                &[Effect::AddStealth(1), Effect::UnlockForBloodIfSuccessful(1)],
            )
        },
    ],
    ..CARD
};

/// "[cel][pro] +1 stealth. [CEL][PRO] Only usable after resolution of a
/// successful action. Unlock this vampire. A vampire can play only one
/// Instantaneous Transformation at superior each turn." The limit is the
/// superior mode's alone. After resolution, after any combat, still during
/// the action (PIB 20150915; LSJ 19981028; ANK 20190425); an unblocked
/// action without effect (LSJ 20070411) or fizzled (ANK 20220218) is
/// successful.
pub static INSTANTANEOUS_TRANSFORMATION: CardDef = CardDef {
    name: "Instantaneous Transformation",
    modes: &[
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::ActingMinion,
            &[Requirement::StealthNeeded],
            &[Effect::AddStealth(1)],
        ),
        ModeDef {
            limit: Some(Limit::PerTurn),
            ..mode_of(
                &[Window::ActionAfterResolution],
                Actor::ActingMinion,
                &[Requirement::ActionSuccessful],
                &[Effect::UnlockActor],
            )
        },
    ],
    ..CARD
};

/// "Put this card on a ready justicar or Inner Circle member and lock
/// them. During this vampire's unlock phase, they burn 5 blood. If they
/// cannot burn the blood, they are burned; otherwise, their controller
/// moves this card to another justicar or Inner Circle member, or burns
/// this card if there are no others." All or nothing on the 5 blood, in
/// torpor too; the move does not lock, may go to the same Methuselah's
/// other one (rules-qa 49.4) or to one in torpor (ANK 20210109). The card
/// stays its player's: the rider fires in the host's controller's unlock
/// phase.
pub static FEAR_OF_MEKHET: CardDef = CardDef {
    name: "Fear of Mekhet",
    modes: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                Requirement::TargetVampire,
                Requirement::ActorReady,
                Requirement::TargetTitleIn(&[TitleClass::Justicar, TitleClass::InnerCircle]),
            ],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Methuselah,
            &[],
            &[Effect::PutOnTarget, Effect::LockTarget],
        )
    }],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Every,
        requires: &[Requirement::HostControllerActive],
        effects: &[Effect::HostBurnsBloodOrBurnedThenMoves(5)],
    }],
    ..CARD
};

/// "Unique. Requires a vampire with capacity 10 or more. Only usable after
/// resolution of a successful bleed against your prey. Put this card on
/// this vampire. This vampire gets +1 bleed. During any Methuselah's minion
/// phase, this vampire can lock this card to take an action." After
/// resolution, after any combat, still during the action (PIB 20150915; LSJ
/// 19981028; ANK 20190425); a bleed redirected away from the prey is not
/// against them. The action: offered only as another Methuselah declares
/// the end of their minion phase; taken, the impulse returns to them and
/// their phase goes on (rules-qa 49.6) — in its own Methuselah's phase the
/// vampire acts as usual (50.1), and the Cog is offered only while the
/// vampire has an action to take (50.2); a redirected bleed is not against
/// the prey (50.3). A normal action: ready, unlocked, locking. An
/// action card canceled as it is played leaves the Cog locked and the
/// action to take again, or to let go (LSJ 20081213-1; ANK 20210124); an
/// action canceled later was taken.
pub static ENKIL_COG: CardDef = CardDef {
    name: "Enkil Cog",
    modes: &[mode_of(
        &[Window::ActionAfterResolution],
        Actor::ActingMinion,
        &[
            Requirement::ActorCapacityAtLeast(10),
            Requirement::ActionSuccessful,
            Requirement::BleedAgainstPrey,
        ],
        &[
            Effect::PutOnActor,
            Effect::Layer(Modifier::Bearer {
                stat: Stat::Bleed,
                n: 1,
                during: &[],
                who: Bearer::Any,
            }),
        ],
    )],
    abilities: &[mode_of(
        &[Window::OthersMinionPhaseEnd],
        Actor::Host,
        &[
            Requirement::CardUnlocked,
            Requirement::HostReady,
            Requirement::HostUnlocked,
        ],
        &[Effect::LockSelf, Effect::TakeAnyAction],
    )],
    ..CARD
};

// ---- The Endless Dance (30th Anniversary) ---------------------------------
//
// The deck's new cards (owner, 2026-10-06), a third appended batch
// (`ADDED_BATCHES`); readings in rules-qa Batch 51.

/// "Requires a titled vampire. Choose an equipment and a vampire.
/// Successful referendum means the chosen vampire takes control of the
/// chosen equipment." Not playable with no equipment or vampire to choose
/// (LSJ 20100129; ANK 20191228). The equipment moves onto the vampire if it
/// can hold it (rules-qa 51.3).
static CONFISCATION_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[
        Term::ChooseCard(CardFilter::Type(CardType::Equipment)),
        Term::ChooseMinions {
            filter: &[Requirement::TargetVampire],
            min: 1,
            max: 1,
        },
    ],
    effect: RefEffect::ChosenVampireTakesChosenCard,
    grants: &[],
    non_anarch_titles_minus: false,
};
political!(
    CONFISCATION,
    "Confiscation",
    "\"Requires a titled vampire. Choose an equipment and a vampire. \
     Successful referendum means the chosen vampire takes control of the \
     chosen equipment.\" Not playable without an equipment and a vampire to \
     choose (LSJ 20100129; ANK 20191228); the equipment moves onto the \
     vampire if it can hold it (rules-qa 51.3).",
    &[Requirement::ActorTitleIn(&[TitleClass::Any])],
    CONFISCATION_REFERENDUM
);

static REINFORCEMENTS_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::ChooseSeats {
        min: 1,
        max: u8::MAX,
    }],
    effect: RefEffect::ChosenShuffleFromAsh(3),
    grants: &[],
    non_anarch_titles_minus: false,
};
/// "Requires a titled vampire. Choose one or more Methuselahs. Successful
/// referendum means each chosen Methuselah chooses up to three library
/// cards in their ash heap and shuffles them into their library. Remove
/// this card from the game, even if the action or the referendum fails or
/// is canceled." The picks one at a time, by card (Ashur Tablets'); not
/// returned to hand by Delaying Tactics (rules-qa 51.6).
pub static REINFORCEMENTS: CardDef = CardDef {
    name: "Reinforcements",
    modes: &[political(
        &[Requirement::ActorTitleIn(&[TitleClass::Any])],
        &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::CallReferendum(&REINFORCEMENTS_REFERENDUM),
            Effect::RemovedOncePlayed,
        ],
    )],
    ..CARD
};

/// "Only usable during the polling step of a political action. [pre] This
/// vampire gets +X+1 votes. [PRE] This vampire gets +2X+1 votes." (X
/// blood.) X may be 0, not less (LSJ 20030502).
pub static AWE: CardDef = CardDef {
    name: "Awe",
    modes: &[
        ModeDef {
            terms: Some(Terms::XBlood),
            ..mode_of(
                &[Window::Referendum],
                Actor::ActingMinion,
                &[Requirement::PollingStep],
                &[Effect::AddVotesXLinear { mul: 1, plus: 1 }],
            )
        },
        ModeDef {
            terms: Some(Terms::XBlood),
            ..mode_of(
                &[Window::Referendum],
                Actor::ActingMinion,
                &[Requirement::PollingStep],
                &[Effect::AddVotesXLinear { mul: 2, plus: 1 }],
            )
        },
    ],
    ..CARD
};

/// "[aus][pre] Burn 1 blood to cancel a strike card (the minion chooses a
/// strike again) or a grapple card as it is played by the opposing minion,
/// and its cost is not paid. A vampire can play only one Charismatic Aura
/// at basic each round. [AUS][PRE] Strike: combat ends." The cancel follows
/// the cancel rules (LSJ 20061207; LSJ 20080630; ANK 20190104; LSJ
/// 20030224); the blood is an effect, not reduced, and must be there to
/// play it (ANK 20210226).
pub static CHARISMATIC_AURA: CardDef = CardDef {
    name: "Charismatic Aura",
    modes: &[
        ModeDef {
            limit: Some(Limit::PerRound),
            ..combat(
                &[Window::CardAsPlayed],
                &[
                    Requirement::PlayedStrikeOrGrappleByOpponent,
                    Requirement::ActorBloodAtLeast(1),
                ],
                &[
                    Effect::ActorBurnsBlood(1),
                    Effect::CancelPlayed { refund: true },
                ],
            )
        },
        combat(
            &[Window::CombatStrikeDeclaration],
            &[],
            &[Effect::Strike(StrikeKind::CombatEnds { unlock: false })],
        ),
    ],
    ..CARD
};

/// "Only one Loup can be played each turn. Put this card in play. Toreador
/// get +1 stealth. Burn this card at the end of the turn or after a
/// Toreador performs a successful action." (Toreador.) Played by an acting
/// Toreador, even if stealth is not yet needed (LSJ 20011214-2); every
/// Toreador's stealth, any Methuselah's; burned after any Toreador's
/// successful action (rules-qa 51.7).
pub static LOUP: CardDef = CardDef {
    name: "Loup",
    modes: &[ModeDef {
        waives_needed: true,
        ..mode_of(
            &[Window::ActionStealthIntercept],
            Actor::ActingMinion,
            &[Requirement::NotPlayedThisTurn],
            &[
                Effect::PutInPlay,
                Effect::Layer(Modifier::ClanStat {
                    clan: "Toreador",
                    stat: Stat::Stealth,
                    n: 1,
                    own: false,
                    titled: false,
                    during: &[],
                }),
            ],
        )
    }],
    riders: &[
        Rider {
            hook: Hook::EndOfTurn,
            scope: RiderScope::Every,
            requires: &[],
            effects: &[Effect::BurnSelf],
        },
        Rider {
            hook: Hook::ClanActionSuccessful("Toreador"),
            scope: RiderScope::Every,
            requires: &[],
            effects: &[Effect::BurnSelf],
        },
    ],
    ..CARD
};

/// "Unique location. You can lock this location to lock a Toreador. If you
/// control the Prince of Paris, you can lock this location to lock a
/// minion. Minions can steal this location as a Ⓓ action." (Toreador.) Its
/// controller's, holding the impulse (ANK 20210608: any time — between two
/// actions of another's turn is not an impulse the engine gives, rules-qa
/// 51.4); a minion attempting to block locked by it fails its block (LSJ
/// 20010702).
pub static THE_LOUVRE: CardDef = CardDef {
    name: "Louvre, Paris, The",
    modes: &[LOCATION],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[Requirement::TargetClan("Toreador"), Requirement::ActorReady],
            ),
            ..mode_of(
                &[Window::AnyImpulse],
                Actor::Controller,
                &[Requirement::CardUnlocked],
                &[Effect::LockSelf, Effect::LockTarget],
            )
        },
        ModeDef {
            target: target(TargetKind::Minion, &[Requirement::ActorReady]),
            ..mode_of(
                &[Window::AnyImpulse],
                Actor::Controller,
                &[
                    Requirement::CardUnlocked,
                    Requirement::ControlsPrinceOf("Paris"),
                ],
                &[Effect::LockSelf, Effect::LockTarget],
            )
        },
        steal_this(&[Requirement::OthersControlThisCard]),
    ],
    ..CARD
};

/// "Unique. Requires a justicar or Inner Circle member. Put this card in
/// play with 4 counters. During a political action performed by a
/// Camarilla vampire you control or its polling step, you can burn 1
/// counter from this card to give the acting vampire +1 stealth or to give
/// each non-Camarilla vampire -1 vote. If this card has no counters, burn
/// it." The stealth when needed; the vote penalty reaches votes already
/// cast, a vampire's votes stopping at 0 (ANK 20251125; rules-qa 51.2).
pub static CAMARILLA_CONCLAVE: CardDef = CardDef {
    name: "Camarilla Conclave",
    modes: &[master_in_play(
        &[Requirement::ControlsReadyTitled(&[
            TitleClass::Justicar,
            TitleClass::InnerCircle,
        ])],
        &[Effect::PutInPlay, Effect::EnterWithCounters(4)],
    )],
    abilities: &[
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::Controller,
            &[
                Requirement::CountersAtLeast(1),
                Requirement::IsPolitical,
                Requirement::OtherActingControlled,
                Requirement::ActingVampire,
                Requirement::ActingSectIn(&[Sect::Camarilla]),
                Requirement::StealthNeeded,
            ],
            &[
                Effect::RemoveCounter(1),
                Effect::AddStealth(1),
                Effect::BurnSelfWithoutCounters,
            ],
        ),
        mode_of(
            &[Window::Referendum],
            Actor::Controller,
            &[
                Requirement::CountersAtLeast(1),
                Requirement::PollingStep,
                Requirement::OtherActingControlled,
                Requirement::ActingVampire,
                Requirement::ActingSectIn(&[Sect::Camarilla]),
            ],
            &[
                Effect::RemoveCounter(1),
                Effect::NonCamarillaVotesMinus(1),
                Effect::BurnSelfWithoutCounters,
            ],
        ),
    ],
    ..CARD
};

/// "Only usable during the polling step of a political action. [pre]
/// Methuselahs can burn pool to get votes during the resolution of this
/// card (resolve this card before moving on). A Methuselah gets +2 votes
/// for each pool they burn. [PRE] As above, but you get +3 votes for each
/// pool you burn." A round of its own as it resolves, from its player
/// clockwise: one pool at a time, on either side, give and take until all
/// pass in succession; nothing left for the rest of the action (RTR
/// 19960530); after the terms are set (RTR 20040501). One pool a decision,
/// either side; the last pool ousts (rules-qa 51.5).
pub static BUSINESS_PRESSURE: CardDef = CardDef {
    name: "Business Pressure",
    modes: &[
        mode_of(
            &[Window::Referendum],
            Actor::ActingMinion,
            &[Requirement::PollingStep],
            &[Effect::PoolForVotesRound { per: 2, own: 2 }],
        ),
        mode_of(
            &[Window::Referendum],
            Actor::ActingMinion,
            &[Requirement::PollingStep],
            &[Effect::PoolForVotesRound { per: 2, own: 3 }],
        ),
    ],
    ..CARD
};
