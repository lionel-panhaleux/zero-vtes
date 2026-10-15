//! The scripted reference bots (DESIGN §11.2 "Bots") and the decklists
//! they pilot: each style beats random play by a wide margin, answers only
//! from its seat's legal view, and every decklist plays to a finish.

use zvtes::agent::{play_out, Agent, RandomLegal};
use zvtes::bots::{Scripted, Style};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::decks::Deck;
use zvtes::game::{Engine, Step};
use zvtes::ids::SeatId;

mod common;
use common::*;

const STYLES: [Style; 2] = [Style::AggroBleed, Style::Wall];

fn precons(seed: u64) -> Vec<Deck> {
    (0..4)
        .map(|s| Deck::precon(((seed * 3 + s * 5) % 14) as usize))
        .collect()
}

/// One bot against three random players, the bot's seat and the decks
/// rotating. Measured over 200 games (2026-09-27): 83–94% game wins and
/// 3.3–3.75 VPs a game per style, against 0.1–0.2 for a random seat; the
/// bounds leave room for engine changes, not for a broken heuristic.
#[test]
fn each_style_beats_random_play() {
    const GAMES: u64 = 24;
    for style in STYLES {
        let (mut wins, mut half_vps) = (0, 0);
        for seed in 0..GAMES {
            let bot = (seed % 4) as usize;
            let mut e = Engine::with_decks(seed, &precons(seed));
            let mut agents: Vec<Box<dyn Agent>> = (0..4)
                .map(|s| -> Box<dyn Agent> {
                    if s == bot {
                        Box::new(Scripted::new(style, seed))
                    } else {
                        Box::new(RandomLegal::new(seed * 10 + s as u64))
                    }
                })
                .collect();
            half_vps += play_out(&mut e, &mut agents)[bot];
            wins += (e.state.game_win() == Some(SeatId(bot as u8))) as u64;
        }
        assert!(
            wins * 10 >= GAMES * 6,
            "{style:?}: {wins}/{GAMES} game wins"
        );
        assert!(
            half_vps as u64 >= GAMES * 4,
            "{style:?}: {half_vps} half-VPs"
        );
    }
}

/// A bot answers the same from its seat's legal view as from the full
/// state (DESIGN §5): it reads no library order, no other hand, no other
/// face-down vampire. Every 3rd decision of bot-only tables is answered
/// twice, from twin bots.
#[test]
fn bots_answer_from_their_legal_view() {
    for seed in 0..6u64 {
        let mut e = Engine::with_decks(seed, &precons(seed));
        let mut bots: Vec<Scripted> = (0..4)
            .map(|s| Scripted::new(STYLES[(seed as usize + s) % 2], seed * 10 + s as u64))
            .collect();
        while let Step::Decision(req) = e.run() {
            let s = req.seat.index();
            if e.decisions.is_multiple_of(3) {
                let mut view = e.clone();
                view.state = e.state.legal_view(req.seat);
                let blind = bots[s].clone().decide(&view, &req);
                let seeing = bots[s].clone().decide(&e, &req);
                assert_eq!(
                    blind, seeing,
                    "seed {seed}, decision {}: {:?}",
                    e.decisions, req.window
                );
            }
            let i = bots[s].decide(&e, &req);
            e.choose(i);
        }
    }
}

/// Every decklist — the 14 precons, the 14 New Blood mini-starters, the
/// anniversary decks and the TWDA decks — plays to a finish under debug assertions, at bot
/// tables (each deck in each style) and at random ones (which also empty a
/// mini's 6-card crypt and withdraw). A finished game always scores
/// someone.
#[test]
fn every_decklist_plays_out() {
    let decks: Vec<Deck> = Deck::all().collect();
    assert_eq!(
        decks.len(),
        28 + zvtes::crypt::ANNIVERSARY_PRECONS.len()
            + zvtes::crypt::SABBAT_PRECONS.len()
            + zvtes::twda::TWDA.len()
    );
    for t in 0..decks.len().div_ceil(4) {
        // Four seats a table; the last one wraps around to the first decks.
        let table: Vec<Deck> = (0..4).map(|s| decks[(t * 4 + s) % decks.len()]).collect();
        for k in 0..=STYLES.len() {
            let seed = (t * 4 + k) as u64;
            let mut e = Engine::with_decks(seed, &table);
            let mut agents: Vec<Box<dyn Agent>> = (0..4)
                .map(|s| -> Box<dyn Agent> {
                    let seed = seed * 10 + s as u64;
                    match STYLES.get(k) {
                        Some(_) => Box::new(Scripted::new(STYLES[(k + s) % 2], seed)),
                        None => Box::new(RandomLegal::new(seed)),
                    }
                })
                .collect();
            play_out(&mut e, &mut agents);
            assert!(e.state.seats.iter().any(|s| s.half_vps > 0), "seed {seed}");
        }
    }
}

/// A block attempt that cannot reach the stealth is offered (rules-qa
/// 73.8) but a bot does not make it (owner, 2026-10-15): against a hunt
/// (+1 stealth) a wall's 0-intercept vampire passes, and blocks once its
/// Methuselah holds Enhanced Senses [aus].
#[test]
fn bots_attempt_only_blocks_that_can_reach() {
    for reach in [false, true] {
        let mut e = Engine::bare(3);
        let h = e.add_minion(SeatId(0), plain("Brujah", 0), 2, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
        if reach {
            hand(&mut e, 1, &["Enhanced Senses"]);
        }
        pick(&mut e, action(h, ActionKind::Hunt));
        let req = request(&mut e);
        assert_eq!(
            (req.seat, req.window),
            (SeatId(1), WindowLabel::ActionBlock)
        );
        assert!(req.options.contains(&block(z)), "reach {reach}");
        assert_eq!(e.block_can_reach(SeatId(1), z), reach);
        for seed in 0..8 {
            let i = Scripted::new(Style::Wall, seed).decide(&e, &req);
            let want = if reach { block(z) } else { Choice::Pass };
            assert_eq!(req.options[i], want, "reach {reach}, seed {seed}");
        }
    }
}
