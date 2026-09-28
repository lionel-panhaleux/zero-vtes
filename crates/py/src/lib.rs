//! `zvtes_py`: the engine for training (DESIGN §5 "Batched stepping").
//!
//! A `GamePool` holds N games, each waiting on one policy decision. `observe`
//! returns that batch as padded arrays (`zvtes::obs`); `act` answers it, then
//! advances every game in parallel until its next policy decision — seats
//! played by a scripted bot or `RandomLegal` are answered inside, and a
//! finished game is reported and replaced by a new one (seed and decks drawn
//! from the pool's own sequence, so a pool is replayable). A game past its
//! decision budget is cut (`truncated`): the rulebook allows unbounded
//! repeats (DESIGN §11.3) and an untrained policy finds them.

use numpy::{PyArray1, PyArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rayon::prelude::*;
use zvtes::agent::{Agent, RandomLegal};
use zvtes::bots::{Scripted, Style};
use zvtes::decision::DecisionRequest;
use zvtes::decks::Deck;
use zvtes::game::{Engine, Step};
use zvtes::obs::{encode, Obs, OPT_F, OPT_IDS, TOK_F, TOK_IDS};
use zvtes::rng::Rng;

/// Who plays a seat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Policy,
    Random,
    Bot(Style),
}

fn kind(name: &str) -> PyResult<Kind> {
    Ok(match name {
        "policy" => Kind::Policy,
        "random" => Kind::Random,
        "aggro" => Kind::Bot(Style::AggroBleed),
        "wall" => Kind::Bot(Style::Wall),
        _ => return Err(PyValueError::new_err(format!("unknown seat kind {name:?}"))),
    })
}

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Policy => "policy",
        Kind::Random => "random",
        Kind::Bot(Style::AggroBleed) => "aggro",
        Kind::Bot(Style::Wall) => "wall",
    }
}

/// One finished game.
struct Episode {
    slot: usize,
    seed: u64,
    decks: Vec<usize>,
    kinds: Vec<Kind>,
    half_vps: Vec<u32>,
    turns: u32,
    decisions: u64,
    truncated: bool,
    log: Option<Vec<usize>>,
}

struct Game {
    engine: Engine,
    seed: u64,
    decks: Vec<usize>,
    kinds: Vec<Kind>,
    agents: Vec<Option<Box<dyn Agent + Send + Sync>>>,
    /// The policy decision the game waits on, and its encoding.
    pending: Option<(DecisionRequest, Obs)>,
}

impl Game {
    fn new(seed: u64, decks: Vec<usize>, kinds: Vec<Kind>) -> Game {
        let list: Vec<Deck> = decks.iter().map(|&d| all_decks()[d]).collect();
        let agents = kinds
            .iter()
            .enumerate()
            .map(|(s, &k)| -> Option<Box<dyn Agent + Send + Sync>> {
                let seed = seed.wrapping_mul(31).wrapping_add(s as u64);
                match k {
                    Kind::Policy => None,
                    Kind::Random => Some(Box::new(RandomLegal::new(seed))),
                    Kind::Bot(style) => Some(Box::new(Scripted::new(style, seed))),
                }
            })
            .collect();
        Game {
            engine: Engine::with_decks(seed, &list),
            seed,
            decks,
            kinds,
            agents,
            pending: None,
        }
    }

    /// Advance to the next policy decision; the finished game, if it
    /// ended (then `pending` is `None`).
    fn advance(&mut self, budget: u64, slot: usize, keep_log: bool) -> Option<Episode> {
        loop {
            let over = |e: &Engine, vps: Vec<u32>, truncated: bool| Episode {
                slot,
                seed: self.seed,
                decks: self.decks.clone(),
                kinds: self.kinds.clone(),
                half_vps: vps,
                turns: e.state.turn,
                decisions: e.decisions,
                truncated,
                log: keep_log.then(|| e.log.clone()),
            };
            if self.engine.decisions >= budget {
                let vps = self.engine.state.seats.iter().map(|s| s.half_vps).collect();
                return Some(over(&self.engine, vps, true));
            }
            match self.engine.run() {
                Step::Over(vps) => return Some(over(&self.engine, vps, false)),
                Step::Decision(req) => match &mut self.agents[req.seat.index()] {
                    Some(agent) => {
                        let i = agent.decide(&self.engine, &req);
                        self.engine.choose(i);
                    }
                    None => {
                        let obs = encode(&self.engine, &req);
                        self.pending = Some((req, obs));
                        return None;
                    }
                },
            }
        }
    }
}

fn all_decks() -> &'static [Deck] {
    static DECKS: std::sync::OnceLock<Vec<Deck>> = std::sync::OnceLock::new();
    DECKS.get_or_init(|| Deck::all().collect())
}

/// N games stepped together. `seats`: one kind per seat ("policy",
/// "random", "aggro", "wall"); with `rotate`, game k seats them shifted by
/// k (the policy meets every position). `decks`: indices into the decklist
/// table (0–13 the precons, 14–27 the New Blood minis), drawn per seat
/// and game.
#[pyclass(module = "zvtes_py")]
struct GamePool {
    games: Vec<Game>,
    rng: Rng,
    kinds: Vec<Kind>,
    rotate: bool,
    deck_pool: Vec<usize>,
    budget: u64,
    keep_logs: bool,
    started: u64,
    finished: Vec<Episode>,
}

impl GamePool {
    fn fresh(&mut self) -> Game {
        let seed = self.rng.next_u64();
        let n = self.kinds.len();
        let decks = (0..n)
            .map(|_| self.deck_pool[self.rng.below(self.deck_pool.len())])
            .collect();
        let shift = if self.rotate {
            (self.started % n as u64) as usize
        } else {
            0
        };
        let kinds = (0..n).map(|s| self.kinds[(s + shift) % n]).collect();
        self.started += 1;
        Game::new(seed, decks, kinds)
    }

    /// Every game to its next policy decision, in parallel; finished games
    /// replaced until each waits on one.
    fn settle(&mut self, py: Python<'_>) {
        loop {
            let (budget, keep) = (self.budget, self.keep_logs);
            let games = &mut self.games;
            let done: Vec<Episode> = py.detach(|| {
                games
                    .par_iter_mut()
                    .enumerate()
                    .filter(|(_, g)| g.pending.is_none())
                    .filter_map(|(i, g)| g.advance(budget, i, keep))
                    .collect()
            });
            if done.is_empty() {
                return;
            }
            for ep in done {
                let slot = ep.slot;
                self.finished.push(ep);
                self.games[slot] = self.fresh();
            }
        }
    }
}

#[pymethods]
impl GamePool {
    // A Python constructor with keyword arguments.
    #[allow(clippy::too_many_arguments)]
    #[new]
    #[pyo3(signature = (n_games, seats, seed=0, rotate=false, decks=None, budget=20_000, keep_logs=false))]
    fn new(
        py: Python<'_>,
        n_games: usize,
        seats: Vec<String>,
        seed: u64,
        rotate: bool,
        decks: Option<Vec<usize>>,
        budget: u64,
        keep_logs: bool,
    ) -> PyResult<Self> {
        if !(2..=6).contains(&seats.len()) {
            return Err(PyValueError::new_err("2 to 6 seats"));
        }
        let kinds = seats
            .iter()
            .map(|s| kind(s))
            .collect::<PyResult<Vec<_>>>()?;
        if !kinds.contains(&Kind::Policy) {
            return Err(PyValueError::new_err("no policy seat"));
        }
        let deck_pool = decks.unwrap_or_else(|| (0..14).collect());
        if deck_pool.is_empty() || deck_pool.iter().any(|&d| d >= all_decks().len()) {
            return Err(PyValueError::new_err("deck indices in 0..28"));
        }
        let mut pool = GamePool {
            games: Vec::with_capacity(n_games),
            rng: Rng::new(seed),
            kinds,
            rotate,
            deck_pool,
            budget,
            keep_logs,
            started: 0,
            finished: Vec::new(),
        };
        for _ in 0..n_games {
            let g = pool.fresh();
            pool.games.push(g);
        }
        pool.settle(py);
        Ok(pool)
    }

    /// The pending decisions, one per game, padded to the batch's largest
    /// token and option counts: a dict of arrays `tok_ids` [B, T, 4] i32,
    /// `tok_f` [B, T, F] f32, `tok_n` [B], `opt_ids` [B, O, 5] i32, `opt_f`
    /// [B, O, G] f32, `opt_n` [B], `seat` [B] (the deciding seat), `window`
    /// [B].
    fn observe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let obs: Vec<(&DecisionRequest, &Obs)> = self
            .games
            .iter()
            .map(|g| {
                let (r, o) = g.pending.as_ref().expect("a pending decision");
                (r, o)
            })
            .collect();
        let b = obs.len();
        let t = obs.iter().map(|(_, o)| o.tok_ids.len()).max().unwrap_or(0);
        let n = obs.iter().map(|(_, o)| o.opt_ids.len()).max().unwrap_or(0);
        let mut tok_ids = vec![0i32; b * t * TOK_IDS];
        let mut tok_f = vec![0f32; b * t * TOK_F];
        let mut opt_ids = vec![0i32; b * n * OPT_IDS];
        let mut opt_f = vec![0f32; b * n * OPT_F];
        let (mut tok_n, mut opt_n, mut seat, mut window) = (vec![], vec![], vec![], vec![]);
        for (i, (r, o)) in obs.iter().enumerate() {
            for (k, row) in o.tok_ids.iter().enumerate() {
                let at = (i * t + k) * TOK_IDS;
                tok_ids[at..at + TOK_IDS].copy_from_slice(row);
            }
            for (k, row) in o.tok_f.iter().enumerate() {
                let at = (i * t + k) * TOK_F;
                tok_f[at..at + TOK_F].copy_from_slice(row);
            }
            for (k, row) in o.opt_ids.iter().enumerate() {
                let at = (i * n + k) * OPT_IDS;
                opt_ids[at..at + OPT_IDS].copy_from_slice(row);
            }
            for (k, row) in o.opt_f.iter().enumerate() {
                let at = (i * n + k) * OPT_F;
                opt_f[at..at + OPT_F].copy_from_slice(row);
            }
            tok_n.push(o.tok_ids.len() as i32);
            opt_n.push(o.opt_ids.len() as i32);
            seat.push(r.seat.0 as i32);
            window.push(r.window as i32);
        }
        let d = PyDict::new(py);
        d.set_item(
            "tok_ids",
            PyArray1::from_vec(py, tok_ids).reshape([b, t, TOK_IDS])?,
        )?;
        d.set_item(
            "tok_f",
            PyArray1::from_vec(py, tok_f).reshape([b, t, TOK_F])?,
        )?;
        d.set_item("tok_n", PyArray1::from_vec(py, tok_n))?;
        d.set_item(
            "opt_ids",
            PyArray1::from_vec(py, opt_ids).reshape([b, n, OPT_IDS])?,
        )?;
        d.set_item(
            "opt_f",
            PyArray1::from_vec(py, opt_f).reshape([b, n, OPT_F])?,
        )?;
        d.set_item("opt_n", PyArray1::from_vec(py, opt_n))?;
        d.set_item("seat", PyArray1::from_vec(py, seat))?;
        d.set_item("window", PyArray1::from_vec(py, window))?;
        Ok(d)
    }

    /// Answer every pending decision (one option index per game), advance,
    /// and return the games finished since the last call: dicts with
    /// `slot`, `seed`, `decks`, `seats`, `half_vps`, `turns`, `decisions`,
    /// `truncated` (and `log` with `keep_logs`). A finished slot already
    /// holds its next game.
    fn act<'py>(
        &mut self,
        py: Python<'py>,
        actions: Vec<usize>,
    ) -> PyResult<Vec<Bound<'py, PyDict>>> {
        if actions.len() != self.games.len() {
            return Err(PyValueError::new_err("one action per game"));
        }
        for (g, &a) in self.games.iter_mut().zip(&actions) {
            let (req, _) = g.pending.take().expect("a pending decision");
            if a >= req.options.len() {
                return Err(PyValueError::new_err(format!(
                    "action {a} of {} options",
                    req.options.len()
                )));
            }
            g.engine.choose(a);
        }
        self.settle(py);
        std::mem::take(&mut self.finished)
            .into_iter()
            .map(|ep| {
                let d = PyDict::new(py);
                d.set_item("slot", ep.slot)?;
                d.set_item("seed", ep.seed)?;
                d.set_item("decks", ep.decks)?;
                d.set_item(
                    "seats",
                    ep.kinds.iter().map(|&k| kind_name(k)).collect::<Vec<_>>(),
                )?;
                d.set_item("half_vps", ep.half_vps)?;
                d.set_item("turns", ep.turns)?;
                d.set_item("decisions", ep.decisions)?;
                d.set_item("truncated", ep.truncated)?;
                if let Some(log) = ep.log {
                    d.set_item("log", log)?;
                }
                Ok(d)
            })
            .collect()
    }

    #[getter]
    fn n_games(&self) -> usize {
        self.games.len()
    }
}

/// Replay a game from its seed, decks and decision log: the final
/// half-VPs per seat (the boundary's determinism check).
#[pyfunction]
fn replay(seed: u64, decks: Vec<usize>, log: Vec<usize>) -> PyResult<Vec<u32>> {
    let list: Vec<Deck> = decks.iter().map(|&d| all_decks()[d]).collect();
    let mut e = Engine::with_decks(seed, &list);
    let mut moves = log.into_iter();
    loop {
        match e.run() {
            Step::Over(vps) => return Ok(vps),
            Step::Decision(_) => match moves.next() {
                Some(i) => e.choose(i),
                None => return Ok(e.state.seats.iter().map(|s| s.half_vps).collect()),
            },
        }
    }
}

/// The encoding's widths and counts, as JSON.
#[pyfunction]
fn schema() -> String {
    zvtes::obs::schema_json()
}

/// The decklist names, by index.
#[pyfunction]
fn deck_names() -> Vec<&'static str> {
    all_decks().iter().map(|d| d.name).collect()
}

#[pymodule]
fn zvtes_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<GamePool>()?;
    m.add_function(wrap_pyfunction!(replay, m)?)?;
    m.add_function(wrap_pyfunction!(schema, m)?)?;
    m.add_function(wrap_pyfunction!(deck_names, m)?)?;
    Ok(())
}
