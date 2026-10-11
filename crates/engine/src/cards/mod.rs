//! Card behavior: the agreed representation (DESIGN §4.5). A card is a set of modes; a mode's *legality
//! half* (windows, actor, requirements, cost, targets, terms) is a closed
//! vocabulary evaluated here by the engine, so the option mask is enumerated
//! from data; its *execution half* (effects) is interpreted at resolution.
//! Definitions live in `defs`.

pub mod defs;
pub mod vampires;

use crate::actions::{ActionCtx, ActionState};
use crate::combat::{CombatCtx, Range, Step};
use crate::crypt::{Sect, Title};
use crate::decision::ActionKind;
use crate::ids::{MinionId, SeatId};
use crate::library::{CardType, Cost, LIBRARY};
use crate::referendum::{RefKind, RefStep, ReferendumCtx};
use crate::state::{CardId, GameState};

/// Play windows: the names of data/window-matrix-reviewed.csv, plus the
/// in-play ability hooks the engine opens (`UnlockPhaseAny`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Window {
    MasterPhase,
    CardAsPlayed,
    DiscardPhase,
    ActionDeclare,
    ActionDeclarePolitical,
    ActionDeclareEquip,
    ActionDeclareRecruit,
    ActionDeclareEmploy,
    ActionAsAnnounced,
    /// As the action is announced, for a Methuselah other than the acting
    /// one, after the acting one's own as-announced effects (Screamer, LSJ
    /// 20081105).
    ReactionAsAnnounced,
    ActionDuring,
    ActionStealthIntercept,
    ActionBlockAttempt,
    ActionBlocked,
    ActionBlocksDeclined,
    ActionWouldBeSuccessful,
    ActionAfterResolution,
    ActionAfterCombat,
    ReactionDuringAction,
    ReactionUnlock,
    ReactionUnlockAttempt,
    Wake,
    Referendum,
    CombatBeforeRange,
    CombatDetermineRange,
    CombatBeforeStrikes,
    CombatStrikeDeclaration,
    CombatAdditionalStrike,
    CombatDamagePrevention,
    CombatPress,
    CombatEndOfRound,
    CombatWouldEnd,
    CombatUntimed,
    /// Both strikes chosen, before they resolve, for a card triggered by
    /// the opposing strike (Rötschreck: "when an opposing minion attempts
    /// to inflict aggravated damage"; LSJ 19990217).
    CombatBeforeResolution,
    /// A Methuselah is about to be ousted, with no pool (Life Boon: "give
    /// pool to a Methuselah with no pool to keep them in the game"): the
    /// decision point `ousting` opens before the oust applies.
    Ousting,
    /// In-play ability hook: during any Methuselah's unlock phase.
    UnlockPhaseAny,
    /// In-play ability hook: during the host's controller's unlock phase
    /// ("during their unlock phase": Vessel).
    UnlockPhaseOwn,
    /// The active Methuselah's discard phase, for an ability spending a
    /// discard phase action ("you can use a discard phase action to …",
    /// Ayse Dhanial): no "during" clause, so not once per phase.
    DiscardPhaseAction,
    /// In-play ability hook: during the controller's influence phase
    /// ("lock during your influence phase to …", Arcane Library): once per
    /// phase per copy.
    InfluencePhaseOwn,
    /// The controller's influence phase, for an ability spending transfers
    /// ("you can use 1 transfer to …", Wider View): no "during" clause, so
    /// as often as the transfers last.
    Transfers,
    /// In-play ability hook: any decision point where the controller
    /// holds the impulse — "can be used during an action, when the
    /// controller has the opportunity to play an effect" (ANK 20200616) —
    /// except the as-played window (cancels and wakes only, rules-qa
    /// 2.4). Matched only where a request site asks for it.
    AnyImpulse,
    /// In-play ability hook: another Methuselah's minion phase, as they
    /// declare its end (the phase rotation, rules-qa 7.9): "during any
    /// Methuselah's minion phase, this vampire can lock this card to take
    /// an action" (Enkil Cog; rules-qa 49.6).
    OthersMinionPhaseEnd,
}

impl Window {
    /// A phase window: an in-play ability offered here follows the
    /// "during X, do Y" template — once per that phase per copy (rules-qa
    /// 6.6).
    pub fn is_phase(self) -> bool {
        matches!(
            self,
            Window::MasterPhase
                | Window::UnlockPhaseAny
                | Window::UnlockPhaseOwn
                | Window::InfluencePhaseOwn
                | Window::DiscardPhase
        )
    }
}

impl Window {
    /// A window of a combat (the combat cards' windows).
    pub fn is_combat(self) -> bool {
        use Window::*;
        matches!(
            self,
            CombatBeforeRange
                | CombatDetermineRange
                | CombatBeforeStrikes
                | CombatStrikeDeclaration
                | CombatAdditionalStrike
                | CombatDamagePrevention
                | CombatPress
                | CombatEndOfRound
                | CombatWouldEnd
                | CombatUntimed
                | CombatBeforeResolution
        )
    }

    pub fn from_name(name: &str) -> Option<Window> {
        use Window::*;
        Some(match name {
            "MASTER_PHASE" => MasterPhase,
            "CARD_AS_PLAYED" => CardAsPlayed,
            "DISCARD_PHASE" => DiscardPhase,
            "ACTION_DECLARE" => ActionDeclare,
            "ACTION_DECLARE_POLITICAL" => ActionDeclarePolitical,
            "ACTION_DECLARE_EQUIP" => ActionDeclareEquip,
            "ACTION_DECLARE_RECRUIT" => ActionDeclareRecruit,
            "ACTION_DECLARE_EMPLOY" => ActionDeclareEmploy,
            "ACTION_AS_ANNOUNCED" => ActionAsAnnounced,
            "ACTION_DURING" => ActionDuring,
            "ACTION_STEALTH_INTERCEPT" => ActionStealthIntercept,
            "ACTION_BLOCK_ATTEMPT" => ActionBlockAttempt,
            "ACTION_BLOCKED" => ActionBlocked,
            "ACTION_BLOCKS_DECLINED" => ActionBlocksDeclined,
            "ACTION_WOULD_BE_SUCCESSFUL" => ActionWouldBeSuccessful,
            "ACTION_AFTER_RESOLUTION" => ActionAfterResolution,
            "ACTION_AFTER_COMBAT" => ActionAfterCombat,
            "REACTION_DURING_ACTION" => ReactionDuringAction,
            "REACTION_UNLOCK" => ReactionUnlock,
            "REACTION_UNLOCK_ATTEMPT" => ReactionUnlockAttempt,
            "WAKE" => Wake,
            "REFERENDUM" => Referendum,
            "COMBAT_BEFORE_RANGE" => CombatBeforeRange,
            "COMBAT_DETERMINE_RANGE" => CombatDetermineRange,
            "COMBAT_BEFORE_STRIKES" => CombatBeforeStrikes,
            "COMBAT_STRIKE_DECLARATION" => CombatStrikeDeclaration,
            "COMBAT_ADDITIONAL_STRIKE" => CombatAdditionalStrike,
            "COMBAT_DAMAGE_PREVENTION" => CombatDamagePrevention,
            "COMBAT_PRESS" => CombatPress,
            "COMBAT_END_OF_ROUND" => CombatEndOfRound,
            "COMBAT_WOULD_END" => CombatWouldEnd,
            "COMBAT_UNTIMED" => CombatUntimed,
            "COMBAT_BEFORE_RESOLUTION" => CombatBeforeResolution,
            "OUSTING" => Ousting,
            _ => return None,
        })
    }
}

/// Who may use a mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Actor {
    /// The Methuselah, no minion (masters, events).
    Methuselah,
    /// The acting minion (action modifiers).
    ActingMinion,
    /// A ready unlocked minion of a Methuselah other than the acting one
    /// (reaction cards); widened to locked minions by a wake or by the
    /// mode's own `LockedActor` requirement.
    ReactingMinion,
    /// A ready unlocked minion of the active Methuselah declaring an action
    /// with this card (action cards, equipment, allies, retainers).
    ActingCandidate,
    /// The host minion of a card in play (in-play abilities).
    Host,
    /// The Methuselah controlling a card in play, no minion: the ability
    /// picks a `target` (a hunting ground's vampire).
    Controller,
    /// The player's minion in the current combat (combat cards). Ready,
    /// except in the end-of-round window, where a minion being burned or
    /// sent to torpor may still play unless the card excludes it.
    Combatant,
    /// An ability's host — on a vampire's own text, the vampire — in the
    /// combat or out of it, in torpor included, used for its controller's
    /// minion in combat: the combat effects apply to that minion (Opikun).
    ForOwnCombatant,
    /// "Action Modifier/Reaction" (Scalpel Tongue, Ominous Chorus): the
    /// acting minion for its controller, a reacting minion for anyone else.
    ActingOrReactingMinion,
    /// "Only usable by a ready vampire other than the acting minion" (Cloak
    /// the Gathering [OBF], Veil the Legions): another ready minion of the
    /// acting Methuselah, locked or not (the mode's requirements narrow
    /// it); only the acting minion's controller plays it (LSJ 19990425).
    OtherOwnMinion,
    /// "Only usable by a ready vampire not involved in the combat involving
    /// another minion you control" (Touch of Valeren [FOR]): the combat
    /// effects apply to that minion.
    OwnOutOfCombat,
    /// The Methuselah controlling the card's chosen vampire, which must be
    /// in play (The Rack: "the chosen vampire can gain 2 blood"; a stolen
    /// one's new controller decides, ANK 20200130-2).
    ChosenController,
    /// A master card played for one of its player's ready unlocked minions,
    /// which meets its requirements and pays its costs (Piper: "lock a
    /// ready unlocked Anarch you control. That Anarch recruits or employs
    /// …").
    MasterForMinion,
    /// Any ready vampire of the Methuselah using it ("any ready vampire can
    /// burn 2 blood to …", Carver's Meat Packing and Storage).
    AnyReadyVampire,
    /// "Only usable by an unlocked vampire not involved in the combat"
    /// (Martyr's Resilience): any Methuselah's (ANK 20181101), in torpor
    /// too (LSJ 20020416).
    UnlockedOutOfCombat,
    /// The vampire whose text it is, out of the combat, its ability used on
    /// a minion in it (Randall: "prevent 1 non-aggravated damage inflicted
    /// on any other minion in combat").
    HostOutOfCombat,
    /// The prey of the Methuselah controlling the vampire whose text it is
    /// ("your prey can burn the Edge … to burn 1 blood from Keith", Keith
    /// Moody): they use it, the vampire its host.
    HostControllersPrey,
}

/// A class of printed title, for "requires a prince or justicar" style
/// requirements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TitleClass {
    Any,
    Primogen,
    Prince,
    Baron,
    Justicar,
    InnerCircle,
    Archbishop,
    Bishop,
    Priscus,
    Cardinal,
    Regent,
}

impl TitleClass {
    pub fn matches(self, t: Title) -> bool {
        match (self, t) {
            (_, Title::None) => false,
            (TitleClass::Any, _) => true,
            (TitleClass::Primogen, Title::Primogen) => true,
            (TitleClass::Prince, Title::Prince(_)) => true,
            (TitleClass::Baron, Title::Baron(_)) => true,
            (TitleClass::Justicar, Title::Justicar(_)) => true,
            (TitleClass::InnerCircle, Title::InnerCircle(_)) => true,
            (TitleClass::Archbishop, Title::Archbishop(_)) => true,
            (TitleClass::Bishop, Title::Bishop) => true,
            (TitleClass::Priscus, Title::Priscus) => true,
            (TitleClass::Cardinal, Title::Cardinal) => true,
            (TitleClass::Regent, Title::Regent) => true,
            _ => false,
        }
    }
}

/// What a minion may hold only one of ("a minion can have only one
/// vehicle", "… only one Kevlar Vest").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Holding {
    Label(crate::library::Label),
    Named(&'static str),
}

/// An ally's permission to play cards "as a vampire".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AsVampire {
    /// The basic Discipline it meets (one).
    pub disciplines: &'static [&'static str],
    /// Its capacity for the cards it so plays (1 unless printed).
    pub capacity: u8,
    /// Only non-action cards (Spectral Servitor).
    pub non_action: bool,
}

/// The trait printed text names the bearer by ("this Ravnos", "this
/// Anarch", "the bearer with Auspex"): read on the current bearer, so a
/// minion without it gets nothing from that text (rules-qa 9.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bearer {
    Any,
    Clan(&'static str),
    Sect(Sect),
    /// "The bearer with Auspex [aus]": lower case at any level, upper case
    /// superior only.
    Discipline(&'static str),
}

impl Bearer {
    pub fn is(self, st: &GameState, m: MinionId) -> bool {
        let v = &st.minions[m.index()];
        match self {
            Bearer::Any => true,
            Bearer::Clan(c) => v.clan() == Some(c),
            Bearer::Sect(s) => st.sect_of(m) == Some(s),
            Bearer::Discipline(d) => st.has_discipline(m, d),
        }
    }
}

/// Does a printed discipline list meet `tag` (lower case: any level; upper
/// case: superior)?
/// The hand cards a cost or a requirement names ("discard two master
/// cards", "a card requiring Dominate [dom]").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CardFilter {
    Any,
    Type(CardType),
    /// A library card whose printed disciplines name this one.
    Discipline(&'static str),
    /// A card of this type bearing this label ("a master archetype card").
    Labeled(CardType, crate::library::Label),
    /// An equipment in play bearing none of these labels ("a
    /// non-electronic, non-vehicle, non-weapon equipment"): a locquipment
    /// is only a location once in play (spec §1.3.1).
    EquipmentWithout(&'static [crate::library::Label]),
    /// A card requiring a Discipline, any one (Aniel).
    RequiresDiscipline,
    /// An ally or retainer card.
    AllyOrRetainer,
    /// A location (a locquipment in play included, spec §1.3.1).
    Location,
    /// A melee weapon: a weapon whose strike is a melee one ("strength+n
    /// damage"), Weighted Walking Stick included.
    MeleeWeapon,
    /// A non-unique equipment card.
    NonUniqueEquipment,
    /// Any card bearing this label (a ghoul ally or retainer).
    HasLabel(crate::library::Label),
    /// A card requiring this clan ("an ally requiring Hecata").
    Clan(&'static str),
    /// A card of this name ("a card named Golconda: Inner Peace").
    Named(&'static str),
    /// A minion card: any library card but a master or an event (rules-qa
    /// 60.3).
    MinionCard,
    /// Concealed Weapon's weapon: a non-unique weapon whose regular strike
    /// inflicts neither aggravated damage nor 4 or more (strength 1, no
    /// bonus from the minion or cards in play: LSJ 20020821); its pool cost
    /// under 3 is checked against the state (`equippable_from_hand`).
    ConcealableWeapon,
}

impl CardFilter {
    pub fn matches(self, card: u16) -> bool {
        let c = &LIBRARY[card as usize];
        match self {
            CardFilter::Any => true,
            CardFilter::Type(t) => c.types.contains(&t),
            CardFilter::Discipline(d) => c.disciplines.contains(&d),
            CardFilter::Labeled(t, l) => c.types.contains(&t) && c.labels.contains(&l),
            CardFilter::EquipmentWithout(ls) => {
                c.types.contains(&CardType::Equipment)
                    && !c.labels.contains(&crate::library::Label::Location)
                    && !ls.iter().any(|l| c.labels.contains(l))
            }
            CardFilter::RequiresDiscipline => !c.disciplines.is_empty(),
            CardFilter::AllyOrRetainer => {
                c.types.contains(&CardType::Ally) || c.types.contains(&CardType::Retainer)
            }
            CardFilter::Location => c.labels.contains(&crate::library::Label::Location),
            CardFilter::MeleeWeapon => melee_strike(card).is_some(),
            CardFilter::HasLabel(l) => c.labels.contains(&l),
            CardFilter::Clan(k) => c.clans.contains(&k),
            CardFilter::Named(n) => c.name == n,
            CardFilter::MinionCard => !c
                .types
                .iter()
                .any(|t| matches!(t, CardType::Master | CardType::Event)),
            CardFilter::ConcealableWeapon => {
                c.labels.contains(&crate::library::Label::Weapon)
                    && !c.labels.contains(&crate::library::Label::Unique)
                    && regular_strike_damage(card)
                        .is_some_and(|(n, aggravated)| !aggravated && n < 4)
            }
            CardFilter::NonUniqueEquipment => {
                c.types.contains(&CardType::Equipment)
                    && !c.labels.contains(&crate::library::Label::Unique)
            }
        }
    }
}

/// A minion statistic a card in play modifies on its bearer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stat {
    /// Of the bearer's bleeds.
    Bleed,
    /// Of the bearer's actions.
    Stealth,
    /// Of the bearer's block attempts.
    Intercept,
    /// Of the minion's strikes: added to its strength of 1 or to a strength
    /// an effect sets (spec §4.4.1).
    Strength,
    /// Votes the vampire can cast (only while ready).
    Votes,
    /// Its controller's hand size.
    HandSize,
    /// The damage of its ranged strikes, at either range (Noluthando).
    RangedStrikeDamage,
    /// Its controller's discard phase actions, read as the discard phase
    /// begins (Sreelekha).
    DiscardPhaseActions,
    /// A press (mandatory) each combat, only usable to continue combat
    /// (Adrino Manauara).
    MandatoryContinuePress,
    /// Optional presses each combat (Serhat Gunde: "1 optional press each
    /// combat").
    CombatPress,
    /// "Zombies (allies and retainers) you recruit or employ get +n
    /// starting life" (Ashur-uballit, while ready).
    ZombieStartingLife,
    /// Of its votes cast against a blood hunt referendum (Jason "Son"
    /// Newberry).
    VotesAgainstBloodHunt,
    /// Votes in favor of a blood hunt referendum called on it, counted as
    /// the referendum is called (Styles Margs).
    BloodHuntVotesInFavor,
    /// While this vampire is ready, minions burn the top card of their
    /// library to attempt to block wraith or zombie allies (Parijat, the
    /// Dark Oracle).
    WraithZombieBlockToll,
    /// Cannot strike: combat ends (Faruq Abd al-Qadir's opponents with
    /// his Methuselah's corruption counters): any positive total bars it.
    NoCombatEnds,
    /// Each vampire casting votes or ballots against a referendum it calls
    /// burns this much blood once results are tallied (Hartmut Stover;
    /// rules-qa 49.2: not a cost).
    AgainstCastersBurnBlood,
    /// Each Methuselah casting votes or ballots against a referendum it
    /// calls — by a minion of theirs too — burns this much pool once
    /// results are tallied, once each (Marcos Belegrad).
    AgainstMethuselahsBurnPool,
    /// Its controller's transfers, read as the influence phase begins
    /// (Ingrid Rossler: "if Ingrid is ready at the start of your influence
    /// phase").
    Transfers,
    /// Allies cannot block it (Stanislava): any positive total bars them.
    AlliesCannotBlock,
    /// Vampires with capacity n or less cannot block it (Rexton: 3).
    SmallVampiresCannotBlock,
    /// The retainers of a minion opposing it in combat lose their text for
    /// that combat (Stanislava; rules-qa 49.3): any positive total.
    OpposingRetainersLoseText,
    /// Blood off its rescue of a vampire from torpor, the rescued one its
    /// opponent ("rescuing an older vampire from torpor costs Adelino -2
    /// blood"): the action's cost, whoever pays it (PIB 20110918).
    RescueDiscount,
    /// Blood or life a minion burns to attempt to block the vampire whose
    /// text it is, its opponent ("minions with 1 or more of your corruption
    /// counters must burn 1 blood or life to attempt to block Sergio").
    BlockToll,
    /// Blood a vampire burns to cast votes against a referendum called by
    /// the vampire whose text it is, its opponent ("vampires must burn 1
    /// blood to cast votes and ballots against referendums called by
    /// Alexander").
    VoteAgainstToll,
    /// Pool off a card the vampire plays or puts in play ("Equipment cost
    /// Anne-Marie -1 pool"), the card read by the printed predicates.
    PoolCostOff,
    /// Blood off such a card ("cost Roger -1 blood").
    BloodCostOff,
    /// Blood or pool off such a card, whichever it costs ("cost Kuyén -1
    /// blood or pool").
    CostOff,
    /// Its blood may pay up to n of the blood or pool cost of a card its
    /// Methuselah plays, the card read by the printed predicates ("each
    /// time you play a location card, you can use up to 1 of Regina's
    /// blood to pay some or all of its blood or pool cost").
    BloodPaysCost,
    /// Cannot perform directed actions (The Guardian, "unless you control a
    /// ready unique older Nosferatu"): any positive total bars them.
    NoDirectedActions,
    /// Blood an action the modified minion announces costs it more,
    /// read with the action's kind and directedness ("directed actions
    /// cost her +1 blood", Daria; "political actions cost Nosferatu 1
    /// additional blood", Marcellus): payable as the action is announced,
    /// paid first at its resolution (`Engine::action_toll`).
    ActionCost,
    /// Blood or life the opposing minion's strike cards cost it more in
    /// combat with the vampire whose text it is ("in combat, strike cards
    /// cost the opposing minion +1 blood or life", Djeneba): an ally pays
    /// in life (`Engine::combat_card_surcharge`).
    StrikeCardToll,
    /// "The action to become Anarch costs vampires you control 0 blood"
    /// (Brunhilde): any positive total makes it free.
    BecomeAnarchFree,
    /// "Strikes with first strike" (Elimelech): any positive total; its
    /// strikes resolve before a normal opposing one.
    FirstStrike,
    /// "He cannot block vampires of those clans or take actions to enter
    /// combat with them" (Reginald Moore): read with the other minion as
    /// the opponent; any positive total bars it.
    NoBlockOrRush,
    /// Blood a card the vampire plays costs it more, the card read by the
    /// printed predicates (Scout Youngwood: "reaction cards cost her +1
    /// blood").
    BloodCostMore,
    /// "Gehenna events do not affect her" (Karen Suadela, while a prince):
    /// any positive total.
    GehennaImmune,
    /// "Can play cards requiring a justicar as if he were a justicar" (Karl
    /// Schrekt): any positive total.
    PlaysAsJusticar,
    /// Blood hunts cannot be called on it (Theo Bell G2, "if you control a
    /// ready prince or justicar"): any positive total.
    NoBloodHunt,
    /// "+1 hunt" printed on a vampire (Ariadne): added to its hunts, the
    /// non-standard ones too (RTR 20030519).
    Hunt,
    /// It cannot play action cards (The Beast): any positive total.
    NoActionCards,
    /// "If The Capuchin is burned, move him to his owner's uncontrolled
    /// region and you get +n transfers at the start of your next influence
    /// phase": the total is n.
    ToUncontrolledWhenBurned,
    /// "Cannot be contested; if a second … would enter play, burn the first
    /// … in play instead" (Jimmy Dunn): any positive total.
    NeverContested,
    /// It cannot have or use equipment or retainers (The Beast).
    NoEquipmentOrRetainers,
    /// "Once each turn, … can reduce the cost of a card he plays by 1
    /// blood" (Nergal): chosen for a blood-costing card he plays
    /// (`PlayCtx::once_off`), once each turn.
    BloodCostOffOncePerTurn,
    /// "If a card … you play is canceled, you can move that card from your
    /// ash heap to your library (shuffle afterward)" (Victor Donaldson):
    /// the card read as `PrintedCtx::card`.
    RecyclesCanceled,
    /// "If Anson is ready during your master phase, you get two master
    /// phase actions (instead of one)": any positive total, read live
    /// (`GameState::mpa_left`).
    TwoMasterPhaseActions,
    /// "Any vampire contesting Rafael's title must yield during their
    /// unlock phase": any positive total on a title rival bars paying.
    RivalsYieldTitle,
    /// "May attempt to block a vampire controlled by another Methuselah,
    /// ignoring the normal prey, predator or target restrictions" (Anneke):
    /// any positive total.
    BlocksAnyVampire,
    /// "Once each turn, when you draw a card from your library, you may
    /// place that card on the bottom of your library and draw a new card"
    /// (Blackhorse Tanner): any positive total (`Interrupt::Redraw`).
    RedrawOncePerTurn,
}

/// Legality predicates — the closed vocabulary.
/// Evaluated by the engine over game state and the current context; a
/// predicate on a target reads the candidate target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Requirement {
    /// On a rider: once one copy of the card its controller controls has
    /// resolved it this turn, the others' are no longer due ("you can burn
    /// only 1 pool each turn with Army of Rats cards").
    OneCopyPerTurn,
    /// The acting minion is not of this clan ("non-Ventrue minions",
    /// allies included).
    ActorNotClan(&'static str),
    /// The minion attempting to block has at least this capacity (Kumpania).
    AttemptingCapacityAtLeast(u8),
    /// The minion attempting to block is an ally or a vampire younger than
    /// the acting one (Crocodile's Tongue).
    AttemptingAllyOrYounger,
    /// The target vampire has at least this capacity (Golconda).
    TargetCapacityAtLeast(u8),
    /// The target vampire has at most this capacity (Scourge of the
    /// Enochians).
    TargetCapacityAtMost(u8),
    /// The target is of the actor's clan (Grooming the Protégé).
    SameClanAsActor,
    /// The user has a minion in the current combat (Mob Connections).
    OwnMinionInCombat,
    /// Some minion attempts to block and needs intercept (WMRH Talk Radio:
    /// "give a minion +1 intercept").
    AnyAttemptNeedsIntercept,
    /// The host was in the combat that just ended and its opponent is not
    /// ready (Monster).
    HostCombatOpponentNotReady,
    /// As above, the opponent a vampire (Valerie Diaz: "if the opposing
    /// vampire is not ready").
    HostCombatOpposingVampireNotReady,
    /// Normal damage from the opposing strike is pending on the actor's
    /// side (Valerie Diaz).
    StrikeNormalDamagePending,
    /// Normal damage from other sources than the opposing strike is pending
    /// on the actor's side (Valerie Diaz).
    EffectNormalDamagePending,
    /// The actor is an ally whose card matches (Mobile HQ: "an Operation
    /// Antigen ally you control").
    ActorAllyCard(CardFilter),
    /// The target minion is an ally whose card matches.
    TargetAllyCard(CardFilter),
    /// The card being played is an action card, its target a minion or
    /// card of the user's with this label (Mobile HQ: "an action card that
    /// targets an Operation Antigen card you control").
    PlayedActionCardAtYour(crate::library::Label),
    /// The host used at least n Disciplines to play cards during the action
    /// (Dabbler).
    HostUsedDisciplines(u8),
    /// The action requires this clan or this Discipline: its card's, or its
    /// granting card's requirement (Cappadocian Crypt). An empty Discipline
    /// matches none: the clan alone ("an action requiring a Gangrel",
    /// Keegan).
    ActionRequiresClanOrDiscipline(&'static str, &'static str),
    /// The target has this Discipline (Powerbase: Munich).
    TargetDiscipline(&'static str),
    /// The player controls no vampire of this name, and has one in their
    /// crypt (The Mithraeum, London).
    NoVampireNamedButInCrypt(&'static str),
    /// The target is /Mithras/ or a Mithraic Mysteries member.
    TargetMithraic,
    /// The target is not a vampire of this name (Mithraic Cultist).
    TargetNotNamed(&'static str),
    /// The target is Anarch.
    TargetAnarch,
    /// The directed action targets a ready vampire of this clan the player
    /// controls (Yoruba Shrine).
    DirectedAtYourReadyClan(&'static str),
    /// The referendum's terms chose a ready vampire of this clan the player
    /// controls (Yoruba Shrine).
    TermsChoseYourReadyClan(&'static str),
    /// The action is an equip with a non-electronic, non-vehicle,
    /// non-weapon equipment card from hand, and the acting minion has not
    /// unlocked this way this turn (British Museum).
    PlainEquipFromHandOncePerTurn,
    /// The player's own action card costs blood (The Line: by a vampire),
    /// or blood or pool (The Shard), or is an equip (Ravnos Cache).
    OwnActionCardCosts(CostSource),
    /// On an ability of a card in play: fewer than n copies of the card its
    /// controller controls have used it this phase ("you can burn counters
    /// from no more than two Split the Veil cards each unlock phase").
    FewerCopiesUsedThisPhase(u8),
    /// No action other than a mandatory one has been performed this turn
    /// (Khabar: Glory).
    NoOptionalActionThisTurn,
    /// An ally is in some Methuselah's ash heap (Psychophagia: ANK
    /// 20200420-3).
    AllyInAnyAsh,
    /// The player's ash heap holds an ally matching one of these (Hel-Blá:
    /// "an ally requiring Hecata or Oblivion [obl]").
    AshHasAlly(&'static [CardFilter]),
    /// On an unlock-phase rider: the active Methuselah controls a vampire
    /// not of this sect, read as it resolves (Judgment: Camarilla
    /// Segregation; ANK 20181016).
    ActiveControlsNonSect(Sect),
    /// The target minion is not of this sect.
    TargetSectNot(Sect),
    /// Target minion is controlled by the player.
    Controlled,
    /// Target minion has at least this much blood.
    BloodAtLeast(u8),
    /// Target seat is not the acting minion's controller (bleed redirects).
    NotActingController,
    /// Target seat is not the player.
    NotSelf,
    /// The card being played (as-played context) is a master card.
    PlayedCardIsMaster,
    /// The card being played was played by another Methuselah.
    PlayedByOtherSeat,
    /// Playable only on another Methuselah's turn (out-of-turn masters).
    NotOwnTurn,
    /// The current action is directed at the player.
    DirectedAtYou,
    /// The current action is a bleed.
    IsBleed,
    /// The current action is a bleed against the acting Methuselah's prey
    /// (Enkil Cog): read live, a redirected bleed is not (rules-qa 50.3);
    /// a prey this bleed ousted still is (rules-qa 61.2).
    BleedAgainstPrey,
    /// The current bleed was successful: it resolved for 1 pool or more
    /// ("after a successful bleed", Ilonka; rulebook "Bleed").
    BleedSuccessful,
    /// The current bleed amount is at least this.
    BleedAmountAtLeast(u8),
    /// Blocks have been declined by all Methuselahs (state C).
    BlocksDeclined,
    /// Stealth is needed: a block attempt is pending with intercept at
    /// least equal to stealth (rules-qa 3.2). Waived by `waives_needed`.
    StealthNeeded,
    /// Intercept is needed: the actor is the attempting minion and stealth
    /// exceeds its intercept (rules-qa 3.2).
    InterceptNeeded,
    /// The actor minion is locked (wakes: "only usable by a locked vampire").
    LockedActor,
    /// The host of this card in play is below its capacity.
    HostBelowCapacity,
    /// Target vampire has not gained blood from a hunting ground this turn.
    NotHuntingGroundFed,
    /// The player controls a ready vampire holding a title of one of these
    /// classes ("requires a ready vampire with a city title", Papillon).
    ControlsReadyTitled(&'static [TitleClass]),
    /// The player controls a ready vampire of one of these sects
    /// ("requires a ready Anarch", Carfax Abbey).
    ControlsReadySect(&'static [Sect]),
    /// The host's controller has at least n pool.
    HostControllerPoolAtLeast(u8),
    /// The host of the card in play is locked.
    HostLocked,
    /// The host of the card in play has at least this much blood.
    HostBloodAtLeast(u8),
    /// The host — an ally, its own card — has fewer life than its starting
    /// life (The Vozhd of Juiz de Fora).
    HostBelowStartingLife,
    /// The host of the card in play — an ally, its own card — has at most
    /// this much blood or life ("if Double Deuce has 2 or fewer life").
    HostBloodAtMost(u8),
    /// The host of the card in play is in torpor (Fame's unlock rider;
    /// a host that left play with its Methuselah does not qualify).
    HostInTorpor,
    /// The host's controller is the active Methuselah: "during this
    /// vampire's unlock phase" (Fear of Mekhet, a master card its player
    /// keeps on another's vampire).
    HostControllerActive,
    /// The player has a master phase action left ("use a master phase
    /// action", Tension in the Ranks).
    MasterPhaseActionLeft,
    /// The player holds at least n cards matching the filter ("discard two
    /// master cards", "discard a card requiring Dominate [dom]").
    HandHas(CardFilter, u8),
    /// The actor is a vampire, not an ally ("Vampires can burn this card",
    /// "only usable by a locked vampire").
    ActorVampire,
    /// This card in play carries at least n counters (a rider with
    /// nothing to pay is not pending: Smiling Jack at 0 counters).
    CountersAtLeast(u8),
    /// This card has n counters, the one its pending bleed gain adds
    /// included (Forward Momentum, `ActionCtx::momentum`).
    CountersWithBleedGainAtLeast(u8),
    /// This card has at most n counters (Powerbase: Madrid: "if this card
    /// has 3 or fewer counters").
    CountersAtMost(u8),
    /// Printed condition: its controller controls a ready vampire of these
    /// titles (Damian: "while you control a ready cardinal").
    ControlsReadyTitledOther(&'static [TitleClass]),
    /// Printed condition: its controller's predator controls more ready
    /// minions than they do (Khin Aye).
    PredatorMoreReadyMinions,
    /// Printed condition: its controller's prey has n pool or fewer
    /// (Üresség).
    PreyPoolAtMost(u8),
    /// The acting Methuselah's prey burned pool during this action
    /// (Věnceslava).
    PreyBurnedPool,
    /// The user controls at least n ready vampires following the path
    /// (Privileged Position).
    ControlsReadyOnPath(&'static str, u8),
    /// Printed condition: the action's target Methuselah controls a
    /// minion of one of these clans (Osvaldo Kühnemann: "+1 bleed against
    /// a Methuselah controlling a Toreador or Ventrue").
    TargetSeatControlsClan(&'static [&'static str]),
    /// The minion target follows the path (Terrifying Visage).
    TargetFollowsPath(&'static str),
    /// The acting minion follows the path (Forward Momentum).
    ActingFollowsPath(&'static str),
    /// No combat is queued in this action (advanced rules 4.9.3: one at a
    /// time).
    NoQueuedCombat,
    /// The acting minion is an ally, or a vampire younger than the actor
    /// (Confusion of the Eye [obf]: "a younger vampire's or an ally's bleed").
    ActingAllyOrYounger,
    /// The current action is a hunt (Kalyani: "-1 stealth when hunting").
    IsHunt,
    /// Printed-modifier predicate: the action is a diablerie (Lisette
    /// Vizquel: "+1 intercept during diablerie actions").
    IsDiablerie,
    /// The actor is not the acting minion (Yawp Court's Sabbat vampire).
    ActorNotActing,
    /// The acting minion is the user's ally or a vampire younger than the
    /// host (Abderrahim: "an ally or younger vampire you control").
    OwnActingAllyOrYounger,
    /// The current action is a recruit or an employ (Frau Schädel).
    IsRecruitOrEmploy,
    /// The current action requires this path: its card (or the card
    /// granting it) has the path icon (Sakura: "an action requiring the
    /// Path of Death and the Soul").
    ActionRequiresPath(&'static str),
    /// The acting minion is an ally of one of these subtypes (Gifts From
    /// Hereafter: "the acting wraith or zombie ally").
    ActingAllyOf(&'static [crate::library::Label]),
    /// The minion target is an ally of one of these subtypes (Fiorella,
    /// Burial Site Hunting Ground).
    TargetAllyOf(&'static [crate::library::Label]),
    /// The minion target is not an ally of these subtypes nor a vampire
    /// (Heartrender: "a non-wraith non-zombie ally"); a retainer target is
    /// read by the card filter.
    TargetAllyNotOf(&'static [crate::library::Label]),
    /// The minion target is another copy of the actor (Gravebound Drone:
    /// "another copy of this ally you control").
    TargetOtherCopyControlled,
    /// The acting minion is a vampire younger than the actor, which is
    /// about to enter combat with it as its blocker (Obedience).
    BlockingYoungerActing,
    /// The actor is a vampire the action's resolution is about to begin a
    /// combat with ("enter combat with" it), the acting minion a vampire
    /// younger than it, blocks declined (Obedience; rules-qa 55.7, 56.2).
    FoughtByYoungerActing,
    /// The own minion in combat is another copy of the host (Gravebound
    /// Drone's prevention).
    OwnCombatantOtherCopy,
    /// The actor is an ally of one of these subtypes (Dance of the Dead).
    ActorAllyOf(&'static [crate::library::Label]),
    /// The minion attempting to block is another ally of the user's of one
    /// of these subtypes (Fiorella).
    AttemptingAllyOf(&'static [crate::library::Label]),
    /// The actor has chosen its strike and has no aim on it yet: an aim is
    /// played on top of the chosen strike ("a minion can play only one aim
    /// each strike", Target Vitals; owner, 2026-10-11: rules-qa 62.1).
    NoAimYet,
    /// This card in play is unlocked ("you can lock this card to …").
    CardUnlocked,
    /// The player has the Edge ("if you have the Edge").
    HasEdge,
    /// The player's hand is not empty ("discard a card").
    HandNotEmpty,
    /// The user's crypt is not empty (Family Gathering, Wider View: RTR
    /// 20000501, LSJ 20070411-2).
    CryptNotEmpty,
    /// The target vampire is in torpor.
    TargetInTorpor,
    /// The target vampire has hostage counters.
    TargetHostage,
    /// This card was not played before in this game.
    NotPlayedThisGame,
    /// "Only one X can be played each turn" (Ashur Tablets): by anyone,
    /// a canceled play counting as for `NotPlayedThisGame`.
    NotPlayedThisTurn,
    /// "Requires n or more other Gehenna events in play": checked as the
    /// card is played only (PIB 20121031).
    GehennaInPlay(u8),
    /// "A Methuselah can have only one" (Raising the Portcullis): not
    /// playable while its player controls a copy (LSJ 20100129).
    OnePerMethuselah,
    /// The card being played is a grapple or aim card played by the minion
    /// opposing this card's bearer (Sword of the Archangel): engine-checked.
    PlayedGrappleOrAimByOpponent,
    /// This weapon's strike resolution burned the opposing vampire this
    /// combat (Sword of the Archangel): engine-checked.
    WeaponBurnedOpponent,
    /// The card being played is a frenzy card played on a vampire of the
    /// clan the user controls (Meditative Grove): engine-checked.
    PlayedFrenzyOnYour(&'static str),
    /// The card being played is a frenzy card played on the actor
    /// (Predator's Transformation [abo] reflex; LSJ 20051113): engine-checked.
    PlayedFrenzyOnActor,
    /// The actor burned a minion of its controller's prey, in this combat
    /// or by this directed action (Predator's Transformation [ABO]; ANK
    /// 20220507: a directed action that ends up burning an ally counts).
    ActorBurnedPreyMinion,
    /// The target is the minion this card is on (Haven Uncovered):
    /// engine-checked.
    TargetIsHost,
    /// The acting minion is the one this card is on (Haven Uncovered's
    /// burn): engine-checked.
    ActorIsHost,
    /// The acting minion is this card's chosen one (Priority Contract):
    /// engine-checked.
    ActorIsChosen,
    /// The target minion is controlled by the user's prey.
    ControlledByPrey,
    /// The target card is controlled by the user's prey (Conquest of
    /// Humanity: "a location controlled by their prey").
    CardControlledByPrey,
    /// The user controls n ready vampires of the clan.
    ControlsReadyClan(&'static str, u8),
    /// The user controls a ready vampire of the clan with blood (Chantry).
    OwnReadyClanWithBlood(&'static str),
    /// The acting minion is a vampire.
    ActingVampire,
    /// The acting minion is a vampire with capacity n or more (Monastery
    /// of Shadows).
    ActingCapacityAtLeast(u8),
    /// A minion another Methuselah controls is attempting to block and needs
    /// intercept (KRCG News Radio's paid half).
    OthersAttemptNeedsIntercept,
    /// The acting minion is of the clan.
    ActingClan(&'static str),
    /// The current action is undirected; in a printed modifier, the action
    /// performed (Castellan).
    ActionUndirected,
    /// Not the first action announced in this minion phase (Channel 10).
    NotFirstActionOfPhase,
    /// The current action is a hunt, by an Anarch the user controls, that
    /// gained blood (The Anarch Free Press).
    AnarchHuntedYours,
    /// The user's ash heap holds a card requiring an Anarch, and their hand
    /// a card (Garibaldi-Meucci Museum).
    AnarchCardInAshAndHand,
    /// The combat is between an Anarch the user controls and another
    /// Anarch (Garibaldi-Meucci Museum).
    AnarchCombatWithYours,
    /// Another Methuselah controls this card ("Vampires controlled by
    /// another Methuselah can steal this location"): engine-checked.
    OthersControlThisCard,
    /// It is the turn of this card's controller ("during this location's
    /// controller's unlock phase", The Rack): engine-checked.
    CardControllersTurn,
    /// The target card is on a minion the user's prey controls (Anarch
    /// Troublemaker).
    TargetOnPreyMinion,
    /// The player has at least n pool (a granted action "that costs 1
    /// pool" is not declared unpayable).
    PoolAtLeast(u8),
    /// Combat: the round is at close range ("only usable at close range").
    RangeClose,
    RangeLong,
    /// Combat: the first round ("only usable during the first round").
    FirstRound,
    NotFirstRound,
    /// Combat: both combatants are still ready (Telepathic Tracking [AUS]).
    BothCombatantsReady,
    /// Combat: the end-of-round window with no press to continue standing
    /// and no replacement yet ("combat would end", spec §4.9.1).
    CombatWouldEnd,
    /// The actor is ready ("not usable by a vampire being burned or going
    /// to torpor").
    ActorReady,
    /// Damage is pending on the actor (prevention cannot be played with
    /// no damage to prevent, spec §4.5.1).
    DamagePending,
    /// Non-aggravated damage is pending on the actor (Soak).
    NonAggravatedDamagePending,
    /// Damage from gun strikes is pending on the actor (Kevlar Vest).
    GunStrikeDamagePending,
    /// Damage from any source but gun strikes is pending on the actor.
    NonGunDamagePending,
    /// This card was on the blocker when it blocked (Sniper Rifle: not an
    /// equipment equipped after the block, rules-qa 10.5).
    SourceHeldWhenBlocking,
    /// The card's controller controls the minion it is on ("if you control
    /// this minion", Vast Wealth; RTR 19951017).
    ControllerControlsHost,
    /// The card's host is ready ("while this Ravnos is ready", Shilmulo
    /// Tarot; "if the bearer is ready", Heart of Nizchetus).
    HostReady,
    /// No press to continue stands ("press, only usable to continue").
    NoContinuePress,
    /// The actor is the minion that blocked this action ("who has
    /// blocked").
    ActorHasBlocked,
    /// The actor holds a title of one of these classes ("requires a prince
    /// or justicar"; effective title: contested or yielded titles do not
    /// qualify).
    ActorTitleIn(&'static [TitleClass]),
    /// The actor holds no title of these classes ("if Valerius is … not a
    /// prince"; effective title).
    ActorTitleNotIn(&'static [TitleClass]),
    /// The actor belongs to one of these sects ("requires an Anarch").
    ActorSectIn(&'static [Sect]),
    /// The actor's capacity is at least n.
    ActorCapacityAtLeast(u8),
    /// The current action is a political action.
    IsPolitical,
    /// The polling step of a political action's referendum ("only usable
    /// during the polling step of a political action"; not the blood hunt,
    /// spec §3.7.5.4).
    PollingStep,
    /// The polling step of any referendum, blood hunts included ("during
    /// the polling step of any referendum", Aisha az-Zahra).
    AnyPollingStep,
    /// The referendum is the one the actor's political action called ("a
    /// referendum called by Anne").
    ActorCalledReferendum,
    /// Target vampire is not abstaining from this referendum.
    NotAbstaining,
    /// The actor is the acting minion ("during an action she performs").
    ActorActing,
    /// The current action succeeded, read after resolution; a fizzled
    /// action counts (ANK 20220218).
    ActionSuccessful,
    /// The action rescued a vampire from torpor: a successful rescue
    /// (Ritual Goblet), not one that fizzled.
    RescuedFromTorpor,
    /// The current action is directed; in a printed modifier, the action
    /// performed (Castellan).
    ActionDirected,
    /// The current action is directed at every Methuselah this card in
    /// play remembers (`CardInPlay::against`), its host's controller not
    /// counted (Shadow Cast [OBL]; LSJ 20100527, rules-qa 24.5).
    DirectedAtRemembered,
    /// The actor diablerised a vampire in this action: as the acting minion
    /// or as the blocker of a leave-torpor action (rules-qa 16.2).
    ActorDiablerised,
    /// The acting minion belongs to another Methuselah: an action of one's
    /// own minions is undirected (LSJ 20030214-2).
    ActingOtherController,
    /// The acting minion is the player's and is not the actor ("another
    /// Anarch you control", Aline Gädeke).
    OtherActingControlled,
    /// The acting minion is the player's, the actor or not ("a minion you
    /// control … for the current action", Maila).
    OwnActing,
    /// "Cannot be used during an action" (Heidelberg Castle): no action,
    /// combat or referendum under way — the blood hunt and the damage after
    /// an action's resolution are still in it (ANK 20171124, LSJ
    /// 20090722-2).
    NoActionUnderway,
    /// The player controls n ready vampires or more.
    ControlsReadyVampires(u8),
    /// The acting minion belongs to one of these sects.
    ActingSectIn(&'static [Sect]),
    /// The Methuselah the action is directed at has cards in hand.
    DirectedHandNotEmpty,
    /// Target minion is controlled by the Methuselah the action is
    /// directed at ("controlled by the target Methuselah", Alicia Cortez).
    ControlledByDirectedSeat,
    /// Target minion is not the vampire whose ability it is ("another ready
    /// Gargoyle you control", Alabástrom).
    NotHost,
    /// The minion attempting to block is the player's and needs intercept
    /// ("give a vampire you control +1 intercept", Oskar Anasov; rules-qa
    /// 3.2).
    YourAttemptNeedsIntercept,
    /// The acting minion is of this clan and is not the actor ("another
    /// Nosferatu", Daria).
    OtherActingClan(&'static str),
    /// The host is unlocked (a lock cost: "can lock to …").
    HostUnlocked,
    /// It is the player's turn ("during your turn", Kalinda).
    OwnTurn,
    /// The actor must hunt: no blood, or made to (Restricted Vitae's
    /// special hunt).
    ActorMustHunt,
    /// The player has a discard phase action left.
    DiscardPhaseActionLeft,
    /// The player has played a master card in the current master phase
    /// ("after playing a master card", Nonu Dis).
    MasterCardPlayed,
    /// The player's ash heap holds a library card ("a library card in your
    /// ash heap": Lenelle, Mora) — burned vampires aside (`Seat::ash_crypt`).
    AshNotEmpty,
    /// The current action requires an Anarch or makes its vampire Anarch
    /// (CrimethInc.; `ActionCtx::requires_anarch`).
    ActionRequiresAnarch,
    /// The player's library holds at least n cards (Liquidation: not
    /// playable with fewer than 7, LSJ 20070901).
    LibraryAtLeast(u8),
    /// The player's ash heap holds at least n cards, burned vampires
    /// included ("remove seven cards in your ash heap", Marchesa Liliana;
    /// rules-qa 42.7).
    AshAtLeast(u8),
    /// No votes or ballots have been cast yet in this referendum.
    NoVotesCast,
    /// The referendum was called by a titled vampire of this sect (T.J.:
    /// "a referendum called by a titled Camarilla vampire"), read now.
    CallerTitledOfSect(Sect),
    /// The action's referendum passed (Voter Captivation).
    ReferendumPassed,
    /// Target minion has cast votes or ballots in this referendum, still
    /// standing (Scalpel Tongue).
    HasCastVotes,
    /// Target minion is younger than the acting minion (lower capacity).
    Younger,
    /// The actor is not voting in favor of the referendum (Protected
    /// District: votes against cannot join a cast in favor).
    ActorNotVotingFor,
    /// Any one of these requirement lists holds in full ("only usable
    /// during a bleed action against you or during the polling step").
    AnyOf(&'static [&'static [Requirement]]),
    /// The actor has no card in play of this kind ("a minion can have only
    /// one vehicle"): an equip is not offered to a minion already holding
    /// one.
    ActorWithout(Holding),
    /// The target minion has no card in play of this kind ("a vampire can
    /// have only one archetype").
    TargetWithout(Holding),
    /// The target holds a title of one of these classes (effective title:
    /// "the title held by a prince", Valerius).
    TargetTitleIn(&'static [TitleClass]),
    /// The target's title is under contest (Ephor: "force any vampire to
    /// yield a contested title").
    TargetTitleContested,
    /// The prey's ash heap holds two copies of a card (Giovanni del
    /// Georgio).
    PreyAshHasPair,
    /// The minion is an infernal vampire ("requires an infernal vampire").
    ActorInfernal,
    /// This card in play was played in mode n (Condemnation: Mute's [DAI],
    /// its third).
    CardMode(u8),
    /// The player has n or more vampires in their uncontrolled region.
    UncontrolledAtLeast(u8),
    /// The player's ash heap holds a library card (The Sargon Fragment).
    OwnAshNotEmpty,
    /// Their prey's ash heap holds a library card (The Erciyes Fragments).
    PreyAshNotEmpty,
    /// The player controls no Great Beast (LSJ 20101130).
    NoGreatBeastControlled,
    /// No card is on this card in play ("only 1 card can be on this card
    /// at a time").
    StoreEmpty,
    /// "Once each turn" on a rider: fires once during each Methuselah's
    /// turn per copy (Rebel), tracked with the turn's ability uses.
    OncePerTurn,
    /// The target vampire is of none of these clans ("a non-Salubri
    /// vampire").
    TargetClanNotIn(&'static [&'static str]),
    /// "Cannot be put on a vampire with superior <Discipline>" (a
    /// Discipline master card), read live (lower-case tag).
    TargetNotSuperior(&'static str),
    /// The target is another minion than the one acting ("another ready
    /// minion").
    OtherThanActor,
    /// This card in play entered before the current turn ("during your next
    /// minion phase", Puppet Master [DOM]).
    EnteredBeforeThisTurn,
    /// This card in play entered before the current action ("during any
    /// action after this one", Under My Skin).
    EnteredBeforeThisAction,
    /// The card in play whose ability this is is the player's ("… you
    /// control can …"). Engine-checked.
    YouControlThisCard,
    /// Target minion is controlled by the player's predator or prey (Pit of
    /// Contemplation).
    ControlledByPredatorOrPrey,
    /// Target vampire's capacity is below the counters on the card in play
    /// whose ability this is (Pit of Contemplation). Engine-checked.
    TargetCapacityBelowCounters,
    /// The target minion carries corruption counters, anyone's (Aniel).
    TargetCorrupted,
    /// Played at the maneuver, only to get to close range: the range is long
    /// (Slam [POT]); any other step, no condition.
    ManeuverToClose,
    /// The opposing minion in combat has a weapon in play (Voracious Vermin
    /// [ANI]: no weapon to burn, the superior version cannot be chosen;
    /// owner, 2026-09-26: rules-qa 22.3).
    OpposingWeapon,
    /// The actor is unlocked ("a ready unlocked vampire").
    ActorUnlocked,
    /// The actor may use equipment: not barred this combat (Form of the
    /// Bat) nor by its own card (Bone Shambler).
    ActorMayUseEquipment,
    /// Combat: the actor successfully inflicted more damage from its strikes
    /// on the opposing vampire this round than it took from theirs (Disarm;
    /// environmental damage does not count, LSJ 20010812).
    InflictedMoreThisRound,
    /// The current action is not a bleed ("not usable during a bleed
    /// action").
    NotBleed,
    /// The acting minion is younger than the actor (Redirection [dom]: "a
    /// younger vampire is bleeding you").
    ActingYounger,
    /// The current action recruits an ally or employs a retainer of one of
    /// these subtypes (Paths in Two Worlds: "a wraith or zombie").
    RecruitingSubtype(&'static [crate::library::Label]),
    /// The host of this card is the minion attempting to block (Dogged
    /// Pursuit's +1 intercept). Engine-checked.
    HostAttempting,
    /// The acting minion is the one that played this card (`CardInPlay::by`;
    /// Melange's "this reacting vampire"). Engine-checked.
    ActingIsBy,
    /// The current bleed is against the controller of this card's host
    /// (Melange). Engine-checked.
    BleedAgainstHostController,
    /// Combat: the opposing minion holds no card in play of this kind ("a
    /// vampire can have only one Disarm").
    OpponentWithout(Holding),
    /// Target: a minion in the combat, or a retainer on one, with damage
    /// pending on it — none to prevent, no prevention (LSJ 20001114); a
    /// Fortitude card cannot reach damage a Fortitude-proof strike
    /// inflicted (LSJ 20050628). Engine-checked.
    TargetInCombatWithDamage,
    /// As `TargetInCombatWithDamage`, non-aggravated damage pending (Randall:
    /// "prevent 1 non-aggravated damage inflicted on any other minion in
    /// combat").
    TargetInCombatWithNormalDamage,
    /// The card being played is of this type (Hide the Mind: "a combat
    /// card", "a reaction card").
    PlayedCardType(CardType),
    /// The card being played requires this Discipline as played — the mode
    /// used, not the card's other Disciplines (PIB 20130704).
    PlayedRequiresDiscipline(&'static str),
    /// On a printed cost reduction: the card played or put in play matches
    /// (`PrintedCtx::card`).
    PlayedCard(CardFilter),
    /// Card target: on the minion in combat with the ability's host (a
    /// strike naming equipment, Heroic Might). Engine-checked.
    TargetOnOpposingCombatant,
    /// The current action was blocked (Freak Drive [FOR]).
    ActionBlocked,
    /// The seat has not played a superior Veil the Legions this turn.
    NoSuperiorVeilThisTurn,
    /// The target card is on the actor ("an animal retainer employed by
    /// this vampire", Pack Alpha [ANI]).
    TargetOnActor,
    /// The combatant has damage pending from the opposing minion's strikes
    /// (Rolling with the Punches [FOR]: "cannot be used if there is no
    /// damage to prevent", LSJ 20001114).
    StrikeDamagePending,
    /// No reaction card was played during the current action — a canceled
    /// one still was (Perfectionist; LSJ 20070330).
    NoReactionPlayed,
    /// The card's host is not in the combat ("to end a combat involving
    /// another minion you control", Saulot's Guiding Wisdom).
    HostOutOfCombat,
    /// The host of the card in play is of this clan ("this Ravnos can
    /// strike").
    HostClan(&'static str),
    /// The minion that blocked the current action holds a title of one of
    /// these classes ("blocked by a prince or an archbishop").
    BlockerTitleIn(&'static [TitleClass]),
    /// The host of the card in play has this discipline (lower case: any
    /// level; upper case: superior) — "the bearer with superior Auspex".
    HostDiscipline(&'static str),
    /// Printed-modifier predicates (`GameState::printed_mod`), read on the
    /// vampire whose text it is, the modified minion ("actor"), and the
    /// opposing minion — the acting minion for a block attempt, the
    /// opposing combatant in combat. "While this vampire is ready".
    ThisVampireReady,
    /// The opposing minion is of this clan ("in combat with a Brujah").
    OpponentClan(&'static str),
    /// The opposing minion is younger / older than the modified minion.
    OpponentYounger,
    OpponentOlder,
    /// The opposing minion is an ally or a younger vampire ("in combat
    /// with an ally or younger vampire", Ragnar Nordstrom).
    OpponentAllyOrYounger,
    /// The opposing minion holds a title (effective title).
    OpponentTitled,
    /// The modified minion carries corruption counters of the printed
    /// vampire's Methuselah ("minions with any of your corruption
    /// counters", Faruq Abd al-Qadir).
    ActorCorruptedBySource,
    /// The actor (the modified minion, for a printed modifier) is of this
    /// clan ("Brujah get +1 strength", "Hecata you control can …").
    ActorClan(&'static str),
    /// The vampire's controller has at most n pool ("while you have 9 or
    /// fewer pool").
    PoolAtMost(u8),
    /// The vampire's controller controls an uncontested location / none.
    ControlsLocation,
    ControlsNoLocation,
    /// The vampire's controller controls a vampire in play of this name
    /// ("while you control Mithras").
    ControlsVampireNamed(&'static str),
    /// The vampire's controller controls no ready unique vampire (a crypt
    /// card, not a created one) of this clan older than it (The Guardian:
    /// "unless you control a ready unique older Nosferatu").
    ControlsNoReadyOlderOfClan(&'static str),
    /// Printed-modifier predicate: the vampire's controller controls no
    /// ready vampire of this clan (a slave's master clan: "Tremere
    /// antitribu slave", rulebook "Slave").
    ControlsNoReadyOfClan(&'static str),
    /// The current action is the player's, blocked, and no slave took its
    /// combat yet (rulebook "Slave").
    NoSlaveInstead,
    /// Printed-modifier predicate: the vampire claims a title of these
    /// classes, its sect not read (Karen Suadela's "while a prince": her
    /// immunity keeps her sect through a Fall).
    ActorClaimsTitleIn(&'static [TitleClass]),
    /// Printed-modifier predicate: the vampire has no title (Gerald
    /// Windham: "while Gerald has no title").
    ActorUntitled,
    /// Printed-modifier predicate: the card is played as a reaction (Scout
    /// Youngwood: "reaction cards cost her +1 blood").
    PlayedAsReaction,
    /// Printed-modifier predicate: the card is played as a combat card
    /// (Adana de Sforza: "combat cards cost Adana 1 fewer blood"); one of
    /// several types is the type it is played as (PIB 20150924-2).
    PlayedAsCombat,
    /// The actor could have performed the action under way, every effect
    /// used on it so far included (Mask of a Thousand Faces): not one an
    /// equipment or a card in play grants (LSJ 20020926), its action card
    /// within its requirements, the modifiers the acting minion played
    /// within its clan and Disciplines (RTR 19980623, RTR 20030519).
    MaskCapable,
    /// The card's controller controls no other copy of its host vampire:
    /// taking control of it would be a self-contest, never chosen (advanced
    /// rules 1.13.1; Puppet Master).
    HostNoCopyOfControllers,
    /// The target is not immune to Gehenna events (Blood Trade's giver,
    /// Karen Suadela).
    NotGehennaImmune,
    /// No ready vampire is Black Hand, any Methuselah's (Fall of the
    /// Sabbat).
    NoReadyBlackHand,
    /// No vampire of this name is ready, any Methuselah's (Valkyrie:
    /// "unless Brunhilde is ready").
    NoReadyNamed(&'static str),
    /// The Methuselah controls a locked minion ("if you control a locked
    /// minion", Elen Kamjian).
    ControlsLockedMinion,
    /// Target minion or card is controlled by another Methuselah (a minion
    /// does not enter combat with one its controller controls).
    NotControlled,
    /// Target card in play is of this kind.
    TargetCard(CardFilter),
    /// Target card is on a ready minion ("from another ready minion").
    TargetOnReadyMinion,
    /// The acting minion could hold the target card: none of its "only
    /// one" limits reached (a second vehicle).
    ActorMayHoldTarget,
    /// Target minion is locked ("a locked vampire", Nathaniel Bordruff).
    TargetLocked,
    /// Target minion is a vampire, not an ally.
    TargetVampire,
    /// The current action is directed at a minion the player controls
    /// (Szlachta Bodyguard).
    DirectedAtYourMinion,
    /// As `DirectedAtYourMinion`, an ally (Screamer).
    DirectedAtYourAlly,
    /// The acting minion is controlled by the player's predator or prey
    /// (Screamer).
    ActingByPredatorOrPrey,
    /// In another Methuselah's minion phase (checked by the engine, which
    /// knows the phase: `Engine::in_others_minion_phase`).
    OthersMinionPhase,
    /// This card's minion — an ally's own card — is locked.
    CardLocked,
    /// The card being played is a strike card of the minion opposing the
    /// ability's host in combat (checked by the engine, which holds the
    /// play: `Engine::played_strike_by_opponent`).
    PlayedStrikeByOpponent,
    /// The card being played is a strike card or a grapple card of the
    /// minion opposing the actor (Charismatic Aura): engine-checked.
    PlayedStrikeOrGrappleByOpponent,
    /// The user controls the prince of this city ("if you control the
    /// Prince of Paris", The Louvre, Paris): its effective title.
    ControlsPrinceOf(&'static str),
    /// In the player's own minion phase, any time of it, actions included
    /// (checked by the engine: `Engine::in_own_minion_phase`; rules-qa
    /// 26.5).
    OwnMinionPhase,
    /// Target minion is controlled by the player's predator.
    ControlledByPredator,
    /// Target minion is controlled by the active Methuselah ("a vampire
    /// that Methuselah controls", Owain Evans in any unlock phase).
    ControlledByActive,
    /// No combat is under way ("except during combat", War Ghoul).
    NotInCombat,
    /// The current block has not been answered by continuing the action as
    /// if unblocked already (Crypt's Sons: once per block).
    NotContinued,
    /// No damage resolution step is under way (Vagabond Mystic, LSJ
    /// 20001111).
    NotInDamageResolution,
    /// This card has at least n life: a retainer's counters, an ally's own
    /// life.
    LifeAtLeast(u8),
    /// The actor is neither attempting to block nor the blocker (Underbridge
    /// Stray "if it is not blocking").
    ActorNotBlocking,
    /// The current action recruits a ghoul ally requiring a Tzimisce
    /// (Szlachta Assistant).
    RecruitingTzimisceGhoul,
    /// This card in play is its superior version (entered with mode 1).
    SuperiorVersion,
    /// This card in play is its basic version (entered with mode 0).
    BasicVersion,
    /// Target minion is another copy of the actor ally ("another copy of
    /// this ally you control", Bone Shambler).
    TargetOtherCopy,
    /// Target minion is unlocked.
    TargetUnlocked,
    /// Target minion is an ally.
    TargetAlly,
    /// The target card is on a vampire (Far Mastery: "a retainer
    /// controlled by another vampire").
    TargetOnVampire,
    /// Target minion is an ally with fewer life than its starting life
    /// (Vagabond Mystic; advanced rules 5.5.2).
    TargetAllyHurt,
    /// No block is being attempted in the current action, nor has one
    /// succeeded ("fail an action" after the as-announced window, ANK
    /// 20260316).
    NoBlockAttempt,
    /// Target minion is an ally or a vampire younger than the acting
    /// minion ("an ally or younger vampire", Adisa).
    AllyOrYounger,
    /// The actor's opposing combatant is an ally of this subtype ("in
    /// combat with a werewolf", Rowena).
    OpponentSubtype(crate::library::Label),
    /// The modified minion is an ally of this subtype ("werewolves you
    /// control get +1 bleed", Jacob Fermor): a printed predicate.
    ActorSubtype(crate::library::Label),
    /// The actor's opposing combatant is a vampire: a card naming the
    /// opposing vampire has no object in an ally (Taste of Vitae; advanced
    /// rules 5.5.1).
    OpponentVampire,
    /// The opposing minion is a vampire not older than the actor ("not
    /// usable in combat with an ally or an older vampire", Alpha Glint).
    OpponentVampireNotOlder,
    /// The actor's opposing combatant is not ready (Hell-for-Leather
    /// [obt]: "only usable if … the opposing minion is not").
    OpponentNotReady,
    /// The target Methuselah is about to be ousted, waiting on a Life
    /// Boon (`Seat::oust_pending`).
    TargetSeatOusting,
    /// The user can give the target Methuselah enough pool to keep them
    /// in the game (at least 1 above what they owe) and keep 1 of their
    /// own (Life Boon).
    CanSaveTarget,
    /// The target Methuselah this card remembers is the active one (Life
    /// Boon: "during each of their unlock phases").
    BoonTargetActive,
    /// The user's vampire in combat faces an opposing strike attempting to
    /// inflict aggravated damage on it, effective at the current range
    /// (Rötschreck: RTR 19980928; a strike only, ANK 20220114; dodged or
    /// not, LSJ 20011005).
    OwnCombatantFacesAggravatedStrike,
    /// The target minion is in the combat under way.
    TargetCombatant,
    /// The current combat's acting side is a vampire the user controls
    /// (Powerbase: Savannah: "combat involving an acting vampire you
    /// control"): the minion considered acting, Deep Song's target
    /// included (ANK 20230814).
    ActingCombatantYourVampire,
    /// The target minion has at most n blood (Jake Washington: "a ready
    /// vampire with no blood").
    BloodAtMost(u8),
    /// The target vampire committed diablerie since the user's last turn
    /// ended — the user's current turn counts (Carlton Van Wyk; LSJ
    /// 20020429).
    TargetDiablerieSinceYourLastTurn,
    /// Target minion, or vampire in the uncontrolled region, is of this
    /// clan ("a Gangrel in your uncontrolled region", Thing).
    TargetClan(&'static str),
    /// The acting minion is controlled by the actor's predator
    /// (Instinctive Reaction, My Enemy's Enemy [AUS]).
    ActingByPredator,
    /// More than two Methuselahs are left in the game ("not usable if
    /// there are only two Methuselahs left", My Enemy's Enemy [AUS]).
    MoreThanTwoLeft,
    /// The minion attempting to block is of one of these sects
    /// (Organized Resistance: "an Anarch you control gets +1 intercept").
    AttemptingSectIn(&'static [crate::crypt::Sect]),
    /// The minion attempting to block is of this clan (Market Square:
    /// "give an Assamite you control +1 intercept").
    AttemptingClan(&'static str),
    /// "A vampire who does not have Fortitude attempts to block" (The Kiss
    /// of Ra): a vampire attempting, without the Discipline, read live.
    AttemptingVampireWithout(&'static str),
    /// The bleed's current target has n or fewer pool (Foreshadowing
    /// Destruction [DOM]: "if the target Methuselah has 9 or fewer pool").
    BleedTargetPoolAtMost(u8),
    /// Target minion is of one of these subtypes ("a wraith or zombie
    /// ally", Shadow Sentinel [OBL]).
    TargetSubtypeIn(&'static [crate::library::Label]),
    /// The card being played is a minion card (not a master: Direct
    /// Intervention).
    PlayedCardIsMinionCard,
    /// The card being played is played by the actor's predator or prey
    /// (Wash).
    PlayedByPredatorOrPrey,
    /// The actor has at least n blood (an effect burning it: ANK
    /// 20210226, "cannot be played if the minion cannot afford").
    ActorBloodAtLeast(u8),
    /// The actor can attempt to block the current action, its lock aside:
    /// its Methuselah may block and has not declined, it is a block
    /// candidate — or it is already attempting (engine-checked:
    /// `Engine::can_attempt_block`; ANK 20180623, LSJ 20010714-2).
    CanAttemptBlock,
    /// As `CanAttemptBlock`, "ignoring the normal prey, predator or target
    /// restrictions for blocking actions" (Eagle's Sight [AUS]): any
    /// Methuselah but the acting one; every other requirement stands — a
    /// barred blocker, a Methuselah who declined (RTR 19950413, RTR
    /// 20020501). Engine-checked: `Engine::can_attempt_block_anyone`.
    CanAttemptBlockAnyone,
    /// The actor may be locked or unlocked ("usable by a locked vampire",
    /// Shadow Sentinel [OBL]: a permission, not a requirement).
    MayBeLocked,
    /// Seat target: the player's prey ("your prey's ash heap", Shroud of
    /// Decay [OBL]).
    TargetPrey,
    /// Seat target: at least n cards in their ash heap, burned vampires
    /// included ("remove 7 cards in your prey's ash heap", Shroud of Decay
    /// [OBL]; rules-qa 42.7).
    TargetAshAtLeast(u8),
    /// Seat target: the predator of the actor's predator (My Enemy's
    /// Enemy [AUS]).
    PredatorsPredator,
}

/// A reaction's gift to its minion in the combat a block begins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockRider {
    /// "1 optional maneuver during the resulting combat" (Spirit's Touch).
    Maneuver,
    /// "1 optional maneuver during the first round of the resulting
    /// combat" (Mind of the Wilds).
    ManeuverFirstRound,
    /// "The blocking minion cannot strike: combat ends" (Mind of the
    /// Wilds): for the rest of the action, any round of any combat (LSJ
    /// 20100308) — `ActionCtx::no_combat_ends`.
    OpponentNoCombatEndsThisAction,
    /// "Can prevent n damage during the first round of the resulting
    /// combat" (Precognition).
    PreventFirstRound(u8),
    /// "Can strike: combat ends during the first round of the resulting
    /// combat" (Night Terrors [OBF][PRE]): `Choice::StrikeCombatEnds`.
    CombatEndsFirstRound,
    /// "Neither combatant can strike during the first round of the
    /// resulting combat" (One With the Land [PRO]).
    NoStrikesFirstRound,
    /// "Cannot use equipment during the resulting combat" (Form of the
    /// Bat): its equipment's abilities are not usable.
    NoEquipment,
    /// "Can burn n blood to unlock after block resolution" (Truth in
    /// Darkness [OBL]): offered after the combat.
    UnlockAfterForBlood(u8),
    /// "1 optional press during that combat" (Hunter's Mark).
    Press,
    /// "The opposing minion cannot strike: combat ends during the first
    /// round of that combat" (Hunter's Mark [CEL][THA]).
    OpponentNoCombatEndsFirstRound,
    /// "Before range is determined during the first round of the resulting
    /// combat, this vampire can gain +n strength that combat" (Show of
    /// Force): `Choice::GainStrength`.
    StrengthBeforeRange(u8),
    /// "If this vampire is blocked, they can prevent n damage during the
    /// resulting combat" (Beast Meld [ani][pro]): any round of it.
    PreventInCombat(u8),
    /// "This Anarch gets +n strength during that combat" (Make the Misere
    /// [pot]).
    Strength(u8),
    /// "At the end of that combat, if this vampire is ready and the opposing
    /// vampire is not, add n blood to a `clan` vampire in your uncontrolled
    /// region" (Hunting the Beast [AUS]): chosen then.
    UncontrolledBloodIfWon { clan: &'static str, n: u8 },
}

/// How a card in play reduces its controller's costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CostSource {
    /// Lock: an action card a vampire plays costs 1 blood less (The Line).
    Line,
    /// With the Edge, lock: a card costs 2 pool or blood less (The Shard).
    Shard,
    /// Its counters pay the blood of an action card a vampire of this clan
    /// plays (Ravnos Carnival).
    Carnival(&'static str),
    /// Lock: its counters pay an equip's blood or pool (Ravnos Cache).
    Cache,
    /// Its counters pay the blood or pool of an equipment an Anarch of its
    /// controller's equips, no lock (Hackerspace).
    Hackerspace,
    /// Burned: a combat card its host plays costs 1 blood less (Focus the
    /// Blood).
    FocusBlood,
}

/// Whom a conditional bonus reads its condition on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum On {
    /// The minion playing the card.
    Actor,
    /// The acting minion.
    Acting,
}

/// How many points a referendum allocates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Points {
    Fixed(u8),
    /// "X is the number of Methuselahs in the game" (Conservative
    /// Agitation).
    Methuselahs,
    /// "Allocate n of their pool" (Parity Shift): n points of the chosen
    /// Methuselah's pool; a shortfall at resolution is re-allocated by the
    /// caller within the terms (LSJ 20010606).
    PoolOfChosen(u8),
}

/// Which Methuselahs may receive points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Among {
    /// Any Methuselah, the caller included.
    Any,
    /// Other Methuselahs only.
    Others,
    /// Any Methuselah but the one chosen by the previous term.
    AnyNotChosen,
    /// Other Methuselahs, the chosen one excluded (Camarilla's Iron Fist).
    OthersNotChosen,
}

/// Which Methuselahs a `ChooseSeat` term may pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SeatFilter {
    Any,
    /// "A Methuselah who has more pool than you do" (Parity Shift).
    MorePoolThanCaller,
}

/// One term of a referendum, chosen stepwise by the caller once the action
/// succeeds (rulebook "The Referendum" step 1; rules-qa 3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Term {
    /// Allocate points among at least `min_seats` Methuselahs (a point
    /// total may exceed a Methuselah's pool: KRC rulings).
    Allocate {
        points: Points,
        among: Among,
        min_seats: u8,
    },
    /// Choose between `min` and `max` Methuselahs.
    ChooseSeats { min: u8, max: u8 },
    /// Choose one Methuselah (the "chosen" one for later terms).
    ChooseSeat(SeatFilter),
    /// Choose between `min` and `max` minions meeting the filter.
    ChooseMinions {
        filter: &'static [Requirement],
        min: u8,
        max: u8,
    },
    /// Choose X minions meeting the filter, X the political card's (My Kin
    /// Against the World); fewer if no more qualify.
    ChooseMinionsX { filter: &'static [Requirement] },
    /// Choose a clan.
    ChooseClan,
    /// Choose a card in play matching the filter ("choose a location",
    /// Disputed Territory): a choice, never a target (rules-qa 38.6).
    ChooseCard(CardFilter),
    /// "Choose a Methuselah or a <card>. If this acting vampire is a
    /// <title>, you can choose both instead" (Cold War): one of the two,
    /// then the other too with one of the `both` titles.
    SeatOrCard {
        card: CardFilter,
        both: &'static [TitleClass],
    },
    /// Allocate `per` points for each minion chosen by the previous term
    /// among one or more Methuselahs, locations and equipment
    /// (Revolutionary Council: "2X points").
    AllocatePerChosenWithCards { per: u8 },
    /// "Distribute n blood from the blood bank among one or more <filter>
    /// vampires" (Dmitra Ilyanova): n picks, one blood each, a vampire
    /// picked again as often as wished.
    DistributeBlood {
        points: u8,
        filter: &'static [Requirement],
    },
}

/// What a passed referendum does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RefEffect {
    /// Each Methuselah burns 1 pool per point allocated.
    BurnAllocated,
    /// Each Methuselah burns 1 pool per minion they control (`locked_only`:
    /// per locked minion, counted when the effect applies).
    BurnPerMinion { locked_only: bool },
    /// Every vampire in play, in torpor too, whose current bleed is `n` or
    /// more is burned (Justicar Retribution), read as the referendum
    /// passes.
    BurnVampiresBleedAtLeast(u8),
    /// The calling vampire and the chosen minions unlock (My Kin Against
    /// the World).
    UnlockCallerAndChosen,
    /// Each chosen Methuselah burns `base` pool, plus `young_bonus` if they
    /// control a ready vampire with capacity `young_cap` or less.
    BurnChosen {
        base: u8,
        young_bonus: u8,
        young_cap: u8,
    },
    /// As `BurnAllocated`, and the chosen Methuselah gains n pool.
    BurnAllocatedChosenGains(u8),
    /// The allocated points move from the chosen Methuselah's pool to the
    /// recipients (Parity Shift).
    MoveAllocatedFromChosen,
    /// Each Methuselah gains 1 pool per vampire of the chosen clan they
    /// control.
    GainPerClanVampire,
    /// Each ready Anarch gains 1 blood; each Methuselah controlling an
    /// Anarch gains 1 pool.
    AnarchSalon,
    /// The chosen vampire goes to its controller's uncontrolled region.
    Banish,
    /// The chosen minions cannot play reaction cards, block or cast votes
    /// or ballots this turn.
    Expel,
    /// Burn the card in play whose granted action called the referendum
    /// ("call a referendum to burn this card").
    BurnSource,
    /// The political action card is put on the chosen vampire, if still in
    /// play (Toreador Justicar: its title card).
    PutOnChosen,
    /// The political action card is put in play, its caller's (Raising the
    /// Portcullis).
    PutInPlay,
    /// The chosen Methuselah takes control of the chosen card (Disputed
    /// Territory): one on a minion moves onto one of theirs (RTR
    /// 19960112), not when it stays with the same Methuselah (LSJ
    /// 19971002).
    ChosenTakesChosenCard,
    /// Each chosen minion is locked, each Methuselah burns 1 pool per point
    /// allocated, each card allocated a point is burned (Revolutionary
    /// Council).
    LockChosenBurnAllocated,
    /// Each Methuselah can choose a ready vampire they control (as it
    /// passes, `RefStep::Choose`), gains pool equal to its capacity, then
    /// burns n pool (Ancient Influence).
    GainChosenCapacityBurn(u8),
    /// Each Methuselah can choose a ready vampire they control, gains n
    /// pool, then burns pool equal to the capacity of their predator's
    /// chosen vampire (Reins of Power).
    GainBurnPredatorsChosenCapacity(u8),
    /// Each Methuselah gains n pool for each vampire they control with
    /// capacity `cap` or more (Political Stranglehold).
    GainPerVampireCapacityAtLeast { cap: u8, n: u8 },
    /// The chosen minion is removed from the game (Permanent Vacation).
    RemoveChosen,
    /// The chosen vampire takes control of the chosen card: it moves onto
    /// them (Confiscation).
    ChosenVampireTakesChosenCard,
    /// Each chosen Methuselah chooses up to n library cards in their ash
    /// heap and shuffles them into their library (Reinforcements).
    ChosenShuffleFromAsh(u8),
    /// The chosen Methuselah burns n pool and the chosen card is burned
    /// (Cold War).
    ChosenBurnsPoolCardBurned(u8),
    /// The calling vampire's controller gains n pool (Black Forest Base).
    CallerGainsPool(u8),
    /// "You burn the Edge to remove that vampire's title and burn n pool
    /// from his or her controller" (Patsy): with the Edge still the
    /// caller's controller's at resolution; without it, nothing.
    BurnEdgeToUntitle(u8),
    /// Each blood the terms distributed (`DistributeBlood`) to a vampire
    /// still ready, from the bank; what finds no room is lost.
    BloodToChosenEach,
    /// "+n hand size until the end of the game" for the caller (Benjamin
    /// Rose), each passed referendum adding n.
    CallerHandSizeForGame(u8),
    /// Each chosen Methuselah burns `base` pool, plus `bonus` if they
    /// control a ready vampire with capacity `cap` or more (Empires Fall).
    BurnChosenPlusIfBig { base: u8, bonus: u8, cap: u8 },
}

impl RefEffect {
    /// Each Methuselah chooses a ready vampire they control once the
    /// referendum passes (rules-qa 49.1).
    pub fn chooses_vampires(self) -> bool {
        matches!(
            self,
            RefEffect::GainChosenCapacityBurn(_) | RefEffect::GainBurnPredatorsChosenCapacity(_)
        )
    }
}

/// A political action card's referendum: its terms, its effect, and the
/// votes it grants while it lasts ("in this referendum, each ready
/// Toreador gets +1 vote", Toreador Justicar).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReferendumDef {
    pub terms: &'static [Term],
    pub effect: RefEffect,
    pub grants: &'static [VoteGrant],
    /// "During that referendum, non-Anarch titles are worth -1 vote" (Fee
    /// Stake: Boston).
    pub non_anarch_titles_minus: bool,
    /// "During this referendum, non-Sabbat vampires cannot cast votes or
    /// ballots" (Cardinal Benediction): only this sect's vampires vote.
    pub only_sect: Option<Sect>,
}

/// Whom a referendum's vote penalty reaches (`ReferendumCtx::penalties`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Penalized {
    /// Vampires not of the sect (Camarilla Conclave: "each non-Camarilla
    /// vampire").
    NotSect(Sect),
    /// Vampires not following the path (Absolute Tyranny [POT][PRE]).
    NotPath(&'static str),
}

impl Penalized {
    pub fn reaches(self, st: &GameState, m: MinionId) -> bool {
        match self {
            Penalized::NotSect(sect) => st.sect_of(m) != Some(sect),
            Penalized::NotPath(path) => !st.follows(m, path),
        }
    }
}

/// The vampires a vote grant reaches, read live while the referendum lasts
/// (a vampire that qualifies when it casts gets them; one that has cast
/// when the grant is made adds them to its cast).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Voters {
    /// "Each ready Toreador" (Toreador Justicar), "each Lasombra you
    /// control" (Power Structure).
    Clan(&'static str),
    /// "Each titled Camarilla vampire you control" (Elysium: The Palace of
    /// Versailles): effective title and sect.
    TitledSect(Sect),
}

/// +n votes to each vampire `voters` names (of the granting Methuselah's
/// when the grant comes from a card in play).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VoteGrant {
    pub voters: Voters,
    pub n: u8,
}

impl Voters {
    pub fn reach(self, st: &GameState, m: MinionId) -> bool {
        let v = &st.minions[m.index()];
        match self {
            Voters::Clan(c) => v.clan() == Some(c),
            Voters::TitledSect(sect) => {
                st.sect_of(m) == Some(sect) && st.title_of(m) != Title::None
            }
        }
    }
}

impl ReferendumDef {
    /// Can the terms be met (LSJ 20100129: a political action cannot be
    /// played if the conditions for its terms cannot be met)? A choice of
    /// Methuselah is feasible only if some choice leaves the following
    /// allocation feasible.
    pub fn feasible(&self, st: &GameState, seat: SeatId, actor: MinionId) -> bool {
        if st.seats[seat.index()].ousted {
            return false;
        }
        let living = st.clockwise_from(seat);
        let mut i = 0;
        while i < self.terms.len() {
            match self.terms[i] {
                Term::ChooseSeat(filter) => {
                    let next = self.terms.get(i + 1).copied();
                    let ok = living.iter().any(|&c| {
                        seat_filter_holds(filter, st, seat, c)
                            && match next {
                                Some(Term::Allocate {
                                    among, min_seats, ..
                                }) => recipients(st, seat, among, Some(c)).len() as u8 >= min_seats,
                                _ => true,
                            }
                    });
                    if !ok {
                        return false;
                    }
                    if matches!(next, Some(Term::Allocate { .. })) {
                        i += 1;
                    }
                }
                Term::Allocate {
                    among, min_seats, ..
                } => {
                    if (recipients(st, seat, among, None).len() as u8) < min_seats {
                        return false;
                    }
                }
                Term::ChooseSeats { min, .. } => {
                    if (living.len() as u8) < min {
                        return false;
                    }
                }
                Term::ChooseMinions { filter, min, .. }
                | Term::DistributeBlood {
                    filter,
                    points: min,
                } => {
                    // A distribution needs one recipient at least.
                    let min = if matches!(self.terms[i], Term::DistributeBlood { .. }) {
                        min.min(1)
                    } else {
                        min
                    };
                    let n = (0..st.minions.len())
                        .map(|i| MinionId(i as u32))
                        .filter(|&m| {
                            st.minions[m.index()].present()
                                && filter.iter().all(|&r| {
                                    holds_for_minion(r, st, seat, None, Some(actor), m, None)
                                })
                        })
                        .count();
                    if (n as u8) < min {
                        return false;
                    }
                }
                // As many as qualify, up to X (none: the term is passed).
                Term::ChooseClan
                | Term::AllocatePerChosenWithCards { .. }
                | Term::ChooseMinionsX { .. } => {}
                // A Methuselah is always there to choose.
                Term::SeatOrCard { .. } => {}
                Term::ChooseCard(filter) => {
                    if !(0..st.cards.len())
                        .any(|i| crate::referendum::chooseable_card(st, CardId(i as u32), filter))
                    {
                        return false;
                    }
                }
            }
            i += 1;
        }
        true
    }
}

/// Methuselahs eligible to receive points, clockwise from the caller.
pub fn recipients(
    st: &GameState,
    caller: SeatId,
    among: Among,
    chosen: Option<SeatId>,
) -> Vec<SeatId> {
    st.clockwise_from(caller)
        .into_iter()
        .filter(|&s| match among {
            Among::Any => true,
            Among::Others => s != caller,
            Among::AnyNotChosen => Some(s) != chosen,
            Among::OthersNotChosen => s != caller && Some(s) != chosen,
        })
        .collect()
}

pub fn seat_filter_holds(filter: SeatFilter, st: &GameState, caller: SeatId, s: SeatId) -> bool {
    match filter {
        SeatFilter::Any => true,
        SeatFilter::MorePoolThanCaller => st.seats[s.index()].pool > st.seats[caller.index()].pool,
    }
}

/// A strike (rulebook "Strike Effects"), from the hands, a card or a weapon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StrikeKind {
    /// Hand strike: strength (+bonus) damage at close range.
    Hand { bonus: u8 },
    /// A melee weapon's "strength+n damage": close range, not a hand strike
    /// (hand-strike effects do not reach it).
    Melee { bonus: u8, aggravated: bool },
    /// Defensive: the opposing strike has no effect on the dodging minion.
    Dodge,
    /// Combat ends before any other strike resolves; `unlock`: "unlock
    /// this vampire before combat ends".
    CombatEnds { unlock: bool },
    /// Fixed damage; `ranged` works at any range.
    Damage {
        n: u8,
        ranged: bool,
        aggravated: bool,
    },
    /// Move up to n blood from the opposing minion to the striker, before
    /// damage is mended.
    StealBlood { n: u8, ranged: bool },
    /// "Send the opposing vampire to torpor or burn the opposing ally"
    /// (Touch of Oblivion [OBL]): at close range (a strike not said to be
    /// ranged, rules-qa 21.1), the vampire wounded (LSJ 20090622-2); no
    /// prevention reaches it.
    TorporOrBurn,
    /// A strike whose effect is its card's rider ("Strike: put this card on
    /// the opposing minion", Touch of Oblivion [obl]): no damage, close
    /// range only (rules-qa 21.1), dodged like any strike.
    CardEffect,
    /// "Ranged strike: burn weapon" (Voracious Vermin [ANI]): the opposing
    /// minion's weapon chosen as the strike is (`Side.strike_weapon`),
    /// burned if it lands and the weapon is still there.
    BurnWeapon,
    /// "Strike: burn equipment" (Heroic Might): the opposing minion's
    /// equipment named with the strike (`Side.strike_weapon`), at close
    /// range only (rules-qa 21.1), burned if it lands and is still there.
    BurnEquipment,
    /// "Strike: ranged; destroy equipment with first strike" (Canine Horde
    /// [ANI]): as `BurnEquipment`, at either range.
    RangedBurnEquipment,
    /// "Strike: put 1 corruption counter on the opposing minion; if the
    /// number of your corruption counters on the minion equals or exceeds
    /// their capacity or cost, you can burn those counters to steal the
    /// opposing minion after the resolution of this strike (ending
    /// combat)" (Contagion [DAI]): close range, dodged like any strike (ANK
    /// 20200909).
    Corrupt,
}

/// "A vampire can play only one X each round / combat / action": one play
/// of the card name per minion per scope (a canceled play still counts:
/// Immortal Grapple rulings).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Limit {
    PerRound,
    PerCombat,
    PerAction,
    /// "During an action … you can …": once each action per copy, every
    /// ability of the card so limited sharing the one use (Camarilla
    /// Conclave's stealth or votes; rules-qa 51.2).
    PerActionCard,
    /// An in-play ability in a phase window is once per phase per copy by
    /// default (the "during X" clause, rules-qa 6.6); with this limit each
    /// mode of the clause is its own once-per-phase use (Carfax Abbey's
    /// two picks).
    PerPhaseEach,
    /// "A referendum ability is usable once per referendum" (advanced
    /// rules §1.2.1): unlocking mid-vote does not restore it.
    PerReferendum,
    /// "Once each turn": once during each Methuselah's turn (advanced rules
    /// §1.2.1), per copy of the card or per vampire ability; on a card
    /// mode, "a vampire can play only one X each turn" (Forced March): per
    /// minion and card, the modes so limited together.
    PerTurn,
    /// "Only one X can be played each action", by any minion (Veil the
    /// Legions).
    OnePerAction,
    /// "A vampire can play only one X at superior each combat" (Terror
    /// Frenzy): per minion and card mode.
    PerCombatThisMode,
    /// Once each combat per copy, every ability of the card so limited
    /// sharing the one use (Dance of the Dead: "1 maneuver or press").
    PerCombatCard,
    /// "A minion can play only one X between their unlock phases" (On the
    /// Qui Vive): per minion and card name, until its controller's next
    /// unlock phase.
    BetweenUnlockPhases,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TargetKind {
    Minion,
    Seat,
    /// A vampire in the player's uncontrolled region (slot index).
    Uncontrolled,
    /// A library card in play (equipment to steal, a card to burn).
    Card,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TargetSpec {
    pub kind: TargetKind,
    pub filter: &'static [Requirement],
}

/// A resolved target of a play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Target {
    #[default]
    None,
    Minion(MinionId),
    Seat(SeatId),
    /// A slot of the player's uncontrolled region.
    Uncontrolled(u8),
    /// A library card in play: the action is directed at its controller
    /// unless that is the acting Methuselah (rules-qa 17.1), not at the
    /// minion it is on (rulebook "Target").
    Card(CardId),
}

impl Target {
    /// The minion targeted, where the mode's target spec makes one.
    #[track_caller]
    pub fn minion(self) -> MinionId {
        match self {
            Target::Minion(m) => m,
            _ => unreachable!("{self:?}: no minion target"),
        }
    }

    /// The card in play targeted, where the mode's target spec makes one.
    #[track_caller]
    pub fn card(self) -> CardId {
        match self {
            Target::Card(c) => c,
            _ => unreachable!("{self:?}: no card target"),
        }
    }
}

/// Where an ability used with `Choice::UseAbility` is printed: a card in
/// play (arena index) or a vampire's own text (`cards::vampires`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AbilitySource {
    Card(CardId),
    Vampire(MinionId),
}

impl AbilitySource {
    /// The card in play of an ability whose effects act on that card.
    pub fn card(self) -> CardId {
        match self {
            AbilitySource::Card(c) => c,
            AbilitySource::Vampire(m) => {
                unreachable!("an effect on its card in play, used from vampire {m:?}")
            }
        }
    }

    /// The key an "each action / combat / round" limit counts uses by: a
    /// card in play is limited per copy (Kevlar Vest's two branches share
    /// one use, rules-qa 10.3); a vampire's abilities are separate grants
    /// (Abaddon's maneuver and press).
    pub fn use_key(self, ability: u8) -> (AbilitySource, u8) {
        match self {
            AbilitySource::Card(_) => (self, 0),
            AbilitySource::Vampire(_) => (self, ability),
        }
    }

    /// The key a "during an action" limit counts uses by: each ability of
    /// a copy on its own (Bone Shambler's lock and Hecata's blood, rules-qa
    /// 42.9).
    pub fn action_key(self, ability: u8) -> (AbilitySource, u8) {
        (self, ability)
    }

    /// The NRA identity of the action this ability grants: the copy of a
    /// card in play and its ability, or the vampire's printed action (spec
    /// §3.8.1) — one action split by target kind keeps one identity
    /// (`Effect::SameActionAs`, Aniel).
    pub fn action_id(self, st: &crate::state::GameState, ability: u8) -> crate::state::ActionId {
        let same = self.mode(st, ability).effects.iter().find_map(|e| match e {
            Effect::SameActionAs(n) => Some(*n),
            _ => None,
        });
        match self {
            AbilitySource::Card(c) => crate::state::ActionId::InPlay(c, same.unwrap_or(ability)),
            AbilitySource::Vampire(m) => {
                crate::state::ActionId::Printed(m, same.unwrap_or(ability))
            }
        }
    }

    /// The definition of ability `ability`, wherever it is printed.
    pub fn mode(self, st: &crate::state::GameState, ability: u8) -> &'static ModeDef {
        let abilities = match self {
            AbilitySource::Card(c) => {
                lookup(st.cards[c.index()].card)
                    .expect("definition")
                    .abilities
            }
            AbilitySource::Vampire(m) => {
                vampire_of(&st.minions[m.index()])
                    .expect("vampire definition")
                    .abilities
            }
        };
        &abilities[ability as usize]
    }
}

/// Numeric terms chosen as the card is played (rules-qa 3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Terms {
    /// An integer in `min..=max` (the amount actually taken at resolution
    /// is capped by what is there: Villein rulings).
    BloodRange { min: u8, max: u8 },
    /// "Any amount of blood" of the target vampire: 0 up to what it has as
    /// the card is played (Minion Tap; rules-qa 38.1), what is left taken
    /// at resolution.
    UpToTargetBlood,
    /// X blood, 0 up to the actor's blood (Hidden Strength: X may exceed
    /// what is needed, never below zero).
    XBlood,
    /// X in `min..=max`, paid in blood: never above the actor's blood
    /// (Monkey Wrench: "X must be 1, 2 or 3").
    XBloodIn { min: u8, max: u8 },
    /// Up to n of the votes the referendum passed by (Voter Captivation
    /// [PRE]: "move up to 2 of those blood to your pool instead").
    UpToMargin(u8),
    /// "X is the number of copies of this card you control" (Creeping
    /// Sabotage): no decision; read when the cost is paid.
    CopiesControlled,
    /// "You can burn a card named X (if any)" (Vessel): which copy in play,
    /// any controller's, or none — announced with the play like any target
    /// (rules-qa 6.7), burned at resolution.
    BurnNamedInPlay(&'static str),
    /// Ability cost "discard n cards" (`of`: the cards that qualify): which
    /// ones, chosen one at a time as the use is announced (Tension in the
    /// Ranks, The Barrens, Kasim Bayar, Alexa Draper).
    Discard { n: u8, of: CardFilter },
    /// A number of cards in `min..=max`, chosen as the ability is used
    /// (Heart of Nizchetus: "up to 3"; rules-qa 11.1).
    Count { min: u8, max: u8 },
    /// "Any amount of blood" moved from the target minion to the host, up
    /// to the host's starting life (The Vozhd of Juiz de Fora): 1 to what
    /// both allow.
    LifeFromTarget,
    /// A card in the user's ash heap, chosen by name as the ability is used
    /// (Mora, the Death Seer).
    AshCard,
    /// A card with two copies or more in the user's prey's ash heap, named
    /// as the action is announced; both removed as it resolves (Giovanni
    /// del Georgio).
    PreyAshPair,
    /// Another vampire of the user's, in torpor or not, able to hold the
    /// target card, chosen as the ability is used; the card moves onto it
    /// (Luciano Carvalho, rules-qa 41.7).
    VampireToHoldTarget,
    /// X pool, 1 to the user's pool (Oxford University, England).
    XPool,
    /// X counters, 1 to the card's counters (Alamut).
    XCounters,
    /// An ally in any Methuselah's ash heap, named as the card is played:
    /// the ash heap, then the ally (Psychophagia; owner, 2026-09-26:
    /// rules-qa 32.12).
    AnyAshAlly,
    /// "Employ a <label> retainer from your hand (requirements and cost apply
    /// as normal)" (Pack Alpha [ani]): the card and its version, named as
    /// the card is played (LSJ 20100130).
    EmployFromHand(crate::library::Label),
    /// As `EmployFromHand`, "ignoring requirements (pay cost as normal)"
    /// (Biothaumaturgic Experiment [tha]): no clan or Discipline needed; a
    /// version depending on a Discipline level still follows the
    /// vampire's (LSJ 20080803): the basic one always.
    EmployFromHandIgnoring(crate::library::Label),
    /// "Recruits or employs an ally or retainer from your hand
    /// (requirements and cost apply as normal)" (Piper), named as it is
    /// played.
    RecruitOrEmployFromHand,
    /// A card in the hand of the Methuselah the action is directed at,
    /// chosen by name, discarded as the ability resolves (Richard de
    /// Worde: the choice shows the hand, his look).
    DirectedHandCard,
    /// "Equip this vampire with <filter> from your hand (requirements and
    /// cost apply as normal)" outside an action (Angel's Gift, Contraband):
    /// the card named as it is played (LSJ 20100130); with `blood_share`,
    /// then how much of its pool cost, up to half rounded down, the vampire
    /// pays in blood (Contraband [OBF]).
    EquipFromHand {
        filter: CardFilter,
        blood_share: bool,
    },
    /// "Choose a ready <clan> you control" as the card is played — with
    /// `other`, another than the play's target (Priority Contract, Toreador
    /// Grand Ball; owner, 2026-09-26: rules-qa 28.5).
    OwnOfClan { clan: &'static str, other: bool },
    /// The pool Life Boon gives the target Methuselah: at least what keeps
    /// them in the game (their debt, plus 1), at most what leaves its
    /// player 1.
    BoonPool,
}

/// Continuous-effect modifiers carried by layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Modifier {
    /// The layer source's host gets n bleed against its controller's prey
    /// (Tier of Souls [ANI]).
    HostBleedAgainstPrey(i8),
    /// "Recruit ally actions cost this vampire -n blood or pool" (Charisma),
    /// on the host.
    HostRecruitDiscount(u8),
    /// The host gets n votes while its controller controls a vampire named
    /// so (Mithraic Cultist: "+1 vote while you control /Mithras/").
    HostVotesWhileControlling { name: &'static str, n: i8 },
    /// The host gets n intercept during actions directed at /Mithras/ or a
    /// Mithraic Mysteries card or member (Mithraic Cultist).
    HostInterceptVsMithraic(i8),
    /// The host gets n intercept against vampires (Carlton Van Wyk: "+1
    /// intercept against vampires"): an acting vampire's actions.
    HostInterceptVsVampires(i8),
    /// "Allies get +1 intercept against vampires" (The Unmasking): every
    /// Methuselah's allies, while the acting minion is a vampire.
    AlliesInterceptVsVampires(i8),
    /// The host is a Mithraic Mysteries member (Mithraic Cultist).
    MithraicMember,
    /// A card in play reducing the cost of its controller's cards.
    CostSource(CostSource),
    /// "This minion cannot be the target of other Methuselahs' actions.
    /// Master cards targeting this minion cost 1 additional pool" (Secure
    /// Haven), on the host.
    Haven,
    /// Non-electronic, non-vehicle, non-weapon equipment cards cost the
    /// controller's minions n less blood or pool to equip (British Museum).
    PlainEquipDiscount(u8),
    /// Minions of the clan get 1 optional maneuver each combat (Brujah
    /// Debate).
    ClanCombatManeuver(&'static str),
    /// "While this Anarch is <clan>, they get +1 vote during referendums
    /// they call" (Fee Stake: Boston): the layer source's host, while of
    /// one of these clans, calling the referendum.
    HostVoteWhenCalling(&'static [&'static str]),
    /// While the layer source is unlocked, vampires of capacity `max` or
    /// less get n bleed against its controller (Aranthebes, The Immortal).
    YoungBleedAgainstController { max: u8, n: i8 },
    /// Every minion of the clan gets n to the stat (Week of Nightmares) —
    /// with `own`, only the layer source's controller's ("Gangrel you
    /// control", Gangrel Revel); with `titled`, only titled ones (New
    /// Carthage); with `during`, only during its actions of these kinds
    /// ("Assamites get +1 stealth when bleeding", The Khabar: Community).
    ClanStat {
        clan: &'static str,
        stat: Stat,
        n: i8,
        own: bool,
        titled: bool,
        during: &'static [ActionKind],
    },
    /// Minions of the clan do not hunt as normal (Week of Nightmares).
    ClanCannotHunt(&'static str),
    /// "Sabbat vampires you control get +1 hunt" (The Hungry Coyote): the
    /// layer source's controller's vampires of the sect.
    OwnSectHunt(Sect, u8),
    /// "This vampire cannot recruit allies or employ retainers"
    /// (Depravity), on the host.
    HostCannotRecruitOrEmploy,
    /// "The employer gets 1 optional press each combat" (Dread Mastiff
    /// [ANI]), on the host.
    HostCombatPress(u8),
    /// "This minion cannot block" (Pentex™ Subversion), on the host.
    HostCannotBlock,
    /// "The other Toreador's non-bleed actions cannot be blocked" (Toreador
    /// Grand Ball): the card's second chosen vampire.
    SecondChosenUnblockable,
    /// "The locked Toreador does not unlock as normal" (Toreador Grand
    /// Ball): the card's first chosen vampire.
    FirstChosenDoesNotUnlock,
    /// "+1 level of <Discipline> and +1 capacity" on the host (a Discipline
    /// master card: Oblivion).
    DisciplineLevel(&'static str),
    /// "This vampire gets +1 level of <Discipline>" (Ritual Goblet): an
    /// equipment's, no capacity with it; nothing on an ally.
    BearerDisciplineLevel(&'static str),
    /// "This vampire gets superior <Discipline>" (The Sargon Fragment): its
    /// level is 2, an equipment's; nothing on an ally.
    BearerSuperiorDiscipline(&'static str),
    /// Cards with this printed name cost `pool` more (or less) to play;
    /// with `on_host_only`, only when played on the layer source's host;
    /// with `owner_only`, only for the layer source's controller ("cost
    /// *you* +1 pool").
    CostMod {
        name: &'static str,
        on_host_only: bool,
        owner_only: bool,
        pool: i8,
    },
    /// Reaction cards cost this much more blood (or life) this action.
    ReactionBloodCost(i8),
    /// Minions younger than this capacity (allies included) get this
    /// intercept modifier this action (Perfect Paragon [PRE]).
    InterceptMod { younger_than: u8, n: i8 },
    /// The bearer "cannot be the target of a directed action performed by
    /// an ally or a vampire with capacity n or less" (Veneficti).
    NotDirectedByAllyOrCapacityAtMost(u8),
    /// "Each time you draw to replace a card this minion played as an
    /// acting minion, you may draw an additional card (discard afterward)"
    /// (Learjet), on its bearer.
    ExtraDrawOnReplace,
    /// +n hunt for the hunt of this action (Hospital Food), action-scoped.
    ActionHunt(u8),
    /// "Gets an additional strike (limited) each round" (Renegade Garou):
    /// the layer's bearer, after each round's normal pair, mandatory.
    AdditionalStrikeEachRound,
    /// "The bearer gets +n hunt" (Aaron's Feeding Razor): more blood from
    /// its hunts (`GameState::hunt_bonus`).
    BearerHunt(u8),
    /// Each strike or damaging effect of the opposing minion in combat
    /// inflicts n less damage on the layer's bearer (Nephandus), read in
    /// `GameState::damage_reduction`.
    OpposingDamageLess(u8),
    /// Minions with any corruption counters get this intercept modifier
    /// this action (Sense the Sin [DAI]), read live.
    CorruptedInterceptMod(i8),
    /// The layer source's host gets this intercept during bleed actions
    /// directed at its controller (Guardian Angel).
    HostInterceptOnBleedAtYou(i8),
    /// "+n intercept during actions directed at their controller" (Abbot):
    /// at them or at a card they control (LSJ 20061222).
    HostInterceptDirectedAtYou(i8),
    /// The layer source's controller gets this hand size (Elder Library).
    HandSize(i8),
    /// The layer source's controller gets +n transfers in their influence
    /// phase (Information Highway).
    Transfers(u8),
    /// The layer source's controller gets +1 hand size per counter on it
    /// (Visit from the Capuchin).
    HandSizePerCounter,
    /// The layer source's host, if `who`, gets `n` to `stat` during its
    /// actions of these kinds (any action when empty): Sport Bike's +1
    /// intercept, Unlicensed Taxicab's +1 stealth during hunt and employ
    /// actions, Treasured Samadji's +1 bleed.
    Bearer {
        stat: Stat,
        n: i8,
        during: &'static [ActionKind],
        who: Bearer,
    },
    /// Allies and vampires younger than the layer source's host, if
    /// `who`, get `n` intercept against its actions (Stolen Police
    /// Cruiser).
    YoungerInterceptAgainstBearer { n: i8, who: Bearer },
    /// "Minions without <Discipline> cannot perform actions directed at this
    /// vampire" (Shadow Cloak [OBL]).
    DirectedOnlyBy(&'static [&'static str]),
    /// "The attached minion burns 1 counter from this card instead of
    /// unlocking as normal. If this card has no counters, burn it" (Touch
    /// of Oblivion): a substitution — an unlocked host burns one too, a
    /// host kept from unlocking does not (advanced rules §5.2.2).
    CounterInsteadOfUnlock,
    /// "Rescuing a (non-<clan>) vampire from torpor costs this <host> -n
    /// blood, and if the action is successful, the rescued vampire gains
    /// `gain` blood" (Saulot's Healing Touch): the action's cost, whoever
    /// pays it (PIB 20110918, LSJ 20020304-2).
    RescueDiscount {
        not_clan: Option<&'static str>,
        n: u8,
        gain: u8,
    },
    /// A retainer "inflicts nR damage on the opposing minion each round of
    /// combat during normal strike resolution" (Murder of Crows):
    /// environmental damage, the retainer its source (advanced rules
    /// 5.6.3) — a dodge does not stop it (rulebook "Environmental
    /// damage"), a combat ends strike does.
    RetainerDamage(u8),
    /// As `RetainerDamage`, at close range only (Dread Mastiff: "each
    /// round of combat during normal strike resolution at close range").
    RetainerDamageClose(u8),
    /// "Minions in combat with the employer cannot strike: combat ends"
    /// (Dog Pack).
    NoCombatEndsAgainstHost,
    /// The minion this card is cannot block vampires (Vagabond Mystic).
    CannotBlockVampires,
    /// "Vampires with capacity n or more cannot attempt to block the
    /// bearer" (The Signet of King Saul).
    BigVampiresCannotBlockBearer(u8),
    /// The minion this card is cannot have or use equipment or retainers
    /// (Bone Shambler).
    NoEquipmentOrRetainers,
    /// The minion this card is cannot have or use equipment (Gravebound
    /// Drone).
    NoEquipmentHeld,
    /// "This vampire cannot have or use any ranged weapons except the
    /// Ivory Bow" (Anachronism): on the card's host; every weapon with a
    /// ranged strike (LSJ 20060825), the named one aside.
    NoRangedWeaponsExcept(&'static str),
    /// Ossian: "In the first round of combat with a vampire who has
    /// played a card that requires Auspex during this action, that vampire
    /// cannot use any maneuvers or strikes" — any action (LSJ 20050201),
    /// read as cards are played (an Auspex strike stays legal until one
    /// is: ANK 20210614).
    FreezesAuspexFoeFirstRound,
    /// Ossian: "gains 1 life at the end of each round for each blood the
    /// opposing vampire used to heal damage or prevent destruction that
    /// round" — after the presses, a round ended early too (RTR 20030519),
    /// over his starting life (ANK 20171109).
    LifePerFoeBloodLost,
    /// "While the employer is in combat, the opposing minion's controller
    /// gets -n hand size" (Raptor [ANI]).
    OpponentHandSizeInCombat(u8),
    /// "While the employer is in combat, the opposing minion's controller
    /// plays with an open hand" (Owl Companion).
    OpponentOpenHandInCombat,
    /// "The attached vampire does not unlock as normal" (Puppet Master
    /// [DOM]): skipped by its controller's unlock phase; other unlock
    /// effects still work.
    HostDoesNotUnlock,
    /// "Your prey plays with an open hand" (Revelations [AUS]): the prey of
    /// this card's controller, read live.
    PreyOpenHand,
    /// The bearer cannot play cards of this name (Preternatural Strength:
    /// "They cannot play cards named Torn Signpost").
    BearerCannotPlay(&'static str),
    /// The minion this card is cannot be the target of directed actions
    /// requiring these Disciplines (Aggressive Corpse: Dominate, Presence).
    ImmuneToDirectedActions(&'static [&'static str]),
    /// The minion this card is — an ally — "can play cards requiring basic
    /// X as a vampire (with capacity n)", action cards excepted with
    /// `non_action` (The Vozhd of Juiz de Fora, Spectral Servitor).
    PlaysAsVampire(AsVampire),
    /// "Allies cannot block this Anarch" (Libertas, on its bearer).
    AlliesCannotBlockBearer,
    /// "Cards requiring Dominate or Presence cost other minions +1 blood
    /// while this Anarch is acting, attempting to block or in combat"
    /// (Libertas): the Disciplines and the surcharge.
    DisciplineSurchargeAround(&'static [&'static str], u8),
    /// "Cards requiring a Discipline cost this vampire +n blood" (Enhanced
    /// Coagulant, once it became that card): the host's cards requiring
    /// a Discipline.
    HostDisciplineCardsCost(u8),
    /// "Gehenna events cannot be played" (The Fourth Cycle).
    NoGehennaEvents,
    /// "Minion cards that change the target of a bleed action cost +n blood
    /// or life" (Narrow Minds).
    RedirectCardsCost(u8),
    /// "Actions performed by vampires in torpor cost +1 blood. Rescuing an
    /// older vampire from torpor costs +1 blood" (Torpid Blood).
    TorpidBlood,
    /// "Cards requiring 1 or more Disciplines at the superior level cost
    /// +n blood" (The Slow Withering; diablerists aside).
    SuperiorCardsCost(u8),
    /// "During an action, cards minions play requiring 1 or more
    /// Disciplines are not replaced until the end of that action or until a
    /// Methuselah's hand is empty" (Blood Weakens; diablerists aside).
    HoldDisciplineReplacements,
    /// "After each Methuselah's minion phase ends, each ready vampire …
    /// with capacity less than the number of Gehenna events in play who
    /// did not hunt … burns 1 blood" (Thirst).
    Thirst,
    /// "Vampires with capacity greater than X are considered vampires with
    /// capacity X (minimum of 1)", X the card's counters (Wormwood).
    CapacityCapByCounters,
    /// "Blood hunts cannot be called on vampires with capacity greater than
    /// the number of counters on this card who diablerize a younger
    /// vampire" (Fueled by Heart's Blood).
    NoBloodHuntAboveCounters,
    /// "During your discard phase, burn another event at random" (The
    /// Uncoiling).
    BurnsAnEventEachDiscard,
    /// "During each Methuselah's discard phase, that Methuselah burns 1
    /// pool for each vampire in torpor they control" (Dragonbound).
    TorporPoolBurn,
    /// "Vampires cannot hunt unless they must hunt" (Restricted Vitae).
    OnlyMustHunt,
    /// "There is no <sect>. Instead, <sect> vampires are considered
    /// Independent" (Fall of the Camarilla, Fall of the Sabbat).
    SectFallen(Sect),
    /// Absimiliard's Army: after a Methuselah is ousted, burn this card;
    /// as it leaves play, its ghouls are removed from the game.
    GhoulArmy,
    /// "If an ally is burned in combat with an acting vampire, add 1
    /// counter to this card and inflict 2 (`damage`) unpreventable
    /// environmental damage on the acting vampire after the combat ends. If
    /// this card has 4 (`burn_at`) counters, burn it" (FBI Special Affairs
    /// Division).
    AllyBurnedInCombat { damage: u8, burn_at: u8 },
    /// "While your prey controls a vampire in torpor, vampires you control
    /// get +n bleed" (Raising the Portcullis), read live.
    OwnBleedWhilePreyTorpor(i8),
    /// "Vampires attempting to block the attached vampire burn n blood"
    /// (Archon): cumulative (LSJ 19980225); a vampire with too little
    /// burns what it has and still blocks (LSJ 19970707).
    BlockersOfHostBurn(u8),
    /// "Blood hunts cannot be called on the attached vampire" (Archon).
    NoBloodHuntOnHost,
    /// The strikes of the minion this card is cannot be dodged: a dodge has
    /// no effect on them (Aggressive Corpse; LSJ 20030902-2).
    UndodgeableStrikes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    /// While the source card stays in play.
    InPlay,
    /// Until the current action ends.
    Action,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Layer {
    pub scope: Scope,
    pub modifier: Modifier,
    pub source: CardId,
}

/// Effect primitives; `X` is the play's terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Move 1 counter from this card to the target minion (Week of
    /// Nightmares' nightmare counters); none left, the card's end: each
    /// vampire of this clan burns 1 blood per counter on it or is burned.
    CounterToTarget(&'static str),
    /// The granted action is a hunt ("… as a +1 stealth hunt action", Week
    /// of Nightmares): a hunt for everything reading one, successful only
    /// if it gains blood (LSJ 20050720-2).
    HuntAction,
    /// Leave rider: "after a vampire with capacity n or less goes to
    /// torpor, put 1 hostage counter on them" (Carver's Meat Packing and
    /// Storage).
    HostageOnTorpor(u8),
    /// Add X blood to the target, X the number of vampires with any hostage
    /// counters.
    BloodPerHostage,
    /// Burn the target vampire's hostage counters.
    BurnTargetHostages,
    /// Warsaw Station: unlock the acting minion of the clan after the
    /// action's resolution if it was successful
    /// (`ActionCtx::unlock_if_successful`).
    UnlockActingIfSuccessful(&'static str),
    /// Move the target vampire from torpor to its controller's ready region
    /// (Chantry).
    TargetLeavesTorpor,
    /// A ready vampire of the clan the user controls burns 1 blood (Chantry's
    /// cost): chosen as it resolves.
    OwnClanBurnsBlood(&'static str),
    /// The card's second chosen vampire is the one its terms chose
    /// (`Terms::OwnOfClan`: Toreador Grand Ball's other Toreador).
    ChooseSecondOfClan(&'static str),
    /// The card's chosen vampire is the one its terms chose (Priority
    /// Contract's Assamite).
    ChooseOwnOfClan(&'static str),
    /// "If the attached minion is about to leave the ready region, you can
    /// burn this card to gain n pool" (Priority Contract): also as its
    /// controller is ousted (ANK 20180129-2), and if the card goes with it
    /// (PIB 20150512).
    PoolWhenHostLeaves(u8),
    /// Leave rider: this card's controller gains n pool when a ready minion
    /// of their prey's is burned or sent to torpor (Frontal Assault).
    ControllerGainsPoolIfPrey(u8),
    /// "During your influence phase, burn this card and burn 1 pool for
    /// each ready minion controlled by your prey" (Frontal Assault): as the
    /// phase begins.
    BurnAtInfluencePerPreyReady,
    /// Add n blood to the acting minion (The Anarch Free Press: "add 1
    /// blood to that Anarch").
    BloodToActing(u8),
    /// "Exchange one card from your hand for one card in your ash heap
    /// requiring an Anarch" (Garibaldi-Meucci Museum): both named as it is
    /// used (LSJ 20000222), only cards whose requirements name Anarch or
    /// baron (ANK 20250320).
    ExchangeForAnarchCard,
    /// "+1 discard phase action", the one Powerbase: Los Angeles tracks
    /// (`Seat::granted_dpa`).
    GrantDiscardAction,
    /// The target vampire is the card's chosen one (`CardInPlay::chosen`,
    /// The Rack), as it is played.
    ChooseTarget,
    /// "As … its controller changes, its controller chooses a ready vampire
    /// they control" (The Rack): read on the card's first mode.
    RechoosesOnControlChange,
    /// Add n blood to the card's chosen vampire, in play (torpor included,
    /// PIB 20150522) — its controller's choice (The Rack, ANK 20200130-2).
    BloodToChosen(u8),
    /// "Give your prey control of this card" (Anarch Troublemaker).
    GiveToPrey,
    /// "Lock up to n vampires they control" (the prey's; Anarch
    /// Troublemaker): chosen as it resolves, a locked one a legal choice
    /// (PIB 20121031).
    LockPreyVampires(u8),
    /// An ability's cost in transfers ("you can use 1 transfer to …"),
    /// spent from the influence phase's (Wider View).
    SpendTransfers(u8),
    /// "Lock during your influence phase to get +n transfer" (Ennoia's
    /// Theater): added to the phase's transfers left, at any moment of the
    /// phase (RTR 20180303).
    GainTransfers(u8),
    /// "A Tremere you control can move 1 blood from themselves to this
    /// card" (Wasserschloss Anif): from the target minion.
    BloodToCard(u8),
    /// "Move all counters on this card to a <clan> in your uncontrolled
    /// region" (Wasserschloss Anif): all of them, over capacity too
    /// (rules-qa 28.3).
    CountersToUncontrolledTarget,
    /// A granted action's stealth for a titled acting vampire, read live
    /// on it ("titled vampires get +1 stealth on that action", Powerbase:
    /// Barranquilla).
    StealthForTitled(i8),
    /// "<Clan> get +n stealth during that action" of a granted action
    /// (Wasserschloss Anif: Malkavians +1; Toreador Grand Ball: Nosferatu
    /// -1), read live on the acting minion.
    StealthForClan {
        clan: &'static str,
        n: i8,
    },
    /// "Draw 1 card from your crypt and then remove a crypt card in your
    /// uncontrolled region from the game" (Wider View): chosen as it
    /// resolves.
    DrawCryptThenRemove,
    /// "Reveal the top card of your crypt. If it is a <clan>, draw it and
    /// add 1 blood to it; otherwise, move it to the bottom of your crypt"
    /// (Family Gathering).
    RevealCryptTop(&'static str),
    /// "Reveal and remove the top card of your crypt from the game. If it
    /// is a <clan>, a non-titled vampire you control becomes Anarch"
    /// (Gilbert Duane): the vampire chosen as it resolves.
    RemoveCryptTopAnarch(&'static str),
    /// "Put this card in play; it becomes a unique mortal ally with …"
    /// (Jake Washington): the master card in play as the ally, with this
    /// type line; not recruited, so it may act this turn.
    BecomesAlly(crate::library::Body),
    /// "Add up to 4 blood to …" (Jake Washington): the ability's count X
    /// (`Terms::Count`) to the target.
    BloodToTargetX,
    /// Lock the target card in play (Powerbase: Savannah's cost: "lock any
    /// other unique location you control").
    LockTargetCard,
    /// "If you control three copies of this card, remove all copies in
    /// play (even controlled by other Methuselahs) from the game to gain 3
    /// pool and choose up to thirteen library cards from your ash heap;
    /// move one of the chosen cards to your hand and shuffle the others
    /// into your library" (Ashur Tablets), checked as a copy enters play;
    /// the cards chosen then (LSJ 20091030).
    AshurTablets,
    /// Life Boon's gift: X (`Terms::BoonPool`) pool from its player to the
    /// target Methuselah, who is no longer about to be ousted; the card in
    /// play remembers them.
    GiveBoonPool,
    /// "Steal 1 pool from that Methuselah" (Life Boon, in their unlock
    /// phase): their last one ousts them.
    StealPoolFromBoonTarget,
    /// "Combat ends. This vampire is locked and sent to torpor"
    /// (Rötschreck, on its target): combat ends at once, no strike
    /// resolving; the lock and torpor land once it has ended, both minions
    /// ready until then (LSJ 20070907) — not if the combat continues (RTR
    /// 20020501).
    RotschreckEndsCombat,
    /// "Put this card in play. It becomes a 1-capacity (non-unique) Anarch
    /// vampire of the same clan as the acting vampire and must hunt this
    /// turn. You can search your library (shuffle afterward), hand and/or
    /// ash heap for a Discipline master card and put it on this new
    /// vampire" (Childe of the Revolution): after `PutInPlay`, of the sect
    /// printed, else of its sire's — the acting vampire's (rulebook
    /// "Sect": new vampires take their sire's; Waters of Duat, rules-qa
    /// 32.6).
    BecomesVampire(Option<Sect>),
    /// Put this card on the target minion.
    PutOnTarget,
    /// Put this card in play with no host (Anarch Revolt).
    PutInPlay,
    /// "Title": this card, on a vampire, represents that title (glossary
    /// "Title": a placeholder, burned if the title is yielded or lost). No
    /// effect of its own: read by `card_title` wherever the card is.
    RepresentsTitle(Title),
    /// Rider effect: the Methuselah whose unlock phase it is burns `pool`
    /// unless they control a ready vampire of `sect` (Anarch Revolt; each
    /// copy in play burns on its own, spec §1.15).
    UnlockBurnPoolUnlessReady {
        sect: Sect,
        pool: u8,
    },
    /// Rider effect: the controller's prey burns n pool (Creeping
    /// Sabotage).
    PreyBurnsPool(u8),
    /// Rider effect: the Methuselah resolving it burns n pool ("that
    /// Methuselah burns 1 pool", Fame's unlock rider).
    BurnPool(u8),
    /// Rider effect: the Methuselah resolving it moves n counters from
    /// their pool to this card ("move 1 counter from your pool to this
    /// card", Smiling Jack) — down to 0 pool ousts them.
    PoolToCounter(u8),
    /// Rider effect: for each counter on this card, the Methuselah
    /// resolving it burns 1 pool or 1 blood from a vampire they control
    /// (Smiling Jack), each unit chosen as it is paid — any mix (ANK
    /// 20181006), a vampire with no blood is no source (LSJ 19971212).
    PoolOrBloodPerCounter,
    /// Rider effect: the Methuselah resolving it burns X pool and/or
    /// cards at random from their hand, X the counters on this card
    /// (Constant Revolution): they announce how many cards (at most their
    /// hand), those are burned at once, the rest is pool (LSJ 20090618).
    PoolOrRandomCardsPerCounter,
    /// Put in play with n counters ("put this card in play with 1
    /// counter").
    EnterWithCounters(u8),
    /// A granted action's pool cost, paid at resolution ("as a Ⓓ action
    /// that costs 1 pool"; unpayable by then it fizzles, rules-qa 3.6).
    PayPool(u8),
    /// Rider effect: the controller of the minion the hook names burns n
    /// pool — the host ("after this vampire goes to torpor, their
    /// controller burns 3 pool", Fame) or the minion that left ("after a
    /// ready minion is burned or sent to torpor, their controller burns 1
    /// pool", Tension in the Ranks) — at once, where it happened (rules-qa
    /// 6.4).
    TheirControllerBurnsPool(u8),
    /// Ability cost: one of the player's master phase actions ("use a
    /// master phase action … to burn this card").
    SpendMasterPhaseAction,
    /// Ability cost: lock this card in play ("you can lock this card to
    /// …").
    LockSelf,
    /// Ability cost: burn the Edge ("burn the Edge to …", Kalinda).
    BurnEdge,
    /// Ability cost: one of the player's discard phase actions.
    SpendDiscardPhaseAction,
    /// Add n blood to the host, up to its capacity ("add 1 blood to her").
    AddHostBlood(u8),
    /// The library card chosen by `Terms::AshCard` goes from the ash heap to
    /// the bottom of the library (Mora, the Death Seer).
    AshCardToLibraryBottom,
    /// Add n counters to this card in play ("add 1 counter to this card
    /// each time you lock it", Dreams of the Sphinx).
    AddCounter(u8),
    /// Burn this card once it carries n counters ("burn this card if it
    /// has 3 counters"), checked as the ability resolves.
    BurnSelfAtCounters(u8),
    /// Rider effect: burn n counters from this card (Visit from the
    /// Capuchin: "instead burn 1 counter from this card").
    RemoveCounter(u8),
    /// Rider effect: burn this card if it has no counters left.
    BurnSelfWithoutCounters,
    /// The player gains n pool.
    GainPool(u8),
    /// The player's hand size grows by n until the end of the turn (the
    /// end of the discard phase, LSJ 20100812) — the hand is drawn up at
    /// once; cards over size afterward stay (ANK 20180318).
    HandSizeBonusUntilEndOfTurn(u8),
    /// Add n blood to the targeted vampire in the player's uncontrolled
    /// region (capped at its capacity).
    BloodToUncontrolledTarget(u8),
    /// Add n blood to the targeted minion (capped at its capacity; The
    /// Coven: "add 2 blood to a ready vampire you control").
    BloodToTarget(u8),
    /// "Move blood, equipment and/or retainers between any two ready
    /// vampires you control" (Heidelberg Castle): from the target to another
    /// ready vampire of the user's, chosen next, then the blood and the
    /// cards (`Interrupt::MoveBetween`).
    MoveBetweenVampires,
    /// "Unlock the acting minion and lock this vampire instead. The action
    /// continues with this vampire as the acting minion" (Mask of a
    /// Thousand Faces): the action's modifiers stay, the inherent ones are
    /// the new acting minion's (RTR 19951110); a blocker attempting still
    /// is (LSJ 19990106).
    TakeOverAction,
    /// "Look at any Methuselah's hand" (SchreckNET): the target seat's hand
    /// is visible to the user until the current action ends.
    PeekHand,
    /// "If this <clan>'s capacity is n or more, he or she unlocks"
    /// (SchreckNET): the acting minion, read at resolution.
    UnlockActorIfClanCapacity {
        clan: &'static str,
        n: u8,
    },
    /// Rider effect: the controller's predator takes control of this card
    /// (The Coven; spec §6.3: requirements are not re-checked, where it
    /// goes when it leaves play is still its owner's).
    PredatorTakesControl,
    /// Rider effect: burn this card (Guardian Angel: "if this vampire is in
    /// torpor, burn this card").
    BurnSelf,
    /// Burn the card in play granting this action (a "burn this card as a
    /// Ⓓ action" granted action), with `DeclareAction(BurnCard)`.
    BurnSource,
    /// Move X blood from the target minion to the player's pool.
    MoveBloodTargetToPoolX,
    /// Push a continuous layer sourced by this card while it is in play.
    Layer(Modifier),
    /// Push a layer scoped to the current action.
    ActionLayer(Modifier),
    /// Cancel the card being played; `refund`: its cost is not paid.
    CancelPlayed {
        refund: bool,
    },
    /// Declare an action of this kind with this card (action cards).
    DeclareAction(ActionKind),
    /// +n stealth for the current action.
    AddStealth(u8),
    /// +n intercept for the actor's current block attempt.
    AddIntercept(u8),
    /// The minion attempting to block gets n intercept for the rest of the
    /// action (Oskar Anasov's +1, Gwenllian Arwyn's -1).
    AttemptingIntercept(i8),
    /// Marker of a granted bleed: no equipment but the granting one may
    /// increase its bleed amount (Camera Phone).
    NoOtherEquipmentBleed,
    /// +n bleed for the current bleed; `limited` binds the rulebook cap.
    BleedMod {
        n: u8,
        limited: bool,
    },
    /// The actor wakes for the rest of the action.
    Wake,
    /// Lock the actor.
    LockActor,
    /// Ability cost: lock the card's host ("this Salubri can lock …").
    LockHost,
    /// "Vampires cannot block this action" (Daring the Dawn).
    VampiresCannotBlock,
    /// "This vampire takes n unpreventable environmental aggravated damage
    /// after action resolution" (Daring the Dawn): if the action reaches
    /// resolution, after combat if blocked (RTR 19980623, ANK 20220331).
    DamageAfterResolution(u8),
    /// "The chosen vampire cannot block this action" (Seduction, The
    /// Sleeping Mind [dom]).
    TargetCannotBlock,
    /// "During this action, minions cannot unlock" (The Sleeping Mind
    /// [DOM]): wakes still work (PIB 20150703).
    NoUnlockThisAction,
    /// "The chosen minion cannot play reaction cards this action"
    /// (Unleashing the Bestial Soul).
    TargetNoReactions,
    /// "The next reaction card played this action costs +n blood or life"
    /// (Unleashing the Bestial Soul [DOM]).
    NextReactionCosts(u8),
    /// "Reduce a bleed against you to 0 (the acting minion can still
    /// increase the bleed amount)" (Visions of Zapathasura [OBF][PRE]): a
    /// reduction by the bleed amount as it resolves.
    ReduceBleedToZero,
    /// "Unlock this minion. The action ends (unsuccessfully). This minion
    /// cannot perform the same action again this turn" (Change of Target);
    /// also before the combat an unblocked action would begin (Obedience,
    /// rules-qa 55.7).
    EndBlockedAction,
    /// "Give the next X actions minions you control perform this turn +1
    /// stealth" (Veil the Legions [OBF]), X blood burned from the actor.
    StealthForNextActions,
    /// End the combat at once, outside strike resolution: no strike
    /// resolves, the end-of-round window still opens (advanced rules
    /// §4.7.2, §4.9).
    EndCombat,
    /// The actor steals the target's title: the target loses it, the actor
    /// gains it (Valerius).
    StealTargetTitle,
    /// Change the bleed's target to the target seat (reopens blocks).
    RedirectBleed,
    /// Burn the acting minion.
    BurnActingMinion,
    /// The action is not successful.
    ActionFails,
    /// Reaction cards are not replaced until the end of the action.
    DeferReactionReplacement,
    /// If this action is blocked, lock the blocking minion and end the
    /// action before block resolution (Mirror Walk [THA]).
    EndActionIfBlocked,
    /// The card (a retainer) enters play on the acting minion with this life.
    EmployRetainer {
        life: u8,
    },
    /// Burn n life from this card: a retainer's counters, an ally's own
    /// life (Crypt's Sons, Underbridge Stray) — its last life burns it. Not
    /// a blood cost: an ally pays it.
    BurnOwnLife(u8),
    /// The action continues as if unblocked, before block resolution
    /// (Crypt's Sons).
    ContinueUnblocked,
    /// Take control of the target ally (Entrancement [PRE], Far Mastery
    /// [DOM]): the cards on it follow it.
    StealTargetAlly,
    /// Rider effect: move n blood from the host to the minion that played
    /// this card (`CardInPlay::by`; Slaughtering the Herd [DOM]).
    HostBloodToBy(u8),
    /// "Allies cannot block this action" (Visions of Gehenna, Invigorate
    /// [dom]).
    AlliesCannotBlock,
    /// "This Anarch gets +n strength this action" (Invigorate [ani]).
    StrengthThisAction(u8),
    /// "This Anarch's hand strikes are aggravated this action" (Invigorate
    /// [pro]).
    AggravatedHandsThisAction,
    /// "This ally cannot gain life" (Aggressive Corpse): set as it enters.
    CannotGainLife,
    /// "This ally can perform actions the turn it is recruited" (Spectral
    /// Servitor).
    ActsWhenRecruited,
    /// "Unlock this vampire if this is their first successful recruit ally
    /// action this turn" (Spectral Servitor): read by the recruit's
    /// resolution.
    UnlockRecruiterOnFirst,
    /// Rider effect: "during your unlock phase, you can burn n pool instead
    /// of burning this ally" (Spectral Servitor [OBL]): the controller's
    /// choice (`Flow::RiderTerms`).
    BurnSelfUnlessPool(u8),
    /// Move X blood from the target minion to the host, as life (The Vozhd
    /// of Juiz de Fora; X the terms).
    StealBloodToHostX,
    /// Add n blood or life to the target minion, not to exceed starting
    /// life (a vampire's capacity; Seraphina).
    BloodToTargetToStart(u8),
    /// Move n blood from the host to the target minion, not to exceed its
    /// starting life (Sri Sansa).
    HostBloodToTarget(u8),
    /// Move n blood or life from the target minion to the host, blood above
    /// its capacity draining as usual (Sri Sansa); an ally left with no
    /// life burns.
    TargetBloodToHost(u8),
    /// The host enters combat with the acting minion, which acts first, the
    /// combat over returning to the after-resolution window (The Guardian;
    /// ANK 20221102).
    EnterCombatWithActing,
    /// "Your prey chooses a ready minion they control; the chosen minion
    /// takes n unpreventable damage" (Gnaeus Aemilius Augustinus): chosen
    /// as it resolves, none if they have none.
    PreyChoosesMinionDamaged(u8),
    /// The acting minion gets -n stealth (Screamer).
    ReduceStealth(u8),
    /// Unlock this card — an ally's own card unlocks the ally (The Vozhd
    /// of Szczecin).
    UnlockSelf,
    /// The action card costs n less blood or pool at resolution (Szlachta
    /// Assistant).
    ReduceActionCost(u8),
    /// The target minion burns n blood ("a Hecata you control can burn 1
    /// blood to …", Bone Shambler [OBL]).
    TargetBurnsBlood(u8),
    /// An ally version's own type line ("[ANI] As above, but this ally has
    /// 2 life and 1 strength", Underbridge Stray).
    Version(crate::library::Body),
    /// "After this ally enters play, burn an ally or retainer you control"
    /// (War Ghoul): an interrupt for the controller's pick.
    EntryBurnAllyOrRetainer,
    /// "Unlock this vampire during the next discard phase" (Feral Hound
    /// [ani], on the employ action's success).
    UnlockActorAtDiscard,
    /// Burn n blood from the host of this card in play (an ability's cost:
    /// Bowl of Convergence [AUS]).
    HostBurnBlood(u8),
    /// An ally's action "that costs n life" (Rotting Behemoth): paid at
    /// resolution like a blood cost, but not one — an ally pays it.
    PayLife(u8),
    /// "After this ally enters play, burn it unless you remove an ally or
    /// vampire in your ash heap from the game" (Rotting Behemoth): an
    /// interrupt for the controller's choice.
    EntryRemoveFromAshOrBurn,
    /// A granted action "that costs n blood": paid by the acting minion at
    /// resolution (fizzles if unpayable by then, rules-qa 3.6).
    PayBlood(u8),
    /// A granted bleed "gets +n bleed if the target Methuselah controls no
    /// ready unlocked minions", read at resolution (Codex of the Edenic
    /// Groundskeepers).
    BleedIfTargetHasNoReadyUnlocked(u8),
    /// On a successful granted burn: the card's host is locked and does
    /// not unlock as normal during its controller's next unlock phase
    /// (Stolen Police Cruiser), if it is still `who`.
    LockHostSkipNextUnlock(Bearer),
    /// Burn 1 blood from the host of this card in play to unlock it.
    HostBurnBloodToUnlock,
    /// Move n blood from the host of this card to its controller's pool
    /// (Blood Doll, Vessel).
    HostBloodToPool(u8),
    /// Move n pool from the host's controller to the host (Blood Doll,
    /// Vessel).
    PoolToHostBlood(u8),
    /// The target vampire gains n blood from a hunting ground (capped at
    /// capacity; it counts as fed by a hunting ground this turn either
    /// way, rules-qa 6.5).
    HuntingGroundBlood(u8),
    /// Burn the card in play chosen by the play's `BurnNamedInPlay` terms
    /// (Vessel), if it is still in play.
    BurnChosenInPlay,
    /// Unlock the actor.
    UnlockActor,
    /// Unlock the target minion (Alabástrom).
    UnlockTarget,
    /// Flip a coin: on tails the announced action fails at once, with no
    /// block and no cost (Evan Klein).
    CoinFlipActionFails,
    /// Reveal the top card of the user's library, which stays on top: the
    /// host burns `burn` blood if it is a master card, else it gets
    /// `stealth` stealth, and only while stealth is needed (Gathii, ANK
    /// 20231031).
    RevealTopCard {
        burn: u8,
        stealth: u8,
    },
    /// The vampire whose text it is goes to torpor (Cedrick Calhoun).
    GoToTorpor,
    /// Lock the target minion.
    LockTarget,
    /// The acting minion's controller discards the chosen count
    /// (`Terms::Count`) of cards at random from hand, replaced as normal
    /// (Ilonka).
    ActingControllerDiscardsAtRandom,
    // Combat effects (`combat`).
    /// Choose this strike for the actor.
    Strike(StrikeKind),
    /// Maneuver now (determine range step): toggle the range.
    Maneuver,
    /// Press now (press step): continue, or end the opponent's press.
    Press,
    /// Bank an optional press for this round's press step.
    OptionalPress,
    /// Bank an optional maneuver for this round ("this round, this vampire
    /// gets 1 optional maneuver", Obedient Flesh [DOM][PRO]).
    OptionalManeuver,
    /// Allies get n intercept for the rest of the action (Obedient Flesh
    /// [dom][pro]: "allies get -1 intercept").
    AlliesIntercept(i8),
    /// Pay a counter's worth of the prey's unlock rider: 1 pool, or a library
    /// card at random from their ash heap removed from the game, for each
    /// counter on this card (The Gate of Acheron): announced as a count.
    PoolOrAshCardsPerCounter,
    /// Move the target vampire to its owner's uncontrolled region, as
    /// banished (Pit of Contemplation).
    TargetToUncontrolled,
    /// If another round occurs, the actor gets 1 optional maneuver then.
    NextRoundManeuver,
    /// Additional strikes this round; `limited`: one such source per
    /// combatant per round (rulebook "Additional Strikes").
    AdditionalStrikes {
        n: u8,
        limited: bool,
    },
    /// The actor gets a strength of n this combat (replaces the base).
    SetStrength(u8),
    /// The actor's hand strikes are aggravated this round.
    AggravatedHands,
    /// The actor's hand damage is aggravated for the rest of the combat
    /// (Bone Spur [PRO]).
    AggravatedHandsThisCombat,
    /// Strike rider: "damage from this strike cannot be prevented by cards
    /// requiring Fortitude" — once it lands, the opposing minion cannot
    /// play Fortitude prevention cards this round, even to cycle them (LSJ
    /// 20050628). Follows the `Strike` it rides in the mode's effects.
    FortitudeProofStrike,
    /// Strike rider: "the opposing vampire's strikes with weapons inflict
    /// no damage this round" — once it lands (a dodge stops it, LSJ
    /// 20040928), its own resolution included (ANK 20170519).
    NullifyOpposingWeapons,
    /// "This combat, the opposing minion takes nR environmental damage each
    /// round during normal strike resolution" (Carrion Crows): dodge or
    /// not; a strike: combat ends comes first.
    EnvironmentalDamage(u8),
    /// Strike rider: "this strike cannot be dodged" (Dust Up [ani]): a dodge
    /// may still be played, it has no effect (LSJ 20030902-2).
    UndodgeableStrike,
    /// Strike rider: "hand strike, aggravated" (Sculpt the Flesh [pro]).
    AggravatedStrike,
    /// "… with first strike" (Donnybrook [obf]): the chosen strike
    /// resolves before a normal one (rulebook "First Strike") — after a
    /// strike: combat ends, a dodge still protecting from it.
    FirstStrike,
    /// Strike rider: "put this card on the opposing minion (with n
    /// counters)" as the strike lands (Sculpt the Flesh [PRO], Touch of
    /// Oblivion [obl]); the card is held until then, else discarded.
    PutOnOpponentOnHit {
        counters: u8,
    },
    /// "After combat ends, put this card on this vampire" (Pass Through
    /// Shadow [OBL]): held; lost if the combat continues (RTR 20020501).
    PutOnActorAfterCombat,
    /// "This round, this vampire gets +n strength" (Bear's Skin).
    StrengthThisRound(u8),
    /// "Can prevent n damage" this round, at will (Bear's Skin).
    PreventThisRound(u8),
    /// "Can prevent n damage each round" this combat (Bear's Skin [ANI]).
    PreventEachRound(u8),
    /// "Gains 1 optional press" usable this round only (Form of the Wolf
    /// [PRO]; TOM 19960521).
    PressThisRound,
    /// "Burn n blood" as the card's effect, not its cost (Rolling with the
    /// Punches [FOR]; ANK 20210226).
    ActorBurnsBlood(u8),
    /// "Prevent all damage from the opposing minion's strikes this round"
    /// (Rolling with the Punches [FOR]).
    PreventOpposingStrikesThisRound,
    /// A strike "with 1 optional maneuver, only usable to get to close
    /// range" (Slam [POT]): at strike declaration, the strike; played at
    /// the maneuver (determine range), a maneuver to close range that
    /// commits this strike as the round's initial strike (`Side.
    /// committed_card`; window review: maneuver riders commit early).
    CommittedStrike(StrikeKind),
    /// "This combat, the opposing minion cannot maneuver to close range,
    /// press to continue combat or use equipment" (Terror Frenzy [ani]): a
    /// frenzy effect on that minion (Tranquility Shield cancels it).
    OpposingRestraint,
    /// "This combat, combat cards cost the opposing vampire +n blood"
    /// (Terror Frenzy [ANI]): end-of-round cards included (ANK 20191221).
    OpposingCombatCardsCost(u8),
    /// "This combat, frenzy cards cannot be used on this vampire; cancel the
    /// effects of frenzy cards that have already been used on this vampire
    /// this combat" (Tranquility Shield).
    FrenzyShield,
    /// "This combat, if a damage is successfully inflicted on this vampire
    /// in a given round, any additional damage inflicted on this vampire in
    /// the same round is automatically prevented"; `aggravated`: aggravated
    /// damage too (Flesh of Marble [PRO]).
    FleshOfMarble {
        aggravated: bool,
    },
    /// "Both combatants and each retainer on them take 1 unpreventable
    /// environmental damage before range is determined each round this
    /// combat"; `escalate`: +1 in each subsequent round (Weather Control
    /// [THA]).
    WeatherDamage {
        escalate: bool,
    },
    /// Prevent up to n damage to the target minion or retainer in combat
    /// (Martyr's Resilience [aus][for]), from a minion out of it.
    PreventOnTarget(u8),
    /// As `PreventOnTarget`, X+n, X the play's terms ([AUS][FOR]).
    PreventOnTargetXPlus(u8),
    /// "Prevent n non-aggravated damage inflicted on any other minion in
    /// combat" (Randall): the target's normal damage only.
    PreventNonAggravatedOnTarget(u8),
    /// Equip the actor with the card its terms named, from hand, paying its
    /// cost — the terms' X of it in blood (Contraband [OBF]); not an action
    /// (LSJ 20080815), so no action's cost reduction applies (LSJ
    /// 20090115-1).
    EquipNamed,
    /// "This round, this vampire gets 1 optional maneuver, only usable to
    /// get to close range" (Angel's Gift).
    OptionalCloseManeuver,
    /// "If this action is blocked, all damage inflicted on vampires during
    /// the resulting combat is aggravated" (Dawn Operation), environmental
    /// and retainer damage included (PIB 20150603-1); with `cancel`, "if a
    /// vampire is currently attempting to block, they can cancel their block
    /// attempt" ([for]).
    DawnOperation {
        cancel: bool,
    },
    /// "If this vampire does not block this action, put this card on them
    /// after action resolution" (Dogged Pursuit [ANI]): one per vampire.
    PutOnActorIfNoBlock,
    /// "If this vampire blocks, put this card on the acting minion; you
    /// still control this card" (Melange [AUS]).
    PutOnActingIfBlocks,
    /// "If they do not block this action, they burn n blood before action
    /// resolution" (Forced Awakening): blocks made before it do not count
    /// (ANK 20230319).
    BurnIfNoBlock(u8),
    /// "If this vampire is blocked, the blocking minion's controller burns
    /// n pool before block resolution" (Aelswith, Terrifying Visage).
    BlockerControllerBurnsPool(u8),
    /// "Minions must burn n blood to attempt to block this action" (Seeds
    /// of Terror): blood only — an ally has none to pay.
    BlockTollBlood(u8),
    /// Lock the minion attempting to block (Hedonism).
    LockAttempting,
    /// Queue a combat between this vampire and the minion attempting to
    /// block (Hedonism [for]/[pre]): it begins after block resolution, or
    /// after a successful action's effects (rules-qa 53.3).
    QueueCombatWithAttempting,
    /// "If the bleed is successful, put this card on this vampire after
    /// action resolution" (Fever Pitch [FOR]/[PRE]); one per vampire.
    PutOnActorIfBleedSuccessful,
    /// "If the bleed is successful, this vampire can burn n blood to draw 1
    /// card from your crypt" (Enthrall [OBF]/[PRE]): `Choice::DrawCrypt`
    /// after resolution.
    CryptForBloodIfBleedSuccessful(u8),
    /// Forward Momentum's counters: +1 after a successful bleed of one's
    /// path vampire, -1 after an unsuccessful one (`Hook::OwnBleedEnds`).
    BleedOutcomeCounters,
    /// This card's pending bleed gain added now (Forward Momentum's unlock,
    /// before its counters are burned).
    TakeBleedGain,
    /// "Remove two copies of the same card in your prey's ash heap from the
    /// game to hunt with +n hunt" (Giovanni del Georgio): the card named as
    /// the action was announced, both copies removed at resolution — gone,
    /// the action fizzles — then the hunt.
    PreyAshPairHunt(u8),
    /// Rulebook "Slave": "if a <clan> controlled by the same Methuselah is
    /// blocked, the controller can lock the slave to cancel the combat,
    /// unlock the acting vampire and have the slave enter combat with the
    /// blocker instead" — the host takes the combat (`ActionCtx::slave`).
    SlaveTakesCombat,
    /// After the end of its controller's minion phase, the host burns and
    /// the top card of their crypt moves to their uncontrolled region
    /// (Tupdog; `Hook::MinionPhaseEnd`).
    BurnHostCryptTopOut,
    /// "Minions with any corruption counters get -1 intercept" this action
    /// (Sense the Sin [DAI]).
    CorruptedIntercept(i8),
    /// Strike rider: "any effect which would make damage from this strike
    /// aggravated leaves the damage normal instead" (Psychic Assault).
    NeverAggravatedStrike,
    /// Strike rider: "if this striking vampire is Black Hand, the damage
    /// from this strike is unpreventable" (Psychic Assault).
    UnpreventableIfBlackHand,
    /// The card entering play gets counters (blood) equal to its target
    /// vampire's capacity, read as it resolves (Powerbase: Barranquilla).
    CountersFromTargetCapacity,
    /// "The acting vampire cannot cast any votes or ballots in this
    /// referendum" (Confusion of the Eye [OBF]).
    ActingAbstains,
    /// "If the referendum requires a titled vampire, the referendum fails"
    /// (Confusion of the Eye [OBF]): a title requirement on the calling card.
    FailIfTitleRequired,
    /// "For the remainder of the combat, each time you replace a card …,
    /// draw an additional card and discard down to your hand size"
    /// (Infernal Pursuit [CEL]): its player's, copies adding up.
    ExtraDrawPerReplacementThisCombat,
    /// Baltimore Purge's unlock-phase rider: the card burned, a vampire not
    /// older than its host chosen per other Methuselah, all to torpor.
    Purge,
    /// "Add enough life to … retainer to reach their starting life"
    /// (Renewed Vigor [obe]): the target retainer's counters up to its
    /// version's starting life, zombie bonuses read now.
    RetainerToStartingLife,
    /// The card's host moves from torpor to the ready region (Renewed Vigor
    /// [for]: "this vampire can burn this card to move from torpor").
    HostLeavesTorpor,
    /// "Burn X counters from this card to reduce a bleed against you by X"
    /// (Illusory Resources): the X of the ability's terms.
    ReduceBleedX,
    /// "If that action is successful, the anarch gains an additional blood"
    /// (Hospital Food): +n hunt for this action.
    HuntBonusThisAction(u8),
    /// "Put this card … on this vampire" with no one-per-vampire limit
    /// (Focus the Blood): as `PutOnActorNow`, copies stacking.
    PutOnActorNowStacking,
    /// An action card's "hunt at +n hunt" (Abactor): 1 + n blood, hunt
    /// bonuses included, as it resolves.
    HuntAt(u8),
    /// After that hunt, if blood was gained, "a referendum is called to call
    /// a blood hunt on this vampire" — no diablerie (Abactor).
    BloodHuntIfHuntGained,
    /// Burn the top n cards of the player's library (Liquidation).
    BurnLibraryTop(u8),
    /// "Search your library (shuffle afterward) and/or hand for a mage ally
    /// he could play. Put that ally in play with life equal to its starting
    /// life and pay its cost as normal" (Antonio d'Erlette): the host's
    /// pick at resolution (`Interrupt::AllySearch`).
    SearchMageAlly,
    /// "You can gain n pool" (Matasuntha, after a minion in combat with her
    /// leaves the ready region): optional (`Interrupt::MayGainPool`).
    MayGainPool(u8),
    /// "+n hand size until your next discard phase" (Fotini Katsikaris).
    HandSizeUntilDiscard(u8),
    /// "Inflicts n unpreventable damage on each anarch controlled by the
    /// target of the bleed after bleeding successfully" (Alonso Petrodon):
    /// each Anarch vampire of the bled Methuselah, in torpor too.
    DamageBledAnarchs(u8),
    /// "Strike cards cost the acting minion +n blood or life during the
    /// resulting combat if this vampire blocks" (Ensnare a Beast [ANI]).
    StrikeTollIfBlocks(u8),
    /// "Anarchs get -n intercept during this action" (Fiendish Tongue).
    SectInterceptThisAction(Sect, i8),
    /// "If this vampire is Tzimisce and the bleed is successful, this
    /// Tzimisce can burn n blood during your next discard phase to unlock"
    /// (Fiendish Tongue): `Choice::UnlockForBlood` there.
    TzimisceDiscardUnlockIfBleedSuccessful(u8),
    /// "At the end of that combat, if only one combatant is ready, the
    /// controller of the opposing minion burns n pool" (Abuse of Power).
    OpposingBurnsPoolIfOneReadyAfterCombat(u8),
    /// "At the end of that combat, if the opposing vampire is not ready,
    /// you can put this card on this acting vampire" (Pillars Fall): put
    /// whenever it can be, one per vampire.
    PutOnActorIfOpposingNotReadyAfterCombat,
    /// "During your prey's unlock phase, for each vampire in torpor they
    /// control, they burn 1 pool. If your prey controls no vampires in
    /// torpor …, burn this card" (Augury of Doom).
    PreyBurnsPoolPerTorpor,
    /// Put this card on the opposing minion, its player keeping control
    /// (Hunting the Quarry [cel]/[tha]).
    PutOnOpposingKeepControl,
    /// Burn the card in play granting this action as the action is
    /// declared ("burn this card to attempt to …", Hunting the Quarry): not
    /// a cost, an action's costs being paid at resolution (owner,
    /// 2026-10-08: rules-qa 54.4).
    BurnSourceOnDeclare,
    /// "Once this round, this vampire can burn n blood to get 1 additional
    /// maneuver, only usable to get to close range" (Dance with the Devil
    /// [OBF]/[THA]).
    CloseManeuverForBlood(u8),
    /// "Once this round, this vampire may burn n blood for an additional
    /// maneuver" (Quick Meld [PRO]): either way.
    ManeuverForBlood(u8),
    /// "This combat, this vampire's hand strikes inflict +n damage" (Rage
    /// of Apedemak).
    HandStrikeDamageThisCombat(u8),
    /// "This combat, you get +n hand size" (Rage of Apedemak).
    HandSizeThisCombat(u8),
    /// Yawp Court: the chosen Sabbat vampire enters combat with the acting
    /// vampire before the referendum; the acting vampire still ready at its
    /// end, the Sabbat vampire takes 2 environmental damage and the
    /// referendum goes on, else the action ends (rules-qa 54.1).
    YawpCourtCombat,
    /// A combat card put on its vampire as it resolves, usable for the
    /// rest of the combat (Wall of Filth).
    PutOnActorNow,
    /// An aim (Target Vitals): if any damage from the strike is
    /// successfully inflicted, +n damage of its kind, and the opposing
    /// minion cannot press this round (rules-qa 55.1).
    AimExtraDamage(u8),
    /// "They can discard two combat cards to cancel this card as it is
    /// played" (Target Vitals): the opposing minion's controller.
    CancelableByDiscardingCombat(u8),
    /// Morbidity: up to n blood moved from the opposing vampire to this
    /// card in play, back to it after combat as the card burns.
    MorbidityTake(u8),
    /// Trap: the card in play with 3 counters for the combat, reset to 3
    /// each time a card is played in it; as each press step opens, it
    /// burns a counter to provide a press to continue — no combatant's
    /// (LSJ 20040521), before any other press (ANK 20200525-1); burned as
    /// the combat ends.
    TrapPresses,
    /// Conquer the Beast: as Trap, the press this vampire's — mandatory,
    /// or `optional` — and this vampire cannot use equipment this combat.
    BeastPresses {
        optional: bool,
    },
    /// The ash heap ally named at play removed from the game for n life to
    /// the target zombie ally (Putrescent Sustenance).
    EatAshAllyToTarget(u8),
    /// The acting ally removed from the game at resolution, to burn the
    /// target ally or retainer (Heartrender; rules-qa 55.4).
    RemoveActorToBurnTarget,
    /// Look at and reorder the top n cards of the library, then unlock
    /// (Eulogio Sánchez de los Reyes).
    ReorderTopThenUnlock(u8),
    /// "After action resolution, this vampire can burn n blood to unlock
    /// the acting ally" (Gifts From Hereafter [DOM]/[OBL]).
    UnlockActingForActorBlood(u8),
    /// +1 life to the target ally, not above its starting life (Burial
    /// Site Hunting Ground), marking the hunting ground used.
    HuntingGroundLife(u8),
    /// Cursed Abattoir: a counter after a zombie of its controller's is
    /// burned (`Hook::OwnZombieBurned`).
    CounterOnZombieBurned,
    /// The host ally burns n life, a cost of its own text (Gravebound
    /// Drone: "this ally can burn 1 life to …") — not a blood cost, which
    /// an ally never pays (advanced rules 5.5.1).
    HostBurnLife(u8),
    /// The target minion burns n blood or life, an ally its last too
    /// (Gravebound Drone [OBL]: "burn 1 life from another copy").
    TargetBurnLife(u8),
    /// As `EndBlockedAction`, the acting minion left locked (Obedience
    /// [DOM]: "do not unlock the acting vampire").
    EndBlockedActionLocked,
    /// "Continue the action as if unblocked" after the resolution of a
    /// blocked action (Go-getter [OBF]).
    ContinueAfterResolution,
    /// "If the action is successful, this vampire can burn n blood to unlock
    /// after action resolution" (Paths in Two Worlds [OBL]).
    UnlockForBloodIfSuccessful(u8),
    /// Unlock rider: the host burns n blood, all or nothing; unable to, it
    /// is burned; otherwise its controller moves this card to another
    /// justicar or Inner Circle member, or burns it if there is none
    /// (Fear of Mekhet; rules-qa 49.4).
    HostBurnsBloodOrBurnedThenMoves(u8),
    /// Unlock rider: the host burns n blood or is burned, its controller's
    /// choice — burned when it has fewer (Society of Leopold).
    HostBurnsBloodOrIsBurned(u8),
    /// "Stun" the target minion: it locks and does not unlock during its
    /// controller's next unlock phase (Mind Numb).
    Stun,
    /// "Titled vampires cannot block this action" (Propaganda).
    TitledCannotBlock,
    /// "The target Methuselah locks a ready unlocked minion they control"
    /// (Propaganda [PRE]): the one named as the card is played.
    TargetLocksReadyUnlocked,
    /// "Steal a location" (Dominate Kine [DOM]): its control changes; one
    /// on a minion moves onto a minion of the new controller's (RTR
    /// 19960112, 19980623).
    StealLocation,
    /// "Steal a vampire in torpor" (Graverobbing): control changes for
    /// good; the cards on it go with it.
    StealTargetVampire,
    /// "This acting vampire can burn n blood to move the stolen vampire to
    /// your ready region" (Graverobbing [DOM]): a burn, not a cost (ANK
    /// 20210226).
    StolenToReadyForBlood(u8),
    /// Burn the target minion (Judgment: Camarilla Segregation: "burn a
    /// non-Camarilla vampire they control … to burn this card").
    BurnTargetMinion,
    /// Instead of burning the card in play (a granted "burn this card"
    /// action's shape): shuffle it into its owner's library (Aranthebes,
    /// The Immortal).
    ShuffleSourceIntoLibrary,
    /// "If this card has no counters, burn it and move a wraith or zombie
    /// ally from your ash heap to your ready region with life equal to its
    /// starting life" (Split the Veil): the ally put in play, not
    /// recruited — its inferior version, no requirement or cost, able to
    /// act this turn (LSJ 20100204).
    RaiseAllyWhenEmpty(&'static [CardFilter]),
    /// "Move an ally requiring Hecata or Oblivion [obl] from your ash heap
    /// to your ready region, locked with life equal to its starting life"
    /// (Hel-Blá): an ally matching one of these, named as the action is
    /// declared (rules-qa 41.6), put in play like Split the Veil's as it
    /// resolves (LSJ 20100204).
    AllyFromAshLocked(&'static [CardFilter]),
    /// "Remove an ally in any Methuselah's ash heap from the game to gain n
    /// blood (and unlock)" (Psychophagia): the ally named as it was played
    /// (`PlayCtx::ash`), if still there.
    EatAshAlly {
        blood: u8,
        unlock: bool,
    },
    /// "If this card has `at` counters, burn it and gain `pool` pool" (Open
    /// War): its controller gains (rules-qa 32.7).
    AtCountersBurnAndGain {
        at: u8,
        pool: u8,
    },
    /// +1 master phase action this phase (The Parthenon).
    GainMasterPhaseAction,
    /// Burn X pool (X from the ability's terms), paid down to 0 ousts
    /// (Oxford University, England).
    PayPoolX,
    /// The Methuselah's own `per` × X votes, on one side (Oxford
    /// University, England: "+2X votes").
    MethuselahVotesX {
        per: u8,
        favor: bool,
    },
    /// The target vampire gains blood to its capacity (Giant's Blood).
    BloodToTargetFull,
    /// Remove the target vampire from the game; its controller gains pool
    /// equal to its capacity (Golconda: Inner Peace).
    RemoveTargetForCapacityPool,
    /// "Their controller can burn n pool to cancel this card as it is
    /// played" (Golconda: Inner Peace): the target minion's controller.
    CancelableForPoolByTargetController(u8),
    /// Move X blood from the actor to the uncontrolled target (Grooming the
    /// Protégé).
    MoveBloodXToUncontrolledTarget,
    /// The user's minion in the combat gets 1 press only usable to
    /// continue combat (Mob Connections).
    ContinuePressForOwnCombatant,
    /// Draw n cards, then discard down to hand size (Fragment of the Book
    /// of Nod).
    DrawThenDiscardDown(u8),
    /// The attempting minion's controller-given intercept comes with a cost:
    /// if that minion does not block the action, this card's controller
    /// burns 1 pool after resolution (WMRH Talk Radio).
    PoolIfAttemptingDoesNotBlock,
    /// The target vampire "can burn 1 blood to get +1 intercept during that
    /// action", once (The Rumor Mill, Tabloid Newspaper).
    TargetBurnsBloodForIntercept,
    /// Remove X counters from this card (X from the ability's terms).
    RemoveCountersX,
    /// The actor gets +X votes (Alamut).
    AddVotesX,
    /// The acting Methuselah's `clan` vampire successfully bleeding: the
    /// pool lost goes on this card as counters (Alamut).
    CountersFromOwnBleed(&'static str),
    /// Move n blood from the target vampire to its controller's pool
    /// (Powerbase: Munich).
    TargetBloodToPool(u8),
    /// Move n pool to the target vampire (Powerbase: Munich).
    PoolToTargetBlood(u8),
    /// Move 1 counter from this card to the target as blood (Twilight Camp).
    CounterToTargetBlood,
    /// "Search your crypt for 1 copy of him, reveal it, and move it to your
    /// uncontrolled region (shuffle afterward)" (The Mithraeum, London).
    CryptNamedToUncontrolled(&'static str),
    /// Unlock the acting minion (Yoruba Shrine).
    UnlockActing,
    /// The action's card costs n blood less (The Line, during the action).
    ReduceActionBlood(u8),
    /// The action's equipment is paid with X counters of this card (Ravnos
    /// Cache, during the action).
    PayActionWithCountersX,
    /// Move n pool to this card as counters, and add m counters from the
    /// bank (Ravnos Cache).
    PoolToCounterPlusBank {
        pool: u8,
        bank: u8,
    },
    /// Enter play with 1 counter per vampire of this clan the player
    /// controls; with none, burned (Ravnos Carnival).
    EnterWithCountersPerOwnClan(&'static str),
    /// "This card comes into play with 1 counter for each Anarch you
    /// control" (Hackerspace): no burn at 0.
    EnterWithCountersPerOwnAnarch,
    /// The hand cards picked by the ability's terms go on this card, face up
    /// and out of play, not replaced (Fleshforge Chamber).
    StorePickedFromHand,
    /// Its controller's vampires of this clan play the cards on it as if
    /// from the hand (Fleshforge Chamber).
    PlayableFromStore(&'static str),
    /// A card played from this one is removed from the game whenever it
    /// would be burned (The Erciyes Fragments).
    StoredRemovedWhenBurned,
    /// The controller names a library card in their prey's ash heap; it
    /// moves onto this card (The Erciyes Fragments).
    StoreFromPreyAsh,
    /// The referendum fails, its polling over (Yoruba Shrine).
    FailReferendum,
    /// Put 1 of the player's corruption counters on the target (Cave of
    /// Apples); if the player's counters on it reach its capacity (or an
    /// ally's cost), they can burn them to steal it.
    CorruptTargetMayStealAtCapacity,
    /// "That Methuselah locks one of the oldest <clan> they control (if
    /// any)" (Brujah Debate): their pick among the ties; a locked one may
    /// be picked (LSJ 20010508).
    LockOldestOfClan(&'static str),
    /// Put this card on the opposing minion now (Disarm): its layers apply
    /// while it is there, the card itself in the ash heap (rules-qa 21.1).
    PutOnOpponent,
    /// Send the opposing vampire to torpor (Disarm).
    OpponentToTorpor,
    /// "With 1 additional <strike>" (Voracious Vermin [ANI]): an additional
    /// strike only usable for that strike (none if its object is gone);
    /// `limited`: "(limited)" (Wind Dance [THA]).
    AdditionalStrikeOnly {
        kind: StrikeKind,
        limited: bool,
    },
    /// "Burn X blood to get X additional strikes" (Lightning Reflexes
    /// [CEL]): X the play's terms; the blood is an effect, not a cost (ANK
    /// 20210226).
    AdditionalStrikesX {
        limited: bool,
    },
    /// The actor burns X blood, X the play's terms (an effect, not a cost:
    /// no cost reduction reaches it, ANK 20210226).
    ActorBurnsBloodX,
    /// "Use a melee weapon strike. This strike is at +n damage"
    /// (Anticipation [aus]): the target weapon's strike, its riders
    /// included, from that card (spec §4.10.4).
    WeaponStrikePlus(u8),
    /// As `WeaponStrikePlus`, "with 1 optional maneuver" (Lam Into [for]):
    /// played at the maneuver it commits the round's initial strike to that
    /// weapon's (as `CommittedStrike`); at strike declaration, the strike.
    CommittedWeaponStrikePlus(u8),
    /// "It becomes a melee weapon equipment" (Weighted Walking Stick): the
    /// card in play counts as weapon equipment.
    BecomesWeapon,
    /// "This equipment is a weapon, gun" (Zip Gun): a gun once in play
    /// (`cards::is_gun`).
    BecomesGun,
    /// "This vampire does not lock for the block" (Read the Winds [ani]).
    NoLockForBlock,
    /// "The blocking vampire burns n blood" (The Kiss of Ra): the minion
    /// attempting, as much as it has.
    AttemptingBurnsBlood(u8),
    /// "Send the blocking vampire to torpor" (The Kiss of Ra [FOR]).
    AttemptingToTorpor,
    /// "The block attempt is canceled … and the current action is ended
    /// (without combat)" (The Kiss of Ra): the action ends at once, before
    /// resolution — no card is played after it (RTR 20180719), it is
    /// canceled (no NRA mark: it may be taken again, unless it was blocked
    /// then continued before; ANK 20211015), still taken (ANK 20210124).
    CancelAction,
    /// "Each Methuselah who controls any vampires with torture counters
    /// burns n pool" (The Path of Lilith, its controller's unlock phase).
    TortureBurnPool(u8),
    /// "If he or she currently has any, they are burned" (Anachronism):
    /// the target's ranged weapons, `except` the named one.
    BurnRangedWeaponsOnTarget {
        except: &'static str,
    },
    /// Weapon strike rider: "Bearer takes 1 damage during strike
    /// resolution when striking with this gun, but only once each combat"
    /// (Zip Gun): environmental (LSJ 19970801), none if the strike never
    /// resolves (LSJ 20100310).
    BearerDamageOncePerCombat,
    /// Weapon strike rider: "for each damage inflicted by this strike (even
    /// if prevented), burn 1 counter from this card. Burn this card if it
    /// has no counters" (Weighted Walking Stick).
    CountersPerDamage,
    /// Employ the retainer named by the play's terms, from hand, on the
    /// actor: requirements and cost apply, not an action (Pack Alpha [ani]).
    EmployNamed,
    /// "This round, this vampire can strike: <strike>" (Hunger of Marduk):
    /// offered at every strike choice of the round, additional strikes
    /// included (ANK 20220207).
    RoundStrike(StrikeKind),
    /// Prevent up to n damage (aggravated included).
    Prevent(u8),
    /// Prevent up to X+n damage.
    PreventXPlus(u8),
    /// Prevent up to n non-aggravated damage.
    PreventNonAggravated(u8),
    /// Strikes that are not hand strikes cannot be used this round.
    OnlyHandStrikes,
    /// The next round, if any, is at close range (skip determine range).
    NextRoundClose,
    /// Would-end replacement: start a new round instead.
    NewRound,
    /// The actor gains blood equal to what the opposing vampire lost to
    /// damage this round.
    GainBloodLostByOpponent,
    /// "If this vampire was blocked while performing an action, the action
    /// continues as if unblocked" (Torrent [CEL]): mandatory and free, as the
    /// after-combat window closes.
    ContinueFreeAfterCombat,
    /// Rider of a strike: combat ends — after combat, if the actor was
    /// blocked, it may burn `blood` to continue the action with +`stealth`
    /// as if unblocked. Lost if the combat continues (spec §4.9.2).
    ContinueAfterCombat {
        blood: u8,
        stealth: u8,
    },
    /// Rider of a strike: combat ends — after combat ends, if the range is
    /// close, n unpreventable damage on the opposing minion, unless the
    /// strike was dodged or the combat continues (Catatonic Fear [PRE]).
    DamageAfterCombatIfClose(u8),
    /// "After combat ends, if the range is close, stun the opposing minion"
    /// (Kiss of Cathari [OBF]/[PRE]): not if the strike is dodged (LSJ
    /// 20070124) or the combat continues (RTR 20020501).
    StunAfterCombatIfClose,
    /// Set this round's range, skipping the determine range step; usable
    /// only while no other effect has set it (Sniper Rifle; Michael Harris:
    /// "set the range for the round"; the first effect setting it has
    /// priority, PIB 20120214).
    SetRange(crate::combat::Range),
    /// "Immune to all non-aggravated damage from a strike" (Valerie Diaz):
    /// the normal damage pending from the opposing strike is gone — not
    /// prevented.
    ImmuneStrikeNonAggravated,
    /// "… or damaging effect": the normal damage pending from other sources
    /// (environmental, retainers), taken as one effect (rules-qa 38.3).
    ImmuneEffectNonAggravated,
    /// The host's initial strike this round must be this card's strike: a
    /// weapon's maneuver used (rulebook "Determine Range"), Sniper Rifle's
    /// range set. One such commitment a round.
    CommitStrike,
    /// Rider of a weapon's strike, read once the strike has resolved: the
    /// bearer gets 1 optional additional strike (limited) this round, only
    /// usable to strike with this weapon (AK-47).
    AdditionalStrikeWithSource,
    /// Absimiliard's Army: the top card of the user's library in play as a
    /// nameless, textless ghoul ally with this type line (not a play: LSJ
    /// 20040531).
    GhoulFromLibraryTop(crate::library::Body),
    /// Rise of the Nephtali: the acting Independent vampire receives the
    /// unique Independent title of Liaison (4 votes); a younger holder
    /// yields at once, as does a younger claimant (no contest).
    GainLiaison,
    /// Blood Trade: 1 blood from the target, a vampire of the user's, to
    /// a vampire of another Methuselah's, their choice.
    GiveBloodToOthers,
    /// "The chosen location is burned unless its controller burns n pool"
    /// (Conquest of Humanity).
    BurnUnlessControllerPays(u8),
    /// "The chosen vampire takes n unpreventable environmental damage" (The
    /// New Inquisition), out of combat.
    DamageTarget(u8),
    /// Nightmares upon Nightmares' unlock-phase rider: each minion of the
    /// active Methuselah's is locked unless they discard a card — not a
    /// mortal ally, nor a vampire of capacity above the Gehenna events in
    /// play.
    LockUnlessDiscard,
    /// Recalled to the Founder's unlock-phase rider: with 3 or more
    /// vampires of a clan, burn one of them; burning one of capacity 6 or
    /// more, ignore it for the rest of the game.
    BurnOfCrowdedClan,
    /// "After this card enters play, burn all Gehenna events … then send
    /// all vampires with capacity n or less to torpor" (The Fourth Cycle;
    /// no conviction card in V5).
    FourthCycle(u8),
    /// Pay 1 pool per other event in play (The Uncoiling's burn): paid
    /// down to 0, its user is ousted.
    PayPoolPerOtherEvent,
    /// Rider of a weapon's strike, read as it resolves: its damage is
    /// unpreventable (Enhanced Coagulant) — kept apart from the pending
    /// damage, out of every prevention's reach, and added as the damage
    /// resolves.
    UnpreventableStrike,
    /// Rider of a weapon's strike, read as it lands on a vampire: the
    /// weapon moves onto the opposing vampire and becomes its other mode —
    /// a card with no type or printed ability, only that mode's layers and
    /// the riders it carries (Enhanced Coagulant; dodged, it stays where it
    /// was, ANK 20220923).
    BecomesOnOpponent,
    /// "Once each combat, can prevent n damage from gun strikes": one point
    /// a use, the rest kept for later in the combat (rules-qa 10.3).
    PreventGunStrikeAllowance(u8),
    /// Prevent up to n damage from sources other than gun strikes.
    PreventNonGun(u8),
    /// Rider of a weapon's strike, had as it is chosen: 1 optional press
    /// only usable to continue combat, once each combat per copy (Righteous
    /// Blade; kept though the strike is dodged or does not resolve,
    /// rules-qa 10.4).
    /// "Discard n cards at random" from the user's hand (a cost; the cards
    /// are replaced).
    DiscardAtRandom(u8),
    /// "+n strength that combat" (Kasim Bayar): added each round.
    StrengthThisCombat(u8),
    ContinuePressEachCombat,
    /// On success, search the library for an equipment the acting vampire
    /// can equip and equip it — requirements and cost as normal, shuffle
    /// afterward (Magic of the Smith; chosen at resolution, spec §1.14.1).
    SearchEquip,
    /// On success, search the library for a card of this kind, reveal it
    /// and move it to hand, or find none; shuffle afterward (Sakhar's
    /// equipment, Dominica's master archetype).
    SearchToHand(CardFilter),
    /// The target vampire gets +1 vote for each counter on this card
    /// (Powerbase: Madrid).
    TargetVotesPerCounter,
    /// A granted action's effect: burn all the counters from the card
    /// granting it (Powerbase: Madrid).
    RemoveAllSourceCounters,
    /// Día de los Muertos: the first referendum a Sabbat vampire of the
    /// player's calls this turn passes automatically.
    AutoPassFirstSabbatReferendum,
    /// Minions burn n blood or life to attempt to block this action
    /// (Unthinkable Humiliation).
    BlockToll(u8),
    /// Every minion gets n intercept this action (Unthinkable Humiliation
    /// [POT][PRE]: -1).
    MinionsIntercept(i8),
    /// "If a Sabbat vampire diablerizes this vampire, move this card to the
    /// diablerist (before the blood hunt is called)" (Regent): a marker the
    /// diablerie reads.
    MovesToSabbatDiablerist,
    /// Add n blood to each vampire of the sect in the player's uncontrolled
    /// region, its printed sect (Magnetic Authority [DOM]/[POT]).
    BloodToEachUncontrolledOfSect(Sect, u8),
    /// +(mul × X + plus) votes to the vampire playing it (Awe: "+X+1",
    /// "+2X+1"), X its blood cost.
    AddVotesXLinear {
        mul: u8,
        plus: u8,
    },
    /// Each non-Camarilla vampire gets -n votes in this referendum, votes
    /// already cast included; a vampire's votes stop at 0 (Camarilla
    /// Conclave; ANK 20251125).
    NonCamarillaVotesMinus(u8),
    /// Each vampire not following the path gets -n votes in this
    /// referendum, as `NonCamarillaVotesMinus` (Absolute Tyranny).
    NotPathVotesMinus(&'static str, u8),
    /// Only vampires of the sect cast votes or ballots in this referendum
    /// (Private Audience: "non-Sabbat vampires cannot").
    BarNonSectVotes(Sect),
    /// Business Pressure: a round where Methuselahs burn pool for votes, one
    /// at a time — `per` votes a pool, `own` for the card's player.
    PoolForVotesRound {
        per: u8,
        own: u8,
    },
    /// The card is removed from the game once played, whatever happens to
    /// its action or referendum, canceled too (Reinforcements).
    RemovedOncePlayed,
    /// The host takes an action, any it may (Enkil Cog): its Methuselah
    /// picks it (`Flow::CogAction`).
    TakeAnyAction,
    /// The printed action is mandatory while its requirements hold: taken
    /// before any other action, and a vampire that cannot take it is stuck
    /// and takes none (spec §3.9, Elen Kamjian).
    MandatoryAction,
    /// On success, the acting minion enters combat with the target, the
    /// combat part of the action (spec §4.1.2).
    EnterCombatWithTarget,
    /// On success, the target card moves to the acting minion (a steal:
    /// not played nor equipped, so no requirement is checked, spec
    /// §3.7.3.2).
    StealTargetCard,
    /// On success, the target vampire yields its contested title (Ephor):
    /// the title is lost, its rivals keep theirs.
    TargetYieldsTitle,
    /// The target minion is locked if unlocked, else unlocked ("lock or
    /// unlock", Lord Tremere).
    ToggleLockTarget,
    /// Unleash Hell's Fury put in play: it can attempt to block actions
    /// directed at its controller as a 9-capacity infernal vampire, +2
    /// intercept, 0 blood (a token minion, out of play but for its
    /// attempts).
    FuryEnters,
    /// The card just put on the acting vampire takes as many counters as
    /// its capacity (Call the Great Beast's X ritual counters).
    CountersFromActorCapacity,
    /// With 11 counters or more, Call the Great Beast rises: off its Baali,
    /// which burns, into its controller's ready region as the Great Beast.
    GreatBeastAtEleven,
    /// Soul Gem of Etrius: its controller draws their crypt's top card; a
    /// younger vampire enters their ready region with the Gem and blood to
    /// capacity, another goes to their uncontrolled region and the Gem
    /// burns.
    SoulGemDraw,
    /// On success, a copy of the library card named at declaration moves
    /// from its Methuselah's ash heap to their hand (The Sargon Fragment),
    /// the card granting the action still on the acting vampire.
    AshCardToHandNamed,
    /// "Gain n pool after action resolution if this action is successful"
    /// (I am Legion [DAI][OBF]): a fizzled action is successful.
    PoolIfSuccessful(u8),
    /// The other cards so labeled on the target minion burn, once this one
    /// is put on it (Condemnation: Mute).
    BurnOthersLabeledOnTarget(crate::library::Label),
    /// The block attempt under way fails, and the blocking minion's
    /// Methuselah cannot attempt to block this action again (Crocodile's
    /// Tongue).
    AttemptFailsBarSeat,
    /// "The blocking minion can burn n blood or life to cancel this card as
    /// it is played" (Crocodile's Tongue).
    CancelableForBlockerBlood(u8),
    /// "You get +n hand size until your next unlock phase" (Juliet Parr).
    HandSizeUntilUnlock(u8),
    /// The target vampire gets a disease counter, one at most (Vampiric
    /// Disease).
    DiseaseTarget,
    /// On success, the vampire of the uncontrolled region named at
    /// declaration is removed from the game, and the target one gets n
    /// blood (Malgorzata); either gone, nothing happens.
    RemoveUncontrolledForBlood(u8),
    /// On success, the card granting the action moves to the target
    /// vampire (Society of Leopold).
    MoveSourceToTarget,
    /// On success, the target card is burned.
    BurnTargetCard,
    /// "Steal up to n blood from" the target (François Villon G2): the
    /// amount chosen at resolution (owner, rules-qa 62.4), among what the
    /// target has (`Interrupt::StealUpTo`).
    StealBloodUpTo(u8),
    /// On success, move up to n blood from the target vampire to the acting
    /// one — as much as there is (rulebook "Target") — the excess over its
    /// capacity draining off.
    StealBloodFromTarget(u8),
    /// The vampire does not unlock as normal during its next unlock phase.
    SkipNextUnlock,
    /// Exchange a card from hand for a library card in the ash heap
    /// (Lenelle): both named as the action is announced (LSJ 20000222),
    /// exchanged on success if both are still there.
    ExchangeHandForAsh,
    /// On success, equip with the first equipment found working down from
    /// the top — requirements and cost as normal, shuffle afterward (Vast
    /// Wealth): one that cannot be placed or paid for in blood is burned
    /// without cost (TOM 19950407); a pool cost that ousts is paid (RTR
    /// 19941109).
    EquipFirstFound,
    /// Move the top n cards of the controller's library onto this card,
    /// out of play — as many as there are (Shilmulo Tarot, on equip and in
    /// its unlock ability; rules-qa 11.2).
    StoreTopCards(u8),
    /// As the card resolves, once it is replaced (rules-qa 12.4), search the
    /// library (shuffle afterward) for up to n non-unique equipment cards,
    /// one at a time, and put them on this card; the card enters contest
    /// only then (spec §1.13.2), and is burned at once if it holds none
    /// (Black Market Cache; rules-qa 11.3). Applied by `resolve_play`.
    SearchStore(u8),
    /// Rider of a draw alternative: "if this card has no cards on it, burn
    /// it", checked as its last card is drawn (Black Market Cache).
    BurnSelfWithoutStored,
    /// "Draw up to X cards without discarding and then move the same number
    /// of cards from your hand to the bottom of your library": X capped by
    /// the library as the ability resolves (Heart of Nizchetus; rules-qa
    /// 9.5, 11.1).
    DrawThenBottom,
    // Referendum effects (`referendum`).
    /// The political action card's referendum (with `DeclareAction`).
    CallReferendum(&'static ReferendumDef),
    /// The actor gets +n votes in this referendum (cast on the same side as
    /// votes it already cast).
    AddVotes(u8),
    /// Each vampire of the user's the grant names gets its votes in this
    /// referendum (Elysium: The Palace of Versailles, Power Structure).
    GrantVotes(VoteGrant),
    /// The user's own votes, cast at once on one side ("lock this card …
    /// to get +3 votes", Ventrue Headquarters): a vote source of their
    /// own, like the Edge.
    MethuselahVotes {
        n: u8,
        favor: bool,
    },
    /// As `AddVotes`, +1 per ready vampire of this sect the player controls,
    /// counted as it resolves (Aisha az-Zahra).
    AddVotesPerReadySect(Sect),
    /// The target vampire abstains: its cast votes and ballots are
    /// canceled and it casts no more; `lock` locks it, `burn` burns blood.
    Abstain {
        lock: bool,
        burn: u8,
    },
    /// The actor gets +n votes against the referendum (and votes against).
    AddVotesAgainst(u8),
    /// Methuselahs casting votes or ballots against burn n pool once the
    /// results are tallied (Scorn of Adonis; once per Scorn per Methuselah).
    TallyPenaltyAgainst(u8),
    /// Non-Camarilla vampires cannot cast votes or ballots (Closed Session).
    BarNonCamarillaVotes,
    /// Cancel the referendum: no tally; the political action card returns
    /// to its owner's hand (discard down afterward). `unlock_acting`:
    /// unlock the acting vampire; `forbid_repeat`: the acting Methuselah's
    /// minions cannot perform the same political action again this turn.
    CancelReferendum {
        unlock_acting: bool,
        forbid_repeat: bool,
    },
    /// After a passed referendum: the actor gains 1 blood per vote of the
    /// margin, X of them to the player's pool instead (Voter Captivation).
    GainBloodPerMargin,
    /// "For each vote by which the referendum passed, distribute 1 blood
    /// from the blood bank among the ready <clan> you control and your
    /// pool. A vampire cannot gain more than 1 blood this way. You cannot
    /// gain more than `pool` pool this way or more than `titled_pool` if
    /// this acting <clan> is titled" (Amici Noctis): chosen as it resolves,
    /// the pool first; what finds no room is lost.
    DistributeMargin {
        clan: &'static str,
        pool: u8,
        titled_pool: u8,
    },
    /// Reduce the current bleed by n (never below 0).
    ReduceBleed(u8),
    /// "You still control this card" (Puppet Master [DOM]): on another
    /// Methuselah's minion, the card stays its player's (a marker read by
    /// `GameState::card_controller`).
    StillYours,
    /// "Unlock the attached vampire and take control of them until the end
    /// of your turn" (Puppet Master [DOM]): the host, for the card's
    /// controller.
    TakeControlOfHost,
    /// This mode is a frenzy card ("[ANI] Frenzy.", Deep Song): a marker for
    /// the cards reading it (Meditative Grove, Tranquility Shield).
    Frenzy,
    /// "Enter combat with and lock a vampire. The target vampire is
    /// considered the acting minion during that combat" (Deep Song [ANI]):
    /// with `EnterCombatWithTarget`, the target locks as the combat begins
    /// and takes the acting minion's side.
    TargetActsInCombat,
    /// What the acting minion gets in the combat the card action's
    /// resolution begins ("this acting vampire gets 1 optional maneuver
    /// during that combat", Fleetness [CEL]).
    InResultingCombat(BlockRider),
    /// "Look at your prey's hand and discard one card of your choice from
    /// it" (Revelations [aus]): the target Methuselah's; they replace it
    /// unseen (LSJ 20100119).
    DiscardFromTargetHand,
    /// Steal n pool from the target Methuselah (Line Brawl [cel]).
    StealPoolFromTarget(u8),
    /// The controller of the target card burns n pool (Rewilding: "burn a
    /// location and burn 2 pool from its controller").
    TargetControllerBurnsPool(u8),
    /// The opposing combatant's controller burns n pool (Hell-for-Leather
    /// [obt]).
    OpposingControllerBurnsPool(u8),
    /// "The target Methuselah discards n cards of their choice" (Shroud of
    /// Decay [obl], if the bleed is successful).
    TargetMethuselahDiscards(u8),
    /// "Remove `remove` cards in your prey's ash heap from the game to burn
    /// `pool` of their pool" (Shroud of Decay [OBL]): the cards chosen by
    /// the player as it resolves (LSJ 20090710).
    RemovePreyAshToBurnPool {
        remove: u8,
        pool: u8,
    },
    /// Ability cost "remove n cards in your ash heap from the game"
    /// (Marchesa Liliana): the user's picks, one card at a time, as the
    /// ability is used (`Interrupt::RemoveFromAsh` on their own ash heap).
    RemoveOwnAshCards(u8),
    /// Unlock the host of this card in play ("this vampire can lock this
    /// card to unlock", Rutor's Hand); not under "cannot unlock".
    UnlockHost,
    /// Lock the card this play put in play ("put this card on this vampire,
    /// locked", Rutor's Hand).
    LockPlayedCard,
    /// The acting minion takes `damage` unpreventable environmental
    /// aggravated damage; with `avoid_for` > 0 it can burn that much blood
    /// to be immune instead (Rutor's Hand [THA]).
    ActorTakesAggravated {
        damage: u8,
        avoid_for: u8,
    },
    /// "Prevent all damage from the opposing minion's strike" (Touch of
    /// Valeren [COMBAT]): the damage pending from it, on the player's
    /// minion in the combat.
    PreventStrikeDamage,
    /// "Prevent N damage from an opposing minion's strike" (Lodin): up to
    /// N of the strike damage pending, aggravated first.
    PreventStrike(u8),
    /// This printed action is ability n's, on another kind of target: one
    /// action for NRA ("burn 1 corruption counter or a card …", Aniel).
    SameActionAs(u8),
    /// The target minion loses 1 corruption counter — whose, if several
    /// Methuselahs' are on it, chosen as the action resolves (Aniel).
    BurnCorruptionFromTarget,
    /// "Once this action, you can burn 1 of your corruption counters from a
    /// blocking minion to have their block attempt fail, and they cannot
    /// attempt to block this action again" (Enchanting Gaze [PRE][PRO]):
    /// one use per card, `Choice::BurnCorruptionToFail`.
    FailBlockForCorruption,
    /// "If the bleed is successful, this vampire can burn n of your
    /// corruption counters from a minion controlled by the target Methuselah
    /// to unlock" (Revelation of the Serpent [PRE][PRO]):
    /// `Choice::BurnCorruptionToUnlock` after resolution.
    UnlockForCorruption(u8),
    /// A bleed action card's effects "if the bleed is successful" (for 1
    /// or more): applied as it resolves, after the pool is burned.
    IfBleedSuccessful(&'static [Effect]),
    /// "You can lock a minion controlled by the target Methuselah", chosen
    /// as it resolves (Break the Bonds [obf]; ANK 20240706).
    LockMinionOfTarget,
    /// "Put 1 corruption counter on a minion controlled by the target
    /// Methuselah", chosen as it resolves (The Platinum Protocol [pro]; ANK
    /// 20240706).
    CorruptMinionOfTarget,
    /// Put this card on the minion playing it (Heart of the City: "put
    /// this card on this vampire").
    PutOnActor,
    /// As `PutOnActor`, the card remembering the Methuselahs the current
    /// action is directed at (`CardInPlay::against`; Spying Mission [OBF],
    /// Shadow Cast [OBL]).
    PutOnActorRemembering,
    /// What the actor gets in the combat its block (or the block of its
    /// action) begins, for each such block this action (LSJ 20010813,
    /// 20010814-2): "if this vampire blocks …".
    IfBlocks(BlockRider),
    /// The actor burns n blood as an effect, not a cost (Second Tradition:
    /// Domain: "this vampire burns 1 blood to unlock"; ANK 20210226).
    BurnActorBlood(u8),
    /// The target minion unlocks, attempts to block and gets +n intercept
    /// for the rest of the action (Organized Resistance: "unlock a locked
    /// Anarch you control; that Anarch attempts to block with +1
    /// intercept").
    TargetAttemptsBlock(u8),
    /// "During this action, this minion can burn 1 blood to get +1
    /// intercept" (Eyes of the Wild): `Choice::BurnBloodForIntercept`.
    BurnForInterceptThisAction,
    /// "Lock this minion after action resolution if they did not block
    /// this action" (Eyes of the Wild).
    LockIfNoBlock,
    /// "Reduce the acting minion's stealth to 0 (the acting minion can
    /// still increase their stealth)": a fixed -X, X the stealth as played
    /// (RTR 20030519), nothing at 0 or less (LSJ 20021211).
    ReduceStealthToZero,
    /// The current block attempt fails; that minion cannot attempt to
    /// block this action again (Forced Confessional [DOM]).
    AttemptFails,
    /// For the rest of the action, minions without Oblivion burn 1 blood
    /// to attempt to block, allies cannot (Where the Veil Thins [OBL]); one
    /// attempting may pay or resign (LSJ 20031121-2).
    VeilThins,
    /// Minions who attempt to block this action and fail become locked
    /// before action resolution (Faceless Night [OBF]).
    LockFailedBlockers,
    /// The Methuselah whose master card this cancels gets +1 master phase
    /// action — at the start of their next master phase if the canceled
    /// card is an out-of-turn master (Wash); not a trifle's bonus (ANK
    /// 20170124).
    CanceledGetsMasterAction,
    /// "The blocking minion's controller can burn n pool to cancel this
    /// card as it is played" (True Love's Face [OBF][PRE]):
    /// `Choice::PayToCancel` in the as-played window.
    CancelableForPool(u8),
    /// The actor attempts to block the action — already attempting, it
    /// keeps its attempt (ANK 20181122-2).
    AttemptBlock,
    /// The actor, if an ally, does not unlock as normal during its next
    /// unlock phase (On the Qui Vive).
    AllySkipsNextUnlock,
    /// The target minion wakes for the rest of the action (Shadow
    /// Sentinel [OBL]).
    WakeTarget,
    /// +X bleed for the current bleed, X the play's (Monkey Wrench);
    /// `limited` binds the rulebook cap.
    BleedModX {
        limited: bool,
    },
    /// `then` (a bleed or intercept bonus for the rest of the action)
    /// counts while `requires` holds of the minion `on`, read live whenever
    /// the stat is ("with an additional +1 bleed if this vampire is
    /// Toreador", Aire of Elation; "if the acting minion is titled",
    /// Protection Racket: ANK 20190416).
    If {
        on: On,
        requires: &'static [Requirement],
        then: &'static Effect,
    },
    /// Allies and vampires younger than the actor get this intercept
    /// modifier for the rest of the action (Perfect Paragon [PRE]).
    YoungerIntercept(i8),
}

/// Trigger points where riders of cards in play fire (mandatory
/// effects in canonical order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hook {
    /// "During their unlock phase": a mandatory effect the active
    /// Methuselah resolves among their own unlock-phase effects, in the
    /// order they choose, and before the impulse leaves them (spec §2.4.4;
    /// rules-qa 6.1). Its condition is read when it resolves (spec §5.2.2,
    /// Anarch Revolt).
    UnlockPhase,
    /// The turn ends (Loup: "burn this card at the end of the turn").
    EndOfTurn,
    /// A minion of the clan performs a successful action (Loup: "after a
    /// Toreador performs a successful action"), as the action ends.
    ClanActionSuccessful(&'static str),
    /// The host of this card went to torpor (Guardian Angel; Fame), fired
    /// at once where the torpor happened (rules-qa 6.4).
    HostToTorpor,
    /// "During your discard phase" (The Coven): a mandatory effect fired as
    /// the impulse leaves the active Methuselah at the end of the phase.
    DiscardPhase,
    /// A ready minion was burned or sent to torpor (Tension in the Ranks),
    /// whoever controls it; a minion removed from the game with its ousted
    /// Methuselah is not burned (spec §1.10), nor is one burned from
    /// torpor or from a contest.
    ReadyMinionBurnedOrTorpor,
    /// The controller would replace a card: the rider's effects happen instead of the draw
    /// (Visit from the Capuchin). Every replacement draw counts, whatever
    /// took the card out of hand (spec §6.8.3), a delayed one when it
    /// finally comes due included.
    WouldReplace,
    /// The card's host was blocked ("if this minion is blocked …, burn
    /// this vehicle"): applied where the block resolves, before combat or
    /// any other effect (ANK 20220116).
    HostBlocked,
    /// The controller would draw a card from their library: a draw
    /// alternative ("you can draw one of those cards instead") offered for
    /// each card drawn, while the library holds one (rules-qa 9.4). The
    /// rider's effects apply as a card is taken from this card.
    WouldDraw,
    /// An action targeting this vampire was announced (Evan Klein): fired
    /// as the as-announced window closes, before any block.
    ActionAnnouncedAtTarget,
    /// A minion in combat with this vampire left the ready region (Egidia
    /// Arrú): fired with `ReadyMinionBurnedOrTorpor`, even if the vampire
    /// left in the same event (advanced rules §6.4.4).
    CombatOpponentLeftReady,
    /// The referendum of this vampire's political action was canceled or
    /// failed (Cedrick Calhoun): applied as the action resolves, before the
    /// after-resolution window.
    OwnReferendumFailed,
    /// After a referendum the vampire called passes (Juliet Parr), as its
    /// political action resolves.
    OwnReferendumPassed,
    /// Its bearer, a vampire, is burned (Soul Gem of Etrius): read on the
    /// card on it, before it burns with it.
    BearerBurned,
    /// After any referendum passes, a blood hunt included (Condemnation:
    /// Mute [DAI]): on a card in play, its host read.
    AnyReferendumPassed,
    /// This vampire successfully hunted: blood was gained (Adisa; a hunt
    /// action gaining none is not a successful hunt, spec §3.7.2.1).
    SuccessfulHunt,
    /// Another Gehenna event was played (Wormwood, Fueled by Heart's
    /// Blood: "burn 1 counter from this card").
    GehennaPlayed,
    /// The host blocked an action (Rebel): fired as the block succeeds,
    /// before block resolution; conditions read on the acting minion and
    /// the action.
    HostBlocks,
    /// The host announces an action (Slaughtering the Herd [DOM]).
    HostAnnouncesAction,
    /// The card's controller's prey is ousted (Khabar: Glory: "you gain 4
    /// additional pool").
    PreyOusted,
    /// This ally is burned ("if she is burned, shuffle her into her
    /// owner's library", Saatet-ta): a replacement of where its card goes —
    /// still burned, so burn effects apply (ANK 20200813); not a discard
    /// (RTR 20010711).
    SelfBurned,
    /// A minion with a contract for which a vampire of the card's
    /// controller is chosen is about to leave the ready region (The Black
    /// Throne: "lock this card … to gain 1 pool"): optional, the card
    /// unlocked.
    ContractHostLeaves,
    /// During each Methuselah's master phase, a mandatory effect ordered
    /// among the phase's others, a copy played in the phase included
    /// (Brujah Debate; `Engine::master_dues`).
    MasterPhaseStart,
    /// A vampire in combat with a vampire of `clan` its controller controls
    /// left the ready region (Dead Pool: "you can add 1 counter").
    CombatOpponentOfOwnClanLeft(&'static str),
    /// A non-Camarilla vampire its controller controls sent an opposing
    /// vampire to torpor in combat (The Path of Lilith: "you may lock this
    /// card to put a torture counter on the opposing vampire").
    OwnNonCamarillaSendsToTorpor,
    /// A vampire from the controller's uncontrolled region entered play
    /// (Twilight Camp).
    OwnVampireInfluenced,
    /// The controller's vampire successfully bled (Alamut).
    OwnBleedSuccessful,
    /// A bleed action of the controller's minion is over (resolved,
    /// blocked or failed), before its after-resolution window (Forward
    /// Momentum); with the riders' requirements read on the acting minion.
    OwnBleedEnds,
    /// A zombie ally (or retainer) of the controller's is burned (Cursed
    /// Abattoir).
    OwnZombieBurned,
    /// The minion that played this card (`CardInPlay::by`) leaves the
    /// ready region: to torpor, burned (Slaughtering the Herd [DOM]).
    ByLeavesReady,
    /// An Anarch comes into play under the controller's control, or a
    /// vampire they control becomes Anarch (Hackerspace: "add 1 counter").
    OwnAnarchArrives,
    /// The host is about to successfully bleed the Methuselah this card
    /// remembers (`CardInPlay::against`) for 1 pool or more (Spying Mission
    /// [OBF]): fired as the bleed resolves, after every modifier and
    /// reaction (LSJ 19980105), each such card at once (ANK 20200609).
    HostAboutToBleedRemembered,
    /// The referendum of any Methuselah's political action passed, read
    /// on a ready vampire after all its effects (Lutz von Hohenzollern).
    PoliticalReferendumPassed,
    /// The controller's minion phase ended, read on their vampires in play,
    /// torpid ones included (Tupdog).
    MinionPhaseEnd,
}

/// Whose turn a phase rider fires on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RiderScope {
    /// Every Methuselah's ("during their unlock phase": Anarch Revolt).
    Every,
    /// The controller's only ("during your unlock phase": Creeping
    /// Sabotage).
    Controller,
    /// Every other Methuselah's (Smiling Jack).
    Others,
    /// The controller's prey's ("during your prey's unlock phase", The Gate
    /// of Acheron).
    Prey,
}

/// A mandatory effect of a card in play, fired at its hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rider {
    pub hook: Hook,
    pub scope: RiderScope,
    /// Host predicates read when the rider is collected and again when it
    /// resolves ("if this vampire is in torpor", Fame); an unmet rider is
    /// not pending.
    pub requires: &'static [Requirement],
    pub effects: &'static [Effect],
}

impl RiderScope {
    pub fn applies(self, st: &GameState, controller: SeatId, active: SeatId) -> bool {
        match self {
            RiderScope::Every => true,
            RiderScope::Controller => controller == active,
            RiderScope::Others => controller != active,
            RiderScope::Prey => controller != active && active == st.prey(controller),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModeDef {
    pub windows: &'static [Window],
    pub actor: Actor,
    pub requires: &'static [Requirement],
    pub target: Option<TargetSpec>,
    pub terms: Option<Terms>,
    /// "Even if stealth is not yet needed": waives `StealthNeeded`.
    pub waives_needed: bool,
    /// "A vampire can play only one X each …" (`Limit`).
    pub limit: Option<Limit>,
    pub effects: &'static [Effect],
}

/// When a card played with a "do not replace until" clause is replaced:
/// at the first moment the condition is met, ahead of any other effect
/// (spec §1.9.2). A canceled card is replaced at once (§1.9.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Until {
    /// "Until your next unlock phase": before cards unlock.
    NextUnlockPhase,
    /// "Until your next discard phase": as the phase begins.
    NextDiscardPhase,
    /// "Until after this action" (Elder Intervention): with the action's
    /// other held cards, at its end.
    EndOfAction,
    /// "Until after combat" (Lam Into): as the combat ends.
    AfterCombat,
    /// "Until a Methuselah is ousted" (Absimiliard's Army).
    MethuselahOusted,
    /// "Until a vampire commits diablerie".
    Diablerie,
    /// "As long as this card is in play": as it leaves play.
    LeavesPlay,
    /// "Until a titled vampire goes to torpor" (The New Inquisition).
    TitledToTorpor,
    /// "Until a vampire successfully hunts" (Restricted Vitae).
    SuccessfulHunt,
    /// "Until a vampire moves from torpor to the ready region" (Torpid
    /// Blood).
    LeavesTorpor,
    /// "Until a vampire becomes a liaison" (Rise of the Nephtali).
    Liaison,
}

/// A mode of `actor` in `windows`: no target, terms or limit (spread
/// `..mode_of(..)` under the fields that differ).
pub const fn mode_of(
    windows: &'static [Window],
    actor: Actor,
    requires: &'static [Requirement],
    effects: &'static [Effect],
) -> ModeDef {
    ModeDef {
        windows,
        actor,
        requires,
        target: None,
        terms: None,
        waives_needed: false,
        limit: None,
        effects,
    }
}

/// A target of `kind` passing `filter`.
pub const fn target(kind: TargetKind, filter: &'static [Requirement]) -> Option<TargetSpec> {
    Some(TargetSpec { kind, filter })
}

/// `+n bleed`, limited or not.
pub const fn bleed(n: u8, limited: bool) -> Effect {
    Effect::BleedMod { n, limited }
}

/// A card's defaults, spread as `..CARD` under its name and modes.
pub const CARD: CardDef = CardDef {
    name: "",
    trifle: false,
    out_of_turn: false,
    delayed_replacement: None,
    modes: &[],
    abilities: &[],
    riders: &[],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CardDef {
    /// Printed name; matched against `library::LIBRARY`.
    pub name: &'static str,
    /// Trifle: refunds a master phase action (rulebook "Trifle").
    pub trifle: bool,
    /// Out-of-turn master card (rulebook "Out-of-turn"; rules-qa 3.3).
    pub out_of_turn: bool,
    /// "Do not replace until …" (Old Friends, Mirror Walk).
    pub delayed_replacement: Option<Until>,
    /// Playable modes (from hand).
    pub modes: &'static [ModeDef],
    /// In-play abilities (the same shape; actor is the host, or a ready
    /// unlocked minion declaring a granted action).
    pub abilities: &'static [ModeDef],
    /// Riders of the card in play: mandatory effects fired at a hook.
    pub riders: &'static [Rider],
}

/// Is library card `card` an ally matching one of `of` (any ally, if
/// empty)? The allies an ash heap offers (Split the Veil, Hel-Blá).
pub fn ash_ally(card: u16, of: &[CardFilter]) -> bool {
    LIBRARY[card as usize].types.contains(&CardType::Ally)
        && (of.is_empty() || of.iter().any(|f| f.matches(card)))
}

/// The printed strike of a melee weapon (Anticipation's "melee weapon
/// strike"): its strike ability's `Melee` kind, if it has one.
pub fn melee_strike(card: u16) -> Option<(usize, StrikeKind)> {
    let def = lookup(card)?;
    def.abilities.iter().enumerate().find_map(|(i, m)| {
        m.effects.iter().find_map(|e| match e {
            Effect::Strike(k @ StrikeKind::Melee { .. }) => Some((i, *k)),
            _ => None,
        })
    })
}

/// The damage of a weapon's regular strike against a generic opponent
/// (RTR 19980623): a strength 1 hand, its printed bonus; whether any strike
/// of it is aggravated. None for a weapon without a damaging strike.
pub fn regular_strike_damage(card: u16) -> Option<(u8, bool)> {
    let def = lookup(card)?;
    def.abilities
        .iter()
        .flat_map(|m| m.effects.iter())
        .filter_map(|e| match e {
            Effect::Strike(StrikeKind::Damage { n, aggravated, .. }) => Some((*n, *aggravated)),
            Effect::Strike(StrikeKind::Melee { bonus, aggravated }) => {
                Some((1 + bonus, *aggravated))
            }
            _ => None,
        })
        .reduce(|(a, x), (b, y)| (a.max(b), x || y))
}

/// A card that becomes weapon equipment once in play (Weighted Walking
/// Stick).
pub fn becomes_weapon(card: u16) -> bool {
    lookup(card).is_some_and(|d| {
        d.modes
            .iter()
            .any(|m| m.effects.contains(&Effect::BecomesWeapon))
    })
}

/// A ranged weapon: a weapon with a ranged strike (LSJ 20060825: "all
/// weapons that provide ranged strikes", Zip Gun included).
pub fn is_ranged_weapon(card: u16) -> bool {
    is_weapon(card)
        && lookup(card).is_some_and(|d| {
            d.abilities.iter().flat_map(|m| m.effects.iter()).any(|e| {
                matches!(
                    e,
                    Effect::Strike(
                        StrikeKind::Damage { ranged: true, .. }
                            | StrikeKind::StealBlood { ranged: true, .. }
                            | StrikeKind::RangedBurnEquipment
                    )
                )
            })
        })
}

/// A gun: printed so, or one once in play (Zip Gun, `Effect::BecomesGun`).
pub fn is_gun(card: u16) -> bool {
    LIBRARY[card as usize]
        .labels
        .contains(&crate::library::Label::Gun)
        || lookup(card).is_some_and(|d| {
            d.modes
                .iter()
                .any(|m| m.effects.contains(&Effect::BecomesGun))
        })
}

/// A weapon: printed so, or one once in play (`becomes_weapon`).
pub fn is_weapon(card: u16) -> bool {
    LIBRARY[card as usize]
        .labels
        .contains(&crate::library::Label::Weapon)
        || becomes_weapon(card)
}

/// The title a title card represents (`Effect::RepresentsTitle` in one of
/// its modes), if it is one.
pub fn card_title(card: u16) -> Option<Title> {
    lookup(card)?.modes.iter().find_map(|m| {
        m.effects.iter().find_map(|e| match e {
            Effect::RepresentsTitle(t) => Some(*t),
            _ => None,
        })
    })
}

/// The library mode (printed Discipline level, windows) of definition mode
/// `mi`: the same index, except where a printed mode offers a choice the
/// definition splits ("+1 bleed (limited), or +1 stealth and +1 bleed
/// (limited)", Revelation of the Serpent: the stealth option only when
/// stealth is needed).
/// A Discipline tag's identity, whatever its level ("obl" for "OBL").
pub fn discipline_key(tag: &str) -> &'static str {
    const KEYS: &[&str] = &[
        "abo", "ani", "aus", "cel", "chi", "dai", "dem", "dom", "fli", "for", "mel", "myt", "nec",
        "obe", "obf", "obl", "obt", "pot", "pre", "pro", "qui", "san", "ser", "spi", "str", "tem",
        "tha", "thn", "val", "vic", "vis", "mal", "flight", "vin",
    ];
    let lower = tag.to_ascii_lowercase();
    KEYS.iter().copied().find(|k| *k == lower).unwrap_or("?")
}

/// A non-electronic, non-vehicle, non-weapon equipment card (British
/// Museum), locations in play included (ANK 20220824).
pub fn plain_equipment(card: u16) -> bool {
    use crate::library::Label;
    let c = &LIBRARY[card as usize];
    c.types.contains(&CardType::Equipment)
        && ![Label::Electronic, Label::Vehicle, Label::Weapon, Label::Gun]
            .iter()
            .any(|l| c.labels.contains(l))
}

pub fn library_mode(card: u16, mi: usize) -> usize {
    match LIBRARY[card as usize].name {
        "Revelation of the Serpent" => [0, 0, 1, 1][mi],
        // [CEL] or [PRE]: the bleed alone, or with the stealth.
        "Subversion" => [0, 1, 1][mi],
        // Each version for blood, or for a zombie ally's life.
        "Putrescent Sustenance" => [0, 0, 1, 1][mi],
        // [DOM]: a younger vampire you control, or one in your uncontrolled
        // region — one target, of either kind.
        "Feast of the Soul's Secrets" => [0, 1, 1][mi],
        // Each version against a block, or against the combat the action
        // itself begins (rules-qa 55.7).
        "Obedience" => [0, 1, 0, 1][mi],
        // [aus]: a hand strike, or a melee weapon's strike.
        "Anticipation" => [0, 0, 1][mi],
        // Each Discipline: a hand strike, or a melee weapon's strike.
        "Lam Into" => [0, 0, 1, 1, 2, 2][mi],
        // [DAI][OBF]: the cancel and the pool, or either.
        "I am Legion" => [0, 1, 2, 2][mi],
        // [obe]'s life: an ally, or a retainer (a card target).
        "Renewed Vigor" => [0, 1, 2, 2, 3][mi],
        // [ABO]: in combat, or after a directed action.
        "Predator's Transformation" => [0, 1, 2, 2][mi],
        // Each version on a minion or on a retainer.
        "Martyr's Resilience" => [0, 0, 1, 1][mi],
        // "To gain 3 blood or to gain 2 blood and unlock".
        "Psychophagia" => [0, 0][mi],
        // The card it becomes, never played.
        "Enhanced Coagulant" => [0, 0][mi],
        _ => mi,
    }
}

/// Behavior of a library card, if implemented. A card without a definition
/// is unplayable (but can be discarded); coverage grows as defs land.
/// Is `p` an action card whose target is a minion or card of `seat`'s with
/// `label` (Mobile HQ)? Played from hand or as if from it (Fleshforge
/// Chamber, LSJ 20060106) — a card Piper names has no as-played window
/// (RTR 20001020). Engine-checked in the as-played window.
pub fn played_action_card_at(
    st: &GameState,
    seat: SeatId,
    label: crate::library::Label,
    p: &PlayCtx,
) -> bool {
    // A political action card's terms choose, never target (rules-qa
    // 38.6).
    let action_card = lookup(p.card).is_some_and(|d| {
        d.modes[p.mode as usize].effects.iter().any(|e| {
            matches!(e, Effect::DeclareAction(k) if *k != crate::decision::ActionKind::Political)
        })
    });
    let labeled = |card: u16| LIBRARY[card as usize].labels.contains(&label);
    action_card
        && match p.target {
            Target::Minion(m) => {
                let v = &st.minions[m.index()];
                v.owner == seat
                    && matches!(v.kind, crate::state::MinionKind::Ally { card, .. }
                        if labeled(card))
            }
            Target::Card(c) => st.card_controller(c) == seat && labeled(st.cards[c.index()].card),
            _ => false,
        }
}

/// A card in play that became another ("it becomes a card with …",
/// Enhanced Coagulant): its mode with no window. Typeless — no equipment,
/// no weapon, no printed ability — only that mode's layers and the riders
/// it carries apply.
pub fn became(card: u16, mode: u8) -> bool {
    lookup(card).is_some_and(|d| {
        d.modes
            .get(mode as usize)
            .is_some_and(|m| m.windows.is_empty())
    })
}

/// Equipment once in play: an equipment card that is not a location
/// ("while in play, this card does not count as equipment", Living Manse).
pub fn is_equipment_in_play(card: u16) -> bool {
    let c = &LIBRARY[card as usize];
    c.types.contains(&CardType::Equipment) && !c.labels.contains(&crate::library::Label::Location)
}

pub fn lookup(card: u16) -> Option<&'static CardDef> {
    // By LIBRARY index, built once: a name search on every call was the
    // engine's hottest path.
    static INDEX: std::sync::OnceLock<Vec<Option<&'static CardDef>>> = std::sync::OnceLock::new();
    INDEX.get_or_init(|| {
        LIBRARY
            .iter()
            .map(|c| defs::DEFS.iter().find(|d| d.name == c.name).copied())
            .collect()
    })[card as usize]
}

/// Whom a vampire's printed modifier applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrintedTo {
    /// The vampire itself.
    Itself,
    /// The minion opposing it ("Brujah get +1 strength in combat with
    /// him").
    Opponent,
    /// The other minions its controller controls ("primogen you control
    /// get +1 stealth").
    OthersControlled,
    /// Every other minion, any Methuselah's ("while she is ready, Inner
    /// Circle members get -2 bleed", Mary Anne Blaire).
    Others,
}

/// How many times a printed modifier counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Per {
    One,
    /// "For each unique equipment attached to him" (uncontested).
    UniqueEquipmentOn,
    /// "For each ready titled vampire controlled by other Methuselahs"
    /// (Gerald Windham).
    ReadyTitledOfOthers,
}

/// A modifier printed on a vampire, read live by `GameState::printed_mod`
/// while the vampire is in play — in torpor included (LSJ 20011022), not
/// while contested.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Printed {
    pub stat: Stat,
    pub n: i8,
    pub to: PrintedTo,
    pub per: Per,
    pub requires: &'static [Requirement],
}

/// "<Vampire> can play cards requiring a <clan> as if he were a <clan>"
/// (Abraham Mellon); with `methuselah`, "you and <vampire> can" (Lady
/// Scarlett Churchill): its Methuselah's cards too.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlaysAs {
    pub clan: &'static str,
    pub methuselah: bool,
}

/// A vampire's defaults, spread as `..VAMP` under its name.
pub const VAMP: VampireDef = VampireDef {
    name: "",
    adv: false,
    group: None,
    printed: &[],
    abilities: &[],
    riders: &[],
    plays_as: None,
    incomplete: None,
};

/// Behavior of a vampire's printed text (`crypt.rs` stays the generated
/// printed data: sect, title, disciplines).
#[derive(Debug)]
pub struct VampireDef {
    /// Printed name; matched against `crypt::CRYPT`.
    pub name: &'static str,
    /// The advanced card of the name (`CryptCard::adv`): its text differs
    /// from the base card's.
    pub adv: bool,
    /// The version of the name, when several share it (Theo Bell G2, G6):
    /// matched against `CryptCard::group`; `None` matches any.
    pub group: Option<u8>,
    pub printed: &'static [Printed],
    /// Abilities printed on the vampire, in the card vocabulary: the actor
    /// `Host` is the vampire itself.
    pub abilities: &'static [ModeDef],
    /// Mandatory effects printed on the vampire, fired at their hooks
    /// (host predicates read on the vampire).
    pub riders: &'static [Rider],
    /// The clan it plays cards requiring as if it were of it.
    pub plays_as: Option<PlaysAs>,
    /// The printed text not implemented yet, if any: a vampire lands with
    /// part of its text and ends with all of it (rules-qa 13.1).
    pub incomplete: Option<&'static str>,
}

/// Whether the card in `mode` leaves the game once played, whatever
/// happens to it (`Effect::RemovedOncePlayed`, Reinforcements).
pub fn removed_once_played(card: u16, mode: u8) -> bool {
    lookup(card).is_some_and(|d| {
        d.modes
            .get(mode as usize)
            .is_some_and(|m| m.effects.contains(&Effect::RemovedOncePlayed))
    })
}

/// Behavior of a minion's printed text: a vampire's `VampireDef` (an ally's
/// text is its card's `CardDef`).
pub fn vampire_of(m: &crate::state::Minion) -> Option<&'static VampireDef> {
    // The Great Beast's text is its card's.
    if let crate::state::MinionKind::GreatBeast { .. } = m.kind {
        return Some(&vampires::GREAT_BEAST);
    }
    m.crypt_card().and_then(vampire)
}

/// Does crypt card `card` print `stat` unconditionally (Jimmy Dunn's
/// "cannot be contested", read off the card out of play)?
pub fn prints(card: u16, stat: Stat) -> bool {
    vampire(card).is_some_and(|d| d.printed.iter().any(|p| p.stat == stat && p.n > 0))
}

/// The Disciplines the Great Beast may take (rulebook "Disciplines"),
/// superior.
pub const GREAT_BEAST_DISCIPLINES: &[&str] = &[
    "ABO", "ANI", "AUS", "CEL", "CHI", "DAI", "DEM", "DOM", "FOR", "MEL", "MYT", "NEC", "OBE",
    "OBF", "OBT", "POT", "PRE", "PRO", "QUI", "SAN", "SER", "SPI", "TEM", "THA", "THN", "VAL",
    "VIC", "VIS",
];

/// Behavior of a crypt card, if its printed text is implemented. A vampire
/// without a definition plays with no text.
pub fn vampire(card: u16) -> Option<&'static VampireDef> {
    // Read on every stat computation: indexed once by crypt card.
    static TABLE: std::sync::OnceLock<Vec<Option<&'static VampireDef>>> =
        std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        crate::crypt::CRYPT
            .iter()
            .map(|c| {
                vampires::VAMPIRES
                    .iter()
                    .find(|d| {
                        d.name == c.name && d.adv == c.adv && d.group.is_none_or(|g| g == c.group)
                    })
                    .copied()
            })
            .collect()
    })[card as usize]
}

/// The card play under announcement / in its as-played window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayCtx {
    pub seat: SeatId,
    pub card: u16,
    pub mode: u8,
    /// The minion playing the card (modifiers, reactions, action cards).
    pub actor: Option<MinionId>,
    pub target: Target,
    pub x: u8,
    /// The card in play chosen by `Terms::BurnNamedInPlay`.
    pub burn: Option<CardId>,
    /// A card in hand and the mode it is used with, named by the terms
    /// (`Terms::EmployFromHand`).
    pub named: Option<(u16, u8)>,
    /// A vampire the card chooses as it is played (`Terms::OwnOfClan`:
    /// Priority Contract's Assamite, Toreador Grand Ball's second
    /// Toreador).
    pub chosen: Option<MinionId>,
    /// A card in a Methuselah's ash heap named as the card is played
    /// (`Terms::AnyAshAlly`, Psychophagia); the card `u16::MAX` while only
    /// the ash heap is chosen.
    pub ash: Option<(SeatId, u16)>,
    /// The cost reductions announced with the card: the source in play and
    /// the amount (The Line, The Shard, Ravnos Carnival, Ravnos Cache).
    pub discounts: [Option<(CardId, u8)>; 3],
    /// The vampire whose blood pays 1 of the card's cost, announced with the
    /// reductions (Regina Blake).
    pub blood_payer: Option<MinionId>,
    /// The card in play the card was played from, not the hand (Fleshforge
    /// Chamber): it is not replaced.
    pub from: Option<CardId>,
    /// Nergal's once-each-turn blood off, chosen for this card (rules-qa
    /// 60.7).
    pub once_off: bool,
}

impl PlayCtx {
    /// A play with no cost source, burn, name, chosen minion or found card.
    pub fn new(
        seat: SeatId,
        card: u16,
        mode: u8,
        actor: Option<MinionId>,
        target: Target,
        x: u8,
    ) -> Self {
        PlayCtx {
            seat,
            card,
            mode,
            actor,
            target,
            x,
            burn: None,
            named: None,
            chosen: None,
            ash: None,
            discounts: [None; 3],
            blood_payer: None,
            from: None,
            once_off: false,
        }
    }
}

/// Does `m` meet a mode's discipline tags? Lower case needs the discipline
/// at any level, upper case needs superior; a combo card needs every tag,
/// any other card needs one (its text lists alternatives).
pub fn has_disciplines(st: &GameState, m: MinionId, card: u16, tags: &[&str]) -> bool {
    // An ally has no Disciplines — but one that plays cards requiring a
    // basic Discipline "as a vampire" meets that one (The Vozhd of Juiz de
    // Fora, Spectral Servitor): at basic level only, one requirement of a
    // card, never two at once (LSJ 20011217; ANK 20180111).
    let vampire = st.minions[m.index()].is_vampire();
    let disc = if vampire {
        &[][..]
    } else {
        st.plays_as_vampire(m).map_or(&[][..], |g| g.disciplines)
    };
    let tags: Vec<&&str> = tags
        .iter()
        .filter(|t| t.len() == 3 && t.chars().all(|c| c.is_ascii_alphabetic()))
        .collect();
    if tags.is_empty() {
        return true;
    }
    let has = |t: &str| {
        if vampire {
            st.has_discipline(m, t)
        } else if t.chars().all(|c| c.is_ascii_uppercase()) {
            disc.contains(&t)
        } else {
            let upper = t.to_ascii_uppercase();
            disc.iter().any(|d| *d == t || *d == upper)
        }
    };
    // A combo card, or a multi-Discipline card's mode (the set of
    // Disciplines used, LSJ 20011204-3): every tag.
    let lib = &LIBRARY[card as usize];
    if lib.combo || lib.labels.contains(&crate::library::Label::MultiDiscipline) {
        tags.iter().all(|t| has(t))
    } else {
        tags.iter().any(|t| has(t))
    }
}

/// Evaluate a target-independent requirement for `seat` playing with
/// `actor`, given the as-played context and the current action.
#[allow(clippy::too_many_arguments)]
pub fn holds(
    req: Requirement,
    st: &GameState,
    seat: SeatId,
    actor: Option<MinionId>,
    played: Option<&PlayCtx>,
    action: Option<&ActionCtx>,
    combat: Option<&CombatCtx>,
    referendum: Option<&ReferendumCtx>,
    waives_needed: bool,
) -> bool {
    // An actor out of the combat plays for its Methuselah's combatant
    // (`Actor::OwnOutOfCombat`).
    let side = combat.and_then(|c| {
        actor
            .and_then(|m| c.side_of_minion(m))
            .or_else(|| actor.and(c.side_of(seat)))
    });
    let political = referendum.filter(|r| r.kind == RefKind::Political);
    match req {
        Requirement::Controlled
        | Requirement::BloodAtLeast(_)
        | Requirement::NotActingController
        | Requirement::NotSelf
        | Requirement::HasCastVotes
        | Requirement::NotAbstaining
        | Requirement::NotControlled
        | Requirement::TargetLocked
        | Requirement::TargetVampire
        | Requirement::TargetDiablerieSinceYourLastTurn
        | Requirement::BloodAtMost(_)
        | Requirement::TargetCombatant
        | Requirement::TargetSeatOusting
        | Requirement::CanSaveTarget
        | Requirement::BoonTargetActive
        | Requirement::TargetCard(_)
        | Requirement::TargetOnReadyMinion
        | Requirement::ActorMayHoldTarget
        | Requirement::DirectedAtRemembered
        | Requirement::TargetCorrupted
        | Requirement::TargetOnOpposingCombatant
        | Requirement::TargetAllyCard(_)
        | Requirement::CardControlledByPrey => true, // target or card predicates
        Requirement::PlayedCardIsMaster => {
            played.is_some_and(|p| LIBRARY[p.card as usize].types.contains(&CardType::Master))
        }
        Requirement::PlayedByOtherSeat => played.is_some_and(|p| p.seat != seat),
        Requirement::PlayedCardIsMinionCard => {
            // Neither a master card nor an event.
            played.is_some_and(|p| {
                let t = LIBRARY[p.card as usize].types;
                !t.contains(&CardType::Master) && !t.contains(&CardType::Event)
            })
        }
        Requirement::PlayedByPredatorOrPrey => played.is_some_and(|p| {
            p.seat != seat && (p.seat == st.predator(seat) || p.seat == st.prey(seat))
        }),
        Requirement::NotOwnTurn => st.active != seat,
        Requirement::DirectedAtYou => action.is_some_and(|a| a.directed_seat(st) == Some(seat)),
        Requirement::IsBleed => action.is_some_and(|a| a.kind == ActionKind::Bleed),
        Requirement::IsHunt => action.is_some_and(|a| a.kind == ActionKind::Hunt || a.is_hunt),
        Requirement::IsDiablerie => action.is_some_and(|a| a.kind == ActionKind::Diablerie),
        Requirement::ActorNotActing => !action.is_some_and(|a| actor == Some(a.acting)),
        Requirement::OwnActingAllyOrYounger => action.is_some_and(|a| {
            a.seat == seat
                && (st.minions[a.acting.index()].is_ally()
                    || actor.is_some_and(|h| st.younger_than(a.acting, h)))
        }),
        Requirement::IsRecruitOrEmploy => {
            action.is_some_and(|a| matches!(a.kind, ActionKind::Recruit | ActionKind::Employ))
        }
        Requirement::ActionRequiresPath(path) => action.is_some_and(|a| {
            let card = a.card.map(|p| p.card).or(match a.source {
                Some(AbilitySource::Card(c)) => Some(st.cards[c.index()].card),
                _ => None,
            });
            card.is_some_and(|c| LIBRARY[c as usize].path == Some(path))
        }),
        Requirement::ActingAllyOf(labels) => action.is_some_and(|a| {
            st.minions[a.acting.index()]
                .kind
                .ally_card()
                .is_some_and(|c| {
                    labels
                        .iter()
                        .any(|l| LIBRARY[c as usize].labels.contains(l))
                })
        }),
        Requirement::FoughtByYoungerActing => action.is_some_and(|a| {
            a.state == ActionState::Declined
                && !a.failed
                && actor.is_some_and(|m| {
                    a.combat_target(st) == Some(m)
                        && st.minions[m.index()].is_vampire()
                        && st.minions[a.acting.index()].is_vampire()
                        && st.younger_than(a.acting, m)
                })
        }),
        Requirement::BlockingYoungerActing => action.is_some_and(|a| {
            a.blocker.is_some()
                && a.blocker == actor
                && st.minions[a.acting.index()].is_vampire()
                && actor.is_some_and(|m| st.younger_than(a.acting, m))
        }),
        Requirement::ActorAllyOf(labels) => actor.is_some_and(|m| {
            st.minions[m.index()].kind.ally_card().is_some_and(|c| {
                labels
                    .iter()
                    .any(|l| LIBRARY[c as usize].labels.contains(l))
            })
        }),
        Requirement::AttemptingAllyOf(labels) => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => {
                Some(m) != actor
                    && st.minions[m.index()].owner == seat
                    && st.minions[m.index()].kind.ally_card().is_some_and(|c| {
                        labels
                            .iter()
                            .any(|l| LIBRARY[c as usize].labels.contains(l))
                    })
            }
            _ => false,
        }),
        Requirement::NoAimYet => combat.is_some_and(|c| {
            actor.and_then(|m| c.side_of_minion(m)).is_some_and(|i| {
                let s = &c.sides[i];
                s.aim == 0 && s.chosen && s.strike.is_some() && !s.aim_declined
            })
        }),
        Requirement::OwnCombatantOtherCopy => true, // a host predicate
        Requirement::PreyBurnedPool => action.is_some_and(|a| a.prey_burned),
        Requirement::ActingFollowsPath(path) => action.is_some_and(|a| st.follows(a.acting, path)),
        Requirement::NoQueuedCombat => !action.is_some_and(|a| a.queued_combat.is_some()),
        // The target counted as seated: a bleed ousting the prey was
        // against the prey (rules-qa 61.2).
        Requirement::BleedAgainstPrey => action.is_some_and(|a| {
            a.kind == ActionKind::Bleed
                && a.bleed_target
                    .is_some_and(|t| st.prey_counting(a.seat, t) == t)
        }),
        Requirement::BleedSuccessful => action.is_some_and(|a| a.bleed_successful),
        Requirement::BleedAmountAtLeast(n) => action.is_some_and(|a| a.bleed_amount(st) >= n),
        Requirement::BlocksDeclined => action.is_some_and(|a| a.state == ActionState::Declined),
        Requirement::StealthNeeded => {
            waives_needed
                || action.is_some_and(|a| {
                    matches!(a.state, ActionState::Attempt(_))
                        && a.attempt_intercept(st) >= a.stealth(st)
                })
        }
        Requirement::InterceptNeeded => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => actor == Some(m) && a.stealth(st) > a.intercept_of(st, m),
            _ => false,
        }),
        Requirement::LockedActor => actor.is_some_and(|m| st.minions[m.index()].locked),
        Requirement::MasterPhaseActionLeft => st.mpa_left(seat) > 0,
        Requirement::HasEdge => st.seats[seat.index()].has_edge,
        Requirement::HandNotEmpty => !st.seats[seat.index()].hand.is_empty(),
        Requirement::CryptNotEmpty => !st.seats[seat.index()].crypt.is_empty(),
        // Only master cards requiring the clan print it: a vampire playing
        // as one for its Methuselah counts (Lady Scarlett Churchill).
        Requirement::ControlsReadyClan(clan, n) => {
            st.minions_of(seat)
                .filter(|&m| st.minions[m.index()].ready() && st.meets_clan(m, &[clan], true))
                .count()
                >= n as usize
        }
        Requirement::OwnReadyClanWithBlood(clan) => st.minions_of(seat).any(|m| {
            let v = &st.minions[m.index()];
            v.ready() && v.clan() == Some(clan) && v.blood > 0
        }),
        Requirement::TargetIsHost
        | Requirement::ActorIsHost
        | Requirement::ActorIsChosen
        | Requirement::PlayedFrenzyOnYour(_)
        | Requirement::PlayedFrenzyOnActor
        | Requirement::NotPlayedThisGame
        | Requirement::NotPlayedThisTurn
        | Requirement::OnePerMethuselah
        | Requirement::PlayedGrappleOrAimByOpponent
        | Requirement::WeaponBurnedOpponent
        | Requirement::TargetInCombatWithDamage
        | Requirement::TargetInCombatWithNormalDamage
        | Requirement::OneCopyPerTurn
        | Requirement::FewerCopiesUsedThisPhase(_)
        | Requirement::ActiveControlsNonSect(_) => true, // engine-checked
        Requirement::NoOptionalActionThisTurn => !st.optional_action,
        Requirement::AshHasAlly(of) => st.seats[seat.index()].ash.iter().any(|&c| ash_ally(c, of)),
        Requirement::AllyInAnyAsh => st.seats.iter().any(|s| {
            !s.ousted
                && s.ash
                    .iter()
                    .any(|&c| LIBRARY[c as usize].types.contains(&CardType::Ally))
        }),
        Requirement::TargetInTorpor
        | Requirement::ControlledByPrey
        | Requirement::TargetSectNot(_)
        | Requirement::TargetCapacityAtLeast(_)
        | Requirement::TargetCapacityAtMost(_)
        | Requirement::NotGehennaImmune
        | Requirement::SameClanAsActor
        | Requirement::TargetDiscipline(_)
        | Requirement::TargetMithraic
        | Requirement::TargetNotNamed(_)
        | Requirement::TargetAnarch
        | Requirement::TargetHostage => {
            true // target predicates
        }
        Requirement::ActingVampire => {
            action.is_some_and(|a| st.minions[a.acting.index()].is_vampire())
        }
        Requirement::ActingCapacityAtLeast(n) => action.is_some_and(|a| {
            st.minions[a.acting.index()].is_vampire()
                && st.capacity_of(a.acting).is_some_and(|c| c >= n)
        }),
        Requirement::ActionUndirected => action.is_some_and(|a| a.directed_seat(st).is_none()),
        Requirement::ActingClan(c) => {
            action.is_some_and(|a| st.minions[a.acting.index()].clan() == Some(c))
        }
        Requirement::NotFirstActionOfPhase => st.phase_actions >= 2,
        Requirement::AnarchHuntedYours => action.is_some_and(|a| {
            (a.kind == ActionKind::Hunt || a.is_hunt)
                && a.hunt_gained
                && a.seat == seat
                && st.sect_of(a.acting) == Some(Sect::Anarch)
        }),
        Requirement::PoolAtLeast(n) => st.seats[seat.index()].pool >= n as i32,
        Requirement::ControlsReadyOnPath(path, n) => {
            st.minions_of(seat)
                .filter(|&m| st.minions[m.index()].ready() && st.follows(m, path))
                .count()
                >= n as usize
        }
        Requirement::ControlsPrinceOf(city) => st
            .minions_of(seat)
            .any(|m| st.title_of(m) == crate::crypt::Title::Prince(city)),
        Requirement::ActorVampire => actor.is_some_and(|m| !st.minions[m.index()].is_ally()),
        Requirement::HandHas(f, n) => st.hand_picks(seat, f).len() >= n as usize,
        Requirement::HostLocked
        | Requirement::HostBloodAtLeast(_)
        | Requirement::HostBloodAtMost(_)
        | Requirement::HostBelowStartingLife
        | Requirement::HostBelowCapacity
        | Requirement::HostControllerPoolAtLeast(_)
        | Requirement::HostInTorpor
        | Requirement::HostControllerActive
        | Requirement::ControlsReadyTitledOther(_)
        | Requirement::PredatorMoreReadyMinions
        | Requirement::PreyPoolAtMost(_)
        | Requirement::TargetSeatControlsClan(_)
        | Requirement::TargetFollowsPath(_)
        | Requirement::TargetAllyOf(_)
        | Requirement::TargetAllyNotOf(_)
        | Requirement::TargetOtherCopyControlled
        | Requirement::CountersAtLeast(_)
        | Requirement::CountersWithBleedGainAtLeast(_)
        | Requirement::CountersAtMost(_)
        | Requirement::CardUnlocked
        | Requirement::NotHuntingGroundFed
        | Requirement::TargetWithout(_)
        | Requirement::TargetTitleIn(_)
        | Requirement::TargetTitleContested
        | Requirement::HostOutOfCombat
        | Requirement::OncePerTurn
        | Requirement::TargetClanNotIn(_)
        | Requirement::TargetNotSuperior(_)
        | Requirement::OtherThanActor
        | Requirement::TargetOnActor => true, // host / card / target predicates
        Requirement::ActorUnlocked => actor.is_some_and(|m| !st.minions[m.index()].locked),
        Requirement::ActorMayUseEquipment => actor.is_some_and(|m| {
            !st.no_equipment(m)
                && combat
                    .and_then(|c| c.side_of_minion(m).map(|i| !c.sides[i].equipment_barred()))
                    .unwrap_or(true)
        }),
        Requirement::InflictedMoreThisRound => combat
            .zip(side)
            .is_some_and(|(c, i)| c.sides[i].inflicted_round > c.sides[1 - i].inflicted_round),
        Requirement::NotBleed => action.is_some_and(|a| a.kind != ActionKind::Bleed),
        Requirement::ActingYounger => action
            .zip(actor)
            .is_some_and(|(a, m)| st.younger_than(a.acting, m)),
        Requirement::ActingAllyOrYounger => action.zip(actor).is_some_and(|(a, m)| {
            st.minions[a.acting.index()].is_ally() || st.younger_than(a.acting, m)
        }),
        Requirement::RecruitingSubtype(labels) => action.is_some_and(|a| {
            matches!(a.kind, ActionKind::Recruit | ActionKind::Employ)
                && a.card.is_some_and(|p| {
                    labels
                        .iter()
                        .any(|l| LIBRARY[p.card as usize].labels.contains(l))
                })
        }),
        Requirement::HostAttempting
        | Requirement::ActingIsBy
        | Requirement::BleedAgainstHostController => true, // engine-checked
        Requirement::PlayedCardType(t) => {
            played.is_some_and(|p| LIBRARY[p.card as usize].types.contains(&t))
        }
        Requirement::PlayedCard(f) => played.is_some_and(|p| f.matches(p.card)),
        Requirement::PlayedRequiresDiscipline(d) => played.is_some_and(|p| {
            LIBRARY[p.card as usize].modes[library_mode(p.card, p.mode as usize)]
                .tags
                .iter()
                .any(|t| t.eq_ignore_ascii_case(d))
        }),
        Requirement::OpponentWithout(h) => combat
            .zip(side)
            .is_some_and(|(c, i)| !st.holds_a(c.sides[1 - i].minion, h)),
        Requirement::ActionBlocked => {
            action.is_some_and(|a| a.outcome == Some(crate::actions::Outcome::Blocked))
        }
        Requirement::NoSuperiorVeilThisTurn => !st.veil_superior,
        Requirement::OpposingWeapon => combat.zip(actor).is_some_and(|(c, m)| {
            c.opponent_of(m).is_some_and(|o| {
                st.equipment_on(o)
                    .into_iter()
                    .any(|w| is_weapon(st.cards[w.index()].card))
            })
        }),
        Requirement::ManeuverToClose => combat.is_none_or(|c| {
            c.step != crate::combat::Step::DetermineRange || c.range == Range::Long
        }),
        Requirement::StrikeDamagePending => combat
            .zip(side)
            .is_some_and(|(c, i)| c.sides[i].strike_damage.total() > 0),
        Requirement::StrikeNormalDamagePending => combat
            .zip(side)
            .is_some_and(|(c, i)| c.sides[i].strike_damage.normal > 0),
        Requirement::EffectNormalDamagePending => combat
            .zip(side)
            .is_some_and(|(c, i)| c.sides[i].pending.normal > c.sides[i].strike_damage.normal),
        Requirement::ActorAllyCard(f) => actor.is_some_and(|m| {
            matches!(st.minions[m.index()].kind, crate::state::MinionKind::Ally { card, .. }
                if f.matches(card))
        }),
        Requirement::PlayedActionCardAtYour(label) => {
            played.is_none_or(|p| played_action_card_at(st, seat, label, p))
        }
        Requirement::NoReactionPlayed => action.is_some_and(|a| !a.reaction_played),
        Requirement::OwnAshNotEmpty => !st.seats[seat.index()].ash.is_empty(),
        Requirement::PreyAshNotEmpty => !st.seats[st.prey(seat).index()].ash.is_empty(),
        Requirement::NoGreatBeastControlled => !st.minions_of(seat).any(|m| {
            matches!(
                st.minions[m.index()].kind,
                crate::state::MinionKind::GreatBeast { .. }
            )
        }),
        Requirement::UncontrolledAtLeast(n) => {
            st.seats[seat.index()].uncontrolled.len() >= n as usize
        }
        Requirement::ActorInfernal => actor.is_some_and(|m| st.is_infernal(m)),
        Requirement::PreyAshHasPair => {
            let ash = &st.seats[st.prey(seat).index()].ash;
            ash.iter()
                .any(|c| ash.iter().filter(|&x| x == c).count() >= 2)
        }
        Requirement::ControlsReadyTitled(classes) => st.minions_of(seat).any(|m| {
            st.minions[m.index()].ready() && classes.iter().any(|c| c.matches(st.title_of(m)))
        }),
        Requirement::ControlsReadySect(sects) => st.minions_of(seat).any(|m| {
            let v = &st.minions[m.index()];
            v.ready() && st.sect_of(m).is_some_and(|s| sects.contains(&s))
        }),
        Requirement::RangeClose => combat.is_some_and(|c| c.range == Range::Close),
        Requirement::RangeLong => combat.is_some_and(|c| c.range == Range::Long),
        Requirement::FirstRound => combat.is_some_and(|c| c.round == 1),
        Requirement::NotFirstRound => combat.is_some_and(|c| c.round > 1),
        Requirement::BothCombatantsReady => combat.is_some_and(|c| c.both_ready(st)),
        Requirement::AnarchCombatWithYours => combat.is_some_and(|c| {
            let anarch = |m: MinionId| st.sect_of(m) == Some(Sect::Anarch);
            let (a, b) = (c.sides[0].minion, c.sides[1].minion);
            anarch(a)
                && anarch(b)
                && (st.minions[a.index()].owner == seat || st.minions[b.index()].owner == seat)
        }),
        Requirement::AnarchCardInAshAndHand => {
            let s = &st.seats[seat.index()];
            !s.hand.is_empty()
                && s.ash.iter().any(|&c| {
                    LIBRARY[c as usize]
                        .labels
                        .contains(&crate::library::Label::RequiresAnarch)
                })
        }
        Requirement::CombatWouldEnd => combat.is_some_and(|c| c.would_end() && !c.end_fixed),
        Requirement::ActorReady => actor.is_some_and(|m| st.minions[m.index()].ready()),
        // The prevention half of a card needs damage to prevent; a mode
        // with another window (Indomitability [FOR]'s press) is unaffected
        // outside the prevention step.
        Requirement::DamagePending => combat.zip(side).is_some_and(|(c, i)| match c.step {
            Step::Prevent(p) => p == i && c.sides[i].pending.total() > 0,
            _ => true,
        }),
        Requirement::NonAggravatedDamagePending => {
            combat.zip(side).is_some_and(|(c, i)| match c.step {
                Step::Prevent(p) => p == i && c.sides[i].pending.normal > 0,
                _ => true,
            })
        }
        Requirement::GunStrikeDamagePending => {
            combat.zip(side).is_some_and(|(c, i)| match c.step {
                Step::Prevent(p) => p == i && c.sides[i].pending.gun > 0,
                _ => true,
            })
        }
        Requirement::NonGunDamagePending => combat.zip(side).is_some_and(|(c, i)| match c.step {
            Step::Prevent(p) => p == i && c.sides[i].pending.total() > c.sides[i].pending.gun,
            _ => true,
        }),
        // Card and host predicates.
        Requirement::SourceHeldWhenBlocking
        | Requirement::ControllerControlsHost
        | Requirement::HostReady => true,
        Requirement::NoContinuePress => combat.is_some_and(|c| !c.press_continue),
        Requirement::ActorHasBlocked => {
            action.is_some_and(|a| a.blocker.is_some() && a.blocker == actor)
        }
        Requirement::ActorTitleIn(classes) => actor.is_some_and(|m| {
            let t = st.title_of(m);
            classes.iter().any(|c| c.matches(t))
                // Karl Schrekt: "as if he were a justicar".
                || classes.contains(&TitleClass::Justicar)
                    && st.printed_mod(m, Stat::PlaysAsJusticar, crate::state::PrintedCtx::default())
                        > 0
        }),
        Requirement::ActorTitleNotIn(classes) => actor.is_some_and(|m| {
            let t = st.title_of(m);
            !classes.iter().any(|c| c.matches(t))
        }),
        // An ally playing a card "as a vampire" counts as independent (LSJ
        // 20020503), with its granted capacity (1 unless printed; rulebook
        // "Allies").
        Requirement::ActorSectIn(sects) => actor.is_some_and(|m| match st.sect_of(m) {
            Some(s) => sects.contains(&s),
            None => st.plays_as_vampire(m).is_some() && sects.contains(&Sect::Independent),
        }),
        Requirement::ActorCapacityAtLeast(n) => actor.is_some_and(|m| {
            st.capacity_of(m)
                .or(st.plays_as_vampire(m).map(|g| g.capacity))
                .is_some_and(|c| c >= n)
        }),
        Requirement::IsPolitical => action.is_some_and(|a| a.kind == ActionKind::Political),
        Requirement::PollingStep => political.is_some_and(|r| r.step == RefStep::Polling),
        Requirement::AnyPollingStep => referendum.is_some_and(|r| r.step == RefStep::Polling),
        Requirement::ActorActing => action.zip(actor).is_some_and(|(a, m)| a.acting == m),
        Requirement::GehennaInPlay(n) => st.gehenna_in_play() >= n as usize,
        Requirement::RescuedFromTorpor => action.is_some_and(|a| {
            a.kind == ActionKind::Rescue && a.outcome == Some(crate::actions::Outcome::Successful)
        }),
        Requirement::ActionSuccessful => action.is_some_and(|a| {
            matches!(
                a.outcome,
                Some(crate::actions::Outcome::Successful | crate::actions::Outcome::Fizzled)
            )
        }),
        Requirement::ActionDirected => action.is_some_and(|a| a.directed_seat(st).is_some()),
        Requirement::ActorDiablerised => action
            .zip(actor)
            .is_some_and(|(a, m)| a.diablerist == Some(m)),
        Requirement::ActingOtherController => action.is_some_and(|a| a.seat != seat),
        Requirement::OtherActingControlled => {
            action.is_some_and(|a| a.seat == seat && actor != Some(a.acting))
        }
        Requirement::OwnActing => action.is_some_and(|a| a.seat == seat),
        Requirement::ActionRequiresAnarch => action.is_some_and(|a| a.requires_anarch(st)),
        Requirement::NoSlaveInstead => action.is_some_and(|a| {
            a.seat == seat
                && a.slave.is_none()
                && !a.continued
                && matches!(a.state, crate::actions::ActionState::Blocked(_))
        }),
        Requirement::CallerTitledOfSect(sect) => {
            referendum.and_then(|r| r.calling).is_some_and(|m| {
                st.sect_of(m) == Some(sect) && st.title_of(m) != crate::crypt::Title::None
            })
        }
        Requirement::MaskCapable => match (action, actor) {
            (Some(a), Some(m)) => mask_capable(st, a, m),
            _ => false,
        },
        Requirement::NoReadyBlackHand => !st
            .minions
            .iter()
            .any(|m| m.ready() && m.vampire().is_some_and(|v| v.black_hand)),
        Requirement::NoActionUnderway => {
            action.is_none() && combat.is_none() && referendum.is_none()
        }
        Requirement::ControlsReadyVampires(n) => {
            st.minions_of(seat)
                .filter(|&m| {
                    let v = &st.minions[m.index()];
                    v.ready() && v.is_vampire()
                })
                .count()
                >= n as usize
        }
        Requirement::ActingSectIn(sects) => {
            action.is_some_and(|a| st.sect_of(a.acting).is_some_and(|s| sects.contains(&s)))
        }
        Requirement::DirectedHandNotEmpty => action
            .and_then(|a| a.directed_seat(st))
            .is_some_and(|t| !st.seats[t.index()].hand.is_empty()),
        // Target predicates (`holds_for_minion`).
        Requirement::ControlledByDirectedSeat
        | Requirement::NotHost
        | Requirement::TargetClan(_)
        | Requirement::PredatorsPredator
        | Requirement::TargetPrey
        | Requirement::TargetAshAtLeast(_) => true,
        Requirement::ActingByPredator => {
            action.is_some_and(|a| a.seat == st.predator(seat) && a.seat != seat)
        }
        Requirement::MoreThanTwoLeft => st.living() > 2,
        Requirement::TargetSubtypeIn(_) | Requirement::MayBeLocked => true,
        Requirement::ActorBloodAtLeast(n) => {
            actor.is_some_and(|m| st.minions[m.index()].blood >= n)
        }
        Requirement::CanAttemptBlock | Requirement::CanAttemptBlockAnyone => true, // engine-checked
        Requirement::BleedTargetPoolAtMost(n) => action
            .and_then(|a| a.bleed_target)
            .is_some_and(|t| st.seats[t.index()].pool <= n as i32),
        Requirement::AttemptingSectIn(sects) => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => st.sect_of(m).is_some_and(|s| sects.contains(&s)),
            _ => false,
        }),
        Requirement::AnyAttemptNeedsIntercept => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => a.stealth(st) > a.intercept_of(st, m),
            _ => false,
        }),
        Requirement::ActionRequiresClanOrDiscipline(clan, disc) => action.is_some_and(|a| {
            let from_card = a.card.is_some_and(|p| {
                let lib = &LIBRARY[p.card as usize];
                lib.clans.contains(&clan)
                    || lib.modes[library_mode(p.card, p.mode as usize)]
                        .tags
                        .iter()
                        .any(|t| discipline_key(t) == disc)
            });
            let from_source = a.source.zip(a.ability).is_some_and(|(src, ab)| {
                src.mode(st, ab).requires.iter().any(|r| match r {
                    Requirement::ActorClan(c) => *c == clan,
                    _ => false,
                })
            });
            from_card || from_source
        }),
        Requirement::NoVampireNamedButInCrypt(name) => {
            !st.minions_of(seat)
                .any(|m| st.minions[m.index()].name() == name)
                && st.seats[seat.index()]
                    .crypt
                    .iter()
                    .any(|&c| crate::crypt::CRYPT[c as usize].name == name)
        }
        Requirement::HostCombatOpponentNotReady
        | Requirement::HostCombatOpposingVampireNotReady
        | Requirement::HostNoCopyOfControllers
        | Requirement::HostUsedDisciplines(_) => true, // engine-checked
        Requirement::DirectedAtYourReadyClan(clan) => action.is_some_and(|a| {
            a.directed_seat(st).is_some()
                && a.target_minion().is_some_and(|m| {
                    let v = &st.minions[m.index()];
                    v.owner == seat && v.ready() && v.clan() == Some(clan)
                })
        }),
        Requirement::TermsChoseYourReadyClan(clan) => political.is_some_and(|r| {
            r.terms.minions.iter().any(|&m| {
                let v = &st.minions[m.index()];
                v.owner == seat && v.ready() && v.clan() == Some(clan)
            })
        }),
        Requirement::OwnActionCardCosts(kind) => action.is_some_and(|a| {
            a.seat == seat
                && a.card.is_some_and(|p| {
                    let blood = blood_cost(st, p.card, p.x);
                    let pool = pool_cost(st, seat, p.card, Target::Minion(a.acting));
                    match kind {
                        CostSource::Line => {
                            blood > a.blood_reduction && st.minions[a.acting.index()].is_vampire()
                        }
                        CostSource::Cache => a.kind == crate::decision::ActionKind::Equip,
                        CostSource::Hackerspace => {
                            a.kind == crate::decision::ActionKind::Equip
                                && st.sect_of(a.acting) == Some(Sect::Anarch)
                        }
                        _ => blood > 0 || pool > 0,
                    }
                })
        }),
        Requirement::PlainEquipFromHandOncePerTurn => action.is_some_and(|a| {
            a.kind == crate::decision::ActionKind::Equip
                && a.card.is_some_and(|p| plain_equipment(p.card))
                && !st
                    .turn_uses
                    .contains(&(AbilitySource::Vampire(a.acting), u8::MAX - 1))
        }),
        Requirement::AttemptingAllyOrYounger => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => {
                st.minions[m.index()].is_ally() || st.younger_than(m, a.acting)
            }
            _ => false,
        }),
        Requirement::AttemptingCapacityAtLeast(n) => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => st.capacity_of(m).is_some_and(|c| c >= n),
            _ => false,
        }),
        Requirement::ActorNotClan(c) => {
            actor.is_some_and(|m| st.minions[m.index()].clan() != Some(c))
        }
        Requirement::OwnMinionInCombat => {
            combat.is_some_and(|cb| cb.sides.iter().any(|s| s.seat == seat))
        }
        Requirement::AttemptingClan(c) => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => st.minions[m.index()].clan() == Some(c),
            _ => false,
        }),
        Requirement::ActorBurnedPreyMinion => actor.is_some_and(|m| {
            let prey = st.prey(st.minions[m.index()].owner);
            combat.is_some_and(|c| c.burned_minions.contains(&(m, prey)))
                || action.is_some_and(|a| a.acting == m && a.burned_owners.contains(&prey))
        }),
        Requirement::AttemptingVampireWithout(d) => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => {
                st.minions[m.index()].is_vampire() && st.discipline_level(m, d) == 0
            }
            _ => false,
        }),
        Requirement::YourAttemptNeedsIntercept => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => {
                st.minions[m.index()].owner == seat && a.stealth(st) > a.intercept_of(st, m)
            }
            _ => false,
        }),
        Requirement::OthersAttemptNeedsIntercept => action.is_some_and(|a| match a.state {
            ActionState::Attempt(m) => {
                st.minions[m.index()].owner != seat && a.stealth(st) > a.intercept_of(st, m)
            }
            _ => false,
        }),
        Requirement::OtherActingClan(c) => action.is_some_and(|a| {
            actor != Some(a.acting) && st.minions[a.acting.index()].clan() == Some(c)
        }),
        // Host predicate (`holds_for_host`).
        Requirement::HostUnlocked => true,
        Requirement::OwnTurn => st.active == seat,
        Requirement::ActorMustHunt => actor.is_some_and(|m| st.must_hunt(m)),
        Requirement::DiscardPhaseActionLeft => st.seats[seat.index()].dpa > 0,
        Requirement::MasterCardPlayed => st.seats[seat.index()].master_played,
        Requirement::AshNotEmpty => !st.seats[seat.index()].ash.is_empty(),
        Requirement::LibraryAtLeast(n) => st.seats[seat.index()].library.len() >= n as usize,
        Requirement::AshAtLeast(n) => {
            let s = &st.seats[seat.index()];
            s.ash.len() + s.ash_crypt.len() >= n as usize
        }
        Requirement::ActorCalledReferendum => {
            political.is_some() && action.zip(actor).is_some_and(|(a, m)| a.acting == m)
        }
        Requirement::NoVotesCast => political.is_some_and(|r| !r.any_cast()),
        Requirement::ReferendumPassed => action.is_some_and(|a| a.referendum_passed),
        Requirement::Younger | Requirement::AllyOrYounger => true, // target predicates
        Requirement::DirectedAtYourMinion => action.is_some_and(|a| {
            a.directed_seat(st) == Some(seat)
                && a.target_minion()
                    .is_some_and(|m| st.minions[m.index()].owner == seat)
        }),
        Requirement::DirectedAtYourAlly => action.is_some_and(|a| {
            a.directed_seat(st) == Some(seat)
                && a.target_minion().is_some_and(|m| {
                    st.minions[m.index()].owner == seat && st.minions[m.index()].is_ally()
                })
        }),
        Requirement::ActingByPredatorOrPrey => {
            action.is_some_and(|a| a.seat == st.predator(seat) || a.seat == st.prey(seat))
        }
        Requirement::OthersMinionPhase
        | Requirement::PlayedStrikeByOpponent
        | Requirement::PlayedStrikeOrGrappleByOpponent
        | Requirement::OwnMinionPhase
        | Requirement::ControlledByPredator
        | Requirement::ControlledByActive
        | Requirement::CardLocked => true, // engine / card / target predicates
        Requirement::NotInCombat => combat.is_none(),
        Requirement::NotContinued => action.is_some_and(|a| !a.continued),
        Requirement::NotInDamageResolution => combat.is_none_or(|c| {
            !matches!(
                c.step,
                crate::combat::Step::Prevent(_)
                    | crate::combat::Step::Allocate { .. }
                    | crate::combat::Step::AllocateSource { .. }
            )
        }),
        Requirement::ActorNotBlocking => action.is_none_or(|a| {
            a.blocker != actor && !matches!(a.state, ActionState::Attempt(m) if Some(m) == actor)
        }),
        Requirement::LifeAtLeast(_)
        | Requirement::TargetAlly
        | Requirement::TargetOnVampire
        | Requirement::SuperiorVersion
        | Requirement::BasicVersion
        | Requirement::TargetAllyHurt
        | Requirement::TargetOtherCopy
        | Requirement::TargetUnlocked => true, // card / target predicates
        Requirement::RecruitingTzimisceGhoul => action.is_some_and(|a| {
            a.kind == ActionKind::Recruit
                && a.card.is_some_and(|p| {
                    let c = &LIBRARY[p.card as usize];
                    c.labels.contains(&crate::library::Label::Ghoul)
                        && c.clans.contains(&"Tzimisce")
                })
        }),
        Requirement::NoBlockAttempt => action.is_some_and(|a| {
            matches!(
                a.state,
                ActionState::AsAnnounced | ActionState::Open | ActionState::Declined
            )
        }),
        Requirement::OpponentSubtype(l) => combat.zip(side).is_some_and(|(c, i)| {
            st.minions[c.sides[1 - i].minion.index()]
                .kind
                .ally_card()
                .is_some_and(|card| LIBRARY[card as usize].labels.contains(&l))
        }),
        Requirement::OpponentVampire => combat
            .zip(side)
            .is_some_and(|(c, i)| !st.minions[c.sides[1 - i].minion.index()].is_ally()),
        Requirement::OpponentVampireNotOlder => combat.zip(side).is_some_and(|(c, i)| {
            let (me, opp) = (c.sides[i].minion, c.sides[1 - i].minion);
            !st.minions[opp.index()].is_ally() && !st.younger_than(me, opp)
        }),
        Requirement::ActingCombatantYourVampire => combat.is_some_and(|c| {
            let v = &st.minions[c.sides[0].minion.index()];
            v.is_vampire() && v.owner == seat
        }),
        Requirement::OwnCombatantFacesAggravatedStrike => combat.is_some_and(|c| {
            c.side_of(seat).is_some_and(|i| {
                st.minions[c.sides[i].minion.index()].is_vampire()
                    && c.aggravated_strike_on(st, action, i)
            })
        }),
        Requirement::OpponentNotReady => combat
            .zip(side)
            .is_some_and(|(c, i)| !st.minions[c.sides[1 - i].minion.index()].ready()),
        Requirement::ActorNotVotingFor => {
            political.zip(actor).is_some_and(|(r, m)| !r.voting_for(m))
        }
        Requirement::ActorWithout(h) => actor.is_some_and(|m| !st.holds_a(m, h)),
        Requirement::ActorClan(c) => actor.and_then(|m| st.minions[m.index()].clan()) == Some(c),
        Requirement::YouControlThisCard
        | Requirement::OthersControlThisCard
        | Requirement::CardControllersTurn
        | Requirement::TargetCapacityBelowCounters => true, // engine-checked
        Requirement::TargetOnPreyMinion => true, // card-target predicate
        Requirement::EnteredBeforeThisTurn
        | Requirement::EnteredBeforeThisAction
        | Requirement::StoreEmpty
        | Requirement::CardMode(_) => true, // card predicate
        Requirement::ControlledByPredatorOrPrey => true, // target predicate
        Requirement::HostClan(_) | Requirement::HostDiscipline(_) => true, // host predicates
        Requirement::ThisVampireReady
        | Requirement::ActorSubtype(_)
        | Requirement::OpponentClan(_)
        | Requirement::OpponentYounger
        | Requirement::OpponentOlder
        | Requirement::OpponentAllyOrYounger
        | Requirement::OpponentTitled
        | Requirement::ActorCorruptedBySource
        | Requirement::PoolAtMost(_)
        | Requirement::ControlsLocation
        | Requirement::ControlsNoLocation
        | Requirement::ControlsVampireNamed(_)
        | Requirement::ControlsNoReadyOlderOfClan(_)
        | Requirement::ControlsNoReadyOfClan(_)
        | Requirement::NoReadyNamed(_)
        | Requirement::PlayedAsReaction
        | Requirement::PlayedAsCombat
        | Requirement::ActorClaimsTitleIn(_) => true, // printed-modifier predicates
        // An actor with no title (Powerbase: Barranquilla's untitled
        // burner); the printed predicate's reading too.
        Requirement::ActorUntitled => {
            actor.is_none_or(|m| st.title_of(m) == crate::crypt::Title::None)
        }
        Requirement::ControlsLockedMinion => {
            st.minions_of(seat).any(|m| st.minions[m.index()].locked)
        }
        Requirement::BlockerTitleIn(classes) => action.and_then(|a| a.blocker).is_some_and(|b| {
            let t = st.title_of(b);
            classes.iter().any(|c| c.matches(t))
        }),
        Requirement::AnyOf(alternatives) => alternatives.iter().any(|reqs| {
            reqs.iter().all(|&r| {
                holds(
                    r,
                    st,
                    seat,
                    actor,
                    played,
                    action,
                    combat,
                    referendum,
                    waives_needed,
                )
            })
        }),
    }
}

/// Evaluate a requirement against a candidate minion target.
pub fn holds_for_minion(
    req: Requirement,
    st: &GameState,
    seat: SeatId,
    referendum: Option<&ReferendumCtx>,
    acting: Option<MinionId>,
    m: MinionId,
    action: Option<&ActionCtx>,
) -> bool {
    let v = &st.minions[m.index()];
    match req {
        Requirement::ControlledByDirectedSeat => action
            .and_then(|a| a.directed_seat(st))
            .is_some_and(|t| v.owner == t),
        Requirement::Controlled => v.owner == seat && v.present(),
        Requirement::NotControlled => v.owner != seat,
        Requirement::TargetLocked => v.locked,
        Requirement::TargetVampire => !v.is_ally(),
        Requirement::TargetFollowsPath(path) => st.follows(m, path),
        Requirement::TargetAllyOf(labels) => v.kind.ally_card().is_some_and(|c| {
            labels
                .iter()
                .any(|l| LIBRARY[c as usize].labels.contains(l))
        }),
        Requirement::TargetAllyNotOf(labels) => v.kind.ally_card().is_some_and(|c| {
            !labels
                .iter()
                .any(|l| LIBRARY[c as usize].labels.contains(l))
        }),
        Requirement::TargetOtherCopyControlled => {
            v.owner == seat
                && acting.is_some_and(|a| a != m && st.minions[a.index()].name() == v.name())
        }
        Requirement::TargetCombatant => st.combatants.is_some_and(|p| p.contains(&m)),
        Requirement::TargetDiablerieSinceYourLastTurn => v
            .diablerie_turn
            .is_some_and(|t| t > st.seats[seat.index()].previous_turn),
        Requirement::BloodAtLeast(n) => v.blood >= n,
        Requirement::BloodAtMost(n) => v.blood <= n,
        Requirement::HasCastVotes => referendum.is_some_and(|r| r.has_cast(m)),
        Requirement::NotAbstaining => !referendum.is_some_and(|r| r.abstaining.contains(&m)),
        Requirement::ActorClan(c) => v.clan() == Some(c),
        Requirement::ActorReady => v.ready(),
        Requirement::ActorTitleIn(classes) => classes.iter().any(|c| c.matches(st.title_of(m))),
        Requirement::ActorSectIn(sects) => st.sect_of(m).is_some_and(|s| sects.contains(&s)),
        Requirement::NotHuntingGroundFed => !v.hunting_ground_fed,
        Requirement::Younger => acting.is_some_and(|a| st.younger_than(m, a)),
        Requirement::TargetAllyHurt => st.ally_body(m).is_some_and(|b| v.blood < b.life),
        Requirement::TargetAllyCard(f) => {
            matches!(v.kind, crate::state::MinionKind::Ally { card, .. } if f.matches(card))
        }
        Requirement::TargetUnlocked => !v.locked,
        Requirement::TargetAlly => v.is_ally(),
        Requirement::ControlledByPredator => v.owner == st.predator(seat),
        Requirement::ControlledByActive => v.owner == st.active,
        Requirement::ControlledByPredatorOrPrey => {
            v.owner != seat && (v.owner == st.predator(seat) || v.owner == st.prey(seat))
        }
        Requirement::TargetOtherCopy => {
            acting.is_some_and(|a| a != m && st.minions[a.index()].name() == v.name())
        }
        Requirement::AllyOrYounger => v.is_ally() || acting.is_some_and(|a| st.younger_than(m, a)),
        Requirement::TargetClan(c) => v.clan() == Some(c),
        // One of the alternatives ("another ready Tremere or Tremere
        // antitribu", Lord Tremere).
        Requirement::AnyOf(alternatives) => alternatives.iter().any(|reqs| {
            reqs.iter()
                .all(|&r| holds_for_minion(r, st, seat, referendum, acting, m, action))
        }),
        // "Of the same clan as this acting vampire" (My Kin Against the
        // World).
        Requirement::SameClanAsActor => {
            acting.is_some_and(|a| v.clan().is_some() && st.minions[a.index()].clan() == v.clan())
        }
        Requirement::TargetWithout(h) => !st.holds_a(m, h),
        Requirement::OtherThanActor => acting != Some(m),
        Requirement::TargetCorrupted => !v.corruption.is_empty(),
        Requirement::TargetNotSuperior(d) => st.discipline_level(m, d) < 2,
        // A hostage cannot be moved to the ready region (Carver's Meat
        // Packing and Storage): no effect can target it to.
        Requirement::TargetInTorpor => v.in_torpor && v.hostage == 0,
        Requirement::TargetHostage => v.hostage > 0,
        Requirement::ControlledByPrey => v.owner == st.prey(seat),
        Requirement::TargetClanNotIn(clans) => v.clan().is_some_and(|c| !clans.contains(&c)),
        Requirement::TargetSectNot(sect) => v.is_vampire() && st.sect_of(m) != Some(sect),
        Requirement::TargetCapacityAtLeast(n) => st.capacity_of(m).is_some_and(|c| c >= n),
        Requirement::TargetCapacityAtMost(n) => st.capacity_of(m).is_some_and(|c| c <= n),
        Requirement::NotGehennaImmune => !st.gehenna_immune(m),
        Requirement::TargetDiscipline(d) => st.has_discipline(m, d),
        Requirement::TargetMithraic => st.is_mithraic(m),
        Requirement::TargetNotNamed(name) => v.name() != name,
        Requirement::TargetAnarch => st.sect_of(m) == Some(Sect::Anarch),
        Requirement::TargetTitleIn(classes) => {
            let t = st.title_of(m);
            classes.iter().any(|c| c.matches(t))
        }
        Requirement::TargetTitleContested => v.title_contested,
        Requirement::TargetSubtypeIn(labels) => v.kind.ally_card().is_some_and(|card| {
            labels
                .iter()
                .any(|l| LIBRARY[card as usize].labels.contains(l))
        }),
        Requirement::BleedTargetPoolAtMost(n) => action
            .and_then(|a| a.bleed_target)
            .is_some_and(|t| st.seats[t.index()].pool <= n as i32),
        _ => true,
    }
}

/// Evaluate a requirement against the vampire in slot `i` of `seat`'s
/// uncontrolled region ("a younger vampire in your uncontrolled region",
/// Govern the Unaligned [DOM]; "a Gangrel", Thing).
pub fn holds_for_uncontrolled(
    req: Requirement,
    st: &GameState,
    seat: SeatId,
    acting: Option<MinionId>,
    i: usize,
) -> bool {
    if i >= st.seats[seat.index()].uncontrolled.len() {
        return false;
    }
    let (capacity, clan) = st.slot_vampire(seat, i);
    match req {
        Requirement::Younger => acting
            .and_then(|a| st.capacity_of(a))
            .is_some_and(|c| capacity < c),
        Requirement::TargetClan(c) => clan == c,
        Requirement::SameClanAsActor => {
            acting.is_some_and(|a| st.minions[a.index()].clan() == Some(clan))
        }
        Requirement::TargetCapacityAtLeast(n) => capacity >= n,
        // Printed on the card in the uncontrolled region (Unholy
        // Sacrament's "titled Sabbat vampire").
        Requirement::ActorSectIn(sects) => sects.contains(&st.slot_sect(seat, i)),
        Requirement::TargetTitleIn(classes) => {
            let title = st.slot_title(seat, i);
            classes.iter().any(|c| c.matches(title))
        }
        _ => true,
    }
}

/// Evaluate a requirement against a candidate seat target.
pub fn holds_for_seat(
    req: Requirement,
    st: &GameState,
    seat: SeatId,
    action: Option<&ActionCtx>,
    t: SeatId,
) -> bool {
    if st.seats[t.index()].ousted {
        return false;
    }
    match req {
        Requirement::NotActingController => action.is_some_and(|a| a.seat != t),
        Requirement::NotSelf => t != seat,
        Requirement::PredatorsPredator => t == st.predator(st.predator(seat)),
        Requirement::TargetPrey => t == st.prey(seat),
        Requirement::TargetSeatOusting => st.seats[t.index()].oust_pending,
        Requirement::CanSaveTarget => st.seats[seat.index()].pool > 1 - st.seats[t.index()].pool,
        Requirement::TargetAshAtLeast(n) => {
            let s = &st.seats[t.index()];
            s.ash.len() + s.ash_crypt.len() >= n as usize
        }
        _ => true,
    }
}

/// Evaluate a requirement against a candidate card target of `actor`'s
/// action.
pub fn holds_for_card_target(
    req: Requirement,
    st: &GameState,
    seat: SeatId,
    actor: Option<MinionId>,
    id: CardId,
) -> bool {
    let c = &st.cards[id.index()];
    match req {
        Requirement::NotControlled => st.card_controller(id) != seat,
        Requirement::Controlled => st.card_controller(id) == seat,
        Requirement::NotHost => actor.is_none() || c.host != actor,
        Requirement::TargetOnActor => actor.is_some() && c.host == actor,
        // A location in play is no equipment, whatever its type (Living
        // Manse; rules-qa 17.4).
        // A card that became another has no type nor label (Enhanced
        // Coagulant): still a card.
        Requirement::TargetCard(f) if f != CardFilter::Any && became(c.card, c.mode) => false,
        // A card that became weapon equipment in play (Weighted Walking
        // Stick, Zip Gun: "this card represents an equipment card").
        Requirement::TargetCard(CardFilter::Type(CardType::Equipment)) => {
            is_equipment_in_play(c.card) || becomes_weapon(c.card)
        }
        Requirement::TargetCard(f) => f.matches(c.card),
        Requirement::TargetOnReadyMinion => c.host.is_some_and(|h| st.minions[h.index()].ready()),
        Requirement::TargetOnVampire => c.host.is_some_and(|h| !st.minions[h.index()].is_ally()),
        Requirement::TargetOnPreyMinion => c
            .host
            .is_some_and(|h| st.minions[h.index()].owner == st.prey(seat)),
        Requirement::CardControlledByPrey => st.card_controller(id) == st.prey(seat),
        Requirement::ActorMayHoldTarget => actor.is_some_and(|m| st.can_hold(m, c.card, &[])),
        // "Lock any other unique location you control" (Powerbase:
        // Savannah): unlocked, and not a card of that name.
        Requirement::TargetUnlocked => !c.locked,
        Requirement::TargetNotNamed(n) => LIBRARY[c.card as usize].name != n,
        // "Burn equipment or retainer" (Donnybrook [obf]).
        Requirement::AnyOf(alternatives) => alternatives.iter().any(|reqs| {
            reqs.iter()
                .all(|&r| holds_for_card_target(r, st, seat, actor, id))
        }),
        _ => true,
    }
}

/// Evaluate a predicate on the card in play itself (an in-play ability's
/// "lock this card", a rider's counters), within the current action if any.
pub fn holds_for_card(
    req: Requirement,
    st: &GameState,
    id: CardId,
    action: Option<&ActionCtx>,
) -> bool {
    let c = &st.cards[id.index()];
    match req {
        Requirement::HostNoCopyOfControllers => c.host.is_none_or(|h| {
            let v = &st.minions[h.index()];
            let to = st.card_controller(id);
            !v.vampire().is_some_and(|cc| cc.unique)
                || !st.minions.iter().enumerate().any(|(j, o)| {
                    j != h.index() && o.owner == to && o.in_play && o.name() == v.name()
                })
        }),
        // The host in the combat that just ended, its opponent not ready
        // (Monster).
        Requirement::HostCombatOpponentNotReady => {
            let host = c.bearer();
            action.and_then(|a| a.last_combat).is_some_and(|pair| {
                host.is_some_and(|h| pair.contains(&h))
                    && pair
                        .iter()
                        .any(|&o| Some(o) != host && !st.minions[o.index()].ready())
            })
        }
        // A burned vampire is not ready; an ally opponent does not count.
        Requirement::HostCombatOpposingVampireNotReady => {
            let host = c.bearer();
            action.and_then(|a| a.last_combat).is_some_and(|pair| {
                host.is_some_and(|h| pair.contains(&h))
                    && pair.iter().any(|&o| {
                        let v = &st.minions[o.index()];
                        Some(o) != host && v.is_vampire() && !v.ready()
                    })
            })
        }
        Requirement::HostUsedDisciplines(n) => {
            let host = c.bearer();
            action.is_some_and(|a| {
                let mut used: Vec<&str> = a
                    .disciplines_played
                    .iter()
                    .filter(|(m, _)| Some(*m) == host)
                    .map(|(_, d)| *d)
                    .collect();
                used.sort_unstable();
                used.dedup();
                used.len() >= n as usize
            })
        }
        // Phase uses by the copies its controller controls (Split the Veil).
        Requirement::FewerCopiesUsedThisPhase(n) => {
            let controller = st.card_controller(id);
            let used = st
                .cards
                .iter()
                .enumerate()
                .filter(|&(j, o)| {
                    o.card == c.card
                        && st.card_controller(CardId(j as u32)) == controller
                        && st
                            .phase_uses
                            .iter()
                            .any(|(src, _)| *src == AbilitySource::Card(CardId(j as u32)))
                })
                .count();
            used < n as usize
        }
        // The remembered set, its host's controller not counted (LSJ
        // 20100527), all among the Methuselahs the action is directed at
        // (owner, 2026-09-26: rules-qa 24.5).
        Requirement::DirectedAtRemembered => {
            let remembered = match c.host {
                Some(h) => c.against.without(st.minions[h.index()].owner),
                None => c.against,
            };
            !remembered.is_empty()
                && action.is_some_and(|a| remembered.is_subset(a.directed_seats(st)))
        }
        // An ally's own card: the ally locks ("this ally can lock to").
        Requirement::CardUnlocked => match c.body {
            Some(m) => !st.minions[m.index()].locked,
            None => !c.locked,
        },
        Requirement::CountersAtLeast(n) => c.counters >= n,
        Requirement::CountersWithBleedGainAtLeast(n) => {
            let gain = action.is_some_and(|a| a.momentum.contains(&(id, true)));
            c.counters + u8::from(gain) >= n
        }
        Requirement::CountersAtMost(n) => c.counters <= n,
        Requirement::BoonTargetActive => c.against.contains(st.active),
        Requirement::EnteredBeforeThisTurn => c.entered < st.turn,
        Requirement::EnteredBeforeThisAction => c.entered_action != st.action_serial,
        Requirement::CardMode(n) => c.mode == n,
        Requirement::StoreEmpty => c.stored.is_empty(),
        Requirement::SuperiorVersion => c.mode > 0,
        Requirement::BasicVersion => c.mode == 0,
        Requirement::CardLocked => match c.body {
            Some(m) => st.minions[m.index()].locked,
            None => c.locked,
        },
        Requirement::LifeAtLeast(n) => match c.body {
            Some(m) => st.minions[m.index()].blood >= n,
            None => c.counters >= n,
        },
        Requirement::ControllerControlsHost => c
            .host
            .is_some_and(|h| st.minions[h.index()].owner == c.controller),
        _ => true,
    }
}

/// Evaluate a host predicate for an in-play ability.
pub fn holds_for_host(req: Requirement, st: &GameState, host: MinionId) -> bool {
    let v = &st.minions[host.index()];
    match req {
        Requirement::HostLocked => v.locked,
        Requirement::HostUnlocked => !v.locked,
        Requirement::HostReady => v.ready(),
        Requirement::HostBloodAtLeast(n) => v.blood >= n,
        Requirement::HostBloodAtMost(n) => v.blood <= n,
        Requirement::HostBelowCapacity => st.capacity_of(host).is_some_and(|c| v.blood < c),
        Requirement::HostBelowStartingLife => st.ally_body(host).is_some_and(|b| v.blood < b.life),
        Requirement::HostControllerPoolAtLeast(n) => st.seats[v.owner.index()].pool >= n as i32,
        Requirement::HostInTorpor => v.in_play && v.in_torpor,
        Requirement::HostControllerActive => v.in_play && v.owner == st.active,
        // Gravebound Drone: another copy of it is the user's combatant.
        Requirement::OwnCombatantOtherCopy => st.combatants.is_some_and(|pair| {
            pair.iter().any(|&m| {
                m != host
                    && st.minions[m.index()].owner == v.owner
                    && st.minions[m.index()].name() == v.name()
            })
        }),
        Requirement::HostClan(c) => v.clan() == Some(c),
        Requirement::HostDiscipline(d) => st.has_discipline(host, d),
        _ => true,
    }
}

/// The value of X for terms the game state determines (no decision):
/// `CopiesControlled` counts the seat's copies of the card in play.
pub fn terms_x(st: &GameState, seat: SeatId, card: u16, mode: &ModeDef) -> Option<u8> {
    match mode.terms {
        Some(Terms::CopiesControlled) => Some(
            st.cards
                .iter()
                .filter(|c| {
                    c.present()
                        && c.owner == seat
                        && LIBRARY[c.card as usize].name == LIBRARY[card as usize].name
                })
                .count() as u8,
        ),
        _ => None,
    }
}

/// Pool cost for `payer` of playing `card` on `target`: base cost folded
/// through the cost layers (spec §1.7.2: cost arithmetic; §1.15: copies add).
pub fn pool_cost(st: &GameState, payer: SeatId, card: u16, target: Target) -> i32 {
    let def = &LIBRARY[card as usize];
    let mut cost = match def.pool_cost {
        Cost::None => 0,
        Cost::Fixed(n) => n as i32,
        Cost::X => 0,
    };
    for l in st.layers.iter().filter(|l| st.layer_active(l)) {
        let Modifier::CostMod {
            name,
            on_host_only,
            owner_only,
            pool,
        } = l.modifier
        else {
            continue;
        };
        if name != def.name {
            continue;
        }
        let source = &st.cards[l.source.index()];
        if on_host_only && Target::from(source.host) != target {
            continue;
        }
        if owner_only && source.owner != payer {
            continue;
        }
        cost += pool as i32;
    }
    // "Master cards targeting this minion cost 1 additional pool" (Secure
    // Haven), whoever plays them (TOM 19951208-1).
    if let Target::Minion(m) = target {
        if def.types.contains(&CardType::Master) {
            cost += st.havens_on(m) as i32;
        }
    }
    cost.max(0)
}

impl From<Option<MinionId>> for Target {
    fn from(m: Option<MinionId>) -> Target {
        match m {
            Some(m) => Target::Minion(m),
            None => Target::None,
        }
    }
}

/// Blood cost of playing `card`, paid by the playing minion at resolution:
/// base cost folded through the action-scoped cost layers (Consign to
/// Oblivion: reaction cards cost +1 blood).
pub fn blood_cost(st: &GameState, card: u16, x: u8) -> u8 {
    let def = &LIBRARY[card as usize];
    let mut cost = match def.blood_cost {
        Cost::None => 0,
        Cost::Fixed(n) => n as i32,
        Cost::X => x as i32,
    };
    if def.types.contains(&CardType::Reaction) {
        for l in &st.layers {
            if let Modifier::ReactionBloodCost(n) = l.modifier {
                cost += n as i32;
            }
        }
    }
    cost.max(0) as u8
}

/// Could `m` have performed the action `a` under way, with every card the
/// acting minion played on it (Mask of a Thousand Faces)? A card's clan and
/// Disciplines are read on `m`; the action's other requirements as for an
/// action card's actor.
pub fn mask_capable(st: &GameState, a: &ActionCtx, m: MinionId) -> bool {
    use crate::decision::ActionKind as K;
    let v = &st.minions[m.index()];
    if m == a.acting || !v.is_vampire() || !v.ready() || v.locked || a.source.is_some() {
        return false;
    }
    // A library card's clan and one of its Disciplines (the level played
    // unknown for a modifier: any).
    let may_play = |card: u16, tags: &[&str]| {
        let c = &LIBRARY[card as usize];
        (c.clans.is_empty() || st.meets_clan(m, c.clans, true))
            && (tags.is_empty() || tags.iter().any(|t| st.has_discipline(m, t)))
    };
    let kind_ok = match a.kind {
        K::Bleed => {
            !v.bled_this_turn
                && st.printed_mod(
                    m,
                    Stat::NoDirectedActions,
                    crate::state::PrintedCtx::default(),
                ) <= 0
        }
        K::Hunt => !st.cannot_hunt(m),
        K::Diablerie => !st.blood_cursed(m),
        K::Rescue | K::EquipFrom => true,
        K::BecomeAnarch => st.may_become_anarch(m),
        K::Political => !v.political_this_turn,
        K::Card | K::Equip | K::Employ | K::Recruit => true,
        K::LeaveTorpor | K::BurnCard | K::StealCard | K::EquipFound | K::Ability => false,
    };
    let card_ok = a.card.is_none_or(|p| {
        let tags = LIBRARY[p.card as usize].modes[library_mode(p.card, p.mode as usize)].tags;
        let reqs_ok = lookup(p.card).is_some_and(|d| {
            d.modes[p.mode as usize].requires.iter().all(|&r| match r {
                Requirement::ActorTitleIn(cs) => cs.iter().any(|c| c.matches(st.title_of(m))),
                Requirement::ActorSectIn(ss) => st.sect_of(m).is_some_and(|s| ss.contains(&s)),
                Requirement::ActorVampire => true,
                _ => true,
            })
        });
        may_play(p.card, tags) && reqs_ok
    });
    let played_ok = a
        .played
        .iter()
        .filter(|(p, _)| *p == a.acting)
        .all(|&(_, name)| {
            let card = crate::state::find_library(name);
            may_play(card, LIBRARY[card as usize].disciplines)
        });
    kind_ok && card_ok && played_ok
}
