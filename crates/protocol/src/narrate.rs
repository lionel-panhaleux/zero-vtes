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

use std::collections::HashSet;

use zvtes::actions::{ActionCtx, Outcome};
use zvtes::cards::{AbilitySource, StrikeKind};
use zvtes::combat::{CombatNote, Range};
use zvtes::decision::ActionKind;
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::referendum::RefKind;
use zvtes::state::MinionKind;

use crate::describe::action_kind;
use crate::view::TableView;

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
    /// Game turn and active seat.
    turn: (u32, u8),
    /// Per seat, the copies in its library and in its crypt (every card
    /// shown): one in the hand or the uncontrolled region next is a draw.
    library: Vec<HashSet<u32>>,
    crypt: Vec<HashSet<u32>>,
    /// The combat's strikes declared: per side, its minion and the round
    /// and pair it declared in, and what (`None`: not declared yet).
    strikes: Vec<(MinionId, u8, u8, Option<String>)>,
}

/// A side's declared strike, as the table sees it ("Ivory Bow: 1R
/// damage"); `None` while it has not declared.
pub(crate) fn declared_strike(e: &Engine, side: &zvtes::combat::Side) -> Option<String> {
    if !side.chosen {
        return None;
    }
    let what = strike_text(side.strike);
    Some(match side.strike_source {
        Some(w) => format!(
            "{}: {what}",
            zvtes::library::LIBRARY[e.state.cards[w.index()].card as usize].name
        ),
        None => what,
    })
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
    /// The minion attempting to block now, if any.
    attempting: Option<MinionId>,
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

/// A strike, as the table sees it.
pub(crate) fn strike_text(s: Option<StrikeKind>) -> String {
    match s {
        None => "no strike".into(),
        Some(StrikeKind::Hand { .. }) => "hand strike".into(),
        Some(StrikeKind::Melee { .. }) => "weapon strike".into(),
        Some(StrikeKind::Dodge) => "dodge".into(),
        Some(StrikeKind::CombatEnds { .. }) => "combat ends".into(),
        Some(StrikeKind::Damage { n, ranged, .. }) => {
            format!("{n}{} damage", if ranged { "R" } else { "" })
        }
        Some(StrikeKind::StealBlood { n, ranged }) => {
            format!("steal {n} blood{}", if ranged { ", ranged" } else { "" })
        }
        Some(StrikeKind::TorporOrBurn) => "torpor or burn".into(),
        Some(StrikeKind::Torpor { .. }) => "torpor".into(),
        Some(StrikeKind::CardEffect) => "strike".into(),
        Some(StrikeKind::BurnWeapon) => "burn weapon".into(),
        Some(StrikeKind::StealWeapon) => "steal weapon".into(),
        Some(StrikeKind::BurnEquipment) => "burn equipment".into(),
        Some(StrikeKind::RangedBurnEquipment) => "burn equipment, ranged".into(),
        Some(StrikeKind::Corrupt) => "corruption counter".into(),
    }
}

/// What a combat did (`Engine::combat_notes`), told: a combat whose
/// windows all elide (hand strikes both ways) leaves no decision.
pub(crate) fn combat_text(e: &Engine, note: &CombatNote) -> String {
    match *note {
        CombatNote::Begins { minions: [a, b] } => {
            format!("Combat: {} vs {}", minion_name(e, a), minion_name(e, b))
        }
        CombatNote::Strikes {
            minions,
            round,
            pair,
            range,
            strikes,
            damage,
        } => {
            let side = |i: usize| {
                let mut s = format!("{} {}", minion_name(e, minions[i]), strike_text(strikes[i]));
                if damage[i] > 0 {
                    s += &format!(" ({} damage)", damage[i]);
                }
                s
            };
            let range = match range {
                Range::Close => "close range",
                Range::Long => "long range",
            };
            let pair = if pair > 0 { ", additional strikes" } else { "" };
            format!("Round {round}{pair}, {range}: {} — {}", side(0), side(1))
        }
        CombatNote::Ends { .. } => "The combat ends".into(),
    }
}

impl Mark {
    /// `all`: the table with every card shown (stable copies).
    pub(crate) fn of(e: &Engine, all: &TableView) -> Mark {
        let st = &e.state;
        let copies = |cards: &[crate::view::CardView]| cards.iter().map(|c| c.copy).collect();
        Mark {
            turn: (st.turn, st.active.0),
            library: all.seats.iter().map(|s| copies(&s.library)).collect(),
            crypt: all.seats.iter().map(|s| copies(&s.crypt)).collect(),
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
                    attempting: match a.state {
                        zvtes::actions::ActionState::Attempt(m) => Some(m),
                        _ => None,
                    },
                    passed: (a.kind == ActionKind::Political && e.referendum().is_none())
                        .then_some(a.referendum_passed),
                }
            }),
            strikes: e
                .combat()
                .map(|c| {
                    c.sides
                        .iter()
                        .map(|s| (s.minion, c.round, c.pair, declared_strike(e, s)))
                        .collect()
                })
                .unwrap_or_default(),
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
    /// The strikes declared since (the acting minion's first: the
    /// blocker chooses knowing it), each told once.
    /// A block attempt that ended with no block (short of the stealth,
    /// or given up): the action goes on.
    pub(crate) fn attempts(&self, e: &Engine, now: &Mark, out: &mut Vec<String>) {
        let (Some(a), Some(b)) = (&self.action, &now.action) else {
            return;
        };
        if !Self::same_action(a, b) {
            return;
        }
        if let Some(m) = a
            .attempting
            .filter(|&m| b.attempting != Some(m) && b.blocker != Some(m))
        {
            out.push(format!("{} fails to block", minion_name(e, m)));
        }
    }

    pub(crate) fn strikes(&self, e: &Engine, now: &Mark, out: &mut Vec<String>) {
        for &(m, round, pair, ref strike) in &now.strikes {
            let Some(strike) = strike else { continue };
            let told = self
                .strikes
                .iter()
                .any(|&(b, r, p, ref s)| b == m && r == round && p == pair && s.is_some());
            if !told {
                out.push(format!("{} strikes: {strike}", minion_name(e, m)));
            }
        }
    }

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

    /// A new turn, and the cards drawn from `self` to `now` (told whatever
    /// drew them: a decision's own draws too).
    pub(crate) fn draws(
        &self,
        now: &Mark,
        all: &TableView,
        names: &[String],
        out: &mut Vec<String>,
    ) {
        let seat = |s: usize| names.get(s).cloned().unwrap_or_default();
        if now.turn.0 != self.turn.0 {
            out.push(format!(
                "Turn {}: {}",
                now.turn.0,
                seat(now.turn.1 as usize)
            ));
        }
        for (s, v) in all.seats.iter().enumerate() {
            let drawn = |before: Option<&HashSet<u32>>, cards: &mut dyn Iterator<Item = u32>| {
                before.map_or(0, |b| cards.filter(|c| b.contains(c)).count())
            };
            let n = drawn(self.library.get(s), &mut v.hand.iter().map(|c| c.copy));
            if n > 0 {
                let cards = if n == 1 { "card" } else { "cards" };
                out.push(format!("{} draws {n} {cards}", seat(s)));
            }
            let n = drawn(
                self.crypt.get(s),
                &mut v.uncontrolled.iter().map(|u| u.card.copy),
            );
            if n > 0 {
                let cards = if n == 1 { "crypt card" } else { "crypt cards" };
                out.push(format!("{} draws {n} {cards}", seat(s)));
            }
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
