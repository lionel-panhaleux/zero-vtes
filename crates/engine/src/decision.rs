use crate::cards::Target;
use crate::game::Contest;
use crate::ids::{MinionId, SeatId};

/// Action kinds: the card-free rulebook actions, plus card actions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ActionKind {
    #[default]
    Bleed,
    Hunt,
    LeaveTorpor,
    /// Rescue a vampire from torpor (`target`); 2 blood, split at resolution.
    Rescue,
    /// Diablerise a vampire in torpor (`target`); triggers the blood hunt.
    Diablerie,
    /// Employ a retainer from hand (rulebook "Employ Retainer": +1 stealth,
    /// undirected; the cost paid at resolution).
    Employ,
    /// Recruit an ally from hand (rulebook "Recruit Ally": +1 stealth,
    /// undirected, cost at resolution; the ally does not act the turn it is
    /// recruited).
    Recruit,
    /// Call a referendum with a political action card (rulebook "Political
    /// Action": +1 stealth, undirected; one per minion per turn).
    Political,
    /// An action card whose effects resolve on success (put in play, …);
    /// its stealth is the card's own ("+1 stealth action").
    Card,
    /// Burn a card in play, granted by that card ("Minions can burn this
    /// card as a Ⓓ action"): directed at its controller.
    BurnCard,
    /// Steal a card in play, granted by that card ("Vampires can steal this
    /// location as a Ⓓ action"): directed at its controller.
    StealCard,
    /// Equip with an equipment card from hand (rulebook "Equip": +1
    /// stealth, undirected; the cost paid at resolution).
    Equip,
    /// Equip with equipment taken from another minion the Methuselah
    /// controls (rulebook "Equip": free, announced with the action; not a
    /// card play, no requirement checked).
    EquipFrom,
    /// Equip with the first equipment found in the library from the top
    /// (Vast Wealth): granted by a card in play, undirected, its stealth
    /// the card's; the search only on success (TOM 19951107).
    EquipFound,
    /// An action granted by an ability ("Sakhar can search your library …
    /// as a +1 stealth action"), resolving on success; directed when its
    /// target is another Methuselah's.
    Ability,
    /// Rulebook "Become Anarch" (advanced rules): a ready untitled
    /// non-Anarch vampire, +1 stealth, undirected; 2 blood, 1 with another
    /// ready Anarch, paid at resolution.
    BecomeAnarch,
    /// Rulebook "Red List": enter combat with a Red List minion marked this
    /// turn (`target`), +1 stealth, directed, 1 blood at resolution; once
    /// each turn per vampire.
    RedListHunt,
    /// An imbued gets a power card from hand (rulebook "Appendix: Imbued
    /// Rules" 3): +1 stealth, undirected; it unlocks if successful.
    Power,
    /// Burn an incapacitated imbued (`target`) and take its equipment
    /// (rulebook "Appendix: Imbued Rules" 5): directed, as a diablerie.
    BurnImbued,
}

/// One indexed option inside a decision request. `Pass` is always explicit;
/// its meaning is window-specific (decline to block, end phase, keep the
/// Edge, forfeit remaining transfers, cast no more votes this rotation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Pass,
    /// Unlock phase: "if you have the Edge, you may gain 1 pool" (the Edge
    /// is kept; the name is historical).
    BurnEdge,
    /// Unlock-phase contest upkeep: pay 1 pool (card) or 1 blood (title),
    /// mandatory, ordered among the active Methuselah's unlock effects
    /// (rules-qa 7.8).
    PayContest(Contest),
    /// Unlock-phase contest: yield — the card is burned / the title is lost.
    YieldContest(Contest),
    /// Play a card from hand: hand index, mode index, the playing minion
    /// (modifiers, reactions, action cards), resolved target.
    Play {
        card: u8,
        mode: u8,
        actor: Option<MinionId>,
        target: Target,
    },
    /// Use an ability: where it is printed (a card in play, or a vampire's
    /// text), its index, and the minion using it for a granted action.
    UseAbility {
        source: crate::cards::AbilitySource,
        ability: u8,
        actor: Option<MinionId>,
        /// The ability's target when it picks one (a hunting ground's
        /// vampire), announced with the use.
        target: Target,
    },
    /// Resolve a mandatory effect of a card in play (arena index) pending
    /// at this hook — the player orders them among their own effects.
    ResolveRider {
        card: u32,
    },
    /// Resolve a mandatory effect printed on this vampire, pending at this
    /// hook (Gnaeus Aemilius Augustinus's unlock-phase damage).
    ResolvePrinted {
        minion: MinionId,
    },
    /// Terms of a play: burn this card in play (arena index) at resolution
    /// — "you can burn a card named X (if any)"; `Pass` for none.
    BurnInPlay {
        card: u32,
    },
    /// A mandatory rider's unit of payment: burn 1 pool (Smiling Jack).
    BurnPool,
    /// A mandatory rider's unit of payment: burn 1 blood from this vampire
    /// the player controls (Smiling Jack).
    BurnBloodFrom {
        minion: MinionId,
    },
    /// Numeric terms of the play being announced (rules-qa 3.5).
    SetX(u8),
    /// Discard phase action: discard the card at this hand index.
    Discard(u8),
    /// A library search: take a copy of this card (LIBRARY index); `Pass`
    /// finds nothing. Offered to the searching player only (spec §6.8.4).
    FindCard(u16),
    /// A card (LIBRARY index) taken from one of the player's zones, by
    /// card: a search of library, hand and ash heap at once (Childe of the
    /// Revolution).
    TakeFrom {
        zone: Zone,
        card: u16,
    },
    /// A vampire in the player's uncontrolled region, by slot: removed
    /// from the game (Wider View).
    Slot(u8),
    /// A vampire in the player's ash heap, by crypt card (CRYPT index):
    /// Rotting Behemoth's entry removes it from the game.
    AshVampire(u16),
    /// A card drawn from the library as it would be (rules-qa 9.4): the
    /// top card, rather than a draw alternative.
    DrawLibrary,
    /// A card drawn instead from those on a card in play (arena index),
    /// by card (LIBRARY index): Black Market Cache, Shilmulo Tarot.
    DrawStored {
        card: u32,
        stored: u16,
    },
    /// Move the card at this hand index to the bottom of the library, under
    /// the ones moved before it (Heart of Nizchetus; rules-qa 9.5).
    Bottom(u8),
    /// Terms of an equip action from another minion: take this equipment
    /// (arena index) too; `Pass` once at least one is taken.
    TakeEquipment {
        card: u32,
    },
    TakeAction {
        minion: MinionId,
        action: ActionKind,
        /// Torpor vampire targeted by Rescue / Diablerie; `None` otherwise.
        target: Option<MinionId>,
    },
    DeclareBlock {
        minion: MinionId,
    },
    /// Rescue cost split: blood paid by the acting vampire (the rescued
    /// vampire pays the rest of the 2).
    PayRescue {
        from_acting: u8,
    },
    /// Polling commitment: a vampire source casts all its votes one way
    /// (rules-qa 1.1: sources are atomic and independent).
    CastVotes {
        minion: MinionId,
        favor: bool,
    },
    /// Polling commitment: burn the Edge for 1 vote (a Methuselah ballot).
    CastEdge {
        favor: bool,
    },
    /// Polling commitment: the political action card that called the
    /// referendum provides 1 vote for the acting Methuselah (rulebook
    /// "Gaining Votes").
    CastCardVote {
        favor: bool,
    },
    /// Polling commitment: burn a political action card from hand (hand
    /// index) for 1 vote; a Methuselah gains at most 1 vote from political
    /// action cards per referendum.
    BurnForVote {
        card: u8,
        favor: bool,
    },
    /// Referendum terms: allocate `n` points to this Methuselah (stepwise,
    /// one seat at a time; rules-qa 3.5).
    AllocateTo {
        seat: SeatId,
        n: u8,
    },
    /// Referendum terms: choose this Methuselah.
    ChooseSeat(SeatId),
    /// Referendum terms: choose this minion.
    ChooseMinion(MinionId),
    /// Referendum terms: choose this clan.
    ChooseClan(&'static str),
    /// A Blood Brother circle (New Blood entering play).
    ChooseCircle(&'static str),
    /// A Discipline, lower case (Sanguine Instruction).
    ChooseDiscipline(&'static str),
    /// Referendum terms: choose this card in play (arena index; Disputed
    /// Territory's location).
    ChooseCard(u32),
    /// Referendum terms: allocate `n` points to this card in play (arena
    /// index; Revolutionary Council's locations and equipment), stepwise
    /// after the Methuselahs.
    AllocateToCard {
        card: u32,
        n: u8,
    },
    /// 1 transfer: move 1 pool onto an uncontrolled vampire.
    Transfer {
        slot: u8,
    },
    /// 2 transfers: move 1 counter from an uncontrolled vampire back to pool.
    Withdraw {
        slot: u8,
    },
    /// 4 transfers and 1 pool: move the top crypt card to the uncontrolled
    /// region.
    DrawCrypt,
    /// Move a fully-funded uncontrolled vampire to the ready region.
    InfluenceOut {
        slot: u8,
    },
    /// 4 transfers and 1 pool: merge the uncontrolled vampire with the
    /// other version (base or advanced) the player controls (rulebook
    /// "Advanced").
    Merge {
        slot: u8,
    },
    /// Combat, choose strike: the default hand strike (rulebook "Strike
    /// Effects").
    HandStrike,
    /// Combat, determine range: use a maneuver banked this round.
    UseManeuver,
    /// Combat, press step: use a press banked this round. Its direction
    /// follows from the state: continue, or end the opponent's press to
    /// continue (rulebook "Press").
    UsePress,
    /// Combat, damage prevention: prevent 1 damage with a prevention banked
    /// this round (Precognition [AUS]).
    UsePrevent,
    /// A block attempt's toll (Where the Veil Thins [OBL]): pay it (a pass
    /// resigns the attempt).
    PayBlockToll,
    /// As a card is played: burn the pool it names to cancel it (True
    /// Love's Face [OBF][PRE], by the blocking minion's controller).
    PayToCancel,
    /// Combat, choose strike: a granted "strike: combat ends" (Night
    /// Terrors [OBF][PRE] in the first round).
    StrikeCombatEnds,
    /// Combat, choose strike: the strike granted for the round (Hunger of
    /// Marduk), a ranged one aimed at a retainer at long range (`at`).
    RoundStrike {
        at: Option<crate::state::CardId>,
    },
    /// After resolution: take the damage an action modifier inflicts then
    /// (Daring the Dawn), ordered among the window's effects.
    TakeDamageAfterResolution,
    /// Combat, choose strike: the strike a card played at the maneuver
    /// committed (Slam [POT]).
    CommittedStrike,
    /// Combat, choose strike: "ranged strike: burn weapon" at this opposing
    /// weapon (Voracious Vermin [ANI]).
    BurnWeaponStrike {
        weapon: u32,
    },
    /// Terms: employ this retainer card from hand with this mode (Pack
    /// Alpha [ani]).
    EmployFromHand {
        card: u16,
        mode: u8,
    },
    /// During a block attempt: the attempting minion burns 1 blood for +1
    /// intercept (Eyes of the Wild).
    BurnBloodForIntercept {
        minion: MinionId,
    },
    /// After block resolution: the blocker burns blood to unlock (Truth in
    /// Darkness [OBL]).
    UnlockForBlood {
        minion: MinionId,
    },
    /// In its controller's unlock phase: burn 1 pool to unlock this locked
    /// infernal minion (rulebook "Infernal").
    UnlockInfernal {
        minion: MinionId,
    },
    /// After combat: continue the action as if unblocked (Form of Mist
    /// [PRO]), paying the rider's blood.
    ContinueAction,
    /// Gain the strength a card offers before range is determined (Show of
    /// Force).
    GainStrength,
    /// Burn blood to be immune to damage instead (Rutor's Hand [THA]).
    BurnBloodToAvoid,
    /// Burn 1 of your corruption counters from the minion attempting to
    /// block: its attempt fails, and it cannot attempt again this action
    /// (Enchanting Gaze [PRE][PRO]).
    BurnCorruptionToFail {
        minion: MinionId,
    },
    /// Burn 2 of your corruption counters from a minion of the bled
    /// Methuselah to unlock the acting vampire (Revelation of the Serpent
    /// [PRE][PRO]).
    BurnCorruptionToUnlock {
        minion: MinionId,
    },
    /// A blocked leave-torpor: the blocking vampire diablerises the acting
    /// vampire (rulebook "Leave Torpor").
    Diablerise,
    /// Cancel the combat (Foresee; Stone Travel [MYT] after a block).
    CancelCombat,
    /// Announce a withdrawal from the game (rulebook "Withdrawing from the
    /// Game"): the library exhausted, the turn begun short of a full hand.
    WithdrawFromGame,
    /// Burn 1 blood from a Sabbat vampire following no path for it to
    /// follow `SABBAT_PATHS[path]` (Black Chantry 2025-06-12).
    FollowPath {
        minion: MinionId,
        path: u8,
    },
}

/// Where a card is taken from (`Choice::TakeFrom`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Zone {
    Library,
    Hand,
    AshHeap,
}

/// One name per legality filter (DESIGN §4.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowLabel {
    /// The unlock-phase effects window (rules-qa 3.7, 6.1, 7.8): the active
    /// Methuselah settles their mandatory effects, ordered among their
    /// optional ones, before the impulse leaves; then every seat's
    /// abilities in rotation.
    UnlockAbilities,
    MasterPhase,
    /// Terms of a play or of an in-play ability, chosen as it is announced
    /// (an amount, a copy in play to burn, the master cards to discard).
    PlayTerms,
    /// The choices a mandatory rider needs as it resolves (Smiling Jack:
    /// pool or blood, one unit at a time).
    RiderTerms,
    /// The end of one of the active Methuselah's phases: the impulse goes
    /// round the other seats for their untimed and mandatory effects (The
    /// Coven's transfer), back to the active player after each (rules-qa
    /// 7.9).
    PhaseImpulse,
    /// The as-played window of a card: cancels (and wakes, during an action)
    /// only (rules-qa 2.4).
    CardAsPlayed,
    MinionPhase,
    /// "As the action is announced" plays.
    ActionAsAnnounced,
    /// The block loop (rulebook states A and B): attempts, modifiers,
    /// reactions.
    ActionBlock,
    /// After resolution of an action that reached it, whatever its outcome
    /// (Voter Captivation, Freak Drive); for a political action, after the
    /// referendum.
    ActionAfterResolution,
    /// A bleed would be successful: the cards played "instead" of its pool
    /// loss (Spying Mission [OBF]), after every other modifier and reaction.
    ActionWouldBeSuccessful,
    /// Blocks declined by all (rulebook state C): redirects, Archon
    /// Investigation, remaining modifiers.
    ActionBlocksDeclined,
    /// Blocked, before block resolution (Crypt's Sons).
    ActionBlocked,
    /// After combat (after *all* combats of the block, rules-qa 2.1):
    /// unlock reactions of the blocker, the Form of Mist continuation.
    ActionAfterCombat,
    /// A blocked leave-torpor: the blocker's diablerie opportunity.
    TorporBlock,
    RescueCost,
    /// An entering ally's effect with a choice (War Ghoul's burn).
    EntryEffect,
    /// A choice made as an effect resolves (a minion to lock or corrupt:
    /// Break the Bonds, The Platinum Protocol).
    ResolutionChoice,
    /// Pay a block attempt's toll or resign it (Where the Veil Thins).
    BlockToll,
    /// Distribute blood among pool and minions (Amici Noctis).
    Distribution,
    /// The blood hunt: polling only, no cards (rulebook "The Blood Hunt").
    BloodHuntPolling,
    /// Terms of a political referendum, chosen by the caller once the
    /// action succeeds (rules-qa 3.5).
    ReferendumTerms,
    /// The polling step of a political referendum: votes and ballots
    /// (unordered commitments) and card plays (rulebook "The Referendum").
    ReferendumPolling,
    /// A hand over its size (rulebook "Drawing Cards": "immediately discard
    /// down"): a card returned to hand, a hand-size bonus ended.
    DiscardDown,
    /// Terms of a rulebook action, chosen as it is announced: the equipment
    /// an equip action takes from another minion.
    ActionTerms,
    /// A library search at an action's or a card's resolution (Magic of the
    /// Smith, Black Market Cache): a card or none, the library shuffled
    /// after.
    LibrarySearch,
    /// The cards an exchange action names as it is announced (Lenelle: the
    /// library card in the ash heap, then the card from hand).
    ActionExchange,
    /// A card drawn while a draw alternative applies: from the library or
    /// from a card in play (rules-qa 9.4); beside Visit from the Capuchin,
    /// the treatment of a replacement (ANK 20200905).
    Draw,
    /// The cards drawn by Heart of Nizchetus moved back to the bottom of
    /// the library, one at a time (rules-qa 9.5, 11.1).
    BottomCards,
    InfluenceTransfers,
    DiscardPhase,
    /// Combat steps (rulebook "Combat Sequence"): ordered windows over every
    /// Methuselah, the acting minion's controller, the opposing one's, then
    /// the others clockwise (rules-qa 98.1).
    CombatBeforeRange,
    CombatDetermineRange,
    CombatBeforeStrikes,
    /// One combatant chooses its strike (acting first, then the opponent).
    CombatStrike,
    /// Both strikes chosen, before they resolve (spec §4.3.2).
    CombatBeforeResolution,
    /// Damage prevention by the minion taking damage (acting side first).
    CombatPrevention,
    /// A partial prevention of mixed damage: how many aggravated points it
    /// covers (`SetX`; rules-qa 4.2).
    CombatPreventionAllocation,
    /// Additional strikes after the normal pair.
    CombatAdditionalStrikes,
    CombatPress,
    /// End of round, shared with the would-end replacement point (spec
    /// §4.9.1).
    CombatEndOfRound,
}

#[derive(Clone, Debug)]
pub struct DecisionRequest {
    pub seat: SeatId,
    pub window: WindowLabel,
    pub options: Vec<Choice>,
}

impl DecisionRequest {
    pub fn new(seat: SeatId, window: WindowLabel, options: Vec<Choice>) -> Self {
        Self {
            seat,
            window,
            options,
        }
    }
}
