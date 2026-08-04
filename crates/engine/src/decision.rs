use crate::ids::{MinionId, SeatId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    Bleed,
    Hunt,
    LeaveTorpor,
}

/// One indexed option inside a decision request. `Pass` is always explicit;
/// its meaning is window-specific (decline to block, end phase, keep the
/// Edge, forfeit remaining transfers).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Pass,
    BurnEdge,
    TakeAction {
        minion: MinionId,
        action: ActionKind,
    },
    DeclareBlock {
        minion: MinionId,
    },
    Transfer {
        slot: u8,
    },
}

/// One name per legality filter (DESIGN §4.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowLabel {
    UnlockEdge,
    MinionPhase,
    ActionBlock,
    InfluenceTransfers,
}

#[derive(Clone, Debug)]
pub struct DecisionRequest {
    pub seat: SeatId,
    pub window: WindowLabel,
    pub options: Vec<Choice>,
}
