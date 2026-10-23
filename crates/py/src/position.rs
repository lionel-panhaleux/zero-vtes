//! A position as a player reads it, for one seat (training board, "The
//! critic against the position, measured"): the pools of the seat, its
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
    // Ready (in play, uncontested, out of torpor), unlocked among them,
    // the blood on the ready ones, those in torpor (an incapacitated
    // imbued with them, as the observation reads it).
    let minions = |x: SeatId| {
        let (mut ready, mut unlocked, mut blood, mut torpor) = (0.0, 0.0, 0.0, 0.0);
        for m in st.minions.iter().filter(|m| m.owner == x && m.present()) {
            if m.in_torpor || m.incapacitated {
                torpor += 1.0;
            } else {
                ready += 1.0;
                blood += m.blood as f32;
                if !m.locked {
                    unlocked += 1.0;
                }
            }
        }
        [ready / 5.0, unlocked / 5.0, blood / 20.0, torpor / 5.0]
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
    let (own, of_prey, of_pred) = (minions(s), minions(prey), minions(pred));
    // A bleed of one a ready minion: the lunge is the prey's pool within
    // the seat's ready minions, lunged the seat's within its predator's.
    let lunge = (own[0] * 5.0 >= seat(prey).pool as f32 && prey != s) as u8 as f32;
    let lunged = (of_pred[0] * 5.0 >= seat(s).pool as f32 && pred != s) as u8 as f32;
    let mut f = vec![
        seat(s).pool as f32 / 30.0,
        seat(prey).pool as f32 / 30.0,
        seat(pred).pool as f32 / 30.0,
        cross_pool / 30.0,
        alive.len() as f32 / 5.0,
        seat(s).half_vps as f32 / 10.0,
        best_other as f32 / 10.0,
    ];
    f.extend(own);
    f.extend(of_prey);
    f.extend(of_pred);
    f.extend([
        seat(s).hand.len() as f32 / 10.0,
        seat(s).library.len() as f32 / 60.0,
        st.turn as f32 / 60.0,
        seat(s).has_edge as u8 as f32,
        lunge,
        lunged,
        (seat(prey).pool as f32 - own[0] * 5.0) / 30.0,
    ]);
    debug_assert_eq!(f.len(), NAMES.len());
    f
}
