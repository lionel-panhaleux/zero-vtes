//! What happened, in order, for a narrator (`Engine::notes`): the
//! engine's own notes (actions, plays, reveals, looks, combats) and, at
//! each of them and at each cause's bounds, the table as it then stood
//! (`Ledger`) with what changed it (`Cause`). A narrator diffs two
//! ledgers in a row; nothing reads the notes inside the engine.

use crate::actions::{ActionCtx, ActionNote, ActionState};
use crate::cards::AbilitySource;
use crate::combat::CombatNote;
use crate::copies::{census, CopyId, Place};
use crate::game::{Engine, Look, PlayNote, Reveal};
use crate::ids::{MinionId, SeatId};
use crate::referendum::RefKind;
use crate::state::CardId;

#[derive(Clone, Debug)]
pub enum Note {
    Action(ActionNote),
    Combat(CombatNote),
    Reveal(Reveal),
    Play(PlayNote),
    Look(Look),
    /// Another minion now acting in the action under way (Mask of a
    /// Thousand Faces): its end is theirs.
    Acting {
        seat: SeatId,
        minion: MinionId,
    },
    /// The table after the changes since the ledger before, made by
    /// `cause` (`None`: by the rules, unscoped).
    Ledger {
        ledger: Box<Ledger>,
        cause: Option<Cause>,
    },
}

/// What changed the table: a cost paid or an effect, by its source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Cost(Source),
    Effect(Source),
}

/// Where a cost or an effect comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// A card played, its copy (from the hand, or by another card).
    Played { card: u16, copy: CopyId },
    /// A card in play.
    Card(CardId),
    /// A minion: its action, its printed ability, the damage it inflicts.
    Minion(MinionId),
    /// The game's rules (a hunt, the oust's reward, a transfer, a
    /// diablerie, blood above capacity).
    Rule(Rule),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Combat damage: a pair of strikes resolving (`CombatNote::Strikes`
    /// before it).
    Strikes,
    Transfer,
    /// An oust: the predator's 6 pool, the ousted Methuselah's pool gone.
    Oust,
    /// The Edge burned for its pool.
    Edge,
    /// A contested card or title paid for.
    Contest,
    /// A disease counter's blood, as its vampire unlocks (Vampiric
    /// Disease).
    Disease,
    Diablerie,
    Capacity,
    /// Pool burned for votes (Business Pressure's round).
    Vote,
}

impl From<AbilitySource> for Source {
    fn from(a: AbilitySource) -> Source {
        match a {
            AbilitySource::Card(c) => Source::Card(c),
            AbilitySource::Vampire(m) => Source::Minion(m),
        }
    }
}

impl ActionCtx {
    /// What the action is of: its card, the card or vampire granting it,
    /// or its acting minion.
    pub(crate) fn cause_source(&self) -> Source {
        match (&self.card, self.source) {
            (Some(p), _) => Source::Played {
                card: p.card,
                copy: *p.copy,
            },
            (None, Some(s)) => s.into(),
            (None, None) => Source::Minion(self.acting),
        }
    }
}

/// The table, as far as a narrator tells it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ledger {
    /// Game turn and active seat.
    pub turn: (u32, SeatId),
    pub pools: Vec<i32>,
    pub ousted: Vec<bool>,
    pub edge: Option<SeatId>,
    /// Per minion id: present, in torpor, blood.
    pub minions: Vec<(bool, bool, u8)>,
    /// Per card id: in play (an ally is its minion) and on which minion.
    pub cards: Vec<Option<(u16, Option<MinionId>)>>,
    /// Every copy and where it is.
    pub census: Vec<(CopyId, u16, Place)>,
    pub referendum: Option<RefLedger>,
    /// The action under way's referendum passed, once it is over (a
    /// political action's).
    pub passed: Option<bool>,
    /// The minion attempting to block the action under way.
    pub attempting: Option<MinionId>,
    /// The minion attempting to block or that blocked it.
    pub blocker: Option<MinionId>,
}

/// A referendum under way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefLedger {
    pub kind: RefKind,
    /// For and against, so far.
    pub votes: (u32, u32),
    pub forced_fail: bool,
    pub canceled: bool,
}

impl Engine {
    /// The table now (`Ledger`).
    pub fn table(&self) -> Ledger {
        let st = &self.state;
        Ledger {
            turn: (st.turn, st.active),
            pools: st.seats.iter().map(|s| s.pool).collect(),
            ousted: st.seats.iter().map(|s| s.ousted).collect(),
            edge: st
                .seats
                .iter()
                .position(|s| s.has_edge)
                .map(|i| SeatId(i as u8)),
            minions: st
                .minions
                .iter()
                .map(|m| (m.present(), m.in_torpor, m.blood))
                .collect(),
            cards: st
                .cards
                .iter()
                .map(|c| (c.present() && c.body.is_none()).then_some((c.card, c.host)))
                .collect(),
            census: census(self),
            referendum: self.referendum().map(|r| RefLedger {
                kind: r.kind,
                votes: r.tally(st),
                forced_fail: r.forced_fail,
                canceled: r.canceled,
            }),
            passed: self
                .action()
                .filter(|a| {
                    a.kind == crate::decision::ActionKind::Political && self.referendum().is_none()
                })
                .map(|a| a.referendum_passed),
            attempting: self.action().and_then(|a| match a.state {
                ActionState::Attempt(m) => Some(m),
                _ => None,
            }),
            blocker: self.action().and_then(|a| match a.state {
                ActionState::Attempt(m) | ActionState::Blocked(m) => Some(m),
                _ => a.blocker,
            }),
        }
    }

    /// `n` noted, after the table as it stood.
    pub(crate) fn push_note(&mut self, n: Note) {
        if self.notes.is_some() {
            self.checkpoint();
            self.notes.as_mut().expect("noting").push(n);
        }
    }

    /// The table noted, if it changed since the last ledger noted.
    pub(crate) fn checkpoint(&mut self) {
        if self.notes.is_none() {
            return;
        }
        let ledger = self.table();
        if self.noted.as_deref() != Some(&ledger) {
            let ledger = Box::new(ledger);
            self.noted = Some(ledger.clone());
            let cause = self.cause;
            self.notes
                .as_mut()
                .expect("noting")
                .push(Note::Ledger { ledger, cause });
        }
    }

    /// `f` run with the table's changes noted as made by `cause`.
    pub(crate) fn caused<T>(&mut self, cause: Cause, f: impl FnOnce(&mut Self) -> T) -> T {
        if self.notes.is_none() {
            return f(self);
        }
        let outer = self.begin_cause(cause);
        let out = f(self);
        self.end_cause(outer);
        out
    }

    /// `f` run as the cost of what is under way (the source of the
    /// effect noted).
    pub(crate) fn paying<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        match self.cause {
            Some(Cause::Effect(s) | Cause::Cost(s)) => self.caused(Cause::Cost(s), f),
            None => f(self),
        }
    }

    /// The changes from here noted as made by `cause`, until `end_cause`
    /// with what this returns (a stretch with no early return; `caused`
    /// otherwise).
    pub(crate) fn begin_cause(&mut self, cause: Cause) -> Option<Cause> {
        if self.notes.is_none() {
            return self.cause;
        }
        self.checkpoint();
        self.tag_interrupts();
        self.cause.replace(cause)
    }

    pub(crate) fn end_cause(&mut self, outer: Option<Cause>) {
        if self.notes.is_none() {
            return;
        }
        self.checkpoint();
        self.tag_interrupts();
        self.cause = outer;
    }

    /// The interrupts queued under the cause now (none included), kept
    /// with it as a cause's bounds pass: their answers are its too
    /// (`interrupt_cause`).
    fn tag_interrupts(&mut self) {
        let queued = &self.interrupts;
        self.interrupt_causes.retain(|(i, _)| queued.contains(i));
        for &i in queued {
            if !self.interrupt_causes.iter().any(|(j, _)| *j == i) {
                self.interrupt_causes.push((i, self.cause));
            }
        }
    }

    /// The changes from here noted as the front interrupt's (what it was
    /// queued under; `outer` if it was not seen queued).
    pub(crate) fn cause_front(&mut self, outer: Option<Cause>) {
        if self.notes.is_some() {
            self.checkpoint();
            self.tag_interrupts();
            self.cause = self.interrupt_cause().unwrap_or(outer);
        }
    }

    /// What the front interrupt was queued under, if seen queued.
    pub(crate) fn interrupt_cause(&self) -> Option<Option<Cause>> {
        let front = self.interrupts.first()?;
        self.interrupt_causes
            .iter()
            .find(|(i, _)| i == front)
            .map(|&(_, c)| c)
    }

    /// The notes so far, drained, the table noted last (`None`: not
    /// noting).
    pub fn take_notes(&mut self) -> Option<Vec<Note>> {
        self.checkpoint();
        self.notes_drained += 1;
        self.notes.as_mut().map(std::mem::take)
    }
}
