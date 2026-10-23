//! The punishing habits (`zvtes_env::habits`): what they take by rule is what
//! they say (a rescue of one's own vampire by a minion with 2 blood, a burn
//! of another's card, a diablerie of another's vampire), what they keep
//! home is the last ready minion after another acted, they read only the
//! seat's legal view, and random games meet every habit.

use zvtes::agent::{Agent, RandomLegal};
use zvtes::cards::{AbilitySource, Effect};
use zvtes::decision::{ActionKind, Choice, DecisionRequest};
use zvtes::decks::Deck;
use zvtes::game::{Engine, Step};
use zvtes_env::habits::{self, Set};

fn precons(seed: u64) -> Vec<Deck> {
    (0..4)
        .map(|s| Deck::precon(((seed * 3 + s * 5) % 14) as usize))
        .collect()
}

#[derive(Default, Debug)]
struct Seen {
    rescue: u32,
    burn: u32,
    diablerie: u32,
    kept_home: u32,
}

/// Random play within the habits: their choice when they have one, else a
/// random option among those they leave.
#[derive(Clone)]
struct Habits(RandomLegal);

impl Agent for Habits {
    fn decide(&mut self, e: &Engine, req: &DecisionRequest) -> usize {
        if let Some(i) = habits::choice(e, req, Set::ALL) {
            return i;
        }
        let Some(mask) = habits::mask(e, req, Set::ALL) else {
            return self.0.choose(req);
        };
        let left: Vec<usize> = (0..mask.len()).filter(|&i| mask[i]).collect();
        left[self.0.rng.below(left.len())]
    }

    fn fork(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

fn check(e: &Engine, req: &DecisionRequest, seen: &mut Seen) {
    let (st, me) = (&e.state, req.seat);
    if let Some(i) = habits::choice(e, req, Set::ALL) {
        match req.options[i] {
            Choice::TakeAction {
                minion,
                action: ActionKind::Rescue,
                target: Some(t),
            } => {
                assert_eq!(st.minions[t.index()].owner, me);
                assert!(st.minions[minion.index()].blood >= 2);
                seen.rescue += 1;
            }
            Choice::TakeAction {
                action: ActionKind::Diablerie,
                target: Some(t),
                ..
            } => {
                assert_ne!(st.minions[t.index()].owner, me);
                seen.diablerie += 1;
            }
            Choice::UseAbility {
                source: AbilitySource::Card(card),
                ability,
                ..
            } => {
                let mode = AbilitySource::Card(card).mode(st, ability);
                assert!(mode
                    .effects
                    .contains(&Effect::DeclareAction(ActionKind::BurnCard)));
                assert_ne!(st.card_controller(card), me);
                seen.burn += 1;
            }
            o => panic!("habits took {o:?}"),
        }
    }
    if let Some(mask) = habits::mask(e, req, Set::ALL) {
        assert!(mask.iter().any(|&b| b) && mask.iter().any(|&b| !b));
        let ready = st.ready_unlocked(me);
        assert_eq!(ready.len(), 1);
        for (o, &kept) in req.options.iter().zip(&mask) {
            if let Choice::TakeAction { minion, action, .. } = *o {
                if minion == ready[0] && !kept {
                    assert!(!matches!(action, ActionKind::Hunt | ActionKind::Rescue));
                }
            }
        }
        seen.kept_home += 1;
    }
}

#[test]
fn habits_do_what_they_say_from_the_legal_view() {
    let mut seen = Seen::default();
    for seed in 0..40u64 {
        let mut e = Engine::with_decks(seed, &precons(seed));
        e.time_limit = 60;
        let mut agents: Vec<Habits> = (0..4)
            .map(|s| Habits(RandomLegal::new(seed * 10 + s)))
            .collect();
        while let Step::Decision(req) = e.run() {
            check(&e, &req, &mut seen);
            let mut view = e.clone();
            view.state = e.state.legal_view(req.seat);
            assert_eq!(
                habits::choice(&view, &req, Set::ALL),
                habits::choice(&e, &req, Set::ALL),
                "seed {seed}"
            );
            assert_eq!(
                habits::mask(&view, &req, Set::ALL),
                habits::mask(&e, &req, Set::ALL)
            );
            let i = agents[req.seat.index()].decide(&e, &req);
            e.choose(i);
            assert!(e.decisions < 40_000, "seed {seed}: no end");
        }
    }
    assert!(
        seen.rescue > 0 && seen.diablerie > 0 && seen.kept_home > 0 && seen.burn > 0,
        "{seen:?}"
    );
}

/// A set without a habit never takes it, and without `home` never masks.
#[test]
fn a_habit_switched_off_is_not_played() {
    let only_rescue = Set::parse("rescue").expect("a set");
    assert_eq!(Set::parse("rescue,burn,diablerie,home"), Ok(Set::ALL));
    assert!(Set::parse("rush").is_err());
    let mut rescues = 0;
    for seed in 0..20u64 {
        let mut e = Engine::with_decks(seed, &precons(seed));
        e.time_limit = 60;
        let mut agents: Vec<Habits> = (0..4)
            .map(|s| Habits(RandomLegal::new(seed * 10 + s)))
            .collect();
        while let Step::Decision(req) = e.run() {
            assert_eq!(habits::mask(&e, &req, only_rescue), None);
            if let Some(i) = habits::choice(&e, &req, only_rescue) {
                assert_eq!(
                    habits::habit(&e, req.seat, &req.options[i]),
                    Some(habits::Habit::Rescue)
                );
                rescues += 1;
            }
            let i = agents[req.seat.index()].decide(&e, &req);
            e.choose(i);
        }
    }
    assert!(rescues > 0);
}
