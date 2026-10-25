//! A position as a player reads it, for one seat (DESIGN §11.2 "The
//! critic against the position"): the pools of the seat, its
//! prey, its predator and the cross-table seats, the minions each side
//! has ready to act or block, the lunge (an oust within reach). Hand-made
//! features the critic's prediction is measured against
//! (`train/position.py`); public information only.

use zvtes::ids::SeatId;
use zvtes::state::GameState;

/// The features' names, in `features`' order.
pub const NAMES: [&str; 26] = [
    "pool",
    "prey_pool",
    "predator_pool",
    "cross_pool",
    "seats",
    "vps",
    "best_other_vps",
    "ready",
    "unlocked",
    "ready_blood",
    "torpor",
    "prey_ready",
    "prey_unlocked",
    "prey_ready_blood",
    "prey_torpor",
    "predator_ready",
    "predator_unlocked",
    "predator_ready_blood",
    "predator_torpor",
    "hand",
    "library",
    "turn",
    "edge",
    "lunge",
    "lunged",
    "prey_margin",
];

/// `NAMES`' values for seat `s` (not ousted), scaled to about [0, 1].
pub fn features(st: &GameState, s: SeatId) -> Vec<f32> {
    let (prey, pred) = (st.prey(s), st.predator(s));
    let seat = |x: SeatId| &st.seats[x.index()];
    let alive: Vec<SeatId> = (0..st.seats.len() as u8)
        .map(SeatId)
        .filter(|&x| !seat(x).ousted)
        .collect();
    let cross: Vec<f32> = alive
        .iter()
        .filter(|&&x| x != s && x != prey && x != pred)
        .map(|&x| seat(x).pool as f32)
        .collect();
    let cross_pool = if cross.is_empty() {
        0.0
    } else {
        cross.iter().sum::<f32>() / cross.len() as f32
    };
    // An ousted seat keeps its VPs.
    let best_other = st
        .seats
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != s.index())
        .map(|(_, x)| x.half_vps)
        .max()
        .unwrap_or(0);
    // The seats' positions as the context reads them (pools, minions,
    // sizes, the lunge both ways).
    let at = |x: SeatId| crate::context::position(st, x);
    let (own, of_prey, of_pred) = (at(s), at(prey), at(pred));
    let mut f = vec![
        own[0],
        of_prey[0],
        of_pred[0],
        cross_pool / 30.0,
        alive.len() as f32 / 5.0,
        own[2],
        best_other as f32 / 10.0,
    ];
    f.extend(&own[3..7]);
    f.extend(&of_prey[3..7]);
    f.extend(&of_pred[3..7]);
    f.extend([
        own[7],
        own[8],
        st.turn as f32 / 60.0,
        own[9],
        own[10],
        own[11],
        own[12],
    ]);
    debug_assert_eq!(f.len(), NAMES.len());
    f
}
