//! The network seats' default politics (`zvtes::politics`): referendum
//! terms by rule, no vote against one's own interest. Random games with the
//! political decks reach the referendums; from their terms on, the caller
//! answers by the rule.

use zvtes::agent::{Agent, RandomLegal};
use zvtes::cards::RefEffect;
use zvtes::decision::{Choice, WindowLabel};
use zvtes::decks::Deck;
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::politics::{mask, terms_choice};
use zvtes::state::CardId;

const POLITICS: &[&str] = &[
    "Parity Shift",
    "Kine Resources Contested",
    "Banishment",
    "Expulsion",
    "Consanguineous Boon",
    "Toreador Justicar",
    "Malkavian Justicar",
    "Archon",
    "Disputed Territory",
    "Revolutionary Council",
];

fn political_decks() -> Vec<Deck> {
    Deck::all()
        .filter(|d| {
            d.library
                .iter()
                .any(|&(c, _)| POLITICS.contains(&LIBRARY[c as usize].name))
        })
        .collect()
}

#[derive(Default)]
struct Seen {
    krc: usize,
    parity: usize,
    removal: usize,
    title: usize,
    boon: usize,
    disputed: usize,
    council: usize,
    votes: usize,
}

/// Vampires of `clan` a Methuselah controls.
fn clan_count(e: &Engine, seat: SeatId, clan: &str) -> usize {
    e.state
        .minions
        .iter()
        .filter(|m| m.present() && m.owner == seat && m.clan() == Some(clan))
        .count()
}

/// Plays `games` random games, answering terms by the rule, and checks
/// the terms when the voting starts and every vote mask.
fn check(seats: usize, games: u64) -> Seen {
    let decks = political_decks();
    let mut seen = Seen::default();
    for g in 0..games {
        let list: Vec<Deck> = (0..seats)
            .map(|s| decks[(g as usize * 3 + s) % decks.len()])
            .collect();
        let mut e = Engine::with_decks(g, &list);
        let mut random = RandomLegal::new(g);
        // The terms in progress: (caller, the seats Parity Shift could take
        // from, the fewest points the caller could give itself).
        let mut terms: Option<(SeatId, Vec<SeatId>, u8)> = None;
        for _ in 0..20_000 {
            let req = match e.run() {
                Step::Over(_) => break,
                Step::Decision(r) => r,
            };
            if req.window == WindowLabel::ReferendumTerms {
                if let Some(i) = terms_choice(&e, &req) {
                    let me = req.seat;
                    let effect = e.referendum().and_then(|r| r.def).map(|d| d.effect);
                    let owner = |m: MinionId| e.state.minions[m.index()].owner;
                    let minions: Vec<MinionId> = req
                        .options
                        .iter()
                        .filter_map(|o| match *o {
                            Choice::ChooseMinion(m) => Some(m),
                            _ => None,
                        })
                        .collect();
                    match (req.options[i], effect) {
                        (Choice::ChooseMinion(m), Some(RefEffect::PutOnChosen)) => {
                            seen.title += 1;
                            if owner(m) != me {
                                assert!(
                                    minions.iter().all(|&x| owner(x) != me),
                                    "game {g}: the title on one's own vampire"
                                );
                            }
                        }
                        (Choice::ChooseClan(c), _) => {
                            seen.boon += 1;
                            let best = req
                                .options
                                .iter()
                                .filter_map(|o| match *o {
                                    Choice::ChooseClan(x) => Some(clan_count(&e, me, x)),
                                    _ => None,
                                })
                                .max()
                                .unwrap_or(0);
                            assert_eq!(clan_count(&e, me, c), best, "game {g}: one's own clan");
                        }
                        _ => {}
                    }
                    let eligible = req
                        .options
                        .iter()
                        .filter_map(|o| match o {
                            Choice::ChooseSeat(s) => Some(*s),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    let least = req
                        .options
                        .iter()
                        .filter_map(|o| match *o {
                            Choice::AllocateTo { seat, n } if seat == req.seat => Some(n),
                            _ => None,
                        })
                        .min();
                    let t = terms.get_or_insert((req.seat, vec![], 0));
                    if !eligible.is_empty() {
                        t.1 = eligible;
                    }
                    if let Some(n) = least {
                        t.2 = n;
                    }
                    e.choose(i);
                    continue;
                }
                if let Some(m) = mask(&e, &req) {
                    check_terms_mask(&e, &req, &m, g, &mut seen);
                    let allowed: Vec<usize> = (0..m.len()).filter(|&i| m[i]).collect();
                    let i = allowed[random.decide(&e, &req) % allowed.len()];
                    e.choose(i);
                    continue;
                }
            }
            if req.window == WindowLabel::ReferendumPolling {
                let r = e.referendum().expect("a referendum");
                if let (Some((caller, eligible, least)), Some(def)) = (terms.take(), r.def) {
                    let st = &e.state;
                    let (prey, predator) = (st.prey(caller), st.predator(caller));
                    let given = |s: SeatId| -> u8 {
                        r.terms
                            .alloc
                            .iter()
                            .filter(|(x, _)| *x == s)
                            .map(|(_, n)| n)
                            .sum()
                    };
                    match def.effect {
                        RefEffect::BurnAllocated => {
                            seen.krc += 1;
                            // Two Methuselahs left: the caller must take some.
                            assert_eq!(given(caller), least, "game {g}: the caller burns least");
                            let total: u8 = r.terms.alloc.iter().map(|(_, n)| n).sum();
                            if prey != predator {
                                assert_eq!(
                                    given(prey),
                                    total - 1,
                                    "game {g}: all but one to the prey"
                                );
                                assert_eq!(
                                    given(predator),
                                    1,
                                    "game {g}: the last to the predator"
                                );
                            }
                        }
                        RefEffect::MoveAllocatedFromChosen => {
                            seen.parity += 1;
                            let chosen = r.terms.chosen.expect("a chosen Methuselah");
                            let first = [prey, predator]
                                .into_iter()
                                .find(|s| eligible.contains(s))
                                .unwrap_or(chosen);
                            assert_eq!(chosen, first, "game {g}: the prey, then the predator");
                            assert_eq!(given(caller), 3, "game {g}: all 3 to the caller");
                        }
                        _ => {}
                    }
                }
                if let Some(mask) = mask(&e, &req) {
                    seen.votes += 1;
                    let me = req.seat;
                    let burned = r.terms.alloc.iter().any(|&(s, n)| s == me && n > 0);
                    let named = r
                        .terms
                        .minions
                        .iter()
                        .any(|&m| e.state.minions[m.index()].owner == me);
                    let effect = r.def.map(|d| d.effect);
                    // Against one's minion banished or expelled, against an
                    // opponent's title.
                    let opposed = match effect {
                        Some(RefEffect::Banish | RefEffect::Expel) => named,
                        Some(RefEffect::PutOnChosen) => !named && !r.terms.minions.is_empty(),
                        _ => false,
                    };
                    for (o, &ok) in req.options.iter().zip(&mask) {
                        match *o {
                            Choice::CastVotes { favor, .. } if r.caller == me => {
                                assert_eq!(ok, favor, "game {g}: the caller votes for")
                            }
                            Choice::CastVotes { favor: true, .. }
                                if burned && def_burns(r.def.map(|d| d.effect)) =>
                            {
                                assert!(!ok, "game {g}: no vote for one's own burn")
                            }
                            Choice::CastVotes { favor: true, .. } if opposed => {
                                assert!(!ok, "game {g}: no vote for what hurts one")
                            }
                            Choice::Pass => {
                                let free = req.options.iter().zip(&mask).any(|(o, &m)| {
                                    m && matches!(
                                        o,
                                        Choice::CastVotes { .. } | Choice::CastCardVote { .. }
                                    )
                                });
                                assert_eq!(ok, !free, "game {g}: free votes are cast");
                            }
                            _ => {}
                        }
                    }
                }
            }
            let i = random.decide(&e, &req);
            e.choose(i);
        }
    }
    seen
}

/// The bounds a network caller keeps where the rule leaves it the terms.
fn check_terms_mask(
    e: &Engine,
    req: &zvtes::decision::DecisionRequest,
    m: &[bool],
    g: u64,
    seen: &mut Seen,
) {
    let (st, me) = (&e.state, req.seat);
    let r = e.referendum().expect("a referendum");
    let effect = r.def.expect("a political referendum").effect;
    let owner = |x: MinionId| st.minions[x.index()].owner;
    let neighbour = |s: SeatId| s != me && (s == st.prey(me) || s == st.predator(me));
    let least = req
        .options
        .iter()
        .filter_map(|o| match *o {
            Choice::AllocateTo { n, .. } | Choice::AllocateToCard { n, .. } => Some(n),
            _ => None,
        })
        .min()
        .unwrap_or(0);
    let near = req
        .options
        .iter()
        .any(|o| matches!(*o, Choice::ChooseMinion(x) if neighbour(owner(x))));
    for (o, &ok) in req.options.iter().zip(m) {
        match (effect, *o) {
            (RefEffect::Banish | RefEffect::Expel, Choice::ChooseMinion(x)) => {
                seen.removal += 1;
                let s = owner(x);
                assert_eq!(
                    ok,
                    s != me && (neighbour(s) || !near),
                    "game {g}: a neighbour's minion"
                );
            }
            (RefEffect::ChosenTakesChosenCard, Choice::ChooseCard(c)) => {
                seen.disputed += 1;
                assert_eq!(
                    ok,
                    st.card_controller(CardId(c)) != me,
                    "game {g}: another's location"
                );
            }
            (RefEffect::LockChosenBurnAllocated, Choice::AllocateTo { seat, n }) => {
                seen.council += 1;
                if seat != st.prey(me) {
                    assert_eq!(ok, n == least, "game {g}: points on the prey only");
                }
            }
            (RefEffect::LockChosenBurnAllocated, Choice::AllocateToCard { card, n }) => {
                seen.council += 1;
                if st.card_controller(CardId(card)) == me || n > 1 {
                    assert_eq!(ok, n == least, "game {g}: one point on another's location");
                }
            }
            _ => {}
        }
    }
}

fn def_burns(effect: Option<RefEffect>) -> bool {
    effect == Some(RefEffect::BurnAllocated)
}

#[test]
fn referendums_by_rule_in_random_games() {
    let mut all = Seen::default();
    for seats in [4, 5] {
        let seen = check(seats, 60);
        assert!(seen.krc > 0, "{seats} seats: no Kine Resources Contested");
        assert!(seen.parity > 0, "{seats} seats: no Parity Shift");
        assert!(seen.votes > 0, "{seats} seats: no steered vote");
        all.removal += seen.removal;
        all.title += seen.title;
        all.boon += seen.boon;
        all.disputed += seen.disputed;
        all.council += seen.council;
    }
    assert!(all.removal > 0, "no banishment or expulsion");
    assert!(all.title > 0, "no title");
    assert!(all.boon > 0, "no Consanguineous Boon");
}
