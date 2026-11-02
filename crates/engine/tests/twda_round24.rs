//! Round 24 (`ADDED_BATCHES`, slice 16aj): Lost Kindred, Twilight
//! Rebellion and Sword of Caine completed (owner, 2026-10-30: set by set,
//! promos skipped). Readings in rules-qa Batch 107. Windows with a single
//! option are elided, so the scripts only stop where a real decision exists.

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay, PrintedCtx};

mod common;
use common::*;

// ---- Helpers --------------------------------------------------------------

/// `n` seats at seat 0's minion phase, turn 4, 10 pool each.
fn game(n: usize) -> Engine {
    let mut e = Engine::bare(n);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    e
}

/// The `nth` untitled vampire of `sect` with no printed-text definition,
/// capacity 4 or more, holding every Discipline of `discs` (case as
/// printed: upper case superior).
fn of_sect(sect: Sect, discs: &[&str], nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.sect == Some(sect)
                && c.title == Title::None
                && c.capacity >= 4
                && !c.imbued
                && zvtes::cards::vampire(*i as u16).is_none()
                && discs.iter().all(|d| {
                    c.disciplines.iter().any(|x| {
                        if d.chars().all(|ch| ch.is_uppercase()) {
                            x == d
                        } else {
                            x.eq_ignore_ascii_case(d)
                        }
                    })
                })
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no {sect:?} vampire with {discs:?}"))
}

/// An Anarch with `discs`.
fn anarch(discs: &[&str], nth: usize) -> u16 {
    of_sect(Sect::Anarch, discs, nth)
}

/// A vampire's own ability used during an action (no actor named).
fn own(m: MinionId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// A card already in play for `seat`, on `host` if given: its id.
fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), SeatId(seat), host, 0));
    CardId(e.state.cards.len() as u32 - 1)
}

/// "Put this card on": the card itself in play, no stand-in, its ash heap
/// without it.
fn placed(e: &Engine, c: CardId) {
    let card = &e.state.cards[c.index()];
    assert!(card.in_play && !card.effect_only, "the card itself");
    assert!(
        !e.state.seats.iter().any(|s| s.ash.contains(&card.card)),
        "not in an ash heap"
    );
}

/// The card `name` in play on `host`.
fn on(e: &Engine, name: &str, host: MinionId) -> Option<CardId> {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == card && c.host == Some(host))
        .map(|i| CardId(i as u32))
}

/// Hand card `card` played in `mode` by `actor` at `target`.
fn play_at(card: u8, mode: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor,
        target,
    }
}

fn ash_has(e: &Engine, seat: u8, name: &str) -> bool {
    e.state.seats[seat as usize]
        .ash
        .contains(&find_library(name))
}

/// The `nth` plain vampire of `clan` (any sect, untitled, no text).
fn clan(clan: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.clan == clan
                && c.title == Title::None
                && c.capacity >= 4
                && !c.imbued
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no plain {clan}"))
}

fn vs(o: MinionId) -> PrintedCtx {
    PrintedCtx {
        opponent: Some(o),
        ..Default::default()
    }
}

/// The `nth` plain vampire holding `disc` at superior (case as printed).
fn superior(disc: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.title == Title::None
                && c.capacity >= 4
                && !c.imbued
                && zvtes::cards::vampire(*i as u16).is_none()
                && c.disciplines.contains(&disc)
        })
        .nth(nth)
        .unwrap_or_else(|| panic!("no vampire with {disc}"))
        .0 as u16
}

/// A plain vampire holding every Discipline of `discs` (any level).
fn with_discs(discs: &[&str]) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.title == Title::None
                && c.capacity >= 4
                && !c.imbued
                && zvtes::cards::vampire(i as u16).is_none()
                && discs
                    .iter()
                    .all(|d| c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d)))
        })
        .unwrap_or_else(|| panic!("no vampire with {discs:?}")) as u16
}

/// A card already in play with its first mode's layers (no play).
fn in_play_with_layers(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    let id = in_play(e, seat, name, host);
    for effect in zvtes::cards::lookup(find_library(name))
        .expect("definition")
        .modes[0]
        .effects
    {
        if let zvtes::cards::Effect::Layer(modifier) = effect {
            e.state.layers.push(zvtes::cards::Layer {
                scope: zvtes::cards::Scope::InPlay,
                modifier: *modifier,
                source: id,
            });
        }
    }
    id
}

// ---- Vampires -------------------------------------------------------------

// Khurshid (+2 strength), Marguerite Foccart's base card (+1 intercept vs.
// political actions; the advanced one's text not), Dirk (titled blockers
// +1 intercept), Ondine (+1 stealth bleeding with the Edge, +1 strength),
// Stephen Bateson (+1 bleed against a Methuselah with a ready Tremere, +1
// stealth).
#[test]
fn printed_strength_intercept_stealth_and_bleed() {
    use zvtes::decision::ActionKind;
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Khurshid"), 4, false);
    let base = CRYPT
        .iter()
        .position(|c| c.name == "Marguerite Foccart" && !c.adv)
        .unwrap() as u16;
    let m = e.add_minion(SeatId(0), base, 4, false);
    let dirk = e.add_minion(SeatId(0), find_card("Dirk"), 1, false);
    let o = e.add_minion(
        SeatId(0),
        find_card("Ondine \"Boudicca\" Sinclair"),
        4,
        false,
    );
    let st = e.add_minion(SeatId(0), find_card("Stephen Bateson"), 4, false);
    let titled = e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    e.state.minions[titled.index()].granted_title = Some(Title::Votes(1));
    let plain = e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    let p = |e: &Engine, m, s, c| e.state.printed_mod(m, s, c);
    let kind = |k| PrintedCtx {
        kind: Some(k),
        ..Default::default()
    };
    assert_eq!(p(&e, k, Stat::Strength, Default::default()), 2);
    assert_eq!(p(&e, m, Stat::Intercept, kind(ActionKind::Political)), 1);
    assert_eq!(p(&e, m, Stat::Intercept, kind(ActionKind::Bleed)), 0);
    assert_eq!(
        p(&e, m, Stat::Strength, vs(titled)),
        0,
        "not the advanced text"
    );
    assert_eq!(p(&e, dirk, Stat::InterceptAgainst, vs(titled)), 1);
    assert_eq!(p(&e, dirk, Stat::InterceptAgainst, vs(plain)), 0);
    assert_eq!(p(&e, o, Stat::Strength, Default::default()), 1);
    assert_eq!(
        p(&e, o, Stat::Stealth, kind(ActionKind::Bleed)),
        0,
        "no Edge"
    );
    e.state.seats[0].has_edge = true;
    assert_eq!(p(&e, o, Stat::Stealth, kind(ActionKind::Bleed)), 1);
    assert_eq!(p(&e, o, Stat::Stealth, kind(ActionKind::Hunt)), 0);
    let at = |s| PrintedCtx {
        kind: Some(ActionKind::Bleed),
        target: Some(SeatId(s)),
        ..Default::default()
    };
    assert_eq!(p(&e, st, Stat::Stealth, Default::default()), 1);
    assert_eq!(p(&e, st, Stat::Bleed, at(1)), 0, "no ready Tremere");
    e.add_minion(SeatId(1), clan("Tremere", 0), 4, false);
    assert_eq!(p(&e, st, Stat::Bleed, at(1)), 1);
}

/// Andy: an older vampire opposing him gets one optional press each
/// combat; Andy himself none.
#[test]
fn andy_gives_an_older_opponent_a_press() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let andy = e.add_minion(SeatId(1), find_card("Andy"), 1, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(andy));
    request(&mut e);
    let c = e.combat().expect("combat");
    assert_eq!((c.sides[0].minion, c.sides[1].minion), (a, andy));
    assert_eq!(
        (c.sides[0].combat_presses, c.sides[1].combat_presses),
        (1, 0)
    );
}

/// Nizzam al-Latif: his hand strike's damage is aggravated: 1 to a
/// vampire sends it to torpor, unhealable.
#[test]
fn nizzam_al_latif_strikes_aggravated_with_his_hands() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nizzam al-Latif"), 9, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    pick(&mut e, bleed(n));
    pick_when(&mut e, block(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[b.index()].in_torpor);
}

/// Toby: during any Methuselah's bleed by an anarch, he burns a blood for
/// +1 bleed, once that action; not for a non-anarch.
#[test]
fn toby_burns_a_blood_for_an_acting_anarch() {
    for anarch_acts in [true, false] {
        let mut e = game(2);
        let a = if anarch_acts {
            e.add_minion(SeatId(0), anarch(&[], 0), 4, false)
        } else {
            e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false)
        };
        e.add_minion(SeatId(0), clan("Ventrue", 1), 4, false);
        let toby = e.add_minion(SeatId(1), find_card("Toby"), 4, false);
        e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
        let boost = Choice::UseAbility {
            source: AbilitySource::Vampire(toby),
            ability: 0,
            actor: None,
            target: Target::None,
        };
        pick(&mut e, bleed(a));
        let mut used = 0;
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            if let Some(i) = req.options.iter().position(|o| *o == boost) {
                used += 1;
                e.choose(i);
            } else {
                e.choose(fallback(&req));
            }
        }
        if anarch_acts {
            assert_eq!(used, 1, "once");
            assert_eq!(e.state.seats[1].pool, 10 - 2);
            assert_eq!(e.state.minions[toby.index()].blood, 3);
        } else {
            assert_eq!((used, e.state.seats[1].pool), (0, 9));
        }
    }
}

/// Sheela Na Gig: during her Methuselah's bleed by a Tremere antitribu,
/// she locks for +1 bleed.
#[test]
fn sheela_na_gig_locks_for_a_tremere_antitribu_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Tremere antitribu", 0), 4, false);
    let s = e.add_minion(SeatId(0), find_card("Sheela Na Gig"), 2, false);
    // A third, unlocked: the minion phase not elided into the next turn.
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    pick(&mut e, bleed(a));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(s),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[s.index()].locked);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Castiel: in combat, before range, he equips with a melee weapon from
/// his Methuselah's hand (a gun not offered); one held already, not.
#[test]
fn castiel_equips_a_melee_weapon_from_hand_before_range() {
    for armed in [false, true] {
        let mut e = game(2);
        let c = e.add_minion(SeatId(0), find_card("Castiel"), 6, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        if armed {
            in_play(&mut e, 0, "Femur of Toomler", Some(c));
        }
        hand(&mut e, 0, &[".44 Magnum", "Crusader Sword, The"]);
        pick(&mut e, bleed(c));
        pick_when(&mut e, block(z));
        if armed {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, own(c, 0));
            continue;
        }
        until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
        // The one melee weapon in hand: named at once (elided).
        pick(&mut e, own(c, 0));
        request(&mut e);
        let sword = find_library("Crusader Sword, The");
        assert!(e
            .state
            .cards
            .iter()
            .any(|x| x.card == sword && x.in_play && x.host == Some(c)));
    }
}

/// Michaela: once each combat, 1 blood to cancel the opposing minion's
/// grapple card as it is played, its cost not paid.
#[test]
fn michaela_cancels_a_grapple_as_played() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    let m = e.add_minion(SeatId(1), find_card("Michaela"), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Immortal Grapple"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(m));
    pick_when(&mut e, play(0, 0, a));
    let before = e.state.minions[m.index()].blood;
    pick_when(&mut e, own(m, 0));
    request(&mut e);
    let grapple = find_library("Immortal Grapple");
    let plays = noted_plays(&mut e);
    assert!(
        plays.iter().any(|p| p.card == grapple && p.canceled),
        "{plays:?}"
    );
    assert_eq!(
        e.state.minions[m.index()].blood,
        before - 1 - 1,
        "the blood, the hand strike"
    );
}

/// Jean Lisle: Crossbreaker's text: a blood for aggravated hand strikes
/// that round, the blocker sent to torpor.
#[test]
fn jean_lisle_buys_aggravated_hand_strikes() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jean Lisle"), 6, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    pick(&mut e, bleed(j));
    pick_when(&mut e, block(z));
    pick_when(&mut e, own(j, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor, "aggravated");
}

/// Ash Harrison: his 1R aggravated strike only at long range: not offered
/// at close range; maneuvered to long (Fake Out), offered, its aggravated
/// damage sending the blocker to torpor.
#[test]
fn ash_harrison_strikes_only_at_long_range() {
    for long in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Ash Harrison"), 6, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        if long {
            hand(&mut e, 0, &["Fake Out"]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if long {
            pick_when(&mut e, play(0, 0, a));
            pick_when(&mut e, own(a, 0));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(e.state.minions[z.index()].in_torpor, "1R aggravated");
        } else {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, own(a, 0));
        }
    }
}

/// Preston Varrick, Zubeida, Fairuza: bleed by what the bled Methuselah
/// controls; Lin Jun in torpor: her predator's Blood Sorcery vampires +1.
#[test]
fn printed_bleed_by_the_bled_methuselahs_minions() {
    use zvtes::decision::ActionKind;
    let mut e = game(3);
    let p = e.add_minion(SeatId(0), find_card("Preston Varrick"), 4, false);
    let z = e.add_minion(SeatId(0), find_card("Zubeida"), 4, false);
    let f = e.add_minion(SeatId(0), find_card("Fairuza"), 4, false);
    let at = PrintedCtx {
        kind: Some(ActionKind::Bleed),
        target: Some(SeatId(1)),
        ..Default::default()
    };
    let b = |e: &Engine, m| e.state.printed_mod(m, Stat::Bleed, at);
    assert_eq!((b(&e, p), b(&e, z), b(&e, f)), (0, 1, 0));
    let mid = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    assert_eq!(b(&e, f), -1, "a ready minion");
    e.state.minions[mid.index()].blood = 4;
    e.add_ally(SeatId(1), find_library("Saatet-ta"));
    assert_eq!(b(&e, p), 1, "a ready ally");
    let big = CRYPT
        .iter()
        .position(|c| c.capacity == 9 && !c.imbued)
        .unwrap() as u16;
    e.add_minion(SeatId(1), big, 9, false);
    assert_eq!(b(&e, z), 0, "a ready vampire above 6");
    // Lin Jun: seat 1's predator is seat 0.
    let t = e.add_minion(SeatId(0), vampire_with("tha", 0), 4, false);
    let o = e.add_minion(SeatId(0), clan("Ventrue", 1), 4, false);
    let jun = e.add_minion(SeatId(1), find_card("Lin Jun"), 4, false);
    let bleed0 = PrintedCtx::default();
    assert_eq!(e.state.printed_mod(t, Stat::Bleed, bleed0), 0, "ready");
    e.state.minions[jun.index()].in_torpor = true;
    assert_eq!(e.state.printed_mod(t, Stat::Bleed, bleed0), 1);
    assert_eq!(e.state.printed_mod(o, Stat::Bleed, bleed0), 0, "no [tha]");
}

/// Carmen: non-Camarilla vampires of her Methuselah +1 stealth on equip
/// actions; Angelique, ready: her Methuselah's Harbingers' [nec] cards 1
/// fewer blood; Chalcedony: a card requiring a Gargoyle.
#[test]
fn printed_stealth_and_costs_for_others() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Carmen"), 4, false);
    let s = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, &[], 0), 4, false);
    let k = e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    let eq = PrintedCtx {
        equip_action: true,
        ..Default::default()
    };
    let st = |e: &Engine, m| e.state.printed_mod(m, Stat::Stealth, eq);
    assert_eq!((st(&e, c), st(&e, s), st(&e, k)), (1, 1, 0));
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Angelique"), 4, false);
    let h = e.add_minion(SeatId(0), find_card("Khurshid"), 4, false);
    let card = zvtes::library::LIBRARY
        .iter()
        .enumerate()
        .position(|(i, l)| l.disciplines == ["nec"] && zvtes::cards::lookup(i as u16).is_some())
        .unwrap() as u16;
    let off = |e: &Engine, m| {
        e.state.printed_mod(
            m,
            Stat::BloodCostOff,
            PrintedCtx {
                card: Some((card, 0)),
                ..Default::default()
            },
        )
    };
    assert_eq!((off(&e, a), off(&e, h)), (1, 1));
    e.state.minions[a.index()].in_torpor = true;
    assert_eq!(off(&e, h), 0, "while she is ready");
    let mut e = game(2);
    let ch = e.add_minion(SeatId(0), find_card("Chalcedony"), 4, false);
    let g = zvtes::library::LIBRARY
        .iter()
        .position(|l| l.clans.contains(&"Gargoyle"))
        .unwrap() as u16;
    let ctx = PrintedCtx {
        card: Some((g, 0)),
        ..Default::default()
    };
    assert_eq!(e.state.printed_mod(ch, Stat::CostOff, ctx), 1);
}

/// Tarautas: while ready, Blood Doll costs every Methuselah 1 more pool.
#[test]
fn tarautas_taxes_blood_dolls() {
    let doll = find_library("Blood Doll");
    let cost = |e: &Engine| zvtes::cards::pool_cost(&e.state, SeatId(1), doll, Target::None);
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tarautas"), 4, false);
    let base = {
        e.state.minions[t.index()].in_torpor = true;
        cost(&e)
    };
    e.state.minions[t.index()].in_torpor = false;
    assert_eq!(cost(&e), base + 1);
}

/// Hecate: after her successful bleed, the top of the bled library burned.
#[test]
fn hecate_burns_the_top_of_the_bled_library() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hecate"), 6, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let vest = find_library("Kevlar Vest");
    // A full hand: the library not drawn up.
    hand(&mut e, 1, &["Conditioning"; 7]);
    e.state.seats[1].library.extend([vest; 2]);
    pick(&mut e, bleed(h));
    pick_when(&mut e, own(h, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ash.contains(&vest));
    assert_eq!(e.state.seats[1].library.len(), 1);
}

/// Fairuza: 1 blood burned from her prey's minion as a directed action.
#[test]
fn fairuza_burns_a_blood_from_a_preys_minion() {
    let mut e = game(2);
    let f = e.add_minion(SeatId(0), find_card("Fairuza"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(f),
            ability: 0,
            actor: Some(f),
            target: Target::Minion(z),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[z.index()].blood, 3);
}

/// Shaggydog: a press in the first round only.
#[test]
fn shaggydog_presses_in_the_first_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Shaggydog"), 7, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, Choice::UsePress);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[z.index()].blood, 4 - 2, "two rounds");
}

// ---- Library --------------------------------------------------------------

/// Vermin Channel: a +3 stealth bleed, its 1 blood paid.
#[test]
fn vermin_channel_bleeds_at_plus_three_stealth() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Nosferatu antitribu", 0), 4, false);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Vermin Channel"]);
    pick(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(e.action().expect("the bleed").stealth(&e.state), 3);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9);
    assert_eq!(e.state.minions[a.index()].blood, 3);
}

/// Zip Line: on the acting minion (the card itself); in its next action,
/// an attempt with +1 intercept needing stealth, burned for +1.
#[test]
fn zip_line_goes_on_the_actor_and_burns_for_stealth() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let iago = e.add_minion(SeatId(1), find_card("Iago Castille"), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Zip Line"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let zip = on(&e, "Zip Line", a).expect("on the actor");
    placed(&e, zip);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(iago));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(zip),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    request(&mut e);
    assert!(!e.state.cards[zip.index()].in_play, "burned");
}

/// Failsafe: in play; at 2 pool, a master phase action burns it for 4
/// pool; at 3, not offered.
#[test]
fn failsafe_gains_four_pool_below_three() {
    for pool in [2, 3] {
        let mut e = game(2);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        let f = in_play(&mut e, 0, "Failsafe", None);
        e.state.seats[0].pool = pool;
        let use_it = Choice::UseAbility {
            source: AbilitySource::Card(f),
            ability: 0,
            actor: None,
            target: Target::None,
        };
        if pool == 3 {
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, use_it);
            continue;
        }
        until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        pick(&mut e, use_it);
        request(&mut e);
        assert_eq!(e.state.seats[0].pool, 6);
        assert!(!e.state.cards[f.index()].in_play, "burned");
    }
}

/// Drop Point Network: with a ready Black Hand vampire and 7 cards in the
/// ash heap, those removed from the game, a minion card searched to hand.
#[test]
fn drop_point_network_searches_a_minion_card() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Nails"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let vest = find_library("Kevlar Vest");
    for _ in 0..7 {
        e.state.seats[0].ash.push(vest);
    }
    let ghoul = find_library("Saatet-ta");
    e.state.seats[0]
        .library
        .extend([find_library("Conditioning"), ghoul]);
    hand(&mut e, 0, &["Drop Point Network"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, play_at(0, 0, None, Target::None));
    for _ in 0..20 {
        let req = request(&mut e);
        if let Some(o) = req
            .options
            .iter()
            .position(|o| *o == Choice::FindCard(ghoul) || *o == Choice::FindCard(vest))
        {
            e.choose(o);
            continue;
        }
        break;
    }
    assert!(!e.state.seats[0].ash.contains(&vest), "seven removed");
    assert!(e.state.seats[0].hand.contains(&ghoul));
}

/// Touch of Pain [POT]: after a successful bleed against its player, 2
/// damage on the acting minion; a second copy not playable that action, by
/// any of its vampires.
#[test]
fn touch_of_pain_damages_the_acting_minion_once_an_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let r = e.add_minion(SeatId(1), superior("POT", 0), 4, false);
    // A second vampire with Potence: the limit is the card's, not the
    // minion's.
    e.add_minion(SeatId(1), superior("POT", 1), 4, false);
    hand(&mut e, 1, &["Touch of Pain", "Touch of Pain"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, r));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { .. })
    });
    assert_eq!(e.state.minions[a.index()].blood, 4 - 2);
}

/// Sanguinary Wind: this round its vampire's hand strike lands through the
/// opposing Dodge, which is still played; without it, dodged.
// Rulings: [LSJ 20030902-2] [LSJ 20060808-1].
#[test]
fn sanguinary_wind_makes_strikes_undodgeable() {
    for wind in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("cel", 0), 4, false);
        e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        if wind {
            hand(&mut e, 0, &["Sanguinary Wind"]);
        }
        hand(&mut e, 1, &["Dodge"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if wind {
            pick_when(&mut e, play(0, 0, a));
        }
        pick_when(&mut e, play(0, 0, z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let hit = e.state.minions[z.index()].blood < 4;
        assert_eq!(hit, wind, "wind: {wind}");
    }
}

/// Relentless Reaper [THN]: both ready as combat would end, a blood burned
/// for a new round.
#[test]
fn relentless_reaper_starts_a_new_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), superior("THN", 0), 6, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Relentless Reaper"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[z.index()].blood,
        4 - 2,
        "a hand strike each round"
    );
}

/// Vengeful Spirit: employed, its employer bleeds at +1.
#[test]
fn vengeful_spirit_gives_its_employer_a_bleed() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Khurshid"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Vengeful Spirit"]);
    pick(&mut e, play(0, 0, h));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(on(&e, "Vengeful Spirit", h).is_some(), "employed");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Blade of Bellona: its strike, strength+1: 2 damage from a 1-strength
/// bearer.
#[test]
fn blade_of_bellona_strikes_at_strength_plus_one() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Salubri antitribu", 0), 6, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let blade = in_play(&mut e, 0, "Blade of Bellona", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(blade),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[z.index()].blood, 4 - 2);
}

/// Blessed Blade [val][REFLEX]: the opposing minion's grapple canceled as
/// it is played.
#[test]
fn blessed_blade_cancels_a_grapple() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Lailah"), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Immortal Grapple"]);
    hand(&mut e, 1, &["Blessed Blade"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, b));
    request(&mut e);
    let grapple = find_library("Immortal Grapple");
    let plays = noted_plays(&mut e);
    assert!(
        plays.iter().any(|p| p.card == grapple && p.canceled),
        "{plays:?}"
    );
}

/// Notorious Brutality: a capacity 8+ Sabbat's action; a non-infernal
/// vampire below capacity 6 may not block, an infernal one (Midget) may.
#[test]
fn notorious_brutality_bars_small_non_infernal_blockers() {
    let big = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.sect == Some(Sect::Sabbat)
                && c.capacity >= 8
                && !c.imbued
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .unwrap() as u16;
    let small = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.capacity < 6 && !c.infernal && !c.imbued && zvtes::cards::vampire(i as u16).is_none()
        })
        .unwrap() as u16;
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), big, 8, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let s = e.add_minion(SeatId(1), small, 3, false);
    let m = e.add_minion(SeatId(1), find_card("Midget"), 3, false);
    hand(&mut e, 0, &["Notorious Brutality"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(m)), "{:?}", req.options);
    assert!(!req.options.contains(&block(s)));
}

/// Empowering the Puppet King [dom]: another unlocked vampire of the
/// bleeder's Methuselah gives +1 bleed.
#[test]
fn empowering_the_puppet_king_adds_a_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Empowering the Puppet King"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Shattering: [chi] directed, a card of another's hand discarded; [dem]
/// an event burned, undirected though another Methuselah controls it;
/// [pro] 2 unpreventable damage on a ready minion. Requires an Anarch.
#[test]
fn shattering_discards_burns_an_event_and_damages() {
    // [dem]
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["dem"]), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let ev = in_play(&mut e, 1, "Blood Weakens", None);
    hand(&mut e, 0, &["Shattering"]);
    pick(&mut e, play_at(0, 1, Some(a), Target::Card(ev)));
    request(&mut e);
    let act = e.action().expect("the action");
    assert_eq!(act.directed_seat(&e.state), None);
    assert_eq!(act.stealth(&e.state), 2);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[ev.index()].in_play, "burned");
    // [pro]
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["pro"]), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Shattering"]);
    pick(&mut e, play_at(0, 2, Some(a), Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[z.index()].blood, 4 - 2);
    // [chi]
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["chi"]), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Shattering"]);
    hand(&mut e, 1, &["Kevlar Vest", "Conditioning"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Seat(SeatId(1))));
    let vest = find_library("Kevlar Vest");
    pick_when(&mut e, Choice::FindCard(vest));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ash.contains(&vest));
}

/// Truth in Ink: a Black Hand vampire reduces a non-Black Hand vampire's
/// bleed by 2; a Black Hand bleeder's, not offered.
#[test]
fn truth_in_ink_reduces_a_non_black_hand_bleed() {
    for hand_bleeds in [false, true] {
        let mut e = game(2);
        let a = if hand_bleeds {
            e.add_minion(SeatId(0), find_card("Nails"), 4, false)
        } else {
            e.add_minion(SeatId(0), clan("Brujah", 0), 4, false)
        };
        e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
        let r = e.add_minion(SeatId(1), find_card("Gatjil Munyarryun"), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Govern the Unaligned"]);
        hand(&mut e, 1, &["Truth in Ink"]);
        e.state.minions[a.index()].granted_title = None;
        pick(&mut e, bleed(a));
        if hand_bleeds {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, r));
            assert_eq!(e.state.seats[1].pool, 9);
        } else {
            pick_when(&mut e, play(0, 0, r));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[1].pool, 10, "bleed 1, 2 off");
        }
    }
}

/// The ready Seraph of the crypt at `nth`.
fn seraph(nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(_, c)| c.seraph && !c.imbued)
        .nth(nth)
        .unwrap()
        .0 as u16
}

/// Seraph's Second: after a Black Hand non-Seraph's action, a Seraph of its
/// Methuselah unlocks it; a second copy not by the same Seraph that turn.
#[test]
fn seraphs_second_unlocks_the_acting_black_hand_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Nails"), 4, false);
    let s = e.add_minion(SeatId(0), seraph(0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Seraph's Second", "Seraph's Second"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: zvtes::decision::ActionKind::Hunt,
            target: None,
        },
    );
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, s));
}

/// Census Taker: on a ready Black Hand vampire; it locks to give another
/// Sabbat's bleed +1, not its own.
#[test]
fn census_taker_locks_for_another_sabbats_bleed() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Nails"), 4, false);
    let a = e.add_minion(SeatId(0), find_card("Gatjil Munyarryun"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Census Taker"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, play_at(0, 0, None, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = on(&e, "Census Taker", t).expect("on the Black Hand vampire");
    placed(&e, c);
    pick(&mut e, bleed(a));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(c),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].locked);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Census Taker: its bearer gives 2 votes to another Sabbat vampire of any
/// Methuselah in a political action's polling.
#[test]
fn census_taker_gives_two_votes() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Nails"), 4, false);
    let caller = e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let g = e.add_minion(SeatId(0), find_card("Gatjil Munyarryun"), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let c = in_play(&mut e, 0, "Census Taker", Some(t));
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, caller));
    to_referendum(&mut e);
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(c),
            ability: 1,
            actor: None,
            target: Target::Minion(g),
        },
    );
    request(&mut e);
    let rf = e.referendum().expect("referendum");
    assert!(rf.bonus.contains(&(g, 2)), "{:?}", rf.bonus);
}

/// Cadet: on a non-Black Hand Sabbat vampire in its player's uncontrolled
/// region, in play there; influenced out, it is on the vampire, who is
/// Black Hand while Sabbat (LSJ 20070322).
// Rulings: [LSJ 20070322].
#[test]
fn cadet_makes_a_controlled_sabbat_vampire_black_hand() {
    let v = of_sect(Sect::Sabbat, &[], 0);
    let mut e = game(2);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: v,
            copy: Default::default(),
            counters: CRYPT[v as usize].capacity,
            banished: None,
        });
    hand(&mut e, 0, &["Cadet"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick_when(&mut e, play_at(0, 0, None, Target::Uncontrolled(0)));
    request(&mut e);
    let c = (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|&c| e.state.cards[c.index()].card == find_library("Cadet"))
        .expect("in play");
    placed(&e, c);
    assert_eq!(e.state.cards[c.index()].on_slot, Some(v));
    pick_when(&mut e, Choice::InfluenceOut { slot: 0 });
    request(&mut e);
    let m = e.state.cards[c.index()].host.expect("on the vampire");
    assert!(e.state.is_black_hand(m));
    e.state.minions[m.index()].sect_override = Some(Sect::Anarch);
    assert!(!e.state.is_black_hand(m), "Black Hand while Sabbat");
}

/// Mustajib: on a younger Sabbat with a successful action this turn,
/// unlocked, Black Hand with the card; not offered on one who has not
/// acted. Not Black Hand if not Sabbat.
// Rulings: [LSJ 20070322].
#[test]
fn mustajib_goes_on_a_younger_sabbat_who_acted() {
    let older = CRYPT
        .iter()
        .position(|c| c.name == "Nizzam al-Latif")
        .unwrap() as u16;
    for acted in [true, false] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), older, 9, false);
        let y = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, &[], 0), 4, false);
        e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Mustajib"]);
        let put = play_at(0, 0, Some(b), Target::Minion(y));
        if !acted {
            assert!(!request(&mut e).options.contains(&put));
            continue;
        }
        pick(&mut e, bleed(y));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[y.index()].locked);
        pick(&mut e, put);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let c = on(&e, "Mustajib", y).expect("on the vampire");
        placed(&e, c);
        assert!(!e.state.minions[y.index()].locked, "unlocked");
        assert!(e.state.is_black_hand(y));
        e.state.minions[y.index()].sect_override = Some(Sect::Anarch);
        assert!(!e.state.is_black_hand(y), "Black Hand while Sabbat");
    }
}

/// Mambo Jeanne: 2 environmental damage on her opponent each close round,
/// beside her hand strike.
#[test]
fn mambo_jeanne_damages_her_opponent() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mambo Jeanne"), 9, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[z.index()].blood, 4 - 3);
}

/// Nuriel: +1 damage with a melee weapon; an additional strike in the
/// first round.
#[test]
fn nuriel_strikes_twice_and_harder_with_melee() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nuriel"), 8, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let blade = in_play(&mut e, 0, "Blade of Bellona", Some(n));
    let strike = Choice::UseAbility {
        source: AbilitySource::Card(blade),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick(&mut e, bleed(n));
    pick_when(&mut e, block(z));
    pick_when(&mut e, strike);
    pick_when(&mut e, own(n, 0));
    pick_when(&mut e, strike);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let cap = e.state.capacity_of(z).unwrap().min(6);
    assert_eq!(e.state.minions[z.index()].blood, cap - 3 - 3);
}

/// Sean Rycek: 2 votes for a lock or a blood, once a referendum: one used,
/// the other not offered.
#[test]
fn sean_rycek_gains_votes_once_a_referendum() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let s = e.add_minion(SeatId(1), find_card("Sean Rycek"), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, caller));
    to_referendum(&mut e);
    pick_when(&mut e, own(s, 0));
    let req = request(&mut e);
    assert!(e.state.minions[s.index()].locked);
    assert!(!req.options.contains(&own(s, 1)), "{:?}", req.options);
    assert!(e.referendum().expect("referendum").bonus.contains(&(s, 2)));
}

/// Sean Rycek burns a boon in play as a directed action.
#[test]
fn sean_rycek_burns_a_boon() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Sean Rycek"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let boon = in_play(&mut e, 1, "Life Boon", None);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(s),
            ability: 2,
            actor: Some(s),
            target: Target::Card(boon),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[boon.index()].in_play);
}

/// Reverend Adams (advanced): an older blocker does not lock; the master
/// phase look needs a Gehenna card.
#[test]
fn reverend_adams_advanced_lets_older_blockers_stay_unlocked() {
    let adv = CRYPT
        .iter()
        .position(|c| c.name == "Reverend Adams" && c.adv)
        .unwrap() as u16;
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), adv, 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let big = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.capacity == 9 && !c.imbued && zvtes::cards::vampire(i as u16).is_none()
        })
        .unwrap() as u16;
    let z = e.add_minion(SeatId(1), big, 9, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    assert_eq!(e.state.printed_mod(r, Stat::BlockerDoesNotLock, vs(z)), 1);
}

/// Laecanus: frenzy cards not playable on him.
#[test]
fn laecanus_bars_frenzy_cards_and_presses_to_end() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Laecanus"), 5, false);
    assert_eq!(
        e.state
            .printed_mod(l, Stat::FrenzyImmune, Default::default()),
        1
    );
}

/// Hagar Stone: +1 hand size while ready; cards played during their
/// controller's turn wait until the vampire locks, the held ones counting
/// against the hand size (ANK 20180318).
#[test]
fn hagar_stone_holds_replacements_until_locked() {
    let mut e = game(2);
    let hagar = e.add_minion(SeatId(0), find_card("Hagar Stone"), 6, false);
    let other = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
    e.state.minions[other.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), 8, "only Hagar's readiness");
    e.state.minions[other.index()].in_torpor = false;
    let stock = find_library("Conditioning");
    e.state.seats[0].library.extend([stock, stock, stock]);
    hand(&mut e, 0, &["Blood Doll"]);
    fill_hand(&mut e, 0);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let size = e.state.seats[0].hand.len();
    pick(&mut e, play_at(0, 0, None, Target::Minion(other)));
    request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), size - 1, "not replaced");
    assert_eq!(e.state.hand_load(SeatId(0)), size, "held counts");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].hand.len(), size - 1);
    pick(&mut e, action(hagar, ActionKind::Hunt));
    request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), size, "replaced as he locks");
}

/// Hagar Stone, unlocked as the turn ends: held cards are replaced then.
#[test]
fn hagar_stone_releases_at_the_end_of_the_turn() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Hagar Stone"), 6, false);
    let other = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let stock = find_library("Conditioning");
    e.state.seats[0].library.extend([stock, stock, stock]);
    hand(&mut e, 0, &["Blood Doll"]);
    fill_hand(&mut e, 0);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, play_at(0, 0, None, Target::Minion(other)));
    request(&mut e);
    assert_eq!(e.state.seats[0].held.total(), 1, "held");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.seats[0].held.total(),
        0,
        "replaced as the turn ended"
    );
}

/// Jack Drake: a younger vampire influenced out may burn a blood to become
/// Anarch; one as old or older is not asked.
#[test]
fn jack_drake_offers_younger_entrants_the_anarch_movement() {
    let cam = (0..CRYPT.len() as u16)
        .find(|&c| {
            CRYPT[c as usize].sect == Some(Sect::Camarilla)
                && CRYPT[c as usize].capacity == 4
                && CRYPT[c as usize].title == Title::None
        })
        .expect("a young Camarilla vampire");
    let old = (0..CRYPT.len() as u16)
        .find(|&c| {
            CRYPT[c as usize].sect == Some(Sect::Camarilla) && CRYPT[c as usize].capacity == 10
        })
        .expect("an old Camarilla vampire");
    for (card, asked) in [(cam, true), (old, false)] {
        let mut e = Engine::bare(3);
        e.add_minion(SeatId(0), find_card("Jack Drake"), 8, false);
        e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.at_master_phase();
        let slot = e.state.seats[0].uncontrolled.len() as u8;
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card,
                copy: Default::default(),
                counters: CRYPT[card as usize].capacity,
                banished: None,
            });
        pick_when(&mut e, Choice::InfluenceOut { slot });
        let v = MinionId(e.state.minions.len() as u32 - 1);
        let turn = Choice::ChooseMinion(v);
        if asked {
            pick(&mut e, turn);
            assert_eq!(e.state.sect_of(v), Some(Sect::Anarch));
            assert_eq!(e.state.minions[v.index()].blood, 3);
        } else {
            assert_eq!(e.state.sect_of(v), Some(Sect::Camarilla));
            assert!(!request(&mut e).options.contains(&turn));
        }
    }
}

/// Calvin Cleaver: adds a blood to an anarch; an anarch of any Methuselah
/// adds a blood to him, once in each player's turn.
// Rulings: [ANK 20180719-4].
#[test]
fn calvin_cleaver_trades_blood_with_anarchs() {
    let mut e = game(2);
    let calvin = e.add_minion(SeatId(1), find_card("Calvin Cleaver"), 1, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 2, false);
    let b = e.add_minion(SeatId(0), anarch(&[], 1), 2, false);
    let give = |m| Choice::UseAbility {
        source: AbilitySource::Vampire(calvin),
        ability: 1,
        actor: Some(m),
        target: Target::Minion(calvin),
    };
    pick(&mut e, give(a));
    let mut guard = 0;
    while blood(&e, calvin) == 1 && guard < 20 {
        guard += 1;
        let req = request(&mut e);
        if req.seat != SeatId(0) && req.options.contains(&Choice::Pass) {
            pick(&mut e, Choice::Pass);
        } else if req.options.contains(&give(b)) {
            break;
        } else {
            let i = fallback(&req);
            e.choose(i);
        }
    }
    assert_eq!(blood(&e, calvin), 2);
    assert!(
        !request(&mut e).options.contains(&give(b)),
        "once each turn"
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let own = Choice::UseAbility {
        source: AbilitySource::Vampire(calvin),
        ability: 0,
        actor: None,
        target: Target::Minion(a),
    };
    let req = request(&mut e);
    assert!(
        req.options.iter().any(|o| matches!(o,
            Choice::UseAbility { source: AbilitySource::Vampire(c), ability: 0, target: Target::Minion(t), .. }
                if *c == calvin && *t == a)),
        "{own:?} on offer: {:?}",
        req.options
    );
}

/// Topaz: equipping from hand, an equipment of the prey's hand instead, the
/// original discarded; only the original paid; the prey's hand refilled.
// Rulings: [LSJ 20080702-4].
#[test]
fn topaz_takes_an_equipment_from_the_preys_hand() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Topaz"), 7, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kevlar Vest"]);
    hand(&mut e, 1, &["Sport Bike"]);
    let bike = find_library("Sport Bike");
    pick(&mut e, play(0, 0, t));
    pick_when(&mut e, Choice::FindCard(bike));
    request(&mut e);
    assert!(on(&e, "Sport Bike", t).is_some_and(|c| !e.state.cards[c.index()].effect_only));
    assert!(on(&e, "Kevlar Vest", t).is_none());
    assert!(ash_has(&e, 0, "Kevlar Vest"), "the original discarded");
    assert!(!e.state.seats[1].hand.contains(&bike));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 10 - 1, "the vest alone");
}

/// Marie-Pierre: as a combat of another Samedi ends, she locks and fights
/// the opposing minion, not acting (ANK 20180929).
#[test]
fn marie_pierre_enters_combat_after_a_samedi_fights() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), clan("Samedi", 0), 4, false);
    let mp = e.add_minion(SeatId(0), find_card("Marie-Pierre"), 4, false);
    let f = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(f));
    pick_when(&mut e, Choice::ChooseMinion(mp));
    let c = e.combat().expect("the new combat");
    let fighters = [c.sides[0].minion, c.sides[1].minion];
    assert!(fighters.contains(&mp) && fighters.contains(&f));
    assert!(e.state.minions[mp.index()].locked);
}

/// Baseball Bat: not replaced until after the action; the successful action
/// unlocks the bearer at the end of the turn.
#[test]
fn baseball_bat_waits_and_unlocks_at_the_end_of_the_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Baseball Bat"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let bat = on(&e, "Baseball Bat", a).expect("on the actor");
    placed(&e, bat);
    assert!(
        !e.state.minions[a.index()].locked,
        "unlocked as the turn ended"
    );
}

/// Exclusion Principle: each Methuselah gains 1 pool per ready Independent
/// or Anarch vampire they control; torpor and other sects count nothing.
#[test]
fn exclusion_principle_pays_for_ready_independents_and_anarchs() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    e.state.minions[caller.index()].granted_title = Some(Title::Votes(1));
    let torpid = e.add_minion(SeatId(0), anarch(&[], 1), 4, false);
    e.state.minions[torpid.index()].in_torpor = true;
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), anarch(&[], 2), 4, false);
    hand(&mut e, 0, &["Exclusion Principle"]);
    let before = pool(&e);
    pick(&mut e, play(0, 0, caller));
    pick_when(&mut e, votes(caller, true));
    close_polling(&mut e);
    let after = pool(&e);
    assert_eq!(after[0] - before[0], 1);
    assert_eq!(after[1] - before[1], 1);
}

// ---- Extra draws ----------------------------------------------------------

/// The extra draw on offer now.
fn extra_draw(e: &mut Engine) -> Option<zvtes::draws::DrawSource> {
    request(e);
    match e.interrupt() {
        Some(zvtes::draws::Interrupt::MayDrawExtra { source, .. }) => Some(source),
        _ => None,
    }
}

/// Visit from the Capuchin with `n` counters on seat 0's side.
fn capuchin(e: &mut Engine, n: u8) -> CardId {
    let c = in_play_with_layers(e, 0, "Visit from the Capuchin", None);
    e.state.cards[c.index()].counters = n;
    c
}

/// "Each time you draw to replace": Capuchin takes the replacement, so no
/// draw and no extra one; offered again once it is gone.
#[test]
fn learjet_brings_nothing_for_a_replacement_capuchin_takes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), superior("dom", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    in_play_with_layers(&mut e, 0, "Learjet", Some(a));
    let c = capuchin(&mut e, 1);
    hand(&mut e, 0, &["Govern the Unaligned", "Govern the Unaligned"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, play(0, 0, a));
    assert_eq!(extra_draw(&mut e), None, "taken by the last counter");
    assert!(!e.state.cards[c.index()].in_play, "burned");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, a));
    assert_eq!(
        extra_draw(&mut e),
        Some(zvtes::draws::DrawSource::Learjet),
        "a draw again"
    );
}

/// "When Ira plays": on the play, Capuchin or not, naming Ira.
#[test]
fn ira_rivers_draws_on_the_play_under_capuchin() {
    let mut e = game(2);
    let ira = e.add_minion(SeatId(0), find_card("Ira Rivers"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    capuchin(&mut e, 3);
    hand(&mut e, 0, &["Revelations"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, play_at(0, 0, Some(ira), Target::Seat(SeatId(1))));
    assert_eq!(
        extra_draw(&mut e),
        Some(zvtes::draws::DrawSource::Vampire(ira))
    );
}

/// Ira with Learjet on an Auspex card, Duality with Learjet on a Vicissitude
/// one: each source its own draw, no text capping them.
#[test]
fn each_source_brings_its_own_extra_draw() {
    let mut e = game(2);
    let ira = e.add_minion(SeatId(0), find_card("Ira Rivers"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    in_play_with_layers(&mut e, 0, "Learjet", Some(ira));
    hand(&mut e, 0, &["Revelations"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, play_at(0, 0, Some(ira), Target::Seat(SeatId(1))));
    let first = extra_draw(&mut e).expect("Ira's");
    pick(&mut e, Choice::DrawLibrary);
    let second = extra_draw(&mut e).expect("Learjet's");
    assert_ne!(first, second);

    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Duality"), 5, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 6, false);
    in_play_with_layers(&mut e, 0, "Learjet", Some(d));
    hand(&mut e, 0, &["Horrid Form"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, bleed(d));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, d));
    assert!(extra_draw(&mut e).is_some());
    pick(&mut e, Choice::DrawLibrary);
    assert!(extra_draw(&mut e).is_some(), "the second source");
}

/// Anonymous Freight: an equip action at +3 stealth, the named equipment on
/// the vampire, its cost paid; Children of Stone [tha]: 1 less.
#[test]
fn anonymous_freight_and_children_of_stone_equip_from_hand() {
    for (name, paid) in [("Anonymous Freight", 1), ("Children of Stone", 0)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_discs(&["tha"]), 4, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 0, &[name, "Kevlar Vest"]);
        pick(&mut e, play(0, 0, a));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(on(&e, "Kevlar Vest", a).is_some(), "{name}");
        assert_eq!(pool(&e)[0], 10 - paid, "{name}");
    }
}

/// Inevitability of the Void [AUS][NEC]: +1 bleed per 10 cards in the bled
/// Methuselah's ash heap.
#[test]
fn inevitability_of_the_void_counts_the_target_ash_heap() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Angelique"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.state.seats[1].ash = vec![find_library("Bonding"); 23].into();
    hand(&mut e, 0, &["Inevitability of the Void"]);
    let before = pool(&e)[1];
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], before - 3, "1 + 2");
}

/// Necrosis [THN]: as [thn], and 1 unpreventable damage on the opposing
/// vampire in the press step, at close range.
#[test]
fn necrosis_burns_in_the_press_step() {
    let left = |mode: u8| {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_discs(&["THN"]), 3, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), find_card("Topaz"), 7, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Necrosis"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(b));
        pick_when(&mut e, play(0, mode, a));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        blood(&e, b)
    };
    assert_eq!(left(1), left(2) + 1);
}

/// Framing an Ancient Grudge: a prince loses the title; a justicar for 3
/// pool more.
#[test]
fn framing_an_ancient_grudge_strips_a_title() {
    for (title, mode, paid) in [
        (Title::Prince("New York"), 0, 0),
        (Title::Justicar("Brujah"), 1, 3),
    ] {
        let mut e = game(2);
        e.add_minion(SeatId(0), clan("Ventrue antitribu", 0), 4, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let t = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        e.state.minions[t.index()].granted_title = Some(title);
        hand(&mut e, 0, &["Framing an Ancient Grudge"]);
        until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        pick(&mut e, play_at(0, mode, None, Target::Minion(t)));
        request(&mut e);
        assert_eq!(e.state.title_of(t), Title::None);
        assert_eq!(pool(&e)[0], 10 - paid);
    }
}

/// Blessings of the Loa: Samedi of capacity 4 or more, +1 bleed against the
/// prey per vampire in torpor that prey controls (owner, 2026-11-02).
#[test]
fn blessings_of_the_loa_counts_your_preys_torpor() {
    let mut e = game(3);
    let s = e.add_minion(SeatId(0), clan("Samedi", 0), 4, false);
    let t = e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    let u = e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    let own = e.add_minion(SeatId(0), clan("Brujah", 2), 4, false);
    in_play_with_layers(&mut e, 0, "Blessings of the Loa", None);
    assert_eq!(e.state.bleed_against_mod(s, SeatId(1)), 0);
    e.state.minions[t.index()].in_torpor = true;
    e.state.minions[u.index()].in_torpor = true;
    e.state.minions[own.index()].in_torpor = true;
    assert_eq!(
        e.state.bleed_against_mod(s, SeatId(1)),
        2,
        "the prey's only"
    );
    assert_eq!(e.state.bleed_against_mod(s, SeatId(2)), 0, "the prey only");
    let b = e.add_minion(SeatId(0), clan("Brujah", 3), 4, false);
    assert_eq!(e.state.bleed_against_mod(b, SeatId(1)), 0, "Samedi only");
}

/// Cloak of Blood: a diablerie by card, a level of a Discipline of the
/// victim's, and 2 more votes in the blood hunt.
#[test]
fn cloak_of_blood_diablerizes_and_takes_a_discipline() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Jack Drake"), 8, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), find_card("Topaz"), 2, false);
    e.state.minions[v.index()].in_torpor = true;
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Cloak of Blood"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Minion(v)));
    pick_when(&mut e, Choice::ChooseDiscipline("for"));
    assert!(!e.state.minions[v.index()].in_play, "diablerized");
    assert_eq!(e.state.discipline_level(a, "for"), 1);
    request(&mut e);
    let r = e.referendum().expect("the blood hunt");
    assert!(r.bonus.contains(&(a, 2)), "{:?}", r.bonus);
}

/// Stealing Years: on the diablerist, capacity +1, not in the ash heap.
#[test]
fn stealing_years_stays_on_the_anarch_with_capacity() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), find_card("Topaz"), 2, false);
    e.state.minions[v.index()].in_torpor = true;
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let cap = e.state.capacity_of(a).expect("capacity");
    hand(&mut e, 0, &["Stealing Years"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Minion(v)));
    request(&mut e);
    assert_eq!(e.action().expect("the action").stealth(&e.state), 1);
    for _ in 0..10 {
        if on(&e, "Stealing Years", a).is_some() {
            break;
        }
        let req = request(&mut e);
        let i = fallback(&req);
        e.choose(i);
    }
    let c = on(&e, "Stealing Years", a).expect("on the anarch");
    placed(&e, c);
    assert_eq!(e.state.capacity_of(a), Some(cap + 1));
}

/// Shoulder Drop: after a hand strike that inflicted damage, 1 more on the
/// opposing minion, which cannot press this round.
#[test]
fn shoulder_drop_adds_a_damage_after_the_strike() {
    let left = |drop: bool| {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
        let b = e.add_minion(SeatId(1), find_card("Topaz"), 7, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Shoulder Drop"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(b));
        if drop {
            pick_when(&mut e, play(0, 0, a));
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        blood(&e, b)
    };
    assert_eq!(left(false), left(true) + 1);
}

/// Blind Spot: while the chosen vampire acts this turn, the chosen younger
/// vampire of the prey cannot block; another's action, it can.
#[test]
fn blind_spot_bars_a_block_this_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Jack Drake"), 8, false);
    let o = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Blind Spot"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, play_at(0, 0, None, Target::Minion(a)));
    pick_when(&mut e, Choice::ChooseMinion(y));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cannot_block(y, a));
    assert!(!e.state.cannot_block(z, a));
    assert!(!e.state.cannot_block(y, o));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cannot_block(y, a), "this turn only");
}

/// Prison of the Mind [DEM]: on the vampire, -1 stealth, -2 intercept; three
/// actions burn it.
#[test]
fn prison_of_the_mind_goes_on_the_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["DEM"]), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Prison of the Mind"]);
    pick(&mut e, play_at(0, 2, Some(a), Target::Minion(t)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let c = on(&e, "Prison of the Mind", t).expect("on the vampire");
    placed(&e, c);
}

/// The combats begun so far, as their minions.
fn fought(notes: &[zvtes::combat::CombatNote]) -> Vec<[MinionId; 2]> {
    notes
        .iter()
        .filter_map(|n| match n {
            zvtes::combat::CombatNote::Begins { minions } => Some(*minions),
            _ => None,
        })
        .collect()
}

/// Follow the Blood: after a successful hunt, a Black Hand vampire of another
/// Methuselah locks and fights the acting vampire.
#[test]
fn follow_the_blood_enters_combat_with_the_hunter() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), clan("Brujah", 0), 2, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Nails"), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Follow the Blood"]);
    e.notes = Some(Vec::new());
    pick(&mut e, action(h, ActionKind::Hunt));
    pick_when(&mut e, play(0, 0, b));
    request(&mut e);
    let combats = fought(&noted_combats(&mut e));
    assert!(combats.iter().any(|m| m.contains(&h) && m.contains(&b)));
    assert!(e.state.minions[b.index()].locked);
}

/// The Framing: put on the anarch by its successful action; blocked later,
/// the card burns and the blocker fights the chosen minion instead.
#[test]
fn the_framing_sends_the_blocker_at_the_chosen_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    let f = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), with_discs(&["aus"]), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Framing, The"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Minion(f)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let card = on(&e, "Framing, The", a).expect("on the anarch");
    placed(&e, card);
    assert_eq!(e.state.cards[card.index()].chosen, vec![f]);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    request(&mut e);
    let combats = fought(&noted_combats(&mut e));
    assert!(
        combats.iter().any(|m| m.contains(&f) && m.contains(&b)),
        "{combats:?}"
    );
    assert!(!combats.iter().any(|m| m.contains(&a)));
    assert!(on(&e, "Framing, The", a).is_none(), "burned");
}

/// Haven Hunting: as an Anarch's combat ends, both ready, burned for another.
#[test]
fn haven_hunting_starts_another_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Topaz"), 7, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let h = in_play(&mut e, 0, "Haven Hunting", None);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, Choice::ResolveRider { card: h.0 });
    request(&mut e);
    let combats = fought(&noted_combats(&mut e));
    assert_eq!(combats.len(), 2, "{combats:?}");
    assert!(!e.state.cards[h.index()].in_play, "burned");
}

/// Power of All: two Anarchs lock to cancel a card played by another
/// Methuselah on their turn.
#[test]
fn power_of_all_cancels_with_two_anarchs() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), anarch(&[], 0), 4, false);
    let y = e.add_minion(SeatId(1), anarch(&[], 1), 4, false);
    hand(&mut e, 0, &["Kevlar Vest"]);
    hand(&mut e, 1, &["Power of All"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    request(&mut e);
    assert!(e.state.minions[x.index()].locked && e.state.minions[y.index()].locked);
    assert_eq!(blood(&e, x), 2, "2 blood");
    assert!(ash_has(&e, 0, "Kevlar Vest"), "canceled");
}

/// Praetorian Backer: the title of cardinal, burned with the last Seraph.
/// Black Hand Emissary: votes per ready Black Hand vampire.
#[test]
fn seraph_cards_follow_their_seraph() {
    let seraph = (0..CRYPT.len() as u16)
        .find(|&c| CRYPT[c as usize].seraph && zvtes::cards::vampire(c).is_none())
        .expect("a plain Seraph");
    let big = (0..CRYPT.len() as u16)
        .find(|&c| {
            let v = &CRYPT[c as usize];
            v.sect == Some(Sect::Sabbat)
                && v.capacity >= 7
                && !v.black_hand
                && v.title == Title::None
        })
        .expect("a big Sabbat vampire");
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), seraph, 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(0), big, 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Praetorian Backer", "Black Hand Emissary"]);
    pick(&mut e, play_at(0, 0, Some(s), Target::Minion(t)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(t), Title::Cardinal);
    let votes = e.state.votes_held(t);
    e.state.minions[s.index()].in_torpor = true;
    request(&mut e);
    assert!(on(&e, "Praetorian Backer", t).is_none(), "burned");
    assert!(e.state.votes_held(t) < votes);
}

/// An ally costing 2.
const ALLY: &str = "Carlton Van Wyk";

/// Keystone Kine [cel][nec][obf]: a bleed at +1, 1 blood, an ally of the prey
/// whose cost is within the bleed burned at resolution.
#[test]
fn keystone_kine_bleeds_gains_and_burns_an_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&["cel"], 0), 3, false);
    e.state.minions[a.index()].gained_levels = vec!["nec", "obf"];
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library(ALLY));
    hand(&mut e, 0, &["Keystone Kine"]);
    let before = pool(&e)[1];
    pick(&mut e, play(0, 6, a));
    pick_when(&mut e, Choice::ChooseMinion(ally));
    request(&mut e);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    assert_eq!(blood(&e, a), 4);
    assert_eq!(pool(&e)[1], before - 2);
}

/// Detect Authority [ani]: the acting vampire burns 2 blood, or the action
/// fails, not blocked.
#[test]
fn detect_authority_makes_the_acting_vampire_pay() {
    for pay in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_discs(&["DEM"]), 5, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let d = e.add_minion(SeatId(1), anarch(&["ani"], 0), 4, false);
        let t = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Prison of the Mind"]);
        hand(&mut e, 1, &["Detect Authority"]);
        pick(&mut e, play_at(0, 2, Some(a), Target::Minion(t)));
        pick_when(&mut e, play(0, 0, d));
        let answer = if pay {
            Choice::BurnBloodToAvoid
        } else {
            Choice::Pass
        };
        pick_when(&mut e, answer);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(on(&e, "Prison of the Mind", t).is_some(), pay, "{pay}");
    }
}

/// Final Loosening [for]: an ally's action fails, it takes 1 damage, the
/// Anarch locks.
#[test]
fn final_loosening_stops_an_acting_ally() {
    let mut e = game(2);
    let ally = e.add_ally(SeatId(0), find_library(ALLY));
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let r = e.add_minion(SeatId(1), anarch(&["for"], 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Final Loosening"]);
    let before = pool(&e)[1];
    let life = e.state.minions[ally.index()].blood;
    pick(&mut e, bleed(ally));
    pick_when(&mut e, play(0, 2, r));
    request(&mut e);
    assert!(e.state.minions[r.index()].locked);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[ally.index()].blood, life - 1);
    assert_eq!(pool(&e)[1], before, "failed");
}

/// The World's a Canvas [dom]: half the blood rounded down on a crypt card
/// of the prey's; [vic]: 4 blood from a titled non-Anarch.
#[test]
fn the_worlds_a_canvas_burns_uncontrolled_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&["dom"], 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    e.state.seats[1]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: clan("Toreador", 0),
            copy: Default::default(),
            counters: 5,
            banished: None,
        });
    hand(&mut e, 0, &["World's a Canvas, The"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Seat(SeatId(1))));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].uncontrolled[0].counters, 3);
}

/// Enrage [PRE]: 2 blood to attempt an action, not to hunt; burned after the
/// action. Unable to pay, the bleed is not offered.
#[test]
fn enrage_tolls_actions_but_hunts() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 3, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let c = in_play_with_layers(&mut e, 0, "Enrage", Some(a));
    e.state.cards[c.index()].mode = 1;
    e.state.layers.retain(|l| l.source != c);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostActionToll(2),
        source: c,
    });
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 1, "2 burned");
    assert!(!e.state.cards[c.index()].in_play, "burned after the action");
}

/// Drink the Blood of Ahriman: +1 strength; each unlock phase 1 blood or the
/// card, its controller's choice; the Lasombra's [obt] cards 1 fewer blood.
// Rulings: [ANK 20220503].
#[test]
fn drink_the_blood_of_ahriman_is_fed_at_unlock() {
    for keep in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), clan("Lasombra", 0), 4, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Drink the Blood of Ahriman"]);
        pick(&mut e, play(0, 0, a));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let c = on(&e, "Drink the Blood of Ahriman", a).expect("on the Lasombra");
        placed(&e, c);
        let answer = if keep {
            Choice::BurnBloodToAvoid
        } else {
            Choice::Pass
        };
        while !matches!(
            e.interrupt(),
            Some(zvtes::draws::Interrupt::BurnOrFail { .. })
        ) {
            let req = request(&mut e);
            if !matches!(
                e.interrupt(),
                Some(zvtes::draws::Interrupt::BurnOrFail { .. })
            ) {
                let i = fallback(&req);
                e.choose(i);
            }
        }
        pick(&mut e, answer);
        request(&mut e);
        assert_eq!(e.state.cards[c.index()].in_play, keep);
        assert_eq!(blood(&e, a), if keep { 3 } else { 4 });
    }
}

/// Joseph Pander: lock and 1 pool make an action directed at a Pander fail;
/// the Pander chosen enters combat as an action, once this turn.
#[test]
fn joseph_pander_guards_and_sends_his_panders() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["DEM"]), 5, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let pander = CRYPT
        .iter()
        .position(|c| c.clan == "Pander")
        .expect("a Pander") as u16;
    let p = e.add_minion(SeatId(1), pander, 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let j = in_play(&mut e, 1, "Joseph Pander", None);
    hand(&mut e, 0, &["Prison of the Mind"]);
    pick(&mut e, play_at(0, 2, Some(a), Target::Minion(p)));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(j),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MasterPhase);
    assert!(on(&e, "Prison of the Mind", p).is_none(), "failed");
    assert_eq!(pool(&e)[1], 9);
}

/// Grey Thorne: at 1 life, a ready Anarch burns a blood to add 1 life.
#[test]
fn grey_thorne_is_healed_by_an_anarch() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let g = e.add_ally(SeatId(0), find_library("Grey Thorne"));
    e.state.minions[g.index()].blood = 1;
    let c = in_play(&mut e, 0, "Grey Thorne", Some(g));
    let _ = c;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, Choice::ChooseMinion(a));
    request(&mut e);
    assert_eq!(e.state.minions[g.index()].blood, 2);
}

/// Epiphany: unlocks the acting vampire, merged with its advanced version
/// from the crypt.
#[test]
fn epiphany_merges_with_the_advanced_version() {
    let base = (0..CRYPT.len() as u16)
        .find(|&c| {
            !CRYPT[c as usize].adv
                && !CRYPT[c as usize].imbued
                && CRYPT
                    .iter()
                    .any(|o| o.adv && o.name == CRYPT[c as usize].name)
        })
        .expect("a vampire with an advanced version");
    let adv = CRYPT
        .iter()
        .position(|o| o.adv && o.name == CRYPT[base as usize].name)
        .expect("advanced") as u16;
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), base, 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    e.state.seats[0].crypt.push(adv);
    hand(&mut e, 0, &["Epiphany"]);
    pick(&mut e, play(0, 0, a));
    request(&mut e);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].merged.is_some());
    assert!(!e.state.minions[a.index()].locked, "unlocked");
}

/// No Confidence: on a titled non-Anarch of the action's target; two lose
/// the title's benefit; burned as the acting vampire leaves the ready region.
#[test]
fn no_confidence_strips_a_title_at_two() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    let t = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.state.minions[t.index()].granted_title = Some(Title::Prince("New York"));
    let x = in_play_with_layers(&mut e, 0, "No Confidence", Some(t));
    assert_eq!(e.state.title_of(t), Title::Prince("New York"), "one");
    let y = in_play_with_layers(&mut e, 0, "No Confidence", Some(t));
    assert_eq!(e.state.title_of(t), Title::None, "two");
    e.state.cards[x.index()].chosen = vec![a];
    e.state.cards[y.index()].chosen = vec![a];
    e.state.minions[a.index()].in_torpor = true;
    request(&mut e);
    assert!(!e.state.cards[x.index()].in_play, "burned");
    assert_eq!(e.state.title_of(t), Title::Prince("New York"));
}

/// Tribunal Judgment: the vampire takes as much damage as it cast votes in
/// the previous referendum.
#[test]
fn tribunal_judgment_damages_a_voter() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Hagar Stone"), 4, false);
    e.state.minions[c.index()].granted_title = Some(Title::Votes(1));
    let o = e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), clan("Ventrue", 0), 5, false);
    e.state.cast_votes = (e.state.turn, vec![(t, 2)]);
    hand(&mut e, 0, &["Tribunal Judgment"]);
    let _ = o;
    pick(&mut e, play(0, 0, c));
    pick_when(&mut e, votes(c, true));
    close_polling(&mut e);
    assert_eq!(blood(&e, t), 3);
}

/// Blood Turnip: as its minion enters combat, it moves with 1 blood to the
/// opposing minion.
#[test]
fn blood_turnip_moves_as_combat_begins() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Topaz"), 7, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let c = in_play_with_layers(&mut e, 1, "Blood Turnip", Some(a));
    let before = blood(&e, b);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    request(&mut e);
    assert_eq!(e.state.cards[c.index()].host, Some(b));
    assert!(blood(&e, b) >= before - 1, "1 blood moved, strikes aside");
}

/// Achilles' Heel: Blood Brothers of the chosen circle +1 bleed against the
/// host's controller while it is in torpor.
#[test]
fn achilles_heel_opens_a_vampire_to_a_circle() {
    let (bb, circle) = CRYPT
        .iter()
        .enumerate()
        .find_map(|(i, c)| c.circle.map(|k| (i as u16, k)))
        .expect("a Blood Brother with a circle");
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), bb, 4, false);
    let v = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let c = in_play_with_layers(&mut e, 0, "Achilles' Heel", Some(v));
    e.state.cards[c.index()].chosen_circle = Some(circle);
    assert_eq!(e.state.bleed_against_mod(b, SeatId(1)), 0);
    e.state.minions[v.index()].in_torpor = true;
    assert_eq!(e.state.bleed_against_mod(b, SeatId(1)), 1);
}

/// Liquefy the Mortal Coil [vic]: 3 blood from the opposing younger vampire;
/// out of blood, the combat ends.
#[test]
fn liquefy_burns_a_younger_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Topaz"), 7, false);
    e.state.minions[a.index()].gained_levels = vec!["vic"];
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Liquefy the Mortal Coil"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 1);
}

/// Elixir of Distillation: the strike burns 1 blood, lowers capacity by 1,
/// the weapon burned; a master phase action locks the vampire to restore it.
#[test]
fn elixir_of_distillation_lowers_capacity_until_a_lock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Topaz"), 7, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let w = in_play(&mut e, 0, "Elixir of Distillation", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let strike = Choice::UseAbility {
        source: AbilitySource::Card(w),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, strike);
    until(&mut e, SeatId(1), WindowLabel::MasterPhase);
    assert!(!e.state.cards[w.index()].in_play, "burned");
    assert_eq!(e.state.capacity_of(b), Some(6));
    assert_eq!(blood(&e, b), 6);
    let stand_in = on(&e, "Elixir of Distillation", b).expect("the reduction");
    assert!(e.state.cards[stand_in.index()].effect_only);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(stand_in),
            ability: 1,
            actor: None,
            target: Target::None,
        },
    );
    request(&mut e);
    assert_eq!(e.state.capacity_of(b), Some(7));
    assert!(e.state.minions[b.index()].locked);
}

/// Eccentric Billionaire: locks to take 1 pool off an equipment.
#[test]
fn eccentric_billionaire_pays_for_equipment() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_ally(SeatId(0), find_library("Eccentric Billionaire"));
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kevlar Vest"]);
    pick(&mut e, play(0, 0, a));
    let req = request(&mut e);
    let i = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::Pass))
        .map_or(0, |p| usize::from(p == 0));
    e.choose(i);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 10, "free");
    let _ = b;
}

/// Trophy: Chosen: in hand, awarded for a Red List minion burned; 4 blood,
/// Sabbat, unlocked, Black Hand, a Seraph if it was Black Hand already.
#[test]
fn trophy_chosen_rewards_a_red_list_kill() {
    let red = (0..CRYPT.len() as u16)
        .find(|&c| CRYPT[c as usize].red_list)
        .expect("a Red List vampire");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.state.minions[a.index()].granted_title = None;
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), red, 2, true);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Trophy: Chosen"]);
    let trophy = find_library("Trophy: Chosen");
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Diablerie,
            target: Some(v),
        },
    );
    pick_when(&mut e, Choice::FindCard(trophy));
    let card = on(&e, "Trophy: Chosen", a).expect("on the vampire");
    pick(&mut e, Choice::ResolveRider { card: card.0 });
    request(&mut e);
    assert_eq!(e.state.sect_of(a), Some(Sect::Sabbat));
    assert!(e.state.is_black_hand(a));
    assert!(!e.state.is_seraph(a));
}

/// Blade Clot: an Anarch's hand strike moves a counter; at their unlock phase
/// the minion goes to torpor; an older rescuer burns the counters.
#[test]
fn blade_clot_sends_the_struck_to_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Topaz"), 7, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let c = in_play_with_layers(&mut e, 0, "Blade Clot", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::MovesClotsOnHit,
        source: c,
    });
    e.state.cards[c.index()].counters = 3;
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, Choice::ResolveRider { card: c.0 });
    request(&mut e);
    assert_eq!(e.state.minions[b.index()].clots, 1);
    assert_eq!(e.state.cards[c.index()].counters, 2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[b.index()].in_torpor);
}

/// Watchtower: The Wolves Feed: Black Hand +1 stealth; at unlock, X pool or
/// burned; a Black Hand vampire can strike for 2R.
#[test]
fn watchtower_feeds_the_black_hand() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Nails"), 4, false);
    let o = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let w = in_play_with_layers(&mut e, 0, "Watchtower: The Wolves Feed", None);
    assert_eq!(e.state.black_hand_stealth(b), 1);
    assert_eq!(e.state.black_hand_stealth(o), 0);
    let _ = w;
}

// ---- Rulings and the round's thin spots -----------------------------------

/// A capacity 9 vampire with no text.
fn big() -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.capacity == 9 && !c.imbued && zvtes::cards::vampire(i as u16).is_none()
        })
        .unwrap() as u16
}

/// A card's ability, from `source`, with no actor or target.
fn card_ability(c: CardId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(c),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// Arianne: on a later round, her strike ends the combat, 1 damage on the
/// opposing minion as it has ended: no combat card for it; the combat going
/// on, none.
// Rulings: [RTR 20020501] [LSJ 19980109]; [LSJ 20031123]; [LSJ 20100204-2].
#[test]
fn arianne_ends_the_combat_and_damages_after_it() {
    let start = |e: &mut Engine, z_hand: &[&str]| {
        let a = e.add_minion(SeatId(0), find_card("Arianne, The Conqueror"), 8, false);
        e.state.minions[a.index()].gained_levels = vec!["thn", "for"];
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), superior("AUS", 0), 6, false);
        e.state.minions[z.index()].gained_levels = vec!["for"];
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(e, 0, &["Relentless Reaper"]);
        hand(e, 1, z_hand);
        pick(e, bleed(a));
        pick_when(e, block(z));
        (a, z)
    };
    // The first round, no such strike; pressed on, it is offered.
    let mut e = game(2);
    let (a, z) = start(&mut e, &["Relentless Reaper"]);
    never_offered_until(&mut e, SeatId(0), WindowLabel::CombatPress, own(a, 0));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, own(a, 0));
    let before = blood(&e, z);
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == z),
    );
    assert_eq!(blood(&e, z), before - 1, "1 after combat, not prevented");

    // A new round in its place: the after-combat damage is lost.
    let mut e = game(2);
    let (a, z) = start(&mut e, &["Telepathic Tracking"]);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, own(a, 0));
    let before = blood(&e, z);
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let strikes = before.saturating_sub(blood(&e, z));
    assert_eq!(
        strikes,
        1 + 1,
        "the third round's hand strike, the card's blood"
    );
}

/// Laecanus: frenzy cards are not played on him; his press ends the combat
/// the opposing minion's press would have continued, once a combat.
#[test]
fn laecanus_ends_the_combat_and_no_frenzy_reaches_him() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Laecanus"), 5, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), with_discs(&["pot"]), 6, false);
    e.state.minions[z.index()].gained_levels = vec!["thn"];
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Relentless Reaper"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(l));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, z));
    pick_when(&mut e, own(l, 0));
    pick_when(&mut e, Choice::UsePress);
    let before = blood(&e, z);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), before, "no second round");

    // Terror Frenzy on the opposing minion: barred on Laecanus only.
    for (card, barred) in [(find_card("Laecanus"), true), (clan("Brujah", 0), false)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), card, 5, false);
        e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), with_discs(&["ani"]), 6, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 1, &["Terror Frenzy"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        let any = |o: &Choice| matches!(o, Choice::Play { card: 0, .. });
        if barred {
            never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, any);
        } else {
            let mut seen = false;
            while !seen {
                let req = request(&mut e);
                if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                    break;
                }
                seen = req.options.iter().any(any);
                e.choose(fallback(&req));
            }
            assert!(seen);
        }
    }
}

/// Reverend Adams (advanced): in the master phase, his Methuselah looks at
/// the top X of the library, X the Gehenna cards in play; none, no look.
#[test]
fn reverend_adams_advanced_looks_by_the_gehenna_cards() {
    let adv = CRYPT
        .iter()
        .position(|c| c.name == "Reverend Adams" && c.adv)
        .unwrap() as u16;
    for gehenna in [0, 2] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), adv, 4, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
        hand(&mut e, 0, &["Blood Doll"]);
        fill_hand(&mut e, 0);
        for name in ["Blood Weakens", "Conquest of Humanity"]
            .iter()
            .take(gehenna)
        {
            in_play(&mut e, 1, name, None);
        }
        let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        assert_eq!(req.options.contains(&own(r, 0)), gehenna > 0);
        if gehenna > 0 {
            e.notes = Some(Vec::new());
            pick(&mut e, own(r, 0));
            request(&mut e);
            let looks = noted_looks(&mut e);
            assert_eq!(looks.len(), 1, "{looks:?}");
        }
    }
}

/// Census Taker: its Black Hand vampire locks to give another Black Hand
/// vampire attempting to block +1 intercept; a block otherwise out of reach.
#[test]
fn census_taker_gives_a_black_hand_blocker_intercept() {
    let mut e = game(2);
    let st = e.add_minion(SeatId(0), find_card("Stephen Bateson"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), find_card("Nails"), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Gatjil Munyarryun"), 4, false);
    let c = in_play(&mut e, 1, "Census Taker", Some(t));
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(st));
    pick_when(&mut e, block(g));
    pick_when(&mut e, card_ability(c, 2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].locked);
    let combats = fought(&noted_combats(&mut e));
    assert_eq!(combats, vec![[st, g]], "blocked");
}

/// Vengeful Spirit: 1 damage from the retainer each close round, through a
/// dodge, not after a strike ending the combat, not repeated on an
/// additional strike; the retainer, not its employer, deals it.
// Rulings: [ANK 20180612] [ANK 20211127]; [RTR 19941109] [TOM 19951216]
// [PIB 20140324] [ANK 20200422]; [RTR 19941109].
#[test]
fn vengeful_spirit_damages_each_close_round() {
    let setup = |z_hand: &[&str], a_hand: &[&str]| {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), find_card("Khurshid"), 4, false);
        e.state.minions[k.index()].gained_levels = vec!["cel"];
        e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), big(), 9, false);
        e.state.minions[z.index()].gained_levels = vec!["pre"];
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Vengeful Spirit"]);
        hand(&mut e, 1, z_hand);
        pick(&mut e, play(0, 0, k));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        hand(&mut e, 0, a_hand);
        pick(&mut e, bleed(k));
        pick_when(&mut e, block(z));
        (e, k, z)
    };
    // A dodge: the hand strike misses, the spirit's damage lands.
    let (mut e, k, z) = setup(&["Dodge"], &[]);
    let before = blood(&e, z);
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), before - 1, "through the dodge");
    assert!(blood(&e, k) <= 4);

    // A strike ending the combat: nothing.
    let (mut e, _, z) = setup(&["Majesty"], &[]);
    let before = blood(&e, z);
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), before - 1, "Majesty's cost alone");

    // An additional strike of the employer: not the retainer's.
    let (mut e, k, z) = setup(&[], &["Blur"]);
    let before = blood(&e, z);
    pick_when(&mut e, play(0, 0, k));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), before - 3 - 3 - 1, "two strikes, 1 damage");
}

/// Blade of Bellona: long range, its maneuver to close commits its strike;
/// Blessed Blade still uses that weapon as the strike after the maneuver.
/// A strike barred, the maneuver is not offered.
// Rulings: [ANK 20180704] [LSJ 20050304]; [LSJ 20021028].
#[test]
fn blade_of_bellona_maneuvers_to_close_and_blessed_blade_strikes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Lailah"), 6, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), big(), 9, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let blade = in_play(&mut e, 0, "Blade of Bellona", Some(a));
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Blessed Blade"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, card_ability(gun, 1));
    pick_when(&mut e, card_ability(blade, 1));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(!req.options.contains(&Choice::HandStrike), "committed");
    let bb = play_at(0, 1, Some(a), Target::Card(blade));
    let before = blood(&e, z);
    e.choose(index_of(&req, bb));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), before - (1 + 1 + 2));

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Lailah"), 6, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let tzimisce = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.clan == "Tzimisce"
                && c.disciplines.contains(&"PRO")
                && !c.imbued
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a Tzimisce with superior Protean") as u16;
    let z = e.add_minion(SeatId(1), tzimisce, 5, false);
    e.state.minions[z.index()].locked = true;
    let blade = in_play(&mut e, 0, "Blade of Bellona", Some(a));
    in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 1, &["One With the Land"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, z));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == blade),
    );
}

/// Hagar Stone and a card's own delay: the longer holds. Bat played by
/// another vampire waits past its action for Hagar's lock; played by Hagar,
/// locked by it, past his lock until the action ends.
// Rulings: [LSJ 20070320-1].
#[test]
fn hagar_stone_and_a_cards_own_delay_the_longer_wins() {
    let setup = || {
        let mut e = game(2);
        let hagar = e.add_minion(SeatId(0), find_card("Hagar Stone"), 6, false);
        let other = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        e.state.seats[0].library = vec![find_library("Conditioning"); 5].into();
        hand(&mut e, 0, &["Baseball Bat"]);
        fill_hand(&mut e, 0);
        (e, hagar, other)
    };
    let (mut e, hagar, other) = setup();
    pick(&mut e, play(0, 0, other));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].held.total(), 1, "past the action");
    pick(&mut e, action(hagar, ActionKind::Hunt));
    request(&mut e);
    assert_eq!(e.state.seats[0].held.total(), 0, "as he locks");

    let (mut e, hagar, _) = setup();
    pick(&mut e, play(0, 0, hagar));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ActionBlock);
    assert!(e.state.minions[hagar.index()].locked);
    assert_eq!(e.state.seats[0].held.total(), 1, "the Bat's own delay");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].held.total(), 0, "after the action");
}

/// Hagar Stone: Learjet's extra draw waits with the held replacement; a
/// draw beyond the hand size while a card is held takes a discard.
// Rulings: [ANK 20180318] [ANK 20231229].
#[test]
fn hagar_stone_holds_learjets_draw_and_shrinks_the_hand() {
    let mut e = game(2);
    let hagar = e.add_minion(SeatId(0), find_card("Hagar Stone"), 6, false);
    let o = e.add_minion(SeatId(0), superior("dom", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    in_play_with_layers(&mut e, 0, "Learjet", Some(o));
    hand(&mut e, 0, &["Govern the Unaligned"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, play(0, 0, o));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.interrupt(), None);
    assert_eq!(e.state.seats[0].held.total(), 1);
    pick(&mut e, action(hagar, ActionKind::Hunt));
    assert_eq!(
        extra_draw(&mut e),
        Some(zvtes::draws::DrawSource::Learjet),
        "with the replacement"
    );

    // Ira Rivers's extra draw on the play: a card held, the draw goes over.
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Hagar Stone"), 6, false);
    let ira = e.add_minion(SeatId(0), find_card("Ira Rivers"), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Revelations"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    let size = e.state.hand_size(SeatId(0));
    pick(&mut e, play_at(0, 0, Some(ira), Target::Seat(SeatId(1))));
    assert!(extra_draw(&mut e).is_some());
    pick(&mut e, Choice::DrawLibrary);
    let req = request(&mut e);
    assert!(
        req.options.iter().any(|o| matches!(o, Choice::Discard(_))),
        "{:?}",
        req.options
    );
    let i = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::Discard(_)))
        .unwrap();
    e.choose(i);
    request(&mut e);
    assert_eq!(e.state.hand_load(SeatId(0)), size);
}

/// Duality with a Vicissitude card: Capuchin takes the replacement, so no
/// extra draw.
#[test]
fn duality_brings_nothing_for_a_replacement_capuchin_takes() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Duality"), 5, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let c = capuchin(&mut e, 2);
    hand(&mut e, 0, &["Horrid Form"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, bleed(d));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, d));
    assert_eq!(extra_draw(&mut e), None);
    assert_eq!(e.state.cards[c.index()].counters, 1, "the counter took it");
}

/// Detect Authority [ser]: +1 intercept, a block otherwise out of reach;
/// [tha]: a bleed against its Methuselah reduced by 2.
#[test]
fn detect_authority_intercepts_and_reduces_a_bleed() {
    let mut e = game(2);
    let st = e.add_minion(SeatId(0), find_card("Stephen Bateson"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let d = e.add_minion(SeatId(1), anarch(&[], 0), 4, false);
    e.state.minions[d.index()].gained_levels = vec!["ser", "tha"];
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Detect Authority"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(st));
    pick_when(&mut e, block(d));
    pick_when(&mut e, play(0, 1, d));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(fought(&noted_combats(&mut e)), vec![[st, d]]);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let d = e.add_minion(SeatId(1), anarch(&[], 0), 4, false);
    e.state.minions[d.index()].gained_levels = vec!["ser", "tha"];
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Detect Authority"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 2, d));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "bleed 1, 2 off");
}

/// Final Loosening [aus]: on a hunt the acting vampire and this Anarch each
/// gain 1 blood; [dem]: +4 votes in a polling step.
#[test]
fn final_loosening_splits_a_hunt_and_gives_votes() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), clan("Brujah", 0), 2, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let r = e.add_minion(SeatId(1), anarch(&[], 0), 3, false);
    e.state.minions[r.index()].gained_levels = vec!["aus", "dem"];
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Final Loosening"]);
    pick(&mut e, action(h, ActionKind::Hunt));
    pick_when(&mut e, play(0, 0, r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, h), blood(&e, r)), (2 + 1, 3 + 1));

    let mut e = game(2);
    let c = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let r = e.add_minion(SeatId(1), anarch(&[], 0), 3, false);
    e.state.minions[r.index()].gained_levels = vec!["aus", "dem"];
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    hand(&mut e, 1, &["Final Loosening"]);
    pick(&mut e, play(0, 0, c));
    to_referendum(&mut e);
    pick_when(&mut e, play(0, 1, r));
    request(&mut e);
    let rf = e.referendum().expect("referendum");
    let gave: Vec<_> = rf
        .records
        .iter()
        .filter(|v| v.source == zvtes::referendum::Source::Vampire(r))
        .map(|v| v.n)
        .collect();
    assert!(gave.contains(&4) || rf.bonus.contains(&(r, 4)), "{gave:?}");
}

/// Their Master's Voice: a locked vampire unlocks a ready slave Gargoyle of
/// its Methuselah; a locked non-slave vampire is no target.
#[test]
fn their_masters_voice_unlocks_a_slave_gargoyle() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let m = e.add_minion(SeatId(1), clan("Tremere", 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Chalcedony"), 7, false);
    let n = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    for v in [m, g, n] {
        e.state.minions[v.index()].locked = true;
    }
    hand(&mut e, 1, &["Their Master's Voice"]);
    pick(&mut e, bleed(a));
    let want = play_at(0, 0, Some(m), Target::Minion(g));
    let req = offering(&mut e, want);
    assert!(!req
        .options
        .contains(&play_at(0, 0, Some(m), Target::Minion(n))));
    pick(&mut e, want);
    request(&mut e);
    assert!(!e.state.minions[g.index()].locked);
    assert!(e.state.minions[n.index()].locked);
}

/// Unleash the Hounds [VIS]: a slave Gargoyle's bleed at +1 for each ready
/// vampire of the enslaving clan its Methuselah controls.
#[test]
fn unleash_the_hounds_counts_the_enslaving_clan() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Chalcedony"), 7, false);
    e.add_minion(SeatId(0), clan("Tremere", 0), 4, false);
    e.add_minion(SeatId(0), clan("Tremere", 1), 4, false);
    let t = e.add_minion(SeatId(0), clan("Tremere", 2), 4, false);
    e.state.minions[t.index()].in_torpor = true;
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Unleash the Hounds"]);
    pick(&mut e, bleed(g));
    pick_when(&mut e, play(0, 2, g));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 3, "1 + 2 ready Tremere");
}

/// Black Hand Emissary: X additional votes, X its controller's ready Black
/// Hand vampires; any Seraph moves it to a ready Black Hand vampire as a +1
/// stealth action directed at its controller.
#[test]
fn black_hand_emissary_gives_votes_and_moves() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), seraph(0), 4, false);
    let y = e.add_minion(SeatId(0), find_card("Nails"), 4, false);
    let t = e.add_minion(SeatId(1), find_card("Gatjil Munyarryun"), 4, false);
    let u = e.add_minion(SeatId(1), find_card("Marge Khan"), 4, false);
    let before = e.state.votes_held(t);
    let c = in_play_with_layers(&mut e, 1, "Black Hand Emissary", Some(t));
    assert_eq!(e.state.votes_held(t), before + 2);
    e.state.minions[u.index()].in_torpor = true;
    assert_eq!(e.state.votes_held(t), before + 1, "ready ones, live");
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(c),
            ability: 0,
            actor: Some(s),
            target: Target::Minion(y),
        },
    );
    request(&mut e);
    let act = e.action().expect("the action");
    assert_eq!(act.directed_seat(&e.state), Some(SeatId(1)));
    assert_eq!(act.stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[c.index()].host, Some(y));
}

/// Unexpected Coalition: each ready Brujah antitribu a vote more; one
/// voting against the player's, none of them counted.
#[test]
fn unexpected_coalition_counts_a_united_clan_only() {
    for united in [true, false] {
        let mut e = game(2);
        let antitribu: Vec<u16> = (0..CRYPT.len() as u16)
            .filter(|&c| CRYPT[c as usize].clan == "Brujah antitribu")
            .collect();
        let c = e.add_minion(SeatId(0), antitribu[0], 4, false);
        e.state.minions[c.index()].granted_title = Some(Title::Votes(1));
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let o = e.add_minion(SeatId(1), antitribu[1], 4, false);
        e.state.minions[o.index()].granted_title = Some(Title::Votes(1));
        e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Ancilla Empowerment", "Unexpected Coalition"]);
        pick(&mut e, play(0, 0, c));
        to_referendum(&mut e);
        pick_when(&mut e, votes(c, true));
        pick_when(&mut e, play(0, 0, c));
        pick_when(&mut e, votes(o, united));
        request(&mut e);
        let tally = e.referendum().expect("referendum").tally(&e.state);
        let want = if united { (4, 0) } else { (1, 1) };
        assert_eq!(tally, want, "united: {united}");
    }
}

/// Enrage [pre]: on a vampire of its Methuselah, +1 strength; it may enter
/// combat as a directed action; burned at the end of the turn.
#[test]
fn enrage_superior_gives_strength_and_a_combat_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pre", 0), 4, false);
    let f = e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Enrage"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Minion(f)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = on(&e, "Enrage", f).expect("on the vampire");
    placed(&e, c);
    e.notes = Some(Vec::new());
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(c),
            ability: 0,
            actor: Some(f),
            target: Target::Minion(z),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(fought(&noted_combats(&mut e)), vec![[f, z]]);
    assert_eq!(blood(&e, z), 4 - 2, "+1 strength");
    assert!(
        on(&e, "Enrage", f).is_none(),
        "burned at the end of the turn"
    );
}

/// Enrage [PRE]: without 2 blood, no action but a hunt; with them, an action
/// canceled as played burns nothing, the card stays.
// Rulings: [ANK 20180327]; [LSJ 20090208].
#[test]
fn enrage_superior_needs_the_blood_and_spares_a_canceled_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRE", 0), 5, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), clan("Ventrue", 0), 1, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Enrage"]);
    pick(&mut e, play_at(0, 1, Some(a), Target::Minion(t)));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(on(&e, "Enrage", t).is_some());
    assert!(!req.options.contains(&bleed(t)), "{:?}", req.options);
    assert!(req.options.contains(&action(t, ActionKind::Hunt)));

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 3, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let w = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let c = in_play(&mut e, 0, "Enrage", Some(a));
    e.state.cards[c.index()].mode = 1;
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostActionToll(2),
        source: c,
    });
    hand(&mut e, 0, &["Kevlar Vest"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    let _ = w;
    pick(&mut e, play(0, 0, a));
    let m = master_play(0);
    pick_when(&mut e, m);
    request(&mut e);
    assert_eq!(blood(&e, a), 3, "no action, no toll");
    assert!(e.state.cards[c.index()].in_play, "stays");
}

/// Cloak of Blood, Stealing Years: the target out of torpor as the action
/// resolves: it fizzles, successful, nothing diablerized, the card not on
/// the anarch; the action's vampire is spent.
// Rulings: [ANK 20210124] [LSJ 20070411] [LSJ 20090514].
#[test]
fn cloak_of_blood_and_stealing_years_fizzle_off_torpor() {
    for (card, actor, cap) in [
        ("Cloak of Blood", find_card("Jack Drake"), 8),
        ("Stealing Years", anarch(&[], 0), 4),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), actor, cap, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let v = e.add_minion(SeatId(1), find_card("Topaz"), 2, false);
        e.state.minions[v.index()].in_torpor = true;
        e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        hand(&mut e, 0, &[card]);
        pick(&mut e, play_at(0, 0, Some(a), Target::Minion(v)));
        e.state.minions[v.index()].in_torpor = false;
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        // "Fizzled" is the engine's successful action with no effect.
        assert_eq!(
            e.last_outcome,
            Some(zvtes::actions::Outcome::Fizzled),
            "{card}"
        );
        assert!(e.state.minions[v.index()].in_play, "{card}");
        assert!(on(&e, card, a).is_none(), "{card}");
        assert!(e.referendum().is_none(), "no blood hunt: {card}");
    }
}

/// Liquefy the Mortal Coil [VIC]: against a younger vampire; no strike this
/// round, no gun maneuver with it, an additional strike still played; next
/// round the strike diablerizes.
// Rulings: [LSJ 20001126]; [LSJ 19970814-2].
#[test]
fn liquefy_superior_diablerizes_next_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), big(), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["vic", "thn", "cel"];
    e.state.minions[a.index()].beast_disciplines[0] = Some("vic");
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 2, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(a));
    hand(
        &mut e,
        0,
        &["Liquefy the Mortal Coil", "Relentless Reaper", "Blur"],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let blur = play(1, 0, a);
    for _ in 0..50 {
        let req = request(&mut e);
        assert!(
            !req.options.iter().any(|o| matches!(o,
                Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == gun)),
            "{:?}",
            req.options
        );
        if req.options.contains(&blur) {
            break;
        }
        e.choose(fallback(&req));
    }
    pick(&mut e, blur);
    pick_when(&mut e, play(0, 1, a));
    let stand_in = on(&e, "Liquefy the Mortal Coil", a).expect("the strike");
    pick_when(&mut e, ability(stand_in.0, 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_play, "diablerized");
    assert!(
        on(&e, "Liquefy the Mortal Coil", a).is_none(),
        "gone with it"
    );
}

/// Watchtower: The Wolves Feed: a Black Hand combatant strikes 2R through
/// it; its Methuselah's ready Black Hand vampire locks to cancel a Gehenna
/// card as it is played, the cost not refunded.
#[test]
fn watchtower_strikes_and_cancels() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), big(), 9, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Nails"), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let w = in_play_with_layers(&mut e, 0, "Watchtower: The Wolves Feed", None);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let strike = Choice::UseAbility {
        source: AbilitySource::Card(w),
        ability: 0,
        actor: Some(b),
        target: Target::None,
    };
    pick_when(&mut e, strike);
    let before = blood(&e, a);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), before - 2, "any Methuselah's Black Hand");

    // The cancel, of its controller's vampire.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Nails"), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), find_card("Marge Khan"), 4, false);
    let w = in_play_with_layers(&mut e, 1, "Watchtower: The Wolves Feed", None);
    hand(&mut e, 0, &["Blood Weakens", "Bonding", "Bonding"]);
    let _ = a;
    let event = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, event);
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(w),
            ability: 1,
            actor: None,
            target: Target::Minion(b),
        },
    );
    request(&mut e);
    let event = find_library("Blood Weakens");
    assert!(
        !e.state.cards.iter().any(|c| c.card == event && c.in_play),
        "canceled"
    );
}

/// A Concealed Weapon strike's terms: the weapon named.
fn name_in_hand(e: &mut Engine, card: &str) {
    pick_when(
        e,
        Choice::EmployFromHand {
            card: find_library(card),
            mode: 0,
        },
    );
}

/// Baseball Bat put on by Concealed Weapon: its "do not replace" is the
/// equip action's, so it is replaced at once. Played normally, blocked, it
/// waits for the end of the action, after the combats.
// Rulings: [LSJ 20080702-2]; [LSJ 20001120]; [LSJ 20101021].
#[test]
fn baseball_bat_replaced_at_once_unless_played_normally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Concealed Weapon", "Baseball Bat"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert!(on(&e, "Baseball Bat", a).is_some());
    assert_eq!(e.state.seats[0].held.total(), 0, "replaced at once");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), anarch(&[], 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["ser"];
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Baseball Bat"]);
    hand(&mut e, 1, &["Detect Authority", "Dodge"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(e.combat().is_some());
    assert_eq!(e.state.seats[0].held.total(), 1, "not during the combat");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].held.total(), 0, "after the action");
    assert!(on(&e, "Baseball Bat", a).is_none(), "blocked");
}

/// Blade Clot: clots add up on a vampire hit twice, an older rescuer burns
/// them all; in torpor its controller keeps it there or burns it; an ally
/// burns at its unlock phase.
// Rulings: [PIB 20150122]; [LSJ 20080612-3].
#[test]
fn blade_clot_adds_up_and_is_rescued_or_burned() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["thn"];
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Topaz"), 7, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let c = in_play_with_layers(&mut e, 0, "Blade Clot", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::MovesClotsOnHit,
        source: c,
    });
    e.state.cards[c.index()].counters = 3;
    hand(&mut e, 0, &["Relentless Reaper"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, Choice::ResolveRider { card: c.0 });
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::ResolveRider { card: c.0 });
    request(&mut e);
    assert_eq!(e.state.minions[b.index()].clots, 2);

    // A torpid vampire stays, or burns.
    for burn in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
        let v = e.add_minion(SeatId(1), clan("Ventrue", 0), 2, true);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        e.state.minions[v.index()].clots = 1;
        let want = if burn {
            Choice::ChooseMinion(v)
        } else {
            Choice::Pass
        };
        let a = MinionId(0);
        pick(&mut e, action(a, ActionKind::Hunt));
        loop {
            let req = request(&mut e);
            if matches!(
                e.interrupt(),
                Some(zvtes::draws::Interrupt::ClotFate { .. })
            ) {
                pick(&mut e, want);
                break;
            }
            e.choose(fallback(&req));
        }
        request(&mut e);
        assert_eq!(e.state.minions[v.index()].in_play, !burn);
        if !burn {
            assert!(e.state.minions[v.index()].in_torpor);
        }
    }

    // An ally burns; an older rescuer takes every clot, a younger one none.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library(ALLY));
    e.state.minions[ally.index()].clots = 1;
    pick(&mut e, action(a, ActionKind::Hunt));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play, "burned");

    for (rescuer, kept) in [(big(), 0), (clan("Brujah", 2), 2)] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), rescuer, 5, false);
        e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
        let v = e.add_minion(SeatId(0), find_card("Topaz"), 1, true);
        e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        e.state.minions[v.index()].clots = 2;
        pick(
            &mut e,
            Choice::TakeAction {
                minion: r,
                action: ActionKind::Rescue,
                target: Some(v),
            },
        );
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[v.index()].in_torpor);
        assert_eq!(e.state.minions[v.index()].clots, kept);
    }
}

/// Blood Turnip: the vampire's blood becomes an opposing ally's life, and
/// the other way around; an opposing Anarch takes the blood and its
/// controller sends the card on, the blood not following it.
// Rulings: [LSJ 20080704].
#[test]
fn blood_turnip_converts_and_an_anarch_redirects() {
    let mut e = game(2);
    let ally = e.add_ally(SeatId(0), find_library(ALLY));
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Topaz"), 6, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let c = in_play_with_layers(&mut e, 1, "Blood Turnip", Some(b));
    let life = blood(&e, ally);
    pick(&mut e, bleed(ally));
    pick_when(&mut e, block(b));
    request(&mut e);
    assert_eq!(e.state.cards[c.index()].host, Some(ally));
    assert_eq!(blood(&e, ally), life + 1, "blood into life");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let x = e.add_minion(SeatId(1), anarch(&[], 0), 4, false);
    let y = e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let c = in_play_with_layers(&mut e, 0, "Blood Turnip", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, Choice::ChooseMinion(y));
    request(&mut e);
    assert_eq!(e.state.cards[c.index()].host, Some(y), "moved on");
    assert_eq!((blood(&e, x) + 1, blood(&e, y)), (5 + 1 - 1, 4));
}

/// Anonymous Freight and Children of Stone [tha] are equip actions: Carmen
/// gives their non-Camarilla vampire +1 stealth; the vampire, unlocked, may
/// equip directly the same turn.
// Rulings: [TOM 19960130]; [ANK 20180817].
#[test]
fn anonymous_freight_and_children_of_stone_are_equip_actions() {
    for (name, stealth) in [("Anonymous Freight", 3 + 1), ("Children of Stone", 2 + 1)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_discs(&["tha"]), 4, false);
        e.state.minions[a.index()].sect_override = Some(Sect::Sabbat);
        e.add_minion(SeatId(0), find_card("Carmen"), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 0, &[name, "Kevlar Vest", "Sport Bike"]);
        pick(&mut e, play(0, 0, a));
        name_in_hand(&mut e, "Kevlar Vest");
        request(&mut e);
        let act = e.action().expect("the action");
        assert_eq!(act.stealth(&e.state), stealth, "{name}");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(on(&e, "Kevlar Vest", a).is_some());
        e.state.minions[a.index()].locked = false;
        let req = request(&mut e);
        assert!(
            req.options.iter().any(|o| matches!(o,
                Choice::Play { actor: Some(m), .. } if *m == a)),
            "{:?}",
            req.options
        );
    }
}

/// Detect Authority [ani]: an action at a retainer does not target the
/// minion; none during a block attempt or after a block.
// Rulings: [LSJ 20090324]; [ANK 20260316] [LSJ 20041022].
#[test]
fn detect_authority_not_at_a_retainer_nor_with_a_block_under_way() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["dem"]), 5, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let d = e.add_minion(SeatId(1), anarch(&["ani"], 0), 4, false);
    let t = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let dog = in_play(&mut e, 1, "Vengeful Spirit", Some(t));
    hand(&mut e, 0, &["Prison of the Mind"]);
    hand(&mut e, 1, &["Detect Authority"]);
    pick(&mut e, play_at(0, 1, Some(a), Target::Card(dog)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, d));

    for (hand_card, mode, actor) in [
        ("Detect Authority", 0u8, true),
        ("Final Loosening", 2, false),
    ] {
        let mut e = game(2);
        let a = if actor {
            e.add_minion(SeatId(0), with_discs(&["DEM"]), 5, false)
        } else {
            e.add_ally(SeatId(0), find_library(ALLY))
        };
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let d = e.add_minion(SeatId(1), anarch(&[], 0), 4, false);
        e.state.minions[d.index()].gained_levels = vec!["ani", "for"];
        let t = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        hand(&mut e, 1, &[hand_card]);
        if actor {
            hand(&mut e, 0, &["Prison of the Mind"]);
            pick(&mut e, play_at(0, 2, Some(a), Target::Minion(t)));
        } else {
            pick(&mut e, bleed(a));
        }
        pick_when(&mut e, block(d));
        never_offered_until(
            &mut e,
            SeatId(0),
            WindowLabel::MinionPhase,
            play(0, mode, d),
        );
    }
}

/// Eccentric Billionaire: its lock announced with the equipment, it stays
/// unlocked if the card is canceled as played; a card otherwise out of the
/// Methuselah's means; a weapon of Concealed Weapon; on the turn it is
/// recruited.
// Rulings: [LSJ 20090705] [ANK 20170605];
// [ANK 20230620] [ANK 20170605] [LSJ 20090705];
// [ANK 20230824] [RTR 20070707]; [LSJ 20090705] [LSJ 20030917] [ANK 20230721];
// [ANK 20180517].
#[test]
fn eccentric_billionaire_locks_as_announced() {
    let pay = |e: &mut Engine| {
        let req = request(e);
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass))
            .map_or(0, |p| usize::from(p == 0));
        e.choose(i);
    };
    // Canceled as played.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_ally(SeatId(0), find_library("Eccentric Billionaire"));
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kevlar Vest"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, play(0, 0, a));
    pay(&mut e);
    pick_when(&mut e, master_play(0));
    request(&mut e);
    assert!(!e.state.minions[b.index()].locked, "canceled, not locked");

    // With the last pool: 1 left, the vest costs 1, unlocked after.
    let mut e = game(2);
    e.state.seats[0].pool = 1;
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let b = e.add_ally(SeatId(0), find_library("Eccentric Billionaire"));
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kevlar Vest"]);
    pick(&mut e, play(0, 0, a));
    pay(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 1, "no pool paid");
    assert!(e.state.minions[b.index()].locked);

    // Concealed Weapon's weapon: not an action, reduced all the same.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_ally(SeatId(0), find_library("Eccentric Billionaire"));
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Concealed Weapon", "Baseball Bat"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pay(&mut e);
    request(&mut e);
    assert!(on(&e, "Baseball Bat", a).is_some());
    assert_eq!(pool(&e)[0], 10);
    assert!(e.state.minions[b.index()].locked);

    // Recruited this turn, it locks for the next equip.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Toreador antitribu", 0), 6, false);
    let c = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Eccentric Billionaire", "Kevlar Vest"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let paid = pool(&e)[0];
    pick(&mut e, play(0, 0, c));
    pay(&mut e);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(on(&e, "Kevlar Vest", c).is_some());
    assert_eq!(pool(&e)[0], paid, "reduced");
}

/// Elixir of Distillation: a combat ended before its strike resolves leaves
/// it in play, the capacity untouched.
// Rulings: [LSJ 20001127-2] [LSJ 20010806-1] [LSJ 19981006].
#[test]
fn elixir_of_distillation_stays_if_the_combat_ends_first() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), with_discs(&["pre"]), 5, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let w = in_play(&mut e, 0, "Elixir of Distillation", Some(a));
    hand(&mut e, 1, &["Majesty"]);
    let cap = e.state.capacity_of(z);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, card_ability(w, 0));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.cards[w.index()].in_play, "not burned");
    assert_eq!(e.state.capacity_of(z), cap);
}

/// Empowering the Puppet King: its acting minion's Methuselah alone.
// Rulings: [LSJ 19990425].
#[test]
fn empowering_the_puppet_king_is_for_the_acting_methuselah() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Empowering the Puppet King"]);
    pick(&mut e, bleed(a));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == z),
    );
}

/// Grey Thorne plays [cel] or [pot] cards as an Anarch vampire, not one
/// requiring both; a ghoul, not mortal.
// Rulings: [ANK 20180731-1]; [ANK 20200203-2] [LSJ 20060515].
#[test]
fn grey_thorne_plays_one_discipline_at_a_time() {
    let mut e = game(2);
    e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let g = e.add_ally(SeatId(0), find_library("Grey Thorne"));
    hand(&mut e, 0, &["Make the Misere"]);
    let card = &zvtes::library::LIBRARY[find_library("Grey Thorne") as usize];
    assert!(card.labels.contains(&zvtes::library::Label::Ghoul));
    assert!(!card.labels.contains(&zvtes::library::Label::Mortal));
    let req = request(&mut e);
    let misere = |mode| play_at(0, mode, Some(g), Target::Minion(z));
    assert!(req.options.contains(&misere(0)), "{:?}", req.options);
    assert!(req.options.contains(&misere(2)), "[pot]");
    assert!(!req.options.contains(&misere(4)), "[cel] and [pot]");
}

/// Haven Hunting: the combat's continuation lost with the new combat; not
/// with a combat queued.
// Rulings: [LSJ 19980109] [RTR 20020501]; [RTR 20020501].
#[test]
fn haven_hunting_loses_the_after_combat_and_waits_for_a_queue() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["pro"];
    e.state.minions[a.index()].beast_disciplines[0] = Some("pro");
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Topaz"), 7, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let h = in_play(&mut e, 0, "Haven Hunting", None);
    hand(&mut e, 0, &["Form of Mist"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::ResolveRider { card: h.0 });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert_eq!(pool(&e)[1], 10, "no continuation");

    // Psyche! queues a combat: Haven Hunting is not asked.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["cel"];
    e.state.minions[a.index()].beast_disciplines[0] = Some("cel");
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Topaz"), 7, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let h = in_play(&mut e, 0, "Haven Hunting", None);
    hand(&mut e, 0, &["Psyche!"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 1, a));
    let rider = Choice::ResolveRider { card: h.0 };
    let once = offered_before(&mut e, rider, SeatId(0), WindowLabel::MinionPhase);
    assert!(once, "the queued combat's end asks, not the first's");
}

/// Keystone Kine [obf]: the ally chosen at resolution, from the bleed's
/// amount then; an imbued's cost its starting life.
// Rulings: [ANK 20240706] [LSJ 20080608]; [ANK 20220528].
#[test]
fn keystone_kine_picks_the_ally_at_resolution() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&["obf"], 0), 3, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library(ALLY));
    hand(&mut e, 0, &["Keystone Kine", "Conditioning"]);
    let mode = 2;
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    pick(&mut e, play_at(0, mode, Some(a), Target::None));
    let want = Choice::ChooseMinion(ally);
    let req = request(&mut e);
    assert!(!req.options.contains(&want), "not at declaration");
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, want);
    request(&mut e);
    assert!(
        !e.state.minions[ally.index()].in_play,
        "the bleed of 2 reaches"
    );

    // An imbued's cost is its starting life: 3 out of a bleed of 1's reach.
    let imbued = (0..CRYPT.len() as u16)
        .find(|&c| CRYPT[c as usize].imbued && CRYPT[c as usize].capacity == 3)
        .expect("an imbued of 3 life");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&["obf"], 0), 3, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let m = e.add_minion(SeatId(1), imbued, 3, false);
    hand(&mut e, 0, &["Keystone Kine"]);
    pick(&mut e, play_at(0, 2, Some(a), Target::None));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::ChooseMinion(m),
    );
    assert!(e.state.minions[m.index()].in_play);
}

/// Relentless Reaper [THN] after a strike ending the combat: the combat
/// goes on, the Molotov Cocktail whose strike never resolved stays off.
// Rulings: [ANK 20200203-1].
#[test]
fn relentless_reaper_does_not_bring_back_a_molotov_cocktail() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), superior("THN", 0), 6, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_discs(&["pre"]), 6, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Molotov Cocktail", "Relentless Reaper"]);
    hand(&mut e, 1, &["Majesty"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 2, a));
    request(&mut e);
    let molotov = find_library("Molotov Cocktail");
    assert!(!e.state.cards.iter().any(|c| c.card == molotov));
}

/// Necrosis: a hand strike's added damage is of its kind: Nizzam's stays
/// aggravated. [THN]: dodged, nothing in the press step, Target Vitals
/// adding to the strike alone; a combat ended first, nothing either.
// Rulings: [TOM 19960225]; [ANK 20200517]; [ANK 20201228-2];
// [LSJ 20010312] [LSJ 20001027].
#[test]
fn necrosis_adds_of_the_strikes_kind_and_presses_only_if_it_landed() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nizzam al-Latif"), 9, false);
    e.state.minions[n.index()].gained_levels = vec!["pot"];
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Necrosis"]);
    hand(&mut e, 1, &["Relentless Reaper"]);
    pick(&mut e, bleed(n));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, n));
    offering(&mut e, play(0, 0, z));
    let d = e.combat().expect("combat").sides[1].pending;
    assert_eq!((d.normal, d.aggravated), (0, 2), "both aggravated");

    let run = |a_hand: &[&str], z_hand: &[&str], disc: &'static str, z_mode: Option<u8>| {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_discs(&["THN"]), 3, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), find_card("Topaz"), 7, false);
        e.state.minions[b.index()].gained_levels = vec![disc];
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Necrosis"]);
        hand(&mut e, 0, a_hand);
        hand(&mut e, 1, z_hand);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(b));
        pick_when(&mut e, play(0, 2, a));
        for _ in a_hand {
            pick_when(&mut e, play(0, 0, a));
        }
        if let Some(m) = z_mode {
            pick_when(&mut e, play(0, m, b));
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        blood(&e, b)
    };
    assert_eq!(
        run(&[], &[], "pre", None),
        7 - 2 - 1 - 1,
        "strength 1 + 2, 1 more"
    );
    assert_eq!(
        run(&[], &["Dodge"], "pre", Some(0)),
        7,
        "dodged, no press damage"
    );
    assert_eq!(
        run(&[], &["Majesty"], "pre", Some(0)),
        7 - 1,
        "Majesty's cost"
    );
    let vitals = run(&["Target Vitals"], &[], "pre", None);
    assert_eq!(
        vitals,
        7 - 2 - 1 - 1 - 2,
        "2 on the strike, none on the press"
    );
}

/// No Confidence through play: after an action requiring an Anarch, on a
/// titled non-Anarch vampire of its target; two of them, no vote, and a
/// rival to the title takes it uncontested; undirected, its predator's or
/// prey's only.
// Rulings: [LSJ 20080602].
#[test]
fn no_confidence_through_play_and_title_yielded() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&["pro"], 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    e.state.minions[t.index()].granted_title = Some(Title::Prince("New York"));
    hand(&mut e, 0, &["Shattering", "No Confidence"]);
    pick(&mut e, play_at(0, 2, Some(a), Target::Minion(t)));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = on(&e, "No Confidence", t).expect("on the vampire");
    placed(&e, c);
    assert_eq!(e.state.cards[c.index()].chosen, vec![a]);

    // Undirected: Haven Hunting; the cross-table vampire is no target.
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let prey = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let cross = e.add_minion(SeatId(2), clan("Ventrue", 1), 4, false);
    e.add_minion(SeatId(3), clan("Ventrue", 2), 4, false);
    for m in [prey, cross] {
        e.state.minions[m.index()].granted_title = Some(Title::Votes(1));
    }
    hand(&mut e, 0, &["Haven Hunting", "No Confidence"]);
    pick(&mut e, play(0, 0, a));
    let req = offering(&mut e, play_at(0, 0, Some(a), Target::Minion(prey)));
    assert!(!req
        .options
        .contains(&play_at(0, 0, Some(a), Target::Minion(cross))));

    // Two: the rival entering play takes the title uncontested.
    let prince = (0..CRYPT.len() as u16)
        .find(|&c| matches!(CRYPT[c as usize].title, Title::Prince(_)))
        .expect("a printed prince");
    let city = CRYPT[prince as usize].title;
    let mut e = game(2);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    e.state.minions[t.index()].granted_title = Some(city);
    in_play_with_layers(&mut e, 0, "No Confidence", Some(t));
    in_play_with_layers(&mut e, 0, "No Confidence", Some(t));
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: prince,
            copy: Default::default(),
            counters: CRYPT[prince as usize].capacity,
            banished: None,
        });
    pick_when(&mut e, Choice::InfluenceOut { slot: 0 });
    request(&mut e);
    let v = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(e.state.title_of(v), city);
    assert!(!e.state.minions[t.index()].title_contested, "yielded");
    assert_eq!(e.state.votes_held(t), 0);
}

/// Seraph's Second: for a vampire whose own action made it Black Hand (the
/// Mateusz way); after a combat too, the Seraph ready.
// Rulings: [LSJ 20070702]; [LSJ 20030130] [ANK 20181219] [ANK 20180906].
#[test]
fn seraphs_second_after_a_block_and_for_a_new_black_hand() {
    let mut e = game(2);
    let m = e.add_minion(
        SeatId(0),
        find_card("Mateusz Kazimirski, Biker of Blood"),
        4,
        false,
    );
    let s = e.add_minion(SeatId(0), seraph(0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Seraph's Second"]);
    assert!(!e.state.is_black_hand(m));
    let become_hand = Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability: 0,
        actor: Some(m),
        target: Target::None,
    };
    pick(&mut e, become_hand);
    pick_when(&mut e, play(0, 0, s));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.is_black_hand(m));
    assert!(!e.state.minions[m.index()].locked, "unlocked");

    // After the combat; a Seraph in torpor is not ready.
    for torpid in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Nails"), 4, false);
        let s = e.add_minion(SeatId(0), seraph(0), 4, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        e.state.minions[s.index()].in_torpor = torpid;
        hand(&mut e, 0, &["Seraph's Second"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        if torpid {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, s));
        } else {
            pick_when(&mut e, play(0, 0, s));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(!e.state.minions[a.index()].locked);
        }
    }
}

/// Shoulder Drop: its damage after the strikes, before the press step, and
/// the opposing minion cannot press this round.
// Rulings: [ANK 20171225].
#[test]
fn shoulder_drop_bars_the_opposing_press() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Topaz"), 7, false);
    e.state.minions[b.index()].gained_levels = vec!["thn"];
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Shoulder Drop"]);
    hand(&mut e, 1, &["Relentless Reaper"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 1, b));
}

/// [MERGED]: a merge takes the advanced card's sect: Reverend Adams to
/// Camarilla, Louis Fortier to Anarch; an anarch token burned with it.
// Rulings: [ANK 20180524].
#[test]
fn merging_takes_the_advanced_cards_sect() {
    for (name, want) in [
        ("Reverend Adams", Sect::Camarilla),
        ("Louis Fortier", Sect::Anarch),
    ] {
        let of = |adv: bool| {
            CRYPT
                .iter()
                .position(|c| c.name == name && c.adv == adv)
                .unwrap() as u16
        };
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of(false), 4, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        e.state.seats[0].crypt.push(of(true));
        e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
        hand(&mut e, 0, &["Epiphany"]);
        pick(&mut e, play(0, 0, a));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(e.state.minions[a.index()].merged.is_some());
        assert_eq!(e.state.sect_of(a), Some(want), "{name}");
    }
}

/// Marie-Pierre: only as the combat has ended, after a "would end" card;
/// not acting then, no action modifier, no Obedience; the first combat's
/// continuation lost.
// Rulings: [ANK 20180928-2]; [ANK 20180929];
// [LSJ 19980109] [RTR 20020501] [ANK 20180928-2].
#[test]
fn marie_pierre_comes_in_after_the_combat_has_ended() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), clan("Samedi", 0), 4, false);
    let mp = e.add_minion(SeatId(0), find_card("Marie-Pierre"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), superior("THN", 0), 6, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Relentless Reaper"]);
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 2, z));
    let join = Choice::ChooseMinion(mp);
    let round = e.combat().map(|c| c.round);
    offering(&mut e, join);
    assert_ne!(e.combat().map(|c| c.round), round, "a second round first");
    e.notes = Some(Vec::new());
    pick(&mut e, join);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[mp.index()].locked);
    assert_eq!(fought(&noted_combats(&mut e)), vec![[mp, z]]);

    // A strike ending the combat with a continuation: lost; no modifier.
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), clan("Samedi", 0), 6, false);
    e.state.minions[s.index()].gained_levels = vec!["pro"];
    e.state.minions[s.index()].beast_disciplines[0] = Some("pro");
    let mp = e.add_minion(SeatId(0), find_card("Marie-Pierre"), 4, false);
    e.state.minions[mp.index()].gained_levels = vec!["dom"];
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), big(), 9, false);
    e.state.minions[z.index()].gained_levels = vec!["dom"];
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Form of Mist", "Conditioning"]);
    hand(&mut e, 1, &["Obedience"]);
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, s));
    pick_when(&mut e, Choice::ChooseMinion(mp));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == mp || *m == z),
    );
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert_eq!(pool(&e)[1], 10, "no continuation");
}

/// Michaela: in her own combat only.
// Rulings: [ANK 20221021].
#[test]
fn michaela_cancels_only_in_her_own_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    let m = e.add_minion(SeatId(1), find_card("Michaela"), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Immortal Grapple"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, own(m, 0));
}

/// Toby and Sheela Na Gig use their +1 bleed from torpor, Toby while locked;
/// none but on a bleed, before resolution.
// Rulings: [ANK 20220705] [LSJ 20100705]; [LSJ 20011022];
// [ANK 20231124] [LSJ 19970718].
#[test]
fn toby_and_sheela_na_gig_reach_a_bleed_from_torpor() {
    for (locked, torpid) in [(true, false), (false, true)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
        e.add_minion(SeatId(0), clan("Ventrue", 1), 4, false);
        let toby = e.add_minion(SeatId(1), find_card("Toby"), 3, false);
        e.state.minions[toby.index()].locked = locked;
        e.state.minions[toby.index()].in_torpor = torpid;
        e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
        pick(&mut e, bleed(a));
        pick_when(&mut e, own(toby, 0));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e)[1], 10 - 2, "locked {locked}, torpor {torpid}");
    }
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Tremere antitribu", 0), 4, false);
    let s = e.add_minion(SeatId(0), find_card("Sheela Na Gig"), 2, true);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, own(s, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 2, "from torpor");

    // A hunt: neither Toby nor a Census Taker's bearer is offered.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Gatjil Munyarryun"), 2, false);
    let t = e.add_minion(SeatId(0), find_card("Nails"), 4, false);
    let c = in_play(&mut e, 0, "Census Taker", Some(t));
    let toby = e.add_minion(SeatId(1), find_card("Toby"), 3, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    pick(&mut e, action(a, ActionKind::Hunt));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == own(toby, 0) || *o == card_ability(c, 0)
    });
}

/// Topaz swaps an equipment put on by Concealed Weapon too.
// Rulings: [PIB 20150105-2].
#[test]
fn topaz_swaps_a_weapon_put_on_in_combat() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Topaz"), 7, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Concealed Weapon", ".44 Magnum"]);
    hand(&mut e, 1, &["Sport Bike"]);
    let bike = find_library("Sport Bike");
    pick(&mut e, bleed(t));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, t));
    pick_when(&mut e, Choice::FindCard(bike));
    request(&mut e);
    assert!(on(&e, "Sport Bike", t).is_some());
    assert!(ash_has(&e, 0, ".44 Magnum"), "the original discarded");
}

/// Tribunal Judgment: the latest referendum since its Methuselah's last
/// turn: this turn's, another's, not the one in that last turn; no vampire
/// who voted lately, the card is not offered.
// Rulings: [LSJ 20020429]; [LSJ 20100129] [ANK 20191228].
#[test]
fn tribunal_judgment_reads_since_your_last_turn() {
    for (turn, offered) in [(4, true), (3, true), (2, false)] {
        let mut e = game(2);
        let c = e.add_minion(SeatId(0), find_card("Hagar Stone"), 4, false);
        e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
        let t = e.add_minion(SeatId(1), clan("Ventrue", 0), 5, false);
        e.state.cast_votes = (turn, vec![(t, 2)]);
        hand(&mut e, 0, &["Tribunal Judgment"]);
        let req = request(&mut e);
        assert_eq!(req.options.contains(&play(0, 0, c)), offered, "{turn}");
    }
}

/// Joseph Pander: its two actions are not cross-restrictive: the chosen
/// Sabbat Pander enters combat, unlocked burns the card the same turn.
// Rulings: [LSJ 20100909].
#[test]
fn joseph_panders_two_actions_do_not_restrict_each_other() {
    let mut e = game(2);
    let pander = CRYPT
        .iter()
        .position(|c| c.clan == "Pander")
        .expect("a Pander") as u16;
    let p = e.add_minion(SeatId(0), pander, 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let j = in_play(&mut e, 0, "Joseph Pander", None);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(j),
            ability: 1,
            actor: None,
            target: Target::Minion(p),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(j),
            ability: 2,
            actor: Some(p),
            target: Target::Minion(z),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[p.index()].locked = false;
    let req = request(&mut e);
    assert!(
        req.options.iter().any(|o| matches!(o,
            Choice::UseAbility { source: AbilitySource::Card(c), ability: 3, actor: Some(m), .. }
                if *c == j && *m == p)),
        "{:?}",
        req.options
    );
}

/// Nuriel's and Blessed Blade's added damage is the weapon's kind: a Femur
/// of Toomler's aggravated strike stays aggravated.
// Rulings: [RTR 19970630] [TOM 19960225].
#[test]
fn added_melee_damage_inherits_aggravation() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nuriel"), 8, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let w = in_play(&mut e, 0, "Femur of Toomler", Some(n));
    hand(&mut e, 1, &["Relentless Reaper"]);
    pick(&mut e, bleed(n));
    pick_when(&mut e, block(z));
    pick_when(&mut e, card_ability(w, 0));
    offering(&mut e, play(0, 0, z));
    let d = e.combat().expect("combat").sides[1].pending;
    assert_eq!((d.normal, d.aggravated), (0, 3), "strength 1 + 1 + 1");

    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Lailah"), 6, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let w = in_play(&mut e, 0, "Femur of Toomler", Some(n));
    hand(&mut e, 0, &["Blessed Blade"]);
    hand(&mut e, 1, &["Relentless Reaper"]);
    pick(&mut e, bleed(n));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play_at(0, 1, Some(n), Target::Card(w)));
    offering(&mut e, play(0, 0, z));
    let d = e.combat().expect("combat").sides[1].pending;
    assert_eq!(
        (d.normal, d.aggravated),
        (0, 1 + 1 + 2),
        "Blessed Blade [val]"
    );
}

/// Power of All: usable with no action, in another's master phase; a
/// canceled modifier's cost still paid; not during a combat.
// Rulings: [PIB 20120808]; [ANK 20260216]; [LSJ 20080618].
#[test]
fn power_of_all_outside_actions_and_not_in_combat() {
    let setup = |e: &mut Engine| {
        let x = e.add_minion(SeatId(1), anarch(&[], 0), 4, false);
        let y = e.add_minion(SeatId(1), anarch(&[], 1), 4, false);
        hand(e, 1, &["Power of All"]);
        (x, y)
    };
    // A master card, no action.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let (x, _) = setup(&mut e);
    hand(&mut e, 0, &["Blood Doll"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, play_at(0, 0, None, Target::Minion(a)));
    pick_when(&mut e, play(0, 0, x));
    request(&mut e);
    assert!(on(&e, "Blood Doll", a).is_none(), "canceled");
    assert!(e.state.minions[x.index()].locked);

    // A modifier's blood.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), vampire_with("dom", 0), 4, false);
    let (x, _) = setup(&mut e);
    hand(&mut e, 0, &["Empowering the Puppet King"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, b));
    pick_when(&mut e, play(0, 0, x));
    request(&mut e);
    assert_eq!(blood(&e, b), 4 - 1, "cost paid");
    assert!(ash_has(&e, 0, "Empowering the Puppet King"));

    // During a combat: not offered.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let (x, _) = setup(&mut e);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Immortal Grapple"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
}

/// Mambo Jeanne's 2 environmental damage: through a dodge, not after a
/// strike ending the combat.
// Rulings: [RTR 19941109] [TOM 19951216] [PIB 20140324] [ANK 20200422].
#[test]
fn mambo_jeanne_damages_through_a_dodge_not_a_combat_end() {
    for (card, left) in [("Dodge", 9 - 2), ("Majesty", 9 - 1)] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Mambo Jeanne"), 9, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), big(), 9, false);
        e.state.minions[z.index()].gained_levels = vec!["pre"];
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 1, &[card]);
        pick(&mut e, bleed(m));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 0, z));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), left, "{card}");
    }
}

/// Trophy: Chosen burned instead: no blood, no unlock, not Sabbat; taken, a
/// Black Hand trait kept through a sect change while the card stays.
// Rulings: [LSJ 20070309-1]; [LSJ 20070322] [ANK 20180807].
#[test]
fn trophy_chosen_burned_instead_does_nothing() {
    let red = (0..CRYPT.len() as u16)
        .find(|&c| CRYPT[c as usize].red_list)
        .expect("a Red List vampire");
    for take in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), clan("Brujah", 0), 2, false);
        e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
        let v = e.add_minion(SeatId(1), red, 2, true);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Trophy: Chosen"]);
        let sect = e.state.sect_of(a);
        pick(
            &mut e,
            Choice::TakeAction {
                minion: a,
                action: ActionKind::Diablerie,
                target: Some(v),
            },
        );
        pick_when(&mut e, Choice::FindCard(find_library("Trophy: Chosen")));
        let card = on(&e, "Trophy: Chosen", a).expect("on the vampire");
        if take {
            pick(&mut e, Choice::ResolveRider { card: card.0 });
            request(&mut e);
            assert_eq!(e.state.sect_of(a), Some(Sect::Sabbat));
            e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
            assert!(e.state.is_black_hand(a), "kept through a sect change");
        } else {
            let before = blood(&e, a);
            pick(&mut e, Choice::Pass);
            request(&mut e);
            assert!(on(&e, "Trophy: Chosen", a).is_none(), "burned");
            assert_eq!(blood(&e, a), before);
            assert_eq!(e.state.sect_of(a), sect);
            assert!(!e.state.is_black_hand(a));
            assert!(e.state.minions[a.index()].locked, "no unlock");
        }
    }
}

/// Anonymous Freight with a Baseball Bat: an equip action of the Bat: replaced
/// after the action, the vampire unlocking at the end of the turn; its pool
/// paid down to 0 ousts its Methuselah.
#[test]
fn anonymous_freight_keeps_the_bats_terms() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Anonymous Freight", "Baseball Bat"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(on(&e, "Baseball Bat", a).is_some());
    assert!(!e.state.minions[a.index()].locked, "unlocked");
}

/// Blood Turnip moves once: a non-Anarch pair, its blood with it (the card
/// not carried back as the other side is read).
#[test]
fn blood_turnip_moves_once_between_non_anarchs() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Ventrue", 2), 4, false);
    e.add_minion(SeatId(0), clan("Ventrue", 3), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Topaz"), 7, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let c = in_play_with_layers(&mut e, 1, "Blood Turnip", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    request(&mut e);
    assert_eq!(e.state.cards[c.index()].host, Some(b));
}

/// Failsafe is a trifle: it gives back its master phase action.
#[test]
fn failsafe_is_a_trifle() {
    assert!(
        zvtes::cards::lookup(find_library("Failsafe"))
            .expect("defined")
            .trifle
    );
}

/// Framing an Ancient Grudge at exactly 3 pool: the justicar's price ousts.
#[test]
fn framing_an_ancient_grudge_ousts_at_three_pool() {
    let mut e = game(3);
    e.add_minion(SeatId(0), clan("Ventrue antitribu", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), clan("Ventrue", 2), 4, false);
    e.add_minion(SeatId(2), clan("Ventrue", 3), 4, false);
    e.state.minions[t.index()].granted_title = Some(Title::Justicar("Brujah"));
    e.state.seats[0].pool = 3;
    hand(&mut e, 0, &["Framing an Ancient Grudge"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, play_at(0, 1, None, Target::Minion(t)));
    request(&mut e);
    assert!(e.state.seats[0].ousted);
}

/// Duality with Learjet on a held replacement: both draws wait with the card
/// (each source its own), both offered as Hagar locks.
#[test]
fn two_sources_on_one_delayed_replacement_keep_both_draws() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Duality"), 5, false);
    let h = e.add_minion(SeatId(0), find_card("Hagar Stone"), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    in_play_with_layers(&mut e, 0, "Learjet", Some(d));
    hand(&mut e, 0, &["Horrid Form"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, bleed(d));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, d));
    request(&mut e);
    assert_eq!(e.state.seats[0].held_draws.len(), 2);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, action(h, ActionKind::Hunt));
    assert!(extra_draw(&mut e).is_some());
    pick(&mut e, Choice::DrawLibrary);
    assert!(extra_draw(&mut e).is_some(), "the second source");
}

/// Capuchin beside a draw alternative: the replacement is a choice, the
/// extra draw waits behind it and goes if Capuchin takes it.
#[test]
fn a_deferred_replacement_capuchin_takes_brings_no_extra_draw() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), superior("dom", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    in_play_with_layers(&mut e, 0, "Learjet", Some(a));
    let c = capuchin(&mut e, 2);
    let cache = in_play(&mut e, 0, "Black Market Cache", None);
    e.state.cards[cache.index()].stored = vec![find_library("Kevlar Vest")].into();
    hand(&mut e, 0, &["Govern the Unaligned"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Bonding"); 5].into();
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ResolveRider { card: c.0 });
    assert_eq!(extra_draw(&mut e), None, "no draw, no extra one");
}

/// Liquefy [VIC]'s diablerie ends the combat: Marie-Pierre is not offered
/// the join, and the blood hunt is out of combat.
#[test]
fn liquefy_diablerie_closes_the_end_of_combat_offers() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), big(), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["vic", "thn", "thn", "cel"];
    e.state.minions[a.index()].beast_disciplines[0] = Some("vic");
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Samedi", 0), 2, false);
    let mp = e.add_minion(SeatId(1), find_card("Marie-Pierre"), 4, false);
    hand(
        &mut e,
        0,
        &[
            "Liquefy the Mortal Coil",
            "Relentless Reaper",
            "Blur",
            "Relentless Reaper",
        ],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let blur = play(1, 0, a);
    for _ in 0..50 {
        let req = request(&mut e);
        if req.options.contains(&blur) {
            break;
        }
        e.choose(fallback(&req));
    }
    pick(&mut e, blur);
    pick_when(&mut e, play(0, 1, a));
    let stand_in = on(&e, "Liquefy the Mortal Coil", a).expect("the strike");
    pick_when(&mut e, ability(stand_in.0, 0));
    for _ in 0..40 {
        if !e.state.minions[z.index()].in_play {
            break;
        }
        let req = request(&mut e);
        assert!(!req.options.contains(&Choice::ChooseMinion(mp)), "no join");
        assert!(
            !req.options
                .iter()
                .any(|o| matches!(o, Choice::Play { mode: 2, .. })),
            "no new round"
        );
        e.choose(fallback(&req));
    }
    assert!(!e.state.minions[z.index()].in_play, "diablerized");
    assert!(e.combat().is_none());
    let a = e.action().expect("its blood hunt");
    assert_eq!(a.outcome, Some(zvtes::actions::Outcome::Blocked));
}

/// Enrage [PRE]: the Red List hunt enters combat, free of the toll: offered
/// below it.
#[test]
fn enrage_leaves_the_red_list_hunt_free() {
    let red = (0..CRYPT.len() as u16)
        .find(|&c| CRYPT[c as usize].red_list)
        .expect("a Red List vampire");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), red, 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let c = in_play(&mut e, 0, "Enrage", Some(a));
    e.state.cards[c.index()].mode = 1;
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostActionToll(2),
        source: c,
    });
    e.state.red_list_marks.push((SeatId(0), v));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&Choice::TakeAction {
        minion: a,
        action: ActionKind::RedListHunt,
        target: Some(v),
    }));
    assert!(!req.options.contains(&bleed(a)), "the toll bars a bleed");
}

/// Liquefy [VIC]'s strike is a choice: a plain hand strike that round
/// diablerizes nothing, and the stand-in goes with the combat.
#[test]
fn liquefy_superior_strike_is_a_choice() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), big(), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["vic", "thn", "cel"];
    e.state.minions[a.index()].beast_disciplines[0] = Some("vic");
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), clan("Ventrue", 0), 2, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(
        &mut e,
        0,
        &["Liquefy the Mortal Coil", "Relentless Reaper", "Blur"],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let blur = play(1, 0, a);
    for _ in 0..50 {
        let req = request(&mut e);
        if req.options.contains(&blur) {
            break;
        }
        e.choose(fallback(&req));
    }
    pick(&mut e, blur);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::HandStrike);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[z.index()].in_play,
        "not diablerized by a hand strike"
    );
    assert!(on(&e, "Liquefy the Mortal Coil", a).is_none());
}

/// Final Loosening [aus]: any blood gain of the acting vampire, a card's:
/// instead each gains 1 (owner, 2026-11-02).
#[test]
fn final_loosening_splits_any_gain() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&["cel"], 0), 2, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let r = e.add_minion(SeatId(1), anarch(&["aus"], 0), 2, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Keystone Kine"]);
    hand(&mut e, 1, &["Final Loosening"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, r));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 3);
    assert_eq!(blood(&e, r), 3);
}

/// Topaz: the original never enters: "instead" (owner, 2026-11-02); waiting
/// on the swap, its copy is in flight (`copies::census`).
#[test]
fn topaz_swap_keeps_the_original_out_of_play() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Topaz"), 7, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kevlar Vest"]);
    hand(&mut e, 1, &["Sport Bike"]);
    let n = e.state.cards.len();
    pick(&mut e, play(0, 0, t));
    offering(&mut e, Choice::FindCard(find_library("Sport Bike")));
    assert_eq!(e.in_flight().len(), 1, "the vest, in flight");
    pick(&mut e, Choice::FindCard(find_library("Sport Bike")));
    request(&mut e);
    assert!(
        !e.state.cards[n..]
            .iter()
            .any(|c| c.card == find_library("Kevlar Vest")),
        "never in play"
    );
    assert!(ash_has(&e, 0, "Kevlar Vest"));
}

/// Blood Turnip: an opposing anarch moves it onto any minion, another
/// Methuselah's too, its controller choosing (owner, 2026-11-02).
#[test]
fn blood_turnip_goes_on_any_minion() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), clan("Ventrue", 1), 4, false);
    let b = e.add_minion(SeatId(1), anarch(&["aus"], 0), 4, false);
    e.state.minions[b.index()].granted_title = None;
    e.add_minion(SeatId(1), clan("Ventrue", 2), 4, false);
    let third = e.add_minion(SeatId(2), clan("Ventrue", 3), 4, false);
    let c = in_play_with_layers(&mut e, 1, "Blood Turnip", Some(a));
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, Choice::ChooseMinion(third));
    request(&mut e);
    assert_eq!(e.state.cards[c.index()].host, Some(third));
}

/// Grey Thorne: any Methuselah's ready anarch, its controller choosing
/// (owner, 2026-11-02).
#[test]
fn grey_thorne_is_healed_by_any_methuselahs_anarch() {
    let mut e = game(2);
    let g = e.add_ally(SeatId(0), find_library("Grey Thorne"));
    e.add_minion(SeatId(0), clan("Ventrue", 2), 4, false);
    let a = e.add_minion(SeatId(1), anarch(&[], 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    e.state.minions[g.index()].blood = 1;
    in_play(&mut e, 0, "Grey Thorne", Some(g));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, Choice::ChooseMinion(a));
    request(&mut e);
    assert_eq!(e.state.minions[g.index()].blood, 2);
    assert_eq!(blood(&e, a), 3);
}

/// Trophy: Chosen: a Black Hand vampire is a Seraph only while the card is on
/// it (owner, 2026-11-02).
#[test]
fn trophy_chosen_makes_a_seraph_only_while_on_it() {
    let red = (0..CRYPT.len() as u16)
        .find(|&c| CRYPT[c as usize].red_list)
        .expect("a Red List vampire");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Nails"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), red, 2, true);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Trophy: Chosen"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Diablerie,
            target: Some(v),
        },
    );
    pick_when(&mut e, Choice::FindCard(find_library("Trophy: Chosen")));
    let card = on(&e, "Trophy: Chosen", a).expect("on the vampire");
    pick(&mut e, Choice::ResolveRider { card: card.0 });
    request(&mut e);
    assert!(e.state.is_seraph(a));
    e.state.cards[card.index()].in_play = false;
    assert!(!e.state.is_seraph(a), "gone with the card");
}

/// Final Loosening [aus] on a gain that sets the blood outright: Touch of
/// Valeren on the acting vampire itself.
#[test]
fn final_loosening_splits_a_set_gain_too() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 2, false);
    let r = e.add_minion(SeatId(1), anarch(&[], 1), 2, false);
    e.state.gain_split = Some((a, r));
    e.state.set_blood(a, 4);
    assert_eq!((blood(&e, a), blood(&e, r)), (3, 3));
    assert_eq!(e.state.gain_split, None, "once");
}

/// The stand-in goes with a combat cut short with the turn.
#[test]
fn a_cut_short_combat_burns_its_stand_in() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), big(), 6, false);
    e.state.minions[b.index()].gained_levels = vec!["vic"];
    e.state.minions[b.index()].beast_disciplines[0] = Some("vic");
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), clan("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), clan("Ventrue", 2), 4, false);
    in_play(&mut e, 0, ".44 Magnum", Some(a));
    hand(&mut e, 1, &["Liquefy the Mortal Coil"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 1, b));
    request(&mut e);
    assert!(on(&e, "Liquefy the Mortal Coil", b).is_some());
    e.at_next_turn();
    assert!(on(&e, "Liquefy the Mortal Coil", b).is_none());
}

/// Final Loosening [aus] on the blood Blood Turnip moves: a gain like any
/// other, the split spent.
#[test]
fn final_loosening_splits_a_turnips_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Ventrue", 2), 2, false);
    let r = e.add_minion(SeatId(0), clan("Ventrue", 3), 2, false);
    let b = e.add_minion(SeatId(1), find_card("Topaz"), 7, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    in_play_with_layers(&mut e, 1, "Blood Turnip", Some(b));
    e.state.gain_split = Some((a, r));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    request(&mut e);
    assert_eq!(e.state.gain_split, None, "spent");
    assert_eq!(blood(&e, r), 3);
}

/// Final Loosening [aus]: a reacting vampire gone by then gains nothing.
#[test]
fn final_loosening_gives_nothing_to_a_vampire_gone() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 2, false);
    let r = e.add_minion(SeatId(1), anarch(&[], 1), 2, false);
    e.state.gain_split = Some((a, r));
    e.state.minions[r.index()].in_play = false;
    e.state.set_blood(a, 4);
    assert_eq!((blood(&e, a), blood(&e, r)), (3, 2));
}

/// Grey Thorne's offers run from the active Methuselah clockwise: its own
/// Methuselah on their turn, before an earlier seat (rules-qa 107.25).
#[test]
fn grey_thorne_asks_from_the_active_methuselah() {
    let mut e = game(2);
    let early = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    e.add_minion(SeatId(0), clan("Ventrue", 2), 4, false);
    let g = e.add_ally(SeatId(1), find_library("Grey Thorne"));
    let own = e.add_minion(SeatId(1), anarch(&[], 1), 4, false);
    e.state.minions[g.index()].blood = 1;
    in_play(&mut e, 1, "Grey Thorne", Some(g));
    for _ in 0..40 {
        let req = request(&mut e);
        assert!(
            !req.options.contains(&Choice::ChooseMinion(early)),
            "seat 1 is asked first"
        );
        if req.options.contains(&Choice::ChooseMinion(own)) {
            pick(&mut e, Choice::ChooseMinion(own));
            break;
        }
        e.choose(fallback(&req));
    }
    request(&mut e);
    assert_eq!(e.state.minions[g.index()].blood, 2);
    assert_eq!(blood(&e, early), 4, "healed once, by the first asked");
}
