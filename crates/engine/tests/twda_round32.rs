//! Round 32 (`ADDED_BATCHES`): Legacies of Blood completed. Windows with a
//! single option are elided, so the scripts only stop where a real decision
//! exists.
#![allow(dead_code)]

use zvtes::cards::{AbilitySource, Holding, Target};
use zvtes::combat::Range;
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId};

mod common;
use common::*;

// ---- Helpers ----------------------------------------------------------------

/// `n` seats at seat 0's minion phase, turn 4, 10 pool each.
fn game(n: usize) -> Engine {
    let mut e = Engine::bare(n);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    e
}

/// The `nth` untitled fixture vampire matching `f` (`fixture_vampire`).
#[track_caller]
fn textless(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    fixture_vampire(|c| f(c) && c.title == Title::None, nth)
}

/// The `nth` textless vampire of `clan` with a capacity in `caps`.
#[track_caller]
fn of_clan(clan: &str, caps: std::ops::RangeInclusive<u8>, nth: usize) -> u16 {
    textless(|c| c.clan == clan && caps.contains(&c.capacity), nth)
}

/// A vampire of `clan` with a capacity in `caps` for `seat`: the `nth`
/// textless one of the clan, or, the clan's textless ones fewer (the
/// round's definitions take them), a textless one of its sect and another
/// clan turned to it (`Minion::clan_change`).
#[track_caller]
fn clan_minion(
    e: &mut Engine,
    seat: SeatId,
    clan: &'static str,
    caps: std::ops::RangeInclusive<u8>,
    nth: usize,
    blood: u8,
) -> MinionId {
    let fit = |c: &zvtes::crypt::CryptCard| c.title == Title::None && caps.contains(&c.capacity);
    let own = CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| !c.imbued && c.clan == clan && fit(c) && fixture_rank(*i as u16).is_some())
        .count();
    if nth < own {
        return e.add_minion(seat, of_clan(clan, caps, nth), blood, false);
    }
    let sect = CRYPT
        .iter()
        .find(|c| c.clan == clan)
        .expect("the clan")
        .sect;
    let m = e.add_minion(
        seat,
        textless(|c| c.sect == sect && c.clan != clan && fit(c), nth - own),
        blood,
        false,
    );
    e.state.minions[m.index()].clan_change = Some(clan);
    m
}

/// The `nth` textless vampire of `sect` with a capacity in `caps`.
#[track_caller]
fn of_sect(sect: Sect, caps: std::ops::RangeInclusive<u8>, nth: usize) -> u16 {
    textless(|c| c.sect == Some(sect) && caps.contains(&c.capacity), nth)
}

/// The crypt card `name`, advanced or not.
#[track_caller]
fn crypt_card(name: &str, adv: bool) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.adv == adv)
        .unwrap_or_else(|| panic!("{name}")) as u16
}

/// The `nth` textless Sabbat vampire.
fn sabbat(nth: usize) -> u16 {
    textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity <= 7, nth)
}

/// The crypt card `name` (not advanced).
#[track_caller]
fn find_card(name: &str) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && !c.adv)
        .unwrap_or_else(|| panic!("no {name}")) as u16
}

/// The card `name` entered play (itself, not a stand-in) on `host`, out of
/// every ash heap: its id.
#[track_caller]
fn placed_on(e: &Engine, name: &str, host: Option<MinionId>) -> CardId {
    let card = find_library(name);
    assert!(
        e.state.seats.iter().all(|s| !s.ash.contains(&card)),
        "{name} in an ash heap"
    );
    let i = e
        .state
        .cards
        .iter()
        .position(|c| c.card == card && c.in_play && !c.effect_only && c.host == host)
        .unwrap_or_else(|| panic!("{name} not in play on {host:?}"));
    CardId(i as u32)
}

/// The effect-only stand-ins of `name` in play.
fn stand_ins(e: &Engine, name: &str) -> Vec<CardId> {
    let card = find_library(name);
    (0..e.state.cards.len())
        .filter(|&i| {
            let c = &e.state.cards[i];
            c.card == card && c.present() && c.effect_only
        })
        .map(|i| CardId(i as u32))
        .collect()
}

fn ash_has(e: &Engine, seat: usize, name: &str) -> bool {
    e.state.seats[seat].ash.contains(&find_library(name))
}

/// The ability `ability` of the card in play `source`, used by `actor` on
/// `target`.
fn use_on(source: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(source),
        ability,
        actor,
        target,
    }
}

/// Pick the first option matching `want`, the fallback until then.
fn pick_matching(e: &mut Engine, want: impl Fn(&Choice) -> bool) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(&want) {
            e.choose(i);
            return;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    panic!("never offered");
}

/// Seat 0's `a` plays the political action card in hand slot 0, chooses
/// `terms` once the action succeeds (a term with one option elided), and
/// wins its referendum.
fn pass_referendum(e: &mut Engine, a: MinionId, terms: &[Choice]) {
    pick(e, play(0, 0, a));
    for &t in terms {
        loop {
            let req = request(e);
            if req.window == WindowLabel::ReferendumTerms && req.options.contains(&t) {
                e.choose(index_of(&req, t));
                break;
            }
            if req.window == WindowLabel::ReferendumPolling {
                break;
            }
            e.choose(fallback(&req));
        }
    }
    pick_matching(e, |o| {
        *o == Choice::CastCardVote { favor: true } || *o == votes(a, true)
    });
    close_polling(e);
}

/// A seat-0 Sabbat vampire with `title`, beside another seat-0 vampire and
/// a seat-1 one (the minion phase not elided).
fn titled_caller(e: &mut Engine, title: Title) -> MinionId {
    let a = e.add_minion(SeatId(0), sabbat(0), 4, false);
    e.state.minions[a.index()].granted_title = Some(title);
    e.add_minion(SeatId(0), sabbat(1), 4, false);
    a
}

/// The ally `m`'s own card.
fn body_of(e: &Engine, m: MinionId) -> CardId {
    CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.body == Some(m))
            .expect("its card") as u32,
    )
}

/// A textless Laibon of capacity 4 to 8, with these levels.
fn laibon(e: &mut Engine, seat: u8, nth: usize, levels: &[&'static str]) -> MinionId {
    let m = e.add_minion(SeatId(seat), of_sect(Sect::Laibon, 4..=8, nth), 5, false);
    e.state.minions[m.index()].gained_levels = levels.to_vec();
    m
}

/// Card `card` of the hand played by `actor` in `mode` at minion `t`.
fn play_at(card: u8, mode: u8, actor: MinionId, t: MinionId) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target: Target::Minion(t),
    }
}

/// Pass to seat 0's next minion phase: whether a determine range step
/// was asked on the way.
fn range_step_until_minion_phase(e: &mut Engine) -> bool {
    let mut step = false;
    loop {
        let req = request(e);
        step |= req.window == WindowLabel::CombatDetermineRange;
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            return step;
        }
        e.choose(fallback(&req));
    }
}

/// The ranges of the strikes the combats noted.
fn strike_ranges(e: &mut Engine) -> Vec<Range> {
    noted_combats(e)
        .into_iter()
        .filter_map(|n| match n {
            zvtes::combat::CombatNote::Strikes { range, .. } => Some(range),
            _ => None,
        })
        .collect()
}

// ---- Allies -----------------------------------------------------------------

/// Asanbonsam Ghoul: "If Asanbonsam Ghoul blocks a directed action, set the
/// range for the first round of the resulting combat to close." Blocking a
/// bleed: no determine range step (Mercury's Arrow, a maneuver, never
/// offered there), every strike close; blocking a hunt (undirected, with a
/// Raven Spy's intercept): the step offered as usual. A ghoul is no mortal.
// Rulings: [PIB 20120214] [RTR 19970630] [ANK 20180720] [ANK 20200203-2].
#[test]
fn asanbonsam_ghoul_blocks_a_directed_action_into_close_range() {
    for directed in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 5, false);
        e.state.minions[a.index()].gained_levels = vec!["cel"];
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
        let g = e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul"));
        assert!(!e.state.is_mortal(g), "a ghoul is a monster");
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        if !directed {
            in_play(&mut e, 1, "Raven Spy", Some(g));
        }
        hand(&mut e, 0, &["Mercury's Arrow"]);
        e.notes = Some(Vec::new());
        if directed {
            pick(&mut e, bleed(a));
        } else {
            pick(&mut e, action(a, ActionKind::Hunt));
        }
        pick_when(&mut e, block(g));
        if directed {
            assert!(!range_step_until_minion_phase(&mut e));
            let ranges = strike_ranges(&mut e);
            assert!(!ranges.is_empty() && ranges.iter().all(|r| *r == Range::Close));
        } else {
            let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
            assert!(req.options.contains(&play(0, 0, a)));
        }
    }
}

/// Asanbonsam Ghoul: "+1 intercept during directed actions": against a
/// bleed at another Methuselah, not against a hunt.
#[test]
fn asanbonsam_ghoul_intercepts_directed_actions_only() {
    for directed in [true, false] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 5, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
        let g = e.add_ally(SeatId(2), find_library("Asanbonsam Ghoul"));
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        if directed {
            pick(&mut e, bleed(a));
        } else {
            pick(&mut e, action(a, ActionKind::Hunt));
        }
        request(&mut e);
        let n = e.action().expect("action").intercept_of(&e.state, g);
        assert_eq!(n, i32::from(directed));
    }
}

/// Gang Tactics before Asanbonsam Ghoul's block (PIB 20120214: Gang
/// Tactics' setting first, the ghoul's after): every round close, the
/// step skipped.
// Rulings: [PIB 20120214] (Gang Tactics).
#[test]
fn gang_tactics_sets_before_an_asanbonsam_ghouls_block() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Nosferatu antitribu", 0), 5, false);
    e.state.minions[a.index()].gained_levels = vec!["cel"];
    e.add_minion(SeatId(0), plain("Nosferatu antitribu", 1), 4, false);
    let g = e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul"));
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    in_play(&mut e, 0, "Amulet of Temporal Perception", Some(a));
    hand(&mut e, 0, &["Gang Tactics", "Mercury's Arrow"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(g));
    assert!(!range_step_until_minion_phase(&mut e));
    let ranges = strike_ranges(&mut e);
    assert!(!ranges.is_empty() && ranges.iter().all(|r| *r == Range::Close));
}

/// Ohoyo Hopoksia standing in for a blocking Asanbonsam Ghoul (rules-qa
/// 124.154): her combat is not the block's resulting combat: the Ghoul's
/// "set the range for the first round of the resulting combat to close"
/// and the bleeder's Gang Tactics both lapse, Ohoyo's combat determining
/// range (owner, 2026-11-19). The Ghoul locks.
#[test]
fn ohoyo_standing_in_for_an_asanbonsam_ghoul_drops_its_range() {
    for gang in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Nosferatu antitribu", 0), 5, false);
        e.state.minions[a.index()].gained_levels = vec!["cel"];
        e.add_minion(SeatId(0), plain("Nosferatu antitribu", 1), 4, false);
        let g = e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul"));
        let o = e.add_ally(SeatId(1), find_library("Ohoyo Hopoksia (Bastet)"));
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        in_play(&mut e, 0, "Amulet of Temporal Perception", Some(a));
        hand(&mut e, 0, &["Gang Tactics", "Mercury's Arrow"]);
        pick(&mut e, bleed(a));
        if gang {
            pick(&mut e, play(0, 0, a));
        }
        let card = body_of(&e, o).0;
        pick_when(&mut e, block(g));
        pick_when(&mut e, Choice::ResolveRider { card });
        assert!(range_step_until_minion_phase(&mut e), "{gang}");
        assert!(e.state.minions[g.index()].locked, "the Ghoul locked");
        assert!(e.state.minions[o.index()].locked, "Ohoyo locked");
    }
}

/// High Top: "+1 intercept … may enter combat with any minion controlled by
/// another Methuselah as a Ⓓ action … an additional strike (limited) each
/// round … may play cards requiring basic Celerity [cel] as a vampire with
/// a capacity of 4." He enters combat with Z, plays Mercury's Arrow [cel]
/// as a vampire (its [CEL] mode never offered), then strikes again: Z
/// takes 1R and High Top's mandatory additional hand strike, 1.
// Rulings: [PIB 20121028] [LSJ 20020503].
// Rulings: [LSJ 20020503] [LSJ 20060126].
#[test]
fn high_top_plays_celerity_and_strikes_twice() {
    let mut e = game(2);
    let h = e.add_ally(SeatId(0), find_library("High Top"));
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 5, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 0, &["Mercury's Arrow"]);
    let top = body_of(&e, h);
    assert_eq!(
        zvtes::cards::plays_card_as_vampire(&e.state, h, find_library("Mercury's Arrow"), 0)
            .map(|g| (g.capacity, g.sect)),
        Some((4, Sect::Independent))
    );
    pick(&mut e, use_on(top, 1, Some(h), Target::Minion(z)));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(!req.options.contains(&play(0, 1, h)), "[CEL] not basic");
    pick(&mut e, play(0, 0, h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 5 - 1, "long: the arrow only");
    let mut e = game(2);
    let h = e.add_ally(SeatId(0), find_library("High Top"));
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 5, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let top = body_of(&e, h);
    pick(&mut e, use_on(top, 1, Some(h), Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 5 - 1 - 1, "close: two hand strikes");
}

/// High Top: "If High Top has less than 3 life during your unlock phase,
/// he gains 1 life." At 2 life he heals to 3; at 3 he stays. His +1
/// intercept against a hunt.
#[test]
fn high_top_heals_below_three_and_intercepts() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 5, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    let h = e.add_ally(SeatId(1), find_library("High Top"));
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    e.state.minions[h.index()].blood = 2;
    pick(&mut e, action(a, ActionKind::Hunt));
    request(&mut e);
    assert_eq!(e.action().expect("action").intercept_of(&e.state, h), 1);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, h), 3);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, h), 3);
}

// ---- Equipment ---------------------------------------------------------------

/// Kerrie: "Alternatively, inflict strength ranged damage as a strike and,
/// after strike resolution, turn this weapon face down (out of play) until
/// the end of the action." [LSJ 20051220] its ranged damage is a melee
/// weapon's: Blood Tempering's +1 adds to it (Z takes 2). Face down, it is
/// no melee weapon its bearer holds; back as the action ends. [LSJ
/// 20060825] a ranged weapon (Anachronism's reader), Dagger too.
// Rulings: [LSJ 20051220] [LSJ 20060825].
#[test]
fn kerrie_ranged_strike_takes_melee_weapon_bonuses() {
    use zvtes::cards::Modifier as M;
    for name in ["Kerrie", "Dagger"] {
        assert!(zvtes::cards::is_ranged_weapon(find_library(name)), "{name}");
        let mut e = game(2);
        let a = laibon(&mut e, 0, 0, &[]);
        laibon(&mut e, 0, 1, &[]);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 5, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 3), 4, false);
        let k = in_play(&mut e, 0, name, Some(a));
        let t = in_play(&mut e, 0, "Blood Tempering", Some(a));
        e.state.layers.retain(|l| l.source != t);
        e.state.layers.push(zvtes::cards::Layer {
            scope: zvtes::cards::Scope::InPlay,
            modifier: M::WeaponDamage(1),
            source: t,
        });
        e.state.cards[t.index()].on_card = Some(k);
        let amulet = in_play(&mut e, 0, "Amulet of Temporal Perception", Some(a));
        e.state.layers.push(zvtes::cards::Layer {
            scope: zvtes::cards::Scope::InPlay,
            modifier: M::HostCombatPress(1),
            source: amulet,
        });
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, use_on(k, 1, None, Target::None));
        pick_when(&mut e, Choice::UsePress);
        assert_eq!(blood(&e, z), 5 - 2, "{name}: strength 1 + 1");
        assert!(e.state.cards[k.index()].out_of_play, "face down");
        assert!(!e.state.holds_a(a, Holding::MeleeWeapon), "out of play");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let c = &e.state.cards[k.index()];
        assert!(c.present() && c.host == Some(a), "back as the action ends");
    }
}

/// Kerrie: "Requires a Laibon." Equipped by a Laibon, not by another
/// vampire.
#[test]
fn kerrie_needs_a_laibon() {
    let mut e = game(2);
    let l = laibon(&mut e, 0, 0, &[]);
    let o = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kerrie"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, l)));
    assert!(!req.options.contains(&play(0, 0, o)));
}

// ---- Retainers ----------------------------------------------------------------

/// Elephant Guardian: "When this minion is in combat with an acting minion,
/// the acting minion takes 2 damage during strike resolution each round if
/// the range is close." Its Laibon blocks A: A takes Z's hand strike and 2;
/// A at long range (Mercury's Arrow): nothing; its Laibon acting and
/// blocked: nothing from it. "A minion can have only one Elephant
/// Guardian": no second employed.
// Rulings: [ANK 20180612] [RTR 19941109].
// Rulings: [ANK 20180612] [ANK 20211127] [RTR 19941109] [TOM 19951216] [PIB 20140324] [ANK 20200422].
#[test]
fn elephant_guardian_tramples_the_acting_minion_at_close_range() {
    for case in ["blocks", "long", "acts"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 5, false);
        e.state.minions[a.index()].gained_levels = vec!["cel"];
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
        let z = laibon(&mut e, 1, 0, &[]);
        let z2 = laibon(&mut e, 1, 1, &[]);
        in_play(&mut e, 1, "Elephant Guardian", Some(z));
        if case == "acts" {
            e.state.turn = 5;
            e.state.active = SeatId(1);
            hand(&mut e, 1, &["Elephant Guardian"]);
            let req = request(&mut e);
            assert!(!req.options.contains(&play(0, 0, z)), "only one");
            assert!(req.options.contains(&play(0, 0, z2)));
            pick(&mut e, bleed(z));
            pick_when(&mut e, block(a));
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, z), 5 - 1, "a's hand strike only");
            continue;
        }
        hand(&mut e, 0, &["Mercury's Arrow"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if case == "long" {
            pick_when(&mut e, play(0, 0, a));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, a), 5 - 1, "the arrow's cost only");
        } else {
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, a), 5 - 1 - 2);
        }
    }
}

/// Stone Dog: "The Gargoyle with this retainer can enter combat with a
/// ready minion controlled by another Methuselah as a Ⓓ action. When the
/// Gargoyle with this retainer is in combat, the opposing minion takes 1
/// damage during the initial strike resolution phase of each round when the
/// range is close." Its Gargoyle enters combat with Z: Z takes the hand
/// strike and 1. On a non-Gargoyle, the action is not offered.
// Rulings: [ANK 20180612] [RTR 19941109] [LSJ 20080604].
// Rulings: [ANK 20180612] [ANK 20211127] [RTR 19941109] [TOM 19951216] [PIB 20140324] [ANK 20200422].
#[test]
fn stone_dog_rushes_and_bites_at_close_range() {
    for gargoyle in [true, false] {
        let mut e = game(2);
        let g = if gargoyle {
            e.add_minion(SeatId(0), of_clan("Gargoyle", 4..=8, 0), 5, false)
        } else {
            e.add_minion(SeatId(0), plain("Ventrue", 0), 5, false)
        };
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 5, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 3), 4, false);
        let dog = in_play(&mut e, 0, "Stone Dog", Some(g));
        let rush = use_on(dog, 0, Some(g), Target::Minion(z));
        if !gargoyle {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, rush);
            continue;
        }
        pick(&mut e, rush);
        assert!(e
            .action()
            .expect("action")
            .directed_seat(&e.state)
            .is_some());
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), 5 - 1 - 1);
    }
}

/// Ilomba: "Put Ilomba on any minion (this is a +1 stealth Ⓓ action if that
/// minion is controlled by another Methuselah)." The employ's own +1
/// stealth, directed at another Methuselah's minion, undirected at one's
/// own: the same stealth either way.
#[test]
fn ilomba_on_another_methuselahs_minion_is_a_directed_employ() {
    let mut stealth = Vec::new();
    for others in [true, false] {
        let mut e = game(2);
        let a = laibon(&mut e, 0, 0, &[]);
        let b = laibon(&mut e, 0, 1, &[]);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Ilomba"]);
        pick(&mut e, play_at(0, 0, a, if others { z } else { b }));
        request(&mut e);
        let ctx = e.action().expect("action");
        assert_eq!(ctx.directed_seat(&e.state), others.then_some(SeatId(1)));
        stealth.push(ctx.stealth(&e.state));
    }
    assert_eq!(stealth, [1, 1]);
}

/// The first offer of card `source`'s ability `ability` (any actor and
/// target), the fallback taken until then.
fn card_ability(e: &mut Engine, source: CardId, ability: u8) -> Choice {
    for _ in 0..300 {
        let req = request(e);
        if let Some(o) = req.options.iter().find(|o| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: i, .. }
                if *c == source && *i == ability)
        }) {
            return *o;
        }
        e.choose(fallback(&req));
    }
    panic!("{source:?}'s ability {ability} never offered");
}

/// Spike-Thrower: "Only usable at long range. … If any damage from this
/// strike is successfully inflicted on an opposing vampire, that vampire
/// does not unlock as normal on his or her next unlock phase." High Top
/// maneuvers to long range and throws: Z, locked, takes 1 and stays locked
/// through its next unlock phase; at close range it is not offered.
// Rulings: [LSJ 20050114] (prevent normal unlock).
#[test]
fn spike_thrower_pins_a_vampire_it_hits() {
    for long in [true, false] {
        let mut e = game(2);
        let h = e.add_ally(SeatId(0), find_library("High Top"));
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 5, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        e.state.minions[z.index()].locked = true;
        let spike = in_play(&mut e, 0, "Spike-Thrower", Some(h));
        let top = body_of(&e, h);
        pick(&mut e, use_on(top, 1, Some(h), Target::Minion(z)));
        let throw = use_on(spike, 0, None, Target::None);
        if !long {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, throw);
            continue;
        }
        let maneuver = card_ability(&mut e, top, 0);
        pick(&mut e, maneuver);
        pick_when(&mut e, throw);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), 5 - 1);
        assert!(e.state.minions[z.index()].skip_unlock);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(e.state.minions[z.index()].locked, "no normal unlock");
    }
}

/// Shaman: "This Laibon has +1 intercept. When acting, this Laibon can
/// shuffle this retainer back into his owner's library to get +1 stealth."
/// Z's Raven Spy outreaches A's bleed: the Shaman shuffled into seat 0's
/// library (not burned) for +1 stealth. A Shaman's Laibon gets +1
/// intercept.
#[test]
fn shaman_intercepts_and_shuffles_in_for_stealth() {
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &[]);
    laibon(&mut e, 0, 1, &[]);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let w = laibon(&mut e, 1, 2, &[]);
    in_play(&mut e, 1, "Shaman", Some(w));
    in_play(&mut e, 1, "Raven Spy", Some(z));
    let shaman = in_play(&mut e, 0, "Shaman", Some(a));
    pick(&mut e, bleed(a));
    request(&mut e);
    assert_eq!(e.action().expect("action").intercept_of(&e.state, w), 1);
    let before = e.action().expect("action").stealth(&e.state);
    pick_when(&mut e, block(z));
    let shuffle = card_ability(&mut e, shaman, 0);
    pick(&mut e, shuffle);
    assert_eq!(e.action().expect("action").stealth(&e.state), before + 1);
    assert!(!e.state.cards[shaman.index()].in_play);
    assert!(e.state.seats[0].library.contains(&find_library("Shaman")));
    assert!(!ash_has(&e, 0, "Shaman"));
}

/// Redcap Wilder: "[myt] Vampires with capacity less than 7 must burn a
/// blood to attempt to block the minion with this retainer. [MYT] As above,
/// but the cost to employ this retainer is reduced by 1 blood." A capacity
/// 4-6 vampire burns 1 to attempt; a capacity 7+ one does not; an ally
/// does not; one with no blood is not offered the attempt. [MYT]: 1 blood.
#[test]
fn redcap_wilder_tolls_small_vampire_blockers() {
    for blocker in ["small", "big", "ally", "empty"] {
        let mut lost = Vec::new();
        for redcap in [true, false] {
            let mut e = game(2);
            let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 5, false);
            e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
            if redcap {
                in_play(&mut e, 0, "Redcap Wilder", Some(a));
            }
            let small = |c: &zvtes::crypt::CryptCard| (4..=6).contains(&c.capacity);
            let z = match blocker {
                "small" => e.add_minion(SeatId(1), textless(small, 2), 4, false),
                "big" => e.add_minion(
                    SeatId(1),
                    textless(|c| (7..=9).contains(&c.capacity), 0),
                    7,
                    false,
                ),
                "ally" => e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul")),
                _ => e.add_minion(SeatId(1), textless(small, 2), 0, false),
            };
            e.add_minion(SeatId(1), textless(small, 3), 4, false);
            let before = blood(&e, z);
            pick(&mut e, bleed(a));
            if blocker == "empty" {
                if redcap {
                    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(z));
                } else {
                    offering(&mut e, block(z));
                }
                continue;
            }
            pick_when(&mut e, block(z));
            lost.push(before - blood(&e, z));
        }
        if blocker != "empty" {
            assert_eq!(lost[0] - lost[1], u8::from(blocker == "small"), "{blocker}");
        }
    }
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 5, false);
    e.state.minions[a.index()].gained_levels = vec!["myt", "myt"];
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Redcap Wilder"]);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 5 - 1);
    placed_on(&e, "Redcap Wilder", Some(a));
}

/// Redcap Wilder, two on the acting minion: each tolls the attempt (rules-qa
/// 69.12): a capacity 4-6 blocker burns 2 to attempt the block.
#[test]
fn redcap_wilder_two_copies_toll_twice() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 5, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    in_play(&mut e, 0, "Redcap Wilder", Some(a));
    in_play(&mut e, 0, "Redcap Wilder", Some(a));
    let small = |c: &zvtes::crypt::CryptCard| (4..=6).contains(&c.capacity);
    let z = e.add_minion(SeatId(1), textless(small, 2), 4, false);
    e.add_minion(SeatId(1), textless(small, 3), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    assert_eq!(blood(&e, z), 4 - 2, "two tolls");
}

/// Kduva's Mask: "This Laibon gets +1 bleed and 2 additional votes.
/// Reaction cards that require Auspex [aus] cost an additional blood while
/// this Laibon is acting." Its Laibon bleeds for 2; Z's Eagle's Sight [aus]
/// costs 1 blood; without the mask, nothing. On a non-Laibon, nothing.
#[test]
fn kduvas_mask_taxes_auspex_reactions_while_its_laibon_acts() {
    let mut left = Vec::new();
    for (mask, laibon_bearer) in [(true, true), (false, true), (true, false)] {
        let mut e = game(2);
        let a = if laibon_bearer {
            laibon(&mut e, 0, 0, &[])
        } else {
            e.add_minion(SeatId(0), plain("Ventrue", 0), 5, false)
        };
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 3), 4, false);
        e.state.minions[z.index()].gained_levels = vec!["aus"];
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 4), 4, false);
        if mask {
            in_play(&mut e, 0, "Kduva's Mask", Some(a));
        }
        hand(&mut e, 1, &["Eagle's Sight"]);
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::Stealth(1),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        pick(&mut e, bleed(a));
        request(&mut e);
        let bonus = mask && laibon_bearer;
        assert_eq!(
            e.action().expect("action").bleed_amount(&e.state),
            1 + u8::from(bonus)
        );
        assert_eq!(e.state.votes_held(a), 2 * u8::from(bonus));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 0, z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        left.push(blood(&e, z));
    }
    assert_eq!(left, [left[1] - 1, left[1], left[1]], "the mask's blood");
}

/// Informant: "The Informant cannot act or block. During your unlock phase,
/// any Methuselah who controls a ready Ishtarri may look at your hand.
/// During your discard phase, your predator takes control of the
/// Informant." Seat 0's Informant passes to seat 3 at seat 0's discard
/// phase; at seat 3's unlock phase seat 2 (a ready Ishtarri) looks at seat
/// 3's hand, seat 1 (none) does not. It is never offered an action.
#[test]
fn informant_shows_the_hand_and_passes_to_the_predator() {
    let mut e = game(4);
    let i = e.add_ally(SeatId(0), find_library("Informant"));
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), of_clan("Ishtarri", 3..=11, 0), 4, false);
    e.add_minion(SeatId(3), plain("Ventrue", 2), 4, false);
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::TakeAction { minion, .. } if *minion == i),
    );
    assert_eq!(e.state.minions[i.index()].owner, SeatId(3));
    e.state.peeks.clear();
    until(&mut e, SeatId(3), WindowLabel::MinionPhase);
    assert!(e.state.peeks.contains(&(SeatId(2), SeatId(3))));
    assert!(!e.state.peeks.contains(&(SeatId(1), SeatId(3))));
}

/// Ghoul Messenger: "The vampire with this retainer may move it to another
/// ready vampire as a +1 stealth action. If the action is successful, the
/// new employer gains 1 blood from the blood bank, and the acting vampire
/// unlocks at the end of the turn." To seat 1's Z: directed, on Z, Z +1
/// blood, A unlocked as the turn ends.
#[test]
fn ghoul_messenger_moves_and_feeds_the_new_employer() {
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &[]);
    laibon(&mut e, 0, 1, &[]);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let gm = in_play(&mut e, 0, "Ghoul Messenger", Some(a));
    let go = use_on(gm, 0, Some(a), Target::Minion(z));
    pick(&mut e, go);
    assert!(e
        .action()
        .expect("action")
        .directed_seat(&e.state)
        .is_some());
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[gm.index()].host, Some(z));
    assert_eq!(blood(&e, z), 4 + 1);
    assert!(e.state.minions[a.index()].locked);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[a.index()].locked,
        "unlocked as the turn ended"
    );
}

/// Ghoul Messenger: "If the action is blocked, cancel combat, burn the
/// Messenger, and do not lock the blocking minion." Blocked by Z: no
/// combat, the Messenger in its owner's ash heap, Z unlocked, A not hurt.
// Rulings: [ANK 20220116] (triggered by block).
// Rulings: [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR 19991206] [LSJ 20091125].
#[test]
fn ghoul_messenger_blocked_burns_without_combat() {
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &[]);
    let b = laibon(&mut e, 0, 1, &[]);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gm = in_play(&mut e, 0, "Ghoul Messenger", Some(a));
    in_play(&mut e, 1, "Raven Spy", Some(z));
    e.notes = Some(Vec::new());
    pick(&mut e, use_on(gm, 0, Some(a), Target::Minion(b)));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Ghoul Messenger"));
    assert!(!e.state.cards[gm.index()].in_play);
    assert!(!e.state.minions[z.index()].locked, "the blocker not locked");
    assert_eq!((blood(&e, a), blood(&e, b)), (5, 5), "no combat, no blood");
    assert!(strike_ranges(&mut e).is_empty());
}

// ---- Political actions --------------------------------------------------------

/// The referendum's tally (votes in favor, against), polling still open.
fn tally(e: &Engine) -> (u32, u32) {
    e.referendum().expect("referendum").tally(&e.state)
}

/// Overseer: "In this referendum, each Guruhi gets one additional vote. Any
/// votes or ballots any magaji cast in favor of this referendum are
/// ignored." The Guruhi caller votes 1 for, seat 1's Guruhi 1 against;
/// seat 2's magaji's 2 in favor ignored, against counted.
#[test]
fn overseer_gives_guruhi_a_vote_and_ignores_magajis_in_favor() {
    for magaji_favor in [true, false] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), of_clan("Guruhi", 5..=11, 0), 5, false);
        e.add_minion(SeatId(0), of_clan("Guruhi", 5..=11, 1), 4, false);
        let z = e.add_minion(SeatId(1), of_clan("Guruhi", 1..=11, 2), 4, false);
        let w = laibon(&mut e, 2, 3, &[]);
        e.state.minions[w.index()].granted_title = Some(Title::Magaji);
        hand(&mut e, 0, &["Overseer"]);
        pick(&mut e, play(0, 0, a));
        to_referendum(&mut e);
        pick_when(&mut e, votes(a, true));
        pick_when(&mut e, votes(z, false));
        pick_when(&mut e, votes(w, magaji_favor));
        request(&mut e);
        let want = if magaji_favor { (1, 1) } else { (1, 3) };
        assert_eq!(tally(&e), want);
    }
}

/// Overseer: "If this referendum passes, put this card on the acting Laibon
/// to represent the Laibon title of magaji (worth 2 votes)." "Requires a
/// Laibon with a capacity above 4": a capacity 4 Laibon is not offered it.
#[test]
fn overseer_makes_its_laibon_magaji() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Guruhi", 5..=11, 0), 5, false);
    let small = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 1..=4, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Overseer"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, small)), "capacity 4");
    pass_referendum(&mut e, a, &[]);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Overseer", Some(a));
    assert_eq!(e.state.title_of(a), Title::Magaji);
    assert_eq!(e.state.votes_held(a), 2);
}

/// Political Flux: "Choose a number from 1 to 12. Successful referendum
/// means this is the amount of pool the next predator will earn for ousting
/// his or her prey (instead of the usual 6 pool)." 12 chosen: seat 0 ousts
/// seat 1 for 12; the next oust pays 6 again.
#[test]
fn political_flux_sets_the_next_oust_reward() {
    let mut e = game(3);
    let a = titled_caller(&mut e, Title::Cardinal);
    let b = e.add_minion(SeatId(0), sabbat(4), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Political Flux"]);
    pass_referendum(&mut e, a, &[set_x(12)]);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.next_oust_pool.0, Some(12));
    e.state.seats[1].pool = 1;
    let before = e.state.seats[0].pool;
    pick(&mut e, bleed(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert_eq!(e.state.seats[0].pool, before + 12);
    assert_eq!(e.state.next_oust_pool.0, None, "used up");
}

/// Political Flux, two Methuselahs ousted at once: every predator paid in
/// that oust takes the Flux's amount (owner, rules-qa 124.173; LSJ
/// 20010612 "Both, yes."). A Flux of 12 standing; seat 0's referendum with
/// Scorn of Adonis, seats 1 and 3 (1 pool each) voting against, both
/// ousted at the tally: seat 0 (seat 1's predator) and seat 2 (seat 3's)
/// gain 12 each.
#[test]
fn political_flux_goes_to_every_predator_of_one_oust() {
    let mut e = game(4);
    let a = titled_caller(&mut e, Title::Cardinal);
    e.state.minions[a.index()].clan_change = Some("Toreador");
    let v1 = e.add_minion(SeatId(1), sabbat(2), 4, false);
    let v3 = e.add_minion(SeatId(3), sabbat(3), 4, false);
    for v in [v1, v3] {
        e.state.minions[v.index()].granted_title = Some(Title::Bishop);
    }
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    e.state.next_oust_pool.0 = Some(12);
    for s in [1, 3] {
        e.state.seats[s].pool = 1;
    }
    hand(&mut e, 0, &["Political Flux", "Scorn of Adonis"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, set_x(1));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, votes(a, true));
    pick_when(&mut e, votes(v1, false));
    pick_when(&mut e, votes(v3, false));
    let before = (e.state.seats[0].pool, e.state.seats[2].pool);
    close_polling(&mut e);
    request(&mut e);
    assert!(e.state.seats[1].ousted && e.state.seats[3].ousted);
    assert_eq!(e.state.seats[0].pool, before.0 + 12);
    assert_eq!(e.state.seats[2].pool, before.1 + 12);
}

/// Pit of Contemplation: "move a vampire controlled by your predator or prey
/// with capacity less than X to their owner's uncontrolled region" — the
/// owner's, not the controller's: seat 1's vampire taken for good from seat
/// 2 goes to seat 2's uncontrolled region (Banishment's own move goes to
/// the controller's).
#[test]
fn pit_of_contemplation_sends_a_vampire_to_its_owners_uncontrolled_region() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), textless(|c| c.capacity == 2, 0), 2, false);
    e.state.minions[v.index()].origin = Some(SeatId(2));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let pit = in_play(&mut e, 0, "Pit of Contemplation", None);
    e.state.cards[pit.index()].counters = 4;
    pick_when(&mut e, use_on(pit, 1, None, Target::Minion(v)));
    request(&mut e);
    assert!(!e.state.minions[v.index()].in_play);
    let home = |s: usize| {
        e.state.seats[s]
            .uncontrolled
            .iter()
            .any(|u| u.banished.as_ref().is_some_and(|b| b.minion == v))
    };
    assert!(home(2), "its owner's");
    assert!(!home(1), "not its controller's");
    assert_eq!(e.state.minions[v.index()].owner, SeatId(2));
}

/// [LSJ 20011015] "The scarce penalty considers the apparent clan while
/// controlled (in case of Clan Impersonation and Banishment)": seat 1's
/// scarce vampire, its clan changed to Ventrue, banished by seat 0's
/// Banishment, keeps the change (rules-qa 25.1: a banished vampire keeps
/// everything); influenced back beside seat 1's Ventrue, its toll is 3.
#[test]
fn a_banished_scarce_vampire_pays_its_toll_by_its_apparent_clan() {
    let mut e = game(2);
    let a = titled_caller(&mut e, Title::Cardinal);
    let card = CRYPT
        .iter()
        .position(|c| {
            c.scarce && !c.adv && c.capacity <= 3 && c.clan != "Ventrue" && c.title == Title::None
        })
        .expect("a small scarce vampire") as u16;
    let t = e.add_minion(SeatId(1), card, 2, false);
    e.state.minions[t.index()].clan_change = Some("Ventrue");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Banishment"]);
    pass_referendum(&mut e, a, &[Choice::ChooseMinion(t)]);
    request(&mut e);
    assert!(!e.state.minions[t.index()].in_play, "banished");
    e.state.seats[1].uncontrolled[0].counters = CRYPT[card as usize].capacity;
    to_window(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
    let before = e.state.seats[1].pool;
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    request(&mut e);
    assert!(e.state.minions[t.index()].in_play, "back");
    assert_eq!(e.state.seats[1].pool, before - 3, "one Ventrue: 3 pool");
}

/// The Eldest Command Undeath: "Choose a vampire controlled by your prey
/// with capacity less than X. Successful referendum means that vampire is
/// burned." X = 3: the prey's capacity-2 vampire chosen and burned, its
/// capacity-3 one never offered (its capacity-1 one is); X blood paid. The
/// prey with no vampire: not offered (rules-digest D1; LSJ 20100129).
// Rulings: [LSJ 20100129] [ANK 20191228].
#[test]
fn eldest_command_undeath_burns_a_small_vampire_of_the_prey() {
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &[]);
    e.state.minions[a.index()].granted_title = Some(Title::Magaji);
    laibon(&mut e, 0, 1, &[]);
    let v2 = e.add_minion(SeatId(1), textless(|c| c.capacity == 2, 0), 2, false);
    let v3 = e.add_minion(SeatId(1), textless(|c| c.capacity == 3, 0), 3, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity == 1, 0), 1, false);
    hand(&mut e, 0, &["Eldest Command Undeath, The"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, set_x(3));
    let req = offering(&mut e, Choice::ChooseMinion(v2));
    assert!(!req.options.contains(&Choice::ChooseMinion(v3)));
    pick(&mut e, Choice::ChooseMinion(v2));
    pick_matching(&mut e, |o| *o == votes(a, true));
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[v2.index()].in_play, "burned");
    assert!(e.state.minions[v3.index()].in_play);
    assert_eq!(blood(&e, a), 5 - 3, "X blood");
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &[]);
    laibon(&mut e, 0, 1, &[]);
    e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul"));
    hand(&mut e, 0, &["Eldest Command Undeath, The"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { card: 0, .. })));
    let _ = a;
}

/// The Eldest Command Undeath: "This acting vampire cannot gain blood this
/// action. Any blood he or she gains goes to the blood bank instead" — the
/// whole action: blocked, the Laibon's Theft of Vitae [tha] in the block's
/// combat steals 1 from the blocker, and none reaches him.
#[test]
fn eldest_command_undeath_bars_blood_in_the_block_combat() {
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &["tha"]);
    e.state.minions[a.index()].blood = 3;
    laibon(&mut e, 0, 1, &[]);
    e.add_minion(SeatId(1), textless(|c| c.capacity == 2, 0), 2, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 0), 4, false);
    in_play(&mut e, 1, "Raven Spy", Some(z));
    hand(
        &mut e,
        0,
        &["Eldest Command Undeath, The", "Theft of Vitae"],
    );
    pick(&mut e, play(0, 0, a));
    pick(&mut e, set_x(3));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let before = blood(&e, a);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 1, "1 stolen");
    assert_eq!(blood(&e, a), before - 1, "none gained; the hand strike's 1");
}

/// [LSJ 20081002] "If you play Voter Captivation on the referendum, you can
/// move up to 2 counters to your pool, and 0 to your vampire." Passed by 3
/// (the magaji's 2 and a political card's vote): [PRE] moves 2 to the
/// pool, the third blood goes to the bank, not to the vampire.
#[test]
fn eldest_command_undeath_sends_voter_captivation_blood_to_the_bank() {
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &["pre", "pre"]);
    e.state.minions[a.index()].granted_title = Some(Title::Magaji);
    laibon(&mut e, 0, 1, &[]);
    let v2 = e.add_minion(SeatId(1), textless(|c| c.capacity == 2, 0), 2, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity == 1, 0), 1, false);
    hand(
        &mut e,
        0,
        &[
            "Eldest Command Undeath, The",
            "Voter Captivation",
            "Political Flux",
        ],
    );
    pick(&mut e, play(0, 0, a));
    pick(&mut e, set_x(3));
    pick_when(&mut e, Choice::ChooseMinion(v2));
    pick_matching(&mut e, |o| *o == votes(a, true));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    let before = (e.state.seats[0].pool, blood(&e, a));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, set_x(2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, before.0 + 2, "2 to the pool");
    assert_eq!(blood(&e, a), before.1, "the third to the bank");
}

/// The Secret Must Be Kept: "Choose an ally or retainer. If the acting
/// Laibon is a magaji, choose up to two allies or retainers instead.
/// Successful referendum means the chosen allies and retainers are burned,
/// and their controllers burn 1 pool (each Methuselah burns no more than 1
/// pool …)." A magaji picks seat 1's ally and retainer: both burned, seat 1
/// burns 1 pool. An untitled Laibon is offered one pick only.
#[test]
fn the_secret_must_be_kept_burns_allies_and_retainers() {
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &[]);
    e.state.minions[a.index()].granted_title = Some(Title::Magaji);
    laibon(&mut e, 0, 1, &[]);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul"));
    let spy = in_play(&mut e, 1, "Raven Spy", Some(z));
    hand(&mut e, 0, &["Secret Must Be Kept, The"]);
    let ally_card = body_of(&e, ally);
    pass_referendum(
        &mut e,
        a,
        &[Choice::ChooseCard(ally_card.0), Choice::ChooseCard(spy.0)],
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play);
    assert!(!e.state.cards[spy.index()].in_play);
    assert!(ash_has(&e, 1, "Raven Spy") && ash_has(&e, 1, "Asanbonsam Ghoul"));
    assert_eq!(e.state.seats[1].pool, 10 - 1, "once");
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &[]);
    e.state.minions[a.index()].granted_title = Some(Title::Primogen);
    laibon(&mut e, 0, 1, &[]);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul"));
    let spy = in_play(&mut e, 1, "Raven Spy", Some(z));
    hand(&mut e, 0, &["Secret Must Be Kept, The"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ChooseCard(spy.0));
    assert_eq!(
        request(&mut e).window,
        WindowLabel::ReferendumPolling,
        "one pick"
    );
}

/// [LSJ 20100129] The Secret Must Be Kept with no ally or retainer in play:
/// not offered.
// Rulings: [LSJ 20100129] [ANK 20191228].
#[test]
fn the_secret_must_be_kept_needs_an_ally_or_retainer() {
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &[]);
    laibon(&mut e, 0, 1, &[]);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Secret Must Be Kept, The"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)));
}

/// Poker: "If all of the damage done by this strike is prevented by a card
/// or cards that require Fortitude [for], burn this weapon after strike
/// resolution." Its 2 prevented by Soak [for]: burned. 3 (A at +1
/// strength), Soak's 2 and Leather Jacket's rest: not all by [for], kept.
/// Soak's 2 of 3: kept. Against a Kiasyd: aggravated (torpor).
#[test]
fn poker_burns_when_fortitude_prevents_all_of_its_strike() {
    for case in ["soak", "mixed", "partial", "kiasyd"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = if case == "kiasyd" {
            e.add_minion(SeatId(1), of_clan("Kiasyd", 4..=8, 0), 3, false)
        } else {
            e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 5, false)
        };
        e.state.minions[z.index()].gained_levels = vec!["for"];
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 3), 4, false);
        let poker = in_play(&mut e, 0, "Poker", Some(a));
        if case != "soak" && case != "kiasyd" {
            e.state.minions[a.index()].timed.push((
                zvtes::cards::Lasting::Strength(1),
                zvtes::state::Expiry::EndOfTurn,
                SeatId(0),
            ));
        }
        let jacket = (case == "mixed").then(|| in_play(&mut e, 1, "Leather Jacket", Some(z)));
        if case != "kiasyd" {
            hand(&mut e, 1, &["Soak"]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, use_on(poker, 0, None, Target::None));
        if case == "kiasyd" {
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(e.state.minions[z.index()].in_torpor, "aggravated");
            continue;
        }
        pick_when(&mut e, play(0, 0, z));
        if let Some(j) = jacket {
            let use_jacket = card_ability(&mut e, j, 0);
            pick(&mut e, use_jacket);
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let burned = case == "soak";
        assert_eq!(!e.state.cards[poker.index()].present(), burned, "{case}");
        assert_eq!(ash_has(&e, 0, "Poker"), burned, "{case}");
    }
}

/// Poker: a [for] prevention split by its preventer between the strike and
/// other damage (rules-qa 29.3) still counts as [for]: Poker's 2 and
/// Murder of Crows' 1 on Z, Soak [for] prevents 2, Z's Methuselah books
/// both to the strike — every point of the strike prevented by Fortitude
/// cards, Poker burns; booked 1 to the strike, it stays.
#[test]
fn poker_counts_a_fortitude_prevention_split_in_the_allocation() {
    for strike in [2, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 5, false);
        e.state.minions[z.index()].gained_levels = vec!["for"];
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 3), 4, false);
        let poker = in_play(&mut e, 0, "Poker", Some(a));
        in_play(&mut e, 0, "Murder of Crows", Some(a));
        hand(&mut e, 1, &["Soak"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, use_on(poker, 0, None, Target::None));
        pick_when(&mut e, play(0, 0, z));
        let req = until(&mut e, SeatId(1), WindowLabel::CombatPreventionAllocation);
        assert_eq!(req.options, [Choice::SetX(1), Choice::SetX(2)]);
        e.choose(index_of(&req, Choice::SetX(strike)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let burned = strike == 2;
        assert_eq!(!e.state.cards[poker.index()].present(), burned, "{strike}");
        assert_eq!(ash_has(&e, 0, "Poker"), burned, "{strike}");
    }
}

/// Swarm: "Put the Swarm on any ready minion (employing the Swarm is a +1
/// stealth Ⓓ action if that minion is controlled by another Methuselah)."
/// On seat 1's Z: directed, the Swarm itself on Z with 1 life, 1 blood
/// paid. "If this action is blocked, put the Swarm on the acting minion
/// instead (no cost is paid)": blocked, on A before combat, unpaid.
// Rulings: [LSJ 20090601] [ANK 20230408].
// Rulings: [ANK 20230408] [LSJ 20050313] [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR 19991206] [LSJ 20091125].
#[test]
fn swarm_goes_on_another_minion_or_on_the_actor_if_blocked() {
    for blocked in [false, true] {
        let mut e = game(2);
        let a = laibon(&mut e, 0, 0, &[]);
        laibon(&mut e, 0, 1, &[]);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        if blocked {
            in_play(&mut e, 1, "Raven Spy", Some(w));
        }
        hand(&mut e, 0, &["Swarm"]);
        pick(&mut e, play_at(0, 0, a, z));
        request(&mut e);
        assert_eq!(
            e.action().expect("action").directed_seat(&e.state),
            Some(SeatId(1))
        );
        if blocked {
            pick_when(&mut e, block(w));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            let c = placed_on(&e, "Swarm", Some(a));
            assert_eq!(e.state.cards[c.index()].counters, 1);
            assert_eq!(blood(&e, a), 5 - 1, "W's hand strike, no cost");
        } else {
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            let c = placed_on(&e, "Swarm", Some(z));
            assert_eq!(e.state.cards[c.index()].counters, 1);
            assert_eq!(blood(&e, a), 5 - 1);
        }
    }
}

/// Swarm: "During this minion's unlock phase, if the Swarm has more than 1
/// life, it burns a life; otherwise, it steals 1 blood or life from this
/// minion." At 1 life on Z: Z loses 1 blood, the Swarm 2 life; the next
/// unlock phase it burns back to 1, Z untouched.
#[test]
fn swarm_feeds_then_dwindles() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 3), 4, false);
    let swarm = in_play(&mut e, 0, "Swarm", Some(z));
    e.state.cards[swarm.index()].counters = 1;
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(
        (blood(&e, z), e.state.cards[swarm.index()].counters),
        (3, 2)
    );
    to_minion_phase(&mut e, SeatId(0));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(
        (blood(&e, z), e.state.cards[swarm.index()].counters),
        (3, 1)
    );
}

/// [LSJ 20090824] An animal retainer put on another minion only when its
/// employ is played: Pack Alpha [ani] employing a Swarm or an Ilomba from
/// hand puts it on the playing Laibon.
// Rulings: [LSJ 20090824] [LSJ 20051121].
#[test]
fn pack_alpha_puts_swarm_or_ilomba_on_its_player() {
    for name in ["Swarm", "Ilomba"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let t = laibon(&mut e, 1, 0, &["ani"]);
        laibon(&mut e, 1, 1, &[]);
        hand(&mut e, 1, &["Pack Alpha", name]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(t));
        pick_when(&mut e, play(0, 0, t));
        request(&mut e);
        placed_on(&e, name, Some(t));
    }
}

/// Duma Rafiki: "During strike resolution, if the range is close, this
/// retainer burns a weapon controlled by the opposing minion or inflicts 1
/// damage on the opposing minion." Z with a .44 Magnum: burned, its strike
/// that round still resolved (A takes 2); the damage chosen instead: Z
/// takes 1 more. Z with no weapon: 1 damage through Z's dodge. Animals are
/// neither mortal nor monster.
// Rulings: [ANK 20180612] [ANK 20211127] [RTR 19941109] [TOM 19951216]
// [PIB 20140324] [ANK 20200422] [ANK 20200203-2].
#[test]
fn duma_rafiki_burns_a_weapon_or_bites_at_close_range() {
    for case in ["burn", "bite", "dodge"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 6, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 6, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 3), 4, false);
        let duma = in_play(&mut e, 0, "Duma Rafiki", Some(a));
        let gun = (case != "dodge").then(|| in_play(&mut e, 1, ".44 Magnum", Some(z)));
        if case == "dodge" {
            e.state.minions[z.index()].gained_levels = vec!["cel"];
            hand(&mut e, 1, &["Side Strike"]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if let Some(g) = gun {
            pick_when(&mut e, use_on(g, 0, None, Target::None));
            let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
            assert!(req.options.contains(&Choice::Pass));
            if case == "burn" {
                pick(&mut e, Choice::BurnInPlay { card: g.0 });
            } else {
                pick(&mut e, Choice::Pass);
            }
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, a), 6 - 2, "the Magnum's strike resolved");
            let burned = case == "burn";
            assert_eq!(ash_has(&e, 1, ".44 Magnum"), burned);
            assert_eq!(blood(&e, z), 6 - 1 - u8::from(!burned));
        } else {
            pick_when(&mut e, play(0, 0, z));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, z), 6 - 1, "through the dodge");
        }
        assert!(e.state.cards[duma.index()].present());
    }
}

/// Ananasi Vampirephile: "Once each turn, she may play a card that requires
/// basic Animalism [ani] as a vampire." Blocking, she plays Pack Alpha as a
/// vampire, its Raven Spy met by the same use (LSJ 20080205-2, LSJ
/// 20060417): the Spy on her. Aid from Bats [ani] then never offered that
/// turn; the next player's turn her grant fits again (ANK 20180719-4).
// Rulings: [LSJ 20060417] [LSJ 20080205-2] [ANK 20180719-4].
// Rulings: [LSJ 20080205-2] [LSJ 20060417].
#[test]
fn ananasi_plays_one_animalism_card_as_a_vampire_each_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 5, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    let n = e.add_ally(SeatId(1), find_library("Ananasi Vampirephile"));
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 1, &["Pack Alpha", "Raven Spy", "Aid from Bats"]);
    let bats = find_library("Aid from Bats");
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(n));
    pick_when(&mut e, play(0, 0, n));
    request(&mut e);
    placed_on(&e, "Raven Spy", Some(n));
    assert!(zvtes::cards::plays_card_as_vampire(&e.state, n, bats, 0).is_none());
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == n),
    );
    to_minion_phase(&mut e, SeatId(1));
    assert!(zvtes::cards::plays_card_as_vampire(&e.state, n, bats, 0).is_some());
}

/// Ananasi Vampirephile: "Once each combat, she can burn a life during the
/// press step to get a press and set the range in the next round of that
/// combat, if any." Used in round 1: 1 life burned, the range asked with no
/// Pass, long chosen: round 2 at long, no determine range step there; not
/// offered again that combat.
// Rulings: [RTR 19970630] [ANK 20180720] [PIB 20120214].
#[test]
fn ananasi_burns_a_life_to_press_and_set_the_next_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 6, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    let n = e.add_ally(SeatId(1), find_library("Ananasi Vampirephile"));
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let card = body_of(&e, n);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(n));
    let press = card_ability(&mut e, card, 1);
    pick(&mut e, press);
    let req = request(&mut e);
    assert_eq!(req.options, [Choice::SetX(0), Choice::SetX(1)], "no Pass");
    pick(&mut e, Choice::SetX(1));
    assert_eq!(blood(&e, n), 4 - 1 - 1, "a life burned, A's hand strike");
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 1, .. }
            if *c == card)
    });
    assert_eq!(strike_ranges(&mut e), [Range::Close, Range::Long]);
}

/// Ananasi Vampirephile: "She can gain a life as a +1 stealth action."
#[test]
fn ananasi_gains_a_life_as_a_stealth_action() {
    let mut e = game(2);
    let n = e.add_ally(SeatId(0), find_library("Ananasi Vampirephile"));
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let card = body_of(&e, n);
    pick(&mut e, use_on(card, 2, Some(n), Target::None));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, n), 4 + 1);
}

/// Seat 0's `a` recruits Bima from hand (slot 0): the entry request.
fn recruit_bima(e: &mut Engine, a: MinionId) -> zvtes::decision::DecisionRequest {
    pick(e, play(0, 0, a));
    loop {
        let req = request(e);
        if req.window == WindowLabel::LibrarySearch || req.window == WindowLabel::MinionPhase {
            return req;
        }
        e.choose(fallback(&req));
    }
}

/// The ally named `name` seat `s` controls.
fn ally_named(e: &Engine, name: &str) -> MinionId {
    let card = find_library(name);
    let i = e
        .state
        .cards
        .iter()
        .find(|c| c.card == card && c.body.is_some() && c.present())
        .and_then(|c| c.body)
        .unwrap_or_else(|| panic!("no {name} in play"));
    i
}

/// Bima: "After Bima enters play, put a master Discipline card from your
/// hand on it or burn Bima. Bima can play cards requiring the basic level
/// of that Discipline as a vampire." [PIB 20150418] its recruit card's
/// replacement (Presence, the library's top) put on it: the card itself on
/// Bima, out of the hand; Bima plays Aire of Elation [pre] as an
/// Independent capacity-1 vampire, not its [PRE] mode, and has no Presence
/// [LSJ 19980303]; Presence burned, the grant is gone [LSJ 20071006]. No
/// master in hand: Bima burns.
// Rulings: [PIB 20150418] [LSJ 20071006] [LSJ 19980303] [ANK 20180111]
// [LSJ 20011217] [LSJ 20020503].
// Rulings: [LSJ 20020503] [LSJ 20060126].
#[test]
fn bima_takes_a_master_discipline_from_hand_or_burns() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ministry", 4..=8, 0), 5, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 0, &["Bima"]);
    e.state.seats[0].library = vec![find_library("Presence")].into();
    let req = recruit_bima(&mut e, a);
    let presence = find_library("Presence");
    let take = Choice::TakeFrom {
        zone: zvtes::decision::Zone::Hand,
        card: presence,
    };
    assert_eq!(req.options, [Choice::Pass, take]);
    pick(&mut e, take);
    let bima = ally_named(&e, "Bima");
    let on = placed_on(&e, "Presence", Some(bima));
    assert!(!e.state.seats[0].hand.contains(&presence));
    let aire = find_library("Aire of Elation");
    let g = zvtes::cards::plays_card_as_vampire(&e.state, bima, aire, 0).expect("[pre]");
    assert_eq!((g.capacity, g.sect), (1, Sect::Independent));
    assert!(zvtes::cards::plays_card_as_vampire(&e.state, bima, aire, 1).is_none());
    assert!(
        !e.state.has_discipline(bima, "pre"),
        "no Presence of its own"
    );
    e.burn_card(on);
    assert!(zvtes::cards::plays_card_as_vampire(&e.state, bima, aire, 0).is_none());
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ministry", 4..=8, 0), 5, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 0, &["Bima"]);
    recruit_bima(&mut e, a);
    assert!(ash_has(&e, 0, "Bima"), "burned");
}

/// [LSJ 20071001-1] "When putting Agent of Power on Bima, its card text is
/// applied as normal (Choose a discipline and burn it in your discard
/// phase)": Presence chosen, Bima plays Aire of Elation as a vampire; the
/// Agent burned in the discard phase. [LSJ 20051116-2] Abombwe put on Bima,
/// its requirement ignored.
// Rulings: [LSJ 20071001-1] [LSJ 20100422] [LSJ 20051116-2].
#[test]
fn bima_takes_agent_of_power_or_abombwe() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ministry", 4..=8, 0), 5, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 0, &["Bima", "Agent of Power"]);
    recruit_bima(&mut e, a);
    pick(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Hand,
            card: find_library("Agent of Power"),
        },
    );
    pick_when(&mut e, Choice::ChooseDiscipline("pre"));
    let bima = ally_named(&e, "Bima");
    placed_on(&e, "Agent of Power", Some(bima));
    let aire = find_library("Aire of Elation");
    assert!(zvtes::cards::plays_card_as_vampire(&e.state, bima, aire, 0).is_some());
    to_minion_phase(&mut e, SeatId(1));
    assert!(
        ash_has(&e, 0, "Agent of Power"),
        "burned in the discard phase"
    );
    assert!(zvtes::cards::plays_card_as_vampire(&e.state, bima, aire, 0).is_none());
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ministry", 4..=8, 0), 5, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    hand(&mut e, 0, &["Bima", "Abombwe"]);
    let req = recruit_bima(&mut e, a);
    let abo = Choice::TakeFrom {
        zone: zvtes::decision::Zone::Hand,
        card: find_library("Abombwe"),
    };
    assert!(req.options.contains(&abo));
    pick(&mut e, abo);
    placed_on(&e, "Abombwe", Some(ally_named(&e, "Bima")));
}

/// [LSJ 20060409] "If they take aggravated damage from a card they play as
/// a vampire (eg. Burst of Sunlight), they lose one life but do not go to
/// torpor": Bima with Thaumaturgy blocks and strikes with Burst of Sunlight
/// [tha]: 1 life lost, Bima still in play, ready.
// Rulings: [LSJ 20060409].
// Rulings: [ANK 20170309-2].
#[test]
fn bima_playing_burst_of_sunlight_loses_a_life_not_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 5, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    hand(&mut e, 1, &["Thaumaturgy", "Burst of Sunlight"]);
    let bima = e.add_ally(SeatId(1), find_library("Bima"));
    e.state.minions[bima.index()].blood = 4;
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    pick(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Hand,
            card: find_library("Thaumaturgy"),
        },
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(bima));
    pick_when(&mut e, play(0, 0, bima));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[bima.index()];
    assert!(v.present() && !v.in_torpor);
    assert_eq!(
        v.blood,
        4 - 1 - 1,
        "its own 1 aggravated and A's hand strike"
    );
}

/// Reliquary: Akunanse Remains: "During your unlock phase, choose Abombwe
/// [abo], Animalism [ani] or Fortitude [for]. Until your next unlock phase,
/// the Akunanse with this equipment has an additional level of the chosen
/// Discipline. The Akunanse with this equipment gets +1 bleed and gains 1
/// capacity." [ani] chosen: a level more, +1 bleed, capacity +1; the next
/// unlock phase [for]: [ani] back. On a non-Akunanse: nothing.
// Rulings: [ANK 20180719-3].
#[test]
fn akunanse_remains_grants_the_chosen_discipline_until_next_unlock() {
    for akunanse in [true, false] {
        let mut e = game(2);
        let k = if akunanse {
            e.add_minion(SeatId(0), of_clan("Akunanse", 4..=8, 0), 4, false)
        } else {
            e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false)
        };
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
        let cap = e.state.capacity_of(k).expect("capacity");
        let (ani, fort) = (
            e.state.discipline_level(k, "ani"),
            e.state.discipline_level(k, "for"),
        );
        in_play(&mut e, 0, "Reliquary: Akunanse Remains", Some(k));
        let b = u8::from(akunanse);
        assert_eq!(e.state.capacity_of(k), Some(cap + b), "older");
        assert_eq!(
            e.state.discipline_level(k, "ani"),
            ani,
            "nothing chosen yet"
        );
        pick(&mut e, bleed(k));
        request(&mut e);
        assert_eq!(e.action().expect("action").bleed_amount(&e.state), 1 + b);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        assert_eq!(
            req.options,
            ["abo", "ani", "for"].map(Choice::ChooseDiscipline)
        );
        pick(&mut e, Choice::ChooseDiscipline("ani"));
        assert_eq!(e.state.discipline_level(k, "ani"), (ani + b).min(2));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick_when(&mut e, Choice::ChooseDiscipline("for"));
        assert_eq!(e.state.discipline_level(k, "ani"), ani);
        assert_eq!(e.state.discipline_level(k, "for"), (fort + b).min(2));
    }
}

/// Reliquary: Biague: "During your unlock phase, if the bearer is a ready
/// Laibon, you may look at the top card of any Methuselah's library. If the
/// card does not require a Discipline, show it to all players, and this
/// Laibon may burn a blood to allow you to look at that Methuselah's
/// hand." Seat 1's top a master: shown, the Laibon burns 1, seat 0 looks
/// at seat 1's hand; a [dom] card on top: no show, no offer; on a
/// non-Laibon, not offered.
#[test]
fn biague_shows_a_disciplineless_top_card_and_buys_a_hand_look() {
    for case in ["master", "dom", "non_laibon"] {
        let mut e = game(2);
        let l = if case == "non_laibon" {
            e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false)
        } else {
            laibon(&mut e, 0, 0, &[])
        };
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
        let top = if case == "dom" {
            "Govern the Unaligned"
        } else {
            "Presence"
        };
        let biague = in_play(&mut e, 0, "Reliquary: Biague", Some(l));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        fill_hand(&mut e, 1);
        e.state.seats[1].library = vec![find_library("Ventrue Justicar"), find_library(top)].into();
        e.notes = Some(Vec::new());
        let look = |o: &Choice| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), target: Target::Seat(SeatId(1)), .. }
                if *c == biague)
        };
        if case == "non_laibon" {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, look);
            continue;
        }
        pick_matching(&mut e, look);
        let req = request(&mut e);
        let buy = Choice::ResolvePrinted { minion: l };
        if case == "dom" {
            assert!(!req.options.contains(&buy));
            assert!(noted_reveals(&mut e).is_empty());
            continue;
        }
        assert!(!noted_reveals(&mut e).is_empty(), "shown");
        e.state.peeks.clear();
        pick(&mut e, buy);
        assert_eq!(blood(&e, l), 5 - 1);
        assert!(e.state.peeks.contains(&(SeatId(0), SeatId(1))));
    }
}

/// Ohoyo Hopoksia (Bastet): "When a minion you control would enter combat
/// with an acting minion, you may lock Ohoyo to cancel that combat and have
/// Ohoyo enter combat with the acting minion instead." Z blocks A's bleed:
/// Ohoyo (recruited this turn) locks and fights A (3 strength), Z stays
/// locked and unhurt, the action blocked; declined, Z fights.
// Rulings: [ANK 20200714] [ANK 20180517].
#[test]
fn ohoyo_takes_the_combat_of_a_blocker() {
    for stand_in in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 6, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        let o = e.add_ally(SeatId(1), find_library("Ohoyo Hopoksia (Bastet)"));
        e.state.minions[o.index()].recruited = true;
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        let card = body_of(&e, o);
        let req = request(&mut e);
        assert_eq!(req.seat, SeatId(1));
        assert_eq!(
            req.options,
            [Choice::Pass, Choice::ResolveRider { card: card.0 }]
        );
        e.choose(usize::from(stand_in));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[z.index()].locked, "the blocker locked");
        assert_eq!(e.state.seats[1].pool, 10, "blocked");
        if stand_in {
            assert!(e.state.minions[o.index()].locked);
            assert_eq!(blood(&e, z), 4, "not in combat");
            assert_eq!(blood(&e, a), 6 - 3, "Ohoyo's 3 strength");
        } else {
            assert!(!e.state.minions[o.index()].locked);
            assert_eq!(blood(&e, z), 4 - 1);
        }
    }
}

/// Ohoyo Hopoksia: "When a minion you control would enter combat with an
/// acting minion" — a rush too: A's Bum's Rush on Z. Ohoyo locks and
/// fights A in Z's place (3 strength; A's hand strike of 1 on her), Z out
/// of the combat; declined, Z fights.
#[test]
fn ohoyo_takes_the_combat_of_a_rushed_minion() {
    for stand_in in [true, false] {
        let mut e = game(2);
        hand(&mut e, 0, &["Bum's Rush"]);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 6, false);
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        let o = e.add_ally(SeatId(1), find_library("Ohoyo Hopoksia (Bastet)"));
        pick(&mut e, play_at(0, 0, a, z));
        let card = body_of(&e, o);
        let req = offering(&mut e, Choice::ResolveRider { card: card.0 });
        assert_eq!(req.seat, SeatId(1));
        assert!(e.combat().is_some(), "asked in the combat begun");
        assert_eq!(
            req.options,
            [Choice::Pass, Choice::ResolveRider { card: card.0 }]
        );
        e.choose(usize::from(stand_in));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if stand_in {
            assert!(e.state.minions[o.index()].locked, "Ohoyo locked");
            assert_eq!(blood(&e, z), 4, "Z not in combat");
            assert_eq!(blood(&e, o), 3 - 1, "Ohoyo struck");
            assert_eq!(blood(&e, a), 6 - 3, "Ohoyo's 3 strength");
        } else {
            assert!(!e.state.minions[o.index()].locked);
            assert_eq!(blood(&e, z), 4 - 1, "Z struck");
            assert_eq!(blood(&e, o), 3);
        }
    }
}

/// The side staying reads its opponent-dependent bonuses live (rules-qa
/// 124.88): independent A bears Redline ("one optional press when in
/// combat with a Camarilla vampire") and rushes Camarilla Z; Ohoyo taking Z's
/// place, A's press is gone; declined, A has it.
#[test]
fn ohoyo_swapped_in_takes_away_a_press_against_the_replaced_sect() {
    for stand_in in [true, false] {
        let mut e = game(2);
        hand(&mut e, 0, &["Bum's Rush"]);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| c.sect == Some(Sect::Independent) && c.capacity >= 4, 0),
            6,
            false,
        );
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
        let z = e.add_minion(
            SeatId(1),
            textless(|c| c.sect == Some(Sect::Camarilla) && c.capacity >= 4, 0),
            4,
            false,
        );
        let o = e.add_ally(SeatId(1), find_library("Ohoyo Hopoksia (Bastet)"));
        in_play(&mut e, 0, "Redline", Some(a));
        pick(&mut e, play_at(0, 0, a, z));
        let card = body_of(&e, o);
        let req = offering(&mut e, Choice::ResolveRider { card: card.0 });
        let presses = |e: &Engine| e.combat().expect("combat").sides[0].combat_presses;
        assert_eq!(presses(&e), 1, "against Z");
        e.choose(index_of(
            &req,
            if stand_in {
                Choice::ResolveRider { card: card.0 }
            } else {
                Choice::Pass
            },
        ));
        request(&mut e);
        assert_eq!(presses(&e), u8::from(!stand_in));
    }
}

/// Ohoyo Hopoksia: "If she has less than 3 life during your unlock phase,
/// she gains 1 life from the blood bank."
#[test]
fn ohoyo_heals_below_three() {
    let mut e = game(2);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let o = e.add_ally(SeatId(1), find_library("Ohoyo Hopoksia (Bastet)"));
    e.state.minions[o.index()].blood = 1;
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(blood(&e, o), 2);
}

/// [LSJ 20060409] "If a card they play as a vampire sends them to torpor
/// (eg. Reform Body), they burn": Bima with Fortitude bleeds with Day
/// Operation [for] ("The acting vampire goes to torpor after resolving the
/// action"): the bleed resolves, then Bima burns.
// Rulings: [LSJ 20060409].
#[test]
fn bima_sent_to_torpor_by_a_card_it_plays_burns() {
    let mut e = game(2);
    hand(&mut e, 0, &["Fortitude", "Day Operation"]);
    let bima = e.add_ally(SeatId(0), find_library("Bima"));
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    pick(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Hand,
            card: find_library("Fortitude"),
        },
    );
    pick(&mut e, bleed(bima));
    pick_when(&mut e, play(0, 0, bima));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "the bleed resolved");
    assert!(!e.state.minions[bima.index()].in_play, "burned");
    assert!(ash_has(&e, 0, "Bima"));
}

/// [LSJ 20020729] "Conditional damage effects (e.g. Poker's aggravated
/// damage against Kiasyd) are not considered when checking Concealed
/// Weapon's 'with a regular strike' qualifier": Concealed Weapon offers
/// Poker.
// Rulings: [LSJ 20020729].
#[test]
fn concealed_weapon_conceals_a_poker() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Concealed Weapon", "Poker"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert_eq!(
        req.window,
        WindowLabel::CombatStrike,
        "the only weapon, named"
    );
    placed_on(&e, "Poker", Some(a));
}

/// Ghouls and animals are no mortals: Asanbonsam Ghoul and Bima, ghoul
/// allies; Ritual Scalpel's "a mortal (ally or retainer)" cuts Malajit
/// Chandramouli, never Duma Rafiki, Elephant Guardian, Swarm nor Ghoul
/// Messenger.
// Rulings: [ANK 20200203-2] [LSJ 20060515].
#[test]
fn round_32_ghouls_and_animals_are_no_mortals() {
    let mut e = game(2);
    for name in ["Asanbonsam Ghoul", "Bima"] {
        let m = e.add_ally(SeatId(0), find_library(name));
        assert!(!e.state.is_mortal(m), "{name}");
    }
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Le Dinh Tho"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let scalpel = in_play(&mut e, 0, "Ritual Scalpel", Some(n));
    let cut = |r: CardId| Choice::UseAbility {
        source: AbilitySource::Card(scalpel),
        ability: 1,
        actor: Some(n),
        target: Target::Card(r),
    };
    let malajit = in_play(&mut e, 1, "Malajit Chandramouli", Some(x));
    let others: Vec<(&str, CardId)> = [
        "Duma Rafiki",
        "Elephant Guardian",
        "Swarm",
        "Ghoul Messenger",
    ]
    .into_iter()
    .map(|name| (name, in_play(&mut e, 1, name, Some(x))))
    .collect();
    let req = request(&mut e);
    assert!(req.options.contains(&cut(malajit)), "{req:?}");
    for (name, r) in others {
        assert!(!req.options.contains(&cut(r)), "{name}");
    }
}

/// Children of Stone [vis]: "Search your library (shuffle afterward) and/or
/// hand for a gargoyle creature retainer and employ it (requirements and
/// cost apply as normal). [VIS] As [vis] above, but reduce its cost by 1
/// blood or pool." [TOM 19960130] "Is an action of the appropriate type for
/// the chosen card": an employ action. Stone Dog found in the library, or
/// in the hand: on the acting vampire, its 3 pool paid, 2 at [VIS].
#[test]
fn children_of_stone_employs_a_stone_dog() {
    use zvtes::decision::Zone;
    for (mode, zone, paid) in [
        (1u8, Zone::Library, 3),
        (2, Zone::Library, 2),
        (1, Zone::Hand, 3),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan("Gargoyle", 4..=8, 0), 5, false);
        e.state.minions[a.index()].gained_levels = vec!["vis", "vis"];
        e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
        if zone == Zone::Hand {
            hand(&mut e, 0, &["Children of Stone", "Stone Dog"]);
        } else {
            hand(&mut e, 0, &["Children of Stone"]);
        }
        fill_hand(&mut e, 0);
        e.state.seats[0].library = if zone == Zone::Hand {
            vec![find_library("Form of Mist")].into()
        } else {
            vec![find_library("Stone Dog"), find_library("Form of Mist")].into()
        };
        pick(&mut e, play(0, mode, a));
        request(&mut e);
        assert!(e.action().expect("action").is_employ_action(&e.state));
        pick_when(
            &mut e,
            Choice::TakeFrom {
                zone,
                card: find_library("Stone Dog"),
            },
        );
        placed_on(&e, "Stone Dog", Some(a));
        assert_eq!(e.state.seats[0].pool, 10 - paid, "mode {mode}, {zone:?}");
    }
}

// ---- Masters ----------------------------------------------------------------

/// A master card from hand slot `card` played on `target`.
fn master_at(card: u8, target: Target) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: None,
        target,
    }
}

/// Ancestor Spirit: "Put this card on a Laibon with capacity above 4. This
/// Laibon has +1 bleed and gets an additional vote in any referendum he or
/// she calls." On another Methuselah's capacity-5+ Laibon (not a capacity-4
/// one): +1 bleed, +1 vote calling, none in another's referendum.
#[test]
fn ancestor_spirit_bleeds_and_votes_in_its_own_referendum() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let big = e.add_minion(SeatId(1), of_sect(Sect::Laibon, 5..=8, 0), 5, false);
    let small = e.add_minion(SeatId(1), of_sect(Sect::Laibon, 1..=4, 0), 4, false);
    hand(&mut e, 0, &["Ancestor Spirit"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req.options.contains(&master_at(0, Target::Minion(small))));
    pick(&mut e, master_at(0, Target::Minion(big)));
    request(&mut e);
    placed_on(&e, "Ancestor Spirit", Some(big));
    assert_eq!(e.state.bearer_always(big, zvtes::cards::Stat::Bleed), 1);
    assert_eq!(e.state.calling_votes(big), 1);
    assert_eq!(e.state.calling_votes(small), 0);
}

/// Bastille Opera House: "You can lock this card during the polling step of
/// a political action to get +1 vote for each ready Daughter of Cacophony
/// you control. If you control no ready Daughters of Cacophony, you can lock
/// and burn this location … to get +1 vote." Two ready Daughters: 2 votes,
/// the burn not offered; none: the burn for 1, the card in the ash heap.
#[test]
fn bastille_opera_house_votes_per_ready_daughter_or_burns_for_one() {
    for daughters in [2usize, 0] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
        for i in 0..daughters {
            clan_minion(&mut e, SeatId(0), "Daughter of Cacophony", 1..=11, i, 4);
        }
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        let b = in_play(&mut e, 0, "Bastille Opera House", None);
        hand(&mut e, 0, &["Political Stranglehold"]);
        pick(&mut e, play(0, 0, a));
        to_referendum(&mut e);
        let per = ability(b.0, 0);
        let burn = ability(b.0, 2);
        let req = offering(&mut e, per);
        assert_eq!(req.options.contains(&burn), daughters == 0, "{daughters}");
        pick(&mut e, if daughters == 0 { burn } else { per });
        request(&mut e);
        assert_eq!(tally(&e), (daughters.max(1) as u32, 0));
        assert_eq!(ash_has(&e, 0, "Bastille Opera House"), daughters == 0);
        assert!(daughters == 0 || e.state.cards[b.index()].locked);
    }
}

/// Conductor: "While this Daughter of Cacophony is unlocked or acting, each
/// unlocked or acting Daughter of Cacophony gets +1 vote." On seat 1's
/// Daughter: seat 2's unlocked Daughter +1, a locked one none; the host
/// locked, none.
#[test]
fn conductor_gives_unlocked_or_acting_daughters_a_vote() {
    let mut e = game(3);
    let doc = |e: &mut Engine, seat, n, blood| {
        clan_minion(e, seat, "Daughter of Cacophony", 1..=11, n, blood)
    };
    doc(&mut e, SeatId(0), 3, 4);
    let host = doc(&mut e, SeatId(1), 0, 4);
    let other = doc(&mut e, SeatId(2), 1, 4);
    let locked = doc(&mut e, SeatId(2), 2, 4);
    e.state.minions[locked.index()].locked = true;
    hand(&mut e, 0, &["Conductor"]);
    e.at_master_phase();
    pick(&mut e, master_at(0, Target::Minion(host)));
    request(&mut e);
    placed_on(&e, "Conductor", Some(host));
    let base = |e: &Engine, m| e.state.votes_held(m) - e.state.clan_votes(m) as u8;
    assert_eq!(e.state.clan_votes(host), 1);
    assert_eq!(e.state.clan_votes(other), 1);
    assert_eq!(e.state.clan_votes(locked), 0);
    assert_eq!(e.state.votes_held(other), base(&e, other) + 1);
    // The locked one acting: counted.
    e.state.acting = Some(locked);
    assert_eq!(e.state.clan_votes(locked), 1);
    e.state.acting = None;
    e.state.minions[host.index()].locked = true;
    assert_eq!(e.state.clan_votes(other), 0);
}

/// Conductor: "While this Daughter of Cacophony is unlocked or acting" — her
/// clan read live (rules-qa 9.1, rules-digest D31): its host turned
/// Malkavian, an unlocked Daughter gets no vote from it.
#[test]
fn conductor_gives_no_vote_once_its_host_is_no_daughter() {
    let mut e = game(2);
    let host = clan_minion(&mut e, SeatId(0), "Daughter of Cacophony", 1..=11, 0, 4);
    let other = clan_minion(&mut e, SeatId(1), "Daughter of Cacophony", 1..=11, 1, 4);
    in_play(&mut e, 0, "Conductor", Some(host));
    assert_eq!(e.state.clan_votes(other), 1);
    e.state.minions[host.index()].clan_change = Some("Malkavian");
    assert_eq!(e.state.clan_votes(other), 0);
}

/// Glutton: "This card costs X pool, where X is the number of Gluttons in
/// your ash heap. … You may lock this card to unlock a ready Ishtarri you
/// control. That Ishtarri burns 1 blood." Two in the ash heap: 2 pool; a
/// locked Ishtarri with no blood unlocks, at 0 blood, in another's turn.
/// Ledger: `[LSJ 19970707]`.
#[test]
fn glutton_costs_its_ash_copies_and_unlocks_a_bloodless_ishtarri() {
    let mut e = game(2);
    let i = e.add_minion(SeatId(0), of_clan("Ishtarri", 1..=11, 0), 0, false);
    e.add_minion(SeatId(0), of_clan("Ishtarri", 1..=11, 1), 3, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.state.minions[i.index()].locked = true;
    let glutton = find_library("Glutton");
    e.state.seats[0].ash = vec![glutton, glutton].into();
    hand(&mut e, 0, &["Glutton"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 8, "X = 2");
    let g = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.card == glutton && c.in_play && !c.effect_only)
            .expect("Glutton in play") as u32,
    );
    // In seat 1's turn (a starving vampire hunts in its own).
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[i.index()].locked = true;
    e.state.minions[i.index()].blood = 0;
    pick_when(&mut e, use_on(g, 0, None, Target::Minion(i)));
    request(&mut e);
    assert!(!e.state.minions[i.index()].locked);
    assert_eq!(blood(&e, i), 0);
    assert!(e.state.cards[g.index()].locked);
}

/// Palace Hunting Ground: "During your unlock phase, you may move 1 blood
/// from the blood bank to a ready Guruhi you control. Not usable to move a
/// blood to a Guruhi with no blood." A 1-blood Guruhi offered and fed, a
/// 0-blood one and a non-Guruhi not.
#[test]
fn palace_hunting_ground_feeds_a_guruhi_with_blood() {
    let mut e = game(2);
    let fed = e.add_minion(SeatId(0), of_clan("Guruhi", 3..=11, 0), 1, false);
    let empty = e.add_minion(SeatId(0), of_clan("Guruhi", 3..=11, 1), 0, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let p = in_play(&mut e, 0, "Palace Hunting Ground", None);
    // Emptied in seat 1's turn (a starving vampire hunts in its own).
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[empty.index()].blood = 0;
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(!req
        .options
        .contains(&use_on(p, 0, None, Target::Minion(empty))));
    assert!(!req
        .options
        .contains(&use_on(p, 0, None, Target::Minion(other))));
    pick(&mut e, use_on(p, 0, None, Target::Minion(fed)));
    request(&mut e);
    assert_eq!(blood(&e, fed), 2);
    assert!(e.state.minions[fed.index()].hunting_ground_fed);
}

/// Port Hunting Ground: an Ishtarri master — not playable with no ready
/// Ishtarri; in play, it feeds any vampire its controller controls.
#[test]
fn port_hunting_ground_needs_an_ishtarri_and_feeds_any_vampire() {
    for ishtarri in [false, true] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
        if ishtarri {
            e.add_minion(SeatId(0), of_clan("Ishtarri", 1..=11, 0), 1, false);
        }
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Port Hunting Ground"]);
        e.at_master_phase();
        let req = request(&mut e);
        assert_eq!(req.options.contains(&master_play(0)), ishtarri);
        if !ishtarri {
            continue;
        }
        pick(&mut e, master_play(0));
        let p = {
            request(&mut e);
            placed_on(&e, "Port Hunting Ground", None)
        };
        pick_when(&mut e, use_on(p, 0, None, Target::Minion(v)));
        request(&mut e);
        assert_eq!(blood(&e, v), 2);
    }
}

/// The Slaughterhouse: "Lock to burn two cards from the top of your prey's
/// library." Seat 1's top two to its ash heap, the card locked; a second
/// copy usable at once.
#[test]
fn slaughterhouse_burns_two_off_the_preys_library() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    for s in 0..3 {
        fill_hand(&mut e, s);
    }
    let deck = [
        "Bum's Rush",
        "Deflection",
        "Govern the Unaligned",
        "Majesty",
        "Forced Awakening",
    ];
    e.state.seats[1].library = deck
        .iter()
        .map(|n| find_library(n))
        .collect::<Vec<_>>()
        .into();
    let top = |e: &Engine| e.state.seats[1].library.len();
    let s1 = in_play(&mut e, 0, "Slaughterhouse, The", None);
    let s2 = in_play(&mut e, 0, "Slaughterhouse, The", None);
    pick(&mut e, ability(s1.0, 0));
    request(&mut e);
    assert_eq!(top(&e), 3);
    assert_eq!(e.state.seats[1].ash.len(), 2);
    assert!(e.state.cards[s1.index()].locked);
    pick(&mut e, ability(s2.0, 0));
    request(&mut e);
    assert_eq!(top(&e), 1);
    assert_eq!(e.state.seats[2].ash.len(), 0, "not the predator's prey");
}

/// Vox Domini: "Only usable during the referendum of a political action. …
/// The referendum fails. Each Methuselah may play only one Vox Domini each
/// game." Seat 1 fails seat 0's Political Stranglehold, its vote in favor
/// cast: no pool gained (3 without it); seat 0's own copy never offered on
/// its turn; seat 1's second copy not offered in the next referendum.
#[test]
fn vox_domini_fails_a_referendum_once_a_game() {
    for vox in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| c.capacity >= 8 && c.clan == "Ventrue", 0),
            4,
            false,
        );
        e.state.minions[a.index()].granted_title = Some(Title::Primogen);
        e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        hand(
            &mut e,
            0,
            &["Political Stranglehold", "Vox Domini", "Ancient Influence"],
        );
        hand(&mut e, 1, &["Vox Domini", "Vox Domini"]);
        pick(&mut e, play(0, 0, a));
        to_referendum(&mut e);
        pick_when(&mut e, votes(a, true));
        loop {
            let req = request(&mut e);
            if e.referendum().is_none() {
                break;
            }
            assert!(
                req.seat != SeatId(0) || !req.options.contains(&master_play(0)),
                "own turn"
            );
            let pick = if vox && req.seat == SeatId(1) && req.options.contains(&master_play(0)) {
                index_of(&req, master_play(0))
            } else {
                fallback(&req)
            };
            e.choose(pick);
        }
        assert_eq!(
            e.state.seats[0].pool,
            if vox { 10 } else { 13 },
            "vox {vox}"
        );
        if !vox {
            continue;
        }
        assert!(e.state.seats[1].ash.contains(&find_library("Vox Domini")));
        // Seat 0's next turn: Ancient Influence, the other copy not offered.
        pick(&mut e, Choice::Pass);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let ai = play(1, 0, a);
        pick(&mut e, ai);
        to_referendum(&mut e);
        loop {
            let req = request(&mut e);
            if e.referendum().is_none() {
                break;
            }
            assert!(
                req.seat != SeatId(1) || !req.options.contains(&master_play(0)),
                "a second Vox Domini"
            );
            e.choose(fallback(&req));
        }
    }
}

/// Secret Passage: "If this minion is the target of a directed action while
/// he or she is ready, you may burn this card to make the action fail." Seat
/// 0's Bum's Rush at seat 1's minion: the card burned, the action failed
/// (resolved, not blocked), no combat. Ledger: `[ANK 20211015] [RTR
/// 20180719]`, `[ANK 20211015]`.
#[test]
fn secret_passage_fails_a_directed_action_as_resolved() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let sp = in_play(&mut e, 1, "Secret Passage", Some(z));
    hand(&mut e, 0, &["Bum's Rush"]);
    e.notes = Some(Vec::new());
    pick(&mut e, play_at(0, 0, a, z));
    let u = card_ability(&mut e, sp, 0);
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(1));
    pick(&mut e, u);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Secret Passage"));
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
    assert!(noted_combats(&mut e).is_empty(), "no combat");
}

/// Secret Passage: not while a block is attempted. Ledger: `[ANK 20260316]
/// [LSJ 20041022]`.
#[test]
fn secret_passage_is_not_burned_during_a_block_attempt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let sp = in_play(&mut e, 1, "Secret Passage", Some(z));
    hand(&mut e, 0, &["Bum's Rush"]);
    pick(&mut e, play_at(0, 0, a, z));
    pick_when(&mut e, block(w));
    let req = request(&mut e);
    assert!(uses(&req, sp.0).is_empty(), "{:?}", req.options);
}

/// Feral Spirit: "Put this card on a Gangrel or Gangrel antitribu you
/// control. This vampire is now Ahrimane and Sabbat. This vampire gains one
/// level of Spiritus [spi]. This vampire is now sterile." On the Gangrel,
/// not on a Brujah; the card itself in play.
#[test]
fn feral_spirit_makes_a_gangrel_a_sterile_sabbat_ahrimane_while_on() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), of_clan("Gangrel", 1..=11, 0), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Feral Spirit"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req.options.contains(&master_at(0, Target::Minion(other))));
    let spi = e.state.discipline_level(g, "spi");
    assert!(!e.state.is_sterile(g));
    pick(&mut e, master_at(0, Target::Minion(g)));
    request(&mut e);
    placed_on(&e, "Feral Spirit", Some(g));
    assert_eq!(e.state.minions[g.index()].clan(), Some("Ahrimane"));
    assert_eq!(e.state.sect_of(g), Some(Sect::Sabbat));
    assert_eq!(e.state.discipline_level(g, "spi"), spi + 1);
    assert!(e.state.is_sterile(g));
}

/// Feral Spirit under a Derange (red, rules-qa Batch 124): the latest
/// override applies — Malkavian while the Derange is on; moved away by its
/// vampire's action, Ahrimane and Sabbat again (`override_without`).
#[test]
fn feral_spirit_and_derange_latest_override_wins() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), of_clan("Gangrel", 1..=11, 0), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Feral Spirit"]);
    e.at_master_phase();
    pick(&mut e, master_at(0, Target::Minion(g)));
    request(&mut e);
    let d = in_play(&mut e, 1, "Derange", Some(g));
    e.state.minions[g.index()].deranged = Some(("Malkavian", Sect::Camarilla, d));
    assert_eq!(e.state.minions[g.index()].clan(), Some("Malkavian"));
    let mv = use_on(d, 1, Some(g), Target::Minion(other));
    pick_when(&mut e, mv);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[d.index()].host, Some(other));
    assert_eq!(e.state.minions[other.index()].clan(), Some("Malkavian"));
    assert_eq!(e.state.minions[g.index()].clan(), Some("Ahrimane"));
    assert_eq!(e.state.sect_of(g), Some(Sect::Sabbat));
}

/// Regenerative Blood: "The Samedi with this card can heal 2 non-aggravated
/// damage for each blood counter he or she burns." Shattering's 2
/// unpreventable damage out of combat: 1 blood (2 without the card); a Bum's
/// Rush's 3-damage hand strike in combat: 2 blood (3 without).
#[test]
fn regenerative_blood_mends_two_damage_per_blood() {
    for rb in [true, false] {
        // Out of combat.
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 1..=11, 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["pro"];
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let s = e.add_minion(SeatId(1), of_clan("Samedi", 4..=11, 0), 5, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if rb {
            in_play(&mut e, 1, "Regenerative Blood", Some(s));
        }
        hand(&mut e, 0, &["Shattering"]);
        pick(&mut e, play_at(0, 2, a, s));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            blood(&e, s),
            if rb { 4 } else { 3 },
            "out of combat, rb {rb}"
        );
        // In combat.
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::Strength(2),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let s = e.add_minion(SeatId(1), of_clan("Samedi", 4..=11, 0), 5, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if rb {
            in_play(&mut e, 1, "Regenerative Blood", Some(s));
        }
        hand(&mut e, 0, &["Bum's Rush"]);
        pick(&mut e, play_at(0, 0, a, s));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, s), if rb { 3 } else { 2 }, "in combat, rb {rb}");
    }
}

/// [LSJ 20100203-2] "The fact it is Unique has no bearing on the burn
/// option." Lazarene Inquisitor, Paris Opera House and Regenerative Blood,
/// a copy of each in play: not discarded while their clan's ready vampire
/// is controlled; discarded without it.
#[test]
fn round32_burn_option_masters_ignore_their_copy_in_play() {
    for (name, clan) in [
        ("Lazarene Inquisitor", "Harbinger of Skulls"),
        ("Paris Opera House", "Daughter of Cacophony"),
        ("Regenerative Blood", "Samedi"),
    ] {
        for meets in [true, false] {
            let mut e = game(2);
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
            if meets {
                clan_minion(&mut e, SeatId(1), clan, 1..=11, 0, 4);
            }
            in_play(&mut e, 1, name, None);
            hand(&mut e, 1, &[name]);
            e.state.seats[1].library.push(find_library("Soak"));
            pick(&mut e, Choice::Pass);
            let found = loop {
                let req = request(&mut e);
                if req.window == WindowLabel::UnlockAbilities && req.seat == SeatId(1) {
                    break Some(req);
                }
                if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                    break None;
                }
                e.choose(fallback(&req));
            };
            let offered = found.is_some_and(|r| r.options.contains(&Choice::Discard(0)));
            assert_eq!(offered, !meets, "{name}, meets: {meets}");
        }
    }
}

/// Lisandro Giovanni ("You and Lisandro can play cards requiring a Samedi
/// … as if he were of that required clan") meets no burn option: [LSJ
/// 20050225] "You can still use the burn options of cards with a
/// requirement he could meet when you play them, but does not meet while in
/// play." Blessings of the Loa and Dirty Contract discarded with him ready,
/// not with a Samedi.
#[test]
fn lisandro_does_not_bar_a_samedi_masters_burn_option() {
    for name in ["Blessings of the Loa", "Dirty Contract"] {
        for samedi in [false, true] {
            let mut e = game(2);
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            e.add_minion(SeatId(1), find_card("Lisandro Giovanni"), 4, false);
            if samedi {
                e.add_minion(SeatId(1), of_clan("Samedi", 1..=11, 0), 4, false);
            }
            hand(&mut e, 1, &[name]);
            e.state.seats[1].library.push(find_library("Soak"));
            pick(&mut e, Choice::Pass);
            let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
            let offered = req.options.contains(&Choice::Discard(0));
            assert_eq!(offered, !samedi, "{name}, samedi {samedi}");
        }
    }
}

/// Lazarene Inquisitor: "The Harbinger of Skulls with this card can burn up
/// to 2 blood from a ready vampire as a +1 stealth Ⓓ action. Any Sabbat
/// vampire can burn this card as a Ⓓ action." 2 of seat 1's vampire's 3
/// blood burned, the amount chosen as it resolves (1 or 2); the same
/// Harbinger then burns the card the same turn. Ledger: `[LSJ 20100909]`.
#[test]
fn lazarene_inquisitor_burns_up_to_two_then_itself_the_same_turn() {
    let mut e = game(2);
    let h = e.add_minion(
        SeatId(0),
        of_clan("Harbinger of Skulls", 1..=11, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 3, false);
    assert_eq!(e.state.sect_of(h), Some(Sect::Sabbat));
    let li = in_play(&mut e, 0, "Lazarene Inquisitor", Some(h));
    pick(&mut e, use_on(li, 0, Some(h), Target::Minion(z)));
    let req = offering(&mut e, Choice::SetX(2));
    assert_eq!(req.options, vec![Choice::SetX(1), Choice::SetX(2)]);
    pick(&mut e, Choice::SetX(2));
    assert_eq!(blood(&e, z), 1);
    // Unlocked again: its other action, not the same one.
    e.state.minions[h.index()].locked = false;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req
        .options
        .contains(&use_on(li, 0, Some(h), Target::Minion(z))));
    pick(&mut e, use_on(li, 1, Some(h), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Lazarene Inquisitor"));
}

/// Lazarene Inquisitor's action requires a Harbinger of Skulls ([LSJ
/// 20080604]): its host deranged into a Malkavian, not offered.
#[test]
fn lazarene_inquisitor_action_needs_a_harbinger_host() {
    for deranged in [false, true] {
        let mut e = game(2);
        let h = e.add_minion(
            SeatId(0),
            of_clan("Harbinger of Skulls", 1..=11, 0),
            4,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
        let li = in_play(&mut e, 0, "Lazarene Inquisitor", Some(h));
        if deranged {
            let d = in_play(&mut e, 1, "Derange", Some(h));
            e.state.minions[h.index()].deranged = Some(("Malkavian", Sect::Camarilla, d));
        }
        let req = request(&mut e);
        let offered = req
            .options
            .contains(&use_on(li, 0, Some(h), Target::Minion(z)));
        assert_eq!(offered, !deranged);
    }
}

/// Paris Opera House: "You can lock this card to give a Daughter of
/// Cacophony you control +1 stealth. You can lock this card and a Daughter
/// of Cacophony you control to give a minion +1 stealth." Seat 0's acting
/// Daughter, blocked by seat 1, gets it (the first); seat 2 locks its own
/// Daughter to give seat 0's acting Ventrue +1 stealth (the second): both
/// bleeds unblocked. Not offered while stealth is not needed.
#[test]
fn paris_opera_house_gives_stealth_to_a_daughter_or_any_minion() {
    let doc = |e: &mut Engine, seat, n, blood| {
        clan_minion(e, seat, "Daughter of Cacophony", 1..=11, n, blood)
    };
    // The first: seat 0's own acting Daughter.
    let mut e = game(2);
    let d = doc(&mut e, SeatId(0), 0, 4);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let p = in_play(&mut e, 0, "Paris Opera House", None);
    pick(&mut e, bleed(d));
    let req = request(&mut e);
    assert!(uses(&req, p.0).is_empty(), "not needed");
    pick_when(&mut e, block(b));
    pick_when(&mut e, use_on(p, 0, None, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9, "unblocked");
    assert!(e.state.cards[p.index()].locked);
    // The second: another Methuselah's acting minion.
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let d = doc(&mut e, SeatId(2), 0, 4);
    let p = in_play(&mut e, 2, "Paris Opera House", None);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let req = request(&mut e);
    assert!(
        !req.options.contains(&use_on(p, 0, None, Target::None)),
        "not a Daughter"
    );
    pick_when(&mut e, use_on(p, 1, None, Target::Minion(d)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9, "unblocked");
    assert!(e.state.minions[d.index()].locked);
}

/// New in Town: "Put this card on any vampire with a capacity below 8 who
/// was moved to the ready region in any Methuselah's influence phase since
/// your last turn. This vampire gets -1 stealth. This vampire can burn this
/// card as a Ⓓ action." Seat 1's vampire influenced in its turn: offered,
/// -1 stealth, its burn action offered in seat 1's turn; seat 0's own,
/// influenced in its last turn, not ("Your last turn does not qualify").
/// Ledger: `[LSJ 20020429]`.
#[test]
fn new_in_town_goes_on_a_vampire_influenced_since_your_last_turn() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let slot = |e: &mut Engine, s: usize, card: u16| {
        e.state.seats[s]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card,
                copy: Default::default(),
                counters: CRYPT[card as usize].capacity,
                banished: None,
            });
    };
    let mine = textless(|c| c.capacity <= 7 && c.clan == "Toreador", 0);
    let theirs = textless(|c| c.capacity <= 7 && c.clan == "Tremere", 0);
    slot(&mut e, 0, mine);
    slot(&mut e, 1, theirs);
    e.state.seats[0].this_turn = 4;
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, Choice::InfluenceOut { slot: 0 });
    let v0 = MinionId(e.state.minions.len() as u32 - 1);
    pick_when(&mut e, Choice::InfluenceOut { slot: 0 });
    let v1 = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(e.state.minions[v1.index()].owner, SeatId(1));
    hand(&mut e, 0, &["New in Town"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(
        !req.options.contains(&master_at(0, Target::Minion(v0))),
        "its last turn"
    );
    pick(&mut e, master_at(0, Target::Minion(v1)));
    request(&mut e);
    let nit = placed_on(&e, "New in Town", Some(v1));
    assert_eq!(e.state.bearer_always(v1, zvtes::cards::Stat::Stealth), -1);
    pick(&mut e, Choice::Pass);
    let burn = use_on(nit, 0, Some(v1), Target::None);
    let req = offering(&mut e, burn);
    assert_eq!(req.seat, SeatId(1));
}

/// Dirty Contract: "Choose any Samedi and put this card on any minion. The
/// chosen Samedi can enter combat with this minion as a +1 stealth Ⓓ
/// action." Seat 0 chooses seat 2's Samedi and puts it on seat 1's minion:
/// that Samedi is "chosen for the contract" — Open Dossier offered to it as
/// seat 1's minion bleeds (LSJ 20070708, LSJ 20071109) — and rushes it in
/// seat 2's turn. Not on a Nosferatu. Ledger: `[LSJ 20070708] [LSJ
/// 20071109]`.
#[test]
fn dirty_contract_chosen_samedi_rushes_and_counts_for_open_dossier() {
    let mut e = game(3);
    e.add_minion(SeatId(0), of_clan("Samedi", 1..=11, 0), 4, false);
    let m = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let nos = e.add_minion(SeatId(1), of_clan("Nosferatu", 1..=11, 0), 4, false);
    let s = e.add_minion(SeatId(2), of_clan("Samedi", 1..=11, 1), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Dirty Contract"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req.options.contains(&master_at(0, Target::Minion(nos))));
    pick(&mut e, master_at(0, Target::Minion(m)));
    pick(&mut e, Choice::ChooseMinion(s));
    request(&mut e);
    let dc = placed_on(&e, "Dirty Contract", Some(m));
    // Seat 1's minion bleeds seat 2: the chosen Samedi may Open Dossier.
    hand(&mut e, 2, &["Open Dossier"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(m));
    let dossier = play(0, 0, s);
    let req = offering(&mut e, dossier);
    assert_eq!(req.seat, SeatId(2));
    pick(&mut e, Choice::Pass);
    // Seat 2's turn: the rush.
    let rush = use_on(dc, 0, Some(s), Target::Minion(m));
    let req = offering(&mut e, rush);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(2), WindowLabel::MinionPhase)
    );
}

/// Dirty Contract's "any Samedi": Lisandro Giovanni ("You and Lisandro can
/// play cards requiring a Samedi … as if he were of that required clan") is
/// one for the Methuselah controlling him (rules-qa 41.2: "her
/// Methuselah's cards"; card text addresses its controller, D11), not his
/// owner: controlled by seat 0 though seat 1's, offered to seat 0; owned
/// by seat 0 but seat 1's now, not.
#[test]
fn dirty_contract_offers_lisandro_to_his_controller() {
    for controlled in [true, false] {
        let mut e = game(3);
        e.add_minion(SeatId(0), of_clan("Samedi", 1..=11, 1), 4, false);
        let m = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let (by, from) = if controlled { (0, 1) } else { (1, 0) };
        let l = e.add_minion(SeatId(by), find_card("Lisandro Giovanni"), 4, false);
        e.state.minions[l.index()].origin = Some(SeatId(from));
        let s = e.add_minion(SeatId(2), of_clan("Samedi", 1..=11, 0), 4, false);
        hand(&mut e, 0, &["Dirty Contract"]);
        e.at_master_phase();
        pick(&mut e, master_at(0, Target::Minion(m)));
        let req = request(&mut e);
        assert!(req.options.contains(&Choice::ChooseMinion(s)));
        assert_eq!(
            req.options.contains(&Choice::ChooseMinion(l)),
            controlled,
            "controlled {controlled}"
        );
    }
}

/// Dirty Contract's action requires a Samedi ([LSJ 20080604]): the chosen
/// Samedi deranged into a Malkavian is not offered it.
#[test]
fn dirty_contract_action_needs_the_chosen_to_be_a_samedi() {
    for deranged in [false, true] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), of_clan("Samedi", 1..=11, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let m = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let dc = in_play(&mut e, 0, "Dirty Contract", Some(m));
        e.state.cards[dc.index()].chosen = vec![s];
        if deranged {
            let d = in_play(&mut e, 1, "Derange", Some(s));
            e.state.minions[s.index()].deranged = Some(("Malkavian", Sect::Camarilla, d));
        }
        let req = request(&mut e);
        let offered = req
            .options
            .contains(&use_on(dc, 0, Some(s), Target::Minion(m)));
        assert_eq!(offered, !deranged);
    }
}

/// Jungle Hunting Ground: "Lock during your unlock phase to add 1 blood to
/// this card if it has fewer than 3 blood. Lock during your unlock phase to
/// move all the blood on this card to a ready Laibon you control." At 2: +1;
/// at 3, the add a legal no-op; the 3 moved to a Laibon (not a Brujah),
/// fed: Port Hunting Ground cannot feed it that turn.
#[test]
fn jungle_hunting_ground_stores_to_three_and_feeds_a_laibon() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 6..=11, 0), 1, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let j = in_play(&mut e, 0, "Jungle Hunting Ground", None);
    let port = in_play(&mut e, 0, "Port Hunting Ground", None);
    e.state.cards[j.index()].counters = 2;
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, use_on(j, 0, None, Target::None));
    assert_eq!(e.state.cards[j.index()].counters, 3);
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, use_on(j, 0, None, Target::None));
    assert_eq!(e.state.cards[j.index()].counters, 3, "a legal no-op at 3");
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(!req
        .options
        .contains(&use_on(j, 1, None, Target::Minion(other))));
    let b = blood(&e, l);
    pick(&mut e, use_on(j, 1, None, Target::Minion(l)));
    let req = request(&mut e);
    assert_eq!(blood(&e, l), b + 3);
    assert_eq!(e.state.cards[j.index()].counters, 0);
    assert!(
        !req.options
            .contains(&use_on(port, 0, None, Target::Minion(l))),
        "fed"
    );
}

/// The Path of Tears: "Only usable after you move a Salubri from your
/// uncontrolled region to your ready region during your influence phase.
/// Put this card on that Salubri. If you burned 3 or more pool for the
/// scarce penalty for this vampire, you gain 3 pool." A scarce Salubri
/// influenced beside another: 3 pool burned, the card's 1 paid, 3 gained;
/// a Salubri with no toll: no gain. Minions can burn it.
#[test]
fn path_of_tears_refunds_a_scarce_salubris_toll() {
    for scarce in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), find_card("Yael"), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let card = find_card(if scarce { "Mr. Noir" } else { "Dominica" });
        assert_eq!(CRYPT[card as usize].scarce, scarce);
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card,
                copy: Default::default(),
                counters: CRYPT[card as usize].capacity,
                banished: None,
            });
        hand(&mut e, 0, &["Path of Tears, The"]);
        pick(&mut e, Choice::Pass);
        pick_when(&mut e, Choice::InfluenceOut { slot: 0 });
        let s = MinionId(e.state.minions.len() as u32 - 1);
        assert_eq!(e.state.seats[0].pool, if scarce { 7 } else { 10 });
        pick(&mut e, master_at(0, Target::Minion(s)));
        request(&mut e);
        let path = placed_on(&e, "Path of Tears, The", Some(s));
        assert_eq!(e.state.seats[0].pool, 9, "scarce {scarce}");
        assert!(e.state.seats[0].out_of_turn.is_some());
        if !scarce {
            continue;
        }
        // Seat 1's vampire burns it.
        let z = e.state.minions_of(SeatId(1)).next().unwrap();
        let burn = use_on(path, 0, Some(z), Target::None);
        pick_when(&mut e, burn);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(ash_has(&e, 0, "Path of Tears, The"));
    }
}

/// The Path of Tears: "A vampire can have only one path"; The Path of the
/// Scorched Heart: "A vampire can follow only one path". A Path card is a
/// path followed: made Sabbat (Into the Fire's "becomes Sabbat"), the
/// Salubri bearing The Path of Tears and the True Brujah bearing the
/// Scorched Heart are offered no Sabbat path (rules-qa 97.1's offer); a
/// Sabbat vampire with no path is.
#[test]
fn path_of_tears_bars_following_a_sabbat_path() {
    let mut e = game(2);
    let free = e.add_minion(SeatId(0), find_card("Hartmut Stover"), 3, false);
    let salubri = e.add_minion(SeatId(0), find_card("Mr. Noir"), 3, false);
    let brujah = clan_minion(&mut e, SeatId(0), "True Brujah", 3..=11, 0, 3);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    in_play(&mut e, 0, "Path of Tears, The", Some(salubri));
    in_play(&mut e, 0, "Path of the Scorched Heart, The", Some(brujah));
    for m in [salubri, brujah] {
        e.state.minions[m.index()].sect_override = Some(Sect::Sabbat);
    }
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let followers: Vec<MinionId> = req
        .options
        .iter()
        .filter_map(|o| match *o {
            Choice::FollowPath { minion, .. } => Some(minion),
            _ => None,
        })
        .collect();
    assert!(followers.contains(&free));
    assert!(!followers.contains(&salubri), "The Path of Tears");
    assert!(
        !followers.contains(&brujah),
        "The Path of the Scorched Heart"
    );
}

/// Call the Great Beast: "The action to put the 11th counter on it is an
/// action to put a vampire into play, a sterile vampire cannot do it" (LSJ
/// 20060815, LSJ 20011212). A Baali made sterile by Barrenness: at 9
/// counters the counter action offered, at 10 not; not sterile, at 10.
/// Ledger: `[LSJ 20060815] [LSJ 20011212]`.
#[test]
fn call_the_great_beast_eleventh_counter_needs_a_fertile_baali() {
    for (counters, sterile, offered) in [(9u8, true, true), (10, true, false), (10, false, true)] {
        let mut e = game(2);
        let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let call = in_play(&mut e, 0, "Call the Great Beast", Some(n));
        e.state.cards[call.index()].counters = counters;
        if sterile {
            in_play(&mut e, 1, "Barrenness", Some(n));
        }
        assert_eq!(e.state.is_sterile(n), sterile);
        let ability = u8::from(counters >= 10);
        let req = request(&mut e);
        let take = use_on(call, ability, Some(n), Target::None);
        assert_eq!(req.options.contains(&take), offered, "{counters} {sterile}");
    }
}

/// Seat 0's ready Laibon of capacity 8 to 11 influenced in its influence
/// phase this turn.
fn influenced_laibon(e: &mut Engine) -> MinionId {
    let card = of_sect(Sect::Laibon, 8..=11, 0);
    e.state.seats[0].this_turn = 4;
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card,
            copy: Default::default(),
            counters: CRYPT[card as usize].capacity,
            banished: None,
        });
    pick(e, Choice::Pass);
    pick_when(e, Choice::InfluenceOut { slot: 0 });
    MinionId(e.state.minions.len() as u32 - 1)
}

/// Static Virtue: "If this Laibon was moved to your ready region during your
/// last influence phase, then for every two capacity above 4 this Laibon
/// has, you may move one Aye and one Orun from your hand or library to this
/// Laibon." Capacity 8: two pairs, from hand and library, the second card of
/// each pair owed (no Pass), each card in play on him. Ledger: `[LSJ
/// 20051127]`.
#[test]
fn static_virtue_moves_aye_and_orun_in_pairs() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let l = influenced_laibon(&mut e);
    assert!(
        (8..=9).contains(&e.state.capacity_of(l).unwrap()),
        "two pairs"
    );
    hand(&mut e, 0, &["Static Virtue", "Aye", "Orun"]);
    fill_hand(&mut e, 0);
    let filler = find_library("Form of Mist");
    e.state.seats[0].library = [
        filler,
        filler,
        filler,
        find_library("Aye"),
        find_library("Orun"),
    ]
    .into_iter()
    .chain(std::iter::repeat_n(filler, 5))
    .collect::<Vec<_>>()
    .into();
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_at(0, Target::Minion(l)));
    let aye = find_library("Aye");
    let orun = find_library("Orun");
    let req = expect(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    assert!(req.options.contains(&Choice::Pass));
    pick(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Hand,
            card: aye,
        },
    );
    // The second of each pair owed (no Pass; a single offer elided).
    let held = |e: &Engine, n| e.state.holds_count(l, Holding::Named(n));
    loop {
        let req = request(&mut e);
        if req.window != WindowLabel::LibrarySearch {
            break;
        }
        let owed = held(&e, "Aye") != held(&e, "Orun");
        assert_eq!(
            req.options.contains(&Choice::Pass),
            !owed,
            "{:?}",
            req.options
        );
        if owed {
            let want = if held(&e, "Aye") > held(&e, "Orun") {
                orun
            } else {
                aye
            };
            assert!(req
                .options
                .iter()
                .all(|o| matches!(o, Choice::TakeFrom { card, .. } if *card == want)));
        }
        let i = req.options.iter().position(|o| *o != Choice::Pass).unwrap();
        e.choose(i);
    }
    assert_eq!(e.state.holds_count(l, Holding::Named("Aye")), 2);
    assert_eq!(e.state.holds_count(l, Holding::Named("Orun")), 2);
    assert!(e
        .state
        .cards
        .iter()
        .filter(|c| c.host == Some(l) && c.in_play)
        .all(|c| !c.effect_only));
    assert!(ash_has(&e, 0, "Static Virtue"));
}

/// Static Virtue: "Otherwise, you may move one Aye or one Orun from your hand
/// or library to this Laibon." A Laibon in play before: one card, from the
/// library.
#[test]
fn static_virtue_moves_one_aye_or_orun_otherwise() {
    let mut e = game(2);
    let l = laibon(&mut e, 0, 0, &[]);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Static Virtue"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Orun"), find_library("Aye")].into();
    e.at_master_phase();
    pick(&mut e, master_at(0, Target::Minion(l)));
    pick(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: find_library("Orun"),
        },
    );
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::LibrarySearch, "one card only");
    assert_eq!(e.state.holds_count(l, Holding::Named("Orun")), 1);
    assert_eq!(e.state.holds_count(l, Holding::Named("Aye")), 0);
}

/// Powerbase: Tshwane: "Lock this location to reduce the cost of a card you
/// play by 1 pool". [ANK 20221102] minion cards too: a master (The
/// Parthenon, 2 pool) and an equipment (.44 Magnum, 2 pool) each 1 pool.
#[test]
fn tshwane_takes_a_pool_off_a_minion_card_and_a_master() {
    // The master.
    let mut e = game(2);
    laibon(&mut e, 0, 0, &[]);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 0, "Powerbase: Tshwane", None);
    hand(&mut e, 0, &["Parthenon, The"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    pick(&mut e, Choice::ResolveRider { card: t.0 });
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 9);
    assert!(e.state.cards[t.index()].locked);
    // The minion card, announced with it.
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &[]);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 0, "Powerbase: Tshwane", None);
    hand(&mut e, 0, &[".44 Magnum"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ResolveRider { card: t.0 });
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 9);
}

/// Powerbase: Tshwane: "(this location is not locked if that card is
/// canceled as it is played)" — [ANK 20230620] [ANK 20170605] [LSJ 20090705]
/// from the announcement to just before the cost is paid: locked during the
/// action, 1 pool off; [LSJ 20090705] [LSJ 20030917] [ANK 20230721] an
/// otherwise unaffordable card announced with it (no Pass), and Direct
/// Intervention canceling it: unlocked, nothing paid.
#[test]
fn tshwane_locks_for_an_action_card_and_stays_unlocked_if_canceled() {
    // During the action.
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &[]);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 0, "Powerbase: Tshwane", None);
    hand(&mut e, 0, &[".44 Magnum"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, use_on(t, 0, None, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 9);
    // Unaffordable without it; canceled.
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &[]);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 0, "Powerbase: Tshwane", None);
    e.state.seats[0].pool = 1;
    hand(&mut e, 0, &[".44 Magnum"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, master_play(0));
    request(&mut e);
    assert!(!e.state.cards[t.index()].locked);
    assert_eq!(e.state.seats[0].pool, 1);
}

/// Powerbase: Tshwane reduces a card not played as an action ([ANK
/// 20230824] [RTR 20070707]): Piper's Double Deuce, 2 pool, costs 1.
#[test]
fn tshwane_reduces_a_card_piper_brings_in() {
    let mut e = game(2);
    let an = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Gangrel" && c.sect == Some(Sect::Anarch), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = in_play(&mut e, 0, "Powerbase: Tshwane", None);
    hand(&mut e, 0, &["Piper", "Double Deuce"]);
    e.at_master_phase();
    pick(&mut e, play(0, 0, an));
    pick_when(&mut e, Choice::ResolveRider { card: t.0 });
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 9, "Double Deuce's 2 pool, 1 off");
    assert!(e.state.cards[t.index()].locked);
}

/// Powerbase: Tshwane with Antonio d'Erlette's mage ally put in play, its
/// cost paid as normal: [LSJ 20090920-2] [RTR 20070707] "Cost reductions
/// apply (eg. Powerbase: Tshwane)": Nephandus's 2 pool, 1 paid.
#[test]
fn tshwane_reduces_what_antonio_derlette_pays() {
    let mut e = game(2);
    let antonio = e.add_minion(SeatId(0), find_card("Antonio d'Erlette"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let t = in_play(&mut e, 0, "Powerbase: Tshwane", None);
    fill_hand(&mut e, 0);
    for _ in 0..3 {
        e.state.seats[0].library.push(find_library("Bonding"));
    }
    e.state.seats[0].library.push(find_library("Nephandus"));
    let fetch = Choice::UseAbility {
        source: AbilitySource::Vampire(antonio),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, fetch);
    pick(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: find_library("Nephandus"),
        },
    );
    pick_when(&mut e, Choice::ResolveRider { card: t.0 });
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 9);
    assert!(e.state.cards[t.index()].locked);
}

/// Powerbase: Tshwane: "Any Laibon may steal this location for his or her
/// controller as a Ⓓ action" — an action requiring a Laibon ([LSJ
/// 20080604]): seat 1's Laibon offered, its Ventrue not.
#[test]
fn tshwane_is_stolen_by_a_laibon_only() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = in_play(&mut e, 0, "Powerbase: Tshwane", None);
    let l = laibon(&mut e, 1, 0, &[]);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&use_on(t, 1, Some(l), Target::None)));
    assert!(!req.options.contains(&use_on(t, 1, Some(v), Target::None)));
}

/// Battle Frenzy: "Choose an Osebo in combat. That Osebo gets an additional
/// strike each round this combat that does not count against the limit.
/// During the press step each round this combat, that Osebo burns 1 blood."
/// Seat 1 plays it on its Osebo rushed by seat 0: two strikes landing on the
/// rusher, the second unrequested (mandatory, [PIB 20121028]), 1 blood
/// burned at the press step; seat 0's Frenzy never offered on its own turn
/// (rulebook "Out-of-turn").
#[test]
fn battle_frenzy_strikes_each_round_and_burns_in_the_press_step() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let o = e.add_minion(SeatId(1), of_clan("Osebo", 1..=11, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bum's Rush", "Frenzy", "Battle Frenzy"]);
    hand(&mut e, 1, &["Battle Frenzy"]);
    pick(&mut e, play_at(0, 0, a, o));
    let bf = master_at(0, Target::Minion(o));
    loop {
        let req = request(&mut e);
        assert!(
            req.seat != SeatId(0)
                || !req
                    .options
                    .iter()
                    .any(|c| matches!(c, Choice::Play { actor: None, .. })),
            "own turn: {:?}",
            req.options
        );
        if req.seat == SeatId(1) && req.options.contains(&bf) {
            pick(&mut e, bf);
            break;
        }
        e.choose(fallback(&req));
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 2, "two strikes of 1");
    assert_eq!(blood(&e, o), 2, "1 damage mended, 1 burned");
}

/// Battle Frenzy played after round 1's normal strikes (rules-digest D22:
/// additional strikes come after the normal pair): its strike that round
/// still (amber, rules-qa Batch 124), then the press step's burn.
#[test]
fn battle_frenzy_after_the_normal_pair_strikes_that_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let o = e.add_minion(SeatId(1), of_clan("Osebo", 1..=11, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bum's Rush"]);
    hand(&mut e, 1, &["Battle Frenzy"]);
    pick(&mut e, play_at(0, 0, a, o));
    let bf = master_at(0, Target::Minion(o));
    loop {
        let req = request(&mut e);
        if req.window == WindowLabel::CombatAdditionalStrikes && req.options.contains(&bf) {
            assert_eq!(blood(&e, a), 3, "after the normal pair");
            pick(&mut e, bf);
            break;
        }
        e.choose(fallback(&req));
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 2, "its strike this round");
    assert_eq!(blood(&e, o), 2, "1 damage mended, 1 burned");
}

/// Battle Frenzy: "The effect in the press step does not happen if the
/// combat ends prematurely (e.g. from torpor or "strike: combat ends")."
/// Seat 0's Majesty ends the combat in round 1: no blood burned. Ledger:
/// `[LSJ 20010312] [LSJ 20001027]`.
#[test]
fn battle_frenzy_burns_nothing_when_the_combat_ends_early() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["pre"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let o = e.add_minion(SeatId(1), of_clan("Osebo", 1..=11, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bum's Rush", "Majesty"]);
    hand(&mut e, 1, &["Battle Frenzy"]);
    pick(&mut e, play_at(0, 0, a, o));
    pick_when(&mut e, master_at(0, Target::Minion(o)));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, o), 4, "no press step");
    assert_eq!(blood(&e, a), 3, "Majesty's blood");
}

// ---- Modifiers and reactions, Kholos onward ---------------------------------

/// High Orun, High Aye: "Requires a Laibon with three or more Orun / Aye."
/// Their burn option is barred by a Laibon with three, in torpor too (ANK
/// 20240610: "cannot be used if control a Laibon with three Orun, even if
/// they are in torpor"), never by one with two. Ledger: High Orun `[ANK
/// 20240610]`.
#[test]
fn high_orun_burn_option_reads_the_orun_count() {
    for (name, held) in [("High Orun", "Orun"), ("High Aye", "Aye")] {
        for three in [false, true] {
            let mut e = game(2);
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            let l = laibon(&mut e, 1, 0, &[]);
            for _ in 0..2 + u8::from(three) {
                in_play(&mut e, 1, held, Some(l));
            }
            e.state.minions[l.index()].in_torpor = three;
            hand(&mut e, 1, &[name]);
            e.state.seats[1].library.push(find_library("Soak"));
            pick(&mut e, Choice::Pass);
            let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
            assert_eq!(
                req.options.contains(&Choice::Discard(0)),
                !three,
                "{name}, three {three}"
            );
        }
    }
}

/// Akunanse Kholo, Guruhi Kholo, Ishtarri Kholo (Osebo Kholo's text, rules-qa
/// 77.1): "Put this card on this Laibon … to represent the unique Laibon
/// title of … Kholo (worth 2 votes). Not usable if there are any older
/// ready untitled Laibon … Whenever this vampire is not a ready Laibon …,
/// move this card to (one of) the oldest ready untitled Laibon …". The
/// younger of two is not offered it; the older takes it, the card itself in
/// play; in torpor, it passes to the younger.
#[test]
fn kholos_title_the_oldest_of_their_clan_and_move_on() {
    for (name, clan) in [
        ("Akunanse Kholo", "Akunanse"),
        ("Guruhi Kholo", "Guruhi"),
        ("Ishtarri Kholo", "Ishtarri"),
    ] {
        let mut e = game(2);
        let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
        e.add_minion(SeatId(0), find_card("Karen Suadela"), 4, false);
        let old = clan_minion(&mut e, SeatId(1), clan, 7..=11, 0, 3);
        let young = clan_minion(&mut e, SeatId(1), clan, 1..=6, 0, 3);
        hand(&mut e, 0, &["Kine Resources Contested"]);
        hand(&mut e, 1, &[name]);
        pick(&mut e, play(0, 0, caller));
        pick(&mut e, alloc(0, 3));
        let req = offering(&mut e, play(0, 1, old));
        assert!(!req.options.contains(&play(0, 1, young)), "{name}: older");
        pick(&mut e, play(0, 1, old));
        request(&mut e);
        let title = Title::Laibon { name, votes: 2 };
        assert_eq!(e.state.title_of(old), title);
        assert_eq!(e.state.title_votes_of(old), 2);
        let card = placed_on(&e, name, Some(old));
        e.state.minions[old.index()].in_torpor = true;
        request(&mut e);
        assert_eq!(e.state.cards[card.index()].host, Some(young), "{name}");
        assert_eq!(e.state.title_of(young), title);
    }
}

/// Ancestor's Insight: "Requires a ready Laibon. [aus] [REACTION] Reduce a
/// bleed against you by 1. [AUS] [ACTION MODIFIER] Only usable during a
/// bleed action. +1 bleed (limited)." The acting Laibon's [AUS] makes 2,
/// seat 1's Laibon's [aus] 1; seat 1's non-Laibon is offered neither.
#[test]
fn ancestors_insight_reduces_a_bleed_or_adds_one() {
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &["aus", "aus"]);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = laibon(&mut e, 1, 1, &["aus"]);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[v.index()].gained_levels = vec!["aus"];
    hand(&mut e, 0, &["Ancestor's Insight"]);
    hand(&mut e, 1, &["Ancestor's Insight"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, a));
    let req = offering(&mut e, play(0, 0, z));
    assert_eq!(e.action().expect("bleed").bleed_amount(&e.state), 2);
    assert!(!req.options.contains(&play(0, 0, v)), "no Laibon");
    pick(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9);
}

/// Excellent Thirst: "Requires a Laibon with no blood. Only usable when
/// this vampire is about to hunt successfully. This Laibon gets +2 hunt."
/// An empty Laibon's hunt gains 3; with 1 blood it is never offered. A
/// card's hunt (Abactor, at +4) takes it too: 1 + 4 + 2. Ledger: `[RTR
/// 20030519] [RTR 20180511]`.
#[test]
fn excellent_thirst_adds_two_to_an_empty_laibons_hunt() {
    for case in ["empty", "one", "abactor"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 8..=11, 0), 5, false);
        e.state.minions[a.index()].blood = u8::from(case == "one");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        // A prince: the blood hunt has a voter.
        e.add_minion(SeatId(1), find_card("Tara"), 4, false);
        hand(&mut e, 0, &["Excellent Thirst"]);
        match case {
            "empty" => {
                pick_when(&mut e, play(0, 0, a));
                until(&mut e, SeatId(0), WindowLabel::MinionPhase);
                assert_eq!(blood(&e, a), 3);
            }
            "one" => {
                pick(&mut e, action(a, ActionKind::Hunt));
                never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
                assert_eq!(blood(&e, a), 2);
            }
            _ => {
                hand(&mut e, 0, &["Abactor"]);
                pick(&mut e, play(1, 0, a));
                pick_when(&mut e, play(0, 0, a));
                expect(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
                assert_eq!(blood(&e, a), 1 + 4 + 2);
            }
        }
    }
}

/// The Guruhi Are the Land: "If this is a bleed action, this Guruhi gets +1
/// bleed (limited). If this is a hunt action, this Guruhi gains an
/// additional blood if successful. If this is an equip action, this Guruhi
/// unlocks if the action is successful." A bleed of 2; Abactor's hunt for
/// 1 + 4 + 1 (a card's hunt, RTR 20030519); an equip leaving it unlocked.
/// Ledger: `[RTR 20030519] [RTR 20180511]`.
#[test]
fn the_guruhi_are_the_land_bleeds_hunts_or_equips() {
    for case in ["bleed", "abactor", "equip"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan("Guruhi", 7..=11, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), find_card("Tara"), 4, false);
        hand(&mut e, 0, &["Guruhi Are the Land, The"]);
        match case {
            "bleed" => {
                pick(&mut e, bleed(a));
                pick_when(&mut e, play(0, 0, a));
                request(&mut e);
                assert_eq!(e.action().expect("bleed").bleed_amount(&e.state), 2);
                until(&mut e, SeatId(0), WindowLabel::MinionPhase);
                assert_eq!(e.state.seats[1].pool, 8);
            }
            "abactor" => {
                e.state.minions[a.index()].blood = 0;
                hand(&mut e, 0, &["Abactor"]);
                pick(&mut e, play(1, 0, a));
                pick_when(&mut e, play(0, 1, a));
                expect(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
                assert_eq!(blood(&e, a), 1 + 4 + 1);
            }
            _ => {
                hand(&mut e, 0, &["Leather Jacket"]);
                pick(&mut e, play(1, 0, a));
                pick_when(&mut e, play(0, 2, a));
                until(&mut e, SeatId(0), WindowLabel::MinionPhase);
                assert!(e
                    .state
                    .cards
                    .iter()
                    .any(|c| c.present() && c.host == Some(a)));
                assert!(!e.state.minions[a.index()].locked, "unlocked");
            }
        }
    }
}

/// `m` gets +n stealth for the turn.
fn stealth_for_turn(e: &mut Engine, m: MinionId, n: i8) {
    e.state.minions[m.index()].timed.push((
        zvtes::cards::Lasting::Stealth(n),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
}

/// Falcon's Eye: "[ani] This vampire burns 1 blood to get +1 intercept."
/// Not offered with no blood (ANK 20210226: "cannot be played if the minion
/// cannot afford to burn the blood"); with 1, it burns it and blocks a +1
/// stealth bleed. Played via [ani], it requires Animalism (LSJ 20011204-3:
/// a [spi]-only vampire is not offered it). Ledger: `[ANK 20210226]`,
/// `[LSJ 20011204-3]`.
/// Ledger: `[LSJ 20011204-3] [PIB 20130704]`.
#[test]
fn falcons_eye_burns_for_intercept() {
    for blood_left in [0, 1] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), blood_left, false);
        e.state.minions[z.index()].gained_levels = vec!["ani"];
        let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.minions[y.index()].gained_levels = vec!["spi"];
        stealth_for_turn(&mut e, a, 1);
        hand(&mut e, 1, &["Falcon's Eye"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if blood_left == 0 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, z));
            assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
        } else {
            let req = offering(&mut e, play(0, 0, z));
            assert!(!req.options.contains(&play(0, 0, y)), "no Animalism");
            pick(&mut e, play(0, 0, z));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, z), 0);
            assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
        }
    }
}

/// Falcon's Eye [SPI]: "This reacting vampire attempts to block the current
/// action, ignoring the normal prey, predator or target restrictions for
/// blocking actions." Seat 2's vampire blocks seat 0's bleed of seat 1;
/// only who may block is widened (RTR 19950413). Ledger: `[RTR 19950413]
/// [LSJ 20020112] [RTR 20020501]`.
#[test]
fn falcons_eye_superior_blocks_another_methuselahs_action() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["spi", "spi"];
    e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
    hand(&mut e, 2, &["Falcon's Eye"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 2, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

/// Glare of Lies: "Only usable if a minion is bleeding you. [aus][obe] You
/// can look at the acting minion's controller's hand. [AUS][OBE] As above,
/// and the bleed amount is reduced by 1 for each action modifier card in
/// their hand." Seat 0's hand holds two action modifiers and a combat
/// card: a bleed of 3 does 1. A combo: a vampire with Auspex only is not
/// offered it.
#[test]
fn glare_of_lies_reduces_by_the_action_modifiers_in_hand() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Bleed(2),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["aus", "aus", "obe", "obe"];
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[y.index()].gained_levels = vec!["aus", "aus"];
    hand(
        &mut e,
        0,
        &["Conditioning", "Conditioning", "Immortal Grapple"],
    );
    hand(&mut e, 1, &["Glare of Lies"]);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, play(0, 1, z));
    assert!(!req.options.contains(&play(0, 1, y)), "a combo");
    pick(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9);
}

/// High Aye: "+1 stealth, +2 stealth if this action is undirected. If the
/// action succeeds, you may discard up to two cards from your hand (draw
/// afterward)." A bleed (directed) gets +1 only, enough past a 0-intercept
/// attempt; once it succeeds, its player discards two cards and draws two.
/// Blocked by a +1-intercept attempt, no discard. Ledger: `[LSJ 20060426]`.
#[test]
fn high_aye_stealth_by_directedness_and_a_discard_after() {
    for blocked in [false, true] {
        let mut e = game(2);
        let a = laibon(&mut e, 0, 0, &[]);
        for _ in 0..3 {
            in_play(&mut e, 0, "Aye", Some(a));
        }
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(
            &mut e,
            0,
            &["High Aye", "Immortal Grapple", "Immortal Grapple"],
        );
        e.state.seats[0].library = vec![find_library("Soak"); 3].into();
        if blocked {
            hand(&mut e, 1, &["Eagle's Sight"]);
            e.state.minions[z.index()].gained_levels = vec!["aus"];
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        let req = offering(&mut e, play(0, 0, a));
        assert!(!req.options.contains(&play(0, 1, a)), "directed: +1");
        pick(&mut e, play(0, 0, a));
        if blocked {
            pick_when(&mut e, play(0, 0, z));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
            assert_eq!(e.state.seats[0].ash.len(), 1, "High Aye only");
        } else {
            let grapple = find_library("Immortal Grapple");
            let i = e.state.seats[0]
                .hand
                .iter()
                .position(|&c| c == grapple)
                .unwrap() as u8;
            pick_when(&mut e, Choice::Discard(i));
            let i = e.state.seats[0]
                .hand
                .iter()
                .position(|&c| c == grapple)
                .unwrap() as u8;
            pick_when(&mut e, Choice::Discard(i));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
            assert!(!e.state.seats[0].hand.contains(&grapple));
            assert_eq!(e.state.seats[0].hand.len(), 3, "drawn back");
        }
    }
}

/// High Orun: "For the remainder of this action, cards that require any
/// Disciplines cost this Laibon 1 less blood, and your hand size is one
/// card larger." Govern the Unaligned's 1 blood, paid at resolution, is
/// reduced to 0 (LSJ 20091002: the current action card); the hand size 8
/// during the action, 7 after. Ledger: `[LSJ 20091002]`.
#[test]
fn high_orun_lowers_discipline_costs_for_the_action() {
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &["dom"]);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    for _ in 0..3 {
        in_play(&mut e, 0, "Orun", Some(a));
    }
    hand(&mut e, 0, &["Govern the Unaligned", "High Orun"]);
    let before = blood(&e, a);
    pick(&mut e, play(0, 0, a));
    // Govern out of the hand: High Orun is its first card.
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
    assert_eq!(blood(&e, a), before, "Govern's blood reduced to 0");
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
}

/// A seat-0 caller (Tara, a prince) playing Kine Resources Contested, its
/// terms chosen, beside another seat-0 vampire: the caller.
fn kine_caller(e: &mut Engine) -> MinionId {
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), find_card("Karen Suadela"), 4, false);
    caller
}

/// Madrigal [MEL]: "This vampire gets +2 votes. … vampires casting votes
/// or ballots in agreement with this vampire gain 1 blood. Vampires casting
/// votes or ballots in opposition to this vampire burn 1 blood. Blood is
/// gained or burned once results are tallied." Played by the acting
/// vampire, which votes in favor: it and seat 1's voter in favor gain 1,
/// seat 2's voter against burns 1 (LSJ 20020123: the acting vampire agrees
/// with itself). Abstaining, nothing moves (LSJ 20020123). Ledger: `[LSJ
/// 20020123]` both rows.
#[test]
fn madrigal_feeds_its_side_and_bleeds_the_other() {
    for abstain in [false, true] {
        let mut e = game(3);
        let caller = kine_caller(&mut e);
        e.state.minions[caller.index()].gained_levels = vec!["mel", "mel"];
        let b = e.add_minion(SeatId(1), textless(|c| c.capacity >= 6, 0), 4, false);
        e.state.minions[b.index()].granted_title = Some(Title::Primogen);
        let c = e.add_minion(SeatId(2), textless(|c| c.capacity >= 6, 1), 4, false);
        e.state.minions[c.index()].granted_title = Some(Title::Primogen);
        hand(&mut e, 0, &["Kine Resources Contested", "Madrigal"]);
        pick(&mut e, play(0, 0, caller));
        pick(&mut e, alloc(0, 3));
        pick_when(&mut e, play(0, 2, caller));
        if !abstain {
            pick_when(&mut e, votes(caller, true));
        }
        pick_when(&mut e, votes(b, true));
        pick_when(&mut e, votes(c, false));
        let caller_blood = blood(&e, caller);
        close_polling(&mut e);
        request(&mut e);
        let moved = i32::from(!abstain);
        let after = |m| blood(&e, m) as i32;
        assert_eq!(
            after(caller),
            caller_blood as i32 + moved,
            "abstain {abstain}"
        );
        assert_eq!(after(b), 4 + moved);
        assert_eq!(after(c), 4 - moved);
    }
}

/// Devil-Channel: Throat [abo]/[ABO]: "Only usable when a non-werewolf ally
/// or younger vampire attempts to block. That minion's blocking attempt
/// fails (do not lock that minion). That minion cannot attempt to block
/// this action again. [ABO] As above, but lock that blocking minion." A
/// younger vampire's attempt fails, it is not offered again, unlocked or
/// locked by the level; High Top (a werewolf) and an older vampire are
/// never offered to it.
#[test]
fn devil_channel_throat_fails_a_younger_blockers_attempt() {
    for case in ["abo", "ABO", "werewolf", "older"] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| (6..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        e.state.minions[a.index()].gained_levels = match case {
            "ABO" => vec!["abo", "abo"],
            _ => vec!["abo"],
        };
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = match case {
            "werewolf" => e.add_ally(SeatId(1), find_library("High Top")),
            "older" => e.add_minion(SeatId(1), textless(|c| c.capacity >= 10, 0), 4, false),
            _ => e.add_minion(SeatId(1), textless(|c| c.capacity <= 4, 0), 4, false),
        };
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Devil-Channel: Throat"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        let mode = if case == "ABO" { 2 } else { 1 };
        if case == "werewolf" || case == "older" {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
            continue;
        }
        pick_when(&mut e, play(0, mode, a));
        let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
        assert!(!req.options.contains(&block(z)), "not again");
        assert_eq!(e.state.minions[z.index()].locked, case == "ABO");
        assert!(e.state.seats[0]
            .ash
            .contains(&find_library("Devil-Channel: Throat")));
    }
}

/// Devil-Channel: Throat [abo] [REFLEX]: "Cancel a frenzy card played on
/// this vampire as it is played." Terror Frenzy played on the acting
/// vampire by its blocker (LSJ 20051113) is canceled, still played. Ledger:
/// `[LSJ 20051113]`.
#[test]
fn devil_channel_throat_reflex_cancels_a_frenzy_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["abo"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("ANI", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Devil-Channel: Throat"]);
    hand(&mut e, 1, &["Terror Frenzy"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, b));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Terror Frenzy")));
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Devil-Channel: Throat")));
}

/// Bind the Night-Walker: "Requires a minion with a Kerrie. Usable by a
/// locked minion. Play during an action directed at this minion or a card
/// on this minion. Only usable when a vampire without Auspex [aus] is
/// acting. The action ends (unsuccessfully), and no cost is paid. A minion
/// may play only one Bind the Night-Walker each turn." Seat 0's Bum's Rush
/// at the locked Kerrie bearer ends at once: no combat, the actor locked,
/// nothing more played (RTR 20180719). The same turn, another vampire's
/// Bum's Rush at it finds no second Bind; an Auspex actor none. Ledger:
/// `[RTR 20180719]`, `[ANK 20211015]`.
#[test]
fn bind_the_night_walker_ends_an_action_at_its_kerrie_bearer() {
    for auspex in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        if auspex {
            e.state.minions[a.index()].gained_levels = vec!["aus"];
        }
        let a2 = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = laibon(&mut e, 1, 0, &[]);
        e.state.minions[z.index()].locked = true;
        in_play(&mut e, 1, "Kerrie", Some(z));
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Bum's Rush", "Bum's Rush"]);
        hand(
            &mut e,
            1,
            &["Bind the Night-Walker", "Bind the Night-Walker"],
        );
        pick(&mut e, play_at(0, 0, a, z));
        if auspex {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, z));
            continue;
        }
        pick_when(&mut e, play(0, 0, z));
        let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
        assert!(e.state.minions[a.index()].locked);
        assert_eq!((blood(&e, a), blood(&e, z)), (4, 5), "no combat");
        assert!(!req.options.contains(&play_at(0, 0, a, z)));
        pick(&mut e, play_at(0, 0, a2, z));
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, z));
    }
}

/// Bind the Night-Walker: not once a block is attempted (rules-qa 102.5's
/// reading; ANK 20260316): seat 1's other vampire attempts to block the
/// Bum's Rush at the Kerrie bearer, and Bind is never offered after.
#[test]
fn bind_the_night_walker_not_after_a_block_attempt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = laibon(&mut e, 1, 0, &[]);
    e.state.minions[z.index()].locked = true;
    in_play(&mut e, 1, "Kerrie", Some(z));
    let y = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bum's Rush"]);
    hand(&mut e, 1, &["Bind the Night-Walker"]);
    pick(&mut e, play_at(0, 0, a, z));
    offering(&mut e, play(0, 0, z));
    pick_when(&mut e, block(y));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, z));
}

/// Internal Recursion: "[pre] During the resulting combat, this vampire can
/// strike: combat ends. If they do so and the acting minion is an ally or
/// younger vampire, unlock this blocking vampire before combat ends. [tem]
/// This blocking vampire sets the range for the first round of the
/// resulting combat. [TEM] The acting minion does not unlock during their
/// next unlock phase." Played once blocked, before combat (LSJ 20020204).
/// [pre] against a younger actor: the blocker ends combat and unlocks;
/// against an older one, it stays locked; ending it with Majesty instead,
/// it stays locked (RTR 20020501: only this strike). [tem] long: the first
/// round at long range, no determine range step (RTR 19970630); after
/// Gang Tactics' close, close (PIB 20120214: the first setter). [TEM]: the
/// actor skips its next unlock. Played via [pre] it requires Presence: a
/// Temporis-only blocker is offered [tem] only (LSJ 20011204-3). Ledger:
/// `[LSJ 20020204]`, `[RTR 20020501]`, `[RTR 19970630]`, `[PIB 20120214]`,
/// `[LSJ 20011204-3]`.
/// Ledger: `[LSJ 20011204-3] [PIB 20130704]`, `[RTR 19970630] [ANK 20180720]`.
#[test]
fn internal_recursion_strikes_ends_sets_range_or_pins() {
    for case in ["pre younger", "pre older", "majesty", "tem", "gang", "TEM"] {
        let mut e = game(2);
        let cap = if case == "pre older" { 10..=11 } else { 1..=4 };
        let a = if case == "gang" {
            let a = e.add_minion(SeatId(0), plain("Nosferatu antitribu", 0), 5, false);
            e.add_minion(SeatId(0), plain("Nosferatu antitribu", 1), 4, false);
            a
        } else {
            // No printed stealth (a fixture rank below 2): the block lands.
            let a = (0..CRYPT.len() as u16)
                .filter(|&i| {
                    let c = &CRYPT[i as usize];
                    !c.imbued && cap.contains(&c.capacity) && fixture_rank(i).is_some_and(|r| r < 2)
                })
                .min_by_key(|&i| (fixture_rank(i), i))
                .expect("a fixture vampire");
            let a = e.add_minion(SeatId(0), a, 4, false);
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            a
        };
        let no_pre = |c: &zvtes::crypt::CryptCard| {
            (6..=8).contains(&c.capacity)
                && !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("pre"))
        };
        let z = e.add_minion(SeatId(1), textless(no_pre, 1), 5, false);
        e.state.minions[z.index()].gained_levels = match case {
            "tem" | "gang" => vec!["tem"],
            "TEM" => vec!["tem", "tem"],
            _ => vec!["pre"],
        };
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 1, &["Internal Recursion", "Majesty"]);
        if case == "gang" {
            hand(&mut e, 0, &["Gang Tactics"]);
        }
        e.notes = Some(Vec::new());
        pick(&mut e, bleed(a));
        if case == "gang" {
            pick(&mut e, play(0, 0, a));
        }
        pick_when(&mut e, block(z));
        let mode = match case {
            "tem" | "gang" => 2,
            "TEM" => 3,
            _ => 0,
        };
        let req = offering(&mut e, play(0, mode, z));
        if mode == 2 {
            assert!(!req.options.contains(&play(0, 0, z)), "no Presence");
        }
        pick(&mut e, play(0, mode, z));
        match case {
            "tem" | "gang" => {
                assert!(!range_step_until_minion_phase(&mut e));
                let want = if case == "gang" {
                    Range::Close
                } else {
                    Range::Long
                };
                assert_eq!(strike_ranges(&mut e)[0], want, "{case}");
            }
            "TEM" => {
                until(&mut e, SeatId(0), WindowLabel::MinionPhase);
                assert!(e.state.minions[a.index()].skip_unlock);
            }
            _ => {
                let end = if case == "majesty" {
                    play(0, 0, z)
                } else {
                    Choice::StrikeCombatEnds
                };
                pick_when(&mut e, end);
                until(&mut e, SeatId(0), WindowLabel::MinionPhase);
                assert_eq!(
                    e.state.minions[z.index()].locked,
                    case != "pre younger",
                    "{case}"
                );
            }
        }
    }
}

/// Hall of Hades' Court [dom]: "Requires a vampire with capacity 5 or more.
/// This vampire gets +2 votes." [TEM]: "Multiplier. Only usable by a ready
/// unlocked vampire other than the acting vampire. The acting vampire's
/// votes are doubled when tallying results. Only one multiplier can be
/// played on a vampire each action." Tara (a prince, 2 votes) gets +2 from
/// [dom], then seat 1's vampire doubles her 4 votes to 8; seat 2's vampire
/// is offered no second multiplier. With Orlando Oriundus ready beside her
/// ("other multipliers may not be played on your vampires"), [TEM] is
/// never offered.
#[test]
fn hall_of_hades_court_votes_and_doubles() {
    for orlando in [false, true] {
        let mut e = game(3);
        let caller = kine_caller(&mut e);
        e.state.minions[caller.index()].gained_levels = vec!["dom"];
        if orlando {
            e.add_minion(SeatId(0), find_card("Orlando Oriundus"), 4, false);
        }
        let h = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[h.index()].gained_levels = vec!["tem", "tem"];
        let h2 = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
        e.state.minions[h2.index()].gained_levels = vec!["tem", "tem"];
        hand(
            &mut e,
            0,
            &["Kine Resources Contested", "Hall of Hades' Court"],
        );
        hand(&mut e, 1, &["Hall of Hades' Court"]);
        hand(&mut e, 2, &["Hall of Hades' Court"]);
        pick(&mut e, play(0, 0, caller));
        pick(&mut e, alloc(0, 3));
        pick_when(&mut e, play(0, 0, caller));
        pick_when(&mut e, votes(caller, true));
        let own = tally(&e).0;
        if orlando {
            assert_eq!(own, 2 * 4, "Orlando's own doubling");
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 3, h));
            continue;
        }
        assert_eq!(own, 4);
        pick_when(&mut e, play(0, 3, h));
        let req = request(&mut e);
        assert_eq!(tally(&e).0, 8);
        assert!(!req.options.contains(&play(0, 3, h2)));
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 3, h2));
    }
}

/// Hall of Hades' Court [tem]: "Cancel a reaction card as it is played by
/// a younger vampire without Temporis [tem], and its cost is not paid." In
/// the polling step, seat 2's younger vampire's Madrigal [pre] (a reaction,
/// +2 votes) is canceled: no votes, the card still played; an older
/// player's Madrigal is not offered to it.
#[test]
fn hall_of_hades_court_cancels_a_younger_non_temporis_reaction() {
    for younger in [true, false] {
        let mut e = game(3);
        let caller = kine_caller(&mut e);
        let h = e.add_minion(
            SeatId(1),
            textless(|c| (6..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        e.state.minions[h.index()].gained_levels = vec!["tem"];
        let cap = if younger { 1..=4 } else { 10..=11 };
        let y = e.add_minion(
            SeatId(2),
            textless(|c| cap.contains(&c.capacity), 1),
            4,
            false,
        );
        e.state.minions[y.index()].gained_levels = vec!["pre"];
        hand(&mut e, 0, &["Kine Resources Contested"]);
        hand(&mut e, 1, &["Hall of Hades' Court"]);
        hand(&mut e, 2, &["Madrigal"]);
        pick(&mut e, play(0, 0, caller));
        pick_when(&mut e, alloc(0, 3));
        pick_when(&mut e, play(0, 0, y));
        if !younger {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, h));
            continue;
        }
        pick_when(&mut e, play(0, 1, h));
        request(&mut e);
        assert!(e.state.seats[2].ash.contains(&find_library("Madrigal")));
        assert_eq!(e.state.votes_held(y), 0, "canceled");
    }
}

/// Aversion: "[cel] Reduce a bleed against you by 1. [val] Burn X pool to
/// reduce a bleed against you by 2X+1." A bleed of 1 does nothing after
/// [cel]; a bleed of 3 nothing after [val] at X = 1, its pool burned.
/// Played via [val], it requires Valeren, not Celerity (LSJ 20011204-3: a
/// [cel]-only vampire is offered only [cel]). Ledger: `[LSJ 20011204-3]`.
/// Ledger: `[LSJ 20011204-3] [PIB 20130704]`.
#[test]
fn aversion_reduces_a_bleed_two_ways() {
    for val in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        if val {
            e.state.minions[a.index()].timed.push((
                zvtes::cards::Lasting::Bleed(2),
                zvtes::state::Expiry::EndOfTurn,
                SeatId(0),
            ));
        }
        let no_cel_val = |c: &zvtes::crypt::CryptCard| {
            c.capacity >= 4
                && !c
                    .disciplines
                    .iter()
                    .any(|d| ["cel", "val"].contains(&d.to_ascii_lowercase().as_str()))
        };
        let z = e.add_minion(SeatId(1), textless(no_cel_val, 0), 4, false);
        e.state.minions[z.index()].gained_levels = vec!["val"];
        let c = e.add_minion(SeatId(1), textless(no_cel_val, 1), 4, false);
        e.state.minions[c.index()].gained_levels = vec!["cel"];
        hand(&mut e, 1, &["Aversion"]);
        pick(&mut e, bleed(a));
        if val {
            let req = offering(&mut e, play(0, 1, z));
            assert!(!req.options.contains(&play(0, 1, c)), "Valeren");
            assert!(!req.options.contains(&play(0, 0, z)), "Celerity");
            pick(&mut e, play(0, 1, z));
            pick(&mut e, set_x(1));
        } else {
            pick_when(&mut e, play(0, 0, c));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - i32::from(val));
        assert_eq!(blood(&e, z), 4 - u8::from(val), "its cost");
    }
}

/// Aversion [val]: "Burn X pool": X up to the whole pool, its Methuselah
/// ousted by the last (rules-digest D3, D14; rules-qa 101.3). At 3 pool,
/// X is offered 0 to 3; X = 3 ousts seat 1.
#[test]
fn aversion_burns_up_to_the_whole_pool() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["val"];
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    e.state.seats[1].pool = 3;
    hand(&mut e, 1, &["Aversion"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, z));
    let req = request(&mut e);
    assert_eq!(req.options, (0..=3).map(set_x).collect::<Vec<_>>());
    e.choose(index_of(&req, set_x(3)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted, "the last pool burned");
}

/// Aversion [VAL]: "Only usable when a minion is bleeding you for 1 or
/// more. Put this card on the acting minion. You still control this card.
/// This minion gets -1 bleed when bleeding you. Any minion may burn this
/// card as a +1 stealth Ⓓ action." The card itself on the bleeder, seat
/// 1's: this bleed of 2 does 1; a next bleed at seat 1 does 0, one at seat
/// 2 its 1. A seat-0 minion then burns it as a +1 stealth action.
#[test]
fn aversion_superior_rides_the_bleeder() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a2 = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Bleed(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["val", "val"];
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
    hand(&mut e, 1, &["Aversion"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 2, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9);
    let card = placed_on(&e, "Aversion", Some(a));
    assert_eq!(e.state.card_controller(card), SeatId(1));
    assert_eq!(e.state.bleed_against_mod(a, SeatId(1)), -1);
    assert_eq!(e.state.bleed_against_mod(a, SeatId(2)), 0);
    pick(&mut e, use_on(card, 0, Some(a2), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[card.index()].present());
    assert!(e.state.seats[1].ash.contains(&find_library("Aversion")));
}

/// Gift of Bellona [aus]: "If this action is blocked, your hand size is one
/// card larger during the resulting combat." Seat 0's hand size is 8 in the
/// combat, 7 after.
#[test]
fn gift_of_bellona_grows_the_hand_in_the_resulting_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["aus"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["pot"];
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Gift of Bellona"]);
    hand(&mut e, 1, &["Torn Signpost"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, block(z));
    let mut seen = false;
    loop {
        let req = request(&mut e);
        if e.combat().is_some() {
            assert_eq!(e.state.hand_size(SeatId(0)), 8);
            seen = true;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(seen, "a combat decision");
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
}

/// Gift of Bellona [val]: "Only usable when an action to equip with a
/// weapon is successful. Unlock this acting vampire." A take of a weapon
/// from another minion, other equipment with it, counts (ANK 20170326):
/// the vampire ends unlocked. Played via [val], it requires Valeren (LSJ
/// 20011204-3: an Auspex-only vampire is offered only [aus]). Ledger: `[ANK
/// 20170326]`, `[LSJ 20011204-3]`.
/// Ledger: `[LSJ 20011204-3] [PIB 20130704]`.
#[test]
fn gift_of_bellona_unlocks_a_weapon_equip() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["val"];
    let g = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[g.index()].gained_levels = vec!["aus"];
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Dagger", Some(g));
    in_play(&mut e, 0, "Leather Jacket", Some(g));
    hand(&mut e, 0, &["Gift of Bellona"]);
    pick(&mut e, take_from(a, g));
    loop {
        let req = request(&mut e);
        if req.window != WindowLabel::ActionTerms {
            break;
        }
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::TakeEquipment { .. }))
            .unwrap_or(0);
        e.choose(i);
    }
    let req = offering(&mut e, play(0, 1, a));
    assert!(!req.options.contains(&play(0, 0, a)), "no Auspex");
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
    assert!(!e.state.minions[a.index()].locked);
}

/// Gift of Bellona [VAL]: "Only usable when an action to equip with a
/// weapon from your hand is blocked. Before combat begins, equip this
/// vampire with the weapon instead of placing it in your ash heap (pay cost
/// as normal). During the first round of this combat, that weapon cannot be
/// used." The Dagger itself on the vampire, its 1 pool paid; its strikes
/// not offered in the first round; and it still fetches the second Dagger
/// from hand (LSJ 20100421). Ledger: `[LSJ 20080815] [ANK 20201229] [ANK
/// 20230824]`, Dagger's `[LSJ 20100421]`.
/// Ledger: `[LSJ 20100725] [LSJ 20100303] [ANK 20180817]`,
/// `[LSJ 20090115-1] [ANK 20170309]`, `[ANK 20230824] [RTR 20070707]`.
#[test]
fn gift_of_bellona_equips_the_blocked_weapon_before_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["val", "val"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Dagger", "Gift of Bellona", "Dagger"]);
    // The equip's +1 stealth offset: the blocker's block succeeds.
    stealth_for_turn(&mut e, a, -1);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 2, a));
    let dagger = find_library("Dagger");
    let mut round_one = None;
    loop {
        let req = request(&mut e);
        // The second Dagger, from hand.
        if let Some(i) = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::TakeFrom { .. }))
        {
            e.choose(i);
            continue;
        }
        if let Some(c) = e.combat() {
            let idle = e.action().and_then(|x| x.idle_weapon).expect("the weapon");
            if req.seat == SeatId(0) && round_one.is_none() && c.round == 1 {
                round_one = Some(req.options.iter().any(|o| {
                    matches!(o, Choice::UseAbility { source: AbilitySource::Card(id), .. }
                        if *id == idle)
                }));
            }
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    // The second Dagger's strikes make the decision; the first's absent.
    assert_eq!(
        round_one,
        Some(false),
        "no strike of that Dagger in round 1"
    );
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    let on: Vec<_> = e
        .state
        .cards
        .iter()
        .filter(|c| c.card == dagger && c.present() && c.host == Some(a))
        .collect();
    assert_eq!(on.len(), 2, "both Daggers");
    assert!(!ash_has(&e, 0, "Dagger"));
    assert_eq!(e.state.seats[0].pool, 9, "the first Dagger's pool");
}

/// Conscripted Statue [vis]: "The opposing minion takes 1 damage during
/// strike resolution each round of the resulting combat when the range is
/// close." Environmental (RTR 19970630): the blocker loses 1 more than
/// without it.
/// Ledger: `[RTR 19970630]`,
/// `[RTR 19941109] [TOM 19951216] [PIB 20140324] [ANK 20200422]`.
#[test]
fn conscripted_statue_hurts_the_blocker_at_close_range() {
    let mut lost = Vec::new();
    for statue in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["vis"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        if statue {
            hand(&mut e, 0, &["Conscripted Statue"]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if statue {
            pick_when(&mut e, play(0, 0, a));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        lost.push(4 - blood(&e, z));
    }
    assert_eq!(lost[1], lost[0] + 1);
}

/// Conscripted Statue [VIS]: "Cancel the resulting combat and put this card
/// into play. This card represents an ally with 2 life and 2 strength.
/// This ally enters combat with the blocking minion. … Burn this card
/// after combat or if the combat is canceled." The statue fights: the
/// blocker loses 2, the acting vampire nothing; the action blocked; the
/// statue card in play during the combat, in the ash heap after.
#[test]
fn conscripted_statue_fights_the_blocker_in_its_place() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["vis", "vis"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["pot"];
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Conscripted Statue"]);
    hand(&mut e, 1, &["Torn Signpost"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let mut fighting = None;
    loop {
        let req = request(&mut e);
        if let Some(c) = e.combat() {
            let s = c.sides[0].minion;
            fighting.get_or_insert((s, c.sides[1].minion));
            placed_on(&e, "Conscripted Statue", None);
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    let (s, o) = fighting.expect("a combat");
    assert_ne!(s, a, "the statue fights");
    assert_eq!(o, z);
    assert!(e.state.minions[s.index()].is_ally());
    assert_eq!(blood(&e, a), 4, "the acting vampire untouched");
    assert_eq!(blood(&e, z), 2, "2 strength");
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert!(!e.state.minions[s.index()].in_play);
    assert!(ash_has(&e, 0, "Conscripted Statue"));
}

/// Conscripted Statue [VIS]: "The vampire playing it is not considered
/// having burned a minion …, but the opponent is" (LSJ 20090922, ANK
/// 20220916): a 3-strength blocker burns the statue in round 1 and is
/// offered Taking the Skin: Minion. Ledger: `[LSJ 20090922] [ANK
/// 20220916]`.
#[test]
fn conscripted_statue_burned_by_the_blocker_is_its_burn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["vis", "vis"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["abo"];
    e.state.minions[z.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Conscripted Statue"]);
    hand(&mut e, 1, &["Taking the Skin: Minion"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, z));
    request(&mut e);
    placed_on(&e, "Taking the Skin: Minion", Some(z));
}

/// Echo of Harmonies [mel]: "This vampire gets +2 votes … usable by a ready
/// vampire other than the acting minion." The acting Methuselah's other
/// vampires (LSJ 19990425), a locked one too (LSJ 19980210), a copy each
/// (ANK 20200515): +2 and +2; another Methuselah's vampire is never offered
/// it. Played via [mel], it requires Melpominee (LSJ 20011204-3). Ledger:
/// `[LSJ 19990425]`, `[LSJ 19980210]`, `[ANK 20200515]`, `[LSJ
/// 20011204-3]`.
/// Ledger: `[LSJ 20011204-3] [PIB 20130704]`.
#[test]
fn echo_of_harmonies_gives_votes_from_another_vampire() {
    let mut e = game(2);
    let caller = kine_caller(&mut e);
    let k = MinionId(1);
    e.state.minions[k.index()].gained_levels = vec!["mel"];
    e.state.minions[k.index()].locked = true;
    let w = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[w.index()].gained_levels = vec!["mel"];
    let y = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[y.index()].gained_levels = vec!["mel"];
    let p = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[p.index()].gained_levels = vec!["pre"];
    hand(
        &mut e,
        0,
        &[
            "Kine Resources Contested",
            "Echo of Harmonies",
            "Echo of Harmonies",
        ],
    );
    hand(&mut e, 1, &["Echo of Harmonies"]);
    pick(&mut e, play(0, 0, caller));
    pick(&mut e, alloc(0, 3));
    let req = offering(&mut e, play(0, 2, k));
    assert!(req.options.contains(&play(0, 2, w)));
    assert!(!req.options.contains(&play(0, 2, p)), "Melpominee");
    pick(&mut e, play(0, 2, k));
    pick_when(&mut e, play(0, 2, w));
    pick_when(&mut e, votes(k, true));
    pick_when(&mut e, votes(w, true));
    assert_eq!(tally(&e).0, 4 + u32::from(e.state.title_votes_of(k)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 2, y));
}

/// Echo of Harmonies [MEL]: "Move the political action card used to call
/// the referendum from your ash heap to this vampire, face down and out of
/// play; this vampire can play it as if from your hand (requirements and
/// cost apply as normal)." After resolution, the Kine Resources Contested
/// goes on Karen, not in the ash heap; next turn she alone is offered it
/// (LSJ 20100426: requirements apply), while Charming Lobby, naming a card
/// in hand, is not offered (ANK 20201029): no effect reading the hand sees
/// it (LSJ 20050118). Canceled by Delaying Tactics, the card back in hand,
/// [MEL] is never offered (LSJ 20020911, LSJ 20050804). Ledger: `[LSJ
/// 20020911]`, `[LSJ 20050804]`, `[LSJ 20100426] [LSJ 20091128] [LSJ
/// 20011205]`, `[ANK 20201029]`, `[LSJ 20050118]`.
#[test]
fn echo_of_harmonies_retains_the_political_card() {
    for delayed in [false, true] {
        let mut e = game(2);
        let caller = kine_caller(&mut e);
        let k = MinionId(1);
        e.state.minions[k.index()].gained_levels = vec!["mel", "mel"];
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[z.index()].granted_title = Some(Title::Primogen);
        hand(
            &mut e,
            0,
            &["Kine Resources Contested", "Echo of Harmonies"],
        );
        let kine = find_library("Kine Resources Contested");
        if delayed {
            hand(&mut e, 1, &["Delaying Tactics"]);
            pick(&mut e, play(0, 0, caller));
            pick(&mut e, alloc(0, 3));
            pick_when(&mut e, play(0, 0, z));
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 4, k));
            assert!(e.state.seats[0].hand.contains(&kine));
            continue;
        }
        pass_referendum(&mut e, caller, &[alloc(0, 3)]);
        pick_when(&mut e, play(0, 4, k));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!ash_has(&e, 0, "Kine Resources Contested"));
        assert!(e.state.minions[k.index()].stored.0.contains(&kine));
        // Seat 0's next turn: Karen plays it from her store.
        hand(&mut e, 0, &["Charming Lobby"]);
        e.state.minions[k.index()].locked = false;
        e.state.minions[caller.index()].locked = false;
        e.state.minions[k.index()].gained_levels = vec!["mel", "mel", "pre"];
        let req = loop {
            pick(&mut e, Choice::Pass);
            let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            if e.state.active == SeatId(0) {
                break req;
            }
        };
        let stored = |o: &Choice, m: MinionId| matches!(o, Choice::Play { card: 1, actor: Some(a), .. } if *a == m);
        assert!(
            req.options.iter().any(|o| stored(o, k)),
            "{:?}",
            req.options
        );
        assert!(!req.options.iter().any(|o| stored(o, caller)), "hers alone");
        assert!(
            !req.options
                .iter()
                .any(|o| matches!(o, Choice::Play { card: 0, .. })),
            "no Charming Lobby: no political card in hand"
        );
    }
}

/// Consume the Dead [REACTION]: "Only usable when a minion or retainer is
/// being burned by any means except diablerie. Usable during combat.
/// Usable by a locked Nagaraja. Remove that minion or retainer from the
/// game instead. This Nagaraja gains 1 blood if the minion was a vampire or
/// gains X+1 blood, where X is the blood or pool cost of that ally or
/// retainer." Moise Kasavubu (2 pool) blocking and burned in combat: seat
/// 2's locked Nagaraja removes him for 3 blood. Eldest Command Undeath
/// burning seat 1's vampire: removed for 1 blood, out of the ash heap; the
/// acting Methuselah's own Nagaraja never plays it (a reaction).
#[test]
fn consume_the_dead_removes_a_burning_minion_instead() {
    for vampire in [false, true] {
        let mut e = game(3);
        let n = clan_minion(&mut e, SeatId(2), "Nagaraja", 4..=11, 0, 1);
        e.state.minions[n.index()].locked = true;
        e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
        hand(&mut e, 2, &["Consume the Dead"]);
        let burned = if vampire {
            let a = laibon(&mut e, 0, 0, &[]);
            e.state.minions[a.index()].granted_title = Some(Title::Magaji);
            laibon(&mut e, 0, 1, &[]);
            let v2 = e.add_minion(SeatId(1), textless(|c| c.capacity == 2, 0), 2, false);
            e.add_minion(SeatId(1), textless(|c| c.capacity == 1, 0), 1, false);
            // The acting Methuselah's Nagaraja: no reaction of its own.
            clan_minion(&mut e, SeatId(0), "Nagaraja", 4..=11, 1, 3);
            hand(
                &mut e,
                0,
                &["Eldest Command Undeath, The", "Consume the Dead"],
            );
            pick(&mut e, play(0, 0, a));
            pick(&mut e, set_x(3));
            pick(&mut e, Choice::ChooseMinion(v2));
            pick_matching(&mut e, |o| *o == votes(a, true));
            close_polling(&mut e);
            v2
        } else {
            let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
            let moise = e.add_ally(SeatId(1), find_library("Moise Kasavubu"));
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
            pick(&mut e, bleed(a));
            pick_when(&mut e, block(moise));
            moise
        };
        pick_when(&mut e, play(0, 2, n));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[burned.index()];
        assert!(!v.in_play, "gone");
        assert_eq!(blood(&e, n), 1 + if vampire { 1 } else { 2 + 1 });
        if vampire {
            assert!(e.state.seats[1].ash_crypt.is_empty(), "removed, not burned");
            assert!(e.state.seats[0]
                .hand
                .contains(&find_library("Consume the Dead")));
        } else {
            assert!(!ash_has(&e, 1, "Moise Kasavubu"), "removed, not burned");
        }
    }
}

/// Consume the Dead [REACTION]: a reaction, so only during an action, and
/// never at a blood hunt's burn ("A burn from a blood hunt referendum opens
/// no such window", advanced rules §5.4.4; rules-qa 122.1). Seat 0's
/// Vulnerability burns seat 1's torpid vampire in the master phase: seat
/// 2's Nagaraja is never offered it. Seat 0's diablerist burned by the
/// blood hunt it passes: never offered either.
#[test]
fn consume_the_dead_not_outside_an_action_nor_at_a_blood_hunt() {
    for hunt in [false, true] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
        e.state.minions[t.index()].in_torpor = true;
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let n = clan_minion(&mut e, SeatId(2), "Nagaraja", 4..=11, 0, 3);
        e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
        hand(&mut e, 2, &["Consume the Dead"]);
        let burned = if hunt {
            e.state.seats[1].has_edge = true;
            pick(
                &mut e,
                Choice::TakeAction {
                    minion: a,
                    action: ActionKind::Diablerie,
                    target: Some(t),
                },
            );
            pick_when(&mut e, Choice::CastEdge { favor: true });
            a
        } else {
            hand(&mut e, 0, &["Vulnerability"]);
            e.at_master_phase();
            pick(&mut e, master_at(0, Target::Minion(t)));
            t
        };
        never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 2, n));
        assert!(!e.state.minions[burned.index()].in_play, "burned");
    }
}

/// Consume the Dead [REACTION]: "a minion or retainer is being burned …
/// Remove that minion or retainer from the game instead. This Nagaraja …
/// gains X+1 blood, where X is the blood or pool cost of that ally or
/// retainer." Seat 0's Nagaraja burns seat 1's Vengeful Spirit (1 blood)
/// with Consume the Dead [ACTION]; seat 2's locked Nagaraja removes it for
/// 2 blood, out of the ash heap. Declined: it burns.
#[test]
fn consume_the_dead_removes_a_burning_retainer() {
    for consume in [true, false] {
        let mut e = game(3);
        let a = clan_minion(&mut e, SeatId(0), "Nagaraja", 4..=11, 0, 4);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[z.index()].locked = true;
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let spirit = in_play(&mut e, 1, "Vengeful Spirit", Some(z));
        let n = clan_minion(&mut e, SeatId(2), "Nagaraja", 4..=11, 1, 1);
        e.state.minions[n.index()].locked = true;
        e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
        hand(&mut e, 0, &["Consume the Dead"]);
        hand(&mut e, 2, &["Consume the Dead"]);
        pick(
            &mut e,
            Choice::Play {
                card: 0,
                mode: 1,
                actor: Some(a),
                target: Target::Card(spirit),
            },
        );
        let react = play(0, 2, n);
        let req = offering(&mut e, react);
        assert_eq!(req.seat, SeatId(2));
        e.choose(if consume {
            index_of(&req, react)
        } else {
            index_of(&req, Choice::Pass)
        });
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.cards[spirit.index()].in_play, "gone");
        assert_eq!(ash_has(&e, 1, "Vengeful Spirit"), !consume);
        assert_eq!(blood(&e, n), 1 + if consume { 1 + 1 } else { 0 });
    }
}

/// Consume the Dead: "by any means except diablerie": seat 0's diablerie of
/// seat 1's torpid vampire is never answered by it. Its burn option, with
/// no ready Nagaraja only (ANK 20240610): a torpid one does not bar it.
/// Ledger: `[ANK 20240610]`.
#[test]
fn consume_the_dead_not_at_a_diablerie_and_its_burn_option() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    e.state.minions[t.index()].in_torpor = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let n = clan_minion(&mut e, SeatId(2), "Nagaraja", 4..=11, 0, 3);
    e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
    hand(&mut e, 2, &["Consume the Dead"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Diablerie,
            target: Some(t),
        },
    );
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 2, n));
    assert!(!e.state.minions[t.index()].in_play, "diablerized");
    for torpid in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let n = clan_minion(&mut e, SeatId(1), "Nagaraja", 4..=11, 0, 3);
        e.state.minions[n.index()].in_torpor = torpid;
        hand(&mut e, 1, &["Consume the Dead"]);
        e.state.seats[1].library.push(find_library("Soak"));
        pick(&mut e, Choice::Pass);
        let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
        assert_eq!(req.options.contains(&Choice::Discard(0)), torpid);
    }
}

/// The Grandest Trick: "Vampires cannot block this minion. This acting
/// vampire is treated as a mortal ally for the duration of the action …
/// will burn if reduced to 0 life … This vampire's blood represents his or
/// her life while he or she is an ally." A Kiasyd's bleed (2 blood paid,
/// 2 left): seat 1's vampire is never offered the block; its 3-strength
/// zombie blocks and the Kiasyd, 2 life, burns — not to torpor.
#[test]
fn the_grandest_trick_bleeds_as_a_mortal_ally() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), of_clan("Kiasyd", 5..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z = e.add_ally(SeatId(1), find_library("Freakish Conglomeration"));
    hand(&mut e, 0, &["Grandest Trick, The"]);
    pick(&mut e, bleed(k));
    pick_when(&mut e, play(0, 0, k));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(e.state.minions[k.index()].is_ally());
    assert!(e.state.is_mortal(k));
    assert!(!req.options.contains(&block(v)), "no vampire blocks it");
    pick(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let kv = &e.state.minions[k.index()];
    assert!(!kv.in_play, "burned as an ally");
    assert!(!kv.in_torpor);
}

/// The Grandest Trick: "Only usable on an action that doesn't cost blood or
/// require a vampire, clan or Discipline." Not on a hunt nor on Govern the
/// Unaligned [dom]; on a plain bleed unblocked, the Kiasyd is a vampire
/// again after the action, its blood kept.
#[test]
fn the_grandest_trick_on_ally_actions_only() {
    for case in ["hunt", "govern", "bleed"] {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), of_clan("Kiasyd", 5..=11, 0), 4, false);
        e.state.minions[k.index()].gained_levels = vec!["dom"];
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Grandest Trick, The", "Govern the Unaligned"]);
        match case {
            "hunt" => pick(&mut e, action(k, ActionKind::Hunt)),
            "govern" => pick(&mut e, play(1, 0, k)),
            _ => pick(&mut e, bleed(k)),
        }
        if case != "bleed" {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, k));
            continue;
        }
        pick_when(&mut e, play(0, 0, k));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let kv = &e.state.minions[k.index()];
        assert!(kv.is_vampire(), "a vampire again");
        assert_eq!(kv.blood, 2);
        assert_eq!(e.state.seats[1].pool, 9);
    }
}

/// Seat 0's `caller` plays Kine Resources Contested (hand slot 0), its
/// terms taken by the fallback, up to the polling step.
fn kine_to_polling(e: &mut Engine, caller: MinionId) {
    pick(e, play(0, 0, caller));
    loop {
        let req = request(e);
        if req.window == WindowLabel::ReferendumPolling {
            return;
        }
        e.choose(fallback(&req));
    }
}

/// Igoli's Loyalty: "Starting with your prey and going clockwise, each
/// Methuselah with any ready titled vampires may elect to cast all of his
/// or her vampires' votes and ballots in favor for 1 pool. Each time a
/// Methuselah declines, this Ishtarri burns 1 blood to add 1 pool to that
/// amount. … When results are tallied, each vampire casting votes or
/// ballots against this referendum burns 2 blood." Seat 1 (no titled
/// vampire) is skipped; seat 2 declines at 1, the Ishtarri burning a blood;
/// the acting Methuselah itself accepts at 2 (LSJ 20051026): +2 pool, its
/// primogen's votes cast in favor; seat 2's primogen, voting against,
/// burns 2 at the tally. Ledger: `[LSJ 20051026]`.
#[test]
fn igolis_loyalty_auctions_the_votes() {
    let mut e = game(3);
    let i = e.add_minion(SeatId(0), of_clan("Ishtarri", 4..=11, 0), 4, false);
    let p0 = e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.state.minions[p0.index()].granted_title = Some(Title::Primogen);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let p2 = e.add_minion(SeatId(2), textless(|c| c.capacity >= 5, 0), 4, false);
    e.state.minions[p2.index()].granted_title = Some(Title::Primogen);
    hand(&mut e, 0, &["Kine Resources Contested", "Igoli's Loyalty"]);
    kine_to_polling(&mut e, i);
    pick_when(&mut e, play(0, 0, i));
    let req = expect(&mut e, SeatId(2), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, Choice::SetX(1)]);
    pick(&mut e, Choice::Pass);
    assert_eq!(blood(&e, i), 4 - 1 - 1, "the card's blood, the raise's");
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, Choice::SetX(2)]);
    pick(&mut e, Choice::SetX(2));
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 12);
    let cast = e
        .referendum()
        .expect("referendum")
        .records
        .iter()
        .find(|r| r.source == zvtes::referendum::Source::Vampire(p0))
        .map(|r| r.favor);
    assert_eq!(cast, Some(true), "forced in favor");
    pick_when(&mut e, votes(p2, false));
    close_polling(&mut e);
    request(&mut e);
    assert_eq!(blood(&e, p2), 2);
}

/// Igoli's Loyalty: "Continue until a Methuselah accepts or this Ishtarri
/// fails to burn a blood." Its last blood burned at seat 2's decline, the
/// Ishtarri cannot pay for seat 0's: the auction ends, no pool gained.
#[test]
fn igolis_loyalty_ends_when_the_ishtarri_runs_dry() {
    let mut e = game(3);
    let i = e.add_minion(SeatId(0), of_clan("Ishtarri", 4..=11, 0), 2, false);
    let p0 = e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.state.minions[p0.index()].granted_title = Some(Title::Primogen);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let p2 = e.add_minion(SeatId(2), textless(|c| c.capacity >= 5, 0), 4, false);
    e.state.minions[p2.index()].granted_title = Some(Title::Primogen);
    hand(&mut e, 0, &["Kine Resources Contested", "Igoli's Loyalty"]);
    kine_to_polling(&mut e, i);
    pick_when(&mut e, play(0, 0, i));
    pick(&mut e, Choice::Pass);
    pick(&mut e, Choice::Pass);
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::ResolutionChoice, "ended");
    assert_eq!(blood(&e, i), 0);
    assert_eq!(e.state.seats[0].pool, 10);
}

/// King's Favor: "This Guruhi gets 3 additional votes. If this Guruhi casts
/// votes in favor, and the referendum passes by the number of votes this
/// Guruhi cast or fewer, put this card in play. While this card is in play,
/// that acting vampire may not block this Guruhi." Its 3 votes alone pass
/// it by 3: the card itself in play, and Tara, the caller, is never offered
/// to block the Guruhi's bleed, her neighbor is. With Tara's votes in
/// favor too, the margin is wider: the card goes to the ash heap.
#[test]
fn kings_favor_enters_play_on_a_narrow_pass() {
    for wide in [false, true] {
        let mut e = game(2);
        let tara = kine_caller(&mut e);
        let karen = MinionId(1);
        let g = e.add_minion(SeatId(1), of_clan("Guruhi", 4..=11, 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Kine Resources Contested"]);
        hand(&mut e, 1, &["King's Favor"]);
        kine_to_polling(&mut e, tara);
        pick_when(&mut e, play(0, 0, g));
        pick_when(&mut e, votes(g, true));
        if wide {
            pick_when(&mut e, votes(tara, true));
        }
        close_polling(&mut e);
        request(&mut e);
        if wide {
            assert!(ash_has(&e, 1, "King's Favor"));
            continue;
        }
        let card = placed_on(&e, "King's Favor", None);
        assert_eq!(e.state.cards[card.index()].chosen, vec![g, tara]);
        e.state.minions[tara.index()].locked = false;
        e.state.minions[karen.index()].locked = false;
        let req = loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break req;
            }
            e.choose(fallback(&req));
        };
        assert!(req.options.contains(&bleed(g)));
        pick(&mut e, bleed(g));
        let req = until(&mut e, SeatId(0), WindowLabel::ActionBlock);
        assert!(!req.options.contains(&block(tara)), "that acting vampire");
        assert!(req.options.contains(&block(karen)));
    }
}

/// King's Favor: "You may burn this card during any Methuselah's unlock
/// phase to move up to 3 blood from that vampire to this Guruhi." In seat
/// 0's unlock phase, 3 of the caller's 4 blood go to the Guruhi, the card
/// burned.
#[test]
fn kings_favor_moves_blood_in_an_unlock_phase() {
    let mut e = game(2);
    let tara = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), find_card("Karen Suadela"), 4, false);
    let g = e.add_minion(SeatId(1), of_clan("Guruhi", 8..=11, 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = in_play(&mut e, 1, "King's Favor", None);
    e.state.cards[card.index()].chosen = vec![g, tara];
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, use_on(card, 0, None, Target::None));
    pick(&mut e, Choice::SetX(3));
    request(&mut e);
    assert_eq!((blood(&e, g), blood(&e, tara)), (5, 1));
    assert!(ash_has(&e, 1, "King's Favor"));
}

/// Gift of Bellona [VAL] and Baseball Bat: "If played normally, it is not
/// replaced if Gift of Bellona is played after being blocked" (ANK
/// 20200517-3): its "do not replace until after this action" still holds.
/// In the combat its replacement is still held back; after the action,
/// released. Ledger: Baseball Bat's `[ANK 20200517-3]`.
#[test]
fn gift_of_bellona_keeps_a_baseball_bat_unreplaced() {
    use zvtes::state::Release;
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["val", "val"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["pot"];
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Baseball Bat", "Gift of Bellona"]);
    hand(&mut e, 1, &["Torn Signpost"]);
    stealth_for_turn(&mut e, a, -1);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 2, a));
    let mut in_combat = None;
    loop {
        let req = request(&mut e);
        if e.combat().is_some() {
            in_combat.get_or_insert(e.state.seats[0].held[Release::EndOfAction]);
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    placed_on(&e, "Baseball Bat", Some(a));
    assert_eq!(in_combat, Some(1), "the Bat's replacement held");
    assert_eq!(
        e.state.seats[0].held[Release::EndOfAction],
        0,
        "released after"
    );
}

/// Falcon's Eye [SPI]: "Only applies to one block attempt. If the action
/// later continues as if unblocked (eg. Form of Mist), another one is
/// needed." (LSJ 20030227; Eagle's Sight's test, for this card.) Ledger:
/// `[LSJ 20030227]`.
#[test]
fn falcons_eye_superior_covers_one_attempt() {
    let mut e = Engine::bare(3);
    let p = e.add_minion(SeatId(0), find_card("Crystal Lynn"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    let z = e.add_minion(SeatId(2), textless(|c| c.capacity >= 5, 0), 5, false);
    e.state.minions[z.index()].gained_levels = vec!["spi", "spi"];
    let y = e.add_minion(SeatId(2), textless(|c| c.capacity >= 5, 1), 5, false);
    e.state.minions[y.index()].gained_levels = vec!["spi", "spi"];
    hand(&mut e, 0, &["Form of Mist"]);
    hand(&mut e, 2, &["Falcon's Eye", "Falcon's Eye"]);
    pick(&mut e, bleed(p));
    pick_when(&mut e, play(0, 2, z));
    pick_when(&mut e, play(0, 1, p));
    pick_when(&mut e, Choice::ContinueAction);
    let req = until(&mut e, SeatId(2), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&block(y)));
    assert!(req.options.contains(&play(0, 2, y)));
}

/// The Grandest Trick: "will burn if reduced to 0 life": Left for Dead "can
/// be used on the vampire" (LSJ 20100527), offered as the Kiasyd would burn
/// in combat (to seat 1, not the active Methuselah); Vagabond Mystic
/// "cannot give him life": no starting life (LSJ
/// 20011214-1), its ability never offered on him. Ledger: `[LSJ 20100527]`,
/// `[LSJ 20011214-1]`.
#[test]
fn the_grandest_trick_ally_has_no_starting_life_and_may_be_left_for_dead() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), of_clan("Kiasyd", 5..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let vm = e.add_ally(SeatId(0), find_library("Vagabond Mystic"));
    let vm_card = body_of(&e, vm);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z = e.add_ally(SeatId(1), find_library("Freakish Conglomeration"));
    hand(&mut e, 0, &["Grandest Trick, The"]);
    hand(&mut e, 1, &["Left for Dead"]);
    pick(&mut e, bleed(k));
    pick_when(&mut e, play(0, 0, k));
    let mystic = |o: &Choice| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), target: Target::Minion(t), .. }
            if *c == vm_card && *t == k)
    };
    let mut offered_lfd = false;
    loop {
        let req = request(&mut e);
        assert!(!req.options.iter().any(mystic), "no starting life");
        if req.window == WindowLabel::ActionBlock && req.options.contains(&block(z)) {
            e.choose(index_of(&req, block(z)));
            continue;
        }
        if req.seat == SeatId(1)
            && req.options.iter().any(
                |o| matches!(o, Choice::Play { card: 0, target: Target::Minion(t), .. } if *t == k),
            )
        {
            offered_lfd = true;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(offered_lfd, "Left for Dead offered");
}

/// Echo of Harmonies [MEL]: "A card played 'as if from hand' can be
/// canceled 'as played'" (LSJ 20060106): the Kine Resources Contested
/// stored on Karen, played by her, is canceled by seat 1's Evil Eye [MAL]
/// (any action card, ANK 20200813-2): to the ash heap, no referendum.
/// Ledger: `[LSJ 20060106]` (Echo's, Evil Eye's).
#[test]
fn echo_of_harmonies_retained_card_is_canceled_as_played() {
    let mut e = game(2);
    kine_caller(&mut e);
    let k = MinionId(1);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["mal", "mal"];
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let kine = find_library("Kine Resources Contested");
    e.state.minions[k.index()]
        .stored
        .0
        .put(kine, zvtes::copies::CopyId::NONE);
    hand(&mut e, 1, &["Evil Eye"]);
    pick(&mut e, play(0, 0, k));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[k.index()].stored.0.is_empty());
    assert!(ash_has(&e, 0, "Kine Resources Contested"));
    assert!(e.referendum().is_none());
}

/// Echo of Harmonies [MEL]: "Cannot be used if no political action card was
/// played … (eg. cannot retrieve a National Guard Support when its text is
/// used to burn it)" (LSJ 20020911): the referendum National Guard
/// Support's ability calls has no card; [MEL] is never offered. Ledger:
/// National Guard Support's `[LSJ 20020911]`.
#[test]
fn echo_of_harmonies_needs_a_political_card() {
    let mut e = game(2);
    let tara = kine_caller(&mut e);
    let k = MinionId(1);
    e.state.minions[k.index()].gained_levels = vec!["mel", "mel"];
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ngs = in_play(&mut e, 1, "National Guard Support", None);
    hand(&mut e, 0, &["Echo of Harmonies"]);
    pick(&mut e, use_on(ngs, 0, Some(tara), Target::None));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 4, k));
}

// ---- Modifiers and reactions, Soar onward -----------------------------------

/// Soar: "Only usable on an undirected action. +1 stealth." A flying
/// vampire's hunt, met by a +1-intercept attempt, is offered it; its bleed
/// is never.
#[test]
fn soar_adds_stealth_on_an_undirected_action_only() {
    for directed in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["FLIGHT", "FLIGHT"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        in_play(&mut e, 1, "Raven Spy", Some(z));
        hand(&mut e, 0, &["Soar"]);
        if directed {
            pick(&mut e, bleed(a));
        } else {
            pick(&mut e, action(a, ActionKind::Hunt));
        }
        pick_when(&mut e, block(z));
        if directed {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
        } else {
            pick_when(&mut e, play(0, 0, a));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
        }
    }
}

/// Strange Day: "Requires a Laibon. Only usable as the action is announced.
/// Vampires cannot block this action. Only one Strange Day can be played in
/// a game." The ally may still block, the vampire never; the second copy
/// is never offered to the next Laibon's action. As announced, before the
/// regular modifiers, the action card replaced first. Ledger:
/// `[ANK 20190425]`, `[ANK 20181208]`.
#[test]
fn strange_day_bars_vampires_once_a_game() {
    let mut e = game(2);
    let a = laibon(&mut e, 0, 0, &[]);
    let b = laibon(&mut e, 0, 1, &[]);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let g = e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul"));
    hand(&mut e, 0, &["Strange Day", "Strange Day"]);
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 0, a));
    let req = offering(&mut e, block(g));
    assert_eq!(blood(&e, a), 3);
    assert!(!req.options.contains(&block(z)), "a vampire cannot block");
    e.choose(fallback(&req));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(b));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, b));
}

/// Skin of the Chameleon [tha]: "Only usable as an action is announced. If
/// this action is blocked, this vampire gets an optional maneuver or press
/// in the resulting combat." Played as the bleed is announced, before a
/// regular modifier; blocked, the acting vampire is offered a maneuver
/// before range. Ledger: `[ANK 20190425]`, `[ANK 20181208]`,
/// `[LSJ 20011204-3] [PIB 20130704]` (the [tha] mode needs Thaumaturgy
/// only).
#[test]
fn skin_of_the_chameleon_gives_a_maneuver_or_press_if_blocked() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tha"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Skin of the Chameleon"]);
    pick(&mut e, bleed(a));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ActionAsAnnounced);
    assert!(
        !req.options.contains(&play(0, 1, a)),
        "[vis] needs Visceratika"
    );
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(z));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert_eq!(blood(&e, a), 3);
    assert!(
        req.options.iter().any(|o| matches!(o, Choice::UseManeuver)),
        "{:?}",
        req.options
    );
}

/// The Missing Voice [cel]: "Requires a vampire with capacity 5 or more.
/// Vampires with capacity 4 or less get -1 intercept." Played before any
/// attempt: a capacity-4 vampire's intercept is -1, an ally's 0; never
/// offered to a capacity-4 actor. Ledger: `[LSJ 20020612]`,
/// `[LSJ 20011204-3] [PIB 20130704]` (a [cel]-only actor plays [cel]).
#[test]
fn the_missing_voice_cel_lowers_small_vampires_not_allies() {
    for cap in [5u8, 4] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity == cap, 0), cap, false);
        e.state.minions[a.index()].gained_levels = vec!["cel"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 1), 4, false);
        let g = e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul"));
        hand(&mut e, 0, &["Missing Voice, The"]);
        pick(&mut e, bleed(a));
        if cap == 4 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
            continue;
        }
        pick_when(&mut e, play(0, 0, a));
        request(&mut e);
        let act = e.action().expect("action");
        assert_eq!(act.intercept_of(&e.state, z), -1);
        assert_eq!(act.intercept_of(&e.state, g), 1, "the ghoul's own +1 only");
    }
}

/// The Missing Voice [mel]: "Allies and younger vampires get -1
/// intercept." [MEL] +1 stealth.
#[test]
fn the_missing_voice_mel_lowers_allies_and_younger() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["mel"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 1), 5, false);
    let y = e.add_minion(SeatId(1), textless(|c| c.capacity == 7, 0), 7, false);
    let g = e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul"));
    hand(&mut e, 0, &["Missing Voice, The"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    let act = e.action().expect("action");
    assert_eq!(act.intercept_of(&e.state, z), -1);
    assert_eq!(act.intercept_of(&e.state, y), 0);
    assert_eq!(act.intercept_of(&e.state, g), 0);
}

/// Uncontrolled Impulse: "+2 stealth. Not usable if any non-mandatory
/// actions have been performed this turn." The turn's first bleed, met by
/// a +1-intercept attempt, is offered it; the turn's second action never.
#[test]
fn uncontrolled_impulse_on_the_turns_first_action_only() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ishtarri", 3..=11, 0), 4, false);
    let b = e.add_minion(SeatId(0), of_clan("Ishtarri", 3..=11, 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Raven Spy", Some(z));
    hand(&mut e, 0, &["Uncontrolled Impulse", "Uncontrolled Impulse"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(z));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, b));
}

/// Uncontrolled Impulse after a mandatory action only: a printed mandatory
/// action is no "non-mandatory action" (advanced rules §3.9; rules-qa
/// 32.10): A's bleed forced by Spirit Marionette [OBE] ("must bleed …
/// before any other action") taken first, B's bleed is still offered it.
#[test]
fn uncontrolled_impulse_after_a_mandatory_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ishtarri", 3..=11, 0), 4, false);
    let b = e.add_minion(SeatId(0), of_clan("Ishtarri", 3..=11, 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Raven Spy", Some(z));
    e.state.forced_bleeds.0.push((a, SeatId(0)));
    hand(&mut e, 0, &["Uncontrolled Impulse"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].locked, "the forced bleed taken");
    assert!(req.options.contains(&bleed(b)));
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, b));
}

/// The ranges of the strikes noted, round by round (one entry a round).
fn first_strike_range(e: &mut Engine) -> Option<Range> {
    strike_ranges(e).first().copied()
}

/// The Abombwe reflex (Predator's Communion, Predator's Mastery, Unseen
/// Hibernation): "[abo] [REFLEX] Cancel a frenzy card played on this
/// vampire as it is played." Terror Frenzy [ani], played on the blocker,
/// is canceled, the reflex card and Terror Frenzy in their ash heaps,
/// Terror Frenzy's blood still paid. Ledger: `[LSJ 20051113]`,
/// `[ANK 20260216]`, `[ANK 20190104] [LSJ 19980212]` (the canceled card
/// played: it is in the ash heap).
#[test]
fn abombwe_reflexes_cancel_a_frenzy_card_on_their_vampire() {
    let mut unplayed = 0;
    for card in [
        "",
        "Predator's Communion",
        "Predator's Mastery",
        "Unseen Hibernation",
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["ani"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[z.index()].gained_levels = vec!["abo"];
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        if !card.is_empty() {
            hand(&mut e, 0, &["Terror Frenzy"]);
            hand(&mut e, 1, &[card]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if !card.is_empty() {
            pick_when(&mut e, play(0, 0, a));
            let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
            e.choose(index_of(&req, play(0, 0, z)));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if card.is_empty() {
            unplayed = blood(&e, a);
            continue;
        }
        assert!(ash_has(&e, 1, card), "{card}");
        assert!(ash_has(&e, 0, "Terror Frenzy"), "{card}");
        assert_eq!(
            blood(&e, a),
            unplayed - 1,
            "{card}: Terror Frenzy's cost paid"
        );
    }
}

/// Predator's Communion [abo]: "+1 intercept. Only usable when a vampire is
/// acting." Offered against a vampire's bleed needing intercept, never
/// against an ally's.
#[test]
fn predators_communion_gives_intercept_against_an_acting_vampire_only() {
    for ally in [false, true] {
        let mut e = game(2);
        let a = if ally {
            e.add_ally(SeatId(0), find_library("Carlton Van Wyk"))
        } else {
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false)
        };
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        stealth_for_turn(&mut e, a, 1);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[z.index()].gained_levels = vec!["abo"];
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 1, &["Predator's Communion"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if ally {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, z));
        } else {
            pick_when(&mut e, play(0, 1, z));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
        }
    }
}

/// Predator's Communion [ABO]: "Only usable by a locked vampire when a
/// vampire is acting. This reacting vampire unlocks." Locked, it unlocks;
/// the block it may then attempt is its own choice.
#[test]
fn predators_communion_superior_unlocks_a_locked_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["abo", "abo"];
    e.state.minions[z.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Predator's Communion"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 2, z));
    let req = offering(&mut e, block(z));
    assert!(!e.state.minions[z.index()].locked);
    e.choose(index_of(&req, block(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

/// Predator's Mastery [ABO]: "+1 bleed (limited) … and non-mortal allies
/// and vampires with capacity less than 5 cannot block this action."
/// Played before any attempt: a capacity-5 vampire and a mortal ally may
/// block, a capacity-4 vampire and a ghoul may not; unblocked, the bleed
/// of 2 lands. [abo] is +1 bleed (limited) only.
#[test]
fn predators_mastery_superior_bars_small_vampires_and_non_mortal_allies() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["abo", "abo"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v4 = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    let v5 = e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 0), 5, false);
    let ghoul = e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul"));
    let mortal = e.add_ally(SeatId(1), find_library("Carlton Van Wyk"));
    hand(&mut e, 0, &["Predator's Mastery"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 2, a));
    let req = offering(&mut e, block(v5));
    assert!(req.options.contains(&block(mortal)));
    assert!(!req.options.contains(&block(v4)));
    assert!(!req.options.contains(&block(ghoul)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8);
}

/// Unseen Hibernation [abo]: "Only usable during an action directed at this
/// vampire. Not usable if the acting minion is an older vampire. The action
/// fails." Bum's Rush at a capacity-6 vampire by a capacity-6 one: it
/// fails, no combat, the action reached resolution (`Outcome::Failed`).
/// The same by a capacity-9 vampire: never offered. Before any block
/// attempt only (the as-announced window passed).
#[test]
fn unseen_hibernation_fails_an_action_at_it() {
    for older in [false, true] {
        let mut e = game(2);
        let cap = if older { 9 } else { 6 };
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity == cap, 0), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity == 6, 1), 4, false);
        e.state.minions[z.index()].gained_levels = vec!["abo"];
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Bum's Rush"]);
        hand(&mut e, 1, &["Unseen Hibernation"]);
        e.notes = Some(Vec::new());
        pick(&mut e, play_at(0, 0, a, z));
        if older {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, z));
            continue;
        }
        pick_when(&mut e, play(0, 1, z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
        assert!(strike_ranges(&mut e).is_empty(), "no combat");
        assert_eq!(blood(&e, z), 3, "its 1 blood");
    }
}

/// Unseen Hibernation [ABO]: "As above, but usable by a locked vampire."
/// A locked target is offered [ABO], not [abo].
#[test]
fn unseen_hibernation_superior_by_a_locked_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity == 6, 1), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["abo", "abo"];
    e.state.minions[z.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Bum's Rush"]);
    hand(&mut e, 1, &["Unseen Hibernation"]);
    pick(&mut e, play_at(0, 0, a, z));
    let req = offering(&mut e, play(0, 2, z));
    assert!(!req.options.contains(&play(0, 1, z)));
    e.choose(index_of(&req, play(0, 2, z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
}

/// Neutral Guard [obe]: "This action, while this vampire is in combat, each
/// round is at long range (skip the determine range step for each round)."
/// Blocked, both rounds of the combat (an Apportation press continuing it)
/// are at long range with no determine range step, and Cold Aura's "set
/// the range for this round to long" is never offered to the blocker:
/// Neutral Guard sets first, and no other effect resets it (Brick by
/// Brick [VIS], Cold Aura, Giulia Giovanni Abruzzina alike). Ledger:
/// `[RTR 19970630] [ANK 20180720]`, `[PIB 20120214]`.
#[test]
fn neutral_guard_keeps_every_round_of_the_block_long() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "tha"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["nec"];
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Neutral Guard", "Apportation"]);
    hand(&mut e, 1, &["Cold Aura"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 1, a));
    pick_when(&mut e, block(z));
    let mut step = false;
    let mut pressed = false;
    loop {
        let req = request(&mut e);
        step |= req.window == WindowLabel::CombatDetermineRange;
        assert!(!req.options.contains(&play(0, 0, z)), "Cold Aura");
        if !pressed && req.options.contains(&play(0, 0, a)) {
            pressed = true;
            e.choose(index_of(&req, play(0, 0, a)));
            continue;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(pressed && !step);
    let ranges = strike_ranges(&mut e);
    assert!(
        ranges.len() >= 2 && ranges.iter().all(|r| *r == Range::Long),
        "{ranges:?}"
    );
}

/// Range setters in order (PIB 20120214: "the first to resolve has
/// priority"; Squirrel Balance resolves before Asanbonsam Ghoul, both before
/// Neutral Guard; Gang Tactics and Storm Sewers before Neutral Guard).
/// Neutral Guard [obe] and Asanbonsam Ghoul's block of a bleed: the first
/// round close. Squirrel Balance [cel] and the ghoul's block: long. Storm
/// Sewers locked and Neutral Guard: every round close. The determine
/// range step skipped each time (RTR 19970630, ANK 20180720). Ledger: `[PIB
/// 20120214]` (Neutral Guard, Squirrel Balance, Asanbonsam Ghoul, Gang
/// Tactics, Storm Sewers), `[RTR 19970630] [ANK 20180720]`.
#[test]
fn neutral_guard_and_squirrel_balance_yield_in_resolution_order() {
    // (Neutral Guard, Squirrel Balance, Storm Sewers, the ghoul blocks) →
    // the first round's range.
    for (ng, sq, ss, ghoul, want) in [
        (true, false, false, true, Range::Close),
        (false, true, false, true, Range::Long),
        (true, false, true, false, Range::Close),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
        e.state.minions[a.index()].gained_levels = vec!["obe", "cel"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let blocker = if ghoul {
            e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul"))
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false)
        };
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let sewers = ss.then(|| in_play(&mut e, 0, "Storm Sewers", None));
        let mut cards = Vec::new();
        if ng {
            cards.push("Neutral Guard");
        }
        if sq {
            cards.push("Squirrel Balance");
        }
        hand(&mut e, 0, &cards);
        e.notes = Some(Vec::new());
        pick(&mut e, bleed(a));
        if let Some(s) = sewers {
            pick(&mut e, ability(s.0, 0));
        }
        if ng {
            pick_when(&mut e, play(0, 1, a));
        }
        if sq {
            pick_when(&mut e, play(0, 0, a));
        }
        pick_when(&mut e, block(blocker));
        assert!(!range_step_until_minion_phase(&mut e));
        assert_eq!(first_strike_range(&mut e), Some(want), "{ng} {sq} {ss}");
        if ss {
            assert!(strike_ranges(&mut e).iter().all(|r| *r == Range::Close));
        }
    }
}

/// Neutral Guard [OBE]: "allies and younger vampires cannot block this
/// action." A capacity-6 actor: an older vampire is offered the block, a
/// younger one and an ally are not. [for]: "Vampires must burn 1 blood to
/// attempt to block this action", an ally free.
#[test]
fn neutral_guard_superior_bars_allies_and_younger() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity == 5, 0), 5, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity == 8, 0), 8, false);
    let g = e.add_ally(SeatId(1), find_library("Carlton Van Wyk"));
    hand(&mut e, 0, &["Neutral Guard"]);
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 2, a));
    let req = offering(&mut e, block(old));
    assert!(!req.options.contains(&block(young)));
    assert!(!req.options.contains(&block(g)));
}

/// Neutral Guard [for]: "Vampires must burn 1 blood to attempt to block
/// this action." A vampire's attempt burns its blood; an ally attempts
/// free. Ledger: `[LSJ 20011204-3] [PIB 20130704]` (a [for]-only actor
/// plays [for]).
#[test]
fn neutral_guard_for_tolls_vampires_only() {
    for ally in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
        e.state.minions[a.index()].gained_levels = vec!["for"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = if ally {
            e.add_ally(SeatId(1), find_library("Carlton Van Wyk"))
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false)
        };
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Neutral Guard"]);
        pick(&mut e, bleed(a));
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 1, a)), "[obe] needs Obeah");
        pick(&mut e, play(0, 0, a));
        let before = blood(&e, z);
        pick_when(&mut e, block(z));
        assert_eq!(blood(&e, z), before - u8::from(!ally));
    }
}

/// Pick the first use of the card in play `card` when offered.
fn pick_use(e: &mut Engine, card: CardId) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(&o) = uses(&req, card.0).first() {
            e.choose(index_of(&req, o));
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("no use of {card:?}");
}

/// Neutral Guard's long range triggers the Improvised Flamethrower: "When
/// range was set to long (e.g. Neutral Guard, Sniper Rifle) … triggers the
/// Flamethrower." The blocker's .44 Magnum hits its bearer at long range:
/// the Flamethrower burns, its bearer takes 2 aggravated (torpor). Ledger:
/// `[LSJ 20100709]`.
#[test]
fn neutral_guard_triggers_the_flamethrower() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let flame = in_play(&mut e, 0, "Improvised Flamethrower", Some(a));
    let magnum = in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Neutral Guard"]);
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 1, a));
    pick_when(&mut e, block(z));
    pick_use(&mut e, magnum);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[flame.index()].in_play, "burned");
    assert!(
        e.state.minions[a.index()].in_torpor,
        "its aggravated damage"
    );
}

/// Peacemaker [pre]: "Only usable if this vampire blocks an action directed
/// at you … Lock this blocking vampire. The action continues as if
/// unblocked. This action, all minions get -1 bleed and -1 strength." The
/// bleed of 2 continues, lands for 1, its blocker locked, no combat.
/// [obe]: the blocker unlocked after resolution.
#[test]
fn peacemaker_continues_the_bleed_at_minus_one() {
    for (mode, levels) in [(0u8, vec!["pre"]), (1, vec!["obe"])] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::Bleed(1),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[z.index()].gained_levels = levels;
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 1, &["Peacemaker"]);
        e.notes = Some(Vec::new());
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, mode, z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
        assert_eq!(e.state.seats[1].pool, 9);
        assert!(strike_ranges(&mut e).is_empty(), "no combat");
        assert_eq!(e.state.minions[z.index()].locked, mode == 0);
    }
}

/// Peacemaker [OBE]: "all minions get -2 bleed and -2 strength and cannot
/// use weapons." After the continuation, a second vampire of the blocked
/// Methuselah blocks: in that combat its .44 Magnum is never offered, and
/// the acting vampire's hand strike (1, -2) inflicts nothing.
#[test]
fn peacemaker_superior_bars_weapons_this_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["obe", "obe"];
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let magnum = in_play(&mut e, 1, ".44 Magnum", Some(y));
    hand(&mut e, 1, &["Peacemaker"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 2, z));
    pick_when(&mut e, block(y));
    loop {
        let req = request(&mut e);
        assert!(uses(&req, magnum.0).is_empty(), "no weapon");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert_eq!(blood(&e, y), 4, "a strength of 0");
}

/// Phantom Speaker [chi]: "Cancel the block resolution (the action is
/// still blocked)." No combat, the action blocked. Ledger:
/// `[LSJ 20011204-3] [PIB 20130704]` (a [chi]-only actor plays [chi]).
#[test]
fn phantom_speaker_chi_cancels_the_blocks_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["chi"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Phantom Speaker"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 0, a));
    assert!(
        !req.options.contains(&play(0, 1, a)),
        "[mel] needs Melpominee"
    );
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert!(strike_ranges(&mut e).is_empty(), "no combat");
    assert_eq!(e.state.seats[1].pool, 10);
}

/// Phantom Speaker [mel]: "Not usable if the blocking minion is an older
/// vampire. The block fails, and the action continues. The blocking minion
/// cannot attempt to block this action again." The equal-capacity blocker
/// is unlocked (not a successful block, rules-qa 121.85) and never offered
/// the block again; its Methuselah's other vampire still is; declined, the
/// bleed lands. An older blocker: never offered. [MEL]: "if this action is
/// successful, lock the blocking minion after action resolution."
#[test]
fn phantom_speaker_mel_fails_the_block_and_bars_the_blocker() {
    for (cap, superior) in [(6u8, false), (6, true), (7, false)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 6, false);
        e.state.minions[a.index()].gained_levels = if superior {
            vec!["mel", "mel"]
        } else {
            vec!["mel"]
        };
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity == cap, 1), cap, false);
        let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Phantom Speaker"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        let mode = if superior { 2 } else { 1 };
        if cap == 7 {
            never_offered_until(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                play(0, mode, a),
            );
            continue;
        }
        pick_when(&mut e, play(0, mode, a));
        let req = offering(&mut e, block(y));
        assert!(!req.options.contains(&block(z)), "barred");
        assert!(!e.state.minions[z.index()].locked, "unlocked as before");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
        assert_eq!(e.state.seats[1].pool, 9);
        assert_eq!(e.state.minions[z.index()].locked, superior);
    }
}

/// Those Who Endure Judge: "The acting vampire gets two additional votes in
/// any referendums conducted during this action." Its Laibon (capacity 5+)
/// calls Kine Resources Contested: two more votes than its own. Not
/// offered to a capacity-4 Laibon.
#[test]
fn those_who_endure_judge_adds_two_votes_to_its_referendum() {
    for cap in [6u8, 4] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(Sect::Laibon, cap..=cap, 0), cap, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(
            &mut e,
            0,
            &["Kine Resources Contested", "Those Who Endure Judge"],
        );
        pick(&mut e, play(0, 0, a));
        if cap == 4 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
            continue;
        }
        pick(&mut e, play(0, 0, a));
        loop {
            let req = request(&mut e);
            if req.window == WindowLabel::ReferendumPolling {
                break;
            }
            e.choose(fallback(&req));
        }
        let r = e.referendum().expect("referendum");
        assert_eq!(r.votes_of(&e.state, a), 2 + e.state.title_votes_of(a));
    }
}

/// Those Who Endure Judge: "Any younger vampire successfully blocking this
/// action burns 1 blood (before combat, if any). If he or she cannot, the
/// block fails and the action continues as if unblocked." A younger
/// blocker with blood burns 1 and the bleed is blocked; one with none
/// fails, unlocked, and the bleed lands. An older blocker pays nothing.
#[test]
fn those_who_endure_judge_tolls_a_younger_blocker() {
    // The blocker's blood at the end and the outcome, the card played or not.
    let run = |cap: u8, b: u8, judge: bool| {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 6..=6, 0), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), textless(|c| c.capacity == cap, 1), b, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        if judge {
            hand(&mut e, 0, &["Those Who Endure Judge"]);
        }
        pick(&mut e, bleed(a));
        if judge {
            pick(&mut e, play(0, 0, a));
        }
        pick_when(&mut e, block(z));
        if judge && b == 0 {
            let req = request(&mut e);
            assert!(!req.options.contains(&block(z)), "failed: not again alike");
            assert!(!e.state.minions[z.index()].locked);
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        (blood(&e, z), e.last_outcome)
    };
    use zvtes::actions::Outcome;
    let (plain_blood, _) = run(5, 3, false);
    assert_eq!(run(5, 3, true), (plain_blood - 1, Some(Outcome::Blocked)));
    assert_eq!(run(5, 0, true).1, Some(Outcome::Successful));
    let (older, _) = run(8, 3, false);
    assert_eq!(run(8, 3, true), (older, Some(Outcome::Blocked)));
}

/// Tourette's Voice [mel]: "Give a minion you control (even this one) +1
/// intercept." Seat 1's vampire gives its attempting minion the intercept
/// a +1-stealth bleed needs: blocked. [MEL] "Give a minion +1 intercept":
/// seat 2's vampire gives it to seat 1's attempting minion, which [mel]
/// cannot. Ledger: `[LSJ 20011204-3] [PIB 20130704]` (a [mel]-only player
/// is not offered [aus]).
#[test]
fn tourettes_voice_gives_an_attempting_minion_intercept() {
    for superior in [false, true] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        stealth_for_turn(&mut e, a, 1);
        let y = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let giver = if superior { 2 } else { 1 };
        let v = e.add_minion(SeatId(giver), plain("Ventrue", 1), 4, false);
        e.state.minions[v.index()].gained_levels = if superior {
            vec!["mel", "mel"]
        } else {
            vec!["mel"]
        };
        e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
        hand(&mut e, giver, &["Tourette's Voice"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(y));
        let req = offering(&mut e, play(0, if superior { 2 } else { 1 }, v));
        assert!(!req.options.contains(&play(0, 0, v)), "no Auspex");
        if superior {
            assert!(!req.options.contains(&play(0, 1, v)), "[mel]: its own");
        }
        e.choose(index_of(&req, play(0, if superior { 2 } else { 1 }, v)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    }
}

/// Speak with Spirits [SPI]: "Only usable by a locked vampire. This vampire
/// unlocks and attempts to block. Once this action, this vampire can burn
/// 1 blood to get +1 intercept." Locked, it unlocks to block a +1-stealth
/// bleed, burns 1 blood for the intercept, and is not offered the burn
/// again. [ani]: "Only usable during a bleed action." Ledger:
/// `[LSJ 20011204-3] [PIB 20130704]`.
#[test]
fn speak_with_spirits_superior_unlocks_to_block_and_burns_once() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    stealth_for_turn(&mut e, a, 1);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["spi", "spi"];
    e.state.minions[z.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Speak with Spirits"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 2, z));
    let burn = Choice::BurnBloodForIntercept { minion: z };
    pick_when(&mut e, burn);
    assert!(!e.state.minions[z.index()].locked);
    assert_eq!(blood(&e, z), 3);
    let req = request(&mut e);
    assert!(!req.options.contains(&burn), "once this action");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

/// Speak with Spirits [ani]: "+1 intercept. Only usable during a bleed
/// action." Offered against a +1-stealth bleed, never against a hunt.
#[test]
fn speak_with_spirits_ani_only_on_a_bleed() {
    for hunt in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        stealth_for_turn(&mut e, a, 1);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[z.index()].gained_levels = vec!["ani"];
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 1, &["Speak with Spirits"]);
        if hunt {
            pick(&mut e, action(a, ActionKind::Hunt));
        } else {
            pick(&mut e, bleed(a));
        }
        pick_when(&mut e, block(z));
        if hunt {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, z));
        } else {
            pick_when(&mut e, play(0, 0, z));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
        }
    }
}

/// Recurring Contemplation [tem]: "Choose an ally or younger vampire. The
/// chosen minion cannot block this action." A younger vampire chosen is
/// never offered the block, an older one is; [pre] offers no vampire.
/// "Only one Recurring Contemplation can be played each turn": the next
/// action this turn is not offered the second copy. Ledger:
/// `[ANK 20190425]`, `[ANK 20181208]`, `[LSJ 20011204-3] [PIB 20130704]`.
#[test]
fn recurring_contemplation_bars_the_chosen_minion_once_a_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["tem", "pre"];
    let b = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 1), 6, false);
    e.state.minions[b.index()].gained_levels = vec!["tem"];
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity == 8, 0), 8, false);
    hand(
        &mut e,
        0,
        &["Recurring Contemplation", "Recurring Contemplation"],
    );
    pick(&mut e, bleed(a));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ActionAsAnnounced);
    assert!(
        !req.options.contains(&play_at(0, 0, a, young)),
        "[pre]: an ally"
    );
    assert!(!req.options.contains(&play_at(0, 1, a, old)), "older");
    e.choose(index_of(&req, play_at(0, 1, a, young)));
    let req = offering(&mut e, block(old));
    assert!(!req.options.contains(&block(young)));
    e.choose(fallback(&req));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(b));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
}

/// Recurring Contemplation [TEM]: "As [tem] above, and burn 1 blood to lock
/// the chosen minion." The blood an effect, not the card's cost: not
/// offered to an acting vampire with no blood (ANK 20210226: "cannot be
/// played if the minion cannot afford to burn the blood"); with 1, the
/// chosen younger vampire is locked. Ledger: `[ANK 20210226]` (both rows).
#[test]
fn recurring_contemplation_superior_burns_blood_to_lock() {
    for b in [0u8, 2] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), b, false);
        e.state.minions[a.index()].gained_levels = vec!["tem", "tem"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let young = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
        e.add_minion(SeatId(1), textless(|c| c.capacity == 8, 0), 8, false);
        hand(&mut e, 0, &["Recurring Contemplation"]);
        if b == 0 {
            // Bloodless, it must hunt (taken on the way): [tem], not [TEM].
            let req = request(&mut e);
            assert_eq!(req.window, WindowLabel::ActionAsAnnounced);
            assert!(req.options.contains(&play_at(0, 1, a, young)));
            assert!(!req.options.contains(&play_at(0, 2, a, young)));
            continue;
        }
        pick(&mut e, bleed(a));
        pick(&mut e, play_at(0, 2, a, young));
        request(&mut e);
        assert!(e.state.minions[young.index()].locked);
        assert_eq!(blood(&e, a), 1);
    }
}

/// Quicksilver Contemplation [aus][tem]: "Only usable during the polling
/// step of a political action. Force a younger vampire to abstain." Seat
/// 1's capacity-8 vampire forces seat 0's capacity-4 voter to abstain; the
/// capacity-9 caller is no target. [tem] alone: "Reduce a bleed against
/// you by 2."
#[test]
fn quicksilver_contemplation_forces_a_younger_vampire_to_abstain() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), textless(|c| c.capacity == 9, 0), 6, false);
    let young = e.add_minion(SeatId(0), textless(|c| c.capacity == 4, 0), 4, false);
    e.state.minions[young.index()].granted_title = Some(Title::Primogen);
    let q = e.add_minion(SeatId(1), textless(|c| c.capacity == 8, 0), 8, false);
    e.state.minions[q.index()].gained_levels = vec!["aus", "tem"];
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    hand(&mut e, 1, &["Quicksilver Contemplation"]);
    kine_to_polling(&mut e, caller);
    let req = offering(&mut e, play_at(0, 2, q, young));
    assert!(!req.options.contains(&play_at(0, 2, q, caller)), "older");
    e.choose(index_of(&req, play_at(0, 2, q, young)));
    request(&mut e);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, young), 0, "abstaining");
}

/// Quicksilver Contemplation [AUS][TEM]: "+2 bleed, not counting against
/// the limit." Played after a limited +1 (Predator's Mastery [abo]): the
/// bleed of 4 lands. [tem] reduces a bleed against its player by 2.
/// Ledger: `[LSJ 20100218] [RTR 20180511-2]`.
#[test]
fn quicksilver_contemplation_superior_bleeds_two_beside_a_limited() {
    for reduce in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["aus", "aus", "tem", "tem", "abo"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[z.index()].gained_levels = vec!["tem"];
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(
            &mut e,
            0,
            &["Predator's Mastery", "Quicksilver Contemplation"],
        );
        if reduce {
            hand(&mut e, 1, &["Quicksilver Contemplation"]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, play(0, 1, a));
        pick_when(&mut e, play(0, 3, a));
        if reduce {
            pick_when(&mut e, play(0, 1, z));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, if reduce { 8 } else { 6 });
    }
}

/// Wanderer's Counsel: "[ACTION MODIFIER][REACTION] Only usable during a
/// referendum. Usable by a locked vampire. This Akunanse gets an additional
/// vote in this referendum." A locked Akunanse of another Methuselah plays
/// it in Kine Resources Contested's polling: one vote.
#[test]
fn wanderers_counsel_votes_from_a_locked_akunanse() {
    let mut e = game(2);
    let caller = kine_caller(&mut e);
    let w = e.add_minion(SeatId(1), of_clan("Akunanse", 3..=11, 0), 4, false);
    e.state.minions[w.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    hand(&mut e, 1, &["Wanderer's Counsel"]);
    kine_to_polling(&mut e, caller);
    pick_when(&mut e, play(0, 0, w));
    request(&mut e);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, w), 1);
}

/// Wanderer's Counsel [ACTION MODIFIER]: "Only usable when this Akunanse
/// successfully hunts. He or she gains 1 additional blood." An unblocked
/// hunt from 1 blood gains 2; blocked, it gains nothing: never offered (a
/// hunt gaining no blood is not successful). At capacity the hunt is still
/// successful, its blood draining off (rules-qa 89.1): offered, a no-op.
/// Ledger: `[LSJ 20050720-2]`.
#[test]
fn wanderers_counsel_adds_a_blood_to_a_successful_hunt() {
    for (b, blocked) in [(1u8, false), (1, true), (4, false)] {
        let mut e = game(2);
        let w = e.add_minion(SeatId(0), of_clan("Akunanse", 4..=4, 0), b, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        in_play(&mut e, 1, "Raven Spy", Some(z));
        hand(&mut e, 0, &["Wanderer's Counsel"]);
        pick(&mut e, action(w, ActionKind::Hunt));
        if blocked {
            pick_when(&mut e, block(z));
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, w));
            continue;
        }
        pick_when(&mut e, play(0, 1, w));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            blood(&e, w),
            (b + 2).min(4),
            "the hunt's 1 and the card's 1"
        );
    }
}

/// Siren's Lure [mel]: "Only usable by a ready unlocked vampire other than
/// the acting minion if a minion attempts to block. That attempt fails and
/// the blocking minion cannot attempt to block this action again. Lock this
/// vampire and the blocking minion. After the action resolution, this
/// vampire and the blocking minion enter combat." The acting Methuselah's
/// other vampire plays it (LSJ 19990425: its controller only); both lock;
/// the next attempt is offered no second copy, neither [mel] nor [MEL]
/// (LSJ 20021121: no other queue until it begins); unblocked, the bleed
/// resolves, then the combat. Ledger: `[LSJ 19990425]`, `[LSJ 20021121]`,
/// `[LSJ 20011204-3] [PIB 20130704]`.
#[test]
fn sirens_lure_fails_the_attempt_and_fights_after_resolution() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    stealth_for_turn(&mut e, a, 1);
    let s = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[s.index()].gained_levels = vec!["mel"];
    let t = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.state.minions[t.index()].gained_levels = vec!["mel", "mel"];
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["mel"];
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Siren's Lure", "Siren's Lure"]);
    hand(&mut e, 1, &["Siren's Lure"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 1, s));
    assert!(
        !req.options.contains(&play(0, 1, a)),
        "other than the acting minion"
    );
    e.choose(index_of(&req, play(0, 1, s)));
    request(&mut e);
    assert!(e.state.minions[z.index()].locked && e.state.minions[s.index()].locked);
    pick_when(&mut e, block(y));
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 1, t)) && !req.options.contains(&play(0, 2, t)));
    e.choose(fallback(&req));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9, "the bleed resolved");
    assert_eq!((blood(&e, s), blood(&e, z)), (3, 3), "the queued combat");
}

/// Siren's Lure [cel]: "Burn 2 blood to get +1 stealth." The blood an
/// effect, needed to play (ANK 20210226: "cannot be played if the minion
/// cannot afford to burn the blood"), not the card's cost: with 1 blood
/// never offered; with 2, burned for the stealth that gets past a +1
/// intercept attempt (the bleeder's +1 stealth already met). Ledger:
/// `[ANK 20210226]` (both rows).
#[test]
fn sirens_lure_cel_burns_two_blood_for_stealth() {
    for b in [1u8, 2] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), b, false);
        e.state.minions[a.index()].gained_levels = vec!["cel"];
        stealth_for_turn(&mut e, a, 1);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        in_play(&mut e, 1, "Raven Spy", Some(z));
        hand(&mut e, 0, &["Siren's Lure"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if b == 1 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
            continue;
        }
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
        assert_eq!(blood(&e, a), 0);
    }
}

/// Terra Incognita: "Requires a ready Laibon. Not usable if you control any
/// ready non-Laibon vampires. Reduce a bleed against you made by a
/// non-Laibon controlled by your predator by 1." A bleed of 2 by the
/// predator's non-Laibon lands for 1; with a ready non-Laibon of its own,
/// never offered.
#[test]
fn terra_incognita_reduces_a_non_laibon_predators_bleed() {
    for mixed in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::Bleed(1),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let l = laibon(&mut e, 1, 0, &[]);
        laibon(&mut e, 1, 1, &[]);
        if mixed {
            e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        }
        hand(&mut e, 1, &["Terra Incognita"]);
        pick(&mut e, bleed(a));
        if mixed {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, l));
            continue;
        }
        pick_when(&mut e, play(0, 0, l));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 9);
    }
}

/// Terra Incognita: "… or burn a blood to cancel a card that would retarget
/// a bleed to you played by a non-Laibon as it is played (no cost is
/// paid)." Seat 0 bleeds seat 1, whose vampire plays Deflection at seat 2:
/// seat 2's Laibon burns a blood to cancel it, Deflection's blood not paid;
/// the bleed lands on seat 1. The cancel at once (only wakes before it),
/// the canceled card still played. Ledger: `[LSJ 20061207] [LSJ
/// 20061013]`, `[LSJ 20021011]`, `[ANK 20190104] [LSJ 19980212]`.
#[test]
fn terra_incognita_cancels_a_redirect_to_you_for_a_blood() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let d = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[d.index()].gained_levels = vec!["dom"];
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let l = laibon(&mut e, 2, 0, &[]);
    hand(&mut e, 1, &["Deflection"]);
    hand(&mut e, 2, &["Terra Incognita"]);
    pick(&mut e, bleed(a));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(d),
            target: Target::Seat(SeatId(2)),
        },
    );
    let req = expect(&mut e, SeatId(2), WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, play(0, 1, l)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, l), 4, "its blood burned");
    assert_eq!(blood(&e, d), 4, "no cost paid");
    assert!(ash_has(&e, 1, "Deflection"));
    assert_eq!((e.state.seats[1].pool, e.state.seats[2].pool), (9, 10));
}

/// Transfusion [nec]: "Only usable after resolution of a successful action.
/// Add 1 blood to a ready vampire." Another Methuselah's vampire gains it.
/// [thn]: "add 1 life to a ghoul": Asanbonsam Ghoul gains a life. Ledger:
/// `[LSJ 20011204-3] [PIB 20130704]`.
#[test]
fn transfusion_adds_blood_or_life_after_a_successful_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["nec", "thn"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
    let g = e.add_ally(SeatId(1), find_library("Asanbonsam Ghoul"));
    let life = blood(&e, g);
    hand(&mut e, 0, &["Transfusion", "Transfusion"]);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, play_at(0, 0, a, z));
    assert!(
        !req.options.contains(&play_at(0, 0, a, g)),
        "[nec]: a vampire"
    );
    e.choose(index_of(&req, play_at(0, 0, a, z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 3);
    pick(&mut e, bleed(b));
    // The second copy, by the second bleeder: the [thn] ghoul.
    e.state.minions[b.index()].gained_levels = vec!["thn"];
    pick_when(&mut e, play_at(0, 2, b, g));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, g), life + 1);
}

/// Transfusion [THN]: "Only usable after combat if this vampire is still
/// ready and successfully inflicted any damage on the opposing minion
/// during that combat. Put 1 transfusion counter on the opposing minion. If
/// the opposing minion has 3 of your transfusion counters, burn them to
/// steal the opposing minion. Only one Transfusion can be played at
/// superior each turn." The blocker, already holding 2 of the acting
/// Methuselah's counters, is hit: the third, burned, and the blocker taken
/// for good (TOM 19960226-1: requirements ignored). A second [THN] that
/// turn is never offered. Dodged, its strike inflicts nothing, its Murder
/// of Crows' environmental damage not counting: never offered. Ledger:
/// `[TOM 19960226-1]`, `[ANK 20200517]`.
#[test]
fn transfusion_superior_counts_hits_and_steals_at_three() {
    // The blocker taken, and the play offered at all.
    let run = |dodge: bool| {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["thn", "thn"];
        let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.state.minions[b.index()].gained_levels = vec!["thn", "thn"];
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[z.index()]
            .transfusion
            .0
            .push((SeatId(0), 2));
        e.state.minions[z.index()]
            .transfusion
            .0
            .push((SeatId(1), 1));
        let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Transfusion", "Transfusion"]);
        if dodge {
            hand(&mut e, 1, &["Dodge"]);
            // Its retainer's damage lands through the dodge: not the
            // vampire's.
            in_play(&mut e, 0, "Murder of Crows", Some(a));
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if dodge {
            pick_when(&mut e, play(0, 0, z));
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 3, a));
            return false;
        }
        pick_when(&mut e, play(0, 3, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[z.index()].owner, SeatId(0), "stolen");
        assert_eq!(
            e.state.minions[z.index()].transfusion.0,
            vec![(SeatId(1), 1)]
        );
        pick(&mut e, bleed(b));
        pick_when(&mut e, block(y));
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 3, b));
        true
    };
    assert!(run(false));
    assert!(!run(true));
}

/// Massassi's Honor: "Only usable when this Osebo is blocked (play before
/// combat, if any). Choose a ready unlocked Osebo you control who could take
/// that action, put this card in play, and move any other cards this Osebo
/// played this action from your ash heap to this card. After this action,
/// the chosen Osebo must attempt the same action … He or she may play cards
/// from this card as if from your hand. Burn this card at the end of that
/// action …" Osebo A's bleed with Conditioning is blocked; the card goes in
/// play (not a stand-in, out of the ash heap) on Osebo B, Conditioning on
/// it. After the combat, B's bleed is the only action (taken on the way);
/// B plays Conditioning from the card, not replaced (LSJ 20081126: the hand
/// keeps its size), the bleed of 3 lands, and the card burns as it ends.
/// Ledger: `[LSJ 20081126]`.
#[test]
fn massassis_honor_hands_a_blocked_bleed_to_another_osebo() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Osebo", 3..=11, 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    let b = e.add_minion(SeatId(0), of_clan("Osebo", 3..=11, 1), 6, false);
    e.state.minions[b.index()].gained_levels = vec!["dom"];
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Conditioning", "Massassi's Honor", "Dodge"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play_at(0, 0, a, b));
    request(&mut e);
    let honor = placed_on(&e, "Massassi's Honor", Some(b));
    assert!(!e.state.cards[honor.index()].effect_only);
    assert!(!ash_has(&e, 0, "Massassi's Honor"));
    assert!(!ash_has(&e, 0, "Conditioning"));
    assert_eq!(
        e.state.cards[honor.index()].stored.to_cards(),
        vec![find_library("Conditioning")]
    );
    // After the combat: B's bleed, the only action.
    loop {
        let req = request(&mut e);
        if e.action().is_some_and(|x| x.acting == b) {
            break;
        }
        assert_ne!(req.window, WindowLabel::MinionPhase, "{:?}", req.options);
        e.choose(fallback(&req));
    }
    let hand_size = e.state.seats[0].hand.len();
    let req = offering(&mut e, play(hand_size as u8, 0, b));
    e.choose(index_of(&req, play(hand_size as u8, 0, b)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].hand.len(), hand_size, "not replaced");
    assert_eq!(e.state.seats[1].pool, 7);
    assert!(ash_has(&e, 0, "Massassi's Honor"), "burned at its end");
    assert!(e.state.handed_off.0.is_empty());
}

/// Massassi's Honor: "Choose a ready unlocked Osebo you control who could
/// take that action" — as it is chosen (rules-digest D1): A's Govern the
/// Unaligned [dom] blocked, Osebo B with Dominate may be chosen, Osebo C
/// without it may not.
#[test]
fn massassis_honor_chooses_an_osebo_who_could_take_the_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Osebo", 3..=11, 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    let b = e.add_minion(SeatId(0), of_clan("Osebo", 3..=11, 1), 6, false);
    e.state.minions[b.index()].gained_levels = vec!["dom"];
    let c = e.add_minion(SeatId(0), of_clan("Osebo", 3..=11, 2), 6, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Govern the Unaligned", "Massassi's Honor"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play_at(0, 0, a, b));
    assert!(!req.options.contains(&play_at(0, 0, a, c)), "no Dominate");
}

/// Massassi's Honor: "Burn this card … if the action is not possible." The
/// chosen Osebo locked by the time the action is owed: the card and the
/// card on it burn, the minion phase free. Ledger: `[LSJ 20021128]` (its
/// cards burned with it).
#[test]
fn massassis_honor_burns_when_the_action_is_not_possible() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Osebo", 3..=11, 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    let b = e.add_minion(SeatId(0), of_clan("Osebo", 3..=11, 1), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Conditioning", "Massassi's Honor"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play_at(0, 0, a, b));
    request(&mut e);
    e.state.minions[b.index()].locked = true;
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&Choice::Pass));
    assert!(ash_has(&e, 0, "Massassi's Honor"));
    assert!(ash_has(&e, 0, "Conditioning"), "burned with it");
    assert!(e.state.handed_off.0.is_empty());
}

/// Winged Second: "Only usable when another minion you control enters
/// combat with a minion without flight. Lock this minion. In that combat,
/// this minion may make a hand or melee weapon strike … on the opposing
/// minion during normal strike resolution (as if at close range). Dodge
/// will avoid this strike …" Seat 1's flier, out of the combat its blocker
/// enters, locks; its own strike request follows the blocker's, the view
/// keeping the blocker as the combatant: a hand strike, beside the
/// blocker's, the acting vampire taking 2 (the blocker's 1, the Second's
/// 1); a Dodge avoids both. "The 'Second' is not in combat, none of their
/// or their retainers' combat-related abilities apply": its Murder of Crows
/// adds nothing. Against a flier, never offered. Ledger: `[LSJ 20051126-2]
/// [ANK 20221021]`.
#[test]
fn winged_second_strikes_beside_its_combatant() {
    for (dodge, flier) in [(false, false), (true, false), (false, true)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 6, false);
        if flier {
            e.state.minions[a.index()].gained_levels = vec!["FLIGHT", "FLIGHT"];
        }
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.minions[w.index()].gained_levels = vec!["FLIGHT", "FLIGHT"];
        in_play(&mut e, 1, "Murder of Crows", Some(w));
        hand(&mut e, 1, &["Winged Second"]);
        if dodge {
            hand(&mut e, 0, &["Dodge"]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if flier {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, w));
            continue;
        }
        pick_when(&mut e, play(0, 0, w));
        if dodge {
            pick_when(&mut e, play(0, 0, a));
        }
        let req = second_strike_request(&mut e);
        assert_eq!(req.seat, SeatId(1));
        let c = e.combat().expect("combat");
        assert_eq!(c.combatant_side(1).minion, z, "the blocker still fights");
        assert_eq!(c.second_side(1).map(|s| s.minion), Some(w));
        pick(&mut e, Choice::HandStrike);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[w.index()].locked);
        assert_eq!(blood(&e, a), if dodge { 6 } else { 4 });
    }
}

/// Pass (or hand strike) until a Winged Second's own strike request.
fn second_strike_request(e: &mut Engine) -> zvtes::decision::DecisionRequest {
    for _ in 0..200 {
        let req = request(e);
        if req.window == WindowLabel::CombatStrike
            && e.combat().is_some_and(|c| c.second_swap.is_some())
        {
            return req;
        }
        e.choose(fallback(&req));
    }
    panic!("no second's strike request");
}

/// Seat 0's capacity-6 vampire at 6 blood bleeds, seat 1's blocks, seat
/// 1's flier plays Winged Second (hand slot 0) beside it: (bleeder,
/// blocker, second).
fn seconded_block(e: &mut Engine) -> (MinionId, MinionId, MinionId) {
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[w.index()].gained_levels = vec!["FLIGHT", "FLIGHT"];
    pick(e, bleed(a));
    pick_when(e, block(z));
    pick_when(e, play(0, 0, w));
    (a, z, w)
}

/// Winged Second: "this minion may make a hand or melee weapon strike":
/// offered no strike first; declined, the acting vampire takes the
/// blocker's 1 only.
#[test]
fn winged_second_may_decline_its_strike() {
    let mut e = game(2);
    hand(&mut e, 1, &["Winged Second"]);
    let (a, z, _) = seconded_block(&mut e);
    let req = second_strike_request(&mut e);
    assert_eq!(req.options[..2], [Choice::Pass, Choice::HandStrike]);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 5, "the blocker's strike only");
    assert_eq!(blood(&e, z), 3, "the bleeder's strike on the blocker");
}

/// Winged Second: "a hand or melee weapon strike (with or without a strike
/// card)": the melee weapon it bears (Baseball Bat, strength+1), or a
/// strike card its player plays for it (Lucky Blow's hand strike at +1),
/// each 2 beside the blocker's 1; its gun (.44 Magnum, ranged) and a
/// Dodge never offered to it.
#[test]
fn winged_second_strikes_with_a_melee_weapon_or_a_strike_card() {
    for card in [false, true] {
        let mut e = game(2);
        hand(&mut e, 1, &["Winged Second", "Lucky Blow", "Dodge"]);
        let w0 = MinionId(e.state.minions.len() as u32 + 3);
        let bat = in_play(&mut e, 1, "Baseball Bat", Some(w0));
        let gun = in_play(&mut e, 1, ".44 Magnum", Some(w0));
        let (a, _, w) = seconded_block(&mut e);
        assert_eq!(w, w0);
        let req = second_strike_request(&mut e);
        let named = |o: &Choice, name: &str| match *o {
            Choice::Play { card, actor, .. } => {
                actor == Some(w) && e.playable_card(SeatId(1), card) == find_library(name)
            }
            _ => false,
        };
        assert!(
            !req.options.iter().any(|o| named(o, "Dodge")),
            "{:?}",
            req.options
        );
        let by = |src: CardId| {
            req.options.iter().position(|o| {
                matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == src)
            })
        };
        assert!(by(gun).is_none(), "a ranged weapon");
        let i = if card {
            req.options
                .iter()
                .position(|o| {
                    named(o, "Lucky Blow")
                        && matches!(
                            o,
                            Choice::Play {
                                target: Target::None,
                                ..
                            }
                        )
                })
                .expect("Lucky Blow's hand strike")
        } else {
            by(bat).expect("the bat's strike")
        };
        e.choose(i);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), 3, "the blocker's 1, the second's 2");
    }
}

/// Winged Second: "This minion may be the target of effects that inflict
/// damage or steal blood as a retainer could be": the bleeder's Target
/// Retainer aims its hand strike at the blocker's second, which loses 1
/// blood; the blocker takes none.
#[test]
fn target_retainer_aims_at_a_winged_second() {
    let mut e = game(2);
    hand(&mut e, 0, &["Target Retainer"]);
    hand(&mut e, 1, &["Winged Second"]);
    let (a, z, w) = seconded_block(&mut e);
    let aim = Choice::Play {
        card: 0,
        mode: 2,
        actor: Some(a),
        target: Target::Minion(w),
    };
    pick_when(&mut e, aim);
    let req = second_strike_request(&mut e);
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, w), 3, "the second struck");
    assert_eq!(blood(&e, z), 4, "the blocker untouched");
    assert_eq!(blood(&e, a), 5);
}

/// Neutral Guard [obe] against the in-combat setters (PIB 20120214: "If
/// another effect was already setting that round's range (e.g. Neutral
/// Guard), that effect has priority"): the Giovanni blocker is offered
/// neither Cold Aura [nec], Brick by Brick [VIS]'s range nor Giulia
/// Giovanni Abruzzina's lock before range; every strike long. Ledger:
/// `[PIB 20120214]` (Cold Aura, Brick by Brick, Giulia Giovanni
/// Abruzzina).
#[test]
fn neutral_guard_sets_before_the_in_combat_setters() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Giovanni", 3..=11, 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["nec", "vis", "vis"];
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let giulia = e.add_ally(SeatId(1), find_library("Giulia Giovanni Abruzzina"));
    let gc = body_of(&e, giulia);
    hand(&mut e, 0, &["Neutral Guard"]);
    hand(&mut e, 1, &["Cold Aura", "Brick by Brick"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 1, a));
    pick_when(&mut e, block(z));
    loop {
        let req = request(&mut e);
        assert!(
            !req.options.iter().any(|o| matches!(o,
                Choice::Play { card: 0, actor: Some(m), .. }
                | Choice::Play { card: 1, mode: 2 | 3, actor: Some(m), .. } if *m == z)),
            "{:?}",
            req.options
        );
        assert!(uses(&req, gc.0).is_empty(), "Giulia");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    let ranges = strike_ranges(&mut e);
    assert!(!ranges.is_empty() && ranges.iter().all(|r| *r == Range::Long));
}

/// Charge of the Buffalo [spi]'s first round, set as its combat begins,
/// before Neutral Guard's (the first to resolve, PIB 20120214): the first
/// round close. There Immortal Grapple [POT] presses on, "that round is at
/// close range": Neutral Guard, already setting every round, has priority,
/// and the second round is long. Ledger: `[PIB 20120214]` (Charge of the
/// Buffalo, Immortal Grapple).
#[test]
fn charge_of_the_buffalo_sets_before_neutral_guard() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "spi", "pot", "pot"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(
        &mut e,
        0,
        &["Charge of the Buffalo", "Neutral Guard", "Immortal Grapple"],
    );
    e.notes = Some(Vec::new());
    pick(&mut e, play_at(0, 1, a, z));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::UsePress);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let ranges = strike_ranges(&mut e);
    assert_eq!(ranges, [Range::Close, Range::Long]);
}

/// Ananasi Vampirephile's press-step setting of the next round yields to
/// Neutral Guard [obe], already setting every round of the acting
/// vampire's combats (PIB 20120214): blocking it, she presses, no range
/// asked, the second round long all the same. Ledger: `[PIB 20120214]`
/// (Ananasi Vampirephile).
#[test]
fn ananasi_next_range_yields_to_neutral_guard() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["obe"];
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    let n = e.add_ally(SeatId(1), find_library("Ananasi Vampirephile"));
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let card = body_of(&e, n);
    hand(&mut e, 0, &["Neutral Guard"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 1, a));
    pick_when(&mut e, block(n));
    let press = card_ability(&mut e, card, 1);
    pick(&mut e, press);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, Choice::SetX(0));
    assert_eq!(blood(&e, n), 4 - 1, "the life burned");
    assert_eq!(strike_ranges(&mut e), [Range::Long, Range::Long]);
}

/// Wanderer's Counsel on a card hunt: Restricted Vitae's steal ("Vampires
/// who must hunt can steal 1 blood from a younger vampire as a Ⓓ hunt
/// action", Week of Nightmares' Ravnos hunt) by a bloodless Akunanse: the
/// card's additional blood also comes from the target (RTR 20030519: "the
/// hunt bonus applies to non-standard hunt actions, blood comes from the
/// target of the hunt"). Ledger: `[RTR 20030519] [RTR 20180511]`, `[RTR
/// 20180511] [RTR 20030519]`.
#[test]
fn wanderers_counsel_adds_to_a_card_hunt_from_its_target() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), of_clan("Akunanse", 6..=11, 0), 0, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity == 3, 0), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let rv = in_play(&mut e, 1, "Restricted Vitae", None);
    hand(&mut e, 0, &["Wanderer's Counsel"]);
    pick_when(&mut e, use_on(rv, 0, Some(h), Target::Minion(young)));
    pick_when(&mut e, play(0, 1, h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, h), blood(&e, young)), (2, 1));
}

/// Squirrel Balance [spi]: "Minions without flight [FLIGHT] or Spiritus
/// [spi] get -1 intercept when attempting to block this action." Played
/// before any attempt: a plain vampire's intercept is -1, a Spiritus
/// vampire's and a flier's 0; [SPI] -2. Ledger: `[LSJ 20020612]`,
/// `[LSJ 20011204-3] [PIB 20130704]` (a [spi]-only actor plays [spi]).
#[test]
fn squirrel_balance_spi_lowers_minions_without_flight_or_spiritus() {
    for (levels, n) in [(vec!["spi"], -1), (vec!["spi", "spi"], -2)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = levels;
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let s = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.minions[s.index()].gained_levels = vec!["spi"];
        let f = e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
        e.state.minions[f.index()].gained_levels = vec!["FLIGHT", "FLIGHT"];
        hand(&mut e, 0, &["Squirrel Balance"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, play(0, (-n) as u8, a));
        request(&mut e);
        let act = e.action().expect("action");
        assert_eq!(act.intercept_of(&e.state, z), n);
        assert_eq!(act.intercept_of(&e.state, s), 0);
        assert_eq!(act.intercept_of(&e.state, f), 0);
    }
}

// ---- Earlier rounds' cards and ranged strikes -------------------------------

/// Khobar Towers, Al-Khubar on Reanimated Corpse, an X-cost ally: "If used
/// on an ally with a cost of X, X pool are gained. The value of X is the
/// one set when the ally was recruited" (ANK 20170716): recruited at X = 3,
/// removed from the game for 3 pool. Ledger: `[ANK 20170716] [RTR 20020501]`
/// (Khobar Towers).
#[test]
fn khobar_towers_gains_the_x_an_ally_was_recruited_at() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 6, 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["thn"];
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 2), 4, false);
    let kt = in_play(&mut e, 0, "Khobar Towers, Al-Khubar", None);
    hand(&mut e, 0, &["Reanimated Corpse"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::SetX(3));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let corpse = (0..e.state.minions.len())
        .map(|i| MinionId(i as u32))
        .find(|&m| e.state.minions[m.index()].is_ally())
        .expect("the corpse recruited");
    assert_eq!(blood(&e, a), 3, "X = 3 paid");
    pick_when(&mut e, use_on(kt, 0, None, Target::Minion(corpse)));
    assert!(!e.state.minions[corpse.index()].in_play, "removed");
    assert_eq!(pool(&e)[0], 13, "X = 3 gained");
}

/// Their Master's Voice: "Unlock a gargoyle creature ally or a ready slave
/// Gargoyle you control." Rock Cat ("Gargoyle creature with 4 life"),
/// locked, is unlocked by a locked Tremere's Their Master's Voice; a locked
/// non-Gargoyle ally is no target.
#[test]
fn their_masters_voice_unlocks_a_gargoyle_creature_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 0), 4, false);
    e.add_minion(SeatId(0), textless(|c| c.capacity >= 4, 1), 4, false);
    let t = clan_minion(&mut e, SeatId(1), "Tremere", 3..=8, 0, 4);
    let cat = e.add_ally(SeatId(1), find_library("Rock Cat"));
    let other = e.add_ally(SeatId(1), find_library("Thadius Zho"));
    for m in [t, cat, other] {
        e.state.minions[m.index()].locked = true;
    }
    hand(&mut e, 1, &["Their Master's Voice"]);
    pick(&mut e, bleed(a));
    let want = play_at(0, 0, t, cat);
    let req = offering(&mut e, want);
    assert!(!req.options.contains(&play_at(0, 0, t, other)));
    pick(&mut e, want);
    request(&mut e);
    assert!(!e.state.minions[cat.index()].locked);
    assert!(e.state.minions[other.index()].locked);
}

/// Winged Second, a target "as a retainer could be": at long range the
/// bleeder's .44 Magnum (2R damage) is aimed at the blocker's second, as
/// at a retainer (rulebook "Retainers and Combat"): the second loses 2
/// blood, the blocker none; the second's hand strike, "as if at close
/// range", still reaches the bleeder.
#[test]
fn a_ranged_strike_is_aimed_at_a_winged_second() {
    let mut e = game(2);
    hand(&mut e, 1, &["Winged Second"]);
    let a0 = MinionId(e.state.minions.len() as u32);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(a0));
    let (a, z, w) = seconded_block(&mut e);
    pick_when(&mut e, use_on(gun, 1, None, Target::None));
    pick_matching(&mut e, |o| {
        matches!(o, Choice::UseAbility {
            source: AbilitySource::Card(c),
            target: Target::Minion(t),
            ..
        } if *c == gun && *t == w)
    });
    let req = second_strike_request(&mut e);
    e.choose(index_of(&req, Choice::HandStrike));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, w), 2, "the second struck");
    assert_eq!(blood(&e, z), 4, "the blocker untouched");
    assert_eq!(
        blood(&e, a),
        5,
        "the second's 1; the blocker's none at long range"
    );
}
