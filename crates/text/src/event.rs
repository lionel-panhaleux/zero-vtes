//! Events in English (docs/protocol.md §9).

use zvtes_api::event::{ActionOutcome, ActionRef, EventKind, ReferendumOutcome};
use zvtes_api::ids::{ActionKind, Range, StrikeKind};
use zvtes_api::view::Strike;

use crate::Names;

/// An event told; `None` for a decision taken (its option tells it).
pub fn event(e: &EventKind, n: &Names) -> Option<String> {
    let signed = |d: i32| {
        if d > 0 {
            format!("+{d}")
        } else {
            format!("−{}", -d)
        }
    };
    let on = |m: &Option<u32>| {
        m.map(|m| format!(" on {}", n.minion(m)))
            .unwrap_or_default()
    };
    Some(match e {
        EventKind::Decided { .. } | EventKind::AutoPassed { .. } | EventKind::Undone { .. } => {
            return None
        }
        EventKind::Turn { turn, active } => format!("Turn {turn}: {}", n.seat(*active)),
        EventKind::Draw { seat, n: k, crypt } => {
            let cards = match (crypt, k) {
                (false, 1) => "card",
                (false, _) => "cards",
                (true, 1) => "crypt card",
                (true, _) => "crypt cards",
            };
            format!("{} draws {k} {cards}", n.seat(*seat))
        }
        EventKind::Pool { seat, delta, pool } => {
            format!("{}: {} pool ({pool})", n.seat(*seat), signed(*delta))
        }
        EventKind::Blood {
            minion,
            delta,
            blood,
        } => format!("{}: {} blood ({blood})", n.minion(*minion), signed(*delta)),
        EventKind::EntersPlay {
            card,
            on: host,
            minion,
            ..
        } => match (minion, card.copy) {
            (Some(_), Some(c)) => format!("{} comes into play", n.minion(c)),
            _ => format!("{} comes into play{}", n.card(card), on(host)),
        },
        EventKind::Moves { card, on: host, .. } => {
            format!("{} moves{}", n.card(card), on(host))
        }
        EventKind::LeavesPlay {
            card,
            minion,
            contested,
            ..
        } => match (minion, card.copy) {
            (Some(_), Some(c)) if *contested => format!("{} is contested", n.minion(c)),
            (Some(_), Some(c)) => format!("{} is burned", n.minion(c)),
            _ => format!("{} leaves play", n.card(card)),
        },
        EventKind::Torpor { minion, enters } => {
            let how = if *enters {
                "goes to torpor"
            } else {
                "leaves torpor"
            };
            format!("{} {how}", n.minion(*minion))
        }
        EventKind::Ousted { seat } => format!("{} is ousted", n.seat(*seat)),
        EventKind::BlockFails { minion } => format!("{} fails to block", n.minion(*minion)),
        EventKind::ActionEnds {
            seat,
            acting,
            action,
            outcome,
            blocker,
        } => {
            let label = format!("{}'s {}", n.minion(*acting), thing(action, n, true));
            match (outcome, blocker) {
                (ActionOutcome::Succeeds, _) => format!("{label} succeeds"),
                (ActionOutcome::Blocked, Some(b)) => {
                    format!("{label} is blocked by {}", n.minion(*b))
                }
                (ActionOutcome::Blocked, None) => format!("{label} is blocked"),
                (ActionOutcome::Fails, _) => format!("{label} fails"),
                (ActionOutcome::Fizzles, _) => format!("{label} fizzles"),
                (ActionOutcome::Ousted, _) => {
                    format!("{label} ends: {} is ousted", n.seat(*seat))
                }
                (ActionOutcome::Ends | ActionOutcome::Unknown, _) => format!("{label} ends"),
            }
        }
        EventKind::ReferendumEnds {
            outcome,
            votes_for: f,
            votes_against: a,
            action,
            hunted,
        } => {
            let label = match (hunted, action) {
                (Some(m), _) => format!("The blood hunt on {}", n.minion(*m)),
                (None, Some(a)) => format!("The referendum ({})", thing(a, n, false)),
                (None, None) => "The referendum".into(),
            };
            match outcome {
                ReferendumOutcome::Canceled => format!("{label} is canceled"),
                ReferendumOutcome::Passes => format!("{label} passes ({f}–{a})"),
                ReferendumOutcome::Fails | ReferendumOutcome::Unknown => {
                    format!("{label} fails ({f}–{a})")
                }
            }
        }
        EventKind::CombatBegins { minions: [a, b] } => {
            format!("Combat: {} vs {}", n.minion(*a), n.minion(*b))
        }
        EventKind::StrikeDeclared { minion, strike: s } => {
            let what = match s {
                Some(s) => match s.card {
                    Some(c) => format!("{}: {}", n.card_id(Some(c)), strike(Some(s))),
                    None => strike(Some(s)),
                },
                None => strike(None),
            };
            format!("{} strikes: {what}", n.minion(*minion))
        }
        EventKind::Strikes {
            round,
            pair,
            range,
            sides,
        } => {
            let side = |i: usize| {
                let Some(s) = sides.get(i) else {
                    return String::new();
                };
                let mut t = format!("{} {}", n.minion(s.minion), strike(s.strike.as_ref()));
                if s.damage > 0 {
                    t += &format!(" ({} damage)", s.damage);
                }
                t
            };
            let range = match range {
                Range::Long => "long range",
                _ => "close range",
            };
            let pair = if *pair > 0 {
                ", additional strikes"
            } else {
                ""
            };
            format!("Round {round}{pair}, {range}: {} — {}", side(0), side(1))
        }
        EventKind::CombatEnds => "The combat ends".into(),
    })
}

/// What an action is: its card, what grants it ("… action" for a card's
/// when `granted`), or its kind.
fn thing(a: &ActionRef, n: &Names, granted: bool) -> String {
    match (&a.card, &a.source) {
        (Some(c), _) => n.card(c),
        // Another vampire's (its crypt card): named as the minion; a card
        // in play, even one that is a minion too (Call the Great Beast),
        // by its card.
        (None, Some(s)) if s.crypt && n.is_minion(s) => n.minion(s.copy.unwrap_or(u32::MAX)),
        (None, Some(s)) if granted => format!("{} action", n.card(s)),
        (None, Some(s)) => n.card(s),
        (None, None) => action_kind(a.kind).into(),
    }
}

pub fn action_kind(k: ActionKind) -> &'static str {
    match k {
        ActionKind::Bleed => "bleed",
        ActionKind::Hunt => "hunt",
        ActionKind::LeaveTorpor => "leave torpor",
        ActionKind::Rescue => "rescue",
        ActionKind::Diablerie => "diablerie",
        ActionKind::Employ => "employ",
        ActionKind::Recruit => "recruit",
        ActionKind::Political => "political action",
        ActionKind::Card | ActionKind::Ability | ActionKind::Unknown => "action",
        ActionKind::BurnCard => "burn card",
        ActionKind::StealCard => "steal card",
        ActionKind::Equip => "equip",
        ActionKind::EquipFrom => "equip from a minion",
        ActionKind::EquipFound => "equip from the library",
        ActionKind::BecomeAnarch => "become Anarch",
        ActionKind::RedListHunt => "enter combat with a Red List minion",
    }
}

/// A strike as the table sees it ("no strike" for none).
pub fn strike(s: Option<&Strike>) -> String {
    let Some(s) = s else {
        return "no strike".into();
    };
    let n = s.n.unwrap_or(0);
    match s.kind {
        StrikeKind::Hand => "hand strike".into(),
        StrikeKind::Melee => "weapon strike".into(),
        StrikeKind::Dodge => "dodge".into(),
        StrikeKind::CombatEnds => "combat ends".into(),
        StrikeKind::Damage => format!("{n}{} damage", if s.ranged { "R" } else { "" }),
        StrikeKind::StealBlood => {
            format!("steal {n} blood{}", if s.ranged { ", ranged" } else { "" })
        }
        StrikeKind::TorporOrBurn => "torpor or burn".into(),
        StrikeKind::Torpor => "torpor".into(),
        StrikeKind::CardEffect | StrikeKind::Unknown => "strike".into(),
        StrikeKind::BurnWeapon => "burn weapon".into(),
        StrikeKind::BurnEquipment => "burn equipment".into(),
        StrikeKind::RangedBurnEquipment => "burn equipment, ranged".into(),
        StrikeKind::Corrupt => "corruption counter".into(),
    }
}
