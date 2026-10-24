//! What happened, told to a seat: decisions taken and their public
//! consequences, the engine's elided steps included. Everything an event
//! says is public to its recipient.

use serde::{Deserialize, Serialize};

use crate::ids::{
    ActionKind, CopyId, DecisionNo, MinionKind, Phase, Range, Seat, Trait, Window, Zone,
};
use crate::option::{OptionView, SlotRef};
use crate::view::{ActionView, Card, CombatView, Marker, ReferendumView, Strike, Target, Title};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Event {
    #[serde(flatten)]
    pub kind: EventKind,
    /// The event told in English, when the seat's settings ask for text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    /// A decision taken, by any seat, as the recipient may know it.
    Decided {
        decision: DecisionNo,
        seat: Seat,
        window: Window,
        /// The card (or minion, by its card) whose terms or effect it
        /// chose, as its `decision` told it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<crate::option::CardRef>,
        /// What an effect asked the choice for, as its `decision` told it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        purpose: Option<crate::ids::Purpose>,
        choice: Box<OptionView>,
    },
    /// A decision passed by itself (the seat's settings left only a pass).
    AutoPassed {
        decision: DecisionNo,
        seat: Seat,
        window: Window,
    },
    /// The seat took back its decisions from `to` on: the client drops
    /// what it showed since (the next view is the table at `to`).
    Undone {
        seat: Seat,
        to: DecisionNo,
    },
    Turn {
        turn: u32,
        active: Seat,
    },
    /// A phase begun (the unlock phase is its turn's `turn`).
    Phase {
        phase: Phase,
    },
    /// The active seat's transfers left, in its influence phase: as it
    /// begins and as they change.
    Transfers {
        left: u8,
    },
    Draw {
        seat: Seat,
        n: u8,
        crypt: bool,
    },
    /// A card an effect revealed to every Methuselah, from `seat`'s crypt
    /// or library (`card.crypt`): a top card, a search's find: its `id`
    /// told, no copy number (it stays in a hidden place, or goes face down
    /// or to a hand).
    Revealed {
        seat: Seat,
        card: Card,
    },
    /// `seat`'s hand looked at, told to the looking Methuselah only
    /// (SchreckNET, Vaticination): its cards as they stood, each `id`
    /// told, no copy number. The look lasts while the action does; a view
    /// after it hides the hand again.
    Looked {
        seat: Seat,
        cards: Vec<Card>,
        /// `library`: the top of the looking seat's own library, top first
        /// (Anna "Dictatrix11" Suljic); absent, the hand.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        zone: Option<crate::ids::Zone>,
    },
    /// A card played resolved: what its mode does (the options' curated
    /// vocabulary, docs/protocol.md §7, `on` as it applies).
    Resolved {
        seat: Seat,
        card: Card,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        effects: Vec<crate::option::Effect>,
    },
    /// A card played canceled: still played, to the ash heap (spec §1.8.3).
    Canceled {
        seat: Seat,
        card: Card,
    },
    Pool {
        seat: Seat,
        delta: i32,
        pool: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cause: Option<Cause>,
    },
    Blood {
        minion: CopyId,
        delta: i32,
        blood: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cause: Option<Cause>,
    },
    /// A minion or a card comes into play (on a minion, or not).
    EntersPlay {
        card: Card,
        /// Its controller.
        seat: Seat,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on: Option<CopyId>,
        /// A minion, of this kind (not a card on the table).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        minion: Option<MinionKind>,
        /// A lasting effect whose card is in the ash heap (a marker on
        /// its minion), not a card of its own: no copy.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        effect: bool,
    },
    /// A card moves from a place to another: drawn, discarded, played
    /// into limbo, burned from a library, stored on a card, into play or
    /// out of it (its `enters_play`/`leaves_play` the log's). `card` as
    /// the recipient may know it: a back where neither place shows it.
    CardMoves {
        card: Card,
        from: ZoneRef,
        to: ZoneRef,
    },
    /// A card in play moves to another minion (or off one).
    Moves {
        card: Card,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on: Option<CopyId>,
        /// A lasting effect whose card is in the ash heap (a marker on
        /// its minion), not a card of its own: no copy.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        effect: bool,
    },
    /// A minion or a card leaves play: to `to` when it went to a seat's
    /// zone (a burned minion to its ash heap), contested (a vampire set
    /// aside while its Methuselahs contest it, a unique card in its own
    /// contest, the cards on a contested minion with it).
    LeavesPlay {
        card: Card,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<Zone>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        minion: Option<MinionKind>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        contested: bool,
        /// A lasting effect whose card is in the ash heap (a marker on
        /// its minion), not a card of its own: no copy.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        effect: bool,
    },
    /// A minion or a card in play locked (true) or unlocked.
    Locked {
        copy: CopyId,
        locked: bool,
    },
    /// The counters on a card in play (`copy`), or the blood counters on
    /// an uncontrolled vampire, by its place (`slot`: another seat's is a
    /// back).
    Counters {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        copy: Option<CopyId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slot: Option<SlotRef>,
        delta: i32,
        counters: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cause: Option<Cause>,
    },
    /// A minion or a card in play now controlled by `seat` (The Coven's
    /// predator, a minion taken).
    Controller {
        copy: CopyId,
        seat: Seat,
    },
    /// A minion's capacity now (a card on it raising it).
    Capacity {
        minion: CopyId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        capacity: Option<u8>,
    },
    /// A minion's markers now (the view's `markers`), told when they
    /// changed since the last view: a Red List mark, a corruption
    /// counter, a lasting effect (its card told entering play too).
    Markers {
        minion: CopyId,
        markers: Vec<Marker>,
    },
    /// A minion's traits now (the view's `traits`), told when they
    /// changed since the last view (Black Hand gained, made Red List or
    /// infernal).
    Traits {
        minion: CopyId,
        traits: Vec<Trait>,
    },
    /// A Methuselah's victory points now, in halves (the view's
    /// `vp_halves`), told when they changed since the last view: an oust's
    /// predator, the last one standing, a survivor at the time limit.
    VictoryPoints {
        seat: Seat,
        vp_halves: u32,
    },
    /// A vampire's title now (absent: none).
    Title {
        minion: CopyId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<Title>,
    },
    Torpor {
        minion: CopyId,
        /// Goes to torpor (true) or leaves it.
        enters: bool,
    },
    /// An imbued enters the incapacitated region (true) or leaves it.
    Incapacitated {
        minion: CopyId,
        enters: bool,
    },
    Ousted {
        seat: Seat,
    },
    /// A withdrawal announced (§1): out of the game at the seat's next
    /// unlock phase unless broken.
    WithdrawalAnnounced {
        seat: Seat,
    },
    /// The withdrawal broken: pool or a vampire's blood lost or spent, or a
    /// minion of the seat in combat.
    WithdrawalBroken {
        seat: Seat,
    },
    /// Out of the game by a withdrawal: half a victory point, nothing for
    /// the predator; `ousted` in the view, no `ousted` event.
    Withdrew {
        seat: Seat,
    },
    /// The Edge goes to `seat` (absent: burned, no one holds it).
    Edge {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        seat: Option<Seat>,
    },
    BlockFails {
        minion: CopyId,
    },
    /// An action announced, decided or forced (taken by the engine itself:
    /// a hunt, Elen Kamjian's bleed); its `action_ends` follows. `acting`:
    /// the minion's card, numbered by its name, and `minion` its kind (it
    /// may have come into play in the same run).
    ActionStarts {
        seat: Seat,
        acting: Card,
        minion: MinionKind,
        action: ActionRef,
    },
    /// Another minion of the action's Methuselah now acting in it (Mask of
    /// a Thousand Faces): the action goes on as theirs, its `action_ends`
    /// naming them. `acting` and `minion` as `action_starts` tells them.
    ActingChanges {
        seat: Seat,
        acting: Card,
        minion: MinionKind,
    },
    /// The action under way as it stands, told when it moved since the
    /// last view (its step, stealth and intercept, the bleed, the seats
    /// that declined to block), the view's `action`.
    ActionState {
        action: Box<ActionView>,
    },
    ActionEnds {
        seat: Seat,
        acting: CopyId,
        action: ActionRef,
        outcome: ActionOutcome,
        /// The minion that blocked it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        blocker: Option<CopyId>,
    },
    ReferendumEnds {
        outcome: ReferendumOutcome,
        votes_for: u32,
        votes_against: u32,
        /// The political action it was called by.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action: Option<ActionRef>,
        /// A blood hunt's: the diablerist hunted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hunted: Option<CopyId>,
    },
    /// The referendum under way as it stands, told when it moved since
    /// the last view (its step, terms and votes), the view's
    /// `referendum`.
    ReferendumState {
        referendum: Box<ReferendumView>,
    },
    CombatBegins {
        minions: [CopyId; 2],
    },
    /// The combat under way as it stands, told when it moved since the
    /// last view (its round, range, each side's strike, maneuvers and
    /// presses), the view's `combat`.
    CombatState {
        combat: Box<CombatView>,
    },
    /// A combatant's strike declared (the acting minion's first: the
    /// blocker chooses knowing it); `None`: no strike.
    StrikeDeclared {
        minion: CopyId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        strike: Option<Strike>,
    },
    /// A pair of strikes resolved: each side's strike and the damage it
    /// dealt before prevention.
    Strikes {
        round: u8,
        pair: u8,
        range: Range,
        sides: Vec<StruckSide>,
    },
    CombatEnds,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StruckSide {
    pub minion: CopyId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strike: Option<Strike>,
    /// The damage the side inflicts this pair, before prevention: its
    /// strike's, its retainers' and environmental damage.
    pub damage: u8,
    /// The strike's share of `damage`.
    #[serde(default)]
    pub strike_damage: u8,
}

/// A seat's zone, and the card or minion a card is on (stored on a card,
/// in play on a minion).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ZoneRef {
    /// The place's holder as the card moved: a zone's seat, a card's
    /// controller, a minion's (the minion's for a card on it).
    pub seat: Seat,
    pub zone: Zone,
    /// The card it is stored on (`stored`), the minion it is on
    /// (`in_play`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<CopyId>,
    /// In an uncontrolled region (`from`: the one it leaves, `to`: the
    /// one it goes to): its place there, from the first, the moves told
    /// before it applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u8>,
}

/// What an action is: its kind, its card, what grants it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ActionRef {
    pub kind: ActionKind,
    /// The action card, for a card action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<Card>,
    /// What grants the action: a card in play, or another vampire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Card>,
    /// What it is aimed at, as it stands when told: a Methuselah (a
    /// bleed's: the one bled), or the minion or card in play it targets
    /// (an uncontrolled vampire not named).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
}

/// What made a `pool`, `blood` or `counters` change: a cost paid or an effect, of a
/// card or of the rules.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Cause {
    pub kind: CauseKind,
    /// A card played or in play, or a minion (its action, its ability).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<Card>,
    /// A rule of the game, with no card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<CauseRule>,
}

crate::ids::ids!(CauseKind {
    /// Paid for it.
    Cost,
    /// Done by it.
    Effect,
});

crate::ids::ids!(CauseRule {
    /// Combat blood: damage, a strike stealing it (its `strikes` event
    /// before).
    Strikes,
    /// A transfer in the influence phase: 1 pool to or from an
    /// uncontrolled vampire.
    Transfer,
    /// An oust: the predator's 6 pool, the ousted Methuselah's pool gone.
    Oust,
    /// The Edge burned for its pool.
    Edge,
    /// A contested card or title paid for.
    Contest,
    /// A disease counter's blood, as its vampire unlocks.
    Disease,
    Diablerie,
    /// Blood above a vampire's capacity draining.
    Capacity,
    /// Pool burned for votes (Business Pressure's round).
    Vote,
    /// A Methuselah withdrawing from the game: their pool gone with them.
    Withdrawal,
});

crate::ids::ids!(ActionOutcome {
    Succeeds,
    Blocked,
    Fails,
    Fizzles,
    /// Its Methuselah was ousted.
    Ousted,
    /// Ended otherwise (its acting minion gone).
    Ends,
});

crate::ids::ids!(ReferendumOutcome {
    Passes,
    Fails,
    Canceled,
});
