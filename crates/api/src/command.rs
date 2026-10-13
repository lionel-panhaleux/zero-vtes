//! What a seat sends.

use serde::{Deserialize, Serialize};

use crate::ids::DecisionNo;
use crate::spec::SettingsPatch;

/// A command as sent: an optional client id, echoed by an `error` it
/// causes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct In {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(flatten)]
    pub command: Command,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// Take the seat (the first frame on a connection: the seat's token,
    /// on a server), or take it again after a disconnection: `after` the
    /// last message's `seq` received, the host resends what followed;
    /// otherwise `joined`, then the pending decision if it is the seat's.
    Join {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after: Option<u64>,
    },
    /// Take an option of the pending decision, by its `index`.
    Answer { decision: DecisionNo, option: u32 },
    /// Answer an allocation under way whole: one amount per recipient, in
    /// the decision's `allocation.recipients` order.
    Allocate {
        decision: DecisionNo,
        amounts: Vec<u8>,
    },
    /// Cast all the votes the decision's `votes` counts one way; a
    /// standing order for the rest of the polling (each vampire's turn
    /// to cast is answered by it).
    VoteAll { decision: DecisionNo, favor: bool },
    /// Take back the seat's last decision, from the pending `decision`
    /// (whose `undo` says it can: nothing decided since, nothing hidden
    /// revealed). Named so a command sent twice undoes once.
    Undo { decision: DecisionNo },
    /// Change some of the seat's settings; the pending decision is sent
    /// again under them.
    Settings(SettingsPatch),
    /// Leave the game: it ends, recorded as abandoned by the seat.
    Leave,
}
