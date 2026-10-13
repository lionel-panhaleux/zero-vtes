//! A1b: today's session (`zvtes_protocol`) read as the contract — its
//! views and options mapped to `api` types. Replaced when the host reads
//! the engine itself (A2: per-viewer copies, `nth`; A3: keys, effects,
//! typed events). Lossy where the session speaks only in words: those
//! words are dropped unless the seat asks for text.

use serde::de::DeserializeOwned;
use zvtes::game::Engine;
use zvtes::ids::MinionId;
use zvtes::state::CardId;
use zvtes_api::ids::{ActionKind, OptionKind, Window, Zone};
use zvtes_api::option::{
    Allocation, CardRef, ChoiceKey, DisciplineLevel, Mode, Recipient, VoteTotals, ZoneRef,
};
use zvtes_api::view::Target;
use zvtes_api::OptionView;
use zvtes_protocol::describe as old_d;
use zvtes_protocol::session as old_s;
use zvtes_protocol::view as old;

use crate::ids;
use crate::view::Viewer;

/// A snake_case id read from the session's words ("leave torpor").
fn id<T: DeserializeOwned>(s: &str) -> Option<T> {
    let snake = s.trim().to_lowercase().replace([' ', '-'], "_");
    serde_json::from_value(serde_json::Value::String(snake)).ok()
}

pub fn window(text: &str) -> Window {
    ids::WINDOWS
        .iter()
        .find(|w| old_d::window_text(**w) == text)
        .map_or(Window::Unknown, |w| ids::window(*w))
}

/// The session's arena ids (minions, cards in play) as the seat's wire
/// copies (`view::Viewer`).
pub struct Ctx<'a> {
    pub text: bool,
    viewer: &'a Viewer,
    e: &'a Engine,
}

impl<'a> Ctx<'a> {
    pub fn new(viewer: &'a Viewer, e: &'a Engine, text: bool) -> Ctx<'a> {
        Ctx { text, viewer, e }
    }

    fn minion(&self, m: u32) -> u32 {
        self.viewer.minion(self.e, MinionId(m)).unwrap_or(u32::MAX)
    }

    fn card_copy(&self, c: u32) -> u32 {
        self.viewer.card(self.e, CardId(c)).unwrap_or(u32::MAX)
    }

    fn text(&self, s: &str) -> Option<String> {
        self.text.then(|| s.to_string())
    }

    fn target(&self, t: &old::TargetView) -> Option<Target> {
        Some(match *t {
            old::TargetView::Minion { minion } => Target::Card {
                copy: self.minion(minion),
            },
            old::TargetView::Seat { seat } => Target::Seat { seat },
            old::TargetView::Card { card_id } => Target::Card {
                copy: self.card_copy(card_id),
            },
            // A slot index without its seat: not mapped in A1b.
            old::TargetView::Uncontrolled { .. } => return None,
        })
    }

    pub fn option(&self, o: &old_d::OptionView) -> OptionView {
        OptionView {
            index: o.index as u32,
            key: ChoiceKey {
                kind: id(&o.kind).unwrap_or(OptionKind::Unknown),
                card: o.card.map(|c| CardRef {
                    id: c.id,
                    copy: c.card_id.map(|i| self.card_copy(i)),
                    from: None,
                }),
                actor: o.actor.map(|m| self.minion(m)),
                minion: o.minion.map(|m| self.minion(m)),
                target: o.target.as_ref().and_then(|t| self.target(t)),
                action: o.action.as_deref().map(action_kind),
                mode: o.mode.as_ref().map(mode),
                amount: o.amount,
                favor: o.favor,
                blood: o.blood,
                zone: o.zone.as_ref().map(|z| ZoneRef {
                    seat: z.seat,
                    zone: id(&z.zone).unwrap_or(Zone::Unknown),
                }),
                value: None,
                ability: None,
            },
            effects: vec![],
            text: self.text(&o.text),
        }
    }

    pub fn allocation(&self, a: &old_s::AllocationView) -> Allocation {
        Allocation {
            points: a.points,
            recipients: a
                .recipients
                .iter()
                .map(|r| Recipient {
                    to: match (r.seat, r.card) {
                        (Some(seat), _) => Target::Seat { seat },
                        (None, Some(c)) => Target::Card {
                            copy: c.card_id.map_or(u32::MAX, |i| self.card_copy(i)),
                        },
                        (None, None) => Target::Seat { seat: u8::MAX },
                    },
                    cap: r.cap,
                    effects: vec![],
                })
                .collect(),
            min_nonzero: a.min_nonzero,
        }
    }
}

/// A choice the session took itself (a standing vote order, an
/// allocation answered whole) as a record key: its minion and cards by
/// the engine's copies.
pub fn engine_key(e: &Engine, o: &old_d::OptionView) -> ChoiceKey {
    let minion = |m: u32| crate::view::minion_copy(&e.state, m as usize).0 as u32;
    let card = |c: u32| e.state.cards[c as usize].copy.0 .0 as u32;
    ChoiceKey {
        kind: id(&o.kind).unwrap_or(OptionKind::Unknown),
        card: o.card.map(|c| CardRef {
            id: c.id,
            copy: c.card_id.map(card),
            from: None,
        }),
        actor: o.actor.map(minion),
        minion: o.minion.map(minion),
        target: o.target.as_ref().and_then(|t| match *t {
            old::TargetView::Minion { minion: m } => Some(Target::Card { copy: minion(m) }),
            old::TargetView::Seat { seat } => Some(Target::Seat { seat }),
            old::TargetView::Card { card_id } => Some(Target::Card {
                copy: card(card_id),
            }),
            old::TargetView::Uncontrolled { .. } => None,
        }),
        action: o.action.as_deref().map(action_kind),
        mode: None,
        amount: o.amount,
        favor: o.favor,
        blood: o.blood,
        zone: None,
        value: None,
        ability: None,
    }
}

pub fn votes(v: &old_s::VoteTotals) -> VoteTotals {
    VoteTotals {
        favor: v.favor,
        against: v.against,
    }
}

fn action_kind(s: &str) -> ActionKind {
    match s {
        "political action" => ActionKind::Political,
        "action" => ActionKind::Card,
        "equip from a minion" => ActionKind::EquipFrom,
        "equip from the library" => ActionKind::EquipFound,
        "enter combat with a Red List minion" => ActionKind::RedListHunt,
        other => id(other).unwrap_or(ActionKind::Unknown),
    }
}

const CARD_TYPES: [&str; 6] = [
    "ACTION",
    "ACTIONMODIFIER",
    "REACTION",
    "COMBAT",
    "REFLEX",
    "POLITICAL",
];

fn mode(m: &old_d::ModeView) -> Mode {
    let mut out = Mode {
        index: m.index,
        line: m.line,
        disciplines: vec![],
        card_type: None,
    };
    for t in &m.tags {
        if CARD_TYPES.contains(&t.as_str()) {
            out.card_type = Some(match t.as_str() {
                "ACTIONMODIFIER" => "action_modifier".into(),
                o => o.to_lowercase(),
            });
        } else {
            out.disciplines.push(DisciplineLevel {
                discipline: t.to_lowercase(),
                superior: t.chars().all(|c| c.is_ascii_uppercase()),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use zvtes_api::ids::Range;

    #[test]
    fn every_window_reads_back_from_its_text() {
        for w in ids::WINDOWS {
            assert_eq!(window(old_d::window_text(w)), ids::window(w), "{w:?}");
        }
    }

    #[test]
    fn session_words_read_as_ids() {
        assert_eq!(action_kind("leave torpor"), ActionKind::LeaveTorpor);
        assert_eq!(action_kind("political action"), ActionKind::Political);
        assert_eq!(id::<Range>("long"), Some(Range::Long));
        assert_eq!(
            id::<OptionKind>("hand_strike"),
            Some(OptionKind::HandStrike)
        );
    }
}
