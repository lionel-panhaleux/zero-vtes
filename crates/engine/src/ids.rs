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
