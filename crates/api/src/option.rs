//! What a seat may do at a decision.

use serde::{Deserialize, Serialize};

use crate::ids::{ActionKind, CopyId, KrcgId, OptionKind, Role, Seat, Zone};
use crate::view::Target;

/// An option offered: its index (what an answer names), what it is, and
/// what it does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OptionView {
    /// The engine's index: stable while the decision is pending, even when
    /// the seat's settings hide other options.
    pub index: u32,
    #[serde(flatten)]
    pub key: ChoiceKey,
    /// What taking it does when its key does not say: a cost, what a
    /// chosen Methuselah or minion gets, an ability's effects, what a
    /// pass lets happen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
    /// The option in English, when the seat's settings ask for text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// What identifies an option: two options of one decision never share a
/// key (copies alike in hand are one option). A record keeps each answer
/// by its key, with the engine's copy numbers: a later engine finds the
/// same option whatever its index or however it describes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ChoiceKey {
    pub kind: OptionKind,
    /// The card played, used, taken, discarded or named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<CardRef>,
    /// The minion playing the card, using the ability, taking the action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<CopyId>,
    /// The minion the option is about otherwise (blocker, voter, payer,
    /// the vampire an ability is on).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minion: Option<CopyId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
    /// The action a minion takes (`take_action`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionKind>,
    /// The mode a card is played in (`play`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Mode>,
    /// An amount: blood, pool, votes, X, points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u8>,
    /// For or against a referendum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favor: Option<bool>,
    /// The blood an ability moves onto (+) or off (−) the vampire it is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blood: Option<i8>,
    /// Where a card chosen by name comes from (a search).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<ZoneRef>,
    /// A clan, a Discipline, a circle chosen: its snake_case id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Which of a card's abilities, when it has several.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ability: Option<u8>,
}

/// One thing an option does, to whom.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Effect {
    pub kind: EffectKind,
    /// The Methuselah it applies to (a pool, a hand).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat: Option<Seat>,
    /// The minion or card in play it applies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy: Option<CopyId>,
    /// Signed: gained (+) or lost (−).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<i32>,
    /// A card named (to hand, discarded).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<KrcgId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Zone>,
    /// What a chosen minion or Methuselah becomes (`role`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
}

crate::ids::ids!(EffectKind {
    Pool,
    Blood,
    Counters,
    Damage,
    Burn,
    Lock,
    Unlock,
    Draw,
    Discard,
    /// A card moved to a zone (`to`): a hand, a library, the
    /// uncontrolled region.
    Move,
    Steal,
    Corruption,
    /// The card or minion stays (a cost paid to keep it).
    Keep,
    /// What a chosen minion or Methuselah becomes for the card (`role`).
    Role,
});

/// A card an option names: its KRCG id when the viewer may know it, its
/// copy when it is one physical card (on the table; not a card in hand,
/// where copies alike are one option).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CardRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<KrcgId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy: Option<CopyId>,
    /// Played from cards stored on this card in play, not from the hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<CopyId>,
}

/// A card's mode: the printed line it is played from, at which
/// Disciplines and levels, as which card type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Mode {
    pub index: u8,
    /// The printed line (one line may split into several modes, told
    /// apart by the option's `effects`).
    pub line: u8,
    /// Empty for a card without Discipline requirement.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disciplines: Vec<DisciplineLevel>,
    /// The card type it is played as, for a card of several types:
    /// `action`, `action_modifier`, `reaction`, `combat`, `reflex`…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DisciplineLevel {
    /// The KRCG abbreviation, lowercase: `dom`, `aus`…
    pub discipline: String,
    pub superior: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ZoneRef {
    pub seat: Seat,
    pub zone: Zone,
}

/// An allocation of points under way, answered whole with
/// `Command::Allocate`: all `points` given, at most `cap` to a recipient,
/// at least `min_nonzero` recipients given some.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Allocation {
    pub points: u8,
    /// In the order `amounts` lists them.
    pub recipients: Vec<Recipient>,
    pub min_nonzero: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Recipient {
    pub to: Target,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap: Option<u8>,
    /// What one point does to it ("burns 1 pool": `pool`, `n` −1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
}

/// The votes the seat can cast now each way, all at once with
/// `Command::VoteAll` (its vampires' and its calling card's).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct VoteTotals {
    pub favor: u8,
    pub against: u8,
}
