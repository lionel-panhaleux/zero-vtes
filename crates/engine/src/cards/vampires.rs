//! Printed text of vampires, keyed by printed name like the library
//! `defs`. The crypt data (`crate::crypt`) stays generated; what the text
//! does is written here: plain modifiers read live by
//! `GameState::printed_mod`, and abilities in the card vocabulary (the
//! actor `Host` is the vampire itself). A vampire may land with part of
//! its text — every vampire ends with all of it (rules-qa 13.1) — the rest
//! quoted in `incomplete`. Slice 9a: plain stat modifiers; 9b: combat
//! abilities; 9c: abilities in referendums, actions and phases; 12a: the
//! pool's last vampires (cost reductions, clans played as, tolls, unlock
//! riders); 12b: the partial ones finished (action and strike-card
//! surcharges, an ash heap cost).

use super::{
    bleed, mode_of, target, Actor, CardFilter, Effect, Hook, Limit, ModeDef, Per, PlaysAs, Printed,
    PrintedTo, Requirement as R, Rider, RiderScope, Stat, StrikeKind, TargetKind, TargetSpec,
    Terms, TitleClass, VampireDef, Window, VAMP,
};
use crate::crypt::Sect;
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
/// directed at the equipment's controller — undirected from a minion of
/// the vampire's own Methuselah.
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
/// on his block attempt, each only when needed (rules-qa 3.2), as often as
/// he can discard (advanced rules §1.5.4).
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
/// unlock phase." A successful hunt gains blood (spec §3.7.2.1).
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
    printed: &[own(Stat::DirectedActionCost, 1, &[R::ActionDirected])],
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
/// (rules-qa 41.4, 42.2; ANK 20180928-1). Read live
/// (`Engine::combat_card_surcharge`).
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
            Effect::SearchToHand(CardFilter::Labeled(CardType::Master, Label::Archetype)),
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
/// as often as she can discard.
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
/// strength. +2 votes." Tolls on the attempt (`Engine::block_toll`) and on
/// votes against his political action's referendum
/// (`Engine::vote_against_toll`), read live: a vampire of lower capacity —
/// never an ally, never himself; a Methuselah's ballots are not a vampire's
/// and a blood hunt called on him is not called by him (rules-qa 41.3,
/// 42.4). In torpor too — his leave-torpor action (LSJ 20011022).
pub static MITHRAS: VampireDef = VampireDef {
    name: "Mithras",
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
/// opposing minion's strike." Every damage pending at the prevention step
/// comes from a strike in the engine today; damage from other sources must
/// stay out of this prevention when it lands.
pub static OPIKUN: VampireDef = VampireDef {
    name: "Opikun",
    abilities: &[ability(
        &[Window::CombatDamagePrevention],
        Actor::ForOwnCombatant,
        &[R::HostBloodAtLeast(1), R::NonAggravatedDamagePending],
        Limit::PerCombat,
        &[Effect::HostBurnBlood(1), Effect::PreventNonAggravated(2)],
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
            Effect::SearchToHand(CardFilter::Type(CardType::Equipment)),
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
/// life to attempt to block Sergio": a toll on the attempt
/// (`Engine::block_toll`), in torpor too — his leave-torpor action (LSJ
/// 20011022).
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
/// usual; an ally giving its last life burns (rules-qa 41.8). No mage ally
/// is in the engine pool: the generator labels none.
pub static SRI_SANSA: VampireDef = VampireDef {
    name: "Sri Sansa",
    abilities: &[
        ModeDef {
            target: target(
                TargetKind::Minion,
                &[
                    R::Controlled,
                    R::TargetAllyCard(CardFilter::HasLabel(Label::Mortal)),
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
                    R::TargetAllyCard(CardFilter::HasLabel(Label::Mortal)),
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
/// baron of the same city contesting it.
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
                &[R::TargetVampire, R::TargetTitleIn(&[TitleClass::Prince])],
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
/// the attempt (`Engine::block_toll`): none possible from an empty library;
/// a forced attempt pays it once its card is replaced (ANK 20260317).
pub static PARIJAT: VampireDef = VampireDef {
    name: "Parijat, the Dark Oracle",
    printed: &[own(Stat::WraithZombieBlockToll, 1, &[])],
    ..VAMP
};

pub static VAMPIRES: [&VampireDef; 126] = [
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
/// called by him passes." (Sybren van Oosten's text.) The advanced card: it contests the base Xaviar as the same vampire; the
/// merge (ANK 20180524: Camarilla, an anarch token burned) waits for the
/// base card in the pool.
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
    abilities: &[mode_of(
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
    )],
    ..VAMP
};

/// "Camarilla Prince of Paris: If François is ready during your unlock
/// phase, he can burn 2 blood to lock a younger vampire. +1 bleed." Once
/// each unlock phase (rules-qa 6.6).
pub static FRANCOIS_VILLON: VampireDef = VampireDef {
    name: "François Villon",
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

// ---- Sabbat V5: Path of Cathari (slice 16g) ------------------------------
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
/// he can gain 1 blood."
pub static GOSTOSO: VampireDef = VampireDef {
    name: "Gostoso",
    abilities: &[mode_of(
        &[Window::ActionAfterResolution],
        Actor::Host,
        &[R::ActorActing, R::BleedAgainstPrey, R::BleedSuccessful],
        &[Effect::AddHostBlood(1)],
    )],
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
