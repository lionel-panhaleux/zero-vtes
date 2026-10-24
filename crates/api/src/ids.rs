//! Identifiers and the closed sets a client branches on.

use serde::{Deserialize, Serialize};

/// A seat, in seating order from 0.
pub type Seat = u8;

/// A card copy, as one viewer knows it. Numbered places only: cards on
/// the table (minions, cards in play and stored on them, uncontrolled
/// vampires), face-up piles (ash heaps, removed, limbo) and the viewer's
/// own hand. A card in a hidden place — another seat's hand, any library
/// or crypt — has no number. A card coming into a numbered place from a
/// hidden one takes a fresh number, never one used before for this
/// viewer: a number tells nothing the card's face does not. Numbers
/// differ between viewers; records use the engine's own (one per card,
/// in decklist order).
pub type CopyId = u32;

pub type KrcgId = u32;

/// A decision's number: the decisions taken so far in the game, every
/// seat's. Answers name it; a stale one is refused.
pub type DecisionNo = u64;

/// A closed set of snake_case ids, read tolerantly: a value a later host
/// adds reads as `unknown` (a new id is a minor protocol version).
macro_rules! ids {
    ($(#[$m:meta])* $name:ident { $($(#[$vm:meta])* $v:ident),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($(#[$vm])* $v,)*
            /// A value this reader does not know.
            #[serde(other)]
            Unknown,
        }
    };
}
pub(crate) use ids;

ids!(
    /// The active Methuselah's phase.
    Phase {
        Unlock,
        Master,
        Minion,
        Influence,
        Discard,
        /// The game is over.
        Over,
    }
);

ids!(
    /// The window a decision is asked in.
    Window {
        UnlockAbilities,
        MasterPhase,
        /// Terms of a play or an in-play ability, chosen as it is announced.
        PlayTerms,
        /// Choices a mandatory rider needs as it resolves.
        RiderTerms,
        /// The end of one of the active Methuselah's phases: the other
        /// seats' untimed effects.
        PhaseImpulse,
        /// A card's as-played window: cancels (and wakes, in an action).
        CardAsPlayed,
        MinionPhase,
        ActionAsAnnounced,
        /// Block attempts, modifiers, reactions.
        ActionBlock,
        ActionAfterResolution,
        ActionWouldBeSuccessful,
        ActionBlocksDeclined,
        ActionBlocked,
        ActionAfterCombat,
        TorporBlock,
        RescueCost,
        EntryEffect,
        /// A choice made as an effect resolves.
        ResolutionChoice,
        BlockToll,
        Distribution,
        BloodHuntPolling,
        ReferendumTerms,
        ReferendumPolling,
        DiscardDown,
        ActionTerms,
        LibrarySearch,
        ActionExchange,
        Draw,
        BottomCards,
        InfluenceTransfers,
        DiscardPhase,
        CombatBeforeRange,
        CombatDetermineRange,
        CombatBeforeStrikes,
        CombatStrike,
        CombatBeforeResolution,
        CombatPrevention,
        CombatPreventionAllocation,
        CombatAdditionalStrikes,
        CombatPress,
        CombatEndOfRound,
    }
);

ids!(ActionKind {
    Bleed,
    Hunt,
    LeaveTorpor,
    Rescue,
    Diablerie,
    Employ,
    Recruit,
    Political,
    /// An action card's action (not one of the kinds above).
    Card,
    BurnCard,
    StealCard,
    Equip,
    EquipFrom,
    EquipFound,
    /// An action granted by a card in play or a vampire's ability.
    Ability,
    BecomeAnarch,
    /// Enter combat with a Red List minion.
    RedListHunt,
    /// An imbued gets a power.
    Power,
    /// Burn an incapacitated imbued.
    BurnImbued,
});

ids!(
    /// Where an action stands.
    ActionStep {
        Announced,
        /// Blocks may be attempted.
        Open,
        /// A block is being attempted.
        Attempt,
        /// Blocks declined by all.
        Declined,
        WouldSucceed,
        Blocked,
        AfterCombat,
        Resolved,
        /// A diablerie after the action (the diablerist's choices).
        Diablerie,
    }
);

ids!(
    /// What a minion phase waits for before it may end (`Decision::due`).
    DueKind {
        /// A vampire that must hunt first: no blood, or a card says so
        /// (rules-qa 17.2).
        Hunt,
        /// A vampire's printed mandatory action (Elen Kamjian).
        Action,
        /// A card taking control of its host this phase (Puppet Master).
        TakeControl,
    }
);

ids!(
    /// A minion's trait (rulebook "Traits"), printed or gained; a trait
    /// with a parameter (Slave, a circle) is not one of them.
    Trait {
        /// Black Hand: printed, or become so (Mateusz Kazimirski).
        BlackHand,
        /// Infernal: printed, a Great Beast's, a Fury's.
        Infernal,
        /// On the Red List.
        RedList,
        /// Blood Cursed.
        BloodCursed,
        /// Scarce: 3 pool burned as it is influenced out for each vampire
        /// of its clan already controlled.
        Scarce,
        /// Sterile: it cannot act to put new vampires in play.
        Sterile,
        /// Seraph: a Black Hand trait (Tattoo Signal requires one).
        Seraph,
    }
);

ids!(MinionKind {
    Vampire,
    Ally,
    /// A library card as a vampire.
    Created,
    Token,
    /// A crypt card as a mortal ally (rulebook "Appendix: Imbued Rules").
    Imbued,
});

ids!(
    /// A seat's zone.
    Zone {
        Crypt,
        Library,
        Hand,
        AshHeap,
        Removed,
        Uncontrolled,
        Ready,
        Torpor,
        /// In play on no minion.
        InPlay,
        /// A card being played, not resolved yet.
        Limbo,
        /// Stored on a card in play (`ZoneRef::on`).
        Stored,
        /// In the uncontrolled region with training counters (Web of
        /// Knives Recruit), out of play: the view's `training`.
        Training,
        /// The incapacitated region: imbued that lost their last life,
        /// controlled, not ready (rulebook "Appendix: Imbued Rules" 5).
        Incapacitated,
    }
);

ids!(Range { Close, Long });

ids!(StrikeKind {
    Hand,
    Melee,
    Dodge,
    CombatEnds,
    Damage,
    StealBlood,
    TorporOrBurn,
    Torpor,
    /// A strike whose effect is its card's text.
    CardEffect,
    BurnWeapon,
    /// The opponent's weapon named with the strike moves to the striker.
    StealWeapon,
    BurnEquipment,
    RangedBurnEquipment,
    Corrupt,
    /// Life burned from the opposing ally (`n`), not damage (Argent Baton).
    BurnLife,
});

ids!(ReferendumStep {
    Terms,
    Polling,
    Shortfall,
    /// A choice the passed referendum's effect asks.
    Choose,
});

ids!(
    /// Where votes come from.
    VoteSource {
        /// A vampire casting its votes.
        Vampire,
        /// Votes printed on a minion that does not cast them itself.
        Printed,
        Edge,
        /// A political action card's vote.
        Card,
        /// A card in play's votes.
        InPlay,

        /// Pool burned for votes.
        Pool,    }
);

ids!(
    /// What an option does (the engine's choice kinds).
    OptionKind {
        Pass,
        EdgePool,
        PayContest,
        YieldContest,
        Play,
        UseAbility,
        ResolveRider,
        ResolvePrinted,
        BurnInPlay,
        BurnPool,
        BurnBloodFrom,
        SetX,
        Discard,
        FindCard,
        TakeFrom,
        Slot,
        AshVampire,
        DrawLibrary,
        DrawStored,
        Bottom,
        TakeEquipment,
        TakeAction,
        DeclareBlock,
        PayRescue,
        CastVotes,
        CastEdge,
        CastCardVote,
        BurnForVote,
        AllocateTo,
        ChooseSeat,
        ChooseMinion,
        ChooseClan,
        ChooseCircle,
        ChooseDiscipline,
        ChooseCard,
        AllocateToCard,
        Transfer,
        Withdraw,
        DrawCrypt,
        InfluenceOut,
        Merge,
        HandStrike,
        UseManeuver,
        UsePress,
        UsePrevent,
        PayBlockToll,
        PayToCancel,
        StrikeCombatEnds,
        RoundStrike,
        TakeDamageAfterResolution,
        CommittedStrike,
        BurnWeaponStrike,
        EmployFromHand,
        BurnBloodForIntercept,
        UnlockForBlood,
        UnlockInfernal,
        ContinueAction,
        GainStrength,
        BurnBloodToAvoid,
        BurnCorruptionToFail,
        BurnCorruptionToUnlock,
        Diablerise,
        CancelCombat,
        /// Announce a withdrawal from the game (§1).
        WithdrawFromGame,
    }
);

ids!(
    /// What a choice an effect asks is for (the engine's interrupt): with
    /// `source`, what the decision chooses. Open: a later engine adds more.
    Purpose {
        /// `seat` moves `card` (Fear of Mekhet) to another justicar or Inner
        /// Circle member, any Methuselah's, in torpor included (ANK 20210109;
        /// rules-qa 49.4: not locked by the move); burned if there is none.
        MoveCardToTitled,
        /// Charnas the Imp: its employer about to be burned, `seat` may put
        /// `card` on another minion in play, any Methuselah's; declined, it
        /// burns.
        MoveOffBurnedBearer,
        /// Game of Malkav: `seat` secretly chooses a number (`set_x`, from
        /// 1); another viewer is told a number was chosen, not which.
        SecretNumber,
        /// Alastor: `seat` may search their library for an equipment card
        /// to put on `minion` (`find_card`, or pass), requirements ignored,
        /// for half its cost rounded down.
        SearchEquipFor,
        /// Summon History: `seat` may search library and/or ash heap for an
        /// ally, retainer or equipment costing `n` or less (`take_from`), or
        /// crypt and/or ash heap for a vampire of capacity `n` or less
        /// (`ash_vampire`), to put in play for `minion`; or pass.
        PutInPlaySearch,
        /// A title card moving on (Osebo Kholo): `seat` picks which of the
        /// oldest ready untitled vampires of its clan and sect gets `card`.
        MoveCardToOldest,
        /// Superior Mettle: `minion` may burn `n` blood to prevent the
        /// opposing initial strike's damage this round (`resolve_printed`, or
        /// pass).
        MayPreventForBlood,
        /// Public Vilification short of blood: `seat` picks the next vampire
        /// to receive 1 blood from `minion`.
        ShortfallPicks,
        /// Baba Yaga: `minion` may unlock (`resolve_printed`, or pass).
        MayUnlock,
        /// Curse of Nitocris: `seat` picks the Methuselah who takes control
        /// of `card`.
        GiveCardTo,
        /// Blood Trade: 1 blood from `from` to a vampire of another
        /// Methuselah's, the player's choice.
        GiveBlood,
        /// Heidelberg Castle: `seat` moves from `from` to another ready
        /// vampire of theirs — chosen first (`to`), then the blood (any
        /// amount, the excess to the bank: ANK 20170804), then equipment and
        /// retainers one at a time, each one `to` could hold (LSJ 19980831),
        /// until done.
        MoveBetween,
        /// Diablerie: `seat`'s diablerist (`to`) takes the victim's
        /// (`minion`) equipment, one card at a time, each one it could
        /// hold, until done; the victim then burns (rulebook "Diablerie").
        /// Also Sacrificial Lamb's burn of a vampire in torpor, not a
        /// diablerie.
        DiablerieEquipment,
        /// "Burned unless its controller burns n pool" (Conquest of Humanity):
        /// `seat` pays or lets `card` burn.
        PayOrBurn,
        /// Nightmares upon Nightmares: `minion` is locked unless its
        /// Methuselah discards a card (not replaced until their master phase).
        LockOrDiscard,
        /// Society of Leopold: `minion` burns `n` blood or is burned, its
        /// Methuselah's choice (Pass: burned) — or `card` is burned instead
        /// (Robert Carter: "Carter's employer burns 1 blood, or Carter is
        /// burned").
        BloodOrBurned,
        /// A replacement Visit from the Capuchin would take instead while a
        /// draw alternative applies too: one treatment or the other (ANK
        /// 20200905; rules-qa 9.4). `except`: the card being replaced.
        Replacement,
        /// Draw up to hand size, each card from the library or a draw
        /// alternative.
        DrawUp,
        /// Heart of Nizchetus: `left` cards still to draw, `drawn` so far; the
        /// same number then goes to the bottom of the library.
        Draws,
        /// `left` cards still to move from hand to the bottom of the library.
        Bottom,
        /// Black Market Cache's search: up to `left` more cards to put on
        /// `card`.
        StoreSearch,
        /// "After this ally enters play, burn an ally or retainer you control"
        /// (War Ghoul): the controller's pick among the others, the entering
        /// ally itself if there is none (RTR 19961113).
        EntryBurn,
        /// "After this ally enters play, burn it unless you remove an ally or
        /// vampire in your ash heap from the game" (Rotting Behemoth): the
        /// controller may decline, and the ally burns (LSJ 20030701-2).
        EntryRemoveOrBurn,
        /// A block attempt made costly as it goes on (Where the Veil Thins
        /// [OBL], Tenebrous Form [OBT]): its minion pays the `n` blood that
        /// card adds, or resigns the attempt and pays nothing (LSJ
        /// 20031121-2); an ally cannot burn blood.
        BlockToll,
        /// Dawn Operation [for]: the vampire attempting to block may cancel
        /// its attempt (`ChooseMinion` it, or pass).
        CancelAttempt,
        /// Amici Noctis: `left` blood to distribute; first how much to the
        /// pool (at most `cap`), the rest among the ready `clan` vampires.
        MarginPool,
        /// Amici Noctis: `left` blood for the ready `clan` vampires below
        /// capacity, 1 each, offered in arena order from minion `next`: give
        /// or skip, a skip only while enough of them are left.
        MarginBlood,
        /// A minion controlled by `of`, chosen as an effect resolves (Break
        /// the Bonds [obf], The Platinum Protocol [pro]: "the target is chosen
        /// upon resolution", ANK 20240706): locked, if the player wants to, or
        /// given one of the player's corruption counters.
        MinionOf,
        /// Whose corruption counter on `minion` burns, several Methuselahs'
        /// being on it (Aniel).
        CorruptionOwner,
        /// `minion` burns `blood` to be immune to `damage` unpreventable
        /// aggravated damage, or takes it (Rutor's Hand [THA]).
        AvoidDamage,
        /// `seat` looks at `of`'s hand and discards a card of their choice
        /// from it, which `of` replaces (Revelations [aus]). Asked first
        /// with no `of` and `choose_seat` options where several hands may
        /// be the one (Vaticination: each other Methuselah's), then with
        /// `of` and `find_card` options.
        DiscardFromHandOf,
        /// `seat` discards `left` more cards of their choice from hand (Shroud
        /// of Decay [obl]); all `done` are replaced once every discard is made
        /// (advanced rules §6.8.3). `seat` reorders the top `total` cards of
        /// their library (Eulogio Sánchez de los Reyes): `placed` of them
        /// chosen so far, from the top down; the rest still in their old order
        /// below them.
        Reorder,
        /// `seat` discards `n` more cards of their choice from hand; `detail`
        /// `combat` (combat cards only) or `animal` (animals only: Call the
        /// Wild Hunt's X).
        DiscardChosen,
        /// `seat` removes `left` more cards of their choice from `of`'s ash
        /// heap from the game; then `of` burns `pool` pool (Shroud of Decay
        /// [OBL]; LSJ 20090710: there is no order to the ash heap) — from
        /// their own, pool 0, Marchesa Liliana's cost. Burned vampires are ash
        /// heap cards too (rules-qa 42.7).
        RemoveFromAsh,
        /// "You can search your library (shuffle afterward), hand and/or ash
        /// heap for a Discipline master card and put it on this new vampire"
        /// (Childe of the Revolution), or on the diablerist of an older
        /// vampire (advanced rules 4): put on, not played.
        DisciplineSearch,
        /// Learjet: `seat` may draw an additional card (`DrawLibrary`), the
        /// hand then discarded down; or pass.
        MayDrawExtra,
        /// Infernal Pursuit [CEL], several cards replaced at once: one
        /// replacement of `seat`'s and its `extra` additional cards, then as
        /// many discarded, before the next (ANK 20180925-1).
        PursuitDraw,
        /// Those `left` extra cards of `seat`'s discarded, not replaced.
        PursuitDiscard,
        /// Baltimore Purge: `seat` chooses a ready vampire of `of`'s not older
        /// than `host` (the purge's vampire), sent to torpor with it once
        /// every Methuselah's is chosen.
        PurgePick,
        /// "Steal up to n blood" (François Villon G2): `seat` sets how much, 1
        /// to `max`, from `from` to `to` (`SetX`).
        StealUpTo,
        /// "Move up to n blood from the acting vampire to this new vampire"
        /// (Third Tradition: Progeny): `seat` sets how much, 0 to `max` of
        /// what `minion` has, to `to` (`SetX`).
        MoveBloodUpTo,
        /// Antonio d'Erlette (`minion`): a mage ally from `seat`'s library or
        /// hand he could play, put in play, its cost paid (Pass: none); the
        /// library shuffled either way.
        AllySearch,
        /// "Remove a crypt card in your uncontrolled region from the game"
        /// (Wider View): the player's pick, the counters on it to the blood
        /// bank; removed cards are public (LSJ 20080314).
        RemoveUncontrolled,
        /// "As … its controller changes, its controller chooses a ready
        /// vampire they control" (The Rack): none if they have none.
        ChooseForCard,
        /// Garibaldi-Meucci Museum: the ash heap card requiring an Anarch,
        /// then the hand card it is exchanged for.
        ExchangeAnarch,
        /// "If the attached minion is about to leave the ready region, you can
        /// burn this card to gain n pool" (Priority Contract).
        PoolForCard,
        /// "You can lock this card … to gain n pool" (The Black Throne).
        LockForPool,
        /// "You can burn those counters to steal that minion" (Cave of
        /// Apples).
        StealForCorruption,
        /// "You can burn 1 blood from this card to make that vampire Anarch"
        /// (Twilight Camp).
        MakeAnarch,
        /// Ashur Tablets' picks from `seat`'s ash heap: first the card to hand
        /// (a pass chooses none), then up to `left` more shuffled into the
        /// library (a pass stops), `to_library` once the hand card is chosen.
        AshurPicks,
        /// "A non-titled vampire you control becomes Anarch" (Gilbert Duane):
        /// the player's pick, none if they have none.
        BecomesAnarch,
        /// Anarch Convert entering play: "you can remove him from the game to
        /// make a non-titled vampire you control Anarch" — that vampire chosen
        /// (`chosen`), then "either gain 1 pool or draw 1 card from your
        /// crypt". Before any contest (LSJ 20080526, RTR 20180303): `forced`
        /// when he came out beside a copy his Methuselah controls (no
        /// voluntary self-contest); declined, his contests are staged.
        AnarchConvert,
        /// "Add n blood to a `clan` vampire in your uncontrolled region"
        /// (Hunting the Beast [AUS]): the player's pick, none if they have
        /// none. With no clan, any of them, and optional ("may move a counter
        /// from the bank to a vampire in his or her uncontrolled region",
        /// Flames of Insurrection).
        UncontrolledBlood,
        /// "Lock this card or burn n pool" (Bleeding the Vine): `ResolveRider`
        /// locks it (if unlocked), `BurnPool` pays.
        LockOrPay,
        /// "Burns n pool unless they burn a vampire they control with
        /// capacity `cap` or more to burn this card" (Antediluvian
        /// Awakening): `BurnPool` pays, `ChooseMinion` burns that vampire and
        /// the card.
        PoolOrVampire,
        /// "That Methuselah chooses either to burn n pool or to lose all
        /// transfers during his or her next influence phase" (Leandro):
        /// `BurnPool` burns, `Pass` loses the transfers.
        PoolOrNoTransfers,
        /// "Each anarch burns 1 blood or becomes Camarilla" (Waiting Game):
        /// `BurnBloodToAvoid` burns, `Pass` makes it Camarilla.
        BloodOrCamarilla,
        /// Maabara: a library card of one's ash heap onto the location
        /// (`FindCard`).
        StoreOwnAsh,
        /// Maabara: a card on the location to the top of one's library
        /// (`FindCard`).
        StoredToLibraryTop,
        /// "Search your library for a <filter> card … add it to your hand"
        /// (Fortschritt Library): a library card (`FindCard`), or none
        /// (Pass).
        LibraryToHand,
        /// "Search your crypt for a `clan` vampire … move them to your
        /// uncontrolled region (shuffle afterward)" (Disciple of Lazarus): a
        /// crypt card (`Choice::AshVampire`, a CRYPT index), or none (Pass).
        CryptSearch,
        /// Solomon Batanea: the slot of `of`'s uncontrolled region that gives
        /// 1 blood to `of`'s pool (`Choice::Slot`, `of`'s region).
        PreySlotToPool,
        /// Erlik entering play: up to `left` more blood moved from `from` to
        /// `seat`'s uncontrolled `clan` vampires, one at a time
        /// (`Choice::Slot`); Pass stops. His contests are staged after (LSJ
        /// 20080526).
        DistributeFrom,
        /// "Move any amount of blood from the Blood Brothers in the <circle>
        /// to any other Blood Brothers in this circle" (Jerry,
        /// Redistribution): one move at a time among `seat`'s — `from` the
        /// source, `to` the destination, then the amount (`Choice::SetX`),
        /// each move a gain of its own (Slake the Thirst; owner, rules-qa
        /// 68.10), one blood move a pair of them (`paired`, source and
        /// destination); Pass ending the moves. With `equipment`, their
        /// equipment too ("move any amount of blood and equipment",
        /// Redistribution), one at a time, as many as wanted (`moving` the one
        /// picked).
        CircleBlood,
        /// "Move any number of blood counters from your pool to 1 or more
        /// vampires you control" (Gird Minions): a vampire (`to`), then the
        /// amount (`Choice::SetX`), up to its room (rulebook: no more blood
        /// than capacity); Pass ends the moves.
        PoolToVampires,
        /// Legacy: `minion`'s base version searched for in `seat`'s crypt
        /// (`Choice::AshVampire`, taken from the crypt first, then the ash
        /// heap), uncontrolled region (`Choice::Slot`) and ash heap, and
        /// merged with it; Pass finds none. The crypt shuffled after.
        BaseSearch,
        /// New Blood entering play: the circle he belongs to
        /// (`Choice::ChooseCircle`, among `state::circles`), or his own circle
        /// of one (Pass).
        ChooseCircle,
        /// "Move a wraith or zombie ally from your ash heap to your ready
        /// region" (Split the Veil): the player's pick among the allies
        /// matching one of `of`; with `named`, the pick is named for the
        /// action being declared, put in play as it resolves (Hel-Blá).
        AllyFromAsh,
        /// Victor Donaldson: `card`, canceled as `seat` played it, may move
        /// from their ash heap to their library, shuffled (`ResolvePrinted`).
        MayRecycleCanceled,
        /// Malgorzata: the vampire of `seat`'s uncontrolled region to remove,
        /// another than the action's target (`Choice::Slot`).
        NameUncontrolled,
        /// Lázár Dobrescu: named as the action is announced (rules-qa default
        /// readings) — `from`, a vampire with blood in `of`'s uncontrolled
        /// region, then the one of `seat`'s that gets it (`Choice::Slot`).
        NameSlotPair,
        /// The Great Beast's three Disciplines, one at a time (`Choice::Slot`
        /// into `GREAT_BEAST_DISCIPLINES`).
        BeastDisciplines,
        /// The Erciyes Fragments: the library card of `seat`'s prey's ash heap
        /// to move onto `card`, face down (`Choice::FindCard`).
        StorePreyAsh,
        /// The Sargon Fragment: the library card of `seat`'s ash heap the
        /// action declared names.
        NameOwnAshCard,
        /// Hand of Conrad: the vampire of `seat`'s ash heap the action
        /// declared names (`Choice::AshVampire`).
        NameAshVampire,
        /// Giovanni del Georgio: the card two copies of which in `of`'s ash
        /// heap the action declared names (`Terms::PreyAshPair`).
        NameAshPair,
        /// "You can put this card on this vampire" (Fever Pitch [FOR]/[PRE]):
        /// `ChooseMinion` it, or pass — optional, though never worse than the
        /// ash heap: an agent may take it unasked (owner, 2026-10-08: rules-qa
        /// 53.5).
        MayPutOn,
        /// "You can add 1 counter to this location" (Cursed Abattoir):
        /// `ResolveRider` it, or pass — never worse to add (rules-qa 55.6).
        MayAddCounter,
        /// "You can move any aim card the bearer uses to this card" (Light
        /// Intensifying Goggles): the card just used, from the ash heap onto
        /// it (`ResolveRider`), or pass.
        MayStoreAim,
        /// The Path of Lilith: `seat` may lock `card` to put a torture counter
        /// on `minion` (`ResolveRider`), or pass.
        MayTorture,
        /// "You can gain n pool" printed on `minion` (Matasuntha): Pass, or
        /// `ResolvePrinted` to gain it — optional, never worse.
        MayGainPool,
        /// "You can get +n hand size until the end of the turn" printed on
        /// `minion` (Sascha Vykos), once each turn: Pass, or `ResolvePrinted`.
        MayGainHandSize,
        /// A ready titled vampire of `seat`'s was burned or sent to torpor:
        /// the out-of-turn card played for it (Leadership Vacuum), or pass.
        ReadyTitledLeft,
        /// Legion's action announced: the master Discipline card named, from
        /// hand or ash heap (`find_card`).
        NameMasterDiscipline,
        /// Filchware's Pawn Shop's equip action announced: `minion`'s
        /// equipment named in an ash heap, the heap first (`choose_seat`),
        /// then the card (`find_card`; `seat`: that heap, `detail`:
        /// `named`).
        NameAshEquipment,
        /// Storage Annex as it is played: the card of the hand put face
        /// down on `card` (`find_card`), replaced.
        StoreOwnHand,
        /// "When it enters play, you may pay n pool to make it <title>"
        /// (Victor Pelletier, `minion`): `SetX(n)` pays n pool for the title
        /// its option's effect names (`role`, `detail`: `primogen`,
        /// `prince_of_paris`…), or pass.
        BuyTitle,
        /// "This acting vampire can burn n blood to move the stolen vampire to
        /// your ready region" (Graverobbing [DOM]): `ChooseMinion` it, or
        /// pass.
        MoveForBlood,
        /// "Once each turn, when you draw a card from your library, you may
        /// place that card on the bottom of your library and draw a new card"
        /// (Blackhorse Tanner, `minion`): `DrawLibrary` to do it with `card`,
        /// just drawn, or pass.
        Redraw,
        /// Imbued `minion` gains a conviction card from `seat`'s hand or ash
        /// heap (its library too, shuffled after, when entering play):
        /// `TakeFrom`, or pass.
        GainConviction,
        /// "[n CONVICTION]": imbued `minion` burns its conviction cards one
        /// at a time, `seat` choosing which (`BurnInPlay`) when they differ.
        BurnConvictions,
        /// An incapacitated imbued burned: ready imbued `minion` may burn a
        /// conviction (`BurnInPlay`) for 1 unpreventable damage to `to`.
        AvengeImbued,
        /// Acting imbued `minion`'s action "costs 1 conviction": `seat`
        /// names which (`BurnInPlay`), burned at resolution.
        NameConviction,
        /// Imbued `minion` entering play: a power requiring its virtue from
        /// `seat`'s library or hand put on it (`TakeFrom`), or pass; the
        /// library shuffled either way.
        PowerSearch,
        /// `seat` may cancel the combat `minion` is in (`CancelCombat`), or
        /// pass: Foresee's imbued (the action continuing as if unblocked
        /// when it blocked), Stone Travel [MYT]'s blocked acting vampire.
        MayCancelCombat,
        /// Goblinism [MYT]: the haven `card` just searched to `seat`'s hand
        /// put on `minion` instead (`FindCard`), its cost paid, or kept
        /// (Pass).
        PutFoundOn,
    }
);
ids!(
    /// What a chosen minion or Methuselah becomes for the card choosing it
    /// (completed with the host, A3).
    Role {
        /// Does not unlock as normal (Toreador Grand Ball's first).
        DoesNotUnlock,
        /// Its non-bleed actions cannot be blocked (the Grand Ball's second).
        UnblockableNonBleed,
        /// The card's chosen minion, for its later effects.
        Chosen,
        /// Becomes Anarch (Gilbert Duane, Anarch Convert).
        Anarch,
        /// No reactions, blocks or votes this turn (Expel).
        Expelled,
        /// Becomes Camarilla (Waiting Game's anarch not burning blood).
        Camarilla,
    }
);

ids!(
    /// Public state on a minion beyond blood and cards (completed with
    /// the host, A3).
    MarkerKind {
        /// Corruption counters (`seat`: whose, `n`).
        Corruption,
        Hostage,
        Nightmare,
        Torture,
        Disease,
        TitleContested,
        Expelled,
        /// A card's lasting effect on it (`card` says which, `n` its
        /// counters).
        Effect,
        /// A Red List minion marked this turn (`seat`: by whom), who may
        /// enter combat with it.
        RedListMark,
        /// Pact counters (`n`; Diabolic Lure).
        Pact,
        /// Taint counters (`n`; Barbaro Lucchese).
        Taint,
        /// Out of play with its cards, its effects remembered (Descent into
        /// Darkness).
        OutOfPlay,
        /// Contested yet in play and usable (a two-player game, §1): 1 pool
        /// in each of its controller's unlock phases, or yielded, burned.
        Contested,
    }
);
