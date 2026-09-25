//! Printed text of vampires, keyed by printed name like the library
//! `defs`. The crypt data (`crate::crypt`) stays generated; what the text
//! does is written here: plain modifiers read live by
//! `GameState::printed_mod`, and abilities in the card vocabulary (the
//! actor `Host` is the vampire itself). A vampire may land with part of
//! its text — every vampire ends with all of it (rules-qa 13.1) — the rest
//! quoted in `incomplete`. Slice 9a: plain stat modifiers; 9b: combat
//! abilities; 9c: abilities in referendums, actions and phases.

use super::{
    Actor, CardFilter, Effect, Hook, Limit, ModeDef, Per, Printed, PrintedTo, Requirement as R,
    Rider, RiderScope, Stat, StrikeKind, TargetKind, TargetSpec, Terms, TitleClass, VampireDef,
    Window,
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
const fn ability(
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
const fn printed_action(target: Option<TargetSpec>, effects: &'static [Effect]) -> ModeDef {
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
const fn enter_combat(filter: &'static [R]) -> ModeDef {
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
const MANEUVER_EACH_COMBAT: ModeDef = ability(
    &[Window::CombatDetermineRange],
    Actor::Host,
    &[],
    Limit::PerCombat,
    &[Effect::Maneuver],
);

/// "Once each combat, … can strike: dodge" (Flávio Gonçalves).
const DODGE_EACH_COMBAT: ModeDef = ability(
    &[Window::CombatStrikeDeclaration],
    Actor::Host,
    &[],
    Limit::PerCombat,
    &[Effect::Strike(StrikeKind::Dodge)],
);

pub static ABADDON: VampireDef = VampireDef {
    name: "Abaddon",
    printed: &[],
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
    riders: &[],
    incomplete: None,
};

/// "During an action, Abraham can discard a card requiring Blood Sorcery
/// [tha] to get +1 intercept or +1 stealth": stealth on his action, intercept
/// on his block attempt, each only when needed (rules-qa 3.2), as often as
/// he can discard (advanced rules §1.5.4).
pub static ABRAHAM_DUSABLE: VampireDef = VampireDef {
    name: "Abraham DuSable",
    printed: &[],
    abilities: &[
        ModeDef {
            windows: &[Window::ActionDuring],
            actor: Actor::Host,
            requires: &[
                R::ActorActing,
                R::StealthNeeded,
                R::HandHas(CardFilter::Discipline("tha"), 1),
            ],
            target: None,
            terms: Some(Terms::Discard {
                n: 1,
                of: CardFilter::Discipline("tha"),
            }),
            waives_needed: false,
            limit: Some(Limit::PerAction),
            effects: &[Effect::AddStealth(1)],
        },
        ModeDef {
            windows: &[Window::ActionStealthIntercept],
            actor: Actor::Host,
            requires: &[
                R::InterceptNeeded,
                R::HandHas(CardFilter::Discipline("tha"), 1),
            ],
            target: None,
            terms: Some(Terms::Discard {
                n: 1,
                of: CardFilter::Discipline("tha"),
            }),
            waives_needed: false,
            limit: Some(Limit::PerAction),
            effects: &[Effect::AddIntercept(1)],
        },
    ],
    riders: &[],
    incomplete: None,
};

/// "Adrino gets 1 press (mandatory) each combat, only usable to continue
/// combat": he must use it before passing the press step, unless a press to
/// continue already stands — it then waits for a later round (rules-qa
/// 14.1).
/// "Adisa can enter combat with an ally or younger vampire as a Ⓓ action. If
/// she successfully hunts, she does not unlock as normal during her next
/// unlock phase." A successful hunt gains blood (spec §3.7.2.1).
pub static ADISA: VampireDef = VampireDef {
    name: "Adisa",
    printed: &[],
    abilities: &[enter_combat(&[
        R::NotControlled,
        R::ActorReady,
        R::TargetVampire,
        R::Younger,
    ])],
    riders: &[Rider {
        hook: Hook::SuccessfulHunt,
        scope: RiderScope::Controller,
        requires: &[],
        effects: &[Effect::SkipNextUnlock],
    }],
    incomplete: Some("Adisa can enter combat with an ally … (waits for allies)."),
};

pub static ADRINO_MANAUARA: VampireDef = VampireDef {
    name: "Adrino Manauara",
    printed: &[own(Stat::MandatoryContinuePress, 1, &[])],
    abilities: &[],
    riders: &[],
    incomplete: None,
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
    riders: &[],
    incomplete: None,
};

/// "If Alabástrom is ready during your turn, you can unlock another ready
/// Gargoyle you control": once that turn (rules-qa 15.2). Flight is a bare
/// trait — no card in the engine pool requires it (rules-qa 16.6).
pub static ALABASTROM: VampireDef = VampireDef {
    name: "Alabástrom",
    printed: &[],
    abilities: &[ModeDef {
        windows: &[Window::AnyImpulse],
        actor: Actor::Host,
        requires: &[R::OwnTurn, R::HostReady],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[
                R::Controlled,
                R::ActorReady,
                R::ActorClan("Gargoyle"),
                R::NotHost,
            ],
        }),
        terms: None,
        waives_needed: false,
        limit: Some(Limit::PerTurn),
        effects: &[Effect::UnlockTarget],
    }],
    riders: &[],
    incomplete: None,
};

pub static ALEXA_DRAPER: VampireDef = VampireDef {
    name: "Alexa Draper",
    printed: &[],
    abilities: &[ModeDef {
        windows: &[Window::Referendum],
        actor: Actor::Host,
        requires: &[
            R::AnyPollingStep,
            R::HandHas(CardFilter::Discipline("dom"), 1),
        ],
        target: None,
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Discipline("dom"),
        }),
        waives_needed: false,
        limit: Some(Limit::PerReferendum),
        effects: &[Effect::AddVotes(1)],
    }],
    riders: &[],
    incomplete: None,
};

/// "After Alicia performs a successful directed action, you can discard a
/// political action card to lock a younger vampire controlled by (one of)
/// the target Methuselah(s)." Once per action (rules-qa 16.1).
pub static ALICIA_CORTEZ: VampireDef = VampireDef {
    name: "Alicia Cortez",
    printed: &[],
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
    riders: &[],
    incomplete: None,
};

/// "Once each turn, Aline can burn 1 blood to unlock after another Anarch
/// you control performs a successful action." In torpor too (LSJ 20011022).
pub static ALINE_GADEKE: VampireDef = VampireDef {
    name: "Aline Gädeke",
    printed: &[],
    abilities: &[ModeDef {
        windows: &[Window::ActionAfterResolution],
        actor: Actor::Host,
        requires: &[
            R::OtherActingControlled,
            R::ActingSectIn(&[Sect::Anarch]),
            R::ActionSuccessful,
            R::HostLocked,
            R::HostBloodAtLeast(1),
        ],
        target: None,
        terms: None,
        waives_needed: false,
        limit: Some(Limit::PerTurn),
        effects: &[Effect::HostBurnBlood(1), Effect::UnlockActor],
    }],
    riders: &[],
    incomplete: None,
};

pub static ANDRE_THE_MANIPULATOR: VampireDef = VampireDef {
    name: "André the Manipulator",
    printed: &[own(Stat::HandSize, 2, &[R::ThisVampireReady])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

pub static ARIANE: VampireDef = VampireDef {
    name: "Ariane",
    printed: &[own(Stat::Stealth, -1, &[R::IsUndirected])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

/// "If Ayse is ready, you can use a discard phase action to add 1 blood to
/// her": each use spends one, not once per phase; not offered at capacity
/// (as the blood shuttles of cards in play).
/// "Aniel can burn 1 corruption counter or a card requiring a Discipline
/// from another ready minion as a +1 stealth Ⓓ action that costs 1
/// blood." The card half: directed at the card's controller, undirected
/// on a minion of Aniel's own Methuselah (rulebook "Directed and Undirected
/// Actions": Ⓓ is a reminder).
pub static ANIEL: VampireDef = VampireDef {
    name: "Aniel",
    printed: &[],
    abilities: &[printed_action(
        Some(TargetSpec {
            kind: TargetKind::Card,
            filter: &[
                R::TargetCard(CardFilter::RequiresDiscipline),
                R::TargetOnReadyMinion,
                R::NotHost,
            ],
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::PayBlood(1),
            Effect::BurnTargetCard,
        ],
    )],
    riders: &[],
    incomplete: Some("Aniel can burn 1 corruption counter … (waits for corruption counters)."),
};

/// "Apolonia can steal 1 blood from a younger vampire as a Ⓓ action." Any
/// younger vampire, one in torpor or with no blood included (rulebook
/// "Target"); undirected on her own Methuselah's.
pub static APOLONIA_CZARNECKI: VampireDef = VampireDef {
    name: "Apolonia Czarnecki",
    printed: &[],
    abilities: &[printed_action(
        Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[R::TargetVampire, R::Younger],
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::StealBloodFromTarget(1),
        ],
    )],
    riders: &[],
    incomplete: None,
};

pub static AYSE_DHANIAL: VampireDef = VampireDef {
    name: "Ayse Dhanial",
    printed: &[],
    abilities: &[ModeDef {
        windows: &[Window::DiscardPhaseAction],
        actor: Actor::Host,
        requires: &[
            R::HostReady,
            R::DiscardPhaseActionLeft,
            R::HostBelowCapacity,
        ],
        target: None,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::SpendDiscardPhaseAction, Effect::AddHostBlood(1)],
    }],
    riders: &[],
    incomplete: None,
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
    abilities: &[],
    riders: &[],
    incomplete: None,
};

/// "Barachiel can enter combat with a minion as a Ⓓ action."
pub static BARACHIEL: VampireDef = VampireDef {
    name: "Barachiel",
    printed: &[],
    abilities: &[enter_combat(&[R::NotControlled, R::ActorReady])],
    riders: &[],
    incomplete: None,
};

pub static BRANIMIRA: VampireDef = VampireDef {
    name: "Branimira",
    printed: &[own(Stat::Intercept, 1, &[R::PoolAtMost(9)])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

pub static BRET_STRYKER: VampireDef = VampireDef {
    name: "Bret Stryker",
    printed: &[own(Stat::Intercept, -1, &[R::OpponentTitled])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

pub static CARMELITA_NEILLSON: VampireDef = VampireDef {
    name: "Carmelita Neillson",
    printed: &[own(Stat::HandSize, 1, &[R::ThisVampireReady])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

pub static CASTELLAN: VampireDef = VampireDef {
    name: "Castellan",
    printed: &[
        own(Stat::Intercept, 1, &[R::IsDirected]),
        own(Stat::Intercept, -1, &[R::IsUndirected]),
    ],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

pub static CATALINA_VEGA: VampireDef = VampireDef {
    name: "Catalina Vega",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

/// "If the referendum of a political action called by Cedrick is canceled
/// or fails, he goes to torpor after resolution."
pub static CEDRICK_CALHOUN: VampireDef = VampireDef {
    name: "Cedrick Calhoun",
    printed: &[],
    abilities: &[],
    riders: &[Rider {
        hook: Hook::OwnReferendumFailed,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::GoToTorpor],
    }],
    incomplete: None,
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
    riders: &[],
    incomplete: None,
};

/// "Daria can lock to give another Nosferatu +1 stealth": the acting
/// minion's, when needed (rules-qa 3.2), any Methuselah's; the lock cost
/// makes it once per unlock (advanced rules §1.5.4), in torpor too (LSJ
/// 20011022).
/// "Dafina can enter combat with a minion as a Ⓓ action."
pub static DAFINA_HANGANU: VampireDef = VampireDef {
    name: "Dafina Hanganu",
    printed: &[],
    abilities: &[enter_combat(&[R::NotControlled, R::ActorReady])],
    riders: &[],
    incomplete: None,
};

pub static DARIA: VampireDef = VampireDef {
    name: "Daria",
    printed: &[],
    abilities: &[ModeDef {
        windows: &[Window::ActionDuring, Window::ReactionDuringAction],
        actor: Actor::Host,
        requires: &[
            R::HostUnlocked,
            R::OtherActingClan("Nosferatu"),
            R::StealthNeeded,
        ],
        target: None,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::LockActor, Effect::AddStealth(1)],
    }],
    riders: &[],
    incomplete: Some("Directed actions cost her +1 blood."),
};

/// "Darius can steal a non-electronic, non-vehicle, non-weapon equipment
/// as a Ⓓ action." No electronic equipment is in the V5 pool.
pub static DARIUS_SHIRAZI: VampireDef = VampireDef {
    name: "Darius Shirazi",
    printed: &[],
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
    riders: &[],
    incomplete: None,
};

pub static DJENEBA: VampireDef = VampireDef {
    name: "Djeneba",
    printed: &[own(Stat::Stealth, 1, &[])],
    abilities: &[],
    riders: &[],
    incomplete: Some("In combat, strike cards cost the opposing minion +1 blood or life."),
};

/// "Dominica can search your library for a master archetype card, reveal it
/// and move it to your hand as a +1 stealth action."
pub static DOMINICA: VampireDef = VampireDef {
    name: "Dominica",
    printed: &[],
    abilities: &[printed_action(
        None,
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::SearchToHand(CardFilter::Labeled(CardType::Master, Label::Archetype)),
        ],
    )],
    riders: &[],
    incomplete: None,
};

pub static DOWAGER_THE: VampireDef = VampireDef {
    name: "Dowager, The",
    printed: &[own(Stat::Intercept, 1, &[R::IsDirected])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

pub static EGIDIA_ARRU: VampireDef = VampireDef {
    name: "Egidia Arrú",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[],
    // "After a minion in combat with Egidia leaves the ready region, their
    // controller burns 1 pool."
    riders: &[Rider {
        hook: Hook::CombatOpponentLeftReady,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::TheirControllerBurnsPool(1)],
    }],
    incomplete: None,
};

/// "As a minion announces an action directed at Evan, flip a coin; if it is
/// tails, the action fails." An action of his controller's own minions is
/// undirected, so it is not flipped (LSJ 20030214-2).
/// "If you control a locked minion, Elen must bleed with +1 bleed as a Ⓓ
/// action unless she must hunt." Her own action, mandatory while the
/// condition holds; a bleed of another source does not discharge it, and
/// having bled before it attached she is stuck (spec §3.9, ANK 20211009-2).
pub static ELEN_KAMJIAN: VampireDef = VampireDef {
    name: "Elen Kamjian",
    printed: &[],
    abilities: &[ModeDef {
        windows: &[Window::ActionDeclare],
        actor: Actor::Host,
        requires: &[R::ControlsLockedMinion],
        target: None,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[
            Effect::DeclareAction(ActionKind::Bleed),
            Effect::MandatoryAction,
            Effect::BleedMod {
                n: 1,
                limited: false,
            },
        ],
    }],
    riders: &[],
    incomplete: None,
};

pub static EVAN_KLEIN: VampireDef = VampireDef {
    name: "Evan Klein",
    printed: &[],
    abilities: &[],
    riders: &[Rider {
        hook: Hook::ActionAnnouncedAtTarget,
        scope: RiderScope::Every,
        requires: &[R::ActingOtherController],
        effects: &[Effect::CoinFlipActionFails],
    }],
    incomplete: None,
};

pub static FARUQ_ABD_AL_QADIR: VampireDef = VampireDef {
    name: "Faruq Abd al-Qadir",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[],
    riders: &[],
    incomplete: Some(
        "Minions with any of your corruption counters in combat with Faruq cannot strike: \
         combat ends.",
    ),
};

pub static FLAVIO_GONCALVES: VampireDef = VampireDef {
    name: "Flávio Gonçalves",
    printed: &[],
    abilities: &[DODGE_EACH_COMBAT],
    riders: &[],
    incomplete: None,
};

/// "During an action Gathii performs, you can reveal the top card of your
/// library. If it is a master card, he burns 1 blood; otherwise, he gets +1
/// stealth": once per action (rules-qa 15.1), usable before stealth is
/// needed, the stealth then not gained (ANK 20231031).
pub static GATHII: VampireDef = VampireDef {
    name: "Gathii",
    printed: &[],
    abilities: &[ModeDef {
        windows: &[Window::ActionDuring],
        actor: Actor::Host,
        requires: &[R::ActorActing],
        target: None,
        terms: None,
        waives_needed: true,
        limit: Some(Limit::PerAction),
        effects: &[Effect::RevealTopCard {
            burn: 1,
            stealth: 1,
        }],
    }],
    riders: &[],
    incomplete: None,
};

pub static GWENLLIAN_ARWYN: VampireDef = VampireDef {
    name: "Gwenllian Arwyn",
    printed: &[own(Stat::Strength, 1, &[])],
    // The minion attempting to block her action, while stealth is needed,
    // for the rest of the action; once per action (rules-qa 15.2).
    abilities: &[ModeDef {
        windows: &[Window::ActionDuring],
        actor: Actor::Host,
        requires: &[R::ActorActing, R::StealthNeeded, R::HostBloodAtLeast(1)],
        target: None,
        terms: None,
        waives_needed: false,
        limit: Some(Limit::PerAction),
        effects: &[Effect::HostBurnBlood(1), Effect::AttemptingIntercept(-1)],
    }],
    riders: &[],
    incomplete: None,
};

pub static HAFTHOR_THORSTEINSSON: VampireDef = VampireDef {
    name: "Hafthor Thorsteinsson",
    printed: &[own(Stat::Intercept, 1, &[])],
    abilities: &[],
    riders: &[],
    incomplete: None,
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
    abilities: &[],
    riders: &[],
    incomplete: None,
};

/// "Once each turn, if Ilonka is ready after a successful bleed against you,
/// she can look at the acting minion's controller's hand, then she can
/// discard 1 card at random from it": the count, 0 or 1, is chosen after
/// the look.
pub static ILONKA: VampireDef = VampireDef {
    name: "Ilonka",
    printed: &[],
    abilities: &[ModeDef {
        windows: &[Window::ActionAfterResolution],
        actor: Actor::Host,
        requires: &[
            R::HostReady,
            R::IsBleed,
            R::DirectedAtYou,
            R::ActionSuccessful,
        ],
        target: None,
        terms: Some(Terms::Count { min: 0, max: 1 }),
        waives_needed: false,
        limit: Some(Limit::PerTurn),
        effects: &[Effect::ActingControllerDiscardsAtRandom],
    }],
    riders: &[],
    incomplete: None,
};

pub static JASON_SON_NEWBERRY: VampireDef = VampireDef {
    name: "Jason \"Son\" Newberry",
    printed: &[
        own(Stat::Bleed, 1, &[]),
        own(Stat::VotesAgainstBloodHunt, 2, &[]),
    ],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

/// "During your turn, you can burn the Edge to unlock Kalinda": wherever her
/// controller holds the impulse on their turn.
pub static KALINDA: VampireDef = VampireDef {
    name: "Kalinda",
    printed: &[],
    abilities: &[ModeDef {
        windows: &[Window::AnyImpulse],
        actor: Actor::Host,
        requires: &[R::OwnTurn, R::HasEdge, R::HostLocked],
        target: None,
        terms: None,
        waives_needed: false,
        limit: Some(Limit::PerTurn),
        effects: &[Effect::BurnEdge, Effect::UnlockActor],
    }],
    riders: &[],
    incomplete: None,
};

pub static KAMILE_PAUKSTYS: VampireDef = VampireDef {
    name: "Kamile Paukstys",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

/// "Once each combat, Kasim can discard a political action card before
/// range is determined to get +2 strength that combat. +1 bleed."
pub static KASIM_BAYAR: VampireDef = VampireDef {
    name: "Kasim Bayar",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[ModeDef {
        windows: &[Window::CombatBeforeRange],
        actor: Actor::Host,
        requires: &[R::HandHas(CardFilter::Type(CardType::PoliticalAction), 1)],
        target: None,
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Type(CardType::PoliticalAction),
        }),
        waives_needed: false,
        limit: Some(Limit::PerCombat),
        effects: &[Effect::StrengthThisCombat(2)],
    }],
    riders: &[],
    incomplete: None,
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
    abilities: &[],
    riders: &[],
    incomplete: None,
};

/// "During a bleed action, Larissa can discard a card requiring Animalism
/// [ani] to get +1 bleed": her own bleed, before resolution (ANK 20231124),
/// as often as she can discard.
pub static LARISSA_MOREIRA: VampireDef = VampireDef {
    name: "Larissa Moreira",
    printed: &[],
    abilities: &[ModeDef {
        windows: &[Window::ActionDuring],
        actor: Actor::Host,
        requires: &[
            R::ActorActing,
            R::IsBleed,
            R::HandHas(CardFilter::Discipline("ani"), 1),
        ],
        target: None,
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Discipline("ani"),
        }),
        waives_needed: false,
        limit: Some(Limit::PerAction),
        effects: &[Effect::BleedMod {
            n: 1,
            limited: false,
        }],
    }],
    riders: &[],
    incomplete: None,
};

/// "Lenelle can exchange a card from your hand for a library card in your
/// ash heap as a +1 stealth action that costs 1 blood." Both cards named as
/// the action is announced (LSJ 20000222, LSJ 20011023); exchanged at
/// resolution if both are still there.
pub static LENELLE: VampireDef = VampireDef {
    name: "Lenelle, Mambo of Birmingham",
    printed: &[],
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
    riders: &[],
    incomplete: None,
};

pub static LENNY_BURKHEAD: VampireDef = VampireDef {
    name: "Lenny Burkhead",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

pub static MARCHESA_LILIANA: VampireDef = VampireDef {
    name: "Marchesa Liliana",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    riders: &[],
    incomplete: Some(
        "During a bleed action, Liliana can remove seven cards in your ash heap from the game \
         to get an additional +1 bleed.",
    ),
};

/// "If Marialena is blocked, she burns 1 blood before block resolution."
pub static MARIALENA: VampireDef = VampireDef {
    name: "Marialena",
    printed: &[],
    abilities: &[],
    riders: &[Rider {
        hook: Hook::HostBlocked,
        scope: RiderScope::Every,
        requires: &[],
        effects: &[Effect::HostBurnBlood(1)],
    }],
    incomplete: None,
};

pub static MARTINA_SRNANKOVA: VampireDef = VampireDef {
    name: "Martina Srnankova",
    printed: &[own(Stat::Intercept, 1, &[R::IsBleed])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

pub static MASSIMILIANO: VampireDef = VampireDef {
    name: "Massimiliano",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

pub static MICKEY_WHEELER: VampireDef = VampireDef {
    name: "Mickey Wheeler",
    printed: &[own(Stat::Strength, 1, &[R::OpponentOlder])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

pub static MITHRAS: VampireDef = VampireDef {
    name: "Mithras",
    printed: &[
        own(Stat::Bleed, 2, &[]),
        own(Stat::Strength, 1, &[]),
        own(Stat::Votes, 2, &[]),
    ],
    abilities: &[],
    riders: &[],
    incomplete: Some(
        "Younger vampires must burn 1 blood to attempt to block Mithras or to cast votes or \
         ballots against a referendum called by him.",
    ),
};

pub static MORA_THE_DEATH_SEER: VampireDef = VampireDef {
    name: "Mora, the Death Seer",
    printed: &[own(Stat::Bleed, 1, &[])],
    // Once per discard phase (a phase-framed clause, advanced rules
    // §1.2.1).
    abilities: &[ModeDef {
        windows: &[Window::DiscardPhase],
        actor: Actor::Host,
        requires: &[R::HostReady, R::AshNotEmpty],
        target: None,
        terms: Some(Terms::AshCard),
        waives_needed: false,
        limit: None,
        effects: &[Effect::AshCardToLibraryBottom],
    }],
    riders: &[],
    incomplete: None,
};

/// "Nathaniel can enter combat with a locked vampire as a Ⓓ action." A
/// target unlocked by resolution: successful, no effect (ANK 20210124).
pub static NATHANIEL_BORDRUFF: VampireDef = VampireDef {
    name: "Nathaniel Bordruff",
    printed: &[],
    abilities: &[enter_combat(&[
        R::NotControlled,
        R::ActorReady,
        R::TargetVampire,
        R::TargetLocked,
    ])],
    riders: &[],
    incomplete: None,
};

pub static NESERIAN: VampireDef = VampireDef {
    name: "Neserian",
    printed: &[
        own(Stat::Bleed, -1, &[R::ControlsNoLocation]),
        own(Stat::Votes, 1, &[R::ControlsLocation]),
    ],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

/// "Noluthando inflicts +1 damage with ranged strikes (even at close
/// range)."
pub static NOLUTHANDO: VampireDef = VampireDef {
    name: "Noluthando",
    printed: &[own(Stat::RangedStrikeDamage, 1, &[])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

pub static NONU_DIS: VampireDef = VampireDef {
    name: "Nonu Dis",
    printed: &[own(Stat::Bleed, 1, &[])],
    // Once per master phase however many master cards are played in it
    // (ANK 20220617). "Follower of Set" is the clan's V5 name, Ministry.
    abilities: &[ModeDef {
        windows: &[Window::MasterPhase],
        actor: Actor::Host,
        requires: &[R::HostReady, R::MasterCardPlayed],
        target: Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[R::Controlled, R::ActorReady, R::ActorClan("Ministry")],
        }),
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::BloodToTarget(1)],
    }],
    riders: &[],
    incomplete: None,
};

pub static OLUWAFUNMILAYO: VampireDef = VampireDef {
    name: "Oluwafunmilayo",
    printed: &[own(Stat::Intercept, 1, &[R::IsPolitical])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

/// "Once each combat involving a vampire you control, Opikun can burn 1
/// blood to prevent up to 2 non-aggravated damage to that vampire from the
/// opposing minion's strike." Every damage pending at the prevention step
/// comes from a strike in the engine today; damage from other sources must
/// stay out of this prevention when it lands.
pub static OPIKUN: VampireDef = VampireDef {
    name: "Opikun",
    printed: &[],
    abilities: &[ability(
        &[Window::CombatDamagePrevention],
        Actor::ForOwnCombatant,
        &[R::HostBloodAtLeast(1), R::NonAggravatedDamagePending],
        Limit::PerCombat,
        &[Effect::HostBurnBlood(1), Effect::PreventNonAggravated(2)],
    )],
    riders: &[],
    incomplete: None,
};

pub static OSKAR_ANASOV: VampireDef = VampireDef {
    name: "Oskar Anasov",
    printed: &[own(Stat::Stealth, 1, &[])],
    // To the vampire of his controller attempting to block, while it needs
    // intercept (rules-qa 3.2), for the rest of the action; in torpor too
    // (LSJ 20011022); "during an action": once per action (rules-qa 15.2).
    abilities: &[ModeDef {
        windows: &[Window::ActionStealthIntercept],
        actor: Actor::Host,
        requires: &[R::YourAttemptNeedsIntercept, R::HostBloodAtLeast(1)],
        target: None,
        terms: None,
        waives_needed: false,
        limit: Some(Limit::PerAction),
        effects: &[Effect::HostBurnBlood(1), Effect::AttemptingIntercept(1)],
    }],
    riders: &[],
    incomplete: None,
};

/// "Once each combat, Phaibun can discard a card at random to strike:
/// dodge."
pub static PHAIBUN: VampireDef = VampireDef {
    name: "Phaibun",
    printed: &[],
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
    riders: &[],
    incomplete: None,
};

pub static QUEEN_ANNE: VampireDef = VampireDef {
    name: "Queen Anne",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[
        ModeDef {
            windows: &[Window::Referendum],
            actor: Actor::Host,
            requires: &[
                R::PollingStep,
                R::ActorCalledReferendum,
                R::HostBloodAtLeast(1),
            ],
            target: Some(TargetSpec {
                kind: TargetKind::Minion,
                filter: &[
                    R::Younger,
                    R::ActorSectIn(&[Sect::Camarilla]),
                    R::NotAbstaining,
                ],
            }),
            terms: None,
            waives_needed: false,
            limit: Some(Limit::PerReferendum),
            effects: &[
                Effect::HostBurnBlood(1),
                Effect::Abstain {
                    lock: false,
                    burn: 0,
                },
            ],
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
    riders: &[],
    incomplete: None,
};

pub static RAGNAR_NORDSTROM: VampireDef = VampireDef {
    name: "Ragnar Nordstrom",
    printed: &[own(Stat::Strength, 1, &[R::OpponentYounger])],
    abilities: &[],
    riders: &[],
    incomplete: Some(
        "Cards requiring Protean [pro] cost Ragnar -1 blood. (His +1 strength in combat with \
         an ally waits for allies.)",
    ),
};

/// "After Richard successfully bleeds, he can look at the target
/// Methuselah's hand, then he can burn 1 blood to discard 1 card from it."
/// Once per action (rules-qa 16.1).
pub static RICHARD_DE_WORDE: VampireDef = VampireDef {
    name: "Richard de Worde",
    printed: &[],
    abilities: &[ModeDef {
        windows: &[Window::ActionAfterResolution],
        actor: Actor::Host,
        requires: &[
            R::ActorActing,
            R::IsBleed,
            R::ActionSuccessful,
            R::HostBloodAtLeast(1),
            R::DirectedHandNotEmpty,
        ],
        target: None,
        terms: Some(Terms::DirectedHandCard),
        waives_needed: false,
        limit: Some(Limit::PerAction),
        effects: &[Effect::HostBurnBlood(1)],
    }],
    riders: &[],
    incomplete: None,
};

pub static RINALDO_ALBIZZI: VampireDef = VampireDef {
    name: "Rinaldo Albizzi",
    printed: &[own(Stat::Intercept, 1, &[])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

pub static ROSE_ABAWI: VampireDef = VampireDef {
    name: "Rose Abawi",
    printed: &[own(Stat::Bleed, 1, &[R::ControlsVampireNamed("Mithras")])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

/// "Rowena can prevent 1 non-aggravated damage each round."
pub static ROWENA: VampireDef = VampireDef {
    name: "Rowena",
    printed: &[],
    abilities: &[ability(
        &[Window::CombatDamagePrevention],
        Actor::Host,
        &[R::NonAggravatedDamagePending],
        Limit::PerRound,
        &[Effect::PreventNonAggravated(1)],
    )],
    riders: &[],
    incomplete: Some(
        "She can strike: combat ends in combat with a werewolf. (Werewolves are allies: waits \
         for allies.)",
    ),
};

pub static ROY: VampireDef = VampireDef {
    name: "Roy",
    printed: &[own(Stat::Strength, 1, &[R::OpponentTitled])],
    abilities: &[],
    // "If Roy is unlocked during your discard phase, lock him": as the phase
    // ends, in torpor too (LSJ 20011022).
    riders: &[Rider {
        hook: Hook::DiscardPhase,
        scope: RiderScope::Controller,
        requires: &[R::HostUnlocked],
        effects: &[Effect::LockActor],
    }],
    incomplete: None,
};

/// "Sakhar can search your library for an equipment card, reveal it and
/// move it to your hand as a +1 stealth action."
/// "Saankaláxt can steal an equipment as a Ⓓ action that costs 1 blood."
/// Not a second vehicle (rules-qa 17.4).
pub static SAANKALAXT: VampireDef = VampireDef {
    name: "Saankaláxt",
    printed: &[],
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
    riders: &[],
    incomplete: None,
};

pub static SAKHAR: VampireDef = VampireDef {
    name: "Sakhar",
    printed: &[],
    abilities: &[printed_action(
        None,
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::SearchToHand(CardFilter::Type(CardType::Equipment)),
        ],
    )],
    riders: &[],
    incomplete: None,
};

pub static SAKU_PIHLAJAMAKI: VampireDef = VampireDef {
    name: "Saku Pihlajamäki",
    printed: &[],
    abilities: &[MANEUVER_EACH_COMBAT],
    riders: &[],
    incomplete: None,
};

/// "Seraphina can add 2 blood or life to a minion you control as a +1
/// stealth action, not to exceed starting life." A vampire's blood stops at
/// its capacity; one in torpor, or Seraphina herself, may be chosen.
pub static SERAPHINA: VampireDef = VampireDef {
    name: "Seraphina",
    printed: &[],
    abilities: &[printed_action(
        Some(TargetSpec {
            kind: TargetKind::Minion,
            filter: &[R::Controlled],
        }),
        &[
            Effect::DeclareAction(ActionKind::Ability),
            Effect::AddStealth(1),
            Effect::BloodToTarget(2),
        ],
    )],
    riders: &[],
    incomplete: None,
};

pub static SREELEKHA: VampireDef = VampireDef {
    name: "Sreelekha",
    printed: &[
        own(Stat::Bleed, 1, &[]),
        own(Stat::DiscardPhaseActions, 1, &[R::ThisVampireReady]),
    ],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

/// "Blood hunt referendums called on Styles get +2 votes in favor of the
/// referendum."
pub static STYLES_MARGS: VampireDef = VampireDef {
    name: "Styles Margs",
    printed: &[own(Stat::BloodHuntVotesInFavor, 2, &[])],
    abilities: &[],
    riders: &[],
    incomplete: None,
};

/// "You can unlock Sybren after a referendum called by him passes."
pub static SYBREN_VAN_OOSTEN: VampireDef = VampireDef {
    name: "Sybren van Oosten",
    printed: &[],
    abilities: &[ModeDef {
        windows: &[Window::ActionAfterResolution],
        actor: Actor::Host,
        requires: &[
            R::ActorActing,
            R::IsPolitical,
            R::ReferendumPassed,
            R::HostLocked,
        ],
        target: None,
        terms: None,
        waives_needed: false,
        limit: None,
        effects: &[Effect::UnlockActor],
    }],
    riders: &[],
    incomplete: None,
};

pub static THEO_BELL: VampireDef = VampireDef {
    name: "Theo Bell",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[enter_combat(&[R::NotControlled, R::ActorReady])],
    riders: &[],
    incomplete: None,
};

pub static VALERIUS: VampireDef = VampireDef {
    name: "Valerius",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    riders: &[],
    incomplete: Some(
        "If Valerius is Camarilla and not a prince, he can steal the title held by a prince as \
         a Ⓓ action.",
    ),
};

pub static VALERIYA_ZINOVIEVA: VampireDef = VampireDef {
    name: "Valeriya Zinovieva",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[],
    riders: &[],
    incomplete: None,
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
    abilities: &[],
    riders: &[],
    incomplete: None,
};

pub static YEWON_ONG: VampireDef = VampireDef {
    name: "Yewon Ong",
    printed: &[],
    abilities: &[ModeDef {
        windows: &[Window::Referendum],
        actor: Actor::Host,
        requires: &[
            R::AnyPollingStep,
            R::HandHas(CardFilter::Discipline("obl"), 1),
        ],
        target: None,
        terms: Some(Terms::Discard {
            n: 1,
            of: CardFilter::Discipline("obl"),
        }),
        waives_needed: false,
        limit: Some(Limit::PerReferendum),
        effects: &[Effect::AddVotes(1)],
    }],
    riders: &[],
    incomplete: None,
};

pub static VAMPIRES: [&VampireDef; 81] = [
    &ABADDON,
    &ABRAHAM_DUSABLE,
    &ADISA,
    &ADRINO_MANAUARA,
    &ALABASTROM,
    &AISHA_AZ_ZAHRA,
    &ALEXA_DRAPER,
    &ALICIA_CORTEZ,
    &ALINE_GADEKE,
    &ANDRE_THE_MANIPULATOR,
    &ANIEL,
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
    &DOMINICA,
    &DOWAGER_THE,
    &EGIDIA_ARRU,
    &ELEN_KAMJIAN,
    &EVAN_KLEIN,
    &FARUQ_ABD_AL_QADIR,
    &FLAVIO_GONCALVES,
    &GATHII,
    &GWENLLIAN_ARWYN,
    &HAFTHOR_THORSTEINSSON,
    &HESHA_RUHADZE,
    &ILONKA,
    &JASON_SON_NEWBERRY,
    &KALINDA,
    &KAMILE_PAUKSTYS,
    &KASIM_BAYAR,
    &KEVIN_JACKSON,
    &LARISSA_MOREIRA,
    &LENELLE,
    &LENNY_BURKHEAD,
    &MARCHESA_LILIANA,
    &MARIALENA,
    &MARTINA_SRNANKOVA,
    &MASSIMILIANO,
    &MICKEY_WHEELER,
    &MITHRAS,
    &MORA_THE_DEATH_SEER,
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
    &RICHARD_DE_WORDE,
    &RINALDO_ALBIZZI,
    &ROSE_ABAWI,
    &ROWENA,
    &ROY,
    &SAANKALAXT,
    &SAKHAR,
    &SAKU_PIHLAJAMAKI,
    &SERAPHINA,
    &SREELEKHA,
    &STYLES_MARGS,
    &SYBREN_VAN_OOSTEN,
    &THEO_BELL,
    &VALERIUS,
    &VALERIYA_ZINOVIEVA,
    &WEIRICH_WALDBURG,
    &YEWON_ONG,
];
