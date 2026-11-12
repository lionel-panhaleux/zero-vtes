//! Where each effect is interpreted: an exhaustive classification of
//! `Effect` (a new variant does not compile until classified), the
//! interpreters dispatching on it, checked over every definition by the
//! tests below.

use super::Effect;

/// The interpreter resolving an effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    /// A card played resolving (its action successful, entering play):
    /// `Engine::apply_card_effect`'s own.
    Play,
    /// Resolved the same from a play and from an in-play ability:
    /// `Engine::apply_shared_effect`.
    Shared,
    /// An in-play ability used, its costs included:
    /// `Engine::resolve_ability`'s own.
    Ability,
    /// The action underway: `Engine::apply_action_effect`.
    Action,
    /// The combat underway: `Engine::apply_combat_effect`.
    Combat,
    /// The referendum underway: `Engine::apply_vote_effect`.
    Vote,
    /// Resolved by the action's success (`Engine::succeed`): an action
    /// card's, a granted action's, a successful bleed's.
    Success,
    /// Resolved as an ally or a retainer enters play
    /// (`Engine::apply_entry_effects`).
    Entry,
    /// Resolved by the loop firing its hook only (a rider's).
    Rider,
    /// Read by name where the card or the ability is offered, declared or
    /// played, or by the event it waits for: the interpreters pass it.
    Marker,
}

/// How a granted action's success resolves an effect of its ability
/// (`Effect::granted`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Granted {
    /// Read as the action is declared or paid: passed as it resolves.
    Passed,
    /// The resolver's own arm (`succeed`'s `ActionKind::Ability`).
    Own,
    /// `Engine::apply_card_effect`'s arm, as a card played resolves it.
    Play,
    /// `Engine::apply_shared_effect`.
    Shared,
}

impl Effect {
    /// The interpreter resolving this effect.
    pub fn family(self) -> Family {
        self.class().0
    }

    /// Does this effect prevent damage? The one prevention class: read it,
    /// never list preventions by hand.
    pub fn prevents(self) -> bool {
        self.class().1
    }

    /// Does this effect take control of its target minion? The one list
    /// (React with Conviction's `PlayedTakesControlOfActor`; the target
    /// masks of `GameState::controls_vampire_copy`): a new control-taking
    /// effect joins it.
    pub fn takes_control(self) -> bool {
        matches!(
            self,
            Effect::StealTargetAlly
                | Effect::StealTargetVampire
                | Effect::TakeControlOfTargetUntilUnlock
                | Effect::TakeControlOfTargetUntilMinionPhaseEnd
                | Effect::TakeControlOfTargetUntilEndOfTurn
                | Effect::TakeControlOfTargetUntilNextTurn
                | Effect::UnlockAndControlTargetUntilEndOfTurn
        )
    }

    /// Does this effect move its target card in play to its chooser's
    /// control by choice? A unique copy its chooser already controls is no
    /// target (rulebook "Contested Cards": never by choice; advanced rules
    /// §1.13.1).
    pub fn takes_card(self) -> bool {
        matches!(
            self,
            Effect::StealTargetCard | Effect::TargetCardToActor | Effect::StealLocation
        )
    }

    /// How a granted action (`ActionKind::Ability`) resolves this effect of
    /// its ability as it succeeds (`Engine::succeed` dispatches on it):
    /// `None`, it cannot sit there.
    pub fn granted(self) -> Option<Granted> {
        use Effect as E;
        Some(match self {
            E::DeclareAction(_)
            | E::AddStealth(_)
            | E::StealthIfOwnTarget(_)
            | E::BurnSourceOnDeclare
            | E::MandatoryAction
            | E::PayBlood(_)
            | E::PayBloodPerCounter
            | E::PayLife(_)
            | E::PayPool(_)
            | E::PayConviction
            | E::SameActionAs(_)
            | E::LockSelf
            | E::TargetCannotBlock
            | E::HuntAction
            | E::InResultingCombat(_)
            | E::DirectedAtSourceController => Granted::Passed,
            E::AddCounter(_)
            | E::GreatBeastAtEleven
            | E::RemoveAllSourceCounters
            | E::CountersToPool
            | E::SearchToHand { .. }
            | E::ExchangeHandForAsh
            | E::AllyFromAshLocked(_)
            | E::AshVampireToUncontrolled
            | E::EnterCombatWithTarget
            | E::StealTargetCard
            | E::TargetCardToChosen
            | E::TargetYieldsTitle
            | E::MoveSourceToTarget
            | E::MoveSourceToActor
            | E::GainLiaison
            | E::AddHostBlood(_)
            | E::AshCardToHandNamed
            | E::AshCardOfToHand(_)
            | E::AshCardToHandOnSuccess(_)
            | E::RemoveUncontrolledForBlood(_)
            | E::DrawThenDiscardDown(_)
            | E::PreyAshPairHunt(_)
            | E::DrawUpToThenDiscardDown(_)
            | E::RemoveActorToBurnTarget
            | E::PaidConvictionToTarget
            | E::ActingGainsConviction
            | E::ReorderTop { .. }
            | E::HeldVampireBecomesGargoyle
            | E::FledglingFromNamedMaster
            | E::SearchDisciplineMasterOntoActor(_)
            | E::SearchArchetypeOntoActor
            | E::SearchEquipForTarget
            | E::SearchEquip(_)
            | E::SearchEquipFree(_)
            | E::BurnCorruptionFromTarget
            | E::GraftOnTarget
            | E::StealTargetTitle
            | E::StealBloodFromTarget(_)
            | E::StealBloodUpTo(_)
            | E::CorruptTargetMayStealAtCapacity
            | E::TakeControlOfTargetUntilNextTurn
            | E::PreySlotToOwnSlot
            | E::ActorBloodToSourceCardUpTo { .. }
            | E::ActorBloodToSourceCard(_)
            | E::StealFromTargetSeatSlot(_)
            | E::LookAtTargetLibraryTopBurnOne(_)
            | E::CryptNamedToUncontrolled(_)
            | E::TargetSeatTakesSourceAlly
            | E::UnlockAndControlTargetUntilEndOfTurn
            | E::DamageSeatSectVampires { .. }
            | E::TargetGetsTitle(_)
            | E::ActorFreedFromSlavery
            | E::HostBloodToUncontrolledTarget(_)
            | E::PeekHand
            | E::BurnSource => Granted::Own,
            E::StealTargetAlly
            | E::StealPoolFromTarget(_)
            | E::DiscardFromTargetHand
            | E::StealLocation
            | E::TargetCardToOwnersLibraryForCost
            | E::CryptClanToUncontrolled { .. } => Granted::Play,
            e if e.family() == Family::Shared => Granted::Shared,
            _ => return None,
        })
    }

    /// The family and whether it prevents, classified together so a new
    /// effect is classified where it is routed.
    fn class(self) -> (Family, bool) {
        use Effect as E;
        match self {
            E::ChooseSecondOfClan(..)
            | E::ChooseOwnOfClan(..)
            | E::ChooseTermsMinion
            | E::ChooseTarget
            | E::ChooseCardCircle
            | E::ChooseActor
            | E::RevealCryptTop { .. }
            | E::EachCryptTopPick(..)
            | E::BecomesAlly(..)
            | E::AshurTablets
            | E::GiveBoonPool
            | E::RotschreckEndsCombat
            | E::FrenzyRound
            | E::BecomesVampire { .. }
            | E::PutOnTarget
            | E::ConvictionOnTarget
            | E::TargetSeatTakesCard
            | E::DualFormCopy
            | E::DescendActor
            | E::DerangeTarget
            | E::PutInPlay
            | E::EnterWithCounters(..)
            | E::EnterWithCountersPerX { .. }
            | E::EntersLocked
            | E::StoreFromOwnHand
            | E::RearrangeEquipment
            | E::LockTargetsSkipUnlock
            | E::ChooseUncontrolledTarget
            | E::GiveHold { .. }
            | E::PutOnTargetCardStoring(..)
            | E::StoreFromHandX(..)
            | E::UnlockActorIfClanCapacity { .. }
            | E::MoveBloodTargetToPoolX
            | E::Layer(..)
            | E::StealTargetAlly
            | E::TrainInUncontrolled { .. }
            | E::PoolToVampires
            | E::BurnChosenInPlay
            | E::CountersFromTargetCapacity
            | E::RetainerToStartingLife
            | E::DiscardFromAllHands
            | E::EachOtherDiscardsAtRandom(..)
            | E::EachOtherLocksYoungerVampire
            | E::StealFromTorpid { .. }
            | E::PutOnActorNowStacking
            | E::LastingOnActor
            | E::PutOnOpponent
            | E::BurnLibraryTop(..)
            | E::PutOnActorNow
            | E::MorbidityTake(..)
            | E::TrapPresses
            | E::BeastPresses { .. }
            | E::EatAshAllyToTarget(..)
            | E::Stun
            | E::StealLocation
            | E::StolenToReadyForBlood(..)
            | E::EatAshAlly { .. }
            | E::BloodToTargetFull
            | E::MoveBloodXToUncontrolledTarget
            | E::HoldTargetVampire
            | E::RememberTargetSeatsVotes
            | E::EnterWithCountersPerOwn { .. }
            | E::EnterWithCountersPerOwnAnarch
            | E::CryptClanToUncontrolled { .. }
            | E::BankBloodAmongUncontrolled { .. }
            | E::RemoveBleedTargetAshCards(..)
            | E::SearchLibraryToHandOrHavenOnActor(..)
            | E::SudarioRefraction
            | E::OwnReadyVampiresBloodToPool
            | E::GainPerUnlockedCircleMember
            | E::BloodToUncontrolledCircle
            | E::PetraResonance
            | E::TakeBleedForTarget
            | E::CryptTopToUncontrolled
            | E::BurnRangedWeaponsOnTarget { .. }
            | E::FourthCycle(..)
            | E::BurnAllBoons
            | E::AutoPassFirstSabbatReferendum
            | E::BloodToEachUncontrolledOfSect(..)
            | E::StealTargetCard
            | E::TargetCardToChosen
            | E::FuryEnters
            | E::CountersFromActorCapacity
            | E::GreatBeastAtEleven
            | E::KaymakliNightmares
            | E::ActingBloodToNewUpTo(..)
            | E::RedeemAshVampire
            | E::AshVampireToReady
            | E::BloodToPickedUncontrolled(..)
            | E::PickedUncontrolledToReady
            | E::SearchLibraryToResearch(..)
            | E::BurnOthersLabeledOnTarget(..)
            | E::LaibonOrCryptDraw(..)
            | E::DiseaseTarget
            | E::StealBloodFromTarget(..)
            | E::NextReferendumPasses
            | E::MergeWithBase
            | E::LockTargetMergeFromCrypt
            | E::DiscardFromTargetHand
            | E::StealPoolFromTarget(..)
            | E::TargetControllerBurnsPool(..)
            | E::ActingControllerBurnsPool(..)
            | E::ActingBurnsBlood(..)
            | E::PutOnActingNow
            | E::RemovePreyAshToBurnPool { .. }
            | E::LockPlayedCard
            | E::ActorTakesAggravated { .. }
            | E::PutOnActor
            | E::PutOnActorRemembering
            | E::CompareCryptTopsWithBlocker { .. }
            | E::If { .. }
            | E::GameOfMalkav
            | E::DrawCryptBloodIfBlackHand(..)
            | E::SearchPutInPlayUpToX { .. }
            | E::ActingBloodToNew(..)
            | E::BankBloodToNew(..)
            | E::ChooseActorAndPicksUnlock
            | E::PutOnTargetCard
            | E::PreyBurnsPoolPerNamed { .. }
            | E::BloodToOwnUncontrolled { .. }
            | E::GainEdgeOrBurnItForPool(..)
            | E::TargetsLeaveTorpor
            | E::SearchLibraryToTop(..)
            | E::ShatterAtCopies { .. }
            | E::DiscardNamedFromHand(..)
            | E::DamageTargets(..)
            | E::RaiseAshAllyWithThis { .. }
            | E::DrainToCardEachRound
            | E::TargetCardToActor
            | E::NextActionFromAsh
            | E::ActionStealthThisTurn { .. }
            | E::DamageTargetCardHost(..)
            | E::LookTopTakeToHand { .. }
            | E::RememberAshNamed
            | E::BurnActorConvictions
            | E::MastersFromHandOntoNew(..)
            | E::SearchRecruit
            | E::GainPoolByOwnPool { .. }
            | E::GuessSecretNumbers(..)
            | E::AuctionControlOfTarget
            | E::TargetCardToOwnersLibraryForCost
            | E::PreyMayTakeFreeEdge => (Family::Play, false),
            E::TargetLeavesTorpor
            | E::PeekNeighborHands
            | E::PeekActingControllersHand
            | E::AshToLibraryTop { .. }
            | E::ActorLooksAtLibraryTop(..)
            | E::TargetLeavesIncapacitated
            | E::BloodToActing(..)
            | E::PreyBurnsPool(..)
            | E::BurnOwnPool(..)
            | E::PreyBurnsPoolIfRicher(..)
            | E::GainPool(..)
            | E::SearchLibraryToHand { .. }
            | E::LoseVictoryPoints(..)
            | E::BloodToUncontrolledTarget(..)
            | E::LockHeldForUncontrolledBloodX(..)
            | E::LockHeldToUncontrolledX(..)
            | E::PoolToUncontrolledTarget(..)
            | E::BloodToTarget(..)
            | E::LockActor
            | E::EndCombat
            | E::BloodToTargetToStart(..)
            | E::TargetBurnsBlood(..)
            | E::TargetBurnLife(..)
            | E::TargetRetainerBurnsLife(..)
            | E::UnlockActor
            | E::SpendOffActionEquip(_)
            | E::ActorLeavesTorpor
            | E::ActorGainsBloodIfAtLeast { .. }
            | E::UnlockActorAtDiscard
            | E::TakeControlOfTargetUntilUnlock
            | E::TakeControlOfTargetUntilMinionPhaseEnd
            | E::TakeControlOfTargetUntilEndOfTurn
            | E::StopControlChange
            | E::SaveFromHuntBurn
            | E::SaveFromTorpor
            | E::MayBleedAgainThisTurn
            | E::BurnActorBlood(..)
            | E::DrawThenDiscardDown(..)
            | E::UnlockTarget
            | E::LockTarget
            | E::TargetToTorpor
            | E::BurnOwnCorruptionFromTarget(..)
            | E::CorruptTargetMinion
            | E::BurnTargetMinion
            | E::TargetLosesTitle
            | E::MergeWithAdvanced
            | E::BurnHalfPreySlot
            | E::ActorGainsBlood(..)
            | E::BurnPreyAllyUpToBleed
            | E::LockOtherAnarch
            | E::FollowActing { .. }
            | E::ChooseBlindSpot
            | E::PlayerBurnsPool(..)
            | E::BurnTargetOnSecondAction
            | E::SacrificeTarget
            | E::CircleBloodMoves { .. }
            | E::BurnTargetCard
            | E::EndTargetCardHold
            | E::StoreTopCards(..)
            | E::HandSizeUntilUnlock(..)
            | E::ActorLasting(..)
            | E::TargetLasting(..)
            | E::TargetBecomesSect(..)
            | E::ActorBecomesSect(..)
            | E::UnlockChosen
            | E::ActorBecomesBlackHand
            | E::RemoveTargetForPool
            | E::RemoveOwnAshCards(..)
            | E::UnlockActing
            | E::BurnBledLibraryTop(..)
            | E::LookAtLibraryTopPerGehenna
            | E::BurnEdge
            | E::TargetBecomesBlackHand
            | E::RemoveCryptTop
            | E::BurnTargetCryptTop
            | E::ActorBecomesClan(..)
            | E::TargetBecomesRedList
            | E::ActorBloodToCapacity
            | E::IfActingUnlockAndFill
            | E::BurnRememberedContract
            | E::DamageTarget(..)
            | E::LockOwnReadyOfClan(..)
            | E::TaintTarget
            | E::StealTargetVampire
            | E::CommandUncontrolled { .. }
            | E::EmployNamed
            | E::EquipNamed
            // A wake's ally rider, in an action or before it (rules-qa 96.4).
            | E::AllySkipsNextUnlock
            | E::DiscardUpTo(..)
            | E::ClanBleedThisTurn(..)
            | E::LookAtRandomPreyHandCard
            | E::DrawThenHandCardAway
            | E::LockActorHeld(..)
            | E::LockTargetCardHost
            | E::TargetToUncontrolled
            | E::FailsIntoCombatWithActor
            | E::AggravatedDamageTarget(..)
            | E::OathCounterOnTarget => (Family::Shared, false),
            E::CounterToTarget(..)
            | E::BloodPerHostage
            | E::BurnTargetHostages
            | E::UnlockActingIfSuccessful(..)
            | E::OwnClanBurnsBlood(..)
            | E::ExchangeForAnarchCard
            | E::GrantDiscardAction
            | E::BloodToChosen(..)
            | E::GiveToPrey
            | E::LockPreyVampires(..)
            | E::BloodToCard(..)
            | E::CountersToUncontrolledTarget
            | E::DrawCryptThenRemove
            | E::RemoveCryptTopAnarch(..)
            | E::BloodToTargetX
            | E::LockTargetCard
            | E::StealPoolFromBoonTarget
            | E::PoolToCounter(..)
            | E::PayPool(..)
            | E::PayConviction
            | E::SpendMasterPhaseAction
            | E::LockSelf
            | E::BurnConviction(..)
            | E::BurnTargetConviction(..)
            | E::FailsIntoCombatWithHost
            | E::PeekOpponentControllersHand
            | E::MayCancelCombat
            | E::DamageOpponentByStrength
            | E::LookAtLibraryTop(..)
            | E::LookAtTurnLibraryTop
            | E::LookAtEachLibraryTop
            | E::LookAtTargetLibraryTopBurnOne(..)
            | E::StealFromTargetSeatSlot(..)
            | E::ActorBloodToSourceCardUpTo { .. }
            | E::ActorBloodToSourceCard(..)
            | E::RemoveHostFromGame
            | E::MayBurnLookedMaster(..)
            | E::MayBottomLookedForBlood(..)
            | E::GainConviction { .. }
            | E::LeaveIncapacitated
            | E::SpendDiscardPhaseAction
            | E::AddHostBlood(..)
            | E::AddCounter(..)
            | E::AddCounterToTargetCard(..)
            | E::BurnSelfAtCounters(..)
            | E::RemoveCounter(..)
            | E::BurnSelfWithoutCounters
            | E::HandSizeBonusUntilEndOfTurn(..)
            | E::MoveBetweenVampires(..)
            | E::PeekHand
            | E::RevealOwnHandToAll
            | E::PeekHostControllersHand
            | E::PredatorTakesControl
            | E::BurnSelf
            | E::LockHost
            | E::LockHostHeld(..)
            | E::LockHostHeldX(..)
            | E::LookAtRandomPredatorHandMayExchange
            | E::SearchOntoHost(..)
            | E::SearchOntoTarget(..)
            | E::BurnOwnLife(..)
            | E::StealBloodToHostX
            | E::HostBloodToTarget(..)
            | E::HostBloodToUncontrolledTarget(..)
            | E::ChooseTargetThisTurn
            | E::ChooseTargetByAbility
            | E::TargetBloodToHost(..)
            | E::EnterCombatWithActing
            | E::TargetEntersCombatWithActing
            | E::ReduceStealth(..)
            | E::UnlockSelf
            | E::ReduceActionCost(..)
            | E::HostBurnBlood(..)
            | E::HostBurnBloodToUnlock
            | E::HostBloodToPool(..)
            | E::ActorBloodToTarget(..)
            | E::PoolToHostBlood(..)
            | E::HuntingGroundBlood(..)
            | E::HuntingGroundTwoOldest
            | E::AddInterceptX
            | E::RevealTopCard { .. }
            | E::BurnTopCardByDiscipline { .. }
            | E::RevealCryptTopForCopy { .. }
            | E::ActingControllerDiscardsAtRandom
            | E::TargetSeatDiscardsAtRandom(..)
            | E::TakeBleedGain
            | E::SlaveTakesCombat
            | E::StandInIfBlocked
            | E::HostLeavesTorpor
            | E::ShuffleHostIntoLibrary
            | E::DamageActingNow(..)
            | E::PayPoolToCardController(..)
            | E::ReorderTopMayBottom(..)
            | E::ReduceBleedX
            | E::SearchMageAlly
            | E::YawpCourtCombat
            | E::HuntingGroundLife(..)
            | E::HostBurnLife(..)
            | E::TargetSeatTakesSourceAlly
            | E::RaiseAllyWhenEmpty(..)
            | E::AtCountersBurnAndGain { .. }
            | E::GainMasterPhaseAction
            | E::PayPoolX
            | E::MethuselahVotesX { .. }
            | E::ContinuePressForOwnCombatant
            | E::RemoveCountersX
            | E::AddVotesX
            | E::TargetBloodToPool(..)
            | E::PoolToTargetBlood(..)
            | E::CounterToTargetBlood
            | E::CryptNamedToUncontrolled(..)
            | E::ReduceActionBlood(..)
            | E::PayActionWithCountersX
            | E::PoolToCounterPlusBank { .. }
            | E::StoreFromPreyAsh
            | E::StoreFromOwnAsh
            | E::StoredToLibraryTop
            | E::WatchDamagesActing(..)
            | E::TargetHostControllerTakesControl
            | E::OthersHandSmallerIfSame(..)
            | E::RaiseNamedAllyFromAsh(..)
            | E::ShuffleHandDrawUp
            | E::PreyUncontrolledBloodToPool(..)
            | E::PeekHandsOfReadyClan(..)
            | E::MarkBleedToll
            | E::HostBleedThisTurn(..)
            | E::BleedToRemembered
            | E::HostVotes(..)
            | E::HostAsIfUnlockedForAuspex
            | E::CountersToPool
            | E::CounterToPool
            | E::CommitStrike
            | E::CommitStrikeOf(..)
            | E::LoadGunFromStore
            | E::DiscardStoredCopy
            | E::ChosenUncontrolledToReady
            | E::GhoulFromLibraryTop(..)
            | E::GiveBloodToOthers
            | E::GiveBloodToOwnAnarch
            | E::BurnUnlessControllerPays(..)
            | E::PayPoolPerOtherEvent
            | E::DiscardAtRandom(..)
            | E::ContinuePressEachCombat
            | E::TargetVotesPerCounter
            | E::TargetVotes(..)
            | E::ToggleLockTarget
            | E::HuntStealthUntilUnlock(..)
            | E::DrawThenBottom
            | E::HandToBottomX
            | E::CardChoosesTarget
            | E::PreyPoolToCounter(..)
            | E::MethuselahVotes { .. }
            | E::TakeControlOfHost
            | E::UnlockHost
            | E::ActionCardToLibraryShuffled
            | E::ReturnHost { .. }
            | E::OutOfPlayHostGainsBlood(..)
            | E::ActorTakesAnyAction
            | E::HostSuperiorChosenUntilMaster
            | E::DiscardHand
            | E::DiscardUpToFromTargetHand(..)
            | E::HandSizePerActiveReadyMinions
            | E::PlayFirstLibraryAim
            | E::RemoveNamedFromPreyAsh { .. }
            | E::MarkTargetCardThisTurn
            | E::HostToUncontrolled
            | E::StealHost
            | E::HealAggravatedForBlood(..) => (Family::Ability, false),
            E::TakeOverAction
            | E::CloseEachRoundIfBlocked
            | E::PoolForInterceptThisAction
            | E::AnimalBoostIfSuccessful { .. }
            | E::PutOnRecruitedIfSuccessful(..)
            | E::ActionLayer(..)
            | E::AddStealth(..)
            | E::AddIntercept(..)
            | E::AttemptingIntercept(..)
            | E::BleedMod { .. }
            | E::Wake
            | E::VampiresCannotBlock
            | E::SmallMonstersCannotBlock { .. }
            | E::SmallNonInfernalCannotBlock(..)
            | E::DamageAfterResolution(..)
            | E::TargetCannotBlock
            | E::NoUnlockThisAction
            | E::TargetNoReactions
            | E::NextReactionCosts(..)
            | E::ReduceBleedToZero
            | E::EndBlockedAction
            | E::StealthForNextActions
            | E::RedirectBleed
            | E::RedirectBleedToActingPrey
            | E::BurnActingMinion
            | E::DamageActing(..)
            | E::BurnActingHeld(..)
            | E::ActionStealthMinus(..)
            | E::DamageActingAndBlocker(..)
            | E::TargetInterceptThisAction(..)
            | E::ActionFails
            | E::ActingBurnsOrFails { .. }
            | E::SplitBloodGain
            | E::ActingSkipsNextUnlock
            | E::DeferReactionReplacement
            | E::EndActionIfBlocked
            | E::MayCancelCombatIfBlocked { .. }
            | E::ContinueUnblocked
            | E::AlliesCannotBlock
            | E::AlliesCannotBlockUnless(..)
            | E::StrengthThisAction(..)
            | E::AggravatedHandsThisAction
            | E::HandStrikeDamageThisAction(..)
            | E::DawnOperation { .. }
            | E::PutOnActorIfNoBlock
            | E::PutOnActingIfBlocks
            | E::BurnIfNoBlock(..)
            | E::BlockTollBlood(..)
            | E::LockAttempting
            | E::QueueCombatWithAttempting
            | E::QueueCombatAfterBlock { .. }
            | E::PutOnActorIfBleedSuccessful
            | E::HuntBonusThisAction(..)
            | E::StrikeTollIfBlocks(..)
            | E::UnlockActingForActorBlood(..)
            | E::EndBlockedActionLocked
            | E::ContinueAfterResolution
            | E::UnlockForBloodIfSuccessful(..)
            | E::PoolIfAttemptingDoesNotBlock
            | E::TargetBurnsBloodForIntercept
            | E::NoLockForBlock
            | E::AttemptingBurnsBlood(..)
            | E::AttemptingToTorpor
            | E::CancelAction
            | E::LockBlocker
            | E::LockPicks
            | E::StealthPerPick
            | E::UnlockPicksIfSuccessful(..)
            | E::PicksBurnBlood
            | E::BleedPerPick
            | E::BleedPerTargetAshTens
            | E::BleedPerEnslavingReady
            | E::CircleIntercept { .. }
            | E::NoDisciplineCardsThisAction
            | E::NoEdgeThisAction
            | E::PerfectClarity { .. }
            | E::TargetGainsActorGain
            | E::BlockToll(..)
            | E::BlockerBurnsBloodOrLife(..)
            | E::VampireBlockToll(..)
            | E::UnlockBlocker
            | E::Unblockable
            | E::NoMoreModifiers
            | E::PactCountersOnActing(..)
            | E::QueueCombatWithLast
            | E::PoolIfSuccessful(..)
            | E::DrawIfReferendumPasses
            | E::UnlockActorIfSuccessful
            | E::AttemptFailsBarSeat
            | E::GainBloodPerMargin
            | E::DistributeMargin { .. }
            | E::ReduceBleed(..)
            | E::FailBlockForCorruption
            | E::UnlockForCorruption(..)
            | E::IfBlocks(..)
            | E::IfAttemptingBlocks(..)
            | E::BlockRiderBoth(..)
            | E::TargetAttemptsBlock(..)
            | E::BurnForInterceptThisAction
            | E::LockIfNoBlock
            | E::ReduceStealthToZero
            | E::AttemptFails
            | E::VeilThins(..)
            | E::LockFailedBlockers
            | E::AttemptBlock
            | E::JointAttemptBlock
            | E::TorporAfterResolution
            | E::BlockerNotLockedForBlock
            | E::ReduceBleedByActingStealth
            | E::WakeTarget
            | E::BleedModX { .. }
            | E::LockActorHeldX(..)
            | E::NamedPaysForStealth(..)
            | E::UnlockDieRoll(..)
            | E::DamageSeatSectVampires { .. }
            | E::UnlockAndControlTargetUntilEndOfTurn
            | E::TargetGetsTitle(..)
            | E::ActorFreedFromSlavery
            | E::CoinFlipBleed(..)
            | E::BlockerControllerOpenHand
            | E::FailOrFightBond
            | E::YoungerIntercept(..)
            | E::ActionCardToHand
            | E::BarSectVotesInReferendum(..)
            | E::StealthOnNextAction(..)
            | E::LockFoe
            | E::NoGearIntercept
            | E::InterceptReactionToll(..)
            | E::TargetNoReactionsOrPowers
            | E::UnlockAtEndIfBlocks
            | E::BleedBurnsNoPool
            | E::FightActingAtResolution => (Family::Action, false),
            E::Strike(..)
            | E::Maneuver
            | E::Press
            | E::OptionalPress
            | E::OptionalManeuver
            | E::NextRoundManeuver
            | E::AdditionalStrikes { .. }
            | E::SetStrength(..)
            | E::OpposingStrengthMinus(..)
            | E::OpposingNoAdditionalStrikes
            | E::AggravatedHands
            | E::AggravatedHandsThisCombat
            | E::FortitudeProofStrike
            | E::NullifyOpposingWeapons
            | E::EnvironmentalDamage { .. }
            | E::EnvironmentalDamageWhileTarget(..)
            | E::UndodgeableStrike
            | E::StunOpponentOnHit
            | E::RetainerDamageWithStrike(..)
            | E::UndodgeableThisRound
            | E::DamageAfterCombat(..)
            | E::AggravatedStrike
            | E::BurnElectronicOnCombatants
            | E::CleaveWeapon
            | E::Expiate
            | E::FirstStrike
            | E::FirstStrikeThisRound
            | E::LoadGun(..)
            | E::BurnOpponentInsteadOfTorpor
            | E::DiablerizeOpponentInsteadOfTorpor
            | E::TorporAfterCombat { .. }
            | E::PutOnOpponentOnHit { .. }
            | E::PutOnSelfAsStrikeResolves
            | E::BurnOpposingAllyOnHit
            | E::PutOnActorAfterCombat
            | E::StrengthThisRound(..)
            | E::StrengthThisRoundVotes
            | E::CombatEndsAfterStrike { .. }
            | E::PressThisRound
            | E::LockHeldToUnlock(..)
            | E::CommittedStrike(..)
            | E::OpposingRestraint
            | E::OpposingCombatCardsCost(..)
            | E::FrenzyShield
            | E::WeatherDamage { .. }
            | E::OptionalCloseManeuver
            | E::NeverAggravatedStrike
            | E::UnpreventableIfBlackHand
            | E::ExtraDrawPerReplacementThisCombat
            | E::PutOnOpposingKeepControl
            | E::CloseManeuverForBlood(..)
            | E::ManeuverForBlood(..)
            | E::HandStrikeDamageThisCombat(..)
            | E::HandStrikeBonusPerHeld(..)
            | E::AggravatedAsNormal { .. }
            | E::NoFrenzyCancelNorEndPress
            | E::DamageEachCombatantBeforeRange(..)
            | E::DamageOpposingOwnCombatantNow(..)
            | E::HandSizeThisCombat(..)
            | E::AimExtraDamage { .. }
            | E::OpponentToTorpor
            | E::DamageOpponentNow(..)
            | E::PreRangeDamage(..)
            | E::ShieldFromOpposingStrike(..)
            | E::OpposingCelerityMayBurnToPrevent(..)
            | E::AdditionalStrikeOnly { .. }
            | E::AdditionalStrikesX { .. }
            | E::ActorBurnsBloodX
            | E::ActorBurnsBlood(..)
            | E::WeaponStrikePlus(..)
            | E::CommittedWeaponStrikePlus(..)
            | E::QueueCombatWithOpponent
            | E::OpponentOpenHandThisCombat
            | E::PlayCombatFromAsh
            | E::AdditionalStrikesEachRound(..)
            | E::ContinuePressThisRound
            | E::EndPressThisRound
            | E::OpposingBurnsBlood(..)
            | E::OpposingHandStrikerBurns { .. }
            | E::OpposingNoManeuverThisRound
            | E::OpposingImprisoned
            | E::ReflectsDamageThisCombat
            | E::OpposingNoInitialStrikeThisRound
            | E::OpposingNoStrikeCardsThisCombat
            | E::EnvironmentalDamageThisRound(..)
            | E::PreventHalfStrike
            | E::HalveOpposingStrikesThisCombat
            | E::CombatManeuver
            | E::FrenzyImmuneThisCombat
            | E::OpposingTakesMoreThisCombat(..)
            | E::OpposingStrikeTollThisCombat(..)
            | E::ResolutionDamageThisRound(..)
            | E::DiscardChosenX(..)
            | E::EnvironmentalDamageThisRoundX
            | E::DisciplineStrikeDamageThisCombat { .. }
            | E::StrikeSelfDamage { .. }
            | E::CoordinateAttack { .. }
            | E::StrikeOnceEachRound(..)
            | E::PreventNonAggravatedEachRound(..)
            | E::RoundStrike(..)
            | E::OnlyHandStrikes
            | E::NextRoundClose
            | E::NewRound
            | E::GainBloodLostByOpponent
            | E::ContinueFreeAfterCombat(..)
            | E::BurnAfterCombat(..)
            | E::ContinueAfterCombat { .. }
            | E::DamageAfterCombatIfClose(..)
            | E::PressDamageIfClose(..)
            | E::LiquefyOnHit(..)
            | E::DistillOnHit(..)
            | E::StrikeDiablerizesNextRound
            | E::DiablerizeOnHit
            | E::GrappleDamage(..)
            | E::StunAfterCombatIfClose
            | E::PutOnOpponentAfterCombatIfClose { .. }
            | E::PutOnOpponentAtRoundEndIfMore
            | E::SetRange(..)
            | E::ImmuneStrikeNonAggravated
            | E::ImmuneEffectNonAggravated
            | E::StrengthThisCombat(..)
            | E::OrgyOfBlood
            | E::DrawOutBeast { .. }
            | E::OpposingControllerBurnsPool(..)
            | E::OpposingNoDodgeThisRound
            | E::MayBurnToPreventInitialStrikes(..)
            | E::OpposingNoCombatEndsThisRound
            | E::StealTargetCardOnHit
            | E::StrikeDamageZero
            | E::OpposingGetsNoStrike
            | E::AdditionalHandStrikesEachRoundX
            | E::ResolutionDamageEachRoundIfClose(..)
            | E::OpponentEntersCombatWithTarget
            | E::PoolIfStrikerBurned(..)
            | E::UnpreventableStrikeDamage
            | E::ContinuePressForBloodThisRound(..)
            | E::UnlockForBloodThisCombat(..)
            | E::OpposingRetainersLoseTextThisCombat
            | E::OpposingNoCloseNorContinue
            | E::AggravatedAsNormalThisCombat
            | E::NamedCardNoEffect { .. }
            | E::OpposingNoStrikeRestOfRound
            | E::IfDodgedSelfDamageAndOpposingPress(..)
            | E::WeaponDamageThisCombat { .. }
            | E::BurnOpposingMeleeWeaponIfItHit
            | E::RoundPressByDamage(..)
            | E::CommitInitialStrike(..)
            | E::OpposingInitialFirstStrike
            | E::HandSizeIfNewRound(..)
            | E::StrikeAlsoDodges
            | E::NoAdditionalStrikesThisRound
            | E::AimAtTargetRetainer => (Family::Combat, false),
            E::Prevent(..)
            | E::PreventXPlus(..)
            | E::PreventNonAggravated(..)
            | E::PreventAggravated(..)
            | E::PreventThisRound(..)
            | E::PreventEachRound(..)
            | E::PreventOpposingStrikesThisRound
            | E::PreventOnTarget(..)
            | E::PreventOnTargetXPlus(..)
            | E::PreventNonAggravatedOnTarget(..)
            | E::PreventNonGun(..)
            | E::PreventStrikeDamage
            | E::PreventStrike { .. }
            | E::FleshOfMarble { .. } => (Family::Combat, true),
            E::PreventGunStrikeAllowance(..) | E::PreventMeleeWeaponStrike(..) => {
                (Family::Ability, true)
            }
            E::RetainerUsesWeapon { .. } => (Family::Ability, false),
            E::CoagulateFromPicks { prevent } => (Family::Success, prevent),
            E::ActingAbstains
            | E::FailIfTitleRequired
            | E::FailReferendum
            | E::AddVotesXLinear { .. }
            | E::NonCamarillaVotesMinus(..)
            | E::NotPathVotesMinus(..)
            | E::BarNonSectVotes(..)
            | E::PoolForVotesRound { .. }
            | E::AddVotes(..)
            | E::RevealCryptTopVoteIfOlder
            | E::ClanCoalition(..)
            | E::GrantVotes(..)
            | E::GrantVotesAll(..)
            | E::PoisonPill
            | E::AddVotesPerReadySect(..)
            | E::Abstain { .. }
            | E::CastTargetVotes(..)
            | E::AddVotesAgainst(..)
            | E::TallyPenaltyAgainst(..)
            | E::TallyBonusForOthers(..)
            | E::BloodForVotesRound { .. }
            | E::GrantForReferendum
            | E::ActorDamagedIfReferendumFails(..)
            | E::CallerBurnsPoolIfFails { .. }
            | E::TargetGainsVotes(..)
            | E::CancelChosenVotes
            | E::PutOnCallingIfPasses(..)
            | E::BarNonCamarillaVotes
            | E::BarCorruptedVotes
            | E::CorruptTarget
            | E::NextReferendumPassesIfPasses
            | E::CancelReferendumPlain
            | E::CancelReferendum { .. } => (Family::Vote, false),
            E::UnlockAtEndIfReady
            | E::BleedPoolCap(..)
            | E::BurnPoolAtEndIfNotReady(..)
            | E::BurnSource
            | E::StealTargetTitle
            | E::PayLife(..)
            | E::PayBlood(..)
            | E::PayBloodPerCounter
            | E::LockHostSkipNextUnlock(..)
            | E::ActingVampireTakesDamage(..)
            | E::TakeControlOfTargetUntilNextTurn
            | E::PreySlotToOwnSlot
            | E::PreyAshPairHunt(..)
            | E::HuntAt(..)
            | E::BloodHuntIfHuntSuccessful
            | E::RemoveActorToBurnTarget
            | E::ReorderTop { .. }
            | E::AllyFromAshLocked(..)
            | E::AshVampireToUncontrolled
            | E::CorruptTargetMayStealAtCapacity
            | E::TauntCombat(..)
            | E::GainLiaison
            | E::SearchEquip(..)
            | E::SearchEquipFree(..)
            | E::SearchEquipForTarget
            | E::SearchArchetypeOntoActor
            | E::SearchDisciplineMasterOntoActor(..)
            | E::FledglingFromNamedMaster
            | E::HeldVampireBecomesGargoyle
            | E::SearchToHand { .. }
            | E::RemoveAllSourceCounters
            | E::EnterCombatWithTarget
            | E::EnterCombatWithRandomPreyMonster
            | E::TargetYieldsTitle
            | E::AshCardToHandNamed
            | E::AshCardToHandOnSuccess(..)
            | E::AshCardOfToHand(..)
            | E::AshCardOfToLibraryTop(..)
            | E::RemoveUncontrolledForBlood(..)
            | E::MoveSourceToTarget
            | E::MoveSourceToActor
            | E::StealBloodUpTo(..)
            | E::ExchangeHandForAsh
            | E::EquipFirstFound { .. }
            | E::EquipStored(..)
            | E::TargetActsInCombat
            | E::InResultingCombat(..)
            | E::TargetMethuselahDiscards(..)
            | E::BurnCorruptionFromTarget
            | E::GraftOnTarget
            | E::IfBleedSuccessful(..)
            | E::LockMinionOfTarget
            | E::CorruptMinionOfTarget
            | E::ActingGainsConviction
            | E::PaidConvictionToTarget
            | E::DrawUpToThenDiscardDown(..) => (Family::Success, false),
            E::CannotGainLife
            | E::ActsWhenRecruited
            | E::UnlockRecruiterOnFirst
            | E::RecruiterUnlocksAtEndOfTurn
            | E::MayRepeatThisTurn
            | E::Version(..)
            | E::EntryBurnAllyOrRetainer
            | E::EntryRemoveFromAshOrBurn
            | E::EntryRemoveAshCardsOrBurn(..)
            | E::EntryStoreEquipmentUpTo(..)
            | E::EntryCountersXPlus(..) => (Family::Entry, false),
            E::HostageOnTorpor(..)
            | E::ControllerGainsPoolIfPrey(..)
            | E::UnlockBurnPoolUnlessReady { .. }
            | E::BurnPool(..)
            | E::PoolOrBloodPerCounter
            | E::PoolOrRandomCardsPerCounter
            | E::TheirControllerBurnsPool(..)
            | E::HostBloodToBy(..)
            | E::BurnSelfUnlessPool(..)
            | E::PreyChoosesMinionDamaged(..)
            | E::ChooseBlockMark
            | E::ControllerChoosesMinionDamaged(..)
            | E::HostDamagesOwnReadyVampire(..)
            | E::ActiveChoosesVampireDamaged(..)
            | E::PreyPoolToController(..)
            | E::OtherReadyImbuedDamaged(..)
            | E::OtherReadyVampireDamaged(..)
            | E::HostBurnsConviction(..)
            | E::ClearCounters
            | E::DamageHost(..)
            | E::MayMoveOffBurnedBearer
            | E::CoinFlipActionFails
            | E::GoToTorpor
            | E::StunHost
            | E::TransfersThisTurn(..)
            | E::PoolOrAshCardsPerCounter
            | E::BlockerControllerBurnsPool(..)
            | E::BleedOutcomeCounters
            | E::BurnHostCryptTopOut
            | E::Purge { .. }
            | E::MayGainPool(..)
            | E::HandSizeUntilDiscard(..)
            | E::DamageBledAnarchs(..)
            | E::BurnPoolPerHuntingGround
            | E::PreyBurnsPoolPerTorpor
            | E::BurnPoolTowardBurnSelf { .. }
            | E::EnvironmentalDamageBeforeRange(..)
            | E::MayAddCounter
            | E::MayMakeAnarchForCounter
            | E::MayJoinCombat(..)
            | E::BurnPoolPerGehennaOrSelf
            | E::ClearChosen
            | E::AnarchMayHealHost
            | E::HostBurnsBloodOrBurnSelf(..)
            | E::FramesBlocker
            | E::MayBurnForNewCombat
            | E::AddCounterForEnteringClan(..)
            | E::ActiveBurnsPoolFixedX
            | E::BurnSelfIfActiveControlsReadyCapacityOrTitled(..)
            | E::AnarchControllerMayBloodToUncontrolled(..)
            | E::MayLockForTorture
            | E::HostBurnsBloodOrBurnedThenMoves(..)
            | E::HostBurnsBloodOrIsBurned(..)
            | E::HostBurnsBloodOrThisBurns(..)
            | E::CountersFromOwnBleed(..)
            | E::PeekBledHandIfBearer
            | E::BearerBloodIfBledPrey(..)
            | E::HunterBloodToCounters { .. }
            | E::MarkHunterForHost
            | E::LockOldestOfClan(..)
            | E::TortureBurnPool(..)
            | E::ActingBurnsBloodOrLife(..)
            | E::YoungerBlockerBurnsBlood(..)
            | E::GainEdge
            | E::BloodByPoolParity { .. }
            | E::BurnHostVampire
            | E::OtherVampireBloodOrPool { .. }
            | E::GehennaPlayerBurnsPool(..)
            | E::DistributeToUncontrolled { .. }
            | E::PreyPaysOrBurnsLocations(..)
            | E::PoxDamage(..)
            | E::BurnAllPox
            | E::BurnPoolOrNoTransfers(..)
            | E::LockOrBurnPool(..)
            | E::DrawCryptOnEntry
            | E::ChooseCircleOnEntry
            | E::BindLocationOnEntry
            | E::NoEntryConviction
            | E::SearchPowerOnEntry(..)
            | E::BurnsBloodOrIsBurnedOnEntry(..)
            | E::AnarchsBloodOrCamarillaAlliesBurn
            | E::LockUnlessDiscard
            | E::BurnOfCrowdedClan
            | E::SoulGemDraw
            | E::BurnPoolUnlessBurnVampire { .. }
            | E::SkipNextUnlock
            | E::BurnSelfWithoutStored
            | E::RevealTopCancels { .. }
            | E::MayUnlockOncePerTurn
            | E::MayHandSizeOncePerTurn(..)
            | E::BuyTitleOnEntry(..)
            | E::CryptSearchOnEntry(..)
            | E::PassesToSect(..)
            | E::DamageUnbledThisTurn { .. }
            | E::BurnBodyIfExcessCreated { .. }
            | E::LockChosen
            | E::TorporMassacre
            | E::CountersToVanquisher { .. }
            | E::RemoveSelfWithoutCounters => (Family::Rider, false),
            // Read where the action is declared or offered: its kind, its
            // cost, its stealth, its bleed (`declare_action`, `stealth`,
            // `bleed_amount`).
            E::HuntAction
            | E::StealthForTitled(..)
            | E::StealthForClan { .. }
            | E::DeclareAction(..)
            | E::NoOtherEquipmentBleed
            | E::EmployRetainer { .. }
            | E::BleedIfTargetHasNoReadyUnlocked(..)
            | E::BurnSourceOnDeclare
            | E::BurnSourceOnResolution
            | E::TitledCannotBlock
            | E::StealthIfOwnTarget(..)
            | E::TakeAnyAction
            | E::MandatoryAction
            | E::CallReferendum(..)
            | E::SameActionAs(..)
            | E::BleedChangeCardsCost(..) => (Family::Marker, false),
            // Read as the card is played: what cancels it, what it cancels,
            // where it is played from (`as_played_request`, `resolve_play`,
            // `cancel_shield`, `virtual_hand`).
            E::CancelPlayed { .. }
            | E::RevealTopForCopy { .. }
            | E::CancelableByDiscardingCombat(..)
            | E::CancelableForPoolByTargetController(..)
            | E::PlayableFromStore(..)
            | E::StoredRemovedWhenBurned
            | E::BarCanceledThisTurn
            | E::BarCanceledForActorThisTurn
            | E::CancelShield
            | E::RemovedOncePlayed
            | E::CancelableForBlockerBlood(..)
            | E::SearchStore(..)
            | E::CanceledGetsMasterAction
            | E::CancelableForPool(..)
            | E::CancelableForPoolByAny(..)
            | E::MovesToOldestUntitled { .. }
            | E::LockCanceledActor
            | E::BarCanceledForActorThisAction => (Family::Marker, false),
            // What the card is (`card_title`, `becomes_weapon`, `is_gun`).
            E::RepresentsTitle(..)
            | E::BecomesWeapon
            | E::BecomesGun
            | E::NotAgainstFlight
            | E::EquipFromAnyAsh
            | E::TorporIfReferendumFails => (Family::Marker, false),
            // Read by the referendum Charming Lobby calls (`start_referendum`,
            // the tally).
            E::NextReferendumPassesForAll | E::ActingVotesInReferendum(..) => {
                (Family::Marker, false)
            }
            // Read by `give_card_control`, `about_to_leave_ready`,
            // `start_influence`, `influence_apply`, the diablerie (Regent),
            // `card_controller` (Melange [AUS]), `settle_servitors`, and
            // `burn_minion` (a granted burn's success).
            E::PoolWhenHostLeaves(..)
            | E::BurnAtInfluencePerPreyReady
            | E::HostOutBloodToOlderUncontrolled
            | E::RechoosesOnControlChange
            | E::SpendTransfers(..)
            | E::GainTransfers(..)
            | E::ShuffleSourceIntoLibrary
            | E::CryptVampireToReady { .. }
            | E::MovesToSabbatDiablerist
            | E::NotTakenByDiablerie
            | E::RemovedAfterSuccessOrWithoutInfernal
            | E::StillYours
            | E::CountersClearOnNewBearer
            | E::BearerAimStore
            | E::BearerStore
            | E::UndirectedAction
            | E::DirectedAtSourceController
            | E::EquipActionUnlocksEndOfTurn
            | E::NamedCardOff { .. }
            | E::Trophy
            | E::TrophyBlood(..)
            | E::GainVictimDiscipline
            | E::BloodHuntVotesForDiablerist(..)
            | E::BloodHuntVotesForAnarchs(..)
            | E::BloodHuntFavorOnlyClans(..) => (Family::Marker, false),
            // Read as the combat begins (Abuse of Power, Pillars Fall), as the
            // action is declared (Fiendish Tongue), as the bleed lands
            // (Enthrall), by `frenzy_on`, and as the action succeeds.
            E::CryptForBloodIfBleedSuccessful(..)
            | E::SectInterceptThisAction(..)
            | E::DiscardUnlockIfBleedSuccessful { .. }
            | E::OpposingBurnsPoolIfOneReadyAfterCombat(..)
            | E::PutOnActorIfOpposingNotReadyAfterCombat
            | E::TargetLocksReadyUnlocked
            | E::PinnedWhileBearerReady
            | E::PinnedAlways
            | E::BurnedSendsBearerToTorpor
            | E::BurnedDamagesBearer(..)
            | E::ControllerGivesCard
            | E::ClanBlocksItsBurn(..)
            | E::ImmuneNonAggravated
            | E::ConvictionCost(..)
            | E::Frenzy
            | E::DeclaredActionLayer(..)
            | E::NoAmmo => (Family::Marker, false),
            // Read when the strike resolves (`resolve_strikes`), with
            // `PutOnActorNow` (Inner Essence).
            E::AggravatedAgainstClan(..)
            | E::ImbuedStrikesPlusX
            | E::SparedByCleave
            | E::BurnAtCombatEnd
            | E::BurnAtActionEnd
            | E::BearerDamageOncePerCombat(..)
            | E::TransfersToTorporedOpponent
            | E::TorporAtWeaponDamage(..)
            | E::BurnAfterStrike
            | E::StunsAfterCombat
            | E::CountersPerDamage
            | E::AdditionalStrikeWithSource
            | E::UnpreventableStrike
            | E::BecomesOnOpponent
            | E::CostsLessBlood(..)
            | E::OutOfPlayUntilActionEnd
            | E::BurnIfDamagePrevented
            | E::MaySecondCopyFromHandOrAsh => (Family::Marker, false),
            // Done as the ability's terms were chosen (`ability_ash_apply`,
            // `ability_discards_done`).
            E::AshCardToLibraryBottom | E::StorePickedFromHand | E::SwapPickedWithStored => {
                (Family::Marker, false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::defs::DEFS;
    use super::super::vampires::VAMPIRES;
    use super::super::ModeDef;
    use super::*;
    use crate::decision::ActionKind;

    /// A position of a definition holding effects, and the interpreter
    /// running them there.
    #[derive(Clone, Copy, Debug)]
    enum At {
        /// A card's mode: `apply_card_effect`, behind an ally's entry for
        /// the effects it resolves first.
        Mode,
        /// An action card's mode: behind the action's success too.
        ActionMode,
        /// An in-play ability: `resolve_ability`.
        Ability,
        /// An ability declaring an action of this kind: its effects resolve
        /// as the action succeeds (`succeed`), read at its declaration.
        Granted(ActionKind),
        /// A rider: the loop firing its hook.
        Rider,
        /// `If`'s bonus, read live for the rest of the action.
        If,
        /// `IfBleedSuccessful`'s body (`bleed_successful_effects`).
        BleedSuccessful,
    }

    fn allowed(at: At, f: Family) -> bool {
        use Family::*;
        match at {
            At::Mode => matches!(f, Play | Shared | Action | Combat | Vote | Entry | Marker),
            At::ActionMode => allowed(At::Mode, f) || f == Success,
            At::Ability => matches!(f, Shared | Ability | Action | Combat | Vote | Marker),
            At::Granted(_) => unreachable!("read by `granted_resolves`"),
            At::Rider => !matches!(f, Play | Combat | Vote | Success | Entry),
            At::If => f == Action,
            At::BleedSuccessful => matches!(f, Shared | Success),
        }
    }

    /// What resolves an effect of a granted action of `kind`: for
    /// `ActionKind::Ability`, the resolver's own dispatch (`Effect::granted`);
    /// for another kind, the effects its resolver, its stealth or its
    /// declaration read by name (`succeed`'s arm of the kind,
    /// `ActionCtx::stealth`, `bleed_amount`, `granted_cost`, `use_ability`).
    fn granted_resolves(kind: ActionKind, e: Effect) -> bool {
        use ActionKind as K;
        use Effect as E;
        if kind == K::Ability {
            return e.granted().is_some();
        }
        let declared = matches!(
            e,
            E::DeclareAction(_)
                | E::AddStealth(_)
                | E::MandatoryAction
                | E::StealthForClan { .. }
                | E::StealthForTitled(_)
        );
        declared
            || match kind {
                K::Bleed => matches!(
                    e,
                    E::BleedMod { .. }
                        | E::BleedIfTargetHasNoReadyUnlocked(_)
                        | E::NoOtherEquipmentBleed
                        | E::BurnSourceOnResolution
                        | E::PayBlood(_)
                        | E::PayBloodPerCounter
                ),
                K::BurnCard => matches!(
                    e,
                    E::BurnSource
                        | E::BurnSelfAtCounters(_)
                        | E::AddCounter(_)
                        | E::ShuffleSourceIntoLibrary
                        | E::ActingVampireTakesDamage(_)
                        | E::LockHostSkipNextUnlock(_)
                        | E::PayBlood(_)
                        | E::PayBloodPerCounter
                        | E::PayPool(_)
                        | E::BurnEdge
                ),
                K::EquipFound => {
                    matches!(
                        e,
                        E::EquipFirstFound { .. } | E::EquipStored(..) | E::EquipFromAnyAsh
                    )
                }
                K::Political => matches!(
                    e,
                    E::CallReferendum(_) | E::TorporIfReferendumFails | E::PayPool(_)
                ),
                _ => false,
            }
    }

    fn declared(m: &ModeDef) -> Option<ActionKind> {
        m.effects.iter().find_map(|e| match e {
            Effect::DeclareAction(k) => Some(*k),
            _ => None,
        })
    }

    #[derive(Default)]
    struct Walk {
        failures: Vec<String>,
        checked: usize,
    }

    impl Walk {
        fn effects(&mut self, name: &str, at: At, effects: &[Effect]) {
            for &e in effects {
                self.checked += 1;
                let sits = match at {
                    At::Granted(kind) => granted_resolves(kind, e),
                    at => allowed(at, e.family()),
                };
                if !sits {
                    self.failures
                        .push(format!("{name}: {e:?} ({:?}) in {at:?}", e.family()));
                }
                match e {
                    // The bonuses `apply_card_effect`'s `If` arm reads.
                    Effect::If { then, .. } => {
                        if !matches!(then, Effect::BleedMod { .. } | Effect::AddIntercept(_)) {
                            self.failures
                                .push(format!("{name}: {then:?} not a live conditional bonus"));
                        }
                        self.effects(name, At::If, std::slice::from_ref(then));
                    }
                    Effect::IfBleedSuccessful(body) => {
                        self.effects(name, At::BleedSuccessful, body)
                    }
                    _ => {}
                }
            }
        }

        fn mode(&mut self, name: &str, m: &ModeDef) {
            let at = if declared(m).is_some() {
                At::ActionMode
            } else {
                At::Mode
            };
            self.effects(name, at, m.effects);
        }

        fn ability(&mut self, name: &str, a: &ModeDef) {
            let at = declared(a).map_or(At::Ability, At::Granted);
            self.effects(name, at, a.effects);
        }
    }

    // A definition's mode reads its card's text line (`library_mode`): one
    // past its lines indexes out of bounds as the card is offered.
    #[test]
    fn every_mode_maps_to_a_line_of_its_card() {
        for d in DEFS.iter() {
            let card = crate::library::LIBRARY
                .iter()
                .find(|c| c.name == d.name)
                .expect("a library card");
            let lines = card.modes.len();
            if d.lines.is_empty() {
                assert!(
                    d.modes.len() <= lines,
                    "{}: {} modes, {lines} lines",
                    d.name,
                    d.modes.len()
                );
            } else {
                assert_eq!(d.lines.len(), d.modes.len(), "{}", d.name);
                assert!(d.lines.iter().all(|&l| (l as usize) < lines), "{}", d.name);
            }
        }
    }

    // An effect where its interpreter does not resolve it would reach an
    // `unreachable!` in a game.
    #[test]
    fn every_effect_sits_where_its_family_resolves() {
        let mut w = Walk::default();
        for d in DEFS.iter() {
            for m in d.modes {
                w.mode(d.name, m);
            }
            for a in d.abilities {
                w.ability(d.name, a);
            }
            for r in d.riders {
                w.effects(d.name, At::Rider, r.effects);
            }
        }
        for v in VAMPIRES.iter() {
            for a in v.abilities {
                w.ability(v.name, a);
            }
            for r in v.riders {
                w.effects(v.name, At::Rider, r.effects);
            }
        }
        assert!(w.checked > 2500, "{} effects walked", w.checked);
        assert!(w.failures.is_empty(), "{}", w.failures.join("\n"));
    }

    // `resolve_ability` resolves an action effect with an action underway
    // only, and fails loudly outside one: every ability carrying one is
    // offered in an action's windows only.
    #[test]
    fn abilities_changing_the_action_are_used_in_one() {
        use super::super::Window as W;
        let in_action = |w: &W| {
            matches!(
                w,
                W::ActionAsAnnounced
                    | W::ReactionAsAnnounced
                    | W::ActionDuring
                    | W::ActionStealthIntercept
                    | W::ActionBlockAttempt
                    | W::ActionBlocked
                    | W::ActionBlocksDeclined
                    | W::ActionWouldBeSuccessful
                    | W::ReactionDuringAction
                    | W::ReactionUnlockAttempt
                    | W::ActionAfterCombat
            )
        };
        let abilities = DEFS
            .iter()
            .flat_map(|d| d.abilities.iter().map(move |a| (d.name, a)))
            .chain(
                VAMPIRES
                    .iter()
                    .flat_map(|v| v.abilities.iter().map(move |a| (v.name, a))),
            );
        let mut failures = Vec::new();
        let mut checked = 0;
        for (name, a) in abilities.filter(|(_, a)| declared(a).is_none()) {
            for e in a.effects.iter().filter(|e| e.family() == Family::Action) {
                checked += 1;
                // A wake outside an action has its own arm (Gracetius).
                let wake_out = |w: &W| *e == Effect::Wake && matches!(w, W::Wake | W::CardAsPlayed);
                if !a.windows.iter().all(|w| in_action(w) || wake_out(w)) {
                    failures.push(format!("{name}: {e:?} in {:?}", a.windows));
                }
            }
        }
        assert!(checked > 50, "{checked} action effects walked");
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
