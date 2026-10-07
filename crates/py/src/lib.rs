//! `zvtes_py`: the engine for training (DESIGN §5 "Batched stepping").
//!
//! A `GamePool` holds N games, each waiting on one network's decision
//! (seats `net:k`: the learner, frozen snapshots — Python routes each row to
//! its network by the `agent` column). `observe` returns that batch as padded
//! arrays (`zvtes::obs`); `act` answers it, then advances every game in
//! parallel until its next network decision — seats played by a scripted bot
//! or `RandomLegal` are answered inside, and a finished game is reported and
//! replaced by a new one: the next spec `queue`d by Python (matchmaking), or
//! the pool's default table (seed and unspecified decks drawn from the pool's
//! own sequence, so a pool is replayable). A game past its
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
use zvtes::obs::{encode, encode_critic, Obs, Version, OPT_F, OPT_IDS, TOK_F, TOK_IDS};
use zvtes::politics;
use zvtes::rng::Rng;

mod human;

/// Who plays a seat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// A network answered from Python, by id (0: the learner).
    Net(u16),
    Random,
    Bot(Style),
}

fn kind(name: &str) -> PyResult<Kind> {
    Ok(match name {
        "policy" => Kind::Net(0),
        "random" => Kind::Random,
        "aggro" => Kind::Bot(Style::AggroBleed),
        "wall" => Kind::Bot(Style::Wall),
        _ => match name.strip_prefix("net:").and_then(|k| k.parse().ok()) {
            Some(k) => Kind::Net(k),
            None => return Err(PyValueError::new_err(format!("unknown seat kind {name:?}"))),
        },
    })
}

fn kind_name(k: Kind) -> String {
    match k {
        Kind::Net(k) => format!("net:{k}"),
        Kind::Random => "random".into(),
        Kind::Bot(Style::AggroBleed) => "aggro".into(),
        Kind::Bot(Style::Wall) => "wall".into(),
    }
}

/// A table Python asks for: seat kinds, and decks (drawn when absent).
struct Spec {
    kinds: Vec<Kind>,
    decks: Option<Vec<usize>>,
}

/// One finished game.
struct Episode {
    slot: usize,
    /// The game's number in start order (0 the first): the queued tables
    /// are the first ones, before the default table takes over.
    game: u64,
    seed: u64,
    time_limit: u32,
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
    number: u64,
    seed: u64,
    decks: Vec<usize>,
    kinds: Vec<Kind>,
    agents: Vec<Option<Box<dyn Agent + Send + Sync>>>,
    /// The encoding each seat's network reads (`Versions::of`).
    versions: Vec<Version>,
    /// The policy decision the game waits on, and its encoding.
    pending: Option<(DecisionRequest, Obs)>,
    /// The pending decision's offered options among the engine's, when
    /// the default politics masked some (`politics`): Python answers
    /// among the offered ones.
    offered: Option<Vec<usize>>,
    /// Network seats play the default politics (`zvtes::politics`):
    /// terms by rule (not sampled), masked options never offered.
    politics: bool,
    /// The pending decision's omniscient encoding (`critic` pools).
    critic: Option<Obs>,
}

impl Game {
    fn new(
        number: u64,
        seed: u64,
        time_limit: u32,
        decks: Vec<usize>,
        kinds: Vec<Kind>,
        versions: &Versions,
        politics: bool,
    ) -> Game {
        let list: Vec<Deck> = decks.iter().map(|&d| all_decks()[d]).collect();
        let agents = kinds
            .iter()
            .enumerate()
            .map(|(s, &k)| -> Option<Box<dyn Agent + Send + Sync>> {
                let seed = seed.wrapping_mul(31).wrapping_add(s as u64);
                match k {
                    Kind::Net(_) => None,
                    Kind::Random => Some(Box::new(RandomLegal::new(seed))),
                    Kind::Bot(style) => Some(Box::new(Scripted::new(style, seed))),
                }
            })
            .collect();
        let mut engine = Engine::with_decks(seed, &list);
        engine.time_limit = time_limit;
        let versions = kinds.iter().map(|&k| versions.of(k)).collect();
        Game {
            engine,
            number,
            seed,
            decks,
            kinds,
            agents,
            versions,
            pending: None,
            offered: None,
            politics,
            critic: None,
        }
    }

    /// Advance to the next policy decision; the finished game, if it
    /// ended (then `pending` is `None`).
    fn advance(
        &mut self,
        budget: u64,
        slot: usize,
        keep_log: bool,
        critic: bool,
    ) -> Option<Episode> {
        loop {
            let over = |e: &Engine, vps: Vec<u32>, truncated: bool| Episode {
                slot,
                game: self.number,
                time_limit: e.time_limit,
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
                // An engine bug: name the game so it replays (the engine's
                // own check is a debug assertion, off in this build).
                Step::Decision(req) if req.options.is_empty() => panic!(
                    "empty {:?} request: seed {} decks {:?} kinds {:?} log {:?}",
                    req.window, self.seed, self.decks, self.kinds, self.engine.log
                ),
                Step::Decision(req) => match &mut self.agents[req.seat.index()] {
                    Some(agent) => {
                        let i = agent.decide(&self.engine, &req);
                        self.engine.choose(i);
                    }
                    None => {
                        let mask = if self.politics {
                            if let Some(i) = politics::terms_choice(&self.engine, &req) {
                                self.engine.choose(i);
                                continue;
                            }
                            politics::mask(&self.engine, &req)
                        } else {
                            None
                        };
                        let (req, offered) = match mask {
                            Some(mask) => {
                                let offered: Vec<usize> =
                                    (0..mask.len()).filter(|&i| mask[i]).collect();
                                if offered.len() == 1 {
                                    self.engine.choose(offered[0]);
                                    continue;
                                }
                                let options = offered.iter().map(|&i| req.options[i]).collect();
                                (
                                    DecisionRequest::new(req.seat, req.window, options),
                                    Some(offered),
                                )
                            }
                            None => (req, None),
                        };
                        self.offered = offered;
                        let obs = encode(&self.engine, &req, self.versions[req.seat.index()]);
                        self.critic =
                            critic.then(|| encode_critic(&self.engine, &req, Version::V2));
                        self.pending = Some((req, obs));
                        return None;
                    }
                },
            }
        }
    }
}

/// Which encoding each network reads: `obs` for all, but the `legacy`
/// ones (frozen networks trained on `V1`).
struct Versions {
    obs: Version,
    legacy: Vec<u16>,
}

impl Versions {
    fn of(&self, k: Kind) -> Version {
        match k {
            Kind::Net(n) if !self.legacy.contains(&n) => self.obs,
            _ => Version::V1,
        }
    }
}

fn version(v: u8) -> PyResult<Version> {
    match v {
        1 => Ok(Version::V1),
        2 => Ok(Version::V2),
        _ => Err(PyValueError::new_err("obs: 1 or 2")),
    }
}

/// Deck indices grouped by clan — by path for the Sabbat V5 precons
/// (`Deck::category`) —, in first-seen order.
fn by_clan(pool: &[usize]) -> Vec<Vec<usize>> {
    let mut groups: Vec<(&str, Vec<usize>)> = Vec::new();
    for &d in pool {
        let clan = all_decks()[d].category();
        match groups.iter_mut().find(|(k, _)| *k == clan) {
            Some((_, g)) => g.push(d),
            None => groups.push((clan, vec![d])),
        }
    }
    groups.into_iter().map(|(_, g)| g).collect()
}

fn all_decks() -> &'static [Deck] {
    static DECKS: std::sync::OnceLock<Vec<Deck>> = std::sync::OnceLock::new();
    DECKS.get_or_init(|| Deck::all().collect())
}

/// N games stepped together. `seats`: one kind per seat ("policy",
/// "random", "aggro", "wall"); with `rotate`, game k seats them shifted by
/// k (the policy meets every position). `decks`: indices into the decklist
/// table (0–13 the precons, 14–27 the New Blood minis), drawn per seat
/// and game. `tables`: specs the first games take (as `queue`). Tables are
/// 4 or 5 seats: tournament tables only. `critic`: `observe` adds the
/// omniscient critic's tokens (`critic_tok_*`). `time_limits`: each game's time
/// limit is drawn in this range of player turns (a time-out scores 0.5 VP
/// per survivor, which counts toward the game win); episodes report it and
/// `replay` takes it. `obs`: the encoding the networks read
/// (`zvtes::obs::Version`, 1 or 2), but the `legacy` network ids, which
/// read `V1`; the critic reads `V2`. `politics`: network seats play the
/// default politics (`zvtes::politics`; DESIGN §11.2): referendum terms by
/// rule, never offered to Python; options the rule masks left out of the
/// request Python sees (its option indices are among those offered).
#[pyclass(module = "zvtes_py")]
struct GamePool {
    games: Vec<Game>,
    rng: Rng,
    kinds: Vec<Kind>,
    rotate: bool,
    /// The pool's decks (`decks`) grouped by clan (`Deck::clan`): a seat draws a clan,
    /// then a deck of it (owner, 2026-09-28: the TWDA is mostly Gangrel).
    by_clan: Vec<Vec<usize>>,
    budget: u64,
    keep_logs: bool,
    /// Also encode each pending decision for the omniscient critic.
    critic: bool,
    /// Network seats play the default politics (`Game::politics`).
    politics: bool,
    started: u64,
    /// Each game's time limit is drawn in this range, player turns
    /// inclusive (owner, 2026-09-28: 60–90).
    time_limits: (u32, u32),
    finished: Vec<Episode>,
    queued: std::collections::VecDeque<Spec>,
    versions: Versions,
}

impl GamePool {
    fn fresh(&mut self) -> Game {
        let seed = self.rng.next_u64();
        let (lo, hi) = self.time_limits;
        let time_limit = lo + self.rng.below((hi - lo + 1) as usize) as u32;
        let (kinds, decks) = match self.queued.pop_front() {
            Some(spec) => (spec.kinds, spec.decks),
            None => {
                let n = self.kinds.len();
                let shift = if self.rotate {
                    (self.started % n as u64) as usize
                } else {
                    0
                };
                ((0..n).map(|s| self.kinds[(s + shift) % n]).collect(), None)
            }
        };
        let decks = decks.unwrap_or_else(|| self.draw_decks(kinds.len()));
        self.started += 1;
        Game::new(
            self.started - 1,
            seed,
            time_limit,
            decks,
            kinds,
            &self.versions,
            self.politics,
        )
    }

    /// A deck: a clan uniformly, then one of its decks.
    fn draw_deck(&mut self) -> usize {
        let clan = &self.by_clan[self.rng.below(self.by_clan.len())];
        clan[self.rng.below(clan.len())]
    }

    /// A table's decks, drawn from the pool seat by seat (`draw_deck`): a deck sharing
    /// more than 20% of its crypt with one already seated is redrawn
    /// (owner, 2026-09-28: crippling contests; the same deck twice shares
    /// all of it), up to a few tries — a small pool may have to accept one.
    fn draw_decks(&mut self, n: usize) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::with_capacity(n);
        for _ in 0..n {
            let mut pick = self.draw_deck();
            for _ in 0..32 {
                let clash = out
                    .iter()
                    .any(|&d| all_decks()[d].crypt_share(&all_decks()[pick]) > 0.2);
                if !clash {
                    break;
                }
                pick = self.draw_deck();
            }
            out.push(pick);
        }
        out
    }

    fn spec(&self, seats: Vec<String>, decks: Option<Vec<usize>>) -> PyResult<Spec> {
        if !(4..=5).contains(&seats.len()) {
            return Err(PyValueError::new_err("4 or 5 seats (tournament tables)"));
        }
        let kinds = seats
            .iter()
            .map(|s| kind(s))
            .collect::<PyResult<Vec<_>>>()?;
        if let Some(d) = &decks {
            if d.len() != kinds.len() || d.iter().any(|&d| d >= all_decks().len()) {
                return Err(PyValueError::new_err("one deck index in 0..28 per seat"));
            }
        }
        Ok(Spec { kinds, decks })
    }

    /// Every game to its next policy decision, in parallel; finished games
    /// replaced until each waits on one.
    fn settle(&mut self, py: Python<'_>) {
        loop {
            let (budget, keep, critic) = (self.budget, self.keep_logs, self.critic);
            let games = &mut self.games;
            let done: Vec<Episode> = py.detach(|| {
                games
                    .par_iter_mut()
                    .enumerate()
                    .filter(|(_, g)| g.pending.is_none())
                    .filter_map(|(i, g)| g.advance(budget, i, keep, critic))
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
    #[pyo3(signature = (n_games, seats, seed=0, rotate=false, decks=None, budget=20_000, keep_logs=false, tables=None, time_limits=(60, 90), critic=false, obs=1, legacy=Vec::new(), politics=false))]
    fn new(
        py: Python<'_>,
        n_games: usize,
        seats: Vec<String>,
        seed: u64,
        rotate: bool,
        decks: Option<Vec<usize>>,
        budget: u64,
        keep_logs: bool,
        tables: Option<Vec<Bound<'_, PyDict>>>,
        time_limits: (u32, u32),
        critic: bool,
        obs: u8,
        legacy: Vec<u16>,
        politics: bool,
    ) -> PyResult<Self> {
        if time_limits.0 == 0 || time_limits.0 > time_limits.1 {
            return Err(PyValueError::new_err(
                "time_limits: (low, high), 0 < low <= high",
            ));
        }
        if !(4..=5).contains(&seats.len()) {
            return Err(PyValueError::new_err("4 or 5 seats (tournament tables)"));
        }
        let kinds = seats
            .iter()
            .map(|s| kind(s))
            .collect::<PyResult<Vec<_>>>()?;
        if !kinds.iter().any(|k| matches!(k, Kind::Net(_))) {
            return Err(PyValueError::new_err("no network seat"));
        }
        let deck_pool = decks.unwrap_or_else(|| (0..14).collect());
        if deck_pool.is_empty() || deck_pool.iter().any(|&d| d >= all_decks().len()) {
            return Err(PyValueError::new_err(format!(
                "deck indices in 0..{}",
                all_decks().len()
            )));
        }
        let mut pool = GamePool {
            games: Vec::with_capacity(n_games),
            rng: Rng::new(seed),
            kinds,
            rotate,
            by_clan: by_clan(&deck_pool),
            budget,
            keep_logs,
            critic,
            politics,
            started: 0,
            time_limits,
            finished: Vec::new(),
            queued: Default::default(),
            versions: Versions {
                obs: version(obs)?,
                legacy,
            },
        };
        if let Some(tables) = tables {
            pool.queue(tables)?;
        }
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
    /// [B, O, G] f32, `opt_n` [B], `seat` [B] (the deciding seat), `agent`
    /// [B] (its network id), `window` [B].
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
        let agent: Vec<i32> = self
            .games
            .iter()
            .map(|g| {
                let (r, _) = g.pending.as_ref().expect("a pending decision");
                match g.kinds[r.seat.index()] {
                    Kind::Net(k) => k as i32,
                    _ => unreachable!("only networks' decisions are pending"),
                }
            })
            .collect();
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
        d.set_item("agent", PyArray1::from_vec(py, agent))?;
        d.set_item("window", PyArray1::from_vec(py, window))?;
        if self.critic {
            let obs: Vec<&Obs> = self
                .games
                .iter()
                .map(|g| g.critic.as_ref().expect("a critic encoding"))
                .collect();
            let t = obs.iter().map(|o| o.tok_ids.len()).max().unwrap_or(0);
            let mut ids = vec![0i32; b * t * TOK_IDS];
            let mut f = vec![0f32; b * t * TOK_F];
            let mut n = Vec::with_capacity(b);
            for (i, o) in obs.iter().enumerate() {
                for (k, row) in o.tok_ids.iter().enumerate() {
                    let at = (i * t + k) * TOK_IDS;
                    ids[at..at + TOK_IDS].copy_from_slice(row);
                }
                for (k, row) in o.tok_f.iter().enumerate() {
                    let at = (i * t + k) * TOK_F;
                    f[at..at + TOK_F].copy_from_slice(row);
                }
                n.push(o.tok_ids.len() as i32);
            }
            d.set_item(
                "critic_tok_ids",
                PyArray1::from_vec(py, ids).reshape([b, t, TOK_IDS])?,
            )?;
            d.set_item(
                "critic_tok_f",
                PyArray1::from_vec(py, f).reshape([b, t, TOK_F])?,
            )?;
            d.set_item("critic_tok_n", PyArray1::from_vec(py, n))?;
        }
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
            let a = g.offered.take().map_or(a, |o| o[a]);
            g.engine.choose(a);
        }
        self.settle(py);
        std::mem::take(&mut self.finished)
            .into_iter()
            .map(|ep| {
                let d = PyDict::new(py);
                d.set_item("slot", ep.slot)?;
                d.set_item("game", ep.game)?;
                d.set_item("seed", ep.seed)?;
                d.set_item("time_limit", ep.time_limit)?;
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

    /// Replace the queue of tables the next new games take, in order:
    /// dicts with `seats` (kinds: "net:k", "random", "aggro", "wall"; 4 or
    /// 5) and optionally `decks` (one index per seat). When it runs out, new
    /// games take the pool's default table.
    fn queue(&mut self, specs: Vec<Bound<'_, PyDict>>) -> PyResult<()> {
        let mut queued = std::collections::VecDeque::new();
        for d in specs {
            let seats: Vec<String> = d
                .get_item("seats")?
                .ok_or_else(|| PyValueError::new_err("a spec needs seats"))?
                .extract()?;
            let decks: Option<Vec<usize>> = match d.get_item("decks")? {
                Some(v) => Some(v.extract()?),
                None => None,
            };
            queued.push_back(self.spec(seats, decks)?);
        }
        self.queued = queued;
        Ok(())
    }

    #[getter]
    fn n_games(&self) -> usize {
        self.games.len()
    }
}

/// Replay a game from its seed, decks and decision log: the final
/// half-VPs per seat (the boundary's determinism check).
#[pyfunction]
#[pyo3(signature = (seed, decks, log, time_limit=zvtes::game::TURN_LIMIT))]
fn replay(seed: u64, decks: Vec<usize>, log: Vec<usize>, time_limit: u32) -> PyResult<Vec<u32>> {
    let list: Vec<Deck> = decks.iter().map(|&d| all_decks()[d]).collect();
    let mut e = Engine::with_decks(seed, &list);
    e.time_limit = time_limit;
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

/// The vocabulary's stable keys, by id (`zvtes::obs::vocab_keys`).
#[pyfunction]
fn vocab_keys() -> Vec<String> {
    zvtes::obs::vocab_keys()
}

/// The decklist families: name -> deck indices ("precons", "minis",
/// "anniversary", "sabbat", "twda": `Deck::all` order).
#[pyfunction]
fn deck_families() -> Vec<(&'static str, Vec<usize>)> {
    use zvtes::crypt::{ANNIVERSARY_PRECONS, MINI_PRECONS, PRECONS, SABBAT_PRECONS};
    let (p, m, a) = (PRECONS.len(), MINI_PRECONS.len(), ANNIVERSARY_PRECONS.len());
    let b = SABBAT_PRECONS.len();
    vec![
        ("precons", (0..p).collect()),
        ("minis", (p..p + m).collect()),
        ("anniversary", (p + m..p + m + a).collect()),
        ("sabbat", (p + m + a..p + m + a + b).collect()),
        ("twda", (p + m + a + b..all_decks().len()).collect()),
    ]
}

/// The exported policy's forward pass on one observation (`zvtes-policy`,
/// what plays outside Python): the options' logits and the value. `weights`
/// is a `.zvp` file (`train/export.py`); the test of their agreement with
/// `train/model.py` reads this.
#[pyfunction]
fn policy_forward(
    weights: &[u8],
    tok_ids: Vec<[i32; TOK_IDS]>,
    tok_f: Vec<[f32; TOK_F]>,
    opt_ids: Vec<[i32; OPT_IDS]>,
    opt_f: Vec<[f32; OPT_F]>,
) -> PyResult<(Vec<f32>, f32)> {
    let net = zvtes_policy::Net::load(weights).map_err(PyValueError::new_err)?;
    Ok(net.forward(&Obs {
        tok_ids,
        tok_f,
        opt_ids,
        opt_f,
    }))
}

#[pymodule]
fn zvtes_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<GamePool>()?;
    m.add_function(wrap_pyfunction!(replay, m)?)?;
    m.add_function(wrap_pyfunction!(schema, m)?)?;
    m.add_function(wrap_pyfunction!(deck_names, m)?)?;
    m.add_function(wrap_pyfunction!(vocab_keys, m)?)?;
    m.add_function(wrap_pyfunction!(deck_families, m)?)?;
    m.add_function(wrap_pyfunction!(policy_forward, m)?)?;
    m.add_function(wrap_pyfunction!(human::replay_decisions, m)?)?;
    m.add_function(wrap_pyfunction!(human::deck_cards, m)?)?;
    Ok(())
}
