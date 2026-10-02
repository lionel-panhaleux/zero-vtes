//! What happened between decisions (docs/human-play.md 2.18): the engine
//! runs every window nobody can act in at once, so an action can resolve,
//! a combat end or a referendum close with no decision to narrate. The
//! session marks the table after each decision and again when the next
//! one is asked; the difference is told as events: how an action or a
//! referendum ended, pool and blood moved, minions and cards in and out
//! of play, torpor. Everything told is public.
//!
//! An action's and a referendum's end are read at every mark — the
//! decision itself may end them — the rest only across the engine's run,
//! a decision's own effect being its text.

use zvtes::actions::{ActionCtx, Outcome};
use zvtes::cards::AbilitySource;
use zvtes::decision::ActionKind;
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::referendum::RefKind;
use zvtes::state::MinionKind;

use crate::describe::action_kind;

/// The table at a decision, as far as events tell it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Mark {
    pools: Vec<i32>,
    ousted: Vec<bool>,
    /// Per minion id: present, in torpor, blood.
    minions: Vec<(bool, bool, u8)>,
    /// Per card id: in play (an ally is its minion) and on which minion.
    cards: Vec<Option<(u16, Option<MinionId>)>>,
    action: Option<ActionMark>,
    referendum: Option<RefMark>,
}

#[derive(Clone, Debug, PartialEq)]
struct ActionMark {
    seat: SeatId,
    acting: MinionId,
    kind: ActionKind,
    card: Option<u16>,
    /// What it is, for a sentence: "Yael's Govern the Unaligned".
    label: String,
    /// The minion attempting the block or that blocked, last seen.
    blocker: Option<MinionId>,
    /// A political action's referendum passed (read once it is over).
    passed: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
struct RefMark {
    label: String,
    votes: (u32, u32),
    forced_fail: bool,
    canceled: bool,
}

pub(crate) fn minion_name(e: &Engine, m: MinionId) -> String {
    let mn = &e.state.minions[m.index()];
    match mn.kind {
        MinionKind::Token { .. } => "a ghoul token".into(),
        _ => mn.name().into(),
    }
}

/// What an action is: its card, what grants it, or its kind ("bleed").
pub(crate) fn action_thing(e: &Engine, a: &ActionCtx) -> String {
    match (&a.card, a.source) {
        (Some(p), _) => LIBRARY[p.card as usize].name.to_string(),
        (None, Some(AbilitySource::Card(c))) => LIBRARY[e.state.cards[c.index()].card as usize]
            .name
            .to_string(),
        (None, Some(AbilitySource::Vampire(m))) if m != a.acting => minion_name(e, m),
        _ => action_kind(a.kind).to_string(),
    }
}

impl Mark {
    pub(crate) fn of(e: &Engine) -> Mark {
        let st = &e.state;
        Mark {
            pools: st.seats.iter().map(|s| s.pool).collect(),
            ousted: st.seats.iter().map(|s| s.ousted).collect(),
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
            action: e.action().map(|a| {
                let thing = match (&a.card, a.source) {
                    (None, Some(AbilitySource::Card(_))) => {
                        format!("{} action", action_thing(e, a))
                    }
                    _ => action_thing(e, a),
                };
                ActionMark {
                    seat: a.seat,
                    acting: a.acting,
                    kind: a.kind,
                    card: a.card.as_ref().map(|p| p.card),
                    label: format!("{}'s {thing}", minion_name(e, a.acting)),
                    blocker: crate::view::blocker(a.state).or(a.blocker),
                    passed: (a.kind == ActionKind::Political && e.referendum().is_none())
                        .then_some(a.referendum_passed),
                }
            }),
            referendum: e.referendum().map(|r| RefMark {
                label: match r.kind {
                    RefKind::BloodHunt { diablerist } => {
                        format!("The blood hunt on {}", minion_name(e, diablerist))
                    }
                    RefKind::Political => match e.action() {
                        Some(a) => format!("The referendum ({})", action_thing(e, a)),
                        None => "The referendum".into(),
                    },
                },
                votes: r.tally(),
                forced_fail: r.forced_fail,
                canceled: r.canceled,
            }),
        }
    }

    /// The same action, still under way.
    fn same_action(a: &ActionMark, b: &ActionMark) -> bool {
        (a.seat, a.acting, a.kind, a.card) == (b.seat, b.acting, b.kind, b.card)
    }

    /// How the action and referendum seen at `self` ended by `now`.
    pub(crate) fn endings(&self, e: &Engine, now: &Mark, names: &[String], out: &mut Vec<String>) {
        if let Some(r) = &self.referendum {
            if now.referendum.is_none() {
                let (f, a) = r.votes;
                // A political action's result, when the action outlives it.
                let passed = now
                    .action
                    .as_ref()
                    .and_then(|a| a.passed)
                    .unwrap_or(f > a && !r.forced_fail);
                out.push(if r.canceled {
                    format!("{} is canceled", r.label)
                } else if passed {
                    format!("{} passes ({f}–{a})", r.label)
                } else {
                    format!("{} fails ({f}–{a})", r.label)
                });
            }
        }
        if let Some(a) = &self.action {
            if now.action.as_ref().is_some_and(|b| Self::same_action(a, b)) {
                return;
            }
            let seat = |s: SeatId| names.get(s.index()).cloned().unwrap_or_default();
            let ended = if e.state.seats[a.seat.index()].ousted {
                format!("{} ends: {} is ousted", a.label, seat(a.seat))
            } else {
                match e.last_outcome {
                    Some(Outcome::Successful) => format!("{} succeeds", a.label),
                    Some(Outcome::Blocked) => match a.blocker {
                        Some(m) => format!("{} is blocked by {}", a.label, minion_name(e, m)),
                        None => format!("{} is blocked", a.label),
                    },
                    Some(Outcome::Failed) => format!("{} fails", a.label),
                    Some(Outcome::Fizzled) => format!("{} fizzles", a.label),
                    None => format!("{} ends", a.label),
                }
            };
            out.push(ended);
        }
    }

    /// What changed on the table from `self` to `now`.
    pub(crate) fn changes(&self, e: &Engine, now: &Mark, names: &[String], out: &mut Vec<String>) {
        let seat = |s: usize| names.get(s).cloned().unwrap_or_default();
        let signed = |d: i32| {
            if d > 0 {
                format!("+{d}")
            } else {
                format!("−{}", -d)
            }
        };
        for (s, (&before, &after)) in self.pools.iter().zip(&now.pools).enumerate() {
            if before != after && !self.ousted[s] {
                out.push(format!(
                    "{}: {} pool ({after})",
                    seat(s),
                    signed(after - before)
                ));
            }
        }
        for (s, (&before, &after)) in self.ousted.iter().zip(&now.ousted).enumerate() {
            if after && !before {
                out.push(format!("{} is ousted", seat(s)));
            }
        }
        // A Methuselah ousted takes their table along: told once.
        let gone = |s: SeatId| now.ousted[s.index()] && !self.ousted[s.index()];
        for (i, after) in now.minions.iter().enumerate() {
            let m = MinionId(i as u32);
            if gone(e.state.minions[i].owner) {
                continue;
            }
            let name = || minion_name(e, m);
            let (present, torpor, blood) = *after;
            let Some(&(was, was_torpor, was_blood)) = self.minions.get(i) else {
                if present {
                    out.push(format!("{} comes into play", name()));
                }
                continue;
            };
            match (was, present) {
                (false, true) => out.push(format!("{} comes into play", name())),
                (true, false) => {
                    let gone = if e.state.minions[i].contested {
                        "is contested"
                    } else {
                        "is burned"
                    };
                    out.push(format!("{} {gone}", name()));
                    continue;
                }
                (false, false) => continue,
                (true, true) => {}
            }
            if torpor != was_torpor && was {
                out.push(if torpor {
                    format!("{} goes to torpor", name())
                } else {
                    format!("{} leaves torpor", name())
                });
            }
            if blood != was_blood && was {
                let d = i32::from(blood) - i32::from(was_blood);
                out.push(format!("{}: {} blood ({blood})", name(), signed(d)));
            }
        }
        for (i, after) in now.cards.iter().enumerate() {
            if gone(e.state.cards[i].controller) {
                continue;
            }
            let before = self.cards.get(i).copied().flatten();
            let on = |host: Option<MinionId>| {
                host.map(|m| format!(" on {}", minion_name(e, m)))
                    .unwrap_or_default()
            };
            match (before, *after) {
                (None, Some((card, host))) => out.push(format!(
                    "{} comes into play{}",
                    LIBRARY[card as usize].name,
                    on(host)
                )),
                (Some((card, _)), None) => {
                    out.push(format!("{} leaves play", LIBRARY[card as usize].name))
                }
                (Some((card, from)), Some((_, to))) if from != to => {
                    out.push(format!("{} moves{}", LIBRARY[card as usize].name, on(to)))
                }
                _ => {}
            }
        }
    }
}
