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

impl Effect {
    /// The interpreter resolving this effect.
    pub fn family(self) -> Family {
        use Effect as E;
        match self {
            E::ChooseSecondOfClan(..)
            | E::ChooseOwnOfClan(..)
            | E::ChooseTarget
            | E::RevealCryptTop { .. }
            | E::BecomesAlly(..)
            | E::AshurTablets
            | E::GiveBoonPool
            | E::RotschreckEndsCombat
            | E::BecomesVampire { .. }
            | E::PutOnTarget
            | E::PutInPlay
            | E::EnterWithCounters(..)
            | E::UnlockActorIfClanCapacity { .. }
            | E::MoveBloodTargetToPoolX
            | E::Layer(..)
            | E::StealTargetAlly
            | E::TrainInUncontrolled { .. }
            | E::PoolToVampires
            | E::BurnChosenInPlay
            | E::EquipNamed
            | E::CountersFromTargetCapacity
            | E::RetainerToStartingLife
            | E::PutOnActorNowStacking
            | E::PutOnOpponent
            | E::BurnLibraryTop(..)
            | E::PutOnActorNow
            | E::MorbidityTake(..)
            | E::TrapPresses
            | E::BeastPresses { .. }
            | E::EatAshAllyToTarget(..)
            | E::Stun
            | E::StealLocation
            | E::StealTargetVampire
            | E::StolenToReadyForBlood(..)
            | E::EatAshAlly { .. }
            | E::BloodToTargetFull
            | E::RemoveTargetForCapacityPool
            | E::MoveBloodXToUncontrolledTarget
            | E::EnterWithCountersPerOwnClan(..)
            | E::EnterWithCountersPerOwnAnarch
            | E::CryptClanToUncontrolled(..)
            | E::RemoveBleedTargetAshCards(..)
            | E::SearchLibraryToHand(..)
            | E::SudarioRefraction
            | E::OwnReadyVampiresBloodToPool
            | E::GainPerUnlockedCircleMember
            | E::BloodToUncontrolledCircle
            | E::PetraResonance
            | E::TakeBleedForTarget
            | E::CryptTopToUncontrolled
            | E::BurnRangedWeaponsOnTarget { .. }
            | E::EmployNamed
            | E::FourthCycle(..)
            | E::AutoPassFirstSabbatReferendum
            | E::BloodToEachUncontrolledOfSect(..)
            | E::StealTargetCard
            | E::FuryEnters
            | E::CountersFromActorCapacity
            | E::GreatBeastAtEleven
            | E::KaymakliNightmares
            | E::TargetBecomesAnarch
            | E::ActingBloodToNewUpTo(..)
            | E::RedeemAshVampire
            | E::BurnOthersLabeledOnTarget(..)
            | E::LaibonOrCryptDraw(..)
            | E::DiseaseTarget
            | E::StealBloodFromTarget(..)
            | E::NextReferendumPasses
            | E::MergeWithBase
            | E::DiscardFromTargetHand
            | E::StealPoolFromTarget(..)
            | E::TargetControllerBurnsPool(..)
            | E::RemovePreyAshToBurnPool { .. }
            | E::LockPlayedCard
            | E::ActorTakesAggravated { .. }
            | E::PutOnActor
            | E::PutOnActorRemembering
            | E::BurnActorBlood(..)
            | E::If { .. } => Family::Play,
            E::TargetLeavesTorpor
            | E::BloodToActing(..)
            | E::PreyBurnsPool(..)
            | E::GainPool(..)
            | E::BloodToUncontrolledTarget(..)
            | E::BloodToTarget(..)
            | E::LockActor
            | E::EndCombat
            | E::BloodToTargetToStart(..)
            | E::TargetBurnsBlood(..)
            | E::UnlockActor
            | E::UnlockTarget
            | E::LockTarget
            | E::BurnTargetMinion
            | E::CircleBloodMoves { .. }
            | E::BurnTargetCard
            | E::StoreTopCards(..) => Family::Shared,
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
            | E::SpendMasterPhaseAction
            | E::LockSelf
            | E::BurnEdge
            | E::SpendDiscardPhaseAction
            | E::AddHostBlood(..)
            | E::AddCounter(..)
            | E::BurnSelfAtCounters(..)
            | E::RemoveCounter(..)
            | E::BurnSelfWithoutCounters
            | E::HandSizeBonusUntilEndOfTurn(..)
            | E::MoveBetweenVampires
            | E::PeekHand
            | E::PredatorTakesControl
            | E::BurnSelf
            | E::LockHost
            | E::BurnOwnLife(..)
            | E::StealBloodToHostX
            | E::HostBloodToTarget(..)
            | E::TargetBloodToHost(..)
            | E::EnterCombatWithActing
            | E::TargetEntersCombatWithActing
            | E::ReduceStealth(..)
            | E::UnlockSelf
            | E::ReduceActionCost(..)
            | E::HostBurnBlood(..)
            | E::HostBurnBloodToUnlock
            | E::HostBloodToPool(..)
            | E::PoolToHostBlood(..)
            | E::HuntingGroundBlood(..)
            | E::HuntingGroundTwoOldest
            | E::AddInterceptX
            | E::TargetBurnsForInterceptThisAction
            | E::RevealTopCard { .. }
            | E::ActingControllerDiscardsAtRandom
            | E::TargetSeatDiscardsAtRandom(..)
            | E::TargetToUncontrolled
            | E::TakeBleedGain
            | E::SlaveTakesCombat
            | E::HostLeavesTorpor
            | E::ReduceBleedX
            | E::SearchMageAlly
            | E::YawpCourtCombat
            | E::HuntingGroundLife(..)
            | E::HostBurnLife(..)
            | E::TargetBurnLife(..)
            | E::RaiseAllyWhenEmpty(..)
            | E::AtCountersBurnAndGain { .. }
            | E::GainMasterPhaseAction
            | E::PayPoolX
            | E::MethuselahVotesX { .. }
            | E::ContinuePressForOwnCombatant
            | E::DrawThenDiscardDown(..)
            | E::RemoveCountersX
            | E::AddVotesX
            | E::TargetBloodToPool(..)
            | E::PoolToTargetBlood(..)
            | E::CounterToTargetBlood
            | E::CryptNamedToUncontrolled(..)
            | E::UnlockActing
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
            | E::TargetInterceptThisAction(..)
            | E::BleedToRemembered
            | E::HostVotes(..)
            | E::HostAsIfUnlockedForAuspex
            | E::CountersToPool
            | E::CounterToPool
            | E::CommitStrike
            | E::GhoulFromLibraryTop(..)
            | E::GiveBloodToOthers
            | E::GiveBloodToOwnAnarch
            | E::BurnUnlessControllerPays(..)
            | E::DamageTarget(..)
            | E::PayPoolPerOtherEvent
            | E::PreventGunStrikeAllowance(..)
            | E::DiscardAtRandom(..)
            | E::ContinuePressEachCombat
            | E::TargetVotesPerCounter
            | E::ToggleLockTarget
            | E::HuntStealthUntilUnlock(..)
            | E::DrawThenBottom
            | E::MethuselahVotes { .. }
            | E::TakeControlOfHost
            | E::RemoveOwnAshCards(..)
            | E::UnlockHost => Family::Ability,
            E::TakeOverAction
            | E::ActionLayer(..)
            | E::AddStealth(..)
            | E::AddIntercept(..)
            | E::AttemptingIntercept(..)
            | E::BleedMod { .. }
            | E::Wake
            | E::VampiresCannotBlock
            | E::DamageAfterResolution(..)
            | E::TargetCannotBlock
            | E::NoUnlockThisAction
            | E::TargetNoReactions
            | E::NextReactionCosts(..)
            | E::ReduceBleedToZero
            | E::EndBlockedAction
            | E::StealthForNextActions
            | E::RedirectBleed
            | E::BurnActingMinion
            | E::ActionFails
            | E::DeferReactionReplacement
            | E::EndActionIfBlocked
            | E::ContinueUnblocked
            | E::AlliesCannotBlock
            | E::StrengthThisAction(..)
            | E::AggravatedHandsThisAction
            | E::DawnOperation { .. }
            | E::PutOnActorIfNoBlock
            | E::PutOnActingIfBlocks
            | E::BurnIfNoBlock(..)
            | E::BlockTollBlood(..)
            | E::LockAttempting
            | E::QueueCombatWithAttempting
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
            | E::CircleIntercept { .. }
            | E::NoDisciplineCardsThisAction
            | E::PerfectClarity { .. }
            | E::TargetGainsActorGain
            | E::BlockToll(..)
            | E::BlockerBurnsBloodOrLife(..)
            | E::VampireBlockToll(..)
            | E::UnlockBlocker
            | E::Unblockable
            | E::NoMoreModifiers
            | E::PoolIfSuccessful(..)
            | E::AttemptFailsBarSeat
            | E::GainBloodPerMargin
            | E::DistributeMargin { .. }
            | E::ReduceBleed(..)
            | E::FailBlockForCorruption
            | E::UnlockForCorruption(..)
            | E::IfBlocks(..)
            | E::TargetAttemptsBlock(..)
            | E::BurnForInterceptThisAction
            | E::LockIfNoBlock
            | E::ReduceStealthToZero
            | E::AttemptFails
            | E::VeilThins(..)
            | E::LockFailedBlockers
            | E::AttemptBlock
            | E::AllySkipsNextUnlock
            | E::WakeTarget
            | E::BleedModX { .. }
            | E::YoungerIntercept(..) => Family::Action,
            E::Strike(..)
            | E::Maneuver
            | E::Press
            | E::OptionalPress
            | E::OptionalManeuver
            | E::NextRoundManeuver
            | E::AdditionalStrikes { .. }
            | E::SetStrength(..)
            | E::AggravatedHands
            | E::AggravatedHandsThisCombat
            | E::FortitudeProofStrike
            | E::NullifyOpposingWeapons
            | E::EnvironmentalDamage(..)
            | E::UndodgeableStrike
            | E::AggravatedStrike
            | E::FirstStrike
            | E::PutOnOpponentOnHit { .. }
            | E::PutOnActorAfterCombat
            | E::StrengthThisRound(..)
            | E::PreventThisRound(..)
            | E::PreventEachRound(..)
            | E::PressThisRound
            | E::PreventOpposingStrikesThisRound
            | E::CommittedStrike(..)
            | E::OpposingRestraint
            | E::OpposingCombatCardsCost(..)
            | E::FrenzyShield
            | E::FleshOfMarble { .. }
            | E::WeatherDamage { .. }
            | E::PreventOnTarget(..)
            | E::PreventOnTargetXPlus(..)
            | E::PreventNonAggravatedOnTarget(..)
            | E::OptionalCloseManeuver
            | E::NeverAggravatedStrike
            | E::UnpreventableIfBlackHand
            | E::ExtraDrawPerReplacementThisCombat
            | E::PutOnOpposingKeepControl
            | E::CloseManeuverForBlood(..)
            | E::ManeuverForBlood(..)
            | E::HandStrikeDamageThisCombat(..)
            | E::HandSizeThisCombat(..)
            | E::AimExtraDamage(..)
            | E::OpponentToTorpor
            | E::AdditionalStrikeOnly { .. }
            | E::AdditionalStrikesX { .. }
            | E::ActorBurnsBloodX
            | E::WeaponStrikePlus(..)
            | E::CommittedWeaponStrikePlus(..)
            | E::QueueCombatWithOpponent
            | E::OpponentOpenHandThisCombat
            | E::PlayCombatFromAsh
            | E::AdditionalStrikesEachRound(..)
            | E::ContinuePressThisRound
            | E::CloseSelfDamage(..)
            | E::CoordinateAttack { .. }
            | E::RoundStrike(..)
            | E::Prevent(..)
            | E::PreventXPlus(..)
            | E::PreventNonAggravated(..)
            | E::OnlyHandStrikes
            | E::NextRoundClose
            | E::NewRound
            | E::GainBloodLostByOpponent
            | E::ContinueFreeAfterCombat(..)
            | E::BurnAfterCombat(..)
            | E::ContinueAfterCombat { .. }
            | E::DamageAfterCombatIfClose(..)
            | E::StunAfterCombatIfClose
            | E::SetRange(..)
            | E::ImmuneStrikeNonAggravated
            | E::ImmuneEffectNonAggravated
            | E::PreventNonGun(..)
            | E::StrengthThisCombat(..)
            | E::OrgyOfBlood
            | E::DrawOutBeast { .. }
            | E::OpposingControllerBurnsPool(..)
            | E::PreventStrikeDamage
            | E::PreventStrike(..) => Family::Combat,
            E::ActingAbstains
            | E::FailIfTitleRequired
            | E::FailReferendum
            | E::AddVotesXLinear { .. }
            | E::NonCamarillaVotesMinus(..)
            | E::NotPathVotesMinus(..)
            | E::BarNonSectVotes(..)
            | E::PoolForVotesRound { .. }
            | E::AddVotes(..)
            | E::GrantVotes(..)
            | E::GrantVotesAll(..)
            | E::PoisonPill
            | E::AddVotesPerReadySect(..)
            | E::Abstain { .. }
            | E::AddVotesAgainst(..)
            | E::TallyPenaltyAgainst(..)
            | E::BarNonCamarillaVotes
            | E::CancelReferendum { .. } => Family::Vote,
            E::UnlockAtEndIfReady
            | E::BurnSource
            | E::StealTargetTitle
            | E::PayLife(..)
            | E::PayBlood(..)
            | E::LockHostSkipNextUnlock(..)
            | E::TakeControlOfTargetUntilNextTurn
            | E::PreySlotToOwnSlot
            | E::PreyAshPairHunt(..)
            | E::HuntAt(..)
            | E::BloodHuntIfHuntGained
            | E::RemoveActorToBurnTarget
            | E::ReorderTopThenUnlock(..)
            | E::AllyFromAshLocked(..)
            | E::CorruptTargetMayStealAtCapacity
            | E::CoagulateFromPicks { .. }
            | E::TauntCombat(..)
            | E::GainLiaison
            | E::SearchEquip
            | E::SearchToHand(..)
            | E::RemoveAllSourceCounters
            | E::EnterCombatWithTarget
            | E::TargetYieldsTitle
            | E::AshCardToHandNamed
            | E::AshCardOfToHand(..)
            | E::RemoveUncontrolledForBlood(..)
            | E::MoveSourceToTarget
            | E::StealBloodUpTo(..)
            | E::ExchangeHandForAsh
            | E::EquipFirstFound
            | E::TargetActsInCombat
            | E::InResultingCombat(..)
            | E::TargetMethuselahDiscards(..)
            | E::BurnCorruptionFromTarget
            | E::IfBleedSuccessful(..)
            | E::LockMinionOfTarget
            | E::CorruptMinionOfTarget => Family::Success,
            E::CannotGainLife
            | E::ActsWhenRecruited
            | E::UnlockRecruiterOnFirst
            | E::Version(..)
            | E::EntryBurnAllyOrRetainer
            | E::UnlockActorAtDiscard
            | E::EntryRemoveFromAshOrBurn => Family::Entry,
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
            | E::CoinFlipActionFails
            | E::GoToTorpor
            | E::PoolOrAshCardsPerCounter
            | E::BlockerControllerBurnsPool(..)
            | E::BleedOutcomeCounters
            | E::BurnHostCryptTopOut
            | E::Purge { .. }
            | E::MayGainPool(..)
            | E::HandSizeUntilDiscard(..)
            | E::DamageBledAnarchs(..)
            | E::PreyBurnsPoolPerTorpor
            | E::CounterOnZombieBurned
            | E::MayMakeAnarchForCounter
            | E::AnarchControllerMayBloodToUncontrolled(..)
            | E::MayLockForTorture
            | E::HostBurnsBloodOrBurnedThenMoves(..)
            | E::HostBurnsBloodOrIsBurned(..)
            | E::HostBurnsBloodOrThisBurns(..)
            | E::CountersFromOwnBleed(..)
            | E::LockOldestOfClan(..)
            | E::TortureBurnPool(..)
            | E::ActingBurnsBloodOrLife(..)
            | E::DistributeToUncontrolled { .. }
            | E::PreyPaysOrBurnsLocations(..)
            | E::BurnPoolOrNoTransfers(..)
            | E::LockOrBurnPool(..)
            | E::DrawCryptOnEntry
            | E::ChooseCircleOnEntry
            | E::BurnsBloodOrIsBurnedOnEntry(..)
            | E::AnarchsBloodOrCamarillaAlliesBurn
            | E::LockUnlessDiscard
            | E::BurnOfCrowdedClan
            | E::SoulGemDraw
            | E::BurnPoolUnlessBurnVampire { .. }
            | E::HandSizeUntilUnlock(..)
            | E::SkipNextUnlock
            | E::BurnSelfWithoutStored => Family::Rider,
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
            | E::TitledCannotBlock
            | E::StealthIfOwnTargetCard(..)
            | E::TakeAnyAction
            | E::MandatoryAction
            | E::CallReferendum(..)
            | E::SameActionAs(..) => Family::Marker,
            // Read as the card is played: what cancels it, what it cancels,
            // where it is played from (`as_played_request`, `resolve_play`,
            // `cancel_shield`, `virtual_hand`).
            E::CancelPlayed { .. }
            | E::CancelableByDiscardingCombat(..)
            | E::CancelableForPoolByTargetController(..)
            | E::PlayableFromStore(..)
            | E::StoredRemovedWhenBurned
            | E::BarCanceledThisTurn
            | E::CancelShield
            | E::RemovedOncePlayed
            | E::CancelableForBlockerBlood(..)
            | E::SearchStore(..)
            | E::CanceledGetsMasterAction
            | E::CancelableForPool(..) => Family::Marker,
            // What the card is (`card_title`, `becomes_weapon`, `is_gun`).
            E::RepresentsTitle(..) | E::BecomesWeapon | E::BecomesGun => Family::Marker,
            // Read on a change of control (`give_card_control`), as a host is
            // about to leave the ready region (`about_to_leave_ready`), as the
            // influence phase begins (`start_influence`) and as transfers are
            // spent (`influence_apply`), by the diablerie (Regent), by the
            // card's controller (`card_controller`: Melange [AUS]), by the
            // servitors (`settle_servitors`), as the card would burn
            // (`burn_minion`, a granted burn's success).
            E::PoolWhenHostLeaves(..)
            | E::BurnAtInfluencePerPreyReady
            | E::RechoosesOnControlChange
            | E::SpendTransfers(..)
            | E::GainTransfers(..)
            | E::ShuffleSourceIntoLibrary
            | E::MovesToSabbatDiablerist
            | E::RemovedAfterSuccessOrWithoutInfernal
            | E::StillYours => Family::Marker,
            // Read as the combat begins (Abuse of Power, Pillars Fall), as the
            // action is declared (Fiendish Tongue's intercept) and as the bleed
            // lands (its unlock, Enthrall's draw); by the frenzy-cancel cards
            // (`frenzy_on`); as the action succeeds.
            E::CryptForBloodIfBleedSuccessful(..)
            | E::SectInterceptThisAction(..)
            | E::DiscardUnlockIfBleedSuccessful { .. }
            | E::OpposingBurnsPoolIfOneReadyAfterCombat(..)
            | E::PutOnActorIfOpposingNotReadyAfterCombat
            | E::TargetLocksReadyUnlocked
            | E::Frenzy => Family::Marker,
            // Read when the strike resolves (`resolve_strikes`), with
            // `PutOnActorNow` (Inner Essence).
            E::AggravatedAgainstClan(..)
            | E::BurnAtCombatEnd
            | E::BearerDamageOncePerCombat
            | E::CountersPerDamage
            | E::AdditionalStrikeWithSource
            | E::UnpreventableStrike
            | E::BecomesOnOpponent => Family::Marker,
            // Done as the ability's terms were chosen (`ability_ash_apply`,
            // `ability_discards_done`).
            E::AshCardToLibraryBottom | E::StorePickedFromHand => Family::Marker,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::defs::DEFS;
    use super::super::vampires::VAMPIRES;
    use super::super::ModeDef;
    use super::*;

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
        /// An ability declaring an action: its effects resolve as the
        /// action succeeds (`succeed`), which reads the card's too.
        Granted,
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
            At::Granted => !matches!(f, Entry | Rider),
            At::Rider => !matches!(f, Play | Combat | Vote | Success | Entry),
            At::If => f == Action,
            At::BleedSuccessful => matches!(f, Shared | Success),
        }
    }

    fn declares(m: &ModeDef) -> bool {
        m.effects
            .iter()
            .any(|e| matches!(e, Effect::DeclareAction(_)))
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
                if !allowed(at, e.family()) {
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
            let at = if declares(m) {
                At::ActionMode
            } else {
                At::Mode
            };
            self.effects(name, at, m.effects);
        }

        fn ability(&mut self, name: &str, a: &ModeDef) {
            let at = if declares(a) {
                At::Granted
            } else {
                At::Ability
            };
            self.effects(name, at, a.effects);
        }
    }

    // F1: an effect in a position whose interpreter does not resolve it
    // reaches an `unreachable!` in a game; here it fails a test.
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
        for (name, a) in abilities.filter(|(_, a)| !declares(a)) {
            for e in a.effects.iter().filter(|e| e.family() == Family::Action) {
                checked += 1;
                if !a.windows.iter().all(in_action) {
                    failures.push(format!("{name}: {e:?} in {:?}", a.windows));
                }
            }
        }
        assert!(checked > 50, "{checked} action effects walked");
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
