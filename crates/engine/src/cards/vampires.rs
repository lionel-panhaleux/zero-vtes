//! Printed text of vampires, keyed by printed name like the library
//! `defs`. The crypt data (`crate::crypt`) stays generated; what the text
//! does is written here: plain modifiers read live by
//! `GameState::printed_mod`, and abilities in the card vocabulary (the
//! actor `Host` is the vampire itself). A vampire may land with part of
//! its text — every vampire ends with all of it (rules-qa 13.1) — the rest
//! quoted in `incomplete`.

use super::{
    bleed, mode_of, target, Actor, CardFilter, Effect, Hook, Limit, ModeDef, Per, PlaysAs, Printed,
    PrintedTo, RefEffect, ReferendumDef, Requirement as R, Rider, RiderScope, Stat, StrikeKind,
    TargetKind, TargetSpec, Term, Terms, TitleClass, VampireDef, Window, VAMP,
};
use crate::crypt::{Sect, Title};
use crate::decision::ActionKind;
use crate::library::{CardType, Label};

const fn own(stat: Stat, n: i8, requires: &'static [R]) -> Printed {
    Printed {
        stat,
        n,
        to: PrintedTo::Itself,
        per: Per::One,
        requires,
    }
}

/// An ability with a limit, no target and no terms.
pub(super) const fn ability(
    windows: &'static [Window],
    actor: Actor,
    requires: &'static [R],
    limit: Limit,
    effects: &'static [Effect],
) -> ModeDef {
    ModeDef {
        windows,
        actor,
        requires,
        target: None,
        terms: None,
        waives_needed: false,
        limit: Some(limit),
        effects,
    }
}

/// A printed action ("… as a +1 stealth action"): the vampire's own,
/// declared in the minion phase, its effects resolving on success.
pub(super) const fn printed_action(
    target: Option<TargetSpec>,
    effects: &'static [Effect],
) -> ModeDef {
    ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::Host,
        requires: &[],
        target,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// "… can enter combat with <filter> as a Ⓓ action": another Methuselah's
/// ready minion, the combat part of the action (spec §4.1.2).
pub(super) const fn enter_combat(filter: &'static [R]) -> ModeDef {
    printed_action(
        Some(TargetSpec {
            kind: TargetKind::Minion,
            filter,
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::EnterCombatWithTarget,
        ],
    )
}

/// "… can steal <filter> equipment as a Ⓓ action": moved to the vampire on
/// success, if it could still hold it (like an equip from another minion),
/// directed at the equipment's controller, never its own Methuselah's
/// (rules-qa 90.4).
const fn steal_equipment(filter: &'static [R], effects: &'static [Effect]) -> ModeDef {
    printed_action(
        Some(TargetSpec {
            kind: TargetKind::Card,
            filter,
        }),
        effects,
    )
}

/// "Gets 1 optional maneuver … each combat" (Abaddon, Saku Pihlajamäki).
pub(super) const MANEUVER_EACH_COMBAT: ModeDef = ability(
    &[Window::CombatDetermineRange],
    Actor::Host,
    &[],
    Limit::PerCombat,
    &[Effect::Maneuver],
);

/// "Once each combat, … can strike: dodge" (Flávio Gonçalves).
pub(super) const DODGE_EACH_COMBAT: ModeDef = ability(
    &[Window::CombatStrikeDeclaration],
    Actor::Host,
    &[],
    Limit::PerCombat,
    &[Effect::Strike(StrikeKind::Dodge)],
);

pub static ABADDON: VampireDef = VampireDef {
    name: "Abaddon",
    abilities: &[
        MANEUVER_EACH_COMBAT,
        ability(
            &[Window::CombatPress],
            Actor::Host,
            &[],
            Limit::PerCombat,
            &[Effect::Press],
        ),
    ],
    ..VAMP
};

/// "During an action, Abraham can discard a card requiring Blood Sorcery
/// [tha] to get +1 intercept or +1 stealth": stealth on his action, intercept
/// on his block attempt, each only when needed (rules-qa 3.2), once each
/// action (rules-qa 6.6).
pub static ABRAHAM_DUSABLE: VampireDef = VampireDef {
    name: "Abraham DuSable",
    abilities: &[
        ModeDef {
            terms: Some(Terms::Discard {
                n: 1,
                of: CardFilter::Discipline("tha"),
            }),
            limit: Some(Limit::PerAction),
            ..mode_of(
                &[Window::ActionDuring],
                Actor::Host,
                &[
                    R::ActorActing,
                    R::StealthNeeded,
                    R::HandHas(CardFilter::Discipline("tha"), 1),
                ],
                &[Effect::AddStealth(1)],
            )
        },
        ModeDef {
            terms: Some(Terms::Discard {
                n: 1,
                of: CardFilter::Discipline("tha"),
            }),
            limit: Some(Limit::PerAction),
            ..mode_of(
                &[Window::ActionStealthIntercept],
                Actor::Host,
                &[
                    R::InterceptNeeded,
                    R::HandHas(CardFilter::Discipline("tha"), 1),
                ],
                &[Effect::AddIntercept(1)],
            )
        },
    ],
    ..VAMP
};

/// "Abraham can play cards requiring a Tremere as if he were a Tremere.
/// While he is ready, you get +1 hand size." Cards he plays, played
/// normally (LSJ 20050119), not his Methuselah's: no minion card in the
/// engine pool requires a Tremere (rules-qa 41.1).
pub static ABRAHAM_MELLON: VampireDef = VampireDef {
    name: "Abraham Mellon",
    printed: &[own(Stat::HandSize, 1, &[R::ThisVampireReady])],
    plays_as: Some(PlaysAs {
        clan: "Tremere",
        methuselah: false,
    }),
    ..VAMP
};

/// "Rescuing an older vampire from torpor costs Adelino -2 blood": the
/// action's cost, whoever pays it (PIB 20110918, LSJ 20020304-2); older:
/// of higher capacity (rules-qa 41.14).
pub static ADELINO_DA_SILVA: VampireDef = VampireDef {
    name: "Adelino Da Silva",
    printed: &[own(Stat::RescueDiscount, 2, &[R::OpponentOlder])],
    ..VAMP
};

/// "Adisa can enter combat with an ally or younger vampire as a Ⓓ action. If
/// she successfully hunts, she does not unlock as normal during her next
/// unlock phase." A successful hunt gets its blood, at capacity too
/// (owner, rules-qa 89.26).
pub static ADISA: VampireDef = VampireDef {
    name: "Adisa",
    abilities: &[enter_combat(&[
        R::NotControlled,
        R::ActorReady,
        R::AllyOrYounger,
    ])],
    riders: &[Rider {
        hook: Hook::SuccessfulHunt,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::SkipNextUnlock],
    }],
    ..VAMP
};

/// "Adrino gets 1 press (mandatory) each combat, only usable to continue
/// combat": he must use it before passing the press step, unless a press to
/// continue already stands — it then waits for a later round (rules-qa
/// 14.1).
pub static ADRINO_MANAUARA: VampireDef = VampireDef {
    name: "Adrino Manauara",
    printed: &[own(Stat::MandatoryContinuePress, 1, &[])],
    ..VAMP
};

pub static AISHA_AZ_ZAHRA: VampireDef = VampireDef {
    name: "Aisha az-Zahra",
    printed: &[own(Stat::Bleed, 1, &[])],
    // A referendum ability: once per referendum, while locked or in torpor
    // (ANK 20220705; advanced rules §3.7.6.4).
    abilities: &[ability(
        &[Window::Referendum],
        Actor::Host,
        &[R::AnyPollingStep, R::HostBloodAtLeast(1)],
        Limit::PerReferendum,
        &[
            Effect::HostBurnBlood(1),
            Effect::AddVotesPerReadySect(Sect::Anarch),
        ],
    )],
    ..VAMP
};

/// "If Alabástrom is ready during your turn, you can unlock another ready
/// Gargoyle you control": once that turn (rules-qa 15.2). Flight is a bare
/// trait — no card in the engine pool requires it (rules-qa 16.6).
pub static ALABASTROM: VampireDef = VampireDef {
    name: "Alabástrom",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                R::Controlled,
                R::ActorReady,
                R::ActorClan("Gargoyle"),
                R::NotHost,
            ],
        ),
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[R::OwnTurn, R::HostReady],
            &[Effect::UnlockTarget],
        )
    }],
    ..VAMP
};

pub static ALEXA_DRAPER: VampireDef = VampireDef {
    name: "Alexa Draper",
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Discipline("dom"),
        }),
        limit: Some(Limit::PerReferendum),
        ..mode_of(
            &[Window::Referendum],
            Actor::Host,
            &[
                R::AnyPollingStep,
                R::HandHas(CardFilter::Discipline("dom"), 1),
            ],
            &[Effect::AddVotes(1)],
        )
    }],
    ..VAMP
};

/// "Vampires must burn 1 blood to cast votes and ballots against
/// referendums called by Alexander": each vampire casting against his
/// political action's referendum, himself included; a Methuselah's ballots
/// are not a vampire's (rules-qa 41.3).
pub static ALEXANDER_SILVERSON: VampireDef = VampireDef {
    name: "Alexander Silverson",
    printed: &[Printed {
        stat: Stat::VoteAgainstToll,
        n: 1,
        to: PrintedTo::Opponent,
        per: Per::One,
        requires: &[],
    }],
    ..VAMP
};

/// "After Alicia performs a successful directed action, you can discard a
/// political action card to lock a younger vampire controlled by (one of)
/// the target Methuselah(s)." Once per action (rules-qa 16.1).
pub static ALICIA_CORTEZ: VampireDef = VampireDef {
    name: "Alicia Cortez",
    abilities: &[ModeDef {
        windows: &[Window::ActionAfterResolution],
        actor: Actor::Host,
        requires: &[
            R::ActorActing,
            R::ActionDirected,
            R::ActionSuccessful,
            R::HandHas(CardFilter::Type(CardType::PoliticalAction), 1),
        ],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            // A locked vampire is a legal target with no effect: kept for
            // the cycling opportunity (rules-qa 16.4).
            filter: &[R::Younger, R::ControlledByDirectedSeat],
        }),
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Type(CardType::PoliticalAction),
        }),
        waives_needed: false,
        limit: Some(Limit::PerAction),
        effects: &[Effect::LockTarget],
    }],
    ..VAMP
};

/// "Once each turn, Aline can burn 1 blood to unlock after another Anarch
/// you control performs a successful action." In torpor too (LSJ 20011022).
pub static ALINE_GADEKE: VampireDef = VampireDef {
    name: "Aline Gädeke",
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::ActionAfterResolution],
            Actor::Host,
            &[
                R::OtherActingControlled,
                R::ActingSectIn(&[Sect::Anarch]),
                R::ActionSuccessful,
                R::HostLocked,
                R::HostBloodAtLeast(1),
            ],
            &[Effect::HostBurnBlood(1), Effect::UnlockActor],
        )
    }],
    ..VAMP
};

pub static ANDRE_THE_MANIPULATOR: VampireDef = VampireDef {
    name: "André the Manipulator",
    printed: &[own(Stat::HandSize, 2, &[R::ThisVampireReady])],
    ..VAMP
};

pub static ARIANE: VampireDef = VampireDef {
    name: "Ariane",
    printed: &[own(Stat::Stealth, -1, &[R::ActionUndirected])],
    ..VAMP
};

/// "Aniel can burn 1 corruption counter or a card requiring a Discipline
/// from another ready minion as a +1 stealth Ⓓ action that costs 1
/// blood." One printed action on two kinds of target (one NRA identity,
/// `Effect::SameActionAs`): a card, directed at its controller, or a
/// minion with any corruption counters, directed at its controller;
/// undirected on Aniel's own Methuselah's (rulebook "Directed and
/// Undirected Actions": Ⓓ is a reminder).
pub static ANIEL: VampireDef = VampireDef {
    name: "Aniel",
    abilities: &[
        printed_action(
            target(
                TargetKind::Card,
                &[
                    R::TargetCard(CardFilter::RequiresDiscipline),
                    R::TargetOnReadyMinion,
                    R::NotHost,
                ],
            ),
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::PayBlood(1),
                Effect::BurnTargetCard,
            ],
        ),
        printed_action(
            target(
                TargetKind::Minion,
                &[R::TargetCorrupted, R::ActorReady, R::OtherThanActor],
            ),
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::PayBlood(1),
                Effect::BurnCorruptionFromTarget,
                Effect::SameActionAs(0),
            ],
        ),
    ],
    ..VAMP
};

/// "Equipment cost Anne-Marie -1 pool": equipment she equips, from hand or
/// put on her by another card (Contraband, a search; rules-qa 36.5, 41.13).
pub static ANNE_MARIE_BOURGEOIS: VampireDef = VampireDef {
    name: "Anne-Marie Bourgeois, Inconnu Recruiter",
    printed: &[own(
        Stat::PoolCostOff,
        1,
        &[R::PlayedCard(CardFilter::Type(CardType::Equipment))],
    )],
    ..VAMP
};

/// "Apolonia can steal 1 blood from a younger vampire as a Ⓓ action." Any
/// younger vampire, one in torpor or with no blood included (rulebook
/// "Target"); undirected on her own Methuselah's.
pub static APOLONIA_CZARNECKI: VampireDef = VampireDef {
    name: "Apolonia Czarnecki",
    abilities: &[printed_action(
        target(TargetKind::Minion, &[R::TargetVampire, R::Younger]),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::StealBloodFromTarget(1),
        ],
    )],
    ..VAMP
};

/// "If Ayse is ready, you can use a discard phase action to add 1 blood to
/// her": each use spends one, not once per phase; not offered at capacity
/// (as the blood shuttles of cards in play).
pub static AYSE_DHANIAL: VampireDef = VampireDef {
    name: "Ayse Dhanial",
    abilities: &[mode_of(
        &[Window::DiscardPhaseAction],
        Actor::Host,
        &[
            R::HostReady,
            R::DiscardPhaseActionLeft,
            R::HostBelowCapacity,
        ],
        &[Effect::SpendDiscardPhaseAction, Effect::AddHostBlood(1)],
    )],
    ..VAMP
};

pub static AZUCENA: VampireDef = VampireDef {
    name: "Azucena",
    printed: &[
        own(
            Stat::Intercept,
            1,
            &[R::OpponentClan("Lasombra"), R::OpponentYounger],
        ),
        own(
            Stat::Intercept,
            -1,
            &[R::OpponentClan("Lasombra"), R::OpponentOlder],
        ),
    ],
    ..VAMP
};

/// "Barachiel can enter combat with a minion as a Ⓓ action."
pub static BARACHIEL: VampireDef = VampireDef {
    name: "Barachiel",
    abilities: &[enter_combat(&[R::NotControlled, R::ActorReady])],
    ..VAMP
};

pub static BRANIMIRA: VampireDef = VampireDef {
    name: "Branimira",
    printed: &[own(Stat::Intercept, 1, &[R::PoolAtMost(9)])],
    ..VAMP
};

pub static BRET_STRYKER: VampireDef = VampireDef {
    name: "Bret Stryker",
    printed: &[own(Stat::Intercept, -1, &[R::OpponentTitled])],
    ..VAMP
};

pub static CARMELITA_NEILLSON: VampireDef = VampireDef {
    name: "Carmelita Neillson",
    printed: &[own(Stat::HandSize, 1, &[R::ThisVampireReady])],
    ..VAMP
};

pub static CASTELLAN: VampireDef = VampireDef {
    name: "Castellan",
    printed: &[
        own(Stat::Intercept, 1, &[R::ActionDirected]),
        own(Stat::Intercept, -1, &[R::ActionUndirected]),
    ],
    ..VAMP
};

pub static CATALINA_VEGA: VampireDef = VampireDef {
    name: "Catalina Vega",
    printed: &[own(Stat::Bleed, 1, &[])],
    ..VAMP
};

/// "If the referendum of a political action called by Cedrick is canceled
/// or fails, he goes to torpor after resolution."
pub static CEDRICK_CALHOUN: VampireDef = VampireDef {
    name: "Cedrick Calhoun",
    riders: &[Rider {
        hook: Hook::OwnReferendumFailed,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::GoToTorpor],
    }],
    ..VAMP
};

/// "Once each combat, Chun Hei can burn 1 blood to get 1 additional strike
/// that does not count against the limit. +1 strength."
pub static CHUN_HEI: VampireDef = VampireDef {
    name: "Chun Hei",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[ability(
        &[Window::CombatAdditionalStrike],
        Actor::Host,
        &[R::HostBloodAtLeast(1)],
        Limit::PerCombat,
        &[
            Effect::HostBurnBlood(1),
            Effect::AdditionalStrikes {
                n: 1,
                limited: false,
            },
        ],
    )],
    ..VAMP
};

/// "Dafina can enter combat with a minion as a Ⓓ action."
pub static DAFINA_HANGANU: VampireDef = VampireDef {
    name: "Dafina Hanganu",
    abilities: &[enter_combat(&[R::NotControlled, R::ActorReady])],
    ..VAMP
};

/// "Daria can lock to give another Nosferatu +1 stealth. Directed actions
/// cost her +1 blood." The stealth: the acting minion's, when needed
/// (rules-qa 3.2), any Methuselah's; the lock cost makes it once per unlock
/// (advanced rules §1.5.4), in torpor too (LSJ 20011022). The blood: the
/// action's cost — with the action's own blood cost, payable as it is
/// announced; paid first as it resolves, unpaid if it is blocked; unpayable
/// by then, the action fizzles (rules-qa 3.6, 42.1, 42.6).
pub static DARIA: VampireDef = VampireDef {
    name: "Daria",
    abilities: &[mode_of(
        &[Window::ActionDuring, Window::ReactionDuringAction],
        Actor::Host,
        &[
            R::HostUnlocked,
            R::OtherActingClan("Nosferatu"),
            R::StealthNeeded,
        ],
        &[Effect::LockActor, Effect::AddStealth(1)],
    )],
    printed: &[own(Stat::ActionCost, 1, &[R::ActionDirected])],
    ..VAMP
};

/// "Darius can steal a non-electronic, non-vehicle, non-weapon equipment
/// as a Ⓓ action." No electronic equipment is in the V5 pool.
pub static DARIUS_SHIRAZI: VampireDef = VampireDef {
    name: "Darius Shirazi",
    abilities: &[steal_equipment(
        &[
            R::TargetCard(CardFilter::EquipmentWithout(&[
                Label::Electronic,
                Label::Vehicle,
                Label::Weapon,
            ])),
            R::NotHost,
            R::NotControlled,
            R::ActorMayHoldTarget,
        ],
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::StealTargetCard,
        ],
    )],
    ..VAMP
};

/// "In combat, strike cards cost the opposing minion +1 blood or life. +1
/// stealth." A card played in a strike mode (a dodge included) by the
/// minion in combat with her, whether she acts or blocks — never her own;
/// an ally pays in life, its last one too: it burns and the combat ends
/// (rules-qa 41.4, 42.2; ANK 20180928-1). Read live.
pub static DJENEBA: VampireDef = VampireDef {
    name: "Djeneba",
    printed: &[
        Printed {
            stat: Stat::StrikeCardToll,
            n: 1,
            to: PrintedTo::Opponent,
            per: Per::One,
            requires: &[],
        },
        own(Stat::Stealth, 1, &[]),
    ],
    ..VAMP
};

/// "Rescuing a vampire from torpor costs Doc Martina -1 blood": the
/// action's cost, whoever pays it (PIB 20110918, LSJ 20020304-2).
pub static DOC_MARTINA: VampireDef = VampireDef {
    name: "Doc Martina",
    printed: &[own(Stat::RescueDiscount, 1, &[])],
    ..VAMP
};

/// "Dominica can search your library for a master archetype card, reveal it
/// and move it to your hand as a +1 stealth action."
pub static DOMINICA: VampireDef = VampireDef {
    name: "Dominica",
    abilities: &[printed_action(
        None,
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::SearchToHand {
                filter: CardFilter::Labeled(CardType::Master, Label::Archetype),
                shown: true,
            },
        ],
    )],
    ..VAMP
};

pub static DOWAGER_THE: VampireDef = VampireDef {
    name: "Dowager, The",
    printed: &[own(Stat::Intercept, 1, &[R::ActionDirected])],
    ..VAMP
};

pub static EGIDIA_ARRU: VampireDef = VampireDef {
    name: "Egidia Arrú",
    printed: &[own(Stat::Strength, 1, &[])],
    // "After a minion in combat with Egidia leaves the ready region, their
    // controller burns 1 pool."
    riders: &[Rider {
        hook: Hook::CombatOpponentLeftReady,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::TheirControllerBurnsPool(1)],
    }],
    ..VAMP
};

/// "If you control a locked minion, Elen must bleed with +1 bleed as a Ⓓ
/// action unless she must hunt." Her own action, mandatory while the
/// condition holds; a bleed of another source does not discharge it, and
/// having bled before it attached she is stuck (spec §3.9, ANK 20211009-2).
pub static ELEN_KAMJIAN: VampireDef = VampireDef {
    name: "Elen Kamjian",
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::Host,
        &[R::ControlsLockedMinion],
        &[
            Effect::DeclareAction(ActionKind::Bleed),
            Effect::MandatoryAction,
            bleed(1, false),
        ],
    )],
    ..VAMP
};

/// "As a minion announces an action directed at Evan, flip a coin; if it is
/// tails, the action fails." An action of his controller's own minions is
/// undirected, so it is not flipped (LSJ 20030214-2).
pub static EVAN_KLEIN: VampireDef = VampireDef {
    name: "Evan Klein",
    riders: &[Rider {
        hook: Hook::ActionAnnouncedAtTarget,
        scope: RiderScope::Every,
        requires: &[R::ActingOtherController],
        effects: &[Effect::CoinFlipActionFails],
    }],
    ..VAMP
};

/// "Minions with any of your corruption counters in combat with Faruq
/// cannot strike: combat ends. +1 strength."
pub static FARUQ_ABD_AL_QADIR: VampireDef = VampireDef {
    name: "Faruq Abd al-Qadir",
    printed: &[
        own(Stat::Strength, 1, &[]),
        Printed {
            stat: Stat::NoCombatEnds,
            n: 1,
            to: PrintedTo::Opponent,
            per: Per::One,
            requires: &[R::ActorCorruptedBySource],
        },
    ],
    ..VAMP
};

pub static FLAVIO_GONCALVES: VampireDef = VampireDef {
    name: "Flávio Gonçalves",
    abilities: &[DODGE_EACH_COMBAT],
    ..VAMP
};

/// "During an action Gathii performs, you can reveal the top card of your
/// library. If it is a master card, he burns 1 blood; otherwise, he gets +1
/// stealth": once per action (rules-qa 15.1), usable before stealth is
/// needed, the stealth then not gained (ANK 20231031).
pub static GATHII: VampireDef = VampireDef {
    name: "Gathii",
    abilities: &[ModeDef {
        waives_needed: true,
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionDuring],
            Actor::Host,
            &[R::ActorActing],
            &[Effect::RevealTopCard {
                burn: 1,
                stealth: 1,
            }],
        )
    }],
    ..VAMP
};

/// "If Aemilius is ready during your unlock phase, your prey chooses a ready
/// minion they control; the chosen minion takes 1 unpreventable damage": a
/// mandatory unlock-phase effect his controller orders among theirs
/// (rules-qa 7.8), his readiness read then too; the minion chosen as it
/// resolves (rules-qa 41.11).
pub static GNAEUS_AEMILIUS_AUGUSTINUS: VampireDef = VampireDef {
    name: "Gnaeus Aemilius Augustinus",
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Controller,
        requires: &[R::HostReady],
        effects: &[Effect::PreyChoosesMinionDamaged(1)],
    }],
    ..VAMP
};

/// "The Guardian cannot perform directed actions unless you control a ready
/// unique older Nosferatu. The Guardian can lock after resolution of a
/// successful bleed against you to enter combat with the acting minion."
/// The acting minion acts first in that combat (ANK 20221102); it returns
/// to the after-resolution window. A created vampire is not unique
/// (rules-qa 41.5).
pub static GUARDIAN_THE: VampireDef = VampireDef {
    name: "Guardian, The",
    printed: &[own(
        Stat::NoDirectedActions,
        1,
        &[R::ControlsNoReadyOlderOfClan("Nosferatu")],
    )],
    abilities: &[mode_of(
        &[Window::ActionAfterResolution],
        Actor::Host,
        &[
            R::HostReady,
            R::HostUnlocked,
            R::IsBleed,
            R::DirectedAtYou,
            R::BleedSuccessful,
        ],
        &[Effect::LockActor, Effect::EnterCombatWithActing],
    )],
    ..VAMP
};

pub static GWENLLIAN_ARWYN: VampireDef = VampireDef {
    name: "Gwenllian Arwyn",
    printed: &[own(Stat::Strength, 1, &[])],
    // The minion attempting to block her action, while stealth is needed,
    // for the rest of the action; once per action (rules-qa 15.2).
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionDuring],
            Actor::Host,
            &[R::ActorActing, R::StealthNeeded, R::HostBloodAtLeast(1)],
            &[Effect::HostBurnBlood(1), Effect::AttemptingIntercept(-1)],
        )
    }],
    ..VAMP
};

pub static HAFTHOR_THORSTEINSSON: VampireDef = VampireDef {
    name: "Hafthor Thorsteinsson",
    printed: &[own(Stat::Intercept, 1, &[])],
    ..VAMP
};

/// An ally requiring Hecata, or printing Oblivion [obl] (Hel-Blá).
const HECATA_OR_OBLIVION: &[CardFilter] =
    &[CardFilter::Clan("Hecata"), CardFilter::Discipline("obl")];

/// "Hel-Blá can move an ally requiring Hecata or Oblivion [obl] from your
/// ash heap to your ready region, locked with life equal to its starting
/// life as a +1 stealth action that costs 1 blood." Put in play, not
/// recruited: no requirement, X = 0, its inferior version, able to act this
/// turn once unlocked (LSJ 20100204, 20040518-2, 20100302-1; RBK
/// recruit-ally). Offered while the ash heap holds one; named as the
/// action is declared, gone by resolution: no effect (rules-qa 41.6).
pub static HEL_BLA: VampireDef = VampireDef {
    name: "Hel-Blá",
    abilities: &[ModeDef {
        requires: &[R::AshHasAlly(HECATA_OR_OBLIVION)],
        ..printed_action(
            None,
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::PayBlood(1),
                Effect::AllyFromAshLocked(HECATA_OR_OBLIVION),
            ],
        )
    }],
    ..VAMP
};

pub static HESHA_RUHADZE: VampireDef = VampireDef {
    name: "Hesha Ruhadze",
    group: Some(6),
    printed: &[Printed {
        stat: Stat::Bleed,
        n: 1,
        to: PrintedTo::Itself,
        per: Per::UniqueEquipmentOn,
        requires: &[],
    }],
    ..VAMP
};

/// "Once each turn, if Ilonka is ready after a successful bleed against you,
/// she can look at the acting minion's controller's hand, then she can
/// discard 1 card at random from it": the count, 0 or 1, is chosen after
/// the look.
pub static ILONKA: VampireDef = VampireDef {
    name: "Ilonka",
    abilities: &[ModeDef {
        terms: Some(Terms::Count { min: 0, max: 1 }),
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::ActionAfterResolution],
            Actor::Host,
            &[
                R::HostReady,
                R::IsBleed,
                R::DirectedAtYou,
                R::BleedSuccessful,
            ],
            &[Effect::ActingControllerDiscardsAtRandom],
        )
    }],
    ..VAMP
};

pub static JASON_SON_NEWBERRY: VampireDef = VampireDef {
    name: "Jason \"Son\" Newberry",
    printed: &[
        own(Stat::Bleed, 1, &[]),
        own(Stat::VotesAgainstBloodHunt, 2, &[]),
    ],
    ..VAMP
};

/// "During your turn, you can burn the Edge to unlock Kalinda": wherever her
/// controller holds the impulse on their turn.
pub static KALINDA: VampireDef = VampireDef {
    name: "Kalinda",
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[R::OwnTurn, R::HasEdge, R::HostLocked],
            &[Effect::BurnEdge, Effect::UnlockActor],
        )
    }],
    ..VAMP
};

pub static KAMILE_PAUKSTYS: VampireDef = VampireDef {
    name: "Kamile Paukstys",
    printed: &[own(Stat::Bleed, 1, &[])],
    ..VAMP
};

/// "Once each combat, Kasim can discard a political action card before
/// range is determined to get +2 strength that combat. +1 bleed."
pub static KASIM_BAYAR: VampireDef = VampireDef {
    name: "Kasim Bayar",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Type(CardType::PoliticalAction),
        }),
        limit: Some(Limit::PerCombat),
        ..mode_of(
            &[Window::CombatBeforeRange],
            Actor::Host,
            &[R::HandHas(CardFilter::Type(CardType::PoliticalAction), 1)],
            &[Effect::StrengthThisCombat(2)],
        )
    }],
    ..VAMP
};

/// "Once each turn, Keegan can unlock after performing a successful action
/// requiring a Gangrel": a fizzled action counts (ANK 20220218); locked,
/// the unlock self-limits like Aline Gädeke's; an action card, a retainer
/// or ally card, or a granted action requiring the clan (rules-qa 41.12).
pub static KEEGAN: VampireDef = VampireDef {
    name: "Keegan",
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::ActionAfterResolution],
            Actor::Host,
            &[
                R::ActorActing,
                R::ActionSuccessful,
                R::ActionRequiresClanOrDiscipline("Gangrel", ""),
                R::HostLocked,
            ],
            &[Effect::UnlockActor],
        )
    }],
    ..VAMP
};

pub static KEVIN_JACKSON: VampireDef = VampireDef {
    name: "Kevin Jackson",
    printed: &[
        own(Stat::Strength, 1, &[R::OpponentClan("Brujah")]),
        Printed {
            stat: Stat::Strength,
            n: 1,
            to: PrintedTo::Opponent,
            per: Per::One,
            requires: &[R::ActorClan("Brujah")],
        },
    ],
    ..VAMP
};

/// "Animals (allies and retainers) cost Kuyén -1 blood or pool": off
/// whichever the card costs, one recruited or employed in a non-standard
/// way too (Piper; ANK 20210913, LSJ 20090116; rules-qa 41.13).
pub static KUYEN: VampireDef = VampireDef {
    name: "Kuyén",
    printed: &[own(
        Stat::CostOff,
        1,
        &[
            R::PlayedCard(CardFilter::AllyOrRetainer),
            R::PlayedCard(CardFilter::HasLabel(Label::Animal)),
        ],
    )],
    ..VAMP
};

/// "You and Lady Scarlett can play cards requiring a Toreador as if she
/// were a Toreador": her own cards (Scorn of Adonis) and her Methuselah's
/// (Art Museum, Toreador Grand Ball — she may be its locked Toreador),
/// played normally (LSJ 20050119); the card's effects in play do not take
/// her for one — chosen by Toreador Grand Ball, she unlocks as normal (ANK
/// 20220816-2, LSJ 20101013; rules-qa 41.2).
pub static LADY_SCARLETT_CHURCHILL: VampireDef = VampireDef {
    name: "Lady Scarlett Churchill",
    plays_as: Some(PlaysAs {
        clan: "Toreador",
        methuselah: true,
    }),
    ..VAMP
};

/// "During a bleed action, Larissa can discard a card requiring Animalism
/// [ani] to get +1 bleed": her own bleed, before resolution (ANK 20231124),
/// once each bleed (rules-qa 6.6).
pub static LARISSA_MOREIRA: VampireDef = VampireDef {
    name: "Larissa Moreira",
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Discipline("ani"),
        }),
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionDuring],
            Actor::Host,
            &[
                R::ActorActing,
                R::IsBleed,
                R::HandHas(CardFilter::Discipline("ani"), 1),
            ],
            &[bleed(1, false)],
        )
    }],
    ..VAMP
};

/// "Lenelle can exchange a card from your hand for a library card in your
/// ash heap as a +1 stealth action that costs 1 blood." Both cards named as
/// the action is announced (LSJ 20000222, LSJ 20011023); exchanged at
/// resolution if both are still there.
pub static LENELLE: VampireDef = VampireDef {
    name: "Lenelle, Mambo of Birmingham",
    abilities: &[ModeDef {
        requires: &[R::HandHas(CardFilter::Any, 1), R::AshNotEmpty],
        ..printed_action(
            None,
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::PayBlood(1),
                Effect::ExchangeHandForAsh,
            ],
        )
    }],
    ..VAMP
};

pub static LENNY_BURKHEAD: VampireDef = VampireDef {
    name: "Lenny Burkhead",
    printed: &[own(Stat::Bleed, 1, &[])],
    ..VAMP
};

/// "During your discard phase, you can move an animal retainer from a
/// vampire you control to another vampire you control": once each discard
/// phase (rules-qa 6.6), in torpor too (LSJ 20011022); the retainer and
/// the vampire it goes to both chosen as the ability is used (rules-qa
/// 41.7).
pub static LUCIANO_CARVALHO: VampireDef = VampireDef {
    name: "Luciano Carvalho",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Card,
            &[
                R::TargetCard(CardFilter::Labeled(CardType::Retainer, Label::Animal)),
                R::TargetOnVampire,
                R::Controlled,
            ],
        ),
        terms: Some(Terms::VampireToHoldTarget),
        ..mode_of(&[Window::DiscardPhase], Actor::Host, &[], &[])
    }],
    ..VAMP
};

/// "Allies and retainers requiring a Tzimisce cost Máddji -1 blood or
/// pool": off whichever the card costs, however it is put in play
/// (rules-qa 36.5, 41.13).
pub static MADDJI: VampireDef = VampireDef {
    name: "Máddji, Mistress of Szlachtas",
    printed: &[own(
        Stat::CostOff,
        1,
        &[
            R::PlayedCard(CardFilter::AllyOrRetainer),
            R::PlayedCard(CardFilter::Clan("Tzimisce")),
        ],
    )],
    ..VAMP
};

/// "During a bleed action, Liliana can remove seven cards in your ash heap
/// from the game to get an additional +1 bleed. +1 bleed." On her own
/// bleed, before resolution, once per action (the "during" template,
/// rules-qa 6.6, 15.2, 42.3, as Larissa Moreira); the seven cards are her
/// Methuselah's picks as she uses it, one at a time (there is no order to
/// the ash heap, LSJ 20090710) — burned vampires included. Not the limited
/// bleed bonus (rules-qa 42.7).
pub static MARCHESA_LILIANA: VampireDef = VampireDef {
    name: "Marchesa Liliana",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionDuring],
            Actor::Host,
            &[R::ActorActing, R::IsBleed, R::AshAtLeast(7)],
            &[Effect::RemoveOwnAshCards(7), bleed(1, false)],
        )
    }],
    ..VAMP
};

/// "If Marialena is blocked, she burns 1 blood before block resolution."
pub static MARIALENA: VampireDef = VampireDef {
    name: "Marialena",
    riders: &[Rider {
        hook: Hook::HostBlocked,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::HostBurnBlood(1)],
    }],
    ..VAMP
};

pub static MARTINA_SRNANKOVA: VampireDef = VampireDef {
    name: "Martina Srnankova",
    printed: &[own(Stat::Intercept, 1, &[R::IsBleed])],
    ..VAMP
};

pub static MASSIMILIANO: VampireDef = VampireDef {
    name: "Massimiliano",
    printed: &[own(Stat::Strength, 1, &[])],
    ..VAMP
};

pub static MICKEY_WHEELER: VampireDef = VampireDef {
    name: "Mickey Wheeler",
    printed: &[own(Stat::Strength, 1, &[R::OpponentOlder])],
    ..VAMP
};

/// "Younger vampires must burn 1 blood to attempt to block Mithras or to
/// cast votes or ballots against a referendum called by him. +2 bleed. +1
/// strength. +2 votes." Tolls on the attempt and on votes against his
/// political action's referendum, read live: a vampire of lower capacity —
/// never an ally, never himself; a Methuselah's ballots are not a vampire's
/// and a blood hunt called on him is not called by him (rules-qa 41.3,
/// 42.4). In torpor too — his leave-torpor action (LSJ 20011022).
pub static MITHRAS: VampireDef = VampireDef {
    name: "Mithras",
    group: Some(6),
    printed: &[
        Printed {
            stat: Stat::BlockToll,
            n: 1,
            to: PrintedTo::Opponent,
            per: Per::One,
            requires: &[R::OpponentOlder],
        },
        Printed {
            stat: Stat::VoteAgainstToll,
            n: 1,
            to: PrintedTo::Opponent,
            per: Per::One,
            requires: &[R::OpponentOlder],
        },
        own(Stat::Bleed, 2, &[]),
        own(Stat::Strength, 1, &[]),
        own(Stat::Votes, 2, &[]),
    ],
    ..VAMP
};

pub static MORA_THE_DEATH_SEER: VampireDef = VampireDef {
    name: "Mora, the Death Seer",
    printed: &[own(Stat::Bleed, 1, &[])],
    // Once per discard phase (a phase-framed clause, advanced rules
    // §1.2.1).
    abilities: &[ModeDef {
        terms: Some(Terms::AshCard),
        ..mode_of(
            &[Window::DiscardPhase],
            Actor::Host,
            &[R::HostReady, R::AshNotEmpty],
            &[Effect::AshCardToLibraryBottom],
        )
    }],
    ..VAMP
};

/// "Once each turn, if she is ready, Anja can burn 1 blood after another
/// Hecata you control performs a successful action requiring Hecata or
/// Oblivion [obl] to unlock that Hecata": a fizzled action counts (ANK
/// 20220218); an acting Hecata unlocked since is a legal no-effect target
/// (rules-qa 4.3, 41.12).
pub static MOTHER_ANJA_GIOVANNI: VampireDef = VampireDef {
    name: "\"Mother\" Anja Giovanni",
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::ActionAfterResolution],
            Actor::Host,
            &[
                R::HostReady,
                R::OtherActingControlled,
                R::ActingClan("Hecata"),
                R::ActionSuccessful,
                R::ActionRequiresClanOrDiscipline("Hecata", "obl"),
                R::HostBloodAtLeast(1),
            ],
            &[Effect::HostBurnBlood(1), Effect::UnlockActing],
        )
    }],
    ..VAMP
};

/// "Nathaniel can enter combat with a locked vampire as a Ⓓ action." A
/// target unlocked by resolution: successful, no effect (ANK 20210124).
pub static NATHANIEL_BORDRUFF: VampireDef = VampireDef {
    name: "Nathaniel Bordruff",
    abilities: &[enter_combat(&[
        R::NotControlled,
        R::ActorReady,
        R::TargetVampire,
        R::TargetLocked,
    ])],
    ..VAMP
};

pub static NESERIAN: VampireDef = VampireDef {
    name: "Neserian",
    printed: &[
        own(Stat::Bleed, -1, &[R::ControlsNoLocation]),
        own(Stat::Votes, 1, &[R::ControlsLocation]),
    ],
    ..VAMP
};

/// "Noluthando inflicts +1 damage with ranged strikes (even at close
/// range)."
pub static NOLUTHANDO: VampireDef = VampireDef {
    name: "Noluthando",
    printed: &[own(Stat::RangedStrikeDamage, 1, &[])],
    ..VAMP
};

pub static NONU_DIS: VampireDef = VampireDef {
    name: "Nonu Dis",
    printed: &[own(Stat::Bleed, 1, &[])],
    // Once per master phase however many master cards are played in it
    // (ANK 20220617). "Follower of Set" is the clan's V5 name, Ministry.
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[R::Controlled, R::ActorReady, R::ActorClan("Ministry")],
        ),
        ..mode_of(
            &[Window::MasterPhase],
            Actor::Host,
            &[R::HostReady, R::MasterCardPlayed],
            &[Effect::BloodToTarget(1)],
        )
    }],
    ..VAMP
};

pub static OLUWAFUNMILAYO: VampireDef = VampireDef {
    name: "Oluwafunmilayo",
    printed: &[own(Stat::Intercept, 1, &[R::IsPolitical])],
    ..VAMP
};

/// "Once each combat involving a vampire you control, Opikun can burn 1
/// blood to prevent up to 2 non-aggravated damage to that vampire from the
/// opposing minion's strike." Strike damage only: not Carrion Crows' nor
/// Pulled Fangs' (rules-qa 74.1).
pub static OPIKUN: VampireDef = VampireDef {
    name: "Opikun",
    abilities: &[ability(
        &[Window::CombatDamagePrevention],
        Actor::ForOwnCombatant,
        &[R::HostBloodAtLeast(1), R::StrikeNormalDamagePending],
        Limit::PerCombat,
        &[
            Effect::HostBurnBlood(1),
            Effect::PreventStrike {
                n: 2,
                aggravated: false,
            },
        ],
    )],
    ..VAMP
};

pub static OSKAR_ANASOV: VampireDef = VampireDef {
    name: "Oskar Anasov",
    printed: &[own(Stat::Stealth, 1, &[])],
    // To the vampire of his controller attempting to block, while it needs
    // intercept (rules-qa 3.2), for the rest of the action; in torpor too
    // (LSJ 20011022); "during an action": once per action (rules-qa 15.2).
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionStealthIntercept],
            Actor::Host,
            &[R::YourAttemptNeedsIntercept, R::HostBloodAtLeast(1)],
            &[Effect::HostBurnBlood(1), Effect::AttemptingIntercept(1)],
        )
    }],
    ..VAMP
};

/// "Once each combat, Phaibun can discard a card at random to strike:
/// dodge."
pub static PHAIBUN: VampireDef = VampireDef {
    name: "Phaibun",
    abilities: &[ability(
        &[Window::CombatStrikeDeclaration],
        Actor::Host,
        &[R::HandNotEmpty],
        Limit::PerCombat,
        &[
            Effect::DiscardAtRandom(1),
            Effect::Strike(StrikeKind::Dodge),
        ],
    )],
    ..VAMP
};

pub static QUEEN_ANNE: VampireDef = VampireDef {
    name: "Queen Anne",
    group: Some(6),
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    R::Younger,
                    R::ActorSectIn(&[Sect::Camarilla]),
                    R::NotAbstaining,
                ],
            ),
            limit: Some(Limit::PerReferendum),
            ..mode_of(
                &[Window::Referendum],
                Actor::Host,
                &[
                    R::PollingStep,
                    R::ActorCalledReferendum,
                    R::HostBloodAtLeast(1),
                ],
                &[
                    Effect::HostBurnBlood(1),
                    Effect::Abstain {
                        lock: false,
                        burn: 0,
                    },
                ],
            )
        },
        ModeDef {
            windows: &[Window::ActionAfterResolution],
            actor: Actor::Host,
            // As the acting minion or as the blocker of a leave-torpor
            // action (rules-qa 16.2).
            requires: &[R::ActorDiablerised, R::HostLocked],
            target: None,
            terms: None,
            waives_needed: false,
            limit: None,
            effects: &[Effect::UnlockActor],
        },
    ],
    ..VAMP
};

/// "Cards requiring Protean [pro] cost Ragnar -1 blood. Ragnar gets +1
/// strength in combat with an ally or younger vampire." Requiring = played
/// using Protean (rules-qa 38.4, 42.5): wherever he pays the card's blood,
/// combat cards included.
pub static RAGNAR_NORDSTROM: VampireDef = VampireDef {
    name: "Ragnar Nordstrom",
    printed: &[
        own(Stat::BloodCostOff, 1, &[R::PlayedRequiresDiscipline("pro")]),
        own(Stat::Strength, 1, &[R::OpponentAllyOrYounger]),
    ],
    ..VAMP
};

/// "Each time you play a location card, you can use up to 1 of Regina's
/// blood to pay some or all of its blood or pool cost": announced with the
/// card like a cost reduction, burned as the cost is paid; a locquipment
/// played as equipment too (LSJ 20080107, LSJ 20080114); in torpor too
/// (LSJ 20011022); a card another puts in play is still played (a search,
/// Contraband: rules-qa 41.13).
pub static REGINA_BLAKE: VampireDef = VampireDef {
    name: "Regina Blake",
    printed: &[own(
        Stat::BloodPaysCost,
        1,
        &[R::PlayedCard(CardFilter::Location)],
    )],
    ..VAMP
};

/// "After Richard successfully bleeds, he can look at the target
/// Methuselah's hand, then he can burn 1 blood to discard 1 card from it."
/// Once per action (rules-qa 16.1).
pub static RICHARD_DE_WORDE: VampireDef = VampireDef {
    name: "Richard de Worde",
    abilities: &[ModeDef {
        terms: Some(Terms::DirectedHandCard),
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionAfterResolution],
            Actor::Host,
            &[
                R::ActorActing,
                R::IsBleed,
                R::BleedSuccessful,
                R::HostBloodAtLeast(1),
                R::DirectedHandNotEmpty,
            ],
            &[Effect::HostBurnBlood(1)],
        )
    }],
    ..VAMP
};

pub static RINALDO_ALBIZZI: VampireDef = VampireDef {
    name: "Rinaldo Albizzi",
    printed: &[own(Stat::Intercept, 1, &[])],
    ..VAMP
};

/// "Once each turn, if Roger is ready during a combat involving a minion
/// you control, he can discard a card requiring Oblivion [obl] to give that
/// minion 1 maneuver": used as the range is determined (rules-qa 41.9).
const ROGER_MANEUVER: ModeDef = ModeDef {
    terms: Some(Terms::Discard {
        n: 1,
        of: CardFilter::Discipline("obl"),
    }),
    limit: Some(Limit::PerTurn),
    ..mode_of(
        &[Window::CombatDetermineRange],
        Actor::ForOwnCombatant,
        &[R::HostReady, R::HandHas(CardFilter::Discipline("obl"), 1)],
        &[Effect::Maneuver],
    )
};

/// "Cards requiring Hecata and/or Oblivion [obl] cost Roger -1 blood": a
/// card requiring the clan, or played using Oblivion (rules-qa 38.4).
pub static ROGER_DE_CAMDEN: VampireDef = VampireDef {
    name: "Roger de Camden",
    printed: &[own(
        Stat::BloodCostOff,
        1,
        &[R::AnyOf(&[
            &[R::PlayedCard(CardFilter::Clan("Hecata"))],
            &[R::PlayedRequiresDiscipline("obl")],
        ])],
    )],
    abilities: &[ROGER_MANEUVER],
    ..VAMP
};

pub static ROSE_ABAWI: VampireDef = VampireDef {
    name: "Rose Abawi",
    printed: &[own(Stat::Bleed, 1, &[R::ControlsVampireNamed("Mithras")])],
    ..VAMP
};

/// "Rowena can prevent 1 non-aggravated damage each round. She can strike:
/// combat ends in combat with a werewolf." A werewolf is an ally of that
/// subtype (advanced rules 5.7.1: the type line only).
pub static ROWENA: VampireDef = VampireDef {
    name: "Rowena",
    abilities: &[
        ability(
            &[Window::CombatDamagePrevention],
            Actor::Host,
            &[R::NonAggravatedDamagePending],
            Limit::PerRound,
            &[Effect::PreventNonAggravated(1)],
        ),
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Host,
            &[R::OpponentSubtype(Label::Werewolf)],
            &[Effect::Strike(StrikeKind::CombatEnds { unlock: false })],
        ),
    ],
    ..VAMP
};

pub static ROY: VampireDef = VampireDef {
    name: "Roy",
    printed: &[own(Stat::Strength, 1, &[R::OpponentTitled])],
    // "If Roy is unlocked during your discard phase, lock him": as the phase
    // ends, in torpor too (LSJ 20011022).
    riders: &[Rider {
        hook: Hook::DiscardPhase,
        scope: RiderScope::Controller,
        requires: &[R::HostUnlocked],
        effects: &[Effect::LockActor],
    }],
    ..VAMP
};

/// "Saankaláxt can steal an equipment as a Ⓓ action that costs 1 blood."
/// Not a second vehicle (rules-qa 17.4).
pub static SAANKALAXT: VampireDef = VampireDef {
    name: "Saankaláxt",
    abilities: &[steal_equipment(
        &[
            R::TargetCard(CardFilter::EquipmentWithout(&[])),
            R::NotHost,
            R::NotControlled,
            R::ActorMayHoldTarget,
        ],
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::PayBlood(1),
            Effect::StealTargetCard,
        ],
    )],
    ..VAMP
};

/// "Sakhar can search your library for an equipment card, reveal it and
/// move it to your hand as a +1 stealth action."
pub static SAKHAR: VampireDef = VampireDef {
    name: "Sakhar",
    abilities: &[printed_action(
        None,
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::SearchToHand {
                filter: CardFilter::Type(CardType::Equipment),
                shown: true,
            },
        ],
    )],
    ..VAMP
};

pub static SAKU_PIHLAJAMAKI: VampireDef = VampireDef {
    name: "Saku Pihlajamäki",
    abilities: &[MANEUVER_EACH_COMBAT],
    ..VAMP
};

/// "Seraphina can add 2 blood or life to a minion you control as a +1
/// stealth action, not to exceed starting life." A vampire's blood stops at
/// its capacity, an ally's life at its starting life (advanced rules
/// 5.5.2); one in torpor, or Seraphina herself, may be chosen.
pub static SERAPHINA: VampireDef = VampireDef {
    name: "Seraphina",
    abilities: &[printed_action(
        target(TargetKind::Minion, &[R::Controlled]),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::BloodToTargetToStart(2),
        ],
    )],
    ..VAMP
};

/// "Minions with 1 or more of your corruption counters must burn 1 blood or
/// life to attempt to block Sergio": a toll on the attempt, in torpor too
/// — his leave-torpor action (LSJ 20011022).
pub static SERGIO_BUENO: VampireDef = VampireDef {
    name: "Sergio Bueno",
    printed: &[Printed {
        stat: Stat::BlockToll,
        n: 1,
        to: PrintedTo::Opponent,
        per: Per::One,
        requires: &[R::ActorCorruptedBySource],
    }],
    ..VAMP
};

pub static SREELEKHA: VampireDef = VampireDef {
    name: "Sreelekha",
    printed: &[
        own(Stat::Bleed, 1, &[]),
        own(Stat::DiscardPhaseActions, 1, &[R::ThisVampireReady]),
    ],
    ..VAMP
};

/// "During your unlock phase, you can move 1 blood or life between Sri and
/// a mortal or mage ally you control, not to exceed their starting life":
/// one move each unlock phase, either way (rules-qa 6.6), in torpor too
/// (LSJ 20011022); the starting-life cap is the ally's — not offered to an
/// ally at its starting life —, Sri's blood above her capacity draining as
/// usual; an ally giving its last life burns (rules-qa 41.8). Mages:
/// Nephandus, Veneficti.
pub static SRI_SANSA: VampireDef = VampireDef {
    name: "Sri Sansa",
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    R::Controlled,
                    R::AnyOf(&[
                        &[R::TargetAllyCard(CardFilter::HasLabel(Label::Mortal))],
                        &[R::TargetAllyCard(CardFilter::HasLabel(Label::Mage))],
                    ]),
                    R::TargetAllyHurt,
                ],
            ),
            ..mode_of(
                &[Window::UnlockPhaseOwn],
                Actor::Host,
                &[R::HostBloodAtLeast(1)],
                &[Effect::HostBloodToTarget(1)],
            )
        },
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    R::Controlled,
                    R::AnyOf(&[
                        &[R::TargetAllyCard(CardFilter::HasLabel(Label::Mortal))],
                        &[R::TargetAllyCard(CardFilter::HasLabel(Label::Mage))],
                    ]),
                ],
            ),
            ..mode_of(
                &[Window::UnlockPhaseOwn],
                Actor::Host,
                &[],
                &[Effect::TargetBloodToHost(1)],
            )
        },
    ],
    ..VAMP
};

/// "Blood hunt referendums called on Styles get +2 votes in favor of the
/// referendum."
pub static STYLES_MARGS: VampireDef = VampireDef {
    name: "Styles Margs",
    printed: &[own(Stat::BloodHuntVotesInFavor, 2, &[])],
    ..VAMP
};

/// "You can unlock Sybren after a referendum called by him passes."
pub static SYBREN_VAN_OOSTEN: VampireDef = VampireDef {
    name: "Sybren van Oosten",
    abilities: &[mode_of(
        &[Window::ActionAfterResolution],
        Actor::Host,
        &[
            R::ActorActing,
            R::IsPolitical,
            R::ReferendumPassed,
            R::HostLocked,
        ],
        &[Effect::UnlockActor],
    )],
    ..VAMP
};

pub static THEO_BELL: VampireDef = VampireDef {
    name: "Theo Bell",
    group: Some(6),
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[enter_combat(&[R::NotControlled, R::ActorReady])],
    ..VAMP
};

/// "Once each turn, if he is ready, Tommaso can burn 1 blood before range
/// is determined in a combat involving a wraith or zombie ally you control
/// to end that combat." From out of the combat, his ally the combatant
/// (rules-qa 41.10).
pub static TOMMASO_SFORZA: VampireDef = VampireDef {
    name: "Tommaso Sforza",
    abilities: &[ability(
        &[Window::CombatBeforeRange],
        Actor::ForOwnCombatant,
        &[
            R::HostReady,
            R::HostBloodAtLeast(1),
            R::AnyOf(&[
                &[R::ActorAllyCard(CardFilter::HasLabel(Label::Wraith))],
                &[R::ActorAllyCard(CardFilter::HasLabel(Label::Zombie))],
            ]),
        ],
        Limit::PerTurn,
        &[Effect::HostBurnBlood(1), Effect::EndCombat],
    )],
    ..VAMP
};

/// "If Valerius is Camarilla and not a prince, he can steal the title held
/// by a prince as a Ⓓ action. +1 bleed." The prince loses the title and
/// Valerius gains it (a title he held is lost: one title per vampire), a
/// baron of the same city contesting it — never his own Methuselah's
/// (advanced rules §1.13.1; rules-qa 100.1).
pub static VALERIUS: VampireDef = VampireDef {
    name: "Valerius",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[ModeDef {
        requires: &[
            R::ActorSectIn(&[Sect::Camarilla]),
            R::ActorTitleNotIn(&[TitleClass::Prince]),
        ],
        ..printed_action(
            target(
                TargetKind::Minion,
                &[
                    R::TargetVampire,
                    R::NotControlled,
                    R::TargetTitleIn(&[TitleClass::Prince]),
                    R::NoOwnClaimOfTargetTitle {
                        besides_actor: true,
                    },
                ],
            ),
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::StealTargetTitle,
            ],
        )
    }],
    ..VAMP
};

pub static VALERIYA_ZINOVIEVA: VampireDef = VampireDef {
    name: "Valeriya Zinovieva",
    printed: &[own(Stat::Strength, 1, &[])],
    ..VAMP
};

pub static WEIRICH_WALDBURG: VampireDef = VampireDef {
    name: "Weirich Waldburg",
    printed: &[Printed {
        stat: Stat::Stealth,
        n: 1,
        to: PrintedTo::OthersControlled,
        per: Per::One,
        requires: &[
            R::ThisVampireReady,
            R::ActorTitleIn(&[TitleClass::Primogen]),
        ],
    }],
    ..VAMP
};

pub static YEWON_ONG: VampireDef = VampireDef {
    name: "Yewon Ong",
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Discipline("obl"),
        }),
        limit: Some(Limit::PerReferendum),
        ..mode_of(
            &[Window::Referendum],
            Actor::Host,
            &[
                R::AnyPollingStep,
                R::HandHas(CardFilter::Discipline("obl"), 1),
            ],
            &[Effect::AddVotes(1)],
        )
    }],
    ..VAMP
};

/// "Independent: While Parijat is ready, minions must burn the top card of
/// their library to attempt to block wraith or zombie allies." A toll on
/// the attempt: none possible from an empty library;
/// a forced attempt pays it once its card is replaced (ANK 20260317).
pub static PARIJAT: VampireDef = VampireDef {
    name: "Parijat, the Dark Oracle",
    printed: &[own(Stat::WraithZombieBlockToll, 1, &[])],
    ..VAMP
};

pub static VAMPIRES: [&VampireDef; 496] = [
    &JOSEF_VON_BAUREN,
    &GABRIELLE_DI_RIGHETTI,
    &HAROLD_TANNER,
    &GIOTTO_VERDUCCI,
    &LEANDRO,
    &GUSTAPHE_BRUNNELLE,
    &VENERE_CARBONI,
    &NERGAL_ADV,
    &ANNABELLE_TRIABELL,
    &GENEVIEVE,
    &DARK_SELINA,
    &DROZODNY,
    &HARDESTADT,
    &MALABRANCA,
    &IAN_CARFAX,
    &VICTORIA_ASH_G7,
    &ALVARO,
    &AMBROSIO,
    &ANTONIO_DELGADO,
    &FRANCISCO_DOMINGO_DE_POLONIA,
    &GRATIANO,
    &ANGELICA,
    &AYELEA,
    &MARCIANA_GIOVANNI,
    &MONTANO,
    &ROYCE,
    &JOST_WERNER,
    &ZOE,
    &ANKA,
    &LAZAR_DOBRESCU,
    &SPLEEN,
    &KHALIL_RAVANA,
    &RICHTER,
    &APPOLONIUS,
    &APPOLONIUS_ADV,
    &DORKA,
    &GABI,
    &VIRI,
    &ZIZI,
    &AGI,
    &BARRY,
    &JERRY,
    &LARRY,
    &TERRY,
    &TOM,
    &DANYLO,
    &GAVRYLO,
    &PAVLO,
    &VOLO,
    &JACK,
    &MARK,
    &HERMANA_HAMBRIENTA_MAYOR,
    &HERMANA_HAMBRIENTA_MENOR,
    &PARIJAT,
    &ABADDON,
    &ABRAHAM_DUSABLE,
    &ABRAHAM_MELLON,
    &ADELINO_DA_SILVA,
    &ADISA,
    &ADRINO_MANAUARA,
    &ALABASTROM,
    &AISHA_AZ_ZAHRA,
    &ALEXA_DRAPER,
    &ALEXANDER_SILVERSON,
    &ALICIA_CORTEZ,
    &ALINE_GADEKE,
    &ANDRE_THE_MANIPULATOR,
    &ANIEL,
    &ANNE_MARIE_BOURGEOIS,
    &APOLONIA_CZARNECKI,
    &ARIANE,
    &AYSE_DHANIAL,
    &AZUCENA,
    &BARACHIEL,
    &BRANIMIRA,
    &BRET_STRYKER,
    &CARMELITA_NEILLSON,
    &CASTELLAN,
    &CATALINA_VEGA,
    &CEDRICK_CALHOUN,
    &CHUN_HEI,
    &DAFINA_HANGANU,
    &DARIA,
    &DARIUS_SHIRAZI,
    &DJENEBA,
    &DOC_MARTINA,
    &DOMINICA,
    &DOWAGER_THE,
    &EGIDIA_ARRU,
    &ELEN_KAMJIAN,
    &EVAN_KLEIN,
    &FARUQ_ABD_AL_QADIR,
    &FLAVIO_GONCALVES,
    &GATHII,
    &GNAEUS_AEMILIUS_AUGUSTINUS,
    &GUARDIAN_THE,
    &GWENLLIAN_ARWYN,
    &HAFTHOR_THORSTEINSSON,
    &HEL_BLA,
    &HESHA_RUHADZE,
    &ILONKA,
    &JASON_SON_NEWBERRY,
    &KALINDA,
    &KAMILE_PAUKSTYS,
    &KASIM_BAYAR,
    &KEEGAN,
    &KEVIN_JACKSON,
    &KUYEN,
    &LADY_SCARLETT_CHURCHILL,
    &LARISSA_MOREIRA,
    &LENELLE,
    &LENNY_BURKHEAD,
    &LUCIANO_CARVALHO,
    &MADDJI,
    &MARCHESA_LILIANA,
    &MARIALENA,
    &MARTINA_SRNANKOVA,
    &MASSIMILIANO,
    &MICKEY_WHEELER,
    &MITHRAS,
    &MORA_THE_DEATH_SEER,
    &MOTHER_ANJA_GIOVANNI,
    &NATHANIEL_BORDRUFF,
    &NESERIAN,
    &NOLUTHANDO,
    &NONU_DIS,
    &OLUWAFUNMILAYO,
    &OPIKUN,
    &OSKAR_ANASOV,
    &PHAIBUN,
    &QUEEN_ANNE,
    &RAGNAR_NORDSTROM,
    &REGINA_BLAKE,
    &RICHARD_DE_WORDE,
    &RINALDO_ALBIZZI,
    &ROGER_DE_CAMDEN,
    &ROSE_ABAWI,
    &ROWENA,
    &ROY,
    &SAANKALAXT,
    &SAKHAR,
    &SAKU_PIHLAJAMAKI,
    &SERAPHINA,
    &SERGIO_BUENO,
    &SREELEKHA,
    &SRI_SANSA,
    &STYLES_MARGS,
    &SYBREN_VAN_OOSTEN,
    &THEO_BELL,
    &TOMMASO_SFORZA,
    &VALERIUS,
    &VALERIYA_ZINOVIEVA,
    &WEIRICH_WALDBURG,
    &YEWON_ONG,
    &GILBERT_DUANE,
    &HARTMUT_STOVER,
    &INGRID_ROSSLER,
    &MARK_DECKER,
    &STANISLAVA,
    &XAVIAR_ADV,
    &DIANA_IADANZA,
    &FRANCOIS_VILLON,
    &MARCOS_BELEGRAD,
    &MODIUS,
    &SILVIAN_POPESCU,
    &AARADHYA,
    &DAMIAN,
    &KHIN_AYE,
    &REXTON,
    &URESSEG,
    &VENCESLAVA,
    &AELSWITH,
    &AGNIESZKA,
    &DEVORAH,
    &FOTINI_KATSIKARIS,
    &GOSTOSO,
    &JURGEN,
    &OSVALDO_KUHNEMANN,
    &VERRIX,
    &ALGIRDAS,
    &ANOUSHA,
    &CROSSBREAKER,
    &KALYANI,
    &SERHAT_GUNDE,
    &ABDERRAHIM,
    &ANXO_VILELA,
    &ASHUR_UBALLIT,
    &EULOGIO,
    &FRAU_SCHADEL,
    &HULDU,
    &SAKURA,
    &ALONSO_PETRODON,
    &ANARCH_CONVERT,
    &GABRIEL_TREMBLAY,
    &MAILA,
    &BRUNHILDE,
    &ELIMELECH,
    &REGINALD_MOORE,
    &THE_MEDIC,
    &VALKYRIE,
    &ARISHAT,
    &DANIELLE_DIRON,
    &SCOUT_YOUNGWOOD,
    &KAREN_SUADELA,
    &BENJAMIN_ROSE,
    &DMITRA_ILYANOVA,
    &GERALD_WINDHAM,
    &INYANGA,
    &LODIN,
    &MATASUNTHA,
    &NADIMA,
    &RANDALL,
    &FAKIR_AL_SIDI,
    &TJ,
    &GIOVANNI_DEL_GEORGIO,
    &MARY_ANNE_BLAIRE,
    &KARL_SCHREKT,
    &VICTOR_DONALDSON,
    &AKSINYA_DACLAU,
    &THEO_BELL_G2,
    &JULIET_PARR,
    &MISTRESS_FANCHON,
    &BEAST,
    &LORD_TREMERE,
    &NERGAL,
    &THE_CAPUCHIN,
    &JIMMY_DUNN,
    &MALGORZATA,
    &ADANA_DE_SFORZA,
    &ALEXANDRA,
    &FRANCOIS_VILLON_G2,
    &GORATRIX,
    &NEIGHBOR_JOHN,
    &OMAYA,
    &UNMADA,
    &THE_UNNAMED,
    &VICTORIA_ASH,
    &ANSON,
    &LUTZ_VON_HOHENZOLLERN,
    &MARCELLUS,
    &OWAIN_EVANS,
    &RAFAEL_DE_CORAZON,
    &ANNEKE,
    &KEITH_MOODY,
    &BLACKHORSE_TANNER,
    &ANTONIO_D_ERLETTE,
    &TUPDOG,
    &LISETTE_VIZQUEL,
    &WISE_FROG,
    &ARIADNE,
    &JACOB_FERMOR,
    &TABITHA_FISK,
    &TATIANA_ROMANOV,
    &ADONAI,
    &CORNELIUS_OTTAVIO,
    &OZMO,
    &QUIRA,
    &SHEILA_MEZARIN,
    &BABALAWO_ALAFIN,
    &EREBUS,
    &ERLIK,
    &MINA_GROTIUS,
    &NICOMEDES,
    &SOLOMON_BATANEA,
    &ARIKA,
    &LE_DINH_THO,
    &LUCINDE,
    &MARCUS_VITEL,
    &QUEEN_ANNE_G2,
    &THE_BARON,
    &CELESTE_LAMONTAGNE,
    &LUDMIJLA_RAKOCZY,
    &PETRA,
    &RURIK_RAKOCZY,
    &OSRIC_VLADISLAV,
    &PERSEPHONE_TAR_ANIS,
    &DIEGO_GIOVANNI,
    &DON_MICHAEL_ANTONIO_GIOVANNI,
    &THOMASSO_GHIBERTI,
    &DONATELLO_GIOVANNI,
    &ANDRE_LEROUX,
    &BLACK_ANNIS,
    &ENKIDU,
    &NEW_BLOOD,
    &EUGENE,
    &HELENA,
    &IRA_RIVERS,
    &JOAQUIN_MURIETTA,
    &KHALU,
    &ABIKU,
    &CESEWAYO,
    &MASSASSI,
    &TATU_SAWOSA,
    &EDDIE_GAINES,
    &MARTA,
    &MIDGET,
    &PERSIA,
    &ALESSANDRO_GARCIA,
    &LAIKA,
    &XIPE_TOTEC,
    &LUC,
    &THUCIMIA,
    &ASGURESH,
    &ADHIAMBO,
    &TYLER_MCGILL,
    &SAULOT_THE_WANDERER,
    &PAULO_DE_CASTILLE,
    &LEILA_MONROE,
    &MATEUSZ_KAZIMIRSKI,
    &LUCINDE_G7,
    &AIDAN_LYLE,
    &CHARLES_DELMARE,
    &ANTON_DE_CONCEPCION,
    &DUALITY,
    &EMILY_CARSON,
    &FRONDATOR,
    &GEM_GHASTLY,
    &JEREMY_WIX_WYZCHOVSKY,
    &KATELINE_NADASDY,
    &L_EPUISETTE,
    &LUCA_ITALICUS,
    &NIKOLAUS_VERMEULEN_G2,
    &NIKOLAUS_VERMEULEN_G6,
    &SELMA_THE_REPUGNANT,
    &UNA,
    &LADY_VADISLAVA,
    &LORD_LEOPOLD_VALDEMAR,
    &GRIMGROTH,
    &DONAL_OCONNOR_G6,
    &MURAT,
    &BELA,
    &TROIUS,
    &MOLLY_MACDONALD,
    &CAROLINA_VALEZ,
    &RUTOR,
    &VASILY,
    &JOAO_BILE,
    &THOMAS_DE_LUTRIUS,
    &EPIKASTA_RIGATOS,
    &CALEBROS,
    &DR_MORROW,
    &ZANE,
    &MUSTAFA,
    &AIMERY_METHUIN,
    &DR_JULIUS_SUTPHEN,
    &LAZVERINUS,
    &ALEX_CAMILLE,
    &BADR,
    &CARNA,
    &CLAUDIO_SEVERINO,
    &DAN_MURDOCK,
    &DE_SADE,
    &GOLD_PAN_DAN,
    &KYLE_STRATHCONA,
    &MASIKA,
    &QUENTIN,
    &STROHMANN,
    &SELENA,
    &MARTIN_FRANCKEL,
    &MELUSINE_D_ANJOU,
    &MITHRAS_G3,
    &THE_DRACON,
    &VLAD_TEPES,
    &ANDREW_STUART,
    &ORLANDO_ORIUNDUS,
    &KARIF_AL_NUMAIR,
    &GERALD_FITZGERALD,
    &RIGHTEOUS_ENDEAVOR,
    &SUHAILAH,
    &SYNNER_G,
    &PAUL_CORDWOOD,
    &BULSCU,
    &MIMIR,
    &BRUCE_DE_GUY,
    &MALDAVIS,
    &WINTERLICH,
    &NEHEMIAH,
    &BABA_YAGA,
    &APPIUS_CLAUDIUS_CORVUS,
    &ABDELSOBEK,
    &GHEDE,
    &JUNIPER,
    &ONAEDO,
    &SAUL_MEIRA,
    &TEGYRIUS,
    &FEO_RAMOS,
    &SHALMATH,
    &CYBELE,
    &CESAR_HOLFIELD,
    &DONALD_CARGILL,
    &EDWARD_VIGNES,
    &IRIS_BENNETT,
    &KATHERINE_STODDARD,
    &LUBOMIRA_HRADOK,
    &WALKER_GRIMES,
    &SKIDMARK,
    &FRANCISCUS,
    &BOLESLAW_GUTOWSKI,
    &HELENA_ADV,
    &MARIS_STRECK,
    &AISLING_STURBRIDGE,
    &BLYTHE_CANDELERIA,
    &DR_SOLOMON_GREY,
    &GENERAL_PERFIDIO_DIOS,
    &KARSH_ADV,
    &KITE,
    &MARIE_FAUCIGNY,
    &MENELE_ADV,
    &QAWIYYA_EL_GHADUBA,
    &RICHARD_TAUBER,
    &RODOLFO,
    &YAZID_TAMARI_ADV,
    &NEFERTITI_ADV,
    &FIORENZA_SAVONA,
    &OKULOS,
    &LEN_KONOPKA,
    &KISHA_BHIMJI,
    &LUCITA,
    &MARTINO_DELLA_PASSAGLIA,
    &NU_THE_PILLAR,
    &XAVIAR,
    &JESSICA_ADV,
    &DR_OLIVER_GENET,
    &HESHA_RUHADZE_G2,
    &JOE_BOOT_HILL,
    &HANNIBAL_ADV,
    &ANU_DIPTINATPA,
    &ECHO,
    &KESTRELLE_HAYES,
    &CLAUDIO_SEVERINO_BASE,
    &PORPHYRY,
    &VERBRUCH,
    &MITCHELL_THE_HEADHUNTER,
    &REVEREND_ADAMS,
    &YONG_SUN,
    &GENINA,
    &GUILLAUME_GIOVANNI,
    &LAMBACH_ADV,
    &FATIMA_AL_FAQADI,
    &WHITE_LILY,
    &SASCHA_VYKOS,
    &TORI_LONGWOOD,
    &VICTOR_PELLETIER,
    &BALDESAR_ROSSELLINI,
    &ALAN_SOVEREIGN_ADV,
    &BARBARO_LUCCHESE,
    &ANGELO,
    &ANTOINETTE_DUCHAMP,
    &CHANDLER_HUNGERFORD,
    &CHRISTINE_BOSCACCI,
    &EURAYLE_GELASIA_MYLONAS,
    &FERGUS_ALEXANDER,
    &GREENSLEEVES,
    &JULIUS,
    &NAVAR_MCCLAREN,
    &SMUDGE_THE_IGNORED,
    &ANNA_DICTATRIX11_SULJIC,
    &BEATRICE_ORACLE171_TREMBLAY,
    &EARL_SHAKA74_DEAMS,
    &ERICK_SHOPHET125_FRANCO,
    &FRANCOIS_WARDEN_LOEHR,
    &INEZ_NURSE216_VILLAGRANDE,
    &JACK_HANNIBAL137_HARMON,
    &JENNIFER_FLAME61_VIDISANIA,
    &JOHN_COP90_OMALLEY,
    &LEAF_POTTER116_PANKOWSKI,
    &LIZ_TICKET312_THORNTON,
    &LUPE_CABBIE22_DROIN,
    &MAMAN_BOUMBA,
    &MARION_TEACHER193_PERKS,
    &PAUL_SIXOFSWORDS29_MORETON,
    &PEDRO_CORTEZ,
    &PETER_OUTBACK295_ROPHAIL,
    &TRAVIS_TRAVELER72_MILLER,
    &XIAN_DZIDZAT155_QUAN,
    &JENNIE_CASSIE247_ORNE,
    &ISANWAYEN,
    &THE_ARCADIAN,
    &OMME_ENBERBENIGHT,
    &PHERYDIMA,
    &RODERICK_PHILLIPS_MARCH,
    &QUINCY_THE_TRAPPER,
    &BARTHOLOMEW,
    &JULIA_PRIMA,
    &KASSIYM_MALIKHAIR,
    &MARCONIUS,
    &BOSS_CALLIHAN,
    &DALIAH,
    &GENGIS,
    &JAVIER_MONTOYA,
    &JENNA_CROSS,
    &MARGUERITE_FOCCART,
    &PHILIP_VAN_VERMEER_IV,
    &SALVADOR_GARCIA,
    &VICTOR_GERARD,
    &VULTURE,
];

/// "Anarch Baron of Miami: During your influence phase, Gilbert can burn 1
/// blood to reveal and remove the top card of your crypt from the game. If
/// it is a Malkavian, a non-titled vampire you control becomes Anarch."
/// (Group 6, a legacy addition for tournament decks.) Once per influence
/// phase (rules-qa 6.6); from torpor too (LSJ 20011022). The vampire made
/// Anarch is picked as it resolves: any non-titled one, in torpor
/// included, himself excluded by his title.
pub static GILBERT_DUANE: VampireDef = VampireDef {
    name: "Gilbert Duane",
    abilities: &[mode_of(
        &[Window::InfluencePhaseOwn],
        Actor::Host,
        &[R::HostBloodAtLeast(1)],
        &[
            Effect::HostBurnBlood(1),
            Effect::RemoveCryptTopAnarch("Malkavian"),
        ],
    )],
    ..VAMP
};

// ---- Reign of Stanislava (25th Anniversary) -------------------------------

/// "Sabbat Archbishop of Berlin: Any vampire casting votes or ballots
/// against a referendum called by Hartmut burns 1 blood once results are
/// tallied. You can burn the Edge during your discard phase to gain 1
/// pool." The burn is no cost: a vampire without blood votes against and
/// burns nothing; Hartmut too if he votes against his own; Methuselah
/// ballots cost nothing (rules-qa 49.2). The Edge burn is his controller's,
/// once per discard phase (rules-qa 6.6), from torpor too (advanced rules:
/// printed abilities stay usable in torpor).
pub static HARTMUT_STOVER: VampireDef = VampireDef {
    name: "Hartmut Stover",
    printed: &[own(Stat::AgainstCastersBurnBlood, 1, &[])],
    abilities: &[mode_of(
        &[Window::DiscardPhase],
        Actor::Host,
        &[R::HasEdge],
        &[Effect::BurnEdge, Effect::GainPool(1)],
    )],
    ..VAMP
};

/// "Camarilla Prince of Geneva: If Ingrid is ready at the start of your
/// influence phase, you get +2 transfers." Read as the phase begins:
/// influenced out during it, she gives none that phase (RTR 20180303).
pub static INGRID_ROSSLER: VampireDef = VampireDef {
    name: "Ingrid Rossler",
    printed: &[own(Stat::Transfers, 2, &[R::ThisVampireReady])],
    ..VAMP
};

/// "Camarilla Prince of Milwaukee: Mark gets an additional +1 strength in
/// combat with a werewolf. +1 strength." A werewolf: an ally of the subtype
/// (Double Deuce).
pub static MARK_DECKER: VampireDef = VampireDef {
    name: "Mark Decker",
    printed: &[
        own(Stat::Strength, 1, &[]),
        own(Stat::Strength, 1, &[R::OpponentSubtype(Label::Werewolf)]),
    ],
    ..VAMP
};

/// "Camarilla Gangrel Inner Circle: Retainers employed by minions opposing
/// Stanislava in combat lose their abilities. Allies cannot block her. +2
/// bleed." The retainers lose their card text for that combat; they stay
/// retainers whose life takes damage (rules-qa 49.3).
pub static STANISLAVA: VampireDef = VampireDef {
    name: "Stanislava",
    printed: &[
        own(Stat::Bleed, 2, &[]),
        own(Stat::AlliesCannotBlock, 1, &[]),
        own(Stat::OpposingRetainersLoseText, 1, &[]),
    ],
    ..VAMP
};

/// "Camarilla Gangrel Justicar: You can unlock Xaviar after a referendum
/// called by him passes." (Sybren van Oosten's text.) The advanced card:
/// it contests the base Xaviar as the same vampire; merged, both cards'
/// text (`XAVIAR_MERGED`).
pub static XAVIAR_ADV: VampireDef = VampireDef {
    name: "Xaviar",
    adv: true,
    abilities: &[mode_of(
        &[Window::ActionAfterResolution],
        Actor::Host,
        &[
            R::ActorActing,
            R::IsPolitical,
            R::ReferendumPassed,
            R::HostLocked,
        ],
        &[Effect::UnlockActor],
    )],
    ..VAMP
};

// ---- The Endless Dance (30th Anniversary) ---------------------------------

/// "Camarilla Toreador Justicar: During a referendum she calls, Diana can
/// burn 1 blood to cancel a reaction card as it is played (cost is still
/// paid). +1 bleed." The cancel follows the cancel rules: in the as-played
/// window, nothing but wakes (LSJ 20061207); the card is still played (ANK
/// 20190104), replaced normally (LSJ 20080630); a limited effect canceled
/// leaves the limit untouched (LSJ 20030224). Any reaction, in the
/// referendum of her political action (rules-qa 51.8).
pub static DIANA_IADANZA: VampireDef = VampireDef {
    name: "Diana Iadanza",
    printed: &[own(Stat::Bleed, 1, &[])],
    // "During a referendum she calls": once in it (15.2).
    abilities: &[ModeDef {
        limit: Some(Limit::PerReferendum),
        ..mode_of(
            &[Window::CardAsPlayed],
            Actor::Host,
            &[
                R::HostBloodAtLeast(1),
                R::PlayedCardType(CardType::Reaction),
                R::ActorCalledReferendum,
            ],
            &[
                Effect::HostBurnBlood(1),
                Effect::CancelPlayed { refund: false },
            ],
        )
    }],
    ..VAMP
};

/// "Camarilla Prince of Paris: If François is ready during your unlock
/// phase, he can burn 2 blood to lock a younger vampire. +1 bleed." Once
/// each unlock phase (rules-qa 6.6).
pub static FRANCOIS_VILLON: VampireDef = VampireDef {
    name: "François Villon",
    group: Some(6),
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[ModeDef {
        target: target(TargetKind::Minion, &[R::TargetVampire, R::Younger]),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[R::HostReady, R::HostBloodAtLeast(2)],
            &[Effect::HostBurnBlood(2), Effect::LockTarget],
        )
    }],
    ..VAMP
};

/// "Camarilla Prince of Bogota: Methuselahs casting (including controlling
/// a minion casting) votes or ballots against a referendum called by Marcos
/// burn 1 pool once results are tallied." Once each Methuselah, however
/// many of their sources voted against; at the tally, all at once (rules-qa
/// 51.1).
pub static MARCOS_BELEGRAD: VampireDef = VampireDef {
    name: "Marcos Belegrad",
    printed: &[own(Stat::AgainstMethuselahsBurnPool, 1, &[])],
    ..VAMP
};

/// "Camarilla Prince of Gary: During the polling step of any referendum,
/// Modius can discard a master card to get +1 vote." Once a referendum; in
/// torpor too (LSJ 20011022), though a vampire in torpor casts no vote.
pub static MODIUS: VampireDef = VampireDef {
    name: "Modius",
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Type(CardType::Master),
        }),
        limit: Some(Limit::PerReferendum),
        ..mode_of(
            &[Window::Referendum],
            Actor::Host,
            &[
                R::AnyPollingStep,
                R::HandHas(CardFilter::Type(CardType::Master), 1),
            ],
            &[Effect::AddVotes(1)],
        )
    }],
    ..VAMP
};

/// "Camarilla: During your unlock phase, you can move 1 blood from a prince
/// you control to Silvian." Once each unlock phase; in torpor too (LSJ
/// 20011022).
pub static SILVIAN_POPESCU: VampireDef = VampireDef {
    name: "Silvian Popescu",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                R::Controlled,
                R::TargetTitleIn(&[TitleClass::Prince]),
                R::BloodAtLeast(1),
            ],
        ),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[],
            &[Effect::TargetBloodToHost(1)],
        )
    }],
    ..VAMP
};

/// "Sabbat: During your unlock phase, an archbishop you control can move 1
/// blood from themselves to Tabitha." As Silvian Popescu's: once each
/// unlock phase, in torpor too (no "if ready"); an archbishop only while
/// Sabbat (the title's rule). An owner-added legacy card.
pub static TABITHA_FISK: VampireDef = VampireDef {
    name: "Tabitha Fisk",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                R::Controlled,
                R::TargetTitleIn(&[TitleClass::Archbishop]),
                R::BloodAtLeast(1),
            ],
        ),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[],
            &[Effect::TargetBloodToHost(1)],
        )
    }],
    ..VAMP
};

/// "Camarilla Prince of Houston: +1 bleed." Delilah Easton and Isabel de
/// Leon ("Camarilla.") print no text. Owner-added legacy cards.
pub static TATIANA_ROMANOV: VampireDef = VampireDef {
    name: "Tatiana Romanov",
    printed: &[own(Stat::Bleed, 1, &[])],
    ..VAMP
};

// ---- Sabbat V5: Path of Power and the Inner Voice ---------------------------

/// "Sabbat cardinal: Aaradhya can unlock after performing a successful
/// political action (even if the referendum failed). +1 bleed." (LSJ
/// 20011022: usable in torpor, though she acts ready.)
pub static AARADHYA: VampireDef = VampireDef {
    name: "Aaradhya, The Callous Tyrant",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[mode_of(
        &[Window::ActionAfterResolution],
        Actor::Host,
        &[
            R::ActorActing,
            R::IsPolitical,
            R::ActionSuccessful,
            R::HostLocked,
        ],
        &[Effect::UnlockActor],
    )],
    ..VAMP
};

/// "Sabbat Archbishop of Pittsburgh: While you control a ready cardinal,
/// Damian gets +1 bleed."
pub static DAMIAN: VampireDef = VampireDef {
    name: "Damian",
    printed: &[own(
        Stat::Bleed,
        1,
        &[R::ControlsReadyTitledOther(&[TitleClass::Cardinal])],
    )],
    ..VAMP
};

/// "Sabbat priscus: While your predator controls more ready minions than
/// you, you get +1 hand size." In torpor too (LSJ 20011022).
pub static KHIN_AYE: VampireDef = VampireDef {
    name: "Khin Aye",
    printed: &[own(Stat::HandSize, 1, &[R::PredatorMoreReadyMinions])],
    ..VAMP
};

/// "Sabbat Archbishop of Geneva: Allies and vampires with capacity 3 or
/// less cannot block Rexton." In torpor too (LSJ 20011022).
pub static REXTON: VampireDef = VampireDef {
    name: "Rexton \"Savage\" Abernathy",
    printed: &[
        own(Stat::AlliesCannotBlock, 1, &[]),
        own(Stat::SmallVampiresCannotBlock, 3, &[]),
    ],
    ..VAMP
};

/// "Sabbat Archbishop of Budapest: While your prey has 10 or fewer pool,
/// Üresség gets +1 vote and +1 bleed."
pub static URESSEG: VampireDef = VampireDef {
    name: "Üresség",
    printed: &[
        own(Stat::Votes, 1, &[R::PreyPoolAtMost(10)]),
        own(Stat::Bleed, 1, &[R::PreyPoolAtMost(10)]),
    ],
    ..VAMP
};

/// "Sabbat Archbishop of Ostrava: Once each turn, after resolution of an
/// action performed by Věnceslava during which your prey burned 1 or more
/// pool, you can gain 1 pool." Not if the referendum's effects ousted her
/// controller (ANK 20220808): the action is over. "Burned": any pool the
/// prey lost, costs paid included (rules-qa 52.4).
pub static VENCESLAVA: VampireDef = VampireDef {
    name: "Věnceslava, The Implacable",
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::ActionAfterResolution],
            Actor::Host,
            &[R::ActorActing, R::PreyBurnedPool],
            &[Effect::GainPool(1)],
        )
    }],
    ..VAMP
};

// ---- Sabbat V5: Path of Cathari --------------------------------------------
//
// Coco Thorn, Cyan Sin, Glissando and Ximena print no text ("Sabbat.").

/// "Sabbat cardinal: If Aelswith is blocked, the blocking minion's
/// controller burns 1 pool before block resolution." Applied first of the
/// "if blocked" effects (ANK 20220116); in torpor too (LSJ 20011022).
pub static AELSWITH: VampireDef = VampireDef {
    name: "Aelswith, The Irresistible",
    riders: &[Rider {
        hook: Hook::HostBlocked,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::BlockerControllerBurnsPool(1)],
    }],
    ..VAMP
};

/// "Sabbat Archbishop of Gdansk: Agnieszka can burn 1 blood to strike:
/// combat ends. +1 bleed."
pub static AGNIESZKA: VampireDef = VampireDef {
    name: "Agnieszka, Tempter of Legions",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[mode_of(
        &[Window::CombatStrikeDeclaration],
        Actor::Host,
        &[R::HostBloodAtLeast(1)],
        &[
            Effect::HostBurnBlood(1),
            Effect::Strike(StrikeKind::CombatEnds { unlock: false }),
        ],
    )],
    ..VAMP
};

/// "Sabbat: While you have 5 or fewer pool, Devorah gets +1 stealth." In
/// torpor too (LSJ 20011022): her leave-torpor action.
pub static DEVORAH: VampireDef = VampireDef {
    name: "Devorah",
    printed: &[own(Stat::Stealth, 1, &[R::PoolAtMost(5)])],
    ..VAMP
};

/// "Sabbat: After Fotini successfully bleeds (for 1 or more), you get +1
/// hand size until your next discard phase." Until it begins (rules-qa
/// 53.4).
pub static FOTINI_KATSIKARIS: VampireDef = VampireDef {
    name: "Fotini Katsikaris",
    riders: &[Rider {
        hook: Hook::OwnBleedSuccessful,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::HandSizeUntilDiscard(1)],
    }],
    ..VAMP
};

/// "Sabbat: After Gostoso successfully bleeds your prey (for 1 or more),
/// he can gain 1 blood." Once, after that bleed (owner, 2026-10-09).
pub static GOSTOSO: VampireDef = VampireDef {
    name: "Gostoso",
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionAfterResolution],
            Actor::Host,
            &[R::ActorActing, R::BleedAgainstPrey, R::BleedSuccessful],
            &[Effect::AddHostBlood(1)],
        )
    }],
    ..VAMP
};

/// "Sabbat Archbishop of Mannheim: Minions must burn 1 blood or life to
/// attempt to block Jürgen." In torpor too (LSJ 20011022).
pub static JURGEN: VampireDef = VampireDef {
    name: "Jürgen, The Libertine",
    printed: &[Printed {
        stat: Stat::BlockToll,
        n: 1,
        to: PrintedTo::Opponent,
        per: Per::One,
        requires: &[],
    }],
    ..VAMP
};

/// "Sabbat: Osvaldo gets +1 bleed against a Methuselah controlling a
/// Toreador or Ventrue." In torpor too: controlled (rules-qa 53.8).
pub static OSVALDO_KUHNEMANN: VampireDef = VampireDef {
    name: "Osvaldo Kühnemann",
    printed: &[own(
        Stat::Bleed,
        1,
        &[R::TargetSeatControlsClan(&["Toreador", "Ventrue"])],
    )],
    ..VAMP
};

/// "Sabbat bishop: Verrix gets +1 stealth during bleed actions."
pub static VERRIX: VampireDef = VampireDef {
    name: "Verrix, Naughty Boy",
    printed: &[own(Stat::Stealth, 1, &[R::IsBleed])],
    ..VAMP
};

// ---- Sabbat V5: Path of Caine ----------------------------------------------
//
// Berenguela, Grażyna, Hekla, Lawbringer, Loyiso, Przemysław and Surapong
// print no text ("Sabbat.", "Sabbat bishop.").

/// "Sabbat bishop: Strike cards cost opposing younger vampires +1 blood." A
/// strike granted later is paid too (LSJ 20090529); paid as declared (LSJ
/// 20090703).
pub static ALGIRDAS: VampireDef = VampireDef {
    name: "Algirdas, The Solar Prophet",
    printed: &[Printed {
        stat: Stat::StrikeCardToll,
        n: 1,
        to: PrintedTo::Opponent,
        per: Per::One,
        requires: &[R::ActorVampire, R::OpponentOlder],
    }],
    ..VAMP
};

/// "Sabbat bishop: +1 strength."
pub static ANOUSHA: VampireDef = VampireDef {
    name: "Anousha, The Maniac",
    printed: &[own(Stat::Strength, 1, &[])],
    ..VAMP
};

/// "Sabbat bishop: Once each round of combat, Crossbreaker can burn 1 blood
/// to make the damage from his hand strikes aggravated that round."
pub static CROSSBREAKER: VampireDef = VampireDef {
    name: "Crossbreaker",
    abilities: &[ability(
        &[Window::CombatUntimed],
        Actor::Host,
        &[R::HostBloodAtLeast(1)],
        Limit::PerRound,
        &[Effect::HostBurnBlood(1), Effect::AggravatedHands],
    )],
    ..VAMP
};

/// "Sabbat bishop: Kalyani gets -1 stealth when hunting. +1 strength."
pub static KALYANI: VampireDef = VampireDef {
    name: "Kalyani, Agent of Doom",
    printed: &[
        own(Stat::Stealth, -1, &[R::IsHunt]),
        own(Stat::Strength, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat: Serhat gets 1 optional press each combat."
pub static SERHAT_GUNDE: VampireDef = VampireDef {
    name: "Serhat Gunde, Combat Junkie",
    printed: &[own(Stat::CombatPress, 1, &[])],
    ..VAMP
};

// ---- Sabbat V5: Path of Death and the Soul ---------------------------------
//
// Aitana, El Monseñor, Nadezhda, Yurena and Zbigniew print no text
// ("Sabbat.").

/// "Sabbat: During an action, Abderrahim can burn 1 blood to give an ally or
/// younger vampire you control +1 stealth. +1 strength." In torpor too (LSJ
/// 20011022); when stealth is needed.
pub static ABDERRAHIM: VampireDef = VampireDef {
    name: "Abderrahim, Death's Hand",
    // "During an action": once in it (6.6).
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionDuring],
            Actor::Host,
            &[
                R::HostBloodAtLeast(1),
                R::OwnActingAllyOrYounger,
                R::StealthNeeded,
            ],
            &[Effect::HostBurnBlood(1), Effect::AddStealth(1)],
        )
    }],
    printed: &[own(Stat::Strength, 1, &[])],
    ..VAMP
};

/// "Sabbat: While you have the Edge, Anxo gets +1 strength."
pub static ANXO_VILELA: VampireDef = VampireDef {
    name: "Anxo Vilela",
    printed: &[own(Stat::Strength, 1, &[R::HasEdge])],
    ..VAMP
};

/// "Sabbat cardinal: While Ashur-uballit is ready, zombies (allies and
/// retainers) you recruit or employ get +1 starting life. +1 bleed." Read
/// as the zombie enters play.
pub static ASHUR_UBALLIT: VampireDef = VampireDef {
    name: "Ashur-uballit, Son of the Abyss",
    printed: &[
        own(Stat::ZombieStartingLife, 1, &[R::ThisVampireReady]),
        own(Stat::Bleed, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat: Eulogio can look at and reorder the top 5 cards of your
/// library, and unlock as a +1 stealth action that costs 1 blood." Both at
/// resolution, the blood too: not paid if blocked (rules-qa 55.3).
pub static EULOGIO: VampireDef = VampireDef {
    name: "Eulogio Sánchez de los Reyes",
    abilities: &[printed_action(
        None,
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::PayBlood(1),
            Effect::ReorderTop { n: 5, unlock: true },
        ],
    )],
    ..VAMP
};

/// "Sabbat: Frau Schädel gets +1 stealth during recruit and employ
/// actions."
pub static FRAU_SCHADEL: VampireDef = VampireDef {
    name: "Frau Schädel",
    printed: &[own(Stat::Stealth, 1, &[R::IsRecruitOrEmploy])],
    ..VAMP
};

/// "Sabbat: Once each combat involving another minion you control, if Huldu
/// is ready, she can burn 1 blood to prevent 1 damage to that minion. +1
/// bleed."
pub static HULDU: VampireDef = VampireDef {
    name: "Huldu, The Desecrator",
    abilities: &[ability(
        &[Window::CombatDamagePrevention],
        Actor::ForOwnCombatant,
        &[
            R::HostReady,
            R::HostOutOfCombat,
            R::HostBloodAtLeast(1),
            R::DamagePending,
        ],
        Limit::PerCombat,
        &[Effect::HostBurnBlood(1), Effect::Prevent(1)],
    )],
    printed: &[own(Stat::Bleed, 1, &[])],
    ..VAMP
};

/// "Sabbat: During your turn, Sakura can burn 1 blood to unlock after
/// performing a successful action requiring the Path of Death and the
/// Soul." An action card (or a card's granted action) with the path icon.
pub static SAKURA: VampireDef = VampireDef {
    name: "Sakura, The Merciless",
    // "During your turn": once in it (the "during" template, 6.6).
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::ActionAfterResolution],
            Actor::Host,
            &[
                R::OwnTurn,
                R::ActorActing,
                R::ActionSuccessful,
                R::ActionRequiresPath("Death and the Soul"),
                R::HostBloodAtLeast(1),
                R::HostLocked,
            ],
            &[Effect::HostBurnBlood(1), Effect::UnlockHost],
        )
    }],
    ..VAMP
};

// ---- TWDA unlocks: staples and steps 1-3 -----------------------------------

/// "Anarch: As the convert enters play, you can remove him from the game to
/// make a non-titled vampire you control Anarch and either gain 1 pool or
/// draw 1 card from your crypt." Engine-side, as he is moved out of the
/// uncontrolled region (`Interrupt::AnarchConvert`): before any contest,
/// beside a copy in play too (LSJ 20080526, RTR 20180303); an Anarch
/// already may be made so (PIB 20121031).
pub static ANARCH_CONVERT: VampireDef = VampireDef {
    name: "Anarch Convert",
    ..VAMP
};

/// "Camarilla Nosferatu Justicar: Alonso inflicts 1 unpreventable damage on
/// each anarch controlled by the target of the bleed after bleeding
/// successfully. +1 bleed." Each Anarch vampire of the bled Methuselah, in
/// torpor too.
pub static ALONSO_PETRODON: VampireDef = VampireDef {
    name: "Alonso Petrodon",
    printed: &[own(Stat::Bleed, 1, &[])],
    riders: &[Rider {
        hook: Hook::OwnBleedSuccessful,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::DamageBledAnarchs(1)],
    }],
    ..VAMP
};

/// "Camarilla primogen: If Maila is ready during your turn, you can discard
/// a political action card to give a minion you control +1 stealth for the
/// current action." Once each of your turns (ANK 20230122); when stealth is
/// needed (rules-qa 3.2).
pub static MAILA: VampireDef = VampireDef {
    name: "Maila",
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Type(CardType::PoliticalAction),
        }),
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::ActionDuring],
            Actor::Host,
            &[
                R::HostReady,
                R::OwnTurn,
                R::OwnActing,
                R::StealthNeeded,
                R::HandHas(CardFilter::Type(CardType::PoliticalAction), 1),
            ],
            &[Effect::AddStealth(1)],
        )
    }],
    ..VAMP
};

/// "Anarch: Once each combat, Gabriel can discard a card requiring an Anarch
/// to get +1 strength that combat. -1 stealth."
pub static GABRIEL_TREMBLAY: VampireDef = VampireDef {
    name: "Gabriel Tremblay",
    printed: &[own(Stat::Stealth, -1, &[])],
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::HasLabel(Label::RequiresAnarch),
        }),
        limit: Some(Limit::PerCombat),
        ..mode_of(
            &[Window::CombatUntimed],
            Actor::Host,
            &[R::HandHas(CardFilter::HasLabel(Label::RequiresAnarch), 1)],
            &[Effect::StrengthThisCombat(1)],
        )
    }],
    ..VAMP
};

// ---- TWDA unlocks: steps 4-9 -----------------------------------------------
//
// Benedito, a Lanterna, Jacques Rouge and Tara print no text but their
// title.

/// "Sabbat. Black Hand. Seraph: Elimelech strikes with first strike. +1
/// bleed. +1 strength." Black Hand and Seraph are generated traits (Tattoo
/// Signal, Corporal Reservoir read them).
pub static ELIMELECH: VampireDef = VampireDef {
    name: "Elimelech the Twice-Damned",
    printed: &[
        own(Stat::FirstStrike, 1, &[]),
        own(Stat::Bleed, 1, &[]),
        own(Stat::Strength, 1, &[]),
    ],
    ..VAMP
};

/// "Camarilla primogen: Reginald gets +1 strength in combat with a Tremere
/// or Tremere antitribu. He cannot block vampires of those clans or take
/// actions to enter combat with them." Read on the opposing minion's clan,
/// live; his rushes are action cards (he prints none).
pub static REGINALD_MOORE: VampireDef = VampireDef {
    name: "Reginald Moore",
    printed: &[
        own(Stat::Strength, 1, &[R::OpponentClan("Tremere")]),
        own(Stat::Strength, 1, &[R::OpponentClan("Tremere antitribu")]),
        own(Stat::NoBlockOrRush, 1, &[R::OpponentClan("Tremere")]),
        own(
            Stat::NoBlockOrRush,
            1,
            &[R::OpponentClan("Tremere antitribu")],
        ),
    ],
    ..VAMP
};

/// "Independent: Valkyrie cannot perform directed actions unless Brunhilde
/// is ready. Non-unique. +1 strength." Any Brunhilde in play, any
/// Methuselah's.
pub static VALKYRIE: VampireDef = VampireDef {
    name: "Valkyrie",
    printed: &[
        own(Stat::NoDirectedActions, 1, &[R::NoReadyNamed("Brunhilde")]),
        own(Stat::Strength, 1, &[]),
    ],
    ..VAMP
};

/// "Anarch: During your turn, you can burn the Edge to add 1 blood to an
/// Anarch you control." Once during the controller's turn (rules-qa 58.7,
/// the "during" template 6.6); any Anarch of
/// theirs, in torpor too, capped at capacity.
pub static THE_MEDIC: VampireDef = VampireDef {
    name: "Medic, The",
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[R::Controlled, R::TargetVampire, R::TargetAnarch],
        }),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[R::OwnTurn, R::HasEdge],
            &[Effect::BurnEdge, Effect::BloodToTarget(1)],
        )
    }],
    ..VAMP
};

/// "Anarch Baron of Stockholm: The action to become Anarch costs vampires
/// you control 0 blood. [MERGED] Brunhilde can enter combat with a
/// non-Anarch vampire as a Ⓓ action." Her base card is not in the pool:
/// no merge, the [MERGED] text never applies.
pub static BRUNHILDE: VampireDef = VampireDef {
    name: "Brunhilde",
    adv: true,
    printed: &[Printed {
        stat: Stat::BecomeAnarchFree,
        n: 1,
        to: PrintedTo::OthersControlled,
        per: Per::One,
        requires: &[],
    }],
    ..VAMP
};

/// "Independent. 1 vote (titled): During the polling step of any
/// referendum, Arishat can burn 1 blood to force a vampire to abstain.
/// Infernal." Once a referendum, while locked or in torpor too (ANK
/// 20220705, LSJ 20011022); her own, prisci's and cast votes and ballots
/// included — those already cast are canceled (LSJ 19971001, LSJ
/// 20100311). Her title survives a sect change (LSJ 20010714); infernal is
/// the crypt card's trait.
pub static ARISHAT: VampireDef = VampireDef {
    name: "Arishat",
    abilities: &[ModeDef {
        target: target(TargetKind::Minion, &[R::TargetVampire, R::NotAbstaining]),
        limit: Some(Limit::PerReferendum),
        ..mode_of(
            &[Window::Referendum],
            Actor::Host,
            &[R::AnyPollingStep, R::HostBloodAtLeast(1)],
            &[
                Effect::HostBurnBlood(1),
                Effect::Abstain {
                    lock: false,
                    burn: 0,
                },
            ],
        )
    }],
    ..VAMP
};

/// "Anarch: During the polling step of any referendum, Danielle can burn 1
/// blood to get +1 vote. [MERGED] Baron of Berlin." Once a referendum,
/// while locked too (ANK 20220705); not a title. Her base card is not in
/// the pool: the [MERGED] text never applies.
pub static DANIELLE_DIRON: VampireDef = VampireDef {
    name: "Danielle Diron",
    adv: true,
    abilities: &[ModeDef {
        limit: Some(Limit::PerReferendum),
        ..mode_of(
            &[Window::Referendum],
            Actor::Host,
            &[R::AnyPollingStep, R::HostBloodAtLeast(1)],
            &[Effect::HostBurnBlood(1), Effect::AddVotes(1)],
        )
    }],
    ..VAMP
};

/// "Independent. 2 votes (titled): During your unlock phase, Scout can burn
/// 2 blood to lock an ally or younger vampire controlled by your prey.
/// Reaction cards cost her +1 blood." Once each unlock phase (rules-qa
/// 6.6), in torpor too (no "if ready"); a ready target; a card of two types
/// costs more only played as a reaction (PIB 20150924-2). Her title
/// survives a sect change (LSJ 20010714).
pub static SCOUT_YOUNGWOOD: VampireDef = VampireDef {
    name: "Scout Youngwood",
    printed: &[own(Stat::BloodCostMore, 1, &[R::PlayedAsReaction])],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[R::ControlledByPrey, R::AllyOrYounger, R::ActorReady],
        ),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[R::HostBloodAtLeast(2)],
            &[Effect::HostBurnBlood(2), Effect::LockTarget],
        )
    }],
    ..VAMP
};

/// "Camarilla Prince of Houston: While Karen is a prince, Gehenna events do
/// not affect her. She gets +1 stealth on undirected actions." Read live on
/// her title. Chosen by an event, she stays unaffected; she takes no action
/// an event provides; she keeps her sect and title through a Fall (LSJ
/// 20100121, LSJ 20081120-2). Not the things events affect around her:
/// allies' intercept, locations on her, her controller's pool for her in
/// torpor (Dragonbound).
pub static KAREN_SUADELA: VampireDef = VampireDef {
    name: "Karen Suadela",
    printed: &[
        own(
            Stat::GehennaImmune,
            1,
            &[R::ActorClaimsTitleIn(&[TitleClass::Prince])],
        ),
        own(Stat::Stealth, 1, &[R::ActionUndirected]),
    ],
    ..VAMP
};

static BENJAMIN_ROSE_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[],
    effect: RefEffect::CallerHandSizeForGame(1),
    grants: &[],
    non_anarch_titles_minus: false,
    titled_minus: false,
    edge_bonus: 0,
    only_sect: None,
};

/// "Camarilla Prince of Washington, D.C.: Benjamin can call a referendum to
/// give you +1 hand size until the end of the game as a +1 stealth
/// political action." A granted political action (no card vote, rules-qa
/// 6.2); "you" his controller as it passes; each pass adds 1, kept after he
/// leaves play.
pub static BENJAMIN_ROSE: VampireDef = VampireDef {
    name: "Benjamin Rose",
    abilities: &[mode_of(
        &[Window::ActionDeclarePolitical],
        Actor::Host,
        &[],
        &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::AddStealth(1),
            Effect::CallReferendum(&BENJAMIN_ROSE_REFERENDUM),
        ],
    )],
    ..VAMP
};

static DMITRA_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::DistributeBlood {
        points: crate::cards::Points::Fixed(4),
        per_max: 0,
        filter: &[R::TargetVampire, R::ActorReady],
    }],
    effect: RefEffect::BloodToChosenEach,
    grants: &[],
    non_anarch_titles_minus: false,
    titled_minus: false,
    edge_bonus: 0,
    only_sect: None,
};

/// "Camarilla Brujah Justicar: Dmitra can call a referendum to distribute 4
/// blood from the blood bank among one or more ready vampires as a +1
/// stealth political action." The 4 picked as terms (rules-qa 3.5), any
/// Methuselah's ready vampires, one picked again as often as wished; at
/// resolution a vampire no longer ready gets nothing, and blood over
/// capacity is lost.
pub static DMITRA_ILYANOVA: VampireDef = VampireDef {
    name: "Dmitra Ilyanova",
    abilities: &[mode_of(
        &[Window::ActionDeclarePolitical],
        Actor::Host,
        &[],
        &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::AddStealth(1),
            Effect::CallReferendum(&DMITRA_REFERENDUM),
        ],
    )],
    ..VAMP
};

// ---- TWDA unlocks: steps 10-15 ---------------------------------------------
//
// Dr. Allan Woodstock and Graham Gottesman print no text but their sect and
// title.

/// "Independent: +1 intercept."
pub static NADIMA: VampireDef = VampireDef {
    name: "Nadima",
    printed: &[own(Stat::Intercept, 1, &[])],
    ..VAMP
};

/// "Camarilla primogen: Allies requiring a Gangrel cost Inyanga 1 fewer
/// blood or pool. +1 intercept." Off whichever the card costs (as Kuyén).
pub static INYANGA: VampireDef = VampireDef {
    name: "Inyanga",
    printed: &[
        own(
            Stat::CostOff,
            1,
            &[
                R::PlayedCard(CardFilter::Type(CardType::Ally)),
                R::PlayedCard(CardFilter::Clan("Gangrel")),
            ],
        ),
        own(Stat::Intercept, 1, &[]),
    ],
    ..VAMP
};

/// "Independent: After a minion in combat with Matasuntha leaves the ready
/// region, you can gain 1 pool. +2 strength." Optional; even if she leaves
/// in the same event (advanced rules §6.4.4).
pub static MATASUNTHA: VampireDef = VampireDef {
    name: "Matasuntha",
    printed: &[own(Stat::Strength, 2, &[])],
    riders: &[Rider {
        hook: Hook::CombatOpponentLeftReady,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::MayGainPool(1)],
    }],
    ..VAMP
};

/// "Camarilla: While Gerald has no title, he gets +1 vote for each ready
/// titled vampire controlled by other Methuselahs. +1 stealth." Counted
/// live as he votes.
pub static GERALD_WINDHAM: VampireDef = VampireDef {
    name: "Gerald Windham",
    printed: &[
        Printed {
            stat: Stat::Votes,
            n: 1,
            to: PrintedTo::Itself,
            per: Per::ReadyTitledOfOthers,
            requires: &[R::ActorUntitled],
        },
        own(Stat::Stealth, 1, &[]),
    ],
    ..VAMP
};

/// "Camarilla Prince of Chicago: Once each combat, Lodin can prevent 1
/// damage from an opposing minion's strike."
pub static LODIN: VampireDef = VampireDef {
    name: "Lodin (Olaf Holte)",
    abilities: &[ability(
        &[Window::CombatDamagePrevention],
        Actor::Host,
        &[R::StrikeDamagePending],
        Limit::PerCombat,
        &[Effect::PreventStrike {
            n: 1,
            aggravated: true,
        }],
    )],
    ..VAMP
};

/// "Independent. 1 vote (titled): Once each minion phase, Randall can burn 1
/// blood to prevent 1 non-aggravated damage inflicted on any other minion
/// in combat." Out of the combat, any Methuselah's combatant, in torpor too
/// (a printed ability with no "if ready"); one minion phase a turn.
pub static RANDALL: VampireDef = VampireDef {
    name: "Randall",
    abilities: &[ModeDef {
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[R::TargetInCombatWithNormalDamage],
        }),
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::CombatDamagePrevention],
            Actor::HostOutOfCombat,
            &[R::HostBloodAtLeast(1)],
            &[
                Effect::HostBurnBlood(1),
                Effect::PreventNonAggravatedOnTarget(1),
            ],
        )
    }],
    ..VAMP
};

/// "Independent: Fakir can burn 3 blood during the polling step of a
/// political action before votes and ballots are cast to have the
/// referendum fail. He gets 1 optional press each combat. +1 bleed. +1
/// strength."
pub static FAKIR_AL_SIDI: VampireDef = VampireDef {
    name: "Fakir al Sidi",
    printed: &[
        own(Stat::CombatPress, 1, &[]),
        own(Stat::Bleed, 1, &[]),
        own(Stat::Strength, 1, &[]),
    ],
    abilities: &[ModeDef {
        limit: Some(Limit::PerReferendum),
        ..mode_of(
            &[Window::Referendum],
            Actor::Host,
            &[R::PollingStep, R::NoVotesCast, R::HostBloodAtLeast(3)],
            &[Effect::HostBurnBlood(3), Effect::FailReferendum],
        )
    }],
    ..VAMP
};

/// "Independent: T.J. may lock and burn 2 blood to cancel a referendum
/// called by a titled Camarilla vampire." In the polling step; the caller
/// read as he uses it.
pub static TJ: VampireDef = VampireDef {
    name: "T.J.",
    abilities: &[mode_of(
        &[Window::Referendum],
        Actor::Host,
        &[
            R::AnyPollingStep,
            R::HostUnlocked,
            R::HostBloodAtLeast(2),
            R::CallerTitledOfSect(Sect::Camarilla),
        ],
        &[
            Effect::LockActor,
            Effect::HostBurnBlood(2),
            Effect::CancelReferendum {
                unlock_acting: false,
                forbid_repeat: false,
            },
        ],
    )],
    ..VAMP
};

/// "Independent: Del Georgio can remove two copies of the same card in your
/// prey's ash heap from the game to hunt with +2 hunt as a +1 stealth
/// action." A hunt action; the card named as it is announced, both copies
/// removed as it resolves (its cost, rules-qa 54.4) — fewer by then, it
/// fizzles.
pub static GIOVANNI_DEL_GEORGIO: VampireDef = VampireDef {
    name: "Giovanni del Georgio",
    abilities: &[ModeDef {
        terms: Some(Terms::PreyAshPair),
        limit: Some(Limit::PerTurn),
        requires: &[R::PreyAshHasPair],
        ..printed_action(
            None,
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::HuntAction,
                Effect::PreyAshPairHunt(2),
            ],
        )
    }],
    ..VAMP
};

/// "Camarilla Ventrue Justicar: Mary Anne can add 2 blood to a vampire in
/// your uncontrolled region as a +1 stealth action. While she is ready,
/// Inner Circle members get -2 bleed." Any Methuselah's Inner Circle
/// members, read live.
pub static MARY_ANNE_BLAIRE: VampireDef = VampireDef {
    name: "Mary Anne Blaire",
    printed: &[Printed {
        stat: Stat::Bleed,
        n: -2,
        to: PrintedTo::Others,
        per: Per::One,
        requires: &[
            R::ThisVampireReady,
            R::ActorTitleIn(&[TitleClass::InnerCircle]),
        ],
    }],
    abilities: &[printed_action(
        Some(TargetSpec {
            kind: TargetKind::Uncontrolled,
            filter: &[],
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::BloodToUncontrolledTarget(2),
        ],
    )],
    ..VAMP
};

/// "Camarilla. 2 votes (titled): Karl can play cards requiring a justicar
/// as if he were a justicar. +1 intercept." The cards' requirement only;
/// he is no justicar otherwise.
pub static KARL_SCHREKT: VampireDef = VampireDef {
    name: "Karl Schrekt",
    printed: &[
        own(Stat::PlaysAsJusticar, 1, &[]),
        own(Stat::Intercept, 1, &[]),
    ],
    ..VAMP
};

/// "Camarilla Prince of Atlanta: If a card named /Golconda: Inner Peace/
/// you play is canceled, you can move that card from your ash heap to your
/// library (shuffle afterward)." However canceled (its own 2-pool cancel,
/// Wash); Victor in play, in torpor too; optional.
pub static VICTOR_DONALDSON: VampireDef = VampireDef {
    name: "Victor Donaldson",
    printed: &[own(
        Stat::RecyclesCanceled,
        1,
        &[R::PlayedCard(CardFilter::Named("Golconda: Inner Peace"))],
    )],
    ..VAMP
};

/// "Sabbat: Once each turn during a bleed against you, if Aksinya is ready
/// and unlocked, you may discard a master card from your hand to redirect
/// the bleed to any other Methuselah." After blocks are declined, a failed
/// attempt against stealth included (LSJ 20000507, ANK 20211003), before
/// the "would be successful" plays (LSJ 19980105); a legal bleed target
/// only (PIB 20130711); once in each Methuselah's turn (ANK 20180719-4).
/// The master is discarded as the cost, replaced as Modius' is.
pub static AKSINYA_DACLAU: VampireDef = VampireDef {
    name: "Aksinya Daclau",
    abilities: &[ModeDef {
        target: Some(TargetSpec {
            kind: TargetKind::Seat,
            filter: &[R::NotActingController, R::NotSelf],
        }),
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Type(CardType::Master),
        }),
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::ActionBlocksDeclined],
            Actor::Host,
            &[
                R::DirectedAtYou,
                R::IsBleed,
                R::BlocksDeclined,
                R::HostReady,
                R::HostUnlocked,
                R::HandHas(CardFilter::Type(CardType::Master), 1),
            ],
            &[Effect::RedirectBleed],
        )
    }],
    ..VAMP
};

// ---- TWDA unlocks: steps 16-20 ---------------------------------------------

/// "Camarilla: Theo may enter combat with any ready minion controlled by
/// another Methuselah as a Ⓓ action. If you control a ready prince or
/// justicar, blood hunts cannot be called on Theo." The G2 card; the
/// prince or justicar another vampire of his Methuselah's, read live.
pub static THEO_BELL_G2: VampireDef = VampireDef {
    name: "Theo Bell",
    group: Some(2),
    printed: &[own(
        Stat::NoBloodHunt,
        1,
        &[R::ControlsReadyTitledOther(&[
            TitleClass::Prince,
            TitleClass::Justicar,
        ])],
    )],
    abilities: &[enter_combat(&[R::NotControlled, R::ActorReady])],
    ..VAMP
};

/// "Camarilla Malkavian Justicar: After a referendum called by Juliet
/// passes, you get +2 hand size until your next unlock phase. Juliet gets
/// 1 optional maneuver each combat."
pub static JULIET_PARR: VampireDef = VampireDef {
    name: "Juliet Parr",
    abilities: &[MANEUVER_EACH_COMBAT],
    riders: &[Rider {
        hook: Hook::OwnReferendumPassed,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::HandSizeUntilUnlock(2)],
    }],
    ..VAMP
};

/// "Camarilla Tremere Inner Circle: Mistress Fanchon can search your
/// library for a minion card, reveal it, and move it to your hand as a +1
/// stealth action. +2 bleed." A minion card: any library card but a master
/// or an event (rules-qa 60.3).
pub static MISTRESS_FANCHON: VampireDef = VampireDef {
    name: "Mistress Fanchon",
    printed: &[own(Stat::Bleed, 2, &[])],
    abilities: &[printed_action(
        None,
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::SearchToHand {
                filter: CardFilter::MinionCard,
                shown: true,
            },
        ],
    )],
    ..VAMP
};

/// "Sabbat: Beast cannot play action cards. He cannot have or use equipment
/// or retainers. Beast may enter combat with any ready minion controlled by
/// another Methuselah as a Ⓓ action. +1 strength." No equip or employ
/// action, no strike stealing equipment or retainers (LSJ 20050315, LSJ
/// 20010210).
pub static BEAST: VampireDef = VampireDef {
    name: "Beast, The Leatherface of Detroit",
    printed: &[
        own(Stat::NoActionCards, 1, &[]),
        own(Stat::NoEquipmentOrRetainers, 1, &[]),
        own(Stat::Strength, 1, &[]),
    ],
    abilities: &[enter_combat(&[R::NotControlled, R::ActorReady])],
    ..VAMP
};

/// "Camarilla: Cards requiring Blood Sorcery [tha] cost Lord Tremere 1
/// fewer blood. During your minion phase, you can lock or unlock another
/// ready Tremere or Tremere antitribu. +2 bleed. +1 intercept." In torpor
/// too (LSJ 20011022); any Methuselah's Tremere.
pub static LORD_TREMERE: VampireDef = VampireDef {
    name: "Lord Tremere",
    printed: &[
        own(
            Stat::BloodCostOff,
            1,
            &[R::PlayedCard(CardFilter::Discipline("tha"))],
        ),
        own(Stat::Bleed, 2, &[]),
        own(Stat::Intercept, 1, &[]),
    ],
    // "During your minion phase": once in it (6.6, rules-qa 58.7) — the
    // lock or the unlock, whichever the vampire's state allows.
    abilities: &[ModeDef {
        target: target(TargetKind::Minion, LORD_TREMERE_TARGET),
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[R::OwnMinionPhase],
            &[Effect::ToggleLockTarget],
        )
    }],
    ..VAMP
};

/// "Another ready Tremere or Tremere antitribu", any Methuselah's.
const LORD_TREMERE_TARGET: &[R] = &[
    R::ActorReady,
    R::NotHost,
    R::AnyOf(&[
        &[R::TargetClan("Tremere")],
        &[R::TargetClan("Tremere antitribu")],
    ]),
];

/// "Independent: Once each turn, Nergal can reduce the cost of a card he
/// plays by 1 blood. Infernal. +2 bleed." His choice of card, announced with
/// it (rules-qa 60.7), once in each Methuselah's turn (ANK 20180719-4),
/// spent as the cost is paid — not by one canceled unpaid (LSJ 20091021-2).
pub static NERGAL: VampireDef = VampireDef {
    name: "Nergal",
    printed: &[
        own(Stat::BloodCostOffOncePerTurn, 1, &[]),
        own(Stat::Bleed, 2, &[]),
    ],
    ..VAMP
};

/// "Independent: If The Capuchin is burned, move him to his owner's
/// uncontrolled region and you get +3 transfers at the start of your next
/// influence phase. +1 bleed. +2 hand size. +1 intercept." A new vampire,
/// nothing kept, his owner's even if another controlled him, the transfers
/// that controller's (ANK 20200417).
pub static THE_CAPUCHIN: VampireDef = VampireDef {
    name: "Capuchin, The",
    printed: &[
        own(Stat::ToUncontrolledWhenBurned, 3, &[]),
        own(Stat::Bleed, 1, &[]),
        own(Stat::HandSize, 2, &[]),
        own(Stat::Intercept, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat: Jimmy Dunn cannot be contested; if a second Jimmy would enter
/// play, burn the first Jimmy in play instead of contesting him." A second
/// can be influenced out by his controller too (RTR 20180303).
pub static JIMMY_DUNN: VampireDef = VampireDef {
    name: "Jimmy Dunn",
    printed: &[own(Stat::NeverContested, 1, &[])],
    ..VAMP
};

/// "Sabbat priscus: Malgorzata may remove a vampire in your uncontrolled
/// region from the game to add 4 blood from the blood bank to another
/// vampire in your uncontrolled region as a +1 stealth action." Both named
/// as the action is announced, the removal at resolution with the blood
/// (either gone by then: nothing).
pub static MALGORZATA: VampireDef = VampireDef {
    name: "Malgorzata",
    abilities: &[ModeDef {
        requires: &[R::UncontrolledAtLeast(2)],
        ..printed_action(
            Some(TargetSpec {
                kind: TargetKind::Uncontrolled,
                filter: &[],
            }),
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::RemoveUncontrolledForBlood(4),
            ],
        )
    }],
    ..VAMP
};

/// Call the Great Beast become a vampire: "who can enter combat with a
/// minion as a Ⓓ action and can prevent 1 damage each combat." Any other
/// ready minion; any damage pending on it, once each combat.
pub static GREAT_BEAST: VampireDef = VampireDef {
    name: "Call the Great Beast",
    abilities: &[
        enter_combat(&[R::ActorReady, R::NotHost]),
        ability(
            &[Window::CombatDamagePrevention],
            Actor::Host,
            &[R::DamagePending],
            Limit::PerCombat,
            &[Effect::Prevent(1)],
        ),
    ],
    ..VAMP
};

// ---- TWDA unlocks, round 2: steps 1-6 --------------------------------------

/// "Camarilla Brujah Inner Circle: Combat cards cost Adana 1 fewer blood.
/// +2 bleed." A card played as a combat card, of several types or not
/// (PIB 20150924-2); blood only — never a "burn blood" effect in a card's
/// text (ANK 20210226).
pub static ADANA_DE_SFORZA: VampireDef = VampireDef {
    name: "Adana de Sforza",
    printed: &[
        own(Stat::BloodCostOff, 1, &[R::PlayedAsCombat]),
        own(Stat::Bleed, 2, &[]),
    ],
    ..VAMP
};

/// "Camarilla Toreador Inner Circle: Once during your turn, you may lock or
/// unlock another ready Toreador. +2 bleed." At any point of the turn — a
/// political action included (TOM 19960214) —, in torpor too (LSJ
/// 20011022), any Methuselah's Toreador (rules-qa 49.5): the lock or the
/// unlock, whichever its state allows (as Lord Tremere, rules-qa 60.6). A
/// minion attempting to block, locked so, fails its attempt (LSJ
/// 20010702).
pub static ALEXANDRA: VampireDef = VampireDef {
    name: "Alexandra",
    printed: &[own(Stat::Bleed, 2, &[])],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[R::ActorReady, R::NotHost, R::TargetClan("Toreador")],
        ),
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[R::OwnTurn],
            &[Effect::ToggleLockTarget],
        )
    }],
    ..VAMP
};

/// "Camarilla Prince of Paris: François may steal up to 2 blood from a
/// younger vampire as a +1 stealth Ⓓ action. +1 bleed." (Group 2; the G6
/// card is another text of the same vampire, rules-qa 60.1.) The amount
/// chosen at resolution, among what the target has (owner, rules-qa 62.4),
/// the excess over his capacity draining.
pub static FRANCOIS_VILLON_G2: VampireDef = VampireDef {
    name: "François Villon",
    group: Some(2),
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[printed_action(
        target(TargetKind::Minion, &[R::TargetVampire, R::Younger]),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::StealBloodUpTo(2),
        ],
    )],
    ..VAMP
};

/// "Sabbat: Goratrix can steal 2 blood as a ranged strike. +1 bleed." Blood
/// or life (ANK 20190701): not damage, dodged like any strike.
pub static GORATRIX: VampireDef = VampireDef {
    name: "Goratrix",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[mode_of(
        &[Window::CombatStrikeDeclaration],
        Actor::Host,
        &[],
        &[Effect::Strike(StrikeKind::StealBlood {
            n: 2,
            ranged: true,
        })],
    )],
    ..VAMP
};

pub static NEIGHBOR_JOHN: VampireDef = VampireDef {
    name: "Neighbor John",
    printed: &[own(Stat::Intercept, 1, &[])],
    ..VAMP
};

/// "Sabbat: Omaya may prevent one damage each combat." Any damage pending
/// on her, aggravated included, once over the combat (rules-qa 14.2).
pub static OMAYA: VampireDef = VampireDef {
    name: "Omaya",
    abilities: &[ability(
        &[Window::CombatDamagePrevention],
        Actor::Host,
        &[R::DamagePending],
        Limit::PerCombat,
        &[Effect::Prevent(1)],
    )],
    ..VAMP
};

/// "Camarilla Malkavian Justicar: While Unmada is ready, you get +1 discard
/// phase action at the start of your discard phase, and +1 hand size." The
/// action if she is ready as the phase begins (as Sreelekha); the hand size
/// read live.
pub static UNMADA: VampireDef = VampireDef {
    name: "Unmada",
    printed: &[
        own(Stat::DiscardPhaseActions, 1, &[R::ThisVampireReady]),
        own(Stat::HandSize, 1, &[R::ThisVampireReady]),
    ],
    ..VAMP
};

/// "Independent: The unnamed can burn 1 blood to strike: 2R aggravated
/// damage. After it successfully bleeds, you can gain 2 pool. Infernal."
/// The blood paid as the strike is chosen; the pool once after each
/// successful bleed, after the Edge (LSJ 20100210-2).
pub static THE_UNNAMED: VampireDef = VampireDef {
    name: "unnamed, The",
    abilities: &[
        mode_of(
            &[Window::CombatStrikeDeclaration],
            Actor::Host,
            &[R::HostBloodAtLeast(1)],
            &[
                Effect::HostBurnBlood(1),
                Effect::Strike(StrikeKind::Damage {
                    n: 2,
                    ranged: true,
                    aggravated: true,
                }),
            ],
        ),
        ModeDef {
            limit: Some(Limit::PerAction),
            ..mode_of(
                &[Window::ActionAfterResolution],
                Actor::Host,
                &[R::ActorActing, R::BleedSuccessful],
                &[Effect::GainPool(2)],
            )
        },
    ],
    ..VAMP
};

/// "Camarilla primogen: Victoria can lock a younger ready vampire as a +1
/// stealth Ⓓ action." Any Methuselah's, locked or not (a no-op lock kept,
/// rules-qa 57.10).
pub static VICTORIA_ASH: VampireDef = VampireDef {
    name: "Victoria Ash",
    group: Some(2),
    abilities: &[printed_action(
        target(
            TargetKind::Minion,
            &[R::TargetVampire, R::ActorReady, R::Younger],
        ),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::LockTarget,
        ],
    )],
    ..VAMP
};

/// "Camarilla Toreador Prince of Seattle: If Anson is ready during your
/// master phase, you get two master phase actions (instead of one)." Ready
/// as the phase begins; a trifle or an extra action adds to the two, an
/// out-of-turn master spends one of them (rules-qa 3.3).
pub static ANSON: VampireDef = VampireDef {
    name: "Anson",
    printed: &[own(Stat::TwoMasterPhaseActions, 1, &[R::ThisVampireReady])],
    ..VAMP
};

/// "Camarilla Malkavian Inner Circle: If Lutz is ready, your prey burns 1
/// pool after the referendum of a political action passes. +2 bleed." Any
/// Methuselah's political action, after all the referendum's effects; not
/// if they oust his controller (ANK 20220808).
pub static LUTZ_VON_HOHENZOLLERN: VampireDef = VampireDef {
    name: "Lutz von Hohenzollern",
    printed: &[own(Stat::Bleed, 2, &[])],
    riders: &[Rider {
        hook: Hook::PoliticalReferendumPassed,
        scope: RiderScope::Every,
        requires: &[R::HostReady],
        effects: &[Effect::PreyBurnsPool(1)],
    }],
    ..VAMP
};

/// "Camarilla Prince of Monaco: If Marcellus is ready, political actions
/// cost Nosferatu 1 additional blood. +1 bleed." Any Methuselah's
/// Nosferatu, an action card's or a granted political action: the action's
/// cost, payable as it is announced, paid first as it resolves (as Daria's,
/// rules-qa 42.1).
pub static MARCELLUS: VampireDef = VampireDef {
    name: "Marcellus",
    printed: &[
        Printed {
            stat: Stat::ActionCost,
            n: 1,
            to: PrintedTo::Others,
            per: Per::One,
            requires: &[
                R::ThisVampireReady,
                R::IsPolitical,
                R::ActorClan("Nosferatu"),
            ],
        },
        own(Stat::Bleed, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat: During any Methuselah's unlock phase, if Owain is ready, you may
/// move a blood from the blood bank to a vampire that Methuselah controls."
/// Once each unlock phase, even across a control change (LSJ 20040127),
/// once the active Methuselah's mandatory effects are done (LSJ 20100723:
/// the unlock rotation); any of their vampires, in torpor included, one at
/// capacity a legal no-op (rules-qa 7.2).
pub static OWAIN_EVANS: VampireDef = VampireDef {
    name: "Owain Evans, The Wanderer",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[R::TargetVampire, R::ControlledByActive],
        ),
        ..mode_of(
            &[Window::UnlockPhaseAny],
            Actor::Host,
            &[R::HostReady],
            &[Effect::BloodToTarget(1)],
        )
    }],
    ..VAMP
};

/// "Camarilla Toreador Inner Circle: Any vampire contesting Rafael's title
/// must yield during their unlock phase. +2 bleed." Not Rafael himself: he
/// pays or yields as usual.
pub static RAFAEL_DE_CORAZON: VampireDef = VampireDef {
    name: "Rafael de Corazon",
    printed: &[
        own(Stat::RivalsYieldTitle, 1, &[]),
        own(Stat::Bleed, 2, &[]),
    ],
    ..VAMP
};

/// "Camarilla Toreador Justicar: Anneke may attempt to block a vampire
/// controlled by another Methuselah, ignoring the normal prey, predator or
/// target restrictions for blocking actions. +1 bleed." Only who may block:
/// every other restriction stands — Blood Bond, Seduction, her Methuselah's
/// decision not to block (RTR 20020501, LSJ 20041203).
pub static ANNEKE: VampireDef = VampireDef {
    name: "Anneke",
    printed: &[
        own(Stat::BlocksAnyVampire, 1, &[]),
        own(Stat::Bleed, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat: Your prey can burn the Edge during your unlock phase to burn 1
/// blood from Keith." His controller's prey's choice, once that unlock
/// phase (rules-qa 6.6), as the impulse reaches them (rules-qa 3.7); with
/// Keith at 0 blood, nothing to burn: not offered.
pub static KEITH_MOODY: VampireDef = VampireDef {
    name: "Keith Moody",
    abilities: &[mode_of(
        &[Window::UnlockPhaseAny],
        Actor::HostControllersPrey,
        &[R::HostControllerActive, R::HasEdge, R::HostBloodAtLeast(1)],
        &[Effect::BurnEdge, Effect::HostBurnBlood(1)],
    )],
    ..VAMP
};

/// "Sabbat. Black Hand: Once each turn, when you draw a card from your
/// library, you may place that card on the bottom of your library and draw
/// a new card." Any library draw of his controller's — a replacement, a
/// draw-up, a card's draw —, once in each Methuselah's turn (ANK
/// 20180719-4), in torpor too; the new card is drawn, the use spent. Black
/// Hand is a trait (LSJ 20070322), generated.
pub static BLACKHORSE_TANNER: VampireDef = VampireDef {
    name: "Blackhorse Tanner",
    printed: &[own(Stat::RedrawOncePerTurn, 1, &[])],
    ..VAMP
};

/// "Sabbat: While Antonio is ready, you can use 4 transfers to search your
/// library (shuffle afterward) and/or hand for a mage ally he could play.
/// Put that ally in play with life equal to its starting life and pay its
/// cost as normal." In the influence phase, while transfers last; the ally
/// picked at resolution, its cost reductions applying — not those of an
/// action (LSJ 20090920-2) —, put in play, not recruited.
pub static ANTONIO_D_ERLETTE: VampireDef = VampireDef {
    name: "Antonio d'Erlette",
    abilities: &[mode_of(
        &[Window::Transfers],
        Actor::Host,
        &[R::HostReady],
        &[Effect::SpendTransfers(4), Effect::SearchMageAlly],
    )],
    ..VAMP
};

/// "Sabbat. Tremere antitribu slave: Visceratika [vis] cards cost Tupdog 1
/// less blood. Tupdog may enter combat with any minion as a Ⓓ action.
/// Tupdogs are not unique. After the end of your minion phase, burn Tupdog
/// and move the top card from your crypt to your uncontrolled region.
/// Flight [FLIGHT]." The slave (rulebook "Slave"): no directed action while
/// his controller controls no ready Tremere antitribu, and, as one of
/// theirs is blocked, he may lock to fight the blocker instead, the acting
/// vampire unlocked. Another Methuselah's minion (rules-qa 17.3). No
/// Visceratika card is in the pool; Flight generated (rules-qa 84.37).
pub static TUPDOG: VampireDef = VampireDef {
    name: "Tupdog",
    printed: &[
        own(Stat::BloodCostOff, 1, &[R::PlayedRequiresDiscipline("vis")]),
        own(
            Stat::NoDirectedActions,
            1,
            &[R::ControlsNoReadyOfClan("Tremere antitribu")],
        ),
    ],
    abilities: &[
        enter_combat(&[R::NotControlled, R::ActorReady]),
        mode_of(
            &[Window::ActionBlocked],
            Actor::Host,
            &[
                R::HostReady,
                R::HostUnlocked,
                R::ActingClan("Tremere antitribu"),
                R::NoSlaveInstead,
            ],
            &[Effect::LockHost, Effect::SlaveTakesCombat],
        ),
    ],
    riders: &[Rider {
        hook: Hook::MinionPhaseEnd,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::BurnHostCryptTopOut],
    }],
    ..VAMP
};

// ---- Owner's list, round 3 -------------------------------------------------

/// "Sabbat bishop: Lisette gets +1 intercept during diablerie actions."
pub static LISETTE_VIZQUEL: VampireDef = VampireDef {
    name: "Lisette Vizquel",
    printed: &[own(Stat::Intercept, 1, &[R::IsDiablerie])],
    ..VAMP
};

/// "Camarilla: Wise Frog strikes with first strike in combat with a younger
/// vampire. Blood cursed." Read on the opposing vampire, live; an ally or an
/// equal capacity not younger. Blood cursed is a generated trait
/// (`CryptCard::blood_cursed`): Alu (G5), "Independent: Blood cursed.",
/// needs no definition; nor do Cameron and Hester Reed ("Sabbat.").
pub static WISE_FROG: VampireDef = VampireDef {
    name: "Wise Frog",
    printed: &[own(Stat::FirstStrike, 1, &[R::OpponentYounger])],
    ..VAMP
};

// ---- Owner's list, round 4 -------------------------------------------------
//
// Gunnar ("Independent.") prints no text.

/// "Camarilla: Blood hunts cannot be called on Ariadne. +1 hunt." The hunt
/// bonus on her non-standard hunts too (RTR 20030519, RTR 20180511).
pub static ARIADNE: VampireDef = VampireDef {
    name: "Ariadne",
    printed: &[own(Stat::NoBloodHunt, 1, &[]), own(Stat::Hunt, 1, &[])],
    ..VAMP
};

/// "Independent: While Jacob is ready, werewolves you control get +1
/// bleed. Jacob gets an optional press each combat." The werewolves are
/// allies of the subtype (Renegade Garou), read live.
pub static JACOB_FERMOR: VampireDef = VampireDef {
    name: "Jacob Fermor",
    printed: &[
        Printed {
            stat: Stat::Bleed,
            n: 1,
            to: PrintedTo::OthersControlled,
            per: Per::One,
            requires: &[R::ThisVampireReady, R::ActorSubtype(Label::Werewolf)],
        },
        own(Stat::CombatPress, 1, &[]),
    ],
    ..VAMP
};

// ---- Round 7: the next hundred by TWDA importance --------------------------

/// "Sabbat: Adonai can enter combat with any ready Camarilla vampire
/// controlled by another Methuselah as a Ⓓ action. He gets +1 strength
/// when in combat with a Tremere. +1 bleed." A Tremere, not a Tremere
/// antitribu (Reginald Moore names both).
pub static ADONAI: VampireDef = VampireDef {
    name: "Adonai",
    printed: &[
        own(Stat::Strength, 1, &[R::OpponentClan("Tremere")]),
        own(Stat::Bleed, 1, &[]),
    ],
    abilities: &[enter_combat(&[
        R::NotControlled,
        R::ActorReady,
        R::TargetVampire,
        R::ActorSectIn(&[Sect::Camarilla]),
    ])],
    ..VAMP
};

/// "Camarilla: +1 bleed. Cornelius gets an additional +1 bleed when
/// bleeding a Methuselah who controls a ready non-Camarilla vampire." Ozmo,
/// Quira and Sheila Mezarin print "+1 bleed." as Tatiana Romanov's.
pub static CORNELIUS_OTTAVIO: VampireDef = VampireDef {
    name: "Cornelius Ottavio",
    printed: &[
        own(Stat::Bleed, 1, &[]),
        own(
            Stat::Bleed,
            1,
            &[R::TargetSeatControlsReadySect {
                sect: Sect::Camarilla,
                not: true,
            }],
        ),
    ],
    ..VAMP
};

/// "Camarilla: +1 bleed."
pub static OZMO: VampireDef = VampireDef {
    name: "Ozmo",
    printed: &[own(Stat::Bleed, 1, &[])],
    ..VAMP
};

/// "Sabbat: +1 bleed."
pub static QUIRA: VampireDef = VampireDef {
    name: "Quira, The Bitch Queen",
    printed: &[own(Stat::Bleed, 1, &[])],
    ..VAMP
};

/// "Sabbat: +1 bleed."
pub static SHEILA_MEZARIN: VampireDef = VampireDef {
    name: "Sheila Mezarin",
    printed: &[own(Stat::Bleed, 1, &[])],
    ..VAMP
};

/// "Laibon: During your unlock phase, if Alafin is ready, you may discard
/// two cards (draw afterward). If the two cards are the same, each other
/// Methuselah's hand size is one card smaller until his or her next unlock
/// phase." The same card: the same name.
pub static BABALAWO_ALAFIN: VampireDef = VampireDef {
    name: "Babalawo Alafin",
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 2,
            of: CardFilter::Any,
        }),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[R::HostReady, R::HandHas(CardFilter::Any, 2)],
            &[Effect::OthersHandSmallerIfSame(1)],
        )
    }],
    ..VAMP
};

/// "Independent: Wraiths (allies and retainers) cost Erebus 1 fewer blood
/// or pool." Off whichever the card costs (as Kuyén).
pub static EREBUS: VampireDef = VampireDef {
    name: "Erebus",
    printed: &[own(
        Stat::CostOff,
        1,
        &[
            R::PlayedCard(CardFilter::AllyOrRetainer),
            R::PlayedCard(CardFilter::HasLabel(Label::Wraith)),
        ],
    )],
    ..VAMP
};

/// "Sabbat: As Erlik enters play, you can distribute up to 5 blood from him
/// to any of your uncontrolled Harbingers of Skulls. +1 bleed. +1 hand
/// size." Before he enters a contest, not after (LSJ 20080526).
pub static ERLIK: VampireDef = VampireDef {
    name: "Erlik",
    printed: &[own(Stat::Bleed, 1, &[]), own(Stat::HandSize, 1, &[])],
    riders: &[Rider {
        hook: Hook::EntersPlay,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::DistributeToUncontrolled {
            n: 5,
            clan: "Harbinger of Skulls",
        }],
    }],
    ..VAMP
};

/// "Laibon: Whenever a vampire controlled by your predator or prey is
/// burned, Mina gains 3 blood. Once each action, she can burn 1 blood to
/// get +1 bleed." The bleed during her bleed, before resolution (ANK
/// 20231124); the blood from torpor too.
pub static MINA_GROTIUS: VampireDef = VampireDef {
    name: "Mina Grotius",
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionDuring],
            Actor::Host,
            &[R::ActorActing, R::IsBleed, R::HostBloodAtLeast(1)],
            &[Effect::HostBurnBlood(1), bleed(1, false)],
        )
    }],
    riders: &[Rider {
        hook: Hook::NeighborVampireBurned,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::AddHostBlood(3)],
    }],
    ..VAMP
};

/// "Sabbat: During your turn, Nicomedes can burn 1 blood to shuffle your
/// hand into your library (draw up afterward). Black Hand." Once each turn
/// ("during", rules-qa 6.6); the draws are replacements (ANK 20180925-1).
pub static NICOMEDES: VampireDef = VampireDef {
    name: "Nicomedes",
    abilities: &[ability(
        &[Window::AnyImpulse],
        Actor::Host,
        &[R::OwnTurn, R::HostBloodAtLeast(1)],
        Limit::PerTurn,
        &[Effect::HostBurnBlood(1), Effect::ShuffleHandDrawUp],
    )],
    ..VAMP
};

/// "Laibon: If Solomon is ready, you may spend transfers to move blood from
/// your prey's uncontrolled vampires to your prey's pool at a cost of two
/// transfers for each blood moved." One blood a use.
pub static SOLOMON_BATANEA: VampireDef = VampireDef {
    name: "Solomon Batanea",
    abilities: &[mode_of(
        &[Window::Transfers],
        Actor::Host,
        &[R::HostReady, R::PreyUncontrolledHasBlood],
        &[
            Effect::SpendTransfers(2),
            Effect::PreyUncontrolledBloodToPool(1),
        ],
    )],
    ..VAMP
};

/// "Camarilla Ventrue Inner Circle: If Arika is ready during your prey's
/// unlock phase, your prey chooses which locations he or she keeps in play.
/// For each location he or she controls, your prey burns 1 pool or burns
/// the location. +2 bleed." A mandatory effect the prey orders among their
/// unlock effects: their locations usable before it (ANK 20200629); once,
/// those put in play after it untouched (ANK 20250121).
pub static ARIKA: VampireDef = VampireDef {
    name: "Arika",
    printed: &[own(Stat::Bleed, 2, &[])],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Prey,
        requires: &[R::HostReady],
        effects: &[Effect::PreyPaysOrBurnsLocations(1)],
    }],
    ..VAMP
};

/// "Independent: Tho may look at another Methuselah's hand and discard one
/// card from it as a +1 stealth Ⓓ action (that Methuselah draws back up to
/// his or her hand size). Tho gets -1 stealth when hunting. Scarce." The
/// replacement unseen (PIB 20121031).
pub static LE_DINH_THO: VampireDef = VampireDef {
    name: "Le Dinh Tho",
    printed: &[own(Stat::Stealth, -1, &[R::IsHunt])],
    abilities: &[printed_action(
        Some(TargetSpec {
            kind: TargetKind::Seat,
            filter: &[R::NotSelf],
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::DiscardFromTargetHand,
        ],
    )],
    ..VAMP
};

/// "Camarilla Ventrue Justicar: Lucinde can enter combat with any ready
/// non-Camarilla vampire controlled by another Methuselah as a Ⓓ action.
/// During your unlock phase, you can burn the Edge to give Lucinde 1 blood
/// from the blood bank."
pub static LUCINDE: VampireDef = VampireDef {
    name: "Lucinde, Alastor",
    group: Some(3),
    abilities: &[
        enter_combat(&[
            R::NotControlled,
            R::ActorReady,
            R::TargetSectNot(Sect::Camarilla),
        ]),
        mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[R::HasEdge],
            &[Effect::BurnEdge, Effect::AddHostBlood(1)],
        ),
    ],
    ..VAMP
};

/// "Camarilla Prince of Washington, D.C.: Marcus can burn a retainer as a +1
/// stealth Ⓓ action. While Marcus is ready and a prince, Sabbat vampires
/// you control get +1 bleed when bleeding a Methuselah who controls a
/// ready Camarilla vampire." Any retainer, directed at its controller.
pub static MARCUS_VITEL: VampireDef = VampireDef {
    name: "Marcus Vitel",
    printed: &[Printed {
        stat: Stat::Bleed,
        n: 1,
        to: PrintedTo::OthersControlled,
        per: Per::One,
        requires: &[
            R::ThisVampireReady,
            R::SourceTitleIn(&[TitleClass::Prince]),
            R::ActorSectIn(&[Sect::Sabbat]),
            R::TargetSeatControlsReadySect {
                sect: Sect::Camarilla,
                not: false,
            },
        ],
    }],
    abilities: &[printed_action(
        Some(TargetSpec {
            kind: TargetKind::Card,
            filter: &[R::TargetCard(CardFilter::Type(CardType::Retainer))],
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::BurnTargetCard,
        ],
    )],
    ..VAMP
};

/// "Camarilla Prince of London: Anne gets an additional +1 bleed when
/// bleeding a Methuselah who controls a ready Tremere. +1 bleed." The G2
/// card (the G6 one is the V5 Queen Anne).
pub static QUEEN_ANNE_G2: VampireDef = VampireDef {
    name: "Queen Anne",
    group: Some(2),
    printed: &[
        own(Stat::Bleed, 1, &[]),
        own(
            Stat::Bleed,
            1,
            &[R::TargetSeatControlsReadyClan {
                clans: &["Tremere"],
                not: false,
            }],
        ),
    ],
    ..VAMP
};

/// "Independent: The Baron has 2 votes (titled). During your unlock phase,
/// if The Baron is ready, you may look at the hand of each Methuselah who
/// controls a ready Giovanni. +1 bleed." His title is no baron's (PIB
/// 20151116) and stays with him through a sect change (LSJ 20010714).
pub static THE_BARON: VampireDef = VampireDef {
    name: "Baron, The",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[mode_of(
        &[Window::UnlockPhaseOwn],
        Actor::Host,
        &[R::HostReady],
        &[Effect::PeekHandsOfReadyClan("Giovanni")],
    )],
    ..VAMP
};

/// "Sabbat: Céleste can strike: combat ends in combat with a werewolf. She
/// gets +1 intercept against Camarilla vampires whose controller controls
/// any Gehenna cards."
pub static CELESTE_LAMONTAGNE: VampireDef = VampireDef {
    name: "Céleste Lamontagne",
    printed: &[own(
        Stat::Intercept,
        1,
        &[
            R::OpponentSectIn(&[Sect::Camarilla]),
            R::OpponentControllerHasGehenna,
        ],
    )],
    abilities: &[mode_of(
        &[Window::CombatStrikeDeclaration],
        Actor::Host,
        &[R::OpponentSubtype(Label::Werewolf)],
        &[Effect::Strike(StrikeKind::CombatEnds { unlock: false })],
    )],
    ..VAMP
};

/// "Sabbat bishop: Cards requiring Animalism [ani] cost Ludmijla -1
/// blood."
pub static LUDMIJLA_RAKOCZY: VampireDef = VampireDef {
    name: "Ludmijla Rakoczy",
    printed: &[own(
        Stat::BloodCostOff,
        1,
        &[R::PlayedRequiresDiscipline("ani")],
    )],
    ..VAMP
};

/// "Camarilla: Damage Petra inflicts on Baali is aggravated. A vampire
/// burns 1 blood after using a press to continue combat with her." Her
/// strikes' damage (no pool card has a vampire inflict other damage).
pub static PETRA: VampireDef = VampireDef {
    name: "Petra",
    printed: &[
        own(Stat::AggravatedDamage, 1, &[R::OpponentClan("Baali")]),
        own(Stat::ContinuePressToll, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat: Rurik gets 1 optional press each combat. While /Ludmijla
/// Rakoczy/ is ready, Rurik gets +1 bleed." Ludmijla any Methuselah's.
pub static RURIK_RAKOCZY: VampireDef = VampireDef {
    name: "Rurik Rakoczy",
    printed: &[own(Stat::Bleed, 1, &[R::ReadyNamed("Ludmijla Rakoczy")])],
    abilities: &[ability(
        &[Window::CombatPress],
        Actor::Host,
        &[],
        Limit::PerCombat,
        &[Effect::Press],
    )],
    ..VAMP
};

/// "Camarilla: Osric can burn a location you control to get +1 stealth for
/// the current action." As often as he has locations, only acting and
/// when stealth is needed (LSJ 20020814, ANK 20221229).
pub static OSRIC_VLADISLAV: VampireDef = VampireDef {
    name: "Osric Vladislav",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Card,
            &[R::Controlled, R::TargetCard(CardFilter::Location)],
        ),
        ..mode_of(
            &[Window::ActionStealthIntercept],
            Actor::Host,
            &[R::ActorActing, R::StealthNeeded],
            &[Effect::BurnTargetCard, Effect::AddStealth(1)],
        )
    }],
    ..VAMP
};

/// "Sabbat: +1 bleed."
pub static PERSEPHONE_TAR_ANIS: VampireDef = VampireDef {
    name: "Persephone Tar-Anis",
    printed: &[own(Stat::Bleed, 1, &[])],
    ..VAMP
};

/// "Independent: Diego can burn a location as a Ⓓ action that costs 2
/// blood." Any location: directed at its controller, undirected at his
/// own.
pub static DIEGO_GIOVANNI: VampireDef = VampireDef {
    name: "Diego Giovanni",
    abilities: &[ModeDef {
        requires: &[R::ActorBloodAtLeast(2)],
        ..printed_action(
            Some(TargetSpec {
                kind: TargetKind::Card,
                filter: &[R::TargetCard(CardFilter::Location)],
            }),
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::PayBlood(2),
                Effect::BurnTargetCard,
            ],
        )
    }],
    ..VAMP
};

/// "Independent. 2 votes (titled): Michael cannot strike: combat ends or
/// press to end combat. He gets +1 strength on any round of combat after
/// the first." His title stays with him through a sect change (LSJ
/// 20010714).
pub static DON_MICHAEL_ANTONIO_GIOVANNI: VampireDef = VampireDef {
    name: "Don Michael Antonio Giovanni",
    printed: &[
        own(Stat::NoCombatEnds, 1, &[]),
        own(Stat::NoPressToEnd, 1, &[]),
        own(Stat::Strength, 1, &[R::NotFirstRound]),
    ],
    ..VAMP
};

/// "Independent: During your unlock phase, you can move 1 blood from a
/// Laibon you control to Thomasso." In torpor too (no "if ready"), as
/// Tabitha Fisk's.
pub static THOMASSO_GHIBERTI: VampireDef = VampireDef {
    name: "Thomasso Ghiberti",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                R::Controlled,
                R::ActorSectIn(&[Sect::Laibon]),
                R::BloodAtLeast(1),
            ],
        ),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[],
            &[Effect::TargetBloodToHost(1)],
        )
    }],
    ..VAMP
};

/// "Independent: During your prey's unlock phase, you can choose a vampire
/// controlled by a Methuselah other than your predator. If the chosen
/// vampire attempts to bleed before your next unlock phase, they move 1
/// blood to Donatello." Each attempt, as it is announced (rules-qa 67.3);
/// in torpor too (no "if ready").
pub static DONATELLO_GIOVANNI: VampireDef = VampireDef {
    name: "Donatello Giovanni",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[R::TargetVampire, R::TargetNotOfPredator],
        ),
        ..mode_of(
            &[Window::UnlockPhaseAny],
            Actor::Host,
            &[R::ActiveIsPrey],
            &[Effect::MarkBleedToll],
        )
    }],
    ..VAMP
};

/// "Camarilla: If a vampire you control would successfully bleed, you can
/// reduce the bleed amount by 1 to give Andre +2 bleed this turn." In the
/// would-be-successful window (LSJ 20081123), once a bleed — several
/// bleeds a turn —, his own for a net +1 (LSJ 20081124-2); not after a
/// Spying Mission, nor one after his reduction to 0 (PIB 20150612).
pub static ANDRE_LEROUX: VampireDef = VampireDef {
    name: "Andre LeRoux",
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionWouldBeSuccessful],
            Actor::Host,
            &[
                R::IsBleed,
                R::OwnActing,
                R::ActingVampire,
                R::ActionNotFailed,
                R::BleedAmountAtLeast(1),
            ],
            &[Effect::ReduceBleed(1), Effect::HostBleedThisTurn(2)],
        )
    }],
    ..VAMP
};

/// "Sabbat: Black Annis can enter combat with any minion controlled by your
/// predator or prey as a Ⓓ action. Other non-hunt actions cost her an
/// additional blood. +1 strength. +1 stealth." The blood paid with the
/// action's cost (as Daria's, rules-qa 42.1); a card's hunt is a hunt
/// (rules-qa 72.5).
pub static BLACK_ANNIS: VampireDef = VampireDef {
    name: "Black Annis",
    printed: &[
        own(Stat::Strength, 1, &[]),
        own(Stat::Stealth, 1, &[]),
        own(Stat::ActionCost, 1, &[R::OtherNonHuntAction]),
    ],
    abilities: &[enter_combat(&[
        R::ControlledByPredatorOrPrey,
        R::ActorReady,
    ])],
    ..VAMP
};

/// "Sabbat. Red List: Enkidu can enter combat with any minion as a Ⓓ
/// action. If Enkidu successfully performs an action to employ a retainer,
/// he unlocks at the end of the turn. He cannot have or use equipment. +2
/// strength." No equip action nor strike to steal equipment (LSJ 20050315,
/// LSJ 20010210); a location in play is no equipment (LSJ 20050315). Red
/// List is the crypt card's trait (rulebook "Traits").
pub static ENKIDU: VampireDef = VampireDef {
    name: "Enkidu, The Noah",
    printed: &[
        own(Stat::Strength, 2, &[]),
        own(Stat::NoEquipment, 1, &[]),
        own(Stat::UnlockAfterEmploy, 1, &[]),
    ],
    abilities: &[enter_combat(&[R::NotControlled, R::ActorReady])],
    ..VAMP
};

/// "Sabbat: As New Blood enters play during your influence phase, choose
/// any circle and draw 1 card from your crypt. New Blood belongs to the
/// chosen circle. Sterile." The circle chosen among those of the crypt (a
/// circle is no clan nor the Inner Circle title, ANK 20190117-2; rules-qa
/// 67.6, 68.1), then the draw.
pub static NEW_BLOOD: VampireDef = VampireDef {
    name: "New Blood",
    riders: &[Rider {
        hook: Hook::EntersPlay,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::ChooseCircleOnEntry, Effect::DrawCryptOnEntry],
    }],
    ..VAMP
};

/// "Camarilla primogen: Once each combat, Eugene can strike: dodge."
pub static EUGENE: VampireDef = VampireDef {
    name: "Eugene",
    abilities: &[DODGE_EACH_COMBAT],
    ..VAMP
};

/// "Camarilla: If Helena is locked and ready, she can burn a blood to be
/// able to attempt to block and/or play reaction cards that require Auspex
/// [aus] as if unlocked for the remainder of the action. +1 bleed." Another
/// Methuselah's action.
pub static HELENA: VampireDef = VampireDef {
    name: "Helena",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ReactionDuringAction, Window::ActionStealthIntercept],
            Actor::Host,
            &[
                R::HostLocked,
                R::HostReady,
                R::HostBloodAtLeast(1),
                R::ActingOtherController,
            ],
            &[Effect::HostBurnBlood(1), Effect::HostAsIfUnlockedForAuspex],
        )
    }],
    ..VAMP
};

/// "Camarilla Prince of Dallas: Ira gets +1 intercept when attempting to
/// block political actions. When Ira plays a card that requires Auspex
/// [aus], you may draw an additional card from your library (discard down
/// to your hand size afterward)." As Learjet's draw.
pub static IRA_RIVERS: VampireDef = VampireDef {
    name: "Ira Rivers",
    printed: &[
        own(Stat::Intercept, 1, &[R::IsPolitical]),
        own(
            Stat::ExtraDrawOnPlay,
            1,
            &[R::PlayedRequiresDiscipline("aus")],
        ),
    ],
    ..VAMP
};

/// "Camarilla: Joaquin gets an optional additional strike (limited) in the
/// first round of combat. At long range, his strikes with guns are at +1
/// damage." The damage of the gun's kind (RTR 19970630), none on a strike
/// dealing none (RTR 19960221).
pub static JOAQUIN_MURIETTA: VampireDef = VampireDef {
    name: "Joaquin Murietta",
    printed: &[own(Stat::GunDamageAtLong, 1, &[])],
    abilities: &[ability(
        &[Window::CombatAdditionalStrike],
        Actor::Host,
        &[R::FirstRound],
        Limit::PerCombat,
        &[Effect::AdditionalStrikes {
            n: 1,
            limited: true,
        }],
    )],
    ..VAMP
};

/// "Laibon: Khalu may burn 5 blood to cancel a minion card played by your
/// predator or prey as it is played (no cost is paid)." At once, as it is
/// played (LSJ 20061207); still played (ANK 20190104).
pub static KHALU: VampireDef = VampireDef {
    name: "Khalu",
    abilities: &[mode_of(
        &[Window::CardAsPlayed],
        Actor::Host,
        &[
            R::HostBloodAtLeast(5),
            R::PlayedCardIsMinionCard,
            R::PlayedByPredatorOrPrey,
        ],
        &[
            Effect::HostBurnBlood(5),
            Effect::CancelPlayed { refund: true },
        ],
    )],
    ..VAMP
};

/// "Laibon: Abiku gets +1 stealth when hunting."
pub static ABIKU: VampireDef = VampireDef {
    name: "Abiku",
    printed: &[own(Stat::Stealth, 1, &[R::IsHunt])],
    ..VAMP
};

/// "Laibon magaji: Once each action, Cesewayo may burn 1 blood to get +1
/// intercept." On his own block attempt, when intercept is needed.
pub static CESEWAYO: VampireDef = VampireDef {
    name: "Cesewayo",
    abilities: &[ability(
        &[Window::ActionStealthIntercept],
        Actor::Host,
        &[
            R::HostAttempting,
            R::HostBloodAtLeast(1),
            R::InterceptNeeded,
        ],
        Limit::PerAction,
        &[Effect::HostBurnBlood(1), Effect::AddIntercept(1)],
    )],
    ..VAMP
};

/// "Laibon: Massassi can enter combat with a ready vampire as a Ⓓ action.
/// If that action is successful, she gets an optional maneuver in the first
/// round of the resulting combat."
pub static MASSASSI: VampireDef = VampireDef {
    name: "Massassi",
    abilities: &[printed_action(
        Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[R::NotControlled, R::ActorReady, R::TargetVampire],
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::EnterCombatWithTarget,
            Effect::InResultingCombat(super::BlockRider::ManeuverFirstRound),
        ],
    )],
    ..VAMP
};

/// "Laibon: +1 strength."
pub static TATU_SAWOSA: VampireDef = VampireDef {
    name: "Tatu Sawosa",
    printed: &[own(Stat::Strength, 1, &[])],
    ..VAMP
};

/// "Camarilla: If Eddie successfully hunts, he does not unlock as normal
/// during his next unlock phase."
pub static EDDIE_GAINES: VampireDef = VampireDef {
    name: "Eddie Gaines",
    riders: &[Rider {
        hook: Hook::SuccessfulHunt,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::SkipNextUnlock],
    }],
    ..VAMP
};

/// "Sabbat: After Marta diablerizes an older vampire, she can gain 1
/// blood." Never worse to take: gained (rules-qa 53.5's reading).
pub static MARTA: VampireDef = VampireDef {
    name: "Marta",
    printed: &[own(Stat::BloodAfterOlderDiablerie, 1, &[])],
    ..VAMP
};

/// "Sabbat: While Midget is acting or is in combat, animal retainers lose
/// their abilities. Infernal." Any Methuselah's (as Stanislava's retainers
/// lose their text, rules-qa 49.3).
pub static MIDGET: VampireDef = VampireDef {
    name: "Midget",
    printed: &[own(Stat::AnimalRetainersLoseText, 1, &[])],
    ..VAMP
};

/// "Camarilla: Once each action, Persia can burn 1 blood to get +1
/// intercept." As Cesewayo's.
pub static PERSIA: VampireDef = VampireDef {
    name: "Persia, The Beautiful Statue",
    abilities: &[ability(
        &[Window::ActionStealthIntercept],
        Actor::Host,
        &[
            R::HostAttempting,
            R::HostBloodAtLeast(1),
            R::InterceptNeeded,
        ],
        Limit::PerAction,
        &[Effect::HostBurnBlood(1), Effect::AddIntercept(1)],
    )],
    ..VAMP
};

/// "Sabbat. Mexico City Circle: Hermanas are not unique. When moved to the
/// ready region in your influence phase, this vampire burns 2 blood or is
/// burned." Its controller's choice, after the excess drained (LSJ
/// 20051126); a vampire it creates is in a circle of one (LSJ 20091012).
pub static HERMANA_HAMBRIENTA_MAYOR: VampireDef = VampireDef {
    name: "Hermana Hambrienta Mayor",
    riders: HERMANA_ENTRY,
    ..VAMP
};

/// As Hermana Hambrienta Mayor (Menor: capacity 2).
pub static HERMANA_HAMBRIENTA_MENOR: VampireDef = VampireDef {
    name: "Hermana Hambrienta Menor",
    riders: HERMANA_ENTRY,
    ..VAMP
};

const HERMANA_ENTRY: &[Rider] = &[Rider {
    hook: Hook::EntersPlay,
    scope: RiderScope::Every,
    requires: &[],
    effects: &[Effect::BurnsBloodOrIsBurnedOnEntry(2)],
}];

/// "Sabbat. Torrance Circle: You may lock Jack when you play a Master:
/// location (unique or not) to reduce the cost of that location by 1 pool.
/// +1 strength. Sterile." Announced with the card, in torpor too (LSJ
/// 20011022).
pub static JACK: VampireDef = VampireDef {
    name: "Jack",
    printed: &[
        own(Stat::LockForLocationDiscount, 1, &[]),
        own(Stat::Strength, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat bishop. Torrance Circle: Mark gets +1 intercept when attempting
/// to block a bleed action. Sterile."
pub static MARK: VampireDef = VampireDef {
    name: "Mark",
    printed: &[own(Stat::Intercept, 1, &[R::IsBleed])],
    ..VAMP
};

/// "Sabbat: Danylo can search your library for a location, reveal it, and
/// move it to your hand as an action that costs 1 pool (shuffle and
/// discard down afterward). Kiev circle. Sterile." A master location, not
/// an equipment that is a location in play (LSJ 20080114, ANK 20221210).
pub static DANYLO: VampireDef = VampireDef {
    name: "Danylo",
    abilities: &[ModeDef {
        requires: &[R::PoolAtLeast(1)],
        ..printed_action(
            None,
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::PayPool(1),
                Effect::SearchToHand {
                    filter: CardFilter::MasterLocation,
                    shown: true,
                },
            ],
        )
    }],
    ..VAMP
};

/// "Sabbat: Allies and younger vampires get -1 intercept against Gavrylo.
/// Kiev circle. Sterile."
pub static GAVRYLO: VampireDef = VampireDef {
    name: "Gavrylo",
    printed: &[own(Stat::InterceptAgainst, -1, &[R::OpponentAllyOrYounger])],
    ..VAMP
};

/// "Sabbat: During your unlock phase, you can move 1 blood from Pavlo to a
/// ready Blood Brother of the Kiev circle. Kiev circle. Sterile." Any
/// Methuselah's.
pub static PAVLO: VampireDef = VampireDef {
    name: "Pavlo",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                R::TargetVampire,
                R::ActorReady,
                R::TargetSameCircle { sanguinus: false },
            ],
        ),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[R::HostBloodAtLeast(1)],
            &[Effect::HostBloodToTarget(1)],
        )
    }],
    ..VAMP
};

/// "Sabbat: Volo gets -1 stealth when hunting. Kiev circle. Sterile."
pub static VOLO: VampireDef = VampireDef {
    name: "Volo",
    printed: &[own(Stat::Stealth, -1, &[R::IsHunt])],
    ..VAMP
};

/// "Sabbat. Chicago Circle: Barry gets +1 strength when in combat with an
/// ally or a younger vampire. Sterile."
pub static BARRY: VampireDef = VampireDef {
    name: "Barry",
    printed: &[own(Stat::Strength, 1, &[R::OpponentAllyOrYounger])],
    ..VAMP
};

/// "Sabbat. Chicago Circle: During your master phase, you may move any
/// amount of blood from the Blood Brothers in the Chicago Circle to any
/// other Blood Brothers in this circle. Sterile." His Methuselah's, 1 blood
/// at a time, once each master phase (rules-qa 68.10); in torpor too.
pub static JERRY: VampireDef = VampireDef {
    name: "Jerry",
    abilities: &[mode_of(
        &[Window::MasterPhase],
        Actor::Host,
        &[],
        &[Effect::CircleBloodMoves { equipment: false }],
    )],
    ..VAMP
};

/// "Sabbat. Chicago Circle: Once each action, Larry can burn 1 blood to get
/// +1 intercept. Sterile." As he attempts to block, while intercept is
/// needed (Oskar Anasov's shape).
pub static LARRY: VampireDef = VampireDef {
    name: "Larry",
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionStealthIntercept],
            Actor::Host,
            &[
                R::HostAttempting,
                R::YourAttemptNeedsIntercept,
                R::HostBloodAtLeast(1),
            ],
            &[Effect::HostBurnBlood(1), Effect::AttemptingIntercept(1)],
        )
    }],
    ..VAMP
};

/// "Sabbat. Chicago Circle: Once each combat, Terry can burn 1 blood to get
/// a press. Sterile." In the press step only (PIB 20150121).
pub static TERRY: VampireDef = VampireDef {
    name: "Terry",
    abilities: &[ability(
        &[Window::CombatPress],
        Actor::Host,
        &[R::HostBloodAtLeast(1)],
        Limit::PerCombat,
        &[Effect::HostBurnBlood(1), Effect::Press],
    )],
    ..VAMP
};

/// "Sabbat. Chicago Circle: Cards that require Sanguinus [san] cost Tom an
/// additional blood. Sterile."
pub static TOM: VampireDef = VampireDef {
    name: "Tom",
    printed: &[own(
        Stat::BloodCostOff,
        -1,
        &[R::PlayedRequiresDiscipline("san")],
    )],
    ..VAMP
};

/// "Sabbat: Dorka can enter combat with a minion controlled by your
/// predator or prey as a Ⓓ action. Budapest circle. Sterile."
pub static DORKA: VampireDef = VampireDef {
    name: "Dorka",
    abilities: &[enter_combat(&[
        R::ControlledByPredatorOrPrey,
        R::ActorReady,
    ])],
    ..VAMP
};

/// "Sabbat: A minion opposing Gabi cannot strike: dodge during the first
/// round of combat. Budapest circle. Sterile."
pub static GABI: VampireDef = VampireDef {
    name: "Gabi",
    printed: &[own(Stat::OpposingNoDodgeFirstRound, 1, &[])],
    ..VAMP
};

/// "Sabbat: While /Zizi/ is in your ready region, Viri gets +1 strength.
/// Budapest circle. Sterile."
pub static VIRI: VampireDef = VampireDef {
    name: "Viri",
    printed: &[own(Stat::Strength, 1, &[R::OwnReadyNamed("Zizi")])],
    ..VAMP
};

/// "Sabbat: If /Viri/ is in your ready region, Zizi can lock to unlock
/// another Blood Brother of the same circle. Budapest circle. Sterile."
/// Any time she is unlocked, a minion attempting to block unlocked too
/// (LSJ 20010702).
pub static ZIZI: VampireDef = VampireDef {
    name: "Zizi",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                R::TargetVampire,
                R::TargetLocked,
                R::TargetSameCircle { sanguinus: false },
            ],
        ),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[R::HostReady, R::HostUnlocked, R::OwnReadyNamed("Viri")],
            &[Effect::LockHost, Effect::UnlockTarget],
        )
    }],
    ..VAMP
};

/// "Sabbat: Cards requiring Sanguinus [san] cost Ági 1 fewer blood.
/// Budapest circle. Sterile."
pub static AGI: VampireDef = VampireDef {
    name: "Ági",
    printed: &[own(
        Stat::BloodCostOff,
        1,
        &[R::PlayedRequiresDiscipline("san")],
    )],
    ..VAMP
};

// ---- Shadow Play, The Embrace, the Appolonius merge ------------------------
//
// Ignacio, Mustafa Rahman, Ramiro, Carter, Lolita, Mercy, Nicholas Chang,
// Remilliard, Ashlesha, Salbatore Bokkengro, Sasha Miklos, Sennuwy, Vedel
// Esbreno, Zip, Andrei Puxon and Ghivran Dalaal print no text.

/// "Sabbat Archbishop of Pittsburgh: After resolution of any political
/// action, if Alvaro abstained (even in torpor), he can gain 2 blood. +1
/// bleed." No vote or ballot standing, title or not (LSJ 20010526); read
/// after the referendum.
pub static ALVARO: VampireDef = VampireDef {
    name: "Alvaro, The Scion of Angelica",
    printed: &[
        own(Stat::Bleed, 1, &[]),
        own(Stat::BloodIfAbstained, 2, &[]),
    ],
    ..VAMP
};

/// "Sabbat cardinal: Monçada cannot block. Actions directed at him cost 1
/// additional pool. If he is ready during your discard phase, you can
/// unlock another ready Lasombra." His own Methuselah's actions at him are
/// undirected, untaxed (LSJ 20030214-2); the pool paid as the action
/// resolves, with its cost.
pub static AMBROSIO: VampireDef = VampireDef {
    name: "Ambrosio Luis Monçada, Plenipotentiary",
    printed: &[
        own(Stat::CannotBlock, 1, &[]),
        own(Stat::DirectedAtCostsPool, 1, &[]),
    ],
    abilities: &[ModeDef {
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[
                R::TargetVampire,
                R::Controlled,
                R::ActorReady,
                R::TargetLocked,
                R::TargetClan("Lasombra"),
                R::OtherThanActor,
            ],
        }),
        ..mode_of(
            &[Window::DiscardPhase],
            Actor::Host,
            &[R::HostReady],
            &[Effect::UnlockTarget],
        )
    }],
    ..VAMP
};

/// "Sabbat Archbishop of Atlanta: If Antonio performs a political action
/// whose referendum passes, he can unlock during the discard phase." That
/// turn's discard phase.
pub static ANTONIO_DELGADO: VampireDef = VampireDef {
    name: "Antonio Delgado",
    abilities: &[mode_of(
        &[Window::DiscardPhase],
        Actor::Host,
        &[R::HostLocked, R::ActorPassedReferendumThisTurn],
        &[Effect::UnlockActor],
    )],
    ..VAMP
};

/// "Sabbat Archbishop of New York: Once each turn, Francisco can burn 1
/// blood to get +1 bleed for the current action." His own bleed, before
/// resolution (ANK 20231124, LSJ 19970718).
pub static FRANCISCO_DOMINGO_DE_POLONIA: VampireDef = VampireDef {
    name: "Francisco Domingo de Polonia",
    abilities: &[ability(
        &[Window::ActionDuring],
        Actor::Host,
        &[R::ActorActing, R::IsBleed, R::HostBloodAtLeast(1)],
        Limit::PerTurn,
        &[Effect::HostBurnBlood(1), bleed(1, false)],
    )],
    ..VAMP
};

/// "Sabbat priscus: Gratiano gets +1 ballot. +1 bleed." Still his ballot
/// with the title lost or changed (LSJ 20040519), cast with his votes on
/// one side (RBK gaining-votes, LSJ 20051113-2).
pub static GRATIANO: VampireDef = VampireDef {
    name: "Gratiano",
    printed: &[own(Stat::Ballots, 1, &[]), own(Stat::Bleed, 1, &[])],
    ..VAMP
};

/// "Sabbat cardinal: During an action, Angelica can discard X master cards
/// to get +X intercept." Once each action, as she needs intercept — more
/// than she needs allowed (LSJ 20010111) —, the cards discarded at once
/// (LSJ 19981005).
pub static ANGELICA: VampireDef = VampireDef {
    name: "Angelica, The Canonicus",
    abilities: &[ModeDef {
        terms: Some(Terms::DiscardAny {
            of: CardFilter::Type(CardType::Master),
        }),
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionStealthIntercept],
            Actor::Host,
            &[
                R::HostAttempting,
                R::InterceptNeeded,
                R::HandHas(CardFilter::Type(CardType::Master), 1),
            ],
            &[Effect::AddInterceptX],
        )
    }],
    ..VAMP
};

/// "Sabbat cardinal: Ayelea may give you control of a Tremere antitribu
/// until the end of your next turn as a Ⓓ action." Another Methuselah's,
/// as it is (rules-qa 69.3), back at the end of her controller's next turn.
pub static AYELEA: VampireDef = VampireDef {
    name: "Ayelea, The Manipulator",
    abilities: &[printed_action(
        Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[
                R::TargetVampire,
                R::NotControlled,
                R::TargetClan("Tremere antitribu"),
            ],
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::TakeControlOfTargetUntilNextTurn,
        ],
    )],
    ..VAMP
};

/// "Independent: Bleed actions cost Marciana 1 additional blood. During an
/// action, you can lock her and choose a non-Sabbat vampire to allow that
/// vampire to burn 1 blood to get +1 intercept." Herself too, though she
/// is locked then (LSJ 20100527); the chosen vampire's burn available all
/// the action (LSJ 20100604-2, LSJ 20100611), once (rules-qa 79.2); in
/// torpor too (LSJ 20010813-2).
pub static MARCIANA_GIOVANNI: VampireDef = VampireDef {
    name: "Marciana Giovanni, Investigator",
    printed: &[own(Stat::ActionCost, 1, &[R::IsBleed])],
    abilities: &[ModeDef {
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[R::TargetVampire, R::TargetSectNot(Sect::Sabbat)],
        }),
        limit: Some(Limit::PerAction),
        // Any action: her Methuselah's own or another's.
        ..mode_of(
            &[Window::ActionDuring, Window::ReactionDuringAction],
            Actor::Host,
            &[R::HostUnlocked],
            &[Effect::LockHost, Effect::TargetBurnsBloodForIntercept],
        )
    }],
    ..VAMP
};

/// "Independent. 2 votes (titled): Cards requiring Obtenebration [obt] cost
/// Montano 1 fewer blood. During an action, he can burn 1 blood to give a
/// minion you control +1 stealth. +1 bleed." Locked or in torpor too (ANK
/// 20220705, LSJ 20100705); the acting minion's stealth, when needed, once
/// in an action (as Abderrahim's).
pub static MONTANO: VampireDef = VampireDef {
    name: "Montano",
    printed: &[
        own(Stat::BloodCostOff, 1, &[R::PlayedRequiresDiscipline("obt")]),
        own(Stat::Bleed, 1, &[]),
    ],
    abilities: &[ability(
        &[Window::ActionDuring],
        Actor::Host,
        &[R::HostBloodAtLeast(1), R::OwnActing, R::StealthNeeded],
        Limit::PerAction,
        &[Effect::HostBurnBlood(1), Effect::AddStealth(1)],
    )],
    ..VAMP
};

/// "Sabbat: Royce gets -1 stealth when bleeding."
pub static ROYCE: VampireDef = VampireDef {
    name: "Royce",
    printed: &[own(Stat::Stealth, -1, &[R::IsBleed])],
    ..VAMP
};

/// "Sabbat: +1 stealth."
pub static JOST_WERNER: VampireDef = VampireDef {
    name: "Jost Werner",
    printed: &[own(Stat::Stealth, 1, &[])],
    ..VAMP
};

/// "Camarilla: Zöe does not get the usual +1 stealth when hunting." The
/// rulebook hunt's +1: a card's hunt has no usual +1, its own stealth
/// stands (Loki's Gift, Week of Nightmares; rules-qa 72.2).
pub static ZOE: VampireDef = VampireDef {
    name: "Zöe",
    printed: &[own(Stat::Stealth, -1, &[R::RulebookHunt])],
    ..VAMP
};

/// "Sabbat: Anka can enter combat with a Lasombra as a Ⓓ action."
pub static ANKA: VampireDef = VampireDef {
    name: "Anka, Priestess of Thorns",
    abilities: &[enter_combat(&[
        R::NotControlled,
        R::ActorReady,
        R::TargetClan("Lasombra"),
    ])],
    ..VAMP
};

/// "Independent: Lázár can move 1 blood from a crypt card in your prey's
/// uncontrolled region to a vampire in your uncontrolled region as a Ⓓ
/// action." Not with none in his controller's (LSJ 19990215); both named
/// as the action is announced (rules-qa default readings), re-checked as it
/// resolves.
pub static LAZAR_DOBRESCU: VampireDef = VampireDef {
    name: "Lázár Dobrescu",
    abilities: &[ModeDef {
        requires: &[R::OwnUncontrolledVampire, R::PreyUncontrolledHasBlood],
        ..printed_action(
            Some(TargetSpec {
                kind: TargetKind::Seat,
                filter: &[R::TargetPrey, R::NotSelf],
            }),
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::PreySlotToOwnSlot,
            ],
        )
    }],
    ..VAMP
};

/// "Independent: If Spleen is blocked by an older vampire, he burns 1 blood
/// before block resolution." Older: of greater capacity (rulebook).
pub static SPLEEN: VampireDef = VampireDef {
    name: "Spleen, Georgio Gutterpunk",
    riders: &[Rider {
        hook: Hook::HostBlocked,
        scope: RiderScope::Every,
        requires: &[R::BlockedByOlder],
        effects: &[Effect::HostBurnBlood(1)],
    }],
    ..VAMP
};

/// "Independent: Khalil gets +1 stealth during diablerie actions."
pub static KHALIL_RAVANA: VampireDef = VampireDef {
    name: "Khalil Ravana",
    printed: &[own(Stat::Stealth, 1, &[R::IsDiablerie])],
    ..VAMP
};

/// "Sabbat: +1 strength."
pub static RICHTER: VampireDef = VampireDef {
    name: "Richter, The Templar of Du Mont",
    printed: &[own(Stat::Strength, 1, &[])],
    ..VAMP
};

/// "Camarilla primogen: Appolonius gets one optional press each combat. +1
/// bleed."
pub static APPOLONIUS: VampireDef = VampireDef {
    name: "Appolonius",
    printed: &[own(Stat::CombatPress, 1, &[]), own(Stat::Bleed, 1, &[])],
    ..VAMP
};

/// "+1 bleed for each titled vampire controlled by the target Methuselah":
/// in torpor too, read live.
const APPOLONIUS_TITLED_BLEED: Printed = Printed {
    stat: Stat::Bleed,
    n: 1,
    to: PrintedTo::Itself,
    per: Per::TitledOfTarget,
    requires: &[],
};

/// "Anarch: Appolonius gets +1 bleed for each titled vampire controlled by
/// the target Methuselah. [MERGED] Baron of London: Appolonius can enter
/// combat with a minion as a Ⓓ action." Unmerged, the [MERGED] text does
/// nothing (`APPOLONIUS_MERGED`).
pub static APPOLONIUS_ADV: VampireDef = VampireDef {
    name: "Appolonius",
    adv: true,
    printed: &[APPOLONIUS_TITLED_BLEED],
    ..VAMP
};

/// Merged Appolonius (rulebook "Advanced"): the base card's optional press
/// and +1 bleed, the advanced card's titled bleed, its [MERGED] rush and
/// title (Baron of London, `CryptCard::merged_title`); Anarch, the advanced
/// card's sect taking precedence.
pub static APPOLONIUS_MERGED: VampireDef = VampireDef {
    name: "Appolonius",
    adv: true,
    printed: &[
        own(Stat::CombatPress, 1, &[]),
        own(Stat::Bleed, 1, &[]),
        APPOLONIUS_TITLED_BLEED,
    ],
    abilities: &[enter_combat(&[R::NotControlled, R::ActorReady])],
    ..VAMP
};

/// Merged vampires (rulebook "Advanced"): the base card's text and the
/// advanced card's in full, its [MERGED] clauses with them — the
/// definition `cards::vampire_of` reads for a merged vampire.
pub static MERGED: [&VampireDef; 4] = [
    &APPOLONIUS_MERGED,
    &HELENA_MERGED,
    &XAVIAR_MERGED,
    &CLAUDIO_SEVERINO_MERGED,
];

// ---- Round 10: the 2025-2026 TWDA decks ------------------------------------
//
// Stavros prints no text ("Sabbat priscus.").

/// "Camarilla primogen: While Annabelle is locked, other Toreador you
/// control get +1 stealth." In torpor too (LSJ 20011022): a locked
/// Annabelle in torpor still gives it.
pub static ANNABELLE_TRIABELL: VampireDef = VampireDef {
    name: "Annabelle Triabell",
    group: Some(6),
    printed: &[Printed {
        stat: Stat::Stealth,
        n: 1,
        to: PrintedTo::OthersControlled,
        per: Per::One,
        requires: &[R::HostLocked, R::ActorClan("Toreador")],
    }],
    ..VAMP
};

/// "Sabbat priscus: Genevieve gets one additional vote during each political
/// action." Cast with her ballot, on the same side (LSJ 20051113-2): her
/// votes and ballot go in one cast.
pub static GENEVIEVE: VampireDef = VampireDef {
    name: "Genevieve",
    printed: &[own(Stat::Votes, 1, &[])],
    ..VAMP
};

/// "Sabbat cardinal: Selina gets +1 stealth during political actions."
pub static DARK_SELINA: VampireDef = VampireDef {
    name: "Dark Selina",
    printed: &[own(Stat::Stealth, 1, &[R::IsPolitical])],
    ..VAMP
};

/// "Sabbat bishop: Drozodny can move a political action card from your ash
/// heap to your hand as a +1 stealth action (discard down afterward)." The
/// card named as the action is declared (LSJ 20010627); a copy still there
/// moves on success.
pub static DROZODNY: VampireDef = VampireDef {
    name: "Drozodny",
    abilities: &[ModeDef {
        requires: &[R::OwnAshHas(CardFilter::Type(CardType::PoliticalAction))],
        ..printed_action(
            None,
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::AshCardOfToHand(CardFilter::Type(CardType::PoliticalAction)),
            ],
        )
    }],
    ..VAMP
};

/// "Camarilla Ventrue Inner Circle: Any vampire contesting Hardestadt's
/// title must yield during their unlock phase. +1 bleed. +2 strength." Not
/// Hardestadt himself (Rafael de Corazon's shape).
pub static HARDESTADT: VampireDef = VampireDef {
    name: "Hardestadt",
    printed: &[
        own(Stat::RivalsYieldTitle, 1, &[]),
        own(Stat::Bleed, 1, &[]),
        own(Stat::Strength, 2, &[]),
    ],
    ..VAMP
};

/// "Sabbat Archbishop of Rome: If your prey has more pool than you,
/// Malabranca may move 1 counter from your prey's pool to yours as a +1
/// stealth Ⓓ action." The condition read as the action is declared and
/// again as it resolves (a target filter).
pub static MALABRANCA: VampireDef = VampireDef {
    name: "Malabranca",
    abilities: &[printed_action(
        Some(TargetSpec {
            kind: TargetKind::Seat,
            filter: &[R::TargetPrey, R::NotSelf, R::TargetPoolMoreThanYours],
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::StealPoolFromTarget(1),
        ],
    )],
    ..VAMP
};

/// "Camarilla Tremere Justicar: Once each turn after a combat involving Ian
/// ends, he can unlock if the opposing minion is not ready. +1 intercept."
/// Monster's shape, without its blood: after the combat, among the
/// after-combat effects; "each turn": each player's (ANK 20180719-4).
pub static IAN_CARFAX: VampireDef = VampireDef {
    name: "Ian Carfax",
    printed: &[own(Stat::Intercept, 1, &[])],
    abilities: &[ability(
        &[Window::ActionAfterCombat],
        Actor::Host,
        &[R::HostCombatOpponentNotReady, R::HostLocked],
        Limit::PerTurn,
        &[Effect::UnlockActor],
    )],
    ..VAMP
};

/// "Camarilla: While Victoria is ready, you can discard 1 card during a
/// bleed action to give an acting Assamite +1 bleed. +1 bleed." Before
/// the bleed resolves (ANK 20231124, LSJ 19970718); an acting Banu Haqim
/// (the clan's V5 name), any Methuselah's — the text names no controller;
/// once each action (rules-qa 6.6, Larissa Moreira's shape).
pub static VICTORIA_ASH_G7: VampireDef = VampireDef {
    name: "Victoria Ash",
    group: Some(7),
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Any,
        }),
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionDuring, Window::ReactionDuringAction],
            Actor::Host,
            &[
                R::HostReady,
                R::IsBleed,
                R::ActingClan("Banu Haqim"),
                R::HandHas(CardFilter::Any, 1),
            ],
            &[bleed(1, false)],
        )
    }],
    ..VAMP
};

// ---- Round 11: the 2025-2026 TWDA decks ------------------------------------
//
// Nichodemus ("Camarilla."), Lachlan, Noddist ("Sabbat Archbishop of
// Chicago.") and Jessica G2 ("Sabbat bishop.") print no text.

/// "Camarilla primogen: Gustaphe can enter combat with a minion controlled
/// by your predator or prey as a Ⓓ action."
pub static GUSTAPHE_BRUNNELLE: VampireDef = VampireDef {
    name: "Gustaphe Brunnelle",
    abilities: &[enter_combat(&[
        R::ControlledByPredatorOrPrey,
        R::ActorReady,
    ])],
    ..VAMP
};

/// "Sabbat priscus: If Venere is in torpor during your unlock phase, he
/// may burn 1 blood to move to your ready region."
pub static VENERE_CARBONI: VampireDef = VampireDef {
    name: "Venere Carboni",
    abilities: &[mode_of(
        &[Window::UnlockPhaseOwn],
        Actor::Host,
        &[R::HostInTorpor, R::HostBloodAtLeast(1)],
        &[Effect::HostBurnBlood(1), Effect::HostLeavesTorpor],
    )],
    ..VAMP
};

/// "Independent: While Nergal is acting, reaction cards cost +1 blood.
/// During an action, he can burn 1 blood to get +1 stealth. Infernal."
/// (Advanced.) A card of two types costs more only played as a reaction
/// (PIB 20150924-2). The stealth when needed, once each action (rules-qa
/// 6.6, 79.1).
pub static NERGAL_ADV: VampireDef = VampireDef {
    name: "Nergal",
    adv: true,
    printed: &[Printed {
        stat: Stat::BloodCostMore,
        n: 1,
        to: PrintedTo::Others,
        per: Per::One,
        requires: &[R::PlayedAsReaction, R::SourceActing],
    }],
    abilities: &[ability(
        &[Window::ActionDuring],
        Actor::Host,
        &[R::HostBloodAtLeast(1), R::ActorActing, R::StealthNeeded],
        Limit::PerAction,
        &[Effect::HostBurnBlood(1), Effect::AddStealth(1)],
    )],
    ..VAMP
};

/// "Camarilla Nosferatu Inner Circle: You can use a discard phase action to
/// discard 1 card at random from your predator or prey's hand. +1 bleed. +1
/// stealth." Each discard phase action a use (ANK 20190725); the hand
/// drawn back up. In torpor too (no "if ready").
pub static JOSEF_VON_BAUREN: VampireDef = VampireDef {
    name: "Josef von Bauren",
    printed: &[own(Stat::Bleed, 1, &[]), own(Stat::Stealth, 1, &[])],
    abilities: &[ModeDef {
        target: Some(TargetSpec {
            kind: TargetKind::Seat,
            filter: &[R::TargetPredatorOrPrey],
        }),
        ..mode_of(
            &[Window::DiscardPhaseAction],
            Actor::Host,
            &[R::DiscardPhaseActionLeft],
            &[
                Effect::SpendDiscardPhaseAction,
                Effect::TargetSeatDiscardsAtRandom(1),
            ],
        )
    }],
    ..VAMP
};

/// "Camarilla Tremere Justicar: Gabrielle can steal 2 blood or life
/// (becoming blood) from a ready minion as a +1 stealth Ⓓ action." An ally
/// left with no life burns; undirected on her own Methuselah's minion.
pub static GABRIELLE_DI_RIGHETTI: VampireDef = VampireDef {
    name: "Gabrielle di Righetti",
    abilities: &[printed_action(
        target(TargetKind::Minion, &[R::ActorReady]),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::StealBloodFromTarget(2),
        ],
    )],
    ..VAMP
};

/// "Camarilla: The controller of the minion in combat with Harold plays
/// with an open hand." Read as the combat goes.
pub static HAROLD_TANNER: VampireDef = VampireDef {
    name: "Harold Tanner",
    printed: &[own(Stat::OpponentOpenHandInCombat, 1, &[])],
    ..VAMP
};

/// "Independent: During another Methuselah's discard phase, you can discard
/// one card (draw up afterward). Vampires must burn 1 blood to attempt to
/// block Giotto. Infernal." The discard in that phase's rotation, once; the
/// toll an ally does not pay; in torpor too (LSJ 20011022).
pub static GIOTTO_VERDUCCI: VampireDef = VampireDef {
    name: "Giotto Verducci",
    printed: &[Printed {
        stat: Stat::BlockToll,
        n: 1,
        to: PrintedTo::Opponent,
        per: Per::One,
        requires: &[R::ActorVampire],
    }],
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Any,
        }),
        ..mode_of(
            &[Window::OthersDiscardPhase],
            Actor::Host,
            &[R::HandHas(CardFilter::Any, 1)],
            &[],
        )
    }],
    ..VAMP
};

/// "Camarilla Malkavian Inner Circle: During each other Methuselah's unlock
/// phase, if Leandro is ready, that Methuselah chooses either to burn 1
/// pool or to lose all transfers during his or her next influence phase.
/// +2 bleed." A mandatory choice, no default (LSJ 20021210-1); with no
/// transfers, other effects adding blood to uncontrolled vampires stay
/// usable (LSJ 20100527).
pub static LEANDRO: VampireDef = VampireDef {
    name: "Leandro",
    printed: &[own(Stat::Bleed, 2, &[])],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Others,
        requires: &[R::HostReady],
        effects: &[Effect::BurnPoolOrNoTransfers(1)],
    }],
    ..VAMP
};

// ---- Round 12: the next 2025-2026 TWDA decks -------------------------------
//
// Readings in rules-qa Batch 74. Omer ("Independent: Blood cursed.") and
// Ermenegildo, The Rake ("Sabbat: Black Hand.") carry generated traits
// only (`CryptCard::blood_cursed`, `CryptCard::black_hand`).

/// "Sabbat: Alessandro gets +1 strength in combat with a Lasombra."
pub static ALESSANDRO_GARCIA: VampireDef = VampireDef {
    name: "Alessandro Garcia",
    printed: &[own(Stat::Strength, 1, &[R::OpponentClan("Lasombra")])],
    ..VAMP
};

/// "Sabbat: Laika can burn 1 blood to strike: 2R damage." The blood paid
/// as the strike is chosen.
pub static LAIKA: VampireDef = VampireDef {
    name: "Laika",
    abilities: &[mode_of(
        &[Window::CombatStrikeDeclaration],
        Actor::Host,
        &[R::HostBloodAtLeast(1)],
        &[
            Effect::HostBurnBlood(1),
            Effect::Strike(StrikeKind::Damage {
                n: 2,
                ranged: true,
                aggravated: false,
            }),
        ],
    )],
    ..VAMP
};

/// "Sabbat Archbishop of Guatemala City: Xipe can steal an ally or
/// retainer as a +1 stealth Ⓓ action that costs 1 blood. +1 strength."
/// Requirements do not apply to a card taken in play (TOM 19960226-1).
pub static XIPE_TOTEC: VampireDef = VampireDef {
    name: "Xipe Totec",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[
        ModeDef {
            requires: &[R::ActorBloodAtLeast(1)],
            ..printed_action(
                target(TargetKind::Minion, &[R::NotControlled, R::TargetAlly]),
                &[
                    Effect::DeclareAction(ActionKind::Ability),
                    Effect::PayBlood(1),
                    Effect::AddStealth(1),
                    Effect::StealTargetAlly,
                ],
            )
        },
        ModeDef {
            requires: &[R::ActorBloodAtLeast(1)],
            ..printed_action(
                target(
                    TargetKind::Card,
                    &[
                        R::TargetCard(CardFilter::Type(CardType::Retainer)),
                        R::NotHost,
                        R::NotControlled,
                        R::ActorMayHoldTarget,
                    ],
                ),
                &[
                    Effect::DeclareAction(ActionKind::Ability),
                    Effect::PayBlood(1),
                    Effect::AddStealth(1),
                    Effect::StealTargetCard,
                ],
            )
        },
    ],
    ..VAMP
};

/// "Sabbat: Luc gets +1 stealth during diablerie actions. He can give you
/// +3 hand size until your next unlock phase as a +1 stealth action."
pub static LUC: VampireDef = VampireDef {
    name: "Luc",
    printed: &[own(Stat::Stealth, 1, &[R::IsDiablerie])],
    abilities: &[printed_action(
        None,
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::HandSizeUntilUnlock(3),
        ],
    )],
    ..VAMP
};

/// "Independent. 1 vote (titled): During a bleed action, Thucimia can give
/// you -2 hand size until your next unlock phase to get +1 bleed. Blood
/// cursed. +1 hand size." Once each bleed action (rules-digest D7), before
/// its resolution (ANK 20231124, LSJ 19970718).
pub static THUCIMIA: VampireDef = VampireDef {
    name: "Thucimia",
    printed: &[own(Stat::HandSize, 1, &[])],
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionDuring],
            Actor::Host,
            &[R::ActorActing, R::IsBleed],
            &[Effect::HandSizeUntilUnlock(-2), bleed(1, false)],
        )
    }],
    ..VAMP
};

/// "Independent: During a combat, Asguresh can cancel a combat card played
/// by the opposing minion as it is played, and its cost is not paid.
/// Scarce." Once each combat (rules-digest D7). A canceled strike is chosen
/// again; a canceled limited effect leaves the limit unspent (LSJ
/// 20100206, LSJ 20030224); its "do not replace until" lapses (LSJ
/// 20011023); a card played from the hand the normal way only (RTR
/// 20001020).
pub static ASGURESH: VampireDef = VampireDef {
    name: "Asguresh",
    abilities: &[ModeDef {
        limit: Some(Limit::PerCombat),
        ..mode_of(
            &[Window::CardAsPlayed],
            Actor::Host,
            &[R::PlayedCombatByOpponent],
            &[Effect::CancelPlayed { refund: true }],
        )
    }],
    ..VAMP
};

/// "Laibon: Adhiambo gets +1 stealth during actions to equip with a weapon
/// card from your hand and actions to enter combat with a minion."
pub static ADHIAMBO: VampireDef = VampireDef {
    name: "Adhiambo",
    printed: &[
        own(Stat::Stealth, 1, &[R::IsEquipWeaponFromHand]),
        own(Stat::Stealth, 1, &[R::ActionEntersCombat]),
    ],
    ..VAMP
};

/// "Camarilla: In a referendum, Tyler gets +1 vote after a Methuselah burns
/// the Edge for a vote. He cannot block Nosferatu."
pub static TYLER_MCGILL: VampireDef = VampireDef {
    name: "Tyler McGill",
    printed: &[
        own(Stat::VotesAfterEdgeBurned, 1, &[]),
        own(Stat::CannotBlock, 1, &[R::OpponentClan("Nosferatu")]),
    ],
    ..VAMP
};

/// "Independent: Damage Baali inflict on Saulot is reduced to 0. During your
/// unlock phase, you can move 2 blood from him to a vampire in play or in
/// your uncontrolled region. Scarce. +1 bleed. +1 strength." Once each
/// unlock phase (rules-digest D7), a vampire in torpor too.
pub static SAULOT_THE_WANDERER: VampireDef = VampireDef {
    name: "Saulot, The Wanderer",
    printed: &[
        own(Stat::Bleed, 1, &[]),
        own(Stat::Strength, 1, &[]),
        own(Stat::NoDamageFromOpponent, 1, &[R::OpponentClan("Baali")]),
    ],
    abilities: &[
        ModeDef {
            target: target(TargetKind::Minion, &[R::TargetVampire, R::OtherThanActor]),
            ..mode_of(
                &[Window::UnlockPhaseOwn],
                Actor::Host,
                &[R::HostBloodAtLeast(2)],
                &[Effect::HostBloodToTarget(2)],
            )
        },
        ModeDef {
            target: target(TargetKind::Uncontrolled, &[]),
            ..mode_of(
                &[Window::UnlockPhaseOwn],
                Actor::Host,
                &[R::HostBloodAtLeast(2)],
                &[Effect::HostBloodToUncontrolledTarget(2)],
            )
        },
    ],
    ..VAMP
};

/// "Sabbat: After Paulo enters play during your influence phase, you can
/// move 1 blood from him to a Lasombra in your uncontrolled region." As
/// Erlik's distribution, up to 1.
pub static PAULO_DE_CASTILLE: VampireDef = VampireDef {
    name: "Paulo de Castille",
    riders: &[Rider {
        hook: Hook::EntersPlay,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::DistributeToUncontrolled {
            n: 1,
            clan: "Lasombra",
        }],
    }],
    ..VAMP
};

/// "Sabbat: Leila can change the sect of an Anarch with no blood to Sabbat
/// as a +1 stealth Ⓓ action." The target re-checked as the action resolves
/// (rules-digest D1).
pub static LEILA_MONROE: VampireDef = VampireDef {
    name: "Leila Monroe",
    abilities: &[printed_action(
        target(
            TargetKind::Minion,
            &[R::TargetVampire, R::TargetAnarch, R::BloodAtMost(0)],
        ),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::TargetBecomesSect(Sect::Sabbat),
        ],
    )],
    ..VAMP
};

/// "Sabbat: If Mateusz is Sabbat and not Black Hand, he can become Black
/// Hand as a +1 stealth action that costs 1 blood." A trait, not a title,
/// kept if he changes sect (LSJ 20070322, ANK 20180807).
pub static MATEUSZ_KAZIMIRSKI: VampireDef = VampireDef {
    name: "Mateusz Kazimirski, Biker of Blood",
    abilities: &[ModeDef {
        requires: &[
            R::ActorSectIn(&[Sect::Sabbat]),
            R::ActorNotBlackHand,
            R::ActorBloodAtLeast(1),
        ],
        ..printed_action(
            None,
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::PayBlood(1),
                Effect::AddStealth(1),
                Effect::ActorBecomesBlackHand,
            ],
        )
    }],
    ..VAMP
};

/// "Camarilla Ventrue Justicar: During your master phase, Lucinde can burn
/// 1 blood to choose a vampire; this turn, Camarilla vampires you control
/// can enter combat with the chosen vampire as a Ⓓ action. +1 bleed." In
/// torpor too (LSJ 20011022); once each master phase (rules-digest D7).
pub static LUCINDE_G7: VampireDef = VampireDef {
    name: "Lucinde, Alastor",
    group: Some(7),
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[
        ModeDef {
            target: target(TargetKind::Minion, &[R::TargetVampire]),
            ..mode_of(
                &[Window::MasterPhase],
                Actor::Host,
                &[R::HostBloodAtLeast(1)],
                &[Effect::HostBurnBlood(1), Effect::ChooseTargetThisTurn],
            )
        },
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    R::TargetChosenByHostThisTurn,
                    R::NotControlled,
                    R::ActorReady,
                ],
            ),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::ActingCandidate,
                &[R::ActorVampire, R::ActorSectIn(&[Sect::Camarilla])],
                &[
                    Effect::DeclareAction(ActionKind::Ability),
                    Effect::EnterCombatWithTarget,
                ],
            )
        },
    ],
    ..VAMP
};

// ---- Round 13: the 2019-2024 products, set by set --------------------------

/// "Camarilla: Aidan gets 1 optional press each combat."
pub static AIDAN_LYLE: VampireDef = VampireDef {
    name: "Aidan Lyle",
    printed: &[own(Stat::CombatPress, 1, &[])],
    ..VAMP
};

/// "Sabbat cardinal: Charles gets 1 optional press each combat."
pub static CHARLES_DELMARE: VampireDef = VampireDef {
    name: "Charles Delmare",
    printed: &[own(Stat::CombatPress, 1, &[])],
    ..VAMP
};

/// "Sabbat Archbishop of Chicago: Antón gets 1 optional maneuver during the
/// first round of combat."
pub static ANTON_DE_CONCEPCION: VampireDef = VampireDef {
    name: "Antón de Concepción",
    abilities: &[ability(
        &[Window::CombatDetermineRange],
        Actor::Host,
        &[R::FirstRound],
        Limit::PerCombat,
        &[Effect::Maneuver],
    )],
    ..VAMP
};

/// "Sabbat: After you replace a card requiring Vicissitude [vic] Duality
/// played, you can draw 1 additional card (discard down afterward)."
pub static DUALITY: VampireDef = VampireDef {
    name: "Duality",
    printed: &[own(
        Stat::ExtraDrawOnPlay,
        1,
        &[R::PlayedRequiresDiscipline("vic")],
    )],
    ..VAMP
};

/// "Camarilla primogen: During your discard phase, Emily can burn the Edge
/// to unlock."
pub static EMILY_CARSON: VampireDef = VampireDef {
    name: "Emily Carson",
    abilities: &[mode_of(
        &[Window::DiscardPhase],
        Actor::Host,
        &[R::HasEdge, R::HostLocked],
        &[Effect::BurnEdge, Effect::UnlockActor],
    )],
    ..VAMP
};

/// "Sabbat bishop: Rescuing a vampire from torpor costs Frondator 1 fewer
/// blood." The action's cost, whoever pays it (PIB 20110918, LSJ
/// 20020304-2).
pub static FRONDATOR: VampireDef = VampireDef {
    name: "Frondator",
    printed: &[own(Stat::RescueDiscount, 1, &[])],
    ..VAMP
};

/// "Camarilla primogen: You and Gem can play cards requiring a Ventrue as if
/// she were a Ventrue." Played normally only (LSJ 20050119); for the card's
/// own effects (LSJ 20050526), as Lady Scarlett Churchill.
pub static GEM_GHASTLY: VampireDef = VampireDef {
    name: "Gem Ghastly",
    plays_as: Some(PlaysAs {
        clan: "Ventrue",
        methuselah: true,
    }),
    ..VAMP
};

/// "Camarilla: Jeremy can bleed with +1 bleed as a Ⓓ action."
pub static JEREMY_WIX_WYZCHOVSKY: VampireDef = VampireDef {
    name: "Jeremy \"Wix\" Wyzchovsky",
    abilities: &[mode_of(
        &[Window::ActionDeclare],
        Actor::Host,
        &[],
        &[Effect::DeclareAction(ActionKind::Bleed), bleed(1, false)],
    )],
    ..VAMP
};

/// "Camarilla: During the polling step of a political action, Kateline can
/// burn 1 blood to force a non-acting vampire to abstain." Once each
/// referendum, locked or in torpor too (ANK 20220705, LSJ 20100705); the
/// abstention cancels the votes and ballots cast (LSJ 20100311), a
/// priscus's too (LSJ 19971001).
pub static KATELINE_NADASDY: VampireDef = VampireDef {
    name: "Kateline Nadasdy",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[R::TargetVampire, R::NotAbstaining, R::TargetNotActing],
        ),
        limit: Some(Limit::PerReferendum),
        ..mode_of(
            &[Window::Referendum],
            Actor::Host,
            &[R::PollingStep, R::HostBloodAtLeast(1)],
            &[
                Effect::HostBurnBlood(1),
                Effect::Abstain {
                    lock: false,
                    burn: 0,
                },
            ],
        )
    }],
    ..VAMP
};

/// "Sabbat: L'Épuisette gets +1 bleed against a Methuselah who controls a
/// ready Toreador."
pub static L_EPUISETTE: VampireDef = VampireDef {
    name: "L'Épuisette",
    printed: &[own(
        Stat::Bleed,
        1,
        &[R::TargetSeatControlsReadyClan {
            clans: &["Toreador"],
            not: false,
        }],
    )],
    ..VAMP
};

/// "Sabbat Archbishop of Philadelphia: Once each round of combat after the
/// first, Luca can strike: 2R damage."
pub static LUCA_ITALICUS: VampireDef = VampireDef {
    name: "Luca Italicus",
    abilities: &[ability(
        &[Window::CombatStrikeDeclaration],
        Actor::Host,
        &[R::NotFirstRound],
        Limit::PerRound,
        &[Effect::Strike(StrikeKind::Damage {
            n: 2,
            ranged: true,
            aggravated: false,
        })],
    )],
    ..VAMP
};

/// "Camarilla Prince of Brussels: During your unlock phase, you may move 1
/// blood from Nikolaus to any Nosferatu in your uncontrolled region."
pub static NIKOLAUS_VERMEULEN_G2: VampireDef = VampireDef {
    name: "Nikolaus Vermeulen",
    group: Some(2),
    abilities: &[ModeDef {
        target: target(TargetKind::Uncontrolled, &[R::TargetClan("Nosferatu")]),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[R::HostBloodAtLeast(1)],
            &[Effect::HostBloodToUncontrolledTarget(1)],
        )
    }],
    ..VAMP
};

/// "Camarilla Prince of Brussels: During your influence phase, you can burn
/// the Edge to add 1 blood to a Nosferatu in your uncontrolled region." In
/// torpor too (LSJ 20011022).
pub static NIKOLAUS_VERMEULEN_G6: VampireDef = VampireDef {
    name: "Nikolaus Vermeulen",
    group: Some(6),
    abilities: &[ModeDef {
        target: target(TargetKind::Uncontrolled, &[R::TargetClan("Nosferatu")]),
        ..mode_of(
            &[Window::InfluencePhaseOwn],
            Actor::Host,
            &[R::HasEdge],
            &[Effect::BurnEdge, Effect::BloodToUncontrolledTarget(1)],
        )
    }],
    ..VAMP
};

/// "Camarilla Prince of Cleveland: Selma gets +1 intercept when attempting
/// to block a Nosferatu vampire."
pub static SELMA_THE_REPUGNANT: VampireDef = VampireDef {
    name: "Selma the Repugnant",
    printed: &[own(Stat::Intercept, 1, &[R::OpponentClan("Nosferatu")])],
    ..VAMP
};

/// "Sabbat Archbishop of Dublin: Combat cards requiring Fortitude [for] cost
/// Una 1 fewer blood."
pub static UNA: VampireDef = VampireDef {
    name: "Una",
    printed: &[own(
        Stat::BloodCostOff,
        1,
        &[R::PlayedAsCombat, R::PlayedRequiresDiscipline("for")],
    )],
    ..VAMP
};

/// "Sabbat priscus: Once each round, Lady Vadislava can burn 1 blood to make
/// the damage from her hand strikes aggravated that round." Before the
/// strikes resolve, after both are chosen too (RTR 19960112).
pub static LADY_VADISLAVA: VampireDef = VampireDef {
    name: "Lady Vadislava",
    abilities: &[ability(
        &[Window::CombatUntimed],
        Actor::Host,
        &[R::HostBloodAtLeast(1)],
        Limit::PerRound,
        &[Effect::HostBurnBlood(1), Effect::AggravatedHands],
    )],
    ..VAMP
};

/// "Sabbat priscus: Leopold can strike, ranged: steal 2 blood or life
/// (becoming blood)."
pub static LORD_LEOPOLD_VALDEMAR: VampireDef = VampireDef {
    name: "Lord Leopold Valdemar",
    abilities: &[mode_of(
        &[Window::CombatStrikeDeclaration],
        Actor::Host,
        &[],
        &[Effect::Strike(StrikeKind::StealBlood {
            n: 2,
            ranged: true,
        })],
    )],
    ..VAMP
};

/// "Camarilla: While Grimgroth is ready, mage allies cost Tremere you control
/// 2 fewer pool to recruit. +2 bleed." Himself and the other Tremere.
pub static GRIMGROTH: VampireDef = VampireDef {
    name: "Grimgroth",
    printed: &[
        own(
            Stat::PoolCostOff,
            2,
            &[
                R::ThisVampireReady,
                R::PlayedCard(CardFilter::Labeled(CardType::Ally, Label::Mage)),
            ],
        ),
        Printed {
            stat: Stat::PoolCostOff,
            n: 2,
            to: PrintedTo::OthersControlled,
            per: Per::One,
            requires: &[
                R::ThisVampireReady,
                R::ActorClan("Tremere"),
                R::PlayedCard(CardFilter::Labeled(CardType::Ally, Label::Mage)),
            ],
        },
        own(Stat::Bleed, 2, &[]),
    ],
    ..VAMP
};

/// "Camarilla Prince of Dublin: During your minion phase, Dónal can burn 1
/// blood to unlock." In torpor too (LSJ 20011022); once in the phase (the
/// "during" template, rules-qa 58.7).
pub static DONAL_OCONNOR_G6: VampireDef = VampireDef {
    name: "Dónal O'Connor",
    group: Some(6),
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[R::OwnMinionPhase, R::HostLocked, R::HostBloodAtLeast(1)],
            &[Effect::HostBurnBlood(1), Effect::UnlockHost],
        )
    }],
    ..VAMP
};

/// "Camarilla Prince of Istanbul: Combat cards requiring Serpentis [ser]
/// cost Followers of Set an additional blood when in combat with Murat."
/// The Ministry, the clan's V5 name (rules-digest §1).
pub static MURAT: VampireDef = VampireDef {
    name: "Murat",
    printed: &[Printed {
        stat: Stat::BloodCostMore,
        n: 1,
        to: PrintedTo::Opponent,
        per: Per::One,
        requires: &[
            R::ActorClan("Ministry"),
            R::PlayedAsCombat,
            R::PlayedRequiresDiscipline("ser"),
        ],
    }],
    ..VAMP
};

/// "Independent: Bela cannot perform political actions. -1 stealth."
pub static BELA: VampireDef = VampireDef {
    name: "Bela",
    printed: &[
        own(Stat::NoPoliticalActions, 1, &[]),
        own(Stat::Stealth, -1, &[]),
    ],
    ..VAMP
};

/// "Camarilla Prince of Geneva: Troius cannot cast votes or ballots against
/// blood hunt referendums. +1 strength."
pub static TROIUS: VampireDef = VampireDef {
    name: "Troius",
    printed: &[
        own(Stat::NoVotesAgainstBloodHunt, 1, &[]),
        own(Stat::Strength, 1, &[]),
    ],
    ..VAMP
};

/// "Camarilla Nosferatu Justicar: Molly gets +1 vote and +1 bleed if your
/// prey controls a non-Camarilla vampire. +1 strength." Read live; one in
/// torpor counts ("controls").
pub static MOLLY_MACDONALD: VampireDef = VampireDef {
    name: "Molly MacDonald",
    printed: &[
        own(
            Stat::Votes,
            1,
            &[R::PreyControlsVampireNotOfSect(Sect::Camarilla)],
        ),
        own(
            Stat::Bleed,
            1,
            &[R::PreyControlsVampireNotOfSect(Sect::Camarilla)],
        ),
        own(Stat::Strength, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat Archbishop of Montreal: Carolina is immune to damage from allies
/// and retainers." An ally's strikes and the retainers' damage in combat;
/// immune: inflicted not at all (LSJ 20010627).
pub static CAROLINA_VALEZ: VampireDef = VampireDef {
    name: "Carolina Vález",
    printed: &[
        own(Stat::NoDamageFromOpponent, 1, &[R::OpponentAlly]),
        own(Stat::NoRetainerDamage, 1, &[]),
    ],
    ..VAMP
};

/// "Camarilla: Tzimisce get -1 intercept against Rutor."
pub static RUTOR: VampireDef = VampireDef {
    name: "Rutor",
    printed: &[own(
        Stat::InterceptAgainst,
        -1,
        &[R::OpponentClan("Tzimisce")],
    )],
    ..VAMP
};

/// "Camarilla Prince of Prague: Contesting a title costs Vasily +1 blood
/// unless you have the Edge."
pub static VASILY: VampireDef = VampireDef {
    name: "Vasily",
    printed: &[own(Stat::TitleContestToll, 1, &[R::NoEdge])],
    ..VAMP
};

/// "Camarilla: While João has no title, he cannot block titled vampires. He
/// gets +1 stealth during actions to put vampires in play." The Embrace,
/// Childe of the Revolution (`Effect::BecomesVampire`).
pub static JOAO_BILE: VampireDef = VampireDef {
    name: "João Bilé",
    printed: &[
        own(Stat::CannotBlock, 1, &[R::ActorUntitled, R::OpponentTitled]),
        own(Stat::Stealth, 1, &[R::ActionPutsVampireInPlay]),
    ],
    ..VAMP
};

/// "Camarilla primogen: Burn 1 pool after Thomas goes to torpor."
pub static THOMAS_DE_LUTRIUS: VampireDef = VampireDef {
    name: "Thomas De Lutrius",
    riders: &[Rider {
        hook: Hook::HostToTorpor,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::TheirControllerBurnsPool(1)],
    }],
    ..VAMP
};

/// "Camarilla Prince of New York: You can move an action card Epikasta
/// played from your ash heap to your library after resolution (shuffle
/// afterward)." Her own action's card, bound for the ash heap as the
/// action ends: sent to the library instead.
pub static EPIKASTA_RIGATOS: VampireDef = VampireDef {
    name: "Epikasta Rigatos",
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionAfterResolution],
            Actor::Host,
            &[R::ActorActing, R::OwnActionCardToAsh],
            &[Effect::ActionCardToLibraryShuffled],
        )
    }],
    ..VAMP
};

/// "Camarilla Prince of New York: If Calebros is the Prince of New York, any
/// older Camarilla vampire controlled by another Methuselah can steal the
/// title from him as a Ⓓ action that Calebros cannot block." The title he
/// holds, read live; an action requiring a Camarilla vampire (LSJ
/// 20080604), directed at his controller.
pub static CALEBROS: VampireDef = VampireDef {
    name: "Calebros, The Martyr",
    abilities: &[ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::OtherMethuselahsMinion,
        requires: &[R::ActorVampire, R::ActorSectIn(&[Sect::Camarilla])],
        target: target(
            TargetKind::Minion,
            &[
                R::TargetIsHost,
                R::NotControlled,
                R::Younger,
                R::TargetTitle(Title::Prince("New York")),
            ],
        ),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::TargetCannotBlock,
            Effect::StealTargetTitle,
        ],
    }],
    ..VAMP
};

/// "Sabbat: Morrow can put 1 graft counter on an ally as a Ⓓ action. An
/// ally with any graft counters gets -1 stealth. Black Hand." Any ally, his
/// controller's too; more counters, no more effect (LSJ 20070306). The -1
/// is his text's: while he is in play, torpor included.
pub static DR_MORROW: VampireDef = VampireDef {
    name: "Dr. Morrow, The Skindoctor",
    printed: &[Printed {
        stat: Stat::Stealth,
        n: -1,
        to: PrintedTo::Others,
        per: Per::One,
        requires: &[R::ActorGraftedAlly],
    }],
    abilities: &[printed_action(
        target(TargetKind::Minion, &[R::TargetAlly]),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::GraftOnTarget,
        ],
    )],
    ..VAMP
};

static ZANE_REFERENDUM: ReferendumDef = ReferendumDef {
    terms: &[Term::ChooseMinions {
        // Never his own controller's prince, nor a city his controller
        // claims: no contest with oneself (advanced rules §1.13.1;
        // rules-qa 100.1).
        filter: &[
            R::TargetVampire,
            R::NotControlled,
            R::TargetTitleIn(&[TitleClass::Prince]),
            R::NoOwnClaimOfTargetTitle {
                besides_actor: false,
            },
        ],
        min: 1,
        max: 1,
    }],
    effect: RefEffect::CallerContestsChosenTitle,
    grants: &[],
    non_anarch_titles_minus: false,
    titled_minus: false,
    edge_bonus: 0,
    only_sect: None,
};

/// "Camarilla primogen: If Zane is not a prince, he can call a referendum
/// to receive (and contest) the title held by a prince as a +1 stealth
/// political action." The prince chosen as terms, any Methuselah's; not
/// called with no prince to choose (LSJ 20100129). Passed, Zane takes the
/// title and contests it with its holder (his primogen title lost: one
/// title).
pub static ZANE: VampireDef = VampireDef {
    name: "Zane",
    abilities: &[mode_of(
        &[Window::ActionDeclarePolitical],
        Actor::Host,
        &[R::ActorTitleNotIn(&[TitleClass::Prince])],
        &[
            Effect::DeclareAction(ActionKind::Political),
            Effect::AddStealth(1),
            Effect::CallReferendum(&ZANE_REFERENDUM),
        ],
    )],
    ..VAMP
};

/// Mustafa's votes forced to one side by another Methuselah's older
/// Toreador, 1 blood moved to him.
const fn mustafa_force(effects: &'static [Effect]) -> ModeDef {
    ModeDef {
        windows: &[Window::Referendum],
        actor: Actor::OtherMethuselahsVampire,
        requires: &[
            R::PollingStep,
            R::ActorNotForcedYet,
            R::ActorClan("Toreador"),
            R::ActorOlderThanHost,
            R::ActorBloodAtLeast(1),
        ],
        target: target(TargetKind::Minion, &[R::TargetIsHost]),
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// "Camarilla Prince of Istanbul: During the polling step of a political
/// action, each older Toreador controlled by another Methuselah can move 1
/// blood from themselves to Mustafa to force him to cast his votes and
/// ballots as that Toreador's controller chooses." The side chosen with the
/// use; a cast of his turned; used again, the last use decides (LSJ
/// 20090304-2). Once each Toreador each polling step (D7), in torpor too
/// (D25); an abstaining Mustafa is not compelled (advanced rules
/// §3.7.6.3).
pub static MUSTAFA: VampireDef = VampireDef {
    name: "Mustafa, The Heir",
    abilities: &[
        mustafa_force(&[Effect::ActorBloodToTarget(1), Effect::CastTargetVotes(true)]),
        mustafa_force(&[
            Effect::ActorBloodToTarget(1),
            Effect::CastTargetVotes(false),
        ]),
    ],
    ..VAMP
};

// ---- Round 14: three sets completed, the TWDA greedy's steps 1-8 -----------

/// "Sabbat: Younger Camarilla vampires get -1 intercept against Aimery."
pub static AIMERY_METHUIN: VampireDef = VampireDef {
    name: "Aimery Methuin",
    printed: &[own(
        Stat::InterceptAgainst,
        -1,
        &[R::OpponentYounger, R::OpponentSectIn(&[Sect::Camarilla])],
    )],
    ..VAMP
};

/// "Sabbat bishop: Younger vampires must burn 1 blood to attempt to block
/// Julius." Mithras G6's toll (rules-qa 41.3, 42.4).
pub static DR_JULIUS_SUTPHEN: VampireDef = VampireDef {
    name: "Dr. Julius Sutphen",
    printed: &[Printed {
        stat: Stat::BlockToll,
        n: 1,
        to: PrintedTo::Opponent,
        per: Per::One,
        requires: &[R::OpponentOlder],
    }],
    ..VAMP
};

/// "Sabbat Archbishop of Houston: +2 strength."
pub static LAZVERINUS: VampireDef = VampireDef {
    name: "Lazverinus, Thrall of Lambach",
    printed: &[own(Stat::Strength, 2, &[])],
    ..VAMP
};

/// "Sabbat: During an action, Alex can burn 1 blood to get +1 intercept."
/// Persia's shape; once each action (rules-qa 6.6, D7).
pub static ALEX_CAMILLE: VampireDef = VampireDef {
    name: "Alex Camille",
    abilities: &[ability(
        &[Window::ActionStealthIntercept],
        Actor::Host,
        &[
            R::HostAttempting,
            R::HostBloodAtLeast(1),
            R::InterceptNeeded,
        ],
        Limit::PerAction,
        &[Effect::HostBurnBlood(1), Effect::AddIntercept(1)],
    )],
    ..VAMP
};

/// "Independent: Cards requiring Obtenebration [obt] cost Badr 1 fewer
/// blood." The mode played (rules-qa 38.4).
pub static BADR: VampireDef = VampireDef {
    name: "Badr, Shadow of Granada",
    printed: &[own(
        Stat::BloodCostOff,
        1,
        &[R::PlayedRequiresDiscipline("obt")],
    )],
    ..VAMP
};

/// "Camarilla primogen: During your unlock phase, Carna can burn one of her
/// retainers to gain 2 blood. +1 intercept." Once each unlock phase
/// (rules-qa 6.6); from torpor too (D25).
pub static CARNA: VampireDef = VampireDef {
    name: "Carna, The Princess Witch",
    printed: &[own(Stat::Intercept, 1, &[])],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Card,
            &[
                R::TargetCard(CardFilter::Type(CardType::Retainer)),
                R::TargetOnActor,
            ],
        ),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[],
            &[Effect::BurnTargetCard, Effect::AddHostBlood(2)],
        )
    }],
    ..VAMP
};

/// "Camarilla: If Claudio is ready, you can use a discard phase action to
/// discard a card requiring Blood Sorcery [tha] to gain 1 pool. [MERGED]
/// Claudio can burn 1 blood to get +1 stealth." (Advanced.) Ayse Dhanial's
/// shape. Unmerged, the [MERGED] text does nothing
/// (`CLAUDIO_SEVERINO_MERGED`).
pub static CLAUDIO_SEVERINO: VampireDef = VampireDef {
    name: "Claudio Severino",
    adv: true,
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Discipline("tha"),
        }),
        ..mode_of(
            &[Window::DiscardPhaseAction],
            Actor::Host,
            &[
                R::HostReady,
                R::DiscardPhaseActionLeft,
                R::HandHas(CardFilter::Discipline("tha"), 1),
            ],
            &[Effect::SpendDiscardPhaseAction, Effect::GainPool(1)],
        )
    }],
    ..VAMP
};

/// "Camarilla: Dan may take an action to allow you to go though your
/// library, find Giant's Blood, and take it into your hand (reshuffle
/// afterwards and discard to your maximum hand size)." Sakhar's search,
/// without stealth.
pub static DAN_MURDOCK: VampireDef = VampireDef {
    name: "Dan Murdock",
    abilities: &[printed_action(
        None,
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::SearchToHand {
                filter: CardFilter::Named("Giant's Blood"),
                shown: false,
            },
        ],
    )],
    ..VAMP
};

/// "Sabbat Archbishop of Paris: Vampires must burn 1 blood to cast votes
/// and ballots against referendums called by De Sade." Alexander
/// Silverson's toll (rules-qa 41.3); the first cast only (ANK 20221019).
pub static DE_SADE: VampireDef = VampireDef {
    name: "De Sade",
    printed: &[Printed {
        stat: Stat::VoteAgainstToll,
        n: 1,
        to: PrintedTo::Opponent,
        per: Per::One,
        requires: &[],
    }],
    ..VAMP
};

/// "Sabbat: Once each turn after combat involving Dan ends, Dan can burn 1
/// blood to unlock if the opposing minion is not ready." Monster's ability
/// (ANK 20230314); each player's turn (ANK 20180719-4).
pub static GOLD_PAN_DAN: VampireDef = VampireDef {
    name: "Gold Pan Dan",
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::ActionAfterCombat],
            Actor::Host,
            &[R::HostCombatOpponentNotReady, R::HostBloodAtLeast(1)],
            &[Effect::HostBurnBloodToUnlock],
        )
    }],
    ..VAMP
};

/// "Sabbat cardinal: During your unlock phase, you may move 1 blood from
/// Kyle to any other controlled Sabbat vampire." Pavlo's shape; any
/// Methuselah's, "controlled" unaddressed (rules-qa 49.5); a torpid one too
/// (D28).
pub static KYLE_STRATHCONA: VampireDef = VampireDef {
    name: "Kyle Strathcona, Cardinal of Canada",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                R::TargetVampire,
                R::NotHost,
                R::ActorSectIn(&[Sect::Sabbat]),
            ],
        ),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[R::HostBloodAtLeast(1)],
            &[Effect::HostBloodToTarget(1)],
        )
    }],
    ..VAMP
};

/// "Camarilla primogen: Once each action, Masika can discard a political
/// action card to give a minion you control +1 intercept. [MERGED] Prince
/// of Lisbon." (Advanced.) London Evening Star's grant, for the action: her
/// controller's minion attempting to block, once it needs intercept (D15).
/// Her base card is not in the pool: the [MERGED] title never applies.
pub static MASIKA: VampireDef = VampireDef {
    name: "Masika",
    adv: true,
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Type(CardType::PoliticalAction),
        }),
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionStealthIntercept],
            Actor::Host,
            &[
                R::YourAttemptNeedsIntercept,
                R::HandHas(CardFilter::Type(CardType::PoliticalAction), 1),
            ],
            &[Effect::AttemptingIntercept(1)],
        )
    }],
    ..VAMP
};

/// "Sabbat bishop: Once each political action, Quentin may burn 1 blood to
/// force a vampire to abstain (this can cancel that vampire's votes and
/// ballots)." Arishat's, in a political action's referendum only.
pub static QUENTIN: VampireDef = VampireDef {
    name: "Quentin",
    abilities: &[ModeDef {
        target: target(TargetKind::Minion, &[R::TargetVampire, R::NotAbstaining]),
        limit: Some(Limit::PerReferendum),
        ..mode_of(
            &[Window::Referendum],
            Actor::Host,
            &[R::PollingStep, R::HostBloodAtLeast(1)],
            &[
                Effect::HostBurnBlood(1),
                Effect::Abstain {
                    lock: false,
                    burn: 0,
                },
            ],
        )
    }],
    ..VAMP
};

/// "Sabbat Archbishop of Berlin: While Strohmann is acting, reaction cards
/// requiring Auspex [aus] cost 1 additional blood. +1 bleed." Nergal's
/// surcharge, on the [aus] mode played (rules-qa 38.4).
pub static STROHMANN: VampireDef = VampireDef {
    name: "Strohmann",
    printed: &[
        Printed {
            stat: Stat::BloodCostMore,
            n: 1,
            to: PrintedTo::Others,
            per: Per::One,
            requires: &[
                R::PlayedAsReaction,
                R::SourceActing,
                R::PlayedRequiresDiscipline("aus"),
            ],
        },
        own(Stat::Bleed, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat. Black Hand: Damage inflicted on Selena by a werewolf is
/// aggravated." The werewolf's damage, read with her as its opponent.
pub static SELENA: VampireDef = VampireDef {
    name: "Selena",
    printed: &[Printed {
        stat: Stat::AggravatedDamage,
        n: 1,
        to: PrintedTo::Opponent,
        per: Per::One,
        requires: &[R::ActorSubtype(Label::Werewolf)],
    }],
    ..VAMP
};

/// "Camarilla: Martin gets -1 bleed when bleeding a Methuselah who controls
/// neither a ready Tremere nor a ready Tremere antitribu."
pub static MARTIN_FRANCKEL: VampireDef = VampireDef {
    name: "Martin Franckel",
    printed: &[own(
        Stat::Bleed,
        -1,
        &[R::TargetSeatControlsReadyClan {
            clans: &["Tremere", "Tremere antitribu"],
            not: true,
        }],
    )],
    ..VAMP
};

/// "Sabbat Archbishop of London: During an action she performs, Mélusine
/// can burn 1 blood to give a younger blocking vampire -2 intercept."
/// Gwenllian Arwyn's shape (rules-qa 15.2); once each action (D7).
pub static MELUSINE_D_ANJOU: VampireDef = VampireDef {
    name: "Mélusine d'Anjou",
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionDuring],
            Actor::Host,
            &[
                R::ActorActing,
                R::StealthNeeded,
                R::AttemptingYoungerVampire,
                R::HostBloodAtLeast(1),
            ],
            &[Effect::HostBurnBlood(1), Effect::AttemptingIntercept(-2)],
        )
    }],
    ..VAMP
};

/// "Camarilla: Mithras gets +2 votes. Once each action he performs, he can
/// burn 2 blood to cancel a reaction card as it is played by a younger
/// vampire, and its cost is not paid. +2 bleed. +1 strength." (Group 3.)
/// The cancel at once (LSJ 20061207); the card still played (ANK
/// 20190104), replaced normally (LSJ 20080630); a limited effect canceled
/// leaves the limit (LSJ 20030224).
pub static MITHRAS_G3: VampireDef = VampireDef {
    name: "Mithras",
    group: Some(3),
    printed: &[
        own(Stat::Votes, 2, &[]),
        own(Stat::Bleed, 2, &[]),
        own(Stat::Strength, 1, &[]),
    ],
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::CardAsPlayed],
            Actor::Host,
            &[
                R::ActorActing,
                R::HostBloodAtLeast(2),
                R::PlayedCardType(CardType::Reaction),
                R::PlayedByYoungerVampire,
            ],
            &[
                Effect::HostBurnBlood(2),
                Effect::CancelPlayed { refund: true },
            ],
        )
    }],
    ..VAMP
};

/// "Independent: Cards requiring Vicissitude [vic] cost The Dracon -1 blood.
/// He inflicts +1 damage or steals 1 additional blood or life with ranged
/// strikes (even at close range). Flight [FLIGHT]. +1 bleed. +2 strength."
/// The added damage inherits the strike's (RTR 19970630); none on a strike
/// dealing none (RTR 19960221). Flight generated (rules-qa 84.37).
pub static THE_DRACON: VampireDef = VampireDef {
    name: "Dracon, The",
    printed: &[
        own(Stat::BloodCostOff, 1, &[R::PlayedRequiresDiscipline("vic")]),
        own(Stat::RangedStrikeDamage, 1, &[]),
        own(Stat::RangedStrikeSteal, 1, &[]),
        own(Stat::Bleed, 1, &[]),
        own(Stat::Strength, 2, &[]),
    ],
    ..VAMP
};

/// "Independent: Vlad uses and can play cards requiring a title or a sect
/// as if he had that required title or were of that required sect. +1
/// bleed." Treated so for all the card's effects (ANK 20201028: he can
/// Archon himself), its in-play actions requiring them included (PIB
/// 20150306); not for a card not requiring them (PIB 20150304: Powerbase:
/// Madrid), nor one played otherwise than from hand (LSJ 20050119: named
/// plays read no sect or title); no title votes in the referendum (ANK
/// 20231221); a title card's title on him dormant off its sect (ANK
/// 20190211).
pub static VLAD_TEPES: VampireDef = VampireDef {
    name: "Vlad Tepes",
    printed: &[
        own(Stat::MeetsSectAndTitleRequirements, 1, &[]),
        own(Stat::Bleed, 1, &[]),
    ],
    ..VAMP
};

/// "Camarilla: As Andrew plays a card requiring Blood Sorcery [tha] (before
/// replacing the card), reveal the top card of your library. If it also
/// requires Blood Sorcery, the card played is canceled and Andrew burns 1
/// blood." Before any other cancel (LSJ 20100728, LSJ 20090213); the card
/// still played (ANK 20190104), its cost paid (RBK cancel-a-card), replaced
/// normally (LSJ 20080630); the revealed card requires every Discipline it
/// prints (LSJ 20011204-3).
pub static ANDREW_STUART: VampireDef = VampireDef {
    name: "Andrew Stuart",
    riders: &[Rider {
        hook: Hook::HostPlaysCard,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::RevealTopCancels {
            disc: "tha",
            burn: 1,
        }],
    }],
    ..VAMP
};

/// "Sabbat bishop: During a referendum, if Orlando is ready, your vampires'
/// votes are doubled when results are tallied, and other multipliers may
/// not be played on your vampires. +1 bleed." Read as it is tallied; no
/// other multiplier is in the pool (the bar inert).
pub static ORLANDO_ORIUNDUS: VampireDef = VampireDef {
    name: "Orlando Oriundus",
    printed: &[
        own(Stat::DoublesControllersVotes, 1, &[R::ThisVampireReady]),
        own(Stat::Bleed, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat: During your turn, you can lock or unlock another ready Black
/// Hand vampire. Black Hand. +1 bleed. +1 strength." Alexandra's toggle,
/// once each turn (rules-qa 6.6); from torpor too (LSJ 20011022); any
/// Methuselah's; a locked blocker's attempt fails, unless woken (LSJ
/// 20010702).
pub static KARIF_AL_NUMAIR: VampireDef = VampireDef {
    name: "Karif al Numair",
    printed: &[own(Stat::Bleed, 1, &[]), own(Stat::Strength, 1, &[])],
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                R::ActorReady,
                R::NotHost,
                R::TargetVampire,
                R::ActorBlackHand,
            ],
        ),
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[R::OwnTurn],
            &[Effect::ToggleLockTarget],
        )
    }],
    ..VAMP
};

/// "Sabbat: During your unlock phase, Gerald can burn 1 blood to make a
/// younger non-titled Sabbat vampire you control Black Hand. Black Hand.
/// Seraph." Once each unlock phase (rules-qa 6.6); Black Hand kept through
/// a sect change (LSJ 20070322, ANK 20180807); Seraph a generated trait
/// (Tattoo Signal reads it).
pub static GERALD_FITZGERALD: VampireDef = VampireDef {
    name: "Gerald FitzGerald",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                R::Controlled,
                R::TargetVampire,
                R::Younger,
                R::ActorSectIn(&[Sect::Sabbat]),
                R::ActorTitleNotIn(&[TitleClass::Any]),
            ],
        ),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[R::HostBloodAtLeast(1)],
            &[Effect::HostBurnBlood(1), Effect::TargetBecomesBlackHand],
        )
    }],
    ..VAMP
};

/// "Sabbat priscus: Righteous gets +1 intercept against infernal minions and
/// +1 strength in combat with an infernal minion."
pub static RIGHTEOUS_ENDEAVOR: VampireDef = VampireDef {
    name: "Righteous Endeavor",
    printed: &[
        own(Stat::Intercept, 1, &[R::OpponentInfernal]),
        own(Stat::Strength, 1, &[R::OpponentInfernal]),
    ],
    ..VAMP
};

/// "Camarilla Prince of Cairo: Suhailah gets +1 stealth on each of her
/// actions if your prey has a ready prince or justicar. +1 bleed." Read
/// live.
pub static SUHAILAH: VampireDef = VampireDef {
    name: "Suhailah",
    printed: &[
        own(
            Stat::Stealth,
            1,
            &[R::PreyControlsReadyTitled(&[
                TitleClass::Prince,
                TitleClass::Justicar,
            ])],
        ),
        own(Stat::Bleed, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat: Synner-G can unlock after successfully equipping with an
/// electronic equipment from your hand." Keegan's shape; a fizzled equip
/// equipped nothing.
pub static SYNNER_G: VampireDef = VampireDef {
    name: "Synner-G",
    abilities: &[mode_of(
        &[Window::ActionAfterResolution],
        Actor::Host,
        &[
            R::ActorActing,
            R::ActionNotFizzled,
            R::IsEquipLabeledFromHand(Label::Electronic),
            R::HostLocked,
        ],
        &[Effect::UnlockActor],
    )],
    ..VAMP
};

/// "Sabbat bishop: Once each action, Paul can remove the top card of your
/// crypt from the game to get +1 intercept or +1 stealth." Abraham
/// DuSable's shape: stealth on his action, intercept on his block attempt,
/// each when needed (rules-qa 3.2).
pub static PAUL_CORDWOOD: VampireDef = VampireDef {
    name: "Paul Cordwood",
    abilities: &[
        ModeDef {
            limit: Some(Limit::PerAction),
            ..mode_of(
                &[Window::ActionDuring],
                Actor::Host,
                &[R::ActorActing, R::StealthNeeded, R::CryptNotEmpty],
                &[Effect::RemoveCryptTop, Effect::AddStealth(1)],
            )
        },
        ModeDef {
            limit: Some(Limit::PerAction),
            ..mode_of(
                &[Window::ActionStealthIntercept],
                Actor::Host,
                &[R::InterceptNeeded, R::CryptNotEmpty],
                &[Effect::RemoveCryptTop, Effect::AddIntercept(1)],
            )
        },
    ],
    ..VAMP
};

/// "Camarilla: Bulscu gets +1 stealth during actions directed at a location
/// or an equipment. +1 strength. [MERGED] Prince of Budapest." (Advanced.)
/// His base card is not in the pool: the [MERGED] title never applies.
pub static BULSCU: VampireDef = VampireDef {
    name: "Bulscu",
    adv: true,
    printed: &[
        own(
            Stat::Stealth,
            1,
            &[R::DirectedAtCard(&[
                CardFilter::Location,
                CardFilter::Type(CardType::Equipment),
            ])],
        ),
        own(Stat::Strength, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat cardinal: Other Methuselahs cannot play master cards targeting
/// Mimir. +1 bleed."
pub static MIMIR: VampireDef = VampireDef {
    name: "Mimir",
    printed: &[
        own(Stat::NoOthersMasterTargeting, 1, &[]),
        own(Stat::Bleed, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat cardinal: Bruce may burn a blood when he announces a political
/// action to prevent Camarilla vampires from casting votes or ballots in
/// the resulting referendum. +1 bleed."
pub static BRUCE_DE_GUY: VampireDef = VampireDef {
    name: "Bruce de Guy",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionAsAnnounced],
            Actor::Host,
            &[R::ActorActing, R::IsPolitical, R::HostBloodAtLeast(1)],
            &[
                Effect::HostBurnBlood(1),
                Effect::BarSectVotesInReferendum(Sect::Camarilla),
            ],
        )
    }],
    ..VAMP
};

/// "Anarch: Primogen get +1 vote in referendums called by Maldavis.
/// [MERGED] Baron of Chicago." (Advanced.) Any Methuselah's primogen. His
/// base card is not in the pool: the [MERGED] title never applies.
pub static MALDAVIS: VampireDef = VampireDef {
    name: "Maldavis",
    adv: true,
    printed: &[Printed {
        stat: Stat::Votes,
        n: 1,
        to: PrintedTo::Others,
        per: Per::One,
        requires: &[
            R::SourceCalledReferendum,
            R::ActorTitleIn(&[TitleClass::Primogen]),
        ],
    }],
    ..VAMP
};

/// "Sabbat: Winterlich can recruit a changeling ally as if he met the
/// requirements of playing that card. While he is ready, changelings cost
/// you 1 fewer pool." Himself and the other minions recruiting or employing
/// one. No changeling in the pool has requirements (Mylan Horseed): the
/// first clause inert.
pub static WINTERLICH: VampireDef = VampireDef {
    name: "Winterlich",
    printed: &[
        own(
            Stat::PoolCostOff,
            1,
            &[R::ThisVampireReady, CHANGELING_PLAYED],
        ),
        Printed {
            stat: Stat::PoolCostOff,
            n: 1,
            to: PrintedTo::OthersControlled,
            per: Per::One,
            requires: &[R::ThisVampireReady, CHANGELING_PLAYED],
        },
    ],
    ..VAMP
};

/// A changeling played (Winterlich).
const CHANGELING_PLAYED: R = R::PlayedCard(CardFilter::HasLabel(Label::Changeling));

/// "Independent. 2 votes (titled): During your unlock phase, you can move
/// up to two cards from your hand to the bottom of your library (draw up
/// afterward). Nehemiah cannot use cards requiring Celerity [cel]. Scarce."
/// Heart of Nizchetus's bottom moves (rules-qa 9.5), the number picked as
/// it is used; a card's [cel] mode only is barred (LSJ 20050322-2); title
/// and scarcity generated (LSJ 20010714, LSJ 20011015).
pub static NEHEMIAH: VampireDef = VampireDef {
    name: "Nehemiah",
    printed: &[own(
        Stat::CannotPlay,
        1,
        &[R::PlayedRequiresDiscipline("cel")],
    )],
    abilities: &[ModeDef {
        terms: Some(Terms::Count { min: 1, max: 2 }),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[R::HandNotEmpty],
            &[Effect::HandToBottomX],
        )
    }],
    ..VAMP
};

/// "Independent: Once each turn, Baba Yaga can unlock when she successfully
/// recruits an ally or employs a retainer. Once each round, she can burn 1
/// blood to make the damage from her hand strikes aggravated that round. +1
/// strength. +1 bleed." A recruit or employ by another card counts (PIB
/// 20150105-2: Pack Alpha); each player's turn (ANK 20180719-4); the
/// aggravation before strikes resolve (RTR 19960112), Crossbreaker's.
pub static BABA_YAGA: VampireDef = VampireDef {
    name: "Baba Yaga, the Iron Hag",
    printed: &[own(Stat::Strength, 1, &[]), own(Stat::Bleed, 1, &[])],
    abilities: &[ability(
        &[Window::CombatUntimed],
        Actor::Host,
        &[R::HostBloodAtLeast(1)],
        Limit::PerRound,
        &[Effect::HostBurnBlood(1), Effect::AggravatedHands],
    )],
    riders: &[Rider {
        hook: Hook::HostRecruitsOrEmploys,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::MayUnlockOncePerTurn],
    }],
    ..VAMP
};

// ---- Round 15: the 2023-07-01 Promo set, the TWDA greedy's steps 1-8 -------

/// "Sabbat: Black Hand. Seraph. +1 stealth." Black Hand and Seraph are
/// generated traits (LSJ 20070322, ANK 20180807).
pub static APPIUS_CLAUDIUS_CORVUS: VampireDef = VampireDef {
    name: "Appius Claudius Corvus",
    printed: &[own(Stat::Stealth, 1, &[])],
    ..VAMP
};

/// "Independent: Abdelsobek can unlock a vampire or mummy you control as a
/// +1 stealth action." No mummy is in the pool; himself included (D28).
pub static ABDELSOBEK: VampireDef = VampireDef {
    name: "Abdelsobek",
    abilities: &[printed_action(
        target(TargetKind::Minion, &[R::Controlled, R::TargetVampire]),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::UnlockTarget,
        ],
    )],
    ..VAMP
};

/// "Independent: Ghede can enter combat with a minion as a Ⓓ action. Combat
/// cards requiring Serpentis [ser] cost him 1 fewer blood." The mode played
/// (rules-qa 38.4).
pub static GHEDE: VampireDef = VampireDef {
    name: "Ghede",
    printed: &[own(
        Stat::BloodCostOff,
        1,
        &[R::PlayedAsCombat, R::PlayedRequiresDiscipline("ser")],
    )],
    abilities: &[enter_combat(&[R::NotControlled, R::ActorReady])],
    ..VAMP
};

/// "Camarilla: Juniper may steal an ally controlled by your predator or prey
/// as a +1 stealth Ⓓ action." Xipe Totec's steal.
pub static JUNIPER: VampireDef = VampireDef {
    name: "Juniper",
    abilities: &[printed_action(
        target(
            TargetKind::Minion,
            &[
                R::NotControlled,
                R::TargetAlly,
                R::ControlledByPredatorOrPrey,
            ],
        ),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::StealTargetAlly,
        ],
    )],
    ..VAMP
};

/// "Laibon: While /Otieno/ is ready, Onaedo gets +1 stealth during diablerie
/// actions." Any Methuselah's Otieno (D11, 73.7); none is in the pool.
pub static ONAEDO: VampireDef = VampireDef {
    name: "Onaedo",
    printed: &[own(
        Stat::Stealth,
        1,
        &[R::IsDiablerie, R::ReadyNamed("Otieno")],
    )],
    ..VAMP
};

/// "Sabbat: During your discard phase, Saul can burn 1 blood to unlock
/// another ready Black Hand vampire. Black Hand." Once that phase (rules-qa
/// 6.6); any Methuselah's vampire (D11).
pub static SAUL_MEIRA: VampireDef = VampireDef {
    name: "Saul Meira",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                R::TargetVampire,
                R::ActorReady,
                R::TargetLocked,
                R::ActorBlackHand,
                R::NotHost,
            ],
        ),
        ..mode_of(
            &[Window::DiscardPhase],
            Actor::Host,
            &[R::HostBloodAtLeast(1)],
            &[Effect::HostBurnBlood(1), Effect::UnlockTarget],
        )
    }],
    ..VAMP
};

/// "Camarilla. 2 votes (titled): While /Victoria Ash/ is in your ready
/// region, Tegyrius gets +2 votes. +1 strength." Viri's shape; his title
/// generated, kept through a sect change (LSJ 20010714).
pub static TEGYRIUS: VampireDef = VampireDef {
    name: "Tegyrius, Vizier",
    printed: &[
        own(Stat::Votes, 2, &[R::OwnReadyNamed("Victoria Ash")]),
        own(Stat::Strength, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat: During your unlock phase, older Sabbat vampires controlled by
/// other Methuselahs can burn 1 blood to lock Feo." His controller's unlock
/// phase, once for each such vampire (rules-qa 6.6); one in torpor too
/// (D25), as Mustafa's Toreador.
pub static FEO_RAMOS: VampireDef = VampireDef {
    name: "Feo Ramos",
    abilities: &[mode_of(
        &[Window::UnlockPhaseAny],
        Actor::OtherMethuselahsVampire,
        &[
            R::HostControllerActive,
            R::HostUnlocked,
            R::ActorSectIn(&[Sect::Sabbat]),
            R::ActorOlderThanHost,
            R::ActorBloodAtLeast(1),
        ],
        &[Effect::BurnActorBlood(1), Effect::LockHost],
    )],
    ..VAMP
};

/// "Independent: Once each turn, Shalmath can unlock. He cannot use cards
/// requiring Celerity [cel]. Scarce." Once in each Methuselah's turn (ANK
/// 20180719-4), whenever he holds the impulse, in torpor too (D25); a
/// card's [cel] mode only is barred (LSJ 20050322-2), Nehemiah's; scarce
/// generated (LSJ 20011015).
pub static SHALMATH: VampireDef = VampireDef {
    name: "Shalmath",
    printed: &[own(
        Stat::CannotPlay,
        1,
        &[R::PlayedRequiresDiscipline("cel")],
    )],
    abilities: &[ability(
        &[Window::AnyImpulse],
        Actor::Host,
        &[R::HostLocked],
        Limit::PerTurn,
        &[Effect::UnlockHost],
    )],
    ..VAMP
};

/// "Independent: If Cybele is unlocked at the start of your master phase,
/// you get +1 master phase action. She can bleed with +2 bleed as a Ⓓ
/// action that costs 1 blood. Infernal." Read as the phase begins;
/// infernal generated.
pub static CYBELE: VampireDef = VampireDef {
    name: "Cybele",
    printed: &[own(Stat::MasterPhaseActionAtStart, 1, &[R::HostUnlocked])],
    abilities: &[ModeDef {
        requires: &[R::ActorBloodAtLeast(1)],
        ..mode_of(
            &[Window::ActionDeclare],
            Actor::Host,
            &[],
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::PayBlood(1),
                bleed(2, false),
            ],
        )
    }],
    ..VAMP
};

/// "Camarilla: At the end of your minion phase, if there are any Gehenna
/// events in play and César did not hunt, he burns 1 blood." Read then; any
/// Methuselah's Gehenna event; in torpor too (D25).
pub static CESAR_HOLFIELD: VampireDef = VampireDef {
    name: "César Holfield",
    riders: &[Rider {
        hook: Hook::MinionPhaseEnd,
        scope: RiderScope::Every,
        requires: &[R::GehennaInPlay(1), R::HostDidNotHunt],
        effects: &[Effect::HostBurnBlood(1)],
    }],
    ..VAMP
};

/// "Camarilla: Donald burns 1 blood after a referendum called by your
/// predator passes." After all its effects, an oust included, and still if
/// it ousted that predator (LSJ 20090115-2); in torpor too (D25).
pub static DONALD_CARGILL: VampireDef = VampireDef {
    name: "Donald Cargill",
    riders: &[Rider {
        hook: Hook::PoliticalReferendumPassed,
        scope: RiderScope::Every,
        requires: &[R::ActingByPredator],
        effects: &[Effect::HostBurnBlood(1)],
    }],
    ..VAMP
};

/// "Camarilla: You may burn a card that requires Dominate [dom] from your
/// hand to cause an action directed at Edward to fail." Not once a block is
/// attempted or made (ANK 20260316, LSJ 20041022); the failed action reached
/// resolution (NRA; ANK 20211015, RTR 20180719), was not blocked (LSJ
/// 20070219) and keeps its after-resolution window (ANK 20221205). The card
/// burned from hand goes to the ash heap and is replaced, as a discard
/// (rules-qa 8.3); in torpor too (D25).
pub static EDWARD_VIGNES: VampireDef = VampireDef {
    name: "Edward Vignes",
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Discipline("dom"),
        }),
        ..mode_of(
            &[Window::ReactionDuringAction],
            Actor::Host,
            &[
                R::ActionTargetsHost,
                R::NoBlockAttempt,
                R::HandHas(CardFilter::Discipline("dom"), 1),
            ],
            &[Effect::ActionFails],
        )
    }],
    ..VAMP
};

/// "Camarilla: If an older vampire blocks Iris, their controller can look at
/// your hand before block resolution." The blocker's controller, once that
/// block.
pub static IRIS_BENNETT: VampireDef = VampireDef {
    name: "Iris Bennett",
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionBlocked],
            Actor::OtherMethuselahsVampire,
            &[R::ActingIsHost, R::ActorHasBlocked, R::ActorOlderThanHost],
            &[Effect::PeekHostControllersHand],
        )
    }],
    ..VAMP
};

/// "Sabbat. Black Hand: Once during your turn, if Katherine is ready, you may
/// discard a card that requires Auspex [aus] from your hand. Draw back up to
/// your hand size afterward." The discard replaced as any; Black Hand
/// generated.
pub static KATHERINE_STODDARD: VampireDef = VampireDef {
    name: "Katherine Stoddard",
    abilities: &[ModeDef {
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Discipline("aus"),
        }),
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[
                R::OwnTurn,
                R::HostReady,
                R::HandHas(CardFilter::Discipline("aus"), 1),
            ],
            &[],
        )
    }],
    ..VAMP
};

/// "Sabbat. Black Hand: If Lubomira successfully bleeds a Methuselah who
/// controls a ready titled vampire, she does not unlock as normal during her
/// next unlock phase." She, not the bled one (PIB 20110915-1); the bled
/// Methuselah after redirects.
pub static LUBOMIRA_HRADOK: VampireDef = VampireDef {
    name: "Lubomira Hradok",
    riders: &[Rider {
        hook: Hook::OwnBleedSuccessful,
        scope: RiderScope::Every,
        requires: &[R::BledControlsReadyTitled],
        effects: &[Effect::SkipNextUnlock],
    }],
    ..VAMP
};

/// "Camarilla: Master cards targeting Walker cost you 1 additional pool."
/// His controller's master cards only (LSJ 20081124-1).
pub static WALKER_GRIMES: VampireDef = VampireDef {
    name: "Walker Grimes",
    printed: &[own(Stat::MasterTargetingCostsYou, 1, &[])],
    ..VAMP
};

/// "Sabbat: When Skidmark is acting, you do not replace any cards you play
/// until the end of the action." From the action card announcing it to the
/// end of the action, its combats and after-resolution effects included
/// (Bum's Rush's delay, LSJ 20001120).
pub static SKIDMARK: VampireDef = VampireDef {
    name: "Skidmark",
    printed: &[own(Stat::HoldsReplacementsWhileActing, 1, &[])],
    ..VAMP
};

/// "Camarilla: Franciscus is burned (without going into torpor) if he takes
/// more than 3 damage during a single combat." The damage landed, after
/// prevention (rules-qa 29.3), every round of it.
pub static FRANCISCUS: VampireDef = VampireDef {
    name: "Franciscus",
    printed: &[own(Stat::BurnedOverCombatDamage, 3, &[])],
    ..VAMP
};

/// "Anarch: During an action he performs, Bolesław can burn 1 blood to reveal
/// the top card of your crypt, then shuffle your crypt. If it is a copy of
/// Bolesław, he gets +1 stealth; otherwise, he gets +1 strength." Once each
/// action (D7), in torpor too (LSJ 20011022); usable before stealth is
/// needed, its stealth then lost (ANK 20231031); the strength for the
/// action; not with an empty crypt, nothing to reveal (advanced rules
/// §1.6.5).
pub static BOLESLAW_GUTOWSKI: VampireDef = VampireDef {
    name: "Bolesław Gutowski",
    abilities: &[ModeDef {
        waives_needed: true,
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionDuring],
            Actor::Host,
            &[R::ActorActing, R::HostBloodAtLeast(1), R::CryptNotEmpty],
            &[
                Effect::HostBurnBlood(1),
                Effect::RevealCryptTopForCopy {
                    stealth: 1,
                    strength: 1,
                },
            ],
        )
    }],
    ..VAMP
};

/// Helena's blood costs waived: reaction cards played using Auspex or
/// Dominate, any X too (LSJ 20051117); a "burn n blood" effect is no cost
/// (rules-digest D20).
const HELENA_FREE_REACTIONS: Printed = own(
    Stat::BloodCostOff,
    i8::MAX,
    &[
        R::PlayedAsReaction,
        R::AnyOf(&[
            &[R::PlayedRequiresDiscipline("aus")],
            &[R::PlayedRequiresDiscipline("dom")],
        ]),
    ],
);

/// "Independent: Helena pays no blood costs for reaction cards requiring
/// Auspex [aus] or Dominate [dom]. Infernal. [MERGED] Helena is immune to
/// damage from demons and gets +1 level of Daimoinon [dai] and
/// Obtenebration [obt]." Unmerged, the [MERGED] text does nothing
/// (`HELENA_MERGED`); infernal generated.
pub static HELENA_ADV: VampireDef = VampireDef {
    name: "Helena",
    adv: true,
    printed: &[HELENA_FREE_REACTIONS],
    ..VAMP
};

/// Merged Helena (rulebook "Advanced"): the base card's Auspex play while
/// locked and +1 bleed, the advanced card's free reactions, its [MERGED]
/// immunity to demons' damage (LSJ 20010627: not inflicted at all) and
/// Discipline levels; Independent, the advanced card's sect, an anarch
/// token burned (ANK 20180524).
pub static HELENA_MERGED: VampireDef = VampireDef {
    name: "Helena",
    adv: true,
    printed: &[
        own(Stat::Bleed, 1, &[]),
        HELENA_FREE_REACTIONS,
        own(
            Stat::NoDamageFromOpponent,
            1,
            &[R::OpponentSubtype(crate::library::Label::Demon)],
        ),
    ],
    abilities: HELENA.abilities,
    levels: &["dai", "obt"],
    ..VAMP
};

/// "Camarilla Malkavian Justicar: Maris can burn a blood to give a blocking
/// minion +1 intercept. She can take a +1 stealth action to allow you to
/// look at and rearrange the top 5 cards of your library." (Group 3, the
/// contrast to "during": owner, 2026-10-17.) Any Methuselah's minion
/// attempting to block, while it needs intercept (rules-qa 3.2); no
/// "during": multiple times each action (LSJ 20020814, rules-qa 79.3);
/// while locked (ANK 20220705), in torpor too (rules-digest D25). Justicar
/// generated.
pub static MARIS_STRECK: VampireDef = VampireDef {
    name: "Maris Streck",
    abilities: &[
        mode_of(
            &[Window::ActionStealthIntercept],
            Actor::Host,
            &[R::AnyAttemptNeedsIntercept, R::HostBloodAtLeast(1)],
            &[Effect::HostBurnBlood(1), Effect::AttemptingIntercept(1)],
        ),
        printed_action(
            None,
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::ReorderTop {
                    n: 5,
                    unlock: false,
                },
            ],
        ),
    ],
    ..VAMP
};

// ---- Round 16: the TWDA greedy's steps 1-6, five promo sets ----------------

/// "Camarilla: Younger Tremere cannot block Aisling." Read on the minion
/// attempting, live.
pub static AISLING_STURBRIDGE: VampireDef = VampireDef {
    name: "Aisling Sturbridge",
    printed: &[own(
        Stat::CannotBeBlockedBy,
        1,
        &[R::OpponentClan("Tremere"), R::OpponentYounger],
    )],
    ..VAMP
};

/// "Camarilla: Blythe gets -1 strength when in combat with a Malkavian."
pub static BLYTHE_CANDELERIA: VampireDef = VampireDef {
    name: "Blythe Candeleria",
    printed: &[own(Stat::Strength, -1, &[R::OpponentClan("Malkavian")])],
    ..VAMP
};

/// "Camarilla: Burn 1 pool as Solomon enters play." Before any contest
/// (LSJ 20080526).
pub static DR_SOLOMON_GREY: VampireDef = VampireDef {
    name: "Dr. Solomon Grey",
    riders: &[Rider {
        hook: Hook::EntersPlay,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::BurnPool(1)],
    }],
    ..VAMP
};

/// "Sabbat bishop: Perfidio can burn 1 blood to unlock and attempt to block
/// an action directed at a location you control." Multiple times in the
/// action (LSJ 20030605): no "during", no limit (rules-qa 79.3); a forced
/// attempt keeps the blocking restrictions (42.8).
pub static GENERAL_PERFIDIO_DIOS: VampireDef = VampireDef {
    name: "General Perfidio Díos",
    abilities: &[ModeDef {
        waives_needed: true,
        ..mode_of(
            &[Window::ReactionUnlockAttempt],
            Actor::Host,
            &[
                R::DirectedAtYourCard(&[CardFilter::Location]),
                R::HostBloodAtLeast(1),
                R::CanAttemptBlock,
            ],
            &[
                Effect::HostBurnBlood(1),
                Effect::UnlockHost,
                Effect::AttemptBlock,
            ],
        )
    }],
    ..VAMP
};

/// "Camarilla: Karsh can play cards named Blood Hunt from your hand or ash
/// heap as a prince. He gets +3 votes in blood hunt referendums. He cannot
/// block infernal vampires. [MERGED] Imperator (3 votes): +1 intercept."
/// (Advanced.) Blood Hunt is not in the pool: its play is inert, as is the
/// [MERGED] text without the base card (DESIGN §11.3).
pub static KARSH_ADV: VampireDef = VampireDef {
    name: "Karsh",
    adv: true,
    printed: &[
        own(Stat::VotesInBloodHunt, 3, &[]),
        own(Stat::CannotBlock, 1, &[R::OpponentInfernal]),
    ],
    ..VAMP
};

/// "Sabbat bishop: +1 intercept."
pub static KITE: VampireDef = VampireDef {
    name: "Kite",
    printed: &[own(Stat::Intercept, 1, &[])],
    ..VAMP
};

/// "Sabbat Archbishop of Geneva: Once each combat, Marie can burn 1 blood
/// to strike: dodge. Non-weapon equipment cost her -1 blood or pool to
/// equip." A locquipment too (LSJ 20080109).
pub static MARIE_FAUCIGNY: VampireDef = VampireDef {
    name: "Marie Faucigny",
    printed: &[
        own(
            Stat::PoolCostOff,
            1,
            &[R::PlayedCard(CardFilter::EquipmentCardWithout(&[
                Label::Weapon,
            ]))],
        ),
        own(
            Stat::BloodCostOff,
            1,
            &[R::PlayedCard(CardFilter::EquipmentCardWithout(&[
                Label::Weapon,
            ]))],
        ),
    ],
    abilities: &[ability(
        &[Window::CombatStrikeDeclaration],
        Actor::Host,
        &[R::HostBloodAtLeast(1)],
        Limit::PerCombat,
        &[Effect::HostBurnBlood(1), Effect::Strike(StrikeKind::Dodge)],
    )],
    ..VAMP
};

/// "Independent: While Helena is ready, minion cards cost Menele 1 fewer
/// blood. [MERGED] Menele can strike: steal 2 blood or life (becoming
/// blood)." (Advanced.) Any Helena, any Methuselah's; the [MERGED] text
/// inert without the base card (DESIGN §11.3).
pub static MENELE_ADV: VampireDef = VampireDef {
    name: "Menele",
    adv: true,
    printed: &[own(
        Stat::BloodCostOff,
        1,
        &[
            R::ReadyNamed("Helena"),
            R::PlayedCard(CardFilter::MinionCard),
        ],
    )],
    ..VAMP
};

/// "Sabbat: Qawiyya can burn 2 blood to unlock during your minion phase.
/// Red List. +2 strength." Once in the phase (ANK 20190207, rules-qa 6.6),
/// in torpor too (D25); Red List generated.
pub static QAWIYYA_EL_GHADUBA: VampireDef = VampireDef {
    name: "Qawiyya el-Ghaduba",
    printed: &[own(Stat::Strength, 2, &[])],
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[R::OwnMinionPhase, R::HostLocked, R::HostBloodAtLeast(2)],
            &[Effect::HostBurnBlood(2), Effect::UnlockHost],
        )
    }],
    ..VAMP
};

/// "Sabbat: Richard gets -1 intercept when attempting to block Lasombra and
/// Tzimisce."
pub static RICHARD_TAUBER: VampireDef = VampireDef {
    name: "Richard Tauber, Ayelea's Puppet",
    printed: &[
        own(Stat::Intercept, -1, &[R::OpponentClan("Lasombra")]),
        own(Stat::Intercept, -1, &[R::OpponentClan("Tzimisce")]),
    ],
    ..VAMP
};

/// "Sabbat bishop: +1 bleed."
pub static RODOLFO: VampireDef = VampireDef {
    name: "Rodolfo",
    printed: &[own(Stat::Bleed, 1, &[])],
    ..VAMP
};

/// "Sabbat: Yazid gets +1 stealth during directed actions. Black Hand.
/// [MERGED] Anarch." (Advanced.) Black Hand generated; the [MERGED] text
/// inert without the base card (DESIGN §11.3).
pub static YAZID_TAMARI_ADV: VampireDef = VampireDef {
    name: "Yazid Tamari",
    adv: true,
    printed: &[own(Stat::Stealth, 1, &[R::ActionDirected])],
    ..VAMP
};

/// "Independent: Nefertiti can search your crypt for a card named Aabbt
/// Kindred (shuffle afterward), reveal it and move it to your uncontrolled
/// region as a +1 stealth action. [MERGED] During your minion phase,
/// Nefertiti can unlock." (Advanced.) Aabbt Kindred is not in the pool:
/// the search finds nothing, a legal no-op (rules-qa 17.4), but not with
/// an empty crypt (rules-qa 78.20); the [MERGED]
/// text inert without the base card (DESIGN §11.3).
pub static NEFERTITI_ADV: VampireDef = VampireDef {
    name: "Nefertiti",
    adv: true,
    abilities: &[ModeDef {
        requires: &[R::CryptNotEmpty],
        ..printed_action(
            None,
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::CryptNamedToUncontrolled("Aabbt Kindred"),
            ],
        )
    }],
    ..VAMP
};

/// "Camarilla: If you burn the Edge for a vote, you get +1 vote. If you
/// burn a political action card for a vote, you get +1 vote." Her
/// controller's, in torpor too (D25).
pub static FIORENZA_SAVONA: VampireDef = VampireDef {
    name: "Fiorenza Savona",
    printed: &[
        own(Stat::EdgeVoteBonus, 1, &[]),
        own(Stat::BurnVoteBonus, 1, &[]),
    ],
    ..VAMP
};

/// "Independent: Once each turn, Okulos can burn 1 blood to draw 1 card
/// (discard down afterward)." Once in each Methuselah's turn (ANK
/// 20180719-4, Shalmath's), in torpor too (D25).
pub static OKULOS: VampireDef = VampireDef {
    name: "Okulos",
    abilities: &[ability(
        &[Window::AnyImpulse],
        Actor::Host,
        &[R::HostBloodAtLeast(1)],
        Limit::PerTurn,
        &[Effect::HostBurnBlood(1), Effect::DrawThenDiscardDown(1)],
    )],
    ..VAMP
};

/// "Sabbat: If Len is ready at the start of your influence phase, you get
/// +1 transfer. Black Hand." Read as the phase begins; Black Hand
/// generated.
pub static LEN_KONOPKA: VampireDef = VampireDef {
    name: "Len Konopka",
    printed: &[own(Stat::Transfers, 1, &[R::ThisVampireReady])],
    ..VAMP
};

// Round 17 (slice 16ac): twenty promo sets and the TWDA greedy's steps 1-6.

/// "Laibon magaji: Kisha gets -1 stealth on non-hunt actions." A card's
/// hunt is a hunt (rules-qa 72.5); title generated.
pub static KISHA_BHIMJI: VampireDef = VampireDef {
    name: "Kisha Bhimji",
    printed: &[own(Stat::Stealth, -1, &[R::OtherNonHuntAction])],
    ..VAMP
};

/// "Independent: Younger Sabbat vampires get -1 intercept against Lucita."
/// Aimery Methuin's shape, read live on the blocker.
pub static LUCITA: VampireDef = VampireDef {
    name: "Lucita",
    printed: &[own(
        Stat::InterceptAgainst,
        -1,
        &[R::OpponentYounger, R::OpponentSectIn(&[Sect::Sabbat])],
    )],
    ..VAMP
};

/// "Independent: Equipment costs Martino 1 fewer blood or pool to equip."
/// Off whichever the card costs, however it is put on him (Contraband, a
/// search; rules-qa 36.5, 41.13).
pub static MARTINO_DELLA_PASSAGLIA: VampireDef = VampireDef {
    name: "Martino della Passaglia",
    printed: &[own(
        Stat::CostOff,
        1,
        &[R::PlayedCard(CardFilter::Type(CardType::Equipment))],
    )],
    ..VAMP
};

/// "Independent: During your minion phase, Nu can burn 1 blood to unlock.
/// Nu cannot use cards requiring Celerity [cel]. Scarce." Dónal O'Connor's
/// unlock, once in the phase, inside an action or a referendum's polling
/// too (LSJ 20020309); a card's [cel] mode only is barred (LSJ
/// 20050322-2); scarcity generated (LSJ 20011015).
pub static NU_THE_PILLAR: VampireDef = VampireDef {
    name: "Nu, The Pillar",
    printed: &[own(
        Stat::CannotPlay,
        1,
        &[R::PlayedRequiresDiscipline("cel")],
    )],
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[R::OwnMinionPhase, R::HostLocked, R::HostBloodAtLeast(1)],
            &[Effect::HostBurnBlood(1), Effect::UnlockHost],
        )
    }],
    ..VAMP
};

const XAVIAR_PREVENT: ModeDef = ability(
    &[Window::CombatDamagePrevention],
    Actor::Host,
    &[R::DamagePending],
    Limit::PerCombat,
    &[Effect::Prevent(1)],
);

const XAVIAR_STRENGTH: ModeDef = ability(
    &[Window::CombatBeforeRange],
    Actor::Host,
    &[R::HostBloodAtLeast(2)],
    Limit::PerCombat,
    &[Effect::HostBurnBlood(2), Effect::StrengthThisCombat(2)],
);

/// "Independent: Xaviar has 2 votes (titled). He can prevent 1 damage each
/// combat. Once each combat, before range is determined, he may burn 2
/// blood to get +2 strength for the remainder of combat." Omaya's
/// prevention (rules-qa 14.2); votes generated, kept through a sect change
/// (LSJ 20010714).
pub static XAVIAR: VampireDef = VampireDef {
    name: "Xaviar",
    abilities: &[XAVIAR_PREVENT, XAVIAR_STRENGTH],
    ..VAMP
};

/// Merged Xaviar (rulebook "Advanced"): the base card's prevention and
/// strength, the advanced card's unlock after his passed referendum; the
/// advanced card's sect (Camarilla, an anarch token burned: ANK 20180524)
/// and title (Gangrel Justicar) taking precedence over the base's votes.
pub static XAVIAR_MERGED: VampireDef = VampireDef {
    name: "Xaviar",
    adv: true,
    abilities: &[XAVIAR_PREVENT, XAVIAR_STRENGTH, XAVIAR_ADV.abilities[0]],
    ..VAMP
};

/// "Advanced, Sabbat bishop: Jessica gets +1 stealth on political actions.
/// [MERGED] Archbishop of Brussels." Merged with the base card (no text),
/// this definition still reads; the merged title is generated
/// (`CryptCard::merged_title`).
pub static JESSICA_ADV: VampireDef = VampireDef {
    name: "Jessica",
    adv: true,
    printed: &[own(Stat::Stealth, 1, &[R::IsPolitical])],
    ..VAMP
};

/// "Independent: Allies and retainers requiring Necromancy [nec] or
/// Giovanni cost Oliver 1 fewer blood or pool." Máddji's shape, one
/// reduction even with both; however it is put in play (rules-qa 36.5,
/// 41.13).
pub static DR_OLIVER_GENET: VampireDef = VampireDef {
    name: "Dr. Oliver Genet",
    printed: &[own(
        Stat::CostOff,
        1,
        &[
            R::PlayedCard(CardFilter::AllyOrRetainer),
            R::AnyOf(&[
                &[R::PlayedRequiresDiscipline("nec")],
                &[R::PlayedCard(CardFilter::Clan("Giovanni"))],
            ]),
        ],
    )],
    ..VAMP
};

/// "Independent: Hesha gets +1 stealth when hunting. +1 hunt." (Group 2.)
/// The hunt bonus on non-standard hunts too, the blood from the hunt's
/// target (RTR 20030519, RTR 20180511).
pub static HESHA_RUHADZE_G2: VampireDef = VampireDef {
    name: "Hesha Ruhadze",
    group: Some(2),
    printed: &[own(Stat::Stealth, 1, &[R::IsHunt]), own(Stat::Hunt, 1, &[])],
    ..VAMP
};

/// "Sabbat: Joe can enter combat with a werewolf as a +1 stealth Ⓓ action
/// and gets 1 optional maneuver during that combat. Black Hand. +1
/// strength." Another Methuselah's werewolf (rules-qa 17.3); Black Hand
/// generated.
pub static JOE_BOOT_HILL: VampireDef = VampireDef {
    name: "Joe \"Boot\" Hill",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[printed_action(
        Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[
                R::NotControlled,
                R::ActorReady,
                R::TargetSubtypeIn(&[Label::Werewolf]),
            ],
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::EnterCombatWithTarget,
            Effect::InResultingCombat(super::BlockRider::Maneuver),
        ],
    )],
    ..VAMP
};

/// "Sabbat cardinal: During each Methuselah's minion phase, Hannibal can
/// burn 2 blood to unlock." (Advanced; the base card is not in the pool.)
/// Once in each minion phase (the "during" template, rules-qa 58.7), in
/// torpor too (D25): one mode for his controller's phase, one for the
/// others'.
pub static HANNIBAL_ADV: VampireDef = VampireDef {
    name: "Hannibal",
    adv: true,
    abilities: &[
        ModeDef {
            limit: Some(Limit::PerTurn),
            ..mode_of(
                &[Window::AnyImpulse],
                Actor::Host,
                &[R::OwnMinionPhase, R::HostLocked, R::HostBloodAtLeast(2)],
                &[Effect::HostBurnBlood(2), Effect::UnlockHost],
            )
        },
        ModeDef {
            limit: Some(Limit::PerTurn),
            ..mode_of(
                &[Window::AnyImpulse],
                Actor::Host,
                &[R::OthersMinionPhase, R::HostLocked, R::HostBloodAtLeast(2)],
                &[Effect::HostBurnBlood(2), Effect::UnlockHost],
            )
        },
    ],
    ..VAMP
};

/// "Independent: During a bleed action, Anu can burn 1 blood to give the
/// acting minion +1 bleed. Scarce." Any Methuselah's bleeding minion, Anu
/// herself included (ANK 20220502), before resolution (ANK 20231124);
/// once each action (the "during" template, rules-qa 79.1); scarcity
/// generated.
pub static ANU_DIPTINATPA: VampireDef = VampireDef {
    name: "Anu Diptinatpa",
    abilities: &[ModeDef {
        limit: Some(Limit::PerAction),
        ..mode_of(
            &[Window::ActionDuring, Window::ReactionDuringAction],
            Actor::Host,
            &[R::IsBleed, R::HostBloodAtLeast(1)],
            &[Effect::HostBurnBlood(1), bleed(1, false)],
        )
    }],
    ..VAMP
};

/// "Camarilla: Echo gets +1 strength in combat with a Red List minion."
pub static ECHO: VampireDef = VampireDef {
    name: "Echo",
    printed: &[own(Stat::Strength, 1, &[R::OpponentRedList])],
    ..VAMP
};

/// "Sabbat. Black Hand: Kestrelle gets +1 stealth during actions directed
/// at a Methuselah who doesn't control a ready Sabbat vampire." Read live
/// on the action's targets, any of them (PIB 20150526: Vaticination);
/// Black Hand generated.
pub static KESTRELLE_HAYES: VampireDef = VampireDef {
    name: "Kestrelle Hayes",
    printed: &[own(
        Stat::Stealth,
        1,
        &[R::TargetSeatControlsNoReadySect(Sect::Sabbat)],
    )],
    ..VAMP
};

const CLAUDIO_TOLL: Printed = Printed {
    stat: Stat::BloodCostMore,
    n: 1,
    to: PrintedTo::Opponent,
    per: Per::One,
    requires: &[R::ActorVampire, R::PlayedAsCombat],
};

const CLAUDIO_NO_GEHENNA: Rider = Rider {
    hook: Hook::UnlockPhase,
    scope: RiderScope::Controller,
    requires: &[R::NoGehennaInPlay],
    effects: &[Effect::BurnPool(1)],
};

/// "Camarilla: Combat cards cost an opposing vampire an additional blood.
/// During your unlock phase, burn 1 pool if there are no Gehenna cards in
/// play." (The base card.) Murat's toll, an ally paying none; the burn in
/// torpor too (D25), read as the phase's riders fire.
pub static CLAUDIO_SEVERINO_BASE: VampireDef = VampireDef {
    name: "Claudio Severino",
    printed: &[CLAUDIO_TOLL],
    riders: &[CLAUDIO_NO_GEHENNA],
    ..VAMP
};

/// Merged Claudio Severino (rulebook "Advanced"): the base card's toll and
/// unlock-phase burn, the advanced card's discard-phase action and its
/// [MERGED] stealth, as often as paid (no "during", rules-qa 79.3).
pub static CLAUDIO_SEVERINO_MERGED: VampireDef = VampireDef {
    name: "Claudio Severino",
    adv: true,
    printed: &[CLAUDIO_TOLL],
    riders: &[CLAUDIO_NO_GEHENNA],
    abilities: &[
        CLAUDIO_SEVERINO.abilities[0],
        mode_of(
            &[Window::ActionDuring],
            Actor::Host,
            &[R::HostBloodAtLeast(1), R::ActorActing, R::StealthNeeded],
            &[Effect::HostBurnBlood(1), Effect::AddStealth(1)],
        ),
    ],
    ..VAMP
};

/// A Tremere slave's printed half (rulebook "Slave"; Tupdog's, rules-qa
/// 62.14): no directed action while its controller controls no ready
/// Tremere.
const TREMERE_SLAVE: Printed = own(
    Stat::NoDirectedActions,
    1,
    &[R::ControlsNoReadyOfClan("Tremere")],
);

/// A Tremere slave's ability: as a Tremere of its controller's is blocked,
/// it may lock to fight the blocker instead (advanced rules §5.7.2).
const TREMERE_SLAVE_FIGHTS: ModeDef = mode_of(
    &[Window::ActionBlocked],
    Actor::Host,
    &[
        R::HostReady,
        R::HostUnlocked,
        R::ActingClan("Tremere"),
        R::NoSlaveInstead,
    ],
    &[Effect::LockHost, Effect::SlaveTakesCombat],
);

/// The slave a card creates (Raw Recruit's Gargoyle, "enslaved to the
/// acting vampire's clan"): as a vampire of that clan its controller
/// controls is blocked, it may lock to fight the blocker instead (advanced
/// rules §5.7.2); its directed-action bar is `GameState::slave_clan`'s.
/// Never a crypt card's: `cards::vampire_of` only.
pub static RECRUITED_SLAVE: VampireDef = VampireDef {
    name: "",
    abilities: &[mode_of(
        &[Window::ActionBlocked],
        Actor::Host,
        &[
            R::HostReady,
            R::HostUnlocked,
            R::ActingOfHostsMasterClan,
            R::NoSlaveInstead,
        ],
        &[Effect::LockHost, Effect::SlaveTakesCombat],
    )],
    ..VAMP
};

/// "Camarilla: Porphyry gets +2 bleed against a Methuselah who controls a
/// location. Flight [FLIGHT]. Tremere slave." An uncontested location, on
/// a minion too, read live on the bleed's target; Flight generated.
pub static PORPHYRY: VampireDef = VampireDef {
    name: "Porphyry",
    printed: &[
        own(Stat::Bleed, 2, &[R::TargetSeatControlsLocation]),
        TREMERE_SLAVE,
    ],
    abilities: &[TREMERE_SLAVE_FIGHTS],
    ..VAMP
};

/// "Camarilla: As Verbruch enters play, you can search your crypt for a
/// Tremere, reveal them, and move them to your uncontrolled region. Flight
/// [FLIGHT]. Tremere slave." Before any contest (LSJ 20080526), the crypt
/// shuffled (Disciple of Lazarus's search); Flight generated.
pub static VERBRUCH: VampireDef = VampireDef {
    name: "Verbruch",
    printed: &[TREMERE_SLAVE],
    abilities: &[TREMERE_SLAVE_FIGHTS],
    riders: &[Rider {
        hook: Hook::EntersPlay,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::CryptSearchOnEntry("Tremere")],
    }],
    ..VAMP
};

/// "Sabbat: Rescuing Mitchell from torpor costs +1 blood." Read as the
/// rescue is announced and as it resolves (rules-qa 48.8).
pub static MITCHELL_THE_HEADHUNTER: VampireDef = VampireDef {
    name: "Mitchell, The Headhunter",
    printed: &[own(Stat::RescueCostMore, 1, &[])],
    ..VAMP
};

/// "Independent: Reverend Adams gets -1 stealth on political actions.
/// Older vampires do not lock for successfully blocking Reverend Adams."
/// Older: of higher capacity; an ally locks.
pub static REVEREND_ADAMS: VampireDef = VampireDef {
    name: "Reverend Adams",
    printed: &[
        own(Stat::Stealth, -1, &[R::IsPolitical]),
        own(Stat::BlockerDoesNotLock, 1, &[R::OpponentOlder]),
    ],
    ..VAMP
};

/// "Sabbat cardinal: Yong-Sun may burn the top card of your prey's crypt
/// as a Ⓓ action." To that Methuselah's ash heap; an empty crypt, nothing
/// (a legal no-op, rules-qa 17.4); title generated.
pub static YONG_SUN: VampireDef = VampireDef {
    name: "Yong-Sun, Harmonist",
    abilities: &[printed_action(
        Some(TargetSpec {
            kind: TargetKind::Seat,
            filter: &[R::TargetPrey, R::NotSelf],
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::BurnTargetCryptTop,
        ],
    )],
    ..VAMP
};

/// "Independent. Red List: If a blood hunt is successfully called on
/// Genina, she goes to torpor instead of being burned. Genina gets +1
/// stealth on diablerie actions and on undirected actions." One stealth
/// for an undirected diablerie; Red List generated.
pub static GENINA: VampireDef = VampireDef {
    name: "Genina, The Red Poet",
    printed: &[
        own(Stat::TorporInsteadOfBloodHuntBurn, 1, &[]),
        own(
            Stat::Stealth,
            1,
            &[R::AnyOf(&[&[R::IsDiablerie], &[R::ActionUndirected]])],
        ),
    ],
    ..VAMP
};

/// "Independent: While Guillaume is ready, you get +1 hand size for each
/// location you control. He gets +1 stealth during recruit and employ
/// actions." Uncontested locations (LSJ 20080107), on minions too.
pub static GUILLAUME_GIOVANNI: VampireDef = VampireDef {
    name: "Guillaume Giovanni",
    printed: &[
        Printed {
            stat: Stat::HandSize,
            n: 1,
            to: PrintedTo::Itself,
            per: Per::LocationsControlled,
            requires: &[R::ThisVampireReady],
        },
        own(Stat::Stealth, 1, &[R::IsRecruitOrEmploy]),
    ],
    ..VAMP
};

/// "Independent. 2 votes (titled): Lambach can search your library for a
/// card named Eye of Hazimel and equip it at no cost as a +1 stealth
/// action (shuffle afterward). [MERGED] Lambach can unlock after
/// successfully performing an action to put a vampire in play."
/// (Advanced.) An equip action (TOM 19960130), the Eye's requirements
/// applying; votes generated (LSJ 20010714). His base card is not in the
/// pool: the [MERGED] text never applies.
pub static LAMBACH_ADV: VampireDef = VampireDef {
    name: "Lambach",
    adv: true,
    abilities: &[printed_action(
        None,
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::SearchEquipFree(&[CardFilter::Named("Eye of Hazimel")]),
        ],
    )],
    ..VAMP
};

/// "Once each combat, before range is determined, <she> can equip with a
/// weapon from your hand (requirements and cost apply as normal)": Fatima
/// al-Faqadi's and White Lily's. Only in her own combat (ANK 20221021);
/// the weapon named in hand as it is used (LSJ 20100130), from the hand
/// only (LSJ 20050913); not an equip action nor a card she plays (LSJ
/// 20080815, ANK 20201229): reductions keyed to an action do not apply,
/// others do (LSJ 20090115-1, ANK 20230824).
const WEAPON_FROM_HAND: ModeDef = ModeDef {
    terms: Some(Terms::EquipFromHand {
        filter: CardFilter::HasLabel(Label::Weapon),
        blood_share: false,
    }),
    limit: Some(Limit::PerCombat),
    ..mode_of(
        &[Window::CombatBeforeRange],
        Actor::Host,
        &[],
        &[Effect::EquipNamed],
    )
};

/// "Independent: Once each combat, before range is determined, Fatima can
/// equip with a weapon from your hand (requirements and cost apply as
/// normal). Blood cursed." (`WEAPON_FROM_HAND`); blood cursed generated.
pub static FATIMA_AL_FAQADI: VampireDef = VampireDef {
    name: "Fatima al-Faqadi",
    abilities: &[WEAPON_FROM_HAND],
    ..VAMP
};

/// "Sabbat: Once each combat, before range is determined, White Lily can
/// equip with a weapon from your hand (requirements and cost apply as
/// normal). Black Hand." (`WEAPON_FROM_HAND`); Black Hand generated.
pub static WHITE_LILY: VampireDef = VampireDef {
    name: "White Lily",
    abilities: &[WEAPON_FROM_HAND],
    ..VAMP
};

/// "Sabbat priscus: Once each turn, after a vampire in combat with Sascha
/// goes to torpor, you can get +2 hand size until the end of the turn."
/// However it goes to torpor, Sascha in torpor too (D25), even if he left
/// in the same event (advanced rules §6.4.4); a decline spends nothing;
/// once in each Methuselah's turn (ANK 20180719-4); title generated.
pub static SASCHA_VYKOS: VampireDef = VampireDef {
    name: "Sascha Vykos, The Angel of Caine",
    riders: &[Rider {
        hook: Hook::CombatOpponentLeftReady,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::MayHandSizeOncePerTurn(2)],
    }],
    ..VAMP
};

/// "Anarch: After your minion phase ends, lock Tori if she did not perform
/// an action this turn." In torpor too (LSJ 20011022, LSJ 20100902-2); an
/// action announced counts, whatever became of it (rules-qa 32.10).
pub static TORI_LONGWOOD: VampireDef = VampireDef {
    name: "Tori Longwood",
    riders: &[Rider {
        hook: Hook::MinionPhaseEnd,
        scope: RiderScope::Every,
        requires: &[R::HostAnnouncedNoAction],
        effects: &[Effect::LockHost],
    }],
    ..VAMP
};

/// "Camarilla: When Victor enters play, you may pay 1 pool to make him a
/// primogen, 2 pool to make him Prince of Paris, or 3 pool to make him
/// Ventrue Justicar. If Victor's title would be contested, he immediately
/// yields instead of contesting." Bought before he enters contest (LSJ
/// 20080526, LSJ 20100902), payable down to 0 pool (a cost), never a title
/// his Methuselah already holds (no contest with oneself, rules-qa 76.2);
/// the yield loses the title, whether he or a rival came second.
pub static VICTOR_PELLETIER: VampireDef = VampireDef {
    name: "Victor Pelletier",
    printed: &[own(Stat::YieldsTitleInsteadOfContest, 1, &[])],
    riders: &[Rider {
        hook: Hook::EntersPlay,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::BuyTitleOnEntry(&[
            (1, Title::Primogen),
            (2, Title::Prince("Paris")),
            (3, Title::Justicar("Ventrue")),
        ])],
    }],
    ..VAMP
};

/// "Independent: Once each turn, if Baldesar is ready, he can reduce the
/// cost of a location or of an equipment you play by 1 blood or pool. +1
/// strength." Announced with the card, spent as it is paid (rules-qa
/// 60.7), a card another card plays included (Contraband, a search:
/// rules-qa 41.13); once in each Methuselah's turn (ANK 20180719-4).
pub static BALDESAR_ROSSELLINI: VampireDef = VampireDef {
    name: "Baldesar Rossellini",
    printed: &[
        own(Stat::Strength, 1, &[]),
        own(
            Stat::SeatCostOffOncePerTurn,
            1,
            &[
                R::ThisVampireReady,
                R::AnyOf(&[
                    &[R::PlayedCard(CardFilter::Location)],
                    &[R::PlayedCard(CardFilter::Type(CardType::Equipment))],
                ]),
            ],
        ),
    ],
    ..VAMP
};

/// "Advanced, Camarilla: While Alan is ready, you may pay some or all of
/// the pool cost of equipping from any investment cards you control.
/// [MERGED] During your master phase, if Alan is ready, you may move a
/// counter from any investment card to your pool." No investment card is
/// in the pool (Wall Street Night's precedent): the text is inert; his
/// base card is not in the pool either.
pub static ALAN_SOVEREIGN_ADV: VampireDef = VampireDef {
    name: "Alan Sovereign",
    adv: true,
    ..VAMP
};

/// "Independent: Barbaro can change his clan to Tremere antitribu as +1
/// stealth action. He can put 1 taint counter on a minion as a +1 stealth
/// Ⓓ action; if the minion has 2 taint counters, burn them and the minion
/// becomes infernal. Infernal." Two actions, each its own (LSJ 20100909);
/// the change for good, a legal no-op once Tremere antitribu; any minion,
/// his own (undirected, D12) and one in torpor included; infernal
/// generated.
pub static BARBARO_LUCCHESE: VampireDef = VampireDef {
    name: "Barbaro Lucchese",
    abilities: &[
        printed_action(
            None,
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::ActorBecomesClan("Tremere antitribu"),
            ],
        ),
        printed_action(
            Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[],
            }),
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::TaintTarget,
            ],
        ),
    ],
    ..VAMP
};

/// "Independent. Red List: All Blood Brothers may play cards that require
/// Sanguinus [san] as if Angelo were a member of any circle. Angelo can
/// burn a location as a Ⓓ action. He cannot have or use retainers.
/// Sterile." Of the circle of any other Blood Brother playing a card
/// requiring Sanguinus, for that card and its effects, never for a card in
/// play's nor a vampire's ability (LSJ 20101013, LSJ 20050119; the circle
/// requirements' `sanguinus`); his own circle of one otherwise
/// (LSJ 20020609); Diego Giovanni's burn without its cost; no employ (LSJ
/// 20050315); Red List and sterile generated.
pub static ANGELO: VampireDef = VampireDef {
    name: "Angelo",
    printed: &[
        own(Stat::AnyCircleForSanguinus, 1, &[]),
        own(Stat::NoRetainers, 1, &[]),
    ],
    abilities: &[printed_action(
        Some(TargetSpec {
            kind: TargetKind::Card,
            filter: &[R::TargetCard(CardFilter::Location)],
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::BurnTargetCard,
        ],
    )],
    ..VAMP
};

/// "Camarilla: Action cards and strike cards that require Celerity [cel]
/// cost Antoinette an additional blood." Action cards of every type
/// (rules-qa 60.4); a strike card is one played in a strike mode, a dodge
/// included (42.2), requiring [cel] in the mode played (38.4).
pub static ANTOINETTE_DUCHAMP: VampireDef = VampireDef {
    name: "Antoinette DuChamp",
    printed: &[own(
        Stat::BloodCostMore,
        1,
        &[
            R::PlayedRequiresDiscipline("cel"),
            R::AnyOf(&[
                &[R::PlayedCard(CardFilter::ActionCard)],
                &[R::PlayedInStrikeMode],
            ]),
        ],
    )],
    ..VAMP
};

/// "Camarilla: Chandler gets +1 intercept when attempting to block other
/// Gangrel."
pub static CHANDLER_HUNGERFORD: VampireDef = VampireDef {
    name: "Chandler Hungerford",
    printed: &[own(Stat::Intercept, 1, &[R::OpponentClan("Gangrel")])],
    ..VAMP
};

/// "Sabbat: Christine gets -1 intercept during bleed actions."
pub static CHRISTINE_BOSCACCI: VampireDef = VampireDef {
    name: "Christine Boscacci",
    printed: &[own(Stat::Intercept, -1, &[R::IsBleed])],
    ..VAMP
};

/// "Independent: Eurayle can unlock a minion as an action; that action gets
/// +2 stealth if you control the minion. Scarce." Any minion, herself
/// included; directed at the minion's controller otherwise (PIB 20121210);
/// scarce generated.
pub static EURAYLE_GELASIA_MYLONAS: VampireDef = VampireDef {
    name: "Eurayle Gelasia Mylonas",
    abilities: &[printed_action(
        target(TargetKind::Minion, &[]),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::StealthIfOwnTarget(2),
            Effect::UnlockTarget,
        ],
    )],
    ..VAMP
};

/// "Camarilla: Fergus can enter combat with a locked Sabbat vampire as a Ⓓ
/// action." A target unlocked by resolution: successful, no effect (ANK
/// 20210124); a ready one, as Nathaniel Bordruff's.
pub static FERGUS_ALEXANDER: VampireDef = VampireDef {
    name: "Fergus Alexander",
    abilities: &[enter_combat(&[
        R::NotControlled,
        R::ActorReady,
        R::TargetVampire,
        R::TargetLocked,
        R::ActorSectIn(&[Sect::Sabbat]),
    ])],
    ..VAMP
};

/// "Sabbat priscus: If you also control Humo, Greensleeves may burn a blood
/// to give Humo +1 bleed during a bleed action." Humo is not in the pool:
/// the text is inert (Alan Sovereign's precedent).
pub static GREENSLEEVES: VampireDef = VampireDef {
    name: "Greensleeves",
    ..VAMP
};

/// "Camarilla: If Julius goes into torpor, burn him." Once in torpor, its
/// riders fired.
pub static JULIUS: VampireDef = VampireDef {
    name: "Julius",
    printed: &[own(Stat::BurnedAfterTorpor, 1, &[])],
    ..VAMP
};

/// "Camarilla: Navar cannot block older vampires."
pub static NAVAR_MCCLAREN: VampireDef = VampireDef {
    name: "Navar McClaren",
    printed: &[own(Stat::CannotBlock, 1, &[R::OpponentOlder])],
    ..VAMP
};

/// "Camarilla: After Smudge diablerizes an older vampire for the first
/// time, he gets +1 strength." Once, for good.
pub static SMUDGE_THE_IGNORED: VampireDef = VampireDef {
    name: "Smudge the Ignored",
    printed: &[own(Stat::Strength, 1, &[R::DiablerizedOlder])],
    ..VAMP
};

// ---- Round 19: the Imbued (rulebook "Appendix: Imbued Rules") ----

/// "An imbued may leave the incapacitated region and gain a life (not to
/// exceed their starting life) by burning 3 conviction during their unlock
/// phase" (rulebook "Appendix: Imbued Rules" 5): every imbued's.
const RECOVER: ModeDef = mode_of(
    &[Window::UnlockPhaseOwn],
    Actor::Host,
    &[R::HostIncapacitated, R::HostConvictionAtLeast(3)],
    &[Effect::BurnConviction(3), Effect::LeaveIncapacitated],
);

/// Jennie "Cassie247" Orne's grant, every imbued's while she is in play:
/// "any incapacitated imbued may move to the ready region and gain a life
/// … by burning 2 conviction during any Methuselah's unlock phase".
const RECOVER_GRANTED: ModeDef = mode_of(
    &[Window::UnlockPhaseAny],
    Actor::Host,
    &[
        R::HostIncapacitated,
        R::ImbuedRecoveryGranted,
        R::HostConvictionAtLeast(2),
    ],
    &[Effect::BurnConviction(2), Effect::LeaveIncapacitated],
);

/// "Anna may move 2 blood from the blood bank to any vampire as a +1
/// stealth action. During your unlock phase, you may look at the top three
/// cards of your library."
pub static ANNA_DICTATRIX11_SULJIC: VampireDef = VampireDef {
    name: "Anna \"Dictatrix11\" Suljic",
    abilities: &[
        printed_action(
            target(TargetKind::Minion, &[R::TargetVampire]),
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::BloodToTarget(2),
            ],
        ),
        mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[],
            &[Effect::LookAtLibraryTop(3)],
        ),
        RECOVER,
        RECOVER_GRANTED,
    ],
    ..VAMP
};

/// "During your unlock phase, another ready imbued you control takes 1
/// unpreventable damage. +1 bleed."
pub static BEATRICE_ORACLE171_TREMBLAY: VampireDef = VampireDef {
    name: "Béatrice \"Oracle171\" Tremblay",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[RECOVER, RECOVER_GRANTED],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::OtherReadyImbuedDamaged(1)],
    }],
    ..VAMP
};

/// "Earl gets +1 stealth on actions other than actions to enter combat.
/// During your unlock phase, if you control more than two ready imbued,
/// Earl burns 1 conviction [1 CONVICTION]." Earl counted among them.
pub static EARL_SHAKA74_DEAMS: VampireDef = VampireDef {
    name: "Earl \"Shaka74\" Deams",
    printed: &[own(Stat::Stealth, 1, &[R::ActionNotEnteringCombat])],
    abilities: &[RECOVER, RECOVER_GRANTED],
    riders: &[Rider {
        hook: Hook::UnlockPhase,
        scope: RiderScope::Controller,
        requires: &[R::HostControllerReadyImbuedAtLeast(3)],
        effects: &[Effect::HostBurnsConviction(1)],
    }],
    ..VAMP
};

/// "If Erick successfully enters combat with a monster as an action, he
/// may gain a conviction [1 CONVICTION] from your hand or ash heap (before
/// combat begins)."
pub static ERICK_SHOPHET125_FRANCO: VampireDef = VampireDef {
    name: "Erick \"Shophet125\" Franco",
    abilities: &[
        ability(
            &[Window::CombatBeforeRange],
            Actor::Host,
            &[
                R::ActorActing,
                R::ActionEntersCombat,
                R::CombatFromSuccess,
                R::OpponentMonster,
                R::FirstRound,
            ],
            Limit::PerCombat,
            &[Effect::GainConviction { library: false }],
        ),
        RECOVER,
        RECOVER_GRANTED,
    ],
    ..VAMP
};

/// "Other Methuselahs' actions that target François cost an additional
/// blood or life."
pub static FRANCOIS_WARDEN_LOEHR: VampireDef = VampireDef {
    name: "François \"Warden\" Loehr",
    printed: &[own(Stat::TargetedCostsBlood, 1, &[])],
    abilities: &[RECOVER, RECOVER_GRANTED],
    ..VAMP
};

/// "When Inez enters play, you may search your library (shuffle afterward)
/// or hand for a power that requires innocence and put it on her." Before
/// any contest (LSJ 20080526).
pub static INEZ_NURSE216_VILLAGRANDE: VampireDef = VampireDef {
    name: "Inez \"Nurse216\" Villagrande",
    abilities: &[RECOVER, RECOVER_GRANTED],
    riders: &[Rider {
        hook: Hook::EntersPlay,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::SearchPowerOnEntry("inn")],
    }],
    ..VAMP
};

/// "Jack gets an optional maneuver on the first round of combat."
pub static JACK_HANNIBAL137_HARMON: VampireDef = VampireDef {
    name: "Jack \"Hannibal137\" Harmon",
    abilities: &[
        ability(
            &[Window::CombatDetermineRange],
            Actor::Host,
            &[R::FirstRound],
            Limit::PerCombat,
            &[Effect::Maneuver],
        ),
        RECOVER,
        RECOVER_GRANTED,
    ],
    ..VAMP
};

/// "Weapons cost Jennifer 1 less pool or blood. +1 bleed. -1 stealth."
pub static JENNIFER_FLAME61_VIDISANIA: VampireDef = VampireDef {
    name: "Jennifer \"Flame61\" Vidisania",
    printed: &[
        own(
            Stat::CostOff,
            1,
            &[R::PlayedCard(CardFilter::HasLabel(Label::Weapon))],
        ),
        own(Stat::Bleed, 1, &[]),
        own(Stat::Stealth, -1, &[]),
    ],
    abilities: &[RECOVER, RECOVER_GRANTED],
    ..VAMP
};

/// "John may enter combat with a monster as a Ⓓ action that costs 1
/// conviction [1 CONVICTION]."
pub static JOHN_COP90_OMALLEY: VampireDef = VampireDef {
    name: "John \"Cop90\" O'Malley",
    abilities: &[
        printed_action(
            target(
                TargetKind::Minion,
                &[R::NotControlled, R::ActorReady, R::TargetMonster],
            ),
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::PayConviction,
                Effect::EnterCombatWithTarget,
            ],
        ),
        RECOVER,
        RECOVER_GRANTED,
    ],
    ..VAMP
};

/// "During your discard phase, Leaf may lock to gain a conviction [1
/// CONVICTION] from your library (shuffle afterward), hand or ash heap."
pub static LEAF_POTTER116_PANKOWSKI: VampireDef = VampireDef {
    name: "Leaf \"Potter116\" Pankowski",
    abilities: &[
        mode_of(
            &[Window::DiscardPhase],
            Actor::Host,
            &[R::HostUnlocked],
            &[Effect::LockHost, Effect::GainConviction { library: true }],
        ),
        RECOVER,
        RECOVER_GRANTED,
    ],
    ..VAMP
};

/// "Liz doesn't gain a conviction [1 CONVICTION] as normal when entering
/// play."
pub static LIZ_TICKET312_THORNTON: VampireDef = VampireDef {
    name: "Liz \"Ticket312\" Thornton",
    abilities: &[RECOVER, RECOVER_GRANTED],
    riders: &[Rider {
        hook: Hook::EntersPlay,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::NoEntryConviction],
    }],
    ..VAMP
};

/// "If Lupe has a vehicle, once each action she may burn a conviction [1
/// CONVICTION] to get +1 stealth or +1 intercept." One use for both.
pub static LUPE_CABBIE22_DROIN: VampireDef = VampireDef {
    name: "Lupe \"Cabbie22\" Droin",
    abilities: &[
        ability(
            &[Window::ActionDuring],
            Actor::Host,
            &[
                R::ActorActing,
                R::ActorHolds(super::Holding::Label(Label::Vehicle)),
                R::StealthNeeded,
                R::HostConvictionAtLeast(1),
            ],
            Limit::PerActionCard,
            &[Effect::BurnConviction(1), Effect::AddStealth(1)],
        ),
        ability(
            &[Window::ActionStealthIntercept],
            Actor::Host,
            &[
                R::ActorHolds(super::Holding::Label(Label::Vehicle)),
                R::InterceptNeeded,
                R::HostConvictionAtLeast(1),
            ],
            Limit::PerActionCard,
            &[Effect::BurnConviction(1), Effect::AddIntercept(1)],
        ),
        RECOVER,
        RECOVER_GRANTED,
    ],
    ..VAMP
};

/// "During a referendum, Maman may burn a conviction [1 CONVICTION] to get 3
/// votes against the referendum. You may lock Maman before range is
/// determined in a combat between two monsters to end that combat."
pub static MAMAN_BOUMBA: VampireDef = VampireDef {
    name: "Maman Boumba",
    abilities: &[
        ability(
            &[Window::Referendum],
            Actor::Host,
            &[R::AnyPollingStep, R::HostConvictionAtLeast(1)],
            Limit::PerReferendum,
            &[Effect::BurnConviction(1), Effect::AddVotesAgainst(3)],
        ),
        mode_of(
            &[Window::CombatBeforeRange],
            Actor::HostOutOfCombat,
            &[R::HostUnlocked, R::CombatBothMonsters],
            &[Effect::LockHost, Effect::EndCombat],
        ),
        RECOVER,
        RECOVER_GRANTED,
    ],
    ..VAMP
};

/// "Marion gets an optional press each combat."
pub static MARION_TEACHER193_PERKS: VampireDef = VampireDef {
    name: "Marion \"Teacher193\" Perks",
    printed: &[own(Stat::CombatPress, 1, &[])],
    abilities: &[RECOVER, RECOVER_GRANTED],
    ..VAMP
};

/// "During your influence phase, you may move a counter from your pool to
/// an imbued in your uncontrolled region." Influenced out, usable in that
/// influence phase (RTR 20180303).
pub static PAUL_SIXOFSWORDS29_MORETON: VampireDef = VampireDef {
    name: "Paul \"Sixofswords29\" Moreton",
    abilities: &[
        ModeDef {
            target: target(TargetKind::Uncontrolled, &[R::TargetImbued]),
            ..mode_of(
                &[Window::InfluencePhaseOwn],
                Actor::Host,
                &[],
                &[Effect::PoolToUncontrolledTarget(1)],
            )
        },
        RECOVER,
        RECOVER_GRANTED,
    ],
    ..VAMP
};

/// "+1 strength. Pedro cannot maneuver to long range, press to end combat,
/// or end combat as a strike."
pub static PEDRO_CORTEZ: VampireDef = VampireDef {
    name: "Pedro Cortez",
    printed: &[
        own(Stat::Strength, 1, &[]),
        own(Stat::NoManeuverToLong, 1, &[]),
        own(Stat::NoPressToEnd, 1, &[]),
        own(Stat::NoCombatEnds, 1, &[]),
    ],
    abilities: &[RECOVER, RECOVER_GRANTED],
    ..VAMP
};

/// "While Peter is in combat with a monster, he may burn a conviction [1
/// CONVICTION] to get a maneuver."
pub static PETER_OUTBACK295_ROPHAIL: VampireDef = VampireDef {
    name: "Peter \"Outback295\" Rophail",
    abilities: &[
        mode_of(
            &[Window::CombatDetermineRange],
            Actor::Host,
            &[R::OpponentMonster, R::HostConvictionAtLeast(1)],
            &[Effect::BurnConviction(1), Effect::Maneuver],
        ),
        RECOVER,
        RECOVER_GRANTED,
    ],
    ..VAMP
};

/// "Travis may move 2 blood from the blood bank to an imbued in your
/// uncontrolled region as a +1 stealth action that costs 1 conviction [1
/// CONVICTION]." The conviction named as the action is announced (RTR
/// 20111202).
pub static TRAVIS_TRAVELER72_MILLER: VampireDef = VampireDef {
    name: "Travis \"Traveler72\" Miller",
    abilities: &[
        printed_action(
            target(TargetKind::Uncontrolled, &[R::TargetImbued]),
            &[
                Effect::DeclareAction(ActionKind::Ability),
                Effect::AddStealth(1),
                Effect::PayConviction,
                Effect::BloodToUncontrolledTarget(2),
            ],
        ),
        RECOVER,
        RECOVER_GRANTED,
    ],
    ..VAMP
};

/// "If Xian successfully blocks a monster and is still ready at the end of
/// the action, he may gain a conviction [1 CONVICTION] from your hand or
/// ash heap."
pub static XIAN_DZIDZAT155_QUAN: VampireDef = VampireDef {
    name: "Xian \"DziDzat155\" Quan",
    abilities: &[
        ability(
            &[Window::ActionAfterCombat],
            Actor::Host,
            &[R::ActorHasBlocked, R::ActingMonster, R::HostReady],
            Limit::PerAction,
            &[Effect::GainConviction { library: false }],
        ),
        RECOVER,
        RECOVER_GRANTED,
    ],
    ..VAMP
};

/// "Any incapacitated imbued may move to the ready region and gain a life
/// (not to exceed his or her starting amount) by burning 2 conviction [2
/// CONVICTION] during any Methuselah's unlock phase." Any Methuselah's
/// imbued; a life only, the rulebook's included.
pub static JENNIE_CASSIE247_ORNE: VampireDef = VampireDef {
    name: "Jennie \"Cassie247\" Orne",
    printed: &[own(Stat::GrantsImbuedRecovery, 1, &[])],
    abilities: &[RECOVER, RECOVER_GRANTED],
    ..VAMP
};

// ---- Round 20: the cards players asked for (/admin/unsupported, 2026-10-23) ----
//
// Andrew Emory ("Sabbat bishop."), Dame Hollerton and Randel, The Coward
// ("Sabbat.") print no text; Banjoko's "Sabbat: Black Hand. Seraph." are
// generated traits (LSJ 20070322, ANK 20180807).

/// "Sabbat: If Isanwayen is ready during your master phase, you can lock
/// him to get +1 master phase action. Rescuing him from torpor costs 1
/// additional blood." The rescue read as announced and as it resolves
/// (rules-qa 48.8).
pub static ISANWAYEN: VampireDef = VampireDef {
    name: "Isanwayen",
    printed: &[own(Stat::RescueCostMore, 1, &[])],
    abilities: &[mode_of(
        &[Window::MasterPhase],
        Actor::Host,
        &[R::HostReady, R::HostUnlocked],
        &[Effect::LockHost, Effect::GainMasterPhaseAction],
    )],
    ..VAMP
};

/// "Sabbat: The Arcadian can bleed with +1 bleed as a +1 stealth Ⓓ action
/// that costs 1 blood."
pub static THE_ARCADIAN: VampireDef = VampireDef {
    name: "Arcadian, The",
    abilities: &[ModeDef {
        requires: &[R::ActorBloodAtLeast(1)],
        ..mode_of(
            &[Window::ActionDeclare],
            Actor::Host,
            &[],
            &[
                Effect::DeclareAction(ActionKind::Bleed),
                Effect::PayBlood(1),
                Effect::AddStealth(1),
                bleed(1, false),
            ],
        )
    }],
    ..VAMP
};

/// "Sabbat: If Bartholomew is unlocked at the beginning of your turn, you
/// gain 1 pool from the blood bank." Read as the turn begins, before the
/// unlock phase unlocks him; in torpor too.
pub static BARTHOLOMEW: VampireDef = VampireDef {
    name: "Bartholomew",
    printed: &[own(Stat::PoolAtTurnStart, 1, &[R::HostUnlocked])],
    ..VAMP
};

/// "Sabbat: Julia can bleed any Methuselah as a Ⓓ action. Once each action,
/// Julia can burn 1 blood to get +1 bleed." The Methuselah chosen as the
/// action is announced; the +1 during her bleed, before resolution (ANK
/// 20231124, LSJ 19970718).
pub static JULIA_PRIMA: VampireDef = VampireDef {
    name: "Julia Prima",
    abilities: &[
        ModeDef {
            target: Some(TargetSpec {
                kind: TargetKind::Seat,
                filter: &[R::NotSelf, R::NotPrey],
            }),
            ..mode_of(
                &[Window::ActionDeclare],
                Actor::Host,
                &[],
                &[Effect::DeclareAction(ActionKind::Bleed)],
            )
        },
        ModeDef {
            limit: Some(Limit::PerAction),
            ..mode_of(
                &[Window::ActionDuring],
                Actor::Host,
                &[R::ActorActing, R::IsBleed, R::HostBloodAtLeast(1)],
                &[Effect::HostBurnBlood(1), bleed(1, false)],
            )
        },
    ],
    ..VAMP
};

/// "Sabbat: During your unlock phase, you can move 1 blood from Kassiym to
/// any ready Pander." Any Methuselah's.
pub static KASSIYM_MALIKHAIR: VampireDef = VampireDef {
    name: "Kassiym Malikhair",
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[R::TargetVampire, R::ActorReady, R::TargetClan("Pander")],
        ),
        ..mode_of(
            &[Window::UnlockPhaseOwn],
            Actor::Host,
            &[R::HostBloodAtLeast(1)],
            &[Effect::HostBloodToTarget(1)],
        )
    }],
    ..VAMP
};

/// "Sabbat Archbishop of Strasbourg: Cards requiring Mytherceria [myt] cost
/// Marconius 1 less blood. +1 strength." The title generated.
pub static MARCONIUS: VampireDef = VampireDef {
    name: "Marconius",
    printed: &[
        own(Stat::BloodCostOff, 1, &[R::PlayedRequiresDiscipline("myt")]),
        own(Stat::Strength, 1, &[]),
    ],
    ..VAMP
};

/// "Sabbat priscus: Cards that require Mytherceria [myt] cost Omme 1 less
/// blood."
pub static OMME_ENBERBENIGHT: VampireDef = VampireDef {
    name: "Omme Enberbenight",
    printed: &[own(
        Stat::BloodCostOff,
        1,
        &[R::PlayedRequiresDiscipline("myt")],
    )],
    ..VAMP
};

/// "Sabbat bishop: During your discard phase, you may burn a pool to unlock
/// Pherydima." In torpor too (Dónal O'Connor's, LSJ 20011022); once in the
/// phase (a phase window).
pub static PHERYDIMA: VampireDef = VampireDef {
    name: "Pherydima",
    abilities: &[mode_of(
        &[Window::DiscardPhase],
        Actor::Host,
        &[R::HostLocked, R::PoolAtLeast(1)],
        &[Effect::PayPool(1), Effect::UnlockHost],
    )],
    ..VAMP
};

/// "Sabbat: While Roderick is ready, your minions get an additional +1
/// stealth on equip actions." Himself too; an equip action as rules-qa
/// 78.28 and 80.44 read it (not a move from another minion).
pub static RODERICK_PHILLIPS_MARCH: VampireDef = VampireDef {
    name: "Roderick Phillips March",
    printed: &[
        own(Stat::Stealth, 1, &[R::ThisVampireReady, R::IsEquipAction]),
        Printed {
            stat: Stat::Stealth,
            n: 1,
            to: PrintedTo::OthersControlled,
            per: Per::One,
            requires: &[R::ThisVampireReady, R::IsEquipAction],
        },
    ],
    ..VAMP
};

/// "Sabbat: Quincy may burn an ally controlled by your predator or prey
/// with two +1 stealth Ⓓ actions." The same action twice, in two different
/// turns (ANK 20170202): the first successful one at an ally recorded, the
/// second burns it; kept while both stay in play.
pub static QUINCY_THE_TRAPPER: VampireDef = VampireDef {
    name: "Quincy, The Trapper",
    abilities: &[printed_action(
        target(
            TargetKind::Minion,
            &[R::TargetAlly, R::ControlledByPredatorOrPrey],
        ),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::BurnTargetOnSecondAction,
        ],
    )],
    ..VAMP
};

// ---- Round 22: Anthology and Anarch Unbound -------------------------------
//
// Cao Nguyen ("Sabbat."), Desiree Narayan, Irena ("Anarch."), Dhita
// Choudhair, Domenic Giovanni, Dr. Lawrence Mayhew, Shahara al-Rashwa
// ("Independent.") and Flavia ("Independent: Blood cursed.", a crypt
// trait) print no other text. Dancin' Dana, Louis Fortier, Sundown and Zack
// North (ADV, "Anarch.") and Sarrasine (ADV, "Camarilla Prince of
// Sydney.") print only [MERGED] text: their base cards are not in the
// pool, so no merge, the [MERGED] text never applies (nor Boss Callihan's,
// Gengis's, Javier's or Marguerite's below).

/// "Anarch: Callihan gets -1 intercept against Sabbat vampires. [MERGED]
/// Baron of New York." The acting minion's sect read live; an ally has none.
pub static BOSS_CALLIHAN: VampireDef = VampireDef {
    name: "Boss Callihan",
    adv: true,
    printed: &[own(
        Stat::Intercept,
        -1,
        &[R::OpponentSectIn(&[Sect::Sabbat])],
    )],
    ..VAMP
};

/// "Anarch: After Daliah enters combat, she can gain 1 blood." Once each
/// combat she is in, as it begins; at capacity a legal no-op (the excess
/// drains).
pub static DALIAH: VampireDef = VampireDef {
    name: "Daliah",
    abilities: &[ability(
        &[Window::CombatBeforeRange],
        Actor::Host,
        &[R::FirstRound],
        Limit::PerCombat,
        &[Effect::AddHostBlood(1)],
    )],
    ..VAMP
};

/// "Anarch: If Gengis is blocked by a titled vampire, he does not unlock as
/// normal during your next unlock phase. [MERGED] Gengis gets +1 level of
/// Auspex [aus] and Celerity [cel]." The blocker's title read as it blocks;
/// an unlock effect still unlocks him.
pub static GENGIS: VampireDef = VampireDef {
    name: "Gengis",
    adv: true,
    riders: &[Rider {
        hook: Hook::HostBlocked,
        scope: RiderScope::Every,
        requires: &[R::BlockerTitleIn(&[TitleClass::Any])],
        effects: &[Effect::SkipNextUnlock],
    }],
    ..VAMP
};

/// "Anarch: During your minion phase, Javier can burn 1 blood to unlock a
/// younger ready Anarch. [MERGED] Baron of Barcelona." Any Methuselah's
/// Anarch, younger than Javier, not the acting minion; once in the phase,
/// in torpor or locked too (Dónal O'Connor's template); an unlocked one a
/// legal no-op.
pub static JAVIER_MONTOYA: VampireDef = VampireDef {
    name: "Javier Montoya",
    adv: true,
    abilities: &[ModeDef {
        target: target(
            TargetKind::Minion,
            &[
                R::TargetVampire,
                R::ActorReady,
                R::TargetAnarch,
                R::YoungerThanHost,
            ],
        ),
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[R::OwnMinionPhase, R::HostBloodAtLeast(1)],
            &[Effect::HostBurnBlood(1), Effect::UnlockTarget],
        )
    }],
    ..VAMP
};

/// "Anarch: Cards that cost blood cost Jenna 1 additional blood to play.
/// Sterile." Once every reduction is taken off, if blood is left to pay
/// (ANK 20200709: her player orders a reduction first); sterile is the
/// crypt card's trait.
pub static JENNA_CROSS: VampireDef = VampireDef {
    name: "Jenna Cross",
    printed: &[own(Stat::BloodCostMoreIfAny, 1, &[])],
    ..VAMP
};

/// "Anarch: Marguerite gets +1 strength in combat with a titled vampire.
/// [MERGED] +1 strength." Roy's shape, the title read live.
pub static MARGUERITE_FOCCART: VampireDef = VampireDef {
    name: "Marguerite Foccart",
    adv: true,
    printed: &[own(Stat::Strength, 1, &[R::OpponentTitled])],
    ..VAMP
};

/// "Anarch: You and Philip can play cards requiring a Ventrue as if he
/// were a Ventrue." Lady Scarlett Churchill's shape: his cards and his
/// Methuselah's, played normally; the card's effects in play do not take
/// him for one (LSJ 20050119, LSJ 20101013).
pub static PHILIP_VAN_VERMEER_IV: VampireDef = VampireDef {
    name: "Philip van Vermeer IV",
    plays_as: Some(PlaysAs {
        clan: "Ventrue",
        methuselah: true,
    }),
    ..VAMP
};

/// "Anarch: While Salvador is ready, cards requiring an Anarch cost you and
/// Anarchs you control 1 fewer blood or pool." A baron's card included (LSJ
/// 20080603: `Label::RequiresAnarch`); an Anarch's card off its blood or
/// pool, a master card's or a non-Anarch's off the pool ("you"); locked
/// too, never in torpor.
pub static SALVADOR_GARCIA: VampireDef = VampireDef {
    name: "Salvador Garcia",
    printed: &[
        own(Stat::CostOff, 1, SALVADOR_ANARCH_CARD),
        Printed {
            stat: Stat::CostOff,
            n: 1,
            to: PrintedTo::OthersControlled,
            per: Per::One,
            requires: SALVADOR_ANARCH_CARD,
        },
        // "Cost you": the pool of a card a non-Anarch of his Methuselah's
        // plays (Vlad Tepes meeting the requirement; rules-qa 105.3).
        Printed {
            stat: Stat::PoolCostOff,
            n: 1,
            to: PrintedTo::OthersControlled,
            per: Per::One,
            requires: &[
                R::ThisVampireReady,
                R::ActorSectIn(&[
                    Sect::Camarilla,
                    Sect::Sabbat,
                    Sect::Independent,
                    Sect::Laibon,
                ]),
                R::PlayedCard(CardFilter::HasLabel(Label::RequiresAnarch)),
            ],
        },
        own(
            Stat::SeatCostOff,
            1,
            &[
                R::ThisVampireReady,
                R::PlayedCard(CardFilter::HasLabel(Label::RequiresAnarch)),
            ],
        ),
    ],
    ..VAMP
};

const SALVADOR_ANARCH_CARD: &[R] = &[
    R::ThisVampireReady,
    R::ActorSectIn(&[Sect::Anarch]),
    R::PlayedCard(CardFilter::HasLabel(Label::RequiresAnarch)),
];

/// "Anarch: You and Victor can play cards requiring a Camarilla vampire as
/// if Victor were a Camarilla vampire. While Victor is a Baron, you and
/// Victor can play cards requiring a Prince as if Victor were a Prince."
/// The card's own requirements, played normally (LSJ 20050119); its effects
/// in play do not take him for one (LSJ 20101013); no prince's votes (ANK
/// 20231221).
pub static VICTOR_GERARD: VampireDef = VampireDef {
    name: "Victor Gerard",
    printed: &[
        own(Stat::PlaysAsCamarilla, 1, &[]),
        own(
            Stat::PlaysAsPrince,
            1,
            &[R::ActorTitleIn(&[TitleClass::Baron])],
        ),
    ],
    ..VAMP
};

/// "Anarch: Once each turn, an Anarch you control can unlock after
/// successfully equipping with a vehicle from your hand." Synner-G's shape
/// for any of his Methuselah's Anarchs, Vulture locked or in torpor too;
/// once in each Methuselah's turn (ANK 20180719-4).
pub static VULTURE: VampireDef = VampireDef {
    name: "Vulture",
    abilities: &[ModeDef {
        limit: Some(Limit::PerTurn),
        ..mode_of(
            &[Window::ActionAfterResolution],
            Actor::Host,
            &[
                R::OwnActing,
                R::ActingSectIn(&[Sect::Anarch]),
                R::ActionNotFizzled,
                R::IsEquipLabeledFromHand(Label::Vehicle),
            ],
            &[Effect::UnlockActing],
        )
    }],
    ..VAMP
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypt::CRYPT;

    // Every imbued may leave the incapacitated region (rulebook "Appendix:
    // Imbued Rules" 5), Jennie's way too: both abilities on each.
    #[test]
    fn every_imbued_carries_the_recovery_abilities() {
        for (i, c) in CRYPT.iter().enumerate().filter(|(_, c)| c.imbued) {
            let def = crate::cards::vampire(i as u16).expect(c.name);
            for r in [RECOVER, RECOVER_GRANTED] {
                assert!(def.abilities.contains(&r), "{}", c.name);
            }
        }
    }
}
