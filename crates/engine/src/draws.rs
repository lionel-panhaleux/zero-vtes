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

/// What brings an extra draw (`Interrupt::MayDrawExtra`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrawSource {
    Learjet,
    /// A vampire's text (Ira Rivers, Duality).
    Vampire(crate::ids::MinionId),
    /// A card's "can draw n cards", all or none (Pandora's Whisper [myt]).
    Card {
        card: u16,
        n: u8,
    },
}

/// The extra draws one card brings (`Engine::extra_draws`).
#[derive(Clone, Copy, Debug, Default)]
pub struct ExtraDraws {
    /// As the card is played, whatever becomes of its replacement.
    pub on_play: Option<DrawSource>,
    /// With its replacement, if that draws.
    pub on_replace: [Option<DrawSource>; 2],
}

/// What the hidden picks of `Interrupt::SecretNumber` are revealed for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretGame {
    /// Numbers (Game of Malkav's reveal; Malkavian Prank's guesses).
    Numbers,
    /// Rock (1), paper (2), scissors (3) between `player` and `other`
    /// (Cracking the Wall, Malkavian Game).
    RockPaperScissors {
        player: SeatId,
        other: SeatId,
        stakes: crate::cards::RpsStakes,
    },
}

/// Whom an `Interrupt::CircleBlood` moves blood and equipment among.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveAmong {
    /// A circle's Blood Brothers, from any to any (Redistribution, Jerry).
    Circle(&'static str),
    /// From this minion to its Methuselah's other ready vampires (Rave).
    From(MinionId),
}

/// A pending choice about cards drawn, moved or stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interrupt {
    /// Fear of Mekhet moves to another justicar or Inner Circle member, in
    /// torpor too, else burns (ANK 20210109; rules-qa 49.4: not locked).
    MoveCardToTitled { seat: SeatId, card: CardId },
    /// Alastor: `seat` may search their library for an equipment to put on
    /// `minion`, requirements ignored, inferior, half its cost rounded down
    /// (the blood the vampire's, X as 0: LSJ 20040518, 20100204); shuffled
    /// afterward, found or not (LSJ 20050331-2). `linked`: Bauble's, at no
    /// cost, of the filters, the card found its chosen one.
    SearchEquipFor {
        seat: SeatId,
        minion: crate::ids::MinionId,
        linked: Option<(CardId, &'static [crate::cards::CardFilter])>,
    },
    /// Jara Drory: `card` looked at in `of`'s hand; `minion` may burn 1
    /// blood to exchange it with a card of `of`'s ash heap (`FindCard`).
    ExchangeLooked {
        seat: SeatId,
        of: SeatId,
        card: u16,
        minion: crate::ids::MinionId,
    },
    /// Aisata Swanou: `seat` moves a hand card to its ash heap (`Discard`)
    /// or the top of its library (`TakeFrom` the hand).
    HandCardAway { seat: SeatId },
    /// Nkishi: `seat` may take a card of these kinds from its library or
    /// ash heap onto `minion`; with `from_hand`, from its hand or library,
    /// `left` cards, or `left` pairs of `held`'s two kinds, the kind `owed`
    /// to close a pair offered alone (Static Virtue; LSJ 20051127).
    SearchOnto {
        seat: SeatId,
        minion: crate::ids::MinionId,
        held: &'static [crate::cards::Holding],
        from_hand: bool,
        left: u8,
        paired: bool,
        owed: Option<crate::cards::Holding>,
    },
    /// Familial Bond: its Laibon's block failed; the acting Methuselah
    /// `seat` may make the action of `minion` fail.
    MayFailAction {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Familial Bond: the action successful, `minion` may lock to enter
    /// combat with `acting`.
    MayLockToFight {
        seat: SeatId,
        minion: crate::ids::MinionId,
        acting: crate::ids::MinionId,
    },
    /// Rachel Brandywine: as `seat`'s discard phase ends, the cards it
    /// discarded in it (`Seat::phase_discards`) may go to its library,
    /// shuffled (`resolve_printed`), or pass; the turn then ends.
    MayReturnDiscards {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Lucas Halton blocked: `minion` burns a blood instead of locking
    /// (`resolve_printed`), or passes and locks.
    MayBurnForBlock {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// Kurt Densch: `seat` looked at its library's top card; `minion` may
    /// burn `n` blood to move it to the bottom (`resolve_printed`), or
    /// pass.
    MayBottomLooked {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// Bamba: `seat` may move `n` blood from `from` to the new vampire
    /// `to`.
    MayMoveBlood {
        seat: SeatId,
        from: crate::ids::MinionId,
        to: crate::ids::MinionId,
        n: u8,
    },
    /// Baba Yaga (`once_each_turn`), Code of Samiel, Anima Gathering's
    /// unlock phase: `minion`, locked, may unlock.
    MayUnlock {
        seat: SeatId,
        minion: crate::ids::MinionId,
        once_each_turn: bool,
    },
    /// `left` cards of the filter moved one at a time from `seat`'s ash
    /// heap to the top of their library (Unity), the bottom with `bottom`,
    /// `Pass` ending it with `up_to` (Academia de Campeões).
    AshToLibraryTop {
        seat: SeatId,
        left: u8,
        filter: crate::cards::CardFilter,
        bottom: bool,
        up_to: bool,
        /// Only the cards just discarded down (`Seat::top_discards`:
        /// Pandora's Whisper [MYT]).
        discarded: bool,
    },
    /// `seat` may cut their library once: the top n cards to the bottom
    /// (Unity; any n, LSJ 20060413).
    CutLibrary { seat: SeatId },
    /// Dual Form: several of `root`'s set left the ready region at once —
    /// `seat` keeps one of those still in play, the others burned
    /// (rules-qa 96.1).
    DualFormKept {
        seat: SeatId,
        root: crate::ids::MinionId,
        /// The first of them leaving, the one an engine before 2.16.0 kept
        /// (a replay's).
        first: crate::ids::MinionId,
    },
    /// Nergal: his card, announced without his reduction, unpayable now
    /// without it — he may take it (rules-qa 96.3).
    ReductionAtResolution {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Curse of Nitocris: `seat`, its controller, picks the Methuselah,
    /// any, who takes control of `card`.
    GiveCardTo { seat: SeatId, card: CardId },
    /// Pool burns owed by separate triggers of one event
    /// (`Engine::burn_order`): `seat`, the acting Methuselah, picks who
    /// burns next.
    BurnOrder { seat: SeatId },
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
    /// the `last` pick reveals them all at once, as `game` says.
    SecretNumber {
        seat: SeatId,
        max: u8,
        last: bool,
        game: SecretGame,
    },
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
    /// discards (not replaced until their master phase); with `of`, a card
    /// matching it, replaced at once (Pariah).
    LockOrDiscard {
        seat: SeatId,
        minion: MinionId,
        of: Option<CardFilter>,
    },
    /// Maabara: a library card of `seat`'s ash heap moved onto `card`.
    StoreOwnAsh { seat: SeatId, card: CardId },
    /// Storage Annex as it is played: a card of `seat`'s hand put face down
    /// on `card`, replaced; Gift of Proteus: `left` cards of the filter, one
    /// at a time, not replaced.
    /// Aaron Fletcher as he enters play: up to `left` more equipment cards
    /// of `seat`'s hand (replaced), then of their ash heap, face down on
    /// `card` (`FindCard`), `Pass` going on to the ash heap, then ending.
    StoreUpTo {
        seat: SeatId,
        card: CardId,
        left: u8,
        from_ash: bool,
    },
    StoreOwnHand {
        seat: SeatId,
        card: CardId,
        left: u8,
        of: crate::cards::CardFilter,
        replace: bool,
        /// Only the action's `earmarked` cards (Gift of Proteus).
        earmarked: bool,
        /// "You can": a `Pass` beside the cards (Shadow Court Satyr).
        optional: bool,
        /// Its cost paid by this vampire and `seat` as it is stored, only
        /// payable cards offered (Inscription).
        payer: Option<crate::ids::MinionId>,
    },
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
    /// A hand size risen: `left` cards still to draw, each so. A count, not
    /// a read of the size: a combat begun meanwhile may lower it again.
    /// Pandora's Whisper's draw after the others' too, `may_top` its [MYT]
    /// discards.
    DrawRise {
        seat: SeatId,
        left: u8,
        may_top: bool,
    },
    /// Heart of Nizchetus: `left` cards still to draw, `drawn` so far; the
    /// same number then goes to the bottom of the library.
    Draws { seat: SeatId, left: u8, drawn: u8 },
    /// `left` cards still to move from hand to the bottom of the library,
    /// to its top with `top` (Hourglass of the Mind [tem]).
    Bottom { seat: SeatId, left: u8, top: bool },
    /// A search onto `card` (Black Market Cache, Mokolé Blood): up to
    /// `left` more cards matching `filter`, the ash heap's too with `ash`.
    StoreSearch {
        seat: SeatId,
        card: CardId,
        left: u8,
        filter: CardFilter,
        ash: bool,
    },
    /// War Ghoul's entry burn: another ally or retainer, else itself (RTR
    /// 19961113).
    EntryBurn {
        seat: SeatId,
        ally: crate::ids::MinionId,
    },
    /// Orun and Aye over `host`'s capacity: its controller picks which
    /// kind burns first (rules-qa 109.2).
    BurnOverCapacity {
        seat: SeatId,
        host: crate::ids::MinionId,
    },
    /// Rotting Behemoth: remove an ally or vampire from the ash heap, or
    /// decline and it burns (LSJ 20030701-2).
    EntryRemoveOrBurn {
        seat: SeatId,
        ally: crate::ids::MinionId,
        /// Cards still to remove; `taken` removed so far (no burn then).
        left: u8,
        taken: u8,
        /// Any card, not only an ally or a vampire (Mulher de Branco).
        any: bool,
    },
    /// Corrupt Construction: `seat` removes an ally, retainer or vampire
    /// card of their ash heap or hand from the game (`TakeFrom`,
    /// `AshVampire`), `ally` gaining 1 life, and is asked again; or stops
    /// (`Pass`). A hand card is not replaced (not played).
    EntryRemoveForLife {
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
    /// at (Vaticination). `up_to` (left, done, filter): up to `left` more
    /// of the filter, a pass stopping, the `done` replaced once over
    /// (Alicia Barrows).
    DiscardFromHandOf {
        seat: SeatId,
        of: Option<SeatId>,
        up_to: Option<(u8, u8, crate::cards::CardFilter)>,
    },
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
        /// "Up to": a pass stops (Reliquary: Trinket).
        optional: bool,
        /// Replaced after combat, not now (The Death of My Conscience).
        hold: bool,
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
    /// with no as-played window: Childe of the Revolution's new vampire, a
    /// diablerist of an older vampire (advanced rules 4); `named`, one of
    /// these from the library only (Lord Ashton).
    DisciplineSearch {
        seat: SeatId,
        minion: crate::ids::MinionId,
        named: &'static [&'static str],
        /// Picks left, a pass ending them, the library shuffled after the
        /// last (Seterpenre's four).
        left: u8,
        /// Its vampire entering play: its contests staged once done (LSJ
        /// 20080526).
        entering: bool,
        /// From the hand only, a pass burning `minion` (Bima).
        hand_or_burn: bool,
        /// From the library and the ash heap only (Deviki Prasanta).
        no_hand: bool,
        /// From the library only (Husamettin).
        library_only: bool,
    },
    /// Learjet, Ira Rivers, Duality: `seat` may draw an additional card
    /// (`DrawLibrary`), the hand then discarded down; or pass.
    /// `replaces`: keyed to a replacement, dropped if that took no draw.
    MayDrawExtra {
        seat: SeatId,
        source: DrawSource,
        replaces: bool,
        /// The card it comes with: its draws go or stay together.
        group: u32,
    },
    /// Steely Tenacity: `seat` replaces `card` by taking it from its ash
    /// heap (`FindCard`) or by drawing (`DrawLibrary`); then the next such
    /// card due `at`, else the `rest` of its replacements due and the
    /// `draws` chosen so far, drawn at once, with up to `jet` of Learjet's
    /// draws: "each time you draw to replace", none for a card taken back.
    RetakeFromAsh {
        seat: SeatId,
        card: u16,
        copy: crate::copies::CopyId,
        rest: u8,
        draws: u8,
        jet: u8,
        except: Option<crate::state::CardId>,
        at: crate::state::Release,
    },
    /// Development: `seat` replaces it with a card of its research area
    /// (`FindCard`) or by drawing (`DrawLibrary`), `except` its own riders.
    ReplaceFromResearch {
        seat: SeatId,
        except: Option<crate::state::CardId>,
    },
    /// Lady Constancia: `seat` replaces the card it discarded, `except`,
    /// with another library card of its ash heap (`FindCard`), or as the
    /// rules do beside a replacement rider (`DrawLibrary`: Visit from the
    /// Capuchin).
    ReplaceFromAsh {
        seat: SeatId,
        except: (u16, crate::copies::CopyId),
    },
    /// A master: Discipline choosing its Discipline, put on rather than
    /// played (Agent of Power by Childe of the Revolution's search): `card`'s
    /// Discipline, one its host lacks at superior (`ChooseDiscipline`).
    PlacedDiscipline {
        seat: SeatId,
        card: crate::state::CardId,
        /// Only these, whatever the level (Reliquary: Akunanse Remains);
        /// empty, any.
        among: &'static [&'static str],
    },
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
    /// [MYT]'s blocked acting vampire; Sowing Dissension's, burning
    /// `blood` and unlocking if `unlock`.
    MayCancelCombat {
        seat: SeatId,
        minion: crate::ids::MinionId,
        blood: u8,
        unlock: bool,
        /// Burned to cancel it (Ghoul Escort).
        burn: Option<CardId>,
    },
    /// `n` damage on `minion` and on its Ilomba `retainer` at once: `seat`,
    /// its controller, picks which resolves first (`ChooseMinion` or
    /// `ChooseCard`; LSJ 20090617-2).
    RetainerDamageOrder {
        seat: SeatId,
        minion: crate::ids::MinionId,
        retainer: CardId,
        n: u8,
    },
    /// "[n CONVICTION]": imbued `minion` burns n of its conviction cards,
    /// one at a time, `seat` choosing among their names (RTR 20111202).
    BurnConvictions {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// "[X CONVICTION]" on an equipment found or named rather than played
    /// (Living Wood Staff through Moise Kasavubu or Angel of Berlin): `seat`
    /// burns `minion`'s convictions one at a time, each adding 1 to the
    /// card's X; a pass stops.
    XConvictions {
        seat: SeatId,
        minion: crate::ids::MinionId,
        card: CardId,
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
    /// put in play paid (Pass: none); the library shuffled. `recruit`: any
    /// ally of the library `minion` could recruit, its requirements and
    /// cost as normal, recruited by it (The Summoning [PRE]).
    AllySearch {
        seat: SeatId,
        minion: crate::ids::MinionId,
        recruit: bool,
    },
    /// Wider View: an uncontrolled crypt card removed, its counters to the
    /// bank; removed cards are public (LSJ 20080314).
    RemoveUncontrolled { seat: SeatId },
    /// Sleight of Hand [CHI]: one of the `last` cards of `from`'s ash heap
    /// matching `of`, just discarded, onto `minion` (`FindCard`), or none
    /// (Pass).
    MayPutDiscardedOnActor {
        seat: SeatId,
        from: SeatId,
        minion: crate::ids::MinionId,
        last: u8,
        of: crate::cards::CardFilter,
    },
    /// Tomb of Rameses III `card`: its chosen vampire to `seat`'s ready
    /// region (`ResolveRider`), or not (Pass).
    MayReadyChosen { seat: SeatId, card: CardId },
    /// Tomb of Rameses III `card`, its control changed: a vampire of the new
    /// controller `seat`'s uncontrolled region (`Slot`), if any.
    ChooseUncontrolledForCard { seat: SeatId, card: CardId },
    /// The Rack: a ready vampire of the new controller's, if any.
    ChooseForCard {
        seat: SeatId,
        card: CardId,
        /// Of this clan only (Toreador Grand Ball, Priority Contract).
        clan: Option<&'static str>,
        /// A second choice, other than the first (Toreador Grand Ball).
        second: bool,
        /// Any minion this Methuselah controls instead (Incriminating
        /// Videotape equipped by a search: LSJ 20090506).
        of: Option<SeatId>,
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
    /// "May take an equipment from it instead (discard the original
    /// equipment card)" (Topaz): `card`, just put on `minion`, swapped.
    SwapEquipFromPrey {
        seat: SeatId,
        minion: crate::ids::MinionId,
        play: crate::cards::PlayCtx,
    },
    /// Haven Hunting may burn for another combat of these two.
    MayNewCombat { seat: SeatId, card: CardId },
    /// Achilles' Heel: the circle chosen.
    ChooseCardCircle { seat: SeatId, card: CardId },
    /// A vampire burned a Red List minion: `seat` may search library, ash
    /// heap and hand for a trophy (`TakeFrom`, once: `searched`), move those
    /// in play not yet awarded (`ResolveRider`), or pass (rulebook "Red
    /// List").
    AwardTrophy {
        seat: SeatId,
        minion: crate::ids::MinionId,
        searched: bool,
    },
    /// Trophy: Chosen moved to `minion`: it takes its blood (`ResolveRider`)
    /// or burns the card (pass; LSJ 20070309-1).
    TakeChosen {
        seat: SeatId,
        minion: crate::ids::MinionId,
        card: CardId,
    },
    /// Blade Clot: a clot counter from `card` to `to`, or pass.
    MoveClot {
        seat: SeatId,
        card: CardId,
        to: crate::ids::MinionId,
    },
    /// A clot counter's unlock phase: `minion` burns (`ChooseMinion`), or goes
    /// to or stays in torpor (pass; LSJ 20080612-3).
    ClotFate {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Marie-Pierre may lock to enter combat with the opposing minion, in
    /// place of side `side`'s combatant, as that combat ends.
    MayJoinCombat {
        seat: SeatId,
        minion: crate::ids::MinionId,
        side: u8,
    },
    /// Cloak of Blood: `minion` may gain a level of a Discipline `victim`
    /// had (`ChooseDiscipline`), or pass.
    GainVictimDiscipline {
        seat: SeatId,
        minion: crate::ids::MinionId,
        victim: crate::ids::MinionId,
    },
    /// The action fails unless `minion` burns `n` blood (Detect Authority
    /// [ani]): `BurnBloodToAvoid`, or pass.
    BurnOrFail {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
        /// Not the action: this card burns instead (Drink the Blood of
        /// Ahriman).
        card: Option<CardId>,
        /// The card's player, hindering the action.
        by: SeatId,
        /// The blood moves onto `card` (Polaris Coach).
        to_card: bool,
    },
    /// The World's a Canvas [dom]: `seat` picks the crypt card of `of`'s
    /// uncontrolled region that burns half its blood.
    PreySlotHalf { seat: SeatId, of: SeatId },
    /// "That vampire may burn a blood to become anarch" (Jack Drake).
    BurnBloodToAnarch {
        seat: SeatId,
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
        /// What the counters are told added by.
        cause: crate::notes::Cause,
        /// Only a vampire of printed capacity above this (Social Ladder's
        /// "older", rules-qa 60.18).
        older_than: Option<u8>,
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
        /// To the top of the library once it is shuffled, not the hand
        /// (Scarlet Lore [myt]).
        top: bool,
        /// The found card shown to all (PIB 20121028: only when printed).
        shown: bool,
        /// "You can search" printed on this vampire (Porphyrion): first
        /// searched (`ResolvePrinted`) or not at all, no shuffle (Pass).
        may: Option<crate::ids::MinionId>,
    },
    /// Research: `left` more cards of the library, by card, face down to
    /// the research area (`FindCard`), or no more (Pass); the library
    /// shuffled after.
    ResearchSearch { seat: SeatId, left: u8 },
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
    /// Afifa, The Herald: `ChooseMinion` burns `blood` from a ready
    /// vampire of `seat`'s other than `except` that has some (LSJ
    /// 19971212), `BurnPool` burns `pool`.
    BloodOrPool {
        seat: SeatId,
        except: crate::ids::MinionId,
        blood: u8,
        pool: u8,
    },
    /// Bleeding the Vine: `ResolveRider` locks it (if unlocked), `BurnPool`
    /// pays.
    LockOrPay {
        seat: SeatId,
        card: CardId,
        pool: u8,
    },
    /// Disciple of Lazarus: a `clan` crypt card (None: any vampire, no
    /// imbued) to the uncontrolled region
    /// (`AshVampire`, a CRYPT index) with `blood` from the bank (Coroner's
    /// Contact), or none (Pass).
    /// `below`: capacity below it; `max_level`: that Discipline at most at
    /// this level (Soul Scan).
    CryptSearch {
        seat: SeatId,
        clan: Option<&'static str>,
        blood: u8,
        below: Option<u8>,
        max_level: Option<(&'static str, u8)>,
        /// A crypt card of this name, any version, an imbued too (Gramle).
        name: Option<&'static str>,
        /// A scarce vampire only (The Stranger Among Us).
        scarce: bool,
        /// With this Discipline, either level (Summon the Serpent).
        with: Option<&'static str>,
    },
    /// Porto de Santos: one of `seat`'s top `n` crypt cards to their
    /// uncontrolled region (`AshVampire`), the others to the crypt's
    /// bottom.
    CryptTopPick { seat: SeatId, n: u8 },
    /// Solomon Batanea: the slot of `of`'s uncontrolled region that gives 1
    /// blood to `of`'s pool (`Choice::Slot`, `of`'s region).
    PreySlotToPool {
        seat: SeatId,
        of: SeatId,
        transfers: bool,
    },
    /// Erlik's entry: up to `left` more blood from `from` to uncontrolled
    /// `clan` vampires (`Slot`); contests staged after (LSJ 20080526).
    DistributeFrom {
        seat: SeatId,
        from: MinionId,
        clan: &'static str,
        left: u8,
    },
    /// Blood (and with `equipment`, equipment) moved `among` a circle's
    /// Blood Brothers (Jerry, Redistribution) or from one minion to the
    /// seat's other ready vampires (Rave): `from`, `to`, then the amount;
    /// each move a gain of its own (rules-qa 68.10), one per pair (`paired`).
    CircleBlood {
        seat: SeatId,
        among: MoveAmong,
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
    /// (`AshVampire`) or uncontrolled region (`Slot`), the crypt only if
    /// `crypt_only` (Gift of Experience); the crypt shuffled.
    BaseSearch {
        seat: SeatId,
        minion: crate::ids::MinionId,
        crypt_only: bool,
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
        /// One of "up to" several names (Pochtli): a pass ends them, the
        /// names gathered in `ActionCtx::ash_names`.
        upto: bool,
    },
    /// Hand of Conrad: the vampire of `seat`'s ash heap the action declared
    /// names (`Choice::AshVampire`); Augustus Giovanni: one `burned` from
    /// play (`Some(true)`: while the seat controlled it, since its last turn).
    NameAshVampire { seat: SeatId, burned: Option<bool> },
    /// Giovanni del Georgio: the card two copies of which in `of`'s ash
    /// heap the action declared names (`Terms::PreyAshPair`).
    NameAshPair { seat: SeatId, of: SeatId },
    /// Imogen: the library card of `of`'s ash heap the action declared
    /// names (`Effect::ExchangeRandomHandForNamedAsh`).
    NameTargetAshCard { seat: SeatId, of: SeatId },
    /// Klaus van der Veken: `seat` may cancel (`ResolveRider`) this use of
    /// `card`'s ability `ability`, a hunting ground of their prey's, the use
    /// spent; or let it resolve (`Pass`).
    MayCancelHuntingGround {
        seat: SeatId,
        minion: MinionId,
        card: CardId,
        ability: u8,
        target: crate::cards::Target,
        x: u8,
        user_seat: SeatId,
        user: Option<MinionId>,
    },
    /// Will of the Council: `seat` may burn `card` (`ResolveRider`) for its
    /// host to take `title`, a prince's gone to torpor, contested; or pass.
    MayTakeTitle {
        seat: SeatId,
        card: CardId,
        title: crate::crypt::Title,
    },
    /// The Becoming: `left` more master Discipline cards of `seat`'s hand
    /// put on `minion` (`TakeFrom` the hand), replaced, or no more (`Pass`).
    MastersFromHand {
        seat: SeatId,
        minion: crate::ids::MinionId,
        left: u8,
    },
    /// Scrounging: `seat` looked at the top `n` cards of their library and
    /// moves `left` more of them to hand (`FindCard`), or no more (`Pass`);
    /// the library then shuffled.
    LookedToHand { seat: SeatId, n: u8, left: u8 },
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
    /// St. Patrick's Cathedral: `seat` may move its library's top `n` cards
    /// (just reordered) to the bottom, in the same order (`ResolveRider`
    /// on `card`), or pass.
    MayBottomTop { seat: SeatId, card: CardId, n: u8 },
    /// The Path of Lilith: `seat` may lock `card` to put a torture counter
    /// on `minion` (`ResolveRider`), or pass.
    MayTorture {
        seat: SeatId,
        card: CardId,
        minion: MinionId,
    },
    /// Weirding Stone: the master card `seat` looked at on the active
    /// Methuselah's library, burned for `n` blood of `card`'s bearer
    /// (`ResolveRider`), or pass.
    MayBurnLookedMaster { seat: SeatId, card: CardId, n: u8 },
    /// Elysium: Sforzesco Castle: `seat` may lock `card` instead of its
    /// blocking `minion` (`ResolveRider`), or pass and lock the vampire.
    MayLockCardForBlock {
        seat: SeatId,
        card: CardId,
        minion: MinionId,
    },
    /// Cold Amber's Hold, Powerbase: Rome: `least` up to `max` blood
    /// (`SetX`) from `from` onto `card` as counters, which burns at
    /// `burn_at`.
    BloodToCardUpTo {
        seat: SeatId,
        from: MinionId,
        card: CardId,
        least: u8,
        max: u8,
        burn_at: Option<u8>,
    },
    /// The Holds: the card of type `of` in `from`'s ash heap `seat` moves
    /// onto `card`, face up (`Choice::FindCard`).
    StoreFromAshOf {
        seat: SeatId,
        card: CardId,
        from: SeatId,
        of: crate::library::CardType,
    },
    /// Mob Rule's resolution: `seat` burns 1 blood of a ready vampire of
    /// theirs of capacity above `min` for `votes` (`extra` more above
    /// `more`), or passes; around the table until `passes` Methuselahs in a
    /// row pass (LSJ 20030602).
    BloodForVotes {
        seat: SeatId,
        min: u8,
        more: u8,
        votes: u8,
        extra: u8,
        passes: u8,
    },
    /// Igoli's Loyalty: `seat` casts all its vampires' votes and ballots in
    /// favor for `offer` pool (`SetX`), or declines, the Ishtarri `by` then
    /// burning 1 blood to raise it for the next Methuselah with a ready
    /// titled vampire; it ends on an accept or a blood it cannot burn (LSJ
    /// 20051026: the acting Methuselah too, declining at will).
    FavorAuction {
        seat: SeatId,
        offer: u8,
        by: crate::ids::MinionId,
    },
    /// San Lorenzo de El Escorial, named as the action is announced (D1):
    /// the uncontrolled minion of `of`'s region (`Slot`), then the blood
    /// (`SetX`, 0 to `n`) `to` steals from it; with `burn`, the n it burns
    /// (Cairo Int'l Airport), no amount asked.
    StealFromSlot {
        seat: SeatId,
        of: SeatId,
        to: crate::ids::MinionId,
        n: u8,
        slot: Option<u8>,
        burn: bool,
        /// Only a card with no more blood than `to` has (Gisela Harden).
        affordable: bool,
    },
    /// Virolax Facility: a vampire of capacity below `below` from `seat`'s
    /// crypt (`AshVampire`) to their ready region with 1 blood, not acting
    /// this turn, or none (Pass); the crypt shuffled.
    CryptToReady { seat: SeatId, below: u8, blood: u8 },
    /// Tapestry of Blood: `seat` looked at the top `n` cards of `of`'s
    /// library and may burn one (`FindCard`), or pass.
    /// `filter` and `optional`: Scrying of Secrets [AUS]'s discard of a
    /// bleed action card (Tapestry: any, optional).
    BurnFromLibraryTop {
        seat: SeatId,
        of: SeatId,
        n: u8,
        filter: crate::cards::CardFilter,
        optional: bool,
    },
    /// Conniver, Curmudgeon: `seat` may lock `card` to move 1 blood from
    /// the bank to its vampire (`ResolveRider`), or pass.
    MayLockForBlood { seat: SeatId, card: CardId },
    /// The Realm of the Black Sun's optional "gain n pool": `ResolveRider`
    /// or Pass.
    MayGainPoolForCard { seat: SeatId, card: CardId, n: u8 },
    /// Matasuntha's optional "gain n pool": `ResolvePrinted` or Pass.
    MayGainPool {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// Dagger: equipped from hand, another copy from `seat`'s hand or ash
    /// heap may go on `minion` (`TakeFrom`), or none (Pass).
    SecondCopyOnto {
        seat: SeatId,
        minion: crate::ids::MinionId,
        card: u16,
    },
    /// A weapon out of play until the action's end that the action's end
    /// also burns (Dagger and Cleave): the active Methuselah returns it
    /// first, the burn then reaching it (`ResolveRider`), or burns first
    /// (Pass; LSJ 20100211-2).
    ReturnBeforeBurn { seat: SeatId, card: CardId },
    /// Blissful Agony [VAL] played by both combatants: the acting
    /// Methuselah picks the striker whose resolves first (`ChooseMinion`),
    /// the other lost (PIB 20120225, LSJ 20011215).
    RedirectFirst {
        seat: SeatId,
        strikers: [crate::ids::MinionId; 2],
    },
    /// Darkness Within beside a steal strike: the acting Methuselah has
    /// `card`'s drain first (`ResolveRider`) or the steals first (Pass;
    /// LSJ 20020301).
    DrainBeforeSteals { seat: SeatId, card: CardId },
    /// Off Kilter's "you can burn the Edge to gain n additional pool", its
    /// acting `minion`'s: `ResolvePrinted` or Pass.
    MayBurnEdgeForPool {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// Khazar's Diary's action announced: the minion card named in an ash
    /// heap, the heap first (`ChooseSeat`), then the card (`AshVampire` a
    /// crypt card, `FindCard` an ally card).
    NameAnyAshMinion { seat: SeatId, from: Option<SeatId> },
    /// Spell of Life with more copies in play than it burns: `seat` picks
    /// `left` more copies of `card` to burn (`ChooseCard`), anyone's (LSJ
    /// 20090428), then `RepresentPicks` for `up_to`.
    BurnCopiesPick {
        seat: SeatId,
        card: u16,
        left: u8,
        up_to: u8,
        of: &'static crate::state::Represent,
    },
    /// Retain the Quick Blood, two or more on `minion`: `seat` picks the one
    /// keeping the `n` blood it spent on `card` in mode `mode`
    /// (`ChooseCard`; rules-qa 121.127).
    KeepSpentBlood {
        seat: SeatId,
        minion: crate::ids::MinionId,
        card: u16,
        mode: u8,
        n: u8,
    },
    /// Two or more standing replacements of `minion`'s burn: `seat`, the
    /// acting Methuselah (else the active one), picks the one applying
    /// (`ResolvePrinted` its own, `ResolveRider` a card's; rules-qa
    /// 121.19); `to_torpor` as in `Engine::replace_burn`.
    BurnReplacement {
        seat: SeatId,
        minion: crate::ids::MinionId,
        to_torpor: bool,
    },
    /// Spell of Life: up to `left` more of `seat`'s uncontrolled vampires
    /// into play representing `of` (`Slot`), or no more (Pass).
    RepresentPicks {
        seat: SeatId,
        left: u8,
        of: &'static crate::state::Represent,
    },
    /// Precision [CEL]: `minion` may burn `blood` to burn `pool` of `of`'s
    /// pool (`ResolvePrinted`), or not (Pass).
    MayBurnBloodToBurnPool {
        seat: SeatId,
        minion: crate::ids::MinionId,
        of: SeatId,
        blood: u8,
        pool: u8,
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
    /// La Viuda Blanca: 1 up to `max` cards drawn (`SetX`), capped by the
    /// library, the hand then discarded down.
    DrawUpTo { seat: SeatId, max: u8 },
    /// Grotesque entering play: the location bound to `minion`
    /// (`ChooseCard`); none left, it is removed from the game (LSJ
    /// 20091216).
    BindLocation {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Lord Ephraim Wainwright: another copy of `card`, the action card he
    /// just played, may move from `seat`'s library to their ash heap, the
    /// library shuffled (`ResolvePrinted`).
    MayMillCopy {
        seat: SeatId,
        minion: crate::ids::MinionId,
        card: u16,
    },
    /// The active Methuselah may skip their entire turn, decided before the
    /// unlock phase (First Tradition: The Masquerade; ANK 20210110).
    MaySkipTurn { seat: SeatId },
    /// Instability: `seat` may take the Edge no Methuselah has
    /// (`ChooseSeat` itself), or not (Pass).
    MayTakeEdge { seat: SeatId },
    /// White Nights Massacre: `seat` burns a vampire in torpor
    /// (`ChooseMinion`), or every one by locking a ready werewolf ally of
    /// theirs (`ChooseMinion`) or discarding a copy of `card` (`Discard`),
    /// or none (Pass).
    TorporMassacre { seat: SeatId, card: u16 },
    /// Bounty: `seat` takes 0 to `max` counters from `card` to their pool
    /// (`SetX`); the card burns once it has none.
    TakeCountersUpTo { seat: SeatId, card: CardId, max: u8 },
    /// Malkavian Prank: `seat` guesses the number `of` holds, 1 to `max`
    /// (`SetX`); the `last` guess reveals them all.
    GuessNumber {
        seat: SeatId,
        of: SeatId,
        max: u8,
        last: bool,
    },
    /// Hostile Takeover's auction for `target`: `seat` outbids the `high`
    /// bid (`SetX`), up to their pool, twice it for the target's controller
    /// (LSJ 19990825), or passes; `passes` in a row since the last bid.
    Bid {
        seat: SeatId,
        target: crate::ids::MinionId,
        high: Option<(SeatId, u8)>,
        passes: u8,
    },
    /// A granted action's X, 1 to `max`, named as it is announced
    /// (`SetX`; The Ghouls of Plaza Morería, rules-qa 3.5).
    ActionX {
        seat: SeatId,
        minion: crate::ids::MinionId,
        max: u8,
    },
    /// Charming Lobby: the X of the card it named, chosen as the action
    /// succeeds (`SetX`; LSJ 20100819).
    NamedX {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Fata Amria, Putrefaction: `seat` may burn `card` instead of
    /// unlocking `minion` as normal (`BurnInPlay`), or unlock it (`Pass`).
    MayBurnInsteadOfUnlock {
        seat: SeatId,
        minion: crate::ids::MinionId,
        card: CardId,
        /// Passing unlocks it: nothing else holds it (Putrefaction).
        unlock: bool,
    },
    /// Target Hand: `seat` may burn a weapon `minion` has (`BurnInPlay`).
    MayBurnWeapon {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Target Head: `seat` may set the next round's range, `SetX(0)` close,
    /// `SetX(1)` long; Ananasi Vampirephile: sets it (`may` false, no Pass).
    MaySetNextRange { seat: SeatId, may: bool },
    /// `blocker`'s block is about to succeed: its controller `seat` may play
    /// a card for the moment (Lesser Boon), or pass (block resolution).
    BlockWouldSucceed {
        seat: SeatId,
        blocker: crate::ids::MinionId,
    },
    /// Paolo Sardenzo's optional "gain n blood" as a wraith enters play:
    /// `ResolvePrinted` or Pass.
    MayGainBlood {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// Auto-da-fé: `seat` burns `pool` (`BurnPool`, offered only while they
    /// have it: rules-digest D3) or `minion` is burned (`Pass`).
    PoolOrMinionBurned {
        seat: SeatId,
        minion: crate::ids::MinionId,
        pool: u8,
    },
    /// Courier: `seat` may lock `card` (`ResolveRider`) to burn the top card
    /// of `of`'s library, the one it looked at, or pass.
    MayLockToDiscardLooked {
        seat: SeatId,
        card: CardId,
        of: SeatId,
    },
    /// Skin Trap [VIC]: `minion` burns `n` blood now (`BurnBloodToAvoid`)
    /// or cannot strike this round (Pass).
    BloodOrNoStrike {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// Nickolai's action announced: the Discipline it gains, one another
    /// ready vampire of `seat`'s has at superior (`ChooseDiscipline`;
    /// `ActionCtx::named_discipline`, D1).
    NameActionDiscipline {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Rico Loco: a `card` from `seat`'s hand or ash heap moved onto
    /// `minion`, no cost (`TakeFrom`), or `minion` takes `damage` (Pass).
    NamedOntoOrDamage {
        seat: SeatId,
        minion: crate::ids::MinionId,
        card: u16,
        damage: u8,
    },
    /// Terrifisto, at his referendum's tally: `minion` burns 1 blood to
    /// inflict `n` damage on a ready vampire that cast votes against it
    /// (`ChooseMinion`), or not (Pass).
    DamageVoterForBlood {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// Lucubratio blocked: `seat` may discard a hand card of his printed
    /// filter instead of locking him (`Discard`), or pass and lock.
    MayDiscardForBlock {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Konrad Fleischer or Jalan-Aajav outside his combat: `minion`, about to
    /// take `n` aggravated damage, may treat some as normal (`ResolvePrinted`,
    /// Jalan for 1 blood), or takes it all aggravated (Pass); the damage waits
    /// for the answer, dealt by `cause` (owner, rules-qa 122.96).
    MayTakeAggravatedAsNormal {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
        cause: Option<crate::notes::Cause>,
    },
    /// Duma Rafiki, before the strikes resolve at close range: `seat`'s
    /// `retainer` burns a weapon `minion` has (`BurnInPlay`), or inflicts its
    /// damage on it (Pass).
    RetainerBurnsWeaponOrDamage {
        seat: SeatId,
        retainer: CardId,
        minion: crate::ids::MinionId,
    },
    /// Reliquary: Biague: `minion` may burn 1 blood (`ResolvePrinted`) for
    /// `seat` to look at `of`'s hand.
    MayBurnBloodToPeek {
        seat: SeatId,
        minion: crate::ids::MinionId,
        of: SeatId,
    },
    /// Ohoyo Hopoksia: `seat` may lock the ally whose `card` this is
    /// (`ResolveRider`) to fight `acting` instead of `replaced`, or not
    /// (Pass): a blocker before its combat, any other in the combat just
    /// begun (`Engine::settle_rush_stand_in`).
    CombatStandIn {
        seat: SeatId,
        card: CardId,
        acting: crate::ids::MinionId,
        replaced: crate::ids::MinionId,
    },
    /// Lazarene Inquisitor's "burn up to n blood": `from` burns 1 to `max`
    /// (`SetX`), chosen by `seat` as the action resolves (rules-qa 62.4).
    BurnBloodUpTo {
        seat: SeatId,
        from: MinionId,
        max: u8,
    },
    /// Muricia's Call, Children of Stone [vis]: a retainer of `seat`'s
    /// library (and hand with `hand`) matching `filter` that `minion`
    /// employs, its requirements and cost as normal less `off`, at the
    /// version its player chooses (`found_version`), `extra_life` more life
    /// (Pass: none); the library shuffled either way.
    RetainerSearch {
        seat: SeatId,
        minion: crate::ids::MinionId,
        filter: CardFilter,
        extra_life: u8,
        off: u8,
        hand: bool,
    },
    /// Olugbenga: `seat` pays its contest of vampire `minion` 1 at a time,
    /// `left` still owed (Democritus's toll), with 1 pool (`BurnPool`) or 1
    /// blood from it or from an Olugbenga of theirs (`BurnBloodFrom`;
    /// `Engine::contest_blood_sources`).
    ContestPayment {
        seat: SeatId,
        minion: crate::ids::MinionId,
        left: u8,
    },
    /// Zhenga's successful action: `seat` may recruit or employ up to `left`
    /// more allies and retainers from hand with `minion`, requirements and
    /// cost as normal, one at a time (`EmployFromHand`, Pass ending), not
    /// played in the normal way (LSJ 20060530-2); the `hired` cards are
    /// replaced once it ends (LSJ 20060530-1).
    ExtraHires {
        seat: SeatId,
        minion: crate::ids::MinionId,
        left: u8,
        hired: u8,
    },
    /// "As any Ravnos enters play, they can choose to become Camarilla"
    /// (Ravnos Acceptance, the other `Modifier::ClanMayJoinSect` cards):
    /// `seat` picks the entering `minion` to join `sect`, or passes.
    MayJoinSect {
        seat: SeatId,
        minion: crate::ids::MinionId,
        sect: crate::crypt::Sect,
    },
    /// Insurance Scam: `seat` burns locations it controls one at a time
    /// (`ChooseCard`), then gains as much pool as `burned` (pass).
    BurnLocationsForPool { seat: SeatId, burned: u8 },
    /// A trophy choosing a clan, moved to a vampire from out of play
    /// (Trophy: Clan Respect): `card`'s clan (`ChooseClan`).
    PlacedClan {
        seat: SeatId,
        card: crate::state::CardId,
    },
    /// Heather Florent: `seat`'s pool from the Edge moved to `minion` as
    /// blood (`ResolvePrinted`) or put in their pool (Pass).
    MayMoveEdgePool {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Patrizia Giovanni's action announced: the retainer of `seat`'s ash
    /// heap matching `of` that `minion` will employ, named (`FindCard`)
    /// among those it may employ, the action's `extra` blood more.
    NameAshRetainer {
        seat: SeatId,
        minion: crate::ids::MinionId,
        of: CardFilter,
        extra: u8,
    },
    /// The version `ctx`'s card enters at (`Choice::Version`), among the
    /// modes of `versions` (a bit a mode) its vampire's Disciplines meet,
    /// then `then` (owner, 2026-11-19: a searched or stored card's
    /// version is its player's choice, as from hand).
    FoundVersion {
        seat: SeatId,
        ctx: crate::cards::PlayCtx,
        versions: u16,
        then: crate::plays::CostThen,
    },
    /// Rabbat's hunt: 1 of the blood it gains goes to `seat`'s pool
    /// (`ResolvePrinted`), its capacity no bar (LSJ 20050331), or to her
    /// (Pass).
    MayHuntBloodToPool {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Brothers Grimm taken: its controller `seat` may discard a master
    /// card (`Discard`) to keep `card`, or pass and `taker` takes it.
    MayDiscardToKeep {
        seat: SeatId,
        card: crate::state::CardId,
        taker: SeatId,
    },
    /// Extremis Boon's auction to keep `target` in the game: `seat` bids
    /// (`SetX`) over the `high` bid, or passes for good (`out`).
    BoonAuction {
        seat: SeatId,
        target: SeatId,
        card: crate::state::CardId,
        high: Option<(SeatId, u8)>,
        out: crate::ids::SeatSet,
    },
    /// Celestial Harmony: `seat` keeps one hand card hidden (`Discard`, a
    /// first copy) and shows `viewer` the others.
    HideOneCard { seat: SeatId, viewer: SeatId },
    /// Harrod: `seat` looks at the card of `of`'s uncontrolled region they
    /// pick (`Slot`).
    LookPreySlot { seat: SeatId, of: SeatId },
    /// Path of Death and the Soul: `seat` may lock `card` and choose a ready
    /// vampire of theirs (`ChooseMinion`) to get `blood` from the bank and
    /// a Master: Discipline card searched (`DisciplineSearch`), or pass.
    MayLockForDisciplineSearch {
        seat: SeatId,
        card: CardId,
        blood: u8,
    },
    /// Kindred Coercion [DOM]: `seat` makes `minion` abstain (Pass) or
    /// changes its votes to a side (`CastVotes`), pick by pick.
    CoercedSide {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// The Art of Memory: `seat` moves a card `minion` played during the
    /// action from their ash heap to hand (`FindCard`), the action card
    /// still bound for it included (LSJ 20041115).
    PlayedCardToHand {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Agaitas: the played card's replacement drawn from `seat`'s own
    /// library (`DrawLibrary`) or its prey's (`ChooseSeat`), face up.
    PreyReplacement {
        seat: SeatId,
        except: Option<CardId>,
    },
    /// Jack Dawson, entering out of the uncontrolled region: a card of
    /// `filter` from `seat`'s hand equipped on `minion`, its cost paid
    /// (`EmployFromHand`), or pass; its contests staged after.
    EquipFromHandOnEntry {
        seat: SeatId,
        minion: crate::ids::MinionId,
        filter: crate::cards::CardFilter,
    },
    /// Qadir ul-Ghani: the clan, any other than his, his action names
    /// (`ChooseClan`), as it is declared.
    NameClan {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
}

/// What a minion chosen at resolution is for (`Interrupt::MinionOf`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinionPurpose {
    /// "One other unlocked ready anarch you control" (Power of All):
    /// mandatory.
    LockOtherAnarch(crate::ids::MinionId),
    /// "Each other Methuselah locks an unlocked ready younger vampire he or
    /// she controls" (Sowing Dissension): younger than the acting minion,
    /// mandatory.
    LockYoungerThan(crate::ids::MinionId),
    /// "You can burn an ally controlled by your prey whose cost is not
    /// greater than the bleed amount" (Keystone Kine): optional.
    BurnAllyUpTo(u8),
    /// "That anarch can move this card onto any minion" (Blood Turnip):
    /// mandatory, any Methuselah's, not that anarch.
    MoveOntoAny(CardId, crate::ids::MinionId),
    /// The ready minion of theirs the equipment goes to, its present one
    /// to keep it (Nod).
    Rearrange(CardId),
    /// "If Grey has only 1 life, any ready anarch may burn a blood to add 1
    /// life to him": optional.
    HealHost(crate::ids::MinionId),
    /// "An ally or younger vampire controlled by your prey" (Blind Spot),
    /// younger than the vampire it is chosen with: mandatory.
    BlindSpot(crate::ids::MinionId),
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
    /// locked or not (LSJ 20010508); "oldest" as the card's player reads
    /// capacities (Orun's).
    LockOldest(&'static str, SeatId),
    /// A vampire of a clan the user controls 3 or more of, burned
    /// (Recalled to the Founder): mandatory; its capacity as the card's
    /// player reads it.
    BurnOfCrowdedClan(SeatId),
    /// A ready minion of the chooser's, which takes n unpreventable damage
    /// (Gnaeus Aemilius Augustinus): mandatory.
    Damage(u8),
    /// A ready vampire of theirs takes n damage (Becoming of Ennoia).
    DamageVampire(u8),
    /// The minion the found equipment goes on (Repo Man): mandatory.
    EquipFoundOn {
        card: u16,
        copy: crate::copies::CopyId,
    },
    /// A vampire of theirs other than `not`, n of its blood moved to `to`
    /// (Loose Cannon [tha]): mandatory.
    BloodTo {
        to: crate::ids::MinionId,
        n: u8,
        not: crate::ids::MinionId,
    },
    /// A vampire in torpor, any Methuselah's, not chosen yet this action,
    /// younger than the actor (if `younger`) while no Gehenna event is in
    /// play; `steal` blood stolen from it (`Effect::StealFromTorpid`). The
    /// first is mandatory; `more` offers Pass.
    StealFromTorpid {
        actor: crate::ids::MinionId,
        more: bool,
        steal: u8,
        younger: bool,
        clan: &'static str,
    },
    /// Another ready imbued of the chooser's than `except`, which takes n
    /// unpreventable damage (Béatrice "Oracle171" Tremblay): mandatory.
    DamageImbued { n: u8, except: crate::ids::MinionId },
    /// Gurchon Hall: one of the two oldest ready vampires gains 1 blood,
    /// mandatory; a tie is the chooser's (rules-qa 69.1).
    OldestHuntingGround(Option<crate::ids::MinionId>),
    /// Any Methuselah's ready minion, marked for this vampire's blocks
    /// (Christopher Houghton): mandatory.
    BlockMark(crate::ids::MinionId),
    /// Another vampire of the hunter's controller's, ready or in torpor,
    /// gains the n blood the hunt held back; Pass, or none left, gives it
    /// to the hunter (Azrael, `Engine::hunt_gain`).
    HuntBloodTo { hunter: crate::ids::MinionId, n: u8 },
    /// One of the two vampires of a successful joint block, the blocking
    /// one (Coterie Tactics): mandatory.
    JointBlocker(crate::ids::MinionId, crate::ids::MinionId),
    /// A ready vampire of theirs takes n unpreventable damage from the ally
    /// `by`, none if immune to it (Wendell Delburton; ANK 20210228): mandatory.
    DamageVampireBy { n: u8, by: crate::ids::MinionId },
    /// A minion of theirs (as the `Holder` admits) able to hold this card,
    /// not its host, recorded to receive it as the action under way
    /// succeeds (Arcane Appraiser, Dauntain Black Magician): mandatory.
    HolderAtSuccess(CardId, crate::cards::Holder),
    /// As `Damage`, the damage a weapon's (Talbot's Chainsaw; LSJ
    /// 20090304-1): none on a minion immune to it (Mictlantecuhtli).
    WeaponDamage(u8),
    /// Another ready vampire of the chooser's than `except`, which takes n
    /// unpreventable damage (Shiloh Marie): mandatory.
    DamageOtherVampire { n: u8, except: crate::ids::MinionId },
    /// A ready vampire of the chooser's gains n blood, a full one legal
    /// (Club Zombie), or one `below_capacity` only (Doyle Fincher): mandatory.
    GainBlood { n: u8, below_capacity: bool },
    /// Another ready minion of the chooser's than `except`, which takes n
    /// damage (Rico Loco): mandatory.
    DamageOtherMinion { n: u8, except: crate::ids::MinionId },
    /// "Lock a ready minion": mandatory; a locked one is a legal choice
    /// (Spirit Marionette [obe]).
    LockReady,
    /// A ready Blood Brother Free Fight locked becomes the chooser's
    /// combatant, the combat going on; `ending`: as its combatant left the
    /// ready region (`CombatCtx::free_fight`): optional.
    BecomeCombatant { ending: bool },
    /// The opposing combatant whose maneuver the chooser cancels (Rigor
    /// Mortis [THN]): optional.
    CancelManeuver(crate::ids::MinionId),
    /// A ready vampire of `clan`, any Methuselah's, other than `from`: n
    /// blood moves from `from` to it (Glass Walker Pact): mandatory.
    BloodFromTo {
        from: crate::ids::MinionId,
        n: u8,
        clan: &'static str,
    },
    /// A vampire of the chooser's named as `card`'s hidden chosen, `pool`
    /// paid; right, the card burns (Betrayer; RTR 19941109): optional.
    GuessChosen { card: CardId, pool: u8 },
    /// A minion of any of these Methuselahs gets the chooser's corruption
    /// counter (Weigh the Heart [AUS][SER]; ANK 20180515): mandatory.
    CorruptAmong(crate::ids::SeatSet),
    /// A minion of theirs (as the `Holder` admits) able to hold this card,
    /// not its host, which it moves onto now (Shepherd's Innocence
    /// [ANI]): mandatory.
    HostCardOn(CardId, crate::cards::Holder),
    /// The vampire in play whose city the title card `CardId` makes its
    /// bearer baron of (Anarch Secession played by Vlad Tepes, "as a
    /// Prince of any city, even a made-up one": PIB 20150306): optional,
    /// a pass keeping the made-up city.
    SecessionCity(CardId),
    /// A locked vampire younger than its host, any Methuselah's, torpid
    /// too, the card `CardId` moves onto (Kindred Society Games):
    /// mandatory.
    MoveToYoungerLocked(CardId),
    /// One of the chooser's two copies of the crypt card, both incoming
    /// at once, burned, the other kept (Parmenides; LSJ 20030305):
    /// mandatory.
    BurnSelfContested(u16),
}

impl Interrupt {
    fn seat(self) -> SeatId {
        match self {
            Interrupt::GiveBlood { seat, .. }
            | Interrupt::MaySkipTurn { seat }
            | Interrupt::MayTakeEdge { seat }
            | Interrupt::TorporMassacre { seat, .. }
            | Interrupt::TakeCountersUpTo { seat, .. }
            | Interrupt::GuessNumber { seat, .. }
            | Interrupt::Bid { seat, .. }
            | Interrupt::DiablerieEquipment { seat, .. }
            | Interrupt::MoveBetween { seat, .. }
            | Interrupt::PayOrBurn { seat, .. }
            | Interrupt::LockOrDiscard { seat, .. }
            | Interrupt::BloodOrBurned { seat, .. }
            | Interrupt::BloodOrCamarilla { seat, .. }
            | Interrupt::Replacement { seat, .. }
            | Interrupt::DrawUp { seat }
            | Interrupt::DrawRise { seat, .. }
            | Interrupt::Redraw { seat, .. }
            | Interrupt::PreyReplacement { seat, .. }
            | Interrupt::Draws { seat, .. }
            | Interrupt::Bottom { seat, .. }
            | Interrupt::StoreSearch { seat, .. }
            | Interrupt::EntryBurn { seat, .. }
            | Interrupt::BurnOverCapacity { seat, .. }
            | Interrupt::EntryRemoveOrBurn { seat, .. }
            | Interrupt::EntryRemoveForLife { seat, .. }
            | Interrupt::BlockToll { seat, .. }
            | Interrupt::CancelAttempt { seat, .. }
            | Interrupt::MarginPool { seat, .. }
            | Interrupt::MarginBlood { seat, .. }
            | Interrupt::MinionOf { seat, .. }
            | Interrupt::CorruptionOwner { seat, .. }
            | Interrupt::DiscardChosen { seat, .. }
            | Interrupt::Reorder { seat, .. }
            | Interrupt::AvoidDamage { seat, .. }
            | Interrupt::MayTakeAggravatedAsNormal { seat, .. }
            | Interrupt::ExtraHires { seat, .. }
            | Interrupt::DiscardFromHandOf { seat, .. }
            | Interrupt::RemoveFromAsh { seat, .. }
            | Interrupt::DisciplineSearch { seat, .. }
            | Interrupt::AllySearch { seat, .. }
            | Interrupt::RetainerSearch { seat, .. }
            | Interrupt::GainConviction { seat, .. }
            | Interrupt::BurnConvictions { seat, .. }
            | Interrupt::XConvictions { seat, .. }
            | Interrupt::AvengeImbued { seat, .. }
            | Interrupt::NameConviction { seat, .. }
            | Interrupt::PowerSearch { seat, .. }
            | Interrupt::MayCancelCombat { seat, .. }
            | Interrupt::RetainerDamageOrder { seat, .. }
            | Interrupt::PutInPlaySearch { seat, .. }
            | Interrupt::MoveCardToOldest { seat, .. }
            | Interrupt::MayPreventForBlood { seat, .. }
            | Interrupt::MayBurnBloodToPeek { seat, .. }
            | Interrupt::CombatStandIn { seat, .. }
            | Interrupt::ShortfallPicks { seat, .. }
            | Interrupt::MayUnlock { seat, .. }
            | Interrupt::MayMoveBlood { seat, .. }
            | Interrupt::MayFailAction { seat, .. }
            | Interrupt::SearchOnto { seat, .. }
            | Interrupt::HandCardAway { seat }
            | Interrupt::ExchangeLooked { seat, .. }
            | Interrupt::MayLockToFight { seat, .. }
            | Interrupt::MayBurnForBlock { seat, .. }
            | Interrupt::MayDiscardForBlock { seat, .. }
            | Interrupt::MayDiscardToKeep { seat, .. }
            | Interrupt::BoonAuction { seat, .. }
            | Interrupt::HideOneCard { seat, .. }
            | Interrupt::LookPreySlot { seat, .. }
            | Interrupt::MayReturnDiscards { seat, .. }
            | Interrupt::MayBottomLooked { seat, .. }
            | Interrupt::AshToLibraryTop { seat, .. }
            | Interrupt::CutLibrary { seat }
            | Interrupt::ReductionAtResolution { seat, .. }
            | Interrupt::NamedX { seat, .. }
            | Interrupt::ActionX { seat, .. }
            | Interrupt::MayBurnInsteadOfUnlock { seat, .. }
            | Interrupt::MayBurnWeapon { seat, .. }
            | Interrupt::MaySetNextRange { seat, .. }
            | Interrupt::BlockWouldSucceed { seat, .. }
            | Interrupt::DualFormKept { seat, .. }
            | Interrupt::GiveCardTo { seat, .. }
            | Interrupt::BurnOrder { seat }
            | Interrupt::StealUpTo { seat, .. }
            | Interrupt::BurnBloodUpTo { seat, .. }
            | Interrupt::MoveBloodUpTo { seat, .. }
            | Interrupt::PurgePick { seat, .. }
            | Interrupt::MayDrawExtra { seat, .. }
            | Interrupt::RetakeFromAsh { seat, .. }
            | Interrupt::ReplaceFromResearch { seat, .. }
            | Interrupt::ReplaceFromAsh { seat, .. }
            | Interrupt::PlacedDiscipline { seat, .. }
            | Interrupt::PursuitDraw { seat, .. }
            | Interrupt::PursuitDiscard { seat, .. }
            | Interrupt::RemoveUncontrolled { seat }
            | Interrupt::ChooseForCard { seat, .. }
            | Interrupt::MayReadyChosen { seat, .. }
            | Interrupt::MayPutDiscardedOnActor { seat, .. }
            | Interrupt::ChooseUncontrolledForCard { seat, .. }
            | Interrupt::ExchangeAnarch { seat, .. }
            | Interrupt::PoolForCard { seat, .. }
            | Interrupt::LockForPool { seat, .. }
            | Interrupt::MakeAnarch { seat, .. }
            | Interrupt::BurnBloodToAnarch { seat, .. }
            | Interrupt::MayJoinSect { seat, .. }
            | Interrupt::BurnLocationsForPool { seat, .. }
            | Interrupt::PlacedClan { seat, .. }
            | Interrupt::BurnOrFail { seat, .. }
            | Interrupt::PreySlotHalf { seat, .. }
            | Interrupt::MayNewCombat { seat, .. }
            | Interrupt::GainVictimDiscipline { seat, .. }
            | Interrupt::MayJoinCombat { seat, .. }
            | Interrupt::SwapEquipFromPrey { seat, .. }
            | Interrupt::StealForCorruption { seat, .. }
            | Interrupt::UncontrolledBlood { seat, .. }
            | Interrupt::CryptSearch { seat, .. }
            | Interrupt::CryptTopPick { seat, .. }
            | Interrupt::LockOrPay { seat, .. }
            | Interrupt::PoolOrVampire { seat, .. }
            | Interrupt::PoolOrNoTransfers { seat, .. }
            | Interrupt::PoolOrMinionBurned { seat, .. }
            | Interrupt::BloodOrPool { seat, .. }
            | Interrupt::ContestPayment { seat, .. }
            | Interrupt::LibraryToHand { seat, .. }
            | Interrupt::ResearchSearch { seat, .. }
            | Interrupt::PutFoundOn { seat, .. }
            | Interrupt::PreySlotToPool { seat, .. }
            | Interrupt::DistributeFrom { seat, .. }
            | Interrupt::ChooseCircle { seat, .. }
            | Interrupt::ChooseCardCircle { seat, .. }
            | Interrupt::MoveClot { seat, .. }
            | Interrupt::ClotFate { seat, .. }
            | Interrupt::AwardTrophy { seat, .. }
            | Interrupt::TakeChosen { seat, .. }
            | Interrupt::CircleBlood { seat, .. }
            | Interrupt::PoolToVampires { seat, .. }
            | Interrupt::BaseSearch { seat, .. }
            | Interrupt::BecomesAnarch { seat }
            | Interrupt::AnarchConvert { seat, .. }
            | Interrupt::AshurPicks { seat, .. }
            | Interrupt::AllyFromAsh { seat, .. }
            | Interrupt::NameAshPair { seat, .. }
            | Interrupt::NameTargetAshCard { seat, .. }
            | Interrupt::MayCancelHuntingGround { seat, .. }
            | Interrupt::LookedToHand { seat, .. }
            | Interrupt::MastersFromHand { seat, .. }
            | Interrupt::MayTakeTitle { seat, .. }
            | Interrupt::NameUncontrolled { seat, .. }
            | Interrupt::NameSlotPair { seat, .. }
            | Interrupt::NameOwnAshCard { seat, .. }
            | Interrupt::NameClan { seat, .. }
            | Interrupt::NameAshVampire { seat, .. }
            | Interrupt::StorePreyAsh { seat, .. }
            | Interrupt::StoreOwnAsh { seat, .. }
            | Interrupt::StoreOwnHand { seat, .. }
            | Interrupt::StoreUpTo { seat, .. }
            | Interrupt::StoredToLibraryTop { seat, .. }
            | Interrupt::BeastDisciplines { seat, .. }
            | Interrupt::MayRecycleCanceled { seat, .. }
            | Interrupt::MayMillCopy { seat, .. }
            | Interrupt::MayPutOn { seat, .. }
            | Interrupt::MayAddCounter { seat, .. }
            | Interrupt::MayStoreAim { seat, .. }
            | Interrupt::MayBottomTop { seat, .. }
            | Interrupt::MayTorture { seat, .. }
            | Interrupt::MayLockForBlood { seat, .. }
            | Interrupt::BurnFromLibraryTop { seat, .. }
            | Interrupt::CryptToReady { seat, .. }
            | Interrupt::StealFromSlot { seat, .. }
            | Interrupt::BloodForVotes { seat, .. }
            | Interrupt::FavorAuction { seat, .. }
            | Interrupt::StoreFromAshOf { seat, .. }
            | Interrupt::BloodToCardUpTo { seat, .. }
            | Interrupt::MayLockCardForBlock { seat, .. }
            | Interrupt::MayBurnLookedMaster { seat, .. }
            | Interrupt::MayLockToDiscardLooked { seat, .. }
            | Interrupt::MayGainPool { seat, .. }
            | Interrupt::MayGainPoolForCard { seat, .. }
            | Interrupt::MayBurnEdgeForPool { seat, .. }
            | Interrupt::MayBurnBloodToBurnPool { seat, .. }
            | Interrupt::NameAnyAshMinion { seat, .. }
            | Interrupt::RepresentPicks { seat, .. }
            | Interrupt::BurnCopiesPick { seat, .. }
            | Interrupt::KeepSpentBlood { seat, .. }
            | Interrupt::BurnReplacement { seat, .. }
            | Interrupt::BloodOrNoStrike { seat, .. }
            | Interrupt::NameActionDiscipline { seat, .. }
            | Interrupt::NamedOntoOrDamage { seat, .. }
            | Interrupt::DamageVoterForBlood { seat, .. }
            | Interrupt::SecondCopyOnto { seat, .. }
            | Interrupt::DrawUpTo { seat, .. }
            | Interrupt::BindLocation { seat, .. }
            | Interrupt::ReturnBeforeBurn { seat, .. }
            | Interrupt::RedirectFirst { seat, .. }
            | Interrupt::DrainBeforeSteals { seat, .. }
            | Interrupt::RetainerBurnsWeaponOrDamage { seat, .. }
            | Interrupt::MayGainHandSize { seat, .. }
            | Interrupt::MayGainBlood { seat, .. }
            | Interrupt::MayMoveEdgePool { seat, .. }
            | Interrupt::NameAshRetainer { seat, .. }
            | Interrupt::FoundVersion { seat, .. }
            | Interrupt::MayHuntBloodToPool { seat, .. }
            | Interrupt::MayLockForDisciplineSearch { seat, .. }
            | Interrupt::CoercedSide { seat, .. }
            | Interrupt::PlayedCardToHand { seat, .. }
            | Interrupt::BuyTitle { seat, .. }
            | Interrupt::EquipFromHandOnEntry { seat, .. }
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
    /// The library cards of `seat`'s ash heap but the copy `except` (Lady
    /// Constancia's replacement), by name.
    fn ash_replacements(&self, seat: SeatId, except: (u16, crate::copies::CopyId)) -> Vec<u16> {
        let mut out: Vec<u16> = self.state.seats[seat.index()]
            .ash
            .pairs()
            .filter(|&p| p != except)
            .map(|p| p.0)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

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

    /// A Hold: `card` of `from`'s ash heap moves onto `on`, face up.
    pub(crate) fn store_from_ash(&mut self, on: CardId, from: SeatId, card: u16) {
        let ash = &mut self.state.seats[from.index()].ash;
        let i = ash
            .iter()
            .rposition(|&c| c == card)
            .expect("in that ash heap");
        let (_, copy) = ash.take(i);
        self.state.cards[on.index()].stored.put(card, copy);
        self.state.cards[on.index()].stored_owner = Some(from);
    }

    /// The distinct cards of `seat`'s hand matching `of` a store may take:
    /// with `earmarked`, among the seat's earmarked ones (Gift of Proteus,
    /// PIB 20150428).
    pub(crate) fn store_candidates(
        &self,
        seat: SeatId,
        of: crate::cards::CardFilter,
        earmarked: bool,
    ) -> Vec<u16> {
        let seat = &self.state.seats[seat.index()];
        let mut out: Vec<u16> = seat
            .hand
            .pairs()
            .filter(|&(c, copy)| of.matches(c) && (!earmarked || seat.earmarked.contains(&copy)))
            .map(|(c, _)| c)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// `store_candidates`, those whose cost `payer` and `seat` can pay now
    /// (rules-digest D3: a payment the payer does not have is not offered).
    pub(crate) fn paid_store_candidates(
        &self,
        seat: SeatId,
        of: crate::cards::CardFilter,
        earmarked: bool,
        payer: Option<crate::ids::MinionId>,
    ) -> Vec<u16> {
        let mut out = self.store_candidates(seat, of, earmarked);
        if let Some(p) = payer {
            out.retain(|&c| {
                let (pool, blood) = self.store_cost(seat, c);
                blood <= self.state.minions[p.index()].blood
                    && pool <= self.state.seats[seat.index()].pool
            });
        }
        out
    }

    /// A stored card's printed cost: (pool, blood).
    fn store_cost(&self, seat: SeatId, card: u16) -> (i32, u8) {
        (
            cards::pool_cost(&self.state, seat, card, crate::cards::Target::None),
            cards::blood_cost(&self.state, card, 0),
        )
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

    /// The Disciplines `victim` had that `m` could still gain a level in.
    fn victim_disciplines(
        &self,
        m: crate::ids::MinionId,
        victim: crate::ids::MinionId,
    ) -> Vec<&'static str> {
        crate::cards::DISCIPLINE_KEYS
            .iter()
            .copied()
            .filter(|d| {
                self.state.printed_discipline_level(victim, d) > 0
                    && self.state.discipline_level(m, d) < 2
            })
            .collect()
    }

    /// A trophy moved to `minion`: Trophy: Chosen's choice of its blood
    /// comes first (LSJ 20070309-1), then the award goes on; its "when
    /// placed" effects (Trophy: Wealth's counters, Trophy: Progeny's vampire)
    /// and its entering choice apply now.
    fn trophy_moved(&mut self, seat: SeatId, minion: crate::ids::MinionId, card: CardId) {
        if crate::cards::trophy_blood(self.state.cards[card.index()].card).is_some() {
            self.interrupts
                .insert(0, Interrupt::TakeChosen { seat, minion, card });
        }
        let mode = &crate::cards::lookup(self.state.cards[card.index()].card)
            .expect("a trophy")
            .modes[0];
        for e in mode.effects {
            match *e {
                // "When this card is placed on a vampire" (Trophy: Wealth).
                crate::cards::Effect::CountersWhenAwarded(n) => {
                    self.state.cards[card.index()].counters += n;
                }
                // Trophy: Progeny: instead of the move, a vampire of the
                // awardee's clan and sect; on a sterile one, an inert trophy.
                crate::cards::Effect::AwardBecomesVampire { born, bank_blood } => {
                    let clan = self.state.minions[minion.index()].clan();
                    let sect = self.state.sect_of(minion);
                    if let Some(sect) = sect.filter(|_| !self.state.is_sterile(minion)) {
                        let to = self.state.minions[minion.index()].owner;
                        let c = &mut self.state.cards[card.index()];
                        c.host = None;
                        c.controller = to;
                        let m = self.card_becomes_vampire(to, card, clan, sect, born);
                        self.interrupts.insert(
                            0,
                            Interrupt::DisciplineSearch {
                                seat: to,
                                minion: m,
                                named: &[],
                                left: 1,
                                entering: false,
                                hand_or_burn: false,
                                no_hand: false,
                                library_only: false,
                            },
                        );
                        self.state.gain_blood(m, bank_blood);
                    }
                }
                _ => {}
            }
        }
        // "When this card enters play, choose …": awarded from out of
        // play, chosen now (Trophy: Discipline, Trophy: Clan Respect).
        let c = &self.state.cards[card.index()];
        match mode.terms {
            Some(crate::cards::Terms::Discipline) if c.discipline.is_none() && c.host.is_some() => {
                self.interrupts.insert(
                    0,
                    Interrupt::PlacedDiscipline {
                        seat,
                        card,
                        among: &[],
                    },
                );
            }
            Some(crate::cards::Terms::Clan) if c.chosen_clan.0.is_none() => {
                self.interrupts
                    .insert(0, Interrupt::PlacedClan { seat, card });
            }
            _ => {}
        }
    }

    /// The locations `seat` controls in play, on a minion or not (Insurance
    /// Scam).
    pub(crate) fn own_locations(&self, seat: SeatId) -> Vec<CardId> {
        let st = &self.state;
        (0..st.cards.len())
            .map(|i| CardId(i as u32))
            .filter(|&c| {
                let k = &st.cards[c.index()];
                k.present()
                    && !k.effect_only
                    && st.card_controller(c) == seat
                    && crate::library::LIBRARY[k.card as usize]
                        .labels
                        .contains(&crate::library::Label::Location)
            })
            .collect()
    }

    /// What `seat` may award `minion`: trophies in its hand, library or
    /// ash heap unless `searched`, and those it has in play, not awarded
    /// yet; none `minion` holds its printed most of (Trophy: Revered).
    fn trophy_options(
        &self,
        seat: SeatId,
        minion: crate::ids::MinionId,
        searched: bool,
    ) -> Vec<Choice> {
        use crate::decision::Zone;
        let st = &self.state;
        let s = &st.seats[seat.index()];
        let room = |card: u16| {
            crate::cards::trophy_cap(card).is_none_or(|n| {
                st.holds_count(
                    minion,
                    crate::cards::Holding::Named(crate::library::LIBRARY[card as usize].name),
                ) < n
            })
        };
        // By zone: the player picks where it comes from (rulebook "Red List").
        [
            (Zone::Hand, &s.hand),
            (Zone::Library, &s.library),
            (Zone::AshHeap, &s.ash),
        ]
        .into_iter()
        .filter(|_| !searched)
        .flat_map(|(zone, pile)| {
            distinct(pile, |c| crate::cards::is_trophy(c) && room(c))
                .into_iter()
                .map(move |card| Choice::TakeFrom { zone, card })
        })
        .chain(
            st.cards
                .iter()
                .enumerate()
                .filter(|(i, c)| {
                    c.present()
                        && c.host.is_none()
                        && crate::cards::is_trophy(c.card)
                        && room(c.card)
                        && st.card_controller(CardId(*i as u32)) == seat
                })
                .map(|(i, _)| Choice::ResolveRider { card: i as u32 }),
        )
        .collect()
    }

    /// A deferred replacement taken by a rider: no draw, so no extra one:
    /// every offer of the next card's goes.
    fn drop_replacement_draws(&mut self, s: SeatId) {
        let next = self.interrupts.iter().find_map(|i| match *i {
            Interrupt::MayDrawExtra {
                seat,
                replaces: true,
                group,
                ..
            } if seat == s => Some(group),
            _ => None,
        });
        self.interrupts.retain(|i| {
            !matches!(i, Interrupt::MayDrawExtra { seat, replaces: true, group, .. }
                if *seat == s && Some(*group) == next)
        });
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

    pub(crate) fn draw_top(&mut self, s: SeatId) {
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
                // No one to set the order: the burns left are one batch.
                if let Interrupt::BurnOrder { .. } = front {
                    self.burn_in_order(true);
                    if self.ousting.is_some() {
                        break;
                    }
                }
                // No one to choose: the first replacement still applying.
                if let Interrupt::BurnReplacement {
                    minion, to_torpor, ..
                } = front
                {
                    if let Some(&r) = self.burn_replacements(minion).first() {
                        self.apply_burn_replacement(minion, r, to_torpor);
                    } else if self.state.minions[minion.index()].present() {
                        self.burn_minion_now(minion);
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
                // A decision while the prey has a card; else drawn as usual.
                Interrupt::PreyReplacement { except, .. } => {
                    let prey = self.state.prey(s);
                    if prey != s && !self.state.seats[prey.index()].library.is_empty() {
                        true
                    } else {
                        self.interrupts.remove(0);
                        self.replace(s, 1, except);
                        continue;
                    }
                }
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
                Interrupt::DrawRise { left, may_top, .. } => {
                    if left == 0 || empty {
                        if may_top && self.state.hand_excess(s) > 0 {
                            self.state.seats[s.index()].discards_may_top.0 = 1;
                        }
                        false
                    } else if choice {
                        true
                    } else {
                        self.draw_top(s);
                        self.interrupts[0] = Interrupt::DrawRise {
                            seat: s,
                            left: left - 1,
                            may_top,
                        };
                        continue;
                    }
                }
                Interrupt::DrawUp { .. } => {
                    if empty || self.state.hand_load(s) >= self.state.hand_size(s) {
                        false
                    } else if choice {
                        true
                    } else {
                        self.draw_top(s);
                        self.show_referendum_choice(s);
                        continue;
                    }
                }
                Interrupt::Draws { left, drawn, .. } => {
                    if left == 0 || empty {
                        self.interrupts[0] = Interrupt::Bottom {
                            seat: s,
                            left: drawn,
                            top: false,
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
                Interrupt::EntryRemoveForLife { seat, ally } => {
                    self.state.minions[ally.index()].in_play
                        && !self.removable_for_life(seat).is_empty()
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
                Interrupt::DiscardFromHandOf {
                    of: Some(of),
                    up_to: Some((left, done, kind)),
                    ..
                } => {
                    let z = &self.state.seats[of.index()];
                    if left > 0 && !z.ousted && z.hand.iter().any(|&c| kind.matches(c)) {
                        return;
                    }
                    self.interrupts.remove(0);
                    if done > 0 && !z.ousted {
                        self.replace(of, done as usize, None);
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
                Interrupt::AvoidDamage { minion, .. }
                | Interrupt::MayTakeAggravatedAsNormal { minion, .. } => present(minion),
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
                    optional,
                    hold,
                } => {
                    let fits = |c: u16| of.is_none_or(|f| f.matches(c));
                    let held = self.state.seats[seat.index()]
                        .hand
                        .iter()
                        .filter(|&&c| fits(c))
                        .count();
                    if left > 0 && (held > left as usize || optional && held > 0) {
                        return;
                    }
                    // As many as it holds, or fewer: all of them — then
                    // the replacements.
                    self.interrupts.remove(0);
                    let st = &mut self.state.seats[seat.index()];
                    let k = if left > 0 { held } else { 0 };
                    let cards = st.hand.take_unless(|c| !(fits(c) && k > 0));
                    st.to_ash_all(cards);
                    self.replace_or_hold(seat, done as usize + k, hold);
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
                // Always asked: the original waits on its answer.
                Interrupt::SwapEquipFromPrey { .. } => true,
                Interrupt::GainVictimDiscipline { minion, victim, .. } => {
                    present(minion) && !self.victim_disciplines(minion, victim).is_empty()
                }
                // Always asked: its answer ends the combat.
                Interrupt::MayJoinCombat { .. } | Interrupt::MayNewCombat { .. } => true,
                // Too little blood now (a second Drink): passing is forced.
                Interrupt::BurnOrFail { .. } => true,
                Interrupt::BurnBloodToAnarch { minion, .. } => {
                    present(minion)
                        && self.state.minions[minion.index()].blood > 0
                        && self.state.sect_of(minion) != Some(crate::crypt::Sect::Anarch)
                }
                Interrupt::MayJoinSect { minion, sect, .. } => {
                    present(minion)
                        && self.state.minions[minion.index()].actual_sect() != Some(sect)
                }
                Interrupt::BurnLocationsForPool { .. } => true,
                Interrupt::PlacedClan { card, .. } => self.state.cards[card.index()].present(),
                Interrupt::LockForPool { card, .. } => {
                    let c = &self.state.cards[card.index()];
                    c.present() && !c.locked
                }
                Interrupt::PoolForCard { .. } => true,
                Interrupt::ExchangeAnarch { seat, .. } => {
                    !self.state.seats[seat.index()].hand.is_empty()
                }
                Interrupt::MayPutDiscardedOnActor { minion, .. } => {
                    present(minion) && !self.discarded_for_actor(front).is_empty()
                }
                Interrupt::MayReadyChosen { card, .. } => {
                    self.state.cards[card.index()].present()
                        && self.state.chosen_slot(card).is_some()
                }
                Interrupt::ChooseUncontrolledForCard { seat, card } => {
                    let n = self.state.seats[seat.index()].uncontrolled.len();
                    if n > 1 && self.state.cards[card.index()].present() {
                        return;
                    }
                    self.interrupts.remove(0);
                    if n == 1 && self.state.cards[card.index()].present() {
                        self.choose_uncontrolled_for(card, seat, 0);
                    }
                    continue;
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
                Interrupt::BurnBloodUpTo { from, .. } => {
                    present(from) && self.state.minions[from.index()].blood > 0
                }
                Interrupt::MoveBloodUpTo { from, to, .. } => {
                    present(from) && present(to) && self.state.minions[from.index()].blood > 0
                }
                Interrupt::MayDrawExtra { seat, .. } => {
                    !self.state.seats[seat.index()].library.is_empty()
                }
                // A single option (the card left the ash heap) is elided:
                // the draw still happens.
                Interrupt::RetakeFromAsh { .. } => true,
                Interrupt::ReplaceFromResearch { .. } => true,
                // None left to take: a draw, as any replacement.
                Interrupt::ReplaceFromAsh { seat, except } => {
                    if self.ash_replacements(seat, except).is_empty() {
                        self.interrupts.remove(0);
                        self.replace(seat, 1, None);
                        continue;
                    }
                    true
                }
                // Nothing left to choose (the card gone, or its host at
                // superior in each): no Discipline.
                Interrupt::PlacedDiscipline { card, among, .. } => {
                    !self.placed_disciplines(card, among).is_empty()
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
                Interrupt::RetainerDamageOrder { minion, .. } => present(minion),
                Interrupt::MayCancelCombat {
                    minion,
                    blood,
                    burn,
                    ..
                } => {
                    if self.state.minions[minion.index()].blood >= blood
                        && burn.is_none_or(|id| self.state.cards[id.index()].present())
                        && self
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
                Interrupt::XConvictions { minion, card, .. } => {
                    if self.state.convictions(minion).is_empty()
                        || !self.state.cards[card.index()].present()
                    {
                        self.interrupts.remove(0);
                        continue;
                    }
                    return;
                }
                // Nothing left to bring: the hired cards replaced now.
                Interrupt::ExtraHires {
                    seat,
                    minion,
                    left,
                    hired,
                } => {
                    if left > 0
                        && present(minion)
                        && !self
                            .employable_from_hand(seat, minion, None, false)
                            .is_empty()
                    {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.replace(seat, hired as usize, None);
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
                Interrupt::AllySearch {
                    minion, recruit, ..
                } => {
                    if present(minion) && !self.searched_allies(s, minion, recruit).is_empty() {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.state.seats[s.index()].library.shuffle(&mut self.rng);
                    continue;
                }
                Interrupt::RetainerSearch {
                    minion,
                    filter,
                    off,
                    hand,
                    ..
                } => {
                    if present(minion)
                        && !self
                            .searched_retainers(s, minion, filter, off, hand)
                            .is_empty()
                    {
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
                Interrupt::DisciplineSearch {
                    minion,
                    named,
                    entering,
                    hand_or_burn,
                    no_hand,
                    library_only,
                    ..
                } => {
                    if present(minion)
                        && !self
                            .discipline_masters(
                                s,
                                minion,
                                named,
                                hand_or_burn,
                                no_hand,
                                library_only,
                            )
                            .is_empty()
                    {
                        return;
                    }
                    self.interrupts.remove(0);
                    // Bima: none in hand, burned.
                    if hand_or_burn {
                        if present(minion) {
                            self.burn_minion(minion);
                        }
                        continue;
                    }
                    self.state.seats[s.index()].library.shuffle(&mut self.rng);
                    if entering && present(minion) {
                        self.stage_contests(minion);
                    }
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
                Interrupt::MinionOf { seat, of, purpose } => {
                    if self.minion_of_options(seat, of, purpose).is_empty() {
                        if let MinionPurpose::HuntBloodTo { hunter, n } = purpose {
                            self.state.gain_blood(hunter, n);
                        }
                        // Both vampires of a joint block gone: it failed.
                        if let (MinionPurpose::JointBlocker(..), Some(a)) =
                            (purpose, self.action.as_mut())
                        {
                            a.fail_attempt(&self.state);
                        }
                        false
                    } else {
                        true
                    }
                }
                Interrupt::AllyFromAsh { seat, of, .. } => !self.ash_allies(seat, of).is_empty(),
                Interrupt::NameAshVampire { seat, burned } => {
                    self.action.is_some() && !self.ash_vampire_names(seat, burned).is_empty()
                }
                Interrupt::BeastDisciplines { minion, .. } => {
                    self.state.minions[minion.index()].in_play
                }
                Interrupt::StoreOwnAsh { seat, card } => {
                    self.state.cards[card.index()].present()
                        && !self.state.seats[seat.index()].ash.is_empty()
                }
                Interrupt::StoreOwnHand {
                    seat,
                    card,
                    of,
                    earmarked,
                    payer,
                    ..
                } => {
                    self.state.cards[card.index()].present()
                        && !self
                            .paid_store_candidates(seat, of, earmarked, payer)
                            .is_empty()
                }
                Interrupt::StoredToLibraryTop { card, .. } => {
                    self.state.cards[card.index()].present()
                        && !self.state.cards[card.index()].stored.is_empty()
                }
                Interrupt::StoreUpTo { card, .. } => self.state.cards[card.index()].present(),
                Interrupt::StorePreyAsh { seat, card } => {
                    self.state.cards[card.index()].present()
                        && !self.state.seats[self.state.prey(seat).index()]
                            .ash
                            .is_empty()
                }
                Interrupt::NameAshPair { .. }
                | Interrupt::NameTargetAshCard { .. }
                | Interrupt::NameAshEquipment { .. }
                | Interrupt::NameAnyAshMinion { .. }
                | Interrupt::NameMasterDiscipline { .. }
                | Interrupt::NameUncontrolled { .. }
                | Interrupt::NameSlotPair { .. }
                | Interrupt::StealFromSlot { .. }
                | Interrupt::NameAshRetainer { .. }
                | Interrupt::NameOwnAshCard { .. }
                | Interrupt::NameClan { .. } => self.action.is_some(),
                Interrupt::MayRecycleCanceled { seat, card, .. } => {
                    self.state.seats[seat.index()].ash.contains(&card)
                }
                Interrupt::MayMillCopy { .. } | Interrupt::FoundVersion { .. } => true,
                Interrupt::MayPutOn { held, .. } => self.may_put_held(held),
                Interrupt::MayAddCounter { card, .. } => self.state.cards[card.index()].present(),
                Interrupt::MayStoreAim { seat, store, card } => {
                    self.state.cards[store.index()].present()
                        && self.state.seats[seat.index()].ash.contains(&card)
                }
                Interrupt::MayBottomTop { .. } | Interrupt::MaySkipTurn { .. } => true,
                Interrupt::MayTakeEdge { .. } => self.state.seats.iter().all(|x| !x.has_edge),
                Interrupt::TorporMassacre { .. } => true,
                Interrupt::TakeCountersUpTo { card, .. } => {
                    self.state.cards[card.index()].present()
                }
                Interrupt::CoercedSide { minion, .. } => {
                    self.referendum.is_some() && present(minion)
                }
                Interrupt::PlayedCardToHand { seat, minion } => {
                    !self.state.seats[seat.index()].ousted
                        && !self.memory_cards(seat, minion).is_empty()
                }
                Interrupt::MayLockForDisciplineSearch { seat, card, .. } => {
                    let c = &self.state.cards[card.index()];
                    c.present() && !c.locked && !self.ready_vampires_of(seat).is_empty()
                }
                Interrupt::MayTorture { card, minion, .. } => {
                    let c = &self.state.cards[card.index()];
                    c.present() && !c.locked && present(minion)
                }
                Interrupt::MayLockForBlood { card, .. } => {
                    let c = &self.state.cards[card.index()];
                    c.present() && !c.locked && c.host.is_some_and(present)
                }
                Interrupt::BurnFromLibraryTop { of, .. } => {
                    !self.state.seats[of.index()].library.is_empty()
                }
                Interrupt::MayTakeTitle { card, .. } => self.state.may_take_princes_title(card),
                Interrupt::MayCancelHuntingGround { .. } => true,
                Interrupt::MastersFromHand { seat, minion, left } => {
                    let masters =
                        crate::state::master_disciplines(&self.state.seats[seat.index()].hand);
                    if left == 0 || !present(minion) || masters.is_empty() {
                        self.interrupts.remove(0);
                        continue;
                    }
                    true
                }
                Interrupt::LookedToHand { seat, n, left } => {
                    if n == 0 || left == 0 || self.state.seats[seat.index()].library.is_empty() {
                        self.interrupts.remove(0);
                        self.state.seats[seat.index()]
                            .library
                            .shuffle(&mut self.rng);
                        continue;
                    }
                    true
                }
                Interrupt::CryptToReady { .. } => true,
                Interrupt::BloodForVotes { .. } => self.referendum.is_some(),
                Interrupt::FavorAuction { .. } => self.referendum.is_some(),
                Interrupt::MayLockCardForBlock { card, minion, .. } => {
                    let c = &self.state.cards[card.index()];
                    if c.in_play && !c.locked {
                        true
                    } else {
                        self.interrupts.remove(0);
                        self.state.minions[minion.index()].locked = true;
                        continue;
                    }
                }
                Interrupt::BloodToCardUpTo { from, card, .. } => {
                    present(from)
                        && self.state.minions[from.index()].blood > 0
                        && self.state.cards[card.index()].in_play
                }
                Interrupt::StoreFromAshOf { card, from, of, .. } => {
                    self.state.cards[card.index()].present()
                        && self.state.seats[from.index()]
                            .ash
                            .iter()
                            .any(|&c| crate::library::LIBRARY[c as usize].types.contains(&of))
                }
                // Needs not be unlocked to look (LSJ 20001102); only to
                // discard.
                Interrupt::MayLockToDiscardLooked { card, of, .. } => {
                    let st = &self.state;
                    let c = &st.cards[card.index()];
                    let locked = match c.body {
                        Some(m) => st.minions[m.index()].locked,
                        None => c.locked,
                    };
                    c.present() && !locked && !st.seats[of.index()].library.is_empty()
                }
                Interrupt::MayBurnLookedMaster { card, n, .. } => {
                    let st = &self.state;
                    let c = &st.cards[card.index()];
                    c.present()
                        && c.host.is_some_and(|h| {
                            st.minions[h.index()].is_vampire() && st.minions[h.index()].blood >= n
                        })
                        && st.library_top_is(st.active, crate::library::CardType::Master)
                }
                Interrupt::MayGainPoolForCard { seat, card, .. } => {
                    !self.state.seats[seat.index()].ousted
                        && self.state.cards[card.index()].present()
                }
                Interrupt::MayGainPool { seat, .. }
                | Interrupt::MayMoveEdgePool { seat, .. }
                | Interrupt::MayHuntBloodToPool { seat, .. } => {
                    !self.state.seats[seat.index()].ousted
                }
                Interrupt::MayBurnEdgeForPool { seat, .. } => {
                    let s = &self.state.seats[seat.index()];
                    !s.ousted && s.has_edge
                }
                Interrupt::RepresentPicks { seat, .. } => {
                    !self.state.seats[seat.index()].ousted && !self.represent_slots(seat).is_empty()
                }
                Interrupt::BurnCopiesPick { seat, .. } => !self.state.seats[seat.index()].ousted,
                Interrupt::KeepSpentBlood {
                    minion, card, mode, ..
                } => !self
                    .state
                    .spent_blood_keepers(minion, card, mode)
                    .is_empty(),
                Interrupt::MayBurnBloodToBurnPool {
                    seat,
                    minion,
                    of,
                    blood,
                    ..
                } => {
                    !self.state.seats[seat.index()].ousted
                        && !self.state.seats[of.index()].ousted
                        && present(minion)
                        && self.state.minions[minion.index()].blood >= blood
                }
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
                Interrupt::EquipFromHandOnEntry {
                    seat,
                    minion,
                    filter,
                } => {
                    if present(minion)
                        && !self.state.seats[seat.index()].ousted
                        && !self
                            .equippable_from_hand(seat, minion, filter, false)
                            .is_empty()
                    {
                        true
                    } else {
                        self.interrupts.remove(0);
                        if present(minion) {
                            self.stage_contests(minion);
                        }
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
                Interrupt::MayGainBlood { seat, minion, .. } => {
                    !self.state.seats[seat.index()].ousted && present(minion)
                }
                Interrupt::MayGainHandSize { seat, minion, .. } => {
                    !self.state.seats[seat.index()].ousted
                        && !self
                            .state
                            .turn_uses
                            .contains(&crate::state::once_hand_size_key(minion))
                }
                // Burden the Mind's toll payable (D3).
                Interrupt::MayUnlock { seat, minion, .. } => {
                    present(minion)
                        && self.state.minions[minion.index()].locked
                        && self.state.unlock_toll_on(minion) <= self.state.seats[seat.index()].pool
                }
                Interrupt::MayFailAction { .. } => self.action.is_some(),
                Interrupt::SearchOnto { minion, .. } => present(minion),
                Interrupt::ReturnBeforeBurn { card, .. } => {
                    let c = &self.state.cards[card.index()];
                    c.in_play && c.out_of_play
                }
                Interrupt::RedirectFirst { .. } | Interrupt::DrainBeforeSteals { .. } => {
                    self.combat.is_some()
                }
                Interrupt::RetainerBurnsWeaponOrDamage { minion, .. } => {
                    self.combat.is_some() && !self.weapons_on(minion).is_empty()
                }
                Interrupt::DrawUpTo { .. } => {
                    if empty {
                        self.interrupts.remove(0);
                        continue;
                    }
                    true
                }
                Interrupt::BindLocation { minion, .. } => {
                    if !present(minion) {
                        self.interrupts.remove(0);
                        continue;
                    }
                    if self.state.bindable_locations(minion).is_empty() {
                        self.interrupts.remove(0);
                        self.remove_from_game(minion);
                        continue;
                    }
                    true
                }
                Interrupt::SecondCopyOnto {
                    seat, minion, card, ..
                } => {
                    let s = &self.state.seats[seat.index()];
                    present(minion) && (s.hand.contains(&card) || s.ash.contains(&card))
                }
                Interrupt::ExchangeLooked {
                    of, card, minion, ..
                } => {
                    present(minion)
                        && self.state.minions[minion.index()].blood > 0
                        && self.state.seats[of.index()].hand.contains(&card)
                        && !self.state.seats[of.index()].ash.is_empty()
                }
                Interrupt::HandCardAway { seat } => {
                    if self.state.seats[seat.index()].hand.is_empty() {
                        self.interrupts.remove(0);
                        continue;
                    }
                    true
                }
                Interrupt::MayLockToFight { minion, acting, .. } => {
                    let v = &self.state.minions[minion.index()];
                    v.ready() && !v.locked && self.state.minions[acting.index()].ready()
                }
                Interrupt::MayMoveBlood { from, to, .. } => {
                    present(from) && present(to) && self.state.minions[from.index()].blood > 0
                }
                Interrupt::MayBurnForBlock { minion, .. }
                | Interrupt::MayDiscardForBlock { minion, .. } => present(minion),
                Interrupt::BoonAuction { .. } => true,
                Interrupt::LookPreySlot { of, .. } => {
                    !self.state.seats[of.index()].uncontrolled.is_empty()
                }
                Interrupt::HideOneCard { seat, .. } => {
                    self.state.seats[seat.index()].hand.len() > 1
                }
                Interrupt::MayDiscardToKeep { card, taker, .. } => {
                    self.state.cards[card.index()].present()
                        && !self.state.seats[taker.index()].ousted
                }
                Interrupt::MayReturnDiscards { .. } => true,
                Interrupt::MayBottomLooked { seat, minion, n } => {
                    present(minion)
                        && self.state.minions[minion.index()].blood >= n
                        && !self.state.seats[seat.index()].library.is_empty()
                }
                Interrupt::ReductionAtResolution { minion, .. }
                | Interrupt::ActionX { minion, .. }
                | Interrupt::NamedX { minion, .. } => {
                    self.action.as_ref().is_some_and(|a| a.acting == minion)
                }
                // The card gone, the minion unlocks as normal.
                Interrupt::MayBurnInsteadOfUnlock { minion, card, .. } => {
                    let c = &self.state.cards[card.index()];
                    if !(c.present() && c.host == Some(minion)) {
                        self.interrupts.remove(0);
                        if self.state.minions[minion.index()].in_play {
                            self.unlock_minion(minion);
                        }
                        continue;
                    }
                    true
                }
                Interrupt::MayBurnWeapon { minion, .. } => !self.weapons_on(minion).is_empty(),
                Interrupt::BlockWouldSucceed { .. } => true,
                Interrupt::MaySetNextRange { .. } => {
                    self.combat.as_ref().is_some_and(|c| c.next_range_open())
                }
                Interrupt::DualFormKept { root, .. } => self.state.dual_set(root).len() > 1,
                Interrupt::AshToLibraryTop {
                    seat,
                    left,
                    filter,
                    discarded,
                    ..
                } => {
                    let heap = &self.state.seats[seat.index()];
                    let open = left > 0
                        && heap.ash.pairs().any(|(c, copy)| {
                            filter.matches(c) && (!discarded || heap.top_discards.0.contains(&copy))
                        });
                    if !open && discarded {
                        self.state.seats[seat.index()].top_discards.0.clear();
                    }
                    open
                }
                Interrupt::CutLibrary { seat } => self.state.seats[seat.index()].library.len() > 1,
                Interrupt::GiveCardTo { card, .. } => self.state.cards[card.index()].present(),
                Interrupt::BurnOrder { .. } => {
                    if self.burn_order_left().len() < 2 {
                        self.interrupts.remove(0);
                        self.burn_in_order(true);
                        if self.ousting.is_some() {
                            break;
                        }
                        continue;
                    }
                    true
                }
                Interrupt::CombatStandIn { card, replaced, .. } => {
                    let live = self.state.cards[card.index()].body.is_some_and(|o| {
                        let v = &self.state.minions[o.index()];
                        v.ready() && !v.locked
                    });
                    if !live && self.in_combat(replaced) {
                        self.interrupts.remove(0);
                        self.settle_rush_stand_in(replaced, None);
                        continue;
                    }
                    live
                }
                Interrupt::MayBurnBloodToPeek { minion, of, .. } => {
                    present(minion)
                        && self.state.minions[minion.index()].blood >= 1
                        && !self.state.seats[of.index()].ousted
                }
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
                Interrupt::UncontrolledBlood {
                    seat,
                    clan,
                    older_than,
                    ..
                } => !self.uncontrolled_older(seat, clan, older_than).is_empty(),
                Interrupt::LockOrPay { card, .. } => self.state.cards[card.index()].present(),
                // The card gone before its effect resolves: nothing is due
                // (owner, rules-qa 70.9).
                Interrupt::PoolOrVampire { seat, card, .. } => {
                    !self.state.seats[seat.index()].ousted
                        && self.state.cards[card.index()].present()
                }
                Interrupt::PoolOrNoTransfers { seat, .. }
                | Interrupt::BloodOrPool { seat, .. }
                | Interrupt::ContestPayment { seat, .. } => !self.state.seats[seat.index()].ousted,
                Interrupt::PoolOrMinionBurned { seat, minion, .. } => {
                    !self.state.seats[seat.index()].ousted && present(minion)
                }
                Interrupt::LibraryToHand { seat, .. } => !self.state.seats[seat.index()].ousted,
                Interrupt::ResearchSearch { left, .. } => {
                    if left > 0 && !empty {
                        true
                    } else {
                        self.interrupts.remove(0);
                        self.state.seats[s.index()].library.shuffle(&mut self.rng);
                        continue;
                    }
                }
                Interrupt::PutFoundOn { seat, minion, card } => {
                    self.may_put_found_on(seat, minion, card)
                }
                Interrupt::CryptTopPick { seat, .. } => {
                    if self.state.seats[seat.index()].crypt.is_empty() {
                        self.interrupts.remove(0);
                        continue;
                    }
                    return;
                }
                Interrupt::CryptSearch { seat, .. } => {
                    if self.crypt_of_clan(front).is_empty() {
                        self.interrupts.remove(0);
                        self.state.seats[seat.index()].crypt.shuffle(&mut self.rng);
                        continue;
                    }
                    true
                }
                Interrupt::ChooseCircle { minion, .. } => {
                    self.state.minions[minion.index()].present()
                }
                Interrupt::CircleBlood { .. } | Interrupt::ChooseCardCircle { .. } => true,
                Interrupt::MoveClot { card, to, .. } => {
                    present(to) && self.state.cards[card.index()].counters > 0
                }
                Interrupt::ClotFate { minion, .. } => present(minion),
                Interrupt::AwardTrophy {
                    seat,
                    minion,
                    searched,
                } => present(minion) && !self.trophy_options(seat, minion, searched).is_empty(),
                Interrupt::TakeChosen { minion, card, .. } => {
                    present(minion) && self.state.cards[card.index()].present()
                }
                Interrupt::PoolToVampires { seat, .. } => !self.state.seats[seat.index()].ousted,
                Interrupt::BaseSearch { minion, .. } => {
                    let v = &self.state.minions[minion.index()];
                    v.present() && v.merged.is_none()
                }
                Interrupt::PreySlotHalf { of, .. } => self.state.seats[of.index()]
                    .uncontrolled
                    .iter()
                    .any(|s| s.counters > 1),
                Interrupt::PreySlotToPool { of, transfers, .. } => {
                    (0..self.state.seats[of.index()].uncontrolled.len())
                        .any(|i| self.state.slot_gives_blood(of, i, transfers))
                }
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
                Interrupt::SecretNumber { .. } | Interrupt::GuessNumber { .. } => true,
                Interrupt::Bid { .. } => true,
                Interrupt::SearchEquipFor {
                    seat,
                    minion,
                    linked,
                } => {
                    if self.state.minions[minion.index()].present()
                        && !self.alastor_equipment(seat, minion, linked).is_empty()
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
                Interrupt::BurnReplacement {
                    minion, to_torpor, ..
                } => match self.burn_replacements(minion)[..] {
                    [_, _, ..] => true,
                    [r] => {
                        self.interrupts.remove(0);
                        self.apply_burn_replacement(minion, r, to_torpor);
                        continue;
                    }
                    // None left: it burns, if still in play.
                    [] => {
                        self.interrupts.remove(0);
                        if present(minion) {
                            self.burn_minion_now(minion);
                        }
                        continue;
                    }
                },
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
                Interrupt::NameActionDiscipline { minion, .. } => {
                    self.action.as_ref().is_some_and(|a| a.acting == minion)
                        && !self.state.others_superior_disciplines(minion).is_empty()
                }
                Interrupt::NamedOntoOrDamage { minion, .. } => present(minion),
                Interrupt::DamageVoterForBlood { minion, .. } => {
                    present(minion)
                        && self.state.minions[minion.index()].blood >= 1
                        && !self.against_voters().is_empty()
                }
                Interrupt::BloodOrNoStrike { minion, n, .. } => {
                    let in_combat = self
                        .combat
                        .as_ref()
                        .is_some_and(|c| c.side_of_minion(minion).is_some());
                    if in_combat && self.state.minions[minion.index()].blood < n {
                        self.interrupts.remove(0);
                        self.bar_strike_this_round(minion);
                        continue;
                    }
                    in_combat
                }
                Interrupt::BurnOverCapacity { host, .. } => {
                    self.over_capacity_burns(host).len() > 1
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
            Interrupt::DrawUp { .. } | Interrupt::DrawRise { .. } | Interrupt::Draws { .. } => {
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
            Interrupt::StoreSearch { filter, ash, .. } => {
                // By card: out of play, it needs no definition to be drawn
                // later.
                let found = distinct(&st.seats[s.index()].library, |c| filter.matches(c));
                let mut options = vec![Choice::Pass];
                options.extend(found.into_iter().map(Choice::FindCard));
                if ash {
                    options.extend(
                        distinct(&st.seats[s.index()].ash, |c| filter.matches(c))
                            .into_iter()
                            .map(|card| Choice::TakeFrom {
                                zone: crate::decision::Zone::AshHeap,
                                card,
                            }),
                    );
                }
                (WindowLabel::LibrarySearch, options)
            }
            Interrupt::EntryBurn { ally, .. } => {
                (WindowLabel::EntryEffect, self.entry_burn_options(s, ally))
            }
            Interrupt::BurnOverCapacity { host, .. } => (
                WindowLabel::ResolutionChoice,
                self.over_capacity_burns(host)
                    .into_iter()
                    .map(|c| Choice::BurnInPlay { card: c.0 })
                    .collect(),
            ),
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
            Interrupt::BloodOrNoStrike { .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::BurnBloodToAvoid],
            ),
            Interrupt::NameActionDiscipline { minion, .. } => (
                WindowLabel::ActionTerms,
                st.others_superior_disciplines(minion)
                    .into_iter()
                    .map(Choice::ChooseDiscipline)
                    .collect(),
            ),
            Interrupt::NamedOntoOrDamage { seat, card, .. } => {
                let s = &st.seats[seat.index()];
                let mut options = vec![Choice::Pass];
                for (zone, pile) in [(Zone::Hand, &s.hand), (Zone::AshHeap, &s.ash)] {
                    if pile.contains(&card) {
                        options.push(Choice::TakeFrom { zone, card });
                    }
                }
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::DamageVoterForBlood { .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(self.against_voters().into_iter().map(Choice::ChooseMinion))
                    .collect(),
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
            Interrupt::LockOrDiscard { of, .. } => {
                let hand = &st.seats[s.index()].hand;
                let mut options = vec![Choice::Pass];
                options.extend(self.discard_choices(s).into_iter().filter(|c| {
                    matches!((c, of), (Choice::Discard(i), Some(f)) if f.matches(hand[*i as usize]))
                        || of.is_none()
                }));
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::FoundVersion { ctx, versions, .. } => (
                WindowLabel::ResolutionChoice,
                crate::plays::version_modes(ctx.card, versions)
                    .into_iter()
                    .map(|mode| Choice::Version {
                        card: ctx.card,
                        mode,
                    })
                    .collect(),
            ),
            Interrupt::NameAshRetainer {
                seat,
                minion,
                of,
                extra,
            } => (
                WindowLabel::ResolutionChoice,
                self.ash_retainers(seat, minion, of, extra)
                    .into_iter()
                    .map(|(card, _)| Choice::FindCard(card))
                    .collect(),
            ),
            // The look made: the cards of the filter, one choice per name.
            Interrupt::DiscardFromHandOf {
                of: Some(of),
                up_to: Some((_, _, kind)),
                ..
            } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        distinct(&st.seats[of.index()].hand, |c| kind.matches(c))
                            .into_iter()
                            .map(Choice::FindCard),
                    )
                    .collect(),
            ),
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
            Interrupt::BurnOrFail { minion, n, .. } if st.minions[minion.index()].blood < n => {
                (WindowLabel::ResolutionChoice, vec![Choice::Pass])
            }
            Interrupt::BurnOrFail { .. } | Interrupt::AvoidDamage { .. } => (
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
            Interrupt::DiscardChosen { of, optional, .. } => (
                WindowLabel::ResolutionChoice,
                optional
                    .then_some(Choice::Pass)
                    .into_iter()
                    .chain(
                        (0..st.seats[s.index()].hand.len())
                            .filter(|&i| crate::state::first_copy(&st.seats[s.index()].hand, i))
                            .filter(|&i| of.is_none_or(|f| f.matches(st.seats[s.index()].hand[i])))
                            .map(|i| Choice::Discard(i as u8)),
                    )
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
            Interrupt::SearchEquipFor {
                seat,
                minion,
                linked,
            } => (
                WindowLabel::LibrarySearch,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.alastor_equipment(seat, minion, linked)
                            .into_iter()
                            .map(Choice::FindCard),
                    )
                    .collect(),
            ),
            Interrupt::SecretNumber { max, .. } | Interrupt::GuessNumber { max, .. } => (
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
            | Interrupt::BurnBloodToAnarch { minion, .. }
            | Interrupt::MayJoinSect { minion, .. }
            | Interrupt::MayJoinCombat { minion, .. }
            | Interrupt::MoveForBlood { minion, .. }
            | Interrupt::CancelAttempt { minion, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ChooseMinion(minion)],
            ),
            Interrupt::GainVictimDiscipline { minion, victim, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.victim_disciplines(minion, victim)
                            .into_iter()
                            .map(Choice::ChooseDiscipline),
                    )
                    .collect(),
            ),
            Interrupt::SwapEquipFromPrey { seat, minion, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.prey_equipment(seat, minion)
                            .into_iter()
                            .map(Choice::FindCard),
                    )
                    .collect(),
            ),
            Interrupt::MakeAnarch { card, .. }
            | Interrupt::MayNewCombat { card, .. }
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
            Interrupt::MayPutDiscardedOnActor { .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.discarded_for_actor(front)
                            .into_iter()
                            .map(Choice::FindCard),
                    )
                    .collect(),
            ),
            Interrupt::MayReadyChosen { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolveRider { card: card.0 }],
            ),
            Interrupt::ChooseUncontrolledForCard { seat, .. } => (
                WindowLabel::ResolutionChoice,
                (0..st.seats[seat.index()].uncontrolled.len())
                    .map(|i| Choice::Slot(i as u8))
                    .collect(),
            ),
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
            Interrupt::ExchangeLooked { of, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        distinct(&st.seats[of.index()].ash, |_| true)
                            .into_iter()
                            .map(Choice::FindCard),
                    )
                    .collect(),
            ),
            Interrupt::LookPreySlot { of, .. } => (
                WindowLabel::ResolutionChoice,
                (0..st.seats[of.index()].uncontrolled.len() as u8)
                    .map(Choice::Slot)
                    .collect(),
            ),
            Interrupt::HideOneCard { seat, .. } => {
                let hand = &st.seats[seat.index()].hand;
                (
                    WindowLabel::ResolutionChoice,
                    (0..hand.len())
                        .filter(|&i| crate::state::first_copy(hand, i))
                        .map(|i| Choice::Discard(i as u8))
                        .collect(),
                )
            }
            Interrupt::HandCardAway { seat } => {
                let hand = &st.seats[seat.index()].hand;
                let firsts = (0..hand.len()).filter(|&i| crate::state::first_copy(hand, i));
                let options = firsts
                    .clone()
                    .map(|i| Choice::Discard(i as u8))
                    .chain(firsts.map(|i| Choice::TakeFrom {
                        zone: crate::decision::Zone::Hand,
                        card: hand[i],
                    }))
                    .collect();
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::DrawUpTo { seat, max } => (
                WindowLabel::ResolutionChoice,
                (1..=max.min(st.seats[seat.index()].library.len().min(255) as u8))
                    .map(Choice::SetX)
                    .collect(),
            ),
            Interrupt::BindLocation { minion, .. } => (
                WindowLabel::ResolutionChoice,
                st.bindable_locations(minion)
                    .into_iter()
                    .map(|c| Choice::ChooseCard(c.0))
                    .collect(),
            ),
            Interrupt::SecondCopyOnto { seat, card, .. } => {
                use crate::decision::Zone;
                let s = &st.seats[seat.index()];
                let mut options = vec![Choice::Pass];
                for (zone, pile) in [(Zone::Hand, &s.hand), (Zone::AshHeap, &s.ash)] {
                    if pile.contains(&card) {
                        options.push(Choice::TakeFrom { zone, card });
                    }
                }
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::ReturnBeforeBurn { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolveRider { card: card.0 }],
            ),
            Interrupt::RedirectFirst { strikers, .. } => (
                WindowLabel::ResolutionChoice,
                strikers.map(Choice::ChooseMinion).to_vec(),
            ),
            Interrupt::DrainBeforeSteals { card, .. } | Interrupt::CombatStandIn { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolveRider { card: card.0 }],
            ),
            Interrupt::RetainerBurnsWeaponOrDamage { minion, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.weapons_on(minion)
                            .into_iter()
                            .map(|w| Choice::BurnInPlay { card: w.0 }),
                    )
                    .collect(),
            ),
            Interrupt::SearchOnto {
                seat,
                held,
                from_hand,
                paired,
                owed,
                ..
            } => {
                use crate::decision::Zone;
                use crate::state::holding_matches;
                let s = &st.seats[seat.index()];
                let piles = if from_hand {
                    [(Zone::Hand, &s.hand), (Zone::Library, &s.library)]
                } else {
                    [(Zone::Library, &s.library), (Zone::AshHeap, &s.ash)]
                };
                let findable = |h| {
                    piles
                        .iter()
                        .any(|(_, p)| p.iter().any(|&c| holding_matches(c, h)))
                };
                // A pair's first card only with its mate findable.
                let wanted = |c: u16| match owed {
                    Some(h) => holding_matches(c, h),
                    None => held.iter().any(|&h| {
                        holding_matches(c, h)
                            && (!paired || held.iter().any(|&o| o != h && findable(o)))
                    }),
                };
                let mut options = Vec::new();
                for (zone, pile) in piles {
                    options.extend(
                        distinct(pile, wanted)
                            .into_iter()
                            .map(|card| Choice::TakeFrom { zone, card }),
                    );
                }
                if owed.is_none() || options.is_empty() {
                    options.insert(0, Choice::Pass);
                }
                (WindowLabel::LibrarySearch, options)
            }
            Interrupt::DisciplineSearch {
                minion,
                named,
                hand_or_burn,
                no_hand,
                library_only,
                ..
            } => {
                let mut options = vec![Choice::Pass];
                options.extend(self.discipline_masters(
                    s,
                    minion,
                    named,
                    hand_or_burn,
                    no_hand,
                    library_only,
                ));
                (WindowLabel::LibrarySearch, options)
            }
            Interrupt::ExtraHires { seat, minion, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.employable_from_hand(seat, minion, None, false)
                            .into_iter()
                            .map(|(card, mode)| Choice::EmployFromHand { card, mode }),
                    )
                    .collect(),
            ),
            Interrupt::MayDrawExtra { .. } => {
                (WindowLabel::Draw, vec![Choice::Pass, Choice::DrawLibrary])
            }
            Interrupt::RetakeFromAsh {
                seat, card, copy, ..
            } => {
                let mut options = vec![Choice::DrawLibrary];
                if self.state.ash_index(seat, card, copy).is_some() {
                    options.push(Choice::FindCard(card));
                }
                (WindowLabel::Draw, options)
            }
            // Beside Visit from the Capuchin, replaced as the rules do
            // (`DrawLibrary`: its counter), its player's choice (rules-qa
            // 8.3; ANK 20200905).
            Interrupt::ReplaceFromAsh { seat, except } => (
                WindowLabel::Draw,
                self.replacement_rider(seat, None)
                    .map(|_| Choice::DrawLibrary)
                    .into_iter()
                    .chain(
                        self.ash_replacements(seat, except)
                            .into_iter()
                            .map(Choice::FindCard),
                    )
                    .collect(),
            ),
            Interrupt::ReplaceFromResearch { seat, .. } => {
                let mut options = vec![Choice::DrawLibrary];
                options.extend(
                    distinct(&st.seats[seat.index()].research, |_| true)
                        .into_iter()
                        .map(Choice::FindCard),
                );
                (WindowLabel::Draw, options)
            }
            Interrupt::BurnLocationsForPool { seat, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.own_locations(seat)
                            .into_iter()
                            .map(|c| Choice::ChooseCard(c.0)),
                    )
                    .collect(),
            ),
            Interrupt::PlacedClan { .. } => (
                WindowLabel::ResolutionChoice,
                crate::referendum::clans()
                    .into_iter()
                    .map(Choice::ChooseClan)
                    .collect(),
            ),
            Interrupt::PlacedDiscipline { card, among, .. } => (
                WindowLabel::ResolutionChoice,
                self.placed_disciplines(card, among)
                    .into_iter()
                    .map(Choice::ChooseDiscipline)
                    .collect(),
            ),
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
            Interrupt::MayLockCardForBlock { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolveRider { card: card.0 }],
            ),
            Interrupt::BloodToCardUpTo {
                from, least, max, ..
            } => (
                WindowLabel::ResolutionChoice,
                (least..=max.min(st.minions[from.index()].blood).max(least))
                    .map(Choice::SetX)
                    .collect(),
            ),
            Interrupt::StealUpTo { from, max, .. } | Interrupt::BurnBloodUpTo { from, max, .. } => {
                (
                    WindowLabel::ResolutionChoice,
                    (1..=max.min(st.minions[from.index()].blood).max(1))
                        .map(Choice::SetX)
                        .collect(),
                )
            }
            Interrupt::AllySearch {
                minion, recruit, ..
            } => {
                let mut options = vec![Choice::Pass];
                options.extend(self.searched_allies(s, minion, recruit));
                (WindowLabel::LibrarySearch, options)
            }
            Interrupt::RetainerSearch {
                minion,
                filter,
                off,
                hand,
                ..
            } => {
                let mut options = vec![Choice::Pass];
                options.extend(
                    self.searched_retainers(s, minion, filter, off, hand)
                        .into_iter()
                        .map(|(zone, card, _)| Choice::TakeFrom { zone, card }),
                );
                (WindowLabel::LibrarySearch, options)
            }
            Interrupt::CryptToReady { seat, below, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        distinct(&st.seats[seat.index()].crypt, |c| {
                            let v = &crate::crypt::CRYPT[c as usize];
                            !v.imbued && v.capacity < below
                        })
                        .into_iter()
                        .map(Choice::AshVampire),
                    )
                    .collect(),
            ),
            Interrupt::MayTakeTitle { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolveRider { card: card.0 }],
            ),
            Interrupt::MastersFromHand { seat, .. } => {
                let mut options = vec![Choice::Pass];
                options.extend(
                    crate::state::master_disciplines(&st.seats[seat.index()].hand)
                        .into_iter()
                        .map(|card| Choice::TakeFrom {
                            zone: Zone::Hand,
                            card,
                        }),
                );
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::LookedToHand { seat, n, .. } => {
                let lib = st.seats[seat.index()].library.to_cards();
                let mut top: Vec<u16> = lib.iter().rev().take(n as usize).copied().collect();
                top.sort_unstable();
                top.dedup();
                let mut options = vec![Choice::Pass];
                options.extend(top.into_iter().map(Choice::FindCard));
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::BurnFromLibraryTop {
                of,
                n,
                filter,
                optional,
                ..
            } => {
                let lib = st.seats[of.index()].library.to_cards();
                let mut top: Vec<u16> = lib
                    .iter()
                    .rev()
                    .take(n as usize)
                    .copied()
                    .filter(|&c| filter.matches(c))
                    .collect();
                top.sort_unstable();
                top.dedup();
                let mut options = if optional {
                    vec![Choice::Pass]
                } else {
                    Vec::new()
                };
                options.extend(top.into_iter().map(Choice::FindCard));
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::MayCancelCombat { .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::CancelCombat],
            ),
            Interrupt::RetainerDamageOrder {
                minion, retainer, ..
            } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::ChooseMinion(minion), Choice::ChooseCard(retainer.0)],
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
            Interrupt::XConvictions { minion, .. } => {
                let mut options = vec![Choice::Pass];
                options.extend(self.conviction_choices(minion));
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
            Interrupt::PoolOrVampire {
                seat, cap, card, ..
            } => (
                WindowLabel::RiderTerms,
                std::iter::once(Choice::BurnPool)
                    .chain(
                        st.minions_of(seat)
                            .filter(|&m| {
                                let by = st.cards[card.index()].owner;
                                st.minions[m.index()].is_vampire()
                                    && st.capacity_by(m, by).is_some_and(|c| c >= cap)
                            })
                            .map(Choice::ChooseMinion),
                    )
                    .collect(),
            ),
            Interrupt::ContestPayment { minion, .. } => (
                WindowLabel::RiderTerms,
                std::iter::once(Choice::BurnPool)
                    .chain(
                        self.contest_blood_sources(minion)
                            .into_iter()
                            .map(|m| Choice::BurnBloodFrom { minion: m }),
                    )
                    .collect(),
            ),
            Interrupt::BloodOrPool { seat, except, .. } => (
                WindowLabel::RiderTerms,
                std::iter::once(Choice::BurnPool)
                    .chain(
                        st.minions_of(seat)
                            .filter(|&m| {
                                let v = &st.minions[m.index()];
                                m != except && v.is_vampire() && v.ready() && v.blood > 0
                            })
                            .map(Choice::ChooseMinion),
                    )
                    .collect(),
            ),
            Interrupt::PoolOrNoTransfers { .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::BurnPool, Choice::Pass],
            ),
            Interrupt::PoolOrMinionBurned { seat, pool, .. } => (
                WindowLabel::ResolutionChoice,
                if st.seats[seat.index()].pool >= pool as i32 {
                    vec![Choice::BurnPool, Choice::Pass]
                } else {
                    vec![Choice::Pass]
                },
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
            Interrupt::LibraryToHand {
                may: Some(minion), ..
            } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolvePrinted { minion }],
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
            Interrupt::ResearchSearch { seat, .. } => (
                WindowLabel::LibrarySearch,
                std::iter::once(Choice::Pass)
                    .chain(
                        crate::state::distinct(&st.seats[seat.index()].library, |_| true)
                            .into_iter()
                            .map(Choice::FindCard),
                    )
                    .collect(),
            ),
            Interrupt::CryptTopPick { seat, n } => {
                let crypt = &st.seats[seat.index()].crypt;
                let k = crypt.len().saturating_sub(n as usize);
                (
                    WindowLabel::ResolutionChoice,
                    distinct(&crypt[k..], |_| true)
                        .into_iter()
                        .map(Choice::AshVampire)
                        .collect(),
                )
            }
            Interrupt::CryptSearch { .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.crypt_of_clan(front)
                            .into_iter()
                            .map(Choice::AshVampire),
                    )
                    .collect(),
            ),
            Interrupt::PreySlotHalf { of, .. } => (
                WindowLabel::ResolutionChoice,
                (0..st.seats[of.index()].uncontrolled.len())
                    .filter(|&i| st.seats[of.index()].uncontrolled[i].counters > 1)
                    .map(|i| Choice::Slot(i as u8))
                    .collect(),
            ),
            Interrupt::StoreFromAshOf { from, of, .. } => (
                WindowLabel::ResolutionChoice,
                crate::state::distinct(&st.seats[from.index()].ash, |c| {
                    crate::library::LIBRARY[c as usize].types.contains(&of)
                })
                .into_iter()
                .map(Choice::FindCard)
                .collect(),
            ),
            Interrupt::FavorAuction { offer, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::SetX(offer)],
            ),
            Interrupt::BloodForVotes { seat, min, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        st.minions_of(seat)
                            .filter(|&m| {
                                let v = &st.minions[m.index()];
                                v.ready()
                                    && v.is_vampire()
                                    && v.blood > 0
                                    && st.capacity_of(m).is_some_and(|c| c > min)
                            })
                            .map(Choice::ChooseMinion),
                    )
                    .collect(),
            ),
            // An empty one too, a legal no-op (D14).
            Interrupt::StealFromSlot {
                of,
                to,
                n,
                slot,
                affordable,
                ..
            } => (
                WindowLabel::ResolutionChoice,
                match slot {
                    None => (0..st.seats[of.index()].uncontrolled.len() as u8)
                        .filter(|&i| {
                            !affordable
                                || st.seats[of.index()].uncontrolled[i as usize].counters
                                    <= st.minions[to.index()].blood
                        })
                        .map(Choice::Slot)
                        .collect(),
                    Some(_) => (0..=n).map(Choice::SetX).collect(),
                },
            ),
            Interrupt::PreySlotToPool { of, transfers, .. } => (
                WindowLabel::ResolutionChoice,
                (0..st.seats[of.index()].uncontrolled.len())
                    .filter(|&i| st.slot_gives_blood(of, i, transfers))
                    .map(|i| Choice::Slot(i as u8))
                    .collect(),
            ),
            Interrupt::CircleBlood {
                seat,
                among,
                from,
                to,
                equipment,
                moving,
                paired,
            } => {
                let (sources, members) = self.move_among(seat, among);
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
                // Equipment only to a minion able to hold it.
                let holds = |c: CardId, t: MinionId| {
                    st.cards[c.index()].host != Some(t)
                        && st.can_hold(t, st.cards[c.index()].card, &[])
                };
                match (from, moving) {
                    (None, None) => {
                        options.extend(
                            sources
                                .iter()
                                .filter(|&&m| {
                                    self.state.minions[m.index()].blood > 0
                                        && members.iter().any(|&t| t != m && !used(m, t))
                                })
                                .map(|&m| Choice::ChooseMinion(m)),
                        );
                        if equipment {
                            for &m in &sources {
                                options.extend(
                                    self.state
                                        .equipment_on(m)
                                        .into_iter()
                                        .filter(|&c| {
                                            !self.state.pinned(c)
                                                && members.iter().any(|&t| holds(c, t))
                                        })
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
                        options.extend(
                            members
                                .iter()
                                .filter(|&&m| holds(c, m))
                                .map(|&m| Choice::ChooseMinion(m)),
                        );
                    }
                }
                (WindowLabel::ResolutionChoice, options)
            }
            Interrupt::BaseSearch {
                seat,
                minion,
                crypt_only,
            } => {
                let name = st.minions[minion.index()].name();
                let base = |c: &u16| {
                    *c != crate::state::CREATED
                        && crate::crypt::CRYPT[*c as usize].name == name
                        && !crate::crypt::CRYPT[*c as usize].adv
                };
                let sv = &st.seats[seat.index()];
                let ash: &[u16] = if crypt_only { &[] } else { &sv.ash_crypt };
                let mut found: Vec<u16> =
                    sv.crypt.iter().chain(ash).copied().filter(base).collect();
                found.sort_unstable();
                found.dedup();
                let mut options = vec![Choice::Pass];
                options.extend(found.into_iter().map(Choice::AshVampire));
                options.extend(
                    (0..sv.uncontrolled.len())
                        .filter(|&i| {
                            !crypt_only
                                && sv.uncontrolled[i].banished.is_none()
                                && base(&sv.uncontrolled[i].card)
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
            Interrupt::AwardTrophy {
                seat,
                minion,
                searched,
            } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(self.trophy_options(seat, minion, searched))
                    .collect(),
            ),
            Interrupt::TakeChosen { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolveRider { card: card.0 }],
            ),
            Interrupt::MoveClot { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolveRider { card: card.0 }],
            ),
            Interrupt::ClotFate { minion, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ChooseMinion(minion)],
            ),
            Interrupt::ChooseCardCircle { .. } => (
                WindowLabel::ResolutionChoice,
                crate::state::circles()
                    .into_iter()
                    .map(Choice::ChooseCircle)
                    .collect(),
            ),
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
            Interrupt::NameAnyAshMinion { from, .. } => (
                WindowLabel::ResolutionChoice,
                match from {
                    None => (0..st.seats.len())
                        .map(|i| SeatId(i as u8))
                        .filter(|t| {
                            let heap = &st.seats[t.index()];
                            !heap.ousted && !crate::state::ash_minions(heap).is_empty()
                        })
                        .map(Choice::ChooseSeat)
                        .collect(),
                    Some(t) => crate::state::ash_minions(&st.seats[t.index()])
                        .into_iter()
                        .map(|(c, crypt)| {
                            if crypt {
                                Choice::AshVampire(c)
                            } else {
                                Choice::FindCard(c)
                            }
                        })
                        .collect(),
                },
            ),
            Interrupt::BurnReplacement { minion, .. } => (
                WindowLabel::ResolutionChoice,
                self.burn_replacements(minion)
                    .into_iter()
                    .map(|r| r.choice(minion))
                    .collect(),
            ),
            Interrupt::KeepSpentBlood {
                minion, card, mode, ..
            } => (
                WindowLabel::ResolutionChoice,
                st.spent_blood_keepers(minion, card, mode)
                    .into_iter()
                    .map(|c| Choice::ChooseCard(c.0))
                    .collect(),
            ),
            Interrupt::BurnCopiesPick { card, .. } => (
                WindowLabel::ResolutionChoice,
                st.copies_in_play(card)
                    .into_iter()
                    .map(|c| Choice::ChooseCard(c.0))
                    .collect(),
            ),
            Interrupt::RepresentPicks { seat, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.represent_slots(seat)
                            .into_iter()
                            .map(|i| Choice::Slot(i as u8)),
                    )
                    .collect(),
            ),
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
            // Each price, payable down to 0 pool (a cost, Default readings).
            Interrupt::EquipFromHandOnEntry {
                seat,
                minion,
                filter,
            } => {
                let mut options = vec![Choice::Pass];
                options.extend(
                    self.equippable_from_hand(seat, minion, filter, false)
                        .into_iter()
                        .map(|(card, mode)| Choice::EmployFromHand { card, mode }),
                );
                (WindowLabel::EntryEffect, options)
            }
            Interrupt::BuyTitle { offers, .. } => (
                WindowLabel::EntryEffect,
                std::iter::once(Choice::Pass)
                    .chain(offers.iter().map(|&(n, _)| Choice::SetX(n)))
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
                seat,
                clan,
                may,
                older_than,
                ..
            } => (
                WindowLabel::ResolutionChoice,
                may.then_some(Choice::Pass)
                    .into_iter()
                    .chain(
                        self.uncontrolled_older(seat, clan, older_than)
                            .into_iter()
                            .map(|i| Choice::Slot(i as u8)),
                    )
                    .collect(),
            ),
            Interrupt::MayPutOn { held, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ChooseMinion(held.minion)],
            ),
            Interrupt::CoercedSide { minion, .. } => (
                WindowLabel::ResolutionChoice,
                vec![
                    Choice::Pass,
                    Choice::CastVotes {
                        minion,
                        favor: true,
                    },
                    Choice::CastVotes {
                        minion,
                        favor: false,
                    },
                ],
            ),
            Interrupt::PlayedCardToHand { seat, minion } => (
                WindowLabel::ResolutionChoice,
                self.memory_cards(seat, minion)
                    .into_iter()
                    .map(Choice::FindCard)
                    .collect(),
            ),
            Interrupt::MayLockForDisciplineSearch { seat, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.ready_vampires_of(seat)
                            .into_iter()
                            .map(Choice::ChooseMinion),
                    )
                    .collect(),
            ),
            Interrupt::MayAddCounter { card, .. }
            | Interrupt::MayGainPoolForCard { card, .. }
            | Interrupt::MayTorture { card, .. }
            | Interrupt::MayLockForBlood { card, .. }
            | Interrupt::MayBurnLookedMaster { card, .. }
            | Interrupt::MayLockToDiscardLooked { card, .. }
            | Interrupt::MayStoreAim { store: card, .. }
            | Interrupt::MayBottomTop { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolveRider { card: card.0 }],
            ),
            Interrupt::Bid {
                seat, target, high, ..
            } => {
                let controller = self.state.minions[target.index()].owner;
                let pool = self.state.seats[seat.index()].pool.max(0);
                let most = if seat == controller { 2 * pool } else { pool };
                let low = high.map_or(1, |(_, b)| b as i32 + 1);
                (
                    WindowLabel::ResolutionChoice,
                    std::iter::once(Choice::Pass)
                        .chain((low..=most.min(u8::MAX as i32)).map(|b| Choice::SetX(b as u8)))
                        .collect(),
                )
            }
            Interrupt::TakeCountersUpTo { card, max, .. } => {
                let max = max.min(self.state.cards[card.index()].counters);
                (
                    WindowLabel::ResolutionChoice,
                    (0..=max).map(Choice::SetX).collect(),
                )
            }
            Interrupt::TorporMassacre { seat, card } => {
                let st = &self.state;
                let torpid = (0..st.minions.len())
                    .map(|i| crate::ids::MinionId(i as u32))
                    .filter(|&m| {
                        let v = &st.minions[m.index()];
                        v.present() && v.in_torpor && v.is_vampire()
                    });
                let wolves = st.minions_of(seat).filter(|&m| {
                    let v = &st.minions[m.index()];
                    v.ready() && !v.locked && st.ally_of(m, &[crate::library::Label::Werewolf])
                });
                let hand = &st.seats[seat.index()].hand;
                let copy = hand
                    .iter()
                    .position(|&c| c == card)
                    .map(|i| Choice::Discard(i as u8));
                (
                    WindowLabel::ResolutionChoice,
                    std::iter::once(Choice::Pass)
                        .chain(torpid.chain(wolves).map(Choice::ChooseMinion))
                        .chain(copy)
                        .collect(),
                )
            }
            Interrupt::MayTakeEdge { seat } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ChooseSeat(seat)],
            ),
            Interrupt::MaySkipTurn { .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::SkipTurn],
            ),
            Interrupt::MayGainPool { minion, .. }
            | Interrupt::MayBurnEdgeForPool { minion, .. }
            | Interrupt::MayBurnBloodToBurnPool { minion, .. }
            | Interrupt::MayGainHandSize { minion, .. }
            | Interrupt::MayGainBlood { minion, .. }
            | Interrupt::MayMoveEdgePool { minion, .. }
            | Interrupt::MayHuntBloodToPool { minion, .. }
            | Interrupt::MayPreventForBlood { minion, .. }
            | Interrupt::MayBurnBloodToPeek { minion, .. }
            | Interrupt::MayUnlock { minion, .. }
            | Interrupt::MayMoveBlood { from: minion, .. }
            | Interrupt::MayFailAction { minion, .. }
            | Interrupt::MayLockToFight { minion, .. }
            | Interrupt::MayBurnForBlock { minion, .. }
            | Interrupt::MayReturnDiscards { minion, .. }
            | Interrupt::MayBottomLooked { minion, .. }
            | Interrupt::MayTakeAggravatedAsNormal { minion, .. }
            | Interrupt::ReductionAtResolution { minion, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolvePrinted { minion }],
            ),
            Interrupt::BoonAuction {
                seat, target, high, ..
            } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.boon_bids(seat, target, high)
                            .into_iter()
                            .map(Choice::SetX),
                    )
                    .collect(),
            ),
            Interrupt::MayDiscardToKeep { seat, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.state
                            .hand_picks(seat, CardFilter::Type(crate::library::CardType::Master))
                            .into_iter()
                            .map(Choice::Discard),
                    )
                    .collect(),
            ),
            Interrupt::MayDiscardForBlock { seat, minion } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        block_discard_filter(&self.state, minion)
                            .map_or(Vec::new(), |f| self.state.hand_picks(seat, f))
                            .into_iter()
                            .map(Choice::Discard),
                    )
                    .collect(),
            ),
            Interrupt::ActionX { max, .. } => (
                WindowLabel::ResolutionChoice,
                (1..=max.max(1)).map(Choice::SetX).collect(),
            ),
            Interrupt::NamedX { minion, .. } => (
                WindowLabel::ResolutionChoice,
                (0..=st.minions[minion.index()].blood)
                    .map(Choice::SetX)
                    .collect(),
            ),
            Interrupt::MayBurnInsteadOfUnlock { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::BurnInPlay { card: card.0 }],
            ),
            Interrupt::BlockWouldSucceed { seat, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(self.block_moment_plays(seat))
                    .collect(),
            ),
            Interrupt::MayBurnWeapon { minion, .. } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain(
                        self.weapons_on(minion)
                            .into_iter()
                            .map(|w| Choice::BurnInPlay { card: w.0 }),
                    )
                    .collect(),
            ),
            Interrupt::MaySetNextRange { may, .. } => (
                WindowLabel::ResolutionChoice,
                may.then_some(Choice::Pass)
                    .into_iter()
                    .chain([Choice::SetX(0), Choice::SetX(1)])
                    .collect(),
            ),
            Interrupt::AshToLibraryTop {
                seat,
                filter,
                up_to,
                discarded,
                ..
            } => {
                let heap = &st.seats[seat.index()];
                let mut found: Vec<u16> = heap
                    .ash
                    .pairs()
                    .filter(|&(c, copy)| {
                        filter.matches(c) && (!discarded || heap.top_discards.0.contains(&copy))
                    })
                    .map(|(c, _)| c)
                    .collect();
                found.sort_unstable();
                found.dedup();
                (
                    WindowLabel::ResolutionChoice,
                    up_to
                        .then_some(Choice::Pass)
                        .into_iter()
                        .chain(found.into_iter().map(Choice::FindCard))
                        .collect(),
                )
            }
            Interrupt::CutLibrary { seat } => (
                WindowLabel::ResolutionChoice,
                std::iter::once(Choice::Pass)
                    .chain((1..st.seats[seat.index()].library.len()).map(|n| Choice::SetX(n as u8)))
                    .collect(),
            ),
            Interrupt::DualFormKept { root, .. } => (
                WindowLabel::ResolutionChoice,
                st.dual_set(root)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect(),
            ),
            Interrupt::Redraw { .. } => {
                (WindowLabel::Draw, vec![Choice::Pass, Choice::DrawLibrary])
            }
            Interrupt::PreyReplacement { seat, .. } => {
                let prey = st.prey(seat);
                let mut options = vec![Choice::DrawLibrary];
                if prey != seat && !st.seats[prey.index()].library.is_empty() {
                    options.push(Choice::ChooseSeat(prey));
                }
                (WindowLabel::Draw, options)
            }
            Interrupt::NameAshVampire { seat, burned } => (
                WindowLabel::ResolutionChoice,
                self.ash_vampire_names(seat, burned)
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
            Interrupt::MayRecycleCanceled { minion, .. }
            | Interrupt::MayMillCopy { minion, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolvePrinted { minion }],
            ),
            Interrupt::BurnOrder { .. } => (
                WindowLabel::ResolutionChoice,
                self.burn_order_left()
                    .into_iter()
                    .map(Choice::ChooseSeat)
                    .collect(),
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
            Interrupt::StoreOwnHand {
                seat,
                of,
                earmarked,
                optional,
                payer,
                ..
            } => (
                WindowLabel::ResolutionChoice,
                optional
                    .then_some(Choice::Pass)
                    .into_iter()
                    .chain(
                        self.paid_store_candidates(seat, of, earmarked, payer)
                            .into_iter()
                            .map(Choice::FindCard),
                    )
                    .collect(),
            ),
            Interrupt::StoreUpTo { seat, from_ash, .. } => {
                let s = &st.seats[seat.index()];
                let pile = if from_ash { &s.ash } else { &s.hand };
                (
                    WindowLabel::ResolutionChoice,
                    std::iter::once(Choice::Pass)
                        .chain(
                            crate::state::distinct(pile, |c| {
                                crate::cards::CardFilter::EquipmentWithout(&[]).matches(c)
                            })
                            .into_iter()
                            .map(Choice::FindCard),
                        )
                        .collect(),
                )
            }
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
            Interrupt::NameClan { minion, .. } => {
                let own = st.minions[minion.index()].clan();
                (
                    WindowLabel::ResolutionChoice,
                    crate::referendum::clans()
                        .into_iter()
                        .filter(|&c| Some(c) != own)
                        .map(Choice::ChooseClan)
                        .collect(),
                )
            }
            Interrupt::NameOwnAshCard { seat, of, upto } => (
                WindowLabel::ResolutionChoice,
                // "You can" (Tye Cooper), "up to" (Pochtli): none too; a
                // name again only for another copy.
                self.action
                    .as_ref()
                    .is_some_and(|a| a.ash_to_hand_on_name || upto)
                    .then_some(Choice::Pass)
                    .into_iter()
                    .chain(
                        crate::state::distinct(&st.seats[seat.index()].ash, |c| {
                            of.matches(c)
                                && (!upto
                                    || self.action.as_ref().is_some_and(|a| {
                                        let named = a.ash_names.iter().filter(|&&n| n == c).count();
                                        let have = st.seats[seat.index()]
                                            .ash
                                            .iter()
                                            .filter(|&&x| x == c)
                                            .count();
                                        named < have
                                    }))
                        })
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
            Interrupt::NameTargetAshCard { of, .. } => (
                WindowLabel::ResolutionChoice,
                crate::state::distinct(&st.seats[of.index()].ash, |_| true)
                    .into_iter()
                    .map(Choice::FindCard)
                    .collect(),
            ),
            Interrupt::MayCancelHuntingGround { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolveRider { card: card.0 }],
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
            Interrupt::MinionOf { seat, of, purpose } => {
                let mut options: Vec<Choice> = self
                    .minion_of_options(seat, of, purpose)
                    .into_iter()
                    .map(Choice::ChooseMinion)
                    .collect();
                if !matches!(
                    purpose,
                    MinionPurpose::StealFromTorpid { more: false, .. }
                        | MinionPurpose::BloodTo { .. }
                        | MinionPurpose::EquipFoundOn { .. }
                        | MinionPurpose::Corrupt
                        | MinionPurpose::CorruptAmong(_)
                        | MinionPurpose::BurnBloodOfClan(_)
                        | MinionPurpose::HostCard(_)
                        | MinionPurpose::LockOldest(..)
                        | MinionPurpose::OldestHuntingGround(_)
                        | MinionPurpose::BurnOfCrowdedClan(_)
                        | MinionPurpose::BurnSelfContested(_)
                        | MinionPurpose::BlindSpot(_)
                        | MinionPurpose::MoveOntoAny(..)
                        | MinionPurpose::MoveToYoungerLocked(_)
                        | MinionPurpose::Rearrange(_)
                        | MinionPurpose::LockOtherAnarch(_)
                        | MinionPurpose::LockYoungerThan(_)
                        | MinionPurpose::Damage(_)
                        | MinionPurpose::DamageVampire(_)
                        | MinionPurpose::DamageImbued { .. }
                        | MinionPurpose::DamageOtherVampire { .. }
                        | MinionPurpose::DamageOtherMinion { .. }
                        | MinionPurpose::BlockMark(_)
                        | MinionPurpose::JointBlocker(..)
                        | MinionPurpose::DamageVampireBy { .. }
                        | MinionPurpose::HolderAtSuccess(..)
                        | MinionPurpose::HostCardOn(..)
                        | MinionPurpose::WeaponDamage(_)
                        | MinionPurpose::GainBlood { .. }
                        | MinionPurpose::LockReady
                        | MinionPurpose::BloodFromTo { .. }
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
            Interrupt::EntryRemoveForLife { .. } => {
                let mut options = vec![Choice::Pass];
                options.extend(self.removable_for_life(s));
                (WindowLabel::EntryEffect, options)
            }
            Interrupt::EntryRemoveOrBurn {
                left, taken, any, ..
            } => {
                // One choice per card: copies are alike; burned only before
                // the first removal, and without enough cards.
                let seat = &st.seats[s.index()];
                let enough = seat.ash.len() + seat.ash_crypt.len() >= left as usize;
                let mut options = Vec::new();
                if taken == 0 {
                    options.push(Choice::Pass);
                }
                if any && enough {
                    options.extend(
                        distinct(&seat.ash, |_| true)
                            .into_iter()
                            .map(Choice::FindCard),
                    );
                    options.extend(
                        distinct(&seat.ash_crypt, |_| true)
                            .into_iter()
                            .map(Choice::AshVampire),
                    );
                    return Some(DecisionRequest::new(s, WindowLabel::EntryEffect, options));
                }
                if any {
                    return Some(DecisionRequest::new(s, WindowLabel::EntryEffect, options));
                }
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

    /// Alastor's half cost of `card`, rounded down: (pool, blood), X as 0;
    /// each half less its payer's reductions, the pool half the caller's,
    /// the blood half the chosen vampire's (owner, rules-qa 124.7; LSJ
    /// 20040518).
    fn alastor_cost(&self, seat: SeatId, minion: crate::ids::MinionId, card: u16) -> (u8, u8) {
        let st = &self.state;
        let pool = crate::cards::pool_cost(st, seat, card, crate::cards::Target::Minion(minion))
            .max(0) as u8;
        let blood = crate::cards::blood_cost(st, card, 0);
        let pool = (pool / 2).saturating_sub(st.seat_equip_discount(seat, card));
        let off = st.printed_cost_off(minion, card, 0);
        let blood = (blood / 2).saturating_sub(off.blood + off.either) + off.blood_extra;
        (pool, blood)
    }

    /// The equipment cards in `seat`'s library Alastor can put on `minion`:
    /// by card (the order is hidden), its half cost payable — the pool
    /// `seat`'s, the blood the vampire's. Requirements are ignored, not
    /// "only one vehicle" and its kin (rules-digest §3.17: never a second).
    /// Bauble's (`linked`): of its filters, at no cost.
    pub(crate) fn alastor_equipment(
        &self,
        seat: SeatId,
        minion: crate::ids::MinionId,
        linked: Option<(CardId, &'static [crate::cards::CardFilter])>,
    ) -> Vec<u16> {
        let st = &self.state;
        crate::state::distinct(&st.seats[seat.index()].library, |c| {
            let (pool, blood) = match linked {
                Some(_) => (0, 0),
                None => self.alastor_cost(seat, minion, c),
            };
            if linked.is_some_and(|(_, f)| !f.iter().all(|f| f.matches(c))) {
                return false;
            }
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
                self.state.gain_pool(s, n as i32);
            }
        }
        self.burn_pool_batch(&loss);
    }

    /// Rock-paper-scissors revealed: paper beats rock, scissors paper, rock
    /// scissors. `Hands`: a tie, each discards `tie_random` at random, else
    /// the loser their hand, all replaced. `Pool`: the winner steals the
    /// stake, as much as the loser has; a tie plays again for `raise` more.
    fn reveal_rock_paper_scissors(
        &mut self,
        player: SeatId,
        other: SeatId,
        stakes: crate::cards::RpsStakes,
    ) {
        use crate::cards::RpsStakes;
        let picks = std::mem::take(&mut self.secret_numbers);
        let pick = |s: SeatId| picks.iter().find(|&&(o, _)| o == s).map_or(1, |&(_, n)| n);
        let (p, o) = (pick(player), pick(other));
        let winner = match (p + 3 - o) % 3 {
            0 => None,
            1 => Some((player, other)),
            _ => Some((other, player)),
        };
        if [player, other]
            .iter()
            .any(|s| self.state.seats[s.index()].ousted)
        {
            return;
        }
        match (stakes, winner) {
            (RpsStakes::Hands { tie_random }, None) => {
                for s in [player, other] {
                    let k = (tie_random as usize).min(self.state.seats[s.index()].hand.len());
                    self.discard_at_random(s, k);
                    self.replace(s, k, None);
                }
            }
            (RpsStakes::Hands { .. }, Some((_, loser))) => {
                let k = self.state.seats[loser.index()].hand.len();
                self.discard_at_random(loser, k);
                self.replace(loser, k, None);
            }
            (RpsStakes::Pool { stake, raise }, None) => {
                let game = SecretGame::RockPaperScissors {
                    player,
                    other,
                    stakes: RpsStakes::Pool {
                        stake: stake.saturating_add(raise),
                        raise,
                    },
                };
                self.push_rock_paper_scissors(game);
            }
            (RpsStakes::Pool { stake, .. }, Some((winner, loser))) => {
                let taken = (stake as i32).min(self.state.seats[loser.index()].pool.max(0));
                let mut loss = vec![0i32; self.state.seats.len()];
                loss[loser.index()] = stake as i32;
                self.state.gain_pool(winner, taken);
                self.burn_pool_batch(&loss);
                // The player ousted by its own master on its turn: the turn
                // passes, as after a deferred oust.
                self.after_deferred_oust();
            }
        }
    }

    /// A round of rock-paper-scissors: the other Methuselah's pick, then
    /// the player's, the last revealing both.
    pub(crate) fn push_rock_paper_scissors(&mut self, game: SecretGame) {
        let SecretGame::RockPaperScissors { player, other, .. } = game else {
            unreachable!("not rock-paper-scissors: {game:?}");
        };
        for (seat, last) in [(other, false), (player, true)] {
            self.interrupts.push(Interrupt::SecretNumber {
                seat,
                max: 3,
                last,
                game,
            });
        }
    }

    /// Hostile Takeover's auction won: the bid to the vampire's controller
    /// and control to the bidder (still stoppable, its bid paid: LSJ
    /// 20040518), or half of it rounded up to the bank if the controller
    /// won. No bid, no change.
    fn settle_auction(&mut self, target: crate::ids::MinionId, high: Option<(SeatId, u8)>) {
        let Some((winner, bid)) = high else {
            return;
        };
        let controller = self.state.minions[target.index()].owner;
        let mut loss = vec![0i32; self.state.seats.len()];
        if winner == controller {
            loss[winner.index()] = (bid as i32 + 1) / 2;
            self.burn_pool_batch(&loss);
            return;
        }
        loss[winner.index()] = bid as i32;
        self.burn_pool_batch(&loss);
        self.state.gain_pool(controller, bid as i32);
        if self.state.minions[target.index()].present() && !self.state.seats[winner.index()].ousted
        {
            self.take_control(target, winner, crate::control::ControlTerm::ForGood);
        }
    }

    /// Malkavian Prank's reveal, all at once: a right guess takes that
    /// much of that Methuselah's pool, a wrong one gives them the number
    /// they held from the bank.
    fn reveal_guesses(&mut self, guesser: SeatId) {
        let picks = std::mem::take(&mut self.secret_numbers);
        let guesses = std::mem::take(&mut self.guesses);
        let mut loss = vec![0i32; self.state.seats.len()];
        for (of, guess) in guesses {
            let Some(&(_, held)) = picks.iter().find(|&&(s, _)| s == of) else {
                continue;
            };
            if guess == held {
                let taken = (held as i32).min(self.state.seats[of.index()].pool.max(0));
                loss[of.index()] += held as i32;
                self.state.gain_pool(guesser, taken);
            } else {
                self.state.gain_pool(of, held as i32);
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

    /// Nod: could `m` take `card` once the rearranging ends? Its "only
    /// one" limits read against the cards already placed, the ones still
    /// to place left out: all chosen, then moved (ANK 20190617-1).
    fn rearrange_fits(&self, m: crate::ids::MinionId, card: CardId) -> bool {
        let st = &self.state;
        let undecided: Vec<CardId> = self
            .interrupts
            .iter()
            .filter_map(|i| match *i {
                Interrupt::MinionOf {
                    purpose: MinionPurpose::Rearrange(c),
                    ..
                } => Some(c),
                _ => None,
            })
            .collect();
        let k = st.cards[card.index()].card;
        let Some(def) = crate::cards::lookup(k) else {
            return true;
        };
        !st.no_equipment(m)
            && !st.ranged_barred(m, k)
            && def
                .modes
                .iter()
                .flat_map(|md| md.requires.iter())
                .all(|r| match *r {
                    crate::cards::Requirement::ActorWithout(h) => {
                        !st.cards.iter().enumerate().any(|(i, c)| {
                            let id = CardId(i as u32);
                            c.present()
                                && c.host == Some(m)
                                && id != card
                                && !undecided.contains(&id)
                                && crate::state::holding_matches(c.card, h)
                        })
                    }
                    _ => true,
                })
    }

    /// The minions of `of` an `Interrupt::MinionOf` may choose.
    /// `by`: the chooser, whose card reads capacities (Blind Spot).
    pub(crate) fn minion_of_options(
        &self,
        by: SeatId,
        of: SeatId,
        purpose: MinionPurpose,
    ) -> Vec<crate::ids::MinionId> {
        let st = &self.state;
        let among: Vec<crate::ids::MinionId> = if matches!(
            purpose,
            MinionPurpose::MoveOntoAny(..)
                | MinionPurpose::MoveToYoungerLocked(_)
                | MinionPurpose::StealFromTorpid { .. }
                | MinionPurpose::BlockMark(_)
                | MinionPurpose::BloodFromTo { .. }
                | MinionPurpose::CorruptAmong(_)
                | MinionPurpose::SecessionCity(_)
        ) {
            (0..st.minions.len())
                .map(|i| crate::ids::MinionId(i as u32))
                .filter(|m| st.minions[m.index()].present())
                .collect()
        } else {
            st.minions_of(of).collect()
        };
        among
            .into_iter()
            .filter(|m| {
                let v = &st.minions[m.index()];
                match purpose {
                    MinionPurpose::LockVampire => v.is_vampire(),
                    MinionPurpose::BlindSpot(first) => v.is_ally() || st.younger_by(*m, first, by),
                    MinionPurpose::BurnAllyUpTo(n) => v.is_ally() && st.ally_cost(*m) <= n,
                    MinionPurpose::MoveOntoAny(_, not) => *m != not,
                    MinionPurpose::MoveToYoungerLocked(c) => st.cards[c.index()]
                        .host
                        .is_some_and(|h| v.is_vampire() && v.locked && st.younger_by(*m, h, by)),
                    // One option a city: its first holder in play.
                    MinionPurpose::SecessionCity(c) => {
                        let city = |x: crate::ids::MinionId| match crate::state::title_key(
                            st.claimed_title(x),
                        ) {
                            Some(crate::state::TitleKey::City(k)) => Some(k),
                            _ => None,
                        };
                        st.cards[c.index()].host != Some(*m)
                            && city(*m).is_some_and(|k| {
                                !(0..m.index()).any(|i| {
                                    let o = crate::ids::MinionId(i as u32);
                                    st.minions[i].present()
                                        && st.cards[c.index()].host != Some(o)
                                        && city(o) == Some(k)
                                })
                            })
                    }
                    MinionPurpose::Rearrange(c) => v.ready() && self.rearrange_fits(*m, c),
                    MinionPurpose::HealHost(h) => {
                        st.minions[h.index()].blood == 1
                            && v.ready()
                            && v.blood > 0
                            && st.sect_of(*m) == Some(crate::crypt::Sect::Anarch)
                    }
                    MinionPurpose::LockYoungerThan(than) => {
                        v.ready() && !v.locked && v.is_vampire() && st.younger_by(*m, than, by)
                    }
                    MinionPurpose::LockOtherAnarch(first) => {
                        *m != first
                            && v.ready()
                            && !v.locked
                            && st.sect_of(*m) == Some(crate::crypt::Sect::Anarch)
                    }
                    MinionPurpose::UnlockAnarch => {
                        v.ready() && st.sect_of(*m) == Some(crate::crypt::Sect::Anarch)
                    }
                    MinionPurpose::BurnBloodOfClan(c) => {
                        v.ready() && v.clan() == Some(c) && v.blood > 0
                    }
                    // Any minion, a torpid one too (RTR 19960112, rules-qa
                    // 88.9).
                    MinionPurpose::HostCard(c) => st.can_hold(*m, st.cards[c.index()].card, &[]),
                    MinionPurpose::HolderAtSuccess(c, h) | MinionPurpose::HostCardOn(c, h) => {
                        h.admits(v)
                            && st.cards[c.index()].host != Some(*m)
                            && st.can_hold(*m, st.cards[c.index()].card, &[])
                    }
                    MinionPurpose::Damage(_)
                    | MinionPurpose::WeaponDamage(_)
                    | MinionPurpose::BlockMark(_) => v.ready(),
                    MinionPurpose::DamageVampire(_) | MinionPurpose::DamageVampireBy { .. } => {
                        v.ready() && v.is_vampire()
                    }
                    MinionPurpose::GainBlood { below_capacity, .. } => {
                        v.ready()
                            && v.is_vampire()
                            && (!below_capacity || st.capacity_of(*m).is_some_and(|c| v.blood < c))
                    }
                    MinionPurpose::EquipFoundOn { card, copy } => {
                        self.equip_found_candidates(of, card, copy).contains(m)
                    }
                    MinionPurpose::BloodTo { not, .. } => {
                        *m != not && v.is_vampire() && v.blood > 0
                    }
                    MinionPurpose::StealFromTorpid { actor, younger, .. } => {
                        v.in_torpor
                            && v.is_vampire()
                            && !self.action.as_ref().is_some_and(|a| a.siphoned.contains(m))
                            && (!younger
                                || st.gehenna_in_play() > 0
                                || st.younger_by(*m, actor, by))
                    }
                    MinionPurpose::DamageImbued { except, .. } => {
                        v.ready() && v.is_imbued() && *m != except
                    }
                    MinionPurpose::DamageOtherVampire { except, .. } => {
                        v.ready() && v.is_vampire() && *m != except
                    }
                    MinionPurpose::DamageOtherMinion { except, .. } => v.ready() && *m != except,
                    MinionPurpose::BloodFromTo { from, clan, .. } => {
                        *m != from && v.ready() && v.is_vampire() && v.clan() == Some(clan)
                    }
                    MinionPurpose::GuessChosen { .. } => v.is_vampire(),
                    MinionPurpose::BecomeCombatant { .. } => {
                        v.ready() && self.free_fighters(by).contains(m)
                    }
                    MinionPurpose::CancelManeuver(o) => {
                        *m == o
                            && self
                                .combat
                                .as_ref()
                                .is_some_and(|c| c.maneuver_undo.is_some())
                    }
                    MinionPurpose::BurnSelfContested(card) => v.crypt_card().is_some_and(|c| {
                        crate::crypt::CRYPT[c as usize].name
                            == crate::crypt::CRYPT[card as usize].name
                    }),
                    MinionPurpose::BurnOfCrowdedClan(_) => {
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
                    MinionPurpose::LockOldest(c, by) => {
                        v.clan() == Some(c)
                            && st.capacity_by(*m, by)
                                == st
                                    .minions_of(of)
                                    .filter(|&o| st.minions[o.index()].clan() == Some(c))
                                    .filter_map(|o| st.capacity_by(o, by))
                                    .max()
                    }
                    MinionPurpose::HuntBloodTo { hunter, .. } => *m != hunter && v.is_vampire(),
                    MinionPurpose::JointBlocker(x, y) => (*m == x || *m == y) && v.ready(),
                    MinionPurpose::Lock | MinionPurpose::Corrupt => true,
                    MinionPurpose::CorruptAmong(at) => at.contains(v.owner),
                    MinionPurpose::LockReady => v.ready(),
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
    /// Imbued `card` of `from`'s ash heap into `s`'s ready region as an
    /// ally, its starting life, able to act (Pressing Flesh, LSJ
    /// 20100211); its ash heap stays `from`'s.
    pub(crate) fn raise_crypt_ally(
        &mut self,
        s: SeatId,
        card: u16,
        copy: crate::copies::CopyId,
        from: SeatId,
    ) -> MinionId {
        let life = crate::crypt::CRYPT[card as usize].capacity;
        let m = MinionId(self.state.minions.len() as u32);
        self.state.minions.push(crate::state::Minion {
            copy: crate::copies::Untracked(copy),
            origin: (from != s).then_some(from),
            ..crate::state::Minion::new(s, crate::state::MinionKind::CryptAlly(card), life)
        });
        crate::cards::widen(&mut self.state.printable.0, card);
        self.enter_card_contest(m);
        self.entered_play_riders(m);
        m
    }

    pub(crate) fn put_ally_in_play(
        &mut self,
        s: SeatId,
        card: u16,
        copy: crate::copies::CopyId,
    ) -> MinionId {
        let m = self.recruit_ally(s, card, copy, 0, 0);
        self.state.minions[m.index()].recruited = false;
        m
    }

    /// The allies in `seat`'s ash heap matching one of `of` (any, if
    /// empty), one entry per card.
    /// Corrupt Construction's removals: ally and retainer cards of the ash
    /// heap and hand, vampires of the ash heap (crypt cards are never in
    /// hand), one choice per card.
    pub(crate) fn removable_for_life(&self, seat: SeatId) -> Vec<Choice> {
        use crate::decision::Zone;
        let s = &self.state.seats[seat.index()];
        let minion = |c| crate::cards::CardFilter::AllyOrRetainer.matches(c);
        let mut options: Vec<Choice> = distinct(&s.ash, minion)
            .into_iter()
            .map(|card| Choice::TakeFrom {
                zone: Zone::AshHeap,
                card,
            })
            .collect();
        options.extend(
            distinct(&s.ash_crypt, crate::state::vampire_card)
                .into_iter()
                .map(Choice::AshVampire),
        );
        options.extend(
            distinct(&s.hand, minion)
                .into_iter()
                .map(|card| Choice::TakeFrom {
                    zone: Zone::Hand,
                    card,
                }),
        );
        options
    }

    /// The vampires of `seat`'s ash heap a `NameAshVampire` may name, by
    /// crypt card.
    fn ash_vampire_names(&self, seat: SeatId, burned: Option<bool>) -> Vec<u16> {
        match burned {
            None => crate::state::distinct(
                &self.state.seats[seat.index()].ash_crypt,
                crate::state::vampire_card,
            ),
            Some(since) => self.state.ash_burned_vampires(seat, &[], since),
        }
    }

    pub(crate) fn ash_allies(&self, seat: SeatId, of: &[crate::cards::CardFilter]) -> Vec<u16> {
        distinct(&self.state.seats[seat.index()].ash, |c| {
            crate::cards::ash_ally(c, of)
        })
    }

    /// The Methuselahs still in the game with a card `of` matches in their
    /// ash heap.
    pub(crate) fn ash_heaps_with(&self, of: crate::cards::CardFilter) -> Vec<SeatId> {
        (0..self.state.seats.len() as u8)
            .map(SeatId)
            .filter(|&s| {
                let seat = &self.state.seats[s.index()];
                !seat.ousted && seat.ash.iter().any(|&c| of.matches(c))
            })
            .collect()
    }

    pub(crate) fn ash_ally_heaps(&self) -> Vec<SeatId> {
        (0..self.state.seats.len() as u8)
            .map(SeatId)
            .filter(|&s| !self.state.seats[s.index()].ousted && !self.ash_allies(s, &[]).is_empty())
            .collect()
    }

    /// The distinct crypt cards of `clan` in `seat`'s crypt (Disciple of
    /// Lazarus's search; `None`: any vampire, no imbued, Evil Jensen), with
    /// `CryptSearch`'s `below`, `max_level` (Soul Scan), `name` (Gramle:
    /// that name's cards only, an imbued too), `scarce` and `with`.
    fn crypt_of_clan(&self, search: Interrupt) -> Vec<u16> {
        let Interrupt::CryptSearch {
            seat,
            clan,
            below,
            max_level,
            name,
            scarce,
            with,
            ..
        } = search
        else {
            unreachable!("not a crypt search");
        };
        distinct(&self.state.seats[seat.index()].crypt, |c| {
            let c = &crate::crypt::CRYPT[c as usize];
            name.is_none_or(|n| c.name == n)
                && (!scarce || c.scarce)
                && with.is_none_or(|d| c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d)))
                && clan.map_or(!c.imbued || name.is_some(), |clan| c.clan == clan)
                && below.is_none_or(|b| !c.imbued && c.capacity < b)
                && max_level.is_none_or(|(d, n)| {
                    let level = c
                        .disciplines
                        .iter()
                        .find(|x| x.eq_ignore_ascii_case(d))
                        .map_or(0, |x| {
                            if x.chars().all(|ch| ch.is_ascii_lowercase()) {
                                1
                            } else {
                                2
                            }
                        });
                    level <= n
                })
        })
    }

    /// `seat`'s uncontrolled vampires of `clan`, or all of them.
    /// As `uncontrolled_of`, of printed capacity above `older_than` if set.
    fn uncontrolled_older(
        &self,
        seat: SeatId,
        clan: Option<&'static str>,
        older_than: Option<u8>,
    ) -> Vec<usize> {
        self.uncontrolled_of(seat, clan)
            .into_iter()
            .filter(|&i| older_than.is_none_or(|c| self.state.slot_vampire(seat, i).0 > c))
            .collect()
    }

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
    /// clan, other than the first chosen for a second choice; with `of`, any
    /// minion of that seat.
    fn choosable(&self, front: Interrupt) -> Vec<crate::ids::MinionId> {
        let Interrupt::ChooseForCard {
            seat,
            card,
            clan,
            second,
            of,
        } = front
        else {
            unreachable!("not a choice for a card");
        };
        let st = &self.state;
        if let Some(of) = of {
            return st.minions_of(of).collect();
        }
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

    /// The cards a `MayPutDiscardedOnActor` may put: among the last ones of
    /// its heap, those of its filter, a location never.
    fn discarded_for_actor(&self, front: Interrupt) -> Vec<u16> {
        let Interrupt::MayPutDiscardedOnActor { from, last, of, .. } = front else {
            unreachable!("not a discard put");
        };
        let ash = &self.state.seats[from.index()].ash;
        let start = ash.len().saturating_sub(last as usize);
        let mut out: Vec<u16> = (start..ash.len())
            .map(|i| ash[i])
            .filter(|&c| of.matches(c))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Card `id`'s chosen vampire: `seat`'s uncontrolled slot `i`.
    fn choose_uncontrolled_for(&mut self, id: CardId, seat: SeatId, i: usize) {
        let u = &self.state.seats[seat.index()].uncontrolled[i];
        let (card, copy) = (u.card, u.copy);
        let c = &mut self.state.cards[id.index()];
        c.chosen_crypt = Some(card);
        c.chosen_copy = copy;
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
    /// 20050117); their burning does not burn it again. Several of the set
    /// leaving at once (Edged Illusion [CHI]), their controller keeps one
    /// of them, the others burned (`Interrupt::DualFormKept`, LSJ
    /// 20050805; rules-qa 96.1).
    fn burn_dual_forms(&mut self, ms: &[crate::ids::MinionId]) {
        let mut handled: Vec<crate::ids::MinionId> = Vec::new();
        for &m in ms {
            let st = &self.state;
            if !st.minions[m.index()].ready()
                || st.linked_leaving.contains(&m)
                || handled.contains(&m)
            {
                continue;
            }
            let root = st.minions[m.index()].dual_of.unwrap_or(m);
            let set: Vec<crate::ids::MinionId> = (0..st.minions.len())
                .map(|i| crate::ids::MinionId(i as u32))
                .filter(|&o| {
                    o != m
                        && (o == root || st.minions[o.index()].dual_of == Some(root))
                        && st.minions[o.index()].present()
                        && !st.linked_leaving.contains(&o)
                })
                .collect();
            let (together, others): (Vec<_>, Vec<_>) = set
                .into_iter()
                .partition(|o| ms.contains(o) && st.minions[o.index()].ready());
            if !together.is_empty() {
                handled.push(m);
                handled.extend(&together);
                let seat = st.minions[m.index()].owner;
                self.interrupts.push(Interrupt::DualFormKept {
                    seat,
                    root,
                    first: m,
                });
            }
            if others.is_empty() {
                continue;
            }
            self.state.linked_leaving.push(m);
            self.state.linked_leaving.extend(&together);
            self.lost_together(|e| {
                for o in others {
                    if e.state.minions[o.index()].present() {
                        e.burn_minion(o);
                    }
                }
            });
            self.state
                .linked_leaving
                .retain(|x| *x != m && !together.contains(x));
        }
    }

    /// Does `seat` hold a card with a mode for `window` (cheap, before the
    /// options are built)?
    pub(crate) fn holds_card_for(&self, seat: SeatId, window: crate::cards::Window) -> bool {
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
                .filter(|(i, c)| {
                    c.present() && c.host == Some(from) && self.state.is_retainer(CardId(*i as u32))
                })
                .map(|(i, _)| CardId(i as u32)),
        );
        out
    }

    /// The cards The Art of Memory may move for `minion`: those it played
    /// this action still in `seat`'s ash heap, and the action card bound
    /// for it (LSJ 20041115).
    pub(crate) fn memory_cards(&self, seat: SeatId, minion: crate::ids::MinionId) -> Vec<u16> {
        let Some(a) = self.action.as_ref() else {
            return Vec::new();
        };
        let ash = &self.state.seats[seat.index()].ash;
        let mut out: Vec<u16> = a
            .played_by
            .iter()
            .filter(|&&(m, c, k)| {
                m == minion && (0..ash.len()).any(|i| ash.to_cards()[i] == c && ash.copy(i) == k)
            })
            .map(|&(_, c, _)| c)
            .collect();
        if a.acting == minion && a.seat == seat {
            out.extend(self.card_bound_for_ash(a));
        }
        out.sort_unstable();
        out.dedup();
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
            .minion_of_options(seat, seat, MinionPurpose::HostCard(card))
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
            self.state.minions[m.index()].take_for_good(seat);
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
        let effects = def.map_or(&[][..], |d| d.modes[0].effects);
        // Tomb of Rameses III: a vampire of the new controller's
        // uncontrolled region, the old choice gone.
        if effects.contains(&crate::cards::Effect::RechoosesOnControlChange)
            && effects.contains(&crate::cards::Effect::ChooseUncontrolledTarget)
        {
            self.state.cards[card.index()].chosen_crypt = None;
            self.interrupts
                .push(Interrupt::ChooseUncontrolledForCard { seat, card });
        } else if effects.contains(&crate::cards::Effect::RechoosesOnControlChange) {
            self.interrupts.push(Interrupt::ChooseForCard {
                seat,
                card,
                clan: None,
                second: false,
                of: None,
            });
        }
    }

    /// Take `seat`'s uncontrolled slot `i` out of the game: the cards on it
    /// burn.
    pub(crate) fn burn_slot(&mut self, seat: SeatId, i: usize) -> crate::state::UncontrolledSlot {
        let (slot, on) = self.state.take_slot(seat, i);
        for c in on {
            self.burn_card_in_play(c);
        }
        slot
    }

    /// Remove `seat`'s uncontrolled vampire `i` from the game, its counters
    /// to the bank; a banished one's cards burn with it.
    pub(crate) fn remove_uncontrolled(&mut self, seat: SeatId, i: usize) {
        let slot = self.burn_slot(seat, i);
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
                self.state.seats[owner.index()].to_ash(card, copy);
            }
        }
        if slot.card != crate::state::CREATED {
            self.state.seats[seat.index()]
                .removed_crypt
                .put(slot.card, *slot.copy);
        }
    }

    /// Burn `seat`'s uncontrolled vampire `i` (Might of the Camarilla): to
    /// its owner's ash heap, its counters to the bank, a banished one's
    /// cards burned with it; not burned from play (rules-qa 113.52).
    pub(crate) fn burn_uncontrolled(&mut self, seat: SeatId, i: usize) {
        let slot = self.burn_slot(seat, i);
        let mut owner = seat;
        if let Some(b) = slot.banished {
            owner = self.state.minions[b.minion.index()].owner_seat();
            for c in b.cards {
                let k = &self.state.cards[c.index()];
                let (to, card, copy) = (k.owner, k.card, *k.copy);
                self.state.seats[to.index()].to_ash(card, copy);
            }
        }
        if slot.card != crate::state::CREATED {
            self.state.seats[owner.index()]
                .ash_crypt
                .put(slot.card, *slot.copy);
        }
    }

    /// The Discipline master cards `s` could take for `minion`, by zone and
    /// card: the library's by card (its order hidden), the hand's and ash
    /// heap's (`hand_only`: the hand's; `no_hand`: not the hand's;
    /// `library_only`: the library's); not one of a Discipline `minion` has at
    /// superior.
    fn discipline_masters(
        &self,
        s: SeatId,
        minion: crate::ids::MinionId,
        named: &'static [&'static str],
        hand_only: bool,
        no_hand: bool,
        library_only: bool,
    ) -> Vec<Choice> {
        let st = &self.state;
        let seat = &st.seats[s.index()];
        // Put on: the card's requirements ignored (LSJ 20051116-2: "eg.
        // diablerie"), the level it adds still capped.
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
        let zones = if hand_only {
            &zones[1..2]
        } else if named.is_empty() && !library_only {
            &zones[..]
        } else {
            &zones[..1]
        };
        for &(zone, cards) in zones
            .iter()
            .filter(|&&(z, _)| !(no_hand && z == Zone::Hand))
        {
            out.extend(
                distinct(cards, |c| {
                    CardFilter::HasLabel(Label::Discipline).matches(c)
                        && (named.is_empty() || CardFilter::NamedAny(named).matches(c))
                        && fits(c)
                })
                .into_iter()
                .map(|card| Choice::TakeFrom { zone, card }),
            );
        }
        out
    }

    /// The Disciplines card `id`, in play on a vampire, may give it: those
    /// it lacks at superior, or the card's own list (`among`).
    fn placed_disciplines(
        &self,
        id: crate::state::CardId,
        among: &'static [&'static str],
    ) -> Vec<&'static str> {
        let st = &self.state;
        let c = &st.cards[id.index()];
        match c.host.filter(|_| c.present()) {
            Some(_) if !among.is_empty() => among.to_vec(),
            Some(m) => crate::state::disciplines()
                .into_iter()
                .filter(|d| st.discipline_level(m, d) < 2)
                .collect(),
            None => Vec::new(),
        }
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
        // Read by the Purge's player, its host's controller (Orun's).
        let by = st.minions[host.index()].owner;
        let cap = st.capacity_by(host, by).unwrap_or(0);
        st.minions_of(of)
            .filter(|&m| {
                let v = &st.minions[m.index()];
                v.ready() && v.is_vampire() && st.capacity_by(m, by).is_some_and(|c| c <= cap)
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
    /// The ready vampires that cast votes against the referendum under way,
    /// not canceled, arena order (Terrifisto).
    pub(crate) fn against_voters(&self) -> Vec<MinionId> {
        let Some(r) = self.referendum.as_ref() else {
            return Vec::new();
        };
        let mut out: Vec<MinionId> = r
            .records
            .iter()
            .filter(|v| !v.canceled && !v.favor)
            .filter_map(|v| match v.source {
                crate::referendum::Source::Vampire(m) => Some(m),
                _ => None,
            })
            .filter(|&m| self.state.minions[m.index()].ready())
            .collect();
        out.sort_by_key(|m| m.0);
        out.dedup();
        out
    }

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
            self.state.seats[owner.index()].to_ash(card, copy);
        }
    }

    /// The mage allies Antonio d'Erlette (`m`) could play, by zone and card:
    /// payable after reductions (LSJ 20090920-2), no unique already in play;
    /// `recruit`: the library's allies `m` could recruit (The Summoning).
    fn searched_allies(&self, s: SeatId, m: MinionId, recruit: bool) -> Vec<Choice> {
        let st = &self.state;
        let seat = &st.seats[s.index()];
        let clan = st.minions[m.index()].clan();
        let ok = |c: u16| {
            let lib = &crate::library::LIBRARY[c as usize];
            let versions = self.recruit_versions(s, m, c).filter(|_| recruit);
            let version = versions.map(|v| v.trailing_zeros() as u8);
            let (pool, blood) = self.searched_ally_cost(s, m, c, version);
            (if recruit {
                lib.types.contains(&crate::library::CardType::Ally) && version.is_some()
            } else {
                CardFilter::HasLabel(Label::Mage).matches(c)
            }) && (lib.clans.is_empty() || clan.is_some_and(|k| lib.clans.contains(&k)))
                && pool <= seat.pool
                && blood <= st.minions[m.index()].blood
                && !(lib.labels.contains(&Label::Unique)
                    && st.cards.iter().enumerate().any(|(i, x)| {
                        x.present() && x.card == c && st.card_controller(CardId(i as u32)) == s
                    }))
        };
        let zones = [(Zone::Library, &seat.library), (Zone::Hand, &seat.hand)];
        let zones = if recruit { &zones[..1] } else { &zones[..] };
        let mut out = Vec::new();
        for &(zone, cards) in zones {
            out.extend(
                distinct(cards, ok)
                    .into_iter()
                    .map(|card| Choice::TakeFrom { zone, card }),
            );
        }
        out
    }

    /// Pandora's Whisper [MYT]: `s` discarded down; its discards may go on
    /// its library, one at a time, the last on top.
    pub(crate) fn end_top_discards(&mut self, s: SeatId) {
        let seat = &mut self.state.seats[s.index()];
        seat.discards_may_top.0 = 0;
        let left = seat.top_discards.0.len() as u8;
        if left == 0 {
            return;
        }
        self.interrupts.push(Interrupt::AshToLibraryTop {
            seat: s,
            left,
            filter: CardFilter::Any,
            bottom: false,
            up_to: true,
            discarded: true,
        });
    }

    /// The retainers of `s`'s library (and hand with `hand`) matching
    /// `filter` that `m` could employ now, `off` less, by zone, with the
    /// versions it may (a bit a mode; Muricia's Call, Children of Stone;
    /// LSJ 20080803).
    fn searched_retainers(
        &self,
        s: SeatId,
        m: MinionId,
        filter: CardFilter,
        off: u8,
        hand: bool,
    ) -> Vec<(Zone, u16, u16)> {
        let zones = [Zone::Library, Zone::Hand];
        let mut out: Vec<(Zone, u16, u16)> = Vec::new();
        for &zone in &zones[..if hand { 2 } else { 1 }] {
            for (card, mode) in self.employable_from(s, m, Some(filter), false, zone, off) {
                match out.iter_mut().find(|(z, c, _)| *z == zone && *c == card) {
                    Some(found) => found.2 |= 1 << mode,
                    None => out.push((zone, card, 1 << mode)),
                }
            }
        }
        out
    }

    /// The versions of ally `card` vampire `m` may recruit, a bit a mode,
    /// its requirements met as normal, never as if (LSJ 20050119: Vlad
    /// Tepes's sect and title too), by its Disciplines (LSJ 20080803),
    /// payable (`searched_ally_cost`); none: `None`.
    fn recruit_versions(&self, s: SeatId, m: MinionId, card: u16) -> Option<u16> {
        let st = &self.state;
        let def = cards::lookup(card)?;
        let payable = |mi: usize| {
            let (pool, blood) = self.searched_ally_cost(s, m, card, Some(mi as u8));
            pool <= st.seats[s.index()].pool && blood <= st.minions[m.index()].blood
        };
        let bits = (0..def.modes.len())
            .filter(|&mi| {
                payable(mi)
                    && cards::has_disciplines(st, m, card, mi)
                    && def.modes[mi].requires.iter().all(|&r| match r {
                        cards::Requirement::ActorSectIn(sects) => {
                            st.sect_of(m).is_some_and(|x| sects.contains(&x))
                        }
                        cards::Requirement::ActorTitleIn(classes) => {
                            classes.iter().any(|c| c.matches(st.title_of(m)))
                        }
                        _ => cards::holds(r, st, s, Some(m), None, None, None, None, false),
                    })
            })
            .fold(0u16, |bits, mi| bits | 1 << mi);
        (bits != 0).then_some(bits)
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
            if holds
                && st
                    .older_ready_untitled(clan, sect, h, c.owner)
                    .next()
                    .is_none()
            {
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
        if let Some(t) = self.state.cards[card.index()].represented_title() {
            self.gain_title(m, t, Some(card));
        }
    }

    /// The oldest ready untitled vampires a moving title card may go to.
    fn oldest_for(&self, card: CardId) -> Vec<MinionId> {
        let st = &self.state;
        let Some((clan, sect)) = moving_title(st.cards[card.index()].card) else {
            return Vec::new();
        };
        // Persona Non Grata's bearer cannot gain it.
        let title = st.cards[card.index()].represented_title();
        let all: Vec<MinionId> = st
            .ready_untitled(clan, sect)
            .filter(|&m| title.is_none_or(|t| st.may_gain_title(m, t)))
            .collect();
        let by = st.cards[card.index()].owner;
        let top = all.iter().filter_map(|&m| st.capacity_by(m, by)).max();
        all.into_iter()
            .filter(|&m| st.capacity_by(m, by) == top)
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

    /// A searched ally (`Interrupt::AllySearch`), its reductions picked
    /// (`ctx.discounts`): paid, then recruited or put in play.
    pub(crate) fn ally_found(
        &mut self,
        ctx: crate::cards::PlayCtx,
        recruit: bool,
        from_hand: bool,
    ) {
        let (s, card) = (ctx.seat, ctx.card);
        let minion = ctx.actor.expect("searching vampire");
        let mode = recruit.then_some(ctx.mode);
        let (pool, blood) = self.searched_ally_cost(s, minion, card, mode);
        let offs = self.take_discounts(&ctx);
        let (pool, blood, used) = self.reduced(pool, blood, (offs.0, offs.1, &offs.2));
        self.spend_counters(&used);
        let paid = Cause::Cost(Source::Played {
            card,
            copy: *ctx.copy,
        });
        self.caused(paid, |e| e.state.lose_blood(minion, blood));
        match mode {
            // Recruited: it cannot act this turn (RBK recruit-ally); the
            // vampire's recruit riders.
            Some(mode) => {
                self.recruit_ally(s, card, *ctx.copy, mode, 0);
                self.recruits_or_employs_riders(minion);
            }
            None => {
                self.put_ally_in_play(s, card, *ctx.copy);
            }
        }
        if from_hand {
            self.replace(s, 1, None);
        }
        self.state.seats[s.index()].library.shuffle(&mut self.rng);
        // Its pool paid last: an oust ends the search.
        if pool > 0 {
            let mut loss = vec![0i32; self.state.seats.len()];
            loss[s.index()] = pool;
            self.caused(paid, |e| e.burn_pool_batch(&loss));
        }
    }

    /// A searched retainer (`Interrupt::RetainerSearch`), its reductions
    /// picked (`ctx.discounts`) and `off` taken: paid, then employed with
    /// `extra_life`; the library shuffled if it was searched.
    pub(crate) fn retainer_found(
        &mut self,
        ctx: crate::cards::PlayCtx,
        extra_life: u8,
        off: u8,
        searched: bool,
    ) {
        let (s, card, mode) = (ctx.seat, ctx.card, ctx.mode);
        let minion = ctx.actor.expect("searching vampire");
        let (mut pool, mut blood) = self.employ_cost(s, minion, card, mode);
        crate::plays::reduce(&mut pool, &mut blood, off);
        let offs = self.take_discounts(&ctx);
        let (pool, blood, used) = self.reduced(pool, blood, (offs.0, offs.1, &offs.2));
        self.spend_counters(&used);
        let blood = self.blood_if_any(minion, card, mode, blood);
        let paid = Cause::Cost(Source::Played {
            card,
            copy: *ctx.copy,
        });
        self.caused(paid, |e| e.state.burn_blood(minion, blood));
        let id = self.put_retainer(s, minion, ctx, false);
        self.state.cards[id.index()].counters += extra_life;
        self.state.cards[id.index()].extra_life.0 += extra_life;
        self.recruits_or_employs_riders(minion);
        if searched {
            self.state.seats[s.index()].library.shuffle(&mut self.rng);
        }
        // Its pool paid last: an oust ends the search.
        if pool > 0 {
            let mut loss = vec![0i32; self.state.seats.len()];
            loss[s.index()] = pool;
            self.caused(paid, |e| e.burn_pool_batch(&loss));
        }
    }

    /// One of Zhenga's hires (`Interrupt::ExtraHires`), its reductions
    /// picked: hired, its entry effects before the next pick (War Ghoul
    /// burning the first; LSJ 20070707), the hires asked on.
    pub(crate) fn extra_hire(&mut self, ctx: crate::cards::PlayCtx, left: u8, hired: u8) {
        let (seat, minion) = (ctx.seat, ctx.actor.expect("the hiring vampire"));
        let before = self.interrupts.len();
        self.hire_from_hand(ctx);
        let entered = self.interrupts.split_off(before);
        self.interrupts.insert(
            0,
            Interrupt::ExtraHires {
                seat,
                minion,
                left: left - 1,
                hired: hired + 1,
            },
        );
        for (i, x) in entered.into_iter().enumerate() {
            self.interrupts.insert(i, x);
        }
    }

    /// The (pool, blood) a searched ally costs, `m`'s printed reductions
    /// and the cost modifiers in play applied; `recruit`, the version
    /// recruited, its surcharges on `m` too, as a normal recruit pays them
    /// (Darlene Killian, Jenna Cross; rules-qa 119.2).
    fn searched_ally_cost(
        &self,
        s: SeatId,
        m: MinionId,
        card: u16,
        recruit: Option<u8>,
    ) -> (i32, u8) {
        use crate::library::Cost;
        let lib = &crate::library::LIBRARY[card as usize];
        let mode = recruit.unwrap_or(0);
        let off = self.state.printed_cost_off(m, card, mode);
        let mut pool = cards::pool_cost(&self.state, s, card, crate::cards::Target::None);
        let mut blood = match lib.blood_cost {
            Cost::Fixed(n) => n,
            Cost::None | Cost::X => 0,
        };
        if recruit.is_some() {
            blood += self.card_surcharge(m, card, mode, 0);
        }
        off.apply(&mut pool, &mut blood);
        if recruit.is_some() {
            blood = self.blood_if_any(m, card, mode, blood);
        }
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
                Interrupt::Bid {
                    seat,
                    target,
                    high,
                    passes,
                },
                c,
            ) => {
                self.interrupts.remove(0);
                let (high, passes) = match c {
                    Choice::SetX(b) => (Some((seat, b)), 0),
                    _ => (high, passes + 1),
                };
                let bidders = self.state.clockwise_from(seat).len() as u8;
                let over = match high {
                    Some(_) => passes + 1 >= bidders,
                    None => passes >= bidders,
                };
                if over {
                    self.settle_auction(target, high);
                } else {
                    self.interrupts.insert(
                        0,
                        Interrupt::Bid {
                            seat: self.state.prey(seat),
                            target,
                            high,
                            passes,
                        },
                    );
                }
            }
            (Interrupt::TakeCountersUpTo { seat, card, .. }, Choice::SetX(n)) => {
                self.interrupts.remove(0);
                self.state.cards[card.index()].counters -= n;
                self.state.gain_pool(seat, n as i32);
                if self.state.cards[card.index()].counters == 0 {
                    self.burn_card_in_play(card);
                }
            }
            (Interrupt::TorporMassacre { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::TorporMassacre { seat, .. }, c) => {
                self.interrupts.remove(0);
                let st = &self.state;
                let all = match c {
                    Choice::ChooseMinion(m) if st.minions[m.index()].in_torpor => {
                        self.burn_minion(m);
                        false
                    }
                    Choice::ChooseMinion(m) => {
                        self.state.minions[m.index()].locked = true;
                        true
                    }
                    Choice::Discard(i) => {
                        let s = &mut self.state.seats[seat.index()];
                        let (card, copy) = s.hand.take(i as usize);
                        s.to_ash(card, copy);
                        self.replace(seat, 1, None);
                        true
                    }
                    other => unreachable!("not a massacre choice: {other:?}"),
                };
                if all {
                    let torpid: Vec<crate::ids::MinionId> = (0..self.state.minions.len())
                        .map(|i| crate::ids::MinionId(i as u32))
                        .filter(|&m| {
                            let v = &self.state.minions[m.index()];
                            v.present() && v.in_torpor && v.is_vampire()
                        })
                        .collect();
                    for m in torpid {
                        if self.state.minions[m.index()].present() {
                            self.burn_minion(m);
                        }
                    }
                }
            }
            (Interrupt::MayTakeEdge { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::MayTakeEdge { seat }, Choice::ChooseSeat(_)) => {
                self.interrupts.remove(0);
                self.gain_edge(seat);
            }
            (Interrupt::MaySkipTurn { seat }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.begin_turn(seat);
            }
            (Interrupt::MaySkipTurn { seat }, Choice::SkipTurn) => {
                self.interrupts.remove(0);
                self.skip_turn(seat);
            }
            (
                Interrupt::MinionOf {
                    purpose: MinionPurpose::HuntBloodTo { hunter, n },
                    ..
                },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
                self.state.gain_blood(hunter, n);
            }
            // Free Fight declined as its combatant left.
            (
                Interrupt::MinionOf {
                    purpose: MinionPurpose::BecomeCombatant { ending: true },
                    ..
                },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
                self.free_fight_declined();
            }
            (
                Interrupt::StealForCorruption { .. }
                | Interrupt::BurnBloodToAnarch { .. }
                | Interrupt::MayJoinSect { .. }
                | Interrupt::GainVictimDiscipline { .. }
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
                // The counters burn even if the change is stopped (LSJ
                // 20070222-2).
                let n = self.state.minions[minion.index()].corruption_of(s);
                self.state.burn_corruption(minion, s, n, s);
                self.take_control(minion, s, crate::control::ControlTerm::ForGood);
            }
            (Interrupt::SwapEquipFromPrey { seat, minion, play }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.put_equipment(seat, minion, play);
            }
            // Paid for the original only, nothing of the taken one's terms
            // applying (LSJ 20080702-4); it stays its Methuselah's card.
            (Interrupt::SwapEquipFromPrey { seat, minion, play }, Choice::FindCard(taken)) => {
                self.interrupts.remove(0);
                self.state.seats[seat.index()]
                    .ash
                    .put(play.card, *play.copy);
                let prey = self.state.prey(seat);
                let hand = &mut self.state.seats[prey.index()].hand;
                let i = hand.iter().position(|&c| c == taken).expect("in hand");
                let (_, copy) = hand.take(i);
                let entering =
                    crate::state::CardInPlay::entering(taken, copy, prey, Some(minion), 0);
                if let Some(id) = self.place_now(entering, crate::plays::ModeLayers::Push, false) {
                    self.enter_library_contest(id);
                }
            }
            (Interrupt::PreySlotHalf { of, .. }, Choice::Slot(i)) => {
                self.interrupts.remove(0);
                let slot = &mut self.state.seats[of.index()].uncontrolled[i as usize];
                slot.counters -= slot.counters / 2;
            }
            (
                Interrupt::BurnOrFail {
                    minion,
                    n,
                    card,
                    by,
                    to_card,
                    ..
                },
                choice,
            ) => {
                self.interrupts.remove(0);
                if let (true, Some(id), Choice::BurnBloodToAvoid) = (to_card, card, choice) {
                    self.state.lose_blood(minion, n);
                    self.state.cards[id.index()].counters += n;
                } else if matches!(choice, Choice::BurnBloodToAvoid) {
                    // An ally pays life, its last burning it (Scobax [STR]).
                    if self.paying(|e| e.state.burn_blood_or_life(minion, n)) {
                        self.lifeless(minion);
                    }
                } else if let Some(id) = card {
                    self.burn_card_in_play(id);
                } else if let Some(a) = self.action.as_mut() {
                    a.failed = true;
                    let acting_seat = a.seat;
                    self.hinder(by, acting_seat);
                }
            }
            (Interrupt::MayNewCombat { card, .. }, choice) => {
                self.interrupts.remove(0);
                if matches!(choice, Choice::ResolveRider { .. }) {
                    self.burn_card_in_play(card);
                    self.combat.as_mut().expect("combat context").psyche = true;
                }
                self.end_combat();
            }
            (Interrupt::MayJoinCombat { minion, side, .. }, choice) => {
                self.interrupts.remove(0);
                if matches!(choice, Choice::ChooseMinion(_)) {
                    let c = self.combat.as_mut().expect("combat context");
                    c.coordinate = Some((
                        minion,
                        side as usize,
                        false,
                        crate::cards::FirstRoundBar::StrikeCards,
                    ));
                    c.joined = true;
                }
                self.end_combat();
            }
            (Interrupt::GainVictimDiscipline { minion, .. }, Choice::ChooseDiscipline(d)) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].gained_levels.push(d);
            }
            (Interrupt::BurnBloodToAnarch { minion, .. }, Choice::ChooseMinion(_)) => {
                self.interrupts.remove(0);
                self.paying(|e| e.state.lose_blood(minion, 1));
                self.make_anarch(minion);
            }
            (Interrupt::MayJoinSect { minion, sect, .. }, Choice::ChooseMinion(_)) => {
                self.interrupts.remove(0);
                self.become_sect(minion, sect);
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
                self.caused(throne, |e| e.state.gain_pool(s, n as i32));
            }
            // Burned with its minion or not (PIB 20150512): the pool all
            // the same.
            (Interrupt::PoolForCard { card, n, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.burn_card_in_play(card);
                let gain = Cause::Effect(Source::Card(card));
                self.caused(gain, |e| e.state.gain_pool(s, n as i32));
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
                seat.to_ash(held, held_copy);
                seat.hand.put(card, copy);
            }
            (
                Interrupt::MayPutDiscardedOnActor {
                    seat, from, minion, ..
                },
                Choice::FindCard(card),
            ) => {
                self.interrupts.remove(0);
                let ash = &mut self.state.seats[from.index()].ash;
                let i = (0..ash.len())
                    .rev()
                    .find(|&i| ash[i] == card)
                    .expect("one of the discarded cards");
                let (card, copy) = ash.take(i);
                let play = cards::PlayCtx {
                    copy: crate::copies::Untracked(copy),
                    ..cards::PlayCtx::new(seat, card, 0, Some(minion), cards::Target::None, 0)
                };
                // Put, not equipped: no cost or requirement; the card stays
                // its owner's, controlled with its bearer.
                if let Some(id) = self.put_equipment(seat, minion, play) {
                    self.state.cards[id.index()].owner = from;
                }
            }
            (Interrupt::MayPutDiscardedOnActor { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::MayReadyChosen { seat, card }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                if let Some(i) = self.state.chosen_slot(card) {
                    let m = self.slot_to_ready(seat, i);
                    self.entry_tolls(seat, m);
                }
            }
            (Interrupt::MayReadyChosen { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::ChooseUncontrolledForCard { seat, card }, Choice::Slot(i)) => {
                self.interrupts.remove(0);
                self.choose_uncontrolled_for(card, seat, i as usize);
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
                self.remove_from_game(convert);
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
                self.caused(anarch, |e| e.state.gain_pool(seat, 1));
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
            (Interrupt::UncontrolledBlood { n, cause, .. }, Choice::Slot(i)) => {
                self.interrupts.remove(0);
                self.caused(cause, |e| e.state.add_uncontrolled_blood(s, i as usize, n));
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
            (
                Interrupt::MayRecycleCanceled { .. } | Interrupt::MayMillCopy { .. },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
            }
            // A search finding no copy only shuffles (rules-digest D14).
            (Interrupt::MayMillCopy { seat, card, .. }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                let s = &mut self.state.seats[seat.index()];
                if let Some(i) = s.library.iter().rposition(|&c| c == card) {
                    let (_, copy) = s.library.take(i);
                    s.to_ash(card, copy);
                    if let Some(a) = self
                        .action
                        .as_mut()
                        .filter(|a| a.card.is_some_and(|p| p.seat == seat && p.card == card))
                    {
                        a.copy_back = Some(card);
                    }
                }
                self.state.seats[seat.index()]
                    .library
                    .shuffle(&mut self.rng);
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
            (
                Interrupt::StoreOwnHand {
                    seat,
                    card: on,
                    left,
                    of,
                    replace,
                    earmarked,
                    optional,
                    payer,
                },
                Choice::FindCard(card),
            ) => {
                self.interrupts.remove(0);
                // Inscription: "this acting vampire pays the cost of that
                // card", its pool its Methuselah's.
                if let Some(p) = payer {
                    let (pool, blood) = self.store_cost(seat, card);
                    self.state.burn_blood(p, blood);
                    self.state.seats[seat.index()].lose_pool(pool);
                }
                let s = &mut self.state.seats[seat.index()];
                let i = (0..s.hand.len())
                    .rev()
                    .find(|&i| {
                        s.hand[i] == card && (!earmarked || s.earmarked.contains(&s.hand.copy(i)))
                    })
                    .expect("in hand");
                let (_, copy) = s.hand.take(i);
                if earmarked {
                    let k = s
                        .earmarked
                        .iter()
                        .position(|&c| c == copy)
                        .expect("earmarked");
                    s.earmarked.swap_remove(k);
                }
                self.state.cards[on.index()].stored.put(card, copy);
                self.state.cards[on.index()].stored_owner = Some(seat);
                if replace {
                    self.replace(seat, 1, None);
                }
                if earmarked && left <= 1 {
                    self.state.seats[seat.index()].earmarked.clear();
                }
                if left > 1 {
                    self.interrupts.insert(
                        0,
                        Interrupt::StoreOwnHand {
                            seat,
                            card: on,
                            left: left - 1,
                            of,
                            replace,
                            earmarked,
                            optional,
                            payer,
                        },
                    );
                }
            }
            (Interrupt::StoreOwnHand { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (
                Interrupt::StoreUpTo {
                    seat,
                    card: on,
                    left,
                    from_ash,
                },
                Choice::FindCard(card),
            ) => {
                self.interrupts.remove(0);
                let s = &mut self.state.seats[seat.index()];
                let pile = if from_ash { &mut s.ash } else { &mut s.hand };
                let i = pile.iter().rposition(|&c| c == card).expect("in the pile");
                let (_, copy) = pile.take(i);
                self.state.cards[on.index()].stored.put(card, copy);
                self.state.cards[on.index()].stored_owner = Some(seat);
                if !from_ash {
                    self.replace(seat, 1, None);
                }
                if left > 1 {
                    self.interrupts.insert(
                        0,
                        Interrupt::StoreUpTo {
                            seat,
                            card: on,
                            left: left - 1,
                            from_ash,
                        },
                    );
                }
            }
            (
                Interrupt::StoreUpTo {
                    seat,
                    card,
                    left,
                    from_ash,
                },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
                if !from_ash {
                    self.interrupts.insert(
                        0,
                        Interrupt::StoreUpTo {
                            seat,
                            card,
                            left,
                            from_ash: true,
                        },
                    );
                }
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
            (Interrupt::StoreFromAshOf { card: on, from, .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                self.store_from_ash(on, from, card);
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
            (Interrupt::NameClan { .. }, Choice::ChooseClan(clan)) => {
                self.interrupts.remove(0);
                self.action.as_mut().expect("action").clan_named = Some(clan);
            }
            (Interrupt::NameOwnAshCard { upto: true, .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                self.action.as_mut().expect("action").ash_names.push(card);
            }
            (Interrupt::NameOwnAshCard { upto: true, .. }, Choice::Pass) => {
                self.interrupts
                    .retain(|i| !matches!(i, Interrupt::NameOwnAshCard { upto: true, .. }));
            }
            (Interrupt::NameOwnAshCard { seat, .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                let a = self.action.as_mut().expect("action");
                a.ash_named = Some(card);
                if std::mem::take(&mut a.ash_to_hand_on_name) {
                    self.ash_named_to_hand(seat, Some(card));
                }
            }
            (Interrupt::FoundVersion { ctx, then, .. }, Choice::Version { mode, .. }) => {
                self.interrupts.remove(0);
                use crate::plays::CostThen;
                // An action's flow, as the choice finding the card left it.
                let resume = matches!(then, CostThen::EquipFound | CostThen::EquipFree)
                    .then(|| std::mem::replace(&mut self.flow, crate::game::Flow::GameOver));
                self.found_versioned(cards::PlayCtx { mode, ..ctx }, then, resume);
            }
            // Its version named with it, as the action is announced.
            (
                Interrupt::NameAshRetainer {
                    seat,
                    minion,
                    of,
                    extra,
                },
                Choice::FindCard(card),
            ) => {
                self.interrupts.remove(0);
                self.action.as_mut().expect("action").ash_named = Some(card);
                let versions = self
                    .ash_retainers(seat, minion, of, extra)
                    .into_iter()
                    .find_map(|(c, v)| (c == card).then_some(v))
                    .expect("an option of ash_retainers");
                let ctx = cards::PlayCtx::new(seat, card, 0, Some(minion), cards::Target::None, 0);
                self.found_version(ctx, versions, crate::plays::CostThen::AshNamed, None);
            }
            (Interrupt::NameOwnAshCard { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.action.as_mut().expect("action").ash_to_hand_on_name = false;
            }
            (Interrupt::NameAshVampire { .. }, Choice::AshVampire(card)) => {
                self.interrupts.remove(0);
                self.action.as_mut().expect("action").ash_vampire_named = Some(card);
            }
            (Interrupt::MayTakeTitle { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            // Contested with its holder (rules-qa 3.1, as superseded
            // 2026-11-11).
            (Interrupt::MayTakeTitle { card, title, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                let host = self.state.cards[card.index()].host.expect("a host");
                self.burn_card_in_play(card);
                self.gain_title(host, title, None);
            }
            (Interrupt::MastersFromHand { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            // Put on by The Becoming: played, not in the normal fashion
            // (rules-qa 119.1): no requirement nor cost.
            (
                Interrupt::MastersFromHand { seat, minion, left },
                Choice::TakeFrom {
                    zone: Zone::Hand,
                    card,
                },
            ) => {
                self.interrupts[0] = Interrupt::MastersFromHand {
                    seat,
                    minion,
                    left: left - 1,
                };
                let copy = self.take_from_zone(seat, Zone::Hand, card);
                self.put_on_minion(seat, card, copy, minion);
                self.replace(seat, 1, None);
            }
            (Interrupt::LookedToHand { seat, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.seats[seat.index()]
                    .library
                    .shuffle(&mut self.rng);
            }
            (Interrupt::LookedToHand { seat, n, left }, Choice::FindCard(card)) => {
                let lib = &mut self.state.seats[seat.index()].library;
                let len = lib.len();
                let k = (len.saturating_sub(n as usize)..len)
                    .rev()
                    .find(|&k| lib.to_cards()[k] == card)
                    .expect("one of the cards looked at");
                let (c, copy) = lib.take(k);
                self.state.seats[seat.index()].hand.put(c, copy);
                self.interrupts[0] = Interrupt::LookedToHand {
                    seat,
                    n: n - 1,
                    left: left - 1,
                };
            }
            (
                Interrupt::NameAshPair { .. } | Interrupt::NameTargetAshCard { .. },
                Choice::FindCard(card),
            ) => {
                self.interrupts.remove(0);
                self.action.as_mut().expect("action").ash_named = Some(card);
            }
            // Klaus van der Veken's cancel: nothing resolves, the use spent.
            (
                Interrupt::MayCancelHuntingGround {
                    card,
                    ability,
                    user_seat,
                    user,
                    ..
                },
                Choice::ResolveRider { .. },
            ) => {
                self.interrupts.remove(0);
                self.spend_canceled_ability(card, ability, user_seat, user);
            }
            (
                Interrupt::MayCancelHuntingGround {
                    card,
                    ability,
                    target,
                    x,
                    user_seat,
                    user,
                    ..
                },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
                self.resolve_cleared_ability(card, ability, target, x, user_seat, user);
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
            (
                Interrupt::SearchEquipFor {
                    seat,
                    minion,
                    linked,
                },
                Choice::FindCard(card),
            ) => {
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
                let (pool, blood) = match linked {
                    Some(_) => (0, 0),
                    None => self.alastor_cost(seat, minion, card),
                };
                self.state.burn_blood(minion, blood);
                // Put on, not equipped: its layers, inferior, nothing it
                // does as it is equipped (no cards stored, no minion
                // chosen: LSJ 20090506), no lock (LSJ 20090415).
                let entering =
                    crate::state::CardInPlay::entering(card, copy, seat, Some(minion), 0);
                if let Some(id) = self.place_now(entering, crate::plays::ModeLayers::Push, false) {
                    if let Some((by, _)) = linked {
                        self.state.cards[by.index()].chosen_card =
                            crate::state::SparseOption(Some(id));
                    }
                    // Mokolé Blood, holding nothing: burned (rules-qa 11.3).
                    if cards::searches_store(card) && self.state.cards[id.index()].stored.is_empty()
                    {
                        self.burn_card_in_play(id);
                    } else {
                        self.enter_library_contest(id);
                    }
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
            (Interrupt::GuessNumber { seat, of, last, .. }, Choice::SetX(n)) => {
                self.interrupts.remove(0);
                self.guesses.push((of, n));
                if last {
                    self.reveal_guesses(seat);
                }
            }
            (
                Interrupt::SecretNumber {
                    seat, last, game, ..
                },
                Choice::SetX(n),
            ) => {
                self.interrupts.remove(0);
                self.secret_numbers.push((seat, n));
                match game {
                    _ if !last => {}
                    SecretGame::Numbers => self.reveal_numbers(),
                    SecretGame::RockPaperScissors {
                        player,
                        other,
                        stakes,
                    } => self.reveal_rock_paper_scissors(player, other, stakes),
                }
            }
            (Interrupt::MoveOffBurnedBearer { card, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.burn_card_in_play(card);
            }
            (Interrupt::MoveForBlood { payer, n, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.state.lose_blood(payer, n);
                self.leave_torpor(m);
            }
            (Interrupt::RemoveUncontrolled { .. }, Choice::Slot(i)) => {
                self.interrupts.remove(0);
                self.remove_uncontrolled(s, i as usize);
            }
            (Interrupt::PursuitDiscard { seat, left }, Choice::Discard(i)) => {
                let st = &mut self.state.seats[seat.index()];
                let (card, copy) = st.hand.take(i as usize);
                st.to_ash(card, copy);
                self.interrupts[0] = Interrupt::PursuitDiscard {
                    seat,
                    left: left - 1,
                };
            }
            (Interrupt::MayDrawExtra { seat, source, .. }, Choice::DrawLibrary) => {
                self.interrupts.remove(0);
                let n = match source {
                    DrawSource::Card { n, .. } => n,
                    _ => 1,
                };
                for _ in 0..n {
                    self.draw_top(seat);
                }
            }
            (Interrupt::MayDrawExtra { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (
                Interrupt::RetakeFromAsh {
                    seat,
                    card,
                    copy,
                    rest,
                    draws,
                    jet,
                    except,
                    at,
                },
                choice,
            ) => {
                self.interrupts.remove(0);
                let mut draws = draws;
                match self.state.ash_index(seat, card, copy) {
                    Some(k) if choice == Choice::FindCard(card) => {
                        let s = &mut self.state.seats[seat.index()];
                        let (card, copy) = s.ash.take(k);
                        s.hand.put(card, copy);
                        self.show_referendum_choice(seat);
                    }
                    _ => draws += 1,
                }
                match self.next_retake(seat, at) {
                    Some((next, copy)) => self.interrupts.insert(
                        0,
                        Interrupt::RetakeFromAsh {
                            seat,
                            card: next,
                            copy,
                            rest: rest - 1,
                            draws,
                            jet,
                            except,
                            at,
                        },
                    ),
                    None => {
                        let count = (rest + draws) as usize;
                        let taken = self.replacements_taken(seat, except, count);
                        self.replace(seat, count, except);
                        let due = self.state.seats[seat.index()]
                            .take_held_draws(at, jet.min(rest + draws) as usize);
                        self.offer_held_draws(seat, due, count - taken);
                    }
                }
            }
            (Interrupt::ReplaceFromAsh { seat, .. }, Choice::DrawLibrary) => {
                self.interrupts.remove(0);
                self.replace(seat, 1, None);
            }
            (Interrupt::ReplaceFromAsh { seat, except }, Choice::FindCard(c)) => {
                self.interrupts.remove(0);
                let s = &mut self.state.seats[seat.index()];
                let k = s
                    .ash
                    .pairs()
                    .enumerate()
                    .filter(|&(_, p)| p.0 == c && p != except)
                    .map(|(k, _)| k)
                    .last()
                    .expect("in the ash heap");
                let (card, copy) = s.ash.take(k);
                s.hand.put(card, copy);
            }
            (Interrupt::ReplaceFromResearch { seat, except }, choice) => {
                self.interrupts.remove(0);
                let s = &mut self.state.seats[seat.index()];
                match choice {
                    Choice::FindCard(c) => {
                        let k = s
                            .research
                            .iter()
                            .position(|&r| r == c)
                            .expect("in research");
                        let (card, copy) = s.research.take(k);
                        s.hand.put(card, copy);
                        self.show_referendum_choice(seat);
                    }
                    _ => self.replace(seat, 1, except),
                }
            }
            (Interrupt::BurnLocationsForPool { seat, burned }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.gain_pool(seat, burned as i32);
            }
            (Interrupt::BurnLocationsForPool { .. }, Choice::ChooseCard(c)) => {
                if let Some(Interrupt::BurnLocationsForPool { burned, .. }) =
                    self.interrupts.first_mut()
                {
                    *burned += 1;
                }
                self.burn_card_in_play(CardId(c));
            }
            (Interrupt::PlacedClan { card, .. }, Choice::ChooseClan(k)) => {
                self.interrupts.remove(0);
                self.state.cards[card.index()].chosen_clan = crate::state::SparseOption(Some(k));
            }
            (Interrupt::PlacedDiscipline { card, .. }, Choice::ChooseDiscipline(d)) => {
                self.interrupts.remove(0);
                self.state.cards[card.index()].discipline = Some(d);
            }
            (Interrupt::PurgePick { host, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.purge_picks.push(m);
                self.finish_purge_if_done(host);
            }
            (Interrupt::MoveBloodUpTo { from, to, .. }, Choice::SetX(n)) => {
                self.interrupts.remove(0);
                let n = n.min(self.state.minions[from.index()].blood);
                self.state.lose_blood(from, n);
                let b = self.state.minions[to.index()].blood + n;
                self.state.set_blood(to, b);
            }
            (
                Interrupt::BloodToCardUpTo {
                    from,
                    card,
                    burn_at,
                    ..
                },
                Choice::SetX(n),
            ) => {
                self.interrupts.remove(0);
                let k = n.min(self.state.minions[from.index()].blood);
                self.state.lose_blood(from, k);
                self.state.cards[card.index()].counters += k;
                if burn_at.is_some_and(|b| self.state.cards[card.index()].counters >= b) {
                    self.burn_card_in_play(card);
                }
            }
            (Interrupt::BurnBloodUpTo { from, .. }, Choice::SetX(n)) => {
                self.interrupts.remove(0);
                let n = n.min(self.state.minions[from.index()].blood);
                self.state.lose_blood(from, n);
            }
            (Interrupt::StealUpTo { from, to, .. }, Choice::SetX(n)) => {
                self.interrupts.remove(0);
                let n = n.min(self.state.minions[from.index()].blood);
                self.state.lose_blood(from, n);
                self.state.gain_blood(to, n);
                self.state.note_taken(to, from, n);
            }
            (Interrupt::RetainerDamageOrder { minion, n, .. }, c) => {
                self.interrupts.remove(0);
                let present = |e: &Self| e.state.minions[minion.index()].present();
                if c == Choice::ChooseMinion(minion) {
                    self.damage_out_of_combat(minion, n, false);
                    self.damage_retainers_on(&[minion], n);
                } else {
                    self.damage_retainers_on(&[minion], n);
                    if present(self) {
                        self.damage_out_of_combat(minion, n, false);
                    }
                }
            }
            (
                Interrupt::MayCancelCombat {
                    minion,
                    blood,
                    unlock,
                    burn,
                    ..
                },
                c,
            ) => {
                self.interrupts.remove(0);
                if c == Choice::CancelCombat {
                    if let Some(id) = burn {
                        self.burn_card_in_play(id);
                    }
                    self.state.lose_blood(minion, blood);
                    if unlock {
                        self.state.minions[minion.index()].locked = false;
                    }
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
                    if zone == Zone::Hand {
                        self.equip_from_hand(s, minion, play);
                        self.replace(s, 1, None);
                    } else {
                        self.put_equipment(s, minion, play);
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
            (Interrupt::XConvictions { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::XConvictions { card: x, .. }, Choice::BurnInPlay { card }) => {
                self.burn_card_in_play(CardId(card));
                let c = &mut self.state.cards[x.index()];
                c.fixed_x = c.fixed_x.saturating_add(1);
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
            (
                Interrupt::AllySearch {
                    minion, recruit, ..
                },
                c,
            ) => {
                self.interrupts.remove(0);
                if let Choice::TakeFrom { zone, card } = c {
                    let versions = self.recruit_versions(s, minion, card).filter(|_| recruit);
                    let copy = self.take_from_zone(s, zone, card);
                    let ctx = crate::cards::PlayCtx {
                        copy: crate::copies::Untracked(copy),
                        ..crate::cards::PlayCtx::new(
                            s,
                            card,
                            0,
                            Some(minion),
                            crate::cards::Target::None,
                            0,
                        )
                    };
                    let then = crate::plays::CostThen::AllyFound {
                        recruit: versions.is_some(),
                        from_hand: zone == Zone::Hand,
                    };
                    self.found_version(ctx, versions.unwrap_or(1), then, None);
                } else {
                    self.state.seats[s.index()].library.shuffle(&mut self.rng);
                }
            }
            (
                Interrupt::RetainerSearch {
                    minion,
                    filter,
                    extra_life,
                    off,
                    hand,
                    ..
                },
                c,
            ) => {
                self.interrupts.remove(0);
                if let Choice::TakeFrom { zone, card } = c {
                    let Some(&(_, _, versions)) = self
                        .searched_retainers(s, minion, filter, off, hand)
                        .iter()
                        .find(|&&(z, k, _)| z == zone && k == card)
                    else {
                        unreachable!("a retainer offered")
                    };
                    let copy = self.take_from_zone(s, zone, card);
                    if zone == Zone::Hand {
                        self.replace(s, 1, None);
                    }
                    let ctx = crate::cards::PlayCtx {
                        copy: crate::copies::Untracked(copy),
                        ..crate::cards::PlayCtx::new(
                            s,
                            card,
                            0,
                            Some(minion),
                            crate::cards::Target::None,
                            0,
                        )
                    };
                    let then = crate::plays::CostThen::RetainerFound { extra_life, off };
                    self.found_version(ctx, versions, then, None);
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
                        // Put in play: played, though not recruited, employed
                        // or equipped: inferior, able to act this turn (LSJ
                        // 20100204).
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
                            self.note_once_played(card);
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
            (
                Interrupt::DisciplineSearch {
                    seat,
                    minion,
                    named,
                    left,
                    entering,
                    hand_or_burn,
                    no_hand,
                    library_only,
                },
                c,
            ) => {
                self.interrupts.remove(0);
                // Bima: a pass burns it.
                if hand_or_burn {
                    match c {
                        Choice::TakeFrom { zone, card } => {
                            let copy = self.take_from_zone(s, zone, card);
                            self.put_on_minion(s, card, copy, minion);
                            self.replace(s, 1, None);
                        }
                        _ => {
                            if self.state.minions[minion.index()].present() {
                                self.burn_minion(minion);
                            }
                        }
                    }
                    return;
                }
                if let Choice::TakeFrom { zone, card } = c {
                    let copy = self.take_from_zone(s, zone, card);
                    self.put_on_minion(s, card, copy, minion);
                    if left > 1 {
                        self.interrupts.insert(
                            0,
                            Interrupt::DisciplineSearch {
                                seat,
                                minion,
                                named,
                                left: left - 1,
                                entering,
                                hand_or_burn,
                                no_hand,
                                library_only,
                            },
                        );
                    }
                    // Out of the hand: drawn back up at once (advanced rules
                    // 6.8).
                    if zone == Zone::Hand {
                        self.replace(s, 1, None);
                    }
                    if left > 1 {
                        return;
                    }
                }
                self.state.seats[s.index()].library.shuffle(&mut self.rng);
                if entering && self.state.minions[minion.index()].present() {
                    self.stage_contests(minion);
                }
            }
            (Interrupt::Replacement { .. }, Choice::ResolveRider { card }) => {
                self.interrupts.remove(0);
                self.apply_replacement_rider(CardId(card));
                self.drop_replacement_draws(s);
            }
            (Interrupt::Replacement { .. }, Choice::DrawStored { card, stored }) => {
                self.interrupts.remove(0);
                self.take_stored(s, card, stored);
                self.show_referendum_choice(s);
            }
            (Interrupt::DrawUp { .. }, Choice::DrawLibrary) => {
                self.draw_top(s);
                self.show_referendum_choice(s);
            }
            (Interrupt::DrawUp { .. }, Choice::DrawStored { card, stored }) => {
                self.take_stored(s, card, stored);
                self.show_referendum_choice(s);
            }
            (Interrupt::DrawRise { left, may_top, .. }, c) => {
                match c {
                    Choice::DrawLibrary => self.draw_top(s),
                    Choice::DrawStored { card, stored } => self.take_stored(s, card, stored),
                    other => unreachable!("illegal draw choice {other:?}"),
                }
                self.interrupts[0] = Interrupt::DrawRise {
                    seat: s,
                    left: left - 1,
                    may_top,
                };
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
            (Interrupt::Bottom { left, top, .. }, Choice::Bottom(i)) => {
                let seat = &mut self.state.seats[s.index()];
                let (card, copy) = seat.hand.take(i as usize);
                // Under the cards moved before it: the last one chosen is
                // the bottom card (rules-qa 9.5); on top, the top card.
                if top {
                    seat.library.put(card, copy);
                } else {
                    seat.library.put_at(0, card, copy);
                }
                self.interrupts[0] = Interrupt::Bottom {
                    seat: s,
                    left: left - 1,
                    top,
                };
            }
            (
                Interrupt::StoreSearch {
                    card,
                    left,
                    filter,
                    ash,
                    ..
                },
                Choice::FindCard(found),
            ) => {
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
                    filter,
                    ash,
                };
            }
            (
                Interrupt::StoreSearch {
                    card,
                    left,
                    filter,
                    ash,
                    ..
                },
                Choice::TakeFrom {
                    zone: crate::decision::Zone::AshHeap,
                    card: found,
                },
            ) => {
                let heap = &mut self.state.seats[s.index()].ash;
                let k = heap
                    .iter()
                    .rposition(|&c| c == found)
                    .expect("found card in the ash heap");
                let (_, copy) = heap.take(k);
                self.state.cards[card.index()].stored.put(found, copy);
                self.interrupts[0] = Interrupt::StoreSearch {
                    seat: s,
                    card,
                    left: left - 1,
                    filter,
                    ash,
                };
            }
            (
                Interrupt::StoreSearch {
                    card, filter, ash, ..
                },
                Choice::Pass,
            ) => {
                self.interrupts[0] = Interrupt::StoreSearch {
                    seat: s,
                    card,
                    left: 0,
                    filter,
                    ash,
                };
            }
            (Interrupt::EntryRemoveForLife { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::EntryRemoveForLife { seat, ally }, c) => {
                self.interrupts.remove(0);
                let s = &mut self.state.seats[seat.index()];
                match c {
                    Choice::TakeFrom { zone, card } => {
                        let pile = match zone {
                            crate::decision::Zone::Hand => &mut s.hand,
                            _ => &mut s.ash,
                        };
                        let k = pile.iter().position(|&x| x == card).expect("the card");
                        let (_, copy) = pile.take(k);
                        s.removed.put(card, copy);
                    }
                    Choice::AshVampire(card) => {
                        let k = s
                            .ash_crypt
                            .iter()
                            .position(|&x| x == card)
                            .expect("in the ash heap");
                        let (_, copy) = s.ash_crypt.take(k);
                        s.removed_crypt.put(card, copy);
                    }
                    other => unreachable!("illegal removal {other:?}"),
                }
                // "Cannot gain life" (Pressing Flesh) read as it gains.
                self.state.gain_blood(ally, 1);
                self.interrupts
                    .insert(0, Interrupt::EntryRemoveForLife { seat, ally });
            }
            (
                Interrupt::EntryRemoveOrBurn {
                    seat: who,
                    ally,
                    left,
                    taken,
                    any,
                },
                c,
            ) => {
                self.interrupts.remove(0);
                if left > 1 && c != Choice::Pass {
                    self.interrupts.insert(
                        0,
                        Interrupt::EntryRemoveOrBurn {
                            seat: who,
                            ally,
                            left: left - 1,
                            taken: taken + 1,
                            any,
                        },
                    );
                }
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
            (
                Interrupt::DiscardFromHandOf {
                    seat,
                    of: None,
                    up_to,
                },
                Choice::ChooseSeat(of),
            ) => {
                self.interrupts[0] = Interrupt::DiscardFromHandOf {
                    seat,
                    of: Some(of),
                    up_to,
                };
            }
            (
                Interrupt::DiscardFromHandOf {
                    of: Some(of),
                    up_to: Some((_, done, _)),
                    ..
                },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
                if done > 0 {
                    self.replace(of, done as usize, None);
                }
            }
            (
                Interrupt::DiscardFromHandOf {
                    seat,
                    of: Some(of),
                    up_to,
                },
                Choice::FindCard(card),
            ) => {
                let st = &mut self.state.seats[of.index()];
                let i = st.hand.iter().position(|&c| c == card).expect("in hand");
                let (_, copy) = st.hand.take(i);
                st.to_ash(card, copy);
                match up_to {
                    Some((left, done, kind)) => {
                        self.interrupts[0] = Interrupt::DiscardFromHandOf {
                            seat,
                            of: Some(of),
                            up_to: Some((left - 1, done + 1, kind)),
                        };
                    }
                    None => {
                        self.interrupts.remove(0);
                        self.replace(of, 1, None);
                    }
                }
            }
            (Interrupt::AvoidDamage { minion, damage, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.damage_out_of_combat(minion, damage, true);
            }
            (
                Interrupt::MayTakeAggravatedAsNormal {
                    minion, n, cause, ..
                },
                c,
            ) => {
                self.interrupts.remove(0);
                let normal = match (c, self.aggravated_as_normal_out_of_combat(minion)) {
                    (Choice::ResolvePrinted { .. }, Some((k, blood))) => {
                        self.state.lose_blood(minion, blood);
                        k
                    }
                    _ => 0,
                };
                let deal = |e: &mut Self| e.damage_out_of_combat_split(minion, n, normal);
                match cause {
                    Some(cause) => self.caused(cause, deal),
                    None => deal(self),
                }
            }
            (Interrupt::AvoidDamage { minion, blood, .. }, Choice::BurnBloodToAvoid) => {
                self.interrupts.remove(0);
                self.state.lose_blood(minion, blood);
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
                    optional,
                    hold,
                },
                Choice::Discard(i),
            ) => {
                let st = &mut self.state.seats[seat.index()];
                let (card, copy) = st.hand.take(i as usize);
                st.to_ash(card, copy);
                self.interrupts[0] = Interrupt::DiscardChosen {
                    seat,
                    left: left - 1,
                    done: done + 1,
                    of,
                    optional,
                    hold,
                };
            }
            (
                Interrupt::DiscardChosen {
                    seat, done, hold, ..
                },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
                self.replace_or_hold(seat, done as usize, hold);
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
            (
                Interrupt::MinionOf {
                    seat,
                    purpose: MinionPurpose::EquipFoundOn { card, copy },
                    ..
                },
                Choice::ChooseMinion(m),
            ) => {
                self.interrupts.remove(0);
                let ctx = crate::actions::found_play(seat, m, card, copy);
                self.equip_found_with_terms(ctx);
            }
            (
                Interrupt::MinionOf {
                    purpose: MinionPurpose::CancelManeuver(_),
                    ..
                },
                Choice::ChooseMinion(_),
            ) => {
                self.interrupts.remove(0);
                if let Some(c) = self.combat.as_mut() {
                    c.cancel_maneuver();
                }
            }
            (
                Interrupt::MinionOf {
                    seat,
                    purpose: MinionPurpose::BecomeCombatant { ending },
                    ..
                },
                Choice::ChooseMinion(m),
            ) => {
                self.interrupts.remove(0);
                if ending {
                    self.free_fight_swapped(seat, m);
                } else {
                    self.become_combatant(seat, m);
                }
            }
            (Interrupt::MinionOf { seat, purpose, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                let v = &mut self.state.minions[m.index()];
                match purpose {
                    MinionPurpose::Lock
                    | MinionPurpose::LockReady
                    | MinionPurpose::LockVampire
                    | MinionPurpose::LockOtherAnarch(_)
                    | MinionPurpose::LockYoungerThan(_)
                    | MinionPurpose::LockOldest(..) => v.locked = true,
                    MinionPurpose::HostCard(c) => self.state.set_host(c, m),
                    // A Ⓓ action burning the ally, for trophies (LSJ
                    // 20080608).
                    MinionPurpose::BurnAllyUpTo(_) => {
                        if let Some(a) = self.state.acting {
                            self.red_list_burned(a, m);
                        }
                        self.burn_minion(m)
                    }
                    MinionPurpose::MoveOntoAny(c, _) | MinionPurpose::Rearrange(c) => {
                        self.state.set_host(c, m)
                    }
                    MinionPurpose::MoveToYoungerLocked(c) => {
                        let from = self.state.cards[c.index()].host;
                        self.state.set_host(c, m);
                        if let Some(h) = from {
                            self.unlock_if_left_paid(h);
                        }
                    }
                    MinionPurpose::HealHost(h) => {
                        let heal = Cause::Effect(Source::Minion(m));
                        self.caused(heal, |e| {
                            e.state.lose_blood(m, 1);
                            let b = e.state.minions[h.index()].blood + 1;
                            e.state.set_blood(h, b);
                        });
                    }
                    MinionPurpose::BlindSpot(first) => self.state.blind_spots.push((first, m)),
                    MinionPurpose::Damage(n)
                    | MinionPurpose::DamageImbued { n, .. }
                    | MinionPurpose::DamageOtherVampire { n, .. }
                    | MinionPurpose::DamageOtherMinion { n, .. } => {
                        self.damage_out_of_combat(m, n, false)
                    }
                    MinionPurpose::WeaponDamage(n) => {
                        if !self.state.weapon_damage_immune(m) {
                            self.damage_out_of_combat(m, n, false)
                        }
                    }
                    // Resolved above.
                    MinionPurpose::EquipFoundOn { .. } => unreachable!("the found equipment"),
                    MinionPurpose::BloodTo { to, n, .. } => {
                        let k = n.min(v.blood);
                        self.state.lose_blood(m, k);
                        self.state.gain_blood(to, k);
                    }
                    MinionPurpose::StealFromTorpid {
                        actor,
                        steal,
                        younger,
                        clan,
                        ..
                    } => {
                        let stolen = steal.min(v.blood);
                        if stolen > 0 {
                            self.state.lose_blood(m, stolen);
                            self.state.gain_blood(actor, stolen);
                        }
                        let of_clan = self.state.minions[actor.index()].clan() == Some(clan);
                        if let Some(a) = self.action.as_mut() {
                            a.siphoned.push(m);
                            if stolen > 0 && of_clan {
                                a.ash_to_hand_on_name = true;
                                for _ in 0..stolen {
                                    self.interrupts.push(Interrupt::NameOwnAshCard {
                                        seat,
                                        of: cards::CardFilter::Any,
                                        upto: false,
                                    });
                                }
                            }
                        }
                        self.interrupts.push(Interrupt::MinionOf {
                            seat,
                            of: seat,
                            purpose: MinionPurpose::StealFromTorpid {
                                actor,
                                more: true,
                                steal,
                                younger,
                                clan,
                            },
                        });
                    }
                    // Becoming of Ennoia: a vampire it does not affect takes
                    // none.
                    MinionPurpose::DamageVampire(n) => {
                        if !self.state.gehenna_immune(m) {
                            self.damage_out_of_combat(m, n, false)
                        }
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
                    MinionPurpose::UnlockAnarch => self.unlock_minion(m),
                    MinionPurpose::GainBlood { n, .. } => self.state.gain_blood(m, n),
                    MinionPurpose::BloodFromTo { from, n, .. } => {
                        let k = n.min(self.state.minions[from.index()].blood);
                        self.state.lose_blood(from, k);
                        self.state.gain_blood(m, k);
                    }
                    // A wrong guess: the pool paid, nothing else.
                    MinionPurpose::GuessChosen { card, pool } => {
                        if !self.pay_pool_for_ability(seat, pool as i32)
                            && self.state.cards[card.index()].present()
                            && self.state.cards[card.index()].chosen.first() == Some(&m)
                        {
                            self.burn_card_in_play(card);
                        }
                    }
                    MinionPurpose::BecomeCombatant { .. } | MinionPurpose::CancelManeuver(_) => {
                        unreachable!("answered above")
                    }
                    MinionPurpose::JointBlocker(x, y) => {
                        self.joint_blocked(m, if m == x { y } else { x })
                    }
                    MinionPurpose::HostCardOn(c, _) => self.move_card_onto(c, m),
                    MinionPurpose::SecessionCity(c) => {
                        let Some(crate::state::TitleKey::City(city)) =
                            crate::state::title_key(self.state.claimed_title(m))
                        else {
                            unreachable!("a city's holder")
                        };
                        let t = crate::crypt::Title::Baron(city);
                        self.state.cards[c.index()].title = crate::state::SparseOption(Some(t));
                        if let Some(h) = self.state.cards[c.index()].host {
                            self.gain_title(h, t, Some(c));
                        }
                    }
                    MinionPurpose::HolderAtSuccess(..) => {
                        if let Some(a) = self.action.as_mut() {
                            a.holder = Some(m);
                        }
                    }
                    MinionPurpose::DamageVampireBy { n, by } => {
                        if !self.state.spared_damage_by(m, by) {
                            self.damage_out_of_combat(m, n, false)
                        }
                    }
                    MinionPurpose::HuntBloodTo { n, .. } => self.state.gain_blood(m, n),
                    MinionPurpose::BurnBloodOfClan(_) => self.state.lose_blood(m, 1),
                    MinionPurpose::Corrupt | MinionPurpose::CorruptAmong(_) => {
                        v.add_corruption(seat, 1)
                    }
                    MinionPurpose::BlockMark(by) => {
                        self.state
                            .unlock_marks
                            .push((crate::state::UnlockMark::Blocks, m, by))
                    }
                    // Capacity 6 or more: that Methuselah ignores it for the
                    // rest of the game.
                    // Karen chosen: not burned (LSJ 20081120-2).
                    MinionPurpose::BurnSelfContested(_) => self.burn_minion(m),
                    MinionPurpose::BurnOfCrowdedClan(by) => {
                        if self.state.capacity_by(m, by).is_some_and(|c| c >= 6) {
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
                self.state.lose_blood(from, n);
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
                self.state.lose_blood(from, 1);
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
            (Interrupt::PreyReplacement { seat, except }, Choice::DrawLibrary) => {
                self.interrupts.remove(0);
                self.replace(seat, 1, except);
            }
            // The prey's card, theirs wherever it goes (RTR 19970425, LSJ
            // 20060921); an Infernal Pursuit's extra draws still from the
            // own library (LSJ 20011202).
            (Interrupt::PreyReplacement { seat, .. }, Choice::ChooseSeat(prey)) => {
                self.interrupts.remove(0);
                if let Some((card, copy)) = self.state.seats[prey.index()].library.take_top() {
                    let st = &mut self.state;
                    st.seats[seat.index()].hand.put(card, copy);
                    st.seats[seat.index()].face_up.0.push(copy);
                    st.foreign.push((crate::copies::Untracked(copy), prey));
                }
                self.draw_up(seat);
                let k = self.state.seats[seat.index()]
                    .timed
                    .total(crate::state::TimedStat::ExtraDraws);
                if k > 0 && self.combat.is_some() {
                    for _ in 0..k {
                        let s = &mut self.state.seats[seat.index()];
                        match s.library.take_top() {
                            Some((c, copy)) => s.hand.put(c, copy),
                            None => break,
                        }
                    }
                }
            }
            (Interrupt::MayGainPool { seat, n, minion }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                let own = Cause::Effect(Source::Minion(minion));
                self.caused(own, |e| e.state.gain_pool(seat, n as i32));
            }
            // The Edge burned first, then the pool: under The Rising it needs
            // a victory point (LSJ 20100207).
            (Interrupt::MayBurnEdgeForPool { seat, n, minion }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                self.lose_edge(seat);
                let own = Cause::Effect(Source::Minion(minion));
                self.caused(own, |e| e.state.gain_pool(seat, n as i32));
            }
            // An effect, not a cost (rules-digest D20).
            (
                Interrupt::MayBurnBloodToBurnPool {
                    minion,
                    of,
                    blood,
                    pool,
                    ..
                },
                Choice::ResolvePrinted { .. },
            ) => {
                self.interrupts.remove(0);
                self.state.burn_blood(minion, blood);
                let mut loss = vec![0; self.state.seats.len()];
                loss[of.index()] = pool as i32;
                self.burn_pool_batch(&loss);
            }
            (Interrupt::MayGainBlood { minion, n, .. }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                self.state.gain_blood(minion, n);
            }
            (Interrupt::MayHuntBloodToPool { seat, minion }, choice) => {
                self.interrupts.remove(0);
                match choice {
                    Choice::ResolvePrinted { .. } => {
                        let hunt = Cause::Effect(Source::Minion(minion));
                        self.caused(hunt, |e| e.state.gain_pool(seat, 1));
                    }
                    _ => self.state.gain_blood(minion, 1),
                }
            }
            (Interrupt::MayMoveEdgePool { seat, minion }, choice) => {
                self.interrupts.remove(0);
                let edge = Cause::Effect(Source::Rule(crate::notes::Rule::Edge));
                match choice {
                    Choice::ResolvePrinted { .. } => {
                        self.caused(edge, |e| e.state.gain_blood(minion, 1));
                    }
                    _ => self.caused(edge, |e| e.state.gain_pool(seat, 1)),
                }
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
            (Interrupt::BurnOrder { .. }, Choice::ChooseSeat(next)) => {
                self.burn_order
                    .as_mut()
                    .expect("burn order")
                    .picked
                    .push(next);
            }
            (Interrupt::GiveCardTo { card, .. }, Choice::ChooseSeat(to)) => {
                self.interrupts.remove(0);
                self.give_card_control(card, to);
            }
            (
                Interrupt::AshToLibraryTop {
                    seat,
                    left,
                    filter,
                    bottom,
                    up_to,
                    discarded,
                },
                Choice::FindCard(card),
            ) => {
                let st = &mut self.state.seats[seat.index()];
                let (card, copy) = if discarded {
                    let k = st
                        .ash
                        .pairs()
                        .position(|(c, copy)| c == card && st.top_discards.0.contains(&copy))
                        .expect("a card discarded");
                    let (card, copy) = st.ash.take(k);
                    st.top_discards.0.retain(|&x| x != copy);
                    (card, copy)
                } else {
                    st.ash.take_card(card).expect("in the ash heap")
                };
                if bottom {
                    st.library.put_at(0, card, copy);
                } else {
                    st.library.put(card, copy);
                }
                self.interrupts[0] = Interrupt::AshToLibraryTop {
                    seat,
                    left: left - 1,
                    filter,
                    bottom,
                    up_to,
                    discarded,
                };
            }
            (Interrupt::AshToLibraryTop { seat, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.seats[seat.index()].top_discards.0.clear();
            }
            (Interrupt::CutLibrary { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::CutLibrary { seat }, Choice::SetX(n)) => {
                self.interrupts.remove(0);
                let library = &mut self.state.seats[seat.index()].library;
                let top = library.take_from(library.len() - n as usize);
                let rest = library.take_all();
                library.put_all(top.pairs().collect::<Vec<_>>());
                library.put_all(rest);
            }
            (Interrupt::DualFormKept { root, .. }, Choice::ChooseMinion(kept)) => {
                self.interrupts.remove(0);
                for c in self.state.dual_set(root) {
                    if c != kept {
                        self.burn_minion(c);
                    }
                }
            }
            (Interrupt::MayBurnInsteadOfUnlock { minion, unlock, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                if unlock {
                    self.unlock_minion(minion);
                }
            }
            (Interrupt::MayBurnInsteadOfUnlock { .. }, Choice::BurnInPlay { card })
            | (Interrupt::MayBurnWeapon { .. }, Choice::BurnInPlay { card }) => {
                self.interrupts.remove(0);
                self.burn_card_in_play(CardId(card));
            }
            (Interrupt::MayBurnWeapon { .. } | Interrupt::MaySetNextRange { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::BlockWouldSucceed { blocker, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.blocked(blocker);
            }
            (
                Interrupt::BlockWouldSucceed { seat, .. },
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
            (Interrupt::MaySetNextRange { .. }, Choice::SetX(n)) => {
                self.interrupts.remove(0);
                if let Some(c) = self.combat.as_mut() {
                    let r = if n == 0 {
                        crate::combat::Range::Close
                    } else {
                        crate::combat::Range::Long
                    };
                    c.next_round_range.get_or_insert(r);
                }
            }
            (Interrupt::ActionX { .. }, Choice::SetX(n)) => {
                self.interrupts.remove(0);
                if let Some(a) = self.action.as_mut() {
                    a.action_x = Some(n);
                }
            }
            (Interrupt::NamedX { .. }, Choice::SetX(n)) => {
                self.interrupts.remove(0);
                if let Some(a) = self.action.as_mut() {
                    a.named_x = Some(n);
                }
            }
            (Interrupt::ReductionAtResolution { .. }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                if let Some(p) = self.action.as_mut().and_then(|a| a.card.as_mut()) {
                    p.once_off = true;
                }
            }
            (Interrupt::MayFailAction { .. }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                if let Some(a) = self.action.as_mut() {
                    a.failed = true;
                }
            }
            // The card's entry effects before the next pick (War Ghoul
            // burning the first; LSJ 20070707), the replacement after the
            // last.
            (
                Interrupt::ExtraHires {
                    seat,
                    minion,
                    left,
                    hired,
                },
                c,
            ) => {
                self.interrupts.remove(0);
                let Choice::EmployFromHand { card, mode } = c else {
                    self.replace(seat, hired as usize, None);
                    return;
                };
                let ctx = crate::cards::PlayCtx::new(
                    seat,
                    card,
                    mode,
                    Some(minion),
                    crate::cards::Target::Minion(minion),
                    0,
                );
                let then = crate::plays::CostThen::ExtraHire { left, hired };
                if !self.found_cost_choices(ctx, then, None) {
                    self.extra_hire(ctx, left, hired);
                }
            }
            (Interrupt::MayLockToFight { minion, acting, .. }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].locked = true;
                self.start_combat_after_resolution(acting, minion);
            }
            (Interrupt::MayBurnForBlock { minion, n, .. }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                self.state
                    .turn_uses
                    .push(crate::state::once_block_burn_key(minion));
                let own = Cause::Effect(Source::Minion(minion));
                self.caused(own, |e| e.state.burn_blood(minion, n));
            }
            // After every discard and redraw (RTR 20030519): those still in
            // the ash heap.
            (Interrupt::MayReturnDiscards { seat, .. }, choice) => {
                self.interrupts.remove(0);
                let cards = self.state.seats[seat.index()]
                    .phase_discards
                    .take()
                    .unwrap_or_default();
                if matches!(choice, Choice::ResolvePrinted { .. }) {
                    let s = &mut self.state.seats[seat.index()];
                    for c in cards {
                        if let Some((card, copy)) = s.ash.take_card(c) {
                            s.library.put(card, copy);
                        }
                    }
                    s.library.shuffle(&mut self.rng);
                }
                self.end_turn();
            }
            (Interrupt::MayLockCardForBlock { card, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.state.cards[card.index()].locked = true;
            }
            (Interrupt::MayLockCardForBlock { minion, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].locked = true;
            }
            (Interrupt::MayDiscardForBlock { seat, minion }, Choice::Discard(i)) => {
                self.interrupts.remove(0);
                self.state
                    .turn_uses
                    .push(crate::state::once_block_discard_key(minion));
                let s = &mut self.state.seats[seat.index()];
                let (c, copy) = s.hand.take(i as usize);
                s.to_ash(c, copy);
                self.replace(seat, 1, None);
            }
            (
                Interrupt::BoonAuction {
                    seat,
                    target,
                    card,
                    high,
                    out,
                },
                c,
            ) => {
                self.interrupts.remove(0);
                let (high, out) = match c {
                    Choice::SetX(b) => (Some((seat, b)), out),
                    _ => (high, out.with(seat)),
                };
                match self.next_boon_bidder(target, seat, high, out) {
                    Some(next) => self.interrupts.insert(
                        0,
                        Interrupt::BoonAuction {
                            seat: next,
                            target,
                            card,
                            high,
                            out,
                        },
                    ),
                    None => self.settle_boon_auction(target, card, high),
                }
            }
            (Interrupt::MayDiscardToKeep { seat, .. }, Choice::Discard(i)) => {
                self.interrupts.remove(0);
                let s = &mut self.state.seats[seat.index()];
                let (c, copy) = s.hand.take(i as usize);
                s.to_ash(c, copy);
                self.replace(seat, 1, None);
            }
            (Interrupt::MayDiscardToKeep { card, taker, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.take_card_control(card, taker);
            }
            (Interrupt::MayDiscardForBlock { minion, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].locked = true;
            }
            (Interrupt::MayBurnForBlock { minion, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].locked = true;
            }
            (Interrupt::MayBottomLooked { seat, minion, n }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                let own = Cause::Effect(Source::Minion(minion));
                self.caused(own, |e| e.state.burn_blood(minion, n));
                let library = &mut self.state.seats[seat.index()].library;
                if let Some((card, copy)) = library.take_top() {
                    library.put_at(0, card, copy);
                }
            }
            (Interrupt::MayMoveBlood { from, to, n, .. }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                let n = n.min(self.state.minions[from.index()].blood);
                self.state.lose_blood(from, n);
                self.state.gain_blood(to, n);
            }
            (
                Interrupt::MayUnlock {
                    seat,
                    minion,
                    once_each_turn,
                },
                Choice::ResolvePrinted { .. },
            ) => {
                self.interrupts.remove(0);
                if once_each_turn {
                    self.state
                        .turn_uses
                        .push(crate::state::once_unlock_key(minion));
                }
                let toll = self.state.unlock_toll_on(minion);
                if toll > 0 {
                    self.pay_pool_for_ability(seat, toll);
                    if self.state.seats[seat.index()].ousted {
                        return;
                    }
                }
                self.effect_unlock(minion);
            }
            (Interrupt::CombatStandIn { card, replaced, .. }, c) => {
                self.interrupts.remove(0);
                let by = match c {
                    Choice::ResolveRider { .. } => self.state.cards[card.index()].body,
                    _ => None,
                };
                if let Some(o) = by {
                    self.state.minions[o.index()].locked = true;
                }
                if self.in_combat(replaced) {
                    self.settle_rush_stand_in(replaced, by);
                } else if let (Some(o), Some(a)) = (by, self.action.as_mut()) {
                    a.blocker_stand_in = Some(o);
                }
            }
            (Interrupt::MayBurnBloodToPeek { seat, minion, of }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                self.state.burn_blood(minion, 1);
                self.peek(seat, of);
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
                | Interrupt::MayBottomTop { .. }
                | Interrupt::MayTorture { .. }
                | Interrupt::MayLockForBlood { .. }
                | Interrupt::MayBurnLookedMaster { .. }
                | Interrupt::MayLockToDiscardLooked { .. }
                | Interrupt::MayGainPool { .. }
                | Interrupt::MayGainPoolForCard { .. }
                | Interrupt::MayBurnEdgeForPool { .. }
                | Interrupt::MayBurnBloodToBurnPool { .. }
                | Interrupt::MayGainHandSize { .. }
                | Interrupt::MayGainBlood { .. }
                | Interrupt::MayPreventForBlood { .. }
                | Interrupt::MayBurnBloodToPeek { .. }
                | Interrupt::MayUnlock { .. }
                | Interrupt::MayMoveBlood { .. }
                | Interrupt::MayFailAction { .. }
                | Interrupt::MayLockToFight { .. }
                | Interrupt::MayBottomLooked { .. }
                | Interrupt::MayLockForDisciplineSearch { .. }
                | Interrupt::ReductionAtResolution { .. },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
            }
            (Interrupt::CoercedSide { minion, .. }, choice) => {
                self.interrupts.remove(0);
                let mut ctx = self.referendum.take().expect("referendum context");
                match choice {
                    Choice::CastVotes { favor, .. } => self.change_votes(&mut ctx, minion, favor),
                    _ => Self::abstain_minion(&mut ctx, minion),
                }
                self.referendum = Some(ctx);
            }
            (Interrupt::PlayedCardToHand { seat, minion }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                let a = self.action.as_mut().expect("the action");
                let bound = a.acting == minion && a.seat == seat;
                let copies: Vec<crate::copies::CopyId> = a
                    .played_by
                    .iter()
                    .filter(|&&(m, c, _)| m == minion && c == card)
                    .map(|&(_, _, k)| k)
                    .collect();
                let ash = &mut self.state.seats[seat.index()].ash;
                match (0..ash.len())
                    .rev()
                    .find(|&k| ash.to_cards()[k] == card && copies.contains(&ash.copy(k)))
                {
                    Some(k) => {
                        let (c, copy) = ash.take(k);
                        self.state.seats[seat.index()].hand.put(c, copy);
                    }
                    // The action card, bound for the ash heap.
                    None if bound => a.card_to_hand = true,
                    None => {}
                }
            }
            // The vampire chosen as the lock is used, then the search, which
            // may fail: the blood still moves (LSJ 20031209).
            (
                Interrupt::MayLockForDisciplineSearch { seat, card, blood },
                Choice::ChooseMinion(m),
            ) => {
                self.interrupts.remove(0);
                self.lock_card(card);
                let by = Cause::Effect(Source::Card(card));
                self.caused(by, |e| e.state.gain_blood(m, blood));
                self.interrupts.insert(
                    0,
                    Interrupt::DisciplineSearch {
                        seat,
                        minion: m,
                        named: &[],
                        left: 1,
                        entering: false,
                        hand_or_burn: false,
                        no_hand: false,
                        library_only: false,
                    },
                );
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
            (Interrupt::MayBottomTop { seat, n, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                let library = &mut self.state.seats[seat.index()].library;
                // Top first, each put under the last: their order kept.
                for _ in 0..n {
                    if let Some((card, copy)) = library.take_top() {
                        library.put_at(0, card, copy);
                    }
                }
            }
            (Interrupt::MayAddCounter { card, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.state.cards[card.index()].counters += 1;
            }
            // Its rider's effects, as the definition gives them.
            (Interrupt::CryptToReady { seat, blood, .. }, Choice::AshVampire(card)) => {
                self.interrupts.remove(0);
                let st = &mut self.state.seats[seat.index()];
                let k = st
                    .crypt
                    .iter()
                    .position(|&c| c == card)
                    .expect("in the crypt");
                let (_, copy) = st.crypt.take(k);
                st.crypt.shuffle(&mut self.rng);
                let forced = self.state.controls_copy(seat, card);
                let m = self.add_minion(seat, card, blood, false);
                let v = &mut self.state.minions[m.index()];
                v.copy = crate::copies::Untracked(copy);
                v.recruited = true;
                self.vampire_entered(seat, m, card, forced);
            }
            (Interrupt::CryptToReady { seat, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.seats[seat.index()].crypt.shuffle(&mut self.rng);
            }
            // "Burn up to one": none.
            (Interrupt::BurnFromLibraryTop { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::BurnFromLibraryTop { of, n, .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                let lib = &mut self.state.seats[of.index()].library;
                let len = lib.len();
                let k = (len.saturating_sub(n as usize)..len)
                    .rev()
                    .find(|&k| lib.to_cards()[k] == card)
                    .expect("one of the top cards");
                let (c, copy) = lib.take(k);
                self.state.seats[of.index()].to_ash(c, copy);
            }
            (Interrupt::MayGainPoolForCard { seat, card, n }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                let by = Cause::Effect(Source::Card(card));
                self.caused(by, |e| e.state.gain_pool(seat, n as i32));
            }
            (Interrupt::MayLockForBlood { card, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                let def = cards::lookup(self.state.cards[card.index()].card).expect("definition");
                let by = Cause::Effect(Source::Card(card));
                // Conniver's, Curmudgeon's.
                for r in def
                    .riders
                    .iter()
                    .filter(|r| matches!(r.hook, Hook::PreyLosesPool | Hook::ChosenMinionBlocked))
                {
                    for &e in r.effects {
                        self.caused(by, |en| en.apply_rider_effect(card, e, r.hook));
                    }
                }
            }
            (Interrupt::MayLockToDiscardLooked { card, of, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.lock_card(card);
                let by = Cause::Effect(Source::Card(card));
                self.caused(by, |e| e.burn_library_top(of, 1));
            }
            (Interrupt::MayBurnLookedMaster { card, n, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                let host = self.state.cards[card.index()].host.expect("a bearer");
                let by = Cause::Effect(Source::Card(card));
                self.caused(by, |e| e.state.burn_blood(host, n));
                self.burn_library_top(self.state.active, 1);
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
            (Interrupt::BloodOrNoStrike { minion, n, .. }, Choice::BurnBloodToAvoid) => {
                self.interrupts.remove(0);
                self.state.burn_blood(minion, n);
            }
            (Interrupt::BloodOrNoStrike { minion, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.bar_strike_this_round(minion);
            }
            (Interrupt::NameActionDiscipline { .. }, Choice::ChooseDiscipline(d)) => {
                self.interrupts.remove(0);
                if let Some(a) = self.action.as_mut() {
                    a.named_discipline = Some(d);
                }
            }
            // Moved, not played: no cost, no equip action; a hand card
            // replaced (spec §6.8.3).
            (
                Interrupt::NamedOntoOrDamage { seat, minion, .. },
                Choice::TakeFrom { zone, card },
            ) => {
                self.interrupts.remove(0);
                let copy = self.take_from_zone(seat, zone, card);
                let play = crate::cards::PlayCtx {
                    copy: crate::copies::Untracked(copy),
                    ..crate::cards::PlayCtx::new(
                        seat,
                        card,
                        0,
                        Some(minion),
                        crate::cards::Target::Minion(minion),
                        0,
                    )
                };
                self.put_equipment(seat, minion, play);
                if zone == Zone::Hand {
                    self.replace(seat, 1, None);
                }
            }
            // Environmental, out of combat (RTR 19970630).
            (Interrupt::NamedOntoOrDamage { minion, damage, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                let own = Cause::Effect(Source::Minion(minion));
                self.caused(own, |e| e.damage_out_of_combat(minion, damage, false));
            }
            (Interrupt::DamageVoterForBlood { minion, n, .. }, Choice::ChooseMinion(v)) => {
                self.interrupts.remove(0);
                let own = Source::Minion(minion);
                self.caused(Cause::Cost(own), |e| e.state.lose_blood(minion, 1));
                self.caused(Cause::Effect(own), |e| e.damage_out_of_combat(v, n, false));
            }
            (Interrupt::DamageVoterForBlood { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
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
            (Interrupt::LockOrDiscard { minion, of, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                if of.is_some() || !self.state.gehenna_immune(minion) {
                    self.state.minions[minion.index()].locked = true;
                }
            }
            (Interrupt::LockOrDiscard { seat, of, .. }, Choice::Discard(i)) => {
                self.interrupts.remove(0);
                let s = &mut self.state.seats[seat.index()];
                let (c, copy) = s.hand.take(i as usize);
                s.to_ash(c, copy);
                if of.is_none() {
                    s.held[Release::MasterPhase] += 1;
                } else {
                    self.replace(seat, 1, None);
                }
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
            (Interrupt::ContestPayment { seat, minion, left }, Choice::BurnPool) => {
                self.interrupts.remove(0);
                let contest = Cause::Cost(Source::Rule(crate::notes::Rule::Contest));
                self.caused(contest, |e| e.state.seats[seat.index()].lose_pool(1));
                if self.state.seats[seat.index()].pool <= 0 {
                    self.oust_and_pass(seat);
                } else if left > 1 {
                    self.interrupts.insert(
                        0,
                        Interrupt::ContestPayment {
                            seat,
                            minion,
                            left: left - 1,
                        },
                    );
                }
            }
            (
                Interrupt::ContestPayment {
                    seat,
                    minion: contested,
                    left,
                },
                Choice::BurnBloodFrom { minion },
            ) => {
                self.interrupts.remove(0);
                let contest = Cause::Cost(Source::Rule(crate::notes::Rule::Contest));
                self.caused(contest, |e| e.state.lose_blood(minion, 1));
                if left > 1 {
                    self.interrupts.insert(
                        0,
                        Interrupt::ContestPayment {
                            seat,
                            minion: contested,
                            left: left - 1,
                        },
                    );
                }
            }
            (Interrupt::BloodOrPool { seat, pool, .. }, Choice::BurnPool) => {
                self.interrupts.remove(0);
                let mut loss = vec![0; self.state.seats.len()];
                loss[seat.index()] = pool as i32;
                self.burn_pool_batch(&loss);
            }
            (Interrupt::BloodOrPool { blood, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                self.state.burn_blood(m, blood);
            }
            (Interrupt::PoolOrNoTransfers { seat, pool }, Choice::BurnPool) => {
                self.interrupts.remove(0);
                self.pay_pool_for_ability(seat, pool as i32);
            }
            // Other effects adding blood to its uncontrolled vampires stay
            // usable (LSJ 20100527).
            // The effect's burn, not a cost: the last pool ousts (D3).
            (Interrupt::PoolOrMinionBurned { seat, pool, .. }, Choice::BurnPool) => {
                self.interrupts.remove(0);
                let mut loss = vec![0i32; self.state.seats.len()];
                loss[seat.index()] = pool as i32;
                self.burn_pool_batch(&loss);
            }
            (Interrupt::PoolOrMinionBurned { minion, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.burn_minion(minion);
            }
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
            (Interrupt::CryptTopPick { seat, n }, Choice::AshVampire(card)) => {
                self.interrupts.remove(0);
                let st = &mut self.state.seats[seat.index()];
                let len = st.crypt.len();
                let from = len.saturating_sub(n as usize);
                let k = (from..len)
                    .rev()
                    .find(|&k| st.crypt[k] == card)
                    .expect("among the top cards");
                let (card, copy) = st.crypt.take(k);
                st.uncontrolled.push(crate::state::UncontrolledSlot {
                    card,
                    copy: crate::copies::Untracked(copy),
                    counters: 0,
                    banished: None,
                });
                // The others, top first, each under the last.
                for _ in 0..(len - from - 1) {
                    if let Some((c, copy)) = st.crypt.take_top() {
                        st.crypt.put_at(0, c, copy);
                    }
                }
            }
            (Interrupt::CryptSearch { seat, blood, .. }, Choice::AshVampire(card)) => {
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
                    counters: blood.min(crate::crypt::CRYPT[card as usize].capacity),
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
            (
                Interrupt::LibraryToHand {
                    seat,
                    haven_on,
                    top,
                    shown,
                    ..
                },
                Choice::FindCard(card),
            ) => {
                self.interrupts.remove(0);
                if let Some(minion) = haven_on.filter(|_| {
                    crate::library::LIBRARY[card as usize]
                        .labels
                        .contains(&crate::library::Label::Haven)
                }) {
                    self.interrupts
                        .insert(0, Interrupt::PutFoundOn { seat, minion, card });
                }
                if shown {
                    self.reveal(seat, card, false);
                }
                let s = &mut self.state.seats[seat.index()];
                let k = s
                    .library
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("found card in the library");
                let (_, copy) = s.library.take(k);
                s.library.shuffle(&mut self.rng);
                let s = &mut self.state.seats[seat.index()];
                if top {
                    s.library.put(card, copy);
                } else {
                    s.hand.put(card, copy);
                }
            }
            (Interrupt::ResearchSearch { seat, left }, Choice::FindCard(card)) => {
                let s = &mut self.state.seats[seat.index()];
                let k = s
                    .library
                    .iter()
                    .rposition(|&c| c == card)
                    .expect("found card in the library");
                let (_, copy) = s.library.take(k);
                s.research.put(card, copy);
                self.interrupts[0] = Interrupt::ResearchSearch {
                    seat,
                    left: left - 1,
                };
            }
            (Interrupt::ResearchSearch { seat, .. }, Choice::Pass) => {
                self.interrupts[0] = Interrupt::ResearchSearch { seat, left: 0 };
            }
            (Interrupt::LibraryToHand { may: Some(_), .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (
                Interrupt::LibraryToHand {
                    seat,
                    filter,
                    haven_on,
                    top,
                    shown,
                    may: Some(_),
                },
                Choice::ResolvePrinted { .. },
            ) => {
                self.interrupts[0] = Interrupt::LibraryToHand {
                    seat,
                    filter,
                    haven_on,
                    top,
                    shown,
                    may: None,
                };
            }
            (Interrupt::LibraryToHand { seat, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.seats[seat.index()]
                    .library
                    .shuffle(&mut self.rng);
            }
            (Interrupt::FavorAuction { seat, offer, by }, c) => {
                self.interrupts.remove(0);
                if let Choice::SetX(_) = c {
                    self.state.gain_pool(seat, offer as i32);
                    let mut ctx = self.referendum.take().expect("the referendum");
                    let voters: Vec<crate::ids::MinionId> = self
                        .state
                        .minions_of(seat)
                        .filter(|&m| {
                            let v = &self.state.minions[m.index()];
                            v.ready() && v.is_vampire()
                        })
                        .collect();
                    for m in voters {
                        ctx.sides_set.retain(|&(x, _)| x != m);
                        self.force_cast(&mut ctx, m, true);
                    }
                    self.referendum = Some(ctx);
                } else {
                    let v = &self.state.minions[by.index()];
                    if v.present() && v.blood > 0 {
                        self.state.burn_blood(by, 1);
                        self.favor_auction_from(seat, offer + 1, by, false);
                    }
                }
            }
            (Interrupt::CryptSearch { seat, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.seats[seat.index()].crypt.shuffle(&mut self.rng);
            }
            (
                Interrupt::BloodForVotes {
                    seat,
                    min,
                    more,
                    votes,
                    extra,
                    passes,
                },
                c,
            ) => {
                self.interrupts.remove(0);
                let passes = match c {
                    Choice::ChooseMinion(m) => {
                        self.state.burn_blood(m, 1);
                        let n = if self.state.capacity_of(m).is_some_and(|c| c > more) {
                            votes + extra
                        } else {
                            votes
                        };
                        if let Some(r) = self.referendum.as_mut() {
                            match r.records.iter_mut().find(|v| {
                                v.source == crate::referendum::Source::Vampire(m) && !v.canceled
                            }) {
                                Some(v) => v.n += n,
                                None => r.bonus.push((m, n)),
                            }
                        }
                        0
                    }
                    _ => passes + 1,
                };
                if (passes as usize) < self.state.living() {
                    self.interrupts.insert(
                        0,
                        Interrupt::BloodForVotes {
                            seat: self.state.prey(seat),
                            min,
                            more,
                            votes,
                            extra,
                            passes,
                        },
                    );
                }
            }
            (
                Interrupt::StealFromSlot {
                    of,
                    n,
                    slot: None,
                    burn: true,
                    ..
                },
                Choice::Slot(i),
            ) => {
                self.interrupts.remove(0);
                let card = self.state.seats[of.index()].uncontrolled[i as usize].card;
                self.action.as_mut().expect("action").slot_steal = Some((i, card, n));
            }
            (
                Interrupt::StealFromSlot {
                    seat,
                    of,
                    to,
                    n,
                    slot: None,
                    burn,
                    affordable,
                },
                Choice::Slot(i),
            ) => {
                self.interrupts[0] = Interrupt::StealFromSlot {
                    seat,
                    of,
                    to,
                    n,
                    slot: Some(i),
                    burn,
                    affordable,
                };
            }
            (
                Interrupt::StealFromSlot {
                    of, slot: Some(i), ..
                },
                Choice::SetX(x),
            ) => {
                self.interrupts.remove(0);
                let card = self.state.seats[of.index()].uncontrolled[i as usize].card;
                self.action.as_mut().expect("action").slot_steal = Some((i, card, x));
            }
            (Interrupt::PreySlotToPool { of, .. }, Choice::Slot(i)) => {
                self.interrupts.remove(0);
                self.state.seats[of.index()].uncontrolled[i as usize].counters -= 1;
                self.state.gain_pool(of, 1);
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
                    e.state.lose_blood(from, 1);
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
                    among,
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
                    among,
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
                    among,
                    from: Some(f),
                    to: Some(t),
                    equipment,
                    mut paired,
                    ..
                },
                Choice::SetX(n),
            ) => {
                // An ally moving its last life is burned (rules-qa 73.5).
                let lifeless = self.state.burn_blood_or_life(f, n);
                let b = self.state.minions[t.index()].blood + n;
                self.state.set_blood(t, b);
                if let Some(slot) = paired.iter_mut().find(|p| p.is_none()) {
                    *slot = Some((f, t));
                }
                self.interrupts[0] = Interrupt::CircleBlood {
                    seat,
                    among,
                    from: None,
                    to: None,
                    equipment,
                    moving: None,
                    paired,
                };
                if lifeless {
                    self.interrupts.remove(0);
                    self.lifeless(f);
                }
            }
            (
                Interrupt::CircleBlood {
                    seat,
                    among,
                    equipment,
                    paired,
                    ..
                },
                Choice::ChooseCard(c),
            ) => {
                self.interrupts[0] = Interrupt::CircleBlood {
                    seat,
                    among,
                    from: None,
                    to: None,
                    equipment,
                    moving: Some(CardId(c)),
                    paired,
                };
            }
            (Interrupt::BaseSearch { seat, minion, .. }, choice) => {
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
                        let (slot, on) = self.state.take_slot(seat, i as usize);
                        self.slot_cards_onto(on, minion);
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
            (Interrupt::AwardTrophy { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            // The others in play may follow (rulebook "Red List"): the
            // award stays, after Trophy: Chosen's choice.
            (Interrupt::AwardTrophy { seat, minion, .. }, Choice::ResolveRider { card }) => {
                let id = CardId(card);
                self.state.set_host(id, minion);
                self.push_mode_layers(id);
                self.trophy_moved(seat, minion, id);
            }
            (
                Interrupt::ExchangeLooked {
                    of, card, minion, ..
                },
                Choice::FindCard(other),
            ) => {
                self.interrupts.remove(0);
                self.state.burn_blood(minion, 1);
                let st = &mut self.state.seats[of.index()];
                let i = st.hand.iter().position(|&c| c == card).expect("in hand");
                let (c, copy) = st.hand.take(i);
                let j = st
                    .ash
                    .iter()
                    .rposition(|&a| a == other)
                    .expect("in the ash heap");
                let (o, ocopy) = st.ash.take(j);
                st.to_ash(c, copy);
                st.hand.put(o, ocopy);
            }
            (Interrupt::ExchangeLooked { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (Interrupt::LookPreySlot { seat, of }, Choice::Slot(i)) => {
                self.interrupts.remove(0);
                let slot = &self.state.seats[of.index()].uncontrolled[i as usize];
                // A banished created vampire: its own library card.
                let own = slot.banished.as_ref().and_then(|b| {
                    b.cards
                        .iter()
                        .find(|c| self.state.cards[c.index()].body == Some(b.minion))
                        .map(|c| self.state.cards[c.index()].card)
                });
                let (card, crypt) = match own.filter(|_| slot.card == crate::state::CREATED) {
                    Some(c) => (c, false),
                    None => (slot.card, true),
                };
                self.push_note(crate::notes::Note::Look(crate::game::Look {
                    viewer: seat,
                    seat: of,
                    cards: vec![card],
                    zone: crate::game::LookedZone::Uncontrolled { crypt },
                }));
            }
            (Interrupt::HideOneCard { seat, viewer }, Choice::Discard(i)) => {
                self.interrupts.remove(0);
                let hand = &self.state.seats[seat.index()].hand;
                let cards = (0..hand.len())
                    .filter(|&k| k != i as usize)
                    .map(|k| hand[k])
                    .collect();
                self.push_note(crate::notes::Note::Look(crate::game::Look {
                    viewer,
                    seat,
                    cards,
                    zone: crate::game::LookedZone::Hand,
                }));
            }
            (Interrupt::HandCardAway { seat }, Choice::Discard(i)) => {
                self.interrupts.remove(0);
                let st = &mut self.state.seats[seat.index()];
                let (card, copy) = st.hand.take(i as usize);
                st.to_ash(card, copy);
            }
            (Interrupt::HandCardAway { seat }, Choice::TakeFrom { card, .. }) => {
                self.interrupts.remove(0);
                let st = &mut self.state.seats[seat.index()];
                let i = st.hand.iter().position(|&c| c == card).expect("in hand");
                let (card, copy) = st.hand.take(i);
                st.library.put(card, copy);
            }
            (
                Interrupt::SearchOnto {
                    seat,
                    minion,
                    held,
                    from_hand,
                    left,
                    paired,
                    owed,
                },
                Choice::TakeFrom { zone, card },
            ) => {
                use crate::decision::Zone;
                let kind = held
                    .iter()
                    .copied()
                    .find(|&h| crate::state::holding_matches(card, h));
                let opens_pair = paired && owed.is_none();
                let done = !opens_pair && left <= 1;
                if done {
                    self.interrupts.remove(0);
                } else if let Some(Interrupt::SearchOnto { left, owed, .. }) =
                    self.interrupts.first_mut()
                {
                    if opens_pair {
                        *owed = held.iter().copied().find(|&h| Some(h) != kind);
                    } else {
                        *owed = None;
                        *left -= 1;
                    }
                }
                let copy = self.take_from_zone(seat, zone, card);
                // Nkishi's library shuffled as taken; Static Virtue's after.
                if if from_hand {
                    done
                } else {
                    zone == Zone::Library
                } {
                    self.state.seats[seat.index()]
                        .library
                        .shuffle(&mut self.rng);
                }
                let entering =
                    crate::state::CardInPlay::entering(card, copy, seat, Some(minion), 0);
                self.place_now(entering, crate::plays::ModeLayers::Push, false);
                // Moved from hand: replaced (spec §6.8.3).
                if zone == Zone::Hand {
                    self.replace(seat, 1, None);
                }
            }
            // Put on: no cost, no fetch of its own; a hand card replaced
            // (spec §6.8.3).
            (Interrupt::SecondCopyOnto { seat, minion, .. }, Choice::TakeFrom { zone, card }) => {
                use crate::decision::Zone;
                self.interrupts.remove(0);
                let copy = self.take_from_zone(seat, zone, card);
                let play = crate::cards::PlayCtx {
                    copy: crate::copies::Untracked(copy),
                    ..crate::cards::PlayCtx::new(
                        seat,
                        card,
                        0,
                        Some(minion),
                        crate::cards::Target::Minion(minion),
                        0,
                    )
                };
                self.put_equipment(seat, minion, play);
                if zone == Zone::Hand {
                    self.replace(seat, 1, None);
                }
            }
            (Interrupt::SecondCopyOnto { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            // Kaymakli Fragment's draw: the hand discarded down as it settles.
            (Interrupt::DrawUpTo { seat, .. }, Choice::SetX(n)) => {
                self.interrupts.remove(0);
                for _ in 0..n {
                    let s = &mut self.state.seats[seat.index()];
                    if let Some((c, copy)) = s.library.take_top() {
                        s.hand.put(c, copy);
                        self.offer_redraw(seat, c);
                    }
                }
            }
            (Interrupt::BindLocation { minion, .. }, Choice::ChooseCard(c)) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].bound_location =
                    crate::state::SparseOption(Some(crate::state::CardId(c)));
            }
            (Interrupt::ReturnBeforeBurn { card, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.return_out_of_play(card);
                if self.state.cards[card.index()].present() {
                    self.burn_card_in_play(card);
                }
            }
            (Interrupt::ReturnBeforeBurn { card, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.return_out_of_play(card);
            }
            (Interrupt::RedirectFirst { .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                if let Some(c) = self.combat.as_mut() {
                    c.redirect_first = c.side_of_minion(m);
                }
            }
            (Interrupt::RetainerBurnsWeaponOrDamage { retainer, .. }, c) => {
                self.interrupts.remove(0);
                let pick = match c {
                    Choice::BurnInPlay { card } => Some(CardId(card)),
                    _ => None,
                };
                if let Some(cb) = self.combat.as_mut() {
                    cb.retainer_picks.push((retainer, pick));
                }
            }
            (Interrupt::DrainBeforeSteals { .. }, Choice::ResolveRider { .. } | Choice::Pass) => {
                self.interrupts.remove(0);
                if let Some(c) = self.combat.as_mut() {
                    c.drain_first = Some(*choice != Choice::Pass);
                }
            }
            (Interrupt::SearchOnto { seat, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.seats[seat.index()]
                    .library
                    .shuffle(&mut self.rng);
            }
            (Interrupt::AwardTrophy { seat, minion, .. }, Choice::TakeFrom { zone, card }) => {
                use crate::decision::Zone;
                if let Some(Interrupt::AwardTrophy { searched, .. }) = self.interrupts.first_mut() {
                    *searched = true;
                }
                let st = &mut self.state.seats[seat.index()];
                let pile = match zone {
                    Zone::Hand => &mut st.hand,
                    Zone::Library => &mut st.library,
                    Zone::AshHeap => &mut st.ash,
                };
                let i = pile.iter().rposition(|&c| c == card).expect("found");
                let (card, copy) = pile.take(i);
                // Reshuffle a library searched (rulebook "Red List").
                if zone == Zone::Library {
                    self.state.seats[seat.index()]
                        .library
                        .shuffle(&mut self.rng);
                }
                let entering =
                    crate::state::CardInPlay::entering(card, copy, seat, Some(minion), 0);
                if let Some(id) = self.place_now(entering, crate::plays::ModeLayers::Push, false) {
                    // "Put 2 counters on this card" (Trophy: No Questions).
                    let mode = &crate::cards::lookup(card).expect("a trophy").modes[0];
                    for e in mode.effects {
                        if let crate::cards::Effect::EnterWithCounters(n) = *e {
                            self.state.cards[id.index()].counters += n;
                        }
                    }
                    self.trophy_moved(seat, minion, id);
                }
                // Back up before the blood hunt, not when the action ends.
                if zone == Zone::Hand {
                    self.draw_up(seat);
                }
            }
            (Interrupt::TakeChosen { card, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.burn_card_in_play(card);
            }
            (Interrupt::TakeChosen { minion, card, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                let seraph = self.state.is_black_hand(minion);
                self.state.layers.push(crate::cards::Layer {
                    scope: crate::cards::Scope::InPlay,
                    modifier: crate::cards::Modifier::HostBlackHand,
                    source: card,
                });
                if seraph {
                    self.state.layers.push(crate::cards::Layer {
                        scope: crate::cards::Scope::InPlay,
                        modifier: crate::cards::Modifier::HostSeraph,
                        source: card,
                    });
                }
                self.state.minions[minion.index()].sect_override = Some(crate::crypt::Sect::Sabbat);
                let trophy = Cause::Effect(Source::Card(card));
                let printed =
                    crate::cards::trophy_blood(self.state.cards[card.index()].card).unwrap_or(0);
                self.caused(trophy, |e| e.state.gain_blood(minion, printed));
                self.effect_unlock(minion);
            }
            (Interrupt::MoveClot { card, to, .. }, choice) => {
                self.interrupts.remove(0);
                if matches!(choice, Choice::ResolveRider { .. }) {
                    self.state.cards[card.index()].counters -= 1;
                    self.state.minions[to.index()].clots += 1;
                    self.state.minions[to.index()].clots_by =
                        Some(self.state.cards[card.index()].owner);
                }
            }
            (Interrupt::ClotFate { minion, .. }, choice) => {
                self.interrupts.remove(0);
                if matches!(choice, Choice::ChooseMinion(_)) {
                    self.burn_minion(minion);
                } else if self.state.minions[minion.index()].ready() {
                    self.state.minions[minion.index()].locked = true;
                    self.minions_to_torpor(&[minion]);
                }
            }
            (Interrupt::ChooseCardCircle { card, .. }, Choice::ChooseCircle(c)) => {
                self.interrupts.remove(0);
                self.state.cards[card.index()].chosen_circle = Some(c);
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
            (Interrupt::NameAnyAshMinion { seat, from: None }, Choice::ChooseSeat(s)) => {
                self.interrupts[0] = Interrupt::NameAnyAshMinion {
                    seat,
                    from: Some(s),
                };
            }
            (
                Interrupt::NameAnyAshMinion { from: Some(s), .. },
                Choice::AshVampire(card) | Choice::FindCard(card),
            ) => {
                let crypt = matches!(choice, Choice::AshVampire(_));
                self.interrupts.remove(0);
                if let Some(a) = self.action.as_mut() {
                    a.ash_minion_named = Some((s, card, crypt));
                }
            }
            (Interrupt::RepresentPicks { .. }, Choice::Pass) => {
                self.interrupts.remove(0);
            }
            (
                Interrupt::BurnReplacement {
                    minion, to_torpor, ..
                },
                choice @ (Choice::ResolvePrinted { .. } | Choice::ResolveRider { .. }),
            ) => {
                self.interrupts.remove(0);
                let r = self
                    .burn_replacements(minion)
                    .into_iter()
                    .find(|r| r.choice(minion) == choice)
                    .expect("an offered replacement");
                self.apply_burn_replacement(minion, r, to_torpor);
            }
            (Interrupt::KeepSpentBlood { n, .. }, Choice::ChooseCard(c)) => {
                self.interrupts.remove(0);
                self.state.cards[c as usize].counters += n;
            }
            (
                Interrupt::BurnCopiesPick {
                    seat,
                    card,
                    left,
                    up_to,
                    of,
                },
                Choice::ChooseCard(c),
            ) => {
                self.burn_card_in_play(CardId(c));
                if left > 1 {
                    self.interrupts[0] = Interrupt::BurnCopiesPick {
                        seat,
                        card,
                        left: left - 1,
                        up_to,
                        of,
                    };
                } else {
                    self.interrupts.remove(0);
                    if up_to > 0 && !self.represent_slots(seat).is_empty() {
                        self.interrupts.insert(
                            0,
                            Interrupt::RepresentPicks {
                                seat,
                                left: up_to,
                                of,
                            },
                        );
                    }
                }
            }
            (Interrupt::RepresentPicks { seat, left, of }, Choice::Slot(i)) => {
                let (slot, on) = self.state.take_slot(seat, i as usize);
                let m = self.raise_represented(seat, slot.card, true, seat, slot.copy, of);
                self.slot_cards_onto(on, m);
                if left > 1 {
                    self.interrupts[0] = Interrupt::RepresentPicks {
                        seat,
                        left: left - 1,
                        of,
                    };
                } else {
                    self.interrupts.remove(0);
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
            (Interrupt::EquipFromHandOnEntry { minion, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.stage_contests(minion);
            }
            // Equipped, not by an equip action (LSJ 20080815): its cost as
            // normal (LSJ 20011214), the card replaced.
            (
                Interrupt::EquipFromHandOnEntry { seat, minion, .. },
                Choice::EmployFromHand { card, mode },
            ) => {
                self.interrupts.remove(0);
                let hand = &mut self.state.seats[seat.index()].hand;
                let i = hand.iter().position(|&c| c == card).expect("in hand");
                let (_, copy) = hand.take(i);
                let (mut pool, mut blood) = (
                    cards::pool_cost(&self.state, seat, card, cards::Target::Minion(minion)),
                    cards::blood_cost(&self.state, card, 0),
                );
                self.state
                    .printed_cost_off_x(minion, card, mode, 0)
                    .apply(&mut pool, &mut blood);
                self.state.seats[seat.index()].lose_pool(pool);
                self.state.lose_blood(minion, blood);
                let play = cards::PlayCtx {
                    copy: crate::copies::Untracked(copy),
                    ..cards::PlayCtx::new(
                        seat,
                        card,
                        mode,
                        Some(minion),
                        cards::Target::Minion(minion),
                        0,
                    )
                };
                let id = self.equip_from_hand(seat, minion, play);
                self.ask_x_convictions(seat, minion, id);
                self.state.equipped_off_action.0.push((minion, card));
                self.replace(seat, 1, None);
                if !self.oust_if_out(seat) {
                    self.stage_contests(minion);
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
            (
                Interrupt::EntryBurn { .. } | Interrupt::BurnOverCapacity { .. },
                Choice::BurnInPlay { card },
            ) => {
                self.interrupts.remove(0);
                self.burn_card_in_play(CardId(card));
            }
            (front, choice) => unreachable!("illegal choice {choice:?} for {front:?}"),
        }
    }

    /// The minions a `CircleBlood` move takes from and the ones it moves
    /// to: a circle's Blood Brothers both, or one minion and the seat's
    /// other ready vampires (Rave).
    fn move_among(&self, seat: SeatId, among: MoveAmong) -> (Vec<MinionId>, Vec<MinionId>) {
        match among {
            MoveAmong::Circle(c) => {
                let members = self.circle_members(seat, c);
                (members.clone(), members)
            }
            MoveAmong::From(f) => {
                let to = self
                    .state
                    .minions_of(seat)
                    .filter(|&m| {
                        let v = &self.state.minions[m.index()];
                        m != f && v.ready() && v.is_vampire()
                    })
                    .collect();
                let from = if self.state.minions[f.index()].present() {
                    vec![f]
                } else {
                    vec![]
                };
                (from, to)
            }
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
        self.state.gain_pool(seat, n as i32);
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
                    && st.card_in_play_matches(
                        CardId(i as u32),
                        crate::cards::CardFilter::AllyOrRetainer,
                    )
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
    /// with its own layers, with no as-played window.
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
            e.state.seats[seat.index()].lose_pool(pool);
            e.state.lose_blood(minion, blood);
        });
        self.note_pool_burned(seat);
        let placed = crate::state::CardInPlay::entering(card, copy, seat, Some(minion), 0);
        self.place_now(placed, crate::plays::ModeLayers::Push, false);
        if pool > 0 && self.state.seats[seat.index()].pool <= 0 {
            self.acting_pool_out(seat);
        }
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

/// The hand cards Lucubratio's text discards instead of locking for a block:
/// its printed requirement's filter (`Stat::MayDiscardInsteadOfBlockLock`).
fn block_discard_filter(
    st: &crate::state::GameState,
    m: crate::ids::MinionId,
) -> Option<crate::cards::CardFilter> {
    let stat = crate::cards::Stat::MayDiscardInsteadOfBlockLock;
    crate::cards::printing(&st.minions[m.index()], stat)?
        .printed
        .iter()
        .filter(|p| p.stat == stat)
        .flat_map(|p| p.requires)
        .find_map(|r| match r {
            crate::cards::Requirement::HandHas(f, _) => Some(*f),
            _ => None,
        })
}
