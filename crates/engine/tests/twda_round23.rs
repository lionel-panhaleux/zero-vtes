//! Round 23 (`ADDED_BATCHES`, slice 16ai): The Unaligned completed and the
//! decks metric's last paying step (owner, 2026-10-29). Readings in rules-qa
//! Batch 106. Windows with a single option are elided, so the scripts only
//! stop where a real decision exists.

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay, PrintedCtx};

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

/// The first vampire ability of `m` the request offers.
fn ability_of(req: &zvtes::decision::DecisionRequest, m: MinionId) -> Option<Choice> {
    req.options.iter().copied().find(
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == m),
    )
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

fn vs(o: MinionId) -> PrintedCtx {
    PrintedCtx {
        opponent: Some(o),
        ..Default::default()
    }
}

// ---- Vampires: printed modifiers --------------------------------------------

// Adri: "+1 intercept against Followers of Set" (Ministry, read live).
// Iago Castille: "+1 intercept". Ivan Krenyenko: equipment 2 fewer, +1
// bleed. Imanna: +1 vote in referendums she calls.
#[test]
fn printed_intercept_bleed_votes_and_costs() {
    let mut e = game(2);
    let adri = e.add_minion(SeatId(0), find_card("Adri, The Seeker"), 2, false);
    let iago = e.add_minion(SeatId(0), find_card("Iago Castille"), 4, false);
    let ivan = e.add_minion(SeatId(0), find_card("Ivan Krenyenko"), 4, false);
    let imanna = e.add_minion(SeatId(0), find_card("Imanna"), 4, false);
    let set = e.add_minion(SeatId(1), clan("Ministry", 0), 4, false);
    let other = e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    let p = |e: &Engine, m, s, c| e.state.printed_mod(m, s, c);
    assert_eq!(p(&e, adri, Stat::Intercept, vs(set)), 1);
    assert_eq!(p(&e, adri, Stat::Intercept, vs(other)), 0);
    assert_eq!(p(&e, iago, Stat::Intercept, vs(other)), 1);
    assert_eq!(p(&e, ivan, Stat::Bleed, Default::default()), 1);
    let off = e
        .state
        .printed_cost_off(ivan, find_library("Kevlar Vest"), 0);
    assert_eq!(off.either, 2);
    let calling = |c| PrintedCtx {
        calling: Some(c),
        ..Default::default()
    };
    assert_eq!(p(&e, imanna, Stat::Votes, calling(imanna)), 1);
    assert_eq!(p(&e, imanna, Stat::Votes, calling(adri)), 0);
}

// Delizbieta of the Dark Eyes: "+1 intercept during actions directed at a
// Ravnos you control" — one in torpor too; not a bleed at her Methuselah.
#[test]
fn delizbieta_intercepts_for_a_ravnos_targeted() {
    let mut e = game(2);
    let d = e.add_minion(
        SeatId(0),
        find_card("Delizbieta of the Dark Eyes"),
        4,
        false,
    );
    let r = e.add_minion(SeatId(0), clan("Ravnos", 0), 4, true);
    let b = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let at = |t: Option<MinionId>| PrintedCtx {
        directed: Some(true),
        target_minions: [t, None],
        ..Default::default()
    };
    assert_eq!(e.state.printed_mod(d, Stat::Intercept, at(Some(r))), 1);
    assert_eq!(e.state.printed_mod(d, Stat::Intercept, at(Some(b))), 0);
    assert_eq!(e.state.printed_mod(d, Stat::Intercept, at(None)), 0);
}

// Elizabeth Dimitros: "/Hesha Ruhadze/ cannot block Elizabeth." Any Hesha.
#[test]
fn hesha_cannot_block_elizabeth_dimitros() {
    let mut e = game(2);
    let el = e.add_minion(SeatId(0), find_card("Elizabeth Dimitros"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let h = e.add_minion(SeatId(1), find_card("Hesha Ruhadze"), 4, false);
    let o = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    pick(&mut e, bleed(el));
    let req = request(&mut e);
    assert!(!req.options.contains(&block(h)), "{:?}", req.options);
    assert!(req.options.contains(&block(o)));
}

// Rosa Hernández: "+1 stealth when employing retainers."
#[test]
fn rosa_hernandez_employs_with_stealth() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rosa Hernández"), 4, false);
    let ctx = |k| PrintedCtx {
        kind: Some(k),
        ..Default::default()
    };
    use zvtes::decision::ActionKind as K;
    assert_eq!(e.state.printed_mod(r, Stat::Stealth, ctx(K::Employ)), 1);
    assert_eq!(e.state.printed_mod(r, Stat::Stealth, ctx(K::Recruit)), 0);
    assert_eq!(e.state.printed_mod(r, Stat::Stealth, ctx(K::Bleed)), 0);
    let _ = &mut e;
}

// Lisandro Giovanni: "You and Lisandro can play cards requiring a Samedi
// or a Ventrue antitribu as if he were of that required clan. +1 stealth."
// His Methuselah's Demonstration (Ventrue antitribu).
// Rulings: [LSJ 20050119] [LSJ 20101013].
// Rulings: [LSJ 20101013] [ANK 20190228] [ANK 20200705-2]
// [LSJ 20070708] [RTR 20070707].
#[test]
fn lisandro_giovanni_plays_ventrue_antitribu_cards() {
    for lisandro in [true, false] {
        let mut e = game(2);
        let v = if lisandro {
            find_card("Lisandro Giovanni")
        } else {
            clan("Giovanni", 0)
        };
        let m = e.add_minion(SeatId(0), v, 4, false);
        e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
        if lisandro {
            assert_eq!(e.state.printed_mod(m, Stat::Stealth, Default::default()), 1);
        }
        hand(&mut e, 0, &["Demonstration"]);
        e.at_master_phase();
        let req = request(&mut e);
        assert_eq!(
            req.options.contains(&master_play(0)),
            lisandro,
            "{:?}",
            req.options
        );
    }
}

// Al-Ashrad: "During the first round of combat, Al-Ashrad can use strikes
// requiring Blood Sorcery [tha] that are not usable during the first
// round": Walk of Flame in round 1.
#[test]
fn al_ashrad_strikes_with_walk_of_flame_in_the_first_round() {
    for al in [true, false] {
        let mut e = game(2);
        let v = if al {
            find_card("Al-Ashrad, Amr of Alamut")
        } else {
            vampire_with("THA", 0)
        };
        let a = e.add_minion(SeatId(0), v, 6, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), clan("Ventrue", 0), 6, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Walk of Flame"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(b));
        if al {
            let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
            assert_eq!(e.combat().expect("combat").round, 1);
            assert!(req.options.contains(&play(0, 1, a)), "{:?}", req.options);
        } else {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 1, a));
        }
    }
}

// ---- Vampires: abilities and riders -----------------------------------------

// Art Morgan: "If Art is ready during your minion phase, you can unlock a
// werewolf you control." Once in the phase.
#[test]
fn art_morgan_unlocks_a_werewolf() {
    let mut e = game(2);
    let art = e.add_minion(SeatId(0), find_card("Art Morgan"), 4, false);
    let w = e.add_ally(SeatId(0), find_library("Ossian"));
    e.state.minions[w.index()].locked = true;
    e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    let req = request(&mut e);
    let c = ability_of(&req, art).expect("offered");
    e.choose(index_of(&req, c));
    request(&mut e);
    assert!(!e.state.minions[w.index()].locked);
    e.state.minions[w.index()].locked = true;
    let req = request(&mut e);
    assert!(ability_of(&req, art).is_none(), "once in the phase");
}

// Glycon: "can enter combat with a locked minion as a Ⓓ action": another
// Methuselah's locked minion, not an unlocked one.
// Rulings: [ANK 20210124] [LSJ 20070411] [LSJ 20090514].
#[test]
fn glycon_enters_combat_with_a_locked_minion() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Glycon"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let l = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.state.minions[l.index()].locked = true;
    let u = e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let req = request(&mut e);
    let rush = |t| Choice::UseAbility {
        source: AbilitySource::Vampire(g),
        ability: 0,
        actor: Some(g),
        target: Target::Minion(t),
    };
    assert!(req.options.contains(&rush(l)), "{:?}", req.options);
    assert!(!req.options.contains(&rush(u)));
}

// Karmenita Yoryari: "can burn an equipment or a location as a Ⓓ action
// that costs 1 blood."
#[test]
fn karmenita_burns_an_equipment() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Karmenita Yoryari"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let vest = in_play(&mut e, 1, "Kevlar Vest", Some(v));
    let burn = Choice::UseAbility {
        source: AbilitySource::Vampire(k),
        ability: 0,
        actor: Some(k),
        target: Target::Card(vest),
    };
    pick(&mut e, burn);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[vest.index()].in_play, "burned");
    assert_eq!(e.state.minions[k.index()].blood, 3);
}

// Lupo Giovanni: "If Lupo is ready during your unlock phase, another ready
// Giovanni you control can gain 1 blood."
#[test]
fn lupo_giovanni_feeds_another_giovanni() {
    let mut e = game(2);
    let lupo = e.add_minion(SeatId(1), find_card("Lupo Giovanni"), 4, false);
    let g = e.add_minion(SeatId(1), clan("Giovanni", 0), 2, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    let feed = Choice::UseAbility {
        source: AbilitySource::Vampire(lupo),
        ability: 0,
        actor: None,
        target: Target::Minion(g),
    };
    assert!(req.options.contains(&feed), "{:?}", req.options);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::UseAbility { target: Target::Minion(t), .. } if *t == lupo)));
    e.choose(index_of(&req, feed));
    request(&mut e);
    assert_eq!(e.state.minions[g.index()].blood, 3);
}

// Madeleine Giovanni: "After a minion in combat with Madeleine leaves the
// ready region, their controller burns 1 pool."
#[test]
fn madeleine_giovanni_taxes_a_minion_sent_to_torpor() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(1), find_card("Madeleine Giovanni"), 6, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    pick(&mut e, bleed(a));
    // Bloodless once announced: Madeleine's hand strike sends it to torpor.
    e.state.minions[a.index()].blood = 0;
    pick_when(&mut e, block(m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].in_torpor);
    assert_eq!(e.state.seats[0].pool, 9, "its controller burned 1");
}

// Bez Dunsirn: "After Bez leaves the ready region, discard 2 cards at
// random" — his controller's, then replaced.
#[test]
fn bez_dunsirn_discards_two_as_he_leaves_ready() {
    let mut e = game(2);
    let bez = e.add_minion(SeatId(1), find_card("Bez Dunsirn"), 0, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    hand(&mut e, 1, &["Kevlar Vest", "Kevlar Vest", "Kevlar Vest"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(bez));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[bez.index()].in_torpor);
    let ash = e.state.seats[1]
        .ash
        .iter()
        .filter(|&&c| c == find_library("Kevlar Vest"))
        .count();
    assert_eq!(ash, 2, "two discarded");
}

// Count Jocalo: after his prey uses a discard phase action, his Methuselah
// looks at that hand. +1 bleed, +1 hand size.
#[test]
fn count_jocalo_looks_after_a_discard_phase_action() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(1), find_card("Count Jocalo"), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    assert_eq!(
        e.state.printed_mod(j, Stat::HandSize, Default::default()),
        1
    );
    hand(&mut e, 0, &["Kevlar Vest"]);
    e.notes = Some(Vec::new());
    pick_when(&mut e, Choice::Discard(0));
    request(&mut e);
    let looks = noted_looks(&mut e);
    assert!(
        looks
            .iter()
            .any(|l| l.viewer == SeatId(1) && l.seat == SeatId(0) && !l.library),
        "{looks:?}"
    );
}

// Rebekah: "During the polling step of any referendum, Rebekah can discard
// a card requiring Auspex to get +2 votes." Enzo Giovanni: "burn 1 blood to
// get +4 votes against the referendum."
// Rulings: [ANK 20220705].
#[test]
fn rebekah_and_enzo_buy_votes() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let r = e.add_minion(SeatId(1), find_card("Rebekah"), 4, false);
    let enzo = e.add_minion(
        SeatId(1),
        find_card("Enzo Giovanni, Pentex Board of Directors"),
        4,
        false,
    );
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, play(0, 0, caller));
    to_referendum(&mut e);
    pick_when(&mut e, own(r, 0));
    pick_when(&mut e, own(enzo, 0));
    request(&mut e);
    let rf = e.referendum().expect("referendum");
    assert!(rf.bonus.contains(&(r, 2)), "{:?}", rf.bonus);
    assert!(rf.bonus.contains(&(enzo, 4)) && rf.forced_against.contains(&enzo));
    assert_eq!(e.state.minions[enzo.index()].blood, 3);
}

// Legba: "Followers of Set controlled by other Methuselahs cannot cast
// votes or ballots in referendums called by Legba." Her own Ministry may.
#[test]
fn legba_bars_other_methuselahs_followers_of_set() {
    let mut e = game(2);
    let legba = e.add_minion(SeatId(0), find_card("Legba"), 6, false);
    let mine = e.add_minion(SeatId(0), clan("Ministry", 0), 4, false);
    e.state.minions[mine.index()].granted_title = Some(Title::Votes(1));
    let theirs = e.add_minion(SeatId(1), clan("Ministry", 1), 4, false);
    e.state.minions[theirs.index()].granted_title = Some(Title::Votes(1));
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, legba));
    to_referendum(&mut e);
    let mut seen = (false, false);
    for _ in 0..30 {
        let req = request(&mut e);
        if req.window != WindowLabel::ReferendumPolling {
            break;
        }
        seen.0 |= req.options.contains(&votes(mine, true));
        seen.1 |= req
            .options
            .iter()
            .any(|o| *o == votes(theirs, true) || *o == votes(theirs, false));
        e.choose(fallback(&req));
    }
    assert_eq!(seen, (true, false));
}

// ---- Library: Hall of Mirrors, Ophidian Gaze --------------------------------

// Hall of Mirrors: "[CHI] This vampire burns 1 blood to cancel the opposing
// minion's strike card as it is played" — the first round only.
// Rulings: [ANK 20210226].
#[test]
fn hall_of_mirrors_cancels_a_strike_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 6, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("CHI", 0), 6, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Side Strike"]);
    hand(&mut e, 1, &["Hall of Mirrors"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, b));
    request(&mut e);
    let side = find_library("Side Strike");
    let plays = noted_plays(&mut e);
    assert!(
        plays.iter().any(|p| p.card == side && p.canceled),
        "{plays:?}"
    );
}

// Ophidian Gaze: "[pre][ser] Reduce a bleed against you by 2."
#[test]
fn ophidian_gaze_reduces_a_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 1), 4, false);
    let r = e.add_minion(SeatId(1), with_discs(&["pre", "ser"]), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Legal Manipulations"]);
    hand(&mut e, 1, &["Ophidian Gaze"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9, "bled 3, 2 off");
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

/// Suspension of Disbelief [CHI]: a reaction redirecting the bleed
/// (Deflection, 1 blood printed) or reducing it (Ophidian Gaze) costs its
/// minion 1 more blood; [chi] adds nothing.
#[test]
fn suspension_of_disbelief_taxes_bleed_changers() {
    for (mode, reaction, discs) in [
        (1, "Deflection", &["dom"][..]),
        (0, "Deflection", &["dom"][..]),
        (1, "Ophidian Gaze", &["pre", "ser"][..]),
    ] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), superior("CHI", 0), 4, false);
        e.add_minion(SeatId(0), superior("CHI", 1), 4, false);
        let r = e.add_minion(SeatId(1), with_discs(discs), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        e.add_minion(SeatId(2), clan("Ventrue", 2), 4, false);
        hand(&mut e, 0, &["Suspension of Disbelief"]);
        hand(&mut e, 1, &[reaction]);
        pick(&mut e, play(0, mode, a));
        let at = if reaction == "Deflection" {
            Target::Seat(SeatId(2))
        } else {
            Target::None
        };
        pick_when(&mut e, play_at(0, 0, Some(r), at));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let paid = 4 - e.state.minions[r.index()].blood;
        let printed = u8::from(reaction == "Deflection");
        assert_eq!(
            paid,
            printed + u8::from(mode == 1),
            "{reaction} under mode {mode}"
        );
        assert!(ash_has(&e, 1, reaction), "{reaction} played");
    }
}

/// Weirding Stone: at another Methuselah's unlock phase, its bearer looks
/// at that library's top card, then may burn it for 1 blood when it is a
/// master; the burn asked only after the look, nothing shown before it.
#[test]
fn weirding_stone_burns_a_master_on_top() {
    for (top, master) in [("Pentex(TM) Subversion", true), ("Deflection", false)] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(0), clan("Ventrue", 1), 4, false);
        e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
        let stone = in_play(&mut e, 0, "Weirding Stone", Some(v));
        hand(&mut e, 1, &["Form of Mist"; 7]);
        e.state.seats[1]
            .library
            .extend([find_library("Form of Mist"), find_library(top)]);
        let look = Choice::UseAbility {
            source: AbilitySource::Card(stone),
            ability: 0,
            actor: None,
            target: Target::None,
        };
        let burn = Choice::ResolveRider { card: stone.0 };
        let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
        assert_eq!(e.state.active, SeatId(1), "{top}");
        assert_eq!(
            req.options
                .iter()
                .filter(|o| matches!(o, Choice::UseAbility { .. }))
                .count(),
            1,
            "{top}: the look alone"
        );
        pick(&mut e, look);
        if master {
            let req = request(&mut e);
            assert_eq!(
                req.options,
                vec![Choice::Pass, burn],
                "{top}: the burn, after"
            );
            pick(&mut e, burn);
            request(&mut e);
            assert_eq!(e.state.minions[v.index()].blood, 3, "1 blood burned");
            assert!(ash_has(&e, 1, top), "the master burned");
        } else {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, burn);
            assert_eq!(e.state.seats[1].library.last(), Some(&find_library(top)));
        }
        never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, look);
    }
}

/// Steely Tenacity: replaced only as its action ends, by taking it back
/// from the ash heap or by drawing (LSJ 20001120); an Anarch's only.
// Rulings: [LSJ 20001120].
#[test]
fn steely_tenacity_comes_back_from_the_ash_heap() {
    let steely = find_library("Steely Tenacity");
    for retake in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), anarch(&["dom"], 0), 4, false);
        let other = e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
        let mut cards = vec!["Steely Tenacity"];
        cards.extend(["Form of Mist"; 6]);
        hand(&mut e, 0, &cards);
        let top = find_library("Deflection");
        e.state.seats[0].library.extend([top]);
        assert!(
            !request(&mut e).options.contains(&play(0, 2, other)),
            "not a Ventrue's"
        );
        pick(&mut e, play(0, 2, a));
        assert_eq!(e.state.seats[0].hand.len(), 6, "not replaced yet");
        let req = until(&mut e, SeatId(0), WindowLabel::Draw);
        assert_eq!(e.state.seats[1].pool, 8, "bled 2");
        assert_eq!(
            req.options,
            vec![Choice::DrawLibrary, Choice::FindCard(steely)]
        );
        if retake {
            pick(&mut e, Choice::FindCard(steely));
            assert!(e.state.seats[0].hand.contains(&steely));
            assert!(!ash_has(&e, 0, "Steely Tenacity"));
            assert_eq!(e.state.seats[0].library.last(), Some(&top), "no draw");
        } else {
            pick(&mut e, Choice::DrawLibrary);
            assert!(e.state.seats[0].hand.contains(&top));
            assert!(ash_has(&e, 0, "Steely Tenacity"));
        }
        assert_eq!(e.state.seats[0].hand.len(), 7);
    }
}

/// Velvet Tongue [SER]: before any vote, corrupted minions cannot vote in
/// this referendum, the clean ones still can; [ser] (another game: one
/// Tongue an action for the caller): a corruption counter on a minion that
/// voted, its votes standing and no steal offered (ANK 20200925).
// Rulings: [ANK 20200925] [ANK 20211105].
#[test]
fn velvet_tongue_silences_the_corrupted() {
    for superior_mode in [true, false] {
        let mut e = game(2);
        let caller = e.add_minion(SeatId(0), superior("SER", 0), 6, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let dirty = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
        let clean = e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        for m in [dirty, clean] {
            e.state.minions[m.index()].granted_title = Some(Title::Votes(1));
        }
        e.state.minions[dirty.index()].corruption = vec![(SeatId(0), 1)];
        hand(&mut e, 0, &["Ancilla Empowerment", "Velvet Tongue"]);
        pick(&mut e, play(0, 0, caller));
        to_referendum(&mut e);
        if superior_mode {
            pick_when(&mut e, play(0, 1, caller));
        }
        let tongue = play_at(0, 0, Some(caller), Target::Minion(clean));
        let mut seen = (false, false);
        let mut corrupted = false;
        for _ in 0..40 {
            let req = request(&mut e);
            if req.window != WindowLabel::ReferendumPolling {
                break;
            }
            seen.0 |= req.options.contains(&votes(dirty, false));
            seen.1 |= req.options.contains(&votes(clean, false));
            let i = if !superior_mode && req.options.contains(&tongue) {
                corrupted = true;
                req.options.iter().position(|o| *o == tongue)
            } else {
                req.options.iter().position(|o| *o == votes(clean, false))
            };
            e.choose(i.unwrap_or_else(|| fallback(&req)));
        }
        assert_eq!(seen, (!superior_mode, true), "[SER] {superior_mode}");
        if !superior_mode {
            assert!(corrupted, "[ser] offered on the minion that voted");
            let v = &e.state.minions[clean.index()];
            assert_eq!(v.corruption, vec![(SeatId(0), 1)]);
            assert_eq!(v.owner, SeatId(1), "no steal");
        }
    }
}

/// Plain untitled Gangrel of capacity `min` or more, youngest first.
fn gangrels(min: u8) -> Vec<u16> {
    let mut v: Vec<u16> = CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.clan == "Gangrel"
                && c.title == Title::None
                && c.capacity >= min
                && !c.imbued
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .map(|(i, _)| i as u16)
        .collect();
    v.sort_by_key(|&i| CRYPT[i as usize].capacity);
    v
}

/// Skaald: put on the Gangrel and unlock them; +2 votes in referendums a
/// Gangrel calls, none in another's; stolen only by an older untitled
/// Gangrel.
// Rulings: [LSJ 20080604].
#[test]
fn skaald_votes_for_gangrel_callers() {
    let g = gangrels(5);
    let (young, old) = (g[0], *g.last().unwrap());
    assert!(CRYPT[old as usize].capacity > CRYPT[young as usize].capacity);
    for gangrel_calls in [true, false] {
        let mut e = game(2);
        let bearer = e.add_minion(SeatId(0), young, 4, false);
        let caller = if gangrel_calls {
            bearer
        } else {
            e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false)
        };
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let thief = e.add_minion(SeatId(1), old, 4, false);
        let younger = e.add_minion(SeatId(1), young, 4, false);
        hand(&mut e, 0, &["Skaald", "Ancilla Empowerment"]);
        pick(&mut e, play(0, 0, bearer));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let skaald = on(&e, "Skaald", bearer).expect("on the Gangrel");
        placed(&e, skaald);
        assert!(!e.state.minions[bearer.index()].locked, "unlocked");
        let ancilla = e.state.seats[0]
            .hand
            .iter()
            .position(|&c| c == find_library("Ancilla Empowerment"))
            .unwrap() as u8;
        pick(&mut e, play(ancilla, 0, caller));
        to_referendum(&mut e);
        let r = e.referendum().expect("referendum");
        assert_eq!(
            r.votes_of(&e.state, bearer),
            if gangrel_calls { 2 } else { 0 }
        );
        if gangrel_calls {
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            let req = request(&mut e);
            let steal = |m: MinionId| {
                req.options.iter().any(|o| {
                    matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), actor: Some(a), .. }
                        if *c == skaald && *a == m)
                })
            };
            assert!(steal(thief), "older: {:?}", req.options);
            assert!(!steal(younger), "not older");
        }
    }
}

/// Serpent's Numbing Kiss [PRE][SER]: after combat at close range, its
/// vampire burns 1 blood and the card goes on the opposing minion, locked;
/// it does not unlock as normal and the card burns in its unlock phase.
// Rulings: [RTR 19970630] [ANK 20180420-1] [ANK 20200705]
// [ANK 20180420-2] [LSJ 20040601].
#[test]
fn serpents_numbing_kiss_holds_the_blocker() {
    let kisser = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.capacity >= 4
                && zvtes::cards::vampire(i as u16).is_none()
                && ["PRE", "SER"].iter().all(|d| c.disciplines.contains(d))
        })
        .expect("a superior pre and ser vampire") as u16;
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), kisser, 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Serpent's Numbing Kiss"]);
    hand(&mut e, 1, &["Form of Mist"; 7]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[a.index()].blood, 3, "1 blood burned");
    let kiss = on(&e, "Serpent's Numbing Kiss", b).expect("on the blocker");
    placed(&e, kiss);
    assert!(e.state.minions[b.index()].locked);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.active, SeatId(1));
    assert!(e.state.minions[b.index()].locked, "did not unlock");
    assert!(!e.state.cards[kiss.index()].in_play, "burned");
}

/// Nar-Sheptha: her blocked vampire's blocker is considered the acting
/// minion (its Methuselah polled first in combat), and her controller has
/// +2 hand size while the vampire is in combat, not after.
#[test]
fn nar_sheptha_turns_the_combat_around() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Banu Haqim", 0), 4, false);
    e.add_minion(SeatId(0), clan("Banu Haqim", 1), 4, false);
    let b = e.add_minion(SeatId(1), with_discs(&["cel"]), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    in_play_with_layers(&mut e, 0, "Nar-Sheptha", Some(a));
    hand(&mut e, 0, &["Side Strike"]);
    hand(&mut e, 1, &["Side Strike"]);
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let req = request(&mut e);
    assert!(e.combat().is_some());
    assert_eq!(e.combat().unwrap().sides[0].minion, b, "the blocker acts");
    assert_eq!(req.seat, SeatId(1), "polled first");
    assert_eq!(e.state.hand_size(SeatId(0)), 9);
    assert_eq!(e.state.hand_size(SeatId(1)), 7);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.hand_size(SeatId(0)), 7, "combat over");
}

/// Gift of Proteus: X Protean cards announced with the action go on it as
/// it resolves; only the bearer plays them, as if from hand; the Gift burns
/// once empty.
// Rulings: [PIB 20150428] [LSJ 20081126].
#[test]
fn gift_of_proteus_holds_protean_cards() {
    let g: Vec<u16> = gangrels(4)
        .into_iter()
        .filter(|&i| {
            CRYPT[i as usize]
                .disciplines
                .iter()
                .any(|d| d.eq_ignore_ascii_case("pro"))
        })
        .collect();
    let mut e = game(2);
    let bearer = e.add_minion(SeatId(0), g[0], 4, false);
    let other = e.add_minion(SeatId(0), g[1], 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let mut cards = vec!["Gift of Proteus", "Loki's Gift", "Wolf Claws"];
    cards.extend(["Deflection"; 4]);
    hand(&mut e, 0, &cards);
    e.state.seats[0]
        .library
        .extend([find_library("Deflection"); 3]);
    pick(&mut e, play(0, 0, bearer));
    let req = request(&mut e);
    assert_eq!(
        req.options,
        vec![Choice::SetX(0), Choice::SetX(1), Choice::SetX(2)]
    );
    pick(&mut e, Choice::SetX(1));
    let loki = find_library("Loki's Gift");
    pick_when(&mut e, Choice::FindCard(loki));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let gift = on(&e, "Gift of Proteus", bearer).expect("on the Gangrel");
    placed(&e, gift);
    assert_eq!(
        e.state.cards[gift.index()]
            .stored
            .iter()
            .collect::<Vec<_>>(),
        vec![&loki]
    );
    let hand = &e.state.seats[0].hand;
    assert!(!hand.contains(&loki) && hand.contains(&find_library("Wolf Claws")));
    e.state.minions[bearer.index()].locked = false;
    let hand_len = e.state.seats[0].hand.len();
    let req = request(&mut e);
    let stored = |m: MinionId| {
        req.options.iter().copied().find(|o| {
            matches!(o, Choice::Play { card, actor: Some(a), .. } if *card as usize >= hand_len && *a == m)
        })
    };
    assert!(stored(other).is_none(), "the bearer's only");
    let play_loki = stored(bearer).expect("as if from hand");
    pick(&mut e, play_loki);
    assert!(!e.state.cards[gift.index()].in_play, "burned once empty");
}

/// Conniver: its prey losing pool on a third Methuselah's turn, its player
/// may lock it for 1 blood from the bank on its vampire; not on the prey's
/// own turn.
#[test]
fn conniver_profits_from_the_prey_losing_pool() {
    let mut e = game(3);
    let v = e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), clan("Ventrue", 1), 4, false);
    for (s, c) in [(1, "Brujah"), (2, "Toreador")] {
        e.add_minion(SeatId(s), clan(c, 0), 4, false);
        e.add_minion(SeatId(s), clan(c, 1), 4, false);
    }
    let conniver = in_play(&mut e, 0, "Conniver", Some(v));
    let lock = Choice::ResolveRider { card: conniver.0 };
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.seats[1].lose_pool(1);
    assert!(
        !request(&mut e).options.contains(&lock),
        "the prey's own turn"
    );
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    e.state.seats[1].lose_pool(1);
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.options.clone()),
        (SeatId(0), vec![Choice::Pass, lock])
    );
    pick(&mut e, lock);
    assert!(e.state.cards[conniver.index()].locked);
    assert_eq!(e.state.minions[v.index()].blood, 5);
    e.state.seats[1].lose_pool(1);
    assert_ne!(request(&mut e).seat, SeatId(0), "locked: no more");
}

/// Vestgeir: a political action card of another Methuselah's ash heap,
/// played as if from hand (his only), then removed from the game, its
/// owner's, instead of burned.
#[test]
fn vestgeir_plays_politics_from_another_ash_heap() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Vestgeir"), 10, false);
    let other = e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    let ancilla = find_library("Ancilla Empowerment");
    e.state.seats[1]
        .ash
        .put(ancilla, zvtes::copies::CopyId::NONE);
    let hand_len = e.state.seats[0].hand.len() as u8;
    let req = request(&mut e);
    let from_ash = |m: MinionId| {
        req.options.iter().copied().find(|o| {
            matches!(o, Choice::Play { card, actor: Some(a), .. } if *card >= hand_len && *a == m)
        })
    };
    assert!(from_ash(other).is_none(), "Vestgeir's only");
    let play_it = from_ash(v).expect("from the other ash heap");
    pick(&mut e, play_it);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.seats.iter().any(|s| s.ash.contains(&ancilla)),
        "not burned"
    );
    assert!(
        e.state.seats[1].removed.contains(&ancilla),
        "removed, its owner's"
    );
}

// ---- Library: From a Sinking Ship, Funeral Wake, Blight, In Memory -------

/// From a Sinking Ship: a minion of a Methuselah at 3 pool or less, not a
/// vampire of capacity 7 or more, taken; once in the game.
// Rulings: [TOM 19960226-1].
#[test]
fn from_a_sinking_ship_takes_a_drowning_minion() {
    let mut e = game(2);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), clan("Ventrue", 1), 4, false);
    let small = e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    let big = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.capacity >= 7
                && c.title == Title::None
                && !c.imbued
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .unwrap() as u16;
    let big = e.add_minion(SeatId(1), big, 4, false);
    hand(
        &mut e,
        0,
        &[
            "From a Sinking Ship",
            "From a Sinking Ship",
            "Pentex(TM) Subversion",
        ],
    );
    hand(&mut e, 1, &["Deflection"]);
    let at = |m: MinionId| play_at(0, 0, None, Target::Minion(m));
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.state.seats[1].pool = 4;
    assert!(!request(&mut e).options.contains(&at(small)), "4 pool");
    e.state.seats[1].pool = 3;
    let req = request(&mut e);
    assert!(!req.options.contains(&at(big)), "capacity 7 or more");
    pick(&mut e, at(small));
    let req = request(&mut e);
    assert_eq!(e.state.minions[small.index()].owner, SeatId(0));
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. })),
        "one in the game"
    );
}

/// Funeral Wake [nec]: a locked vampire unlocks a wraith ally its
/// Methuselah controls, during another's action.
#[test]
fn funeral_wake_unlocks_a_wraith() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), clan("Ventrue", 1), 4, false);
    let r = e.add_minion(SeatId(1), with_discs(&["nec"]), 4, false);
    let wraith = e.add_ally(SeatId(1), find_library("Screamer"));
    for m in [r, wraith] {
        e.state.minions[m.index()].locked = true;
    }
    hand(&mut e, 1, &["Funeral Wake"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 0, Some(r), Target::Minion(wraith)));
    request(&mut e);
    assert!(!e.state.minions[wraith.index()].locked);
}

/// Blight [NEC]: before range, the opposing minion -1 strength this combat
/// (its 1-strength hand strike harmless) and no additional strike card
/// (LSJ 20011214-5).
// Rulings: [LSJ 20011214-5].
#[test]
fn blight_weakens_and_bars_additional_strikes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), superior("NEC", 0), 6, false);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(1), with_discs(&["cel"]), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Blight"]);
    hand(&mut e, 1, &["Blur"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 1, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, b));
    assert_eq!(
        e.state.minions[a.index()].blood,
        6,
        "no damage at strength 0"
    );
}

/// In Memory of the Two Lands: a counter per Follower of Set and mummy of
/// its Methuselah's; a counter for +1 bleed on the mummy's bleed, once an
/// action; burned with none left.
// Rulings: [ANK 20231124] [LSJ 19970718].
#[test]
fn in_memory_of_the_two_lands_spends_its_counters() {
    let mut e = game(2);
    e.add_minion(SeatId(0), clan("Ministry", 0), 4, false);
    let mummy = e.add_ally(SeatId(0), find_library("Saatet-ta"));
    e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    hand(&mut e, 0, &["In Memory of the Two Lands"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, play_at(0, 0, None, Target::None));
    request(&mut e);
    let card = (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|&c| e.state.cards[c.index()].card == find_library("In Memory of the Two Lands"))
        .expect("in play");
    assert_eq!(
        e.state.cards[card.index()].counters,
        2,
        "a Ministry and a mummy"
    );
    let pool = e.state.seats[1].pool;
    pick_when(&mut e, bleed(mummy));
    let boost = Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, boost);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let bled = pool - e.state.seats[1].pool;
    assert_eq!(e.state.cards[card.index()].counters, 1);
    assert_eq!(bled, i32::from(e.state.base_bleed(mummy)) + 1, "+1 bleed");
}

// ---- Ledger: the round's edges ---------------------------------------------

/// A plain vampire holding all of `discs` as printed (case: superior).
fn holding(discs: &[&str]) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.capacity >= 4
                && !c.imbued
                && zvtes::cards::vampire(i as u16).is_none()
                && discs.iter().all(|d| c.disciplines.contains(d))
        })
        .unwrap_or_else(|| panic!("no vampire with {discs:?}")) as u16
}

/// Velvet Tongue: under its [SER] bar, a minion corrupted by [ser] after
/// it voted loses those votes (PIB 20141026-2).
// Rulings: [PIB 20141026-2].
#[test]
fn velvet_tongue_voids_votes_cast_before_the_corruption() {
    let mut e = game(3);
    let caller = e.add_minion(SeatId(0), superior("SER", 0), 6, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let voter = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    let snake = e.add_minion(SeatId(2), with_discs(&["ser"]), 4, false);
    e.add_minion(SeatId(2), clan("Toreador", 0), 4, false);
    e.state.minions[voter.index()].granted_title = Some(Title::Votes(1));
    hand(&mut e, 0, &["Ancilla Empowerment", "Velvet Tongue"]);
    hand(&mut e, 2, &["Velvet Tongue"]);
    pick(&mut e, play(0, 0, caller));
    to_referendum(&mut e);
    pick_when(&mut e, play(0, 1, caller));
    pick_when(&mut e, votes(voter, false));
    assert_eq!(e.referendum().unwrap().votes_of(&e.state, voter), 1, "cast");
    pick_when(&mut e, play_at(0, 0, Some(snake), Target::Minion(voter)));
    request(&mut e);
    let r = e.referendum().expect("still polling");
    assert_eq!(r.votes_of(&e.state, voter), 0, "corrupted: no votes");
}

/// Ophidian Gaze [PRE][SER]: an action modifier played after a successful
/// political action (Voter Captivation, Freak Drive) is canceled as played
/// (ANK 20180909).
// Rulings: [ANK 20180909].
#[test]
fn ophidian_gaze_cancels_after_success_modifiers() {
    for (modifier, disc) in [("Voter Captivation", "pre"), ("Freak Drive", "for")] {
        let mut e = game(2);
        let caller = e.add_minion(SeatId(0), with_discs(&[disc]), 6, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let gazer = e.add_minion(SeatId(1), holding(&["PRE", "SER"]), 6, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        e.state.minions[caller.index()].granted_title = Some(Title::Votes(3));
        hand(&mut e, 0, &["Ancilla Empowerment", modifier]);
        hand(&mut e, 1, &["Ophidian Gaze"]);
        e.notes = Some(Vec::new());
        pick(&mut e, play(0, 0, caller));
        pick_when(&mut e, votes(caller, true));
        pick_when(&mut e, play(0, 0, caller));
        pick_when(&mut e, play(0, 1, gazer));
        request(&mut e);
        let card = find_library(modifier);
        let plays = noted_plays(&mut e);
        assert!(
            plays.iter().any(|p| p.card == card && p.canceled),
            "{modifier}: {plays:?}"
        );
    }
}

/// Serpent's Numbing Kiss [PRE][SER]: a dodged strike puts nothing on the
/// opponent (RTR 20041202); with no blood to burn it is not offered, nor
/// Hall of Mirrors [CHI] (ANK 20210226).
// Rulings: [RTR 20041202].
#[test]
fn burn_blood_strikes_need_the_blood_and_an_undodged_strike() {
    let kisser = holding(&["PRE", "SER"]);
    // Dodged.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), kisser, 4, false);
    e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), with_discs(&["cel"]), 4, false);
    e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Serpent's Numbing Kiss"]);
    hand(&mut e, 1, &["Side Strike"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, "Serpent's Numbing Kiss", b), None);
    assert_eq!(e.state.minions[a.index()].blood, 4, "no blood burned");
    // No blood.
    for (card, actor) in [
        ("Serpent's Numbing Kiss", kisser),
        ("Hall of Mirrors", superior("CHI", 0)),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), actor, 1, false);
        e.add_minion(SeatId(0), clan("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), with_discs(&["cel"]), 4, false);
        e.add_minion(SeatId(1), clan("Ventrue", 1), 4, false);
        hand(&mut e, 0, &[card]);
        hand(&mut e, 1, &["Side Strike"]);
        pick(&mut e, bleed(a));
        e.state.minions[a.index()].blood = 0;
        pick_when(&mut e, block(b));
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
    }
}

/// Steely Tenacity's replacement edges: out of the ash heap, nothing to
/// take back (RBK targeting-of-cards); canceled (Hall of Mirrors), replaced
/// at once with neither delay nor choice (LSJ 20080630); with the
/// Capuchin's replacement, its player chooses (ANK 20200905); held by The
/// Meddling of Semsith, still taken back once the hold ends (LSJ 20100527).
// Rulings: [ANK 20200905] [LSJ 20100527] [RBK targeting-of-cards].
#[test]
fn steely_tenacity_replacement_edges() {
    let steely = find_library("Steely Tenacity");
    let setup = |e: &mut Engine| {
        let a = e.add_minion(SeatId(0), anarch(&["dom"], 0), 4, false);
        e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
        let mut cards = vec!["Steely Tenacity"];
        cards.extend(["Form of Mist"; 6]);
        hand(e, 0, &cards);
        e.state.seats[0]
            .library
            .extend([find_library("Deflection"); 3]);
        a
    };
    // Out of the ash heap while the choice is pending: drawn.
    let mut e = game(2);
    let a = setup(&mut e);
    pick(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::Draw);
    let ash = &mut e.state.seats[0].ash;
    let k = ash
        .iter()
        .position(|&c| c == steely)
        .expect("in the ash heap");
    ash.take(k);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::FindCard(steely),
    );
    assert_eq!(e.state.seats[0].hand.len(), 7, "drawn");
    // The Capuchin: both replacements offered.
    let mut e = game(2);
    let a = setup(&mut e);
    let capuchin = in_play_with_layers(&mut e, 0, "Visit from the Capuchin", None);
    e.state.cards[capuchin.index()].counters = 1;
    e.state.seats[0]
        .hand
        .put(find_library("Form of Mist"), zvtes::copies::CopyId::NONE);
    pick(&mut e, play(0, 2, a));
    let req = until(&mut e, SeatId(0), WindowLabel::Draw);
    assert!(
        req.options.contains(&Choice::FindCard(steely)),
        "{:?}",
        req.options
    );
    pick(&mut e, Choice::FindCard(steely));
    request(&mut e);
    assert_eq!(
        e.state.cards[capuchin.index()].counters,
        1,
        "the Capuchin's not taken"
    );
    assert!(e.state.seats[0].hand.contains(&steely));
    // The Meddling: held to the minion phase's end, then taken back.
    let mut e = game(2);
    let a = setup(&mut e);
    let meddling = in_play_with_layers(&mut e, 1, "Meddling of Semsith, The", None);
    e.state.cards[meddling.index()].against = zvtes::ids::SeatSet::single(SeatId(0));
    pick(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].hand.len(), 6, "held");
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::Draw);
    assert!(req.options.contains(&Choice::FindCard(steely)));
}

/// Steely Tenacity [ani] canceled as played by Hall of Mirrors [CHI]: its
/// replacement clauses canceled with it, replaced at once (LSJ 20080630).
// Rulings: [LSJ 20011023] [LSJ 20080630]; [LSJ 20080630] [LSJ 20011023].
#[test]
fn steely_tenacity_canceled_is_replaced_at_once() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&["ani"], 0), 4, false);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(1), superior("CHI", 0), 6, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    let mut cards = vec!["Steely Tenacity"];
    cards.extend(["Form of Mist"; 6]);
    hand(&mut e, 0, &cards);
    e.state.seats[0]
        .library
        .extend([find_library("Deflection"); 3]);
    hand(&mut e, 1, &["Hall of Mirrors"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, b));
    request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), 7, "replaced at once");
    assert!(e.state.seats[0].retakes.is_empty());
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::FindCard(find_library("Steely Tenacity")),
    );
}

/// Gift of Proteus: a card played from it as if from hand can be canceled
/// as played (LSJ 20060106: Hall of Mirrors on a stored Body Flare).
// Rulings: [LSJ 20060106].
#[test]
fn a_card_played_from_gift_of_proteus_can_be_canceled() {
    let g: Vec<u16> = gangrels(4)
        .into_iter()
        .filter(|&i| {
            CRYPT[i as usize]
                .disciplines
                .iter()
                .any(|d| d.eq_ignore_ascii_case("pro"))
        })
        .collect();
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), g[0], 4, false);
    e.add_minion(SeatId(0), g[1], 4, false);
    let b = e.add_minion(SeatId(1), superior("CHI", 0), 6, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    let gift = in_play(&mut e, 0, "Gift of Proteus", Some(a));
    let flare = find_library("Body Flare");
    e.state.cards[gift.index()]
        .stored
        .put(flare, zvtes::copies::CopyId::NONE);
    hand(&mut e, 1, &["Hall of Mirrors"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let from_gift = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a))
        .expect("Body Flare from the Gift");
    pick(&mut e, from_gift);
    pick_when(&mut e, play(0, 1, b));
    request(&mut e);
    let plays = noted_plays(&mut e);
    assert!(
        plays.iter().any(|p| p.card == flare && p.canceled),
        "{plays:?}"
    );
}

/// Gift of Proteus: two cards announced, one spent before resolution (Form
/// of Mist [PRO] continuing the blocked action): the one left goes on it
/// (PIB 20150428).
// Rulings: [PIB 20150428].
#[test]
fn gift_of_proteus_takes_what_is_left() {
    let bearer = gangrels(4)
        .into_iter()
        .find(|&i| CRYPT[i as usize].disciplines.contains(&"PRO"))
        .expect("a Gangrel with superior Protean");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), bearer, 6, false);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    // +1 intercept against the Gift's +1 stealth.
    let b = e.add_minion(SeatId(1), find_card("Iago Castille"), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    let mut cards = vec!["Gift of Proteus", "Form of Mist", "Wolf Claws"];
    cards.extend(["Deflection"; 4]);
    hand(&mut e, 0, &cards);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::SetX(2));
    pick_when(&mut e, block(b));
    let mist = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Form of Mist"))
        .unwrap() as u8;
    pick_when(&mut e, play(mist, 1, a));
    pick_when(&mut e, Choice::ContinueAction);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let gift = on(&e, "Gift of Proteus", a).expect("on the Gangrel");
    let stored: Vec<u16> = e.state.cards[gift.index()].stored.iter().copied().collect();
    assert_eq!(stored, vec![find_library("Wolf Claws")]);
}

/// Nar-Sheptha over Deep Song (ANK 20211022): Deep Song's target with her
/// leaves the Deep Song player the acting minion; without her, the target.
// Rulings: [ANK 20211022].
#[test]
fn nar_sheptha_supersedes_deep_song() {
    for with_nar in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), superior("ANI", 0), 6, false);
        e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
        let t = e.add_minion(SeatId(1), with_discs(&["cel"]), 4, false);
        e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
        if with_nar {
            in_play_with_layers(&mut e, 1, "Nar-Sheptha", Some(t));
        }
        hand(&mut e, 0, &["Deep Song"]);
        hand(&mut e, 1, &["Side Strike"]);
        pick(&mut e, play_at(0, 1, Some(a), Target::Minion(t)));
        until(&mut e, SeatId(1), WindowLabel::CombatStrike);
        let acting = e.combat().expect("combat").sides[0].minion;
        assert_eq!(
            acting,
            if with_nar { a } else { t },
            "Nar-Sheptha: {with_nar}"
        );
    }
}

/// Nar-Sheptha burned in combat: the acting minion is the usual one again
/// at once, for the rest of the combat (ANK 20211022) — the same round's
/// press step already the usual order.
// Rulings: [ANK 20211022].
#[test]
fn nar_sheptha_gone_the_acting_minion_returns() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["cel"]), 6, false);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(1), clan("Brujah", 0), 6, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    let nar = in_play_with_layers(&mut e, 0, "Nar-Sheptha", Some(a));
    hand(&mut e, 1, &["Side Strike"]);
    hand(&mut e, 0, &["Psyche!"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    request(&mut e);
    assert_eq!(e.combat().unwrap().sides[0].minion, b, "the blocker acts");
    e.state.cards[nar.index()].in_play = false;
    until(&mut e, SeatId(0), WindowLabel::CombatPress);
    let c = e.combat().expect("the press step");
    assert_eq!(
        (c.round, c.sides[0].minion),
        (1, a),
        "at once, in the same round"
    );
}

/// Nar-Sheptha: a combat Psyche! [CEL] queues, her +2 hand size again
/// (LSJ 20030530).
// Rulings: [LSJ 20030530].
#[test]
fn nar_sheptha_hand_size_in_a_queued_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), superior("CEL", 0), 6, false);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(1), clan("Brujah", 0), 6, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    in_play_with_layers(&mut e, 0, "Nar-Sheptha", Some(a));
    hand(&mut e, 1, &["Side Strike", "Side Strike"]);
    hand(&mut e, 0, &["Psyche!"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    let c = e.combat().expect("the queued combat");
    assert_eq!(
        (c.round, c.sides[0].minion),
        (1, b),
        "the blocker acting again"
    );
    assert_eq!(e.state.hand_size(SeatId(0)), 9);
}

/// FBI Special Affairs Division: an ally made the acting minion by
/// Nar-Sheptha, burned in combat, triggers nothing (ANK 20180913-2).
// Rulings: [ANK 20180913-2].
#[test]
fn fbi_ignores_an_ally_considered_acting() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), clan("Ventrue", 0), 5, false);
    e.add_minion(SeatId(0), clan("Ventrue", 1), 5, false);
    let ally = e.add_ally(SeatId(1), find_library("Underbridge Stray"));
    e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(2), clan("Toreador", 0), 4, false);
    e.add_minion(SeatId(2), clan("Toreador", 1), 4, false);
    in_play_with_layers(&mut e, 0, "Nar-Sheptha", Some(a));
    let fbi = in_play_with_layers(&mut e, 2, "FBI Special Affairs Division", None);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ally));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play, "burned in combat");
    assert_eq!(e.state.minions[a.index()].blood, 5, "no FBI damage");
    assert_eq!(e.state.cards[fbi.index()].counters, 0);
}

/// Gift of Proteus: only cards in hand before its replacement was drawn go
/// on it — the Form of Mist drawn as the replacement is not offered.
// Rulings: [PIB 20150428].
#[test]
fn gift_of_proteus_ignores_its_replacement() {
    let g: Vec<u16> = gangrels(4)
        .into_iter()
        .filter(|&i| {
            CRYPT[i as usize]
                .disciplines
                .iter()
                .any(|d| d.eq_ignore_ascii_case("pro"))
        })
        .collect();
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), g[0], 4, false);
    e.add_minion(SeatId(0), g[1], 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    let mut cards = vec!["Gift of Proteus", "Wolf Claws"];
    cards.extend(["Deflection"; 5]);
    hand(&mut e, 0, &cards);
    let mist = find_library("Form of Mist");
    e.state.seats[0].library.extend([mist]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::SetX(1));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::FindCard(mist),
    );
    let gift = on(&e, "Gift of Proteus", a).expect("on the Gangrel");
    let stored: Vec<u16> = e.state.cards[gift.index()].stored.iter().copied().collect();
    assert_eq!(stored, vec![find_library("Wolf Claws")]);
    assert!(
        e.state.seats[0].hand.contains(&mist),
        "the replacement, kept"
    );
}

/// From a Sinking Ship takes no vampire whose title its player claims:
/// no contest with oneself (rules-qa 100.1, advanced rules §1.13.1).
#[test]
fn from_a_sinking_ship_takes_no_title_its_player_claims() {
    let mut e = game(2);
    let own = e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), clan("Ventrue", 1), 4, false);
    let rival = e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    let plain = e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    for m in [own, rival] {
        let v = &mut e.state.minions[m.index()];
        v.granted_title = Some(Title::Prince("Paris"));
        v.title_contested = true;
    }
    hand(&mut e, 0, &["From a Sinking Ship", "Pentex(TM) Subversion"]);
    hand(&mut e, 1, &["Deflection"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.state.seats[1].pool = 3;
    let req = request(&mut e);
    let at = |m: MinionId| play_at(0, 0, None, Target::Minion(m));
    assert!(
        !req.options.contains(&at(rival)),
        "its title claimed: not a target"
    );
    assert!(req.options.contains(&at(plain)));
}

/// Two Steely Tenacity due at one action's end ([dom] bleeding, blocked,
/// [ani] striking in the combat): each offered back from the ash heap in turn,
/// none lost to a plain draw (LSJ 20100527: each keeps its choice).
#[test]
fn two_steely_tenacity_each_come_back() {
    let steely = find_library("Steely Tenacity");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Clara Hjortshøj"), 6, false);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    let t = e.add_minion(SeatId(1), clan("Brujah", 0), 6, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    let mut cards = vec!["Steely Tenacity", "Steely Tenacity"];
    cards.extend(["Form of Mist"; 5]);
    hand(&mut e, 0, &cards);
    e.state.seats[0]
        .library
        .extend([find_library("Deflection"); 3]);
    pick(&mut e, play(0, 2, a));
    pick_when(&mut e, block(t));
    pick_when(&mut e, play(0, 0, a));
    let mut taken = 0;
    for _ in 0..40 {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        if req.options.contains(&Choice::FindCard(steely)) {
            taken += 1;
            pick(&mut e, Choice::FindCard(steely));
            continue;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(taken, 2, "each Steely Tenacity offered");
    let back = e.state.seats[0]
        .hand
        .iter()
        .filter(|&&c| c == steely)
        .count();
    assert_eq!(back, 2);
}

/// Funeral Wake [NEC]: up to two allies unlocked, the second picked by its
/// terms; picking the first again unlocks none more.
#[test]
fn funeral_wake_superior_unlocks_two() {
    for second in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
        e.add_minion(SeatId(0), clan("Ventrue", 1), 4, false);
        let r = e.add_minion(SeatId(1), superior("NEC", 0), 4, false);
        let w1 = e.add_ally(SeatId(1), find_library("Screamer"));
        let w2 = e.add_ally(SeatId(1), find_library("Spectral Servitor"));
        for m in [w1, w2] {
            e.state.minions[m.index()].locked = true;
        }
        hand(&mut e, 1, &["Funeral Wake"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, play_at(0, 1, Some(r), Target::Minion(w1)));
        pick(&mut e, Choice::ChooseMinion(if second { w2 } else { w1 }));
        request(&mut e);
        assert!(!e.state.minions[w1.index()].locked);
        assert_eq!(
            e.state.minions[w2.index()].locked,
            !second,
            "second: {second}"
        );
    }
}

/// In Memory of the Two Lands: its strength mode on the acting mummy uses
/// the action's one use, the bleed mode then not offered; burned once its
/// last counter is spent.
#[test]
fn in_memory_of_the_two_lands_one_use_an_action() {
    let mut e = game(2);
    let mummy = e.add_ally(SeatId(0), find_library("Saatet-ta"));
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    let card = in_play(&mut e, 0, "In Memory of the Two Lands", None);
    e.state.cards[card.index()].counters = 1;
    let use_of = |ability, target| Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability,
        actor: None,
        target,
    };
    pick(&mut e, bleed(mummy));
    pick_when(&mut e, use_of(1, Target::Minion(mummy)));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_of(0, Target::None),
    );
    assert!(
        !e.state.cards[card.index()].in_play,
        "no counter left: burned"
    );
}

/// From a Sinking Ship on an ally: taken, as a vampire is (106.3).
#[test]
fn from_a_sinking_ship_takes_an_ally() {
    let mut e = game(2);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), clan("Ventrue", 1), 4, false);
    let theirs = e.add_ally(SeatId(1), find_library("Screamer"));
    e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    hand(&mut e, 0, &["From a Sinking Ship", "Pentex(TM) Subversion"]);
    hand(&mut e, 1, &["Deflection"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.state.seats[1].pool = 3;
    pick(&mut e, play_at(0, 0, None, Target::Minion(theirs)));
    request(&mut e);
    assert_eq!(e.state.minions[theirs.index()].owner, SeatId(0));
}

/// Blight [NEC]: its opponent presses to continue no more, a press card not
/// offered while none was pressed to end against (106.5).
#[test]
fn blight_bars_presses_to_continue() {
    for superior_mode in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), superior("NEC", 0), 6, false);
        e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
        let b = e.add_minion(SeatId(1), with_discs(&["cel"]), 6, false);
        e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
        hand(&mut e, 0, &["Blight"]);
        hand(&mut e, 1, &["Psyche!"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(b));
        pick_when(&mut e, play(0, u8::from(superior_mode), a));
        let mut offered = false;
        for _ in 0..40 {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            offered |= req.options.contains(&play(0, 0, b));
            e.choose(fallback(&req));
        }
        assert_eq!(offered, !superior_mode, "[NEC]: {superior_mode}");
    }
}

/// Nar-Sheptha's swapped order is the combat's order only: a blocked bleed
/// still counts as the turn's first block combat (NSA Trio's count,
/// `vampire_block_combat_turn`), the blocker's equipment held at the block.
#[test]
fn nar_sheptha_keeps_the_block_combat_count() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["cel"]), 6, false);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(1), with_discs(&["cel"]), 6, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    in_play_with_layers(&mut e, 0, "Nar-Sheptha", Some(a));
    hand(&mut e, 0, &["Side Strike"]);
    hand(&mut e, 1, &["Side Strike"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    request(&mut e);
    assert_eq!(e.combat().unwrap().sides[0].minion, b, "swapped");
    assert_eq!(e.state.vampire_block_combat_turn, Some(e.state.turn));
}

/// Nar-Sheptha burned in a combat Psyche! [CEL] queued: that combat's
/// usual order comes back too (the queued combat built from the natural
/// pair; ANK 20211022).
#[test]
fn nar_sheptha_gone_in_a_queued_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), superior("CEL", 0), 6, false);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(1), clan("Brujah", 0), 6, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    let nar = in_play_with_layers(&mut e, 0, "Nar-Sheptha", Some(a));
    hand(&mut e, 0, &["Psyche!", "Psyche!"]);
    hand(&mut e, 1, &["Side Strike", "Side Strike"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    assert_eq!(
        e.combat().unwrap().sides[0].minion,
        b,
        "queued: still swapped"
    );
    e.state.cards[nar.index()].in_play = false;
    until(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert_eq!(
        e.combat().unwrap().sides[0].minion,
        a,
        "the usual order again"
    );
}

/// Anathema's "reduced to 0" reads each side's own blood across a swap:
/// a blocker that entered at 0 blood is not burned when Nar-Sheptha's
/// order reverts.
#[test]
fn anathema_follows_its_vampire_across_a_swap() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_discs(&["cel"]), 3, false);
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(1), clan("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    let nar = in_play_with_layers(&mut e, 0, "Nar-Sheptha", Some(a));
    in_play_with_layers(&mut e, 1, "Anathema", Some(b));
    hand(&mut e, 0, &["Side Strike", "Psyche!"]);
    hand(&mut e, 1, &["Deflection"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    e.state.minions[b.index()].blood = 0;
    request(&mut e);
    e.state.cards[nar.index()].in_play = false;
    pick_when(&mut e, play(0, 0, a));
    let pool = e.state.seats[0].pool;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[b.index()].in_play, "not burned by Anathema");
    assert_eq!(e.state.seats[0].pool, pool);
}

/// Nar-Sheptha burned in a combat Pocket Out of Time [TEM] started: that
/// combat began from the first's natural pair (`last_combat`), so the
/// usual order comes back (106.10).
#[test]
fn nar_sheptha_gone_in_a_pocket_out_of_time_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), clan("Brujah", 0), 8, false);
    // Superior Temporis, granted as round 15's tests do.
    e.state.minions[a.index()].beast_disciplines[0] = Some("TEM");
    e.add_minion(SeatId(0), clan("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(1), with_discs(&["cel"]), 8, false);
    e.add_minion(SeatId(1), clan("Brujah", 1), 4, false);
    let nar = in_play_with_layers(&mut e, 0, "Nar-Sheptha", Some(a));
    hand(&mut e, 0, &["Pocket Out of Time"]);
    hand(&mut e, 1, &["Side Strike", "Side Strike", "Side Strike"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert_eq!(
        e.combat().expect("the first combat").sides[0].minion,
        b,
        "swapped"
    );
    pick_when(&mut e, play(0, 2, a));
    let mut started = false;
    for _ in 0..40 {
        let req = request(&mut e);
        let pocket = req.options.iter().copied().find(|o| {
            matches!(
                o,
                Choice::UseAbility {
                    source: AbilitySource::Card(_),
                    ..
                }
            )
        });
        if let Some(o) = pocket {
            // Burned as the new combat begins.
            e.state.cards[nar.index()].in_play = false;
            pick(&mut e, o);
            started = true;
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(started, "Pocket Out of Time's new combat");
    request(&mut e);
    assert_eq!(
        e.combat().expect("the new combat").sides[0].minion,
        a,
        "its own pair's order"
    );
}
