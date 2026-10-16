//! A game record read back (docs/protocol.md §11): its deal, and each
//! decision answered as recorded — by its key (`ChoiceKey` with the
//! engine's copies), so a record keeps reading when the engine changes the
//! options it offers or their order; it stops where the rules changed (the
//! engine asks a decision the game never had, or no longer offers what was
//! chosen) — but for the block windows an earlier engine elided
//! (`Replay::elided`). `Replay` reads at the engine's level (training
//! starts, tools); `Replayer` tells it too, as a seat seeing everything
//! would read it (human games as data: `zvtes_py`).

use std::panic::{catch_unwind, AssertUnwindSafe};

use zvtes::decision::{Choice, DecisionRequest};
use zvtes::game::{self, Engine};
use zvtes_api::ids::Window;
use zvtes_api::message::EndReason;
use zvtes_api::option::{ChoiceKey, OptionView};
use zvtes_api::record::{By, Entry, GameRecord, GameResult};
use zvtes_api::{Player, Seat};

use crate::deck::Resolved;
use crate::describe::{self, EngineNames};
use crate::numbering::Decks;
use crate::session::{deal, Next, Session, Taken};
use crate::view::Viewer;

/// A decision as recorded: its number, seat, option and key.
#[derive(Clone, Debug)]
pub struct Recorded {
    pub decision: u64,
    pub seat: Seat,
    pub option: u32,
    pub choice: ChoiceKey,
    pub by: By,
}

/// A record's game: its deal and its decisions, the ones an undo took
/// back left out.
pub struct Replay {
    seed: u64,
    time_limit: Option<u32>,
    decks: Vec<Resolved>,
    pub decisions: Vec<Recorded>,
    /// Seat names (the human's as recorded) and the seats a human played.
    pub names: Vec<String>,
    pub humans: Vec<Seat>,
    /// Played on an engine before rules-qa 73.8, which elided the block
    /// windows offering only attempts that cannot reach the stealth: read
    /// past them (`Replay::elided`), as the record reads further so.
    elides: bool,
}

impl Replay {
    pub fn new(record: &GameRecord) -> Result<Replay, String> {
        let spec = &record.spec;
        let decks = spec
            .seats
            .iter()
            .enumerate()
            .map(|(seat, s)| crate::deck::resolve(seat as Seat, &s.deck))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("{e:?}"))?;
        let seed = spec.seed.ok_or("a record without its seed")?;
        let mut decisions: Vec<Recorded> = Vec::new();
        for entry in &record.entries {
            match entry {
                Entry::Decision {
                    decision,
                    seat,
                    option,
                    choice,
                    by,
                    ..
                } => decisions.push(Recorded {
                    decision: *decision,
                    seat: *seat,
                    option: *option,
                    choice: choice.clone(),
                    by: *by,
                }),
                // The game went on from `to`: what followed is taken back.
                Entry::Undo { to, .. } => decisions.retain(|d| d.decision < *to),
                Entry::Settings { .. }
                | Entry::Refused { .. }
                | Entry::Feedback { .. }
                | Entry::Unknown => {}
            }
        }
        let mut replay = Replay {
            seed: seed as u64,
            time_limit: spec.time_limit,
            decks,
            decisions,
            names: spec.seats.iter().map(|s| s.name.clone()).collect(),
            humans: (0..spec.seats.len())
                .filter(|&i| matches!(spec.seats[i].player, Player::Human { .. }))
                .map(|i| i as Seat)
                .collect(),
            elides: false,
        };
        let plain = replay.read(usize::MAX).1;
        replay.elides = true;
        replay.elides = replay.read(usize::MAX).1 > plain;
        Ok(replay)
    }

    /// The game replayed to decision `at` (or as far as it reads), and the
    /// decisions read.
    pub fn read(&self, at: usize) -> (Engine, usize) {
        let mut e = self.deal();
        // Combats and reveals are not told here.
        e.combat_notes = None;
        e.reveal_notes = None;
        let mut reader = self.reader();
        let mut n = 0;
        while n < at {
            let game::Step::Decision(req) = e.run() else {
                break;
            };
            if let Some(pass) = self.elided(&e, &req) {
                e.choose(pass);
                continue;
            }
            match reader.next(&e, &req) {
                Some(Ok(i)) => e.choose(i),
                _ => break,
            }
            n += 1;
        }
        (e, n)
    }

    /// The game as dealt.
    pub fn deal(&self) -> Engine {
        deal(self.seed, self.time_limit, &self.decks)
    }

    /// The Pass of a block window offering only attempts that cannot reach
    /// the stealth, in a record from an engine that elided it (`elides`):
    /// taken without reading a recorded decision.
    pub fn elided(&self, e: &Engine, req: &DecisionRequest) -> Option<usize> {
        if !self.elides || !e.futile_blocks(req) {
            return None;
        }
        req.options.iter().position(|o| *o == Choice::Pass)
    }

    /// The record's decisions read in order.
    pub fn reader(&self) -> Reader<'_> {
        Reader {
            replay: self,
            next: 0,
        }
    }

    pub fn seats(&self) -> usize {
        self.decks.len()
    }
}

/// A game its host lost (a server restart) ended as its record stands: the
/// victory points where the record reads to on this engine (none when it
/// does not read at all), and why it did not read to its end.
pub fn ended(record: &GameRecord, reason: EndReason) -> GameResult {
    let read = catch_unwind(AssertUnwindSafe(|| {
        let replay = Replay::new(record)?;
        let (e, n) = replay.read(usize::MAX);
        let all = replay.decisions.len();
        let vp: Vec<u32> = e.state.seats.iter().map(|s| s.half_vps).collect();
        Ok::<_, String>((
            vp,
            (n < all).then(|| format!("the record reads {n} of its {all} decisions")),
            Some(e.state.turn),
        ))
    }));
    let none = vec![0; record.spec.seats.len()];
    let (vp_halves, failure, turns) = match read {
        Ok(Ok(read)) => read,
        Ok(Err(why)) => (none, Some(format!("the record does not read: {why}")), None),
        Err(_) => (none, Some("the record's replay panicked".into()), None),
    };
    GameResult {
        winner: crate::game::winner(&vp_halves),
        vp_halves,
        reason,
        failure,
        turns,
    }
}

/// A record's decisions, answered one at a time as a game replaying it
/// asks them (`Replay::reader`).
pub struct Reader<'a> {
    replay: &'a Replay,
    next: usize,
}

impl Reader<'_> {
    /// How many decisions were read.
    pub fn position(&self) -> usize {
        self.next
    }

    /// The option the record takes at the decision `req` asks (on `e`, as
    /// it stands then): the option whose key is the recorded one (its
    /// recorded index among several alike). `None` when the record ends;
    /// an error when no option is it (the rules changed).
    pub fn next(&mut self, e: &Engine, req: &DecisionRequest) -> Option<Result<usize, String>> {
        let d = self.replay.decisions.get(self.next)?;
        self.next += 1;
        // A record by index alone (its keys unknown): the index.
        if d.choice.kind == zvtes_api::ids::OptionKind::Unknown {
            let i = d.option as usize;
            return Some(if i < req.options.len() {
                Ok(i)
            } else {
                Err(format!(
                    "decision {}: option {i} of {} is refused",
                    d.decision,
                    req.options.len()
                ))
            });
        }
        // A keyed record names its seat.
        if d.seat != req.seat.0 {
            return Some(Err(format!(
                "decision {}: seat {} asked, the record's is seat {}'s",
                d.decision, req.seat.0, d.seat
            )));
        }
        let alike: Vec<usize> = (0..req.options.len())
            .filter(|&i| describe::key(e, req, &req.options[i], &EngineNames) == d.choice)
            .collect();
        let i = d.option as usize;
        Some(match alike.as_slice() {
            // No key alike (a key written by an earlier engine, its fields
            // since grown): the recorded index, if it is that kind of choice.
            [] if i < req.options.len()
                && describe::key(e, req, &req.options[i], &EngineNames).kind == d.choice.kind =>
            {
                Ok(i)
            }
            [] => Err(format!(
                "decision {}: seat {}'s {:?} is not offered",
                d.decision, d.seat, d.choice.kind
            )),
            many if many.contains(&i) => Ok(i),
            [first, ..] => Ok(*first),
        })
    }
}

/// A decision read back: what was offered and chosen, told.
#[derive(Clone, Debug)]
pub struct Read {
    pub decision: u64,
    pub seat: Seat,
    pub turn: u32,
    pub window: Window,
    /// Each option told, and the KRCG id of the card it names.
    pub options: Vec<String>,
    pub cards: Vec<Option<u32>>,
    /// What happened since the previous decision, told.
    pub before: Vec<String>,
    /// The record's option (`None` where the record ends or stops).
    pub chosen: Option<usize>,
}

/// What reading a record comes to next.
pub enum Step {
    /// A decision: the engine stands at it (`Replayer::engine`,
    /// `request`) until `take` answers it.
    Decision(Read),
    /// The game's end: victory points in halves, and what led to it.
    Over {
        vp_halves: Vec<u32>,
        before: Vec<String>,
    },
    /// The record stops reading here (the decision is the last `Read`).
    Stopped { why: String, last: Read },
}

/// A record read back and told, every card shown: the decisions in turn,
/// the engine at each (for encodings).
pub struct Replayer {
    replay: Replay,
    next: usize,
    session: Session,
    viewer: Viewer,
    dealt: Decks,
    known: zvtes_text::Names<'static>,
    pending: Option<usize>,
}

impl Replayer {
    pub fn new(record: &GameRecord) -> Result<Replayer, String> {
        let replay = Replay::new(record)?;
        let agents = (0..replay.seats()).map(|_| None).collect();
        let session = Session::new(replay.seed, replay.time_limit, replay.decks.clone(), agents);
        let dealt = Decks::dealt(session.engine());
        let mut viewer = Viewer::new(None);
        let mut known = zvtes_text::Names::new(crate::cards::cards());
        known.see(&viewer.view(session.engine(), &dealt, &replay.names));
        Ok(Replayer {
            replay,
            next: 0,
            session,
            viewer,
            dealt,
            known,
            pending: None,
        })
    }

    pub fn names(&self) -> &[String] {
        &self.replay.names
    }

    pub fn humans(&self) -> &[Seat] {
        &self.replay.humans
    }

    pub fn engine(&self) -> &Engine {
        self.session.engine()
    }

    pub fn request(&self) -> Option<&DecisionRequest> {
        self.session.request()
    }

    /// The table seen whole, and what happened since, told.
    fn told(&mut self) -> Vec<String> {
        let told = self.session.take_told();
        let e = self.session.engine();
        let v = self.viewer.view(e, &self.dealt, &self.replay.names);
        let mut before = Vec::new();
        for t in told {
            let Some(kind) = self.viewer.event(e, &self.dealt, t.fact) else {
                continue;
            };
            self.known.hear(&kind);
            if t.told {
                before.extend(zvtes_text::event(&kind, &self.known));
            }
        }
        self.known.see(&v);
        before
    }

    /// The next decision (answer it with `take`), or the end.
    pub fn step(&mut self) -> Step {
        let shown = loop {
            let shown = match self.session.advance() {
                Next::Over => {
                    let before = self.told();
                    let vp_halves = self.session.over().unwrap_or_default().to_vec();
                    return Step::Over { vp_halves, before };
                }
                Next::Ask { shown, .. } => shown,
                Next::Take { .. } => unreachable!("every seat is read from the record"),
            };
            let req = self.session.request().expect("asked");
            match self.replay.elided(self.session.engine(), req) {
                Some(pass) => self.session.take(pass, Taken::Passed),
                None => break shown,
            }
        };
        let before = self.told();
        let req = self.session.request().expect("asked").clone();
        let e = self.session.engine();
        let names = &self.viewer.numbering;
        let views: Vec<OptionView> = shown
            .iter()
            .map(|&i| {
                let c = &req.options[i];
                OptionView {
                    index: i as u32,
                    key: describe::key(e, &req, c, names),
                    effects: describe::effects(e, &req, c, names),
                    abilities: describe::abilities(e, c),
                    text: None,
                }
            })
            .collect();
        let window = crate::ids::window(req.window);
        let purpose = e.interrupt().map(|i| crate::ids::purpose(&i));
        let about = describe::about(e, &req, names);
        let options = views
            .iter()
            .map(|o| {
                zvtes_text::option(
                    o,
                    window,
                    purpose,
                    about.as_ref(),
                    req.seat.0,
                    &views,
                    &self.known,
                )
                .unwrap_or_default()
            })
            .collect();
        let cards = views
            .iter()
            .map(|o| o.key.card.as_ref().and_then(|c| c.id))
            .collect();
        let mut read = Read {
            decision: self.session.decision(),
            seat: req.seat.0,
            turn: e.state.turn,
            window,
            options,
            cards,
            before,
            chosen: None,
        };
        let mut reader = Reader {
            replay: &self.replay,
            next: self.next,
        };
        match reader.next(e, &req) {
            None => Step::Decision(read),
            Some(Err(why)) => Step::Stopped { why, last: read },
            Some(Ok(i)) => {
                self.next = reader.next;
                self.pending = Some(i);
                read.chosen = Some(i);
                Step::Decision(read)
            }
        }
    }

    /// Answer the decision `step` read with the record's option.
    pub fn take(&mut self) {
        if let Some(i) = self.pending.take() {
            self.session.take(i, Taken::Bot);
        }
    }
}
