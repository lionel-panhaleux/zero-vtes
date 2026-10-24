//! A game record read back (docs/protocol.md §11): its deal, and each
//! decision answered as recorded — by its key (`ChoiceKey` with the
//! engine's copies), so a record keeps reading when the engine changes the
//! options it offers or their order; it stops where the rules changed (the
//! engine asks a decision the game never had, or no longer offers what was
//! chosen, or, from 2.4.0, the position differs from the one recorded:
//! `past`) — but for the block windows an earlier engine elided
//! (`Replay::elided`) and the Passes of a window no longer asked. `Replay`
//! reads at the engine's level (training starts, tools); `Replayer` tells
//! it too, as a seat seeing everything would read it (human games as
//! data: `zvtes_py`).

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

/// A decision as recorded: its number, seat, option and key, the window
/// and the position's digest (records from 2.4.0).
#[derive(Clone, Debug)]
pub struct Recorded {
    pub decision: u64,
    pub seat: Seat,
    pub option: u32,
    pub choice: ChoiceKey,
    pub by: By,
    pub window: Option<Window>,
    pub digest: Option<String>,
    /// Its index in the record's `entries`.
    pub entry: usize,
}

impl Recorded {
    /// The decision the record's entry `i` records, if it is one.
    pub fn of(i: usize, entry: &Entry) -> Option<Recorded> {
        match entry {
            Entry::Decision {
                decision,
                seat,
                option,
                choice,
                by,
                window,
                digest,
                ..
            } => Some(Recorded {
                decision: *decision,
                seat: *seat,
                option: *option,
                choice: choice.clone(),
                by: *by,
                window: *window,
                digest: digest.clone(),
                entry: i,
            }),
            _ => None,
        }
    }
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
    /// Played on an engine before rules-qa 82.1 (protocol 1.9.0), which
    /// reopened the block window at the acting Methuselah after an attempt
    /// failed short of the stealth: its recorded passes there are read past
    /// (`Reader::next`).
    reopens: bool,
    /// The options its engine did not offer yet (`Then`): a window with
    /// one option besides them (elided then) takes it unread
    /// (`Replay::elided`).
    then: Then,
}

/// The choices a record's engine did not offer: a withdrawal before
/// protocol 2.13.0, a path followed before 2.14.0.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Then {
    before_withdrawals: bool,
    before_paths: bool,
}

impl Then {
    fn of(protocol: &str) -> Then {
        Then {
            before_withdrawals: before(protocol, (2, 13)),
            before_paths: before(protocol, (2, 14)),
        }
    }

    /// `c` came after the record's engine.
    fn unoffered(self, c: &Choice) -> bool {
        match c {
            Choice::WithdrawFromGame => self.before_withdrawals,
            Choice::FollowPath { .. } => self.before_paths,
            _ => false,
        }
    }
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
        for (i, entry) in record.entries.iter().enumerate() {
            match entry {
                Entry::Decision { .. } => decisions.extend(Recorded::of(i, entry)),
                // The game went on from `to`: what followed is taken back.
                Entry::Undo { to, .. } => decisions.retain(|d| d.decision < *to),
                Entry::Settings { .. }
                | Entry::Refused { .. }
                | Entry::Feedback { .. }
                | Entry::Unknown => {}
            }
        }
        let mut replay = Replay {
            seed,
            time_limit: spec.time_limit,
            decks,
            decisions,
            names: spec.seats.iter().map(|s| s.name.clone()).collect(),
            humans: (0..spec.seats.len())
                .filter(|&i| matches!(spec.seats[i].player, Player::Human { .. }))
                .map(|i| i as Seat)
                .collect(),
            elides: false,
            reopens: before(&record.protocol, (1, 9)),
            then: Then::of(&record.protocol),
        };
        let plain = replay.read(usize::MAX).1;
        replay.elides = true;
        replay.elides = replay.read(usize::MAX).1 > plain;
        Ok(replay)
    }

    /// The game replayed to decision `at` (or as far as it reads), and the
    /// decisions read.
    pub fn read(&self, at: usize) -> (Engine, usize) {
        let (e, n, _) = self.read_counting(at);
        (e, n)
    }

    /// As `read`, with the record's decisions consumed too (the passes
    /// rules-qa 82.1 moved read past included).
    pub fn read_counting(&self, at: usize) -> (Engine, usize, usize) {
        let mut e = self.deal();
        // Combats, reveals, looks, plays and actions are not told here.
        e.notes = None;
        let mut reader = self.reader();
        let (mut n, mut consumed) = (0, 0);
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
            consumed = reader.position();
        }
        // The game goes on from here as any other (`Start::position`).
        e.short_declines = None;
        (e, n, consumed)
    }

    /// The game as dealt.
    pub fn deal(&self) -> Engine {
        let mut e = deal(self.seed, self.time_limit, &self.decks);
        if self.reopens {
            e.short_declines = Some(Vec::new());
        }
        e
    }

    /// The Pass of a block window offering only attempts that cannot reach
    /// the stealth, in a record from an engine that elided it (`elides`),
    /// or the lone option beside those a record's engine did not offer
    /// (`then`): taken without reading a recorded decision.
    pub fn elided(&self, e: &Engine, req: &DecisionRequest) -> Option<usize> {
        let old: Vec<usize> = (0..req.options.len())
            .filter(|&i| !self.then.unoffered(&req.options[i]))
            .collect();
        if let [only] = old[..] {
            if old.len() < req.options.len() {
                return Some(only);
            }
        }
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
            declines: 0,
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
        let (e, _, n) = replay.read_counting(usize::MAX);
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
    /// The engine's short declines read past so far
    /// (`Engine::short_declines`).
    declines: usize,
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
        // Before rules-qa 82.1, an attempt failed short of the stealth
        // reopened the block window at the acting Methuselah, the seats
        // through the blocker asked again (the blocker's pass its decline):
        // their recorded passes there, right after it, are read past; a
        // play there instead stops the record (the rules changed).
        let notes = e.short_declines.as_deref().unwrap_or_default();
        for asked in &notes[self.declines.min(notes.len())..] {
            // A seat offered only futile attempts was not asked before
            // rules-qa 73.8 either (`Replay::elided`).
            for r in asked.iter().filter(|r| !(r.futile && self.replay.elides)) {
                match self.replay.decisions.get(self.next) {
                    Some(d)
                        if d.seat == r.seat.0
                            && d.choice.kind == zvtes_api::ids::OptionKind::Pass =>
                    {
                        self.next += 1;
                    }
                    _ => break,
                }
            }
        }
        self.declines = notes.len();
        // Before rules-qa 88.5, Anarch Revolt's burn was ordered among the
        // unlock-phase riders: a rider's order recorded there that the
        // engine no longer asks (the Revolt not offered, a lone rider
        // elided) is read past.
        while let Some(d) = self.replay.decisions.get(self.next) {
            if d.choice.kind != zvtes_api::ids::OptionKind::ResolveRider
                || recorded_option(e, req, d, self.replay.then).is_ok()
            {
                break;
            }
            self.next += 1;
        }
        let rest = self.replay.decisions.get(self.next..).unwrap_or_default();
        match past(e, req, rest) {
            Ok(n) => self.next += n,
            Err(why) => return Some(Err(why)),
        }
        let d = self.replay.decisions.get(self.next)?;
        self.next += 1;
        Some(recorded_option(e, req, d, self.replay.then))
    }
}

/// How many of the record's decisions `rest` (the next first) are read
/// past at the decision `req` asks (on `e`, as it stands then): the Passes
/// of a window the engine no longer asks, the decision after them the one
/// asked, in its window and on its position. An error when the next was
/// taken in another window (one newly asked: a rules change) or on another
/// position (a rules change moving hidden state with no decision: the game
/// read is no longer the game played). A record before 2.4.0 holds neither
/// and is read as before.
pub(crate) fn past(e: &Engine, req: &DecisionRequest, rest: &[Recorded]) -> Result<usize, String> {
    let Some(d) = rest.first() else {
        return Ok(0);
    };
    let window = crate::ids::window(req.window);
    let digest = || crate::digest::digest(e);
    if let Some(w) = d.window.filter(|&w| w != window) {
        let gone = rest
            .iter()
            .take_while(|r| {
                r.window == Some(w) && r.choice.kind == zvtes_api::ids::OptionKind::Pass
            })
            .count();
        let asked = rest.get(gone).is_some_and(|n| {
            n.seat == req.seat.0
                && n.window == Some(window)
                && n.digest.as_ref().is_none_or(|x| *x == digest())
        });
        return if gone > 0 && asked {
            Ok(gone)
        } else {
            Err(format!(
                "decision {}: seat {} asked in {window:?}, the record's in {w:?}",
                d.decision, req.seat.0
            ))
        };
    }
    match &d.digest {
        Some(x) if *x != digest() => Err(format!(
            "decision {}: the position differs from the record's",
            d.decision
        )),
        _ => Ok(0),
    }
}

/// The option `d` takes at the decision `req` asks (on `e`, as it stands
/// then): the option whose key is the recorded one (its recorded index
/// among several alike); an error when no option is it (the rules
/// changed).
pub(crate) fn recorded_option(
    e: &Engine,
    req: &DecisionRequest,
    d: &Recorded,
    then: Then,
) -> Result<usize, String> {
    // A record by index alone (its keys unknown): the index, among the
    // options of its time (`Then`).
    if d.choice.kind == zvtes_api::ids::OptionKind::Unknown {
        let i = d.option as usize;
        let then: Vec<usize> = (0..req.options.len())
            .filter(|&j| !then.unoffered(&req.options[j]))
            .collect();
        return if let Some(&i) = then.get(i) {
            Ok(i)
        } else {
            Err(format!(
                "decision {}: option {i} of {} is refused",
                d.decision,
                req.options.len()
            ))
        };
    }
    // A keyed record names its seat.
    if d.seat != req.seat.0 {
        return Err(format!(
            "decision {}: seat {} asked, the record's is seat {}'s",
            d.decision, req.seat.0, d.seat
        ));
    }
    let keys: Vec<ChoiceKey> = req
        .options
        .iter()
        .map(|c| describe::key(e, req, c, &EngineNames))
        .collect();
    let mut alike: Vec<usize> = (0..keys.len()).filter(|&i| keys[i] == d.choice).collect();
    // A key written before its kind's fields grew (2.5.0: a bleed's
    // target, a slot, an actor): every field it has, alike.
    if alike.is_empty() {
        alike = (0..keys.len())
            .filter(|&i| covers(&d.choice, &keys[i]))
            .collect();
    }
    let i = d.option as usize;
    match alike.as_slice() {
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
    }
}

/// Every field `recorded` sets, `now` sets alike.
fn covers(recorded: &ChoiceKey, now: &ChoiceKey) -> bool {
    fn same<T: PartialEq>(r: &Option<T>, n: &Option<T>) -> bool {
        r.is_none() || r == n
    }
    let (r, n) = (recorded, now);
    r.kind == n.kind
        && same(&r.card, &n.card)
        && same(&r.actor, &n.actor)
        && same(&r.minion, &n.minion)
        && same(&r.target, &n.target)
        && same(&r.action, &n.action)
        && same(&r.mode, &n.mode)
        && same(&r.amount, &n.amount)
        && same(&r.favor, &n.favor)
        && same(&r.blood, &n.blood)
        && same(&r.zone, &n.zone)
        && same(&r.value, &n.value)
        && same(&r.ability, &n.ability)
        && same(&r.slot, &n.slot)
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
    /// The record's entry it took (its index in `entries`): not the n-th
    /// decision entry, Passes read past and undos taken back between.
    pub entry: Option<usize>,
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
    /// `Reader::declines`, kept between steps.
    declines: usize,
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
        let mut session =
            Session::new(replay.seed, replay.time_limit, replay.decks.clone(), agents);
        if replay.reopens {
            session.keep_short_declines();
        }
        let dealt = Decks::dealt(session.engine());
        let mut viewer = Viewer::new(None);
        let mut known = zvtes_text::Names::new(crate::cards::cards());
        known.see(&viewer.view(session.engine(), &dealt, &replay.names));
        Ok(Replayer {
            replay,
            next: 0,
            declines: 0,
            session,
            viewer,
            dealt,
            known,
            pending: None,
        })
    }

    /// The record's decisions read so far, the passes read past included
    /// (`Reader::next`).
    pub fn position(&self) -> usize {
        self.next
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
        let (v, named) = self.viewer.tell(e, &self.dealt, &self.replay.names, told);
        let mut before = Vec::new();
        for (t, kind) in named {
            let Some(kind) = kind else {
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
                    // The game over before a rider's order recorded last
                    // (rules-qa 88.5: the Revolt's burn ousting at the
                    // pass): read past it.
                    let rest = self.replay.decisions.get(self.next..).unwrap_or_default();
                    if rest
                        .iter()
                        .all(|d| d.choice.kind == zvtes_api::ids::OptionKind::ResolveRider)
                    {
                        self.next += rest.len();
                    }
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
                    apart: describe::ability_apart(e, c),
                    user: describe::ability_user(e, &req, c, names),
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
            entry: None,
        };
        let mut reader = Reader {
            replay: &self.replay,
            next: self.next,
            declines: self.declines,
        };
        match reader.next(e, &req) {
            None => Step::Decision(read),
            Some(Err(why)) => Step::Stopped { why, last: read },
            Some(Ok(i)) => {
                self.next = reader.next;
                self.declines = reader.declines;
                self.pending = Some(i);
                read.chosen = Some(i);
                read.entry = Some(self.replay.decisions[reader.next - 1].entry);
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

/// Whether the protocol version `v` ("1.8.0") is before `major.minor`.
fn before(v: &str, (major, minor): (u32, u32)) -> bool {
    let mut parts = v.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    let (a, b) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0));
    (a, b) < (major, minor)
}

#[cfg(test)]
mod covering {
    use zvtes_api::ids::{ActionKind, OptionKind};
    use zvtes_api::option::ChoiceKey;
    use zvtes_api::view::Target;

    fn bleed(actor: u32, target: Option<Target>) -> ChoiceKey {
        ChoiceKey {
            kind: OptionKind::TakeAction,
            card: None,
            actor: Some(actor),
            minion: None,
            target,
            action: Some(ActionKind::Bleed),
            mode: None,
            amount: None,
            favor: None,
            blood: None,
            zone: None,
            value: None,
            ability: None,
            slot: None,
        }
    }

    /// A key recorded before a bleed named its seat reads as the bleed
    /// it was, not as another minion's.
    #[test]
    fn a_key_recorded_with_fewer_fields_covers_the_option_now() {
        let now = bleed(7, Some(Target::Seat { seat: 1 }));
        assert!(super::covers(&bleed(7, None), &now));
        assert!(!super::covers(&bleed(8, None), &now));
        assert!(!super::covers(
            &bleed(7, Some(Target::Seat { seat: 2 })),
            &now
        ));
    }
}
