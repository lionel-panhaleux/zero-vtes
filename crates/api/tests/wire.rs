//! The JSON a client sees and sends, pinned.

use serde_json::{json, Value};
use zvtes_api::event::{ActionOutcome, ActionRef};
use zvtes_api::ids::{ActionKind, MinionKind, OptionKind, Phase, Window};
use zvtes_api::option::{CardRef, ChoiceKey, DisciplineLevel, Effect, EffectKind, Mode};
use zvtes_api::view::{Card, Target};
use zvtes_api::*;

fn key(kind: OptionKind) -> ChoiceKey {
    ChoiceKey {
        kind,
        card: None,
        actor: None,
        minion: None,
        target: None,
        action: None,
        mode: None,
        amount: None,
        favor: None,
        blood: None,
        zone: None,
        value: None,
        ability: None,
    }
}

fn option(index: u32, key: ChoiceKey) -> OptionView {
    OptionView {
        index,
        key,
        effects: vec![],
        text: None,
    }
}

fn effect(kind: EffectKind) -> Effect {
    Effect {
        kind,
        seat: None,
        copy: None,
        n: None,
        card: None,
        to: None,
        role: None,
    }
}

fn view() -> TableView {
    TableView {
        viewer: Some(0),
        decision: 12,
        turn: 2,
        active: 0,
        phase: Phase::Minion,
        seats: vec![],
        action: None,
        combat: None,
        referendum: None,
    }
}

#[test]
fn commands_read_from_json() {
    let cases: Vec<(Value, Command)> = vec![
        (
            json!({"type": "join", "token": "t0k"}),
            Command::Join {
                token: Some("t0k".into()),
                after: None,
            },
        ),
        (
            json!({"type": "join", "after": 41}),
            Command::Join {
                token: None,
                after: Some(41),
            },
        ),
        (
            json!({"type": "answer", "decision": 12, "option": 3}),
            Command::Answer {
                decision: 12,
                option: 3,
            },
        ),
        (
            json!({"type": "allocate", "decision": 12, "amounts": [2, 1]}),
            Command::Allocate {
                decision: 12,
                amounts: vec![2, 1],
            },
        ),
        (
            json!({"type": "vote_all", "decision": 12, "favor": true}),
            Command::VoteAll {
                decision: 12,
                favor: true,
            },
        ),
        (
            json!({"type": "undo", "decision": 12}),
            Command::Undo { decision: 12 },
        ),
        (
            json!({"type": "settings", "opportunities": false}),
            Command::Settings(SettingsPatch {
                opportunities: Some(false),
                views: None,
                text: None,
            }),
        ),
        (json!({"type": "leave"}), Command::Leave),
    ];
    for (j, c) in cases {
        assert_eq!(
            serde_json::from_value::<Command>(j.clone()).unwrap(),
            c,
            "{j}"
        );
        assert_eq!(serde_json::to_value(&c).unwrap(), j);
    }
    let sent: In = serde_json::from_value(json!({"id": 7, "type": "undo", "decision": 3})).unwrap();
    assert_eq!(sent.id, Some(7));
    assert_eq!(sent.command, Command::Undo { decision: 3 });
}

#[test]
fn a_decision_reads_flat() {
    let mut play = key(OptionKind::Play);
    play.card = Some(CardRef {
        id: Some(100_845),
        copy: None,
        from: None,
    });
    play.actor = Some(41);
    play.target = Some(Target::Seat { seat: 1 });
    play.mode = Some(Mode {
        index: 0,
        line: 0,
        disciplines: vec![DisciplineLevel {
            discipline: "dom".into(),
            superior: false,
        }],
        card_type: None,
    });
    let mut keep = option(4, key(OptionKind::ResolveRider));
    keep.effects = vec![
        Effect {
            seat: Some(0),
            n: Some(-2),
            ..effect(EffectKind::Pool)
        },
        Effect {
            copy: Some(9),
            ..effect(EffectKind::Keep)
        },
    ];
    let mut shown = option(3, play);
    shown.text = Some("Play Govern the Unaligned at [dom] with Yael → Seat 2".into());
    let m = Out {
        seq: 5,
        reply_to: None,
        message: Message::Decision(Decision {
            decision: 12,
            seat: 0,
            window: Window::MinionPhase,
            source: None,
            purpose: None,
            options: vec![option(0, key(OptionKind::Pass)), shown, keep],
            undo: false,
            allocation: None,
            votes: None,
            prompt: None,
            view: view(),
        }),
    };
    let j = serde_json::to_value(&m).unwrap();
    assert_eq!(
        j,
        json!({
            "seq": 5, "type": "decision", "decision": 12, "seat": 0, "window": "minion_phase",
            "options": [
                {"index": 0, "kind": "pass"},
                {"index": 3, "kind": "play", "card": {"id": 100845}, "actor": 41,
                 "target": {"kind": "seat", "seat": 1},
                 "mode": {"index": 0, "line": 0,
                          "disciplines": [{"discipline": "dom", "superior": false}]},
                 "text": "Play Govern the Unaligned at [dom] with Yael → Seat 2"},
                {"index": 4, "kind": "resolve_rider",
                 "effects": [{"kind": "pool", "seat": 0, "n": -2},
                             {"kind": "keep", "copy": 9}]},
            ],
            "undo": false,
            "view": {"viewer": 0, "decision": 12, "turn": 2, "active": 0,
                     "phase": "minion", "seats": []},
        })
    );
    assert_eq!(serde_json::from_value::<Out>(j).unwrap(), m);
}

#[test]
fn events_are_tagged_by_kind() {
    let e = Event {
        kind: EventKind::EntersPlay {
            card: Card {
                copy: Some(7),
                id: Some(200_001),
                crypt: true,
                nth: Some(1),
            },
            seat: 2,
            on: None,
            minion: Some(MinionKind::Vampire),
            effect: false,
        },
        text: None,
    };
    let j = serde_json::to_value(&e).unwrap();
    assert_eq!(
        j,
        json!({"kind": "enters_play",
               "card": {"copy": 7, "id": 200001, "crypt": true, "nth": 1}, "seat": 2,
               "minion": "vampire"})
    );
    assert_eq!(serde_json::from_value::<Event>(j).unwrap(), e);
    let e = Event {
        kind: EventKind::ActionEnds {
            seat: 1,
            acting: 7,
            action: ActionRef {
                kind: ActionKind::Bleed,
                card: None,
                source: None,
            },
            outcome: ActionOutcome::Blocked,
            blocker: Some(12),
        },
        text: None,
    };
    let j = serde_json::to_value(&e).unwrap();
    assert_eq!(
        j,
        json!({"kind": "action_ends", "seat": 1, "acting": 7, "action": {"kind": "bleed"},
               "outcome": "blocked", "blocker": 12})
    );
    assert_eq!(serde_json::from_value::<Event>(j).unwrap(), e);
    let e = Event {
        kind: EventKind::CombatEnds,
        text: Some("The combat ends".into()),
    };
    let j = serde_json::to_value(&e).unwrap();
    assert_eq!(j, json!({"kind": "combat_ends", "text": "The combat ends"}));
    assert_eq!(serde_json::from_value::<Event>(j).unwrap(), e);
}

#[test]
fn a_card_back_is_empty() {
    let back = Card {
        copy: None,
        id: None,
        crypt: false,
        nth: None,
    };
    assert_eq!(serde_json::to_value(back).unwrap(), json!({}));
}

#[test]
fn ids_a_later_host_adds_read_as_unknown() {
    let w: Window = serde_json::from_value(json!("a_window_from_the_future")).unwrap();
    assert_eq!(w, Window::Unknown);
    let k: ChoiceKey = serde_json::from_value(json!({"kind": "new_kind"})).unwrap();
    assert_eq!(k.kind, OptionKind::Unknown);
}

#[test]
fn a_spec_defaults_a_human_seat_settings() {
    let s: GameSpec = serde_json::from_value(json!({
        "seats": [
            {"name": "Me", "deck": {"200001": 12}, "player": {"type": "human"}},
            {"name": "Bot", "deck": {}, "player": {"type": "bot", "profile": "p9"}},
        ]
    }))
    .unwrap();
    assert_eq!(s.seed, None);
    assert_eq!(
        s.seats[0].player,
        Player::Human {
            settings: Settings::default()
        }
    );
    let partial: Settings = serde_json::from_value(json!({"text": true})).unwrap();
    assert!(partial.text && partial.opportunities);
    let mut s = Settings::default();
    s.apply(SettingsPatch {
        opportunities: Some(false),
        views: None,
        text: None,
    });
    assert!(!s.opportunities && !s.text);
}
