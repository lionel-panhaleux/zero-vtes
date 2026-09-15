//! Printed text of vampires, keyed by printed name like the library
//! `defs`. The crypt data (`crate::crypt`) stays generated; what the text
//! does is written here: plain modifiers read live by
//! `GameState::printed_mod`, and abilities in the card vocabulary (the
//! actor `Host` is the vampire itself). A vampire may land with part of
//! its text — every vampire ends with all of it (rules-qa 13.1) — the rest
//! quoted in `incomplete`. Slice 9a: plain stat modifiers; 9b: combat
//! abilities.

use super::{
    Actor, Effect, Limit, ModeDef, Per, Printed, PrintedTo, Requirement as R, Stat, StrikeKind,
    Terms, TitleClass, VampireDef, Window,
};
use crate::library::CardType;

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
    incomplete: None,
};

pub static AISHA_AZ_ZAHRA: VampireDef = VampireDef {
    name: "Aisha az-Zahra",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    incomplete: Some(
        "During the polling step of any referendum, Aisha can burn 1 blood to get +1 vote for \
         each ready Anarch you control.",
    ),
};

pub static ANDRE_THE_MANIPULATOR: VampireDef = VampireDef {
    name: "André the Manipulator",
    printed: &[own(Stat::HandSize, 2, &[R::ThisVampireReady])],
    abilities: &[],
    incomplete: None,
};

pub static ARIANE: VampireDef = VampireDef {
    name: "Ariane",
    printed: &[own(Stat::Stealth, -1, &[R::IsUndirected])],
    abilities: &[],
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
    incomplete: None,
};

pub static BRANIMIRA: VampireDef = VampireDef {
    name: "Branimira",
    printed: &[own(Stat::Intercept, 1, &[R::PoolAtMost(9)])],
    abilities: &[],
    incomplete: None,
};

pub static BRET_STRYKER: VampireDef = VampireDef {
    name: "Bret Stryker",
    printed: &[own(Stat::Intercept, -1, &[R::OpponentTitled])],
    abilities: &[],
    incomplete: None,
};

pub static CARMELITA_NEILLSON: VampireDef = VampireDef {
    name: "Carmelita Neillson",
    printed: &[own(Stat::HandSize, 1, &[R::ThisVampireReady])],
    abilities: &[],
    incomplete: None,
};

pub static CASTELLAN: VampireDef = VampireDef {
    name: "Castellan",
    printed: &[
        own(Stat::Intercept, 1, &[R::IsDirected]),
        own(Stat::Intercept, -1, &[R::IsUndirected]),
    ],
    abilities: &[],
    incomplete: None,
};

pub static CATALINA_VEGA: VampireDef = VampireDef {
    name: "Catalina Vega",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
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
    incomplete: None,
};

pub static DJENEBA: VampireDef = VampireDef {
    name: "Djeneba",
    printed: &[own(Stat::Stealth, 1, &[])],
    abilities: &[],
    incomplete: Some("In combat, strike cards cost the opposing minion +1 blood or life."),
};

pub static DOWAGER_THE: VampireDef = VampireDef {
    name: "Dowager, The",
    printed: &[own(Stat::Intercept, 1, &[R::IsDirected])],
    abilities: &[],
    incomplete: None,
};

pub static EGIDIA_ARRU: VampireDef = VampireDef {
    name: "Egidia Arrú",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[],
    incomplete: Some(
        "After a minion in combat with Egidia leaves the ready region, their controller burns \
         1 pool.",
    ),
};

pub static FARUQ_ABD_AL_QADIR: VampireDef = VampireDef {
    name: "Faruq Abd al-Qadir",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[],
    incomplete: Some(
        "Minions with any of your corruption counters in combat with Faruq cannot strike: \
         combat ends.",
    ),
};

pub static FLAVIO_GONCALVES: VampireDef = VampireDef {
    name: "Flávio Gonçalves",
    printed: &[],
    abilities: &[DODGE_EACH_COMBAT],
    incomplete: None,
};

pub static GWENLLIAN_ARWYN: VampireDef = VampireDef {
    name: "Gwenllian Arwyn",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[],
    incomplete: Some(
        "During an action she performs, Gwenllian can burn 1 blood to give a blocking vampire \
         -1 intercept.",
    ),
};

pub static HAFTHOR_THORSTEINSSON: VampireDef = VampireDef {
    name: "Hafthor Thorsteinsson",
    printed: &[own(Stat::Intercept, 1, &[])],
    abilities: &[],
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
    incomplete: None,
};

pub static JASON_SON_NEWBERRY: VampireDef = VampireDef {
    name: "Jason \"Son\" Newberry",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    incomplete: Some("Jason gets +2 votes when casting votes against blood hunt referendums."),
};

pub static KAMILE_PAUKSTYS: VampireDef = VampireDef {
    name: "Kamile Paukstys",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
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
        requires: &[R::HandOfType(CardType::PoliticalAction, 1)],
        target: None,
        terms: Some(Terms::Discard {
            n: 1,
            of: Some(CardType::PoliticalAction),
        }),
        waives_needed: false,
        limit: Some(Limit::PerCombat),
        effects: &[Effect::StrengthThisCombat(2)],
    }],
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
    incomplete: None,
};

pub static LENNY_BURKHEAD: VampireDef = VampireDef {
    name: "Lenny Burkhead",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    incomplete: None,
};

pub static MARCHESA_LILIANA: VampireDef = VampireDef {
    name: "Marchesa Liliana",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    incomplete: Some(
        "During a bleed action, Liliana can remove seven cards in your ash heap from the game \
         to get an additional +1 bleed.",
    ),
};

pub static MARTINA_SRNANKOVA: VampireDef = VampireDef {
    name: "Martina Srnankova",
    printed: &[own(Stat::Intercept, 1, &[R::IsBleed])],
    abilities: &[],
    incomplete: None,
};

pub static MASSIMILIANO: VampireDef = VampireDef {
    name: "Massimiliano",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[],
    incomplete: None,
};

pub static MICKEY_WHEELER: VampireDef = VampireDef {
    name: "Mickey Wheeler",
    printed: &[own(Stat::Strength, 1, &[R::OpponentOlder])],
    abilities: &[],
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
    incomplete: Some(
        "Younger vampires must burn 1 blood to attempt to block Mithras or to cast votes or \
         ballots against a referendum called by him.",
    ),
};

pub static MORA_THE_DEATH_SEER: VampireDef = VampireDef {
    name: "Mora, the Death Seer",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    incomplete: Some(
        "If Mora is ready during your discard phase, you can move a library card from your ash \
         heap to the bottom of your library.",
    ),
};

pub static NESERIAN: VampireDef = VampireDef {
    name: "Neserian",
    printed: &[
        own(Stat::Bleed, -1, &[R::ControlsNoLocation]),
        own(Stat::Votes, 1, &[R::ControlsLocation]),
    ],
    abilities: &[],
    incomplete: None,
};

/// "Noluthando inflicts +1 damage with ranged strikes (even at close
/// range)."
pub static NOLUTHANDO: VampireDef = VampireDef {
    name: "Noluthando",
    printed: &[own(Stat::RangedStrikeDamage, 1, &[])],
    abilities: &[],
    incomplete: None,
};

pub static NONU_DIS: VampireDef = VampireDef {
    name: "Nonu Dis",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    incomplete: Some(
        "If Nonu Dis is ready during your master phase, you can add 1 blood to a ready Follower \
         of Set you control after playing a master card.",
    ),
};

pub static OLUWAFUNMILAYO: VampireDef = VampireDef {
    name: "Oluwafunmilayo",
    printed: &[own(Stat::Intercept, 1, &[R::IsPolitical])],
    abilities: &[],
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
    incomplete: None,
};

pub static OSKAR_ANASOV: VampireDef = VampireDef {
    name: "Oskar Anasov",
    printed: &[own(Stat::Stealth, 1, &[])],
    abilities: &[],
    incomplete: Some(
        "During an action, Oskar can burn 1 blood to give a vampire you control +1 intercept.",
    ),
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
    incomplete: None,
};

pub static QUEEN_ANNE: VampireDef = VampireDef {
    name: "Queen Anne",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    incomplete: Some(
        "During the polling step of a referendum called by Anne, she can burn 1 blood to force \
         a younger Camarilla vampire to abstain. After she diablerizes a vampire, she can \
         unlock.",
    ),
};

pub static RAGNAR_NORDSTROM: VampireDef = VampireDef {
    name: "Ragnar Nordstrom",
    printed: &[own(Stat::Strength, 1, &[R::OpponentYounger])],
    abilities: &[],
    incomplete: Some(
        "Cards requiring Protean [pro] cost Ragnar -1 blood. (His +1 strength in combat with \
         an ally waits for allies.)",
    ),
};

pub static RINALDO_ALBIZZI: VampireDef = VampireDef {
    name: "Rinaldo Albizzi",
    printed: &[own(Stat::Intercept, 1, &[])],
    abilities: &[],
    incomplete: None,
};

pub static ROSE_ABAWI: VampireDef = VampireDef {
    name: "Rose Abawi",
    printed: &[own(Stat::Bleed, 1, &[R::ControlsVampireNamed("Mithras")])],
    abilities: &[],
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
    incomplete: Some(
        "She can strike: combat ends in combat with a werewolf. (Werewolves are allies: waits \
         for allies.)",
    ),
};

pub static ROY: VampireDef = VampireDef {
    name: "Roy",
    printed: &[own(Stat::Strength, 1, &[R::OpponentTitled])],
    abilities: &[],
    incomplete: Some("If Roy is unlocked during your discard phase, lock him."),
};

pub static SAKU_PIHLAJAMAKI: VampireDef = VampireDef {
    name: "Saku Pihlajamäki",
    printed: &[],
    abilities: &[MANEUVER_EACH_COMBAT],
    incomplete: None,
};

pub static SREELEKHA: VampireDef = VampireDef {
    name: "Sreelekha",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    incomplete: Some(
        "If Sreelekha is ready at the start of your discard phase, you get +1 discard phase \
         action.",
    ),
};

pub static THEO_BELL: VampireDef = VampireDef {
    name: "Theo Bell",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[],
    incomplete: Some("Theo can enter combat with a minion as a Ⓓ action."),
};

pub static VALERIUS: VampireDef = VampireDef {
    name: "Valerius",
    printed: &[own(Stat::Bleed, 1, &[])],
    abilities: &[],
    incomplete: Some(
        "If Valerius is Camarilla and not a prince, he can steal the title held by a prince as \
         a Ⓓ action.",
    ),
};

pub static VALERIYA_ZINOVIEVA: VampireDef = VampireDef {
    name: "Valeriya Zinovieva",
    printed: &[own(Stat::Strength, 1, &[])],
    abilities: &[],
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
    incomplete: None,
};

pub static VAMPIRES: [&VampireDef; 49] = [
    &ABADDON,
    &AISHA_AZ_ZAHRA,
    &ANDRE_THE_MANIPULATOR,
    &ARIANE,
    &AZUCENA,
    &BRANIMIRA,
    &BRET_STRYKER,
    &CARMELITA_NEILLSON,
    &CASTELLAN,
    &CATALINA_VEGA,
    &CHUN_HEI,
    &DJENEBA,
    &DOWAGER_THE,
    &EGIDIA_ARRU,
    &FARUQ_ABD_AL_QADIR,
    &FLAVIO_GONCALVES,
    &GWENLLIAN_ARWYN,
    &HAFTHOR_THORSTEINSSON,
    &HESHA_RUHADZE,
    &JASON_SON_NEWBERRY,
    &KAMILE_PAUKSTYS,
    &KASIM_BAYAR,
    &KEVIN_JACKSON,
    &LENNY_BURKHEAD,
    &MARCHESA_LILIANA,
    &MARTINA_SRNANKOVA,
    &MASSIMILIANO,
    &MICKEY_WHEELER,
    &MITHRAS,
    &MORA_THE_DEATH_SEER,
    &NESERIAN,
    &NOLUTHANDO,
    &NONU_DIS,
    &OLUWAFUNMILAYO,
    &OPIKUN,
    &OSKAR_ANASOV,
    &PHAIBUN,
    &QUEEN_ANNE,
    &RAGNAR_NORDSTROM,
    &RINALDO_ALBIZZI,
    &ROSE_ABAWI,
    &ROWENA,
    &ROY,
    &SAKU_PIHLAJAMAKI,
    &SREELEKHA,
    &THEO_BELL,
    &VALERIUS,
    &VALERIYA_ZINOVIEVA,
    &WEIRICH_WALDBURG,
];
