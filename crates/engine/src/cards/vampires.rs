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
    PrintedTo, RefEffect, ReferendumDef, Requirement as R, Rider, RiderScope, Stat, StrikeKind,
    TargetKind, TargetSpec, Term, Terms, TitleClass, VampireDef, Window, VAMP,
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
/// usual; an ally giving its last life burns (rules-qa 41.8). Mages:
/// Nephandus, Veneficti (slice 16n).
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

pub static VAMPIRES: [&VampireDef; 289] = [
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
/// Sabbat (the title's rule). An owner-added legacy card (2026-10-11).
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
/// Leon ("Camarilla.") print no text. Owner-added legacy cards
/// (2026-10-11).
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
/// he can gain 1 blood." Once, after that bleed (owner, 2026-10-09, from
/// play: offered again after each use).
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

// ---- Sabbat V5: Path of Caine (slice 16h) --------------------------------
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

// ---- Sabbat V5: Path of Death and the Soul (slice 16i) ---------------------
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
            Effect::ReorderTopThenUnlock(5),
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

// ---- TWDA unlocks: staples and steps 1-3 (slice 16j) --------------------------

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

// ---- TWDA unlocks: steps 4-9 (slice 16k) -------------------------------------
//
// Benedito, a Lanterna, Jacques Rouge and Tara print no text but their
// title.

/// "Sabbat. Black Hand. Seraph: Elimelech strikes with first strike. +1
/// bleed. +1 strength." Black Hand and Seraph are inert (no V5 card reads
/// them).
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
/// Anarch you control." Once during the controller's turn (owner,
/// 2026-10-09: rules-qa 58.7, the "during" template 6.6); any Anarch of
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
        points: 4,
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

// ---- TWDA unlocks: steps 10-15 (slice 16l) -----------------------------------
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
        &[Effect::PreventStrike(1)],
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

// ---- TWDA unlocks: steps 16-20 (slice 16m) -----------------------------------

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
/// or an event (owner, 2026-10-09: rules-qa 60.3).
pub static MISTRESS_FANCHON: VampireDef = VampireDef {
    name: "Mistress Fanchon",
    printed: &[own(Stat::Bleed, 2, &[])],
    abilities: &[printed_action(
        None,
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::SearchToHand(CardFilter::MinionCard),
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
/// too (LSJ 20011022); any Methuselah's Tremere; as often as wished.
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

// ---- TWDA unlocks, round 2: steps 1-6 (slice 16n) ----------------------------

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
        requires: &[],
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
/// Visceratika card is in the pool; Flight is a bare trait (rules-qa
/// 16.6).
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

// ---- Owner's list, round 3 (slice 16o) -------------------------------------

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

// ---- Owner's list, round 4 (slice 16p) ---------------------------------------
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

// ---- Round 7: the next hundred by TWDA importance (slice 16s) ------------

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
            &[R::TargetSeatControlsReadyClan(&["Tremere"])],
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
/// action's cost (as Daria's, rules-qa 42.1).
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
                Effect::SearchToHand(CardFilter::MasterLocation),
            ],
        )
    }],
    ..VAMP
};

/// "Sabbat: Allies and younger vampires get -1 intercept against Gavrylo.
/// Kiev circle. Sterile."
pub static GAVRYLO: VampireDef = VampireDef {
    name: "Gavrylo",
    printed: &[own(Stat::YoungerInterceptAgainst, -1, &[])],
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
            &[R::TargetVampire, R::ActorReady, R::TargetSameCircle],
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
        &[Effect::CircleBloodMoves],
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
    printed: &[own(Stat::Strength, 1, &[R::ReadyNamed("Zizi")])],
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
            &[R::TargetVampire, R::TargetLocked, R::TargetSameCircle],
        ),
        ..mode_of(
            &[Window::AnyImpulse],
            Actor::Host,
            &[R::HostReady, R::HostUnlocked, R::ReadyNamed("Viri")],
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

// ---- Shadow Play, The Embrace, the Appolonius merge (slice 16u) -------------
//
// Ignacio, Mustafa Rahman, Ramiro, Carter, Lolita, Mercy, Nicholas Chang,
// Remilliard, Ashlesha, Salbatore Bokkengro, Sasha Miklos, Sennuwy, Vedel
// Esbreno, Zip, Andrei Puxon and Ghivran Dalaal print no text.

/// "Sabbat Archbishop of Pittsburgh: After resolution of any political
/// action, if Alvaro abstained (even in torpor), he can gain 2 blood. +1
/// bleed." No vote or ballot standing, title or not (LSJ 20010526); read
/// after the referendum (`Engine::resolve_political`).
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
/// is locked then (LSJ 20100527); any number of times in the action (LSJ
/// 20100604-2, LSJ 20100611); in torpor too (LSJ 20010813-2).
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
            &[Effect::LockHost, Effect::TargetBurnsForInterceptThisAction],
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
/// rulebook hunt's +1: a granted hunt's own stealth stays (Week of
/// Nightmares).
pub static ZOE: VampireDef = VampireDef {
    name: "Zöe",
    printed: &[own(Stat::Stealth, -1, &[R::IsHunt])],
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
        requires: &[R::OwnUncontrolledNotEmpty, R::PreyUncontrolledHasBlood],
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
pub static MERGED: [&VampireDef; 1] = [&APPOLONIUS_MERGED];

// ---- Round 10: the 2025-2026 TWDA decks (slice 16v) --------------------------
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
