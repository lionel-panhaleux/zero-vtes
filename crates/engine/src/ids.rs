#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct SeatId(pub u8);

impl SeatId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MinionId(pub u32);

impl MinionId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A set of Methuselahs, by seat ("the same set of Methuselahs", Shadow
/// Cast [OBL]).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct SeatSet(pub u8);

impl SeatSet {
    pub const EMPTY: SeatSet = SeatSet(0);

    pub fn single(s: SeatId) -> SeatSet {
        SeatSet(1 << s.0)
    }

    pub fn contains(self, s: SeatId) -> bool {
        self.0 & (1 << s.0) != 0
    }

    pub fn without(self, s: SeatId) -> SeatSet {
        SeatSet(self.0 & !(1 << s.0))
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Every seat of `self` is in `other`.
    pub fn is_subset(self, other: SeatSet) -> bool {
        self.0 & !other.0 == 0
    }
}
