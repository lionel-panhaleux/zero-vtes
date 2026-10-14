//! Cards stored on cards and the draws they change (slice 8c-ii): draw
//! alternatives ("if you would draw a card from your library, you can draw
//! one of those cards instead": Black Market Cache, Shilmulo Tarot), a
//! replacement's treatment beside Visit from the Capuchin, Heart of
//! Nizchetus' draws and bottom move, Black Market Cache's search.
//!
//! These choices interrupt whatever drew the cards, as a discard-down does:
//! `Engine::interrupts` is served, in order, before any other decision. A
//! Methuselah with no draw alternative and nothing queued draws at once.

use crate::cards::{self, CardFilter, Effect, Hook};
use crate::decision::{Choice, DecisionRequest, WindowLabel, Zone};
use crate::game::Engine;
use crate::ids::{MinionId, SeatId};
use crate::library::Label;
use crate::state::{distinct, CardId};

/// A pending choice about cards drawn, moved or stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interrupt {
    /// `seat` moves `card` (Fear of Mekhet) to another justicar or Inner
    /// Circle member, any Methuselah's, in torpor included (ANK 20210109;
    /// rules-qa 49.4: not locked by the move); burned if there is none.
    MoveCardToTitled { seat: SeatId, card: CardId },
    /// Blood Trade: 1 blood from `from` to a vampire of another
    /// Methuselah's, the player's choice; with `anarchs`, to another ready
    /// Anarch of the player's (The Status Perfectus).
    GiveBlood {
        seat: SeatId,
        from: MinionId,
        anarchs: bool,
    },
    /// Heidelberg Castle: `seat` moves from `from` to another ready vampire
    /// of theirs — chosen first (`to`), then the blood (any amount, the
    /// excess to the bank: ANK 20170804), then equipment and retainers one
    /// at a time, each one `to` could hold (LSJ 19980831), until done.
    MoveBetween {
        seat: SeatId,
        from: MinionId,
        to: Option<MinionId>,
        blood_done: bool,
    },
    /// "Burned unless its controller burns n pool" (Conquest of Humanity):
    /// `seat` pays or lets `card` burn.
    PayOrBurn {
        seat: SeatId,
        card: CardId,
        pool: u8,
    },
    /// Nightmares upon Nightmares: `minion` is locked unless its
    /// Methuselah discards a card (not replaced until their master
    /// phase).
    LockOrDiscard { seat: SeatId, minion: MinionId },
    /// Society of Leopold: `minion` burns `n` blood or is burned, its
    /// Methuselah's choice (Pass: burned) — or `card` is burned instead
    /// (Robert Carter: "Carter's employer burns 1 blood, or Carter is
    /// burned").
    /// Maabara: a library card of `seat`'s ash heap moved onto `card`.
    StoreOwnAsh { seat: SeatId, card: CardId },
    /// Maabara: a card on `card` moved to the top of `seat`'s library.
    StoredToLibraryTop { seat: SeatId, card: CardId },
    /// "Each anarch burns 1 blood or becomes Camarilla" (Waiting Game): its
    /// controller's choice — `BurnBloodToAvoid` burns, `Pass` makes it
    /// Camarilla (the only way with no blood).
    BloodOrCamarilla { seat: SeatId, minion: MinionId },
    BloodOrBurned {
        seat: SeatId,
        minion: MinionId,
        n: u8,
        card: Option<CardId>,
    },
    /// A replacement Visit from the Capuchin would take instead while a
    /// draw alternative applies too: one treatment or the other (ANK
    /// 20200905; rules-qa 9.4). `except`: the card being replaced.
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
    /// Black Market Cache's search: up to `left` more cards to put on
    /// `card`.
    StoreSearch {
        seat: SeatId,
        card: CardId,
        left: u8,
    },
    /// "After this ally enters play, burn an ally or retainer you control"
    /// (War Ghoul): the controller's pick among the others, the entering
    /// ally itself if there is none (RTR 19961113).
    EntryBurn {
        seat: SeatId,
        ally: crate::ids::MinionId,
    },
    /// "After this ally enters play, burn it unless you remove an ally or
    /// vampire in your ash heap from the game" (Rotting Behemoth): the
    /// controller may decline, and the ally burns (LSJ 20030701-2).
    EntryRemoveOrBurn {
        seat: SeatId,
        ally: crate::ids::MinionId,
    },
    /// A block attempt made costly as it goes on (Where the Veil Thins
    /// [OBL], Tenebrous Form [OBT]): its minion pays the `n` blood that
    /// card adds, or resigns the attempt and pays nothing (LSJ 20031121-2);
    /// an ally cannot burn blood.
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
    /// Amici Noctis: `left` blood to distribute; first how much to the
    /// pool (at most `cap`), the rest among the ready `clan` vampires.
    MarginPool {
        seat: SeatId,
        left: u8,
        cap: u8,
        clan: &'static str,
    },
    /// Amici Noctis: `left` blood for the ready `clan` vampires below
    /// capacity, 1 each, offered in arena order from minion `next`: give or
    /// skip, a skip only while enough of them are left.
    MarginBlood {
        seat: SeatId,
        left: u8,
        next: u32,
        clan: &'static str,
    },
    /// A minion controlled by `of`, chosen as an effect resolves (Break the
    /// Bonds [obf], The Platinum Protocol [pro]: "the target is chosen upon
    /// resolution", ANK 20240706): locked, if the player wants to, or given
    /// one of the player's corruption counters.
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
    /// `seat` looks at `of`'s hand and discards a card of their choice from
    /// it, which `of` replaces (Revelations [aus]).
    DiscardFromHandOf { seat: SeatId, of: SeatId },
    /// `seat` discards `left` more cards of their choice from hand (Shroud
    /// of Decay [obl]); all `done` are replaced once every discard is made
    /// (advanced rules §6.8.3).
    /// `seat` reorders the top `total` cards of their library (Eulogio
    /// Sánchez de los Reyes): `placed` of them chosen so far, from the top
    /// down; the rest still in their old order below them.
    Reorder { seat: SeatId, total: u8, placed: u8 },
    DiscardChosen {
        seat: SeatId,
        left: u8,
        done: u8,
        /// Combat cards only (Target Vitals' cancel: "discard two combat
        /// cards").
        combat_only: bool,
    },
    /// `seat` removes `left` more cards of their choice from `of`'s ash
    /// heap from the game; then `of` burns `pool` pool (Shroud of Decay
    /// [OBL]; LSJ 20090710: there is no order to the ash heap) — from their
    /// own, pool 0, Marchesa Liliana's cost. Burned vampires are ash heap
    /// cards too (rules-qa 42.7).
    RemoveFromAsh {
        seat: SeatId,
        of: SeatId,
        left: u8,
        pool: u8,
    },
    /// "You can search your library (shuffle afterward), hand and/or ash
    /// heap for a Discipline master card and put it on this new vampire"
    /// (Childe of the Revolution): put on, not played.
    DisciplineSearch {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// Learjet: `seat` may draw an additional card (`DrawLibrary`), the
    /// hand then discarded down; or pass.
    MayDrawExtra { seat: SeatId },
    /// Infernal Pursuit [CEL], several cards replaced at once: one
    /// replacement of `seat`'s and its `extra` additional cards, then as
    /// many discarded, before the next (ANK 20180925-1).
    PursuitDraw { seat: SeatId, extra: u8 },
    /// Those `left` extra cards of `seat`'s discarded, not replaced.
    PursuitDiscard { seat: SeatId, left: u8 },
    /// Baltimore Purge: `seat` chooses a ready vampire of `of`'s not older
    /// than `host` (the purge's vampire), sent to torpor with it once every
    /// Methuselah's is chosen.
    PurgePick {
        seat: SeatId,
        of: SeatId,
        host: MinionId,
    },
    /// "Move up to n blood from the acting vampire to this new vampire"
    /// (Third Tradition: Progeny): `seat` sets how much, 0 to `max` of what
    /// `from` has (`SetX`).
    MoveBloodUpTo {
        seat: SeatId,
        from: MinionId,
        to: MinionId,
        max: u8,
    },
    /// "Steal up to n blood" (François Villon G2): `seat` sets how much,
    /// 1 to `max`, from `from` to `to` (`SetX`).
    StealUpTo {
        seat: SeatId,
        from: MinionId,
        to: MinionId,
        max: u8,
    },
    /// Antonio d'Erlette (`minion`): a mage ally from `seat`'s library or
    /// hand he could play, put in play, its cost paid (Pass: none); the
    /// library shuffled either way.
    AllySearch {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// "Remove a crypt card in your uncontrolled region from the game"
    /// (Wider View): the player's pick, the counters on it to the blood
    /// bank; removed cards are public (LSJ 20080314).
    RemoveUncontrolled { seat: SeatId },
    /// "As … its controller changes, its controller chooses a ready vampire
    /// they control" (The Rack): none if they have none.
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
    /// Ashur Tablets' picks from `seat`'s ash heap: first the card to hand
    /// (a pass chooses none), then up to `left` more shuffled into the
    /// library (a pass stops), `to_library` once the hand card is chosen.
    AshurPicks {
        seat: SeatId,
        left: u8,
        to_library: bool,
    },
    /// "A non-titled vampire you control becomes Anarch" (Gilbert Duane):
    /// the player's pick, none if they have none.
    BecomesAnarch { seat: SeatId },
    /// Anarch Convert entering play: "you can remove him from the game to
    /// make a non-titled vampire you control Anarch" — that vampire chosen
    /// (`chosen`), then "either gain 1 pool or draw 1 card from your
    /// crypt". Before any contest (LSJ 20080526, RTR 20180303): `forced`
    /// when he came out beside a copy his Methuselah controls (no voluntary
    /// self-contest); declined, his contests are staged.
    AnarchConvert {
        seat: SeatId,
        convert: crate::ids::MinionId,
        forced: bool,
        chosen: Option<crate::ids::MinionId>,
    },
    /// "Add n blood to a `clan` vampire in your uncontrolled region"
    /// (Hunting the Beast [AUS]): the player's pick, none if they have none.
    /// With no clan, any of them, and optional ("may move a counter from
    /// the bank to a vampire in his or her uncontrolled region", Flames of
    /// Insurrection).
    UncontrolledBlood {
        seat: SeatId,
        clan: Option<&'static str>,
        n: u8,
        may: bool,
    },
    /// "Burns 1 pool unless they burn a vampire they control with capacity
    /// 4 or more to burn this card" (Antediluvian Awakening): `BurnPool`
    /// pays, `ChooseMinion` burns that vampire and the card.
    PoolOrVampire {
        seat: SeatId,
        card: CardId,
        pool: u8,
        cap: u8,
    },
    /// "Search your library for a <filter> card. Show it to all players and
    /// add it to your hand" (Fortschritt Library): a library card
    /// (`Choice::FindCard`), or none (Pass); the library shuffled
    /// afterward, the hand discarded down as it settles.
    LibraryToHand {
        seat: SeatId,
        filter: crate::cards::CardFilter,
    },
    /// "That Methuselah chooses either to burn n pool or to lose all
    /// transfers during his or her next influence phase" (Leandro):
    /// `BurnPool` burns, `Pass` loses the transfers — no default (LSJ
    /// 20021210-1).
    PoolOrNoTransfers { seat: SeatId, pool: u8 },
    /// "Lock this card or burn n pool" (Bleeding the Vine): `ResolveRider`
    /// locks it (if unlocked), `BurnPool` pays.
    LockOrPay {
        seat: SeatId,
        card: CardId,
        pool: u8,
    },
    /// "Search your crypt for a `clan` vampire … move them to your
    /// uncontrolled region (shuffle afterward)" (Disciple of Lazarus): a
    /// crypt card (`Choice::AshVampire`, a CRYPT index), or none (Pass).
    CryptSearch { seat: SeatId, clan: &'static str },
    /// Solomon Batanea: the slot of `of`'s uncontrolled region that gives 1
    /// blood to `of`'s pool (`Choice::Slot`, `of`'s region).
    PreySlotToPool { seat: SeatId, of: SeatId },
    /// Erlik entering play: up to `left` more blood moved from `from` to
    /// `seat`'s uncontrolled `clan` vampires, one at a time (`Choice::Slot`);
    /// Pass stops. His contests are staged after (LSJ 20080526).
    DistributeFrom {
        seat: SeatId,
        from: MinionId,
        clan: &'static str,
        left: u8,
    },
    /// "Move any amount of blood from the Blood Brothers in the <circle> to
    /// any other Blood Brothers in this circle" (Jerry, Redistribution):
    /// one move at a time among `seat`'s — `from` the source, `to` the
    /// destination, then the amount (`Choice::SetX`), each move a gain of
    /// its own (Slake the Thirst; owner, rules-qa 68.10), one blood move a
    /// pair of them (`paired`, source and destination); Pass ending the
    /// moves. With `equipment`, their equipment too ("move any amount of
    /// blood and equipment", Redistribution), one at a time, as many as
    /// wanted (`moving` the one picked).
    CircleBlood {
        seat: SeatId,
        circle: &'static str,
        from: Option<MinionId>,
        to: Option<MinionId>,
        equipment: bool,
        moving: Option<CardId>,
        paired: [Option<(MinionId, MinionId)>; 12],
    },
    /// "Move any number of blood counters from your pool to 1 or more
    /// vampires you control" (Gird Minions): a vampire (`to`), then the
    /// amount (`Choice::SetX`), up to its room (rulebook: no more blood
    /// than capacity); Pass ends the moves.
    PoolToVampires {
        seat: SeatId,
        to: Option<crate::ids::MinionId>,
    },
    /// Legacy: `minion`'s base version searched for in `seat`'s crypt
    /// (`Choice::AshVampire`, taken from the crypt first, then the ash
    /// heap), uncontrolled region (`Choice::Slot`) and ash heap, and merged
    /// with it; Pass finds none. The crypt shuffled after.
    BaseSearch {
        seat: SeatId,
        minion: crate::ids::MinionId,
    },
    /// New Blood entering play: the circle he belongs to
    /// (`Choice::ChooseCircle`, among `state::circles`), or his own circle
    /// of one (Pass).
    ChooseCircle { seat: SeatId, minion: MinionId },
    /// "Move a wraith or zombie ally from your ash heap to your ready
    /// region" (Split the Veil): the player's pick among the allies
    /// matching one of `of`; with `named`, the pick is named for the
    /// action being declared, put in play as it resolves (Hel-Blá).
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
    /// Lázár Dobrescu: named as the action is announced (rules-qa default
    /// readings) — `from`, a vampire with blood in `of`'s uncontrolled
    /// region, then the one of `seat`'s that gets it (`Choice::Slot`).
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
    /// The Sargon Fragment: the library card of `seat`'s ash heap the
    /// action declared names, among those matching `of` (Drozodny: a
    /// political action card).
    NameOwnAshCard {
        seat: SeatId,
        of: crate::cards::CardFilter,
    },
    /// Giovanni del Georgio: the card two copies of which in `of`'s ash
    /// heap the action declared names (`Terms::PreyAshPair`).
    NameAshPair { seat: SeatId, of: SeatId },
    /// "You can put this card on this vampire" (Fever Pitch [FOR]/[PRE]):
    /// `ChooseMinion` it, or pass — optional, though never worse than the
    /// ash heap: an agent may take it unasked (owner, 2026-10-08: rules-qa
    /// 53.5).
    MayPutOn {
        seat: SeatId,
        held: crate::combat::Held,
    },
    /// "You can add 1 counter to this location" (Cursed Abattoir):
    /// `ResolveRider` it, or pass — never worse to add (rules-qa 55.6).
    MayAddCounter { seat: SeatId, card: CardId },
    /// The Path of Lilith: `seat` may lock `card` to put a torture counter
    /// on `minion` (`ResolveRider`), or pass.
    MayTorture {
        seat: SeatId,
        card: CardId,
        minion: MinionId,
    },
    /// "You can gain n pool" printed on `minion` (Matasuntha): Pass, or
    /// `ResolvePrinted` to gain it — optional, never worse.
    MayGainPool {
        seat: SeatId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// "This acting vampire can burn n blood to move the stolen vampire to
    /// your ready region" (Graverobbing [DOM]): `ChooseMinion` it, or pass.
    MoveForBlood {
        seat: SeatId,
        payer: crate::ids::MinionId,
        minion: crate::ids::MinionId,
        n: u8,
    },
    /// "Once each turn, when you draw a card from your library, you may
    /// place that card on the bottom of your library and draw a new card"
    /// (Blackhorse Tanner, `minion`): `DrawLibrary` to do it with `card`,
    /// just drawn, or pass.
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
    /// A location on a minion, stolen, moves onto one of the new
    /// controller's able to hold it (Dominate Kine [DOM]; RTR 19960112):
    /// mandatory.
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
    /// One of the two oldest ready vampires of the chooser's, other than
    /// the one already picked, gains 1 blood (Gurchon Hall): mandatory; a
    /// tie for oldest is the chooser's pick (rules-qa 69.1).
    OldestHuntingGround(Option<crate::ids::MinionId>),
}

impl Interrupt {
    fn seat(self) -> SeatId {
        match self {
            Interrupt::GiveBlood { seat, .. }
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
            | Interrupt::StorePreyAsh { seat, .. }
            | Interrupt::StoreOwnAsh { seat, .. }
            | Interrupt::StoredToLibraryTop { seat, .. }
            | Interrupt::BeastDisciplines { seat, .. }
            | Interrupt::MayRecycleCanceled { seat, .. }
            | Interrupt::MayPutOn { seat, .. }
            | Interrupt::MayAddCounter { seat, .. }
            | Interrupt::MayTorture { seat, .. }
            | Interrupt::MayGainPool { seat, .. }
            | Interrupt::MoveForBlood { seat, .. }
            | Interrupt::MoveCardToTitled { seat, .. } => seat,
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
        for (i, c) in st.cards.iter().enumerate() {
            let id = CardId(i as u32);
            if !c.present() || c.stored.is_empty() || st.card_controller(id) != s {
                continue;
            }
            let applies = cards::lookup(c.card).is_some_and(|d| {
                d.riders.iter().any(|r| {
                    r.hook == Hook::WouldDraw && r.scope.applies(st, s, s) && st.rider_holds(r, id)
                })
            });
            if !applies {
                continue;
            }
            out.extend(distinct(&c.stored, |_| true).into_iter().map(|stored| {
                Choice::DrawStored {
                    card: i as u32,
                    stored,
                }
            }));
        }
        out
    }

    /// The card whose `WouldReplace` rider takes a replacement of `s`
    /// instead (Visit from the Capuchin), `except` the card being replaced
    /// ("a card other than this card").
    pub(crate) fn replacement_rider(&self, s: SeatId, except: Option<CardId>) -> Option<CardId> {
        let st = &self.state;
        (0..st.cards.len() as u32).map(CardId).find(|&id| {
            let c = &st.cards[id.index()];
            Some(id) != except
                && c.present()
                && st.card_controller(id) == s
                && cards::lookup(c.card).is_some_and(|d| {
                    d.riders.iter().any(|r| {
                        r.hook == Hook::WouldReplace
                            && r.scope.applies(&self.state, s, s)
                            && st.rider_holds(r, id)
                    })
                })
        })
    }

    /// A replacement taken by that rider instead: a counter burned, the
    /// card burned with none left.
    pub(crate) fn apply_replacement_rider(&mut self, id: CardId) {
        let def = cards::lookup(self.state.cards[id.index()].card).expect("definition");
        for r in def.riders.iter().filter(|r| r.hook == Hook::WouldReplace) {
            for e in r.effects {
                match *e {
                    Effect::RemoveCounter(k) => {
                        let c = &mut self.state.cards[id.index()];
                        c.counters = c.counters.saturating_sub(k);
                    }
                    Effect::BurnSelfWithoutCounters => {
                        if self.state.cards[id.index()].counters == 0 {
                            self.burn_card_in_play(id);
                        }
                    }
                    other => unreachable!("not a replacement rider effect: {other:?}"),
                }
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

    /// The vampire of `s`'s whose "once each turn, when you draw a card from
    /// your library, you may place that card on the bottom … and draw a new
    /// card" is unspent this turn (Blackhorse Tanner): in torpor too (a
    /// printed ability with no "if ready"), each Methuselah's turn (ANK
    /// 20180719-4).
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
            for e in r.effects {
                match *e {
                    Effect::BurnSelfWithoutStored => {
                        if self.state.cards[id.index()].stored.is_empty() {
                            self.burn_card_in_play(id);
                        }
                    }
                    other => unreachable!("not a draw alternative effect: {other:?}"),
                }
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
        while let Some(&front) = self.interrupts.first() {
            let s = front.seat();
            if self.state.seats[s.index()].ousted {
                self.interrupts.remove(0);
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
                    !self.state.seats[of.index()].ousted
                        && !self.state.seats[of.index()].hand.is_empty()
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
                    combat_only,
                } => {
                    let fits = |c: u16| {
                        !combat_only
                            || crate::library::LIBRARY[c as usize]
                                .types
                                .contains(&crate::library::CardType::Combat)
                    };
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
                Interrupt::AllySearch { minion, .. } => {
                    if present(minion) && !self.mage_allies(s, minion).is_empty() {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.state.seats[s.index()].library.shuffle(&mut self.rng);
                    continue;
                }
                Interrupt::DisciplineSearch { minion, .. } => {
                    if present(minion) && !self.discipline_masters(s).is_empty() {
                        return;
                    }
                    self.interrupts.remove(0);
                    self.state.seats[s.index()].library.shuffle(&mut self.rng);
                    continue;
                }
                Interrupt::CorruptionOwner { minion, .. } => {
                    if self.state.minions[minion.index()].corruption.len() > 1 {
                        return;
                    }
                    self.interrupts.remove(0);
                    let v = &mut self.state.minions[minion.index()];
                    if let [(owner, _)] = v.corruption[..] {
                        v.burn_corruption(owner, 1);
                    }
                    continue;
                }
                Interrupt::MinionOf { of, purpose, .. } => {
                    !self.minion_of_options(of, purpose).is_empty()
                }
                Interrupt::AllyFromAsh { seat, of, .. } => !self.ash_allies(seat, of).is_empty(),
                Interrupt::BeastDisciplines { minion, .. } => {
                    self.state.minions[minion.index()].in_play
                }
                Interrupt::StoreOwnAsh { seat, card } => {
                    self.state.cards[card.index()].present()
                        && !self.state.seats[seat.index()].ash.is_empty()
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
                | Interrupt::NameUncontrolled { .. }
                | Interrupt::NameSlotPair { .. }
                | Interrupt::NameOwnAshCard { .. } => self.action.is_some(),
                Interrupt::MayRecycleCanceled { seat, card, .. } => {
                    self.state.seats[seat.index()].ash.contains(&card)
                }
                Interrupt::MayPutOn { held, .. } => self.may_put_held(held),
                Interrupt::MayAddCounter { card, .. } => self.state.cards[card.index()].present(),
                Interrupt::MayTorture { card, minion, .. } => {
                    let c = &self.state.cards[card.index()];
                    c.present() && !c.locked && present(minion)
                }
                Interrupt::MayGainPool { seat, .. } => !self.state.seats[seat.index()].ousted,
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
                // The card gone (another copy's vampire burned it), the pool
                // is still due: the rider fired.
                Interrupt::PoolOrVampire { seat, .. } => !self.state.seats[seat.index()].ousted,
                Interrupt::PoolOrNoTransfers { seat, .. } => !self.state.seats[seat.index()].ousted,
                Interrupt::LibraryToHand { seat, .. } => !self.state.seats[seat.index()].ousted,
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
                Interrupt::MoveBetween {
                    seat,
                    from,
                    to,
                    blood_done,
                } => match to {
                    _ if !self.state.minions[from.index()].ready() => false,
                    None => !self.move_recipients(seat, from).is_empty(),
                    Some(to) if !self.state.minions[to.index()].ready() => false,
                    // No blood to move: on to the cards.
                    Some(_) if !blood_done && self.state.minions[from.index()].blood == 0 => {
                        self.interrupts[0] = Interrupt::MoveBetween {
                            seat,
                            from,
                            to,
                            blood_done: true,
                        };
                        continue;
                    }
                    Some(_) if !blood_done => true,
                    Some(to) => !self.movable_cards(from, to).is_empty(),
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
            Interrupt::MoveBetween {
                seat,
                from,
                to,
                blood_done,
            } => (
                WindowLabel::ResolutionChoice,
                match to {
                    None => self
                        .move_recipients(seat, from)
                        .into_iter()
                        .map(Choice::ChooseMinion)
                        .collect(),
                    Some(_) if !blood_done => (0..=st.minions[from.index()].blood)
                        .map(Choice::SetX)
                        .collect(),
                    Some(to) => {
                        let mut options = vec![Choice::Pass];
                        options.extend(
                            self.movable_cards(from, to)
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
            Interrupt::DiscardFromHandOf { of, .. } => (
                WindowLabel::ResolutionChoice,
                distinct(&st.seats[of.index()].hand, |_| true)
                    .into_iter()
                    .map(Choice::FindCard)
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
            Interrupt::DiscardChosen { combat_only, .. } => (
                WindowLabel::ResolutionChoice,
                (0..st.seats[s.index()].hand.len())
                    .filter(|&i| crate::state::first_copy(&st.seats[s.index()].hand, i))
                    .filter(|&i| {
                        !combat_only
                            || crate::library::LIBRARY[st.seats[s.index()].hand[i] as usize]
                                .types
                                .contains(&crate::library::CardType::Combat)
                    })
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
            Interrupt::DisciplineSearch { .. } => {
                let mut options = vec![Choice::Pass];
                options.extend(self.discipline_masters(s));
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
                seat, card, cap, ..
            } => (
                WindowLabel::RiderTerms,
                std::iter::once(Choice::BurnPool)
                    .chain(
                        st.minions_of(seat)
                            .filter(|&m| {
                                st.cards[card.index()].present()
                                    && st.minions[m.index()].is_vampire()
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
            Interrupt::LibraryToHand { seat, filter } => (
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
            Interrupt::MayAddCounter { card, .. } | Interrupt::MayTorture { card, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolveRider { card: card.0 }],
            ),
            Interrupt::MayGainPool { minion, .. } => (
                WindowLabel::ResolutionChoice,
                vec![Choice::Pass, Choice::ResolvePrinted { minion }],
            ),
            Interrupt::Redraw { .. } => {
                (WindowLabel::Draw, vec![Choice::Pass, Choice::DrawLibrary])
            }
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
                crate::state::distinct(&st.seats[seat.index()].ash, |c| of.matches(c))
                    .into_iter()
                    .map(Choice::FindCard)
                    .collect(),
            ),
            Interrupt::NameSlotPair { seat, of, from } => (
                WindowLabel::ResolutionChoice,
                match from {
                    None => (0..st.seats[of.index()].uncontrolled.len() as u8)
                        .filter(|&i| st.seats[of.index()].uncontrolled[i as usize].counters > 0)
                        .map(Choice::Slot)
                        .collect(),
                    Some(_) => (0..st.seats[seat.index()].uncontrolled.len() as u8)
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
                    distinct(&st.seats[s.index()].ash_crypt, |_| true)
                        .into_iter()
                        .map(Choice::AshVampire),
                );
                (WindowLabel::EntryEffect, options)
            }
        };
        Some(DecisionRequest::new(s, window, options))
    }

    /// The ready vampires a `ChooseForCard` may pick: its seat's, of its
    /// clan, other than the first chosen for a second choice.
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
                    // "Any ready minion" (LSJ 19990928).
                    MinionPurpose::HostCard(c) => {
                        v.ready() && st.can_hold(*m, st.cards[c.index()].card, &[])
                    }
                    MinionPurpose::Damage(_) => v.ready(),
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
        let m = self.recruit_ally(s, card, copy, 0);
        self.state.minions[m.index()].recruited = false;
        self.state.minions[m.index()].locked |= locked;
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

    /// The slots of `seat`'s uncontrolled region holding a `clan` vampire.
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

    fn uncontrolled_of_clan(&self, seat: SeatId, clan: &'static str) -> Vec<usize> {
        (0..self.state.seats[seat.index()].uncontrolled.len())
            .filter(|&i| self.state.slot_vampire(seat, i).1 == clan)
            .collect()
    }

    /// The non-titled vampires `seat` controls, in torpor too (Gilbert
    /// Duane: an Anarch one a legal no-op pick).
    /// The vampires Anarch Convert can make Anarch: non-titled ones of
    /// `seat`'s, already Anarch included (PIB 20121031), not himself.
    fn converts(&self, seat: SeatId, convert: crate::ids::MinionId) -> Vec<crate::ids::MinionId> {
        self.untitled_vampires(seat)
            .into_iter()
            .filter(|&m| m != convert)
            .collect()
    }

    fn untitled_vampires(&self, seat: SeatId) -> Vec<crate::ids::MinionId> {
        let st = &self.state;
        st.minions_of(seat)
            .filter(|&m| {
                st.minions[m.index()].is_vampire() && st.title_of(m) == crate::crypt::Title::None
            })
            .collect()
    }

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

    /// Minions about to leave the ready region (burned, to torpor, out of
    /// play, removed with their ousted Methuselah): the cards on them that
    /// pay for it offer their pool (Priority Contract).
    pub(crate) fn about_to_leave_ready(&mut self, ms: &[crate::ids::MinionId]) {
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
                    // The Black Throne of the Methuselah whose vampire the
                    // contract chose (PIB 20150512, ANK 20180129-2).
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
        for (id, r) in self.state.card_riders(Hook::ContractHostLeaves) {
            let c = &self.state.cards[id.index()];
            if !c.present() || c.locked || self.state.card_controller(id) != x {
                continue;
            }

            for e in r.effects {
                if let crate::cards::Effect::GainPool(n) = *e {
                    self.interrupts.push(Interrupt::LockForPool {
                        seat: x,
                        card: id,
                        n,
                    });
                }
            }
        }
    }

    /// `seat` takes control of the card in play `card` (stolen, given): a
    /// card that chooses on a change of control has its new controller
    /// choose (The Rack).
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
    fn move_recipients(&self, seat: SeatId, from: MinionId) -> Vec<MinionId> {
        self.state
            .minions_of(seat)
            .filter(|&m| {
                let v = &self.state.minions[m.index()];
                m != from && v.ready() && v.is_vampire()
            })
            .collect()
    }

    /// The equipment and retainers on `from` that `to` could have (LSJ
    /// 19980831: not on a vampire barred from them).
    fn movable_cards(&self, from: MinionId, to: MinionId) -> Vec<CardId> {
        if self.state.no_equipment_or_retainers(to) {
            return Vec::new();
        }
        let mut out: Vec<CardId> = self
            .state
            .equipment_on(from)
            .into_iter()
            .filter(|c| {
                self.state
                    .can_hold(to, self.state.cards[c.index()].card, &[])
            })
            .collect();
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
    /// Territory): one on a minion goes on a ready minion of the new
    /// controller, their choice — burned if none can hold it (LSJ
    /// 19990928; rules-qa 39.4).
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

    pub(crate) fn give_card_control(&mut self, card: CardId, seat: SeatId) {
        self.state.cards[card.index()].controller = seat;
        let rechooses = cards::lookup(self.state.cards[card.index()].card).is_some_and(|d| {
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

    /// Remove `seat`'s uncontrolled vampire `i` from the game: its crypt
    /// card to the removed cards, its counters to the blood bank; a
    /// banished one leaves with the cards on it, burned (a created one's
    /// own card removed with it).
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

    /// The Discipline master cards `s` could take, by zone and card: the
    /// library's by card (its order hidden), the hand's and ash heap's.
    fn discipline_masters(&self, s: SeatId) -> Vec<Choice> {
        let seat = &self.state.seats[s.index()];
        let mut out = Vec::new();
        let zones = [
            (Zone::Library, &seat.library),
            (Zone::Hand, &seat.hand),
            (Zone::AshHeap, &seat.ash),
        ];
        for (zone, cards) in zones {
            out.extend(
                distinct(cards, |c| {
                    CardFilter::HasLabel(Label::Discipline).matches(c)
                })
                .into_iter()
                .map(|card| Choice::TakeFrom { zone, card }),
            );
        }
        out
    }

    /// "Each other Methuselah chooses a ready vampire they control who is
    /// not older than this vampire; if you control a ready Lasombra, you
    /// choose the vampires instead" (Baltimore Purge): one pick per other
    /// Methuselah with a candidate, clockwise from the controller, in the
    /// open; the host's capacity read as each pick is made.
    pub(crate) fn start_purge(&mut self, host: MinionId) {
        let owner = self.state.minions[host.index()].owner;
        let lasombra = self.state.minions_of(owner).any(|m| {
            let v = &self.state.minions[m.index()];
            v.ready() && v.is_vampire() && v.clan() == Some("Lasombra")
        });
        self.purge_picks.clear();
        let mut any = false;
        for of in self.state.clockwise_from(owner) {
            if of == owner || self.state.seats[of.index()].ousted {
                continue;
            }
            if !self.purge_candidates(of, host).is_empty() {
                let seat = if lasombra { owner } else { of };
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

    /// The mage allies `s` could put in play for Antonio d'Erlette
    /// (`m`), by zone and card: one he could play — its clan requirement
    /// met — whose cost is payable after reductions (LSJ 20090920-2), not a
    /// unique one `s` already has in play.
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
                let v = &mut self.state.minions[minion.index()];
                let n = v.corruption_of(s);
                v.burn_corruption(s, n);
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
                self.state.seats[s.index()].pool += n as i32;
            }
            // Burned with its minion or not (PIB 20150512): the pool all
            // the same.
            (Interrupt::PoolForCard { card, n, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.burn_card_in_play(card);
                self.state.seats[s.index()].pool += n as i32;
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
            (Interrupt::AnarchConvert { seat, .. }, Choice::Pass) => {
                self.interrupts.remove(0);
                self.state.seats[seat.index()].pool += 1;
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
            (Interrupt::NameOwnAshCard { .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
                self.action.as_mut().expect("action").ash_named = Some(card);
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
                self.state.cards[card.index()].host = Some(m);
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
            (Interrupt::AllySearch { minion, .. }, c) => {
                self.interrupts.remove(0);
                if let Choice::TakeFrom { zone, card } = c {
                    let (pool, blood) = self.searched_ally_cost(s, minion, card);
                    let seat = &mut self.state.seats[s.index()];
                    let from = match zone {
                        Zone::Library => &mut seat.library,
                        Zone::Hand => &mut seat.hand,
                        Zone::AshHeap => &mut seat.ash,
                    };
                    let k = from.iter().rposition(|&x| x == card).expect("found card");
                    let (_, copy) = from.take(k);
                    self.state.minions[minion.index()].blood -= blood;
                    // Put in play, not recruited: it may act this turn
                    // (as Raise the Dead's, LSJ 20100204).
                    let m = self.recruit_ally(s, card, copy, 0);
                    self.state.minions[m.index()].recruited = false;
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
            (Interrupt::DisciplineSearch { minion, .. }, c) => {
                self.interrupts.remove(0);
                if let Choice::TakeFrom { zone, card } = c {
                    let seat = &mut self.state.seats[s.index()];
                    let from = match zone {
                        Zone::Library => &mut seat.library,
                        Zone::Hand => &mut seat.hand,
                        Zone::AshHeap => &mut seat.ash,
                    };
                    let k = from.iter().rposition(|&x| x == card).expect("found card");
                    let (_, copy) = from.take(k);
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
            (Interrupt::DiscardFromHandOf { of, .. }, Choice::FindCard(card)) => {
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
                    combat_only,
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
                    combat_only,
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
            (Interrupt::CorruptionOwner { minion, .. }, Choice::ChooseSeat(owner)) => {
                self.interrupts.remove(0);
                self.state.minions[minion.index()].burn_corruption(owner, 1);
            }
            (Interrupt::MinionOf { seat, purpose, .. }, Choice::ChooseMinion(m)) => {
                self.interrupts.remove(0);
                let v = &mut self.state.minions[m.index()];
                match purpose {
                    MinionPurpose::Lock
                    | MinionPurpose::LockVampire
                    | MinionPurpose::LockOldest(_) => v.locked = true,
                    MinionPurpose::HostCard(c) => self.state.cards[c.index()].host = Some(m),
                    MinionPurpose::Damage(n) => self.damage_out_of_combat(m, n, false),
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
            (Interrupt::MoveBetween { seat, from, .. }, Choice::ChooseMinion(to)) => {
                self.interrupts[0] = Interrupt::MoveBetween {
                    seat,
                    from,
                    to: Some(to),
                    blood_done: false,
                };
            }
            // The excess over what `to` can hold goes to the bank.
            (
                Interrupt::MoveBetween {
                    seat,
                    from,
                    to: Some(to),
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
                };
            }
            (Interrupt::MoveBetween { to: Some(to), .. }, Choice::ChooseCard(c)) => {
                self.state.cards[c as usize].host = Some(to);
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
            (Interrupt::MayGainPool { seat, n, .. }, Choice::ResolvePrinted { .. }) => {
                self.interrupts.remove(0);
                self.state.seats[seat.index()].pool += n as i32;
            }
            (
                Interrupt::MayPutOn { .. }
                | Interrupt::MayAddCounter { .. }
                | Interrupt::MayTorture { .. }
                | Interrupt::MayGainPool { .. },
                Choice::Pass,
            ) => {
                self.interrupts.remove(0);
            }
            (Interrupt::MayPutOn { held, .. }, Choice::ChooseMinion(_)) => {
                self.interrupts.remove(0);
                self.place_held(held, held.minion);
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
            (Interrupt::PayOrBurn { seat, pool, .. }, Choice::PayToCancel) => {
                self.interrupts.remove(0);
                self.pay_pool_for_ability(seat, pool as i32);
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
            (Interrupt::BloodOrBurned { minion, n, .. }, Choice::BurnBloodToAvoid) => {
                self.interrupts.remove(0);
                self.state.burn_blood(minion, n);
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
                s.held.master += 1;
            }
            (Interrupt::LockOrPay { card, .. }, Choice::ResolveRider { .. }) => {
                self.interrupts.remove(0);
                self.state.cards[card.index()].locked = true;
            }
            (Interrupt::LockOrPay { seat, pool, .. }, Choice::BurnPool) => {
                self.interrupts.remove(0);
                self.pay_pool_for_ability(seat, pool as i32);
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
            (Interrupt::PoolOrVampire { seat, pool, .. }, Choice::BurnPool) => {
                self.interrupts.remove(0);
                self.pay_pool_for_ability(seat, pool as i32);
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
            (Interrupt::LibraryToHand { seat, .. }, Choice::FindCard(card)) => {
                self.interrupts.remove(0);
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
                self.state.minions[from.index()].blood -= 1;
                self.state.add_uncontrolled_blood(seat, i as usize, 1);
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
                        self.state.cards[c.index()].host = Some(m);
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

    /// The pool amounts Amici Noctis may give: at most its cap, and at
    /// least what the vampires cannot take (all of it distributed where it
    /// fits).
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
}
