//! What happened between decisions: the engine runs every window nobody
//! can act in at once, so an action can resolve, a combat end or a
//! referendum close with no decision to tell. The engine notes it in
//! order (`zvtes::notes`), the table's ledger between its notes with
//! the cause of each change; the session tells the notes and the
//! difference of each two ledgers in a row as facts in the engine's ids
//! (the host names them for each viewer: `events.rs`): how an action or
//! a referendum ended, pool and blood moved, minions and cards in and
//! out of play, torpor, the Edge. What only a decision shows (a block
//! attempt given up, a strike declared) is read from its marks. Every
//! fact told is public.
//!
//! Every change is a fact; `told` marks the ones a log tells in words
//! (a decision's own effects are its option's to say, unless the option
//! does not: a rider, an ability).
//!
//! The marks read the engine alone (and its census for copies), not a
//! view.

use std::collections::{HashMap, HashSet};

use zvtes::actions::{ActionCtx, Outcome};
use zvtes::cards::{AbilitySource, StrikeKind};
use zvtes::combat::CombatNote;
use zvtes::copies::{minion_copy, CopyId, Place};
use zvtes::decision::ActionKind;
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::notes::{Cause, Ledger};
use zvtes::referendum::RefKind;
use zvtes::state::CardId;

/// What happened, in the engine's ids.
#[derive(Clone, Debug, PartialEq)]
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
        cause: Option<Cause>,
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
        cause: Option<Cause>,
    },
    CardEnters {
        card: CardId,
        on: Option<MinionId>,
    },
    /// To `to`, or contested: in its own contest, or out of play with
    /// its contested minion (rules-qa 83.1).
    CardLeaves {
        card: CardId,
        to: Option<Place>,
        contested: bool,
    },
    CardMoves {
        card: CardId,
        on: Option<MinionId>,
    },
    BlockFails {
        minion: MinionId,
    },
    /// A strike declared (`None`: no strike), from a card or weapon, and
    /// what it was told to inflict as declared.
    Strike {
        minion: MinionId,
        strike: Option<StrikeKind>,
        source: Option<CardId>,
        told: Option<zvtes::combat::StrikeHit>,
    },
    /// An action announced (`Note::Action`).
    ActionStarts(ActionFact),
    /// Another minion now acting in the action under way.
    ActingChanges {
        seat: SeatId,
        minion: MinionId,
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
    /// A crypt's or library's card revealed to all (`Note::Reveal`).
    Revealed {
        seat: SeatId,
        card: u16,
        crypt: bool,
    },
    /// A play resolved or canceled (`Note::Play`).
    Played(zvtes::game::PlayNote),
    /// A hand looked at, for its viewer only (`Note::Look`).
    Looked(zvtes::game::Look),
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
    pub(crate) fn of(a: &ActionCtx) -> ActionFact {
        ActionFact {
            seat: a.seat,
            acting: a.acting,
            kind: a.kind,
            card: a.card.as_ref().map(|p| (p.card, *p.copy)),
            source: a.source.filter(|&s| s != AbilitySource::Vampire(a.acting)),
        }
    }
}

/// A fact, whether a log tells it in words, and the minions as it found
/// them.
#[derive(Clone, Debug, PartialEq)]
pub struct Told {
    pub fact: Fact,
    pub told: bool,
    pub faces: Faces,
}

impl Told {
    /// Its minions' faces set by `Session::observe`, for the batch.
    pub(crate) fn new(fact: Fact, told: bool) -> Told {
        Told {
            fact,
            told,
            faces: Faces::default(),
        }
    }
}

/// Each minion's copy and face (card, crypt) by `MinionId`, when a fact
/// was told: facts are named at the next view, and a merge before it
/// changes both (an event naming the merged copy would name a minion the
/// viewer never saw).
pub type Faces = std::sync::Arc<[(CopyId, u16, bool)]>;

pub(crate) fn faces(e: &Engine) -> Faces {
    (0..e.state.minions.len())
        .map(|i| {
            let (card, crypt) = crate::view::minion_face(&e.state.minions[i]);
            (minion_copy(&e.state, i), card, crypt)
        })
        .collect()
}

fn tell(out: &mut Vec<Told>, fact: Fact, told: bool) {
    out.push(Told::new(fact, told));
}

/// The table at a decision, as far as events tell it: the engine's
/// ledger, the action under way, and what only a decision shows (the
/// strikes declared).
#[derive(Clone, Debug, Default)]
pub(crate) struct Mark {
    pub(crate) ledger: Ledger,
    pub(crate) action: Option<ActionFact>,
    /// The combat's strikes declared: per side, its minion and the round
    /// and pair it declared in, and what (`None`: not declared yet).
    strikes: Vec<(MinionId, u8, u8, Option<Declared>)>,
}

/// A strike declared: what, from which card, told as what damage.
#[derive(Clone, Copy, Debug)]
struct Declared {
    strike: Option<StrikeKind>,
    source: Option<CardId>,
    told: Option<zvtes::combat::StrikeHit>,
}

impl Mark {
    pub(crate) fn of(e: &Engine) -> Mark {
        Mark {
            ledger: e.table(),
            action: e.action().map(ActionFact::of),
            strikes: e
                .combat()
                .map(|c| {
                    c.sides
                        .iter()
                        .enumerate()
                        .map(|(i, s)| {
                            let declared = s.chosen.then(|| Declared {
                                strike: s.strike,
                                source: s.strike_source,
                                told: s
                                    .strike
                                    .filter(|_| s.strike_at.is_none())
                                    .and_then(|k| c.strike_told(&e.state, e.action(), i, k)),
                            });
                            (s.minion, c.round, c.pair, declared)
                        })
                        .collect()
                })
                .unwrap_or_default(),
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
                    told: d.told,
                };
                tell(out, fact, true);
            }
        }
    }
}

/// Per seat, the copies in its library, crypt, hand and uncontrolled
/// region: one from the library in the hand next is a draw.
fn piles(l: &Ledger) -> [Vec<HashSet<CopyId>>; 4] {
    let n = l.pools.len();
    let mut piles = std::array::from_fn(|_| vec![HashSet::new(); n]);
    for &(copy, _, place) in &l.census {
        let (s, pile) = match place {
            Place::Zone(s, "library") => (s, 0),
            Place::Zone(s, "crypt") => (s, 1),
            Place::Zone(s, "hand") => (s, 2),
            Place::Uncontrolled(s) => (s, 3),
            _ => continue,
        };
        piles[pile][s as usize].insert(copy);
    }
    piles
}

/// What changed on the table from `before` to `after`, made by `cause`;
/// `told`: in words too (across the engine's run, or a decision whose
/// option does not tell its effects); `open`: the action under way, a
/// political referendum's.
pub(crate) fn changes(
    e: &Engine,
    (before, after): (&Ledger, &Ledger),
    cause: Option<Cause>,
    told: bool,
    open: Option<ActionFact>,
    out: &mut Vec<Told>,
) {
    if after.turn.0 != before.turn.0 {
        let fact = Fact::Turn {
            turn: after.turn.0,
            active: after.turn.1,
        };
        tell(out, fact, true);
    }
    // A block attempt that ended with no block (short of the stealth,
    // or given up): the action goes on, or ends unblocked.
    if let Some(m) = before.attempting {
        if after.attempting != Some(m) && after.blocker != Some(m) {
            tell(out, Fact::BlockFails { minion: m }, true);
        }
    }
    if let (Some(r), None) = (&before.referendum, &after.referendum) {
        let (f, a) = r.votes;
        let political = r.kind == RefKind::Political;
        let fact = Fact::ReferendumEnds {
            // A political action's result, when the action outlives it.
            passed: after
                .passed
                .filter(|_| political)
                .unwrap_or(f > a && !r.forced_fail),
            canceled: r.canceled,
            votes: r.votes,
            action: open.filter(|_| political),
            hunted: match r.kind {
                RefKind::BloodHunt { diablerist } => Some(diablerist),
                RefKind::Political => None,
            },
        };
        tell(out, fact, true);
    }
    let (from, to) = (piles(before), piles(after));
    let drawn =
        |from: &[HashSet<CopyId>], to: &[HashSet<CopyId>], s: usize| match (from.get(s), to.get(s))
        {
            (Some(b), Some(a)) => a.iter().filter(|c| b.contains(c)).count(),
            _ => 0,
        };
    for s in 0..after.pools.len() {
        for (crypt, n) in [
            (false, drawn(&from[0], &to[2], s)),
            (true, drawn(&from[1], &to[3], s)),
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
    // Where a card that left play went, by its copy.
    let mut places: Option<HashMap<CopyId, Place>> = None;
    let mut place = |c: CopyId| {
        places
            .get_or_insert_with(|| after.census.iter().map(|&(c, _, p)| (c, p)).collect())
            .get(&c)
            .copied()
    };
    for (s, (&was, &now)) in before.pools.iter().zip(&after.pools).enumerate() {
        if was != now && !before.ousted[s] {
            let fact = Fact::Pool {
                seat: SeatId(s as u8),
                delta: now - was,
                pool: now,
                cause,
            };
            tell(out, fact, told);
        }
    }
    for (s, (&was, &now)) in before.ousted.iter().zip(&after.ousted).enumerate() {
        if now && !was {
            let fact = Fact::Ousted {
                seat: SeatId(s as u8),
            };
            tell(out, fact, told);
        }
    }
    // The Edge changing hands is told always: no option says it.
    if after.edge != before.edge {
        tell(out, Fact::Edge { seat: after.edge }, true);
    }
    // A Methuselah ousted takes their table along: told once.
    let gone = |s: SeatId| after.ousted[s.index()] && !before.ousted[s.index()];
    for (i, now) in after.minions.iter().enumerate() {
        let m = MinionId(i as u32);
        if gone(e.state.minions[i].owner) {
            continue;
        }
        let (present, torpor, blood) = *now;
        let Some(&(was, was_torpor, was_blood)) = before.minions.get(i) else {
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
                cause,
            };
            tell(out, fact, told);
        }
    }
    for (i, now) in after.cards.iter().enumerate() {
        if gone(e.state.cards[i].controller) {
            continue;
        }
        let c = CardId(i as u32);
        let was = before.cards.get(i).copied().flatten();
        match (was, *now) {
            (None, Some((_, host))) => {
                tell(out, Fact::CardEnters { card: c, on: host }, told);
            }
            (Some(_), None) => {
                let card = &e.state.cards[i];
                let fact = Fact::CardLeaves {
                    card: c,
                    to: place(*card.copy),
                    contested: card.in_play
                        && (card.contested
                            || card
                                .host
                                .is_some_and(|m| e.state.minions[m.index()].contested)),
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

#[cfg(test)]
mod tests {
    use zvtes::cards::Target;
    use zvtes::combat::StrikeHit;
    use zvtes::crypt::CRYPT;
    use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
    use zvtes::game::{Engine, Step};
    use zvtes::ids::SeatId;
    use zvtes::state::{find_card, find_library, CardInPlay};

    use super::Mark;

    fn next(e: &mut Engine) -> DecisionRequest {
        match e.run() {
            Step::Decision(r) => r,
            Step::Over(_) => panic!("game over"),
        }
    }

    /// Passes (or the first option) until `done` holds of a decision.
    fn until(e: &mut Engine, done: impl Fn(&DecisionRequest) -> Option<usize>) {
        loop {
            let r = next(e);
            if let Some(i) = done(&r) {
                e.choose(i);
                return;
            }
            let pass = r.options.iter().position(|o| *o == Choice::Pass);
            e.choose(pass.unwrap_or(0));
        }
    }

    fn pick(e: &mut Engine, want: Choice) {
        until(e, |r| r.options.iter().position(|o| *o == want));
    }

    // A strike declared is marked with what it inflicts as declared (the
    // `strike_declared` event's `n`): Torn Signpost [POT] sets a strength of
    // 3, the hand strike for 3.
    /// The cards on a minion going into a contest are told leaving
    /// contested with it (rules-qa 83.1).
    #[test]
    fn a_card_on_a_contested_minion_is_told_leaving_contested() {
        let mut e = Engine::bare(2);
        let m = e.add_minion(SeatId(0), find_card("Farah Sarroub"), 4, false);
        e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
        let laptop = find_library("Laptop Computer");
        e.state
            .cards
            .push(CardInPlay::new(laptop, SeatId(0), Some(m), 0));
        let card = e.state.cards.len() - 1;
        let before = Mark::of(&e);
        e.state.minions[m.index()].contested = true;
        e.state.cards[card].out_of_play = true;
        let mut out = Vec::new();
        let after = Mark::of(&e).ledger;
        super::changes(&e, (&before.ledger, &after), None, true, None, &mut out);
        assert!(
            out.iter().any(|t| matches!(
                t.fact,
                super::Fact::CardLeaves {
                    contested: true,
                    ..
                }
            )),
            "{out:?}"
        );
    }

    #[test]
    fn a_declared_strike_is_marked_with_its_damage() {
        let pot = CRYPT
            .iter()
            .enumerate()
            .find(|(i, c)| {
                c.disciplines.contains(&"POT") && zvtes::cards::vampire(*i as u16).is_none()
            })
            .map(|(i, _)| i as u16)
            .expect("a vampire with POT");
        let mut e = Engine::bare(2);
        let a = e.add_minion(SeatId(0), pot, 4, false);
        e.add_minion(SeatId(0), find_card("Farah Sarroub"), 4, false);
        e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
        let z = e.add_minion(SeatId(1), find_card("Octane"), 4, false);
        e.state.turn = 4;
        // The blocker's weapon: a strike to choose, so its window stays.
        let femur = find_library("Femur of Toomler");
        e.state
            .cards
            .push(CardInPlay::new(femur, SeatId(1), Some(z), 0));
        e.state.seats[0].hand.push(find_library("Torn Signpost"));
        pick(
            &mut e,
            Choice::TakeAction {
                minion: a,
                action: ActionKind::Bleed,
                target: None,
            },
        );
        pick(&mut e, Choice::DeclareBlock { minion: z });
        pick(
            &mut e,
            Choice::Play {
                card: 0,
                mode: 1,
                actor: Some(a),
                target: Target::None,
            },
        );
        // Up to the blocker's strike choice, the acting minion's declared.
        loop {
            let r = next(&mut e);
            if (r.seat, r.window) == (SeatId(1), WindowLabel::CombatStrike) {
                break;
            }
            let pass = r.options.iter().position(|o| *o == Choice::Pass);
            e.choose(pass.unwrap_or(0));
        }
        let (_, _, _, declared) = Mark::of(&e).strikes[0];
        assert_eq!(
            declared.and_then(|d| d.told),
            Some(StrikeHit {
                n: 3,
                aggravated: false
            })
        );
    }
}
