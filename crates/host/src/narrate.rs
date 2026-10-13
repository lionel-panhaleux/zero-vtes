//! What happened between decisions: the engine runs every window nobody
//! can act in at once, so an action can resolve, a combat end or a
//! referendum close with no decision to tell. The session marks the table
//! after each decision and again when the next one is asked; the
//! difference is told as facts in the engine's ids (the host names them
//! for each viewer: `events.rs`): how an action or a referendum ended,
//! pool and blood moved, minions and cards in and out of play, torpor,
//! the Edge. Everything told is public.
//!
//! Every change is a fact; `told` marks the ones a log tells in words
//! (a decision's own effects are its option's to say, unless the option
//! does not: a rider, an ability).
//!
//! The marks read the engine alone (and `census` for copies), not a view.

use std::collections::{HashMap, HashSet};

use zvtes::actions::{ActionCtx, ActionState, Outcome};
use zvtes::cards::{AbilitySource, StrikeKind};
use zvtes::combat::CombatNote;
use zvtes::copies::{census, minion_copy, CopyId, Place};
use zvtes::decision::ActionKind;
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::referendum::RefKind;
use zvtes::state::CardId;

/// What happened, in the engine's ids.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fact {
    /// A decision taken here (the host tells it from its answer).
    Decided(u64),
    Turn {
        turn: u32,
        active: SeatId,
    },
    Draw {
        seat: SeatId,
        n: u8,
        crypt: bool,
    },
    Pool {
        seat: SeatId,
        delta: i32,
        pool: i32,
    },
    Ousted {
        seat: SeatId,
    },
    /// The Edge's holder now (`None`: burned, no one holds it).
    Edge {
        seat: Option<SeatId>,
    },
    MinionEnters {
        minion: MinionId,
    },
    /// Burned (to `to`), or contested.
    MinionLeaves {
        minion: MinionId,
        contested: bool,
        to: Option<Place>,
    },
    Torpor {
        minion: MinionId,
        enters: bool,
    },
    Blood {
        minion: MinionId,
        delta: i32,
        blood: u8,
    },
    CardEnters {
        card: CardId,
        on: Option<MinionId>,
    },
    CardLeaves {
        card: CardId,
        to: Option<Place>,
    },
    CardMoves {
        card: CardId,
        on: Option<MinionId>,
    },
    BlockFails {
        minion: MinionId,
    },
    /// A strike declared (`None`: no strike), from a card or weapon.
    Strike {
        minion: MinionId,
        strike: Option<StrikeKind>,
        source: Option<CardId>,
    },
    ActionEnds {
        action: ActionFact,
        /// `None`: ended otherwise (its acting minion gone).
        outcome: Option<Outcome>,
        /// Its Methuselah was ousted.
        ousted: bool,
        blocker: Option<MinionId>,
    },
    ReferendumEnds {
        passed: bool,
        canceled: bool,
        votes: (u32, u32),
        action: Option<ActionFact>,
        /// A blood hunt's diablerist.
        hunted: Option<MinionId>,
    },
    Combat(CombatNote),
}

/// An action as its end tells it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionFact {
    pub seat: SeatId,
    pub acting: MinionId,
    pub kind: ActionKind,
    /// The action card and its copy.
    pub card: Option<(u16, CopyId)>,
    /// What grants it: a card in play, another vampire.
    pub source: Option<AbilitySource>,
}

impl ActionFact {
    fn of(a: &ActionCtx) -> ActionFact {
        ActionFact {
            seat: a.seat,
            acting: a.acting,
            kind: a.kind,
            card: a.card.as_ref().map(|p| (p.card, *p.copy)),
            source: a.source.filter(|&s| s != AbilitySource::Vampire(a.acting)),
        }
    }
}

/// A fact, and whether a log tells it in words.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Told {
    pub fact: Fact,
    pub told: bool,
}

fn tell(out: &mut Vec<Told>, fact: Fact, told: bool) {
    out.push(Told { fact, told });
}

/// The table at a decision, as far as events tell it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Mark {
    pools: Vec<i32>,
    ousted: Vec<bool>,
    edge: Option<SeatId>,
    /// Per minion id: present, in torpor, blood.
    minions: Vec<(bool, bool, u8)>,
    /// Per card id: in play (an ally is its minion) and on which minion.
    cards: Vec<Option<(u16, Option<MinionId>)>>,
    action: Option<ActionMark>,
    referendum: Option<RefMark>,
    /// Game turn and active seat.
    turn: (u32, u8),
    /// Per seat, the copies in its library, crypt, hand and uncontrolled
    /// region: one from the library in the hand next is a draw.
    library: Vec<HashSet<CopyId>>,
    crypt: Vec<HashSet<CopyId>>,
    hand: Vec<HashSet<CopyId>>,
    uncontrolled: Vec<HashSet<CopyId>>,
    /// The combat's strikes declared: per side, its minion and the round
    /// and pair it declared in, and what (`None`: not declared yet).
    strikes: Vec<(MinionId, u8, u8, Option<Declared>)>,
}

/// A strike declared: what, from which card.
#[derive(Clone, Copy, Debug)]
struct Declared {
    strike: Option<StrikeKind>,
    source: Option<CardId>,
}

#[derive(Clone, Debug, PartialEq)]
struct ActionMark {
    fact: ActionFact,
    /// The minion attempting the block or that blocked, last seen.
    blocker: Option<MinionId>,
    /// The minion attempting to block now, if any.
    attempting: Option<MinionId>,
    /// A political action's referendum passed (read once it is over).
    passed: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
struct RefMark {
    action: Option<ActionFact>,
    hunted: Option<MinionId>,
    votes: (u32, u32),
    forced_fail: bool,
    canceled: bool,
}

impl Mark {
    pub(crate) fn of(e: &Engine) -> Mark {
        let st = &e.state;
        let n = st.seats.len();
        let mut piles = vec![vec![HashSet::new(); n]; 4];
        for (copy, _, place) in census(e) {
            let (s, pile) = match place {
                Place::Zone(s, "library") => (s, 0),
                Place::Zone(s, "crypt") => (s, 1),
                Place::Zone(s, "hand") => (s, 2),
                Place::Uncontrolled(s) => (s, 3),
                _ => continue,
            };
            piles[pile][s as usize].insert(copy);
        }
        let [library, crypt, hand, uncontrolled] = <[_; 4]>::try_from(piles).expect("four piles");
        Mark {
            turn: (st.turn, st.active.0),
            library,
            crypt,
            hand,
            uncontrolled,
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
            action: e.action().map(|a| ActionMark {
                fact: ActionFact::of(a),
                blocker: match a.state {
                    ActionState::Attempt(m) | ActionState::Blocked(m) => Some(m),
                    _ => None,
                }
                .or(a.blocker),
                attempting: match a.state {
                    ActionState::Attempt(m) => Some(m),
                    _ => None,
                },
                passed: (a.kind == ActionKind::Political && e.referendum().is_none())
                    .then_some(a.referendum_passed),
            }),
            strikes: e
                .combat()
                .map(|c| {
                    c.sides
                        .iter()
                        .map(|s| {
                            let declared = s.chosen.then_some(Declared {
                                strike: s.strike,
                                source: s.strike_source,
                            });
                            (s.minion, c.round, c.pair, declared)
                        })
                        .collect()
                })
                .unwrap_or_default(),
            referendum: e.referendum().map(|r| {
                let (action, hunted) = match r.kind {
                    RefKind::BloodHunt { diablerist } => (None, Some(diablerist)),
                    RefKind::Political => (e.action().map(ActionFact::of), None),
                };
                RefMark {
                    action,
                    hunted,
                    votes: r.tally(),
                    forced_fail: r.forced_fail,
                    canceled: r.canceled,
                }
            }),
        }
    }

    /// The same action, still under way.
    fn same_action(a: &ActionMark, b: &ActionMark) -> bool {
        let key = |f: &ActionFact| (f.seat, f.acting, f.kind, f.card.map(|c| c.0));
        key(&a.fact) == key(&b.fact)
    }

    /// A block attempt that ended with no block (short of the stealth,
    /// or given up): the action goes on, or ends without being blocked
    /// in the same run (a bleed resolved at once).
    pub(crate) fn attempts(&self, e: &Engine, now: &Mark, out: &mut Vec<Told>) {
        let Some(a) = &self.action else {
            return;
        };
        let Some(m) = a.attempting else {
            return;
        };
        let failed = match &now.action {
            Some(b) if Self::same_action(a, b) => b.attempting != Some(m) && b.blocker != Some(m),
            _ => e.last_outcome != Some(Outcome::Blocked),
        };
        if failed {
            tell(out, Fact::BlockFails { minion: m }, true);
        }
    }

    /// The strikes declared since (the acting minion's first: the
    /// blocker chooses knowing it), each told once.
    pub(crate) fn strikes(&self, now: &Mark, out: &mut Vec<Told>) {
        for &(m, round, pair, declared) in &now.strikes {
            let Some(d) = declared else { continue };
            let told = self
                .strikes
                .iter()
                .any(|&(b, r, p, s)| b == m && r == round && p == pair && s.is_some());
            if !told {
                let fact = Fact::Strike {
                    minion: m,
                    strike: d.strike,
                    source: d.source,
                };
                tell(out, fact, true);
            }
        }
    }

    /// How the action and referendum seen at `self` ended by `now`.
    pub(crate) fn endings(&self, e: &Engine, now: &Mark, out: &mut Vec<Told>) {
        if let Some(r) = &self.referendum {
            if now.referendum.is_none() {
                let (f, a) = r.votes;
                // A political action's result, when the action outlives it.
                let passed = now
                    .action
                    .as_ref()
                    .and_then(|a| a.passed)
                    .unwrap_or(f > a && !r.forced_fail);
                let fact = Fact::ReferendumEnds {
                    passed,
                    canceled: r.canceled,
                    votes: r.votes,
                    action: r.action,
                    hunted: r.hunted,
                };
                tell(out, fact, true);
            }
        }
        if let Some(a) = &self.action {
            if now.action.as_ref().is_some_and(|b| Self::same_action(a, b)) {
                return;
            }
            let ousted = e.state.seats[a.fact.seat.index()].ousted;
            let outcome = e.last_outcome;
            let fact = Fact::ActionEnds {
                action: a.fact,
                outcome: outcome.filter(|_| !ousted),
                ousted,
                blocker: a.blocker.filter(|_| outcome == Some(Outcome::Blocked)),
            };
            tell(out, fact, true);
        }
    }

    /// A new turn, and the cards drawn from `self` to `now` (told whatever
    /// drew them: a decision's own draws too).
    pub(crate) fn draws(&self, now: &Mark, out: &mut Vec<Told>) {
        if now.turn.0 != self.turn.0 {
            let fact = Fact::Turn {
                turn: now.turn.0,
                active: SeatId(now.turn.1),
            };
            tell(out, fact, true);
        }
        let drawn = |from: &[HashSet<CopyId>], to: &[HashSet<CopyId>], s: usize| match (
            from.get(s),
            to.get(s),
        ) {
            (Some(b), Some(a)) => a.iter().filter(|c| b.contains(c)).count(),
            _ => 0,
        };
        for s in 0..now.hand.len() {
            for (crypt, n) in [
                (false, drawn(&self.library, &now.hand, s)),
                (true, drawn(&self.crypt, &now.uncontrolled, s)),
            ] {
                if n == 0 {
                    continue;
                }
                let fact = Fact::Draw {
                    seat: SeatId(s as u8),
                    n: n.min(255) as u8,
                    crypt,
                };
                tell(out, fact, true);
            }
        }
    }

    /// What changed on the table from `self` to `now`; `told`: in words
    /// too (across the engine's run, or a decision whose option does not
    /// tell its effects).
    pub(crate) fn changes(&self, e: &Engine, now: &Mark, told: bool, out: &mut Vec<Told>) {
        // Where a card that left play went, by its copy.
        let mut places: Option<HashMap<CopyId, Place>> = None;
        let mut place = |c: CopyId| {
            places
                .get_or_insert_with(|| census(e).into_iter().map(|(c, _, p)| (c, p)).collect())
                .get(&c)
                .copied()
        };
        for (s, (&before, &after)) in self.pools.iter().zip(&now.pools).enumerate() {
            if before != after && !self.ousted[s] {
                let fact = Fact::Pool {
                    seat: SeatId(s as u8),
                    delta: after - before,
                    pool: after,
                };
                tell(out, fact, told);
            }
        }
        for (s, (&before, &after)) in self.ousted.iter().zip(&now.ousted).enumerate() {
            if after && !before {
                let fact = Fact::Ousted {
                    seat: SeatId(s as u8),
                };
                tell(out, fact, told);
            }
        }
        // The Edge changing hands is told always: no option says it.
        if now.edge != self.edge {
            tell(out, Fact::Edge { seat: now.edge }, true);
        }
        // A Methuselah ousted takes their table along: told once.
        let gone = |s: SeatId| now.ousted[s.index()] && !self.ousted[s.index()];
        for (i, after) in now.minions.iter().enumerate() {
            let m = MinionId(i as u32);
            if gone(e.state.minions[i].owner) {
                continue;
            }
            let (present, torpor, blood) = *after;
            let Some(&(was, was_torpor, was_blood)) = self.minions.get(i) else {
                if present {
                    tell(out, Fact::MinionEnters { minion: m }, told);
                }
                continue;
            };
            match (was, present) {
                (false, true) => {
                    tell(out, Fact::MinionEnters { minion: m }, told);
                }
                (true, false) => {
                    let fact = Fact::MinionLeaves {
                        minion: m,
                        contested: e.state.minions[i].contested,
                        to: place(minion_copy(&e.state, i)),
                    };
                    tell(out, fact, told);
                    continue;
                }
                (false, false) => continue,
                (true, true) => {}
            }
            if torpor != was_torpor && was {
                let fact = Fact::Torpor {
                    minion: m,
                    enters: torpor,
                };
                tell(out, fact, told);
            }
            if blood != was_blood && was {
                let fact = Fact::Blood {
                    minion: m,
                    delta: i32::from(blood) - i32::from(was_blood),
                    blood,
                };
                tell(out, fact, told);
            }
        }
        for (i, after) in now.cards.iter().enumerate() {
            if gone(e.state.cards[i].controller) {
                continue;
            }
            let c = CardId(i as u32);
            let before = self.cards.get(i).copied().flatten();
            match (before, *after) {
                (None, Some((_, host))) => {
                    tell(out, Fact::CardEnters { card: c, on: host }, told);
                }
                (Some(_), None) => {
                    let fact = Fact::CardLeaves {
                        card: c,
                        to: place(*e.state.cards[i].copy),
                    };
                    tell(out, fact, told);
                }
                (Some((_, from)), Some((_, to))) if from != to => {
                    tell(out, Fact::CardMoves { card: c, on: to }, told);
                }
                _ => {}
            }
        }
    }
}
