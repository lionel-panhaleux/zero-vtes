//! Round 26 (`ADDED_BATCHES`, slice 16al): Gehenna and Anarchs completed
//! (owner, 2026-11-06). Readings in rules-qa Batch 111. Windows with a
//! single option are elided, so the scripts only stop where a real decision
//! exists.

use zvtes::cards::Target;
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId, CardInPlay};

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

/// The `nth` untitled vampire matching `f` with no printed-text
/// definition.
fn textless(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            f(c) && c.title == Title::None
                && !c.imbued
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a textless vampire")
}

/// The `nth` untitled textless vampire of `sect`, capacity 4 or more.
fn of_sect(sect: Sect, nth: usize) -> u16 {
    textless(|c| c.sect == Some(sect) && c.capacity >= 4, nth)
}

/// A capacity-10 Black Hand vampire whose text acts in her own combats
/// and bleeds only (Marthe Dizier): no untitled textless one is left.
fn elder() -> u16 {
    find_card("Marthe Dizier")
}

/// The crypt card `name`, advanced or not.
fn find_card(name: &str) -> u16 {
    find_vampire(name, false)
}

/// The crypt card `name` of that advanced-ness.
fn find_vampire(name: &str, adv: bool) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.adv == adv)
        .unwrap_or_else(|| panic!("no {name} adv {adv}")) as u16
}

/// A card already in play for `seat`, on `host` if given: its id.
fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), SeatId(seat), host, 0));
    CardId(e.state.cards.len() as u32 - 1)
}

/// The cards `name` in play on `host`.
fn on(e: &Engine, name: &str, host: MinionId) -> Vec<CardId> {
    let card = find_library(name);
    (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .filter(|c| {
            let x = &e.state.cards[c.index()];
            x.in_play && x.card == card && x.host == Some(host)
        })
        .collect()
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

/// Seat 0 plays its hand's `name` (mode 0) on `m` in its master phase.
fn master_on(e: &mut Engine, name: &str, m: MinionId) {
    let card = find_library(name);
    let i = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == card)
        .expect("in hand") as u8;
    pick_when(e, play_at(i, 0, None, Target::Minion(m)));
    request(e);
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

/// Seat 0's `a` plays the political action card in hand slot 0 and wins
/// its referendum.
fn pass_referendum(e: &mut Engine, a: MinionId) {
    pick(e, play(0, 0, a));
    pick_matching(e, |o| {
        *o == Choice::CastCardVote { favor: true } || *o == votes(a, true)
    });
    close_polling(e);
}

/// Seat 0's `a` bleeds, seat 1's `b` blocks: the combat's first round.
fn bleed_blocked(e: &mut Engine, a: MinionId, b: MinionId) {
    pick_when(e, bleed(a));
    pick_when(e, block(b));
}

// ---- Crusades ---------------------------------------------------------------

/// Crusade: Amsterdam, Berlin, Dublin, Frankfurt, Geneva, Paris,
/// Washington, D.C.: "If this referendum passes, put this card on the
/// acting vampire to represent the unique Sabbat title of Archbishop of X
/// … If this vampire is <clan>, he or she unlocks during your next discard
/// phase." The card itself on the caller, its title his; the clan unlocks,
/// another does not.
#[test]
fn crusades_title_their_caller_and_unlock_their_clan() {
    for (card, city, clan) in [
        ("Crusade: Amsterdam", "Amsterdam", Some("Ventrue antitribu")),
        ("Crusade: Berlin", "Berlin", Some("Lasombra")),
        ("Crusade: Dublin", "Dublin", Some("Gangrel antitribu")),
        ("Crusade: Frankfurt", "Frankfurt", None),
        ("Crusade: Geneva", "Geneva", Some("Malkavian antitribu")),
        ("Crusade: Paris", "Paris", None),
        (
            "Crusade: Washington, D.C.",
            "Washington, D.C.",
            Some("Ventrue antitribu"),
        ),
    ] {
        for own in [true, false] {
            if clan.is_none() && own {
                continue;
            }
            let mut e = game(2);
            let a = e.add_minion(
                SeatId(0),
                textless(
                    |c| {
                        c.sect == Some(Sect::Sabbat)
                            && c.capacity >= 4
                            && match clan {
                                Some(k) if own => c.clan == k,
                                Some(k) => c.clan != k,
                                None => true,
                            }
                    },
                    0,
                ),
                4,
                false,
            );
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
            hand(&mut e, 0, &[card]);
            pass_referendum(&mut e, a);
            let c = on(&e, card, a);
            assert_eq!(c.len(), 1, "{card}");
            assert!(!e.state.cards[c[0].index()].effect_only);
            assert_eq!(e.state.title_of(a), Title::Archbishop(city), "{card}");
            hand(&mut e, 0, &["Dodge"]);
            until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
            assert_eq!(
                !e.state.minions[a.index()].locked,
                own,
                "{card} {clan:?} {own}"
            );
        }
    }
}

// ---- Strikes, presses, maneuvers --------------------------------------------

/// Pushing the Limit, Undead Strength: "Strike: hand strike or use a melee
/// weapon strike. This strike is at +N damage." The hand strike at +2 / +1
/// (basic).
#[test]
fn pushing_the_limit_and_undead_strength_add_to_the_hand_strike() {
    for (card, bonus) in [("Pushing the Limit", 2), ("Undead Strength", 1)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &[card]);
        bleed_blocked(&mut e, a, b);
        pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, b), 4 - 1 - bonus, "{card}");
    }
}

/// Sacrament of Carnage, Starvation of Marena: "Strike: 2R damage" (with 1
/// optional press for Starvation), at long range too.
#[test]
fn sacrament_and_starvation_strike_at_range() {
    for card in ["Sacrament of Carnage", "Starvation of Marena"] {
        let disc = if card == "Sacrament of Carnage" {
            "pot"
        } else {
            "vic"
        };
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with(disc, 0), 8, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &[card]);
        bleed_blocked(&mut e, a, b);
        pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
        if card == "Starvation of Marena" {
            let req = to_window(&mut e, SeatId(0), WindowLabel::CombatPress);
            assert!(req.options.contains(&Choice::UsePress), "{card}");
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(blood(&e, b) <= 8 - 2, "{card}");
    }
}

/// Scorpion Sting [ANI]: "hand strike at +1 damage … cannot be dodged": a
/// dodge played against it has no effect (LSJ 20030902-2).
/// [LSJ 20030902-2] [LSJ 20060808-1] The dodge may be played, to no effect.
#[test]
fn scorpion_sting_superior_cannot_be_dodged() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ANI", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("chi", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Scorpion Sting"]);
    hand(&mut e, 1, &["Occlusion"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play_at(0, 1, Some(a), Target::None));
    pick_when(&mut e, play_at(0, 0, Some(b), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 4 - 2, "the dodge had no effect");
}

/// Dead-End Alley: "Press, only usable to continue combat"; Boxed In:
/// "Do not replace until after combat. Press." Boxed In stays out of the
/// hand's count until the combat ends.
#[test]
fn dead_end_alley_and_boxed_in_press() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Boxed In", "Dead-End Alley"]);
    bleed_blocked(&mut e, a, b);
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(req.options.contains(&play_at(0, 0, Some(a), Target::None)));
    assert!(req.options.contains(&play_at(1, 0, Some(a), Target::None)));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    assert_eq!(e.state.seats[0].hand.len(), 1, "Boxed In not replaced yet");
}

/// Form of the Ghost: "[pro] Maneuver. [PRO] Press."
#[test]
fn form_of_the_ghost_maneuvers_or_presses() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRO", 0), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Form of the Ghost"]);
    bleed_blocked(&mut e, a, b);
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&play_at(0, 0, Some(a), Target::None)));
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(req.options.contains(&play_at(0, 1, Some(a), Target::None)));
}

/// Blow Torch: "Weapon. 1 aggravated damage each strike": a close-range
/// weapon strike, aggravated.
#[test]
fn blow_torch_strikes_for_one_aggravated() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let torch = in_play(&mut e, 0, "Blow Torch", Some(a));
    bleed_blocked(&mut e, a, b);
    pick_matching(
        &mut e,
        |o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == torch),
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[b.index()].in_torpor,
        "1 aggravated, not mended"
    );
}

/// Skin of Night [FOR]: "treats aggravated damage as normal damage … and
/// prevent 1 damage": only with damage to prevent (LSJ 20001114).
/// [LSJ 20001114] No damage to prevent, not usable.
#[test]
fn skin_of_night_superior_needs_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Skin of Night"]);
    bleed_blocked(&mut e, a, b);
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatPrevention);
    assert!(req.options.contains(&play_at(0, 1, Some(a), Target::None)));
    pick_when(&mut e, play_at(0, 1, Some(a), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "1 damage, 1 prevented");
}

// ---- Action modifiers and reactions -----------------------------------------

/// Alacrity [CEL]: "+1 stealth … if this action is blocked, this acting
/// vampire gets an optional maneuver during the first round of the
/// resulting combat." Met by Spirit's Touch's +1 intercept, the block made:
/// the maneuver offered at the first range.
#[test]
fn alacrity_superior_maneuvers_when_blocked() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Alacrity"]);
    hand(&mut e, 1, &["Spirit's Touch", "Spirit's Touch"]);
    e.state.seats[1].hand.truncate(1);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play_at(0, 1, Some(a), Target::None));
    pick_when(&mut e, play_at(0, 0, Some(b), Target::None));
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver), "{req:?}");
}

/// Dread Gaze: "Only usable during a referendum. [pre] This reacting
/// vampire gains 2 votes. [PRE] … 4 votes."
#[test]
fn dread_gaze_gives_the_reacting_vampire_votes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let r = e.add_minion(SeatId(1), vampire_with("PRE", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    hand(&mut e, 1, &["Dread Gaze"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, alloc(0, 1));
    let req = to_window(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert!(req.options.contains(&play_at(0, 1, Some(r), Target::None)));
}

/// Irregular Protocol: "Lock this reacting vampire to force the acting
/// vampire to abstain (this can cancel that vampire's votes and ballots)."
#[test]
fn irregular_protocol_makes_the_caller_abstain() {
    let mut e = game(2);
    let prince = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            matches!(c.title, Title::Prince(_)) && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a textless prince") as u16;
    let a = e.add_minion(SeatId(0), prince, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let r = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    hand(&mut e, 1, &["Irregular Protocol"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, alloc(0, 1));
    pick(&mut e, votes(a, true));
    pick_when(&mut e, play_at(0, 0, Some(r), Target::None));
    request(&mut e);
    assert!(e.state.minions[r.index()].locked);
    close_polling(&mut e);
    assert_eq!(pool(&e), vec![10, 10], "the caller's votes canceled: fails");
}

/// Friend of Mine [obt]: "Only usable after a vampire successfully bleeds
/// you. The acting vampire burns 2 blood."
#[test]
fn friend_of_mine_burns_a_successful_bleeder() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    // No Anarch prints Obtenebration: a Lasombra made Anarch.
    let r = e.add_minion(SeatId(1), vampire_with("obt", 0), 4, false);
    e.state.minions[r.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Friend of Mine"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 2, Some(r), Target::None));
    request(&mut e);
    assert_eq!(blood(&e, a), 5 - 2);
}

/// The Mole [dom]: "Only usable when you are being bled by a younger
/// vampire. Lock this vampire to cause the action to fail." Not by a bleed
/// of an Anarch ("a non-anarch minion is acting").
#[test]
fn the_mole_fails_a_younger_non_anarchs_bleed() {
    for anarch_bleeder in [false, true] {
        let mut e = game(2);
        let bleeder = if anarch_bleeder {
            textless(|c| c.sect == Some(Sect::Anarch) && c.capacity <= 4, 0)
        } else {
            textless(|c| c.sect == Some(Sect::Camarilla) && c.capacity <= 4, 0)
        };
        let a = e.add_minion(SeatId(0), bleeder, 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let r = e.add_minion(
            SeatId(1),
            textless(
                |c| {
                    c.sect == Some(Sect::Anarch)
                        && c.capacity >= 6
                        && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("dom"))
                },
                0,
            ),
            4,
            false,
        );
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 1, &["Mole, The"]);
        pick_when(&mut e, bleed(a));
        let offered = offered_before(
            &mut e,
            play_at(0, 2, Some(r), Target::None),
            SeatId(0),
            WindowLabel::MinionPhase,
        );
        assert_eq!(offered, !anarch_bleeder);
    }
}

// ---- Actions ----------------------------------------------------------------

/// Force of Will: "Only usable by a locked vampire. [for] Ⓓ Bleed with +1
/// bleed. After resolution, this vampire takes 2 unpreventable aggravated
/// damage": offered to a locked vampire only.
#[test]
fn force_of_will_bleeds_from_a_locked_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 4, false);
    let u = e.add_minion(SeatId(0), vampire_with("for", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[a.index()].locked = true;
    hand(&mut e, 0, &["Force of Will"]);
    let req = to_minion_phase(&mut e, SeatId(0));
    assert!(req.options.contains(&play(0, 0, a)));
    assert!(!req.options.contains(&play(0, 0, u)));
    let before = e.state.seats[1].pool;
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, before - 2);
    assert!(
        e.state.minions[a.index()].in_torpor,
        "2 aggravated, unmended"
    );
}

/// Go Anarch: "Put this card on the acting vampire; they become Anarch and
/// unlock." No blood paid (ANK 20181017).
/// [ANK 20181017] Not the default action.
#[test]
fn go_anarch_makes_the_actor_anarch_and_unlocks() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Go Anarch"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.sect_of(a), Some(Sect::Anarch));
    assert!(!e.state.minions[a.index()].locked);
    assert_eq!(blood(&e, a), 4);
    assert_eq!(on(&e, "Go Anarch", a).len(), 1);
}

// ---- Masters ----------------------------------------------------------------

/// Into the Fire, Out of the Frying Pan, Galaric's Legacy, Seattle
/// Committee: sect changes on a ready vampire you control.
#[test]
fn sect_changing_masters() {
    for (card, from, to) in [
        ("Into the Fire", Sect::Camarilla, Sect::Sabbat),
        ("Out of the Frying Pan", Sect::Sabbat, Sect::Camarilla),
        ("Galaric's Legacy", Sect::Camarilla, Sect::Anarch),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(from, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &[card]);
        until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        master_on(&mut e, card, a);
        assert_eq!(e.state.sect_of(a), Some(to), "{card}");
    }
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let sc = in_play(&mut e, 0, "Seattle Committee", None);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick_matching(
        &mut e,
        |o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == sc),
    );
    request(&mut e);
    assert_eq!(e.state.sect_of(a), Some(Sect::Anarch));
    assert!(e.state.cards[sc.index()].locked);
}

/// Presence: "Master: Discipline … gains 1 level of Presence [pre] …
/// Cannot be placed on a vampire with superior Presence."
#[test]
fn presence_adds_a_level() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let before = e.state.discipline_level(a, "pre");
    hand(&mut e, 0, &["Presence"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Presence", a);
    assert_eq!(e.state.discipline_level(a, "pre"), before + 1);
}

/// Carnivale: "Put this card on a ready Toreador. You may burn this card
/// to give this Toreador +1 stealth for the current action." Not on
/// another clan.
#[test]
fn carnivale_goes_on_a_toreador() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
    let br = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Carnivale"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(!req
        .options
        .contains(&play_at(0, 0, None, Target::Minion(br))));
    master_on(&mut e, "Carnivale", t);
    assert_eq!(on(&e, "Carnivale", t).len(), 1);
}

// ---- Anarch actions ---------------------------------------------------------

/// Principia Discordia [ser]: "Ⓓ Steal 1 blood from a ready vampire"; at
/// one with no blood too (RTR 20010711).
/// [RTR 20010711] A vampire with no blood can be targeted.
#[test]
fn principia_discordia_steals_a_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 2, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Principia Discordia"]);
    pick(&mut e, play_at(0, 2, Some(a), Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, t), 2);
    assert_eq!(blood(&e, a), 3);
}

/// Reformation [chi]: "Ⓓ Steal an equipment from your predator or prey."
#[test]
fn reformation_steals_an_equipment() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("chi", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(t));
    hand(&mut e, 0, &["Reformation"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Card(gun)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[gun.index()].host, Some(a));
}

/// Smoke and Mirrors [obf]: "+2 stealth. Not usable during a bleed or
/// political action."
#[test]
fn smoke_and_mirrors_obfuscate_is_not_for_a_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Smoke and Mirrors"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play_at(0, 1, Some(a), Target::None)),
        "{req:?}"
    );
}

/// Smoke and Mirrors [obt]: "The blocking vampire gets -1 intercept." A
/// reduction: [LSJ 20020612] "Can be played any time during the action
/// before resolution; a block attempt is not required" — played before any
/// attempt, to no effect.
#[test]
fn smoke_and_mirrors_obtenebration_needs_no_block_attempt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Smoke and Mirrors"]);
    pick_when(&mut e, bleed(a));
    let req = request(&mut e);
    let obt = play_at(0, 2, Some(a), Target::None);
    assert_eq!(req.seat, SeatId(0));
    assert!(e.action().expect("the action").blocker.is_none());
    assert!(req.options.contains(&obt), "{req:?}");
    pick(&mut e, obt);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Smoke and Mirrors")));
    assert_eq!(e.state.seats[1].pool, 9, "the bleed went on");
}

/// Sleep Unseen: "Only usable at the end of a successful action … Minions
/// without Auspex cannot perform actions directed at the vampire with this
/// card." On the actor; a vampire without Auspex cannot bleed-target it
/// (a Ⓓ enter-combat elsewhere aside).
/// [PIB 20150915] [LSJ 19981028] [ANK 20190425] Played after resolution, still
/// during the action.
#[test]
fn sleep_unseen_goes_on_after_a_success() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Sleep Unseen"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    request(&mut e);
    assert_eq!(on(&e, "Sleep Unseen", a).len(), 1);
}

/// Body Bag: "If the anarch with this card is ready, he or she can burn 2
/// blood to cause an action directed at him or her to fail." Not for a
/// non-Anarch (LSJ 20030607).
/// [LSJ 20030607] A non-Anarch may equip it, a haven without its effect.
#[test]
fn body_bag_fails_an_action_at_its_anarch() {
    for anarch in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if anarch {
            e.state.minions[t.index()].sect_override = Some(Sect::Anarch);
        }
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let bag = in_play(&mut e, 1, "Body Bag", Some(t));
        hand(&mut e, 0, &["Principia Discordia"]);
        e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
        pick(&mut e, play_at(0, 2, Some(a), Target::Minion(t)));
        let offered = offered_before(
            &mut e,
            ability(bag.0, 0),
            SeatId(0),
            WindowLabel::MinionPhase,
        );
        assert_eq!(offered, anarch);
    }
}

/// Cooler: "Put 4 blood counters on this card. During your unlock phase,
/// if the vampire with this equipment is ready, you may move 1 blood from
/// this card to this vampire."
#[test]
fn cooler_feeds_its_bearer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = in_play(&mut e, 0, "Cooler", Some(a));
    e.state.cards[c.index()].counters = 4;
    to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(c),
            ability: 0,
            actor: None,
            target: Target::Minion(a),
        },
    );
    request(&mut e);
    assert_eq!(blood(&e, a), 2);
    assert_eq!(e.state.cards[c.index()].counters, 3);
}

// ---- Politics ---------------------------------------------------------------

/// Esgrima: "Lock during the referendum of a political action to give 3
/// additional votes to a vampire with a capacity above 6. During your
/// discard phase, your predator takes control of Esgrima." Not to a
/// capacity 6 or less.
#[test]
fn esgrima_gives_votes_then_passes_to_the_predator() {
    let mut e = game(2);
    let big = e.add_minion(SeatId(0), textless(|c| c.capacity >= 7, 0), 4, false);
    let small = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let es = in_play(&mut e, 0, "Esgrima", None);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, big));
    pick(&mut e, alloc(0, 1));
    let req = request(&mut e);
    let at = |m: MinionId| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(es),
        ability: 0,
        actor: None,
        target: Target::Minion(m),
    };
    assert!(req.options.contains(&at(big)), "{req:?}");
    assert!(!req.options.contains(&at(small)));
    pick(&mut e, at(big));
    close_polling(&mut e);
    assert!(e.state.cards[es.index()].locked);
    hand(&mut e, 0, &["Dodge"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[es.index()].controller, SeatId(1));
}

/// Firebrand: "If this referendum is successful, put this card on the
/// acting anarch … During your minion phase, this anarch can burn 1 blood
/// to unlock a ready younger anarch."
#[test]
fn firebrand_goes_on_its_anarch_and_unlocks_a_younger_one() {
    let mut e = game(2);
    // No textless Anarch of capacity 8: a Camarilla one made Anarch.
    let a = e.add_minion(SeatId(0), elder(), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    let y = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Anarch) && c.capacity <= 4, 0),
        2,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[y.index()].locked = true;
    hand(&mut e, 0, &["Firebrand"]);
    pass_referendum(&mut e, a);
    let f = on(&e, "Firebrand", a);
    assert_eq!(f.len(), 1);
    let use_on_y = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(f[0]),
        ability: 0,
        actor: None,
        target: Target::Minion(y),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&use_on_y), "{req:?}");
    pick(&mut e, use_on_y);
    request(&mut e);
    assert!(!e.state.minions[y.index()].locked);
    assert_eq!(blood(&e, a), 3);
}

/// Servitor of Irad: "After any Methuselah plays a Gehenna card, you may
/// draw two additional cards from your library if this vampire is ready.
/// Discard down to your hand size afterward."
#[test]
fn servitor_of_irad_draws_after_a_gehenna_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Servitor of Irad", Some(a));
    hand(&mut e, 0, &["Fueled by Heart's Blood"]);
    for _ in 0..10 {
        e.state.seats[0].library.push(find_library("Form of Mist"));
    }
    let lib = e.state.seats[0].library.len();
    to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, play_at(0, 0, None, Target::None));
    request(&mut e);
    // No replacement ("do not replace until"), two drawn.
    assert!(
        e.state.seats[0].library.len() <= lib - 2,
        "{}",
        e.state.seats[0].library.len()
    );
}

// ---- Vampires ---------------------------------------------------------------

/// Antonino: "+1 stealth on undirected actions": a hunt, not a bleed.
#[test]
fn antonino_sneaks_on_undirected_actions() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Antonino"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick_when(&mut e, action(a, zvtes::decision::ActionKind::Hunt));
    // The hunt's +1 and his.
    assert_eq!(e.action().expect("the hunt").stealth(&e.state), 2);
}

/// Brunhilde (base): "+1 bleed when bleeding a Methuselah who controls a
/// ready Ventrue. +1 strength."
#[test]
fn brunhilde_bleeds_more_against_a_ventrue() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Brunhilde"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick_when(&mut e, bleed(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Lisa Noble: "-1 strength on the first round of combat": her hand strike
/// does nothing then.
#[test]
fn lisa_noble_is_weaker_in_the_first_round() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lisa Noble"), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.notes = Some(Vec::new());
    bleed_blocked(&mut e, l, b);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 4, "0 strength in round 1");
}

/// Beckett (base): "Beckett can allow you to draw 3 cards from your library
/// as a +1 stealth action. Discard down to your hand size afterward."
#[test]
fn beckett_draws_three_as_an_action() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Beckett"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for _ in 0..10 {
        e.state.seats[0].library.push(find_library("Form of Mist"));
    }
    fill_hand(&mut e, 0);
    let req = to_minion_phase(&mut e, SeatId(0));
    let act = req
        .options
        .iter()
        .find(|o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(v), .. } if *v == b))
        .cloned()
        .expect("Beckett's action");
    pick(&mut e, act);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].library.len(), 7, "three drawn");
    assert_eq!(e.state.seats[0].hand.len(), 7, "discarded down");
}

/// John Paleologus: "If John is ready during your unlock phase, he can burn
/// 2 blood to burn 1 pool from your prey."
#[test]
fn john_paleologus_burns_the_preys_pool() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("John Paleologus"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let act = req
        .options
        .iter()
        .find(|o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(v), .. } if *v == j))
        .cloned()
        .expect("John's ability");
    pick(&mut e, act);
    request(&mut e);
    assert_eq!(blood(&e, j), 2);
    assert_eq!(e.state.seats[1].pool, 9);
}

/// Torvus Bloodbeard: "During any Methuselah's minion phase, you may burn
/// the Edge to unlock Torvus." Once in a minion phase.
#[test]
fn torvus_burns_the_edge_to_unlock() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Torvus Bloodbeard"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].has_edge = true;
    pick_when(&mut e, bleed(t));
    let req = to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let act = req
        .options
        .iter()
        .find(|o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(v), .. } if *v == t))
        .cloned()
        .expect("Torvus's unlock");
    pick(&mut e, act);
    request(&mut e);
    assert!(!e.state.minions[t.index()].locked);
    assert!(!e.state.seats[0].has_edge);
}

/// Faruq: "may enter combat with any Assamite controlled by another
/// Methuselah as a Ⓓ action. In that combat, Faruq gets +1 strength."
#[test]
fn faruq_fights_a_banu_haqim_stronger() {
    let mut e = game(2);
    let f = e.add_minion(SeatId(0), find_card("Faruq"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), plain("Banu Haqim", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = to_minion_phase(&mut e, SeatId(0));
    let at = |m: MinionId| {
        req.options.iter().any(|o| {
            matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(x), target: Target::Minion(t), .. }
                if *x == f && *t == m)
        })
    };
    assert!(at(h) && !at(v));
}

/// The vampire `m`'s ability `ai` in `req`, if offered (any target).
fn vampire_use(req: &zvtes::decision::DecisionRequest, m: MinionId, ai: u8) -> Option<Choice> {
    req.options
        .iter()
        .find(|o| {
            matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(v), ability, .. }
                if *v == m && *ability == ai)
        })
        .cloned()
}

/// Assad Salhoum: "When Assad successfully blocks, he gains 1 blood from
/// the blood bank (before combat begins, if any)."
#[test]
fn assad_salhoum_gains_a_blood_blocking() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let s = e.add_minion(SeatId(1), find_card("Assad Salhoum"), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(s));
    request(&mut e);
    // 1 gained before combat, 1 lost to the hand strike.
    assert_eq!(blood(&e, s), 2);
}

/// Danielle Diron (base): "During your unlock phase, if Danielle is ready
/// and has no blood, move 2 blood from the blood bank to her."
#[test]
fn danielle_diron_refills_when_empty() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Danielle Diron"), 0, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    // Locked: no hunt before her next unlock phase.
    e.state.minions[d.index()].locked = true;
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, d), 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, d), 2);
}

/// Ramona (base) and advanced: blood from a ready Gangrel to her, from her
/// to one, in the unlock phase.
#[test]
fn ramona_moves_blood_with_her_gangrel() {
    for adv in [false, true] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), find_vampire("Ramona", adv), 2, false);
        let g = e.add_minion(SeatId(0), plain("Gangrel", 0), 2, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
        let u = vampire_use(&req, r, 0).expect("Ramona's ability");
        pick(&mut e, u);
        request(&mut e);
        let (from, to) = if adv { (r, g) } else { (g, r) };
        assert_eq!((blood(&e, from), blood(&e, to)), (1, 3), "adv {adv}");
    }
}

/// Matthew Romans: "During your unlock phase, Matthew can burn 1 blood to
/// lock a younger non-Independent, non-Anarch vampire controlled by your
/// prey."
#[test]
fn matthew_romans_locks_a_younger_prey_vampire() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Matthew Romans"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(
        SeatId(1),
        textless(|c| c.sect == Some(Sect::Camarilla) && c.capacity <= 5, 0),
        4,
        false,
    );
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let u = vampire_use(&req, m, 0).expect("Matthew's ability");
    assert!(matches!(u, Choice::UseAbility { target: Target::Minion(x), .. } if x == t));
    pick(&mut e, u);
    request(&mut e);
    assert!(e.state.minions[t.index()].locked);
    assert_eq!(blood(&e, m), 2);
}

/// Juggler: "Once each combat, Juggler can burn 1 blood to get a maneuver."
#[test]
fn juggler_buys_a_maneuver() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Juggler"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    bleed_blocked(&mut e, j, b);
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    let u = vampire_use(&req, j, 1).expect("Juggler's maneuver");
    pick(&mut e, u);
    request(&mut e);
    assert_eq!(blood(&e, j), 3);
}

// ---- Gehenna cards played ---------------------------------------------------

/// Seat 0 plays Fueled by Heart's Blood (a Gehenna event) in its discard
/// phase.
fn play_gehenna(e: &mut Engine) {
    hand(e, 0, &["Fueled by Heart's Blood"]);
    to_window(e, SeatId(0), WindowLabel::DiscardPhase);
    pick(e, play_at(0, 0, None, Target::None));
    request(e);
}

/// Antara: "He burns 1 blood after any Methuselah plays a Gehenna event";
/// Cyscek: "If Cyscek is ready after any Methuselah plays a Gehenna event,
/// you gain the Edge and that Methuselah burns 1 pool"; Jesse Menks: "If
/// you play a Gehenna card and Jesse is ready, he unlocks"; Marcel de
/// Breau: "If you have the Edge, you gain a pool when any Methuselah plays
/// a Gehenna card."
#[test]
fn vampires_react_to_a_gehenna_card_played() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jesse Menks"), 4, false);
    e.state.minions[j.index()].locked = true;
    let an = e.add_minion(SeatId(1), find_card("Antara"), 4, false);
    let c = e.add_minion(SeatId(1), find_card("Cyscek"), 4, false);
    let _ = c;
    e.add_minion(SeatId(0), find_card("Marcel de Breau"), 4, false);
    play_gehenna(&mut e);
    assert!(
        !e.state.minions[j.index()].locked,
        "Jesse: his player's card"
    );
    assert_eq!(blood(&e, an), 3, "Antara burns 1");
    assert!(
        e.state.seats[1].has_edge,
        "Cyscek's controller gains the Edge"
    );
    // Seat 0 burned 1 pool for Cyscek; Marcel's controller no longer had
    // the Edge by then, or never had it.
    assert_eq!(e.state.seats[0].pool, 9);
}

/// Cyscek's pool burn ousting the Methuselah who played the Gehenna event
/// in their own discard phase: the turn passes, the game goes on (a
/// pool burn's oust does not pass the turn itself; the play's settling
/// once left the game over with three Methuselahs, fuzz seed 45).
#[test]
fn cyscek_ousting_the_active_player_passes_the_turn() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), find_card("Cyscek"), 4, false);
    hand(&mut e, 0, &["Fueled by Heart's Blood"]);
    to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    e.state.seats[0].pool = 1;
    pick(&mut e, play_at(0, 0, None, Target::None));
    let next = request(&mut e);
    assert!(e.state.seats[0].ousted);
    assert_eq!(e.state.living(), 2);
    assert_eq!((e.state.active, next.seat), (SeatId(1), SeatId(1)));
}

/// Marcel de Breau: with the Edge, a pool for a Gehenna card played.
#[test]
fn marcel_de_breau_gains_with_the_edge() {
    for edge in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(1), find_card("Marcel de Breau"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.seats[1].has_edge = edge;
        play_gehenna(&mut e);
        assert_eq!(e.state.seats[1].pool, 10 + edge as i32, "edge {edge}");
    }
}

/// Beckett (advanced): "While Beckett is ready, you have +X hand size,
/// where X is the number of Gehenna cards in play. If Beckett leaves the
/// ready region, discard your hand."
#[test]
fn beckett_advanced_holds_more_per_gehenna_card() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_vampire("Beckett", true), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let size = e.state.hand_size(SeatId(0));
    in_play(&mut e, 1, "Fueled by Heart's Blood", None);
    in_play(&mut e, 0, "Fueled by Heart's Blood", None);
    assert_eq!(e.state.hand_size(SeatId(0)), size + 2);
    e.state.minions[b.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), size, "not ready");
}

/// Emergency Preparations: "Burn 1 pool to cancel a Gehenna card as it is
/// played." The card still played, not in play (LSJ 20080630).
#[test]
fn emergency_preparations_cancels_a_gehenna_card() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Fueled by Heart's Blood"]);
    hand(&mut e, 1, &["Emergency Preparations"]);
    to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, play_at(0, 0, None, Target::None));
    pick_when(&mut e, play_at(0, 0, None, Target::None));
    request(&mut e);
    assert_eq!(e.state.gehenna_in_play(), 0);
    assert_eq!(e.state.seats[1].pool, 9);
}

/// rules-qa 111.2 (owner, 2026-11-08: corrected): a Gehenna card canceled
/// as it is played is still played (ANK 20190104) and fires the "plays a
/// Gehenna card" triggers: Marcel de Breau's Methuselah, with the Edge,
/// cancels seat 0's with Emergency Preparations (1 pool) and gains 1.
#[test]
fn a_canceled_gehenna_card_fires_the_gehenna_triggers() {
    let mut e = game(2);
    e.add_minion(SeatId(1), find_card("Marcel de Breau"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Fueled by Heart's Blood"]);
    hand(&mut e, 1, &["Emergency Preparations"]);
    to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, play_at(0, 0, None, Target::None));
    pick_when(&mut e, play_at(0, 0, None, Target::None));
    request(&mut e);
    assert_eq!(e.state.gehenna_in_play(), 0, "canceled");
    assert_eq!(
        e.state.seats[1].pool,
        10 - 1 + 1,
        "the cancel's pool, Marcel's"
    );
}

/// Jan Pieterzoon (advanced): "Jan can burn 3 blood to cancel a Gehenna
/// card as it is played."
#[test]
fn jan_pieterzoon_cancels_a_gehenna_card() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let j = e.add_minion(SeatId(1), find_vampire("Jan Pieterzoon", true), 5, false);
    hand(&mut e, 0, &["Fueled by Heart's Blood"]);
    to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, play_at(0, 0, None, Target::None));
    let req = request(&mut e);
    let u = vampire_use(&req, j, 0).expect("Jan's cancel");
    pick(&mut e, u);
    request(&mut e);
    assert_eq!(e.state.gehenna_in_play(), 0);
    assert_eq!(blood(&e, j), 2);
}

// ---- Torpor actions ---------------------------------------------------------

/// Rapid Healing: "Only usable by a vampire in torpor. [for] The acting
/// vampire leaves torpor." No blood paid (ANK 20181017); [FOR] +1 blood.
/// [ANK 20181017] Not the default action.
#[test]
fn rapid_healing_leaves_torpor_for_free() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 1, false);
    e.state.minions[a.index()].in_torpor = true;
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Rapid Healing"]);
    let req = to_minion_phase(&mut e, SeatId(0));
    assert!(req.options.contains(&play(0, 1, a)), "{req:?}");
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_torpor);
    assert_eq!(blood(&e, a), 2);
}

/// Rapid Healing blocked: "there is no combat … that vampire may
/// diablerize the acting vampire."
/// [LSJ 19980126] A leave torpor action: blocked, the diablerie offered.
#[test]
fn rapid_healing_blocked_offers_the_diablerie() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 1, false);
    e.state.minions[a.index()].in_torpor = true;
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Rapid Healing"]);
    hand(&mut e, 1, &["Spirit's Touch"]);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play_at(0, 0, Some(b), Target::None));
    let req = until(&mut e, SeatId(1), WindowLabel::TorporBlock);
    assert_eq!(req.window, WindowLabel::TorporBlock);
    assert!(req.options.contains(&Choice::Diablerise));
}

/// Movement of the Slow Body: "Only usable by a locked vampire in torpor.
/// [pro] The acting vampire unlocks": it stays in torpor.
#[test]
fn movement_of_the_slow_body_unlocks_in_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pro", 0), 1, false);
    e.state.minions[a.index()].in_torpor = true;
    e.state.minions[a.index()].locked = true;
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Movement of the Slow Body"]);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[a.index()];
    assert!(v.in_torpor && !v.locked);
}

/// Root of Vitality [vic]: "Rescue a vampire from torpor": no blood paid
/// (ANK 20181017).
/// [ANK 20181017] Not the default action.
#[test]
fn root_of_vitality_rescues_for_free() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("vic", 0), 2, false);
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
    e.state.minions[t.index()].in_torpor = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Root of Vitality"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(t)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_torpor);
    assert_eq!((blood(&e, a), blood(&e, t)), (2, 1));
}

// ---- Archetypes -------------------------------------------------------------

/// Loner: "During your influence phase, if this vampire successfully
/// performed an action this turn and none of your other minions attempted
/// any actions this turn, he or she gains 1 blood." Another minion's
/// action bars it.
#[test]
fn loner_gains_when_acting_alone() {
    for alone in [true, false] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), plain("Brujah", 0), 2, false);
        let o = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let c = in_play(&mut e, 0, "Loner", Some(l));
        pick_when(&mut e, bleed(l));
        if !alone {
            pick_when(&mut e, bleed(o));
        }
        let offered = offered_before(&mut e, ability(c.0, 0), SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(offered, alone, "alone {alone}");
    }
}

/// Bravo: "when this vampire successfully performs an action to enter
/// combat with another, he or she gains 1 blood from the blood bank when
/// the combat ends, if he or she is still ready."
#[test]
fn bravo_gains_after_its_combat() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Faruq"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), plain("Banu Haqim", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = in_play(&mut e, 0, "Bravo", Some(b));
    let req = to_minion_phase(&mut e, SeatId(0));
    let u = req
        .options
        .iter()
        .find(|o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(x), target: Target::Minion(t), .. } if *x == b && *t == h))
        .cloned()
        .expect("Faruq's combat");
    pick(&mut e, u);
    pick_when(&mut e, ability(c.0, 0));
    request(&mut e);
    assert!(blood(&e, b) >= 4, "its gain after the combat");
}

/// Entrenching: "If this vampire has 4 or more blood, he or she gains 4
/// blood." Read at resolution: 3 blood, nothing.
#[test]
fn entrenching_doubles_down_from_four() {
    for start in [4u8, 3] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), elder(), start, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Entrenching"]);
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let want = if start >= 4 { start + 4 } else { start };
        assert_eq!(blood(&e, a), want, "from {start}");
    }
}

/// Secret Horde: "Put this card in play and move 2X blood from the blood
/// bank to this card. You may use a master phase action to move 1 blood
/// from this card to your pool."
#[test]
fn secret_horde_banks_twice_x() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Secret Horde"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, play_at(0, 0, None, Target::None));
    pick(&mut e, Choice::SetX(3));
    request(&mut e);
    let c = (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|c| e.state.cards[c.index()].card == find_library("Secret Horde"))
        .expect("in play");
    assert_eq!(e.state.cards[c.index()].counters, 6);
    assert_eq!(e.state.seats[0].pool, 7);
}

// ---- Referendums ------------------------------------------------------------

/// Diversity: "each Methuselah gains X pool, where X is the number of clans
/// to which his or her ready vampires belong."
#[test]
fn diversity_pays_per_clan() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Diversity"]);
    pass_referendum(&mut e, a);
    assert_eq!(pool(&e), vec![12, 11]);
}

/// Finding the Path: "Choose at least half of the Methuselahs in the game
/// … each chosen Methuselah burns 1 pool and each of the other Methuselahs
/// gains 1 pool."
#[test]
fn finding_the_path_shifts_pool() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    for s in 1..4 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    hand(&mut e, 0, &["Finding the Path"]);
    pick(&mut e, play(0, 0, a));
    let req = to_window(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert!(
        !req.options.contains(&Choice::Pass),
        "two at least: {req:?}"
    );
    pick(&mut e, Choice::ChooseSeat(SeatId(1)));
    pick(&mut e, Choice::ChooseSeat(SeatId(2)));
    pick(&mut e, Choice::Pass);
    pick_matching(&mut e, |o| {
        *o == Choice::CastCardVote { favor: true } || *o == votes(a, true)
    });
    close_polling(&mut e);
    assert_eq!(pool(&e), vec![11, 9, 9, 11]);
}

/// Snipe Hunt: "lock all vampires with a capacity less than 4. Vampires
/// with a capacity less than 4 do not unlock as normal during the unlock
/// phase. Burn this card during your next master phase."
#[test]
fn snipe_hunt_pins_small_vampires() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 6, 0), 4, false);
    let small = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Snipe Hunt"]);
    pass_referendum(&mut e, a);
    assert!(e.state.minions[small.index()].locked);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[small.index()].locked, "no unlock as normal");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let burned = !e
        .state
        .cards
        .iter()
        .any(|c| c.present() && c.card == find_library("Snipe Hunt"));
    assert!(burned, "burned in the next master phase");
}

/// Year of Fortune: "Each chosen Methuselah gets +1 hand size"; another
/// copy cannot choose them again.
#[test]
fn year_of_fortune_grows_the_chosen_hands() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let size = e.state.hand_size(SeatId(0));
    hand(&mut e, 0, &["Year of Fortune"]);
    pick(&mut e, play(0, 0, a));
    to_window(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    pick(&mut e, Choice::ChooseSeat(SeatId(0)));
    pick(&mut e, Choice::Pass);
    pick_matching(&mut e, |o| {
        *o == Choice::CastCardVote { favor: true } || *o == votes(a, true)
    });
    close_polling(&mut e);
    assert_eq!(e.state.hand_size(SeatId(0)), size + 1);
    assert_eq!(e.state.hand_size(SeatId(1)), size);
}

/// Sweeper: "put this card on the chosen anarch. During your unlock phase,
/// if the anarch with this card is ready, you may look at the top card of
/// each Methuselah's library (including your own)."
#[test]
fn sweeper_looks_at_each_library() {
    let mut e = game(2);
    let baron = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            matches!(c.title, Title::Baron(_)) && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a textless baron") as u16;
    let a = e.add_minion(SeatId(0), baron, 4, false);
    let an = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Anarch), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for s in 0..2 {
        e.state.seats[s].library.push(find_library("Form of Mist"));
    }
    hand(&mut e, 0, &["Sweeper"]);
    pick(&mut e, play(0, 0, a));
    to_window(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    pick(&mut e, Choice::ChooseMinion(an));
    pick_matching(&mut e, |o| {
        *o == Choice::CastCardVote { favor: true } || *o == votes(a, true)
    });
    close_polling(&mut e);
    let s = on(&e, "Sweeper", an);
    assert_eq!(s.len(), 1);
    to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    e.notes = Some(Vec::new());
    pick(&mut e, ability(s[0].0, 0));
    request(&mut e);
    assert_eq!(noted_looks(&mut e).len(), 2);
}

/// Pack Tactics: "+2 intercept. A vampire cannot play both Pack Tactics and
/// Elder Intervention during the same action": the one after the other
/// not offered to the same vampire.
#[test]
fn pack_tactics_excludes_elder_intervention() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    // Stealth 3: intercept needed past Pack Tactics' +2.
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Stealth(3),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    hand(&mut e, 1, &["Pack Tactics", "Elder Intervention"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play_at(0, 0, Some(b), Target::None));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(1), "{req:?}");
    assert!(
        !plays(&req)
            .iter()
            .any(|p| matches!(p, Choice::Play { actor: Some(x), .. } if *x == b)),
        "{req:?}"
    );
}

/// Momentum's Edge: "Only usable if you have one or more victory points …
/// lock this card during your unlock phase to gain 1 pool."
#[test]
fn momentums_edge_needs_a_victory_point() {
    for vp in [0u32, 2] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[0].half_vps = vp;
        hand(&mut e, 0, &["Momentum's Edge"]);
        to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let offered = offered_before(
            &mut e,
            play_at(0, 0, None, Target::None),
            SeatId(0),
            WindowLabel::MinionPhase,
        );
        assert_eq!(offered, vp > 0);
    }
}

/// Aeron: "Minions opposing Aeron in combat take an additional point of
/// damage during strike resolution if the range is close."
/// [RTR 19970630] [LSJ 20070328] The added point is environmental.
#[test]
fn aeron_hurts_at_close_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Aeron"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    bleed_blocked(&mut e, a, b);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // His hand strike and 1 environmental.
    assert!(blood(&e, b) <= 4 - 2, "{}", blood(&e, b));
}

/// Karsh (base): "may enter combat with a locked non-Camarilla vampire
/// controlled by another Methuselah as a Ⓓ action."
#[test]
fn karsh_fights_a_locked_non_camarilla() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Karsh"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let s = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 0), 4, false);
    let c = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 0), 4, false);
    e.state.minions[s.index()].locked = true;
    e.state.minions[c.index()].locked = true;
    let req = to_minion_phase(&mut e, SeatId(0));
    let at = |m: MinionId| {
        req.options.iter().any(|o| {
            matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(x), target: Target::Minion(t), .. }
                if *x == k && *t == m)
        })
    };
    assert!(at(s) && !at(c));
}

/// Jeremy Talbot: "Jeremy gains 1 blood if your pool total is even and
/// burns 1 blood if your pool total is odd."
#[test]
fn jeremy_talbot_reads_the_pool_parity() {
    for (pool, want) in [(10, 3), (9, 1)] {
        let mut e = game(2);
        let j = e.add_minion(SeatId(0), find_card("Jeremy Talbot"), 2, false);
        e.state.minions[j.index()].locked = true;
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[0].pool = pool;
        to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, j), want, "pool {pool}");
    }
}

/// Mukhtar Bey: "If there are five Gehenna cards in play during your unlock
/// phase, burn Mukhtar."
#[test]
fn mukhtar_bey_burns_under_five_gehenna_cards() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mukhtar Bey"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for _ in 0..5 {
        in_play(&mut e, 1, "Fueled by Heart's Blood", None);
    }
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[m.index()].present());
}

/// Sandra White: "If Sandra diablerizes an older vampire, the controller of
/// that vampire burns 1 pool."
#[test]
fn sandra_white_costs_an_older_victims_pool() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Sandra White"), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    e.state.minions[v.index()].in_torpor = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick_when(
        &mut e,
        Choice::TakeAction {
            minion: s,
            action: zvtes::decision::ActionKind::Diablerie,
            target: Some(v),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[v.index()].present());
    assert_eq!(e.state.seats[1].pool, 9);
}

/// Hector Trelane: "If Hector is sent to torpor or burned in combat, he is
/// burned, and the controller of the opposing minion gains 4 pool."
#[test]
fn hector_trelane_burns_and_pays_his_opponent() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hector Trelane"), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Undead Strength"]);
    bleed_blocked(&mut e, h, b);
    pick_when(&mut e, play_at(0, 0, Some(b), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[h.index()].present(), "burned");
    assert_eq!(e.state.seats[1].pool, 14);
}

// ---- Merged vampires --------------------------------------------------------

/// Ramona merged: "[MERGED] During your master phase, you may move 1 blood
/// from Ramona to your pool", her two cards' unlock moves kept.
#[test]
fn ramona_merged_moves_blood_to_the_pool() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_vampire("Ramona", true), 3, false);
    e.state.minions[r.index()].merged = Some(find_vampire("Ramona", false));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let u = vampire_use(&req, r, 2).expect("the merged pool move");
    pick(&mut e, u);
    request(&mut e);
    assert_eq!((blood(&e, r), e.state.seats[0].pool), (2, 11));
}

/// Quentin King III merged: "Once each action, Quentin can burn 1 blood to
/// get +1 bleed"; unmerged, nothing. [ANK 20231124] [LSJ 19970718] "can
/// only be used during a bleed action before Action Resolution": taken in
/// his bleed, the prey burns 2.
#[test]
fn quentin_king_merged_bleeds_more() {
    for merged in [true, false] {
        let mut e = game(2);
        let q = e.add_minion(SeatId(0), find_vampire("Quentin King III", true), 3, false);
        if merged {
            e.state.minions[q.index()].merged = Some(find_vampire("Quentin King III", false));
        }
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let abilities =
            zvtes::cards::vampire_of(&e.state.minions[q.index()]).map_or(0, |d| d.abilities.len());
        assert_eq!(abilities, if merged { 2 } else { 0 });
        if merged {
            pick(&mut e, bleed(q));
            let mut req = request(&mut e);
            while vampire_use(&req, q, 1).is_none() {
                e.choose(fallback(&req));
                req = request(&mut e);
            }
            assert!(e.action().is_some(), "before resolution");
            pick(&mut e, vampire_use(&req, q, 1).expect("the bleed"));
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            assert_eq!((blood(&e, q), e.state.seats[1].pool), (2, 8));
        }
    }
    // A hunt: never offered.
    let mut e = game(2);
    let q = e.add_minion(SeatId(0), find_vampire("Quentin King III", true), 3, false);
    e.state.minions[q.index()].merged = Some(find_vampire("Quentin King III", false));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, action(q, zvtes::decision::ActionKind::Hunt));
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(v), ability: 1, .. } if *v == q),
    );
}

/// Lucita (advanced): "During an action Lucita performs, she can burn 2
/// blood to cancel a reaction card as it is played by a vampire with
/// capacity 5 or less, and its cost is not paid."
#[test]
fn lucita_cancels_a_small_vampires_reaction() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_vampire("Lucita", true), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r = e.add_minion(
        SeatId(1),
        textless(|c| c.capacity <= 5 && c.disciplines.contains(&"aus"), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Spirit's Touch"]);
    e.state.minions[l.index()].timed.push((
        zvtes::cards::Lasting::Stealth(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    pick_when(&mut e, bleed(l));
    pick_when(&mut e, block(r));
    pick_when(&mut e, play_at(0, 0, Some(r), Target::None));
    let req = request(&mut e);
    let u = vampire_use(&req, l, 0).expect("Lucita's cancel");
    pick(&mut e, u);
    request(&mut e);
    assert_eq!(blood(&e, l), 2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "blocked no more: bled");
}

/// Marcus Vitel (advanced): "Any Brujah can enter combat with Marcus as a
/// Ⓓ action": another Methuselah's Brujah, not a Ventrue.
#[test]
fn marcus_vitel_draws_brujah_into_combat() {
    let mut e = game(2);
    let br = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let m = e.add_minion(SeatId(1), find_vampire("Marcus Vitel", true), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let req = to_minion_phase(&mut e, SeatId(0));
    let by = |a: MinionId| {
        req.options.iter().any(|o| {
            matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(x), actor: Some(y), target: Target::Minion(t), .. }
                if *x == m && *y == a && *t == m)
        })
    };
    assert!(by(br) && !by(v), "{req:?}");
}

/// Giangaleazzo: "If there are any Gehenna events in play during your
/// discard phase, he can burn 1 blood to unlock."
#[test]
fn giangaleazzo_unlocks_under_gehenna() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Giangaleazzo, The Traitor"), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Fueled by Heart's Blood", None);
    e.state.minions[g.index()].locked = true;
    let req = to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    let u = vampire_use(&req, g, 1).expect("his unlock");
    pick(&mut e, u);
    request(&mut e);
    assert!(!e.state.minions[g.index()].locked);
    assert_eq!(blood(&e, g), 2);
}

/// Alfred Benezri (advanced): "If Alfred is ready, he can lock before range
/// is determined to end combat involving two other Sabbat vampires."
#[test]
fn alfred_benezri_ends_a_sabbat_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 0), 4, false);
    let al = e.add_minion(SeatId(0), find_vampire("Alfred Benezri", true), 4, false);
    let b = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    bleed_blocked(&mut e, a, b);
    let req = request(&mut e);
    let u = vampire_use(&req, al, 0).expect("Alfred's end");
    pick(&mut e, u);
    request(&mut e);
    assert!(e.state.minions[al.index()].locked);
    assert_eq!((blood(&e, a), blood(&e, b)), (4, 4), "no strike");
}

/// Theo Bell (advanced): "If there are any Gehenna cards in play, any
/// justicar or Inner Circle member can call a referendum to burn Theo as a
/// +1 stealth political action. If that referendum is canceled or fails,
/// the acting vampire goes to torpor."
#[test]
fn theo_bell_can_be_hunted_by_a_justicar() {
    for pass in [true, false] {
        let mut e = game(2);
        let justicar = CRYPT
            .iter()
            .enumerate()
            .position(|(i, c)| {
                matches!(c.title, Title::Justicar(_)) && zvtes::cards::vampire(i as u16).is_none()
            })
            .expect("a textless justicar") as u16;
        let j = e.add_minion(SeatId(0), justicar, 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let t = e.add_minion(SeatId(1), find_vampire("Theo Bell", true), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        in_play(&mut e, 1, "Fueled by Heart's Blood", None);
        let req = to_minion_phase(&mut e, SeatId(0));
        let call = req
            .options
            .iter()
            .find(|o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(x), actor: Some(y), .. } if *x == t && *y == j))
            .cloned()
            .expect("the referendum to burn Theo");
        pick(&mut e, call);
        pick_matching(&mut e, |o| *o == votes(j, pass));
        close_polling(&mut e);
        request(&mut e);
        assert_eq!(e.state.minions[t.index()].present(), !pass, "pass {pass}");
        assert_eq!(e.state.minions[j.index()].in_torpor, !pass, "pass {pass}");
    }
}

// Theo Bell (advanced): "any justicar or Inner Circle member can call a
// referendum to burn Theo" is an ability, no card Karl plays: [LSJ 20101013]
// "not treated as meeting the requirements for effects the card has from
// being in play" — with a Gehenna card in play, Karl is not offered it.
#[test]
fn karl_schrekt_cannot_call_theo_bells_referendum() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Karl Schrekt"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let theo = e.add_minion(SeatId(1), find_vampire("Theo Bell", true), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Fueled by Heart's Blood", None);
    let req = to_minion_phase(&mut e, SeatId(0));
    assert!(
        !req.options.iter().any(|o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(x), .. } if *x == theo)),
        "{:?}",
        req.options
    );
}

/// Gharston Roland: "cannot perform an action if any minion has already
/// performed an action (successful or not) this turn."
#[test]
fn gharston_roland_acts_first_or_not_at_all() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Gharston Roland"), 4, false);
    let o = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = to_minion_phase(&mut e, SeatId(0));
    assert!(req.options.contains(&bleed(g)));
    pick(&mut e, bleed(o));
    assert!(!offered_before(
        &mut e,
        bleed(g),
        SeatId(1),
        WindowLabel::MinionPhase
    ));
}

/// Iliana: "Contracts cannot be placed on Iliana." Shakar: the Hunt not
/// offered on her, offered on another vampire.
#[test]
fn iliana_takes_no_contract() {
    let mut e = game(2);
    let bh = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| c.black_hand && zvtes::cards::vampire(i as u16).is_none())
        .expect("a textless Black Hand vampire") as u16;
    e.add_minion(SeatId(0), bh, 4, false);
    let il = e.add_minion(SeatId(1), find_card("Iliana"), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Shakar: the Hunt"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(
        req.options
            .contains(&play_at(0, 0, None, Target::Minion(v))),
        "{req:?}"
    );
    assert!(!req
        .options
        .contains(&play_at(0, 0, None, Target::Minion(il))));
}

/// Count Germaine: "Minions opposing Germaine in combat cannot use
/// weapons": the blocker's Blow Torch strike not offered against him.
#[test]
fn count_germaine_disarms_his_opponents() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Count Germaine"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let torch = in_play(&mut e, 1, "Blow Torch", Some(b));
    bleed_blocked(&mut e, g, b);
    assert!(!offered_before(
        &mut e,
        ability(torch.0, 0),
        SeatId(0),
        WindowLabel::MinionPhase
    ));
}

/// Ferox (advanced): "Minions opposing Ferox in combat cannot play cards
/// that require Necromancy [nec] or Blood Sorcery [tha]. He cannot commit
/// diablerie."
#[test]
fn ferox_bars_sorcery_against_him() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("tha", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let f = e.add_minion(
        SeatId(1),
        find_vampire("Ferox, The Rock Lord", true),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Theft of Vitae"]);
    bleed_blocked(&mut e, a, f);
    assert!(!offered_before(
        &mut e,
        play_at(0, 0, Some(a), Target::None),
        SeatId(1),
        WindowLabel::MinionPhase
    ));
    assert!(e.state.cannot_diablerize(f));
}

/// Michael Luther: "When Michael enters combat, you may draw 1 card.
/// Discard down to your hand size afterward."
#[test]
fn michael_luther_draws_entering_combat() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Michael Luther"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    for _ in 0..5 {
        e.state.seats[0].library.push(find_library("Form of Mist"));
    }
    fill_hand(&mut e, 0);
    let lib = e.state.seats[0].library.len();
    bleed_blocked(&mut e, m, b);
    request(&mut e);
    assert_eq!(e.state.seats[0].library.len(), lib - 1);
}

/// Amenophobis: "+1 bleed against the Methuselah who has the Edge."
#[test]
fn amenophobis_bleeds_the_edge_holder_harder() {
    for edge in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Amenophobis"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[1].has_edge = edge;
        pick_when(&mut e, bleed(a));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - 1 - edge as i32, "edge {edge}");
    }
}

/// Harry Reese: "Damage-dealing strikes Harry makes against non-mortal
/// allies are at +1 damage": any ally without the Mortal label, a
/// werewolf or an Animal (no monster, rules-qa 102.8) blocker takes 2 from
/// his hand strike, a mortal 1 (owner, 2026-11-08, rules-qa 111.47).
#[test]
fn harry_reese_hits_non_mortal_allies_harder() {
    for (ally, lost) in [
        ("Renegade Garou", 2u8),
        ("Underbridge Stray", 2),
        ("47th Street Royals", 1),
    ] {
        let mut e = game(2);
        let h = e.add_minion(SeatId(0), find_card("Harry Reese"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let a = e.add_ally(SeatId(1), find_library(ally));
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.notes = Some(Vec::new());
        bleed_blocked(&mut e, h, a);
        to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let hits: Vec<_> = noted_combats(&mut e)
            .into_iter()
            .filter_map(|n| match n {
                zvtes::combat::CombatNote::Strikes {
                    minions, damage, ..
                } => Some(if minions[0] == h {
                    damage[0]
                } else {
                    damage[1]
                }),
                _ => None,
            })
            .collect();
        assert_eq!(hits.first().copied(), Some(lost), "{ally} {hits:?}");
    }
}

/// Spider: "If Spider is blocked and the blocking minion is still ready at
/// the end of the action, Spider burns 1 blood."
#[test]
fn spider_pays_for_a_standing_blocker() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Spider"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    bleed_blocked(&mut e, s, b);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    // 1 damage mended, 1 for the standing blocker.
    assert_eq!(blood(&e, s), 4 - 1 - 1);
}

/// Muhandis: "must burn 1 blood to attempt to block a vampire."
#[test]
fn muhandis_pays_to_block_a_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let m = e.add_minion(SeatId(1), find_card("Muhandis"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick_when(&mut e, bleed(a));
    let req = request(&mut e);
    let toll = req
        .options
        .iter()
        .any(|o| matches!(o, Choice::PayBlockToll | Choice::DeclareBlock { .. }));
    assert!(toll, "{req:?}");
    pick_matching(
        &mut e,
        |o| matches!(o, Choice::DeclareBlock { minion } if *minion == m),
    );
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    // 1 to attempt, 1 to mend the hand strike.
    assert_eq!(blood(&e, m), 2);
}

/// Boss Callihan (base): "Callihan cannot rescue other vampires from
/// torpor."
#[test]
fn boss_callihan_rescues_no_one() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Boss Callihan"), 4, false);
    let o = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(0), plain("Brujah", 2), 1, false);
    e.state.minions[t.index()].in_torpor = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = to_minion_phase(&mut e, SeatId(0));
    let rescue = |m: MinionId| Choice::TakeAction {
        minion: m,
        action: zvtes::decision::ActionKind::Rescue,
        target: Some(t),
    };
    assert!(req.options.contains(&rescue(o)), "{req:?}");
    assert!(!req.options.contains(&rescue(c)));
}

/// Maldavis (base): "Maldavis gets -1 bleed when bleeding a Methuselah who
/// controls a ready primogen."
#[test]
fn maldavis_bleeds_less_against_a_primogen() {
    for primogen in [true, false] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Maldavis"), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let target = if primogen {
            CRYPT
                .iter()
                .enumerate()
                .position(|(i, c)| {
                    c.title == Title::Primogen && zvtes::cards::vampire(i as u16).is_none()
                })
                .expect("a textless primogen") as u16
        } else {
            plain("Ventrue", 0)
        };
        let t = e.add_minion(SeatId(1), target, 4, false);
        // Locked: no block.
        e.state.minions[t.index()].locked = true;
        pick_when(&mut e, bleed(m));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, if primogen { 10 } else { 9 });
    }
}

/// Car Bomb: "If the acting minion has a vehicle, the action fails, the
/// vehicle is burned, and the acting minion takes 2 damage (not
/// preventable). Otherwise, the action gets -1 stealth."
/// [PIB 20150820] Used as the action is announced, [LSJ 20081105] after the
/// acting Methuselah's own as-announced effects (none here), [ANK 20190425]
/// before any other modifier or reaction.
#[test]
fn car_bomb_wrecks_a_vehicle() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r = e.add_minion(
        SeatId(1),
        textless(|c| c.sect == Some(Sect::Anarch), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let bike = in_play(&mut e, 0, "Sport Bike", Some(a));
    hand(&mut e, 1, &["Car Bomb"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 0, Some(r), Target::None));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[bike.index()].in_play, "the vehicle burned");
    assert_eq!(blood(&e, a), 2);
    assert_eq!(e.state.seats[1].pool, 10, "the bleed failed");
}

/// Uncontrollable Rage [pro]: "This vampire's hand strikes inflict +1
/// damage, aggravated, until the end of the action": 2 aggravated on the
/// blocker, unmendable: torpor.
/// [TOM 19960225] [LSJ 20011204] All the damage aggravated, the +1 too.
#[test]
fn uncontrollable_rage_aggravates_the_hands() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pro", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Uncontrollable Rage"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    pick_when(&mut e, block(b));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[b.index()];
    assert!(
        v.in_torpor && v.blood == 3,
        "2 aggravated: 1 wounds, 1 burns a blood"
    );
}

/// Sleep of Reason [dem]: "Ⓓ Send a younger vampire with 0 or 1 blood to
/// torpor."
#[test]
fn sleep_of_reason_sends_a_weak_vampire_to_torpor() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.capacity >= 6 && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("dem")),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), textless(|c| c.capacity <= 2, 0), 1, false);
    e.state.minions[t.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Sleep of Reason"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(t)));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].in_torpor);
}

/// Songs of the Distant Vitae [QUI]: "+1 bleed against the controller of
/// any vampire who has successfully hunted since your last turn."
/// [LSJ 20020429] Since the end of your last turn: a hunt this turn counts.
#[test]
fn songs_of_the_distant_vitae_bleeds_hunters_harder() {
    for hunted in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("QUI", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let h = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, false);
        e.state.minions[h.index()].locked = true;
        if hunted {
            e.state.minions[h.index()].hunted_turn = Some(e.state.turn);
        }
        e.state.seats[0].previous_turn = e.state.turn - 2;
        hand(&mut e, 0, &["Songs of the Distant Vitae"]);
        pick_when(&mut e, play(0, 1, a));
        to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.seats[1].pool,
            10 - 1 - hunted as i32,
            "hunted {hunted}"
        );
    }
}

/// Revelation of Desire [SER]: "Burn 1 of your corruption counters from a
/// minion controlled by the target Methuselah to get +3 bleed."
#[test]
fn revelation_of_desire_spends_corruption_for_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("SER", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let c = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[c.index()].locked = true;
    e.state.minions[c.index()].corruption.push((SeatId(0), 1));
    hand(&mut e, 0, &["Revelation of Desire"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 1, Some(a), Target::Minion(c)));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 4);
    assert_eq!(e.state.minions[c.index()].corruption_of(SeatId(0)), 0);
}

/// Kahina the Sorceress: "call a referendum to inflict 1 damage on each
/// minion who has any of your corruption counters": 1 each (LSJ
/// 20030820).
/// [LSJ 20030820] One damage a minion.
#[test]
fn kahina_hurts_the_corrupted() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Kahina the Sorceress"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let c = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    let d = e.add_minion(SeatId(1), plain("Ventrue", 1), 3, false);
    e.state.minions[c.index()].corruption.push((SeatId(0), 2));
    let req = to_minion_phase(&mut e, SeatId(0));
    let u = vampire_use(&req, k, 0).expect("her referendum");
    pick(&mut e, u);
    pick_matching(&mut e, |o| *o == votes(k, true));
    close_polling(&mut e);
    request(&mut e);
    assert_eq!((blood(&e, c), blood(&e, d)), (2, 3));
}

/// Ahriman's Demesne: "Only usable at long range. Not usable during the
/// first round of combat." Not offered in the first round.
#[test]
fn ahrimans_demesne_waits_for_a_later_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("OBT", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Ahriman's Demesne"]);
    bleed_blocked(&mut e, a, b);
    assert!(!offered_before(
        &mut e,
        play_at(0, 1, Some(a), Target::None),
        SeatId(1),
        WindowLabel::MinionPhase
    ));
}

/// Amaranth: "Only usable when the opposing vampire should go to torpor.
/// Diablerize the opposing vampire instead."
#[test]
fn amaranth_diablerizes_instead_of_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), elder(), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), textless(|c| c.capacity <= 4, 0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Amaranth"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[b.index()].present(), "diablerized");
    assert!(e.state.minions[a.index()].diablerie_turn.is_some());
}

/// Personal Involvement: "If your prey has more pool than you (after paying
/// the cost of this card), your prey burns 3 pool. Any Methuselah can
/// cancel this card as it is played by burning 2 pool (the cost of this
/// card is not paid in that case)."
#[test]
fn personal_involvement_hits_a_richer_prey_unless_canceled() {
    for cancel in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[1].pool = 12;
        hand(&mut e, 0, &["Personal Involvement"]);
        until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        pick(&mut e, play_at(0, 0, None, Target::None));
        let req = request(&mut e);
        if cancel {
            assert_eq!(req.seat, SeatId(1));
            pick(&mut e, Choice::PayToCancel);
        } else if req.seat == SeatId(1) {
            pick(&mut e, Choice::Pass);
        }
        request(&mut e);
        if cancel {
            assert_eq!(pool(&e), vec![10, 10], "canceled: its cost not paid");
        } else {
            assert_eq!(pool(&e), vec![7, 9]);
        }
    }
}

/// Church of the Order of St. Blaise: "Lock this card to add one counter to
/// a location you control that uses counters."
/// [PIB 20110712-1] [LSJ 20040518-2] Blood or pool counters alike:
/// Barranquilla's are blood.
#[test]
fn church_of_st_blaise_adds_a_counter() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ch = in_play(&mut e, 0, "Church of the Order of St. Blaise", None);
    let loc = in_play(&mut e, 0, "Powerbase: Barranquilla", None);
    let before = e.state.cards[loc.index()].counters;
    let use_on = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(ch),
        ability: 0,
        actor: None,
        target: Target::Card(loc),
    };
    pick_when(&mut e, use_on);
    request(&mut e);
    assert_eq!(e.state.cards[loc.index()].counters, before + 1);
    assert!(e.state.cards[ch.index()].locked);
}

/// Paragon: "A younger vampire who successfully blocks this vampire burns 1
/// blood (before combat begins, if any)."
#[test]
fn paragon_taxes_a_younger_blocker() {
    let mut e = game(2);
    let v = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Ventrue" && c.capacity >= 6, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), textless(|c| c.capacity <= 4, 0), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 0, "Paragon", Some(v));
    pick_when(&mut e, bleed(v));
    pick_when(&mut e, block(b));
    request(&mut e);
    // 1 for blocking, 1 to mend the hand strike.
    assert_eq!(blood(&e, b), 1);
}

/// Games of Instinct: "At the end of that combat, if only one combatant is
/// ready, that ready vampire gains blood equal to the opposing vampire's
/// capacity from the blood bank." [LSJ 20100324] The gain applied after the
/// combat ends.
#[test]
fn games_of_instinct_feeds_the_lone_survivor() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Sabbat) && (5..=6).contains(&c.capacity),
            0,
        ),
        1,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), elder(), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let cap = e.state.capacity_of(t).unwrap();
    hand(&mut e, 0, &["Games of Instinct"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(t)));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].in_torpor);
    // 1, less 1 mended, plus the capacity.
    assert_eq!(blood(&e, a), cap.min(e.state.capacity_of(a).unwrap()));
}

/// Border Skirmish: "Each other Methuselah discards one card at random from
/// his or her hand, and this anarch unlocks at the end of the turn."
#[test]
fn border_skirmish_shakes_every_hand() {
    let mut e = game(3);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Anarch), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    for s in 1..3 {
        hand(&mut e, s as u8, &["Dodge", "Dodge"]);
        e.state.seats[s].library.push(find_library("Form of Mist"));
    }
    hand(&mut e, 0, &["Border Skirmish"]);
    pick_when(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert!(
        e.state.minions[a.index()].locked,
        "until the end of the turn: {req:?}"
    );
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    for s in 1..3 {
        let seat = &e.state.seats[s];
        assert_eq!(
            (seat.ash.len(), seat.hand.len()),
            (1, 2),
            "seat {s}: one discarded, replaced"
        );
    }
    assert!(!e.state.minions[a.index()].locked);
}

/// Sowing Dissension: "Requires a ready anarch. Ⓓ Each other Methuselah
/// locks an unlocked ready younger vampire he or she controls." Directed at
/// each other Methuselah (PIB 20150821): seat 1 locks its younger vampire,
/// not its older one; seat 2, holding only older ones, locks nothing.
/// [PIB 20150821] [RTR 20080808] Directed at all other Methuselahs.
#[test]
fn sowing_dissension_locks_a_younger_vampire_at_each_other_table() {
    let mut e = game(3);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Anarch) && (6..=8).contains(&c.capacity),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 4, false);
    let old = e.add_minion(SeatId(1), elder_camarilla(), 4, false);
    let other = e.add_minion(SeatId(2), elder(), 4, false);
    hand(&mut e, 0, &["Sowing Dissension"]);
    pick_when(&mut e, play(0, 0, a));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let locked = |m: MinionId| e.state.minions[m.index()].locked;
    assert!(locked(young), "the younger vampire");
    assert!(!locked(old) && !locked(other), "older ones stay");
}

/// Sowing Dissension: "If this action is blocked by a non-titled vampire,
/// the acting anarch can burn 1 blood to cancel combat and unlock."
#[test]
fn sowing_dissension_buys_its_way_out_of_an_untitled_block() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Anarch) && (6..=8).contains(&c.capacity),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Sowing Dissension"]);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, Choice::CancelCombat);
    request(&mut e);
    assert!(e.combat().is_none(), "combat canceled");
    let v = &e.state.minions[a.index()];
    assert_eq!((v.blood, v.locked), (3, false), "1 blood burned, unlocked");
}

/// Sowing Dissension: a titled blocker leaves no way out.
#[test]
fn sowing_dissension_cannot_escape_a_titled_blocker() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Anarch) && (6..=8).contains(&c.capacity),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let prince = CRYPT
        .iter()
        .position(|c| matches!(c.title, Title::Prince(_)) && !c.imbued)
        .expect("a prince") as u16;
    let b = e.add_minion(SeatId(1), prince, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Sowing Dissension"]);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, block(b));
    let r = request(&mut e);
    assert!(!r.options.contains(&Choice::CancelCombat), "{r:?}");
    assert!(e.state.minions[a.index()].locked, "the action spent");
}

/// The `nth` prince with no printed-text definition.
fn textless_prince(nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            matches!(c.title, Title::Prince(_)) && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a textless prince")
}

/// Bribes: "Only usable during a referendum before votes and ballots are
/// cast. Gain 1 pool. Any other Methuselah who casts one or more votes or
/// ballots in favor of and does not cast votes or ballots against the
/// referendum gains 1 pool when the results of the referendum are
/// tallied." Seat 1's prince votes for: +1; seat 2's against: nothing.
#[test]
fn bribes_pays_the_yes_voters_at_the_tally() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), textless_prince(0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let yes = e.add_minion(SeatId(1), textless_prince(1), 5, false);
    let no = e.add_minion(SeatId(2), textless_prince(2), 5, false);
    hand(&mut e, 0, &["Ancilla Empowerment", "Bribes"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(pool(&e)[0], 11, "Bribes: 1 pool");
    pick_when(&mut e, votes(a, true));
    pick_when(&mut e, votes(yes, true));
    pick_when(&mut e, votes(no, false));
    close_polling(&mut e);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let p = pool(&e);
    // Ancilla Empowerment: 1 pool a minion.
    assert_eq!((p[1], p[2]), (10 + 1 - 1, 10 - 1), "the yes voter bribed");
}

/// Black Hand Ritual: "Ⓓ burn any Gehenna card in play. Any 'Do not replace
/// until' directive on that card is ended (and the controller draws to
/// replace it, if it hadn't been replaced yet)." Restricted Vitae still
/// waiting on a successful hunt: burned, and seat 1 draws now.
#[test]
fn black_hand_ritual_burns_a_gehenna_card_and_ends_its_hold() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.black_hand, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let rv = in_play(&mut e, 1, "Restricted Vitae", None);
    e.state.cards[rv.index()].hold = Some((SeatId(1), zvtes::state::Release::SuccessfulHunt));
    e.state.seats[1].held[zvtes::state::Release::SuccessfulHunt] = 1;
    hand(&mut e, 1, &["Dodge"]);
    e.state.seats[1].library.push(find_library("Form of Mist"));
    hand(&mut e, 0, &["Black Hand Ritual"]);
    pick_when(&mut e, play_at(0, 1, Some(a), Target::Card(rv)));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[rv.index()].in_play, "burned");
    let seat = &e.state.seats[1];
    assert_eq!(
        (
            seat.hand.len(),
            seat.held[zvtes::state::Release::SuccessfulHunt]
        ),
        (2, 0),
        "its replacement drawn"
    );
}

/// Black Hand Ritual: "Unlock a younger Black Hand vampire." An older one
/// is no target.
#[test]
fn black_hand_ritual_unlocks_a_younger_black_hand_vampire() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.black_hand && c.capacity >= 7, 0),
        4,
        false,
    );
    let young = e.add_minion(
        SeatId(0),
        textless(|c| c.black_hand && c.capacity <= 5, 0),
        4,
        false,
    );
    let old = e.add_minion(SeatId(1), elder(), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[young.index()].locked = true;
    e.state.minions[old.index()].locked = true;
    hand(&mut e, 0, &["Black Hand Ritual"]);
    let req = request(&mut e);
    assert!(
        !req.options
            .contains(&play_at(0, 0, Some(a), Target::Minion(old))),
        "older: no target"
    );
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(young)));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[young.index()].locked);
}

/// Aura of Invincibility: "If this referendum passes, put this card on the
/// acting vampire and put a counter on this card." The card itself, out of
/// the ash heap.
#[test]
fn aura_of_invincibility_goes_on_the_caller_when_it_passes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ancilla Empowerment", "Aura of Invincibility"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let aura = on(&e, "Aura of Invincibility", a);
    assert_eq!(aura.len(), 1, "on the caller");
    let c = &e.state.cards[aura[0].index()];
    assert_eq!((c.counters, c.effect_only), (1, false));
    assert!(!e.state.seats[0]
        .ash
        .iter()
        .any(|&x| x == find_library("Aura of Invincibility")));
}

/// Aura of Invincibility: "This vampire gets an additional vote for each
/// counter on this card in referendums he or she calls. Add a counter to
/// this card when a referendum called by this vampire passes." Two counters
/// and the card's vote beat a prince's two.
#[test]
fn aura_of_invincibility_votes_its_counters_and_grows() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), textless_prince(0), 4, false);
    let aura = in_play(&mut e, 0, "Aura of Invincibility", Some(a));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostVotesPerCounterWhenCalling,
        source: aura,
    });
    e.state.cards[aura.index()].counters = 2;
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, votes(a, true));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    pick_when(&mut e, votes(p, false));
    close_polling(&mut e);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10 - 2, 10 - 1], "passed");
    assert_eq!(e.state.cards[aura.index()].counters, 3);
}

/// Aura of Invincibility: "If a referendum called by this vampire fails,
/// burn this card and send this vampire to torpor."
#[test]
fn aura_of_invincibility_fails_into_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), textless_prince(0), 4, false);
    let aura = in_play(&mut e, 0, "Aura of Invincibility", Some(a));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostVotesPerCounterWhenCalling,
        source: aura,
    });
    e.state.cards[aura.index()].counters = 1;
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, votes(a, true));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    pick_when(&mut e, votes(p, false));
    close_polling(&mut e);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10], "tied: failed");
    assert!(!e.state.cards[aura.index()].in_play, "burned");
    assert!(e.state.minions[a.index()].in_torpor);
}

/// Bear-Baiting's table: an anarch bleeding, an older Camarilla blocker,
/// one vampire in each crypt (`mine`, `theirs` capacities).
fn bear_baiting_table(mine: u8, theirs: u8) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Anarch) && (6..=8).contains(&c.capacity),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), elder_camarilla(), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    for (s, cap) in [(0, mine), (1, theirs)] {
        // In the crypt only: its text never read.
        let v = CRYPT
            .iter()
            .position(|c| c.capacity == cap && !c.imbued)
            .expect("a vampire of that capacity") as u16;
        e.state.seats[s].crypt.put(v, zvtes::copies::CopyId::NONE);
    }
    hand(&mut e, 0, &["Bear-Baiting"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    (e, a, b)
}

/// Bear-Baiting: "Otherwise, cancel the combat and place this card on the
/// acting anarch. This anarch gets +2 bleed when bleeding the blocking
/// vampire's controller." Mine older: no combat, the bleed still blocked;
/// the card itself on the anarch.
#[test]
fn bear_baiting_cancels_the_combat_and_baits_the_blocker() {
    let (mut e, a, b) = bear_baiting_table(9, 3);
    pick_when(&mut e, play(0, 0, a));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, a), blood(&e, b)),
        (4 - 1, 4),
        "its cost; no combat"
    );
    assert_eq!(pool(&e)[1], 10, "blocked");
    let card = on(&e, "Bear-Baiting", a);
    assert_eq!(card.len(), 1);
    assert!(!e.state.cards[card[0].index()].effect_only);
    assert!(!e.state.seats[0]
        .ash
        .iter()
        .any(|&x| x == find_library("Bear-Baiting")));
    assert_eq!(e.state.bleed_against_mod(a, SeatId(1)), 2);
}

/// Bear-Baiting: "If yours is younger, this anarch takes 1 unpreventable
/// damage (and combat occurs as normal)."
/// [RTR 19970630] The damage is environmental: no strike's, unpreventable.
#[test]
fn bear_baiting_bites_back_when_your_crypt_is_younger() {
    let (mut e, a, b) = bear_baiting_table(3, 9);
    pick_when(&mut e, play(0, 0, a));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Its cost, the damage, the blocker's hand strike; the anarch's back.
    assert_eq!(
        (blood(&e, a), blood(&e, b)),
        (4 - 1 - 1 - 1, 4 - 1),
        "combat as normal"
    );
    assert!(on(&e, "Bear-Baiting", a).is_empty());
}

/// Bear-Baiting: "Cannot be played when the target crypt is empty" (RTR
/// 20000501).
/// [RTR 20000501] [LSJ 20070411-2] Not with an empty crypt.
#[test]
fn bear_baiting_needs_a_crypt_to_compare() {
    let (mut e, a, _) = bear_baiting_table(3, 9);
    e.state.seats[1].crypt.take_all();
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)), "{req:?}");
}

/// Becoming of Ennoia: "During each Methuselah's unlock phase, he or she
/// chooses a ready vampire he or she controls. The chosen vampire takes 1
/// unpreventable damage." Seat 1's event, seat 0's unlock phase: seat 0
/// chooses among its vampires, its ally aside.
/// [PIB 20121031] Alone in play: its requirement is read as it is played only.
#[test]
fn becoming_of_ennoia_bites_each_methuselah_at_unlock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Becoming of Ennoia", None);
    let req = to_window(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::ChooseMinion(a), Choice::ChooseMinion(b)],
        "mandatory, its vampires"
    );
    pick(&mut e, Choice::ChooseMinion(b));
    request(&mut e);
    assert_eq!((blood(&e, a), blood(&e, b)), (4, 3));
}

/// Becoming of Ennoia: "Earth Meld cards cost 2 additional blood."
#[test]
fn becoming_of_ennoia_makes_earth_meld_cost_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pro", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let ennoia = in_play(&mut e, 1, "Becoming of Ennoia", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::NamedCardsCost("Earth Meld", 2),
        source: ennoia,
    });
    hand(&mut e, 0, &["Earth Meld"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 5 - 2, "2 blood, combat ended");
}

/// Becoming of Ennoia: "Requires at least two other Gehenna cards
/// controlled by other Methuselahs in play." Two of your own are not
/// enough.
#[test]
fn becoming_of_ennoia_needs_other_methuselahs_gehenna() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Wormwood", None);
    in_play(&mut e, 0, "Torpid Blood", None);
    hand(&mut e, 0, &["Becoming of Ennoia"]);
    let req = to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(!req.options.contains(&master_play(0)), "own Gehenna cards");
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Wormwood", None);
    in_play(&mut e, 1, "Torpid Blood", None);
    hand(&mut e, 0, &["Becoming of Ennoia"]);
    let req = to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(req.options.contains(&master_play(0)), "{req:?}");
}

/// Curmudgeon: "During your prey's unlock phase, you may choose a minion
/// controlled by your prey. If that minion is blocked this turn, you may
/// lock this card to move 1 blood from the blood bank to this vampire."
#[test]
fn curmudgeon_profits_from_the_chosen_minion_blocked() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    let w = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let m = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let cur = in_play(&mut e, 0, "Curmudgeon", Some(v));
    let choose = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(cur),
        ability: 0,
        actor: None,
        target: Target::Minion(m),
    };
    to_window(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    pick_when(&mut e, choose);
    pick_when(&mut e, bleed(m));
    pick_when(&mut e, block(w));
    let req = request(&mut e);
    assert!(
        req.options.contains(&Choice::ResolveRider { card: cur.0 }),
        "{req:?}"
    );
    pick(&mut e, Choice::ResolveRider { card: cur.0 });
    request(&mut e);
    assert_eq!(blood(&e, v), 4);
    assert!(e.state.cards[cur.index()].locked);
}

/// Traditionalist: "Once per turn, when this vampire casts his or her votes
/// or ballots against a referendum and the referendum fails, he or she
/// gains 1 blood from the blood bank."
#[test]
fn traditionalist_gains_for_a_referendum_voted_down() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), textless_prince(0), 3, false);
    in_play(&mut e, 1, "Traditionalist", Some(p));
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    pick_when(&mut e, votes(p, false));
    close_polling(&mut e);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10], "failed");
    assert_eq!(blood(&e, p), 4);
}

/// Gift of Experience: "Choose an unlocked vampire you control. Lock that
/// vampire. Search your crypt for an advancement card of that vampire ...
/// and put that card under or over the chosen vampire." Theo Bell finds
/// his advanced version in the crypt.
#[test]
fn gift_of_experience_advances_from_the_crypt() {
    let mut e = game(2);
    let theo = e.add_minion(SeatId(0), find_vampire("Theo Bell", false), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let adv = find_vampire("Theo Bell", true);
    e.state.seats[0].crypt.put(adv, zvtes::copies::CopyId::NONE);
    hand(&mut e, 0, &["Gift of Experience"]);
    pick_when(&mut e, play_at(0, 0, None, Target::Minion(theo)));
    request(&mut e);
    let v = &e.state.minions[theo.index()];
    assert!(v.locked && v.merged.is_some(), "locked, merged");
    assert!(e.state.seats[0].crypt.is_empty());
}

/// Gift of Experience: "(or the base vampire if an advancement is chosen)",
/// from the crypt only: the base in the ash heap is not offered.
#[test]
fn gift_of_experience_finds_the_base_in_the_crypt_only() {
    let mut e = game(2);
    let theo = e.add_minion(SeatId(0), find_vampire("Theo Bell", true), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let base = find_vampire("Theo Bell", false);
    e.state.seats[0]
        .ash_crypt
        .put(base, zvtes::copies::CopyId::NONE);
    hand(&mut e, 0, &["Gift of Experience"]);
    pick_when(&mut e, play_at(0, 0, None, Target::Minion(theo)));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::AshVampire(base)), "{req:?}");
    assert!(e.state.minions[theo.index()].locked, "locked all the same");
}

/// Siphon's table: a Giovanni of capacity 6+ acting, seat 1's younger and
/// older vampires in torpor, a Dodge in seat 0's ash heap.
fn siphon_table() -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let g = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Giovanni" && (6..=9).contains(&c.capacity), 0),
        3,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 2, true);
    let old = e.add_minion(SeatId(1), elder(), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0]
        .ash
        .put(find_library("Dodge"), zvtes::copies::CopyId::NONE);
    hand(&mut e, 0, &["Siphon"]);
    pick_when(&mut e, play(0, 0, g));
    (e, g, young, old)
}

/// Siphon: "Choose one or more vampires in torpor, who must be younger if
/// there are no Gehenna events in play. Steal 1 blood from each chosen
/// vampire. If this acting vampire is Giovanni, you can also move one card
/// from your ash heap to your hand for each blood stolen." The older one
/// is no choice; the Dodge comes back, Siphon itself not in the ash heap.
/// [PIB 20110725] The vampires chosen as it resolves; this Siphon in limbo
/// then, not in the ash heap.
#[test]
fn siphon_steals_from_younger_torpor_and_the_giovanni_digs_the_ash() {
    let (mut e, g, young, old) = siphon_table();
    // The younger one, the only choice, chosen (elided); then the ash.
    let req = to_window(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::FindCard(find_library("Dodge"))],
        "Siphon itself is not in the ash heap"
    );
    pick(&mut e, Choice::FindCard(find_library("Dodge")));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, g), blood(&e, young), blood(&e, old)), (4, 1, 2));
    assert!(e.state.seats[0].hand.contains(&find_library("Dodge")));
}

/// Siphon: with a Gehenna event in play, an older vampire may be chosen
/// too, each once.
#[test]
fn siphon_reaches_older_vampires_under_gehenna() {
    let (mut e, g, young, old) = siphon_table();
    in_play(&mut e, 1, "Wormwood", None);
    pick_when(&mut e, Choice::ChooseMinion(old));
    pick_when(&mut e, Choice::FindCard(find_library("Dodge")));
    let req = request(&mut e);
    assert!(
        req.options.contains(&Choice::Pass) && !req.options.contains(&Choice::ChooseMinion(old)),
        "more optional, each once: {req:?}"
    );
    pick(&mut e, Choice::ChooseMinion(young));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, g), blood(&e, young), blood(&e, old)), (5, 1, 1));
}

/// Emergency Powers: "Put this card on the acting Seraph. During each
/// political action, this Seraph gets X additional votes, where X is the
/// number of Gehenna cards in play." Wormwood in play: one vote, cast.
#[test]
fn emergency_powers_votes_with_gehenna() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), textless(|c| c.seraph, 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Wormwood", None);
    hand(&mut e, 0, &["Emergency Powers", "Ancilla Empowerment"]);
    pick_when(&mut e, play(0, 0, s));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let ep = on(&e, "Emergency Powers", s);
    assert_eq!(ep.len(), 1, "on the Seraph");
    e.state.minions[s.index()].locked = false;
    pick_when(&mut e, play(0, 0, s));
    let req = to_window(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert!(req.options.contains(&votes(s, true)), "{req:?}");
}

/// Emergency Powers: "If there are no Gehenna cards in play, burn this
/// card."
#[test]
fn emergency_powers_burns_without_gehenna() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), textless(|c| c.seraph, 0), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ww = in_play(&mut e, 1, "Wormwood", None);
    let ep = in_play(&mut e, 0, "Emergency Powers", Some(s));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::GehennaBound,
        source: ep,
    });
    request(&mut e);
    assert!(e.state.cards[ep.index()].in_play, "Wormwood there");
    e.state.cards[ww.index()].in_play = false;
    request(&mut e);
    assert!(!e.state.cards[ep.index()].in_play, "burned");
}

/// Gambit Accepted: "Only usable if you have at least 1 victory point. Lose
/// 1 victory point and put this card in play. During your unlock phase,
/// move 1 pool from your prey's pool to your pool."
#[test]
fn gambit_accepted_trades_a_victory_point_for_pool() {
    let mut e = game(3);
    for s in 0..3 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    hand(&mut e, 0, &["Gambit Accepted"]);
    assert!(
        !plays(&request(&mut e)).contains(&master_play(0)),
        "no victory point"
    );
    e.state.seats[0].half_vps = 2;
    pick_when(&mut e, master_play(0));
    request(&mut e);
    assert_eq!(e.state.seats[0].half_vps, 0);
    let g = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Gambit Accepted"))
        .expect("in play");
    assert!(!e.state.cards[g].effect_only);
    to_window(&mut e, SeatId(2), WindowLabel::MinionPhase);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((pool(&e)[0], pool(&e)[1]), (11, 9), "1 pool from the prey");
}

/// Gambit Accepted: "If you are ousted, your predator gains an additional
/// victory point."
#[test]
fn gambit_accepted_pays_the_predator_on_an_oust() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 3), 4, false);
    for s in 1..3 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    let g = in_play(&mut e, 1, "Gambit Accepted", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::PredatorVpIfOusted(1),
        source: g,
    });
    e.state.seats[1].pool = 1;
    pick_when(&mut e, bleed(a));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert_eq!(
        e.state.seats[0].half_vps,
        2 + 2,
        "the oust's and the Gambit's"
    );
}

/// Gambit Accepted: "At the end of the game, if you have not been ousted,
/// you gain an additional victory point." Seat 0 ousts the last other:
/// its prey's, the last standing's, the Gambit's.
#[test]
fn gambit_accepted_scores_at_the_end_of_the_game() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 3), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let g = in_play(&mut e, 0, "Gambit Accepted", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::VpAtGameEndIfStanding(1),
        source: g,
    });
    e.state.seats[1].pool = 1;
    pick_when(&mut e, bleed(a));
    loop {
        match e.run() {
            zvtes::game::Step::Over(vps) => {
                assert_eq!(vps[0], 2 + 2 + 2);
                break;
            }
            zvtes::game::Step::Decision(_) => e.choose(0),
        }
    }
}

/// Ecstatic Agony [AUS]: "once per combat, this vampire may press to
/// continue combat if any damage has been successfully inflicted on him or
/// her in the current round", and "+X strength, where X is the amount of
/// damage successfully inflicted on this vampire in the previous round":
/// struck for 1, it presses and strikes for 2 next round.
#[test]
fn ecstatic_agony_feeds_on_the_last_rounds_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let ag = in_play(&mut e, 0, "Ecstatic Agony", Some(a));
    e.state.cards[ag.index()].mode = 1;
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostStrengthPerLastRoundDamage,
        source: ag,
    });
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let press = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(ag),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, press);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (4 - 1 - 1, 4 - 1 - 2));
}

/// Loose Cannon's table: an anarch of capacity 5-9 with `disc` bleeding with
/// the card's `mode`, blocked by seat 1's `blocker` (4 blood); seat 1 also
/// holds a younger and an older vampire, unlocked, 3 blood each.
fn loose_cannon_table(
    disc: &'static str,
    mode: u8,
    blocker_blood: u8,
) -> (Engine, MinionId, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| {
                (5..=9).contains(&c.capacity)
                    && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case(disc))
            },
            0,
        ),
        4,
        false,
    );
    // Become anarch (no textless anarch has [dem] or [vic]).
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), blocker_blood, false);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 3, false);
    let old = e.add_minion(SeatId(1), elder(), 3, false);
    hand(&mut e, 0, &["Loose Cannon"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, play(0, mode, a));
    pick_when(&mut e, block(b));
    (e, a, b, young, old)
}

/// Loose Cannon [dem]: "If this action is blocked, ... before block resolution:
/// Lock a younger non-blocking vampire controlled by the blocking minion's
/// controller." [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR 19991206]
/// [LSJ 20091125] Applied as the block is declared, before any other card or
/// effect.
#[test]
fn loose_cannon_locks_a_younger_vampire_of_the_blocker() {
    let (mut e, _, _, young, old) = loose_cannon_table("dem", 0, 4);
    request(&mut e);
    let locked = |m: MinionId| e.state.minions[m.index()].locked;
    assert!(locked(young) && !locked(old));
}

/// Loose Cannon [tha]: "Move 2 blood from a non-blocking vampire controlled
/// by the blocking minion's controller to this vampire."
#[test]
fn loose_cannon_draws_blood_from_the_blockers_side() {
    let (mut e, a, b, _, old) = loose_cannon_table("tha", 1, 4);
    let req = to_window(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(
        !req.options.contains(&Choice::ChooseMinion(b)),
        "non-blocking"
    );
    pick(&mut e, Choice::ChooseMinion(old));
    request(&mut e);
    // Its cost, the 2 moved, the blocker's hand strike.
    assert_eq!((blood(&e, a), blood(&e, old)), (4 - 1 + 2 - 1, 3 - 2));
}

/// Loose Cannon [vic]: "Damage from this vampire's hand strikes is
/// aggravated during the resulting combat." The blocker struck for 1
/// aggravated goes to torpor, its blood unspent.
#[test]
fn loose_cannon_aggravates_the_hand_strikes() {
    let (mut e, _, b, _, _) = loose_cannon_table("vic", 2, 4);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[b.index()];
    assert!(v.in_torpor && v.blood == 4, "aggravated");
}

/// Improvised Tactics' table: an anarch bleeding with the card's lasting
/// effect in `mode` on it, blocked.
fn improvised_table(mode: u8) -> (Engine, MinionId, CardId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let it = in_play(&mut e, 0, "Improvised Tactics", Some(a));
    let c = &mut e.state.cards[it.index()];
    c.mode = mode;
    c.effect_only = true;
    if mode == 0 {
        e.state.layers.push(zvtes::cards::Layer {
            scope: zvtes::cards::Scope::InPlay,
            modifier: zvtes::cards::Modifier::DrawEachPressStep,
            source: it,
        });
    }
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    (e, a, it)
}

/// Improvised Tactics [aus]: "During the press step each round, draw one
/// card. Discard down to your hand size afterward." A full hand: one
/// drawn, one discarded (a single round: no press).
#[test]
fn improvised_tactics_draws_at_each_press_step() {
    let (mut e, _, _) = improvised_table(0);
    hand(&mut e, 0, &["Villein"; 7]);
    e.state.seats[0].library.push(find_library("Villein"));
    // The discard among identical cards elides.
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let seat = &e.state.seats[0];
    assert!(seat.library.is_empty(), "one drawn");
    assert_eq!(seat.hand.len(), 7, "down to hand size");
    assert!(
        seat.ash.iter().any(|&c| c == find_library("Villein")),
        "one discarded"
    );
}

/// Improvised Tactics [pot]: "At long range, each round, this minion may
/// strike for 2R damage each strike." Not at close range; at long range
/// (Fake Out's maneuver), 2 damage the blocker's hand strike cannot answer.
#[test]
fn improvised_tactics_strikes_at_long_range() {
    let (mut e, a, it) = improvised_table(1);
    hand(&mut e, 0, &["Fake Out"]);
    fill_hand(&mut e, 0);
    let strike = |e: &mut Engine| {
        let req = request(e);
        req.options.iter().copied().find(|o| {
            matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), ability: 0, .. } if *c == it)
        })
    };
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    let fake = *req
        .options
        .iter()
        .find(|o| matches!(o, Choice::Play { card: 0, .. }))
        .expect("Fake Out");
    pick(&mut e, fake);
    to_window(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let s = strike(&mut e).expect("offered at long range");
    pick(&mut e, s);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let z = MinionId(2);
    assert_eq!((blood(&e, a), blood(&e, z)), (4, 2));
}

/// At close range, no such strike.
#[test]
fn improvised_tactics_strikes_only_at_long_range() {
    let (mut e, a, it) = improvised_table(1);
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == it),
    );
    assert_eq!(blood(&e, a), 3, "hand strikes only");
}

/// Improvised Tactics [pro]: "Once each round, this vampire may burn 1
/// blood to get one maneuver."
#[test]
fn improvised_tactics_buys_a_maneuver_with_blood() {
    let (mut e, a, it) = improvised_table(2);
    let buy = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(it),
        ability: 1,
        actor: None,
        target: Target::None,
    };
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&buy), "{req:?}");
    pick(&mut e, buy);
    request(&mut e);
    assert_eq!(blood(&e, a), 3);
}

/// Sword of Troile: "Each strike the bearer makes at close range must be
/// with this weapon if possible. Bearer cannot use maneuvers to go to long
/// range." No hand strike, no Dodge, no maneuver (Form of Mist) offered.
#[test]
fn sword_of_troile_holds_its_bearer_to_the_blade() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let sword = in_play(&mut e, 0, "Sword of Troile", Some(a));
    for modifier in [
        zvtes::cards::Modifier::CloseStrikesWithThis,
        zvtes::cards::Modifier::BearerNoManeuverToLong,
    ] {
        e.state.layers.push(zvtes::cards::Layer {
            scope: zvtes::cards::Scope::InPlay,
            modifier,
            source: sword,
        });
    }
    hand(&mut e, 0, &["Dodge", "Form of Mist"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    // The sword's strike alone (elided): strength+2.
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::HandStrike,
    );
    assert_eq!(blood(&e, z), 4 - 3);
}

/// Sword of Nuln: "Vampires must spend twice the normal amount of blood to
/// heal the damage from this weapon's strikes." Struck for 1 with 1 blood:
/// paid, still unmended, to torpor empty (LSJ 20090901).
/// [LSJ 20090901] [LSJ 20030522-2] Pays what it can, unmended.
#[test]
fn sword_of_nuln_doubles_the_mending() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Tremere", 0), 4, false);
    e.add_minion(SeatId(0), plain("Tremere", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let sword = in_play(&mut e, 0, "Sword of Nuln", Some(a));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::MendingCost(2),
        source: sword,
    });
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(sword),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert!(v.in_torpor && v.blood == 0, "to torpor empty");
}

/// Meat Hook's table: seat 0's bearer bleeding, blocked by `blocker`.
fn meat_hook_table(blocker: u16) -> (Engine, MinionId, MinionId, CardId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), blocker, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let hook = in_play(&mut e, 0, "Meat Hook", Some(a));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::ContinuePressOnHitOnce,
        source: hook,
    });
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    (e, a, z, hook)
}

/// Meat Hook: "Once each combat, if this weapon is used to strike at close
/// range and the opponent doesn't dodge the strike, the bearer gets an
/// optional press this round, only usable to continue combat."
#[test]
fn meat_hook_hooks_a_press_on_a_hit() {
    let (mut e, a, z, hook) = meat_hook_table(plain("Ventrue", 0));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(hook),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(req.options.len() > 1, "a press: {req:?}");
    assert!(blood(&e, z) < 4 && blood(&e, a) < 4, "both struck");
}

/// Meat Hook: "At close range, a vampire with Potence [pot] may destroy
/// this weapon as a strike with first strike that costs 1 blood."
#[test]
fn meat_hook_is_torn_away_by_potence() {
    let (mut e, _, z, hook) = meat_hook_table(vampire_with("pot", 0));
    let tear = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(hook),
        ability: 1,
        actor: Some(z),
        target: Target::Card(hook),
    };
    pick_when(&mut e, tear);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[hook.index()].in_play, "destroyed");
    // Its 1 blood, and the bearer's hand strike (the walk's first choice).
    assert_eq!(blood(&e, z), 4 - 1 - 1);
}

/// Flash Grenade at close range: "the opposing minion ... is locked and does
/// not unlock as normal on his or her next unlock phase. If the bearer is a
/// vampire and the grenade is used at close range, the bearer is also
/// locked ... Burn this weapon after use."
#[test]
fn flash_grenade_blinds_both_at_close_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let g = in_play(&mut e, 0, "Flash Grenade", Some(a));
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(g),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    for m in [a, z] {
        let v = &e.state.minions[m.index()];
        assert!(v.locked && v.skip_unlock, "{m:?}");
    }
    assert!(!e.state.cards[g.index()].in_play, "burned after use");
    assert_eq!(blood(&e, z), 4, "combat ended");
}

/// The ammo's table: seat 0's bearer of a .44 Magnum bleeding, blocked by a
/// 6-blood blocker, the gun's strike chosen, the ammo in hand.
fn ammo_table(ammo: &str) -> (Engine, MinionId, MinionId, CardId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), elder(), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(a));
    hand(&mut e, 0, &[ammo]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(gun),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    (e, a, z, gun)
}

/// Scattershot: "This gun inflicts +2 damage at close range ... each strike
/// for the remainder of this combat." 2R + 2.
/// [TOM 19960225] The added damage the strike's own; [RTR 19990105] played once
/// the strikes are declared, before they resolve.
#[test]
fn scattershot_widens_a_gun_at_close_range() {
    let (mut e, a, z, _) = ammo_table("Scattershot");
    pick_when(&mut e, play(0, 0, a));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 6 - 4);
}

/// Caseless Rounds: "once each round when the bearer strikes with this gun,
/// the bearer gets an optional additional strike (limited), only usable to
/// strike with this gun." The round it is played: two shots.
/// [RTR 19990105] Played once the strikes are declared, before they resolve.
#[test]
fn caseless_rounds_fire_twice() {
    let (mut e, a, z, gun) = ammo_table("Caseless Rounds");
    pick_when(&mut e, play(0, 0, a));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(gun),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 6 - 2 - 2);
}

/// Scattershot: "No more than one ammo card can be used on a gun each
/// combat." A second ammo card is not offered on the loaded gun.
#[test]
fn one_ammo_card_a_gun_each_combat() {
    let (mut e, a, _, _) = ammo_table("Scattershot");
    e.state.seats[0].hand.push(find_library("Caseless Rounds"));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Frenzy: "Choose a vampire in combat. In this round, that vampire cannot
/// use equipment ... This round has a press, only usable to continue
/// combat." Seat 2 plays it on the acting vampire: its Magnum barred the
/// first round, and a second round follows, the gun back.
#[test]
fn frenzy_bars_equipment_and_forces_a_second_round() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(a));
    hand(&mut e, 2, &["Frenzy"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(a),
        },
    );
    let gun_strike = |o: &Choice| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == gun);
    // Round 1: the hand strike alone (elided); round 2, the gun again.
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert_eq!(e.combat().map(|c| c.round), Some(2), "the round's press");
    assert!(
        req.options.iter().any(gun_strike),
        "that round only: {req:?}"
    );
    assert_eq!((blood(&e, a), blood(&e, z)), (5 - 1, 5 - 1), "hand strikes");
    pick(&mut e, Choice::HandStrike);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (5 - 2, 5 - 2), "two rounds");
}

/// Highway Haven: RV: "If the bearer is an anarch, any vampire attempting
/// an action directed at this anarch doesn't unlock as normal on his or her
/// next unlock phase (even if the action isn't successful)."
#[test]
fn highway_haven_rv_slows_its_anarchs_attackers() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let z = e.add_minion(SeatId(1), of_sect(Sect::Anarch, 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let rv = in_play(&mut e, 1, "Highway Haven: RV", Some(z));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::ActingAtAnarchBearerSkipsUnlock,
        source: rv,
    });
    hand(&mut e, 0, &["Bum's Rush"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(z)));
    request(&mut e);
    assert!(e.state.minions[a.index()].skip_unlock);
}

/// Highway Haven: RV: "This minion gets an optional press to end combat on
/// the first round of combat." The round's press to continue (Frenzy)
/// answered: one round only.
#[test]
fn highway_haven_rv_presses_out_of_the_first_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let rv = in_play(&mut e, 1, "Highway Haven: RV", Some(z));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::BearerFirstRoundEndPress,
        source: rv,
    });
    hand(&mut e, 0, &["Frenzy"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    // Frenzy on its opponent: a press to continue the round has; the RV's
    // press ends the combat.
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(a),
        },
    );
    pick_when(&mut e, Choice::UsePress);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (4 - 1, 4 - 1), "one round");
    let _ = rv;
}

/// Tapestry of Blood: "This vampire can look at the top three cards of any
/// Methuselah's library and burn up to one of those as a +1 stealth action
/// that costs 1 blood." The fourth card down is no choice.
#[test]
fn tapestry_of_blood_burns_from_a_librarys_top() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let t = in_play(&mut e, 0, "Tapestry of Blood", Some(a));
    hand(&mut e, 1, &["Villein"; 7]);
    for name in ["Bribes", "Dodge", "Form of Mist", "Villein"] {
        e.state.seats[1].library.push(find_library(name));
    }
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(t),
            ability: 0,
            actor: Some(a),
            target: Target::Seat(SeatId(1)),
        },
    );
    let req = to_window(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(
        !req.options
            .contains(&Choice::FindCard(find_library("Bribes"))),
        "three cards only: {req:?}"
    );
    pick(&mut e, Choice::FindCard(find_library("Form of Mist")));
    request(&mut e);
    assert!(e.state.seats[1]
        .ash
        .iter()
        .any(|&c| c == find_library("Form of Mist")));
    assert_eq!(e.state.seats[1].library.len(), 3);
    assert_eq!(blood(&e, a), 3, "its cost");
    assert!(
        e.state.minions[a.index()].locked,
        "unlocks at the end of the turn"
    );
}

/// Tapestry of Blood: "… burn up to one of those": none burned (Pass).
#[test]
fn tapestry_of_blood_may_burn_none() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let t = in_play(&mut e, 0, "Tapestry of Blood", Some(a));
    hand(&mut e, 1, &["Villein"; 7]);
    for name in ["Dodge", "Form of Mist"] {
        e.state.seats[1].library.push(find_library(name));
    }
    pick_when(&mut e, use_card(t, 0, Some(a), Target::Seat(SeatId(1))));
    let req = to_window(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&Choice::Pass), "{req:?}");
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert_eq!(e.state.seats[1].library.len(), 2, "none burned");
    assert!(e.state.seats[1].ash.is_empty());
}

/// Virolax Facility: "When a vampire you control with capacity above 7 is
/// burned, burn this card and search your crypt for a vampire with capacity
/// below 5. Place that vampire in your ready region with 1 blood. That
/// vampire cannot act this turn."
#[test]
fn virolax_facility_replaces_a_burned_elder() {
    let mut e = game(2);
    let big = e.add_minion(SeatId(0), elder(), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vf = in_play(&mut e, 0, "Virolax Facility", None);
    let small = textless(|c| c.capacity <= 3, 0);
    e.state.seats[0]
        .crypt
        .put(small, zvtes::copies::CopyId::NONE);
    e.burn(big);
    let req = request(&mut e);
    assert!(!e.state.minions[big.index()].in_play, "burned");
    assert!(!e.state.cards[vf.index()].in_play, "the Facility burned");
    assert_eq!(req.options, vec![Choice::Pass, Choice::AshVampire(small)]);
    pick(&mut e, Choice::AshVampire(small));
    request(&mut e);
    let new = e
        .state
        .minions
        .iter()
        .position(|v| v.in_play && v.crypt_card() == Some(small))
        .expect("the younger vampire in play");
    let v = &e.state.minions[new];
    assert_eq!(v.blood, 1);
    assert!(v.recruited, "cannot act this turn");
}

/// San Lorenzo de El Escorial: "The vampire with this location may steal up
/// to 2 blood from an uncontrolled minion in your predator's uncontrolled
/// region as a +1 stealth Ⓓ action." 3 seats: seat 2 is seat 0's predator.
#[test]
fn san_lorenzo_drains_the_predators_uncontrolled() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let sl = in_play(&mut e, 0, "San Lorenzo de El Escorial, Spain", Some(a));
    e.state.seats[2]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: plain("Toreador", 0),
            copy: zvtes::copies::Untracked(zvtes::copies::CopyId::NONE),
            counters: 3,
            banished: None,
        });
    let use_at = |s: u8| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(sl),
        ability: 0,
        actor: Some(a),
        target: Target::Seat(SeatId(s)),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&use_at(1)), "the prey is no target");
    pick_when(&mut e, use_at(2));
    pick_when(&mut e, Choice::SetX(2));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 3 + 2);
    assert_eq!(e.state.seats[2].uncontrolled[0].counters, 1);
}

/// San Lorenzo de El Escorial: the uncontrolled minion and the amount
/// (0 to 2) are named as the action is announced, before any block (D1,
/// rules-qa 31.2, 38.1); an empty one may be named, a legal no-op (D14);
/// with nothing in the predator's region the action is not offered (D14:
/// a card naming an object there is none of).
#[test]
fn san_lorenzo_names_its_minion_and_amount_as_announced() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let sl = in_play(&mut e, 0, "San Lorenzo de El Escorial, Spain", Some(a));
    let use_at = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(sl),
        ability: 0,
        actor: Some(a),
        target: Target::Seat(SeatId(2)),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&use_at), "an empty region: {req:?}");
    for counters in [0, 3] {
        e.state.seats[2]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card: plain("Toreador", counters as usize),
                copy: zvtes::copies::Untracked(zvtes::copies::CopyId::NONE),
                counters,
                banished: None,
            });
    }
    pick_when(&mut e, use_at);
    let req = request(&mut e);
    assert_eq!(
        req.options,
        vec![Choice::Slot(0), Choice::Slot(1)],
        "the empty one too"
    );
    pick(&mut e, Choice::Slot(1));
    let req = request(&mut e);
    assert_eq!(
        req.options,
        vec![Choice::SetX(0), Choice::SetX(1), Choice::SetX(2)]
    );
    pick(&mut e, Choice::SetX(1));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(2), WindowLabel::ActionBlock),
        "named before the block"
    );
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 3 + 1, "the amount named");
    assert_eq!(e.state.seats[2].uncontrolled[1].counters, 2);
}

// ---- Christopher Houghton, Hanna Redmonds, Jezebelle, Seren Sukardi ---------

/// Christopher Houghton: "Once each combat, Christopher may burn 1 blood
/// before range is determined to strike with first strike that round."
/// His hand strike sends the 0-blood blocker (Khurshid, +2 strength) to
/// torpor before its 3 damage lands: he only burns the blood.
#[test]
fn christopher_houghton_burns_a_blood_to_strike_first() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Christopher Houghton"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let k = e.add_minion(SeatId(1), find_card("Khurshid"), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    bleed_blocked(&mut e, c, k);
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    let u = vampire_use(&req, c, 0).expect("Christopher's first strike");
    pick(&mut e, u);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, c), 4 - 1, "the blood burned, no damage taken");
    assert!(e.state.minions[k.index()].in_torpor);
}

/// Christopher Houghton: "During your unlock, choose a ready minion. Until
/// your next unlock, he gets +2 intercept when attempting to block that
/// minion." Seat 1 chooses seat 0's `a`: +2 against it, none against `b`;
/// at the next unlock the mark is replaced by the new choice.
#[test]
fn christopher_houghton_intercepts_the_chosen_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let c = e.add_minion(SeatId(1), find_card("Christopher Houghton"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    pick_when(&mut e, Choice::ChooseMinion(a));
    pick_when(&mut e, bleed(a));
    request(&mut e);
    assert_eq!(e.action().expect("a's bleed").intercept_of(&e.state, c), 2);
    pick_when(&mut e, bleed(b));
    request(&mut e);
    assert_eq!(e.action().expect("b's bleed").intercept_of(&e.state, c), 0);
    pick_when(&mut e, Choice::ChooseMinion(b));
    request(&mut e);
    assert_eq!(
        e.state.unlock_marks,
        [(zvtes::state::UnlockMark::Blocks, b, c)],
        "the previous mark gone at the unlock"
    );
}

/// Hanna Redmonds: "An older vampire opposing Hanna gets one optional
/// maneuver or press on the first round of combat." Serhat Gunde (also 1
/// optional press each combat) is offered a maneuver in the first round,
/// presses with his own, and has none left in the second.
#[test]
fn hanna_redmonds_gives_an_older_opponent_a_first_round_maneuver() {
    let mut e = game(2);
    let s = e.add_minion(
        SeatId(0),
        find_card("Serhat Gunde, Combat Junkie"),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), find_card("Hanna Redmonds"), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    bleed_blocked(&mut e, s, h);
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver));
    pick_when(&mut e, Choice::UsePress);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::UseManeuver,
    );
}

/// Hanna Redmonds: "An older vampire opposing Hanna …": a vampire of her
/// capacity gets nothing.
#[test]
fn hanna_redmonds_gives_nothing_to_a_vampire_her_age() {
    let mut e = game(2);
    let y = e.add_minion(SeatId(0), textless(|c| c.capacity == 2, 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), find_card("Hanna Redmonds"), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    bleed_blocked(&mut e, y, h);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::UseManeuver,
    );
}

/// Seat 0's `caller` calls Praxis Seizure: York; Jezebelle's votes as the
/// polling opens.
fn jezebelle_votes(e: &mut Engine, caller: MinionId, j: MinionId, card: &str) -> u8 {
    hand(e, 0, &[card]);
    pick(e, play(0, 0, caller));
    to_referendum(e);
    e.referendum()
        .expect("the referendum")
        .votes_of(&e.state, j)
}

/// Jezebelle: "If you don't control a ready prince, Jezebelle gets an
/// additional vote in any referendum to declare a prince called by another
/// vampire." Praxis Seizure: York called by seat 0: 1 vote; none once seat
/// 1 controls a ready prince, none in another referendum.
#[test]
fn jezebelle_votes_to_declare_another_vampires_prince() {
    for (prince, card, want) in [
        (false, "Praxis Seizure: York", 1),
        (true, "Praxis Seizure: York", 0),
        (false, "Kine Resources Contested", 0),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let j = e.add_minion(SeatId(1), find_card("Jezebelle"), 4, false);
        if prince {
            e.add_minion(SeatId(1), textless_prince(0), 4, false);
        }
        assert_eq!(jezebelle_votes(&mut e, a, j, card), want, "{prince} {card}");
    }
}

/// Jezebelle: "… called by another vampire": her own Praxis Seizure gets
/// no vote more.
#[test]
fn jezebelle_gets_no_vote_calling_herself() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jezebelle"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert_eq!(jezebelle_votes(&mut e, j, j, "Praxis Seizure: York"), 0);
}

/// Seren Sukardi bleeding, blocked by a [PRO] vampire holding two Form of
/// the Ghost (maneuver, then press), a .44 Magnum on it if `gun`: the
/// blocker goes to long range, strikes with the gun if any, presses to
/// continue. The second round's request where Seren's strike is offered,
/// if it is before the combat ends.
fn seren_second_round(gun: bool) -> (Engine, MinionId, MinionId, bool) {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Seren Sukardi"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("PRO", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Form of the Ghost", "Form of the Ghost"]);
    let g = gun.then(|| in_play(&mut e, 1, ".44 Magnum", Some(b)));
    bleed_blocked(&mut e, s, b);
    pick_when(&mut e, play_at(0, 0, Some(b), Target::None));
    if let Some(g) = g {
        pick_when(
            &mut e,
            Choice::UseAbility {
                source: zvtes::cards::AbilitySource::Card(g),
                ability: 0,
                actor: None,
                target: Target::None,
            },
        );
    }
    pick_when(&mut e, play_at(0, 1, Some(b), Target::None));
    let mut offered = false;
    for _ in 0..50 {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        if let Some(u) = vampire_use(&req, s, 0) {
            offered = true;
            pick(&mut e, u);
            continue;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    (e, s, b, offered)
}

/// Seren Sukardi: "If in the previous round the range was long and no
/// damage was successfully inflicted on Seren, he can strike: send the
/// opposing vampire to torpor." The blocker torpid after the second round.
#[test]
fn seren_sukardi_strikes_to_torpor_after_a_quiet_long_round() {
    let (e, _, b, offered) = seren_second_round(false);
    assert!(offered, "Seren's strike offered in the second round");
    assert!(e.state.minions[b.index()].in_torpor);
}

/// Seren Sukardi: "… and no damage was successfully inflicted on Seren":
/// shot at long range, no torpor strike next round.
#[test]
fn seren_sukardi_cannot_strike_to_torpor_once_damaged() {
    let (e, s, b, offered) = seren_second_round(true);
    assert!(!offered);
    assert!(blood(&e, s) < 6, "the gun landed");
    assert!(!e.state.minions[b.index()].in_torpor);
}

/// Seren Sukardi: "If in the previous round the range was long …": not in
/// the first round, nor after a round at close range.
#[test]
fn seren_sukardi_needs_a_long_previous_round() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Seren Sukardi"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("PRO", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Form of the Ghost"]);
    bleed_blocked(&mut e, s, b);
    let mut offered = false;
    let mut pressed = false;
    for _ in 0..50 {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        offered |= vampire_use(&req, s, 0).is_some();
        let press = play_at(0, 1, Some(b), Target::None);
        if !pressed && req.options.contains(&press) {
            pressed = true;
            pick(&mut e, press);
            continue;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    assert!(pressed, "a second round");
    assert!(!offered);
}

/// Peace Treaty: "Successful referendum means all weapons are burned. A
/// Methuselah may keep any of his or her minions' weapons by repaying their
/// pool cost to equip." Seat 0 keeps its Magnum (2 pool); seat 1's burns.
#[test]
fn peace_treaty_burns_the_weapons_not_paid_for() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let mine = in_play(&mut e, 0, ".44 Magnum", Some(a));
    let theirs = in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Peace Treaty"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    let req = request(&mut e);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::ChooseCard(mine.0)],
        "its own only"
    );
    pick(&mut e, Choice::ChooseCard(mine.0));
    pick_when(&mut e, Choice::Pass);
    pick_when(&mut e, Choice::Pass);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[mine.index()].in_play, "kept");
    assert!(!e.state.cards[theirs.index()].in_play, "burned");
    assert_eq!(pool(&e), vec![10 - 2, 10], "the kept gun's 2");
}

/// Tegyrius (advanced): "While Tegyrius is ready, each Camarilla Assamite
/// can burn 1 blood to get +1 vote during the polling step of any
/// referendum." Another Methuselah's Banu Haqim, once in the referendum.
#[test]
fn tegyrius_lets_each_camarilla_banu_haqim_buy_a_vote() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_vampire("Tegyrius, Vizier", true), 5, false);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 5, false);
    let bh = e.add_minion(
        SeatId(1),
        textless(
            |c| c.clan == "Banu Haqim" && c.sect == Some(Sect::Camarilla) && c.capacity >= 4,
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, a));
    let buy = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(t),
        ability: 0,
        actor: Some(bh),
        target: Target::None,
    };
    pick_when(&mut e, buy);
    pick_when(&mut e, votes(bh, false));
    assert_eq!(blood(&e, bh), 3, "1 blood for the vote");
    let req = request(&mut e);
    assert!(
        !req.options.contains(&buy),
        "once in the referendum: {req:?}"
    );
}

/// Rant!: "During this referendum, each ready anarch may burn 1 blood to
/// gain 1 additional vote. If the referendum fails, this acting vampire
/// takes 2 unpreventable damage." Seat 1's anarch buys a vote against: the
/// card's vote tied, the referendum fails.
#[test]
fn rant_rallies_every_anarch_and_hurts_on_a_failure() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), of_sect(Sect::Anarch, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ancilla Empowerment", "Rant!"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    let rant = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Rant!"))
        .map(|i| CardId(i as u32))
        .expect("its effect in play");
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(rant),
            ability: 0,
            actor: Some(b),
            target: Target::None,
        },
    );
    pick_when(&mut e, votes(b, false));
    close_polling(&mut e);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10], "failed");
    assert_eq!((blood(&e, a), blood(&e, b)), (5 - 2, 4 - 1));
    assert!(!e.state.cards[rant.index()].in_play, "the grant ended");
}

/// Emissary: "Choose a ready unlocked anarch you control. Lock the chosen
/// anarch. That anarch gains 3 votes. Any Camarilla vampire older than that
/// anarch can lock to cancel that anarch's votes and ballots."
#[test]
fn emissary_lends_votes_an_elder_can_silence() {
    let mut e = game(2);
    let baron = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            matches!(c.title, Title::Baron(_)) && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a textless baron") as u16;
    let b = e.add_minion(SeatId(0), baron, 5, false);
    let an = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 3, false);
    let elder = e.add_minion(SeatId(1), elder_camarilla(), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ancilla Empowerment", "Emissary"]);
    pick(&mut e, play(0, 0, b));
    pick_when(&mut e, play_at(0, 0, Some(b), Target::Minion(an)));
    let req = request(&mut e);
    assert!(e.state.minions[an.index()].locked, "the anarch locks");
    assert!(req.options.contains(&votes(an, true)), "3 votes: {req:?}");
    pick(&mut e, votes(an, true));
    let em = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Emissary"))
        .map(|i| CardId(i as u32))
        .expect("its effect in play");
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(em),
            ability: 0,
            actor: Some(elder),
            target: Target::None,
        },
    );
    request(&mut e);
    assert!(e.state.minions[elder.index()].locked, "the elder locks");
    close_polling(&mut e);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10], "its votes canceled: failed");
}

/// A Camarilla vampire of capacity 9 or more with no printed-text
/// definition, titled or not (the textless untitled ones run out).
fn elder_camarilla() -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.sect == Some(Sect::Camarilla)
                && c.capacity >= 9
                && !c.imbued
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("an elder Camarilla vampire") as u16
}

/// Magazine: "Put this card on a gun on this minion and put an ammo card
/// from your hand on this card. When using this gun, you may use the effect
/// of the ammo card as if it were played from your hand." Scattershot on
/// it: +2 at close range when the Magazine is used; the Magazine burns with
/// its gun (LSJ 20021128).
/// [LSJ 20021128] Leaving play, the card on it burns.
#[test]
fn magazine_feeds_its_gun_and_goes_with_it() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), elder(), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(a));
    hand(&mut e, 0, &["Magazine", "Scattershot", "Villein"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Card(gun),
        },
    );
    request(&mut e);
    let mag = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Magazine"))
        .map(|i| CardId(i as u32))
        .expect("the Magazine in play");
    assert_eq!(
        e.state.cards[mag.index()].stored.to_cards(),
        vec![find_library("Scattershot")],
        "the ammo on it"
    );
    assert!(!e.state.seats[0].hand.contains(&find_library("Scattershot")));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(gun),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(mag),
            ability: 0,
            actor: Some(a),
            target: Target::None,
        },
    );
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 6 - 4, "2R + 2");
    e.burn_card(gun);
    request(&mut e);
    assert!(!e.state.cards[mag.index()].in_play, "burned with its gun");
}

/// Mob Rule: "Each vampire with a capacity above 4 can burn blood to gain
/// votes ... A vampire with a capacity above 7 gains an additional vote for
/// each blood he or she burns." Around the table, one blood at a time, until
/// all pass (LSJ 20030602): seat 0's elder burns twice (4 votes), seat 1's
/// once (1 vote); a capacity-4 vampire is no choice.
/// [LSJ 20030602] Decided during the resolution, one blood at a time.
#[test]
fn mob_rule_sells_votes_for_blood_around_the_table() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), elder_camarilla(), 6, false);
    let small = e.add_minion(SeatId(0), textless(|c| c.capacity == 4, 0), 4, false);
    let b = e.add_minion(
        SeatId(1),
        textless(|c| (5..=7).contains(&c.capacity), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ancilla Empowerment", "Mob Rule"]);
    let before = zvtes::state::title_votes(e.state.title_of(a));
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    let req = to_window(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(
        !req.options.contains(&Choice::ChooseMinion(small)),
        "capacity 4: {req:?}"
    );
    pick(&mut e, Choice::ChooseMinion(a));
    pick_when(&mut e, Choice::ChooseMinion(b));
    pick_when(&mut e, Choice::ChooseMinion(a));
    pick_when(&mut e, Choice::Pass);
    pick_when(&mut e, Choice::Pass);
    request(&mut e);
    assert_eq!((blood(&e, a), blood(&e, b)), (4, 3));
    let r = e.referendum().expect("the referendum");
    let got = |m: MinionId| r.votes_of(&e.state, m);
    assert_eq!((got(a), got(b)), (before + 4, 1), "2 a blood above 7; 1");
}

/// Cold Amber's Hold: "Give this card to another Methuselah and move an
/// action card from that Methuselah's ash heap to this card. The chosen
/// action costs that Methuselah's vampires an additional blood. Any vampire
/// can move up to 4 blood to this card as a +1 stealth action. Burn this
/// card when it has 4 blood counters." The card on it burned with it (LSJ
/// 20021128).
/// [LSJ 20021128] Leaving play, the card on it burns.
#[test]
fn cold_ambers_hold_taxes_an_action_until_bled_off() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a2 = e.add_minion(SeatId(0), plain("Brujah", 1), 5, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[1]
        .ash
        .put(find_library("Bum's Rush"), zvtes::copies::CopyId::NONE);
    hand(&mut e, 0, &["Cold Amber's Hold"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Seat(SeatId(1))));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let hold = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Cold Amber's Hold"))
        .map(|i| CardId(i as u32))
        .expect("in play");
    assert_eq!(e.state.card_controller(hold), SeatId(1), "given");
    assert_eq!(
        e.state.cards[hold.index()].stored.to_cards(),
        vec![find_library("Bum's Rush")],
        "the action from that ash heap"
    );
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(hold),
            ability: 0,
            actor: Some(a2),
            target: Target::None,
        },
    );
    pick_when(&mut e, Choice::SetX(4));
    request(&mut e);
    assert_eq!(blood(&e, a2), 1, "4 blood moved");
    assert!(!e.state.cards[hold.index()].in_play, "4 counters: burned");
    assert!(
        e.state.seats[1]
            .ash
            .iter()
            .any(|&c| c == find_library("Bum's Rush")),
        "the card on it burned with it"
    );
    let _ = z;
}

/// A Hold of seat 0's (fixture) with `stored` on it, its surcharge layer.
fn hold_fixture(e: &mut Engine, name: &str, stored: &str) -> CardId {
    let h = in_play(e, 0, name, None);
    e.state.cards[h.index()]
        .stored
        .put(find_library(stored), zvtes::copies::CopyId::NONE);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::StoredCardsCostBlood(1),
        source: h,
    });
    h
}

/// The Holds: "The chosen action costs that Methuselah's vampires an
/// additional blood." Bum's Rush, free, costs seat 0's vampire 1 blood.
#[test]
fn a_hold_taxes_the_chosen_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hold_fixture(&mut e, "Cold Amber's Hold", "Bum's Rush");
    hand(&mut e, 0, &["Bum's Rush"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(z)));
    pick_when(&mut e, Choice::Pass);
    request(&mut e);
    assert_eq!(blood(&e, a), 3, "1 more blood");
}

/// Chiram's Hold: "That Methuselah may burn this card by discarding a copy
/// of the chosen combat card during his or her discard phase."
#[test]
fn a_hold_burns_for_a_discarded_copy() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let h = hold_fixture(&mut e, "Chiram's Hold", "Dodge");
    hand(&mut e, 0, &["Dodge", "Villein"]);
    let free = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(h),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, free);
    request(&mut e);
    assert!(!e.state.cards[h.index()].in_play, "burned");
    assert!(
        !e.state.seats[0].hand.contains(&find_library("Dodge")),
        "the copy discarded"
    );
}

/// Chiram's Hold: "Requires a ready vampire in combat with an acting minion
/// ... Give this card to the controller of the acting minion and move a
/// combat card from that Methuselah's ash heap to this card."
#[test]
fn chirams_hold_goes_to_the_acting_methuselah() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[0]
        .ash
        .put(find_library("Dodge"), zvtes::copies::CopyId::NONE);
    hand(&mut e, 1, &["Chiram's Hold"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    request(&mut e);
    let h = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Chiram's Hold"))
        .map(|i| CardId(i as u32))
        .expect("in play");
    assert_eq!(e.state.card_controller(h), SeatId(0));
    assert_eq!(
        e.state.cards[h.index()].stored.to_cards(),
        vec![find_library("Dodge")]
    );
}

/// Elder Michaelis's Hold: "Lock this reacting vampire. Give this card to
/// the controller of the acting minion and move an action modifier from
/// that Methuselah's ash heap to this card."
#[test]
fn elder_michaeliss_hold_locks_its_reacter() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[0]
        .ash
        .put(find_library("Alacrity"), zvtes::copies::CopyId::NONE);
    hand(&mut e, 1, &["Elder Michaelis's Hold"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, z));
    request(&mut e);
    assert!(
        e.state.minions[z.index()].locked,
        "the reacting vampire locks"
    );
    let h = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Elder Michaelis's Hold"))
        .map(|i| CardId(i as u32))
        .expect("in play");
    assert_eq!(e.state.card_controller(h), SeatId(0));
    assert_eq!(
        e.state.cards[h.index()].stored.to_cards(),
        vec![find_library("Alacrity")]
    );
}

/// Regarhagan's Hold: "That Methuselah may move 1 pool to this card as a
/// master phase action. Burn this card when it has 2 pool counters."
#[test]
fn regarhagans_hold_is_bought_off_with_pool() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let h = hold_fixture(&mut e, "Regarhagan's Hold", "Wake with Evening's Freshness");
    e.state.cards[h.index()].counters = 1;
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(h),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    request(&mut e);
    assert_eq!(pool(&e)[0], 9);
    assert!(!e.state.cards[h.index()].in_play, "2 counters: burned");
}

/// Liberty Club Intrigue: "Choose up to two Anarch vampires with
/// capacities of 5 or less. Each chosen vampire is locked and does not
/// unlock as normal on his or her next unlock phase." Two of seat 1's,
/// picked as played.
#[test]
fn liberty_club_intrigue_shackles_two_young_anarchs() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), textless_prince(0), 5, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let x = e.add_minion(
        SeatId(1),
        textless(|c| c.sect == Some(Sect::Anarch) && c.capacity <= 5, 0),
        3,
        false,
    );
    let y = e.add_minion(
        SeatId(1),
        textless(|c| c.sect == Some(Sect::Anarch) && c.capacity <= 5, 1),
        3,
        false,
    );
    hand(&mut e, 0, &["Liberty Club Intrigue"]);
    pick_when(&mut e, play_at(0, 0, Some(p), Target::Minion(x)));
    pick_when(&mut e, Choice::ChooseMinion(y));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    for m in [x, y] {
        let v = &e.state.minions[m.index()];
        assert!(v.locked && v.skip_unlock, "{m:?}");
    }
}

/// Elysium: Sforzesco Castle: "When a vampire you control blocks a
/// Camarilla vampire, you may lock this card instead of locking the
/// blocking vampire."
#[test]
fn sforzesco_castle_locks_for_the_blocker() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let castle = in_play(&mut e, 1, "Elysium: Sforzesco Castle", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::LocksInsteadOfBlockerOfCamarilla,
        source: castle,
    });
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, Choice::ResolveRider { card: castle.0 });
    request(&mut e);
    assert!(e.state.cards[castle.index()].locked, "the castle locks");
    assert!(!e.state.minions[z.index()].locked, "the blocker does not");
}

// ---- Afifa, Alicia Barrows, Hazimel, Jacob, Marthe Dizier, Shahid -----------

/// Seat `seat`'s hand card `name` played in `mode` by `actor` (no target)
/// once offered, the others passed on the way.
fn play_named(e: &mut Engine, seat: u8, name: &str, mode: u8, actor: Option<MinionId>) {
    let card = find_library(name);
    for _ in 0..200 {
        let req = request(e);
        let i = e.state.seats[seat as usize]
            .hand
            .iter()
            .position(|&c| c == card);
        let want = i.map(|i| play_at(i as u8, mode, actor, Target::None));
        if req.seat == SeatId(seat) {
            if let Some(k) = want.and_then(|w| req.options.iter().position(|o| *o == w)) {
                e.choose(k);
                return;
            }
        }
        let k = fallback(&req);
        e.choose(k);
    }
    panic!("{name} never offered");
}

/// Uses vampire `m`'s ability `ai` once offered; false if `seat`'s
/// `window` opens first.
fn use_before(e: &mut Engine, m: MinionId, ai: u8, seat: SeatId, window: WindowLabel) -> bool {
    for _ in 0..300 {
        let req = request(e);
        if let Some(u) = vampire_use(&req, m, ai) {
            pick(e, u);
            return true;
        }
        if (req.seat, req.window) == (seat, window) {
            return false;
        }
        let k = fallback(&req);
        e.choose(k);
    }
    panic!("neither offered nor reached");
}

/// Afifa with a second vampire for seat 0 in its discard phase, a Gehenna
/// event in hand; seat 1 its prey.
fn afifa_discard_phase() -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let f = e.add_minion(SeatId(0), find_card("Afifa, The Herald"), 1, false);
    let v = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Blood Weakens"]);
    to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    (e, f, v)
}

/// Afifa, The Herald: "When you put a Gehenna card in play, your prey burns
/// 1 pool." Blood Weakens put in play in the discard phase: the prey burns
/// 1, and no toll follows.
#[test]
fn afifa_makes_the_prey_burn_for_a_gehenna_card() {
    let (mut e, _, v) = afifa_discard_phase();
    play_named(&mut e, 0, "Blood Weakens", 0, None);
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        *o == Choice::ChooseMinion(v)
    });
    assert_eq!(pool(&e), vec![10, 9]);
    assert_eq!(blood(&e, v), 4);
}

/// Afifa, The Herald: "If you do not put a Gehenna card in play during
/// your discard phase, a ready vampire you control other than Afifa burns
/// 1 blood, or you burn 1 pool." Her controller's choice, Afifa not one;
/// either way.
#[test]
fn afifa_tolls_a_discard_phase_without_gehenna() {
    for vampire in [true, false] {
        let (mut e, f, v) = afifa_discard_phase();
        pick(&mut e, Choice::Pass);
        let req = request(&mut e);
        assert_eq!(req.seat, SeatId(0));
        assert_eq!(req.options, vec![Choice::BurnPool, Choice::ChooseMinion(v)]);
        assert!(!req.options.contains(&Choice::ChooseMinion(f)));
        let c = if vampire {
            Choice::ChooseMinion(v)
        } else {
            Choice::BurnPool
        };
        pick(&mut e, c);
        request(&mut e);
        let want = if vampire { (10, 3) } else { (9, 4) };
        assert_eq!((e.state.seats[0].pool, blood(&e, v)), want);
    }
}

/// Alicia Barrows for seat 0, with the Edge if `edge`; seat 1's hand three
/// Gehenna events and four other cards, two cards in its library.
fn alicia(edge: bool) -> (Engine, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Alicia Barrows"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[0].has_edge = edge;
    hand(
        &mut e,
        1,
        &[
            "Blood Weakens",
            "Thirst",
            "Wormwood",
            "Villein",
            "Villein",
            "Villein",
            "Villein",
        ],
    );
    for _ in 0..2 {
        e.state.seats[1].library.push(find_library("Form of Mist"));
    }
    (e, a)
}

/// Alicia Barrows: "During your unlock phase, if you have the Edge, you
/// can look at another Methuselah's hand and discard up to 2 Gehenna event
/// cards from it." Seat 1's hand looked at, only its Gehenna events
/// offered, two discarded and replaced.
#[test]
fn alicia_barrows_discards_gehenna_events_from_a_hand() {
    let (mut e, a) = alicia(true);
    let u = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(a),
        ability: 0,
        actor: None,
        target: Target::Seat(SeatId(1)),
    };
    let req = offering(&mut e, u);
    assert_eq!(e.state.active, SeatId(0));
    pick(&mut e, req.options[index_of(&req, u)]);
    assert!(e.state.peeks.contains(&(SeatId(0), SeatId(1))));
    let gehenna = ["Blood Weakens", "Thirst", "Wormwood"].map(find_library);
    let mut offered: Vec<Choice> = gehenna.into_iter().map(Choice::FindCard).collect();
    offered.sort_by_key(|c| match c {
        Choice::FindCard(x) => *x,
        _ => 0,
    });
    offered.insert(0, Choice::Pass);
    assert_eq!(request(&mut e).options, offered);
    pick(&mut e, Choice::FindCard(gehenna[0]));
    pick(&mut e, Choice::FindCard(gehenna[1]));
    request(&mut e);
    let z = &e.state.seats[1];
    assert!(z.ash.contains(&gehenna[0]) && z.ash.contains(&gehenna[1]));
    assert!(z.hand.contains(&gehenna[2]), "up to 2");
    assert_eq!(z.hand.len(), 7, "replaced");
    assert!(z.library.is_empty());
}

/// Alicia Barrows: "… if you have the Edge …": not offered without it.
#[test]
fn alicia_barrows_needs_the_edge() {
    let (mut e, a) = alicia(false);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(v), .. } if *v == a),
    );
}

/// Seat 0's `actor` attempting to diablerize seat 1's torpid `t`.
fn diablerie_at(actor: MinionId, t: MinionId) -> Choice {
    Choice::TakeAction {
        minion: actor,
        action: zvtes::decision::ActionKind::Diablerie,
        target: Some(t),
    }
}

/// Seat 0's vampire of `clan` with `b` blood, seat 1's Hazimel and a plain
/// vampire in torpor.
fn hazimel(clan: &str, b: u8) -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain(clan, 0), b, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), find_card("Hazimel"), 4, true);
    let o = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    (e, a, h, o)
}

/// Hazimel: "Non-Ravnos vampires must burn 2 blood to announce an action
/// … directed at Hazimel …": burned as the diablerie is announced, before
/// any block; with 1 blood, the action at him is masked, not at another.
#[test]
fn hazimel_taxes_an_action_announced_at_him() {
    let (mut e, a, h, _) = hazimel("Brujah", 4);
    pick(&mut e, diablerie_at(a, h));
    request(&mut e);
    assert_eq!(blood(&e, a), 4 - 2);
    let (mut e, a, h, o) = hazimel("Brujah", 1);
    let req = request(&mut e);
    assert!(!req.options.contains(&diablerie_at(a, h)));
    assert!(req.options.contains(&diablerie_at(a, o)));
}

/// Hazimel: "Non-Ravnos vampires …": a Ravnos pays nothing.
#[test]
fn hazimel_does_not_tax_a_ravnos() {
    let (mut e, a, h, _) = hazimel("Ravnos", 1);
    pick(&mut e, diablerie_at(a, h));
    request(&mut e);
    assert_eq!(blood(&e, a), 1);
}

/// Hazimel: "… directed at Hazimel or a card or counter on him": Conceal
/// at the .44 Magnum on him, 2 blood burned as it is announced.
#[test]
fn hazimel_taxes_an_action_at_a_card_on_him() {
    let mut e = game(2);
    let obf = (0..)
        .map(|n| vampire_with("obf", n))
        .find(|&v| CRYPT[v as usize].clan != "Ravnos")
        .expect("a non-Ravnos [obf] vampire");
    let a = e.add_minion(SeatId(0), obf, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), find_card("Hazimel"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let c = in_play(&mut e, 1, ".44 Magnum", Some(h));
    hand(&mut e, 0, &["Conceal"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Card(c)));
    request(&mut e);
    assert_eq!(blood(&e, a), 4 - 2 - 1, "and Conceal's blood");
}

/// Hazimel: "+1 bleed."
#[test]
fn hazimel_bleeds_for_two() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hazimel"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, true);
    pick(&mut e, bleed(h));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Jacob, The Glitch: "Jacob's capacity is increased by 4 while he is
/// controlled." 6 in play, 6 blood kept; 2 as his card reads it.
#[test]
fn jacob_the_glitch_holds_six_blood_in_play() {
    let mut e = game(2);
    let card = find_card("Jacob, The Glitch");
    let j = e.add_minion(SeatId(0), card, 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    request(&mut e);
    assert_eq!(CRYPT[card as usize].capacity, 2);
    assert_eq!(e.state.capacity_of(j), Some(6));
    assert_eq!(blood(&e, j), 6);
}

/// Jacob, The Glitch: "Your predator can use a master phase action to
/// discard a master card to force you to discard your hand. Draw back up
/// to your hand size afterward." Seat 1 (seat 0's predator) in its master
/// phase: its Villein discarded, seat 0's hand discarded and drawn anew.
#[test]
fn jacob_the_glitchs_predator_discards_his_hand() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jacob, The Glitch"), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Form of Mist"; 7]);
    for _ in 0..7 {
        e.state.seats[0].library.push(find_library("Thirst"));
    }
    hand(&mut e, 1, &["Villein"]);
    assert!(use_before(
        &mut e,
        j,
        0,
        SeatId(0),
        WindowLabel::MasterPhase
    ));
    assert_eq!(e.state.active, SeatId(1));
    request(&mut e);
    let thirst = find_library("Thirst");
    let z = &e.state.seats[0];
    assert_eq!(z.hand.len(), 7);
    assert!(z.hand.iter().all(|&c| c == thirst), "drawn back up");
    assert_eq!(z.ash.len(), 7);
    assert!(e.state.seats[1].ash.contains(&find_library("Villein")));
    assert_eq!(e.state.seats[1].mpa, 0, "the master phase action spent");
}

/// Marthe Dizier bleeding with `b` blood, blocked by a plain vampire; seat
/// 0's hand `cards`, its library empty.
fn marthe_blocked(cards: &[&str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Marthe Dizier"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, cards);
    bleed_blocked(&mut e, m, b);
    (e, m, b)
}

/// Is Marthe's ability offered before seat 0's minion phase?
fn marthe_offered(e: &mut Engine, m: MinionId) -> bool {
    use_before(e, m, 0, SeatId(0), WindowLabel::MinionPhase)
}

/// Marthe Dizier: "When a combat involving Marthe ends, she can burn a
/// blood to move a card she played during that combat from your ash heap
/// to your hand." Her dodge back in hand; Thirst, in the ash heap before,
/// stays: played, in the normal manner [LSJ 20041119].
#[test]
fn marthe_dizier_takes_back_a_card_she_played() {
    let (mut e, m, _) = marthe_blocked(&["Form of Mist"]);
    e.state.seats[0].ash.push(find_library("Thirst"));
    play_named(&mut e, 0, "Form of Mist", 0, Some(m));
    assert!(marthe_offered(&mut e, m));
    request(&mut e);
    let z = &e.state.seats[0];
    assert_eq!(z.hand.to_vec(), vec![find_library("Form of Mist")]);
    assert_eq!(z.ash.to_vec(), vec![find_library("Thirst")]);
    assert_eq!(blood(&e, m), 6 - 1);
}

/// Marthe Dizier: "… a card she played during that combat …": none
/// played, not offered.
#[test]
fn marthe_dizier_needs_a_card_she_played() {
    let (mut e, m, _) = marthe_blocked(&[]);
    e.state.seats[0].ash.push(find_library("Form of Mist"));
    assert!(!marthe_offered(&mut e, m));
}

/// Marthe Dizier: once each combat [LSJ 20040617]: her dodge and her press
/// of the first round, her dodge of the second; one taken back.
#[test]
fn marthe_dizier_takes_back_one_card_each_combat() {
    let (mut e, m, _) = marthe_blocked(&["Form of Mist", "Psyche!", "Form of Mist"]);
    play_named(&mut e, 0, "Form of Mist", 0, Some(m));
    play_named(&mut e, 0, "Psyche!", 0, Some(m));
    play_named(&mut e, 0, "Form of Mist", 0, Some(m));
    assert!(marthe_offered(&mut e, m));
    assert!(!marthe_offered(&mut e, m));
    assert_eq!(e.state.seats[0].ash.len(), 2);
    assert_eq!(e.state.seats[0].hand.len(), 1);
}

/// Marthe Dizier: a card canceled as it is played is still played [LSJ
/// 19980212] [ANK 20190701]: her dodge canceled by Direct Intervention.
#[test]
fn marthe_dizier_takes_back_a_canceled_card() {
    let (mut e, m, _) = marthe_blocked(&["Form of Mist"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    play_named(&mut e, 0, "Form of Mist", 0, Some(m));
    play_named(&mut e, 1, "Direct Intervention", 0, None);
    request(&mut e);
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Direct Intervention")));
    assert!(marthe_offered(&mut e, m));
    request(&mut e);
    assert_eq!(
        e.state.seats[0].hand.to_vec(),
        vec![find_library("Form of Mist")]
    );
}

/// Marthe Dizier: between combats, a Psyche! she plays [PIB 20111208]:
/// taken back before the queued combat.
#[test]
fn marthe_dizier_takes_back_psyche_between_combats() {
    let (mut e, m, _) = marthe_blocked(&["Psyche!"]);
    play_named(&mut e, 0, "Psyche!", 1, Some(m));
    let psyche = find_library("Psyche!");
    for _ in 0..100 {
        let req = request(&mut e);
        if let Some(u) = vampire_use(&req, m, 0) {
            assert!(e.combat().is_none(), "between the combats");
            assert!(e.action().is_some_and(|a| a.psyche_next.is_some()));
            pick(&mut e, u);
            request(&mut e);
            assert_eq!(e.state.seats[0].hand.to_vec(), vec![psyche]);
            return;
        }
        let k = fallback(&req);
        e.choose(k);
    }
    panic!("never offered");
}

/// Marthe Dizier: "+1 bleed."
#[test]
fn marthe_dizier_bleeds_for_two() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Marthe Dizier"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, true);
    pick(&mut e, bleed(m));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

/// Shahid: "You may lock Shahid during any other Methuselah's unlock phase
/// to increase your hand size by X for the remainder of the turn, where X
/// is the number of ready minions that Methuselah controls." In seat 0's
/// unlock phase (two ready minions, one torpid): +2 until the turn ends;
/// never in seat 1's own; in torpor too [LSJ 20011022].
#[test]
fn shahid_locks_for_the_unlocking_methuselahs_minions() {
    for torpor in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 2), 4, true);
        let s = e.add_minion(SeatId(1), find_card("Shahid"), 4, torpor);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 1, &["Form of Mist"; 7]);
        for _ in 0..4 {
            e.state.seats[1].library.push(find_library("Thirst"));
        }
        let used = |o: &Choice| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(v), .. } if *v == s);
        never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, used);
        assert!(use_before(
            &mut e,
            s,
            0,
            SeatId(0),
            WindowLabel::MinionPhase
        ));
        assert_eq!(e.state.active, SeatId(0));
        request(&mut e);
        assert!(e.state.minions[s.index()].locked);
        assert_eq!(e.state.hand_size(SeatId(1)), 9);
        assert_eq!(e.state.seats[1].hand.len(), 9, "drawn up");
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.hand_size(SeatId(1)), 7, "the turn over");
    }
}

/// Ghouled: "If this action is successful, put this card on that ally and add 1
/// additional life to the ally from the blood bank. This ally is considered a
/// ghoul and not a mortal." [ANK 20200203-2] [LSJ 20060515] A ghoul is a
/// monster, not a mortal.
#[test]
fn ghouled_makes_the_recruited_mortal_a_ghoul() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Neighborhood Watch Commander", "Ghouled"]);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let ally = e
        .state
        .minions
        .iter()
        .position(|m| m.in_play && m.is_ally())
        .map(|i| MinionId(i as u32))
        .expect("recruited");
    let g = on(&e, "Ghouled", ally);
    assert_eq!(g.len(), 1, "on the ally");
    assert!(!e.state.cards[g[0].index()].effect_only);
    use zvtes::library::Label;
    assert!(e.state.has_subtype(ally, Label::Ghoul) && !e.state.has_subtype(ally, Label::Mortal));
    let life = e.state.minions[ally.index()].blood;
    let printed = zvtes::library::LIBRARY[find_library("Neighborhood Watch Commander") as usize]
        .body
        .map_or(0, |b| b.life);
    assert_eq!(life, printed + 1, "1 more life");
}

/// Ghouled: "Once each combat, this ally can play a strike card that
/// requires basic Potence [pot] as a vampire." Thrown Gate offered, once.
#[test]
fn ghouled_ally_strikes_with_potence_once_a_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let g = e.add_ally(SeatId(1), find_library("Neighborhood Watch Commander"));
    e.state.minions[g.index()].blood = 5;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = in_play(&mut e, 1, "Ghouled", Some(g));
    for modifier in [
        zvtes::cards::Modifier::BearerSubtypeInstead {
            is: zvtes::library::Label::Ghoul,
            not: zvtes::library::Label::Mortal,
        },
        zvtes::cards::Modifier::PlaysAsVampire(zvtes::cards::AsVampire {
            disciplines: &["pot"],
            capacity: 1,
            non_action: false,
            non_ally: false,
            sect: Sect::Independent,
            strike_only: true,
            once_per_combat: true,
            combat_only: false,
        }),
    ] {
        e.state.layers.push(zvtes::cards::Layer {
            scope: zvtes::cards::Scope::InPlay,
            modifier,
            source: card,
        });
    }
    hand(&mut e, 1, &["Thrown Gate", "Thrown Gate"]);
    hand(&mut e, 0, &["Frenzy"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(g));
    // Frenzy: a second round.
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(a),
        },
    );
    let req = to_window(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(req.options.contains(&play(0, 0, g)), "{req:?}");
    pick(&mut e, play(0, 0, g));
    // Round 2 (the round's press): the second Thrown Gate not offered.
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, g));
    assert!(
        !e.state.minions[g.index()].as_vampire_spent,
        "back after the combat"
    );
}

/// Alia: "She can move a vampire from your torpor region to your ready
/// region as a +1 stealth action."
#[test]
fn alia_lifts_a_vampire_out_of_torpor() {
    let mut e = game(2);
    let alia = e.add_ally(SeatId(0), find_library("Alia, God's Messenger"));
    let t = e.add_minion(SeatId(0), plain("Brujah", 0), 2, true);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let card = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.body == Some(alia))
        .map(|i| CardId(i as u32))
        .expect("her card");
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(card),
            ability: 0,
            actor: Some(alia),
            target: Target::Minion(t),
        },
    );
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_torpor);
}

/// Alia: "Actions directed at her cost +1 pool." Seat 0's Bum's Rush at
/// her: 1 pool.
#[test]
fn alia_taxes_actions_at_her() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let alia = e.add_ally(SeatId(1), find_library("Alia, God's Messenger"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bum's Rush"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(alia)));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[0], 9);
}

/// Alia: "Alia can play cards requiring basic Auspex [aus] and/or basic
/// Obeah [obe] as a vampire." Revelations [aus] offered to her.
#[test]
fn alia_plays_auspex_as_a_vampire() {
    let mut e = game(2);
    let alia = e.add_ally(SeatId(0), find_library("Alia, God's Messenger"));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Revelations"]);
    let offered = plays(&request(&mut e))
        .iter()
        .any(|c| matches!(c, Choice::Play { actor: Some(m), .. } if *m == alia));
    assert!(offered, "Revelations by Alia");
}

/// Alia: "Requires 2 or more Gehenna events in play." One is not enough.
#[test]
fn alia_needs_two_gehenna_events() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Wormwood", None);
    hand(&mut e, 0, &["Alia, God's Messenger"]);
    assert!(
        !plays(&request(&mut e)).contains(&play(0, 0, a)),
        "one Gehenna card"
    );
    in_play(&mut e, 1, "Torpid Blood", None);
    assert!(plays(&request(&mut e)).contains(&play(0, 0, a)), "two");
}

/// Gather: "Put this card in play, locked, and choose a younger Gangrel in
/// your uncontrolled region."
#[test]
fn gather_chooses_a_younger_gangrel() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Gangrel" && c.capacity >= 6, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let young = textless(|c| c.clan == "Gangrel" && c.capacity <= 3, 0);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: young,
            copy: zvtes::copies::Untracked(zvtes::copies::CopyId::NONE),
            counters: 1,
            banished: None,
        });
    hand(&mut e, 0, &["Gather"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Uncontrolled(0)));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let g = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Gather"))
        .expect("in play");
    let c = &e.state.cards[g];
    assert!(c.locked, "locked");
    assert_eq!(c.chosen_crypt, Some(young));
}

/// Gather: "During the influence phase, you may lock this card to move that
/// Gangrel from your uncontrolled region to your ready region, with any
/// counters he or she has." Seat 1's Gather, used as seat 0's influence
/// phase ends (any Methuselah's, ANK 20240118).
/// [ANK 20240118] Any Methuselah's influence phase; [LSJ 20030520-1] from the
/// uncontrolled region only.
#[test]
fn gather_brings_out_its_gangrel_in_any_influence_phase() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let young = textless(|c| c.clan == "Gangrel" && c.capacity <= 3, 0);
    e.state.seats[1]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: young,
            copy: zvtes::copies::Untracked(zvtes::copies::CopyId::NONE),
            counters: 2,
            banished: None,
        });
    let g = in_play(&mut e, 1, "Gather", None);
    e.state.cards[g.index()].chosen_crypt = Some(young);
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(g),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    request(&mut e);
    let m = e
        .state
        .minions
        .iter()
        .position(|v| v.in_play && v.crypt_card() == Some(young))
        .expect("out");
    assert_eq!(e.state.minions[m].owner, SeatId(1));
    assert_eq!(e.state.minions[m].blood, 2, "its counters");
}

/// Jack of Both Sides: "This vampire equips or employs an equipment or
/// retainer from your hand. The cost of that card is reduced by 1 pool or by
/// up to 2 blood." A .44 Magnum (2 pool) for 1.
/// [LSJ 20100130] [ANK 20180925-2] [LSJ 19991207] The equipment named from the
/// hand as the action is declared.
#[test]
fn jack_of_both_sides_equips_for_less() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Jack of Both Sides", ".44 Magnum"]);
    pick_when(&mut e, play(0, 0, a));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, ".44 Magnum", a).len(), 1, "equipped");
    assert_eq!(pool(&e)[0], 10 - 1);
}

/// Jack of Both Sides, employing: a Twisted Bloodhound (1 pool) for nothing.
#[test]
fn jack_of_both_sides_employs_for_less() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Jack of Both Sides", "Twisted Bloodhound"]);
    pick_when(&mut e, play(0, 1, a));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, "Twisted Bloodhound", a).len(), 1, "employed");
    let printed =
        match zvtes::library::LIBRARY[find_library("Twisted Bloodhound") as usize].pool_cost {
            zvtes::library::Cost::Fixed(n) => n as i32,
            _ => 0,
        };
    assert_eq!(pool(&e)[0], 10 - (printed - 1).max(0));
}

/// Repo Man: "Repo Man can retrieve the first vehicle you find in your
/// library as a +2 stealth action. You then place the vehicle on any ready
/// minion you control who meets the requirements of the card (pay cost to
/// equip as normal)." The Sport Bike under a Dodge, onto the vampire chosen.
#[test]
fn repo_man_repossesses_a_vehicle_for_a_vampire() {
    let mut e = game(2);
    let r = e.add_ally(SeatId(0), find_library("Repo Man"));
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let w = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Villein"; 7]);
    for name in ["Sport Bike", "Dodge"] {
        e.state.seats[0].library.push(find_library(name));
    }
    let card = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.body == Some(r))
        .map(|i| CardId(i as u32))
        .expect("his card");
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(card),
            ability: 0,
            actor: Some(r),
            target: Target::None,
        },
    );
    let req = to_window(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(
        req.options.contains(&Choice::ChooseMinion(v))
            && req.options.contains(&Choice::ChooseMinion(w)),
        "{req:?}"
    );
    pick(&mut e, Choice::ChooseMinion(w));
    request(&mut e);
    assert_eq!(on(&e, "Sport Bike", w).len(), 1, "on the chosen one");
}

/// Repo Man: "If the action to recruit Repo Man is successful, unlock the
/// acting minion at the end of the turn."
#[test]
fn repo_man_unlocks_his_recruiter_at_the_end_of_the_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Repo Man"]);
    pick_when(&mut e, play(0, 0, a));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].locked, "locked by the recruit");
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[a.index()].locked,
        "unlocked as the turn ended"
    );
}

// ---- Lord Ashton, Smallpox Griet, Sascha Vykos, Tusk ------------------------

/// A textless Gangrel without superior Fortitude.
fn gangrel(nth: usize) -> u16 {
    textless(
        |c| c.clan == "Gangrel" && c.capacity >= 4 && !c.disciplines.contains(&"FOR"),
        nth,
    )
}

/// Lord Ashton's ability used by `actor`, if offered in `req`.
fn ashton_use(req: &zvtes::decision::DecisionRequest, ashton: MinionId, actor: MinionId) -> bool {
    req.options.iter().any(|o| {
        matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(v), actor: Some(a), .. }
            if *v == ashton && *a == actor)
    })
}

/// Lord Ashton: "Any other Gangrel can search his or her controller's
/// library for an Animalism, Fortitude or Protean Master: Discipline card
/// and move it to himself or herself as a +1 stealth action." Another
/// Methuselah's Gangrel, Lord Ashton in torpor too: Fortitude from its
/// own library onto it; neither Celerity nor a Protean in the ash heap
/// offered; an action requiring a Gangrel [LSJ 20080604].
#[test]
fn lord_ashton_lets_another_gangrel_fetch_a_discipline_master() {
    for torpor in [false, true] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), gangrel(0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let l = e.add_minion(SeatId(1), find_card("Lord Ashton"), 3, torpor);
        let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[v.index()].locked = true;
        fill_hand(&mut e, 0);
        e.state.seats[0].ash.push(find_library("Protean"));
        for name in ["Celerity", "Fortitude", "Thirst"] {
            e.state.seats[0].library.push(find_library(name));
        }
        let before = e.state.discipline_level(g, "for");
        let req = request(&mut e);
        assert!(ashton_use(&req, l, g), "torpor {torpor}");
        let u = req
            .options
            .iter()
            .find(|o| matches!(o, Choice::UseAbility { actor: Some(a), .. } if *a == g))
            .cloned()
            .unwrap();
        pick(&mut e, u);
        let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
        let fortitude = find_library("Fortitude");
        let wanted = |c: u16| Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: c,
        };
        assert!(!req.options.contains(&wanted(find_library("Celerity"))));
        assert!(!req.options.iter().any(
            |o| matches!(o, Choice::TakeFrom { zone, .. } if *zone != zvtes::decision::Zone::Library)
        ));
        pick(&mut e, wanted(fortitude));
        request(&mut e);
        assert_eq!(on(&e, "Fortitude", g).len(), 1);
        assert!(!e.state.seats[0].library.contains(&fortitude));
        assert_eq!(e.state.discipline_level(g, "for"), before + 1);
    }
}

/// Lord Ashton: "Any other Gangrel …": not Lord Ashton himself, nor a
/// vampire of another clan.
#[test]
fn lord_ashton_is_for_other_gangrel_only() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lord Ashton"), 3, false);
    let g = e.add_minion(SeatId(0), gangrel(0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert!(ashton_use(&req, l, g));
    assert!(!ashton_use(&req, l, l));
    assert!(!ashton_use(&req, l, b));
}

/// Smallpox Griet bleeding, blocked by a plain vampire with `b` blood;
/// two presses to continue in hand.
fn smallpox_blocked(b: u8) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Smallpox Griet"), 9, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), b, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Dead-End Alley", "Dead-End Alley"]);
    bleed_blocked(&mut e, s, v);
    (e, s, v)
}

/// Smallpox Griet: "If Smallpox successfully inflicts hand damage in
/// consecutive rounds of a combat, the opponent gets a pox counter." No
/// counter after the first round; one after the second, another after the
/// third: every time [PIB 20141210].
#[test]
fn smallpox_griet_poxes_after_consecutive_hand_damage() {
    let (mut e, s, v) = smallpox_blocked(9);
    for pox in [0, 1] {
        let req = to_window(&mut e, SeatId(0), WindowLabel::CombatPress);
        assert_eq!(e.state.minions[v.index()].pox, pox);
        let press = req
            .options
            .iter()
            .find(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == s))
            .cloned()
            .expect("the press");
        pick(&mut e, press);
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[v.index()].pox, 2);
}

/// Smallpox Griet: "A minion with any pox counters takes 1 damage during
/// his or her unlock phase." Environmental, out of combat [RTR 19970630]:
/// seat 1's poxed vampire in seat 1's unlock
/// phase, not another; seat 0's in seat 0's.
#[test]
fn smallpox_griet_pox_damages_in_its_minions_unlock_phase() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Smallpox Griet"), 9, false);
    let own = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[v.index()].pox = 2;
    e.state.minions[own.index()].pox = 1;
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, v), blood(&e, w), blood(&e, own)), (3, 4, 4));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, v), blood(&e, own)), (3, 3));
}

/// Smallpox Griet: "Burn all pox counters when Smallpox leaves the ready
/// region": sent to torpor.
#[test]
fn smallpox_griet_leaving_the_ready_region_burns_the_pox() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Smallpox Griet"), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[v.index()].pox = 1;
    e.damage(s, 2);
    assert!(e.state.minions[s.index()].in_torpor);
    assert_eq!(e.state.minions[v.index()].pox, 0);
}

/// Sascha Vykos (advanced, or merged) for seat 1, `torpor` or ready; seat
/// 0 at its master phase with Presence in hand.
fn sascha_master_phase(merged: bool, torpor: bool) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let s = e.add_minion(
        SeatId(1),
        find_vampire("Sascha Vykos, The Angel of Caine", true),
        4,
        torpor,
    );
    if merged {
        e.state.minions[s.index()].merged = Some(find_card("Sascha Vykos, The Angel of Caine"));
    }
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Presence"]);
    fill_hand(&mut e, 1);
    for _ in 0..3 {
        e.state.seats[1].library.push(find_library("Thirst"));
    }
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    (e, a, s)
}

/// Sascha Vykos (advanced): "If Sascha is ready, you can draw 1 card after
/// another Methuselah plays a master card." Discarding afterward [ANK
/// 20190606-2]; the merged Sascha too, the base card's text with it.
#[test]
fn sascha_vykos_draws_after_another_methuselahs_master() {
    for merged in [false, true] {
        let (mut e, a, s) = sascha_master_phase(merged, false);
        master_on(&mut e, "Presence", a);
        let req = expect(&mut e, SeatId(1), WindowLabel::Draw);
        assert!(req.options.contains(&Choice::Pass), "a choice");
        pick(&mut e, Choice::DrawLibrary);
        expect(&mut e, SeatId(1), WindowLabel::DiscardDown);
        e.choose(0);
        request(&mut e);
        let z = &e.state.seats[1];
        assert_eq!((z.hand.len(), z.library.len(), z.ash.len()), (7, 2, 1));
        if merged {
            let def = zvtes::cards::vampire_of(&e.state.minions[s.index()]).unwrap();
            assert_eq!(def.riders, zvtes::cards::vampires::SASCHA_VYKOS.riders);
        }
    }
}

/// Sascha Vykos (advanced): "If Sascha is ready …": not in torpor.
#[test]
fn sascha_vykos_draws_only_while_ready() {
    let (mut e, a, _) = sascha_master_phase(false, true);
    master_on(&mut e, "Presence", a);
    let req = request(&mut e);
    assert_ne!((req.seat, req.window), (SeatId(1), WindowLabel::Draw));
    assert_eq!(e.state.seats[1].library.len(), 3);
}

/// Sascha Vykos (advanced): "… another Methuselah …": not after his own
/// controller's master card.
#[test]
fn sascha_vykos_ignores_his_own_methuselahs_master() {
    let mut e = game(2);
    let s = e.add_minion(
        SeatId(0),
        find_vampire("Sascha Vykos, The Angel of Caine", true),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 2), 4, false);
    hand(&mut e, 0, &["Presence"]);
    fill_hand(&mut e, 0);
    for _ in 0..3 {
        e.state.seats[0].library.push(find_library("Thirst"));
    }
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Presence", s);
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::Draw);
    assert_eq!(e.state.seats[0].library.len(), 2, "the replacement only");
}

/// Tusk, The Talebearer bleeding, `top` on his library, at his action's
/// announcement with his ability used.
fn tusk_burns(top: &str) -> (Engine, MinionId) {
    let mut e = game(2);
    let t = e.add_minion(
        SeatId(0),
        find_vampire("Tusk, The Talebearer", true),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(find_library("Thirst"));
    e.state.seats[0].library.push(find_library(top));
    pick(&mut e, bleed(t));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    assert!(req.options.contains(&Choice::Pass), "a choice");
    let u = vampire_use(&req, t, 0).expect("Tusk's burn");
    e.choose(index_of(&req, u));
    request(&mut e);
    assert!(e.state.seats[0].ash.contains(&find_library(top)));
    assert_eq!(e.state.seats[0].library.len(), 1);
    (e, t)
}

/// Tusk, The Talebearer: "During the declaration of Tusk's action, you may
/// burn the top card of your library. If the burned card requires
/// Obfuscate [obf], Tusk gets +1 stealth for the current action." Once.
#[test]
fn tusk_burns_an_obfuscate_card_for_stealth() {
    let (mut e, t) = tusk_burns("Cloak the Gathering");
    let a = e.action().expect("the bleed");
    assert_eq!(a.stealth(&e.state), 1);
    assert_eq!(blood(&e, t), 4);
    let req = request(&mut e);
    assert!(vampire_use(&req, t, 0).is_none(), "once");
}

/// Tusk, The Talebearer: "Otherwise, Tusk burns 1 blood."
#[test]
fn tusk_burns_a_blood_for_another_card() {
    let (e, t) = tusk_burns("Thirst");
    let a = e.action().expect("the bleed");
    assert_eq!(a.stealth(&e.state), 0);
    assert_eq!(blood(&e, t), 3);
}

// ---- The Diamond Thunderbolt ------------------------------------------------

/// Seat 0's [dom] vampire plays Graverobbing on seat 1's torpid `z`; seat 1
/// holds The Diamond Thunderbolt. Returns (engine, robber, z).
fn graverobbing_at_a_thunderbolt() -> (Engine, MinionId, MinionId) {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, true);
    hand(&mut e, 0, &["Graverobbing"]);
    hand(&mut e, 1, &["Diamond Thunderbolt, The"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Minion(z),
        },
    );
    (e, a, z)
}

/// The Diamond Thunderbolt: "Only usable when control of a vampire you
/// control is about to change … Control of that vampire does not change,
/// and that vampire gains 1 blood." [LSJ 20040518] "The bid is still
/// paid": the action that would steal is played and paid, and succeeds.
/// [LSJ 20040518] [LSJ 20040518-2] Costs and bids are still paid.
#[test]
fn diamond_thunderbolt_stops_a_steal() {
    let (mut e, a, z) = graverobbing_at_a_thunderbolt();
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    let thunder = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(z),
    };
    assert_eq!(req.options, vec![Choice::Pass, thunder]);
    assert_eq!(e.state.minions[z.index()].owner, SeatId(1), "not yet");
    pick(&mut e, thunder);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert_eq!((v.owner, v.blood, v.in_torpor), (SeatId(1), 2, true));
    assert!(e.state.minions[a.index()].locked, "the action paid");
    assert!(
        e.state.seats[1].out_of_turn.is_some(),
        "out of turn: the next master phase's action"
    );
    assert!(e.state.control_change.is_none());
}

/// Two control changes at once wait in turn (rules-qa 111.5): seat 0's
/// acting unnamed and seat 1's blocker each strike with Contagion [DAI],
/// each Methuselah's counters reaching the other vampire's capacity; both
/// steals taken, seat 1 is asked for its Thunderbolt while the unnamed,
/// queued behind, is still seat 0's (seat 0 may not, in its own turn).
#[test]
fn diamond_thunderbolt_answers_two_steals_in_turn() {
    let mut e = game(3);
    let unnamed = e.add_minion(SeatId(0), find_card("unnamed, The"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("cel", 0), 5, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 5, false);
    e.add_minion(SeatId(2), plain("Ventrue", 2), 5, false);
    for (m, by) in [(z, 0), (unnamed, 1)] {
        let cap = e.state.capacity_of(m).unwrap();
        e.state.minions[m.index()].add_corruption(SeatId(by), cap - 1);
    }
    let contagion = find_library("Contagion");
    let mut strikes = Vec::new();
    for (seat, host) in [(0, unnamed), (1, z)] {
        e.state
            .cards
            .push(CardInPlay::new(contagion, SeatId(seat), Some(host), 1));
        strikes.push(Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(CardId(e.state.cards.len() as u32 - 1)),
            ability: 0,
            actor: None,
            target: Target::None,
        });
    }
    hand(&mut e, 1, &["Diamond Thunderbolt, The"]);
    pick(&mut e, bleed(unnamed));
    pick(&mut e, block(z));
    for strike in strikes {
        pick_when(&mut e, strike);
    }
    pick_when(&mut e, Choice::ChooseMinion(z));
    pick_when(&mut e, Choice::ChooseMinion(unnamed));
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert!(req
        .options
        .contains(&play_at(0, 0, None, Target::Minion(z))));
    assert_eq!(e.state.minions[unnamed.index()].owner, SeatId(0), "waits");
    assert_eq!(e.state.control_queue.len(), 1);
    pick(&mut e, play_at(0, 0, None, Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let (u, v) = (
        &e.state.minions[unnamed.index()],
        &e.state.minions[z.index()],
    );
    assert_eq!(
        (v.owner, u.owner),
        (SeatId(1), SeatId(1)),
        "z kept, the unnamed taken"
    );
    assert!(e.state.control_change.is_none() && e.state.control_queue.is_empty());
}

/// Let pass, the steal goes through as before.
#[test]
fn diamond_thunderbolt_may_let_the_steal_go() {
    let (mut e, _, z) = graverobbing_at_a_thunderbolt();
    until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert_eq!((v.owner, v.blood), (SeatId(0), 1));
    assert_eq!(e.state.seats[1].hand.len(), 1, "still in hand");
}

/// "… or when a vampire you control is about to enter combat": as its block
/// succeeds, the blocker gains 1 blood (3 + 1, less the hand strike's 1)
/// and the combat goes on.
#[test]
fn diamond_thunderbolt_as_a_vampire_is_about_to_enter_combat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 3, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 3, false);
    hand(&mut e, 1, &["Diamond Thunderbolt, The"]);
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, block(z));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlocked);
    let thunder = Choice::Play {
        card: 0,
        mode: 1,
        actor: None,
        target: Target::Minion(z),
    };
    assert!(req.options.contains(&thunder), "{:?}", req.options);
    assert!(
        !req.options.iter().any(|o| matches!(o,
            Choice::Play { target: Target::Minion(t), .. } if *t == a)),
        "only a vampire you control"
    );
    e.notes = Some(Vec::new());
    pick(&mut e, thunder);
    request(&mut e);
    assert!(!noted_combats(&mut e).is_empty(), "the combat goes on");
    assert_eq!((blood(&e, a), blood(&e, z)), (3, 3), "each hand strike's 1");
}

// ---- Redline ----------------------------------------------------------------

/// Seat 0's Sabbat `d` diablerizes seat 1's torpid vampire; seat 1's
/// prince passes the blood hunt; seat 2 holds Redline. Returns (engine,
/// d), at the burn about to happen.
fn blood_hunt_at_a_redline() -> (Engine, MinionId) {
    let mut e = game(3);
    let d = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 0), 4, false);
    e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 1), 4, false);
    let z = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 0), 1, true);
    let p = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    e.add_minion(SeatId(2), of_sect(Sect::Camarilla, 2), 4, false);
    hand(&mut e, 2, &["Redline"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: zvtes::decision::ActionKind::Diablerie,
            target: Some(z),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    pick(&mut e, votes(p, true));
    (e, d)
}

/// Redline: "Only usable when a blood hunt referendum passes and would burn
/// a vampire controlled by another Methuselah. That vampire is not burned.
/// Put this card on that vampire and take control of him or her. This
/// vampire is independent. This vampire gets one optional press when in
/// combat with a Camarilla vampire …" The card itself goes on it (not the
/// ash heap); the sect change held by it (rules-qa 104.3).
#[test]
fn redline_saves_a_hunted_vampire_and_takes_it() {
    let (mut e, d) = blood_hunt_at_a_redline();
    let req = until(&mut e, SeatId(2), WindowLabel::ResolutionChoice);
    let redline = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(d),
    };
    assert_eq!(req.options, vec![Choice::Pass, redline]);
    assert!(e.state.minions[d.index()].in_play, "not yet burned");
    pick(&mut e, redline);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[d.index()];
    assert!(v.in_play && !v.in_torpor, "not burned");
    assert_eq!(v.owner, SeatId(2));
    assert_eq!(e.state.sect_of(d), Some(Sect::Independent));
    let card = find_library("Redline");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == card && c.present() && c.host == Some(d) && !c.effect_only));
    assert!(!e.state.seats[2].ash.contains(&card));
    let cam = MinionId(2);
    assert_eq!(e.state.sect_of(cam), Some(Sect::Camarilla));
    assert_eq!(e.state.combat_presses_of(d, cam), 1);
    assert_eq!(e.state.combat_presses_of(d, MinionId(1)), 0, "a Sabbat");
}

/// Let pass, the blood hunt burns as before.
#[test]
fn redline_may_let_the_hunt_burn() {
    let (mut e, d) = blood_hunt_at_a_redline();
    until(&mut e, SeatId(2), WindowLabel::ResolutionChoice);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[d.index()].in_play, "burned");
}

/// Redline taking a vampire that claims a unique title one of its player's
/// vampires claims: taken, and the two contest the title (rules-qa 3.1;
/// owner, 2026-11-11; fuzz seed 110 once left the taken one's lost title
/// contested).
#[test]
fn redline_taking_a_rival_of_its_players_title_contests_it() {
    let (mut e, d) = blood_hunt_at_a_redline();
    let r = e.add_minion(SeatId(2), of_sect(Sect::Sabbat, 2), 4, false);
    for m in [d, r] {
        e.state.minions[m.index()].granted_title = Some(Title::Archbishop("Paris"));
    }
    e.stage_contests(d);
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(d),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[d.index()].owner, SeatId(2));
    for m in [d, r] {
        assert_eq!(e.state.claimed_title(m), Title::Archbishop("Paris"));
        assert!(e.state.minions[m.index()].title_contested, "{m:?}");
    }
}

/// "… and can enter combat with any Camarilla vampire as a +1 stealth Ⓓ
/// action": any Methuselah's, not a non-Camarilla one.
#[test]
fn redline_enters_combat_with_a_camarilla_vampire() {
    let (mut e, d) = blood_hunt_at_a_redline();
    until(&mut e, SeatId(2), WindowLabel::ResolutionChoice);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(d),
        },
    );
    let req = to_minion_phase(&mut e, SeatId(2));
    let redline = e
        .state
        .cards
        .iter()
        .position(|c| c.card == find_library("Redline") && c.present())
        .expect("Redline in play") as u32;
    let combat = |t: u32| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(CardId(redline)),
        ability: 0,
        actor: Some(d),
        target: Target::Minion(MinionId(t)),
    };
    assert!(req.options.contains(&combat(3)), "seat 1's prince");
    assert!(!req.options.contains(&combat(1)), "a Sabbat");
    pick(&mut e, combat(3));
    let a = e.action().expect("the action");
    assert_eq!(a.stealth(&e.state), 1);
}

// ---- Veil of Darkness -------------------------------------------------------

/// Seat 1's Veil of Darkness in play; seat 0's [obt] vampire `v` (6
/// blood) with two Black Metamorphosis in hand, `top` on its library.
fn veiled(top: &str) -> (Engine, MinionId) {
    let mut e = game(2);
    let v = e.add_minion(
        SeatId(0),
        textless(|c| c.disciplines.contains(&"obt") && c.capacity >= 6, 0),
        6,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let veil = in_play(&mut e, 1, "Veil of Darkness", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::FirstDisciplineCardRevealsTop {
            disc: "obt",
            burn: 2,
        },
        source: veil,
    });
    hand(&mut e, 0, &["Black Metamorphosis", "Black Metamorphosis"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(find_library(top));
    (e, v)
}

/// Veil of Darkness: "Each turn, when a vampire plays his or her first card
/// that turn that requires any Disciplines, that vampire's controller
/// reveals the top card of his or her library … If it is a master card,
/// the original card has no effect (no cost is paid), and in addition, if
/// the original card required Obtenebration [obt], the vampire burns 2
/// blood." [LSJ 20090323] Still played, the vampire not locked, it may act
/// again: its second card that turn is no first one.
#[test]
fn veil_of_darkness_voids_a_first_discipline_card_on_a_master() {
    let (mut e, v) = veiled("Minion Tap");
    pick(&mut e, play(0, 0, v));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 4, "no cost paid, 2 burned for [obt]");
    assert!(on(&e, "Black Metamorphosis", v).is_empty(), "no effect");
    assert!(!e.state.minions[v.index()].locked);
    assert!(req.options.contains(&play(0, 0, v)), "may act again");
    pick(&mut e, play(0, 0, v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 2, "the second paid");
    assert_eq!(on(&e, "Black Metamorphosis", v).len(), 1);
}

/// Not a master on top: the card plays as normal.
#[test]
fn veil_of_darkness_lets_a_card_play_over_a_non_master() {
    let (mut e, v) = veiled("Form of Mist");
    pick(&mut e, play(0, 0, v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 4, "paid");
    assert_eq!(on(&e, "Black Metamorphosis", v).len(), 1);
}

// ---- Brinksmanship ----------------------------------------------------------

/// Seat 1's Brinksmanship in play, four seats; seat 0 bare (library and
/// hand empty), the others with a library; seat 2's vampire may bleed seat
/// 3's, seat 3's seat 0's. Returns (engine, seat 0's vampire, seat 3's).
fn on_the_brink() -> (Engine, MinionId, MinionId) {
    let mut e = Engine::bare(4);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let b = e.add_minion(SeatId(3), plain("Ventrue", 2), 4, false);
    for s in 1..4 {
        for _ in 0..20 {
            e.state.seats[s].library.push(find_library("Form of Mist"));
        }
    }
    let brink = in_play(&mut e, 1, "Brinksmanship", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::ForcedWithdrawals,
        source: brink,
    });
    (e, v, b)
}

/// Brinksmanship: "Any Methuselah who has exhausted his or her library and
/// begins his or her unlock phase with less than a full hand must attempt
/// to withdraw. … If any Methuselah successfully withdraws, you are
/// ousted." The attempt made for them, none to choose.
#[test]
fn brinksmanship_forces_a_withdrawal_that_ousts_its_player() {
    let (mut e, _, _) = on_the_brink();
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.seats[0].withdrawal,
        zvtes::state::Withdrawal::Announced
    );
    assert!(e.state.seats[1].withdrawal == zvtes::state::Withdrawal::None);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(3), WindowLabel::MinionPhase);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    let s = &e.state.seats;
    assert!(s[0].withdrew, "kept a round");
    assert!(s[1].ousted && !s[1].withdrew, "its player ousted");
}

/// "You are ousted" once per player, however many copies they control and
/// in whatever order: seat 1's two copies around seat 2's oust it once, its
/// predator (seat 3) paid once.
#[test]
fn brinksmanship_ousts_a_player_with_two_copies_once() {
    let (mut e, _, _) = on_the_brink();
    for seat in [2, 1] {
        let c = in_play(&mut e, seat, "Brinksmanship", None);
        e.state.layers.push(zvtes::cards::Layer {
            scope: zvtes::cards::Scope::InPlay,
            modifier: zvtes::cards::Modifier::ForcedWithdrawals,
            source: c,
        });
    }
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(3), WindowLabel::MinionPhase);
    let pool = e.state.seats[3].pool;
    pick(&mut e, Choice::Pass);
    let mut left = 100;
    while let zvtes::game::Step::Decision(req) = e.run() {
        left -= 1;
        assert!(left > 0, "the game ends");
        e.choose(fallback(&req));
    }
    let s = &e.state.seats;
    assert!(s[0].withdrew && s[1].ousted && s[2].ousted, "seat 3 alone");
    assert_eq!(s[3].pool, pool + 6, "one oust's pool");
    assert_eq!(s[3].half_vps, 4, "one oust's victory point, the survivor's");
}

/// "On that Methuselah's next unlock phase, if the withdrawal fails, that
/// Methuselah is ousted": broken by a combat (rules-qa 95.4).
#[test]
fn brinksmanship_ousts_a_failed_withdrawal() {
    let (mut e, v, b) = on_the_brink();
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(3), WindowLabel::MinionPhase);
    pick(&mut e, bleed(b));
    pick(&mut e, block(v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let s = &e.state.seats;
    assert!(s[0].ousted && !s[0].withdrew);
    assert!(!s[1].ousted);
}

// ---- Chain of Command -------------------------------------------------------

/// An uncontrolled slot of `card` with `counters`.
fn slot(card: u16, counters: u8) -> zvtes::state::UncontrolledSlot {
    zvtes::state::UncontrolledSlot {
        card,
        copy: zvtes::copies::Untracked(zvtes::copies::CopyId::NONE),
        counters,
        banished: None,
    }
}

/// Seat 0's elder `a` (6 blood, [dom] given) with Chain of Command; its
/// uncontrolled region: two young vampires (1 and 0 counters), one not
/// younger than `a`.
/// Returns (engine, a, young ones).
fn chain_of_command() -> (Engine, MinionId, [u16; 2]) {
    let mut e = game(2);
    let elder = elder_camarilla();
    let a = e.add_minion(SeatId(0), elder, 6, false);
    let slots = &mut e.state.minions[a.index()].beast_disciplines;
    slots[0] = Some("dom");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let young = [
        textless(|c| c.capacity <= 3, 0),
        textless(|c| c.capacity <= 3, 1),
    ];
    let old = CRYPT
        .iter()
        .position(|c| {
            c.capacity >= CRYPT[elder as usize].capacity && c.name != CRYPT[elder as usize].name
        })
        .expect("not younger") as u16;
    let u = &mut e.state.seats[0].uncontrolled;
    u.push(slot(young[0], 1));
    u.push(slot(old, 0));
    u.push(slot(young[1], 0));
    hand(&mut e, 0, &["Chain of Command"]);
    fill_hand(&mut e, 0);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::None,
        },
    );
    (e, a, young)
}

fn in_play_of(e: &Engine, card: u16) -> Option<MinionId> {
    e.state
        .minions
        .iter()
        .position(|v| v.in_play && v.crypt_card() == Some(card))
        .map(|i| MinionId(i as u32))
}

/// Chain of Command: "[dom] Put X younger vampires from your uncontrolled
/// region in play with 1 blood from the blood bank each (and with any
/// blood they already have). … Those vampires must bleed. When one of them
/// bleeds successfully … move them all to the bottom of your crypt." The
/// vampires named as the card is played, X their number (D1, rules-qa
/// 31.2): X blood paid; the elder not offered; the phase withheld until
/// one bleeds.
/// [LSJ 20041201] Back to the crypt: a crypt card never goes to the library.
#[test]
fn chain_of_command_sends_younger_vampires_to_bleed() {
    let (mut e, a, young) = chain_of_command();
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::PlayTerms);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::Slot(0), Choice::Slot(2)]
    );
    pick(&mut e, Choice::Slot(0));
    pick(&mut e, Choice::Slot(2));
    request(&mut e);
    assert!(e.action().is_some(), "named before it resolves");
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "X = 2 blood");
    let (y0, y1) = (
        in_play_of(&e, young[0]).expect("first"),
        in_play_of(&e, young[1]).expect("second"),
    );
    assert_eq!((blood(&e, y0), blood(&e, y1)), (2, 1));
    assert!(!req.options.contains(&Choice::Pass), "they must bleed");
    assert_eq!(
        e.dues(&req),
        [
            zvtes::actions::Due::Bleed(y0),
            zvtes::actions::Due::Bleed(y1)
        ]
    );
    pick(&mut e, bleed(y0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(in_play_of(&e, young[0]).is_none() && in_play_of(&e, young[1]).is_none());
    let crypt = &e.state.seats[0].crypt;
    let bottom: Vec<u16> = (0..2).map(|i| *crypt.get(i).expect("bottom")).collect();
    assert!(bottom.contains(&young[0]) && bottom.contains(&young[1]));
    assert_eq!(e.state.seats[1].pool, 10 - 1, "bled");
}

/// Chain of Command: "(X blood.)" X the vampires named less [DOM]'s extra
/// one, within the actor's blood: a 2-blood elder names two for [dom].
#[test]
fn chain_of_command_names_as_many_as_its_blood() {
    let (mut e, a, _) = chain_of_command();
    e.state.minions[a.index()].blood = 2;
    pick(&mut e, Choice::Slot(0));
    let req = request(&mut e);
    assert_eq!(req.options, vec![Choice::Pass, Choice::Slot(2)]);
    pick(&mut e, Choice::Slot(2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 0, "X = 2");
}

/// Two copies of a unique vampire: only one may be named (LSJ 20040722).
/// [LSJ 20040722] By the rule prohibiting self-contesting.
#[test]
fn chain_of_command_names_one_copy_of_a_unique_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), elder_camarilla(), 6, false);
    e.state.minions[a.index()].beast_disciplines[0] = Some("dom");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let young = textless(|c| c.capacity <= 3 && c.unique, 0);
    let u = &mut e.state.seats[0].uncontrolled;
    u.push(slot(young, 0));
    u.push(slot(young, 0));
    hand(&mut e, 0, &["Chain of Command"]);
    fill_hand(&mut e, 0);
    pick(&mut e, play_at(0, 0, Some(a), Target::None));
    let req = request(&mut e);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::Slot(0), Choice::Slot(1)]
    );
    pick(&mut e, Choice::Slot(0));
    let req = request(&mut e);
    assert_ne!(
        req.window,
        WindowLabel::PlayTerms,
        "the other copy not offered"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(in_play_of(&e, young).is_some());
    assert_eq!(e.state.seats[0].uncontrolled.len(), 1);
}

/// "You cannot choose any unique vampires already in play": a vampire some
/// Methuselah controls is not offered.
#[test]
fn chain_of_command_skips_a_vampire_in_play() {
    let mut e = game(2);
    let elder = elder_camarilla();
    let a = e.add_minion(SeatId(0), elder, 6, false);
    let slots = &mut e.state.minions[a.index()].beast_disciplines;
    slots[0] = Some("dom");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let young = [
        textless(|c| c.capacity <= 3, 0),
        textless(|c| c.capacity <= 3, 1),
    ];
    e.add_minion(SeatId(1), young[1], 2, false);
    let u = &mut e.state.seats[0].uncontrolled;
    u.push(slot(young[0], 0));
    u.push(slot(young[1], 0));
    hand(&mut e, 0, &["Chain of Command"]);
    fill_hand(&mut e, 0);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::None,
        },
    );
    let req = request(&mut e);
    assert_eq!(req.options, vec![Choice::Pass, Choice::Slot(0)]);
    pick(&mut e, Choice::Slot(0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(in_play_of(&e, young[0]).is_some(), "the only one offered");
    assert_eq!(e.state.seats[0].uncontrolled.len(), 1, "the other stays");
}

/// Karsh (advanced): "[MERGED] Imperator (3 votes): +1 intercept": merged,
/// a unique Camarilla title worth 3 votes and +1 intercept; not before.
/// [ANK 20190407] "[MERGED] Imperator is a unique Camarilla title worth 3
/// votes, it is the same as the one granted by Imperator."
#[test]
fn karsh_merged_is_the_imperator() {
    for merged in [false, true] {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), find_vampire("Karsh", true), 4, false);
        if merged {
            e.state.minions[k.index()].merged = Some(find_card("Karsh"));
        }
        let want = if merged {
            Title::Camarilla {
                name: "Imperator",
                votes: 3,
            }
        } else {
            Title::None
        };
        assert_eq!(e.state.claimed_title(k), want);
        assert_eq!(e.state.title_votes_of(k), if merged { 3 } else { 0 });
        assert_eq!(
            e.state
                .printed_mod(k, zvtes::cards::Stat::Intercept, Default::default()),
            i32::from(merged)
        );
    }
}

/// Menele merged: "[MERGED] Menele can strike: steal 2 blood or life
/// (becoming blood)": blocked, he steals 2 of the blocker's blood; his
/// base card's +1 bleed kept.
#[test]
fn menele_merged_strikes_to_steal_two_blood() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_vampire("Menele", true), 4, false);
    e.state.minions[m.index()].merged = Some(find_vampire("Menele", false));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert_eq!(
        e.state
            .printed_mod(m, zvtes::cards::Stat::Bleed, Default::default()),
        1
    );
    pick(&mut e, bleed(m));
    pick(&mut e, block(b));
    let mut req = request(&mut e);
    while vampire_use(&req, m, 1).is_none() {
        e.choose(fallback(&req));
        req = request(&mut e);
    }
    pick(&mut e, vampire_use(&req, m, 1).expect("the strike"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 2);
}

/// Ardan Lane: "When stealing blood with a strike that requires Blood
/// Sorcery [tha], Ardan steals an additional blood." Theft of Vitae [THA]
/// steals 3 (and he takes the blocker's hand strike).
#[test]
fn ardan_lane_steals_one_more_with_blood_sorcery() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Ardan Lane"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Theft of Vitae"]);
    fill_hand(&mut e, 0);
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, block(z));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let theft = *req
        .options
        .iter()
        .find(|o| {
            matches!(
                o,
                Choice::Play {
                    card: 0,
                    mode: 1,
                    ..
                }
            )
        })
        .expect("Theft of Vitae [THA]");
    pick(&mut e, theft);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, z)), (4 + 3 - 1, 4 - 3));
}

/// Nedal, The Careless: "Combat cards Nedal plays are not replaced until
/// your next unlock phase." Target Vitals, its replacement held a turn.
#[test]
fn nedal_replaces_his_combat_cards_at_the_next_unlock_phase() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nedal, The Careless"), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Target Vitals"]);
    fill_hand(&mut e, 0);
    for _ in 0..5 {
        e.state.seats[0].library.push(find_library("Form of Mist"));
    }
    pick(&mut e, bleed(n));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, block(z));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let aim = *req
        .options
        .iter()
        .find(|o| matches!(o, Choice::Play { card: 0, .. }))
        .expect("Target Vitals");
    pick(&mut e, aim);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.hand_size(SeatId(0)) - 1,
        e.state.seats[0].hand.len()
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.hand_size(SeatId(0)), e.state.seats[0].hand.len());
}

// ---- Grading fixes ----------------------------------------------------------

/// The ability `ai` of `card`, used by `actor` at `target`.
fn use_card(card: CardId, ai: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(card),
        ability: ai,
        actor,
        target,
    }
}

/// The action under way is directed at no Methuselah.
fn undirected(e: &mut Engine) -> bool {
    request(e);
    e.action()
        .expect("the action")
        .directed_seat(&e.state)
        .is_none()
}

/// Principia Discordia: "[aus] Ⓓ Burn an equipment. … [ser] Ⓓ Steal 1
/// blood from a ready vampire." One's own equipment or vampire too,
/// undirected (D12, rules-qa 20.1, 89.9; a blood steal from one's own,
/// 71.13); never the acting vampire itself.
#[test]
fn principia_discordia_aims_at_ones_own_undirected() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("aus", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(b));
    hand(&mut e, 0, &["Principia Discordia"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Card(gun)));
    assert!(undirected(&mut e));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[gun.index()].in_play, "burned");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 2, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Principia Discordia"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&play_at(0, 2, Some(a), Target::Minion(a))));
    pick(&mut e, play_at(0, 2, Some(a), Target::Minion(b)));
    assert!(undirected(&mut e));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, b)), (3, 3));
}

/// Reformation [ser]: "Put 1 corruption counter on any vampire. If the
/// number of your corruption counters on the vampire equals or exceeds
/// their capacity, you can burn those counters to steal the target
/// vampire." One's own vampire, undirected (D12, rules-qa 20.1); never a
/// steal from oneself (90.4).
#[test]
fn reformation_corrupts_ones_own_vampire_without_a_steal() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    let t = e.add_minion(SeatId(0), textless(|c| c.capacity == 2, 0), 2, false);
    e.state.minions[t.index()].add_corruption(SeatId(0), 1);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Reformation"]);
    pick_when(&mut e, play_at(0, 2, Some(a), Target::Minion(t)));
    assert!(undirected(&mut e));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::ChooseMinion(_))
    });
    let v = &e.state.minions[t.index()];
    assert_eq!(v.corruption_of(SeatId(0)), 2, "the counter");
    assert_eq!(v.owner, SeatId(0));
}

/// Sleep of Reason [dem]: "Ⓓ Send a younger vampire with 0 or 1 blood to
/// torpor." Songs of the Distant Vitae [qui]: "Ⓓ Send a vampire who has
/// committed diablerie since your last turn to torpor." One's own vampire
/// too, undirected (D12, rules-qa 20.1); never the acting one.
#[test]
fn sleep_of_reason_and_songs_send_ones_own_to_torpor() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.capacity >= 6 && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("dem")),
            0,
        ),
        4,
        false,
    );
    let t = e.add_minion(SeatId(0), textless(|c| c.capacity <= 2, 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Sleep of Reason"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(t)));
    assert!(undirected(&mut e));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].in_torpor);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("qui", 0), 4, false);
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].previous_turn = e.state.turn - 2;
    for m in [a, t] {
        e.state.minions[m.index()].diablerie_turn = Some(e.state.turn);
    }
    hand(&mut e, 0, &["Songs of the Distant Vitae"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&play_at(0, 0, Some(a), Target::Minion(a))));
    pick(&mut e, play_at(0, 0, Some(a), Target::Minion(t)));
    assert!(undirected(&mut e));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].in_torpor);
}

/// Jack of Both Sides: "This vampire equips or employs an equipment or
/// retainer from your hand." An employ action for what reads one (TOM
/// 19960130): Rosa Hernández's "+1 stealth when employing retainers".
/// [TOM 19960130] An action of the chosen card's type.
#[test]
fn jack_of_both_sides_employs_as_an_employ_action() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Rosa Hernández"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Jack of Both Sides", "Twisted Bloodhound"]);
    pick_when(&mut e, play(0, 1, r));
    request(&mut e);
    let a = e.action().expect("the employ");
    assert!(a.is_employ_action(&e.state));
    assert_eq!(a.stealth(&e.state), 2 + 1, "Jack's and Rosa's");
}

/// Biothaumaturgic Experiment [tha]: "Employ an animal retainer from your
/// hand ignoring requirements": not an employ action (ANK 20180817), unlike
/// Jack of Both Sides'.
#[test]
fn biothaumaturgic_experiment_is_no_employ_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("tha", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Biothaumaturgic Experiment", "Raven Spy"]);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert!(!e.action().expect("the action").is_employ_action(&e.state));
}

/// Redline: "… can enter combat with any Camarilla vampire as a +1 stealth
/// Ⓓ action": another Methuselah's, never its own controller's (rules-qa
/// 17.3).
#[test]
fn redline_enters_no_combat_with_its_own_camarilla() {
    let (mut e, d) = blood_hunt_at_a_redline();
    until(&mut e, SeatId(2), WindowLabel::ResolutionChoice);
    pick(&mut e, play_at(0, 0, None, Target::Minion(d)));
    let req = to_minion_phase(&mut e, SeatId(2));
    let redline = e
        .state
        .cards
        .iter()
        .position(|c| c.card == find_library("Redline") && c.present())
        .expect("Redline in play") as u32;
    let own = MinionId(4);
    assert_eq!(e.state.minions[own.index()].owner, SeatId(2));
    assert_eq!(e.state.sect_of(own), Some(Sect::Camarilla));
    let combat = |t: MinionId| use_card(CardId(redline), 0, Some(d), Target::Minion(t));
    assert!(req.options.contains(&combat(MinionId(3))));
    assert!(!req.options.contains(&combat(own)));
}

/// The Holds: "The chosen … card costs that Methuselah's vampires an
/// additional blood": not an ally (D16). Seat 0's Ossian dodges with
/// the Dodge stored on Chiram's Hold, at no cost.
#[test]
fn a_hold_does_not_tax_an_ally() {
    let mut e = game(2);
    let g = e.add_ally(SeatId(0), find_library("Ossian"));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hold_fixture(&mut e, "Chiram's Hold", "Dodge");
    hand(&mut e, 0, &["Dodge"]);
    let req = request(&mut e);
    let enter = *req
        .options
        .iter()
        .find(|o| {
            matches!(o, Choice::UseAbility { actor: Some(m), target: Target::Minion(t), .. }
                if *m == g && *t == z)
        })
        .expect("Ossian enters combat");
    pick(&mut e, enter);
    offering(&mut e, play_at(0, 0, Some(g), Target::None));
}

/// Chiram's Hold: "move a combat card from that Methuselah's ash heap to
/// this card". Elder Michaelis's Hold: "… an action modifier …". None
/// there, not played (D14: a card naming an object there is none of;
/// rules-qa 60.17).
#[test]
fn a_hold_needs_its_card_in_the_ash_heap() {
    for name in ["Chiram's Hold", "Elder Michaelis's Hold"] {
        for stocked in [false, true] {
            let mut e = game(2);
            let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
            let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
            e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
            e.state.seats[0]
                .ash
                .put(find_library("Bum's Rush"), zvtes::copies::CopyId::NONE);
            if stocked {
                let card = if name == "Chiram's Hold" {
                    "Dodge"
                } else {
                    "Alacrity"
                };
                e.state.seats[0]
                    .ash
                    .put(find_library(card), zvtes::copies::CopyId::NONE);
            }
            hand(&mut e, 1, &[name]);
            pick_when(&mut e, bleed(a));
            if name == "Chiram's Hold" {
                pick_when(&mut e, block(z));
            }
            let offered =
                offered_before(&mut e, play(0, 0, z), SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(offered, stocked, "{name} stocked {stocked}");
        }
    }
}

/// Cold Amber's Hold: "Give this card to another Methuselah and move an
/// action card from that Methuselah's ash heap to this card." Not at a
/// Methuselah with none (D14, rules-qa 60.17); the card named as the
/// action is announced (D1, 60.17's Sargon Fragment), gone by resolution
/// a fizzle: no Hold given.
#[test]
fn cold_ambers_hold_names_its_card_as_announced() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[1]
        .ash
        .put(find_library("Dodge"), zvtes::copies::CopyId::NONE);
    hand(&mut e, 0, &["Cold Amber's Hold"]);
    let hold_at = play_at(0, 0, Some(a), Target::Seat(SeatId(1)));
    let req = request(&mut e);
    assert!(!req.options.contains(&hold_at), "no action card there");
    for c in ["Bum's Rush", "Computer Hacking"] {
        e.state.seats[1]
            .ash
            .put(find_library(c), zvtes::copies::CopyId::NONE);
    }
    pick(&mut e, hold_at);
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::PlayTerms);
    let rush = Choice::FindCard(find_library("Bum's Rush"));
    assert_eq!(req.options.len(), 2, "{req:?}");
    assert!(req.options.contains(&rush));
    pick(&mut e, rush);
    request(&mut e);
    assert!(e.action().is_some(), "named before it resolves");
    let ash = &mut e.state.seats[1].ash;
    let i = ash
        .iter()
        .position(|&c| c == find_library("Bum's Rush"))
        .expect("there");
    ash.take(i);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state
            .cards
            .iter()
            .any(|c| c.in_play && c.card == find_library("Cold Amber's Hold")),
        "fizzled"
    );
}

/// Cooler: "… you may move 1 blood from this card to this vampire." An
/// ally bearer gets nothing ("this vampire", D16).
#[test]
fn cooler_feeds_no_ally() {
    let mut e = game(2);
    let g = e.add_ally(SeatId(0), find_library("Ossian"));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = in_play(&mut e, 0, "Cooler", Some(g));
    e.state.cards[c.index()].counters = 4;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_card(c, 0, None, Target::Minion(g)),
    );
}

/// Drum of Xipe Totec: "The vampire with this equipment has superior
/// Celerity [CEL]. This vampire gets one optional maneuver each combat."
/// An ally bearer gets neither (D16): seat 1's Ossian blocks, no
/// maneuver offered.
#[test]
fn drum_of_xipe_totec_gives_an_ally_no_maneuver() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let g = e.add_ally(SeatId(1), find_library("Ossian"));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let d = in_play(&mut e, 1, "Drum of Xipe Totec", Some(g));
    bleed_blocked(&mut e, a, g);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. }
            if *c == d)
    });
}

/// Entrenching: "If this vampire has 4 or more blood, he or she gains 4
/// blood." Played by an ally (no requirement), a legal no-op (D16): the
/// Ossian's 4 life stay 4.
#[test]
fn entrenching_gives_an_ally_nothing() {
    let mut e = game(2);
    let g = e.add_ally(SeatId(0), find_library("Ossian"));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Entrenching"]);
    let life = blood(&e, g);
    assert!(life >= 4);
    pick_when(&mut e, play(0, 0, g));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, g), life);
}

/// Emergency Preparations: "… unlock a vampire with a capacity above 7."
/// An unlocked one too, a legal no-op (D14, rules-qa 57.10).
#[test]
fn emergency_preparations_may_unlock_an_unlocked_elder() {
    let mut e = game(2);
    let big = e.add_minion(SeatId(0), elder(), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for g in ["Fueled by Heart's Blood", "Unmasking, The"] {
        in_play(&mut e, 1, g, None);
    }
    assert!(e.state.gehenna_in_play() >= 2);
    hand(&mut e, 0, &["Emergency Preparations"]);
    let req = request(&mut e);
    assert!(
        req.options
            .contains(&play_at(0, 1, None, Target::Minion(big))),
        "{req:?}"
    );
}

/// Firebrand: "During your minion phase, this anarch can burn 1 blood to
/// unlock a ready younger anarch." An unlocked one too, a legal no-op
/// (D14, rules-qa 57.10, 73.3), known to gain nothing.
#[test]
fn firebrand_may_unlock_an_unlocked_anarch() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), elder(), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    let y = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Anarch) && c.capacity <= 4, 0),
        2,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let f = in_play(&mut e, 0, "Firebrand", Some(a));
    let u = use_card(f, 0, None, Target::Minion(y));
    let req = request(&mut e);
    assert!(req.options.contains(&u), "{req:?}");
    assert!(e.gains_nothing(SeatId(0), &u));
}

/// Étienne Fauberge: "Actions directed at Étienne cost 1 additional blood."
/// An ally pays no blood (rules-qa 73.5): Ossian's combat action is
/// not offered at him (LSJ 20030815); a vampire's Principia Discordia at
/// him costs 1 blood, paid as it resolves.
/// [LSJ 20030815] Allies have no blood.
#[test]
fn etienne_fauberge_costs_a_blood_to_act_at() {
    let mut e = game(2);
    let g = e.add_ally(SeatId(0), find_library("Ossian"));
    let a = e.add_minion(SeatId(0), vampire_with("qui", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    let et = e.add_minion(SeatId(1), find_card("Étienne Fauberge"), 4, false);
    let o = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Principia Discordia"]);
    let req = request(&mut e);
    let ally_at = |t: MinionId| {
        req.options.iter().any(|o| {
            matches!(o, Choice::UseAbility { actor: Some(m), target: Target::Minion(x), .. }
                if *m == g && *x == t)
        })
    };
    assert!(ally_at(o) && !ally_at(et), "{req:?}");
    e.state.minions[o.index()].locked = true;
    pick(&mut e, play_at(0, 1, Some(a), Target::Minion(et)));
    request(&mut e);
    let before = blood(&e, a);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), before - 1, "the additional blood");
    assert!(e.state.minions[et.index()].locked);
}

/// Guillermo Arsuaga: "A haven played on or by Guillermo costs 1 less blood
/// or pool." Secure Haven (1 pool) played on him by his Methuselah costs
/// nothing; on another vampire, its full cost.
#[test]
fn guillermo_arsuaga_takes_a_haven_for_less() {
    for on_him in [true, false] {
        let mut e = game(2);
        let gu = e.add_minion(SeatId(0), find_card("Guillermo Arsuaga"), 4, false);
        let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Secure Haven"]);
        until(&mut e, SeatId(0), WindowLabel::MasterPhase);
        master_on(&mut e, "Secure Haven", if on_him { gu } else { b });
        assert_eq!(pool(&e)[0], if on_him { 10 } else { 9 }, "on him {on_him}");
    }
}

/// Hanna Redmonds: "An older vampire opposing Hanna gets one optional
/// maneuver or press on the first round of combat." Read live (D13): once
/// Serhat Gunde is no longer older, the maneuver is withdrawn.
#[test]
fn hanna_redmonds_reads_the_opponents_age_live() {
    let mut e = game(2);
    let s = e.add_minion(
        SeatId(0),
        find_card("Serhat Gunde, Combat Junkie"),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), find_card("Hanna Redmonds"), 2, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    bleed_blocked(&mut e, s, h);
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&Choice::UseManeuver));
    let down =
        e.state.capacity_of(s).expect("capacity") - e.state.capacity_of(h).expect("capacity");
    let src = in_play(&mut e, 0, ".44 Magnum", Some(s));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostCapacityDown(down),
        source: src,
    });
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::UseManeuver), "{req:?}");
}

/// Maldavis: "Any ready primogen can burn 1 blood from Maldavis as a +1
/// stealth Ⓓ action." Any Methuselah's primogen (D11), his own
/// controller's too, undirected (D12).
#[test]
fn maldavis_is_bled_by_his_own_primogen() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Maldavis"), 3, false);
    let p = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| c.title == Title::Primogen && zvtes::cards::vampire(i as u16).is_none())
        .expect("a textless primogen") as u16;
    let p = e.add_minion(SeatId(0), p, 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    let u = vampire_use(&req, m, 0).expect("Maldavis's action offered");
    assert!(
        matches!(u, Choice::UseAbility { actor: Some(x), .. } if x == p),
        "{u:?}"
    );
    pick(&mut e, u);
    assert!(undirected(&mut e));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, m), 2);
}

// ---- Cry Wolf, Delivery Truck, Nod, Undead Persistence ----------------------

/// The card in play that is ally `m`.
fn ally_card(e: &Engine, m: MinionId) -> CardId {
    e.state
        .cards
        .iter()
        .position(|c| c.in_play && c.body == Some(m))
        .map(|i| CardId(i as u32))
        .expect("the ally's card")
}

/// Cry Wolf's enter-combat action at `b`.
fn cry_wolf_at(e: &Engine, w: MinionId, b: MinionId) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(ally_card(e, w)),
        ability: 0,
        actor: Some(w),
        target: Target::Minion(b),
    }
}

/// Cry Wolf: "This werewolf must enter combat with any minion controlled
/// by another Methuselah as a Ⓓ action. He gets one optional press each
/// combat. This werewolf does not unlock as normal during your unlock
/// phase. If he is locked during your unlock phase, burn him." His action
/// the only one offered until taken, his press in its combat; locked by
/// it, burned at his next unlock phase.
#[test]
fn cry_wolf_must_fight_and_burns_when_still_locked() {
    let mut e = game(2);
    let w = e.add_ally(SeatId(0), find_library("Cry Wolf"));
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), elder(), 8, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&cry_wolf_at(&e, w, b)), "{req:?}");
    assert!(!req.options.contains(&bleed(v)), "his action first");
    assert!(
        !req.options.contains(&Choice::Pass),
        "the phase waits for it"
    );
    let go = cry_wolf_at(&e, w, b);
    pick(&mut e, go);
    let req = offering(&mut e, Choice::UsePress);
    assert_eq!(req.seat, SeatId(0), "his press");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[w.index()].locked);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[w.index()].present(),
        "burned, still locked"
    );
}

/// Cry Wolf: "If he unlocks after having performed his mandatory action,
/// he's free to take any action." [LSJ 20090226]
#[test]
fn cry_wolf_unlocked_after_his_action_is_free() {
    let mut e = game(2);
    let w = e.add_ally(SeatId(0), find_library("Cry Wolf"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), elder(), 8, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let go = cry_wolf_at(&e, w, b);
    pick(&mut e, go);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[w.index()].locked = false;
    let req = request(&mut e);
    assert!(req.options.contains(&bleed(w)), "{req:?}");
    assert!(req.options.contains(&Choice::Pass), "not forced again");
}

/// Cry Wolf: "If you manage to unlock it during the unlock phase, it does
/// not need to burn." [LSJ 20031014] Unlocked before the phase ends, he
/// stays.
#[test]
fn cry_wolf_unlocked_during_the_unlock_phase_stays() {
    let mut e = game(2);
    let w = e.add_ally(SeatId(0), find_library("Cry Wolf"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), elder(), 8, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let go = cry_wolf_at(&e, w, b);
    pick(&mut e, go);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(e.state.minions[w.index()].locked, "no normal unlock");
    e.state.minions[w.index()].locked = false;
    e.choose(index_of(&req, Choice::Pass));
    request(&mut e);
    assert!(e.state.minions[w.index()].present(), "unlocked: not burned");
}

/// Cry Wolf: "Requires a ready anarch. If the action to recruit this ally
/// is blocked, the acting anarch goes to torpor." "If the action is
/// blocked, *all* effects that would be triggered by a successful block
/// must be applied (ordered by the acting Methuselah). Other effects and
/// modifiers can only be used after all triggered effects have been
/// handled." [ANK 20220116] [ANK 20211207] [ANK 20210627] [RTR 19991206]
/// [LSJ 20091125] Blocked by a +1 intercept vampire (the recruit's +1
/// stealth): in torpor before block resolution, no combat.
#[test]
fn cry_wolf_blocked_recruit_sends_the_anarch_to_torpor() {
    for blocked in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let b = e.add_minion(SeatId(1), find_card("Hafthor Thorsteinsson"), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Cry Wolf"]);
        pick_when(&mut e, play(0, 0, a));
        if blocked {
            pick_when(&mut e, block(b));
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[a.index()].in_torpor, blocked);
        assert_eq!((blood(&e, a), blood(&e, b)), (4, 4), "no combat");
        let card = find_library("Cry Wolf");
        assert_eq!(e.state.seats[0].ash.contains(&card), blocked);
        assert_eq!(
            e.state.cards.iter().any(|c| c.in_play && c.card == card),
            !blocked
        );
    }
}

/// Delivery Truck: "During your master phase, you may show a non-location
/// equipment card from your hand to the other players and place it face
/// down on the Delivery Truck if it doesn't already have one. … Any minion
/// you control may equip that card (face up) as a +1 stealth action
/// (requirements and cost apply as normal)." Stored from hand, then
/// equipped by another of its controller's minions, its cost paid.
#[test]
fn delivery_truck_carries_an_equipment_to_any_own_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let w = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let truck = in_play(&mut e, 0, "Delivery Truck", Some(a));
    hand(&mut e, 0, &["Villein", ".44 Magnum"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let store = ability(truck.0, 0);
    assert!(req.options.contains(&store), "{req:?}");
    pick(&mut e, store);
    request(&mut e);
    assert_eq!(
        e.state.cards[truck.index()].stored.to_cards(),
        vec![find_library(".44 Magnum")]
    );
    let equip = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(truck),
        ability: 1,
        actor: Some(w),
        target: Target::None,
    };
    pick_when(&mut e, equip);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, ".44 Magnum", w).len(), 1, "equipped by the other");
    assert!(e.state.cards[truck.index()].stored.to_cards().is_empty());
    let cost = match zvtes::library::LIBRARY[find_library(".44 Magnum") as usize].pool_cost {
        zvtes::library::Cost::Fixed(n) => n as i32,
        _ => 0,
    };
    assert_eq!(pool(&e)[0], 10 - cost);
}

/// Delivery Truck: "Does not prevent the acting vampire from playing the
/// equipment/retainer/ally directly this turn if he unlocks (not the same
/// action)." [ANK 20180817] "Is an action of the appropriate type for the
/// chosen card (recruit/employ/equip)." [TOM 19960130] An equip action:
/// Hag's Wrinkles [obf] plays on it against a block; unlocked, the vampire
/// equips the same card from hand.
#[test]
fn delivery_truck_equip_is_the_trucks_equip_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Hafthor Thorsteinsson"), 4, false);
    let truck = in_play(&mut e, 0, "Delivery Truck", Some(a));
    let (k, copy) = (find_library(".44 Magnum"), zvtes::copies::CopyId::NONE);
    e.state.cards[truck.index()].stored.put(k, copy);
    hand(&mut e, 0, &["Hag's Wrinkles", ".44 Magnum"]);
    let equip = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(truck),
        ability: 1,
        actor: Some(a),
        target: Target::None,
    };
    pick(&mut e, equip);
    pick_when(&mut e, block(b));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, ".44 Magnum", a).len(), 1);
    e.state.minions[a.index()].locked = false;
    let req = request(&mut e);
    assert!(
        req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == a)),
        "{req:?}"
    );
}

/// Delivery Truck: "(requirements and cost apply as normal)". Elixir of
/// Distillation, an Anarch's only: not offered to a Ventrue; an Anarch's
/// equip of it an action requiring an Anarch, CrimethInc. [pro] follows
/// it (rules-qa 62.21, superseding ANK 20180817's reading).
#[test]
fn delivery_truck_applies_the_stored_cards_requirements() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Anarch) && c.capacity >= 4 && c.disciplines.contains(&"pro"),
            0,
        ),
        4,
        false,
    );
    let w = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let truck = in_play(&mut e, 0, "Delivery Truck", Some(w));
    let k = find_library("Elixir of Distillation");
    e.state.cards[truck.index()]
        .stored
        .put(k, zvtes::copies::CopyId::NONE);
    hand(&mut e, 0, &["CrimethInc."]);
    let by = |m| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(truck),
        ability: 1,
        actor: Some(m),
        target: Target::None,
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&by(w)), "not an Anarch");
    assert!(req.options.contains(&by(a)), "{req:?}");
    pick(&mut e, by(a));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, "Elixir of Distillation", a).len(), 1);
    assert!(
        !e.state.minions[a.index()].locked,
        "unlocked by CrimethInc."
    );
}

/// Nod: "Beginning with you and going clockwise once around the table,
/// each Methuselah can rearrange the equipment on his or her ready
/// minions." Each Methuselah moves their own, the player first.
#[test]
fn nod_lets_each_methuselah_rearrange_their_equipment() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a2 = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let b2 = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let x = in_play(&mut e, 0, ".44 Magnum", Some(a));
    let y = in_play(&mut e, 1, ".44 Magnum", Some(b));
    hand(&mut e, 0, &["Nod"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_play(0));
    let req = to_window(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::ChooseMinion(a), Choice::ChooseMinion(a2)]
    );
    pick(&mut e, Choice::ChooseMinion(a2));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(1));
    pick(&mut e, Choice::ChooseMinion(b2));
    request(&mut e);
    assert_eq!(e.state.cards[x.index()].host, Some(a2));
    assert_eq!(e.state.cards[y.index()].host, Some(b2));
}

/// Nod: "Choose for each equipment which minion will receive it, then
/// transfer all the equipment accordingly." [ANK 20190617-1] Two vehicles
/// swap: the second minion, holding its own, may take the first; the
/// second then goes to the first minion.
#[test]
fn nod_chooses_every_move_before_any() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a2 = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let x = in_play(&mut e, 0, "Sport Bike", Some(a));
    let y = in_play(&mut e, 0, "Sport Bike", Some(a2));
    hand(&mut e, 0, &["Nod"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_play(0));
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::ChooseMinion(a2)), "{req:?}");
    pick(&mut e, Choice::ChooseMinion(a2));
    request(&mut e);
    assert_eq!(e.state.cards[x.index()].host, Some(a2));
    assert_eq!(e.state.cards[y.index()].host, Some(a));
}

/// A textless vampire with basic Fortitude only.
fn fortitude(nth: usize) -> u16 {
    textless(|c| c.capacity >= 4 && c.disciplines.contains(&"for"), nth)
}

/// Undead Persistence: "[for] Only usable when this vampire should go into
/// torpor. Instead, this vampire gets an optional press, will not go to
/// torpor until combat ends but is still considered wounded and can be
/// burned as normal. … This vampire is sent into torpor after combat." The
/// blocker hit at 0 blood: still ready with its press, in torpor after.
#[test]
fn undead_persistence_holds_torpor_until_the_combat_ends() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), elder(), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let a = e.add_minion(SeatId(1), fortitude(0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Undead Persistence"]);
    bleed_blocked(&mut e, b, a);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    let req = offering(&mut e, Choice::UsePress);
    assert_eq!(req.seat, SeatId(1), "its press");
    assert!(e.state.minions[a.index()].ready(), "not in torpor yet");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].in_torpor, "after the combat");
}

/// Undead Persistence: "Sequencing rules apply, and if played first,
/// effects that stop a vampire from going to torpor would prevent the use
/// of effects that are only usable on a vampire going to torpor." [RTR
/// 20010711] The acting player first: Decapitate [POT] played, Undead
/// Persistence never offered, the vampire burned.
#[test]
fn decapitate_played_first_bars_undead_persistence() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let a = e.add_minion(SeatId(1), fortitude(0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Decapitate"]);
    hand(&mut e, 1, &["Undead Persistence"]);
    bleed_blocked(&mut e, b, a);
    pick_when(&mut e, play_at(0, 2, Some(b), Target::None));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        play_at(0, 0, Some(a), Target::None),
    );
    assert!(!e.state.minions[a.index()].present(), "burned");
}

/// Undead Persistence: "[for] Cannot be used twice by the same vampire
/// during the same combat: because of the first, the vampire is not going
/// into torpor and does not meet the requirement to play it anymore."
/// [LSJ 20030214] Hit again in a second round, no second copy offered.
#[test]
fn undead_persistence_is_not_played_twice_in_a_combat() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), elder(), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let a = e.add_minion(SeatId(1), fortitude(0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Undead Persistence", "Undead Persistence"]);
    bleed_blocked(&mut e, b, a);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    pick_when(&mut e, Choice::UsePress);
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        play_at(0, 0, Some(a), Target::None),
    );
    assert_eq!(blood(&e, b), 6, "a second round");
    assert!(e.state.minions[a.index()].in_torpor);
}

/// Undead Persistence: "[for] If an effect causes a new combat to occur at
/// the end of combat, then the "go to torpor" effect will be delayed until
/// after that new combat. However, the vampire will no longer benefit from
/// Undead Persistence in the new combat, so it will end immediately with
/// the vampire not ready unless he plays a new Undead Persistence in the
/// new combat." [LSJ 20030214] [ANK 20190624] Psyche! [CEL]'s new combat:
/// over before any strike, or a second copy holding the torpor off.
#[test]
fn undead_persistence_lapses_in_a_new_combat() {
    for second in [false, true] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), vampire_with("CEL", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let a = e.add_minion(SeatId(1), fortitude(0), 0, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let ups: &[&str] = if second {
            &["Undead Persistence", "Undead Persistence"]
        } else {
            &["Undead Persistence"]
        };
        hand(&mut e, 1, ups);
        hand(&mut e, 0, &["Psyche!"]);
        bleed_blocked(&mut e, b, a);
        pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
        pick_when(&mut e, play_at(0, 1, Some(b), Target::None));
        if second {
            pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
            request(&mut e);
            assert!(e.state.minions[a.index()].ready(), "held off again");
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(e.state.minions[a.index()].in_torpor, "{second}");
        if !second {
            assert_eq!(blood(&e, b), 3, "no strike in the new combat");
        }
    }
}

/// Undead Persistence: "[for] The optional press can only be used during
/// the current round." [TOM 19960521] Combat continued by the opponent's
/// press, none for it the next round.
#[test]
fn undead_persistence_press_is_for_its_round_only() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), elder(), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let a = e.add_minion(SeatId(1), fortitude(0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Undead Persistence"]);
    hand(&mut e, 0, &["Boxed In"]);
    bleed_blocked(&mut e, b, a);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    pick_when(&mut e, play_at(0, 0, Some(b), Target::None));
    let mut offers = 0;
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        offers += usize::from(req.options.contains(&Choice::UsePress));
        e.choose(fallback(&req));
    }
    assert_eq!(offers, 1, "to end the first round only");
    assert_eq!(blood(&e, b), 6, "a second round");
}

/// Undead Persistence: "If three rounds of combat pass with no cards
/// played, combat ends." Presses of its own to spare: its round and three
/// with no card, then the combat ends, a press left.
#[test]
fn undead_persistence_combat_ends_after_three_quiet_rounds() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), elder(), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let a = e.add_minion(SeatId(1), fortitude(0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let presses = in_play(&mut e, 1, "Dread Mastiff", Some(a));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostCombatPress(4),
        source: presses,
    });
    hand(&mut e, 1, &["Undead Persistence"]);
    bleed_blocked(&mut e, b, a);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    let mut presses = 0;
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        let i = req.options.iter().position(|o| *o == Choice::UsePress);
        presses += usize::from(i.is_some());
        e.choose(i.unwrap_or_else(|| fallback(&req)));
    }
    assert_eq!(presses, 4, "one each round");
    assert_eq!(blood(&e, b), 8 - 4, "its round and three quiet ones");
    assert!(e.state.minions[a.index()].in_torpor);
}

/// Undead Persistence: "[FOR] Prevent 2 damage." "If fewer points of
/// (preventable) damage are being resolved, then the effect prevents all
/// of those points." [RTR 20041202] "Unused prevention points can't be
/// carried over unless optional." [ANK 20200318] [LSJ 20001114] 1 point
/// all prevented, nothing left for the next round's.
#[test]
fn undead_persistence_superior_prevents_what_there_is() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), elder(), 8, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Undead Persistence", "Boxed In"]);
    bleed_blocked(&mut e, a, b);
    let prevent = play_at(0, 1, Some(a), Target::None);
    pick_when(&mut e, prevent);
    let req = offering(&mut e, play_at(0, 0, Some(a), Target::None));
    assert_eq!(blood(&e, a), 4, "1 point, all prevented");
    e.choose(index_of(&req, play_at(0, 0, Some(a), Target::None)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 3, "the next round's point not prevented");
}

/// Undead Persistence, in Ahriman's Demesne's rulings: "[OBT] The opponent
/// does not go to torpor, Undead Persistence cannot be played." [LSJ
/// 20040608] Burned instead: never offered.
#[test]
fn undead_persistence_is_not_offered_against_ahrimans_demesne() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("OBT", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), fortitude(0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Boxed In", "Fake Out", "Ahriman's Demesne"]);
    hand(&mut e, 1, &["Undead Persistence"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    pick_when(&mut e, play_at(0, 1, Some(a), Target::None));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        play_at(0, 0, Some(b), Target::None),
    );
    assert!(!e.state.minions[b.index()].present(), "burned");
}

/// Undead Persistence: "[FOR] Cannot be used if there is no damage to
/// prevent." [LSJ 20001114] The opposing strike a dodge: never offered.
#[test]
fn undead_persistence_superior_needs_damage_to_prevent() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("FOR", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Undead Persistence"]);
    hand(&mut e, 1, &["Dodge"]);
    bleed_blocked(&mut e, a, b);
    pick_when(&mut e, play_at(0, 0, Some(b), Target::None));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        play_at(0, 1, Some(a), Target::None),
    );
    assert_eq!(blood(&e, a), 4, "no damage");
}

// ---- Rulings ledger ---------------------------------------------------------

/// The vampire `m`'s ability `ability` at `target`.
fn host_ability(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(m),
        ability,
        actor: None,
        target,
    }
}

/// Seat 0's Kine Resources Contested called by `caller` in a 2-seat game
/// (seat 0 taking its 3 points), the polling step reached.
fn kine_called(e: &mut Engine, caller: MinionId) {
    hand(e, 0, &["Kine Resources Contested"]);
    pick(e, play(0, 0, caller));
    pick(e, alloc(0, 3));
}

/// Aura of Invincibility: "If this referendum passes, put this card on the
/// acting vampire". [LSJ 20040730] Its first referendum failing, the card
/// stays in the ash heap and the vampire is not sent to torpor.
#[test]
fn aura_of_invincibility_failing_its_first_referendum_costs_nothing() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), textless_prince(0), 4, false);
    hand(&mut e, 0, &["Ancilla Empowerment", "Aura of Invincibility"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    pick_when(&mut e, votes(p, false));
    close_polling(&mut e);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        on(&e, "Aura of Invincibility", a).is_empty(),
        "not put in play"
    );
    assert!(!e.state.minions[a.index()].in_torpor);
    assert!(e.state.seats[0]
        .ash
        .iter()
        .any(|&x| x == find_library("Aura of Invincibility")));
}

/// Aura of Invincibility: "A vampire can have only one Aura of
/// Invincibility." [LSJ 20040730-2] Not even played to cycle it: a vampire
/// with one is never offered another.
#[test]
fn aura_of_invincibility_is_not_offered_to_a_vampire_with_one() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Aura of Invincibility", Some(a));
    hand(&mut e, 0, &["Ancilla Empowerment", "Aura of Invincibility"]);
    pick(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Secret Horde: "move 2X blood from the blood bank to this card … Burn
/// this card when it has no counters." [LSJ 20030502] X may be 0, never
/// less; [LSJ 20070308] put in play with no counters, it burns at once.
#[test]
fn secret_horde_at_x_zero_burns_at_once() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Secret Horde"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, play_at(0, 0, None, Target::None));
    pick(&mut e, Choice::SetX(0));
    request(&mut e);
    assert!(!e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == find_library("Secret Horde")));
    assert!(e.state.seats[0]
        .ash
        .iter()
        .any(|&x| x == find_library("Secret Horde")));
    assert_eq!(e.state.seats[0].pool, 10);
}

/// Leadership Vacuum's X: [ANK 20211009] [LSJ 20040518-2] "The votes
/// include bonus votes from cards or ability … but not conditional votes
/// (eg. Sundown, Aura of Invincibility)." A prince bearing an Aura with two
/// counters (votes in referendums he calls only), Sundown a prince (his
/// vote bought with blood in a referendum): X their title's votes alone.
#[test]
fn leadership_vacuum_counts_no_conditional_votes() {
    for aura in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let v = if aura {
            e.add_minion(SeatId(1), textless_prince(0), 4, false)
        } else {
            let s = e.add_minion(SeatId(1), find_card("Sundown"), 4, false);
            e.state.minions[s.index()].granted_title = Some(Title::Prince("Paris"));
            s
        };
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let title = e.state.votes_held(v);
        assert_eq!(title, 2, "a prince's");
        if aura {
            let c = in_play(&mut e, 1, "Aura of Invincibility", Some(v));
            e.state.layers.push(zvtes::cards::Layer {
                scope: zvtes::cards::Scope::InPlay,
                modifier: zvtes::cards::Modifier::HostVotesPerCounterWhenCalling,
                source: c,
            });
            e.state.cards[c.index()].counters = 2;
        }
        hand(&mut e, 0, &["Leadership Vacuum"]);
        e.burn(v);
        pick(&mut e, play_at(0, 0, None, Target::Seat(SeatId(1))));
        request(&mut e);
        let lv = e
            .state
            .cards
            .iter()
            .position(|c| c.in_play && c.card == find_library("Leadership Vacuum"))
            .expect("in play");
        assert_eq!(e.state.cards[lv].fixed_x, title, "aura {aura}");
    }
}

/// Peace Treaty: [RTR 20010711] "Can be called even if there are no weapons
/// in play."
#[test]
fn peace_treaty_is_called_with_no_weapon_in_play() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Peace Treaty"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].locked, "its action taken");
}

/// Peace Treaty: [ANK 20190724] "One can decide to burn a weapon with no
/// pool cost (not paying zero)": the Meat Hook (no cost) is offered to
/// keep, and let go it burns.
#[test]
fn peace_treaty_burns_a_free_weapon_not_kept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let hook = in_play(&mut e, 0, "Meat Hook", Some(a));
    hand(&mut e, 0, &["Peace Treaty"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    let req = request(&mut e);
    assert_eq!(req.options, vec![Choice::Pass, Choice::ChooseCard(hook.0)]);
    pick(&mut e, Choice::Pass);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[hook.index()].in_play, "burned");
    assert_eq!(pool(&e)[0], 10);
}

/// Merging changes the sect: [ANK 20180524] "[MERGED] Is Independent / Is
/// Sabbat / Is Anarch. If the base version was the one in play, it changes
/// sect as it is merged. If it had an anarch token, it is burned." Each
/// merging vampire's base card in play (made Anarch by a card: its token),
/// advanced from the crypt by Gift of Experience.
#[test]
fn a_merge_takes_the_advanced_cards_sect() {
    for (name, sect) in [
        ("Theo Bell", Sect::Independent),
        ("Marcus Vitel", Sect::Sabbat),
        ("Sundown", Sect::Anarch),
        ("Brunhilde", Sect::Anarch),
        ("Xaviar", Sect::Camarilla),
        ("Louis Fortier", Sect::Anarch),
        ("Gengis", Sect::Anarch),
        ("Marguerite Foccart", Sect::Anarch),
        ("Menele", Sect::Independent),
    ] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), find_vampire(name, false), 4, false);
        e.state.minions[v.index()].sect_override = Some(Sect::Anarch);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[0]
            .crypt
            .put(find_vampire(name, true), zvtes::copies::CopyId::NONE);
        hand(&mut e, 0, &["Gift of Experience"]);
        pick_when(&mut e, play_at(0, 0, None, Target::Minion(v)));
        request(&mut e);
        let m = &e.state.minions[v.index()];
        assert!(m.merged.is_some(), "{name} merged");
        assert_eq!(m.sect_override, None, "{name}: the token gone");
        assert_eq!(e.state.sect_of(v), Some(sect), "{name}");
    }
}

/// Reformation: [RTR 19980928] "[chi] Cannot be used if there is no
/// equipment to destroy": nothing on the predator's or prey's minions, no
/// [chi]. [ANK 20210226] "The card cannot be played if the minion cannot
/// afford to burn the blood": [dom] with no blood, not offered.
#[test]
fn reformation_needs_an_equipment_or_the_blood() {
    for dom_blood in [0, 1] {
        let mut e = game(2);
        let c = e.add_minion(SeatId(0), vampire_with("chi", 0), 4, false);
        let d = e.add_minion(SeatId(0), vampire_with("dom", 0), dom_blood, false);
        for m in [c, d] {
            e.state.minions[m.index()].sect_override = Some(Sect::Anarch);
        }
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        in_play(&mut e, 1, "Poacher's Hunting Ground", None);
        hand(&mut e, 0, &["Reformation"]);
        let req = to_minion_phase(&mut e, SeatId(0));
        let offered = |mode: u8, m: MinionId| {
            req.options.iter().any(|o| {
                matches!(o, Choice::Play { card: 0, mode: x, actor: Some(y), .. }
                    if *x == mode && *y == m)
            })
        };
        assert!(!offered(0, c), "no equipment: {req:?}");
        assert_eq!(offered(1, d), dom_blood > 0, "[dom] with {dom_blood} blood");
    }
}

/// Guillermo Arsuaga: "A haven played on or by Guillermo costs 1 less blood
/// or pool." [LSJ 20011022] The passive ability applies in torpor too:
/// Secure Haven put on him there costs nothing.
#[test]
fn guillermo_arsuaga_discounts_a_haven_in_torpor() {
    let mut e = game(2);
    let gu = e.add_minion(SeatId(0), find_card("Guillermo Arsuaga"), 4, true);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Secure Haven"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Secure Haven", gu);
    assert_eq!(pool(&e)[0], 10);
    assert_eq!(on(&e, "Secure Haven", gu).len(), 1);
}

/// Matthew Romans: "During your unlock phase, Matthew can burn 1 blood to
/// lock a younger non-Independent, non-Anarch vampire controlled by your
/// prey." [LSJ 20011022] The ability can be used when in torpor.
#[test]
fn matthew_romans_locks_from_torpor() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Matthew Romans"), 3, true);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(
        SeatId(1),
        textless(|c| c.sect == Some(Sect::Camarilla) && c.capacity <= 5, 0),
        4,
        false,
    );
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let u = vampire_use(&req, m, 0).expect("Matthew's ability, in torpor");
    pick(&mut e, u);
    request(&mut e);
    assert!(e.state.minions[t.index()].locked);
    assert_eq!(blood(&e, m), 2);
}

/// Joseph O'Grady: "During a referendum, Joseph may burn 1 blood to gain 1
/// vote." [ANK 20220705] "The ability can be used while locked, only once
/// per referendum."
#[test]
fn joseph_ogrady_buys_a_vote_once_while_locked() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let j = e.add_minion(SeatId(1), find_card("Joseph O'Grady"), 3, false);
    e.state.minions[j.index()].locked = true;
    kine_called(&mut e, caller);
    let vote = host_ability(j, 0, Target::None);
    pick_when(&mut e, vote);
    assert_eq!(blood(&e, j), 2);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, vote);
}

/// Firebrand: "An anarch may have only 1 Firebrand." [LSJ 20100129] [ANK
/// 20191228] A political action whose terms cannot be met is not called:
/// the anarch bearing one is not offered it, another anarch is.
#[test]
fn firebrand_is_not_called_by_an_anarch_bearing_one() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 4, false);
    let b = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Firebrand", Some(a));
    hand(&mut e, 0, &["Firebrand"]);
    let req = to_minion_phase(&mut e, SeatId(0));
    assert!(!req.options.contains(&play(0, 0, a)), "{req:?}");
    assert!(req.options.contains(&play(0, 0, b)), "{req:?}");
}

/// Year of Fortune: "Choose one or more Methuselahs who are not chosen for
/// any other Year of Fortune in play." [LSJ 20100129] [ANK 20191228] Every
/// Methuselah chosen for the one in play, another is not called. "Any
/// vampire can call a referendum to burn this card": [PIB 20121028] burned,
/// the bonus disappears with it.
#[test]
fn year_of_fortune_bonus_and_terms_go_with_the_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(0), textless_prince(0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let size = e.state.hand_size(SeatId(0));
    hand(&mut e, 0, &["Year of Fortune", "Year of Fortune"]);
    pick(&mut e, play(0, 0, a));
    to_window(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    pick(&mut e, Choice::ChooseSeat(SeatId(0)));
    pick(&mut e, Choice::ChooseSeat(SeatId(1)));
    pick_matching(&mut e, |o| *o == Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    assert_eq!(e.state.hand_size(SeatId(0)), size + 1);
    assert_eq!(e.state.hand_size(SeatId(1)), size + 1);
    let yof = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Year of Fortune"))
        .map(|i| CardId(i as u32))
        .expect("in play");
    let req = to_minion_phase(&mut e, SeatId(0));
    assert!(!req.options.contains(&play(0, 0, p)), "none left to choose");
    pick_matching(&mut e, |o| {
        matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), actor: Some(x), .. }
            if *c == yof && *x == p)
    });
    pick_when(&mut e, votes(p, true));
    close_polling(&mut e);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[yof.index()].in_play, "burned");
    assert_eq!(e.state.hand_size(SeatId(0)), size);
    assert_eq!(e.state.hand_size(SeatId(1)), size);
}

/// Sundown (advanced) merged: "If Sundown is ready during the polling step
/// of any referendum, you can discard a political action card to force any
/// vampire to abstain." [LSJ 20100311] Abstaining cancels the votes the
/// vampire cast; [ANK 20220705] "Can use his ability while locked", "only
/// once per referendum"; [LSJ 19971001] a priscus's votes and ballots too.
#[test]
fn sundown_merged_forces_an_abstention_once_while_locked() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    let pr = e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 0), 4, false);
    e.state.minions[pr.index()].granted_title = Some(Title::Priscus);
    let s = e.add_minion(SeatId(1), find_vampire("Sundown", true), 3, false);
    e.state.minions[s.index()].merged = Some(find_card("Sundown"));
    e.state.minions[s.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        1,
        &["Kine Resources Contested", "Kine Resources Contested"],
    );
    kine_called(&mut e, caller);
    let force = |t: MinionId| host_ability(s, 1, Target::Minion(t));
    pick_when(&mut e, votes(caller, true));
    pick_when(&mut e, force(caller));
    request(&mut e);
    let r = e.referendum().expect("referendum");
    assert!(r.abstaining.contains(&caller), "its votes canceled");
    assert_eq!(
        e.state.seats[1].hand.len(),
        1,
        "a political action discarded"
    );
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, force(pr));
}

/// Michael Luther: "You may lock Michael during a referendum to change the
/// votes and ballots of a Camarilla vampire to votes and ballots of your
/// choice." [LSJ 20011022] Usable in torpor; [LSJ 20041207] "only usable
/// once each referendum, even if he unlocks somehow during the
/// referendum."
#[test]
fn michael_luther_turns_votes_from_torpor_once_a_referendum() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), find_card("Tara"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ml = e.add_minion(SeatId(1), find_card("Michael Luther"), 3, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    kine_called(&mut e, caller);
    let his = |o: &Choice| {
        matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(x), .. }
            if *x == ml)
    };
    pick_matching(&mut e, his);
    assert!(e.state.minions[ml.index()].locked, "locked from torpor");
    e.state.minions[ml.index()].locked = false;
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, his);
}

/// Pass every decision until `seat`'s `window`; true if a press ability of
/// `m`'s was offered, asserting it was offered at the press step only.
fn presses_only_in_the_press_step(e: &mut Engine, m: MinionId, seat: SeatId) -> bool {
    let mut seen = false;
    for _ in 0..200 {
        let req = request(e);
        if (req.seat, req.window) == (seat, WindowLabel::MinionPhase) {
            return seen;
        }
        let his = req.options.iter().any(|o| {
            matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(x), .. }
                if *x == m)
        });
        if his {
            assert_eq!(req.window, WindowLabel::CombatPress, "{req:?}");
            seen = true;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    panic!("no end");
}

/// Aeron: "Once each combat, Aeron may burn a blood for a press." [PIB
/// 20150121] "His ability to get a press can only be used during the press
/// step." Velya: "Once each combat, Velya can discard one card requiring
/// Auspex [aus] to get 1 press": [PIB 20150121] the press step only too.
#[test]
fn aeron_and_velya_press_only_in_the_press_step() {
    for name in ["Aeron", "Velya, The Flayer"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card(name), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let b = e.add_minion(SeatId(1), elder(), 8, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Eagle's Sight"]);
        bleed_blocked(&mut e, a, b);
        assert!(
            presses_only_in_the_press_step(&mut e, a, SeatId(0)),
            "{name}"
        );
    }
}

/// Aeron's additional damage is environmental: [RTR 19941109] [TOM
/// 19951216] [PIB 20140324] [ANK 20200422] "Damage is inflicted even
/// against a dodge. A "strike: combat ends" ends combat before damage is
/// applied." The blocker's Dodge: 1 still; its Flash Grenade: none.
#[test]
fn aeron_hurts_through_a_dodge_not_a_combat_end() {
    for grenade in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), find_card("Aeron"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let g = in_play(&mut e, 1, "Flash Grenade", Some(b));
        if grenade {
            bleed_blocked(&mut e, a, b);
            pick_when(&mut e, use_card(g, 0, None, Target::None));
        } else {
            e.state.cards[g.index()].in_play = false;
            hand(&mut e, 1, &["Dodge"]);
            bleed_blocked(&mut e, a, b);
            pick_when(&mut e, play_at(0, 0, Some(b), Target::None));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            blood(&e, b),
            if grenade { 4 } else { 3 },
            "grenade {grenade}"
        );
    }
}

/// Give `m` the Discipline `disc` (a free slot).
fn give(e: &mut Engine, m: MinionId, disc: &'static str) {
    let slots = &mut e.state.minions[m.index()].beast_disciplines;
    let free = slots.iter().position(Option::is_none).expect("a free slot");
    slots[free] = Some(disc);
}

/// Seat 0's locked `a` with Fortitude (and `discs`), a ready vampire beside
/// it, seat 1's Auspex `z` (no Fortitude, an Eagle's Sight in hand) and
/// another; `cards` in seat 0's hand.
fn force_of_will_table(discs: &[&'static str], cards: &[&str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 5, false);
    for d in discs {
        give(&mut e, a, d);
    }
    e.state.minions[a.index()].locked = true;
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, cards);
    hand(&mut e, 1, &["Eagle's Sight"]);
    (e, a, z)
}

/// Force of Will: "After resolution, this vampire takes 2 unpreventable
/// aggravated damage even if the action is blocked." [ANK 20220331] [ANK
/// 20210124] [LSJ 20020927] [LSJ 20021115] [ANK 20221011-2] "The damage is
/// applied if the action reaches resolution, even if it ends before combat
/// (eg. Mirror Walk, Change of Target …)": blocked (Eagle's Sight's
/// intercept), the action ended by Change of Target, or by Mirror Walk
/// [THA]; torpor either way.
#[test]
fn force_of_will_burns_after_an_action_ended_before_combat() {
    for card in ["Change of Target", "Mirror Walk"] {
        let (mut e, a, z) = force_of_will_table(&["THA"], &["Force of Will", card]);
        pick(&mut e, play(0, 0, a));
        pick_when(&mut e, block(z));
        let mode = u8::from(card == "Mirror Walk");
        pick_when(&mut e, play(0, mode, a));
        if mode == 1 {
            pick_when(&mut e, play(0, 0, z));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.combatants.is_none(), "{card}: no combat");
        assert_eq!(e.state.seats[1].pool, 10, "{card}: no bleed");
        assert!(e.state.minions[a.index()].in_torpor, "{card}");
    }
}

/// Force of Will: [ANK 20220401] [ANK 20211015] [ANK 20210124] "The damage
/// is not applied if the action is canceled before resolution (eg. Tangle
/// Atropos' Hand, The Kiss of Ra)": a Brujah's block attempt answered by
/// either, no damage.
#[test]
fn force_of_will_spares_an_action_canceled_before_resolution() {
    for card in ["Tangle Atropos' Hand", "Kiss of Ra, The"] {
        let (mut e, a, z) = force_of_will_table(&["pot"], &["Force of Will", card]);
        pick(&mut e, play(0, 0, a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[a.index()].in_torpor, "{card}");
        assert_eq!(e.state.seats[1].pool, 10, "{card}: no bleed");
    }
}

/// Force of Will: [LSJ 20020927] "Mask of a Thousand Faces cannot be used
/// to take over, since it requires an unlocked vampire and the action
/// requires a locked one": seat 0's unlocked Obfuscate vampire is never
/// offered the Mask on it.
#[test]
fn mask_of_a_thousand_faces_does_not_take_over_force_of_will() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("for", 0), 5, false);
    e.state.minions[a.index()].locked = true;
    let m = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Force of Will", "Mask of a Thousand Faces"]);
    pick(&mut e, play(0, 0, a));
    let masked = |o: &Choice| matches!(o, Choice::Play { card: 0, actor: Some(x), .. } if *x == m);
    let req = request(&mut e);
    assert!(!req.options.iter().any(masked), "{req:?}");
    pick_when(&mut e, block(z));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, masked);
}

/// Force of Will: [LSJ 20010715] "Cards usable after action resolution (eg.
/// Freak Drive) can be played before or after the damage is taken": the
/// damage is one of the after-resolution choices, Freak Drive [for] the
/// other, in either order.
#[test]
fn force_of_will_damage_is_ordered_with_freak_drive() {
    for damage_first in [true, false] {
        let (mut e, a, _) = force_of_will_table(&[], &["Force of Will", "Freak Drive"]);
        pick(&mut e, play(0, 0, a));
        let req = until(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
        assert!(req.options.contains(&Choice::TakeDamageAfterResolution));
        assert!(req.options.contains(&play(0, 0, a)), "Freak Drive");
        if damage_first {
            pick(&mut e, Choice::TakeDamageAfterResolution);
        } else {
            pick(&mut e, play(0, 0, a));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[a.index()].in_torpor, "first {damage_first}");
    }
}

/// Seat 0's Dominate elder with Chain of Command (and `extra` in hand)
/// naming the one young vampire `young` in its uncontrolled region;
/// returns the engine and that vampire once in play.
fn chain_one(young: u16, extra: &[&str]) -> (Engine, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), elder_camarilla(), 6, false);
    e.state.minions[a.index()].beast_disciplines[0] = Some("dom");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].uncontrolled.push(slot(young, 0));
    hand(&mut e, 0, &["Chain of Command"]);
    hand(&mut e, 0, extra);
    fill_hand(&mut e, 0);
    pick(&mut e, play_at(0, 0, Some(a), Target::None));
    pick(&mut e, Choice::Slot(0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let y = in_play_of(&e, young).expect("in play");
    (e, y)
}

/// Chain of Command: [LSJ 20040616] "If a commanded vampire is burned (eg.
/// Daring the Dawn), he is not brought back to the crypt": its bleed with
/// Daring the Dawn blocked by an ally, whose hit takes its last blood, the
/// 2 aggravated damage after combat burn it: the ash heap keeps it.
#[test]
fn chain_of_command_leaves_a_burned_vampire_in_the_ash_heap() {
    let young = textless(|c| c.capacity <= 3 && c.disciplines.contains(&"for"), 0);
    let (mut e, y) = chain_one(young, &["Daring the Dawn"]);
    let ally = e.add_ally(SeatId(1), find_library("47th Street Royals"));
    pick(&mut e, bleed(y));
    pick_when(&mut e, play(0, 0, y));
    pick_when(&mut e, block(ally));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[y.index()].in_play, "burned");
    assert!(e.state.seats[0].crypt.iter().all(|&c| c != young));
    assert!(e.state.seats[0].ash_crypt.contains(&young));
}

/// Chain of Command: "When one of them bleeds successfully, or when all
/// that can bleed have, move them all to the bottom of your crypt." [LSJ
/// 20040518-2] None of them able to bleed any more (its bleed blocked, it
/// is locked), they all go back at once.
#[test]
fn chain_of_command_returns_when_none_can_bleed() {
    let young = textless(|c| c.capacity <= 3, 0);
    let (mut e, y) = chain_one(young, &[]);
    let z = MinionId(2);
    pick(&mut e, bleed(y));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "blocked");
    assert!(!e.state.minions[y.index()].in_play);
    assert_eq!(e.state.seats[0].crypt.first(), Some(&young), "the bottom");
}

/// Galaric's Legacy and Go Anarch: "If this vampire changes sects, burn this
/// card." [LSJ 20100811] Into the Fire and Out of the Frying Pan are a
/// change of the actual sect: Galaric's Legacy burns as Into the Fire makes
/// its vampire Sabbat, which Out of the Frying Pan then makes Camarilla
/// (three trifles in one master phase); Go Anarch burns at the next one's
/// Into the Fire.
#[test]
fn sect_changing_masters_burn_the_sect_cards() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
    let w = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(
        &mut e,
        0,
        &[
            "Galaric's Legacy",
            "Into the Fire",
            "Out of the Frying Pan",
            "Go Anarch",
            "Into the Fire",
        ],
    );
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Galaric's Legacy", v);
    assert_eq!(e.state.sect_of(v), Some(Sect::Anarch));
    assert_eq!(on(&e, "Galaric's Legacy", v).len(), 1);
    master_on(&mut e, "Into the Fire", v);
    assert_eq!(e.state.sect_of(v), Some(Sect::Sabbat));
    assert!(on(&e, "Galaric's Legacy", v).is_empty(), "burned");
    master_on(&mut e, "Out of the Frying Pan", v);
    assert_eq!(e.state.sect_of(v), Some(Sect::Camarilla));
    let go = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Go Anarch"))
        .expect("in hand") as u8;
    pick_when(&mut e, play(go, 0, w));
    to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(on(&e, "Go Anarch", w).len(), 1);
    master_on(&mut e, "Into the Fire", w);
    assert_eq!(e.state.sect_of(w), Some(Sect::Sabbat));
    assert!(on(&e, "Go Anarch", w).is_empty(), "burned");
}

/// Into the Fire, Out of the Frying Pan: [LSJ 20060904] "If the target
/// changes sect to a sect inappropriate for his title, he loses its benefit
/// until his sect is appropriate." A Camarilla prince made Sabbat holds no
/// votes; made Camarilla again, his prince's two.
#[test]
fn a_title_idles_while_its_sect_is_changed() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), textless_prince(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Into the Fire", "Out of the Frying Pan"]);
    assert_eq!(e.state.votes_held(p), 2);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Into the Fire", p);
    assert_eq!(e.state.votes_held(p), 0, "Sabbat: no prince");
    master_on(&mut e, "Out of the Frying Pan", p);
    assert_eq!(e.state.votes_held(p), 2, "Camarilla again");
}

/// Bear-Baiting's table with `cards` added to seat 0's hand and a ready
/// Obfuscate vampire beside the anarch.
fn bear_baiting_more(mine: u8, cards: &[&str]) -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Anarch) && (6..=8).contains(&c.capacity),
            0,
        ),
        4,
        false,
    );
    let m = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    let b = e.add_minion(SeatId(1), elder_camarilla(), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    for (s, cap) in [(0, mine), (1, 12 - mine)] {
        let v = CRYPT
            .iter()
            .position(|c| c.capacity == cap && !c.imbued)
            .expect("a vampire of that capacity") as u16;
        e.state.seats[s].crypt.put(v, zvtes::copies::CopyId::NONE);
    }
    hand(&mut e, 0, &["Bear-Baiting"]);
    hand(&mut e, 0, cards);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    (e, a, m, b)
}

/// Bear-Baiting: [LSJ 20040312] "Change of Target can be played after
/// Bear-Baiting (younger, older, or the same age)": the combat canceled
/// (mine older) or not (mine younger), the action is ended by it.
#[test]
fn change_of_target_follows_bear_baiting() {
    for mine in [9, 3] {
        let (mut e, a, _, b) = bear_baiting_more(mine, &["Change of Target"]);
        pick_when(&mut e, play(0, 0, a));
        to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[a.index()].ready() && !e.state.minions[a.index()].locked);
        assert_eq!(blood(&e, b), 4, "no combat ({mine})");
    }
}

/// Bear-Baiting: [LSJ 20090322] [ANK 20181003] [LSJ 20010803] "If the
/// combat is canceled, the action is still blocked … but an effect can be
/// used to continue the action as if unblocked (eg. Crypt's Sons …)": the
/// combat canceled, the anarch's Crypt's Sons burns a life and the bleed
/// goes on, unblocked.
#[test]
fn crypts_sons_continues_after_bear_baiting_cancels_the_combat() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Anarch) && (6..=8).contains(&c.capacity),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), elder_camarilla(), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    for (s, cap) in [(0, 9), (1, 3)] {
        let v = CRYPT
            .iter()
            .position(|c| c.capacity == cap && !c.imbued)
            .expect("a vampire of that capacity") as u16;
        e.state.seats[s].crypt.put(v, zvtes::copies::CopyId::NONE);
    }
    let sons = in_play(&mut e, 0, "Crypt's Sons", Some(a));
    e.state.cards[sons.index()].counters = 3;
    hand(&mut e, 0, &["Bear-Baiting"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, use_card(sons, 0, None, Target::None));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[sons.index()].counters, 2, "a life burned");
    assert!(
        e.state.seats[1].pool < 10,
        "continued as if unblocked: bled"
    );
    assert_eq!(blood(&e, b), 4, "no combat");
}

/// Drum of Xipe Totec: "The vampire with this equipment has superior
/// Celerity [CEL]. This vampire gets one optional maneuver each combat."
/// [RTR 20010711] "The minion cannot use the discipline nor the maneuver
/// provided if some effect prevents him to use the equipment (eg. Drawing
/// Out the Beast)": the blocker bearing it, under Drawing Out the Beast,
/// has no Celerity and is offered no maneuver from it.
#[test]
fn drum_of_xipe_totec_is_idle_under_drawing_out_the_beast() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ani", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(
        SeatId(1),
        textless(
            |c| c.capacity >= 4 && !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("cel")),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let d = in_play(&mut e, 1, "Drum of Xipe Totec", Some(z));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::BearerSuperiorDiscipline("cel"),
        source: d,
    });
    hand(&mut e, 0, &["Drawing Out the Beast"]);
    hand(&mut e, 1, &["Dodge"]);
    assert_eq!(e.state.discipline_level(z, "cel"), 2, "the Drum's");
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    let mut struck = false;
    loop {
        let req = request(&mut e);
        assert!(
            !req.options.iter().any(|o| matches!(o,
                Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == d)),
            "{req:?}"
        );
        if (req.seat, req.window) == (SeatId(1), WindowLabel::CombatStrike) && !struck {
            assert_eq!(e.state.discipline_level(z, "cel"), 0, "no Celerity");
            struck = true;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    assert!(struck);
}

/// Seat 0's `a` bleeding, blocked by seat 1's `z` bearing a Flash Grenade
/// `g`; `cards` in seat 0's hand.
fn grenade_table(a_card: u16, cards: &[&str]) -> (Engine, MinionId, MinionId, CardId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), a_card, 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let g = in_play(&mut e, 1, "Flash Grenade", Some(z));
    hand(&mut e, 0, cards);
    (e, a, z, g)
}

/// Flash Grenade: [RTR 20041202] [LSJ 20060808] "If the opponent dodges,
/// the "is locked and does not unlock as normal" effect do not apply, but
/// it still burns."
#[test]
fn flash_grenade_dodged_burns_without_its_lock() {
    let (mut e, a, z, g) = grenade_table(plain("Brujah", 0), &["Dodge"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, use_card(g, 0, None, Target::None));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].skip_unlock, "dodged: no lock");
    assert!(!e.state.cards[g.index()].in_play, "still burned");
}

/// Flash Grenade: [LSJ 20001127-2] [LSJ 20010806-1] "Still burns when used
/// if the opponent uses a "strike: combat ends" too. It does not if combat
/// ends before strike resolution (eg. Rötschreck)." Against the acting
/// vampire's own grenade both burn; against an aggravated hand strike
/// (Uncontrollable Rage), seat 1's Rötschreck on its blocker ends the
/// combat first: the grenade stays.
#[test]
fn flash_grenade_burns_unless_the_combat_ended_before_it() {
    let (mut e, a, z, g) = grenade_table(plain("Brujah", 0), &[]);
    let mine = in_play(&mut e, 0, "Flash Grenade", Some(a));
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, use_card(mine, 0, None, Target::None));
    pick_when(&mut e, use_card(g, 0, None, Target::None));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[mine.index()].in_play && !e.state.cards[g.index()].in_play);

    let (mut e, a, z, g) = grenade_table(vampire_with("pro", 0), &["Uncontrollable Rage"]);
    hand(&mut e, 1, &["Rötschreck"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    pick_when(&mut e, block(z));
    // Seat 0's hand strike (the fallback), then seat 1's grenade.
    pick_when(&mut e, use_card(g, 0, None, Target::None));
    pick_when(&mut e, play_at(0, 0, None, Target::Minion(z)));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor, "Rötschreck");
    assert!(e.state.cards[g.index()].in_play, "not used: not burned");
}

/// Skin of Night: "[for] This vampire treats aggravated damage as normal
/// damage for the remainder of this round." [LSJ 20040812-2] [PIB 20130327]
/// A prevention of non-aggravated damage "Cannot be used to prevent
/// aggravated damage, even if the minion treats them as normal damage (eg.
/// Skin of Night)": the blocker's Soak is never offered against the
/// aggravated hand strike; the damage mended as normal.
#[test]
fn skin_of_night_leaves_no_soak_against_aggravated_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pro", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("for", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Uncontrollable Rage"]);
    hand(&mut e, 1, &["Skin of Night", "Soak"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, b));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(x), .. } if *x == b),
    );
    let v = &e.state.minions[b.index()];
    assert!(!v.in_torpor, "mended as normal damage");
}

/// Mukhtar Bey: "Mukhtar cannot use presses to continue combat." [LSJ
/// 20040521] Frenzy "Can be played on a minion that cannot play or use
/// presses (eg. Mukhtar Bey) and does provide the presses, the minion is not
/// the one pressing"; "He can play Trap and it provides presses": Frenzy on
/// him, a second round; his Trap, four.
#[test]
fn mukhtar_bey_gets_the_presses_of_frenzy_and_trap() {
    for (card, rounds) in [("Frenzy", 2), ("Trap", 4)] {
        let mut e = game(2);
        let mk = e.add_minion(SeatId(0), find_card("Mukhtar Bey"), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &[card]);
        bleed_blocked(&mut e, mk, z);
        let target = if card == "Frenzy" {
            Target::Minion(mk)
        } else {
            Target::None
        };
        let actor = (card == "Trap").then_some(mk);
        pick_when(&mut e, play_at(0, 0, actor, target));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), 6 - rounds, "{card}: {rounds} rounds");
    }
}

/// Frenzy: "This round has a press, only usable to continue combat." [ANK
/// 20200525-1] "The provided press is applied before any other press.
/// Another press cannot be used to continue (eg. Boxed In …) if there's
/// already a press to continue": Boxed In is not offered in the Frenzy's
/// round, and is in the next.
#[test]
fn frenzys_press_leaves_boxed_in_for_the_next_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Frenzy", "Boxed In"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play_at(0, 0, None, Target::Minion(a)));
    let boxed = play(0, 0, a);
    let mut offered_in = Vec::new();
    loop {
        let req = request(&mut e);
        if req.options.contains(&boxed) {
            offered_in.push(e.combat().expect("combat").round);
            e.choose(index_of(&req, boxed));
            continue;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = fallback(&req);
        e.choose(i);
    }
    assert_eq!(offered_in, vec![2]);
}

/// Gambit Accepted: "If you are ousted, your predator gains an additional
/// victory point." [RTR 20040501] "If the controller of a Gambit in play
/// withdraws, his or her predator gets a VP from the Gambit."
#[test]
fn gambit_accepted_pays_the_predator_on_a_withdrawal() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 3), 4, false);
    for s in 1..3 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    let g = in_play(&mut e, 1, "Gambit Accepted", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::PredatorVpIfOusted(1),
        source: g,
    });
    let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    e.choose(index_of(&req, Choice::WithdrawFromGame));
    // Out at its next unlock phase (rulebook "Withdrawing from the Game").
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].withdrew, "withdrawn");
    assert_eq!(e.state.seats[0].half_vps, 2, "the Gambit's");
}

/// Ghouled: [LSJ 20030520-2] "The ally is recruited as a mortal and changed
/// to a ghoul afterwards (eg. Sunset Strip, Hollywood can be used to reduce
/// its cost). His "starting life" is still the base mortal amount." The
/// Neighborhood Watch Commander (2 blood) for 1.
#[test]
fn ghouled_recruit_is_a_mortal_for_sunset_strip() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let strip = in_play(&mut e, 0, "Sunset Strip, Hollywood", None);
    hand(&mut e, 0, &["Neighborhood Watch Commander", "Ghouled"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, use_card(strip, 1, None, Target::None));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[strip.index()].locked);
    assert_eq!(blood(&e, a), 4 - (2 - 1) - 1, "2 blood less 1, Ghouled's 1");
    let ally = e
        .state
        .minions
        .iter()
        .position(|m| m.in_play && m.is_ally())
        .map(|i| MinionId(i as u32))
        .expect("recruited");
    assert_eq!(on(&e, "Ghouled", ally).len(), 1);
}

/// Repo Man: "Unique ghoul". [ANK 20200203-2] [LSJ 20060515] "Ghouls are
/// monsters, not mortal": Sunset Strip, Hollywood does not reduce his
/// recruit (mortal allies only).
#[test]
fn repo_man_is_no_mortal_for_sunset_strip() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let strip = in_play(&mut e, 0, "Sunset Strip, Hollywood", None);
    hand(&mut e, 0, &["Repo Man"]);
    pick(&mut e, play(0, 0, a));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == Choice::ResolveRider { card: strip.0 } || *o == use_card(strip, 1, None, Target::None)
    });
    assert_eq!(blood(&e, a), 4 - 1);
}

/// Improvised Tactics: "[aus] During the press step each round, draw one
/// card." [LSJ 20010312] [LSJ 20001027] "The effect in the press step does
/// not happen if the combat ends prematurely (e.g. from torpor or "strike:
/// combat ends")": the blocker's Flash Grenade ends it, nothing drawn.
#[test]
fn improvised_tactics_draws_nothing_when_the_combat_ends_early() {
    let (mut e, _, _) = improvised_table(0);
    let z = MinionId(2);
    let g = in_play(&mut e, 1, "Flash Grenade", Some(z));
    hand(&mut e, 0, &["Villein"; 7]);
    e.state.seats[0].library.push(find_library("Villein"));
    pick_when(&mut e, use_card(g, 0, None, Target::None));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].library.len(), 1, "no draw");
}

/// [LSJ 20060620] "You can draw more cards than are in the library or crypt
/// (even from an empty library or crypt); draw as much as you can." Beckett
/// draws 3 from a library of 1, Servitor of Irad 2 from a library of 1,
/// Improvised Tactics [aus] 1 from an empty one.
#[test]
fn draws_take_what_the_library_has() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Beckett"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].library.push(find_library("Form of Mist"));
    hand(&mut e, 0, &["Villein"; 3]);
    let req = to_minion_phase(&mut e, SeatId(0));
    let act = vampire_use(&req, b, 0).expect("Beckett's action");
    pick(&mut e, act);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].library.is_empty());
    assert_eq!(e.state.seats[0].hand.len(), 4, "the one there was");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Servitor of Irad", Some(a));
    e.state.seats[0].library.push(find_library("Form of Mist"));
    play_gehenna(&mut e);
    assert!(e.state.seats[0].library.is_empty(), "the one there was");

    let (mut e, _, _) = improvised_table(0);
    hand(&mut e, 0, &["Villein"; 6]);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].hand.len(), 6, "none to draw");
}

/// Loner: "if this vampire successfully performed an action this turn and
/// none of your other minions attempted any actions this turn". [LSJ
/// 20060616-1] "Only considers the minions controlled during the influence
/// phase": the ally that bled first was burned in the block's combat, so
/// the Loner, bleeding after, gains.
#[test]
fn loner_ignores_a_minion_no_longer_controlled() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), plain("Brujah", 0), 2, false);
    let o = e.add_ally(SeatId(0), find_library("Neighborhood Watch Commander"));
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = in_play(&mut e, 0, "Loner", Some(l));
    bleed_blocked(&mut e, o, z);
    pick_when(&mut e, bleed(l));
    assert!(!e.state.minions[o.index()].present(), "burned");
    assert!(offered_before(
        &mut e,
        ability(c.0, 0),
        SeatId(1),
        WindowLabel::MinionPhase
    ));
}

/// Mirror Walk: [RTR 20180719] [ANK 20200207] [ANK 20210627] "[THA] If the
/// action is blocked, it ends unsuccessfully immediately, no other action
/// modifier or reaction can be played afterwards (eg. … Sowing
/// Dissension)": blocked by an untitled vampire (Eagle's Sight's
/// intercept), the action ended by Mirror Walk [THA], Sowing Dissension's
/// way out is not offered.
#[test]
fn mirror_walk_leaves_no_sowing_dissension_escape() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Anarch) && (6..=8).contains(&c.capacity),
            0,
        ),
        4,
        false,
    );
    give(&mut e, a, "THA");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Sowing Dissension", "Mirror Walk"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, b));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::CancelCombat,
    );
    assert!(e.state.minions[b.index()].locked, "the blocker locked");
    assert!(e.state.minions[a.index()].locked, "no escape: still locked");
}

/// Principia Discordia: [RTR 20010711] "[qui] can target a vampire with no
/// blood": locked, burning none. [ANK 20210124] [LSJ 20070411] [LSJ
/// 20090514] "If the action is unblocked when it resolves and the target is
/// locked, the action "fizzles"": the target locks itself first (its
/// Elder Michaelis's Hold, 1 blood), no blood burned by the action.
#[test]
fn principia_discordia_quietus_at_an_empty_or_locked_vampire() {
    for fizzle in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("qui", 0), 4, false);
        e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let t = e.add_minion(SeatId(1), plain("Ventrue", 0), u8::from(fizzle) * 2, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        e.state.seats[0]
            .ash
            .put(find_library("Conditioning"), zvtes::copies::CopyId::NONE);
        hand(&mut e, 0, &["Principia Discordia"]);
        if fizzle {
            hand(&mut e, 1, &["Elder Michaelis's Hold"]);
        }
        pick_when(&mut e, play_at(0, 1, Some(a), Target::Minion(t)));
        if fizzle {
            pick_when(&mut e, play(0, 0, t));
        }
        to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[t.index()];
        assert!(v.locked, "fizzle {fizzle}");
        // Fizzled: 2 less the Hold's 1 blood only.
        assert_eq!(v.blood, u8::from(fizzle), "fizzle {fizzle}");
    }
}

/// Principia Discordia: [LSJ 20090324] "[aus] is directed against the
/// Methuselah controlling the equipment, not the minion it's on".
#[test]
fn principia_discordia_auspex_is_directed_at_the_equipments_controller() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("aus", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(t));
    hand(&mut e, 0, &["Principia Discordia"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Card(gun)));
    request(&mut e);
    let act = e.action().expect("the action");
    assert_eq!(act.directed_seat(&e.state), Some(SeatId(1)));
}

/// Improvised Tactics: [LSJ 20030611] "[pot] Is a strike that requires
/// Potence, so would be enhanced by Increased Strength": its 2R strike at
/// long range (Fake Out's maneuver) inflicts 3 after Increased Strength
/// [pot].
#[test]
fn improvised_tactics_potence_strike_takes_increased_strength() {
    let (mut e, a, it) = improvised_table(1);
    give(&mut e, a, "pot");
    let z = MinionId(2);
    hand(&mut e, 0, &["Increased Strength", "Fake Out"]);
    fill_hand(&mut e, 0);
    pick_when(&mut e, play(0, 0, a));
    let req = to_window(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    let fake = *req
        .options
        .iter()
        .find(|o| matches!(o, Choice::Play { card: 0, .. }))
        .expect("Fake Out");
    pick(&mut e, fake);
    pick_when(&mut e, use_card(it, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 3);
}

/// The action under way, at seat 1's block window, gets `f`'s change;
/// then on to seat 1's minion phase.
fn mid_action(e: &mut Engine, f: impl FnOnce(&mut Engine)) {
    expect(e, SeatId(1), WindowLabel::ActionBlock);
    f(e);
    until(e, SeatId(1), WindowLabel::MinionPhase);
}

/// Actions whose target or actor no longer qualifies by resolution "fizzle"
/// (successful, no effect, not paid: ANK 20210124, rules-qa 17.4, against
/// the database's "cost, if any, is paid"): Karsh's combat at a target
/// unlocked by then ([ANK 20210124] [LSJ 20070411] [LSJ 20090514]: "If the
/// action is unblocked when it resolves and the target is unlocked"); Root
/// of Vitality's rescue of a vampire out of torpor by then ("and the target
/// is no longer in torpor"); Sleep of Reason at a target with two blood by
/// then ([LSJ 20090514] [LSJ 20070411]: "If the action is not blocked and
/// the target has 2 or more blood, the action "fizzles""); Rapid Healing
/// and Movement of the Slow Body by a vampire out of torpor by then ([ANK
/// 20220218]: "When the acting vampire leaves torpor before the action
/// resolves (e.g. Warsaw Station) the action immediately "fizzles"").
#[test]
fn actions_fizzle_on_what_changed_by_resolution() {
    use zvtes::actions::Outcome;
    // Karsh.
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Karsh"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let s = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[s.index()].locked = true;
    pick_matching(&mut e, |o| {
        matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(x), target: Target::Minion(t), .. }
            if *x == k && *t == s)
    });
    mid_action(&mut e, |e| e.state.minions[s.index()].locked = false);
    assert_eq!(e.last_outcome, Some(Outcome::Fizzled), "Karsh");
    assert_eq!((blood(&e, k), blood(&e, s)), (6, 4), "no combat");

    // Root of Vitality.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("vic", 0), 2, false);
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Root of Vitality"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(t)));
    mid_action(&mut e, |e| e.state.minions[t.index()].in_torpor = false);
    assert_eq!(e.last_outcome, Some(Outcome::Fizzled), "Root of Vitality");

    // Sleep of Reason.
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.capacity >= 6 && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("dem")),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), textless(|c| c.capacity <= 2, 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Sleep of Reason"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(t)));
    mid_action(&mut e, |e| e.state.minions[t.index()].blood = 2);
    assert_eq!(e.last_outcome, Some(Outcome::Fizzled), "Sleep of Reason");
    assert!(!e.state.minions[t.index()].in_torpor);
    assert_eq!(blood(&e, a), 4, "not paid");

    // Rapid Healing [FOR], Movement of the Slow Body [pro].
    for (card, disc, mode) in [
        ("Rapid Healing", "FOR", 1),
        ("Movement of the Slow Body", "pro", 0),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with(disc, 0), 1, true);
        e.state.minions[a.index()].locked = mode == 0;
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &[card]);
        pick_when(&mut e, play(0, mode, a));
        mid_action(&mut e, |e| e.state.minions[a.index()].in_torpor = false);
        assert_eq!(e.last_outcome, Some(Outcome::Fizzled), "{card}");
        assert_eq!(blood(&e, a), 1, "{card}: no blood gained");
    }
}

/// Amenophobis: "You can draw 1 card … after resolution of any successful
/// action requiring Serpentis [ser]." [ANK 20220218] "The after action
/// resolution effect can be used if the action "fizzles"": Principia
/// Discordia [ser] at a vampire gone to torpor by resolution.
#[test]
fn amenophobis_draws_after_a_fizzled_serpentis_action() {
    let mut e = game(2);
    let am = e.add_minion(SeatId(0), find_card("Amenophobis"), 4, false);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Principia Discordia"]);
    e.state.seats[0].library.push(find_library("Villein"));
    pick_when(&mut e, play_at(0, 2, Some(a), Target::Minion(t)));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.state.minions[t.index()].in_torpor = true;
    pick_when(&mut e, host_ability(am, 0, Target::None));
    request(&mut e);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert!(e.state.seats[0].library.is_empty(), "drawn");
}

/// Amaranth: "Only usable when the opposing vampire should go to torpor."
/// [RTR 20010711] "Sequencing rules apply, and if played first, effects
/// that stop a vampire from going to torpor would prevent the use of
/// effects that are only usable on a vampire going to torpor": Amaranth
/// played first, the blocker's Undead Persistence is not offered.
#[test]
fn amaranth_played_first_bars_undead_persistence() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let a = e.add_minion(SeatId(1), fortitude(0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Amaranth"]);
    hand(&mut e, 1, &["Undead Persistence"]);
    bleed_blocked(&mut e, b, a);
    pick_when(&mut e, play_at(0, 0, Some(b), Target::None));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        play_at(0, 0, Some(a), Target::None),
    );
    assert!(!e.state.minions[a.index()].present(), "diablerized");
}

/// Smoke and Mirrors: "[chi] +1 stealth, with 1 optional maneuver during
/// the resulting combat if this action is blocked." [LSJ 20010813] [LSJ
/// 20010819-2] "The maneuver can only be used if the block is successful,
/// and only in the resulting (block-induced) combat. It does not carry over
/// to a follow-up combat (eg. [CEL] Psyche!)."
#[test]
fn smoke_and_mirrors_maneuver_is_not_carried_into_psyches_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("chi", 0), 6, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    give(&mut e, a, "CEL");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Smoke and Mirrors", "Psyche!"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, z));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(
        req.options.contains(&Choice::UseManeuver),
        "the block's combat"
    );
    pick_when(&mut e, play(0, 1, a));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == Choice::UseManeuver
    });
    assert!(e.state.seats[0].ash.contains(&find_library("Psyche!")));
}

/// Songs of the Distant Vitae [QUI]: "+1 bleed against the controller of any
/// vampire who has successfully hunted since your last turn." [LSJ
/// 20050720-2] "A hunt is not successful if no blood is gained, even if the
/// hunt action is still successful": seat 1's diseased vampire gains none,
/// so no +1; one at its capacity gains it, drained off (rules-qa 89.1,
/// 89.26), and one below it: +1.
#[test]
fn songs_of_the_distant_vitae_counts_no_hunt_gaining_nothing() {
    for (case, more) in [("diseased", 0), ("full", 1), ("below", 1)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("QUI", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let hv = plain("Ventrue", 0);
        let cap = CRYPT[hv as usize].capacity;
        let h = e.add_minion(SeatId(1), hv, if case == "full" { cap } else { 1 }, false);
        e.state.minions[h.index()].disease = case == "diseased";
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Songs of the Distant Vitae"]);
        to_minion_phase(&mut e, SeatId(1));
        pick(&mut e, action(h, zvtes::decision::ActionKind::Hunt));
        to_minion_phase(&mut e, SeatId(0));
        let pool = e.state.seats[1].pool;
        pick_when(&mut e, play(0, 1, a));
        to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, pool - 1 - more, "{case}");
    }
}

/// Sword of Nuln: "Vampires must spend twice the normal amount of blood to
/// heal the damage from this weapon's strikes." [PIB 20130415] "If the
/// wielder uses Taste of Vitae, he gains as much blood as the opponent has
/// lost": struck for 1, the blocker spends 2; the wielder's Taste of Vitae
/// gains 2.
#[test]
fn sword_of_nuln_feeds_taste_of_vitae_the_doubled_loss() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Tremere", 0), 3, false);
    e.add_minion(SeatId(0), plain("Tremere", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let sword = in_play(&mut e, 0, "Sword of Nuln", Some(a));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::MendingCost(2),
        source: sword,
    });
    hand(&mut e, 0, &["Taste of Vitae"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, use_card(sword, 0, None, Target::None));
    pick_when(&mut e, play(0, 0, a));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 2, "the mending doubled");
    let cap = e.state.capacity_of(a).expect("a vampire");
    assert_eq!(blood(&e, a), (3 - 1 + 2).min(cap), "1 mended, 2 tasted");
}

/// The Veil of Darkness layer of seat 1's (fixture), `top` on seat 0's
/// library, notes kept.
fn veil_over(e: &mut Engine, top: &str) {
    let veil = in_play(e, 1, "Veil of Darkness", None);
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::FirstDisciplineCardRevealsTop {
            disc: "obt",
            burn: 2,
        },
        source: veil,
    });
    e.state.seats[0].library.push(find_library(top));
    e.notes = Some(Vec::new());
}

/// Veil of Darkness: "when a vampire plays his or her first card that turn
/// that requires any Disciplines, that vampire's controller reveals the top
/// card". [LSJ 20050606] "Does not affect allies who play cards requiring a
/// Discipline "as a vampire"": Alia's Revelations [aus], nothing revealed,
/// the action taken. Karen Suadela: [LSJ 20100121] [LSJ 20081120-2] "Her
/// controller doesn't reveal cards due to Veil of Darkness when she plays
/// cards requiring disciplines": her Undead Strength [POT] lands its +2.
#[test]
fn veil_of_darkness_reveals_nothing_for_alia_or_karen() {
    let mut e = game(2);
    let alia = e.add_ally(SeatId(0), find_library("Alia, God's Messenger"));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Revelations"]);
    veil_over(&mut e, "Minion Tap");
    pick_matching(
        &mut e,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == alia),
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(noted_reveals(&mut e).len(), 0, "Alia");
    assert!(e.state.seats[0].ash.contains(&find_library("Revelations")));

    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Karen Suadela"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Undead Strength"]);
    veil_over(&mut e, "Minion Tap");
    bleed_blocked(&mut e, k, z);
    pick_when(&mut e, play(0, 2, k));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(noted_reveals(&mut e).len(), 0, "Karen");
    assert_eq!(blood(&e, z), 6 - 3, "1 strength + 2");
}

/// Veil of Darkness: [LSJ 20100728] [LSJ 20090213] "The effect needs to be
/// applied before anyone has an opportunity to play another cancellation
/// effect (eg. Direct Intervention)": the first Discipline card voided by
/// a master on top, seat 1's Direct Intervention is never offered on it;
/// with no master there, it is.
#[test]
fn veil_of_darkness_voids_before_direct_intervention() {
    for master in [true, false] {
        let (mut e, v) = veiled(if master { "Minion Tap" } else { "Form of Mist" });
        hand(&mut e, 1, &["Direct Intervention"]);
        pick(&mut e, play(0, 0, v));
        let offered = offered_before(
            &mut e,
            play_at(0, 0, None, Target::None),
            SeatId(0),
            WindowLabel::MinionPhase,
        );
        assert_eq!(offered, !master, "master {master}");
    }
}

/// Lucita (advanced): "During an action Lucita performs, she can burn 2 blood
/// to cancel a reaction card as it is played by a vampire with capacity 5 or
/// less". [LSJ 20011022] "The ability can be used when in torpor": her
/// leave-torpor action, the blocker's Spirit's Touch canceled.
#[test]
fn lucita_cancels_a_reaction_from_torpor() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_vampire("Lucita", true), 4, true);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r = e.add_minion(
        SeatId(1),
        textless(|c| c.capacity <= 5 && c.disciplines.contains(&"aus"), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Spirit's Touch"]);
    e.state.minions[l.index()].timed.push((
        zvtes::cards::Lasting::Stealth(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    pick_when(&mut e, action(l, zvtes::decision::ActionKind::LeaveTorpor));
    pick_when(&mut e, block(r));
    pick_when(&mut e, play_at(0, 0, Some(r), Target::None));
    let req = request(&mut e);
    let u = vampire_use(&req, l, 0).expect("Lucita's cancel, in torpor");
    pick(&mut e, u);
    request(&mut e);
    assert_eq!(blood(&e, l), 2);
}

/// "Requires a baron": [LSJ 20080603] [LSJ 20050128] "Counts as requiring an
/// Anarch (for eg. Powerbase: Los Angeles)": Emissary and Sweeper discarded
/// with the Powerbase's extra discard phase action unlock a ready Anarch.
#[test]
fn baron_cards_count_as_requiring_an_anarch() {
    for card in ["Emissary", "Sweeper"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_sect(Sect::Anarch, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Soak", card]);
        let la = in_play(&mut e, 0, "Powerbase: Los Angeles", None);
        e.state.minions[a.index()].locked = true;
        until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
        pick(&mut e, use_card(la, 0, None, Target::None));
        pick(&mut e, Choice::Discard(0));
        pick(&mut e, Choice::Discard(0));
        pick(&mut e, Choice::ChooseMinion(a));
        request(&mut e);
        assert!(!e.state.minions[a.index()].locked, "{card}");
    }
}

/// The Holds: [LSJ 20021128] "If it leaves play, the card on it is
/// burned": Chiram's Hold and Elder Michaelis's Hold burned for a discarded
/// copy, Regarhagan's Hold at its second pool counter; the card on each to
/// its owner's ash heap.
#[test]
fn a_hold_leaving_play_burns_its_card() {
    for (name, stored) in [
        ("Chiram's Hold", "Dodge"),
        ("Elder Michaelis's Hold", "Conditioning"),
        ("Regarhagan's Hold", "Wake with Evening's Freshness"),
    ] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let h = hold_fixture(&mut e, name, stored);
        if name == "Regarhagan's Hold" {
            e.state.cards[h.index()].counters = 1;
        } else {
            hand(&mut e, 0, &[stored, "Villein"]);
        }
        pick_when(&mut e, use_card(h, 0, None, Target::None));
        request(&mut e);
        assert!(!e.state.cards[h.index()].in_play, "{name} burned");
        assert!(
            e.state.seats[0].ash.contains(&find_library(stored)),
            "{name}: its card burned"
        );
    }
}

/// Zayyat: "Zayyat can move up to 3 blood from himself to a younger vampire
/// in your uncontrolled region as a +1 stealth action." The amount named as
/// it is declared (D1, rules-qa 111.51), one action a count; 2 moved, the
/// older not offered (its resolution once panicked: a granted action's
/// unhandled effect); with 2 blood, no count of 3.
#[test]
fn zayyat_moves_blood_to_a_younger_uncontrolled_vampire() {
    let mut e = game(2);
    let zy = e.add_minion(SeatId(0), find_card("Zayyat, The Sandstorm"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let young = textless(|c| c.capacity <= 3, 0);
    let old = CRYPT
        .iter()
        .position(|c| c.capacity >= CRYPT[find_card("Zayyat, The Sandstorm") as usize].capacity)
        .expect("not younger") as u16;
    let u = &mut e.state.seats[0].uncontrolled;
    u.push(slot(young, 0));
    u.push(slot(old, 0));
    let req = request(&mut e);
    let moves: Vec<Choice> = req
        .options
        .iter()
        .copied()
        .filter(|o| {
            matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(x), ability: 0..=2, .. } if *x == zy)
        })
        .collect();
    assert_eq!(moves.len(), 3, "one a count, the younger only: {moves:?}");
    let two = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(zy),
        ability: 1,
        actor: Some(zy),
        target: Target::Uncontrolled(0),
    };
    assert!(moves.contains(&two), "{moves:?}");
    pick(&mut e, two);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, zy), 6 - 2);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 2);

    let mut e = game(2);
    let zy = e.add_minion(SeatId(0), find_card("Zayyat, The Sandstorm"), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].uncontrolled.push(slot(young, 0));
    let req = request(&mut e);
    let count = |ability| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(zy),
        ability,
        actor: Some(zy),
        target: Target::Uncontrolled(0),
    };
    assert!(req.options.contains(&count(1)), "{:?}", req.options);
    assert!(!req.options.contains(&count(2)), "3 above his 2 blood");
}

/// Zayyat: the amount named as declared, what he has of it moved as it
/// resolves (rules-qa 111.51): his count of 3 declared, he is down to 1
/// blood by resolution, 1 moves.
#[test]
fn zayyat_moves_what_he_has_as_it_resolves() {
    let mut e = game(2);
    let zy = e.add_minion(SeatId(0), find_card("Zayyat, The Sandstorm"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(slot(textless(|c| c.capacity <= 3, 0), 0));
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Vampire(zy),
            ability: 2,
            actor: Some(zy),
            target: Target::Uncontrolled(0),
        },
    );
    e.state.minions[zy.index()].blood = 1;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, zy), 0);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 1);
}

/// Edward Neally: "Edward can look at your prey's hand as a +1 stealth Ⓓ
/// action that costs 1 blood." Taken, its blood paid and the prey's hand
/// seen (its resolution once panicked: a granted action's unhandled
/// effect).
#[test]
fn edward_neally_looks_at_his_preys_hand() {
    let mut e = game(2);
    let ed = e.add_minion(SeatId(0), find_card("Edward Neally"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Blood Doll"]);
    e.notes = Some(Vec::new());
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Vampire(ed),
            ability: 1,
            actor: Some(ed),
            target: Target::Seat(SeatId(1)),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ed), 4 - 1);
    let looks = noted_looks(&mut e);
    assert!(
        looks.iter().any(|l| l.viewer == SeatId(0)
            && l.seat == SeatId(1)
            && !l.library
            && l.cards == vec![find_library("Blood Doll")]),
        "{looks:?}"
    );
}

/// Cristos Mantigo: "Cristos can search your library for an equipment card
/// and move it to your hand as a +1 stealth action. Discard down to your
/// hand size and shuffle afterward." The equipment found, not revealed, the
/// hand discarded down (its resolution once panicked: a granted action's
/// unhandled effect).
#[test]
fn cristos_mantigo_searches_for_an_equipment() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Cristos Mantigo"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let magnum = find_library(".44 Magnum");
    e.state.seats[0].library = vec![find_library("Blood Doll"), magnum].into();
    fill_hand(&mut e, 0);
    let size = e.state.seats[0].hand.len();
    e.notes = Some(Vec::new());
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Vampire(c),
            ability: 0,
            actor: Some(c),
            target: Target::None,
        },
    );
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    assert_eq!(req.options, vec![Choice::Pass, Choice::FindCard(magnum)]);
    pick(&mut e, Choice::FindCard(magnum));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].library.len(), 1);
    assert_eq!(e.state.seats[0].hand.len(), size, "discarded down");
    assert_eq!(e.state.seats[0].ash.len(), 1);
    assert_eq!(noted_reveals(&mut e), Vec::new());
}

/// Highway Haven: RV: "Any werewolf may burn this equipment and enter
/// combat with this minion as a +1 stealth Ⓓ action." Successful, the RV
/// burns and the werewolf fights its former bearer (its resolution once
/// panicked: a granted action's unhandled effect).
#[test]
fn highway_haven_rv_is_burned_by_a_werewolf() {
    let mut e = game(2);
    let w = e.add_ally(SeatId(0), find_library("Double Deuce"));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let rv = in_play(&mut e, 1, "Highway Haven: RV", Some(z));
    pick(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(rv),
            ability: 0,
            actor: Some(w),
            target: Target::Minion(z),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[rv.index()].in_play);
    assert_eq!(
        e.state.seats[1].ash,
        vec![find_library("Highway Haven: RV")]
    );
    assert!(blood(&e, z) < 4, "they fought");
}

/// Zayyat: "Once each turn, he can burn 2 blood to get +2 stealth for the
/// current action." [ANK 20180719-4] "Once each turn" means once during
/// each player's turn: used on his bleed, not on his hunt after it that
/// turn (unlocked).
#[test]
fn zayyat_buys_stealth_once_each_turn() {
    let mut e = game(2);
    let zy = e.add_minion(SeatId(0), find_card("Zayyat, The Sandstorm"), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let stealth = |o: &Choice| {
        matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(x), ability: 3, .. }
            if *x == zy)
    };
    pick_when(&mut e, bleed(zy));
    pick_when(&mut e, block(y));
    pick_matching(&mut e, stealth);
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, zy), 8 - 2);
    e.state.minions[zy.index()].locked = false;
    pick(&mut e, action(zy, zvtes::decision::ActionKind::Hunt));
    pick_when(&mut e, block(w));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, stealth);
}

/// Effects failing an action: [ANK 20260316] [LSJ 20041022] "Cannot be used
/// while a block is being attempted, or has already been successful": Body
/// Bag's burn and The Mole [dom] are not offered once seat 1's other
/// vampire attempts the block (successful: Eagle's Sight against Principia
/// Discordia's stealth).
#[test]
fn action_failing_effects_wait_for_no_block_attempt() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[t.index()].sect_override = Some(Sect::Anarch);
    let y = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    let bag = in_play(&mut e, 1, "Body Bag", Some(t));
    hand(&mut e, 0, &["Principia Discordia"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    pick(&mut e, play_at(0, 2, Some(a), Target::Minion(t)));
    pick_when(&mut e, block(y));
    // The attempt under way, then successful (Eagle's Sight against the
    // action's +1 stealth): no Body Bag throughout.
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&ability(bag.0, 0)), "{req:?}");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = req
            .options
            .iter()
            .position(|o| *o == play(0, 0, y))
            .unwrap_or_else(|| fallback(&req));
        e.choose(i);
    }
    assert!(e.state.minions[y.index()].locked, "blocked");

    let mut e = game(2);
    let b = textless(|c| c.sect == Some(Sect::Camarilla) && c.capacity <= 4, 0);
    let a = e.add_minion(SeatId(0), b, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r = e.add_minion(
        SeatId(1),
        textless(
            |c| {
                c.sect == Some(Sect::Anarch)
                    && c.capacity >= 6
                    && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("dom"))
            },
            0,
        ),
        4,
        false,
    );
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Mole, The"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(y));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play_at(0, 2, Some(r), Target::None),
    );
}

/// Meat Hook: "Strike: strength+1 damage, only usable once each round."
/// [ANK 20230316] "A second copy allows a second use in the same round":
/// after the first hook's strike, Blur's additional strike is offered with
/// the second hook, not the first.
#[test]
fn a_second_meat_hook_strikes_again_in_the_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("cel", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), elder(), 8, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let h1 = in_play(&mut e, 0, "Meat Hook", Some(a));
    let h2 = in_play(&mut e, 0, "Meat Hook", Some(a));
    hand(&mut e, 0, &["Blur"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, use_card(h1, 0, None, Target::None));
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, use_card(h2, 0, None, Target::None));
    assert!(!req.options.contains(&use_card(h1, 0, None, Target::None)));
}

/// Ammo: "Only usable before resolution of a gun's strike." [RTR 19990105]
/// After the strikes are declared, before they resolve; [LSJ 20020425]
/// "Cannot be played on an opponent's weapon": the blocker's gun strike
/// takes neither Scattershot nor Caseless Rounds from the unarmed bleeder.
#[test]
fn ammo_is_not_played_on_an_opponents_gun() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Scattershot", "Caseless Rounds"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, use_card(gun, 0, None, Target::None));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(x), .. } if *x == a),
    );
}

/// The Mole: "[ani] Only usable by a locked vampire. This vampire unlocks and
/// attempts to block." [LSJ 20090514] "If the action lacks a suitable target
/// because of the unlock (e.g. Ambush), the block attempt still happens, as
/// does the resulting combat if the block is successful."
#[test]
fn the_mole_unlocks_an_ambushs_target_to_block_it() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let r = e.add_minion(
        SeatId(1),
        textless(
            |c| c.sect == Some(Sect::Anarch) && c.disciplines.contains(&"ani"),
            0,
        ),
        4,
        false,
    );
    e.state.minions[r.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Ambush"]);
    hand(&mut e, 1, &["Mole, The"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Minion(r)));
    pick_when(&mut e, play_at(0, 0, Some(r), Target::None));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert_eq!((blood(&e, a), blood(&e, r)), (3, 3), "the block's combat");
}

/// Jack of Both Sides: "This vampire equips or employs an equipment or
/// retainer from your hand." [LSJ 20050913] "Cannot select cards that are
/// "playable as if in hand" (e.g. from The Erciyes Fragments)": nothing in
/// hand, the .44 Magnum on the Fragments: not offered. [ANK 20180817] "Does
/// not prevent the acting vampire from playing the equipment/retainer/ally
/// directly this turn if he unlocks": the Magnum equipped by Jack, the
/// vampire unlocked equips a Flash Grenade from hand.
#[test]
fn jack_of_both_sides_reaches_the_hand_only_and_bars_nothing_after() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let frag = in_play(&mut e, 0, "Erciyes Fragments, The", None);
    e.state.cards[frag.index()].stored = vec![find_library(".44 Magnum")].into();
    hand(&mut e, 0, &["Jack of Both Sides"]);
    let req = to_minion_phase(&mut e, SeatId(0));
    assert!(!req.options.contains(&play(0, 0, a)), "{req:?}");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Jack of Both Sides", ".44 Magnum"]);
    pick_when(&mut e, play(0, 0, a));
    to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(on(&e, ".44 Magnum", a).len(), 1);
    e.state.minions[a.index()].locked = false;
    hand(&mut e, 0, &["Flash Grenade"]);
    let k = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Flash Grenade"))
        .expect("in hand") as u8;
    let req = request(&mut e);
    assert!(req.options.contains(&play(k, 0, a)), "{req:?}");
}

/// Additional damage inherits the base damage's properties: [TOM 19960225]
/// (Scorpion Sting's +1) and [RTR 19970630] [TOM 19960225] (Pushing the
/// Limit's +2, Undead Strength's +1): under Uncontrollable Rage [pro] the
/// hand strikes' whole damage is aggravated, so the blocker's Soak (non-
/// aggravated damage only) is never offered; without the Rage, it is.
#[test]
fn added_hand_strike_damage_inherits_aggravation() {
    for (card, disc) in [
        ("Scorpion Sting", "ani"),
        ("Pushing the Limit", "pot"),
        ("Undead Strength", "pot"),
    ] {
        for rage in [true, false] {
            let mut e = game(2);
            let a = e.add_minion(SeatId(0), vampire_with("pro", 0), 6, false);
            give(&mut e, a, disc);
            e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
            let z = e.add_minion(SeatId(1), vampire_with("for", 0), 6, false);
            e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
            hand(&mut e, 0, &[card, "Uncontrollable Rage"]);
            hand(&mut e, 1, &["Soak"]);
            pick_when(&mut e, bleed(a));
            if rage {
                pick_when(&mut e, play(1, 0, a));
            }
            pick_when(&mut e, block(z));
            pick_when(&mut e, play(0, 0, a));
            let soak = offered_before(&mut e, play(0, 0, z), SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(soak, !rage, "{card}, rage {rage}");
        }
    }
}

/// Repo Man: "You then place the vehicle on any ready minion you control
/// who meets the requirements of the card (pay cost to equip as normal)."
/// [RTR 19941109] [RTR 20010711] "Paying for the equipment found is not
/// optional if a ready minion meet the requirements. If paying the cost
/// will oust the controlling Methuselah, then she is ousted": no pass, and
/// the Sport Bike's last pool ousts seat 0.
#[test]
fn repo_man_pays_for_the_vehicle_even_to_an_oust() {
    let mut e = game(2);
    let r = e.add_ally(SeatId(0), find_library("Repo Man"));
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].pool = 1;
    hand(&mut e, 0, &["Villein"; 7]);
    e.state.seats[0].library.push(find_library("Sport Bike"));
    let card = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.body == Some(r))
        .map(|i| CardId(i as u32))
        .expect("his card");
    pick_when(&mut e, use_card(card, 0, Some(r), Target::None));
    let req = to_window(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(!req.options.contains(&Choice::Pass), "{req:?}");
    pick(&mut e, Choice::ChooseMinion(v));
    let _ = e.run();
    assert!(e.state.seats[0].ousted, "the bike's pool paid");
}

/// A Hermana Hambrienta Mayor (Mexico City Circle) for seat 0.
fn hermana(e: &mut Engine, blood: u8) -> MinionId {
    e.add_minion(
        SeatId(0),
        find_card("Hermana Hambrienta Mayor"),
        blood,
        false,
    )
}

/// Slake the Thirst: "Only usable after this Blood Brother gains blood …
/// The chosen Blood Brother gains the same amount of blood." [ANK 20201228]
/// "Can be played after a diablerie, before the blood hunt ([san] after
/// Amaranth …)": the Hermana's Amaranth on the blocker her Coma sent to
/// torpor with its 3 blood, Slake [san] offered at once, the combat
/// suspended for the window before the blood hunt (rules-qa 113.10;
/// advanced rules §3.7.8.1), the hunt next in it.
/// [LSJ 20100325] "[SAN] Can be played at any time blood is gained during
/// an action, including after action resolution (eg. Games of Instinct
/// …)": the Hermana's Games of Instinct gain, Slake [SAN] offered after it.
#[test]
fn slake_the_thirst_follows_amaranth_and_games_of_instinct() {
    let slake_at = |a: MinionId, b: MinionId| {
        move |o: &Choice| {
            matches!(o, Choice::Play { actor: Some(x), target: Target::Minion(y), .. }
                if *x == a && *y == b)
        }
    };
    let mut e = game(2);
    let a = hermana(&mut e, 4);
    give(&mut e, a, "san");
    give(&mut e, a, "dem");
    let b = hermana(&mut e, 1);
    let z = e.add_minion(SeatId(1), textless(|c| c.capacity <= 4, 0), 3, false);
    let p = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    hand(&mut e, 0, &["Coma", "Amaranth", "Slake the Thirst"]);
    bleed_blocked(&mut e, a, z);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play_at(0, 0, Some(a), Target::None));
    let req = loop {
        let req = request(&mut e);
        if req.options.iter().any(slake_at(a, b)) {
            break req;
        }
        e.choose(fallback(&req));
    };
    assert!(!e.state.minions[z.index()].in_play, "diablerized");
    let state = e.action().map(|x| x.state);
    assert!(
        matches!(state, Some(zvtes::actions::ActionState::AfterDiablerie(..)))
            && e.combat().is_some(),
        "its window, in the combat suspended: {req:?}"
    );
    pick_matching(&mut e, slake_at(a, b));
    let req = loop {
        let req = request(&mut e);
        if req.window == WindowLabel::BloodHuntPolling {
            break req;
        }
        assert!(e.referendum().is_none(), "{req:?}");
        e.choose(fallback(&req));
    };
    assert_eq!(blood(&e, b), 4, "the same gain, before the hunt: {req:?}");
    assert!(e.combat().is_some(), "the hunt in the combat");

    let mut e = game(2);
    let a = hermana(&mut e, 1);
    give(&mut e, a, "SAN");
    let b = hermana(&mut e, 1);
    let z = e.add_minion(SeatId(1), elder(), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Games of Instinct", "Slake the Thirst"]);
    pick_when(&mut e, play_at(0, 0, Some(a), Target::Minion(z)));
    pick_matching(&mut e, slake_at(a, b));
    request(&mut e);
    assert!(blood(&e, b) > 1, "the same gain");
}

/// The Diamond Thunderbolt: [LSJ 20070222-2] "If used to cancel the change
/// of control of a Corruption action, only the control change is
/// prevented: the corruption counters are burned." Seat 1's Reformation
/// [ser] brings its counters on seat 0's vampire to its capacity and burns
/// them to steal it; seat 0's Thunderbolt keeps it, the counters gone.
#[test]
fn diamond_thunderbolt_lets_the_corruption_counters_burn() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), textless(|c| c.capacity == 2, 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[t.index()].add_corruption(SeatId(1), 1);
    let a = e.add_minion(SeatId(1), vampire_with("ser", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Diamond Thunderbolt, The"]);
    hand(&mut e, 1, &["Reformation"]);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, play_at(0, 2, Some(a), Target::Minion(t)));
    pick_when(&mut e, Choice::ChooseMinion(t));
    pick_when(&mut e, play_at(0, 0, None, Target::Minion(t)));
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[t.index()];
    assert_eq!(v.owner, SeatId(0), "kept");
    assert_eq!(v.corruption_of(SeatId(1)), 0, "the counters burned");
}

/// Jack of Both Sides: "This vampire … employs an equipment or retainer
/// from your hand." [LSJ 20080803] "When choosing a card whose effect depend
/// on Discipline level, the vampire decides in respect to his disciplines
/// the normal way": Raven Spy employed by an inferior Animalism vampire is
/// the [ani] version (1 life), by a superior one the [ANI] (2 life).
#[test]
fn jack_of_both_sides_employs_by_the_vampires_level() {
    for (disc, life) in [("ani", 1), ("ANI", 2)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with(disc, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Jack of Both Sides", "Raven Spy"]);
        pick_when(&mut e, play(0, 1, a));
        let superior = Choice::EmployFromHand {
            card: find_library("Raven Spy"),
            mode: 1,
        };
        let req = request(&mut e);
        if life == 2 {
            e.choose(index_of(&req, superior));
        } else {
            assert!(!req.options.contains(&superior), "{req:?}");
        }
        to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let spy = on(&e, "Raven Spy", a);
        assert_eq!(spy.len(), 1, "{disc}: employed");
        assert_eq!(e.state.cards[spy[0].index()].counters, life, "{disc}");
    }
}

/// Siphon: "Choose one or more vampires in torpor … Steal 1 blood from each
/// chosen vampire." [RTR 20010711] [PIB 20110725] "Vampires with no blood
/// can be chosen": an empty younger vampire in torpor is offered beside one
/// with blood.
#[test]
fn siphon_may_choose_an_empty_vampire() {
    let (mut e, _, young, _) = siphon_table();
    e.state.minions[young.index()].blood = 0;
    let other = MinionId(e.state.minions.len() as u32);
    e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 1), 2, true);
    let req = to_window(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(
        req.options.contains(&Choice::ChooseMinion(young))
            && req.options.contains(&Choice::ChooseMinion(other)),
        "{req:?}"
    );
}

/// Bear-Baiting: [LSJ 20090322] [ANK 20181003] [LSJ 20010803] "If the
/// combat is canceled, the action is still blocked, Mask of a Thousand
/// Faces cannot be used to continue it": the combat canceled (seat 0's
/// crypt older), the other Obfuscate vampire is never offered the mask; the
/// combat going on (younger), it is.
#[test]
fn mask_of_a_thousand_faces_cannot_continue_after_bear_baiting() {
    for mine in [9, 3] {
        let (mut e, _, m, _) = bear_baiting_more(mine, &["Mask of a Thousand Faces"]);
        let mask = play(0, 0, m);
        if mine == 9 {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, mask);
        } else {
            offering(&mut e, mask);
        }
    }
}

/// Church of the Order of St. Blaise: [LSJ 20050105-1] "Church of the
/// Order of St. Blaise may be used to add a counter to it before it burns
/// if it has zero counters": Powerbase: Barranquilla's last blood moved to
/// the pool, the Church adds a counter and it stays; without the Church, it
/// burns as its controller passes.
#[test]
fn church_of_st_blaise_saves_an_emptied_barranquilla() {
    for church in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 0), 4, false);
        e.add_minion(SeatId(0), of_sect(Sect::Sabbat, 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        let ch = in_play(&mut e, 0, "Church of the Order of St. Blaise", None);
        let loc = in_play(&mut e, 0, "Powerbase: Barranquilla", None);
        e.state.cards[loc.index()].counters = 1;
        to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
        until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
        pick(&mut e, use_card(loc, 0, None, Target::None));
        assert_eq!(e.state.seats[0].pool, 11);
        assert!(e.state.cards[loc.index()].in_play, "not burned yet");
        if church {
            pick(&mut e, use_card(ch, 0, None, Target::Card(loc)));
        }
        to_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let c = &e.state.cards[loc.index()];
        assert_eq!(c.in_play, church, "church: {church}");
        if church {
            assert_eq!(c.counters, 1);
        }
    }
}

/// Hector Trelane: [LSJ 20091210] "The opponent gains pool if he's burned
/// because of a bloodhunt during combat (eg. after Amaranth)": Hector's
/// Amaranth diablerizes the blocker, the blood hunt passes and burns him,
/// the combat suspended for it (rules-qa 113.10) then over, the blocker's
/// controller gaining 4 pool once.
#[test]
fn hector_trelane_burned_by_a_blood_hunt_after_amaranth_pays_the_opponent() {
    let mut e = game(3);
    let h = e.add_minion(SeatId(0), find_card("Hector Trelane"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 0), 0, false);
    let p = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Paris"));
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Amaranth"]);
    bleed_blocked(&mut e, h, b);
    pick_when(&mut e, play_at(0, 0, Some(h), Target::None));
    until(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    assert!(!e.state.minions[b.index()].present(), "diablerized");
    assert!(
        e.combat().is_some(),
        "the hunt in the combat (rules-qa 113.10)"
    );
    pick(&mut e, votes(p, true));
    request(&mut e);
    assert!(!e.state.minions[h.index()].present(), "burned by the hunt");
    assert!(e.combat().is_none(), "the combat over with him");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 14);
}

/// Alia, God's Messenger: [RTR 19970630] [LSJ 20080717] "They cannot use a
/// card to commit diablerie (eg. Amaranth)": Alia and 47th Street Royals
/// blocking, their strike sending the acting vampire to torpor, are never
/// offered Amaranth; a vampire blocking is.
#[test]
fn allies_playing_as_vampires_cannot_use_amaranth() {
    for blocker in ["Alia, God's Messenger", "47th Street Royals", ""] {
        let mut e = game(2);
        // Not starving: no hunt due first; emptied once it bleeds.
        let a = e.add_minion(SeatId(0), textless(|c| c.capacity <= 4, 0), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let b = if blocker.is_empty() {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false)
        } else {
            e.add_ally(SeatId(1), find_library(blocker))
        };
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 1, &["Amaranth"]);
        pick(&mut e, bleed(a));
        e.state.minions[a.index()].blood = 0;
        pick_when(&mut e, block(b));
        let amaranth = play_at(0, 0, Some(b), Target::None);
        if blocker.is_empty() {
            offering(&mut e, amaranth);
        } else {
            never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, amaranth);
            assert!(e.state.minions[a.index()].in_torpor, "{blocker}: torpor");
        }
    }
}

/// Bear-Baiting's canceled combat bars the mask on that block only: the
/// action continued by Crypt's Sons and blocked again, the new block offers
/// Mask of a Thousand Faces.
#[test]
fn a_new_block_after_bear_baiting_and_crypts_sons_offers_the_mask() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Anarch) && (6..=8).contains(&c.capacity),
            0,
        ),
        4,
        false,
    );
    let m = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    let b = e.add_minion(SeatId(1), elder_camarilla(), 4, false);
    let b2 = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    for (s, cap) in [(0, 9), (1, 3)] {
        let v = CRYPT
            .iter()
            .position(|c| c.capacity == cap && !c.imbued)
            .expect("a vampire of that capacity") as u16;
        e.state.seats[s].crypt.put(v, zvtes::copies::CopyId::NONE);
    }
    let sons = in_play(&mut e, 0, "Crypt's Sons", Some(a));
    e.state.cards[sons.index()].counters = 3;
    hand(&mut e, 0, &["Bear-Baiting", "Mask of a Thousand Faces"]);
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, use_card(sons, 0, None, Target::None));
    pick_when(&mut e, block(b2));
    let req = offering(&mut e, play(0, 0, m));
    assert_eq!(req.window, WindowLabel::ActionBlocked);
}

/// rules-qa 111.54 (owner, 2026-11-08): Derange's sect holds over a later
/// actual sect change (LSJ 20100811, ANK 20190619): Into the Fire on the
/// deranged vampire changes its underlying sect, not the one it is
/// considered; [LSJ 20021209]'s "the most recent one takes precedence"
/// holds between two overrides only (the pool has one).
#[test]
fn derange_holds_over_into_the_fire() {
    let mut e = game(2);
    let mal = textless(
        |c| c.clan == "Malkavian" && c.sect == Some(Sect::Camarilla) && c.capacity >= 5,
        0,
    );
    let a = e.add_minion(SeatId(0), mal, 5, false);
    let young = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Anarch) && c.capacity <= 3, 0),
        2,
        false,
    );
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Derange", "Into the Fire"]);
    fill_hand(&mut e, 0);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Minion(young),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.sect_of(young), Some(Sect::Camarilla), "deranged");
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    master_on(&mut e, "Into the Fire", young);
    assert_eq!(
        e.state.minions[young.index()].sect_override,
        Some(Sect::Sabbat),
        "the underlying change made"
    );
    assert_eq!(
        e.state.sect_of(young),
        Some(Sect::Camarilla),
        "Derange holds"
    );
}

// ---- Inert clauses that gained a pool tenant (rules-qa Batch 115) ----------

/// Louis Fortier merged: "[MERGED] Louis gets +1 level of Dominate [dom]
/// and Presence [pre]"; unmerged, his printed inferior levels.
#[test]
fn louis_fortier_merged_gains_superior_dominate_and_presence() {
    for merged in [true, false] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), find_vampire("Louis Fortier", true), 4, false);
        if merged {
            e.state.minions[l.index()].merged = Some(find_vampire("Louis Fortier", false));
        }
        let want = if merged { 2 } else { 1 };
        for d in ["dom", "pre"] {
            assert_eq!(e.state.discipline_level(l, d), want, "{d}");
        }
        assert_eq!(e.state.discipline_level(l, "aus"), 1);
    }
}

/// Gengis merged: "[MERGED] Gengis gets +1 level of Auspex [aus] and
/// Celerity [cel]", his base card's Brujah takeover kept (a Brujah of
/// another Methuselah may take it).
#[test]
fn gengis_merged_gains_levels_and_keeps_the_brujah_takeover() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(1), find_vampire("Gengis", true), 4, false);
    e.state.minions[g.index()].merged = Some(find_vampire("Gengis", false));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    for d in ["aus", "cel"] {
        assert_eq!(e.state.discipline_level(g, d), 2, "{d}");
    }
    assert_eq!(e.state.discipline_level(g, "pot"), 1);
    let req = request(&mut e);
    assert!(req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { actor: Some(a), target: Target::Minion(t), .. }
            if *a == b && *t == g)));
}

/// Marguerite Foccart merged: "[MERGED] +1 strength", her base card's +1
/// intercept against political actions kept.
#[test]
fn marguerite_foccart_merged_strikes_harder() {
    for merged in [true, false] {
        let mut e = game(2);
        let m = e.add_minion(
            SeatId(0),
            find_vampire("Marguerite Foccart", true),
            4,
            false,
        );
        if merged {
            e.state.minions[m.index()].merged = Some(find_vampire("Marguerite Foccart", false));
        }
        let d = zvtes::cards::vampire_of(&e.state.minions[m.index()]).expect("defined");
        let printed = |stat| d.printed.iter().filter(|p| p.stat == stat).count();
        assert_eq!(
            e.state
                .printed_mod(m, zvtes::cards::Stat::Strength, Default::default()),
            i32::from(merged)
        );
        assert_eq!(printed(zvtes::cards::Stat::Intercept), usize::from(merged));
    }
}

/// Tegyrius, Vizier merged: his base card's "+1 strength" kept beside his
/// advanced card's polling votes; unmerged, the advanced card alone.
#[test]
fn tegyrius_merged_keeps_his_base_cards_strength() {
    for merged in [true, false] {
        let mut e = game(2);
        let t = e.add_minion(SeatId(0), find_vampire("Tegyrius, Vizier", true), 4, false);
        if merged {
            e.state.minions[t.index()].merged = Some(find_vampire("Tegyrius, Vizier", false));
        }
        let d = zvtes::cards::vampire_of(&e.state.minions[t.index()]).expect("defined");
        assert_eq!(d.abilities.len(), 1, "the polling votes");
        assert_eq!(
            e.state
                .printed_mod(t, zvtes::cards::Stat::Strength, Default::default()),
            i32::from(merged)
        );
    }
}

/// A merged vampire has both cards' text (rules-qa 111.19, 115.5): every
/// base and advanced pair in the pool whose base card's text does
/// something reads a merged definition, not the advanced card's alone.
#[test]
fn every_merge_keeps_its_base_cards_text() {
    use zvtes::cards::{vampire, vampire_of};
    let acts = |d: &zvtes::cards::VampireDef| {
        !d.printed.is_empty()
            || !d.abilities.is_empty()
            || !d.riders.is_empty()
            || d.plays_as.is_some()
            || !d.levels.is_empty()
    };
    let mut missing = Vec::new();
    for (adv, c) in CRYPT.iter().enumerate().filter(|(_, c)| c.adv) {
        let Some(base) = CRYPT.iter().position(|b| !b.adv && b.name == c.name) else {
            continue;
        };
        if !vampire(base as u16).is_some_and(acts) {
            continue;
        }
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), adv as u16, 4, false);
        e.state.minions[m.index()].merged = Some(base as u16);
        let merged = vampire_of(&e.state.minions[m.index()]);
        if merged.is_none_or(|d| vampire(adv as u16).is_some_and(|a| std::ptr::eq(a, d))) {
            missing.push(c.name);
        }
    }
    assert!(missing.is_empty(), "no merged definition: {missing:?}");
}

/// The Fourth Cycle: "burn all Gehenna events and conviction cards": an
/// imbued's conviction, another Methuselah's, to its owner's ash heap.
#[test]
fn the_fourth_cycle_burns_the_conviction_cards() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 0), 6, false);
    let c = find_card("Marion \"Teacher193\" Perks");
    let i = e.add_minion(SeatId(1), c, CRYPT[c as usize].capacity, false);
    let strike = find_library("Strike with Conviction");
    e.state
        .cards
        .push(CardInPlay::new(strike, SeatId(1), Some(i), 0));
    in_play(&mut e, 1, "Thirst", None);
    in_play(&mut e, 1, "Torpid Blood", None);
    assert_eq!(e.state.convictions(i).len(), 1);
    hand(&mut e, 0, &["Fourth Cycle, The"]);
    until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, play_at(0, 0, None, Target::None));
    request(&mut e);
    assert!(e.state.convictions(i).is_empty());
    assert!(e.state.seats[1].ash.contains(&strike));
}

/// Blood Trade: "Burn all boons": Life Boon and Major Boon in play, any
/// Methuselah's, to their owners' ash heaps.
#[test]
fn blood_trade_burns_the_boons_in_play() {
    let mut e = game(3);
    for s in 0..3 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    let life = in_play(&mut e, 1, "Life Boon", None);
    let major = in_play(&mut e, 2, "Major Boon", None);
    hand(&mut e, 0, &["Blood Trade"]);
    until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, play_at(0, 0, None, Target::None));
    request(&mut e);
    assert!(!e.state.cards[life.index()].in_play && !e.state.cards[major.index()].in_play);
    assert!(e.state.seats[1].ash.contains(&find_library("Life Boon")));
    assert!(e.state.seats[2].ash.contains(&find_library("Major Boon")));
}

/// Blood Trade: "No more boons can be put in play": Major Boon and Life
/// Boon, which enter play, are not played (rules-qa 115.7); without Blood
/// Trade both are offered. Seat 1 bleeds seat 2, who has 1 pool; seat 0
/// holds the boons.
#[test]
fn blood_trade_bars_the_boons_entering_play() {
    for trade in [false, true] {
        for boon in ["Major Boon", "Life Boon"] {
            let mut e = game(3);
            e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
            let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
            e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
            e.state.seats[2].pool = 1;
            if trade {
                hand(&mut e, 0, &[boon, "Blood Trade"]);
                until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
                pick(&mut e, play_at(1, 0, None, Target::None));
            } else {
                hand(&mut e, 0, &[boon]);
            }
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            pick(&mut e, bleed(y));
            let want = if boon == "Major Boon" {
                master_play(0)
            } else {
                play_at(0, 0, None, Target::Seat(SeatId(2)))
            };
            let offered = offered_before(&mut e, want, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(offered, !trade, "{boon}, Blood Trade {trade}");
            if trade && boon == "Life Boon" {
                assert!(e.state.seats[2].ousted, "the oust waits on no boon");
            }
        }
    }
}
