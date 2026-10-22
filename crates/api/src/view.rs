//! The table as a seat sees it: a snapshot, never the engine's state.

use serde::{Deserialize, Serialize};

use crate::ids::{
    ActionKind, ActionStep, CopyId, DecisionNo, KrcgId, MarkerKind, MinionKind, Phase, Range,
    ReferendumStep, Seat, StrikeKind, Trait, VoteSource,
};
use crate::option::Mode;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TableView {
    /// The viewing seat; absent: every card shown (records, tests).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewer: Option<Seat>,
    /// The next decision's number.
    pub decision: DecisionNo,
    /// Game turn, from 1.
    pub turn: u32,
    pub active: Seat,
    pub phase: Phase,
    pub seats: Vec<SeatView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combat: Option<CombatView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub referendum: Option<ReferendumView>,
    /// The active seat's transfers left, during its influence phase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfers: Option<u8>,
}

/// A physical card: its copy where the viewer may follow it, its KRCG id
/// where the viewer sees its face. A card in another seat's hand, a
/// library or a crypt has neither (a card back).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Card {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy: Option<CopyId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<KrcgId>,
    /// A crypt card (else a library card), hidden or not.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub crypt: bool,
    /// Its label among the copies of the same card from the same deck the
    /// viewer sees (1, 2…: "Embrace #2"): the lowest number no other
    /// visible copy holds, given as it comes into sight. With `id` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nth: Option<u8>,
    /// A minion's number among the minions of its name the viewer has
    /// met this game, from the second on ("War Ghoul (2)", whatever its
    /// deck), kept once given (an undo takes back those given since); none
    /// for the first, nor a ghoul token. On a minion in the view and on
    /// its entering or leaving play.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numbered: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SeatView {
    pub seat: Seat,
    pub name: String,
    pub pool: i32,
    /// Victory points, in halves of a point (a survivor at the time limit
    /// scores 1 half).
    pub vp_halves: u32,
    pub ousted: bool,
    pub edge: bool,
    pub prey: Seat,
    pub predator: Seat,
    /// Top last.
    pub crypt: Vec<Card>,
    /// Top last.
    pub library: Vec<Card>,
    pub hand: Vec<Card>,
    pub ash_heap: Vec<Card>,
    pub removed: Vec<Card>,
    pub uncontrolled: Vec<SlotView>,
    /// Library cards in the uncontrolled region with training counters
    /// (Web of Knives Recruit), out of play: `counters` the training
    /// counters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub training: Vec<InPlayView>,
    pub ready: Vec<MinionView>,
    pub torpor: Vec<MinionView>,
    /// Imbued with no life, controlled, not ready (rulebook "Appendix:
    /// Imbued Rules" 5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub incapacitated: Vec<MinionView>,
    /// Cards in play the seat controls on no minion.
    pub in_play: Vec<InPlayView>,
    /// Cards the seat is playing, not resolved yet.
    pub limbo: Vec<Card>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SlotView {
    pub card: Card,
    pub counters: u8,
    /// A banished vampire, waiting to be influenced out again.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub banished: bool,
    /// The cards it carries there, out of play until it returns.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<Card>,
}

impl SeatView {
    /// Every minion the seat controls: ready, in torpor, incapacitated.
    pub fn minions(&self) -> impl Iterator<Item = &MinionView> + Clone {
        self.ready
            .iter()
            .chain(&self.torpor)
            .chain(&self.incapacitated)
    }

    /// As `minions`, mutably.
    pub fn minions_mut(&mut self) -> impl Iterator<Item = &mut MinionView> {
        self.ready
            .iter_mut()
            .chain(&mut self.torpor)
            .chain(&mut self.incapacitated)
    }
}

/// A minion: named by its card's copy everywhere (options, events).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MinionView {
    pub card: Card,
    pub kind: MinionKind,
    pub name: String,
    /// Blood (a vampire) or life (an ally).
    pub blood: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u8>,
    pub locked: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub contested: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<Title>,
    /// The Methuselah whose ash heap it goes to, when not its controller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<Seat>,
    pub cards: Vec<InPlayView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<Marker>,
    /// Its traits as they stand (Black Hand gained included).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<Trait>,
}

/// A vampire's title.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Title {
    /// `primogen`, `prince`, `baron`, `justicar`, `inner_circle`,
    /// `archbishop`, `bishop`, `cardinal`, `priscus`, `regent`, `magaji`,
    /// `independent` or `laibon` (a unique title named in `of`), `votes` (a
    /// title by its votes only).
    pub id: String,
    /// The city (a prince, a baron, an archbishop), the clan (a justicar,
    /// the Inner Circle) or an independent or Laibon title's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<String>,
    /// Its votes, when the title says them (`independent`, `laibon`,
    /// `votes`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub votes: Option<u8>,
}

/// Public state on a minion beyond blood and cards: counters, marks,
/// effects lasting a while.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Marker {
    pub kind: MarkerKind,
    /// The Methuselah it belongs to (corruption counters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat: Option<Seat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>,
    /// The card it comes from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<KrcgId>,
    /// English, when the seat's settings ask for text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct InPlayView {
    pub card: Card,
    pub controller: Seat,
    pub locked: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub counters: u8,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub contested: bool,
    /// Cards stored on it (face down ones hidden to others), top last.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stored: Vec<Card>,
}

fn is_zero(n: &u8) -> bool {
    *n == 0
}

/// What an action or an option is aimed at: a Methuselah, or a card on
/// the table (a minion, an uncontrolled vampire, a card in play).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    Seat { seat: Seat },
    Card { copy: CopyId },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ActionView {
    pub seat: Seat,
    pub acting: CopyId,
    pub kind: ActionKind,
    pub step: ActionStep,
    /// The action card, for a card action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<Card>,
    /// Its mode (Disciplines, level) and X.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Mode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<u8>,
    /// The card in play granting the action (a location's, an equipment's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<CopyId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
    /// The Methuselah bled, for a bleed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bleed_target: Option<Seat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bleed: Option<u8>,
    pub stealth: i32,
    /// The minion attempting the block, or that blocked, and its intercept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker: Option<CopyId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intercept: Option<i32>,
    /// The Methuselahs who declined to block.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub declined: Vec<Seat>,
    /// The attempt under way is short of the stealth and its Methuselah
    /// passed: blocks are not declined yet, and it declines unless the
    /// table helps the attempt before the round ends.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub declining: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CombatView {
    /// The acting minion first.
    pub minions: [CopyId; 2],
    pub round: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
    /// Each side, the acting minion's first.
    pub sides: Vec<CombatSide>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CombatSide {
    pub minion: CopyId,
    /// Its strike once declared (the blocker chooses knowing the acting
    /// minion's), kept shown once resolved until the next pair.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strike: Option<Strike>,
    pub maneuvers: u8,
    pub presses: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Strike {
    pub kind: StrikeKind,
    /// Damage or blood stolen; in a combat, a damage strike's: what it
    /// inflicts now if it lands, before prevention.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ranged: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub aggravated: bool,
    /// The card it comes from (a weapon, a strike card).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<KrcgId>,
    /// Resolved this pair: its damage, before prevention.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<u8>,
    /// English, when the seat's settings ask for text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReferendumView {
    pub caller: Seat,
    pub step: ReferendumStep,
    /// The calling minion, if any (a blood hunt's has none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calling: Option<CopyId>,
    /// The political card (or what calls it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<KrcgId>,
    pub votes_for: u32,
    pub votes_against: u32,
    pub terms: Terms,
    /// Every vote cast so far, in order.
    pub votes: Vec<Vote>,
}

/// A referendum's terms chosen so far.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Terms {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seats: Vec<Seat>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<CopyId>,
    /// Snake_case clan id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clan: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allocated: Vec<Allocated>,
    /// The Methuselahs points are still to be allocated to, in order: the
    /// first is allocated to now, the last gets the rest.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<Seat>,
}

/// Points allocated to a Methuselah or a card in play.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Allocated {
    pub to: Target,
    pub points: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Vote {
    pub seat: Seat,
    pub source: VoteSource,
    /// The minion or card in play casting it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy: Option<CopyId>,
    pub favor: bool,
    pub n: u8,
    /// An abstention: not tallied.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub canceled: bool,
}
