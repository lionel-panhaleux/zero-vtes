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
        EventKind::Resolved { card, effects, .. } => match crate::effect::label(effects, n) {
            l if l.is_empty() => format!("{} resolves", n.card(card)),
            l => format!("{} resolves: {l}", n.card(card)),
        },
        EventKind::Canceled { card, .. } => format!("{} is canceled", n.card(card)),
        EventKind::Revealed { seat, card } => {
            let from = if card.crypt { "crypt" } else { "library" };
            format!(
                "{} reveals {} from their {from}",
                n.seat(*seat),
                n.card(card)
            )
        }
        EventKind::Looked { seat, cards } => {
            let names: Vec<String> = cards.iter().map(|c| n.card(c)).collect();
            match names.is_empty() {
                true => format!("You look at {}'s hand: empty", n.seat(*seat)),
                false => format!("You look at {}'s hand: {}", n.seat(*seat), names.join(", ")),
            }
        }
        EventKind::Pool { seat, delta, pool } => {
            format!("{}: {} pool ({pool})", n.seat(*seat), signed(*delta))
        }
        EventKind::Blood {
            minion,
            delta,
            blood,
        } => format!("{}: {} blood ({blood})", n.minion(*minion), signed(*delta)),
        // A lasting effect whose card is in the ash heap: told as the
        // card's effect, never as the card in play.
        EventKind::EntersPlay {
            card,
            on: host,
            effect: true,
            ..
        } => format!("{}'s effect begins{}", n.card(card), on(host)),
        EventKind::EntersPlay {
            card,
            on: host,
            minion,
            ..
        } => match (minion, card.copy) {
            (Some(_), Some(c)) => format!("{} comes into play", n.minion(c)),
            _ => format!("{} comes into play{}", n.card(card), on(host)),
        },
        EventKind::Moves {
            card,
            on: host,
            effect: true,
        } => {
            let to = host.map(|m| format!(" to {}", n.minion(m)));
            format!("{}'s effect moves{}", n.card(card), to.unwrap_or_default())
        }
        EventKind::Moves { card, on: host, .. } => {
            format!("{} moves{}", n.card(card), on(host))
        }
        EventKind::LeavesPlay {
            card, effect: true, ..
        } => format!("{}'s effect ends", n.card(card)),
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
        EventKind::Edge { seat: Some(s) } => format!("{} gains the Edge", n.seat(*s)),
        EventKind::Edge { seat: None } => "The Edge is burned".into(),
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
                // Retainers' and environmental damage apart from the
                // strike's: strike-only prevention covers the latter only.
                let besides = s.damage.saturating_sub(s.strike_damage);
                match (s.strike_damage, besides) {
                    (0, 0) => {}
                    (n, 0) => t += &format!(" ({n} damage)"),
                    (0, m) => t += &format!(" ({m} damage besides the strike)"),
                    (n, m) => t += &format!(" ({n} damage, {m} more besides the strike)"),
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

/// A declared strike with the damage it inflicts if it lands ("hand
/// strike, 3 damage, aggravated").
pub fn strike_declared(s: &Strike) -> String {
    let mut t = strike(Some(s));
    if let (StrikeKind::Hand | StrikeKind::Melee, Some(n)) = (s.kind, s.n) {
        t += &format!(", {n} damage");
    }
    if s.aggravated {
        t += ", aggravated";
    }
    t
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
        StrikeKind::StealWeapon => "steal weapon".into(),
        StrikeKind::BurnEquipment => "burn equipment".into(),
        StrikeKind::RangedBurnEquipment => "burn equipment, ranged".into(),
        StrikeKind::Corrupt => "corruption counter".into(),
        StrikeKind::BurnLife => format!("burn {n} life, ranged"),
    }
}

#[cfg(test)]
mod tests {
    use zvtes_api::event::EventKind;
    use zvtes_api::ids::{KrcgId, MinionKind};
    use zvtes_api::view::Card;

    use crate::{Cards, Names};

    struct Two;

    impl Cards for Two {
        fn name(&self, id: KrcgId) -> Option<&str> {
            match id {
                1 => Some("Lucian"),
                2 => Some("Torn Signpost"),
                _ => None,
            }
        }
    }

    fn card(id: KrcgId, copy: Option<u32>) -> Card {
        Card {
            copy,
            id: Some(id),
            crypt: false,
            nth: None,
            numbered: None,
        }
    }

    // A round's line keeps the strike's damage apart from the rest (its
    // retainers' and environmental damage): strike-only prevention covers
    // the strike's only.
    #[test]
    fn a_rounds_line_keeps_the_strike_damage_apart() {
        use zvtes_api::event::StruckSide;
        use zvtes_api::ids::{Range, StrikeKind};
        use zvtes_api::view::Strike;
        let cards = Two;
        let mut n = Names::new(&cards);
        n.hear(&EventKind::EntersPlay {
            card: card(1, Some(7)),
            seat: 0,
            on: None,
            minion: Some(MinionKind::Vampire),
            effect: false,
        });
        let hands = Strike {
            kind: StrikeKind::Hand,
            n: Some(1),
            ranged: false,
            aggravated: false,
            card: None,
            resolved: None,
            text: None,
        };
        let line = |damage, strike_damage| {
            let e = EventKind::Strikes {
                round: 1,
                pair: 0,
                range: Range::Close,
                sides: vec![
                    StruckSide {
                        minion: 7,
                        strike: Some(hands.clone()),
                        damage,
                        strike_damage,
                    },
                    StruckSide {
                        minion: 7,
                        strike: None,
                        damage: 0,
                        strike_damage: 0,
                    },
                ],
            };
            super::event(&e, &n).expect("told")
        };
        let lucian =
            |t: &str| format!("Round 1, close range: Lucian hand strike{t} — Lucian no strike");
        assert_eq!(line(2, 1), lucian(" (1 damage, 1 more besides the strike)"));
        assert_eq!(line(1, 1), lucian(" (1 damage)"));
        assert_eq!(line(1, 0), lucian(" (1 damage besides the strike)"));
        assert_eq!(line(0, 0), lucian(""));
    }

    // A lasting effect whose card is in the ash heap (`effect`) is told as
    // the card's effect, never as the card coming into play.
    #[test]
    fn a_lasting_effect_is_told_as_an_effect() {
        let cards = Two;
        let mut n = Names::new(&cards);
        n.hear(&EventKind::EntersPlay {
            card: card(1, Some(7)),
            seat: 0,
            on: None,
            minion: Some(MinionKind::Vampire),
            effect: false,
        });
        let told = |e: EventKind| super::event(&e, &n).expect("told");
        assert_eq!(
            told(EventKind::EntersPlay {
                card: card(2, None),
                seat: 1,
                on: Some(7),
                minion: None,
                effect: true,
            }),
            "Torn Signpost's effect begins on Lucian"
        );
        assert_eq!(
            told(EventKind::Moves {
                card: card(2, None),
                on: Some(7),
                effect: true,
            }),
            "Torn Signpost's effect moves to Lucian"
        );
        assert_eq!(
            told(EventKind::LeavesPlay {
                card: card(2, None),
                to: None,
                minion: None,
                contested: false,
                effect: true,
            }),
            "Torn Signpost's effect ends"
        );
        // The card itself, in play.
        assert_eq!(
            told(EventKind::EntersPlay {
                card: card(2, Some(9)),
                seat: 1,
                on: Some(7),
                minion: None,
                effect: false,
            }),
            "Torn Signpost comes into play on Lucian"
        );
    }
    // How a card played finished: resolved with what its mode does (none
    // said when it does nothing the vocabulary names), or canceled.
    #[test]
    fn a_play_resolved_or_canceled_is_told() {
        use zvtes_api::option::{Effect, EffectKind};
        let cards = Two;
        let n = Names::new(&cards);
        let told = |e: EventKind| super::event(&e, &n).expect("told");
        assert_eq!(
            told(EventKind::Resolved {
                seat: 0,
                card: card(2, Some(9)),
                effects: vec![Effect {
                    n: Some(1),
                    ..Effect::new(EffectKind::Stealth)
                }],
            }),
            "Torn Signpost resolves: +1 stealth"
        );
        assert_eq!(
            told(EventKind::Resolved {
                seat: 0,
                card: card(2, Some(9)),
                effects: vec![],
            }),
            "Torn Signpost resolves"
        );
        assert_eq!(
            told(EventKind::Canceled {
                seat: 1,
                card: card(2, Some(9)),
            }),
            "Torn Signpost is canceled"
        );
    }
}
