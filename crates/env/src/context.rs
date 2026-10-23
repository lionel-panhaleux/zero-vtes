//! The seats' context (DESIGN §5 "The seats' context"): for each seat, in
//! relative order from the viewer, where it stands (pool, VPs, its minions
//! ready to act or block and what they can do, the lunge) and what it has
//! shown over the whole game (`zvtes::profile`). Public information only.
//! Laid out feature by feature, each across the seats (`names`): a trained
//! network reads it by position, so a feature is appended at the end of
//! `names` and adds `SLOTS` columns there, never inserted (`Net::load` and
//! `remap_vocab` widen an older layer with zero columns at the end;
//! `tests/context.rs` holds the names to their order).

use zvtes::game::Engine;
use zvtes::ids::SeatId;
use zvtes::profile::PROFILE;
use zvtes::state::GameState;

/// The relative seats encoded (tables have 4 or 5).
pub const SLOTS: usize = 5;

/// A seat's position, `position`'s order.
pub const POSITION: [&str; 15] = [
    "pool",
    "ousted",
    "vps",
    "ready",
    "unlocked",
    "ready_blood",
    "torpor",
    "hand",
    "library",
    "edge",
    "lunge",
    "lunged",
    "prey_margin",
    "ready_capacity",
    "ready_equipment",
];

/// The Disciplines whose levels a seat's ready vampires sum
/// (`zvtes::state::disciplines` at the context's making; one the pool
/// gains later is a feature appended at the end of `names`).
pub const DISCIPLINES: [&str; 25] = [
    "obf", "pre", "dom", "pot", "cel", "for", "tha", "qui", "aus", "pro", "obl", "ani", "chi",
    "dem", "abo", "nec", "obe", "vic", "obt", "san", "spi", "dai", "ser", "vis", "myt",
];

/// A seat's features: present, its position, its Disciplines, its
/// profile per round.
pub const PER_SEAT: usize = 1 + POSITION.len() + DISCIPLINES.len() + PROFILE.len();

/// The context's length: each feature for `SLOTS` seats, zeros past the
/// table.
pub const LEN: usize = SLOTS * PER_SEAT;

/// The features' names, in the context's order: feature `f` of the seat
/// `k` seats after the viewer is column `f * SLOTS + k`.
pub fn names() -> Vec<String> {
    let mut out = vec!["present".to_string()];
    out.extend(POSITION.iter().map(|n| n.to_string()));
    out.extend(DISCIPLINES.iter().map(|d| format!("discipline:{d}")));
    out.extend(PROFILE.iter().map(|n| n.to_string()));
    out
}

/// A seat's minions: ready (in play, uncontested, out of torpor), unlocked
/// among them, the blood on the ready ones, those in torpor (an
/// incapacitated imbued with them, as the observation reads it).
pub fn minions(st: &GameState, s: SeatId) -> [f32; 4] {
    let (mut ready, mut unlocked, mut blood, mut torpor) = (0.0, 0.0, 0.0, 0.0);
    for m in st.minions.iter().filter(|m| m.owner == s && m.present()) {
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
    [ready, unlocked, blood, torpor]
}

/// Seat `s`'s ready minions (`minions`'s), by id.
fn ready(st: &GameState, s: SeatId) -> impl Iterator<Item = zvtes::ids::MinionId> + '_ {
    st.minions_of(s).filter(|m| {
        let m = &st.minions[m.index()];
        !m.in_torpor && !m.incapacitated
    })
}

/// Seat `s`'s position (`POSITION`), scaled to about [0, 1]: the lunge is
/// its prey's pool within a bleed of one from each of its ready minions,
/// lunged its own pool within its predator's.
pub fn position(st: &GameState, s: SeatId) -> [f32; POSITION.len()] {
    let seat = &st.seats[s.index()];
    let (prey, pred) = (st.prey(s), st.predator(s));
    let own = minions(st, s);
    let of_pred = minions(st, pred);
    let prey_pool = st.seats[prey.index()].pool as f32;
    let capacity: f32 = ready(st, s)
        .map(|m| st.capacity_of(m).unwrap_or(0) as f32)
        .sum();
    let equipment: f32 = ready(st, s).map(|m| st.equipment_on(m).len() as f32).sum();
    [
        seat.pool as f32 / 30.0,
        seat.ousted as u8 as f32,
        seat.half_vps as f32 / 10.0,
        own[0] / 5.0,
        own[1] / 5.0,
        own[2] / 20.0,
        own[3] / 5.0,
        seat.hand.len() as f32 / 10.0,
        seat.library.len() as f32 / 60.0,
        seat.has_edge as u8 as f32,
        (!seat.ousted && prey != s && own[0] >= prey_pool) as u8 as f32,
        (!seat.ousted && pred != s && of_pred[0] >= seat.pool as f32) as u8 as f32,
        (prey_pool - own[0]) / 30.0,
        capacity / 30.0,
        equipment / 5.0,
    ]
}

/// The levels seat `s`'s ready vampires have in each of `DISCIPLINES`,
/// summed, over 6.
pub fn disciplines(st: &GameState, s: SeatId) -> [f32; DISCIPLINES.len()] {
    let mut out = [0.0; DISCIPLINES.len()];
    for m in ready(st, s) {
        for (x, d) in out.iter_mut().zip(DISCIPLINES) {
            *x += st.discipline_level(m, d) as f32 / 6.0;
        }
    }
    out
}

/// The context `viewer` reads (`LEN`, `names`' order): for each seat from
/// the viewer's, clockwise, present, its position, its Disciplines and its
/// profile per round (a round is a turn of every seat).
pub fn features(e: &Engine, viewer: SeatId) -> Vec<f32> {
    let st = &e.state;
    let n = st.seats.len();
    let rounds = (st.turn as f32 / n as f32).max(1.0);
    let mut out = vec![0.0; LEN];
    for k in 0..n.min(SLOTS) {
        let s = SeatId(((viewer.index() + k) % n) as u8);
        let profile = e.profiles[s.index()]
            .counts
            .map(|c| c as f32 / rounds / 3.0);
        let seat = std::iter::once(1.0)
            .chain(position(st, s))
            .chain(disciplines(st, s))
            .chain(profile);
        for (f, x) in seat.enumerate() {
            out[f * SLOTS + k] = x;
        }
    }
    out
}
