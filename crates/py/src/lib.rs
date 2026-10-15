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
use zvtes::decision::{Choice, DecisionRequest};
use zvtes::decks::Deck;
use zvtes::game::{Engine, Step};
use zvtes::library::{CardType, LIBRARY};
use zvtes::obs::{encode, encode_critic, Obs, Version, OPT_F, OPT_IDS, TOK_F, TOK_IDS};
use zvtes::rng::Rng;
use zvtes::{habits, politics};
use zvtes_host::start::Start;
use zvtes_policy::Nudges;

mod human;

/// Who plays a seat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// A network answered from Python, by id (0: the learner); with
    /// the punishing habits (`zvtes::habits`) when flagged (`net:k+habits`).
    Net(u16, bool),
    Random,
    Bot(Style),
}

fn kind(name: &str) -> PyResult<Kind> {
    Ok(match name {
        "policy" => Kind::Net(0, false),
        "random" => Kind::Random,
        "aggro" => Kind::Bot(Style::AggroBleed),
        "wall" => Kind::Bot(Style::Wall),
        _ => match name.strip_prefix("net:").and_then(|k| {
            let (k, habits) = match k.strip_suffix("+habits") {
                Some(k) => (k, true),
                None => (k, false),
            };
            k.parse().ok().map(|k| (k, habits))
        }) {
            Some((k, habits)) => Kind::Net(k, habits),
            None => return Err(PyValueError::new_err(format!("unknown seat kind {name:?}"))),
        },
    })
}

fn kind_name(k: Kind) -> String {
    match k {
        Kind::Net(k, false) => format!("net:{k}"),
        Kind::Net(k, true) => format!("net:{k}+habits"),
        Kind::Random => "random".into(),
        Kind::Bot(Style::AggroBleed) => "aggro".into(),
        Kind::Bot(Style::Wall) => "wall".into(),
    }
}

/// A table Python asks for: seat kinds, and decks (a seat's drawn when
/// absent).
struct Spec {
    kinds: Vec<Kind>,
    decks: Option<Vec<Option<usize>>>,
    /// A start (`starts`): the game replayed to one of its decisions, its
    /// decks the replay's.
    start: Option<usize>,
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
    /// The game began from a start: (start index, decision).
    start: Option<(usize, usize)>,
    /// The engine panicked (`Game::advance_caught`): its message.
    panic: Option<String>,
}

struct Game {
    engine: Engine,
    /// Begun from a start (`Episode::start`).
    start: Option<(usize, usize)>,
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
    /// The habits of the `net:k+habits` seats.
    habits: habits::Set,
    /// Network seats never decline a chore (`zvtes_policy::chores`).
    chores: bool,
    /// The pending decision's omniscient encoding (`critic` pools).
    critic: Option<Obs>,
    /// The engine panicked on the policy's answer (`GamePool::act`): the
    /// game ends at the next `advance`, as a `panic` episode.
    crashed: Option<String>,
}

/// A caught panic's message.
fn panic_message(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| e.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
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
        let mut engine = Engine::with_decks(seed, &list);
        engine.time_limit = time_limit;
        Game::with_engine(number, seed, engine, decks, kinds, versions, politics)
    }

    /// A game on an engine already dealt (a start's position); `decks`
    /// empty for a start.
    fn with_engine(
        number: u64,
        seed: u64,
        engine: Engine,
        decks: Vec<usize>,
        kinds: Vec<Kind>,
        versions: &Versions,
        politics: bool,
    ) -> Game {
        let agents = kinds
            .iter()
            .enumerate()
            .map(|(s, &k)| -> Option<Box<dyn Agent + Send + Sync>> {
                let seed = seed.wrapping_mul(31).wrapping_add(s as u64);
                match k {
                    Kind::Net(..) => None,
                    Kind::Random => Some(Box::new(RandomLegal::new(seed))),
                    Kind::Bot(style) => Some(Box::new(Scripted::new(style, seed))),
                }
            })
            .collect();
        let versions = kinds.iter().map(|&k| versions.of(k)).collect();
        Game {
            engine,
            start: None,
            number,
            seed,
            decks,
            kinds,
            agents,
            versions,
            pending: None,
            offered: None,
            politics,
            habits: habits::Set::ALL,
            chores: false,
            critic: None,
            crashed: None,
        }
    }

    /// `advance`, an engine panic caught (owner's p9 run, 2026-10-10: one
    /// game's panic stopped a 40-hour run): the game ends as a truncated
    /// `panic` episode, named on stderr (seed, decks, seats, start, log)
    /// so it replays.
    fn advance_caught(
        &mut self,
        budget: u64,
        slot: usize,
        keep_log: bool,
        critic: bool,
    ) -> Option<Episode> {
        let message = match self.crashed.take() {
            Some(m) => m,
            None => match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.advance(budget, slot, keep_log, critic)
            })) {
                Ok(ep) => return ep,
                Err(e) => panic_message(&*e),
            },
        };
        self.pending = None;
        eprintln!(
            "engine panic: {message:?} seed {} time_limit {} decks {:?} seats {:?} start {:?} log {:?}",
            self.seed,
            self.engine.time_limit,
            self.decks,
            self.kinds.iter().map(|&k| kind_name(k)).collect::<Vec<_>>(),
            self.start,
            self.engine.log
        );
        let e = &self.engine;
        Some(Episode {
            slot,
            game: self.number,
            time_limit: e.time_limit,
            seed: self.seed,
            decks: self.decks.clone(),
            kinds: self.kinds.clone(),
            half_vps: e.state.seats.iter().map(|s| s.half_vps).collect(),
            turns: e.state.turn,
            decisions: e.decisions,
            truncated: true,
            log: keep_log.then(|| e.log.clone()),
            start: self.start,
            panic: Some(message),
        })
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
                start: self.start,
                panic: None,
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
                        let habits = matches!(self.kinds[req.seat.index()], Kind::Net(_, true));
                        if habits {
                            if let Some(i) = habits::choice(&self.engine, &req, self.habits) {
                                self.engine.choose(i);
                                continue;
                            }
                        }
                        let mask = if self.politics {
                            if let Some(i) = politics::terms_choice(&self.engine, &req) {
                                self.engine.choose(i);
                                continue;
                            }
                            politics::mask(&self.engine, &req)
                        } else {
                            None
                        };
                        let mask = zvtes_policy::combine(
                            mask,
                            habits
                                .then(|| habits::mask(&self.engine, &req, self.habits))
                                .flatten(),
                        );
                        // The chores (`zvtes_policy::chores`): never
                        // declined, as the politics rule.
                        let mask = zvtes_policy::combine(
                            mask,
                            self.chores
                                .then(|| zvtes_policy::chores::mask(&self.engine, &req))
                                .flatten(),
                        );
                        // Never a futile block attempt, as the server's bots
                        // (`zvtes_policy::reachable_blocks`): the outermost.
                        let mask = zvtes_policy::combine(
                            zvtes_policy::reachable_blocks(&self.engine, &req),
                            mask,
                        );
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
                        // The critic reads `V2`, lighter (`V3`) for a
                        // learner reading it.
                        let cv = if self.versions.contains(&Version::V3) {
                            Version::V3
                        } else {
                            Version::V2
                        };
                        self.critic = critic.then(|| encode_critic(&self.engine, &req, cv));
                        self.pending = Some((req, obs));
                        return None;
                    }
                },
            }
        }
    }
}

/// Token rows of each observation (`rows`: ids and features), padded to
/// the longest: `{name}_ids` [B, T, 4] i32, `{name}_f` [B, T, F] f32,
/// `{name}_n` [B].
#[allow(clippy::type_complexity)]
fn padded(
    py: Python<'_>,
    d: &Bound<'_, PyDict>,
    name: &str,
    obs: &[&Obs],
    rows: fn(&Obs) -> (&Vec<[i32; TOK_IDS]>, &Vec<[f32; TOK_F]>),
) -> PyResult<()> {
    let b = obs.len();
    let t = obs.iter().map(|o| rows(o).0.len()).max().unwrap_or(0);
    let mut ids = vec![0i32; b * t * TOK_IDS];
    let mut f = vec![0f32; b * t * TOK_F];
    let mut n = Vec::with_capacity(b);
    for (i, o) in obs.iter().enumerate() {
        let (oi, of) = rows(o);
        for (k, row) in oi.iter().enumerate() {
            let at = (i * t + k) * TOK_IDS;
            ids[at..at + TOK_IDS].copy_from_slice(row);
        }
        for (k, row) in of.iter().enumerate() {
            let at = (i * t + k) * TOK_F;
            f[at..at + TOK_F].copy_from_slice(row);
        }
        n.push(oi.len() as i32);
    }
    d.set_item(
        format!("{name}_ids"),
        PyArray1::from_vec(py, ids).reshape([b, t, TOK_IDS])?,
    )?;
    d.set_item(
        format!("{name}_f"),
        PyArray1::from_vec(py, f).reshape([b, t, TOK_F])?,
    )?;
    d.set_item(format!("{name}_n"), PyArray1::from_vec(py, n))?;
    Ok(())
}

/// Which encoding each network reads: `obs` for all, but the `net_obs`
/// ones (frozen networks: the version each was trained on) and the
/// `legacy` ones (`V1`).
struct Versions {
    obs: Version,
    legacy: Vec<u16>,
    net_obs: Vec<(u16, Version)>,
}

impl Versions {
    fn of(&self, k: Kind) -> Version {
        match k {
            Kind::Net(n, _) => match self.net_obs.iter().find(|&&(m, _)| m == n) {
                Some(&(_, v)) => v,
                None if self.legacy.contains(&n) => Version::V1,
                None => self.obs,
            },
            _ => Version::V1,
        }
    }
}

fn version(v: u8) -> PyResult<Version> {
    match v {
        1 => Ok(Version::V1),
        2 => Ok(Version::V2),
        3 => Ok(Version::V3),
        _ => Err(PyValueError::new_err("obs: 1, 2 or 3")),
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
/// (`zvtes::obs::Version`, 1 to 3), but the `net_obs` network ids (id,
/// version), which read their own, and the `legacy` ones, which read `V1`;
/// the critic reads `V2`, or `V3` when a network does. `politics`: network seats play the
/// default politics (`zvtes::politics`; DESIGN §11.2): referendum terms by
/// rule, never offered to Python; options the rule masks left out of the
/// request Python sees (its option indices are among those offered).
/// `replays`: the games a queued table may `start` from (game records,
/// `zvtes_protocol::start`). `habits`: the habits of the `net:k+habits`
/// seats (`zvtes::habits::Set::parse`), all by default. `chores`: network
/// seats never decline a chore (`zvtes_policy::chores`: the Edge's pool, a
/// hunting ground's blood). Without `refill`,
/// a game finished once the queue is empty leaves the pool instead of the
/// default table taking its slot (an evaluation: no throwaway games in the
/// batches; the pool ends empty).
#[pyclass(module = "zvtes_py")]
struct GamePool {
    games: Vec<Game>,
    rng: Rng,
    kinds: Vec<Kind>,
    rotate: bool,
    refill: bool,
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
    /// The games a queued table may start from (`replays`).
    starts: Vec<Start>,
    /// The habits of the `net:k+habits` seats (`habits`).
    habits: habits::Set,
    /// Network seats never decline a chore (`chores`).
    chores: bool,
    /// Added to every network seat's logits (`opt_bias`; owner,
    /// 2026-10-05: encouraged, not forced): `zvtes_policy::Nudges`.
    nudges: Nudges,
    /// The learner's minion-phase decisions offering each habit
    /// (`zvtes::habits::Habit`: rescue, burn, diablerie) and those taking
    /// it, since the last `habits()`.
    habit_counts: [(u64, u64); 3],
    /// The learner's discard phases with a card to cycle: (those, those
    /// passed), (those holding a reaction to cycle, those cycling one),
    /// since the last `habits()`.
    discard_counts: [(u64, u64); 2],
}

impl GamePool {
    fn fresh(&mut self) -> Game {
        let seed = self.rng.next_u64();
        let (lo, hi) = self.time_limits;
        let time_limit = lo + self.rng.below((hi - lo + 1) as usize) as u32;
        let (kinds, decks, start) = match self.queued.pop_front() {
            Some(spec) => (spec.kinds, spec.decks, spec.start),
            None => {
                let n = self.kinds.len();
                let shift = if self.rotate {
                    (self.started % n as u64) as usize
                } else {
                    0
                };
                (
                    (0..n).map(|s| self.kinds[(s + shift) % n]).collect(),
                    None,
                    None,
                )
            }
        };
        if let Some(i) = start {
            // A decision in the game's last four fifths: past the opening.
            let start = &self.starts[i];
            let lo = start.readable / 5;
            let at = lo + self.rng.below(start.readable - lo);
            let engine = start.position(at, seed, time_limit);
            self.started += 1;
            let mut g = Game::with_engine(
                self.started - 1,
                seed,
                engine,
                Vec::new(),
                kinds,
                &self.versions,
                self.politics,
            );
            g.start = Some((i, at));
            g.habits = self.habits;
            g.chores = self.chores;
            return g;
        }
        let decks = self.draw_decks(decks.unwrap_or_else(|| vec![None; kinds.len()]));
        self.started += 1;
        let mut g = Game::new(
            self.started - 1,
            seed,
            time_limit,
            decks,
            kinds,
            &self.versions,
            self.politics,
        );
        g.habits = self.habits;
        g.chores = self.chores;
        g
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
    /// Seats given a deck (`pinned`, one entry a seat) keep it; the
    /// others are drawn against every deck already chosen, pinned ones too.
    fn draw_decks(&mut self, pinned: Vec<Option<usize>>) -> Vec<usize> {
        let mut out: Vec<usize> = pinned.iter().flatten().copied().collect();
        let fixed = out.len();
        for _ in pinned.iter().filter(|d| d.is_none()) {
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
        let mut drawn = out.split_off(fixed).into_iter();
        pinned
            .into_iter()
            .map(|d| d.unwrap_or_else(|| drawn.next().expect("a deck a seat")))
            .collect()
    }

    fn spec(
        &self,
        seats: Vec<String>,
        decks: Option<Vec<Option<usize>>>,
        start: Option<usize>,
    ) -> PyResult<Spec> {
        if !(4..=5).contains(&seats.len()) {
            return Err(PyValueError::new_err("4 or 5 seats (tournament tables)"));
        }
        let kinds = seats
            .iter()
            .map(|s| kind(s))
            .collect::<PyResult<Vec<_>>>()?;
        if let Some(d) = &decks {
            let n = all_decks().len();
            if d.len() != kinds.len() || d.iter().flatten().any(|&d| d >= n) {
                return Err(PyValueError::new_err(format!(
                    "one deck index in 0..{n} (or None: drawn) per seat"
                )));
            }
        }
        if let Some(i) = start {
            let Some(st) = self.starts.get(i) else {
                return Err(PyValueError::new_err(format!("no start {i}")));
            };
            if decks.is_some() || st.seats() != kinds.len() {
                return Err(PyValueError::new_err(format!(
                    "start {i}: {} seats, its decks",
                    st.seats()
                )));
            }
        }
        Ok(Spec {
            kinds,
            decks,
            start,
        })
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
                    .filter_map(|(i, g)| g.advance_caught(budget, i, keep, critic))
                    .collect()
            });
            if done.is_empty() {
                return;
            }
            // Highest slot first: a slot left empty is removed without
            // moving the slots still to settle.
            let mut done = done;
            done.sort_by_key(|ep| std::cmp::Reverse(ep.slot));
            for ep in done {
                let slot = ep.slot;
                self.finished.push(ep);
                if self.refill || !self.queued.is_empty() {
                    self.games[slot] = self.fresh();
                } else {
                    self.games.remove(slot);
                }
            }
        }
    }
}

#[pymethods]
impl GamePool {
    // A Python constructor with keyword arguments.
    #[allow(clippy::too_many_arguments)]
    #[new]
    #[pyo3(signature = (n_games, seats, seed=0, rotate=false, decks=None, budget=20_000, keep_logs=false, tables=None, time_limits=(60, 90), critic=false, obs=1, legacy=Vec::new(), net_obs=Vec::new(), politics=false, replays=Vec::new(), habits="rescue,burn,diablerie,home", refill=true, chores=false))]
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
        net_obs: Vec<(u16, u8)>,
        politics: bool,
        replays: Vec<String>,
        habits: &str,
        refill: bool,
        chores: bool,
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
        if !kinds.iter().any(|k| matches!(k, Kind::Net(..))) {
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
            refill,
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
                net_obs: net_obs
                    .into_iter()
                    .map(|(n, v)| Ok((n, version(v)?)))
                    .collect::<PyResult<_>>()?,
            },
            starts: replays
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let st = Start::new(r)
                        .map_err(|e| PyValueError::new_err(format!("replay {i}: {e}")))?;
                    if st.readable < 10 {
                        return Err(PyValueError::new_err(format!(
                            "replay {i}: reads only {} decisions",
                            st.readable
                        )));
                    }
                    Ok(st)
                })
                .collect::<PyResult<_>>()?,
            habit_counts: [(0, 0); 3],
            habits: habits::Set::parse(habits).map_err(PyValueError::new_err)?,
            chores,
            nudges: Nudges::default(),
            discard_counts: [(0, 0); 2],
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
    /// [B, O, G] f32, `opt_n` [B], `opt_bias` [B, O] f32 (added to the
    /// logits before sampling: the `nudges`), `seat` [B] (the deciding seat), `agent`
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
                    Kind::Net(k, _) => k as i32,
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
        let mut bias = vec![0f32; b * n];
        if !self.nudges.is_zero() {
            for (i, g) in self.games.iter().enumerate() {
                let (r, _) = g.pending.as_ref().expect("a pending decision");
                for (k, o) in r.options.iter().enumerate() {
                    bias[i * n + k] = self.nudges.bias(&g.engine, r, o);
                }
            }
        }
        d.set_item("opt_bias", PyArray1::from_vec(py, bias).reshape([b, n])?)?;
        d.set_item("seat", PyArray1::from_vec(py, seat))?;
        d.set_item("agent", PyArray1::from_vec(py, agent))?;
        d.set_item("window", PyArray1::from_vec(py, window))?;
        // `V3`'s decks apart (`Obs::deck_ids`), empty before.
        padded(
            py,
            &d,
            "deck",
            &obs.iter().map(|(_, o)| *o).collect::<Vec<_>>(),
            |o| (&o.deck_ids, &o.deck_f),
        )?;
        if self.critic {
            let obs: Vec<&Obs> = self
                .games
                .iter()
                .map(|g| g.critic.as_ref().expect("a critic encoding"))
                .collect();
            padded(py, &d, "critic_tok", &obs, |o| (&o.tok_ids, &o.tok_f))?;
            padded(py, &d, "critic_deck", &obs, |o| (&o.deck_ids, &o.deck_f))?;
        }
        Ok(d)
    }

    /// Answer every pending decision (one option index per game), advance,
    /// and return the games finished since the last call: dicts with
    /// `slot`, `seed`, `decks`, `seats`, `half_vps`, `turns`, `decisions`,
    /// `truncated` (and `log` with `keep_logs`; `panic`, the message, for a
    /// game the engine panicked in: `Game::advance_caught`). A finished slot already
    /// holds its next game (but past the queue without `refill`: the slot
    /// is gone).
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
            if g.kinds[req.seat.index()] == Kind::Net(0, false) {
                let of = |o| habits::habit(&g.engine, req.seat, o).map(|h| h as usize);
                let mut offered = [false; 3];
                for o in &req.options {
                    if let Some(h) = of(o) {
                        offered[h] = true;
                    }
                }
                for (h, &yes) in offered.iter().enumerate() {
                    self.habit_counts[h].0 += yes as u64;
                }
                if let Some(h) = of(&req.options[a]) {
                    self.habit_counts[h].1 += 1;
                }
                if zvtes_policy::cycling(&g.engine, &req) {
                    let hand = &g.engine.state.seats[req.seat.index()].hand;
                    let reaction = |o: &Choice| {
                        matches!(*o, Choice::Discard(i)
                            if LIBRARY[hand[i as usize] as usize].types.contains(&CardType::Reaction))
                    };
                    self.discard_counts[0].0 += 1;
                    self.discard_counts[0].1 += (req.options[a] == Choice::Pass) as u64;
                    if req.options.iter().any(reaction) {
                        self.discard_counts[1].0 += 1;
                        self.discard_counts[1].1 += reaction(&req.options[a]) as u64;
                    }
                }
            }
            let a = g.offered.take().map_or(a, |o| o[a]);
            let engine = &mut g.engine;
            if let Err(e) =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| engine.choose(a)))
            {
                g.crashed = Some(panic_message(&*e));
            }
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
                if let Some(start) = ep.start {
                    d.set_item("start", start)?;
                }
                if let Some(panic) = ep.panic {
                    d.set_item("panic", panic)?;
                }
                Ok(d)
            })
            .collect()
    }

    /// Replace the queue of tables the next new games take, in order:
    /// dicts with `seats` (kinds: "net:k", "random", "aggro", "wall"; 4 or
    /// 5), optionally `decks` (one index per seat, None for one drawn) or `start` (an index into
    /// `replays`: the game from one of its decisions, its decks the
    /// replay's, as many seats as it has). When it runs out, new
    /// games take the pool's default table.
    fn queue(&mut self, specs: Vec<Bound<'_, PyDict>>) -> PyResult<()> {
        let mut queued = std::collections::VecDeque::new();
        for d in specs {
            let seats: Vec<String> = d
                .get_item("seats")?
                .ok_or_else(|| PyValueError::new_err("a spec needs seats"))?
                .extract()?;
            let decks: Option<Vec<Option<usize>>> = match d.get_item("decks")? {
                Some(v) => Some(v.extract()?),
                None => None,
            };
            let start: Option<usize> = match d.get_item("start")? {
                Some(v) => Some(v.extract()?),
                None => None,
            };
            queued.push_back(self.spec(seats, decks, start)?);
        }
        self.queued = queued;
        Ok(())
    }

    #[getter]
    fn n_games(&self) -> usize {
        self.games.len()
    }

    /// The learner's habits since the last call: per habit ("rescue",
    /// "burn", "diablerie"), (decisions offering it, decisions taking it);
    /// "keep": (discard phases with a card to cycle, those passed);
    /// "reaction": (those holding a reaction to cycle, those cycling one).
    fn habits<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (name, n) in ["rescue", "burn", "diablerie"]
            .iter()
            .zip(std::mem::take(&mut self.habit_counts))
        {
            d.set_item(name, n)?;
        }
        for (name, n) in ["keep", "reaction"]
            .iter()
            .zip(std::mem::take(&mut self.discard_counts))
        {
            d.set_item(name, n)?;
        }
        Ok(d)
    }

    /// The nudges on every network seat's logits (`opt_bias`,
    /// `zvtes_policy::Nudges`): a dict "rescue" (on a rescue of one's own
    /// vampire), "pass" (on passing one's discard phase with a card to
    /// cycle), "reaction" (taken off cycling a reaction), "bleed" (taken
    /// off a bleed for nothing: `zvtes_policy::empty_bleed`), "idle" (taken
    /// off ending the influence phase with a crypt draw still offered:
    /// `zvtes_policy::idle_influence`), "draw" (on a crypt draw there),
    /// "stuck" (taken off `zvtes_policy::stuck_transfer`), "nightmare"
    /// (taken off `zvtes_policy::own_nightmare`), all 0 by default.
    /// Set with any of the keys; those left out are kept.
    #[getter]
    fn nudges(&self) -> std::collections::BTreeMap<&'static str, f32> {
        let n = self.nudges;
        [
            ("rescue", n.rescue),
            ("pass", n.pass),
            ("reaction", n.reaction),
            ("bleed", n.bleed),
            ("idle", n.idle),
            ("draw", n.draw),
            ("stuck", n.stuck),
            ("nightmare", n.nightmare),
        ]
        .into()
    }

    #[setter]
    fn set_nudges(&mut self, d: std::collections::HashMap<String, f32>) -> PyResult<()> {
        for (k, v) in d {
            match k.as_str() {
                "rescue" => self.nudges.rescue = v,
                "pass" => self.nudges.pass = v,
                "reaction" => self.nudges.reaction = v,
                "bleed" => self.nudges.bleed = v,
                "idle" => self.nudges.idle = v,
                "draw" => self.nudges.draw = v,
                "stuck" => self.nudges.stuck = v,
                "nightmare" => self.nudges.nightmare = v,
                _ => {
                    return Err(PyValueError::new_err(format!(
                        "nudge {k:?}: rescue, pass, reaction, bleed, idle, draw, stuck, nightmare"
                    )))
                }
            }
        }
        Ok(())
    }

    /// The decisions this engine reads from each start (`replays`), by
    /// index.
    #[getter]
    fn starts(&self) -> Vec<usize> {
        self.starts.iter().map(|s| s.readable).collect()
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

/// Block windows in a game replayed from its log (`replay`'s arguments):
/// (those offering a futile attempt, `Engine::block_can_reach`; the
/// futile attempts taken; those offering only futile ones; those of them
/// attempted) — the rollouts' mask leaves the second and fourth at 0.
#[pyfunction]
#[pyo3(signature = (seed, decks, log, time_limit=zvtes::game::TURN_LIMIT))]
fn futile_blocks(
    seed: u64,
    decks: Vec<usize>,
    log: Vec<usize>,
    time_limit: u32,
) -> (u32, u32, u32, u32) {
    let list: Vec<Deck> = decks.iter().map(|&d| all_decks()[d]).collect();
    let mut e = Engine::with_decks(seed, &list);
    e.time_limit = time_limit;
    let mut counts = (0, 0, 0, 0);
    for i in log {
        let Step::Decision(req) = e.run() else { break };
        let futile = |o: &Choice| matches!(*o, Choice::DeclareBlock { minion } if !e.block_can_reach(req.seat, minion));
        if req.options.iter().any(futile) {
            counts.0 += 1;
            counts.1 += futile(&req.options[i]) as u32;
        }
        if e.futile_blocks(&req) {
            counts.2 += 1;
            counts.3 += (req.options[i] != Choice::Pass) as u32;
        }
        e.choose(i);
    }
    counts
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
/// `train/model.py` reads this. `deck_ids`/`deck_f`: `V3`'s deck rows.
#[pyfunction]
#[pyo3(signature = (weights, tok_ids, tok_f, opt_ids, opt_f, deck_ids=Vec::new(), deck_f=Vec::new()))]
fn policy_forward(
    weights: &[u8],
    tok_ids: Vec<[i32; TOK_IDS]>,
    tok_f: Vec<[f32; TOK_F]>,
    opt_ids: Vec<[i32; OPT_IDS]>,
    opt_f: Vec<[f32; OPT_F]>,
    deck_ids: Vec<[i32; TOK_IDS]>,
    deck_f: Vec<[f32; TOK_F]>,
) -> PyResult<(Vec<f32>, f32)> {
    let net = zvtes_policy::Net::load(weights).map_err(PyValueError::new_err)?;
    Ok(net.forward(&Obs {
        tok_ids,
        tok_f,
        opt_ids,
        opt_f,
        deck_ids,
        deck_f,
    }))
}

#[pymodule]
fn zvtes_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<GamePool>()?;
    m.add_function(wrap_pyfunction!(replay, m)?)?;
    m.add_function(wrap_pyfunction!(futile_blocks, m)?)?;
    m.add_function(wrap_pyfunction!(schema, m)?)?;
    m.add_function(wrap_pyfunction!(deck_names, m)?)?;
    m.add_function(wrap_pyfunction!(vocab_keys, m)?)?;
    m.add_function(wrap_pyfunction!(deck_families, m)?)?;
    m.add_function(wrap_pyfunction!(policy_forward, m)?)?;
    m.add_function(wrap_pyfunction!(human::replay_decisions, m)?)?;
    m.add_function(wrap_pyfunction!(human::deck_cards, m)?)?;
    Ok(())
}
