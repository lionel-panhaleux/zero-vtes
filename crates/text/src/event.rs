//! Events in English (docs/protocol.md §9).

use zvtes_api::event::{
    ActionOutcome, ActionRef, Cause, CauseKind, CauseRule, EventKind, ReferendumOutcome,
};
use zvtes_api::ids::{ActionKind, MarkerKind, Range, StrikeKind};
use zvtes_api::view::{Marker, Strike};

use crate::Names;

/// What made a change, after it: ", paid for X", ", by X".
fn why(cause: &Option<Cause>, n: &Names) -> String {
    let Some(c) = cause else {
        return String::new();
    };
    let what = match (&c.card, c.rule) {
        (Some(card), _) => match card.copy {
            Some(m) if n.is_minion(card) => n.minion(m),
            _ => n.card(card),
        },
        (None, Some(r)) => {
            return match r {
                CauseRule::Strikes => ", by the strikes",
                CauseRule::Transfer => ", by a transfer",
                CauseRule::Oust => ", by the oust",
                CauseRule::Edge => ", by the Edge",
                CauseRule::Contest => ", paid for the contest",
                CauseRule::Disease => ", by disease",
                CauseRule::Diablerie => ", by diablerie",
                CauseRule::Capacity => ", above capacity",
                CauseRule::Vote => ", paid for votes",
                CauseRule::Unknown => "",
            }
            .into()
        }
        (None, None) => return String::new(),
    };
    match c.kind {
        CauseKind::Cost => format!(", paid for {what}"),
        _ => format!(", by {what}"),
    }
}

/// An event told; `None` for a decision taken (its option tells it) and
/// for the state a view shows (a phase, a lock, counters, the action's
/// and the referendum's state: no log line).
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
        EventKind::Decided { .. }
        | EventKind::AutoPassed { .. }
        | EventKind::Undone { .. }
        | EventKind::Phase { .. }
        | EventKind::Transfers { .. }
        | EventKind::Locked { .. }
        | EventKind::Counters { .. }
        | EventKind::Controller { .. }
        | EventKind::Capacity { .. }
        | EventKind::Title { .. }
        | EventKind::Markers { .. }
        | EventKind::Traits { .. }
        | EventKind::CardMoves { .. }
        | EventKind::CombatState { .. }
        | EventKind::ActionState { .. }
        | EventKind::ReferendumState { .. } => return None,
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
        EventKind::Pool {
            seat,
            delta,
            pool,
            cause,
        } => format!(
            "{}: {} pool ({pool}){}",
            n.seat(*seat),
            signed(*delta),
            why(cause, n)
        ),
        EventKind::Blood {
            minion,
            delta,
            blood,
            cause,
        } => format!(
            "{}: {} blood ({blood}){}",
            n.minion(*minion),
            signed(*delta),
            why(cause, n)
        ),
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
        EventKind::ActingChanges { acting, .. } => {
            let acting = n.minion(acting.copy.unwrap_or(u32::MAX));
            format!("{acting} takes over the action")
        }
        EventKind::ActionStarts { acting, action, .. } => {
            let acting = n.minion(acting.copy.unwrap_or(u32::MAX));
            format!("{acting}'s {} is announced", thing(action, n, true))
        }
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
            format!(
                "{} strikes: {}",
                n.minion(*minion),
                strike_named(s.as_ref(), n)
            )
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
                let mut t = format!(
                    "{} {}",
                    n.minion(s.minion),
                    strike_named(s.strike.as_ref(), n)
                );
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
pub fn strike_declared(s: &Strike, n: &Names) -> String {
    let mut t = strike_named(Some(s), n);
    if let (StrikeKind::Hand | StrikeKind::Melee, Some(n)) = (s.kind, s.n) {
        t += &format!(", {n} damage");
    }
    if s.aggravated {
        t += ", aggravated";
    }
    t
}

/// A strike, its card named ("Torn Signpost: weapon strike").
fn strike_named(s: Option<&Strike>, n: &Names) -> String {
    match s.and_then(|s| s.card) {
        Some(c) => format!("{}: {}", n.card_id(Some(c)), strike(s)),
        None => strike(s),
    }
}

/// A marker on a minion, as its data says ("2 corruption counters
/// (Ana)", "Red List mark (Ben)", "Bonding's effect, 1 counter").
pub fn marker(m: &Marker, n: &Names) -> String {
    let k = m.n.unwrap_or(0);
    let counters = |what: &str| match k {
        1 => format!("1 {what} counter"),
        k => format!("{k} {what} counters"),
    };
    let of = |s: Option<u8>| s.map(|s| format!(" ({})", n.seat(s))).unwrap_or_default();
    match m.kind {
        MarkerKind::Corruption => format!("{}{}", counters("corruption"), of(m.seat)),
        MarkerKind::Hostage => counters("hostage"),
        MarkerKind::Nightmare => counters("nightmare"),
        MarkerKind::Torture => counters("torture"),
        MarkerKind::Pact => counters("pact"),
        MarkerKind::Taint => counters("taint"),
        MarkerKind::Disease => "a disease counter".into(),
        MarkerKind::TitleContested => "its title contested".into(),
        MarkerKind::Expelled => "expelled".into(),
        MarkerKind::RedListMark => format!("Red List mark{}", of(m.seat)),
        MarkerKind::OutOfPlay => "out of play".into(),
        MarkerKind::Effect => {
            let card = n.card_id(m.card);
            match m.n {
                Some(1) => format!("{card}'s effect, 1 counter"),
                Some(k) => format!("{card}'s effect, {k} counters"),
                None => format!("{card}'s effect"),
            }
        }
        MarkerKind::Unknown => "a marker".into(),
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

    /// The oracle of markers: each kind's English, every kind listed (a
    /// new one fails to compile here until it is).
    #[test]
    fn every_marker_reads_as_its_data_says() {
        use zvtes_api::ids::MarkerKind as K;
        use zvtes_api::view::Marker;
        let cards = Two;
        let mut n = Names::new(&cards);
        n.seats = vec!["Ana".into(), "Ben".into()];
        let m = |kind, seat, k, card| Marker {
            kind,
            seat,
            n: k,
            card,
            text: None,
        };
        let told = |k: K| -> (Marker, &str) {
            match k {
                K::Corruption => (m(k, Some(1), Some(2), None), "2 corruption counters (Ben)"),
                K::Hostage => (m(k, None, Some(1), None), "1 hostage counter"),
                K::Nightmare => (m(k, None, Some(2), None), "2 nightmare counters"),
                K::Torture => (m(k, None, Some(3), None), "3 torture counters"),
                K::Pact => (m(k, None, Some(1), None), "1 pact counter"),
                K::Taint => (m(k, None, Some(2), None), "2 taint counters"),
                K::Disease => (m(k, None, None, None), "a disease counter"),
                K::TitleContested => (m(k, None, None, None), "its title contested"),
                K::Expelled => (m(k, None, None, None), "expelled"),
                K::RedListMark => (m(k, Some(0), None, None), "Red List mark (Ana)"),
                K::OutOfPlay => (m(k, None, None, None), "out of play"),
                K::Effect => (
                    m(k, None, Some(1), Some(2)),
                    "Torn Signpost's effect, 1 counter",
                ),
                K::Unknown => (m(k, None, None, None), "a marker"),
            }
        };
        // Every kind told: the list's match is exhaustive, so a new kind
        // fails to compile here until it is listed.
        macro_rules! every {
            ($($k:ident),* $(,)?) => {{
                fn listed(k: K) -> bool {
                    match k {
                        $(K::$k)|* => true,
                    }
                }
                [$(K::$k),*].map(|k| {
                    assert!(listed(k));
                    k
                })
            }};
        }
        let kinds = every!(
            Corruption,
            Hostage,
            Nightmare,
            Torture,
            Pact,
            Taint,
            Disease,
            TitleContested,
            Expelled,
            RedListMark,
            OutOfPlay,
            Effect,
            Unknown,
        );
        for k in kinds {
            let (marker, want) = told(k);
            assert_eq!(crate::marker(&marker, &n), want);
        }
    }

    /// The oracle of strikes: a card's strike named by its card, a hand
    /// strike's damage told.
    #[test]
    fn a_strike_reads_with_its_card() {
        use zvtes_api::ids::StrikeKind as K;
        use zvtes_api::view::Strike;
        let cards = Two;
        let n = Names::new(&cards);
        let s = |kind, n_, card| Strike {
            kind,
            n: n_,
            ranged: false,
            aggravated: false,
            card,
            resolved: None,
            text: None,
        };
        assert_eq!(
            crate::strike_declared(&s(K::Melee, Some(2), Some(2)), &n),
            "Torn Signpost: weapon strike, 2 damage"
        );
        assert_eq!(
            crate::strike_declared(&s(K::Hand, Some(1), None), &n),
            "hand strike, 1 damage"
        );
        assert_eq!(
            crate::strike_declared(&s(K::CardEffect, None, Some(2)), &n),
            "Torn Signpost: strike"
        );
    }
}
