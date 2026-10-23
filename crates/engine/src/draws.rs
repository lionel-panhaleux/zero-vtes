//! Interrupts: choices owed by one player in the middle of a resolution,
//! served in order before any other decision (engine.md §6.2), among them
//! draws, draw alternatives (Black Market Cache, Shilmulo Tarot),
//! replacement treatments (Visit from the Capuchin) and searches (§6.14).

use crate::cards::{self, CardFilter, Effect, Hook};
use crate::decision::{Choice, DecisionRequest, WindowLabel, Zone};
use crate::game::Engine;
use crate::ids::{MinionId, SeatId};
use crate::library::Label;
use crate::notes::{Cause, Source};
use crate::plays::HookAt;
use crate::state::{distinct, CardId, CardInPlay, Release};

/// A pending choice about cards drawn, moved or stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interrupt {
    /// Fear of Mekhet moves to another justicar or Inner Circle member, in
    /// torpor too, else burns (ANK 20210109; rules-qa 49.4: not locked).
    MoveCardToTitled { seat: SeatId, card: CardId },
    /// Alastor: `seat` may search their library for an equipment to put on
    /// `minion`, requirements ignored, inferior, half its cost rounded down
    /// (the blood the vampire's, X as 0: LSJ 20040518, 20100204); shuffled
    /// afterward, found or not (LSJ 20050331-2).
    SearchEquipFor {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Baba Yaga: `minion`, locked, may unlock (once each turn).
    MayUnlock {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Curse of Nitocris: `seat`, its controller, picks the Methuselah,
    /// any, who takes control of `card`.
    GiveCardTo { seat: SeatId, card: CardId },
    /// Public Vilification short of blood: `seat` picks, one blood at a
    /// time, which of the vampires allotted it receive from `from`.
    ShortfallPicks {
        seat: SeatId,
        from: crate::ids::MinionId,
        /// One slot per blood allotted (Public Vilification's 3), fixed for
        /// `Copy`.
        to: [Option<crate::ids::MinionId>; 8],
    },
    /// Superior Mettle [FOR], Stunt Cycle's opponent: `minion` may burn `n`
    /// blood to prevent the opposing strike's damage this round.
    MayPreventForBlood {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// Osebo Kholo: its title card moves to the oldest ready untitled vampire
    /// of its clan and sect, `seat` (its controller) choosing among equals.
    MoveCardToOldest { seat: SeatId, card: CardId },
    /// Summon History: `seat` may put in play for `minion` a card found
    /// costing or of capacity `x` or less (LSJ 20100204, PIB 20111101).
    PutInPlaySearch {
        seat: SeatId,
        minion: crate::ids::MinionId,
        x: u8,
        vampire: bool,
    },
    /// Game of Malkav: `seat` secretly chooses a number from 1 to `max`
    /// (`Choice::SetX`, kept apart from the state, `Engine::secret_numbers`);
    /// the `last` pick reveals them all at once.
    SecretNumber { seat: SeatId, max: u8, last: bool },
    /// Charnas the Imp: its employer about to be burned, `seat` (the
    /// employer's controller) may put `card` on another minion in play, any
    /// Methuselah's; declined, it burns.
    MoveOffBurnedBearer {
        seat: SeatId,
        card: CardId,
        from: crate::ids::MinionId,
    },
    /// Blood Trade: 1 blood from `from` to a vampire of another
    /// Methuselah's, the player's choice; with `anarchs`, to another ready
    /// Anarch of the player's (The Status Perfectus).
    GiveBlood {
        seat: SeatId,
        from: MinionId,
        anarchs: bool,
    },
    /// Diablerie step 2 (rulebook "Diablerie"): the diablerist may take
    /// any equipment on the victim it can hold, a card at a time; as it
    /// ends, step 3 (the victim burns, its blood to the diablerist). Not
    /// `diablerie`: Sacrificial Lamb's burn, blood as the victim's capacity.
    DiablerieEquipment {
        seat: SeatId,
        diablerist: MinionId,
        victim: MinionId,
        diablerie: bool,
    },
    /// Heidelberg Castle: `to` first, then blood (excess to the bank, ANK
    /// 20170804), then equipment and retainers `to` can hold (LSJ 19980831).
    MoveBetween {
        seat: SeatId,
        from: MinionId,
        to: Option<MinionId>,
        blood_done: bool,
        how: crate::cards::Moves,
    },
    /// "Burned unless its controller burns n pool" (Conquest of Humanity):
    /// `seat` pays or lets `card` burn.
    PayOrBurn {
        seat: SeatId,
        card: CardId,
        pool: u8,
    },
    /// Nightmares upon Nightmares: `minion` locks unless its Methuselah
    /// discards (not replaced until their master phase).
    LockOrDiscard { seat: SeatId, minion: MinionId },
    /// Maabara: a library card of `seat`'s ash heap moved onto `card`.
    StoreOwnAsh { seat: SeatId, card: CardId },
    /// Storage Annex as it is played: a card of `seat`'s hand put face down
    /// on `card`, replaced.
    StoreOwnHand { seat: SeatId, card: CardId },
    /// Maabara: a card on `card` moved to the top of `seat`'s library.
    StoredToLibraryTop { seat: SeatId, card: CardId },
    /// Waiting Game: an anarch burns 1 blood (`BurnBloodToAvoid`) or
    /// becomes Camarilla (`Pass`, the only way with no blood).
    BloodOrCamarilla { seat: SeatId, minion: MinionId },
    /// Society of Leopold: `minion` burns `n` blood or is burned (Pass); or
    /// `card` burns instead (Robert Carter: "or Carter is burned").
    BloodOrBurned {
        seat: SeatId,
        minion: MinionId,
        n: u8,
        card: Option<CardId>,
    },
    /// Visit from the Capuchin's replacement beside a draw alternative: one
    /// or the other (ANK 20200905; rules-qa 9.4); `except` the card replaced.
    Replacement {
        seat: SeatId,
        except: Option<CardId>,
    },
    /// Draw up to hand size, each card from the library or a draw
    /// alternative.
    DrawUp { seat: SeatId },
    /// Heart of Nizchetus: `left` cards still to draw, `drawn` so far; the
    /// same number then goes to the bottom of the library.
    Draws { seat: SeatId, left: u8, drawn: u8 },
    /// `left` cards still to move from hand to the bottom of the library.
    Bottom { seat: SeatId, left: u8 },
    /// Black Market Cache's search: up to `left` more cards onto `card`.
    StoreSearch {
        seat: SeatId,
        card: CardId,
        left: u8,
    },
    /// War Ghoul's entry burn: another ally or retainer, else itself (RTR
    /// 19961113).
    EntryBurn {
        seat: SeatId,
        ally: crate::ids::MinionId,
    },
    /// Rotting Behemoth: remove an ally or vampire from the ash heap, or
    /// decline and it burns (LSJ 20030701-2).
    EntryRemoveOrBurn {
        seat: SeatId,
        ally: crate::ids::MinionId,
    },
    /// A toll landing on a block attempt under way (Where the Veil Thins
    /// [OBL], Tenebrous Form [OBT]): pay `n` blood or resign (LSJ
    /// 20031121-2); an ally cannot burn blood.
    BlockToll {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// Dawn Operation [for]: the vampire attempting to block may cancel its
    /// attempt (`ChooseMinion` it, or pass).
    CancelAttempt {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Amici Noctis: of `left` blood, how much to the pool (at most `cap`);
    /// the rest to the ready `clan` vampires.
    MarginPool {
        seat: SeatId,
        left: u8,
        cap: u8,
        clan: &'static str,
    },
    /// Amici Noctis: 1 blood each to the ready `clan` vampires below
    /// capacity, from minion `next`; a skip only while enough are left.
    MarginBlood {
        seat: SeatId,
        left: u8,
        next: u32,
        clan: &'static str,
    },
    /// A minion of `of`'s chosen at resolution, for `purpose` (Break the
    /// Bonds [obf], The Platinum Protocol [pro]; ANK 20240706).
    MinionOf {
        seat: SeatId,
        of: SeatId,
        purpose: MinionPurpose,
    },
    /// Whose corruption counter on `minion` burns, several Methuselahs'
    /// being on it (Aniel).
    CorruptionOwner {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// `minion` burns `blood` to be immune to `damage` unpreventable
    /// aggravated damage, or takes it (Rutor's Hand [THA]).
    AvoidDamage {
        seat: SeatId,
        minion: crate::ids::MinionId,
        blood: u8,
        damage: u8,
    },
    /// Revelations [aus]: `seat` discards a card from `of`'s hand, which `of`
    /// replaces; `of` none: the hand chosen first, among every hand looked
    /// at (Vaticination).
    DiscardFromHandOf { seat: SeatId, of: Option<SeatId> },
    /// Eulogio Sánchez de los Reyes: the top `total` library cards
    /// reordered, `placed` chosen so far from the top down.
    Reorder { seat: SeatId, total: u8, placed: u8 },
    /// Shroud of Decay [obl]: `left` more discards from hand; the `done`
    /// replaced once all are made (advanced rules §6.8.3).
    DiscardChosen {
        seat: SeatId,
        left: u8,
        done: u8,
        /// Of this kind only: combat cards (Target Vitals' cancel: "discard
        /// two combat cards"), animals (Call the Wild Hunt).
        of: Option<crate::cards::CardFilter>,
    },
    /// `left` more cards removed from `of`'s ash heap, then `of` burns `pool`
    /// (Shroud of Decay [OBL], LSJ 20090710: no order to the ash heap;
    /// Marchesa Liliana's own, pool 0). Burned vampires count (rules-qa 42.7).
    RemoveFromAsh {
        seat: SeatId,
        of: SeatId,
        left: u8,
        pool: u8,
    },
    /// A Discipline master from library, hand or ash heap put on `minion`,
    /// not played: Childe of the Revolution's new vampire, a diablerist of
    /// an older vampire (advanced rules 4).
    DisciplineSearch {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Learjet: `seat` may draw an additional card (`DrawLibrary`), the
    /// hand then discarded down; or pass.
    MayDrawExtra { seat: SeatId },
    /// Infernal Pursuit [CEL]: one replacement and its `extra` draws, then
    /// as many discards, before the next (ANK 20180925-1).
    PursuitDraw { seat: SeatId, extra: u8 },
    /// Those `left` extra cards of `seat`'s discarded, not replaced.
    PursuitDiscard { seat: SeatId, left: u8 },
    /// Baltimore Purge: a ready vampire of `of`'s not older than `host`, sent
    /// to torpor with it once every Methuselah's is chosen.
    PurgePick {
        seat: SeatId,
        of: SeatId,
        host: MinionId,
    },
    /// Third Tradition: Progeny: 0 to `max` blood from `from` to `to`
    /// (`SetX`).
    MoveBloodUpTo {
        seat: SeatId,
        from: MinionId,
        to: MinionId,
        max: u8,
    },
    /// François Villon G2's "steal up to n blood": 1 to `max` (`SetX`).
    StealUpTo {
        seat: SeatId,
        from: MinionId,
        to: MinionId,
        max: u8,
    },
    /// An incapacitated imbued burned: `imbued`, ready, may burn a
    /// conviction for 1 unpreventable damage to `acting` (rulebook
    /// "Appendix: Imbued Rules" 5).
    AvengeImbued {
        seat: SeatId,
        imbued: crate::ids::MinionId,
        acting: crate::ids::MinionId,
    },
    /// An action "that costs 1 conviction": which of acting imbued
    /// `minion`'s it pays with, named as announced (`ActionCtx::conviction`;
    /// RTR 20111202).
    NameConviction {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// A power requiring `virtue` from `seat`'s library (shuffled after) or
    /// hand put on imbued `minion` (Inez "Nurse216" Villagrande), or none.
    PowerSearch {
        seat: SeatId,
        minion: crate::ids::MinionId,
        virtue: &'static str,
    },
    /// `seat` may cancel the combat `minion` is in (`CancelCombat`): Foresee's
    /// imbued (blocking, the action continues as if unblocked), Stone Travel
    /// [MYT]'s blocked acting vampire.
    MayCancelCombat {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// "[n CONVICTION]": imbued `minion` burns n of its conviction cards,
    /// one at a time, `seat` choosing among their names (RTR 20111202).
    BurnConvictions {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// An imbued gains a conviction card from the player's hand or ash
    /// heap, `library` too (shuffled after), or none (rulebook "Appendix:
    /// Imbued Rules" 2: as it enters play with none; Erick, Leaf, Abjure).
    GainConviction {
        seat: SeatId,
        minion: crate::ids::MinionId,
        library: bool,
    },
    /// Antonio d'Erlette: a mage ally he could play, from library or hand,
    /// put in play paid (Pass: none); the library shuffled.
    AllySearch {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Wider View: an uncontrolled crypt card removed, its counters to the
    /// bank; removed cards are public (LSJ 20080314).
    RemoveUncontrolled { seat: SeatId },
    /// The Rack: a ready vampire of the new controller's, if any.
    ChooseForCard {
        seat: SeatId,
        card: CardId,
        /// Of this clan only (Toreador Grand Ball, Priority Contract).
        clan: Option<&'static str>,
        /// A second choice, other than the first (Toreador Grand Ball).
        second: bool,
    },
    /// Garibaldi-Meucci Museum: the ash heap card requiring an Anarch, then
    /// the hand card it is exchanged for.
    ExchangeAnarch { seat: SeatId, ash: Option<u16> },
    /// "If the attached minion is about to leave the ready region, you can
    /// burn this card to gain n pool" (Priority Contract).
    PoolForCard { seat: SeatId, card: CardId, n: u8 },
    /// "You can lock this card … to gain n pool" (The Black Throne).
    LockForPool { seat: SeatId, card: CardId, n: u8 },
    /// "You can burn those counters to steal that minion" (Cave of Apples).
    StealForCorruption {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// "You can burn 1 blood from this card to make that vampire Anarch"
    /// (Twilight Camp).
    MakeAnarch {
        seat: SeatId,
        card: CardId,
        minion: crate::ids::MinionId,
    },
    /// Ashur Tablets: the ash heap card to hand, then (`to_library`) up to
    /// `left` more shuffled into the library; a pass skips or stops.
    AshurPicks {
        seat: SeatId,
        left: u8,
        to_library: bool,
    },
    /// Gilbert Duane: a non-titled vampire of the player's becomes Anarch.
    BecomesAnarch { seat: SeatId },
    /// Anarch Convert's entry: the vampire made Anarch, then 1 pool or a
    /// crypt draw; before any contest (LSJ 20080526, RTR 20180303), `forced`
    /// beside a copy his Methuselah controls (no voluntary self-contest).
    AnarchConvert {
        seat: SeatId,
        convert: crate::ids::MinionId,
        forced: bool,
        chosen: Option<crate::ids::MinionId>,
    },
    /// `n` blood to an uncontrolled `clan` vampire (Hunting the Beast
    /// [AUS]); no clan and optional with `may` (Flames of Insurrection).
    UncontrolledBlood {
        seat: SeatId,
        clan: Option<&'static str>,
        n: u8,
        may: bool,
    },
    /// Antediluvian Awakening: `BurnPool` pays, `ChooseMinion` burns a
    /// capacity `cap`+ vampire and the card.
    PoolOrVampire {
        seat: SeatId,
        card: CardId,
        pool: u8,
        cap: u8,
    },
    /// Fortschritt Library: a `filter` card searched to hand (`FindCard`) or
    /// none (Pass); the library shuffled. With `haven_on`, a haven found may
    /// then go on that vampire (Goblinism [MYT]: `PutFoundOn`).
    LibraryToHand {
        seat: SeatId,
        filter: crate::cards::CardFilter,
        haven_on: Option<crate::ids::MinionId>,
    },
    /// Goblinism [MYT]: the haven `card` just searched to `seat`'s hand may
    /// be put on `minion` instead (`FindCard`), its cost paid, or stay
    /// (Pass).
    PutFoundOn {
        seat: SeatId,
        minion: crate::ids::MinionId,
        card: u16,
    },
    /// Leandro: `BurnPool` burns, `Pass` loses the next influence phase's
    /// transfers; no default (LSJ 20021210-1).
    PoolOrNoTransfers { seat: SeatId, pool: u8 },
    /// Bleeding the Vine: `ResolveRider` locks it (if unlocked), `BurnPool`
    /// pays.
    LockOrPay {
        seat: SeatId,
        card: CardId,
        pool: u8,
    },
    /// Disciple of Lazarus: a `clan` crypt card to the uncontrolled region
    /// (`AshVampire`, a CRYPT index) or none (Pass).
    CryptSearch { seat: SeatId, clan: &'static str },
    /// Solomon Batanea: the slot of `of`'s uncontrolled region that gives 1
    /// blood to `of`'s pool (`Choice::Slot`, `of`'s region).
    PreySlotToPool { seat: SeatId, of: SeatId },
    /// Erlik's entry: up to `left` more blood from `from` to uncontrolled
    /// `clan` vampires (`Slot`); contests staged after (LSJ 20080526).
    DistributeFrom {
        seat: SeatId,
        from: MinionId,
        clan: &'static str,
        left: u8,
    },
    /// Blood (and with `equipment`, equipment) moved among a circle's Blood
    /// Brothers (Jerry, Redistribution): `from`, `to`, then the amount; each
    /// move a gain of its own (rules-qa 68.10), one per pair (`paired`).
    CircleBlood {
        seat: SeatId,
        circle: &'static str,
        from: Option<MinionId>,
        to: Option<MinionId>,
        equipment: bool,
        moving: Option<CardId>,
        paired: [Option<(MinionId, MinionId)>; 12],
    },
    /// Gird Minions: pool to vampires, `to` then the amount (`SetX`) up to
    /// capacity (rulebook); Pass ends.
    PoolToVampires {
        seat: SeatId,
        to: Option<crate::ids::MinionId>,
    },
    /// Merging an advanced vampire: its base searched in crypt and ash heap
    /// (`AshVampire`) or uncontrolled region (`Slot`); the crypt shuffled.
    BaseSearch {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// New Blood's entry: his circle (`ChooseCircle`), or one of his own
    /// (Pass).
    ChooseCircle { seat: SeatId, minion: MinionId },
    /// An ally matching `of` from the ash heap to play (Split the Veil);
    /// `named` at declaration, put in play at resolution (Hel-Blá).
    AllyFromAsh {
        seat: SeatId,
        of: &'static [crate::cards::CardFilter],
        named: bool,
    },
    /// Victor Donaldson: `card`, canceled as `seat` played it, may move
    /// from their ash heap to their library, shuffled (`ResolvePrinted`).
    MayRecycleCanceled {
        seat: SeatId,
        minion: MinionId,
        card: u16,
    },
    /// Malgorzata: the vampire of `seat`'s uncontrolled region to remove,
    /// another than the action's target (`Choice::Slot`).
    NameUncontrolled { seat: SeatId, target: u8 },
    /// Lázár Dobrescu, named at announcement (rules-qa default readings):
    /// `from` in `of`'s uncontrolled region, then `seat`'s receiver.
    NameSlotPair {
        seat: SeatId,
        of: SeatId,
        from: Option<u8>,
    },
    /// The Great Beast's three Disciplines, one at a time (`Choice::Slot`
    /// into `GREAT_BEAST_DISCIPLINES`).
    BeastDisciplines { seat: SeatId, minion: MinionId },
    /// The Erciyes Fragments: the library card of `seat`'s prey's ash heap
    /// to move onto `card`, face down (`Choice::FindCard`).
    StorePreyAsh { seat: SeatId, card: CardId },
    /// The Sargon Fragment: the ash heap card matching `of` the action
    /// names (Drozodny: a political action card).
    NameOwnAshCard {
        seat: SeatId,
        of: crate::cards::CardFilter,
    },
    /// Hand of Conrad: the vampire of `seat`'s ash heap the action declared
    /// names (`Choice::AshVampire`).
    NameAshVampire { seat: SeatId },
    /// Giovanni del Georgio: the card two copies of which in `of`'s ash
    /// heap the action declared names (`Terms::PreyAshPair`).
    NameAshPair { seat: SeatId, of: SeatId },
    /// Fever Pitch [FOR]/[PRE]'s optional placement: `ChooseMinion` or pass
    /// (owner, 2026-10-08: rules-qa 53.5).
    MayPutOn {
        seat: SeatId,
        held: crate::combat::Held,
    },
    /// "You can add 1 counter to this location" (Cursed Abattoir):
    /// `ResolveRider` it, or pass — never worse to add (rules-qa 55.6).
    MayAddCounter { seat: SeatId, card: CardId },
    /// Light Intensifying Goggles: `seat` may move `card`, its bearer's aim
    /// card just used (canceled too), from their ash heap onto `store`
    /// (`ResolveRider`), or pass (owner, 2026-10-18: asked each time).
    MayStoreAim {
        seat: SeatId,
        store: CardId,
        card: u16,
    },
    /// The Path of Lilith: `seat` may lock `card` to put a torture counter
    /// on `minion` (`ResolveRider`), or pass.
    MayTorture {
        seat: SeatId,
        card: CardId,
        minion: MinionId,
    },
    /// Matasuntha's optional "gain n pool": `ResolvePrinted` or Pass.
    MayGainPool {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// A ready titled vampire of `of`'s left the ready region: `seat` may
    /// play a card for it (Leadership Vacuum), or pass.
    ReadyTitledLeft { seat: SeatId, of: SeatId },
    /// Legion's action announced: the master Discipline card named, from
    /// hand or ash heap (`FindCard`).
    NameMasterDiscipline { seat: SeatId },
    /// Filchware's Pawn Shop's action announced: `minion`'s equipment named
    /// in an ash heap, the heap first (`ChooseSeat`), then the card
    /// (`FindCard`).
    NameAshEquipment {
        seat: SeatId,
        minion: crate::ids::MinionId,
        from: Option<SeatId>,
    },
    /// Victor Pelletier entering play: `SetX(n)` pays n pool for the title
    /// offered at that price, or Pass; his contests staged after.
    BuyTitle {
        seat: SeatId,
        minion: crate::ids::MinionId,
        offers: &'static [(u8, crate::crypt::Title)],
    },
    /// Sascha Vykos's optional "+n hand size until the end of the turn",
    /// once each turn: `ResolvePrinted` or Pass (a pass spends nothing).
    MayGainHandSize {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// Graverobbing [DOM]: `payer` burns `n` blood to ready the stolen
    /// vampire (`ChooseMinion`), or pass.
    MoveForBlood {
        seat: SeatId,
        payer: crate::ids::MinionId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// Blackhorse Tanner: `card`, just drawn, to the library bottom and a
    /// new draw (`DrawLibrary`), or pass.
    Redraw {
        seat: SeatId,
        minion: crate::ids::MinionId,
        card: u16,
    },
}

/// What a minion chosen at resolution is for (`Interrupt::MinionOf`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinionPurpose {
    /// "You can lock a minion": optional; a locked one is a legal choice.
    Lock,
    /// "Put 1 corruption counter on a minion": mandatory.
    Corrupt,
    /// "Lock up to two vampires they control": optional, vampires only.
    LockVampire,
    /// "You can unlock a ready Anarch" (Powerbase: Los Angeles): optional,
    /// the user's own.
    UnlockAnarch,
    /// A ready vampire of the clan the user controls, with blood, burns 1
    /// (Chantry's cost): mandatory.
    BurnBloodOfClan(&'static str),
    /// A stolen location on a minion moves onto one of the new controller's
    /// (Dominate Kine [DOM]; RTR 19960112): mandatory.
    HostCard(CardId),
    /// One of the oldest vampires of the clan (Brujah Debate): mandatory,
    /// locked or not (LSJ 20010508).
    LockOldest(&'static str),
    /// A vampire of a clan the user controls 3 or more of, burned
    /// (Recalled to the Founder): mandatory.
    BurnOfCrowdedClan,
    /// A ready minion of the chooser's, which takes n unpreventable damage
    /// (Gnaeus Aemilius Augustinus): mandatory.
    Damage(u8),
    /// Another ready imbued of the chooser's than `except`, which takes n
    /// unpreventable damage (Béatrice "Oracle171" Tremblay): mandatory.
    DamageImbued { n: u8, except: crate::ids::MinionId },
    /// Gurchon Hall: one of the two oldest ready vampires gains 1 blood,
    /// mandatory; a tie is the chooser's (rules-qa 69.1).
    OldestHuntingGround(Option<crate::ids::MinionId>),
}

impl Interrupt {
    fn seat(self) -> SeatId {
        match self {
            Interrupt::GiveBlood { seat, .. }
            | Interrupt::DiablerieEquipment { seat, .. }
            | Interrupt::MoveBetween { seat, .. }
            | Interrupt::PayOrBurn { seat, .. }
            | Interrupt::LockOrDiscard { seat, .. }
            | Interrupt::BloodOrBurned { seat, .. }
            | Interrupt::BloodOrCamarilla { seat, .. }
            | Interrupt::Replacement { seat, .. }
            | Interrupt::DrawUp { seat }
            | Interrupt::Redraw { seat, .. }
            | Interrupt::Draws { seat, .. }
            | Interrupt::Bottom { seat, .. }
            | Interrupt::StoreSearch { seat, .. }
            | Interrupt::EntryBurn { seat, .. }
            | Interrupt::EntryRemoveOrBurn { seat, .. }
            | Interrupt::BlockToll { seat, .. }
            | Interrupt::CancelAttempt { seat, .. }
            | Interrupt::MarginPool { seat, .. }
            | Interrupt::MarginBlood { seat, .. }
            | Interrupt::MinionOf { seat, .. }
            | Interrupt::CorruptionOwner { seat, .. }
            | Interrupt::DiscardChosen { seat, .. }
            | Interrupt::Reorder { seat, .. }
            | Interrupt::AvoidDamage { seat, .. }
            | Interrupt::DiscardFromHandOf { seat, .. }
            | Interrupt::RemoveFromAsh { seat, .. }
            | Interrupt::DisciplineSearch { seat, .. }
            | Interrupt::AllySearch { seat, .. }
            | Interrupt::GainConviction { seat, .. }
            | Interrupt::BurnConvictions { seat, .. }
            | Interrupt::AvengeImbued { seat, .. }
            | Interrupt::NameConviction { seat, .. }
            | Interrupt::PowerSearch { seat, .. }
            | Interrupt::MayCancelCombat { seat, .. }
            | Interrupt::PutInPlaySearch { seat, .. }
            | Interrupt::MoveCardToOldest { seat, .. }
            | Interrupt::MayPreventForBlood { seat, .. }
            | Interrupt::ShortfallPicks { seat, .. }
            | Interrupt::MayUnlock { seat, .. }
            | Interrupt::GiveCardTo { seat, .. }
            | Interrupt::StealUpTo { seat, .. }
            | Interrupt::MoveBloodUpTo { seat, .. }
            | Interrupt::PurgePick { seat, .. }
            | Interrupt::MayDrawExtra { seat }
            | Interrupt::PursuitDraw { seat, .. }
            | Interrupt::PursuitDiscard { seat, .. }
            | Interrupt::RemoveUncontrolled { seat }
            | Interrupt::ChooseForCard { seat, .. }
            | Interrupt::ExchangeAnarch { seat, .. }
            | Interrupt::PoolForCard { seat, .. }
            | Interrupt::LockForPool { seat, .. }
            | Interrupt::MakeAnarch { seat, .. }
            | Interrupt::StealForCorruption { seat, .. }
            | Interrupt::UncontrolledBlood { seat, .. }
            | Interrupt::CryptSearch { seat, .. }
            | Interrupt::LockOrPay { seat, .. }
            | Interrupt::PoolOrVampire { seat, .. }
            | Interrupt::PoolOrNoTransfers { seat, .. }
            | Interrupt::LibraryToHand { seat, .. }
            | Interrupt::PutFoundOn { seat, .. }
            | Interrupt::PreySlotToPool { seat, .. }
            | Interrupt::DistributeFrom { seat, .. }
            | Interrupt::ChooseCircle { seat, .. }
            | Interrupt::CircleBlood { seat, .. }
            | Interrupt::PoolToVampires { seat, .. }
            | Interrupt::BaseSearch { seat, .. }
            | Interrupt::BecomesAnarch { seat }
            | Interrupt::AnarchConvert { seat, .. }
            | Interrupt::AshurPicks { seat, .. }
            | Interrupt::AllyFromAsh { seat, .. }
            | Interrupt::NameAshPair { seat, .. }
            | Interrupt::NameUncontrolled { seat, .. }
            | Interrupt::NameSlotPair { seat, .. }
            | Interrupt::NameOwnAshCard { seat, .. }
            | Interrupt::NameAshVampire { seat }
            | Interrupt::StorePreyAsh { seat, .. }
            | Interrupt::StoreOwnAsh { seat, .. }
            | Interrupt::StoreOwnHand { seat, .. }
            | Interrupt::StoredToLibraryTop { seat, .. }
            | Interrupt::BeastDisciplines { seat, .. }
            | Interrupt::MayRecycleCanceled { seat, .. }
            | Interrupt::MayPutOn { seat, .. }
            | Interrupt::MayAddCounter { seat, .. }
            | Interrupt::MayStoreAim { seat, .. }
            | Interrupt::MayTorture { seat, .. }
            | Interrupt::MayGainPool { seat, .. }
            | Interrupt::MayGainHandSize { seat, .. }
            | Interrupt::BuyTitle { seat, .. }
            | Interrupt::NameAshEquipment { seat, .. }
            | Interrupt::NameMasterDiscipline { seat }
            | Interrupt::ReadyTitledLeft { seat, .. }
            | Interrupt::MoveForBlood { seat, .. }
            | Interrupt::MoveCardToTitled { seat, .. }
            | Interrupt::MoveOffBurnedBearer { seat, .. }
            | Interrupt::SecretNumber { seat, .. }
            | Interrupt::SearchEquipFor { seat, .. } => seat,
        }
    }
}

impl Engine {
    /// The cards `s` could draw instead of their library's top card, by
    /// card on each card in play (rules-qa 9.4); none from an empty library
    /// (rules-qa 8.3), none from a contested card (spec §1.13.3).
    pub(crate) fn draw_alternatives(&self, s: SeatId) -> Vec<Choice> {
        let st = &self.state;
        let mut out = Vec::new();
        if st.seats[s.index()].library.is_empty() {
            return out;
        }
        let on = |c: &CardInPlay| !c.stored.is_empty();
        let at = HookAt {
            seat: Some(s),
            ..HookAt::on(&on)
        };
        let mut cards: Vec<CardId> = self
            .fired_riders(Hook::WouldDraw, &at)
            .into_iter()
            .map(|(id, _)| id)
            .filter(|&id| st.card_controller(id) == s)
            .collect();
        cards.dedup();
        for id in cards {
            let stored = &st.cards[id.index()].stored;
            out.extend(
                distinct(stored, |_| true)
                    .into_iter()
                    .map(|stored| Choice::DrawStored { card: id.0, stored }),
            );
        }
        out
    }

    /// The card whose `WouldReplace` rider takes a replacement of `s`
    /// instead (Visit from the Capuchin), `except` the card being replaced
    /// ("a card other than this card").
    pub(crate) fn replacement_rider(&self, s: SeatId, except: Option<CardId>) -> Option<CardId> {
        self.fired_riders(Hook::WouldReplace, &HookAt::of(s))
            .into_iter()
            .map(|(id, _)| id)
            .find(|&id| Some(id) != except && self.state.card_controller(id) == s)
    }

    /// A replacement taken by that rider instead: a counter burned, the
    /// card burned with none left.
    pub(crate) fn apply_replacement_rider(&mut self, id: CardId) {
        let def = cards::lookup(self.state.cards[id.index()].card).expect("definition");
        for r in def.riders.iter().filter(|r| r.hook == Hook::WouldReplace) {
            for &e in r.effects {
                self.apply_rider_effect(id, e, Hook::WouldReplace);
            }
        }
    }

    /// Whether `s`'s draws are choices now: a draw alternative applies, or
    /// choices of theirs are still queued (their draws keep their order).
    pub(crate) fn draws_interrupt(&self, s: SeatId) -> bool {
        self.interrupts.iter().any(|i| i.seat() == s) || !self.draw_alternatives(s).is_empty()
    }

    /// Move the top `n` cards of the card's controller's library onto it,
    /// as many as there are (rules-qa 11.2).
    pub(crate) fn store_top_cards(&mut self, id: CardId, n: u8) {
        let s = self.state.card_controller(id);
        for _ in 0..n {
            let Some((top, copy)) = self.state.seats[s.index()].library.take_top() else {
                break;
            };
            self.state.cards[id.index()].stored.put(top, copy);
        }
    }

    fn draw_top(&mut self, s: SeatId) {
        let seat = &mut self.state.seats[s.index()];
        if let Some((c, copy)) = seat.library.take_top() {
            seat.hand.put(c, copy);
            self.offer_redraw(s, c);
        }
    }

    /// `s`'s vampire with an unspent once-each-turn redraw (Blackhorse
    /// Tanner): in torpor too, each Methuselah's turn (ANK 20180719-4).
    fn redrawer(&self, s: SeatId) -> Option<MinionId> {
        let st = &self.state;
        st.minions_of(s).find(|&m| {
            st.printed_mod(
                m,
                cards::Stat::RedrawOncePerTurn,
                crate::state::PrintedCtx::default(),
            ) > 0
                && !st.turn_uses.contains(&crate::state::once_redraw_key(m))
        })
    }

    /// `card` was just drawn from `s`'s library: offer its redraw, if any
    /// — with no other card in the library there is nothing new to draw.
    pub(crate) fn offer_redraw(&mut self, s: SeatId, card: u16) {
        if self.state.seats[s.index()].library.is_empty() {
            return;
        }
        if let Some(minion) = self.redrawer(s) {
            self.interrupts.push(Interrupt::Redraw {
                seat: s,
                minion,
                card,
            });
        }
    }

    /// A card drawn from a card in play instead; the card's draw rider
    /// follows ("if this location has no cards on it, burn it").
    fn take_stored(&mut self, s: SeatId, card: u32, stored: u16) {
        let id = CardId(card);
        let on = &mut self.state.cards[id.index()].stored;
        let k = on.iter().rposition(|&c| c == stored).expect("stored card");
        let (_, copy) = on.take(k);
        self.state.seats[s.index()].hand.put(stored, copy);
        let def = cards::lookup(self.state.cards[id.index()].card).expect("definition");
        for r in def.riders.iter().filter(|r| r.hook == Hook::WouldDraw) {
            for &e in r.effects {
                self.apply_rider_effect(id, e, Hook::WouldDraw);
            }
        }
    }

    /// The end of Black Market Cache's search: the library is shuffled;
    /// holding no card, the location is burned at once (rules-qa 11.3),
    /// else a unique copy enters contest now that its cards are on it
    /// (spec §1.13.2).
    fn finish_store_search(&mut self, s: SeatId, card: CardId) {
        self.state.seats[s.index()].library.shuffle(&mut self.rng);
        if self.state.cards[card.index()].stored.is_empty() {
            self.burn_card_in_play(card);
        } else {
            self.enter_library_contest(card);
        }
    }

    /// Settle the queued interrupts needing no decision, front first, until
    /// one needs a decision or none is left: those of an ousted Methuselah
    /// are dropped; with no draw alternative left, draws and replacements
    /// happen as usual.
    pub(crate) fn settle_interrupts(&mut self) {
        let outer = self.cause;
        self.settle_interrupts_now(outer);
        self.end_cause(outer);
    }

    fn settle_interrupts_now(&mut self, outer: Option<Cause>) {
        while let Some(&front) = self.interrupts.first() {
            self.cause_front(outer);
            let s = front.seat();
            if self.state.seats[s.index()].ousted {
                self.interrupts.remove(0);
                // Charnas the Imp kept from its employer's burn: no one to
                // move it, it burns.
                if let Interrupt::MoveOffBurnedBearer { card, .. } = front {
                    if self.state.cards[card.index()].in_play {
                        self.burn_card_in_play(card);
                    }
                }
                // The diablerie goes on without its choice.
                if let Interrupt::DiablerieEquipment {
                    diablerist,
                    victim,
                    diablerie,
                    ..
                } = front
                {
                    self.taken_from_burns(diablerist, victim, diablerie);
                }
                continue;
            }
            let empty = self.state.seats[s.index()].library.is_empty();
            let choice = !self.draw_alternatives(s).is_empty();
            let attempting = |m: MinionId| {
                self.action
                    .as_ref()
                    .is_some_and(|a| a.state == crate::actions::ActionState::Attempt(m))
            };
            let present = |m: MinionId| self.state.minions[m.index()].present();
            // Waiting on a decision; if not, the arms that settle with an
            // effect do it and go on, the others are dropped below.
            let wait = match front {
                Interrupt::Replacement { except, .. } => {
                    // With an empty library nothing is replaced, so nothing
                    // is replaced instead (rules-qa 8.3).
                    match self.replacement_rider(s, except).filter(|_| !empty) {
                        None => false,
                        Some(_) if choice => true,
                        Some(id) => {
                            self.interrupts.remove(0);
                            self.apply_replacement_rider(id);
                            continue;
                        }
                    }
                }
                Interrupt::DrawUp { .. } => {
                    if empty || self.state.hand_load(s) >= self.state.hand_size(s) {
                        false
                    } else if choice {
                        true
                    } else {
                        self.draw_top(s);
                        continue;
                    }
                }
                Interrupt::Draws { left, drawn, .. } => {
                    if left == 0 || empty {
                        self.interrupts[0] = Interrupt::Bottom {
                            seat: s,
                            left: drawn,
                        };
                    } else if choice {
                        return;
                    } else {
                        self.draw_top(s);
                        self.interrupts[0] = Interrupt::Draws {
                            seat: s,
                            left: left - 1,
                            drawn: drawn + 1,
                        };
                    }
                    continue;
                }
                Interrupt::Bottom { left, .. } => {
                    left > 0 && !self.state.seats[s.index()].hand.is_empty()
                }
                Interrupt::StoreSearch { card, left, .. } => {
                    if left > 0 {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.finish_store_search(s, card);
                    continue;
                }
                Interrupt::EntryRemoveOrBurn { ally, .. } => {
                    self.state.minions[ally.index()].in_play
                }
                // Nothing to cancel once the attempt is over.
                Interrupt::CancelAttempt { minion, .. } => attempting(minion),
                Interrupt::BlockToll { minion, n, .. } => {
                    let v = &self.state.minions[minion.index()];
                    let affordable = !v.is_ally() && v.blood >= n;
                    if attempting(minion) && !affordable {
                        self.interrupts.remove(0);
                        self.resign_attempt();
                        continue;
                    }
                    attempting(minion)
                }
                Interrupt::MarginPool { .. } => {
                    let range = self.margin_pool_range(front);
                    if range.len() > 1 {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.margin_to_pool(front, range[0]);
                    continue;
                }
                Interrupt::MarginBlood { left, .. } => {
                    let open = self.margin_recipients(front);
                    if left > 0 && open.len() > left as usize {
                        return;
                    }
                    self.interrupts.remove(0);
                    for m in open.into_iter().take(left as usize) {
                        self.state.gain_blood(m, 1);
                    }
                    continue;
                }
                Interrupt::DiscardFromHandOf { of, .. } => {
                    let open = |s: SeatId| {
                        !self.state.seats[s.index()].ousted
                            && !self.state.seats[s.index()].hand.is_empty()
                    };
                    match of {
                        Some(of) => open(of),
                        None => (0..self.state.seats.len()).any(|i| open(SeatId(i as u8))),
                    }
                }
                Interrupt::AvoidDamage { minion, .. } => present(minion),
                // One card or one kind left to place: no choice.
                Interrupt::Reorder {
                    seat,
                    total,
                    placed,
                } => {
                    let lib = &self.state.seats[seat.index()].library;
                    let n = lib.len();
                    let unplaced = &lib[n - total as usize..n - placed as usize];
                    let kinds = distinct(unplaced, |_| true).len();
                    if kinds > 1 {
                        return;
                    }
                    self.interrupts.remove(0);
                    continue;
                }
                Interrupt::DiscardChosen {
                    seat,
                    left,
                    done,
                    of,
                } => {
                    let fits = |c: u16| of.is_none_or(|f| f.matches(c));
                    let held = self.state.seats[seat.index()]
                        .hand
                        .iter()
                        .filter(|&&c| fits(c))
                        .count();
                    if left > 0 && held > left as usize {
                        return;
                    }
                    // As many as it holds, or fewer: all of them — then
                    // the replacements.
                    self.interrupts.remove(0);
                    let st = &mut self.state.seats[seat.index()];
                    let k = if left > 0 { held } else { 0 };
                    let cards = st.hand.take_unless(|c| !(fits(c) && k > 0));
                    st.ash.put_all(cards);
                    self.replace(seat, done as usize + k, None);
                    continue;
                }
                Interrupt::RemoveFromAsh { of, left, pool, .. } => {
                    // Library cards and burned vampires alike (rules-qa
                    // 42.7).
                    let st = &self.state.seats[of.index()];
                    let kinds =
                        distinct(&st.ash, |_| true).len() + distinct(&st.ash_crypt, |_| true).len();
                    if left > 0 && !st.ousted && kinds > 1 {
                        return;
                    }
                    // One kind of card left, or none: no choice.
                    self.interrupts.remove(0);
                    let seat = &mut self.state.seats[of.index()];
                    let n = (left as usize).min(seat.ash.len());
                    let removed = seat.ash.take_bottom(n);
                    seat.removed.put_all(removed);
                    let k = (left as usize - n).min(seat.ash_crypt.len());
                    let removed = seat.ash_crypt.take_bottom(k);
                    seat.removed_crypt.put_all(removed);
                    let mut loss = vec![0; self.state.seats.len()];
                    loss[of.index()] = pool as i32;
                    self.burn_pool_batch(&loss);
                    continue;
                }
                Interrupt::StealForCorruption { minion, .. } => present(minion),
                Interrupt::MakeAnarch { card, minion, .. } => {
                    let c = &self.state.cards[card.index()];
                    c.present() && c.counters > 0 && present(minion)
                }
                Interrupt::LockForPool { card, .. } => {
                    let c = &self.state.cards[card.index()];
                    c.present() && !c.locked
                }
                Interrupt::PoolForCard { .. } => true,
                Interrupt::ExchangeAnarch { seat, .. } => {
                    !self.state.seats[seat.index()].hand.is_empty()
                }
                Interrupt::ChooseForCard { card, .. } => {
                    let ready = self.choosable(front);
                    if ready.len() > 1 && self.state.cards[card.index()].present() {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.set_chosen(front, ready.first().copied());
                    continue;
                }
                Interrupt::RemoveUncontrolled { seat } => {
                    match self.state.seats[seat.index()].uncontrolled.len() {
                        0 => false,
                        1 => {
                            self.interrupts.remove(0);
                            self.remove_uncontrolled(seat, 0);
                            continue;
                        }
                        _ => true,
                    }
                }
                // What is left to steal, re-read: the minions still there.
                Interrupt::StealUpTo { from, to, .. } => present(from) && present(to),
                Interrupt::MoveBloodUpTo { from, to, .. } => {
                    present(from) && present(to) && self.state.minions[from.index()].blood > 0
                }
                Interrupt::MayDrawExtra { seat } => {
                    !self.state.seats[seat.index()].library.is_empty()
                }
                // "Replace and draw extra cards individually …, discarding
                // each time X cards (where X is the number of extra cards
                // you drew)" (ANK 20180925-1).
                Interrupt::PursuitDraw { seat, extra } => {
                    self.draw_top(seat);
                    let mut drawn = 0;
                    for _ in 0..extra {
                        let st = &mut self.state.seats[seat.index()];
                        match st.library.take_top() {
                            Some((c, copy)) => st.hand.put(c, copy),
                            None => break,
                        }
                        drawn += 1;
                    }
                    self.interrupts[0] = Interrupt::PursuitDiscard { seat, left: drawn };
                    continue;
                }
                Interrupt::PursuitDiscard { seat, left } => {
                    left > 0 && !self.state.seats[seat.index()].hand.is_empty()
                }
                Interrupt::PurgePick { of, host, .. } => {
                    if self.purge_candidates(of, host).is_empty() {
                        self.interrupts.remove(0);
                        self.finish_purge_if_done(host);
                        continue;
                    }
                    true
                }
                Interrupt::MayCancelCombat { minion, .. } => {
                    if self
                        .combat
                        .as_ref()
                        .is_some_and(|c| c.side_of_minion(minion).is_some())
                    {
                        return;
                    }
                    self.interrupts.remove(0);
                    continue;
                }
                Interrupt::PowerSearch { minion, virtue, .. } => {
                    if present(minion) && self.power_sources(s, minion, virtue).len() > 1 {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.state.seats[s.index()].library.shuffle(&mut self.rng);
                    continue;
                }
                Interrupt::NameConviction { minion, .. } => {
                    let held = self.state.convictions(minion);
                    let first = held.first().map(|c| self.state.cards[c.index()].card);
                    if held
                        .iter()
                        .all(|c| Some(self.state.cards[c.index()].card) == first)
                    {
                        self.interrupts.remove(0);
                        if let Some(a) = self.action.as_mut() {
                            a.conviction = held.first().copied();
                        }
                        continue;
                    }
                    return;
                }
                Interrupt::AvengeImbued { imbued, acting, .. } => {
                    if self.state.minions[imbued.index()].ready()
                        && present(acting)
                        && !self.state.convictions(imbued).is_empty()
                    {
                        return;
                    }
                    self.interrupts.remove(0);
                    continue;
                }
                Interrupt::BurnConvictions { minion, n, .. } => {
                    let held = self.state.convictions(minion);
                    if n == 0 || held.is_empty() {
                        self.interrupts.remove(0);
                        continue;
                    }
                    // One name: no choice.
                    let first = self.state.cards[held[0].index()].card;
                    if held
                        .iter()
                        .all(|c| self.state.cards[c.index()].card == first)
                    {
                        self.burn_card_in_play(held[0]);
                        if let Interrupt::BurnConvictions { n, .. } = &mut self.interrupts[0] {
                            *n -= 1;
                        }
                        continue;
                    }
                    return;
                }
                Interrupt::GainConviction {
                    minion, library, ..
                } => {
                    if present(minion) && self.conviction_sources(s, library).len() > 1 {
                        return;
                    }
                    self.interrupts.remove(0);
                    if library {
                        self.state.seats[s.index()].library.shuffle(&mut self.rng);
                    }
                    continue;
                }
                Interrupt::AllySearch { minion, .. } => {
                    if present(minion) && !self.mage_allies(s, minion).is_empty() {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.state.seats[s.index()].library.shuffle(&mut self.rng);
                    continue;
                }
                Interrupt::ShortfallPicks { from, to, .. } => {
                    if self.state.minions[from.index()].blood > 0 && to.iter().any(Option::is_some)
                    {
                        return;
                    }
                    self.interrupts.remove(0);
                    continue;
                }
                Interrupt::MoveCardToOldest { card, .. } => {
                    let to = self.oldest_for(card);
                    if to.len() > 1 && self.state.cards[card.index()].present() {
                        return;
                    }
                    self.interrupts.remove(0);
                    if let Some(&m) = to.first() {
                        self.move_title_card(card, m);
                    }
                    continue;
                }
                Interrupt::PutInPlaySearch {
                    minion, x, vampire, ..
                } => {
                    if present(minion) && !self.put_in_play_finds(s, minion, x, vampire).is_empty()
                    {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.shuffle_searched(s, vampire);
                    continue;
                }
                Interrupt::DisciplineSearch { minion, .. } => {
                    if present(minion) && !self.discipline_masters(s, minion).is_empty() {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.state.seats[s.index()].library.shuffle(&mut self.rng);
                    continue;
                }
                Interrupt::CorruptionOwner { seat, minion } => {
                    if self.state.minions[minion.index()].corruption.len() > 1 {
                        return;
                    }
                    self.interrupts.remove(0);
                    if let [(owner, _)] = self.state.minions[minion.index()].corruption[..] {
                        self.state.burn_corruption(minion, owner, 1, seat);
                    }
                    continue;
                }
                Interrupt::MinionOf { of, purpose, .. } => {
                    !self.minion_of_options(of, purpose).is_empty()
                }
                Interrupt::AllyFromAsh { seat, of, .. } => !self.ash_allies(seat, of).is_empty(),
                Interrupt::NameAshVampire { seat } => {
                    self.action.is_some() && !self.state.seats[seat.index()].ash_crypt.is_empty()
                }
                Interrupt::BeastDisciplines { minion, .. } => {
                    self.state.minions[minion.index()].in_play
                }
                Interrupt::StoreOwnAsh { seat, card } => {
                    self.state.cards[card.index()].present()
                        && !self.state.seats[seat.index()].ash.is_empty()
                }
                Interrupt::StoreOwnHand { seat, card } => {
                    self.state.cards[card.index()].present()
                        && !self.state.seats[seat.index()].hand.is_empty()
                }
                Interrupt::StoredToLibraryTop { card, .. } => {
                    self.state.cards[card.index()].present()
                        && !self.state.cards[card.index()].stored.is_empty()
                }
                Interrupt::StorePreyAsh { seat, card } => {
                    self.state.cards[card.index()].present()
                        && !self.state.seats[self.state.prey(seat).index()]
                            .ash
                            .is_empty()
                }
                Interrupt::NameAshPair { .. }
                | Interrupt::NameAshEquipment { .. }
                | Interrupt::NameMasterDiscipline { .. }
                | Interrupt::NameUncontrolled { .. }
                | Interrupt::NameSlotPair { .. }
                | Interrupt::NameOwnAshCard { .. } => self.action.is_some(),
                Interrupt::MayRecycleCanceled { seat, card, .. } => {
                    self.state.seats[seat.index()].ash.contains(&card)
                }
                Interrupt::MayPutOn { held, .. } => self.may_put_held(held),
                Interrupt::MayAddCounter { card, .. } => self.state.cards[card.index()].present(),
                Interrupt::MayStoreAim { seat, store, card } => {
                    self.state.cards[store.index()].present()
                        && self.state.seats[seat.index()].ash.contains(&card)
                }
                Interrupt::MayTorture { card, minion, .. } => {
                    let c = &self.state.cards[card.index()];
                    c.present() && !c.locked && present(minion)
                }
                Interrupt::MayGainPool { seat, .. } => !self.state.seats[seat.index()].ousted,
                Interrupt::ReadyTitledLeft { seat, of } => {
                    if !self.state.seats[seat.index()].ousted
                        && !self.titled_left_plays(seat, of).is_empty()
                    {
                        true
                    } else {
                        self.interrupts.remove(0);
                        continue;
                    }
                }
                Interrupt::BuyTitle { seat, minion, .. } => {
                    if self.state.minions[minion.index()].present()
                        && !self.state.seats[seat.index()].ousted
                    {
                        true
                    } else {
                        self.interrupts.remove(0);
                        if self.state.minions[minion.index()].present() {
                            self.stage_contests(minion);
                        }
                        continue;
                    }
                }
                Interrupt::MayGainHandSize { seat, minion, .. } => {
                    !self.state.seats[seat.index()].ousted
                        && !self
                            .state
                            .turn_uses
                            .contains(&crate::state::once_hand_size_key(minion))
                }
                Interrupt::MayUnlock { minion, .. } => {
                    present(minion) && self.state.minions[minion.index()].locked
                }
                Interrupt::GiveCardTo { card, .. } => self.state.cards[card.index()].present(),
                Interrupt::MayPreventForBlood { minion, n, .. } => {
                    present(minion)
                        && self.state.minions[minion.index()].blood >= n
                        && self
                            .combat
                            .as_ref()
                            .is_some_and(|c| c.side_of_minion(minion).is_some())
                }
                // Unspent still, the card still in hand, another to draw.
                Interrupt::Redraw { seat, minion, card } => {
                    self.redrawer(seat) == Some(minion)
                        && self.state.seats[seat.index()].hand.contains(&card)
                        && !self.state.seats[seat.index()].library.is_empty()
                }
                Interrupt::UncontrolledBlood { seat, clan, .. } => {
                    !self.uncontrolled_of(seat, clan).is_empty()
                }
                Interrupt::LockOrPay { card, .. } => self.state.cards[card.index()].present(),
                // The card gone before its effect resolves: nothing is due
                // (owner, rules-qa 70.9).
                Interrupt::PoolOrVampire { seat, card, .. } => {
                    !self.state.seats[seat.index()].ousted
                        && self.state.cards[card.index()].present()
                }
                Interrupt::PoolOrNoTransfers { seat, .. } => !self.state.seats[seat.index()].ousted,
                Interrupt::LibraryToHand { seat, .. } => !self.state.seats[seat.index()].ousted,
                Interrupt::PutFoundOn { seat, minion, card } => {
                    self.may_put_found_on(seat, minion, card)
                }
                Interrupt::CryptSearch { seat, clan } => {
                    if self.crypt_of_clan(seat, clan).is_empty() {
                        self.interrupts.remove(0);
                        self.state.seats[seat.index()].crypt.shuffle(&mut self.rng);
                        continue;
                    }
                    true
                }
                Interrupt::ChooseCircle { minion, .. } => {
                    self.state.minions[minion.index()].present()
                }
                Interrupt::CircleBlood { .. } => true,
                Interrupt::PoolToVampires { seat, .. } => !self.state.seats[seat.index()].ousted,
                Interrupt::BaseSearch { minion, .. } => {
                    let v = &self.state.minions[minion.index()];
                    v.present() && v.merged.is_none()
                }
                Interrupt::PreySlotToPool { of, .. } => self.state.seats[of.index()]
                    .uncontrolled
                    .iter()
                    .any(|u| u.counters > 0),
                Interrupt::DistributeFrom {
                    seat,
                    from,
                    clan,
                    left,
                } => {
                    let v = &self.state.minions[from.index()];
                    if v.present()
                        && v.blood > 0
                        && left > 0
                        && !self.uncontrolled_of_clan(seat, clan).is_empty()
                    {
                        true
                    } else {
                        self.interrupts.remove(0);
                        if self.state.minions[from.index()].present() {
                            self.stage_contests(from);
                        }
                        continue;
                    }
                }
                Interrupt::BecomesAnarch { seat } => !self.untitled_vampires(seat).is_empty(),
                Interrupt::AnarchConvert {
                    seat,
                    convert,
                    chosen: None,
                    ..
                } => {
                    if !self.state.minions[convert.index()].in_play {
                        false
                    } else if self.converts(seat, convert).is_empty() {
                        self.interrupts.remove(0);
                        self.stage_contests(convert);
                        continue;
                    } else {
                        true
                    }
                }
                Interrupt::AnarchConvert { .. } => true,
                // Nothing left to choose: the chosen cards are shuffled in.
                Interrupt::AshurPicks {
                    seat,
                    left,
                    to_library,
                } => {
                    if left > 0 && !self.state.seats[seat.index()].ash.is_empty() {
                        return;
                    }
                    self.interrupts.remove(0);
                    if to_library {
                        self.state.seats[seat.index()]
                            .library
                            .shuffle(&mut self.rng);
                    }
                    continue;
                }
                Interrupt::SecretNumber { .. } => true,
                Interrupt::SearchEquipFor { seat, minion } => {
                    if self.state.minions[minion.index()].present()
                        && !self.alastor_equipment(seat, minion).is_empty()
                    {
                        true
                    } else {
                        self.interrupts.remove(0);
                        self.state.seats[seat.index()]
                            .library
                            .shuffle(&mut self.rng);
                        continue;
                    }
                }
                Interrupt::MoveOffBurnedBearer { card, from, .. } => {
                    if !self.state.cards[card.index()].in_play {
                        false
                    } else if self.other_hosts(from).is_empty() {
                        self.interrupts.remove(0);
                        self.burn_card_in_play(card);
                        continue;
                    } else {
                        true
                    }
                }
                Interrupt::MoveCardToTitled { card, .. } => {
                    if !self.state.cards[card.index()].present() {
                        false
                    } else if self.titled_hosts(card).is_empty() {
                        self.interrupts.remove(0);
                        self.burn_card_in_play(card);
                        continue;
                    } else {
                        true
                    }
                }
                Interrupt::MoveForBlood {
                    payer, minion, n, ..
                } => {
                    let v = &self.state.minions[minion.index()];
                    let p = &self.state.minions[payer.index()];
                    v.present() && v.in_torpor && v.hostage == 0 && p.present() && p.blood >= n
                }
                Interrupt::GiveBlood {
                    seat,
                    from,
                    anarchs,
                } => {
                    let v = &self.state.minions[from.index()];
                    v.present()
                        && v.blood > 0
                        && !self.blood_recipients(seat, from, anarchs).is_empty()
                }
                Interrupt::DiablerieEquipment {
                    diablerist,
                    victim,
                    diablerie,
                    ..
                } => {
                    if self.diablerie_equipment(diablerist, victim).is_empty() {
                        self.interrupts.remove(0);
                        self.taken_from_burns(diablerist, victim, diablerie);
                        continue;
                    }
                    true
                }
                Interrupt::MoveBetween {
                    seat,
                    from,
                    to,
                    blood_done,
                    how,
                } => match to {
                    _ if !self.state.minions[from.index()].ready() => false,
                    None => !self.move_recipients(seat, from, how).is_empty(),
                    Some(to) if !self.state.minions[to.index()].ready() => false,
                    // No blood to move: on to the cards.
                    Some(_) if !blood_done && self.state.minions[from.index()].blood == 0 => {
                        self.interrupts[0] = Interrupt::MoveBetween {
                            seat,
                            from,
                            to,
                            blood_done: true,
                            how,
                        };
                        continue;
                    }
                    Some(_) if !blood_done => true,
                    Some(to) => !self.movable_cards(from, to, how).is_empty(),
                },
                Interrupt::PayOrBurn { card, .. } => self.state.cards[card.index()].present(),
                Interrupt::BloodOrBurned {
                    minion,
                    card: Some(c),
                    ..
                } => {
                    self.state.minions[minion.index()].in_play
                        && self.state.cards[c.index()].present()
                }
                Interrupt::LockOrDiscard { minion, .. }
                | Interrupt::BloodOrBurned { minion, .. } => {
                    self.state.minions[minion.index()].in_play
                }
                Interrupt::BloodOrCamarilla { minion, .. } => {
                    self.state.minions[minion.index()].in_play
                        && self.state.sect_of(minion) == Some(crate::crypt::Sect::Anarch)
                }
                Interrupt::EntryBurn { ally, .. } => {
                    if self.state.minions[ally.index()].in_play
                        && self.entry_burn_options(s, ally).is_empty()
                    {
                        self.interrupts.remove(0);
                        self.burn_minion(ally);
                        continue;
                    }
                    self.state.minions[ally.index()].in_play
                }
            };
            if wait {
                return;
            }
            self.interrupts.remove(0);
        }
    }

    /// The decision of the front interrupt, once settled.
    pub(crate) fn interrupt_request(&self) -> Option<DecisionRequest> {
        let &front = self.interrupts.first()?;
        let s = front.seat();
        let st = &self.state;
        let (window, options) = match front {
            Interrupt::Replacement { except, .. } => {
                let id = self
                    .replacement_rider(s, except)
                    .expect("settled replacement");
                let mut options = vec![Choice::ResolveRider { card: id.0 }];
                options.extend(self.draw_alternatives(s));
                (WindowLabel::Draw, options)
            }
            Interrupt::DrawUp { .. } | Interrupt::Draws { .. } => {
                let mut options = vec![Choice::DrawLibrary];
                options.extend(self.draw_alternatives(s));
                (WindowLabel::Draw, options)
            }
            Interrupt::Bottom { .. } => (
                WindowLabel::BottomCards,
                (0..st.seats[s.index()].hand.len())
                    .filter(|&i| crate::state::first_copy(&st.seats[s.index()].hand, i))
                    .map(|i| Choice::Bottom(i as u8))
                    .collect(),
            ),
            Interrupt::StoreSearch { .. } => {
                // Any non-unique equipment, by card: out of play, it needs
                // no definition to be drawn later.
                let found = distinct(&st.seats[s.index()].library, |c| {
                    CardFilter::NonUniqueEquipment.matches(c)
                });
                let mut options = vec![Choice::Pass];
                options.extend(found.into_iter().map(Choice::FindCard));
                (WindowLabel::LibrarySearch, options)
            }
            Interrupt::EntryBurn { ally, .. } => {
                (WindowLabel::EntryEffect, self.entry_burn_options(s, ally))
            }
            Interrupt::GiveBlood {
                seat,
                from,
                anarchs,
            } => (
                WindowLabel::ResolutionChoice,
                self.blood_recipients(seat, from, anarchs)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect(),
            ),
            Interrupt::DiablerieEquipment {
                diablerist, victim, ..
            } => (WindowLabel::ResolutionChoice, {
                let mut options = vec![Choice::Pass];
                options.extend(
                    self.diablerie_equipment(diablerist, victim)
                        .into_iter()
                        .map(|c| Choice::ChooseCard(c.0)),
                );
                options
            }),
            Interrupt::MoveBetween {
                seat,
                from,
                to,
                blood_done,
                how,
            } => (
                WindowLabel::ResolutionChoice,
                match to {
                    None => self
                        .move_recipients(seat, from, how)
                        .into_iter()
                        .map(Choice::ChooseMinion)
                        .collect(),
                    Some(_) if !blood_done => (0..=st.minions[from.index()].blood)
                        .map(Choice::SetX)
                        .collect(),
                    Some(to) => {
                        let mut options = vec![Choice::Pass];
                        options.extend(
                            self.movable_cards(from, to, how)
                                .into_iter()
                                .map(|c| Choice::ChooseCard(c.0)),
                        );
                        options
                    }
                },
            ),
            // Paying is offered if the pool covers it (down to 0 at most).
            Interrupt::PayOrBurn { pool, .. } => (
                WindowLabel::ResolutionChoice,
                if st.seats[s.index()].pool >= pool as i32 {
                    vec![Choice::Pass, Choice::PayToCancel]
                } else {
                    vec![Choice::Pass]
                },
            ),
            Interrupt::BloodOrCamarilla { minion, .. } => (
                WindowLabel::ResolutionChoice,
                if st.minions[minion.index()].blood >= 1 {
                    vec![Choice::Pass, Choice::BurnBloodToAvoid]
                } else {
                    vec![Choice::Pass]
                },
            ),
            // Burned, or the blood burned while it has enough.
            Interrupt::BloodOrBurned { minion, n, .. } => (
                WindowLabel::ResolutionChoice,
                if st.minions[minion.index()].blood >= n {
                    vec![Choice::Pass, Choice::BurnBloodToAvoid]
                } else {
                    vec![Choice::Pass]
                },
            ),
            // Lock it, or discard a card (one choice per card name).
            Interrupt::LockOrDiscard { .. } => {
                let mut options = vec![Choice::Pass];
                options.extend(self.discard_choices(s));
                (WindowLabel::ResolutionChoice, options)
            }
            // The look: every card, one choice per card name.
            Interrupt::DiscardFromHandOf { of: Some(of), .. } => (
                WindowLabel::ResolutionChoice,
                distinct(&st.seats[of.index()].hand, |_| true)
                    .into_iter()
                    .map(Choice::FindCard)
                    .collect(),
            ),
            // Vaticination: whose hand, each looked at (`peeks`).
            Interrupt::DiscardFromHandOf { of: None, .. } => (
                WindowLabel::ResolutionChoice,
                (0..st.seats.len())
                    .map(|i| SeatId(i as u8))
                    .filter(|s| !st.seats[s.index()].ousted && !st.seats[s.index()].hand.is_empty())
                    .map(Choice::ChooseSeat)
                    .collect(),
            ),
            Interrupt::AvoidDamage { .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::BurnBloodToAvoid],
            ),
            Interrupt::Reorder { total, placed, .. } => {
                let lib = &st.seats[s.index()].library;
                let n = lib.len();
                (
                    WindowLabel::ResolutionChoice,
                    distinct(&lib[n - total as usize..n - placed as usize], |_| true)
                        .into_iter()
                        .map(Choice::FindCard)
                        .collect(),
                )
            }
            Interrupt::DiscardChosen { of, .. } => (
                WindowLabel::ResolutionChoice,
                (0..st.seats[s.index()].hand.len())
                    .filter(|&i| crate::state::first_copy(&st.seats[s.index()].hand, i))
                    .filter(|&i| of.is_none_or(|f| f.matches(st.seats[s.index()].hand[i])))
                    .map(|i| Choice::Discard(i as u8))
                    .collect(),
            ),
            Interrupt::RemoveFromAsh { of, .. } => (
                WindowLabel::ResolutionChoice,
                distinct(&st.seats[of.index()].ash, |_| true)
                    .into_iter()
                    .map(Choice::FindCard)
                    .chain(
                        distinct(&st.seats[of.index()].ash_crypt, |_| true)
                            .into_iter()
                            .map(Choice::AshVampire),
                    )
                    .collect(),
            ),
            Interrupt::MoveCardToTitled { card, .. } => (
                WindowLabel::ResolutionChoice,
                self.titled_hosts(card)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect(),
            ),
            // A library search: the card stays hidden from the others.
            Interrupt::SearchEquipFor { seat, minion } => (
                WindowLabel::LibrarySearch,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.alastor_equipment(seat, minion)
                            .into_iter()
                            .map(Choice::FindCard),
                    )
                    .collect(),
            ),
            Interrupt::SecretNumber { max, .. } => (
                WindowLabel::ResolutionChoice,
                (1..=max).map(Choice::SetX).collect(),
            ),
            // "Can": declining burns it.
            Interrupt::MoveOffBurnedBearer { from, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(self.other_hosts(from).into_iter().map(Choice::ChooseMinion))
                    .collect(),
            ),
            Interrupt::StealForCorruption { minion, .. }
            | Interrupt::MoveForBlood { minion, .. }
            | Interrupt::CancelAttempt { minion, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ChooseMinion(minion)],
            ),
            Interrupt::MakeAnarch { card, .. }
            | Interrupt::LockForPool { card, .. }
            | Interrupt::PoolForCard { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolveRider { card: card.0 }],
            ),
            Interrupt::ExchangeAnarch { ash: None, .. } => (
                WindowLabel::ResolutionChoice,
                distinct(&st.seats[s.index()].ash, |c| {
                    CardFilter::HasLabel(Label::RequiresAnarch).matches(c)
                })
                .into_iter()
                .map(Choice::FindCard)
                .collect(),
            ),
            Interrupt::ExchangeAnarch { ash: Some(_), .. } => {
                (WindowLabel::ResolutionChoice, self.discard_choices(s))
            }
            Interrupt::ChooseForCard { .. } => (
                WindowLabel::ResolutionChoice,
                self.choosable(front)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect(),
            ),
            Interrupt::RemoveUncontrolled { .. } => (
                WindowLabel::ResolutionChoice,
                (0..st.seats[s.index()].uncontrolled.len())
                    .map(|i| Choice::Slot(i as u8))
                    .collect(),
            ),
            Interrupt::DisciplineSearch { minion, .. } => {
                let mut options = vec![Choice::Pass];
                options.extend(self.discipline_masters(s, minion));
                (WindowLabel::LibrarySearch, options)
            }
            Interrupt::MayDrawExtra { .. } => {
                (WindowLabel::Draw, vec![Choice::Pass, Choice::DrawLibrary])
            }
            Interrupt::PursuitDraw { .. } => unreachable!("settled without a decision"),
            Interrupt::PursuitDiscard { .. } => (WindowLabel::DiscardDown, self.discard_choices(s)),
            Interrupt::PurgePick { of, host, .. } => (
                WindowLabel::ResolutionChoice,
                self.purge_candidates(of, host)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect(),
            ),
            Interrupt::MoveBloodUpTo { from, max, .. } => (
                WindowLabel::ResolutionChoice,
                (0..=max.min(st.minions[from.index()].blood))
                    .map(Choice::SetX)
                    .collect(),
            ),
            Interrupt::StealUpTo { from, max, .. } => (
                WindowLabel::ResolutionChoice,
                (1..=max.min(st.minions[from.index()].blood).max(1))
                    .map(Choice::SetX)
                    .collect(),
            ),
            Interrupt::AllySearch { minion, .. } => {
                let mut options = vec![Choice::Pass];
                options.extend(self.mage_allies(s, minion));
                (WindowLabel::LibrarySearch, options)
            }
            Interrupt::MayCancelCombat { .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::CancelCombat],
            ),
            Interrupt::PowerSearch { minion, virtue, .. } => (
                WindowLabel::LibrarySearch,
                self.power_sources(s, minion, virtue),
            ),
            Interrupt::NameConviction { minion, .. } => {
                let options = self.conviction_choices(minion);
                (WindowLabel::ActionTerms, options)
            }
            Interrupt::AvengeImbued { imbued, .. } => {
                let mut options = vec![Choice::Pass];
                options.extend(self.conviction_choices(imbued));
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::BurnConvictions { minion, .. } => {
                let options = self.conviction_choices(minion);
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::GainConviction { library, .. } => (
                if library {
                    WindowLabel::LibrarySearch
                } else {
                    WindowLabel::ResolutionChoice
                },
                self.conviction_sources(s, library),
            ),
            Interrupt::ShortfallPicks { to, .. } => {
                let mut picks: Vec<MinionId> = Vec::new();
                for &t in to.iter().flatten() {
                    if !picks.contains(&t) {
                        picks.push(t);
                    }
                }
                (
                    WindowLabel::ResolutionChoice,
                    picks.into_iter().map(Choice::ChooseMinion).collect(),
                )
            }
            Interrupt::MoveCardToOldest { card, .. } => (
                WindowLabel::ResolutionChoice,
                self.oldest_for(card)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect(),
            ),
            Interrupt::PutInPlaySearch {
                minion, x, vampire, ..
            } => {
                let mut options = vec![Choice::Pass];
                options.extend(self.put_in_play_finds(s, minion, x, vampire));
                (WindowLabel::LibrarySearch, options)
            }
            Interrupt::CorruptionOwner { minion, .. } => (
                WindowLabel::ResolutionChoice,
                st.minions[minion.index()]
                    .corruption
                    .iter()
                    .map(|&(s, _)| Choice::ChooseSeat(s))
                    .collect(),
            ),
            Interrupt::AshurPicks { seat, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        distinct(&st.seats[seat.index()].ash, |_| true)
                            .into_iter()
                            .map(Choice::FindCard),
                    )
                    .collect(),
            ),
            Interrupt::BecomesAnarch { seat } => (
                WindowLabel::ResolutionChoice,
                self.untitled_vampires(seat)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect(),
            ),
            Interrupt::AnarchConvert {
                seat,
                convert,
                forced,
                chosen: None,
            } => (
                WindowLabel::EntryEffect,
                (!forced)
                    .then_some(Choice::Pass)
                    .into_iter()
                    .chain(
                        self.converts(seat, convert)
                            .into_iter()
                            .map(Choice::ChooseMinion),
                    )
                    .collect(),
            ),
            // Pass: gain 1 pool.
            Interrupt::AnarchConvert { .. } => (
                WindowLabel::EntryEffect,
                vec![Choice::Pass, Choice::DrawCrypt],
            ),
            Interrupt::PoolOrVampire { seat, cap, .. } => (
                WindowLabel::RiderTerms,
                std::iter::once(Choice::BurnPool)
                    .chain(
                        st.minions_of(seat)
                            .filter(|&m| {
                                st.minions[m.index()].is_vampire()
                                    && st.capacity_of(m).is_some_and(|c| c >= cap)
                            })
                            .map(Choice::ChooseMinion),
                    )
                    .collect(),
            ),
            Interrupt::PoolOrNoTransfers { .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::BurnPool, Choice::Pass],
            ),
            Interrupt::LockOrPay { card, .. } => (
                WindowLabel::RiderTerms,
                if st.cards[card.index()].locked {
                    vec![Choice::BurnPool]
                } else {
                    vec![Choice::ResolveRider { card: card.0 }, Choice::BurnPool]
                },
            ),
            // By card, not by position: the order of the library is hidden.
            Interrupt::PutFoundOn { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::FindCard(card)],
            ),
            Interrupt::LibraryToHand { seat, filter, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        crate::state::distinct(&st.seats[seat.index()].library, |c| {
                            filter.matches(c)
                        })
                        .into_iter()
                        .map(Choice::FindCard),
                    )
                    .collect(),
            ),
            Interrupt::CryptSearch { seat, clan } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.crypt_of_clan(seat, clan)
                            .into_iter()
                            .map(Choice::AshVampire),
                    )
                    .collect(),
            ),
            Interrupt::PreySlotToPool { of, .. } => (
                WindowLabel::ResolutionChoice,
                (0..st.seats[of.index()].uncontrolled.len())
                    .filter(|&i| st.seats[of.index()].uncontrolled[i].counters > 0)
                    .map(|i| Choice::Slot(i as u8))
                    .collect(),
            ),
            Interrupt::CircleBlood {
                seat,
                circle,
                from,
                to,
                equipment,
                moving,
                paired,
            } => {
                let members = self.circle_members(seat, circle);
                let used = |f: MinionId, t: MinionId| paired.contains(&Some((f, t)));
                let mut options = vec![Choice::Pass];
                if let (Some(f), Some(_)) = (from, to) {
                    let n = st.minions[f.index()].blood;
                    return Some(DecisionRequest::new(
                        seat,
                        WindowLabel::ResolutionChoice,
                        (1..=n).map(Choice::SetX).collect(),
                    ));
                }
                match (from, moving) {
                    (None, None) => {
                        options.extend(
                            members
                                .iter()
                                .filter(|&&m| {
                                    self.state.minions[m.index()].blood > 0
                                        && members.iter().any(|&t| t != m && !used(m, t))
                                })
                                .map(|&m| Choice::ChooseMinion(m)),
                        );
                        if equipment {
                            for &m in &members {
                                options.extend(
                                    self.state
                                        .equipment_on(m)
                                        .into_iter()
                                        .filter(|&c| !self.state.pinned(c))
                                        .map(|c| Choice::ChooseCard(c.0)),
                                );
                            }
                        }
                    }
                    (Some(f), _) => options.extend(
                        members
                            .iter()
                            .filter(|&&m| m != f && !used(f, m))
                            .map(|&m| Choice::ChooseMinion(m)),
                    ),
                    (None, Some(c)) => {
                        let on = st.cards[c.index()].host;
                        options.extend(
                            members
                                .iter()
                                .filter(|&&m| Some(m) != on)
                                .map(|&m| Choice::ChooseMinion(m)),
                        );
                    }
                }
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::BaseSearch { seat, minion } => {
                let name = st.minions[minion.index()].name();
                let base = |c: &u16| {
                    *c != crate::state::CREATED
                        && crate::crypt::CRYPT[*c as usize].name == name
                        && !crate::crypt::CRYPT[*c as usize].adv
                };
                let sv = &st.seats[seat.index()];
                let mut found: Vec<u16> = sv
                    .crypt
                    .iter()
                    .chain(&sv.ash_crypt)
                    .copied()
                    .filter(base)
                    .collect();
                found.sort_unstable();
                found.dedup();
                let mut options = vec![Choice::Pass];
                options.extend(found.into_iter().map(Choice::AshVampire));
                options.extend(
                    (0..sv.uncontrolled.len())
                        .filter(|&i| {
                            sv.uncontrolled[i].banished.is_none() && base(&sv.uncontrolled[i].card)
                        })
                        .map(|i| Choice::Slot(i as u8)),
                );
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::PoolToVampires { seat, to } => {
                let pool = st.seats[seat.index()].pool.max(0) as u8;
                let room = |m: crate::ids::MinionId| {
                    st.capacity_of(m)
                        .unwrap_or(0)
                        .saturating_sub(st.minions[m.index()].blood)
                };
                if let Some(t) = to {
                    return Some(DecisionRequest::new(
                        seat,
                        WindowLabel::ResolutionChoice,
                        (1..=pool.min(room(t))).map(Choice::SetX).collect(),
                    ));
                }
                let mut options = vec![Choice::Pass];
                if pool > 0 {
                    options.extend(
                        st.minions_of(seat)
                            .filter(|&m| st.minions[m.index()].is_vampire() && room(m) > 0)
                            .map(Choice::ChooseMinion),
                    );
                }
                (WindowLabel::ResolutionChoice, options)
            }
            // Pass: his own circle of one (owner, rules-qa 68.1).
            Interrupt::ChooseCircle { .. } => (
                WindowLabel::EntryEffect,
                std::iter::once(Choice::Pass)
                    .chain(
                        crate::state::circles()
                            .into_iter()
                            .map(Choice::ChooseCircle),
                    )
                    .collect(),
            ),
            Interrupt::ReadyTitledLeft { seat, of } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(self.titled_left_plays(seat, of))
                    .collect(),
            ),
            Interrupt::NameMasterDiscipline { seat } => {
                let z = &st.seats[seat.index()];
                let mut cards: Vec<u16> = crate::state::master_disciplines(&z.hand);
                cards.extend(crate::state::master_disciplines(&z.ash));
                cards.sort_unstable();
                cards.dedup();
                (
                    WindowLabel::ResolutionChoice,
                    cards.into_iter().map(Choice::FindCard).collect(),
                )
            }
            Interrupt::NameAshEquipment { minion, from, .. } => {
                let found = self.ash_equipment_for(minion);
                (
                    WindowLabel::ResolutionChoice,
                    match from {
                        None => {
                            let mut seats: Vec<SeatId> = found.iter().map(|&(s, _)| s).collect();
                            seats.dedup();
                            seats.into_iter().map(Choice::ChooseSeat).collect()
                        }
                        Some(s) => found
                            .into_iter()
                            .filter(|&(f, _)| f == s)
                            .map(|(_, c)| Choice::FindCard(c))
                            .collect(),
                    },
                )
            }
            // Each price, payable down to 0 pool (a cost, Default readings);
            // not a title its Methuselah already holds: no contest with
            // oneself (rules-qa 76.2).
            Interrupt::BuyTitle {
                seat,
                minion,
                offers,
            } => (
                WindowLabel::EntryEffect,
                std::iter::once(Choice::Pass)
                    .chain(
                        offers
                            .iter()
                            .filter(|&&(_, t)| {
                                let key = crate::state::title_key(t);
                                !self.state.minions_of(seat).any(|o| {
                                    o != minion
                                        && crate::state::title_key(self.state.claimed_title(o))
                                            == key
                                })
                            })
                            .map(|&(n, _)| Choice::SetX(n)),
                    )
                    .collect(),
            ),
            Interrupt::DistributeFrom { seat, clan, .. } => (
                WindowLabel::EntryEffect,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.uncontrolled_of_clan(seat, clan)
                            .into_iter()
                            .map(|i| Choice::Slot(i as u8)),
                    )
                    .collect(),
            ),
            Interrupt::UncontrolledBlood {
                seat, clan, may, ..
            } => (
                WindowLabel::ResolutionChoice,
                may.then_some(Choice::Pass)
                    .into_iter()
                    .chain(
                        self.uncontrolled_of(seat, clan)
                            .into_iter()
                            .map(|i| Choice::Slot(i as u8)),
                    )
                    .collect(),
            ),
            Interrupt::MayPutOn { held, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ChooseMinion(held.minion)],
            ),
            Interrupt::MayAddCounter { card, .. }
            | Interrupt::MayTorture { card, .. }
            | Interrupt::MayStoreAim { store: card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolveRider { card: card.0 }],
            ),
            Interrupt::MayGainPool { minion, .. }
            | Interrupt::MayGainHandSize { minion, .. }
            | Interrupt::MayPreventForBlood { minion, .. }
            | Interrupt::MayUnlock { minion, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolvePrinted { minion }],
            ),
            Interrupt::Redraw { .. } => {
                (WindowLabel::Draw, vec![Choice::Pass, Choice::DrawLibrary])
            }
            Interrupt::NameAshVampire { seat } => (
                WindowLabel::ResolutionChoice,
                crate::state::distinct(
                    &st.seats[seat.index()].ash_crypt,
                    crate::state::vampire_card,
                )
                .into_iter()
                .map(Choice::AshVampire)
                .collect(),
            ),
            Interrupt::AllyFromAsh { seat, of, .. } => (
                WindowLabel::ResolutionChoice,
                self.ash_allies(seat, of)
                    .into_iter()
                    .map(Choice::FindCard)
                    .collect(),
            ),
            Interrupt::MayRecycleCanceled { minion, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolvePrinted { minion }],
            ),
            Interrupt::GiveCardTo { .. } => (
                WindowLabel::ResolutionChoice,
                (0..st.seats.len())
                    .map(|i| SeatId(i as u8))
                    .filter(|s| !st.seats[s.index()].ousted)
                    .map(Choice::ChooseSeat)
                    .collect(),
            ),
            Interrupt::BeastDisciplines { minion, .. } => {
                let chosen = st.minions[minion.index()].beast_disciplines;
                (
                    WindowLabel::ResolutionChoice,
                    (0..crate::cards::GREAT_BEAST_DISCIPLINES.len())
                        .filter(|&i| {
                            !chosen.contains(&Some(crate::cards::GREAT_BEAST_DISCIPLINES[i]))
                        })
                        .map(|i| Choice::Slot(i as u8))
                        .collect(),
                )
            }
            Interrupt::StoreOwnAsh { seat, .. } => (
                WindowLabel::ResolutionChoice,
                crate::state::distinct(&st.seats[seat.index()].ash, |_| true)
                    .into_iter()
                    .map(Choice::FindCard)
                    .collect(),
            ),
            Interrupt::StoreOwnHand { seat, .. } => (
                WindowLabel::ResolutionChoice,
                crate::state::distinct(&st.seats[seat.index()].hand, |_| true)
                    .into_iter()
                    .map(Choice::FindCard)
                    .collect(),
            ),
            Interrupt::StoredToLibraryTop { card, .. } => (
                WindowLabel::ResolutionChoice,
                crate::state::distinct(&st.cards[card.index()].stored, |_| true)
                    .into_iter()
                    .map(Choice::FindCard)
                    .collect(),
            ),
            Interrupt::StorePreyAsh { seat, .. } => (
                WindowLabel::ResolutionChoice,
                crate::state::distinct(&st.seats[st.prey(seat).index()].ash, |_| true)
                    .into_iter()
                    .map(Choice::FindCard)
                    .collect(),
            ),
            Interrupt::NameOwnAshCard { seat, of } => (
                WindowLabel::ResolutionChoice,
                // "You can" (Tye Cooper): none too.
                self.action
                    .as_ref()
                    .is_some_and(|a| a.ash_to_hand_on_name)
                    .then_some(Choice::Pass)
                    .into_iter()
                    .chain(
                        crate::state::distinct(&st.seats[seat.index()].ash, |c| of.matches(c))
                            .into_iter()
                            .map(Choice::FindCard),
                    )
                    .collect(),
            ),
            Interrupt::NameSlotPair { seat, of, from } => (
                WindowLabel::ResolutionChoice,
                match from {
                    None => (0..st.seats[of.index()].uncontrolled.len() as u8)
                        .filter(|&i| st.seats[of.index()].uncontrolled[i as usize].counters > 0)
                        .map(Choice::Slot)
                        .collect(),
                    // "To a vampire in your uncontrolled region": no imbued
                    // (rulebook "Appendix: Imbued Rules", Lázár Dobrescu).
                    Some(_) => (0..st.seats[seat.index()].uncontrolled.len() as u8)
                        .filter(|&i| {
                            let c = st.seats[seat.index()].uncontrolled[i as usize].card;
                            c == crate::state::CREATED || !crate::crypt::CRYPT[c as usize].imbued
                        })
                        .map(Choice::Slot)
                        .collect(),
                },
            ),
            Interrupt::NameUncontrolled { seat, target } => (
                WindowLabel::ResolutionChoice,
                (0..st.seats[seat.index()].uncontrolled.len() as u8)
                    .filter(|&i| i != target)
                    .map(Choice::Slot)
                    .collect(),
            ),
            Interrupt::NameAshPair { of, .. } => {
                let ash = &st.seats[of.index()].ash;
                (
                    WindowLabel::ResolutionChoice,
                    crate::state::distinct(ash, |c| ash.iter().filter(|&&x| x == c).count() >= 2)
                        .into_iter()
                        .map(Choice::FindCard)
                        .collect(),
                )
            }
            Interrupt::MinionOf { of, purpose, .. } => {
                let mut options: Vec<Choice> = self
                    .minion_of_options(of, purpose)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect();
                if !matches!(
                    purpose,
                    MinionPurpose::Corrupt
                        | MinionPurpose::BurnBloodOfClan(_)
                        | MinionPurpose::HostCard(_)
                        | MinionPurpose::LockOldest(_)
                        | MinionPurpose::OldestHuntingGround(_)
                        | MinionPurpose::BurnOfCrowdedClan
                        | MinionPurpose::Damage(_)
                        | MinionPurpose::DamageImbued { .. }
                ) {
                    options.insert(0, Choice::Pass);
                }
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::BlockToll { .. } => (
                WindowLabel::BlockToll,
                vec![Choice::PayBlockToll, Choice::Pass],
            ),
            Interrupt::MarginPool { .. } => (
                WindowLabel::Distribution,
                self.margin_pool_range(front)
                    .into_iter()
                    .map(|n| Choice::AllocateTo { seat: s, n })
                    .collect(),
            ),
            Interrupt::MarginBlood { .. } => {
                let first = self.margin_recipients(front)[0];
                (
                    WindowLabel::Distribution,
                    vec![Choice::ChooseMinion(first), Choice::Pass],
                )
            }
            Interrupt::EntryRemoveOrBurn { .. } => {
                // One choice per card: copies are alike.
                let mut options = vec![Choice::Pass];
                options.extend(self.ash_allies(s, &[]).into_iter().map(Choice::FindCard));
                options.extend(
                    distinct(&st.seats[s.index()].ash_crypt, crate::state::vampire_card)
                        .into_iter()
                        .map(Choice::AshVampire),
                );
                (WindowLabel::EntryEffect, options)
            }
        };
        Some(DecisionRequest::new(s, window, options))
    }

    /// Alastor's half cost of `card`, rounded down: (pool, blood), X as 0.
    fn alastor_cost(&self, seat: SeatId, minion: crate::ids::MinionId, card: u16) -> (u8, u8) {
        let pool = crate::cards::pool_cost(
            &self.state,
            seat,
            card,
            crate::cards::Target::Minion(minion),
        )
        .max(0) as u8;
        let blood = crate::cards::blood_cost(&self.state, card, 0);
        (pool / 2, blood / 2)
    }

    /// The equipment cards in `seat`'s library Alastor can put on `minion`:
    /// by card (the order is hidden), its half cost payable — the pool
    /// `seat`'s, the blood the vampire's. Requirements are ignored, not
    /// "only one vehicle" and its kin (rules-digest §3.17: never a second).
    pub(crate) fn alastor_equipment(&self, seat: SeatId, minion: crate::ids::MinionId) -> Vec<u16> {
        let st = &self.state;
        crate::state::distinct(&st.seats[seat.index()].library, |c| {
            let (pool, blood) = self.alastor_cost(seat, minion, c);
            crate::library::LIBRARY[c as usize]
                .types
                .contains(&crate::library::CardType::Equipment)
                && crate::cards::lookup(c).is_some_and(|d| {
                    d.modes[0].requires.iter().all(|r| match *r {
                        crate::cards::Requirement::ActorWithout(h) => !st.holds_a(minion, h),
                        _ => true,
                    })
                })
                && st.seats[seat.index()].pool >= pool as i32
                && st.minions[minion.index()].blood >= blood
        })
    }

    /// Game of Malkav's reveal: each Methuselah gains the pool they chose,
    /// or burns it if another chose exactly 1 less; all at once.
    fn reveal_numbers(&mut self) {
        let picks = std::mem::take(&mut self.secret_numbers);
        let mut loss = vec![0i32; self.state.seats.len()];
        for &(s, n) in &picks {
            let undercut = picks.iter().any(|&(o, m)| o != s && m + 1 == n);
            if undercut {
                loss[s.index()] += n as i32;
            } else {
                self.state.seats[s.index()].pool += n as i32;
            }
        }
        self.burn_pool_batch(&loss);
    }

    /// The minions a card leaving `from` may go on
    /// (`Interrupt::MoveOffBurnedBearer`): every other one in play, arena
    /// order.
    pub(crate) fn other_hosts(&self, from: crate::ids::MinionId) -> Vec<crate::ids::MinionId> {
        let st = &self.state;
        (0..st.minions.len())
            .map(|i| crate::ids::MinionId(i as u32))
            .filter(|&m| m != from && st.minions[m.index()].present())
            .collect()
    }

    /// The vampires `card` may move to (`Interrupt::MoveCardToTitled`):
    /// every other justicar or Inner Circle member in play, arena order.
    pub(crate) fn titled_hosts(&self, card: CardId) -> Vec<crate::ids::MinionId> {
        use crate::cards::TitleClass;
        let st = &self.state;
        let host = st.cards[card.index()].host;
        (0..st.minions.len())
            .map(|i| crate::ids::MinionId(i as u32))
            .filter(|&m| {
                Some(m) != host
                    && st.minions[m.index()].present()
                    && st.minions[m.index()].is_vampire()
                    && [TitleClass::Justicar, TitleClass::InnerCircle]
                        .iter()
                        .any(|c| c.matches(st.title_of(m)))
            })
            .collect()
    }

    /// The minions of `of` an `Interrupt::MinionOf` may choose.
    pub(crate) fn minion_of_options(
        &self,
        of: SeatId,
        purpose: MinionPurpose,
    ) -> Vec<crate::ids::MinionId> {
        let st = &self.state;
        st.minions_of(of)
            .filter(|m| {
                let v = &st.minions[m.index()];
                match purpose {
                    MinionPurpose::LockVampire => v.is_vampire(),
                    MinionPurpose::UnlockAnarch => {
                        v.ready() && st.sect_of(*m) == Some(crate::crypt::Sect::Anarch)
                    }
                    MinionPurpose::BurnBloodOfClan(c) => {
                        v.ready() && v.clan() == Some(c) && v.blood > 0
                    }
                    // Any minion, a torpid one too (RTR 19960112, rules-qa
                    // 88.9).
                    MinionPurpose::HostCard(c) => st.can_hold(*m, st.cards[c.index()].card, &[]),
                    MinionPurpose::Damage(_) => v.ready(),
                    MinionPurpose::DamageImbued { except, .. } => {
                        v.ready() && v.is_imbued() && *m != except
                    }
                    MinionPurpose::BurnOfCrowdedClan => {
                        v.is_vampire()
                            && v.clan().is_some_and(|c| {
                                st.minions_of(of)
                                    .filter(|&o| {
                                        let w = &st.minions[o.index()];
                                        w.is_vampire() && w.clan() == Some(c)
                                    })
                                    .count()
                                    >= 3
                            })
                    }
                    // The oldest ready vampires left, the one picked
                    // excluded.
                    MinionPurpose::OldestHuntingGround(first) => {
                        let left = |o: &crate::ids::MinionId| {
                            Some(*o) != first
                                && st.minions[o.index()].ready()
                                && st.minions[o.index()].is_vampire()
                        };
                        left(m)
                            && st.capacity_of(*m)
                                == st
                                    .minions_of(of)
                                    .filter(left)
                                    .filter_map(|o| st.capacity_of(o))
                                    .max()
                    }
                    MinionPurpose::LockOldest(c) => {
                        v.clan() == Some(c)
                            && st.capacity_of(*m)
                                == st
                                    .minions_of(of)
                                    .filter(|&o| st.minions[o.index()].clan() == Some(c))
                                    .filter_map(|o| st.capacity_of(o))
                                    .max()
                    }
                    MinionPurpose::Lock | MinionPurpose::Corrupt => true,
                }
            })
            .collect()
    }

    /// Move `card`, an ally in `s`'s ash heap, to the ready region (locked
    /// with `locked`): put in play, not recruited — its inferior version,
    /// its starting life, able to act this turn (LSJ 20100204).
    pub(crate) fn raise_ash_ally(&mut self, s: SeatId, card: u16, locked: bool) {
        let ash = &mut self.state.seats[s.index()].ash;
        let k = ash.iter().rposition(|&c| c == card).expect("ash ally");
        let (_, copy) = ash.take(k);
        let m = self.put_ally_in_play(s, card, copy);
        self.state.minions[m.index()].locked |= locked;
    }

    /// An ally put in play, not recruited: inferior, its starting life,
    /// able to act this turn (LSJ 20100204).
    fn put_ally_in_play(&mut self, s: SeatId, card: u16, copy: crate::copies::CopyId) -> MinionId {
        let m = self.recruit_ally(s, card, copy, 0);
        self.state.minions[m.index()].recruited = false;
        m
    }

    /// The allies in `seat`'s ash heap matching one of `of` (any, if
    /// empty), one entry per card.
    pub(crate) fn ash_allies(&self, seat: SeatId, of: &[crate::cards::CardFilter]) -> Vec<u16> {
        distinct(&self.state.seats[seat.index()].ash, |c| {
            crate::cards::ash_ally(c, of)
        })
    }

    /// The Methuselahs still in the game with an ally in their ash heap.
    pub(crate) fn ash_ally_heaps(&self) -> Vec<SeatId> {
        (0..self.state.seats.len() as u8)
            .map(SeatId)
            .filter(|&s| !self.state.seats[s.index()].ousted && !self.ash_allies(s, &[]).is_empty())
            .collect()
    }

    /// The distinct crypt cards of `clan` in `seat`'s crypt (Disciple of
    /// Lazarus's search).
    fn crypt_of_clan(&self, seat: SeatId, clan: &'static str) -> Vec<u16> {
        distinct(&self.state.seats[seat.index()].crypt, |c| {
            crate::crypt::CRYPT[c as usize].clan == clan
        })
    }

    /// `seat`'s uncontrolled vampires of `clan`, or all of them.
    fn uncontrolled_of(&self, seat: SeatId, clan: Option<&'static str>) -> Vec<usize> {
        match clan {
            Some(c) => self.uncontrolled_of_clan(seat, c),
            None => (0..self.state.seats[seat.index()].uncontrolled.len()).collect(),
        }
    }

    /// The slots of `seat`'s uncontrolled region holding a `clan` vampire.
    fn uncontrolled_of_clan(&self, seat: SeatId, clan: &'static str) -> Vec<usize> {
        (0..self.state.seats[seat.index()].uncontrolled.len())
            .filter(|&i| self.state.slot_vampire(seat, i).1 == clan)
            .collect()
    }

    /// The vampires Anarch Convert can make Anarch: non-titled ones of
    /// `seat`'s, already Anarch included (PIB 20121031), not himself.
    fn converts(&self, seat: SeatId, convert: crate::ids::MinionId) -> Vec<crate::ids::MinionId> {
        self.untitled_vampires(seat)
            .into_iter()
            .filter(|&m| m != convert)
            .collect()
    }

    /// The non-titled vampires `seat` controls, in torpor too (Gilbert
    /// Duane: an Anarch one a legal no-op pick).
    fn untitled_vampires(&self, seat: SeatId) -> Vec<crate::ids::MinionId> {
        let st = &self.state;
        st.minions_of(seat)
            .filter(|&m| {
                st.minions[m.index()].is_vampire() && st.title_of(m) == crate::crypt::Title::None
            })
            .collect()
    }

    /// The ready vampires a `ChooseForCard` may pick: its seat's, of its
    /// clan, other than the first chosen for a second choice.
    fn choosable(&self, front: Interrupt) -> Vec<crate::ids::MinionId> {
        let Interrupt::ChooseForCard {
            seat,
            card,
            clan,
            second,
        } = front
        else {
            unreachable!("not a choice for a card");
        };
        let st = &self.state;
        let first = st.cards[card.index()].chosen.first().copied();
        st.minions_of(seat)
            .filter(|&m| {
                let v = &st.minions[m.index()];
                v.ready()
                    && v.is_vampire()
                    && clan.is_none_or(|c| v.clan() == Some(c))
                    && !(second && Some(m) == first)
            })
            .collect()
    }

    /// Record a `ChooseForCard` pick: the card's chosen vampire, or its
    /// second one; none if there was none to choose.
    fn set_chosen(&mut self, front: Interrupt, m: Option<crate::ids::MinionId>) {
        let Interrupt::ChooseForCard { card, second, .. } = front else {
            unreachable!("not a choice for a card");
        };
        let chosen = &mut self.state.cards[card.index()].chosen;
        if second {
            chosen.truncate(1);
            chosen.extend(m);
        } else {
            *chosen = m.into_iter().collect();
        }
    }

    /// Dual Form: a vampire or its copy leaving the ready region, the
    /// others of the set are burned, whatever chain they form (LSJ
    /// 20050117); their burning does not burn it again.
    fn burn_dual_forms(&mut self, ms: &[crate::ids::MinionId]) {
        for &m in ms {
            let st = &self.state;
            if !st.minions[m.index()].ready() || st.linked_leaving.contains(&m) {
                continue;
            }
            let root = st.minions[m.index()].dual_of.unwrap_or(m);
            let others: Vec<crate::ids::MinionId> = (0..st.minions.len())
                .map(|i| crate::ids::MinionId(i as u32))
                .filter(|&o| {
                    o != m
                        && (o == root || st.minions[o.index()].dual_of == Some(root))
                        && st.minions[o.index()].present()
                        && !st.linked_leaving.contains(&o)
                })
                .collect();
            if others.is_empty() {
                continue;
            }
            self.state.linked_leaving.push(m);
            for o in others {
                if self.state.minions[o.index()].present() {
                    self.burn_minion(o);
                }
            }
            self.state.linked_leaving.retain(|&x| x != m);
        }
    }

    /// Does `seat` hold a card with a mode for `window` (cheap, before the
    /// options are built)?
    fn holds_card_for(&self, seat: SeatId, window: crate::cards::Window) -> bool {
        self.state.seats[seat.index()].hand.iter().any(|&c| {
            crate::cards::lookup(c)
                .is_some_and(|d| d.modes.iter().any(|m| m.windows.contains(&window)))
        })
    }

    /// The plays `seat` may make for `of`'s vampire just gone (Leadership
    /// Vacuum): at `of` only.
    pub(crate) fn titled_left_plays(&self, seat: SeatId, of: SeatId) -> Vec<Choice> {
        self.play_options(seat, &[crate::cards::Window::ReadyTitledLeft], None)
            .into_iter()
            .filter(|o| matches!(o, Choice::Play { target: crate::cards::Target::Seat(t), .. } if *t == of))
            .collect()
    }

    /// Leadership Vacuum: ready titled vampires about to be burned or sent
    /// to torpor (never banished, removed or descended), their votes read
    /// now (ANK 20211009), offered to each other Methuselah holding the
    /// card.
    pub(crate) fn titled_leaving(&mut self, ms: &[crate::ids::MinionId]) {
        for &m in ms {
            let v = &self.state.minions[m.index()];
            if !(v.ready() && v.is_vampire()) || self.state.title_of(m) == crate::crypt::Title::None
            {
                continue;
            }
            let (of, votes) = (v.owner, self.state.votes_held(m));
            self.state.titled_left.push((of, votes));
            // Clockwise from the active Methuselah, the vampire's aside
            // ("usable during your turn").
            let n = self.state.seats.len();
            for k in 0..n {
                let seat = SeatId(((self.state.active.0 as usize + k) % n) as u8);
                if seat != of
                    && !self.state.seats[seat.index()].ousted
                    && self.holds_card_for(seat, crate::cards::Window::ReadyTitledLeft)
                {
                    self.interrupts
                        .push(Interrupt::ReadyTitledLeft { seat, of });
                }
            }
        }
    }

    /// Minions about to leave the ready region: the cards on them that pay
    /// for it offer their pool (Priority Contract).
    pub(crate) fn about_to_leave_ready(&mut self, ms: &[crate::ids::MinionId]) {
        self.burn_dual_forms(ms);
        for &m in ms {
            if !self.state.minions[m.index()].ready() {
                continue;
            }
            for ci in 0..self.state.cards.len() {
                let id = CardId(ci as u32);
                let c = &self.state.cards[ci];
                if !c.present() || c.host != Some(m) {
                    continue;
                }
                let n = cards::lookup(c.card).and_then(|d| {
                    d.modes[c.mode as usize]
                        .effects
                        .iter()
                        .find_map(|e| match e {
                            crate::cards::Effect::PoolWhenHostLeaves(n) => Some(*n),
                            _ => None,
                        })
                });
                if let Some(n) = n {
                    let seat = self.state.card_controller(id);
                    self.interrupts
                        .push(Interrupt::PoolForCard { seat, card: id, n });
                }
                // The Black Throne of the Methuselah whose vampire a contract
                // chose (PIB 20150512, ANK 20180129-2), any contract.
                let contract = crate::library::LIBRARY[self.state.cards[ci].card as usize]
                    .labels
                    .contains(&crate::library::Label::Contract);
                if contract {
                    if let Some(&chosen) = self.state.cards[ci].chosen.first() {
                        let x = self.state.minions[chosen.index()].owner;
                        self.contract_riders(x);
                    }
                }
            }
        }
    }

    /// The Black Throne of `x`'s: unlocked, offered to lock for pool.
    fn contract_riders(&mut self, x: SeatId) {
        let on = |c: &CardInPlay| !c.locked;
        let at = HookAt {
            seat: Some(x),
            ..HookAt::on(&on)
        };
        for (id, r) in self.fired_riders(Hook::ContractHostLeaves, &at) {
            for e in r.effects {
                match *e {
                    Effect::GainPool(n) => self.interrupts.push(Interrupt::LockForPool {
                        seat: x,
                        card: id,
                        n,
                    }),
                    other => unreachable!("not a contract rider effect: {other:?}"),
                }
            }
        }
    }

    /// Vampires of other Methuselahs a vampire of `seat`'s may give blood
    /// to (Blood Trade).
    fn blood_recipients(&self, seat: SeatId, from: MinionId, anarchs: bool) -> Vec<MinionId> {
        (0..self.state.minions.len())
            .map(|i| MinionId(i as u32))
            .filter(|&m| {
                let v = &self.state.minions[m.index()];
                if anarchs {
                    m != from
                        && v.owner == seat
                        && v.ready()
                        && self.state.sect_of(m) == Some(crate::crypt::Sect::Anarch)
                } else {
                    v.present() && v.is_vampire() && v.owner != seat
                }
            })
            .collect()
    }

    /// The other ready vampires of `seat`'s Heidelberg Castle moves to
    /// from `from`.
    fn move_recipients(
        &self,
        seat: SeatId,
        from: MinionId,
        how: crate::cards::Moves,
    ) -> Vec<MinionId> {
        self.state
            .minions_of(seat)
            .filter(|&m| {
                let v = &self.state.minions[m.index()];
                m != from
                    && v.ready()
                    && v.is_vampire()
                    && how.sect.is_none_or(|s| self.state.sect_of(m) == Some(s))
            })
            .collect()
    }

    /// The equipment on a diablerie's victim its diablerist could take:
    /// one it could hold, not one barred from it (Soul Gem of Etrius).
    pub(crate) fn diablerie_equipment(
        &self,
        diablerist: MinionId,
        victim: MinionId,
    ) -> Vec<CardId> {
        self.holdable_equipment(victim, diablerist)
            .into_iter()
            .filter(|c| {
                let c = &self.state.cards[c.index()];
                !cards::lookup(c.card).is_some_and(|d| {
                    d.modes[c.mode as usize]
                        .effects
                        .contains(&Effect::NotTakenByDiablerie)
                })
            })
            .collect()
    }

    /// The equipment on `from` that `to` could hold (LSJ 19980831: not on
    /// a vampire barred from it), not pinned to it (Eye of Hazimel).
    fn holdable_equipment(&self, from: MinionId, to: MinionId) -> Vec<CardId> {
        self.state
            .equipment_on(from)
            .into_iter()
            .filter(|&c| {
                !self.state.pinned(c)
                    && self
                        .state
                        .can_hold(to, self.state.cards[c.index()].card, &[])
            })
            .collect()
    }

    /// The equipment and retainers on `from` that `to` could have.
    fn movable_cards(&self, from: MinionId, to: MinionId, how: crate::cards::Moves) -> Vec<CardId> {
        if self.state.no_equipment_or_retainers(to) {
            return Vec::new();
        }
        let mut out = self.holdable_equipment(from, to);
        if !how.retainers || self.state.no_retainers(to) {
            return out;
        }
        out.extend(
            self.state
                .cards
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    c.present()
                        && c.host == Some(from)
                        && crate::library::LIBRARY[c.card as usize]
                            .types
                            .contains(&crate::library::CardType::Retainer)
                })
                .map(|(i, _)| CardId(i as u32)),
        );
        out
    }

    /// A location changing controller (Dominate Kine [DOM], Disputed
    /// Territory): one on a minion moves to a new controller's minion that
    /// can hold it, else burns (RTR 19960112; rules-qa 39.4, 88.9).
    pub(crate) fn give_location(&mut self, card: CardId, seat: SeatId) {
        self.give_card_control(card, seat);
        if self.state.cards[card.index()].host.is_none() {
            return;
        }
        if self
            .minion_of_options(seat, MinionPurpose::HostCard(card))
            .is_empty()
        {
            self.burn_card_in_play(card);
        } else {
            self.interrupts.push(Interrupt::MinionOf {
                seat,
                of: seat,
                purpose: MinionPurpose::HostCard(card),
            });
        }
    }

    /// `seat` takes control of `card` in play; a card that chooses on a
    /// change of control has its new controller choose (The Rack).
    pub(crate) fn give_card_control(&mut self, card: CardId, seat: SeatId) {
        self.state.cards[card.index()].controller = seat;
        // An ally's card: the ally changes hands (Vivienne Géroux), and
        // enters any contest of its name.
        if let Some(m) = self.state.cards[card.index()].body {
            self.state.minions[m.index()].owner = seat;
            self.enter_card_contest(m);
        }
        let def = cards::lookup(self.state.cards[card.index()].card);
        // The choice its ability made was the old controller's (Powerbase:
        // Luanda, rules-qa 77.16).
        if def.is_some_and(|d| {
            d.abilities
                .iter()
                .any(|a| a.effects.contains(&crate::cards::Effect::CardChoosesTarget))
        }) {
            self.state.cards[card.index()].chosen.clear();
        }
        let rechooses = def.is_some_and(|d| {
            d.modes[0]
                .effects
                .contains(&crate::cards::Effect::RechoosesOnControlChange)
        });
        if rechooses {
            self.interrupts.push(Interrupt::ChooseForCard {
                seat,
                card,
                clan: None,
                second: false,
            });
        }
    }

    /// Remove `seat`'s uncontrolled vampire `i` from the game, its counters
    /// to the bank; a banished one's cards burn with it.
    pub(crate) fn remove_uncontrolled(&mut self, seat: SeatId, i: usize) {
        let slot = self.state.seats[seat.index()].uncontrolled.remove(i);
        if let Some(b) = slot.banished {
            for c in b.cards {
                let copy = *self.state.cards[c.index()].copy;
                if self.state.cards[c.index()].body.is_some() {
                    let card = self.state.cards[c.index()].card;
                    self.state.seats[seat.index()].removed.put(card, copy);
                    continue;
                }
                // Out of play on it: straight to its owner's ash heap.
                let (owner, card) = (
                    self.state.cards[c.index()].owner,
                    self.state.cards[c.index()].card,
                );
                self.state.seats[owner.index()].ash.put(card, copy);
            }
        }
        if slot.card != crate::state::CREATED {
            self.state.seats[seat.index()]
                .removed_crypt
                .put(slot.card, *slot.copy);
        }
    }

    /// The Discipline master cards `s` could take for `minion`, by zone and
    /// card: the library's by card (its order hidden), the hand's and ash
    /// heap's; not one of a Discipline `minion` has at superior.
    fn discipline_masters(&self, s: SeatId, minion: crate::ids::MinionId) -> Vec<Choice> {
        let st = &self.state;
        let seat = &st.seats[s.index()];
        // Put on, not played: the card's requirements ignored (LSJ
        // 20051116-2: "eg. diablerie"), the level it adds still capped.
        let fits = |c: u16| {
            cards::lookup(c).is_some_and(|d| {
                d.modes[0].target.is_none_or(|t| {
                    t.filter
                        .iter()
                        .filter(|r| matches!(r, cards::Requirement::TargetNotSuperior(_)))
                        .all(|&r| cards::holds_for_minion(r, st, s, None, None, minion, None))
                })
            })
        };
        let mut out = Vec::new();
        let zones = [
            (Zone::Library, &seat.library),
            (Zone::Hand, &seat.hand),
            (Zone::AshHeap, &seat.ash),
        ];
        for (zone, cards) in zones {
            out.extend(
                distinct(cards, |c| {
                    CardFilter::HasLabel(Label::Discipline).matches(c) && fits(c)
                })
                .into_iter()
                .map(|card| Choice::TakeFrom { zone, card }),
            );
        }
        out
    }

    /// Baltimore Purge: one pick per other Methuselah with a candidate (the
    /// controller's with a ready `chooser_clan`), clockwise, in the open.
    pub(crate) fn start_purge(&mut self, host: MinionId, chooser_clan: &str) {
        let owner = self.state.minions[host.index()].owner;
        let chooses = self.state.minions_of(owner).any(|m| {
            let v = &self.state.minions[m.index()];
            v.ready() && v.is_vampire() && v.clan() == Some(chooser_clan)
        });
        self.purge_picks.clear();
        let mut any = false;
        for of in self.state.clockwise_from(owner) {
            if of == owner || self.state.seats[of.index()].ousted {
                continue;
            }
            if !self.purge_candidates(of, host).is_empty() {
                let seat = if chooses { owner } else { of };
                self.interrupts
                    .push(Interrupt::PurgePick { seat, of, host });
                any = true;
            }
        }
        if !any {
            self.finish_purge_if_done(host);
        }
    }

    /// `of`'s ready vampires not older than `host`.
    fn purge_candidates(&self, of: SeatId, host: MinionId) -> Vec<MinionId> {
        let st = &self.state;
        let cap = st.capacity_of(host).unwrap_or(0);
        st.minions_of(of)
            .filter(|&m| {
                let v = &st.minions[m.index()];
                v.ready() && v.is_vampire() && st.capacity_of(m).is_some_and(|c| c <= cap)
            })
            .collect()
    }

    /// Every pick made: the vampire and the chosen ones go to torpor at
    /// once, nothing played between (LSJ 20050111).
    fn finish_purge_if_done(&mut self, host: MinionId) {
        if self
            .interrupts
            .iter()
            .any(|i| matches!(i, Interrupt::PurgePick { .. }))
        {
            return;
        }
        let mut ms = std::mem::take(&mut self.purge_picks);
        if self.state.minions[host.index()].ready() {
            ms.insert(0, host);
        }
        ms.retain(|&m| self.state.minions[m.index()].ready());
        if !ms.is_empty() {
            self.minions_to_torpor(&ms);
        }
    }

    /// The powers requiring `virtue` imbued `m` could have, from `s`'s
    /// library and hand, by zone and card, and `Pass` (no second copy).
    fn power_sources(&self, s: SeatId, m: MinionId, virtue: &str) -> Vec<Choice> {
        let st = &self.state;
        let seat = &st.seats[s.index()];
        let lib = &crate::library::LIBRARY;
        let fits = |c: u16| {
            let l = &lib[c as usize];
            l.types.contains(&crate::library::CardType::Power)
                && l.disciplines.contains(&virtue)
                && !st.holds_a(m, crate::cards::Holding::Named(l.name))
        };
        let mut out = vec![Choice::Pass];
        for (zone, cards) in [(Zone::Library, &seat.library), (Zone::Hand, &seat.hand)] {
            out.extend(
                distinct(cards, fits)
                    .into_iter()
                    .map(|card| Choice::TakeFrom { zone, card }),
            );
        }
        out
    }

    /// The conviction cards `s` could give an imbued, by zone and card, and
    /// `Pass` (hand and ash heap; the library too with `library`).
    fn conviction_sources(&self, s: SeatId, library: bool) -> Vec<Choice> {
        let seat = &self.state.seats[s.index()];
        let mut out = vec![Choice::Pass];
        let zones = [
            (Zone::Library, &seat.library),
            (Zone::Hand, &seat.hand),
            (Zone::AshHeap, &seat.ash),
        ];
        for (zone, cards) in zones.into_iter().skip(usize::from(!library)) {
            out.extend(
                distinct(cards, |c| {
                    crate::library::LIBRARY[c as usize]
                        .types
                        .contains(&crate::library::CardType::Conviction)
                })
                .into_iter()
                .map(|card| Choice::TakeFrom { zone, card }),
            );
        }
        out
    }

    /// One `BurnInPlay` per distinct conviction on imbued `m`: copies of
    /// one card are the same choice.
    fn conviction_choices(&self, m: MinionId) -> Vec<Choice> {
        let st = &self.state;
        let mut seen: Vec<u16> = Vec::new();
        let mut options = Vec::new();
        for c in st.convictions(m) {
            let card = st.cards[c.index()].card;
            if !seen.contains(&card) {
                seen.push(card);
                options.push(Choice::BurnInPlay { card: c.0 });
            }
        }
        options
    }

    /// The last copy of `card` taken out of `s`'s `zone`, for a search or a
    /// take from hand or ash heap.
    fn take_from_zone(&mut self, s: SeatId, zone: Zone, card: u16) -> crate::copies::CopyId {
        let seat = &mut self.state.seats[s.index()];
        let from = match zone {
            Zone::Library => &mut seat.library,
            Zone::Hand => &mut seat.hand,
            Zone::AshHeap => &mut seat.ash,
        };
        let k = from.iter().rposition(|&x| x == card).expect("found card");
        from.take(k).1
    }

    /// A conviction card of `owner`'s gained by imbued `m`: on it, to its
    /// owner's ash heap above five (rulebook "Appendix: Imbued Rules" 2).
    pub(crate) fn gain_conviction(
        &mut self,
        m: MinionId,
        card: u16,
        copy: crate::copies::CopyId,
        owner: SeatId,
    ) {
        if self.state.minions[m.index()].present() && self.state.convictions(m).len() < 5 {
            self.enter_play(crate::state::CardInPlay::entering(
                card,
                copy,
                owner,
                Some(m),
                0,
            ));
        } else {
            self.state.seats[owner.index()].ash.put(card, copy);
        }
    }

    /// The mage allies Antonio d'Erlette (`m`) could play, by zone and card:
    /// payable after reductions (LSJ 20090920-2), no unique already in play.
    fn mage_allies(&self, s: SeatId, m: MinionId) -> Vec<Choice> {
        let st = &self.state;
        let seat = &st.seats[s.index()];
        let clan = st.minions[m.index()].clan();
        let ok = |c: u16| {
            let lib = &crate::library::LIBRARY[c as usize];
            let (pool, blood) = self.searched_ally_cost(s, m, c);
            CardFilter::HasLabel(Label::Mage).matches(c)
                && (lib.clans.is_empty() || clan.is_some_and(|k| lib.clans.contains(&k)))
                && pool <= seat.pool
                && blood <= st.minions[m.index()].blood
                && !(lib.labels.contains(&Label::Unique)
                    && st.cards.iter().enumerate().any(|(i, x)| {
                        x.present() && x.card == c && st.card_controller(CardId(i as u32)) == s
                    }))
        };
        let mut out = Vec::new();
        for (zone, cards) in [(Zone::Library, &seat.library), (Zone::Hand, &seat.hand)] {
            out.extend(
                distinct(cards, ok)
                    .into_iter()
                    .map(|card| Choice::TakeFrom { zone, card }),
            );
        }
        out
    }

    /// Osebo Kholo: each title card whose vampire is not a ready one of its
    /// clan and sect, or has an older ready untitled one beside it, moves to
    /// the oldest ready untitled one, if any (`Effect::MovesToOldestUntitled`).
    pub(crate) fn settle_moving_titles(&mut self) {
        let st = &self.state;
        let mut due = Vec::new();
        for (i, c) in st.cards.iter().enumerate() {
            let id = CardId(i as u32);
            let (Some(h), Some((clan, sect))) = (c.host, moving_title(c.card)) else {
                continue;
            };
            if !c.present() {
                continue;
            }
            let v = &st.minions[h.index()];
            let holds = v.ready() && v.clan() == Some(clan) && st.sect_of(h) == Some(sect);
            if holds && st.older_ready_untitled(clan, sect, h).next().is_none() {
                continue;
            }
            let pending = self
                .interrupts
                .iter()
                .any(|i| matches!(i, Interrupt::MoveCardToOldest { card, .. } if *card == id));
            if !pending && !self.oldest_for(id).is_empty() {
                due.push(Interrupt::MoveCardToOldest {
                    seat: st.card_controller(id),
                    card: id,
                });
            }
        }
        self.interrupts.extend(due);
    }

    /// A title card moved to `m`: its title `m`'s, contested as gained.
    fn move_title_card(&mut self, card: CardId, m: MinionId) {
        self.state.set_host(card, m);
        if let Some(t) = cards::card_title(self.state.cards[card.index()].card) {
            self.gain_title(m, t, Some(card));
        }
    }

    /// The oldest ready untitled vampires a moving title card may go to.
    fn oldest_for(&self, card: CardId) -> Vec<MinionId> {
        let st = &self.state;
        let Some((clan, sect)) = moving_title(st.cards[card.index()].card) else {
            return Vec::new();
        };
        let all: Vec<MinionId> = st.ready_untitled(clan, sect).collect();
        let top = all.iter().filter_map(|&m| st.capacity_of(m)).max();
        all.into_iter()
            .filter(|&m| st.capacity_of(m) == top)
            .collect()
    }

    /// What Summon History may put in play for `m`: the library and ash
    /// heap allies, retainers and equipment costing `x` or less (a cost X
    /// as 0, PIB 20111101) `m` may hold, or (`vampire`) the crypt and ash
    /// heap's non-scarce vampires of capacity `x` or less.
    fn put_in_play_finds(&self, s: SeatId, m: MinionId, x: u8, vampire: bool) -> Vec<Choice> {
        use crate::library::{CardType, Cost};
        let seat = &self.state.seats[s.index()];
        if vampire {
            let ok = |c: u16| {
                let v = &crate::crypt::CRYPT[c as usize];
                !v.imbued && !v.scarce && v.capacity <= x
            };
            let mut cards = distinct(&seat.crypt, ok);
            cards.extend(distinct(&seat.ash_crypt, ok));
            cards.sort_unstable();
            cards.dedup();
            return cards.into_iter().map(Choice::AshVampire).collect();
        }
        let ok = |c: u16| {
            let lib = &crate::library::LIBRARY[c as usize];
            let cost = |k: Cost| match k {
                Cost::Fixed(n) => n,
                Cost::None | Cost::X => 0,
            };
            let kind = [CardType::Ally, CardType::Retainer, CardType::Equipment]
                .iter()
                .any(|t| lib.types.contains(t));
            kind && cost(lib.pool_cost).max(cost(lib.blood_cost)) <= x
                && (lib.types.contains(&CardType::Ally) || self.state.can_hold(m, c, &[]))
        };
        let mut out = Vec::new();
        for (zone, cards) in [(Zone::Library, &seat.library), (Zone::AshHeap, &seat.ash)] {
            out.extend(
                distinct(cards, ok)
                    .into_iter()
                    .map(|card| Choice::TakeFrom { zone, card }),
            );
        }
        out
    }

    /// The pile Summon History searched, shuffled afterward.
    fn shuffle_searched(&mut self, s: SeatId, vampire: bool) {
        let seat = &mut self.state.seats[s.index()];
        if vampire {
            seat.crypt.shuffle(&mut self.rng);
        } else {
            seat.library.shuffle(&mut self.rng);
        }
    }

    /// The (pool, blood) a searched ally costs, `m`'s printed reductions
    /// and the cost modifiers in play applied.
    fn searched_ally_cost(&self, s: SeatId, m: MinionId, card: u16) -> (i32, u8) {
        use crate::library::Cost;
        let lib = &crate::library::LIBRARY[card as usize];
        let off = self.state.printed_cost_off(m, card, 0);
        let mut pool = cards::pool_cost(&self.state, s, card, crate::cards::Target::None);
        let mut blood = match lib.blood_cost {
            Cost::Fixed(n) => n,
            Cost::None | Cost::X => 0,
        };
        off.apply(&mut pool, &mut blood);
        (pool, blood)
    }

    /// One discard choice per card name in `s`'s hand (its first copy).
    pub(crate) fn discard_choices(&self, s: SeatId) -> Vec<Choice> {
        let hand = &self.state.seats[s.index()].hand;
        (0..hand.len())
            .filter(|&i| !hand[..i].contains(&hand[i]))
            .map(|i| Choice::Discard(i as u8))
            .collect()
    }

    pub(crate) fn interrupt_apply(&mut self, choice: &Choice) {
        match self.interrupt_cause().flatten() {
            Some(cause) => self.caused(cause, |e| e.interrupt_apply_now(choice)),
            None => self.interrupt_apply_now(choice),
        }
    }

    fn interrupt_apply_now(&mut self, choice: &Choice) {
        let front = self.interrupts[0];
        let s = front.seat();
        match (front, *choice) {
            (
                Interrupt::StealForCorruption { .. }
                | Interrupt::MakeAnarch { .. }
                | Interrupt::LockForPool { .. }
                | Interrupt::PoolForCard { .. }
                | Interrupt::MoveForBlood { .. }
                | Interrupt::CancelAttempt { .. }
                | Interrupt::MinionOf { .. },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
            }
            // The player's counters burned; the minion theirs for good.
            (Interrupt::StealForCorruption { minion, .. }, Choice::ChooseMinion(_)) => {
                self.interrupts.remove(0);
                // Mid-combat (Contagion [DAI]): the combat ends, damage
                // still pending resolving first (as `take_control`).
                if self
                    .combat
                    .as_ref()
                    .is_some_and(|c| c.side_of_minion(minion).is_some())
                {
                    let pending = self.combat.as_ref().is_some_and(|c| {
                        c.sides
                            .iter()
                            .any(|s| s.pending.total() > 0 || s.unpreventable > 0)
                    });
                    if !(pending && self.apply_damage()) && self.combat.is_some() {
                        self.end_combat_now();
                    }
                    if !self.state.minions[minion.index()].in_play {
                        return;
                    }
                }
                let n = self.state.minions[minion.index()].corruption_of(s);
                self.state.burn_corruption(minion, s, n, s);
                let v = &mut self.state.minions[minion.index()];
                if v.owner != s {
                    let origin = v.origin.or(v.temp_from).unwrap_or(v.owner);
                    v.origin = (origin != s).then_some(origin);
                    v.temp_from = None;
                    v.owner = s;
                    if v.is_ally() {
                        self.enter_card_contest(minion);
                    } else {
                        self.enter_title_contest(minion);
                    }
                }
            }
            (Interrupt::MakeAnarch { card, minion, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.make_anarch(minion);
                let c = &mut self.state.cards[card.index()];
                c.counters -= 1;
                if c.counters == 0 {
                    self.burn_card_in_play(card);
                }
            }
            (Interrupt::LockForPool { card, n, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.state.cards[card.index()].locked = true;
                let throne = Cause::Effect(Source::Card(card));
                self.caused(throne, |e| e.state.seats[s.index()].pool += n as i32);
            }
            // Burned with its minion or not (PIB 20150512): the pool all
            // the same.
            (Interrupt::PoolForCard { card, n, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.burn_card_in_play(card);
                let gain = Cause::Effect(Source::Card(card));
                self.caused(gain, |e| e.state.seats[s.index()].pool += n as i32);
            }
            (Interrupt::ExchangeAnarch { ash: None, .. }, Choice::FindCard(card)) => {
                self.interrupts[0] = Interrupt::ExchangeAnarch {
                    seat: s,
                    ash: Some(card),
                };
            }
            (
                Interrupt::ExchangeAnarch {
                    ash: Some(card), ..
                },
                Choice::Discard(i),
            ) => {
                self.interrupts.remove(0);
                let seat = &mut self.state.seats[s.index()];
                let (held, held_copy) = seat.hand.take(i as usize);
                let k = seat
                    .ash
                    .iter()
                    .position(|&c| c == card)
                    .expect("named ash card");
                let (_, copy) = seat.ash.take(k);
                seat.ash.put(held, held_copy);
                seat.hand.put(card, copy);
            }
            (Interrupt::ChooseForCard { .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.set_chosen(front, Some(m));
            }
            (Interrupt::AshurPicks { to_library, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                if to_library {
                    self.state.seats[s.index()].library.shuffle(&mut self.rng);
                }
            }
            (
                Interrupt::AshurPicks {
                    seat,
                    left,
                    to_library,
                },
                Choice::FindCard(card),
            ) => {
                let st = &mut self.state.seats[seat.index()];
                let i = st
                    .ash
                    .iter()
                    .position(|&c| c == card)
                    .expect("in the ash heap");
                let (card, copy) = st.ash.take(i);
                if to_library {
                    st.library.put(card, copy);
                } else {
                    st.hand.put(card, copy);
                }
                self.interrupts[0] = Interrupt::AshurPicks {
                    seat,
                    left: left - 1,
                    to_library: true,
                };
            }
            (
                Interrupt::AnarchConvert {
                    convert,
                    chosen: None,
                    ..
                },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
                self.stage_contests(convert);
            }
            (
                Interrupt::AnarchConvert {
                    seat,
                    convert,
                    forced,
                    chosen: None,
                },
                Choice::ChooseMinion(m),
            ) => {
                // Removed from the game, the cards on him with him (none
                // yet: he is entering play).
                self.remove_minion(convert);
                if let Some(card) = self.state.minions[convert.index()].crypt_card() {
                    let copy = *self.state.minions[convert.index()].copy;
                    self.state.seats[seat.index()].removed_crypt.put(card, copy);
                }
                self.make_anarch(m);
                self.interrupts[0] = Interrupt::AnarchConvert {
                    seat,
                    convert,
                    forced,
                    chosen: Some(m),
                };
            }
            (Interrupt::AnarchConvert { seat, convert, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                let anarch = Cause::Effect(Source::Minion(convert));
                self.caused(anarch, |e| e.state.seats[seat.index()].pool += 1);
            }
            // From an empty crypt, nothing (LSJ 20060620).
            (Interrupt::AnarchConvert { seat, .. }, Choice::DrawCrypt) => {
                self.interrupts.remove(0);
                let st = &mut self.state.seats[seat.index()];
                if let Some((card, copy)) = st.crypt.take_top() {
                    st.uncontrolled.push(crate::state::UncontrolledSlot {
                        card,
                        copy: crate::copies::Untracked(copy),
                        counters: 0,
                        banished: None,
                    });
                }
            }
            (Interrupt::BecomesAnarch { .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.make_anarch(m);
            }
            (Interrupt::UncontrolledBlood { n, .. }, Choice::Slot(i)) => {
                self.interrupts.remove(0);
                self.state.add_uncontrolled_blood(s, i as usize, n);
            }
            (Interrupt::UncontrolledBlood { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::MayRecycleCanceled { seat, card, .. }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                let s = &mut self.state.seats[seat.index()];
                let i = s
                    .ash
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("in the ash heap");
                let (_, copy) = s.ash.take(i);
                s.library.put(card, copy);
                self.state.seats[seat.index()]
                    .library
                    .shuffle(&mut self.rng);
            }
            (Interrupt::MayRecycleCanceled { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (
                Interrupt::NameSlotPair {
                    seat,
                    of,
                    from: None,
                },
                Choice::Slot(i),
            ) => {
                self.interrupts[0] = Interrupt::NameSlotPair {
                    seat,
                    of,
                    from: Some(i),
                };
            }
            (
                Interrupt::NameSlotPair {
                    seat,
                    of,
                    from: Some(f),
                },
                Choice::Slot(i),
            ) => {
                self.interrupts.remove(0);
                let named = |s: SeatId, k: u8| {
                    (k, self.state.seats[s.index()].uncontrolled[k as usize].card)
                };
                let pair = (named(of, f), named(seat, i));
                self.action.as_mut().expect("action").slot_pair = Some(pair);
            }
            (Interrupt::NameUncontrolled { seat, .. }, Choice::Slot(i)) => {
                self.interrupts.remove(0);
                let card = self.state.seats[seat.index()].uncontrolled[i as usize].card;
                self.action.as_mut().expect("action").uncontrolled_named = Some((i, card));
            }
            (Interrupt::BeastDisciplines { minion, .. }, Choice::Slot(i)) => {
                let d = crate::cards::GREAT_BEAST_DISCIPLINES[i as usize];
                let v = &mut self.state.minions[minion.index()];
                if let Some(slot) = v.beast_disciplines.iter_mut().find(|s| s.is_none()) {
                    *slot = Some(d);
                }
                if v.beast_disciplines.iter().all(|s| s.is_some()) {
                    self.interrupts.remove(0);
                }
            }
            (Interrupt::StoreOwnAsh { seat, card: on }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                let ash = &mut self.state.seats[seat.index()].ash;
                let i = ash
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("in the ash heap");
                let (_, copy) = ash.take(i);
                self.state.cards[on.index()].stored.put(card, copy);
            }
            (Interrupt::StoreOwnHand { seat, card: on }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                let hand = &mut self.state.seats[seat.index()].hand;
                let i = hand.iter().rposition(|&c| c == card).expect("in hand");
                let (_, copy) = hand.take(i);
                self.state.cards[on.index()].stored.put(card, copy);
                self.state.cards[on.index()].stored_owner = Some(seat);
                self.replace(seat, 1, None);
            }
            (Interrupt::StoredToLibraryTop { seat, card: on }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                let stored = &mut self.state.cards[on.index()].stored;
                let i = stored
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("on the location");
                let (_, copy) = stored.take(i);
                self.state.seats[seat.index()].library.put(card, copy);
            }
            (Interrupt::StorePreyAsh { seat, card: on }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                let prey = self.state.prey(seat);
                let ash = &mut self.state.seats[prey.index()].ash;
                let i = ash
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("in the prey's ash heap");
                let (_, copy) = ash.take(i);
                self.state.cards[on.index()].stored.put(card, copy);
                self.state.cards[on.index()].stored_owner = Some(prey);
                self.state.taken_across += 1;
            }
            (Interrupt::NameOwnAshCard { seat, .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                let a = self.action.as_mut().expect("action");
                a.ash_named = Some(card);
                if std::mem::take(&mut a.ash_to_hand_on_name) {
                    self.ash_named_to_hand(seat, Some(card));
                }
            }
            (Interrupt::NameOwnAshCard { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.action.as_mut().expect("action").ash_to_hand_on_name = false;
            }
            (Interrupt::NameAshVampire { .. }, Choice::AshVampire(card)) => {
                self.interrupts.remove(0);
                self.action.as_mut().expect("action").ash_vampire_named = Some(card);
            }
            (Interrupt::NameAshPair { .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                self.action.as_mut().expect("action").ash_named = Some(card);
            }
            (Interrupt::AllyFromAsh { named, .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                if named {
                    self.action.as_mut().expect("action").ash_named = Some(card);
                } else {
                    self.raise_ash_ally(s, card, false);
                }
            }
            (Interrupt::MoveCardToTitled { card, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.state.set_host(card, m);
            }
            (Interrupt::MoveOffBurnedBearer { card, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.state.set_host(card, m);
            }
            (Interrupt::SearchEquipFor { seat, minion }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                let library = &mut self.state.seats[seat.index()].library;
                let k = library
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("found card in the library");
                let (_, copy) = library.take(k);
                self.state.seats[seat.index()]
                    .library
                    .shuffle(&mut self.rng);
                let (pool, blood) = self.alastor_cost(seat, minion, card);
                self.state.burn_blood(minion, blood);
                // Put on, not equipped: its layers, inferior, nothing it
                // does as it is equipped (no cards stored, no minion
                // chosen: LSJ 20090506), no lock (LSJ 20090415).
                let entering =
                    crate::state::CardInPlay::entering(card, copy, seat, Some(minion), 0);
                if let Some(id) = self.place_now(entering, crate::plays::ModeLayers::Push, false) {
                    self.enter_library_contest(id);
                }
                // The last pool paid ousts (costs payable even so).
                let mut loss = vec![0i32; self.state.seats.len()];
                loss[seat.index()] = pool as i32;
                self.burn_pool_batch(&loss);
            }
            (Interrupt::SearchEquipFor { seat, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.seats[seat.index()]
                    .library
                    .shuffle(&mut self.rng);
            }
            (Interrupt::SecretNumber { seat, last, .. }, Choice::SetX(n)) => {
                self.interrupts.remove(0);
                self.secret_numbers.push((seat, n));
                if last {
                    self.reveal_numbers();
                }
            }
            (Interrupt::MoveOffBurnedBearer { card, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.burn_card_in_play(card);
            }
            (Interrupt::MoveForBlood { payer, n, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.state.minions[payer.index()].blood -= n;
                self.leave_torpor(m);
            }
            (Interrupt::RemoveUncontrolled { .. }, Choice::Slot(i)) => {
                self.interrupts.remove(0);
                self.remove_uncontrolled(s, i as usize);
            }
            (Interrupt::PursuitDiscard { seat, left }, Choice::Discard(i)) => {
                let st = &mut self.state.seats[seat.index()];
                let (card, copy) = st.hand.take(i as usize);
                st.ash.put(card, copy);
                self.interrupts[0] = Interrupt::PursuitDiscard {
                    seat,
                    left: left - 1,
                };
            }
            (Interrupt::MayDrawExtra { seat }, Choice::DrawLibrary) => {
                self.interrupts.remove(0);
                self.draw_top(seat);
            }
            (Interrupt::MayDrawExtra { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::PurgePick { host, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.purge_picks.push(m);
                self.finish_purge_if_done(host);
            }
            (Interrupt::MoveBloodUpTo { from, to, .. }, Choice::SetX(n)) => {
                self.interrupts.remove(0);
                let n = n.min(self.state.minions[from.index()].blood);
                self.state.minions[from.index()].blood -= n;
                self.state.minions[to.index()].blood += n;
            }
            (Interrupt::StealUpTo { from, to, .. }, Choice::SetX(n)) => {
                self.interrupts.remove(0);
                let n = n.min(self.state.minions[from.index()].blood);
                self.state.minions[from.index()].blood -= n;
                self.state.gain_blood(to, n);
            }
            (Interrupt::MayCancelCombat { minion, .. }, c) => {
                self.interrupts.remove(0);
                if c == Choice::CancelCombat {
                    if let Some(a) = self.action.as_mut() {
                        if a.blocked_by.contains(&minion) {
                            a.continued = true;
                        }
                    }
                    self.end_combat_now();
                }
            }
            (Interrupt::PowerSearch { minion, .. }, c) => {
                self.interrupts.remove(0);
                if let Choice::TakeFrom { zone, card } = c {
                    let copy = self.take_from_zone(s, zone, card);
                    let play = crate::cards::PlayCtx {
                        copy: crate::copies::Untracked(copy),
                        ..crate::cards::PlayCtx::new(
                            s,
                            card,
                            0,
                            Some(minion),
                            crate::cards::Target::Minion(minion),
                            0,
                        )
                    };
                    self.put_equipment(s, minion, play);
                    if zone == Zone::Hand {
                        self.replace(s, 1, None);
                    }
                }
                self.state.seats[s.index()].library.shuffle(&mut self.rng);
            }
            (Interrupt::NameConviction { .. }, Choice::BurnInPlay { card }) => {
                self.interrupts.remove(0);
                if let Some(a) = self.action.as_mut() {
                    a.conviction = Some(CardId(card));
                }
            }
            (Interrupt::AvengeImbued { acting, .. }, c) => {
                self.interrupts.remove(0);
                if let Choice::BurnInPlay { card } = c {
                    self.burn_card_in_play(CardId(card));
                    if self.state.minions[acting.index()].present() {
                        self.damage_out_of_combat(acting, 1, false);
                    }
                }
            }
            (Interrupt::BurnConvictions { .. }, Choice::BurnInPlay { card }) => {
                self.burn_card_in_play(CardId(card));
                if let Interrupt::BurnConvictions { n, .. } = &mut self.interrupts[0] {
                    *n -= 1;
                }
            }
            (
                Interrupt::GainConviction {
                    minion, library, ..
                },
                c,
            ) => {
                self.interrupts.remove(0);
                if let Choice::TakeFrom { zone, card } = c {
                    let copy = self.take_from_zone(s, zone, card);
                    self.gain_conviction(minion, card, copy, s);
                    if zone == Zone::Hand {
                        self.replace(s, 1, None);
                    }
                }
                if library {
                    self.state.seats[s.index()].library.shuffle(&mut self.rng);
                }
            }
            (Interrupt::AllySearch { minion, .. }, c) => {
                self.interrupts.remove(0);
                if let Choice::TakeFrom { zone, card } = c {
                    let (pool, blood) = self.searched_ally_cost(s, minion, card);
                    let copy = self.take_from_zone(s, zone, card);
                    self.state.minions[minion.index()].blood -= blood;
                    self.put_ally_in_play(s, card, copy);
                    if zone == Zone::Hand {
                        self.replace(s, 1, None);
                    }
                    self.state.seats[s.index()].library.shuffle(&mut self.rng);
                    // Its pool paid last: an oust ends the search.
                    if pool > 0 {
                        self.burn_pool_batch(&{
                            let mut loss = vec![0i32; self.state.seats.len()];
                            loss[s.index()] = pool;
                            loss
                        });
                    }
                } else {
                    self.state.seats[s.index()].library.shuffle(&mut self.rng);
                }
            }
            (Interrupt::ShortfallPicks { seat, from, mut to }, Choice::ChooseMinion(m)) => {
                if let Some(slot) = to.iter_mut().find(|t| **t == Some(m)) {
                    *slot = None;
                }
                self.state.burn_blood(from, 1);
                self.state.gain_blood(m, 1);
                self.interrupts[0] = Interrupt::ShortfallPicks { seat, from, to };
            }
            (Interrupt::MoveCardToOldest { card, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.move_title_card(card, m);
            }
            (
                Interrupt::PutInPlaySearch {
                    minion, vampire, ..
                },
                c,
            ) => {
                self.interrupts.remove(0);
                match c {
                    Choice::TakeFrom { zone, card } => {
                        let seat = &mut self.state.seats[s.index()];
                        let from = match zone {
                            Zone::Library => &mut seat.library,
                            Zone::AshHeap => &mut seat.ash,
                            Zone::Hand => unreachable!("Summon History searches no hand"),
                        };
                        let k = from.iter().rposition(|&x| x == card).expect("found card");
                        let (_, copy) = from.take(k);
                        self.shuffle_searched(s, false);
                        let lib = &crate::library::LIBRARY[card as usize];
                        // Put in play, not recruited, employed or equipped:
                        // inferior, able to act this turn (LSJ 20100204).
                        if lib.types.contains(&crate::library::CardType::Ally) {
                            self.put_ally_in_play(s, card, copy);
                        } else if lib.types.contains(&crate::library::CardType::Retainer) {
                            let play = crate::cards::PlayCtx {
                                copy: crate::copies::Untracked(copy),
                                ..crate::cards::PlayCtx::new(
                                    s,
                                    card,
                                    0,
                                    Some(minion),
                                    crate::cards::Target::Minion(minion),
                                    0,
                                )
                            };
                            self.put_retainer(s, minion, play, true);
                        } else {
                            let entering = CardInPlay::entering(card, copy, s, Some(minion), 0);
                            if let Some(id) =
                                self.place_now(entering, crate::plays::ModeLayers::Push, false)
                            {
                                self.enter_library_contest(id);
                            }
                        }
                    }
                    // The crypt's copy if there is one (as Legacy's merge).
                    Choice::AshVampire(card) => {
                        let seat = &mut self.state.seats[s.index()];
                        let (pile, k) = match seat.crypt.iter().position(|&c| c == card) {
                            Some(k) => (&mut seat.crypt, k),
                            None => {
                                let k = seat
                                    .ash_crypt
                                    .iter()
                                    .position(|&c| c == card)
                                    .expect("in the ash heap");
                                (&mut seat.ash_crypt, k)
                            }
                        };
                        let (_, copy) = pile.take(k);
                        self.shuffle_searched(s, true);
                        let forced = self.state.controls_copy(s, card);
                        let m = self.add_minion(s, card, 0, false);
                        self.state.minions[m.index()].copy = crate::copies::Untracked(copy);
                        self.vampire_entered(s, m, card, forced);
                    }
                    _ => self.shuffle_searched(s, vampire),
                }
            }
            (Interrupt::DisciplineSearch { minion, .. }, c) => {
                self.interrupts.remove(0);
                if let Choice::TakeFrom { zone, card } = c {
                    let copy = self.take_from_zone(s, zone, card);
                    self.put_on_minion(s, card, copy, minion);
                    // Out of the hand: drawn back up at once (advanced rules
                    // 6.8).
                    if zone == Zone::Hand {
                        self.replace(s, 1, None);
                    }
                }
                self.state.seats[s.index()].library.shuffle(&mut self.rng);
            }
            (Interrupt::Replacement { .. }, Choice::ResolveRider { card }) => {
                self.interrupts.remove(0);
                self.apply_replacement_rider(CardId(card));
            }
            (Interrupt::Replacement { .. }, Choice::DrawStored { card, stored }) => {
                self.interrupts.remove(0);
                self.take_stored(s, card, stored);
            }
            (Interrupt::DrawUp { .. }, Choice::DrawLibrary) => self.draw_top(s),
            (Interrupt::DrawUp { .. }, Choice::DrawStored { card, stored }) => {
                self.take_stored(s, card, stored)
            }
            (Interrupt::Draws { left, drawn, .. }, c) => {
                match c {
                    Choice::DrawLibrary => self.draw_top(s),
                    Choice::DrawStored { card, stored } => self.take_stored(s, card, stored),
                    other => unreachable!("illegal draw choice {other:?}"),
                }
                self.interrupts[0] = Interrupt::Draws {
                    seat: s,
                    left: left - 1,
                    drawn: drawn + 1,
                };
            }
            (Interrupt::Bottom { left, .. }, Choice::Bottom(i)) => {
                let seat = &mut self.state.seats[s.index()];
                let (card, copy) = seat.hand.take(i as usize);
                // Under the cards moved before it: the last one chosen is
                // the bottom card (rules-qa 9.5).
                seat.library.put_at(0, card, copy);
                self.interrupts[0] = Interrupt::Bottom {
                    seat: s,
                    left: left - 1,
                };
            }
            (Interrupt::StoreSearch { card, left, .. }, Choice::FindCard(found)) => {
                let library = &mut self.state.seats[s.index()].library;
                let k = library
                    .iter()
                    .rposition(|&c| c == found)
                    .expect("found card in the library");
                let (_, copy) = library.take(k);
                self.state.cards[card.index()].stored.put(found, copy);
                self.interrupts[0] = Interrupt::StoreSearch {
                    seat: s,
                    card,
                    left: left - 1,
                };
            }
            (Interrupt::StoreSearch { card, .. }, Choice::Pass) => {
                self.interrupts[0] = Interrupt::StoreSearch {
                    seat: s,
                    card,
                    left: 0,
                };
            }
            (Interrupt::EntryRemoveOrBurn { ally, .. }, c) => {
                self.interrupts.remove(0);
                let seat = &mut self.state.seats[s.index()];
                match c {
                    Choice::FindCard(card) => {
                        let k = seat
                            .ash
                            .iter()
                            .position(|&x| x == card)
                            .expect("in the ash heap");
                        let (_, copy) = seat.ash.take(k);
                        seat.removed.put(card, copy);
                    }
                    Choice::AshVampire(card) => {
                        let k = seat
                            .ash_crypt
                            .iter()
                            .position(|&x| x == card)
                            .expect("in the ash heap");
                        let (_, copy) = seat.ash_crypt.take(k);
                        seat.removed_crypt.put(card, copy);
                    }
                    Choice::Pass => self.burn_minion(ally),
                    other => unreachable!("illegal entry choice {other:?}"),
                }
            }
            (Interrupt::BlockToll { minion, n, .. }, Choice::PayBlockToll) => {
                self.interrupts.remove(0);
                self.state.burn_blood(minion, n);
            }
            (Interrupt::CancelAttempt { .. }, Choice::ChooseMinion(_)) => {
                self.interrupts.remove(0);
                self.resign_attempt();
            }
            (Interrupt::BlockToll { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.resign_attempt();
            }
            (Interrupt::MarginPool { .. }, Choice::AllocateTo { n, .. }) => {
                self.interrupts.remove(0);
                self.margin_to_pool(front, n);
            }
            (
                Interrupt::MarginBlood {
                    seat, left, clan, ..
                },
                Choice::ChooseMinion(m),
            ) => {
                self.state.gain_blood(m, 1);
                self.interrupts[0] = Interrupt::MarginBlood {
                    seat,
                    left: left - 1,
                    next: m.0 + 1,
                    clan,
                };
            }
            (
                Interrupt::MarginBlood {
                    seat, left, clan, ..
                },
                Choice::Pass,
            ) => {
                let skipped = self.margin_recipients(front)[0];
                self.interrupts[0] = Interrupt::MarginBlood {
                    seat,
                    left,
                    next: skipped.0 + 1,
                    clan,
                };
            }
            (Interrupt::DiscardFromHandOf { seat, of: None }, Choice::ChooseSeat(of)) => {
                self.interrupts[0] = Interrupt::DiscardFromHandOf { seat, of: Some(of) };
            }
            (Interrupt::DiscardFromHandOf { of: Some(of), .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                let st = &mut self.state.seats[of.index()];
                let i = st.hand.iter().position(|&c| c == card).expect("in hand");
                let (_, copy) = st.hand.take(i);
                st.ash.put(card, copy);
                self.replace(of, 1, None);
            }
            (Interrupt::AvoidDamage { minion, damage, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.damage_out_of_combat(minion, damage, true);
            }
            (Interrupt::AvoidDamage { minion, blood, .. }, Choice::BurnBloodToAvoid) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].blood -= blood;
            }
            // The chosen card to the next place down from the top.
            (
                Interrupt::Reorder {
                    seat,
                    total,
                    placed,
                },
                Choice::FindCard(card),
            ) => {
                let lib = &mut self.state.seats[seat.index()].library;
                let n = lib.len();
                let to = n - 1 - placed as usize;
                let from = (n - total as usize..=to)
                    .rev()
                    .find(|&k| lib[k] == card)
                    .expect("the chosen card among the top ones");
                let (c, copy) = lib.take(from);
                lib.put_at(to, c, copy);
                self.interrupts[0] = Interrupt::Reorder {
                    seat,
                    total,
                    placed: placed + 1,
                };
            }
            (
                Interrupt::DiscardChosen {
                    seat,
                    left,
                    done,
                    of,
                },
                Choice::Discard(i),
            ) => {
                let st = &mut self.state.seats[seat.index()];
                let (card, copy) = st.hand.take(i as usize);
                st.ash.put(card, copy);
                self.interrupts[0] = Interrupt::DiscardChosen {
                    seat,
                    left: left - 1,
                    done: done + 1,
                    of,
                };
            }
            (
                Interrupt::RemoveFromAsh {
                    seat,
                    of,
                    left,
                    pool,
                },
                c @ (Choice::FindCard(_) | Choice::AshVampire(_)),
            ) => {
                let st = &mut self.state.seats[of.index()];
                match c {
                    Choice::FindCard(card) => {
                        let i = st
                            .ash
                            .iter()
                            .position(|&x| x == card)
                            .expect("in the ash heap");
                        let (_, copy) = st.ash.take(i);
                        st.removed.put(card, copy);
                    }
                    Choice::AshVampire(card) => {
                        let i = st
                            .ash_crypt
                            .iter()
                            .position(|&x| x == card)
                            .expect("in the ash heap");
                        let (_, copy) = st.ash_crypt.take(i);
                        st.removed_crypt.put(card, copy);
                    }
                    _ => unreachable!(),
                }
                self.interrupts[0] = Interrupt::RemoveFromAsh {
                    seat,
                    of,
                    left: left - 1,
                    pool,
                };
            }
            (Interrupt::CorruptionOwner { seat, minion }, Choice::ChooseSeat(owner)) => {
                self.interrupts.remove(0);
                self.state.burn_corruption(minion, owner, 1, seat);
            }
            (Interrupt::MinionOf { seat, purpose, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                let v = &mut self.state.minions[m.index()];
                match purpose {
                    MinionPurpose::Lock
                    | MinionPurpose::LockVampire
                    | MinionPurpose::LockOldest(_) => v.locked = true,
                    MinionPurpose::HostCard(c) => self.state.set_host(c, m),
                    MinionPurpose::Damage(n) | MinionPurpose::DamageImbued { n, .. } => {
                        self.damage_out_of_combat(m, n, false)
                    }
                    // A vampire fed by another hunting ground this turn
                    // still counts as one of the two.
                    MinionPurpose::OldestHuntingGround(first) => {
                        if !v.hunting_ground_fed {
                            v.hunting_ground_fed = true;
                            self.state.gain_blood(m, 1);
                        }
                        if first.is_none() {
                            self.interrupts.insert(
                                0,
                                Interrupt::MinionOf {
                                    seat,
                                    of: seat,
                                    purpose: MinionPurpose::OldestHuntingGround(Some(m)),
                                },
                            );
                        }
                    }
                    MinionPurpose::UnlockAnarch => self.state.unlock(m),
                    MinionPurpose::BurnBloodOfClan(_) => v.blood -= 1,
                    MinionPurpose::Corrupt => v.add_corruption(seat, 1),
                    // Capacity 6 or more: that Methuselah ignores it for the
                    // rest of the game.
                    // Karen chosen: not burned (LSJ 20081120-2).
                    MinionPurpose::BurnOfCrowdedClan => {
                        if self.state.capacity_of(m).is_some_and(|c| c >= 6) {
                            self.state.seats[seat.index()].recalled_ignored = true;
                        }
                        if !self.state.gehenna_immune(m) {
                            self.burn_minion(m);
                        }
                    }
                }
            }
            (
                Interrupt::MoveBetween {
                    seat, from, how, ..
                },
                Choice::ChooseMinion(to),
            ) => {
                self.interrupts[0] = Interrupt::MoveBetween {
                    seat,
                    from,
                    to: Some(to),
                    blood_done: false,
                    how,
                };
            }
            // The excess over what `to` can hold goes to the bank.
            (
                Interrupt::MoveBetween {
                    seat,
                    from,
                    to: Some(to),
                    how,
                    ..
                },
                Choice::SetX(n),
            ) => {
                self.state.minions[from.index()].blood -= n;
                self.state.gain_blood(to, n);
                self.interrupts[0] = Interrupt::MoveBetween {
                    seat,
                    from,
                    to: Some(to),
                    blood_done: true,
                    how,
                };
            }
            (Interrupt::DiablerieEquipment { diablerist, .. }, Choice::ChooseCard(c)) => {
                self.state.set_host(CardId(c), diablerist);
            }
            (
                Interrupt::DiablerieEquipment {
                    diablerist,
                    victim,
                    diablerie,
                    ..
                },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
                self.taken_from_burns(diablerist, victim, diablerie);
            }
            (Interrupt::MoveBetween { to: Some(to), .. }, Choice::ChooseCard(c)) => {
                self.state.set_host(CardId(c), to);
            }
            (Interrupt::MoveBetween { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            // To Karen, the blood goes to the bank (LSJ 20081120-2).
            (Interrupt::GiveBlood { from, .. }, Choice::ChooseMinion(to)) => {
                self.interrupts.remove(0);
                self.state.minions[from.index()].blood -= 1;
                if !self.state.gehenna_immune(to) {
                    self.state.gain_blood(to, 1);
                }
            }
            (Interrupt::Redraw { seat, minion, card }, Choice::DrawLibrary) => {
                self.interrupts.remove(0);
                self.state
                    .turn_uses
                    .push(crate::state::once_redraw_key(minion));
                let st = &mut self.state.seats[seat.index()];
                let i = st
                    .hand
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("drawn card");
                let (card, copy) = st.hand.take(i);
                st.library.put_at(0, card, copy);
                // The new card is a library draw too, the use spent.
                if let Some((c, copy)) = st.library.take_top() {
                    st.hand.put(c, copy);
                }
            }
            (Interrupt::Redraw { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::MayGainPool { seat, n, minion }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                let own = Cause::Effect(Source::Minion(minion));
                self.caused(own, |e| e.state.seats[seat.index()].pool += n as i32);
            }
            (Interrupt::MayGainHandSize { seat, minion, n }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                self.state
                    .turn_uses
                    .push(crate::state::once_hand_size_key(minion));
                self.state.seats[seat.index()].timed.add(
                    crate::state::TimedStat::HandSize,
                    crate::state::Expiry::EndOfTurn,
                    n as i8,
                );
                self.draw_up(seat);
            }
            (Interrupt::GiveCardTo { card, .. }, Choice::ChooseSeat(to)) => {
                self.interrupts.remove(0);
                self.give_card_control(card, to);
            }
            (Interrupt::MayUnlock { minion, .. }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                self.state
                    .turn_uses
                    .push(crate::state::once_unlock_key(minion));
                self.effect_unlock(minion);
            }
            (Interrupt::MayPreventForBlood { minion, n, .. }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                self.state.burn_blood(minion, n);
                if let Some(c) = self.combat.as_mut() {
                    if let Some(i) = c.side_of_minion(minion) {
                        c.sides[i].prevent_strike(u8::MAX, true);
                    }
                }
            }
            (
                Interrupt::MayPutOn { .. }
                | Interrupt::MayAddCounter { .. }
                | Interrupt::MayStoreAim { .. }
                | Interrupt::MayTorture { .. }
                | Interrupt::MayGainPool { .. }
                | Interrupt::MayGainHandSize { .. }
                | Interrupt::MayPreventForBlood { .. }
                | Interrupt::MayUnlock { .. },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
            }
            (Interrupt::MayPutOn { held, .. }, Choice::ChooseMinion(_)) => {
                self.interrupts.remove(0);
                self.place_held(held, held.minion);
            }
            (Interrupt::MayStoreAim { seat, store, card }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                let ash = &mut self.state.seats[seat.index()].ash;
                // The one just used: the latest copy on the heap.
                let i = ash
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("in the ash heap");
                let (_, copy) = ash.take(i);
                self.state.cards[store.index()].stored.put(card, copy);
            }
            (Interrupt::MayAddCounter { card, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.state.cards[card.index()].counters += 1;
            }
            (Interrupt::MayTorture { card, minion, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.state.cards[card.index()].locked = true;
                self.state.minions[minion.index()].torture += 1;
            }
            // Not paid: burned.
            (Interrupt::PayOrBurn { card, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.burn_card_in_play(card);
            }
            (Interrupt::PayOrBurn { seat, pool, card }, Choice::PayToCancel) => {
                self.interrupts.remove(0);
                let upkeep = Cause::Cost(Source::Card(card));
                self.caused(upkeep, |e| e.pay_pool_for_ability(seat, pool as i32));
            }
            (Interrupt::BloodOrCamarilla { minion, .. }, Choice::BurnBloodToAvoid) => {
                self.interrupts.remove(0);
                self.state.burn_blood(minion, 1);
            }
            // Its title kept (LSJ 20010714's reading).
            (Interrupt::BloodOrCamarilla { minion, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].sect_override =
                    Some(crate::crypt::Sect::Camarilla);
            }
            (Interrupt::BloodOrBurned { minion, card, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                match card {
                    Some(c) => self.burn_card_in_play(c),
                    None => self.burn_minion(minion),
                }
            }
            (
                Interrupt::BloodOrBurned {
                    minion, n, card, ..
                },
                Choice::BurnBloodToAvoid,
            ) => {
                self.interrupts.remove(0);
                let kept = Cause::Cost(card.map_or(Source::Minion(minion), Source::Card));
                self.caused(kept, |e| e.state.burn_blood(minion, n));
            }
            // Karen is not locked (LSJ 20081120-2).
            (Interrupt::LockOrDiscard { minion, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                if !self.state.gehenna_immune(minion) {
                    self.state.minions[minion.index()].locked = true;
                }
            }
            (Interrupt::LockOrDiscard { seat, .. }, Choice::Discard(i)) => {
                self.interrupts.remove(0);
                let s = &mut self.state.seats[seat.index()];
                let (c, copy) = s.hand.take(i as usize);
                s.ash.put(c, copy);
                s.held[Release::MasterPhase] += 1;
            }
            (Interrupt::LockOrPay { card, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.state.cards[card.index()].locked = true;
            }
            (Interrupt::LockOrPay { seat, pool, card }, Choice::BurnPool) => {
                self.interrupts.remove(0);
                let kept = Cause::Cost(Source::Card(card));
                self.caused(kept, |e| e.pay_pool_for_ability(seat, pool as i32));
            }
            (Interrupt::PoolOrNoTransfers { seat, pool }, Choice::BurnPool) => {
                self.interrupts.remove(0);
                self.pay_pool_for_ability(seat, pool as i32);
            }
            // Other effects adding blood to its uncontrolled vampires stay
            // usable (LSJ 20100527).
            (Interrupt::PoolOrNoTransfers { seat, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.seats[seat.index()].no_transfers_next = true;
            }
            (
                Interrupt::PoolOrVampire {
                    seat, pool, card, ..
                },
                Choice::BurnPool,
            ) => {
                self.interrupts.remove(0);
                let kept = Cause::Cost(Source::Card(card));
                self.caused(kept, |e| e.pay_pool_for_ability(seat, pool as i32));
            }
            (Interrupt::PoolOrVampire { card, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.burn_minion(m);
                if self.state.cards[card.index()].present() {
                    self.burn_card_in_play(card);
                }
            }
            (Interrupt::CryptSearch { seat, .. }, Choice::AshVampire(card)) => {
                self.interrupts.remove(0);
                self.reveal(seat, card, true);
                let st = &mut self.state.seats[seat.index()];
                let k = st
                    .crypt
                    .iter()
                    .position(|&c| c == card)
                    .expect("in the crypt");
                let (card, copy) = st.crypt.take(k);
                st.uncontrolled.push(crate::state::UncontrolledSlot {
                    card,
                    copy: crate::copies::Untracked(copy),
                    counters: 0,
                    banished: None,
                });
                self.state.seats[seat.index()].crypt.shuffle(&mut self.rng);
            }
            (Interrupt::PutFoundOn { seat, minion, card }, Choice::FindCard(_)) => {
                self.interrupts.remove(0);
                self.put_found_on(seat, minion, card);
            }
            (Interrupt::PutFoundOn { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::LibraryToHand { seat, haven_on, .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                if let Some(minion) = haven_on.filter(|_| {
                    crate::library::LIBRARY[card as usize]
                        .labels
                        .contains(&crate::library::Label::Haven)
                }) {
                    self.interrupts
                        .insert(0, Interrupt::PutFoundOn { seat, minion, card });
                }
                self.reveal(seat, card, false);
                let s = &mut self.state.seats[seat.index()];
                let k = s
                    .library
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("found card in the library");
                let (_, copy) = s.library.take(k);
                s.hand.put(card, copy);
                s.library.shuffle(&mut self.rng);
            }
            (Interrupt::LibraryToHand { seat, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.seats[seat.index()]
                    .library
                    .shuffle(&mut self.rng);
            }
            (Interrupt::CryptSearch { seat, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.seats[seat.index()].crypt.shuffle(&mut self.rng);
            }
            (Interrupt::PreySlotToPool { of, .. }, Choice::Slot(i)) => {
                self.interrupts.remove(0);
                let st = &mut self.state.seats[of.index()];
                st.uncontrolled[i as usize].counters -= 1;
                st.pool += 1;
            }
            (
                Interrupt::DistributeFrom {
                    seat,
                    from,
                    clan,
                    left,
                },
                Choice::Slot(i),
            ) => {
                let own = Cause::Effect(Source::Minion(from));
                self.caused(own, |e| {
                    e.state.minions[from.index()].blood -= 1;
                    e.state.add_uncontrolled_blood(seat, i as usize, 1);
                });
                self.interrupts[0] = Interrupt::DistributeFrom {
                    seat,
                    from,
                    clan,
                    left: left - 1,
                };
            }
            (
                Interrupt::CircleBlood {
                    seat,
                    circle,
                    from,
                    equipment,
                    moving,
                    paired,
                    ..
                },
                Choice::ChooseMinion(m),
            ) => {
                let next = |from, to| Interrupt::CircleBlood {
                    seat,
                    circle,
                    from,
                    to,
                    equipment,
                    moving: None,
                    paired,
                };
                match (from, moving) {
                    (None, None) => self.interrupts[0] = next(Some(m), None),
                    (Some(f), _) => self.interrupts[0] = next(Some(f), Some(m)),
                    (None, Some(c)) => {
                        self.state.set_host(c, m);
                        self.interrupts[0] = next(None, None);
                    }
                }
            }
            // A move of n blood: one gain (Slake the Thirst answers one move).
            (
                Interrupt::CircleBlood {
                    seat,
                    circle,
                    from: Some(f),
                    to: Some(t),
                    equipment,
                    mut paired,
                    ..
                },
                Choice::SetX(n),
            ) => {
                self.state.burn_blood(f, n);
                let b = self.state.minions[t.index()].blood + n;
                self.state.set_blood(t, b);
                if let Some(slot) = paired.iter_mut().find(|p| p.is_none()) {
                    *slot = Some((f, t));
                }
                self.interrupts[0] = Interrupt::CircleBlood {
                    seat,
                    circle,
                    from: None,
                    to: None,
                    equipment,
                    moving: None,
                    paired,
                };
            }
            (
                Interrupt::CircleBlood {
                    seat,
                    circle,
                    equipment,
                    paired,
                    ..
                },
                Choice::ChooseCard(c),
            ) => {
                self.interrupts[0] = Interrupt::CircleBlood {
                    seat,
                    circle,
                    from: None,
                    to: None,
                    equipment,
                    moving: Some(CardId(c)),
                    paired,
                };
            }
            (Interrupt::BaseSearch { seat, minion }, choice) => {
                self.interrupts.remove(0);
                let sv = &mut self.state.seats[seat.index()];
                let card = match choice {
                    Choice::AshVampire(card) => {
                        if let Some(k) = sv.crypt.iter().position(|&c| c == card) {
                            Some(sv.crypt.take(k))
                        } else {
                            let k = sv
                                .ash_crypt
                                .iter()
                                .position(|&c| c == card)
                                .expect("in the ash heap");
                            Some(sv.ash_crypt.take(k))
                        }
                    }
                    Choice::Slot(i) => {
                        let slot = sv.uncontrolled.remove(i as usize);
                        Some((slot.card, *slot.copy))
                    }
                    _ => None,
                };
                self.state.seats[seat.index()].crypt.shuffle(&mut self.rng);
                if let Some((card, copy)) = card {
                    self.merge(minion, card, copy);
                }
            }
            (Interrupt::PoolToVampires { seat, to: None }, Choice::ChooseMinion(m)) => {
                self.interrupts[0] = Interrupt::PoolToVampires { seat, to: Some(m) };
            }
            // Paid down to 0, the mover is ousted (its turn passes).
            (Interrupt::PoolToVampires { seat, to: Some(t) }, Choice::SetX(n)) => {
                self.interrupts[0] = Interrupt::PoolToVampires { seat, to: None };
                let b = self.state.minions[t.index()].blood + n;
                self.state.set_blood(t, b);
                self.pay_pool_for_ability(seat, n as i32);
            }
            (Interrupt::PoolToVampires { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::CircleBlood { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::ChooseCircle { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::ChooseCircle { minion, .. }, Choice::ChooseCircle(c)) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].chosen_circle = Some(c);
            }
            (Interrupt::ReadyTitledLeft { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (
                Interrupt::ReadyTitledLeft { seat, .. },
                Choice::Play {
                    card,
                    mode,
                    actor,
                    target,
                },
            ) => {
                self.interrupts.remove(0);
                let resume = self.flow.clone();
                self.announce_play(seat, card, mode, actor, target, resume);
            }
            (Interrupt::NameMasterDiscipline { .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                if let Some(a) = self.action.as_mut() {
                    a.named_master = Some(card);
                }
            }
            (
                Interrupt::NameAshEquipment {
                    seat,
                    minion,
                    from: None,
                },
                Choice::ChooseSeat(s),
            ) => {
                self.interrupts[0] = Interrupt::NameAshEquipment {
                    seat,
                    minion,
                    from: Some(s),
                };
            }
            (Interrupt::NameAshEquipment { from: Some(s), .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                if let Some(a) = self.action.as_mut() {
                    a.ash_pick = Some((s, card));
                }
            }
            (Interrupt::BuyTitle { minion, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.stage_contests(minion);
            }
            (
                Interrupt::BuyTitle {
                    seat,
                    minion,
                    offers,
                },
                Choice::SetX(n),
            ) => {
                self.interrupts.remove(0);
                let title = offers
                    .iter()
                    .find(|&&(p, _)| p == n)
                    .map(|&(_, t)| t)
                    .expect("an offered price");
                let mut loss = vec![0i32; self.state.seats.len()];
                loss[seat.index()] = n as i32;
                let price = Cause::Cost(Source::Minion(minion));
                self.caused(price, |e| e.burn_pool_batch(&loss));
                if !self.state.seats[seat.index()].ousted {
                    self.gain_title(minion, title, None);
                    self.stage_contests(minion);
                }
            }
            (Interrupt::DistributeFrom { from, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.stage_contests(from);
            }
            (Interrupt::EntryBurn { .. }, Choice::BurnInPlay { card }) => {
                self.interrupts.remove(0);
                self.burn_card_in_play(CardId(card));
            }
            (front, choice) => unreachable!("illegal choice {choice:?} for {front:?}"),
        }
    }

    /// `seat`'s Blood Brothers of `circle` in play (in torpor too).
    pub(crate) fn circle_members(&self, seat: SeatId, circle: &'static str) -> Vec<MinionId> {
        self.state
            .minions_of(seat)
            .filter(|&m| {
                let v = &self.state.minions[m.index()];
                v.clan() == Some("Blood Brother") && v.circle() == Some(circle)
            })
            .collect()
    }

    /// The ready `clan` vampires below capacity the Methuselah controls,
    /// from minion `next` on (Amici Noctis).
    fn margin_recipients(&self, front: Interrupt) -> Vec<crate::ids::MinionId> {
        let (s, from, clan) = match front {
            Interrupt::MarginPool { seat, clan, .. } => (seat, 0, clan),
            Interrupt::MarginBlood {
                seat, next, clan, ..
            } => (seat, next, clan),
            _ => unreachable!("not a distribution"),
        };
        let st = &self.state;
        st.minions_of(s)
            .filter(|m| {
                let v = &st.minions[m.index()];
                m.0 >= from
                    && v.ready()
                    && v.clan() == Some(clan)
                    && st.capacity_of(*m).is_some_and(|c| v.blood < c)
            })
            .collect()
    }

    /// The pool amounts Amici Noctis may give: at most its cap, at least
    /// what the vampires cannot take.
    fn margin_pool_range(&self, front: Interrupt) -> Vec<u8> {
        let Interrupt::MarginPool { left, cap, .. } = front else {
            unreachable!("not the pool step");
        };
        let room = self.margin_recipients(front).len().min(u8::MAX as usize) as u8;
        let hi = left.min(cap);
        let lo = left.saturating_sub(room).min(hi);
        (lo..=hi).collect()
    }

    /// `n` to the pool; the rest goes to the vampires.
    fn margin_to_pool(&mut self, front: Interrupt, n: u8) {
        let Interrupt::MarginPool {
            seat, left, clan, ..
        } = front
        else {
            unreachable!("not the pool step");
        };
        self.state.seats[seat.index()].pool += n as i32;
        if left > n {
            self.interrupts.insert(
                0,
                Interrupt::MarginBlood {
                    seat,
                    left: left - n,
                    next: 0,
                    clan,
                },
            );
        }
    }

    /// The allies (by card) and retainers `s` controls, other than the
    /// entering `ally`: what War Ghoul's entry may burn.
    fn entry_burn_options(&self, s: SeatId, ally: crate::ids::MinionId) -> Vec<Choice> {
        let st = &self.state;
        st.cards
            .iter()
            .enumerate()
            .filter(|&(i, c)| {
                c.present()
                    && c.body != Some(ally)
                    && st.card_controller(CardId(i as u32)) == s
                    && c.bearer().is_some_and(|m| st.minions[m.index()].present())
                    && crate::cards::CardFilter::AllyOrRetainer.matches(c.card)
            })
            .map(|(i, _)| Choice::BurnInPlay { card: i as u32 })
            .collect()
    }

    /// Goblinism [MYT]: the haven still in `seat`'s hand, `minion` with no
    /// haven yet ("a minion can have only one haven"), its cost payable.
    fn may_put_found_on(&self, seat: SeatId, minion: crate::ids::MinionId, card: u16) -> bool {
        let st = &self.state;
        let (pool, blood) = self.found_on_cost(seat, minion, card);
        st.minions[minion.index()].present()
            && st.seats[seat.index()].hand.contains(&card)
            && !st.holds_a(minion, crate::cards::defs::ONE_HAVEN)
            && st.seats[seat.index()].pool >= pool
            && st.minions[minion.index()].blood >= blood
    }

    /// The pool and blood the haven costs "as normal" (Therbold Realty's
    /// discount included).
    fn found_on_cost(&self, seat: SeatId, minion: crate::ids::MinionId, card: u16) -> (i32, u8) {
        let st = &self.state;
        let pool = crate::cards::pool_cost(st, seat, card, crate::cards::Target::Minion(minion));
        let blood =
            crate::cards::blood_cost(st, card, 0).saturating_sub(st.equip_discount(minion, card));
        (pool.max(0), blood)
    }

    /// Put the haven from `seat`'s hand on `minion`, its cost paid: in play
    /// with its own layers, not played.
    fn put_found_on(&mut self, seat: SeatId, minion: crate::ids::MinionId, card: u16) {
        let (pool, blood) = self.found_on_cost(seat, minion, card);
        let s = &mut self.state.seats[seat.index()];
        let i = s
            .hand
            .iter()
            .rposition(|&c| c == card)
            .expect("the haven in hand");
        let (_, copy) = s.hand.take(i);
        self.paying(|e| {
            e.state.seats[seat.index()].pool -= pool;
            e.state.minions[minion.index()].blood -= blood;
        });
        self.note_pool_burned(seat);
        let placed = crate::state::CardInPlay::entering(card, copy, seat, Some(minion), 0);
        self.place_now(placed, crate::plays::ModeLayers::Push, false);
    }
}

/// The clan and sect a title card moves among (Osebo Kholo).
fn moving_title(card: u16) -> Option<(&'static str, crate::crypt::Sect)> {
    cards::lookup(card)?.modes.iter().find_map(|m| {
        m.effects.iter().find_map(|e| match *e {
            Effect::MovesToOldestUntitled { clan, sect } => Some((clan, sect)),
            _ => None,
        })
    })
}
