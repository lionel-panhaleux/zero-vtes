//! Where each requirement is read, and the requirement each ability cost
//! implies: conventions of the definitions, checked over every definition
//! by the tests below.

use super::{Actor, Effect, Hook, ModeDef, Requirement};

/// An evaluator of requirements: which positions of a definition really
/// read a requirement depends on the evaluator the engine runs there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    /// `holds`: the play's context (its seat and actor, the played card,
    /// the action, combat or referendum underway).
    Play,
    /// `holds_for_host`: the host of the card in play, or the vampire
    /// whose text it is.
    Host,
    /// `holds_for_card`: the card in play itself.
    Card,
    /// `holds_for_minion`: a minion target, a referendum term's minion
    /// choice, a live conditional bonus (`Effect::If`).
    MinionTarget,
    /// `holds_for_uncontrolled`: a vampire in the uncontrolled region.
    UncontrolledTarget,
    /// `holds_for_seat`: a Methuselah target.
    SeatTarget,
    /// `holds_for_card_target`: a card in play targeted.
    CardTarget,
    /// `GameState::printed_holds`: a modifier printed on a vampire.
    Printed,
    /// `GameState::rider_holds`' own arms: a rider's card predicates.
    Rider,
    /// Read by name where the engine enumerates a position, not by an
    /// evaluator: `holds` passes it as `true`.
    Engine(Site),
}

/// The engine code reading an engine-checked requirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Site {
    /// `Engine::play_options` (and `reacting_candidates`), on a card mode's
    /// requirements.
    PlayOptions,
    /// `Engine::play_options`, on a card mode's minion target filter.
    PlayTarget,
    /// `Engine::card_target_qualifies`: any card target.
    CardTargetQualifies,
    /// `Engine::ability_targets_by` and `minion_target_qualifies_by`: an
    /// in-play ability's minion target.
    AbilityTargets,
    /// `Engine::ability_gate`, every in-play ability: its phase.
    AbilityWindow,
    /// `Engine::ability_holds`, every in-play ability: a requirement read
    /// by name.
    AbilityGate,
    /// `Engine::ability_holds`, a card's ability only: a requirement on
    /// the card in play read by name (its controller, bearer, chosen
    /// vampire), and an `ActingCandidate` ability's `CardUnlocked`.
    AbilityGateCard,
    /// `Engine::ability_options`, an `Actor::Controller` ability's minion
    /// target.
    ControllerTarget,
    /// `Engine::vampire_ability_options`.
    VampireAbility,
    /// `Engine::as_played_request`: an ability in `Window::CardAsPlayed`.
    AsPlayed,
    /// The site firing a card's riders at this hook, past
    /// `Engine::fired_riders`.
    CardRider(Hook),
    /// The site firing a vampire's printed riders at this hook, past
    /// `Engine::printed_riders`.
    VampireRider(Hook),
}

impl Requirement {
    /// The evaluators that really read this requirement (any other passes
    /// it as `true`): it belongs in a position whose evaluators meet these.
    pub fn slots(self) -> &'static [Slot] {
        use Requirement as R;
        use Slot as S;
        match self {
            R::ActorNotClan(..)
            | R::ActorNotBlackHand
            | R::AttemptingCapacityAtLeast(..)
            | R::AttemptingAllyOrYounger
            | R::AttemptingYoungerVampire
            | R::OwnMinionInCombat
            | R::AnyAttemptNeedsIntercept
            | R::StrikeNormalDamagePending
            | R::EffectNormalDamagePending
            | R::ActorAllyCard(..)
            | R::ActionRequiresClanOrDiscipline(..)
            | R::NoVampireNamedButInCrypt(..)
            | R::DirectedAtYourReadyClan(..)
            | R::TermsChoseYourReadyClan(..)
            | R::PlainEquipFromHandOncePerTurn
            | R::OwnActionCardCosts(..)
            | R::OwnActionCardToAsh
            | R::NoOptionalActionThisTurn
            | R::AllyInAnyAsh
            | R::AshHasAlly(..)
            | R::PlayedCardIsMaster
            | R::PlayedByOtherSeat
            | R::NotOwnTurn
            | R::DirectedAtYou
            | R::ActingNotYourPredator
            | R::BleedAgainstPrey
            | R::BleedSuccessful
            | R::BleedAmountAtLeast(..)
            | R::BlocksDeclined
            | R::StealthNeeded
            | R::InterceptNeeded
            | R::LockedActor
            | R::ControlsReadyTitled(..)
            | R::ControlsReadySect(..)
            | R::ControlsReadyOfSect(..)
            | R::MasterPhaseActionLeft
            | R::HandHas(..)
            | R::PreyBurnedPool
            | R::ControlsReadyOnPath(..)
            | R::ActingFollowsPath(..)
            | R::NoQueuedCombat
            | R::ActingAllyOrYounger
            | R::ActorNotActing
            | R::OwnActingAllyOrYounger
            | R::ActionRequiresPath(..)
            | R::ActingAllyOf(..)
            | R::BlockingYoungerActing
            | R::FoughtByYoungerActing
            | R::BlockingActingOfSect(..)
            | R::FoughtByActingOfSect(..)
            | R::TargetOfActingOfSect(..)
            | R::TermsChoseActor
            | R::BlockedByVampireOfSect(..)
            | R::ActorAllyOf(..)
            | R::AttemptingAllyOf(..)
            | R::NoAimYet
            | R::HandNotEmpty
            | R::CryptNotEmpty
            | R::ActorBurnedPreyMinion
            | R::ActorBurnedMinion
            | R::ControlsReadyClan(..)
            | R::OwnReadyClanWithBlood(..)
            | R::ActingVampire
            | R::ActingCapacityAtLeast(..)
            | R::OthersAttemptNeedsIntercept
            | R::ActingClan(..)
            | R::NotFirstActionOfPhase
            | R::AnarchHuntedYours
            | R::AnarchCardInAshAndHand
            | R::AnarchCombatWithYours
            | R::PoolAtLeast(..)
            | R::RangeClose
            | R::RangeLong
            | R::FirstRound
            | R::BothCombatantsReady
            | R::CombatWouldEnd
            | R::DamagePending
            | R::OpponentGoingToTorpor
            | R::ActorNotGoingDown
            | R::NonAggravatedDamagePending
            | R::GunStrikeDamagePending
            | R::NonGunDamagePending
            | R::NoContinuePress
            | R::ActorHasBlocked
            | R::ActorCapacityAtLeast(..)
            | R::PollingStep
            | R::ActorNotForcedYet
            | R::AnyPollingStep
            | R::ActorCalledReferendum
            | R::ActorActing
            | R::ActionSuccessful
            | R::RescuedFromTorpor
            | R::ActorDiablerised
            | R::ActingOtherController
            | R::OtherActingControlled
            | R::OwnActing
            | R::NoActionUnderway
            | R::ActionUnderway
            | R::ControlsReadyVampires(..)
            | R::ActingSectIn(..)
            | R::DirectedHandNotEmpty
            | R::YourAttemptNeedsIntercept
            | R::OtherActingClan(..)
            | R::OwnTurn
            | R::ActorMustHunt
            | R::DiscardPhaseActionLeft
            | R::NoOwnClaimOf(..)
            | R::NoActionAnnouncedThisTurn
            | R::MasterCardPlayed
            | R::AshNotEmpty
            | R::ActionRequiresAnarch
            | R::LibraryAtLeast(..)
            | R::AshAtLeast(..)
            | R::NoVotesCast
            | R::CallerTitledOfSect(..)
            | R::ReferendumPassed
            | R::ActorNotVotingFor
            | R::ActorWithout(..)
            | R::ActorHoldsFewer(..)
            | R::PreyAshHasPair
            | R::ActorInfernal
            | R::ActorSeraph
            | R::LastCombatantsReady
            | R::NotPlayedThisAction(..)
            | R::UncontrolledAtLeast(..)
            | R::OwnAshNotEmpty
            | R::PreyAshNotEmpty
            | R::NoGreatBeastControlled
            | R::ManeuverToClose
            | R::OpposingWeapon
            | R::ActorUnlocked
            | R::ActorMayUseEquipment
            | R::InflictedMoreThisRound
            | R::NotBleed
            | R::ActingYounger
            | R::RecruitingSubtype(..)
            | R::OpponentWithout(..)
            | R::PlayedCardType(..)
            | R::PlayedNotCardType(..)
            | R::NoCryptEmpty
            | R::PredatorAndPreyReadyVampires
            | R::ActionBlocked
            | R::NoSuperiorVeilThisTurn
            | R::StrikeDamagePending
            | R::NoReactionPlayed
            | R::BlockerTitleIn(..)
            | R::BlockerOwnAnarch
            | R::BlockerOwn
            | R::BlockerReadyAfterItsCombat
            | R::ActorNotBlocker
            | R::SameCircleAsActing
            | R::SameCircleAsOwnCombatant { .. }
            | R::OwnCombatantsOpponentReady
            | R::BoonableBleed
            | R::BleedAtYouUnboonedForOne
            | R::ControlsOtherReadyUnlockedSameCircle
            | R::ActorUnprintedCircle
            | R::ActorGainedBlood
            | R::NoSlaveInstead
            | R::MaskCapable
            | R::NoReadyBlackHand
            | R::BlackHandVampireInPlay
            | R::ControlsLockedMinion
            | R::DirectedAtYourMinion
            | R::DirectedAtYourAlly
            | R::DirectedAtYourCard(_)
            | R::ActingByPredatorOrPrey
            | R::ActorUnmergedAdvanced
            | R::ActorPassedReferendumThisTurn
            | R::OwnUncontrolledVampire
            | R::PlayedByAllyOrCapacityAtMost(..)
            | R::PlayedByYoungerVampire
            | R::PlayedActionCardTargetsYou
            | R::OwnAshVampire
            | R::OpposingAllyBurned
            | R::OwnAshHas(..)
            | R::ControlsPrinceOf(..)
            | R::NotInCombat
            | R::NotContinued
            | R::NotInDamageResolution
            | R::ActorNotBlocking
            | R::RecruitingTzimisceGhoul
            | R::NoBlockAttempt
            | R::OpponentVampire
            | R::OpponentLacksDiscipline(..)
            | R::OpponentVampireNotOlder
            | R::OpponentNotReady
            | R::OpponentPresent
            | R::CombatantFacesAggravatedStrike
            | R::ActingCombatantYourVampire
            | R::ActingByPredator
            | R::MoreThanTwoLeft
            | R::AttemptingSectIn(..)
            | R::AttemptingClan(..)
            | R::ActionUnsuccessful
            | R::NoWatchDamageThisAction
            | R::NoCombatQueued
            | R::AttemptingVampireWithout(..)
            | R::PlayedCardIsMinionCard
            | R::PlayedByPredatorOrPrey
            | R::ActorBloodAtLeast(..)
            | R::ActorNotSterile
            | R::ActiveIsPrey
            | R::ActionNotFailed
            | R::PlayedNotOutOfTurn
            | R::BlockerNotReady
            | R::NoCombatEndsStrike
            | R::CombatFromBlock
            | R::NoSlaveTookCombat
            | R::OthersHuntingGroundsAtLeast(..)
            | R::PreyUncontrolledHasBlood
            | R::BleedTargetAshAtLeast(..)
            | R::NoOlderReadyUntitled { .. }
            | R::IsEquipLabeledFromHand(..)
            | R::ControlsReadyBlackHand
            | R::ControlsReadySeraph => &[S::Play],
            R::ActorBlackHand | R::ActorTitleNotIn(..) => &[S::Play, S::MinionTarget],
            R::HostBelowCapacity
            | R::HostControllerPoolAtLeast(..)
            | R::HostBloodAtLeast(..)
            | R::HostBelowStartingLife
            | R::HostBloodAtMost(..)
            | R::HostInTorpor
            | R::HostDidNotHunt
            | R::HostAnnouncedNoAction
            | R::HostControllerActive
            | R::OwnCombatantOtherCopy
            | R::HostReady
            | R::HostVampire => &[S::Host],
            R::HostCombatOpposingVampireNotReady
            | R::HostUsedDisciplines(..)
            | R::FewerCopiesUsedThisPhase(..)
            | R::CountersWithBleedGainAtLeast(..)
            | R::ControllerControlsHost
            | R::DirectedAtRemembered
            | R::StoreEmpty
            | R::StoreNotEmpty
            | R::EnteredBeforeThisTurn
            | R::EnteredBeforeThisAction
            | R::HostNoCopyOfControllers
            | R::HostOutOfPlay
            | R::LifeAtLeast(..)
            | R::BoonTargetActive
            | R::FewerCopiesThanReadyOfClan(..) => &[S::Card],
            R::TargetCapacityAtMost(..)
            | R::TargetAllyCard(..)
            | R::TargetDiscipline(..)
            | R::TargetMithraic
            | R::TargetAnarch
            | R::TargetSectNot(..)
            | R::BloodAtLeast(..)
            | R::NotHuntingGroundFed
            | R::TargetFollowsPath(..)
            | R::TargetAllyOf(..)
            | R::TargetAllyNotOf(..)
            | R::TargetOtherCopyControlled
            | R::TargetInTorpor
            | R::TargetRedList
            | R::TargetNotBlackHand
            | R::TargetNotRedList
            | R::TargetNotBlocker
            | R::TargetNotActing
            | R::TargetHostage
            | R::ControlledByPrey
            | R::NotAbstaining
            | R::ControlledByDirectedSeat
            | R::HasCastVotes
            | R::TargetWithout(..)
            | R::TargetTitleContested
            | R::TargetClanNotIn(..)
            | R::TargetNotSuperior(..)
            | R::OtherThanActor
            | R::ControlledByPredatorOrPrey
            | R::TargetCorrupted
            | R::TargetLacksActorSuperior
            | R::NotGehennaImmune
            | R::TargetLocked
            | R::TargetVampire
            | R::TargetMonster
            | R::TargetConvictionUnplayed
            | R::TargetEnteredSinceYourLastMinionPhase
            | R::TargetEnteredInYourLastInfluencePhase
            | R::TargetAllyOrCapacityAtMost(..)
            | R::ControlledByPredator
            | R::ControlledByActive
            | R::TargetOtherCopy
            | R::TargetAlly
            | R::TargetAllyHurt
            | R::AllyOrYounger
            | R::TargetCombatant
            | R::TargetFacesAggravatedStrike
            | R::BloodAtMost(..)
            | R::TargetDiablerieSinceYourLastTurn
            | R::TargetBledSinceYourLastTurn { .. }
            | R::TargetUntitled
            | R::TargetInfernal
            | R::TargetSubtypeIn(..)
            | R::TargetNotOfPredator
            | R::NoSelfContest(..)
            | R::NoOwnClaimOfTargetTitle { .. } => &[S::MinionTarget],
            R::TargetCirclelessBloodBrother => &[S::UncontrolledTarget],
            R::TargetImbued => &[S::MinionTarget, S::UncontrolledTarget],
            R::NotActingController
            | R::NotSelf
            | R::TargetPoolMoreThanYours
            | R::TargetSeatOusting
            | R::TargetSeatLostTitled
            | R::CanSaveTarget
            | R::TargetPrey
            | R::TargetPredatorOrPrey
            | R::TargetAshAtLeast(..)
            | R::PredatorsPredator => &[S::SeatTarget],
            R::CardControlledByPrey
            | R::CardControlledByPredatorOrPrey
            | R::TargetOnPreyMinion
            | R::TargetOnActor
            | R::TargetCard(..)
            | R::TargetOnReadyMinion
            | R::ActorMayHoldTarget
            | R::TargetWeapon
            | R::TargetOnVampire
            | R::TargetOnLockedMinion
            | R::TargetOnMinionNotOfSect(..)
            | R::TargetOnPredatorOrPreyMinion
            | R::TargetCardNot(..) => &[S::CardTarget],
            R::ControlsReadyTitledOther(..)
            | R::PredatorMoreReadyMinions
            | R::PreyPoolAtMost(..)
            | R::PreyControlsVampireNotOfSect(..)
            | R::TargetSeatControlsClan(..)
            | R::ThisVampireReady
            | R::ActorGraftedAlly
            | R::OpponentClan(..)
            | R::OpponentYounger
            | R::OpponentOlder
            | R::OpponentAllyOrYounger
            | R::OpponentTitled
            | R::ActorCorruptedBySource
            | R::PoolAtMost(..)
            | R::ControlsLocation
            | R::ControlsNoLocation
            | R::ControlsVampireNamed(..)
            | R::ControlsNoReadyOlderOfClan(..)
            | R::ControlsNoReadyOfClan(..)
            | R::SourceActing
            | R::ActorClaimsTitleIn(..)
            | R::PlayedAsReaction
            | R::PlayedInStrikeMode
            | R::DiablerizedOlder
            | R::PlayedAsCombat
            | R::NoReadyNamed(..)
            | R::OpponentSectIn(..)
            | R::OpponentControllerHasGehenna
            | R::OtherNonHuntAction
            | R::TargetSeatControlsReadyClan { .. }
            | R::TargetSeatControlsReadySect { .. }
            | R::OpponentInfernal
            | R::OpponentRedList
            | R::TargetSeatControlsNoReadySect(..)
            | R::TargetSeatControlsLocation
            | R::PreyControlsReadyTitled(..)
            | R::DirectedAtCard(..)
            | R::SourceCalledReferendum
            | R::SourceTitleIn(..) => &[S::Printed],
            R::ActiveControlsNonSect(..) | R::ActiveIsAgainst => &[S::Rider],
            R::ActorReady | R::BleedTargetPoolAtMost(..) => &[S::Play, S::MinionTarget],
            R::AnyOf(..) => &[S::Play, S::MinionTarget, S::CardTarget, S::Printed],
            R::ActorTitleIn(..) | R::ActorClan(..) => &[S::Play, S::MinionTarget, S::Printed],
            R::ActorSubtype(..) => &[S::Play, S::Printed],
            R::ActorSectIn(..) => &[S::Play, S::MinionTarget, S::UncontrolledTarget, S::Printed],
            R::IsBleed
            | R::ActorVampire
            | R::ActorImbued
            | R::IsHunt
            | R::RulebookHunt
            | R::IsDiablerie
            | R::IsRecruitOrEmploy
            | R::IsEquipWeaponFromHand
            | R::ActionEntersCombat
            | R::ActionPutsVampireInPlay
            | R::OpponentAlly
            | R::NoEdge
            | R::HasEdge
            | R::ActionUndirected
            | R::NotFirstRound
            | R::IsPolitical
            | R::ActionDirected
            | R::PlayedRequiresDiscipline(..)
            | R::PlayedRequiresAnyDiscipline
            | R::PlayedCard(..)
            | R::ActorUntitled
            | R::ActorFollowsNoPath
            | R::ActorNotRedList
            | R::ActorSlave
            | R::ActorFoughtItsContractsMinion
            | R::OpponentSubtype(..)
            | R::ReadyNamed(..)
            | R::IsEquipAction
            | R::OwnReadyNamed(..) => &[S::Play, S::Printed],
            R::HostLocked | R::HostUnlocked => &[S::Host, S::Printed],
            R::ActingMonster | R::ActingImbued => &[S::Play],
            R::HostConvictionAtLeast(..)
            | R::HostIncapacitated
            | R::HostControllerReadyImbuedAtLeast(..)
            | R::ImbuedRecoveryGranted => &[S::Host],
            R::OpponentMonster
            | R::OpponentNotInfernal
            | R::OpponentCapacityAtMost(..)
            | R::CombatMonsterVsMortal
            | R::CombatBothMonsters
            | R::ActorMonster
            | R::ActorHolds(..)
            | R::ActionTargetsImbued
            | R::CombatFromSuccess
            | R::PlayedTargetsActor
            | R::PlayedTakesControlOfActor => &[S::Play],
            R::HostNameUnusedThisAction | R::HostInCombat => &[S::Engine(Site::AbilityGate)],
            R::ActionNotEnteringCombat => &[S::Printed],
            R::HostMayReactLocked => &[S::Engine(Site::AbilityGate)],
            R::HostMayReact => &[S::Engine(Site::AbilityGate)],
            R::HostClan(..) | R::HostDiscipline(..) => &[S::Card, S::Host],
            R::CountersAtLeast(..) | R::CountersAtMost(..) | R::CardMode(..) | R::BasicVersion => {
                &[S::Card, S::Rider]
            }
            R::TargetNotNamed(..) | R::Controlled | R::NotControlled | R::TargetUnlocked => {
                &[S::MinionTarget, S::CardTarget]
            }
            R::ActorAndTargetSect(..) => &[S::MinionTarget],
            R::TargetCapacityAtLeast(..)
            | R::SameClanAsActor
            | R::Younger
            | R::TargetTitleIn(..)
            | R::TargetTitle(..)
            | R::TargetSameCircle { .. }
            | R::TargetClan(..) => &[S::MinionTarget, S::UncontrolledTarget],
            // A card's host or a vampire's own (Dive into Madness, Mustafa).
            R::ActorOlderThanHost => &[S::Engine(Site::AbilityGate)],
            R::ActingOfHostsMasterClan => &[S::Engine(Site::AbilityGate)],
            R::ActingIsHost => &[
                S::Engine(Site::AbilityGateCard),
                S::Engine(Site::AbilityGate),
            ],
            R::ActionTargetsHost => &[S::Card, S::Engine(Site::AbilityGate)],
            R::ActorIsChosen
            | R::ActorOfByCircle
            | R::ActorIsBysMaster
            | R::ActingIsBy
            | R::BleedAgainstHostController
            | R::ActorIsHost
            | R::CardControllersTurn
            | R::YouControlThisCard
            | R::OthersControlThisCard
            | R::WeaponBurnedOpponent
            | R::NotPlacedThisRound
            | R::SourceHeldWhenBlocking => &[S::Engine(Site::AbilityGateCard)],
            R::TargetCapacityBelowCounters | R::TargetBloodAtMostCounters => {
                &[S::Engine(Site::ControllerTarget)]
            }
            R::TargetIsHost | R::TargetChosenByHostThisTurn | R::TargetInCombatWithNormalDamage => {
                &[S::Engine(Site::AbilityTargets)]
            }
            R::OthersMinionPhase | R::InMasterPhase => &[S::Engine(Site::AbilityWindow)],
            R::PlayedGrappleOrAimByOpponent
            | R::PlayedCombatByOpponent
            | R::PlayedFrenzyOnYour(..) => &[S::Engine(Site::AsPlayed)],
            R::OncePerTurn => &[S::Engine(Site::CardRider(Hook::HostBlocks))],
            R::OneCopyPerTurn => &[S::Engine(Site::CardRider(Hook::UnlockPhase))],
            R::TargetOnOpposingCombatant => &[S::Engine(Site::CardTargetQualifies)],
            R::NotPlayedThisGame
            | R::NotPlayedThisGameBySeat
            | R::NotPlayedThisTurn
            | R::OnePerMethuselah
            | R::PlayedFrenzyOnActor
            | R::PlayedStrikeOrGrappleByOpponent
            | R::PlayedByOpposingMinion
            | R::PlayedRestrictsStrikesByOpponent
            | R::PlayedAimFrenzyGrappleByOpponent
            | R::CanAttemptBlockAnyone
            | R::MayBeLocked => &[S::Engine(Site::PlayOptions)],
            R::BlockedByOlder => &[S::Engine(Site::VampireRider(Hook::HostBlocked))],
            R::BledControlsReadyTitled => {
                &[S::Engine(Site::VampireRider(Hook::OwnBleedSuccessful))]
            }
            R::GehennaInPlay(..) | R::NoGehennaInPlay => &[S::Play, S::Host],
            R::PlayedActionCardAtYour(..) => &[S::Play, S::Engine(Site::AsPlayed)],
            R::CardUnlocked | R::CardLocked => &[S::Card, S::Engine(Site::AbilityGateCard)],
            R::ReferendumToBurnThisCard => &[S::Card],
            R::NotFirstChosen | R::TargetHasCorruption => &[S::MinionTarget],
            R::HostCombatOpponentNotReady => &[S::Card, S::Engine(Site::VampireAbility)],
            R::SuperiorVersion => &[S::Card, S::Rider],
            R::NotHost => &[
                S::CardTarget,
                S::Engine(Site::AbilityTargets),
                S::Engine(Site::AbilityGateCard),
            ],
            R::OwnMinionPhase => &[S::Engine(Site::AbilityWindow)],
            R::PlayedStrikeByOpponent => &[S::Engine(Site::PlayOptions), S::Engine(Site::AsPlayed)],
            R::HostAttempting | R::HostOutOfCombat => &[S::Engine(Site::AbilityGate)],
            R::CanAttemptBlock => &[
                S::Engine(Site::PlayOptions),
                S::Engine(Site::PlayTarget),
                S::Engine(Site::AbilityGate),
            ],
            R::TargetInCombatWithDamage => &[
                S::Engine(Site::PlayTarget),
                S::Engine(Site::CardTargetQualifies),
                S::Engine(Site::AbilityTargets),
            ],
            R::TargetWeaponStrikeUsable => &[S::Engine(Site::CardTargetQualifies)],
        }
    }
}

/// The requirement an ability of `payer` paying `cost` carries, offered
/// only when payable (abilities only: a rider burns what there is).
pub fn implied_requirement(payer: Actor, cost: Effect) -> Option<Requirement> {
    match cost {
        Effect::LockSelf => Some(Requirement::CardUnlocked),
        Effect::LockHost => Some(Requirement::HostUnlocked),
        Effect::BurnConviction(n) => Some(Requirement::HostConvictionAtLeast(n)),
        Effect::BurnActorBlood(n) => Some(Requirement::ActorBloodAtLeast(n)),
        Effect::HostBurnBlood(n) | Effect::HostBurnLife(n) => Some(match payer {
            Actor::AnyReadyVampire | Actor::ActingMinion => Requirement::ActorBloodAtLeast(n),
            _ => Requirement::HostBloodAtLeast(n),
        }),
        _ => None,
    }
}

/// The requirements `ability`'s costs imply (`implied_requirement`).
pub fn implied_requirements(ability: &ModeDef) -> impl Iterator<Item = Requirement> + '_ {
    ability
        .effects
        .iter()
        .filter_map(|&e| implied_requirement(ability.actor, e))
}

#[cfg(test)]
mod tests {
    use super::super::defs::DEFS;
    use super::super::vampires::VAMPIRES;
    use super::super::{TargetKind, Term, Window};
    use super::*;

    /// Requirements no evaluator reads where they sit, known: the card, the
    /// requirement, why. An entry nothing matches fails the test: drop it
    /// once its gap is closed.
    const KNOWN_GAPS: &[(&str, Requirement, &str)] = &[];

    /// The evaluators `Engine::ability_gate` runs on a card ability's
    /// requirements, by actor: the card's (an `ActingCandidate`'s for
    /// `CardUnlocked` only), the host's if any, the play's.
    fn card_ability_readers(ability: &ModeDef) -> Option<Vec<Slot>> {
        use Slot::*;
        let mut readers = match ability.actor {
            Actor::Controller
            | Actor::ControllersPrey
            | Actor::Methuselah
            | Actor::ChosenController => {
                vec![Card, Play]
            }
            Actor::AnyReadyVampire
            | Actor::OpposingCombatant
            | Actor::ActingMinion
            | Actor::Host
            | Actor::HostOutOfCombat
            | Actor::ForOwnCombatant
            | Actor::UnlockedOutOfCombat => vec![Card, Host, Play],
            Actor::ActingCandidate => vec![Host, Play],
            _ => return None,
        };
        readers.extend(GATE);
        Some(as_played(ability, readers))
    }

    /// The engine sites reading every in-play card ability.
    const GATE: [Slot; 3] = [
        Slot::Engine(Site::AbilityWindow),
        Slot::Engine(Site::AbilityGate),
        Slot::Engine(Site::AbilityGateCard),
    ];

    /// The evaluators the engine runs on a vampire's printed ability.
    fn vampire_ability_readers(ability: &ModeDef) -> Vec<Slot> {
        use Slot::*;
        let readers = vec![
            Host,
            Play,
            Engine(Site::AbilityWindow),
            Engine(Site::AbilityGate),
            Engine(Site::VampireAbility),
        ];
        as_played(ability, readers)
    }

    /// An ability cancelling a card as it is played is filtered again by
    /// `Engine::as_played_request`.
    fn as_played(ability: &ModeDef, mut readers: Vec<Slot>) -> Vec<Slot> {
        if ability.windows.contains(&Window::CardAsPlayed) {
            readers.push(Slot::Engine(Site::AsPlayed));
        }
        readers
    }

    /// The evaluators the engine runs on a card rider's requirements at
    /// `hook`: `Engine::fired_riders`' (`None`: a hook read past it — a
    /// card's destination as it is burned, Saatet-ta).
    fn card_rider_readers(hook: Hook) -> Option<Vec<Slot>> {
        use Slot::*;
        if hook == Hook::SelfBurned {
            return None;
        }
        let mut readers = if hook.reads_action(false) {
            vec![Play, Rider]
        } else {
            vec![Rider, Host]
        };
        readers.push(Engine(Site::CardRider(hook)));
        Some(readers)
    }

    /// As `card_rider_readers`, for a vampire's printed riders:
    /// `Engine::printed_riders`' (`None`: read on the crypt card before the
    /// vampire is in play).
    fn vampire_rider_readers(hook: Hook) -> Option<Vec<Slot>> {
        use Slot::*;
        if hook == Hook::EntersPlay {
            return None;
        }
        let mut readers = if hook.reads_action(true) {
            vec![Play, Host]
        } else {
            vec![Host]
        };
        readers.push(Engine(Site::VampireRider(hook)));
        Some(readers)
    }

    #[derive(Default)]
    struct Walk {
        /// The card, the requirement not read (none: a position not
        /// classified), the message.
        failures: Vec<(&'static str, Option<Requirement>, String)>,
        checked: usize,
    }

    impl Walk {
        fn check(&mut self, name: &'static str, at: &str, reqs: &[Requirement], readers: &[Slot]) {
            for &r in reqs {
                self.checked += 1;
                if !r.slots().iter().any(|s| readers.contains(s)) {
                    let msg = format!(
                        "{name}: {r:?} at {at}, read by {:?}, where {readers:?} read",
                        r.slots()
                    );
                    self.failures.push((name, Some(r), msg));
                }
                // The alternatives are read by the evaluators reading the
                // `AnyOf` itself.
                if let Requirement::AnyOf(alternatives) = r {
                    let live: Vec<Slot> = readers
                        .iter()
                        .copied()
                        .filter(|s| r.slots().contains(s))
                        .collect();
                    for alt in alternatives {
                        self.check(name, at, alt, &live);
                    }
                }
            }
        }

        fn unclassified(&mut self, name: &'static str, what: String) {
            self.failures
                .push((name, None, format!("{name}: {what} not classified")));
        }

        /// Conditional bonuses and referendum term choices nested in
        /// effects: read on a minion.
        fn effects(&mut self, name: &'static str, at: &str, effects: &[Effect]) {
            for e in effects {
                match *e {
                    Effect::If { requires, then, .. } => {
                        self.check(name, at, requires, &[Slot::MinionTarget]);
                        self.effects(name, at, std::slice::from_ref(then));
                    }
                    Effect::IfBleedSuccessful(es) => self.effects(name, at, es),
                    Effect::CallReferendum(d) => {
                        for t in d.terms {
                            if let Term::ChooseMinions { filter, .. }
                            | Term::ChooseMinionsX { filter }
                            | Term::DistributeBlood { filter, .. } = *t
                            {
                                self.check(name, at, filter, &[Slot::MinionTarget]);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        /// A mode's requirements read by `readers`, its target filter by
        /// the target kind's evaluator (and, for a minion, `minion`'s
        /// engine checks), its nested conditions.
        fn mode(
            &mut self,
            name: &'static str,
            at: &str,
            m: &ModeDef,
            readers: &[Slot],
            minion: &[Site],
        ) {
            self.check(name, at, m.requires, readers);
            if let Some(t) = m.target {
                let by: Vec<Slot> = match t.kind {
                    TargetKind::Minion => std::iter::once(Slot::MinionTarget)
                        .chain(minion.iter().map(|&s| Slot::Engine(s)))
                        .collect(),
                    TargetKind::Seat => vec![Slot::SeatTarget],
                    TargetKind::Uncontrolled => vec![Slot::UncontrolledTarget],
                    TargetKind::Card => {
                        vec![Slot::CardTarget, Slot::Engine(Site::CardTargetQualifies)]
                    }
                };
                self.check(name, &format!("{at}'s {:?} target", t.kind), t.filter, &by);
            }
            self.effects(name, at, m.effects);
        }
    }

    // A requirement no evaluator reads where it sits passes as `true`: the
    // mask would be silently permissive.
    #[test]
    fn every_requirement_sits_where_it_is_read() {
        let mut w = Walk::default();
        for d in DEFS.iter() {
            for m in d.modes {
                let readers = [Slot::Play, Slot::Engine(Site::PlayOptions)];
                w.mode(d.name, "a mode", m, &readers, &[Site::PlayTarget]);
            }
            for (i, a) in d.abilities.iter().enumerate() {
                let at = format!("ability {i} ({:?})", a.actor);
                let Some(readers) = card_ability_readers(a) else {
                    w.unclassified(d.name, at);
                    continue;
                };
                let mut minion = vec![Site::AbilityTargets];
                if matches!(a.actor, Actor::Controller | Actor::ControllersPrey) {
                    minion.push(Site::ControllerTarget);
                }
                w.mode(d.name, &at, a, &readers, &minion);
            }
            for r in d.riders {
                let at = format!("its {:?} rider", r.hook);
                match card_rider_readers(r.hook) {
                    Some(readers) => w.check(d.name, &at, r.requires, &readers),
                    None if r.requires.is_empty() => {}
                    None => w.unclassified(d.name, at.clone()),
                }
                w.effects(d.name, &at, r.effects);
            }
        }
        for v in VAMPIRES.iter() {
            for (i, a) in v.abilities.iter().enumerate() {
                let at = format!("ability {i} ({:?})", a.actor);
                let readers = vampire_ability_readers(a);
                w.mode(v.name, &at, a, &readers, &[Site::AbilityTargets]);
            }
            for r in v.riders {
                let at = format!("its {:?} rider", r.hook);
                match vampire_rider_readers(r.hook) {
                    Some(readers) => w.check(v.name, &at, r.requires, &readers),
                    None if r.requires.is_empty() => {}
                    None => w.unclassified(v.name, at.clone()),
                }
                w.effects(v.name, &at, r.effects);
            }
            for p in v.printed {
                w.check(v.name, "a printed modifier", p.requires, &[Slot::Printed]);
            }
        }
        assert!(w.checked > 2500, "{} requirements walked", w.checked);
        let known = |name: &str, r: Option<Requirement>| {
            KNOWN_GAPS
                .iter()
                .any(|&(n, q, _)| n == name && Some(q) == r)
        };
        let unknown: Vec<&str> = w
            .failures
            .iter()
            .filter(|(n, r, _)| !known(n, *r))
            .map(|(_, _, msg)| msg.as_str())
            .collect();
        assert!(unknown.is_empty(), "{}", unknown.join("\n"));
        for &(name, r, why) in KNOWN_GAPS {
            assert!(
                w.failures
                    .iter()
                    .any(|(n, q, _)| *n == name && *q == Some(r)),
                "{name}'s {r:?} is read now ({why} closed?): drop it from KNOWN_GAPS"
            );
        }
    }

    // An ability paying a cost carries the requirement the cost implies, or
    // it is offered unpayable.
    #[test]
    fn every_ability_cost_carries_its_requirement() {
        use Requirement as R;
        let mut failures = Vec::new();
        let mut checked = 0;
        let abilities = DEFS
            .iter()
            .flat_map(|d| d.abilities.iter().map(move |a| (d.name, a)))
            .chain(
                VAMPIRES
                    .iter()
                    .flat_map(|v| v.abilities.iter().map(move |a| (v.name, a))),
            );
        for (name, a) in abilities {
            for want in implied_requirements(a) {
                checked += 1;
                // Requiring more blood than the cost does too.
                let met = a.requires.iter().any(|&r| match (want, r) {
                    (R::HostBloodAtLeast(n), R::HostBloodAtLeast(m))
                    | (R::ActorBloodAtLeast(n), R::ActorBloodAtLeast(m)) => m >= n,
                    _ => r == want,
                });
                if !met {
                    failures.push(format!("{name}: {:?} without {want:?}", a.effects));
                }
            }
        }
        assert!(checked > 100, "{checked} costs walked");
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// A card put on the opposing minion needs it there: no target once it
    /// burned (rules-qa 75.1).
    #[test]
    fn every_card_put_on_the_opponent_needs_it_present() {
        let mut checked = 0;
        for d in DEFS.iter() {
            for m in d
                .modes
                .iter()
                .filter(|m| m.effects.contains(&Effect::PutOnOpponent))
            {
                checked += 1;
                assert!(
                    m.requires.contains(&Requirement::OpponentPresent),
                    "{}: {:?} without OpponentPresent",
                    d.name,
                    m.effects
                );
            }
        }
        assert!(checked >= 3, "{checked} modes walked");
    }
}
