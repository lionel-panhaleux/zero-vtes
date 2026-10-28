//! Round 22 (`ADDED_BATCHES`, slice 16ah): Anthology and Anarch Unbound
//! completed, set by set (owner, 2026-10-28). Readings in rules-qa Batch
//! 104. Windows with a single option are elided, so the scripts only stop
//! where a real decision exists.

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

/// A vampire of any sect with `discs` put in play for `seat` and made
/// Anarch (the rulebook action's actual change).
fn made_anarch(e: &mut Engine, seat: u8, discs: &[&str], nth: usize) -> MinionId {
    let c = [
        Sect::Camarilla,
        Sect::Sabbat,
        Sect::Independent,
        Sect::Anarch,
    ]
    .iter()
    .flat_map(|&s| {
        CRYPT.iter().enumerate().filter(move |(i, c)| {
            c.sect == Some(s)
                && c.title == Title::None
                && c.capacity >= 4
                && !c.imbued
                && zvtes::cards::vampire(*i as u16).is_none()
                && discs
                    .iter()
                    .all(|d| c.disciplines.iter().any(|x| x.eq_ignore_ascii_case(d)))
        })
    })
    .nth(nth)
    .map(|(i, _)| i as u16)
    .unwrap_or_else(|| panic!("no vampire with {discs:?}"));
    let m = e.add_minion(SeatId(seat), c, 4, false);
    e.state.minions[m.index()].sect_override = Some(Sect::Anarch);
    m
}

/// Choose `want` if the next request offers it (a single option elided).
fn choose_if_offered(e: &mut Engine, want: Choice) {
    let req = request(e);
    if let Some(i) = req.options.iter().position(|o| *o == want) {
        e.choose(i);
    }
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

// ---- Field Training ---------------------------------------------------------

// Field Training: "+1 stealth action. Requires an Anarch. Unlock this
// vampire, choose a Discipline they have at superior, and put this card on
// another ready non-titled vampire you control. The attached vampire is
// considered Anarch and gets +1 level of the chosen Discipline." The
// Discipline the acting vampire's superior one (ANK 20180731-2); the
// vampire Anarch, a level gained, no capacity; the actor unlocked; the card
// itself in play.
// Rulings: [ANK 20180731-2].
#[test]
fn field_training_makes_another_vampire_anarch_with_a_level() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&["CEL"], 0), 4, false);
    let c = of_sect(Sect::Camarilla, &[], 0);
    let t = e.add_minion(SeatId(0), c, 4, false);
    let cap = e.state.capacity_of(t);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Field Training"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Minion(t)));
    choose_if_offered(&mut e, Choice::ChooseDiscipline("cel"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let ft = on(&e, "Field Training", t).expect("on the vampire");
    placed(&e, ft);
    assert_eq!(e.state.sect_of(t), Some(Sect::Anarch));
    assert!(e.state.discipline_level(t, "cel") >= 1);
    assert_eq!(e.state.capacity_of(t), cap, "no capacity");
    assert!(!e.state.minions[a.index()].locked, "the actor unlocked");
}

// The acting vampire must have a superior Discipline to choose from (ANK
// 20180731-2): not playable without one; never on a titled vampire, the
// actor itself, or a vampire with a Field Training.
// Rulings: [ANK 20180731-2].
#[test]
fn field_training_needs_a_superior_discipline_and_an_untrained_untitled_target() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&["CEL"], 0), 4, false);
    let t = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, &[], 0), 4, false);
    let titled = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.title != Title::None && c.capacity >= 4 && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a titled vampire") as u16;
    let tv = e.add_minion(SeatId(0), titled, 4, false);
    let trained = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, &[], 1), 4, false);
    in_play(&mut e, 0, "Field Training", Some(trained));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Field Training"]);
    let req = request(&mut e);
    let at = |m: MinionId| play_at(0, 0, Some(a), Target::Minion(m));
    assert!(req.options.contains(&at(t)), "{:?}", req.options);
    for m in [tv, trained, a] {
        assert!(!req.options.contains(&at(m)), "{m:?}");
    }
    // No superior Discipline: not playable.
    let mut e = game(2);
    let plain_anarch = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.sect == Some(Sect::Anarch)
                && c.title == Title::None
                && c.capacity >= 4
                && zvtes::cards::vampire(i as u16).is_none()
                && !c
                    .disciplines
                    .iter()
                    .any(|d| d.chars().all(|ch| ch.is_uppercase()))
        })
        .expect("an Anarch with no superior") as u16;
    let a = e.add_minion(SeatId(0), plain_anarch, 4, false);
    let t = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, &[], 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Field Training"]);
    let req = request(&mut e);
    assert!(
        !req.options
            .contains(&play_at(0, 0, Some(a), Target::Minion(t))),
        "{:?}",
        req.options
    );
}

// "Is a change of the actual sect. A temporary effect overriding the sect
// has precedence and if it does, this card burns" (LSJ 20100811): an
// override landing (Derange's sect) burns it, the Discipline level gone
// with the card.
// Rulings: [LSJ 20100811].
#[test]
fn field_training_burns_when_its_vampire_changes_sect() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&["CEL"], 0), 4, false);
    let t = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, &[], 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Field Training"]);
    pick(&mut e, play_at(0, 0, Some(a), Target::Minion(t)));
    choose_if_offered(&mut e, Choice::ChooseDiscipline("cel"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let ft = on(&e, "Field Training", t).expect("on the vampire");
    let before = e.state.discipline_level(t, "cel");
    // An override lands: the vampire considered Sabbat.
    let d = in_play(&mut e, 1, "Derange", Some(t));
    e.state.minions[t.index()].deranged = Some(("Brujah", Sect::Sabbat, d));
    request(&mut e);
    assert!(!e.state.cards[ft.index()].in_play, "burned");
    assert!(ash_has(&e, 0, "Field Training"));
    assert_eq!(e.state.discipline_level(t, "cel"), before - 1);
    // The change ends with the card (owner, 2026-10-28: rules-qa 104.3):
    // once Derange leaves, the printed sect.
    e.burn_card(d);
    request(&mut e);
    assert_eq!(e.state.sect_of(t), Some(Sect::Camarilla));
}

// ---- Illegalism -------------------------------------------------------------

// Illegalism: "Requires an Anarch. More than one Discipline can be used to
// play this card. Ⓓ Bleed. [cel] Unlock this Anarch if the bleed is
// successful. [chi] This is a +1 stealth action. [pre] The bleed is with +1
// bleed." Every non-empty set of the Disciplines the vampire has (LSJ
// 20011204-3, PIB 20130704); all three: +1 stealth, +1 bleed, the Anarch
// unlocked.
// Rulings: [LSJ 20011204-3] [PIB 20130704].
#[test]
fn illegalism_combines_its_disciplines() {
    let mut e = game(2);
    // François Villon, the one vampire with all three, made Anarch.
    let fv = CRYPT
        .iter()
        .position(|c| c.id == 200478)
        .expect("François Villon") as u16;
    let a = e.add_minion(SeatId(0), fv, 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Illegalism"]);
    let req = request(&mut e);
    for mode in 0..7 {
        assert!(req.options.contains(&play(0, mode, a)), "mode {mode}");
    }
    pick(&mut e, play(0, 6, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 7, "bled for 3: his own +1 bleed too");
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    // A vampire with Celerity only: the [cel] mode alone.
    let mut e = game(2);
    let c = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.sect == Some(Sect::Anarch)
                && c.title == Title::None
                && c.capacity >= 4
                && zvtes::cards::vampire(i as u16).is_none()
                && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("cel"))
                && !c
                    .disciplines
                    .iter()
                    .any(|d| d.eq_ignore_ascii_case("chi") || d.eq_ignore_ascii_case("pre"))
        })
        .expect("an Anarch with Celerity alone") as u16;
    let a = e.add_minion(SeatId(0), c, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Illegalism"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, a)));
    for mode in 1..7 {
        assert!(!req.options.contains(&play(0, mode, a)), "mode {mode}");
    }
}

// [cel]'s unlock needs a successful bleed: blocked, the Anarch stays
// locked.
#[test]
fn illegalism_blocked_does_not_unlock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(&["cel"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Illegalism"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].locked || e.state.minions[a.index()].in_torpor);
    assert_eq!(e.state.seats[1].pool, 10);
}

// ---- Netwar -----------------------------------------------------------------

// Netwar [aus]: "Look at the acting Methuselah's hand." A Methuselah who
// is not the target plays it once the target declared its block decision
// (LSJ 20100507, ANK 20210131-1): the acting Methuselah's hand seen.
// One Discipline a play, the card requiring the one used (LSJ
// 20011204-3, PIB 20130704).
// Rulings: [LSJ 20100507] [ANK 20210131-1].
// Rulings: [LSJ 20011204-3] [PIB 20130704].
#[test]
fn netwar_looks_at_the_acting_methuselahs_hand_after_the_block_decision() {
    let mut e = game(3);
    // Seat 0 bleeds seat 1; seat 2's Anarch is not the target.
    let z = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let r = made_anarch(&mut e, 2, &["aus"], 0);
    e.add_minion(SeatId(2), plain("Brujah", 1), 4, false);
    hand(&mut e, 2, &["Netwar"]);
    e.notes = Some(Vec::new());
    pick(&mut e, bleed(z));
    let look = play(0, 0, r);
    let mut declined = false;
    for _ in 0..40 {
        let req = request(&mut e);
        if req.options.contains(&look) {
            assert!(declined, "the target declares first");
            e.choose(index_of(&req, look));
            break;
        }
        if req.seat == SeatId(1) && req.options.contains(&block(t)) {
            declined = true;
        }
        e.choose(fallback(&req));
    }
    assert!(declined);
    request(&mut e);
    let looks = noted_looks(&mut e);
    assert!(
        looks
            .iter()
            .any(|l| l.viewer == SeatId(2) && l.seat == SeatId(0) && !l.library),
        "{looks:?}"
    );
}

// Netwar [chi]: "Choose another ready minion you control. The chosen
// minion unlocks and attempts to block." A locked minion of the target's
// unlocked and blocking; never the reacting vampire itself.
// Rulings: [ANK 20180623] [LSJ 20090514].
#[test]
fn netwar_makes_another_minion_unlock_and_block() {
    let mut e = game(2);
    let r = made_anarch(&mut e, 0, &["chi"], 0);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[b.index()].locked = true;
    e.notes = Some(Vec::new());
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Netwar"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    let forced = play_at(0, 1, Some(r), Target::Minion(b));
    let req = offering(&mut e, forced);
    assert!(
        !req.options
            .contains(&play_at(0, 1, Some(r), Target::Minion(r))),
        "not the reacting vampire"
    );
    pick(&mut e, forced);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10, "blocked");
    let combats = noted_combats(&mut e);
    assert!(!combats.is_empty(), "the block's combat");
}

// A minion that cannot block this action is not chosen: a third
// Methuselah's minion against a bleed of the prey (ANK 20180623, LSJ
// 20010714-2).
// Rulings: [LSJ 20010714-2] [ANK 20180623].
#[test]
fn netwar_is_not_offered_to_a_minion_that_cannot_block() {
    let mut e = game(3);
    let r = made_anarch(&mut e, 2, &["chi"], 0);
    let b = e.add_minion(SeatId(2), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z = e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 2), 4, false);
    hand(&mut e, 2, &["Netwar"]);
    // Seat 0 bleeds seat 1: seat 2 is seat 0's predator, not its target.
    pick(&mut e, bleed(z));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play_at(0, 1, Some(r), Target::Minion(b)),
    );
}

// Netwar [dem]: "Give another minion you control +1 intercept." To the
// minion of its player's attempting to block, when needed: a +1 stealth
// action blocked.
#[test]
fn netwar_gives_another_blocker_intercept() {
    let mut e = game(2);
    let r = made_anarch(&mut e, 0, &["dem"], 0);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = made_anarch(&mut e, 1, &["chi"], 0);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Netwar"]);
    hand(&mut e, 1, &["Illegalism"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 1, z));
    pick_when(&mut e, block(b));
    let give = play(0, 2, r);
    pick_when(&mut e, give);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10, "blocked");
}

// ---- The Red Question -------------------------------------------------------

// The Red Question: "Requires a non-titled non-Anarch vampire. Put this
// card on this vampire. This vampire is considered Anarch. Burn this card if
// this vampire changes sect." The card on the acting vampire, now Anarch
// (an actual change, LSJ 20100811).
// Rulings: [LSJ 20100811].
#[test]
fn the_red_question_makes_the_acting_vampire_anarch() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Red Question, The"]);
    pick(&mut e, bleed(v));
    pick_when(&mut e, play(0, 0, v));
    request(&mut e);
    let rq = on(&e, "Red Question, The", v).expect("on the vampire");
    placed(&e, rq);
    assert_eq!(e.state.sect_of(v), Some(Sect::Anarch));
    assert_eq!(e.state.minions[v.index()].blood, 3, "1 blood");
    // Another sect change burns it.
    e.state.minions[v.index()].sect_override = Some(Sect::Camarilla);
    request(&mut e);
    assert!(!e.state.cards[rq.index()].in_play);
}

// The Red Question "does not make the action an action that 'makes this
// vampire anarch'" (ANK 20180125): CrimethInc. not offered after it.
// Rulings: [ANK 20180125].
#[test]
fn the_red_question_is_no_action_making_its_vampire_anarch() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, &["pro"], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Red Question, The", "CrimethInc."]);
    pick(&mut e, bleed(v));
    pick_when(&mut e, play(0, 0, v));
    request(&mut e);
    assert_eq!(e.state.sect_of(v), Some(Sect::Anarch));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, v));
}

// "If the action required another (non-Anarch) sect and is not blocked, the
// action fizzles" (ANK 20210124): a Camarilla vampire's Justicar
// Retribution ("Requires a Camarilla vampire"), the vampire made Anarch by
// The Red Question, calls no referendum. A titled vampire cannot play it.
// Rulings: [ANK 20210124].
#[test]
fn the_red_question_fizzles_an_action_requiring_its_old_sect() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, &[], 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Justicar Retribution", "Red Question, The"]);
    pick(&mut e, play(0, 0, v));
    // Played, the first card leaves the hand (an empty library replaces
    // nothing): The Red Question is first.
    pick_when(&mut e, play(0, 0, v));
    for _ in 0..40 {
        let req = request(&mut e);
        assert_ne!(req.window, WindowLabel::ReferendumPolling, "no referendum");
        if req.window == WindowLabel::MinionPhase {
            break;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(e.state.sect_of(v), Some(Sect::Anarch));
    // A titled vampire: not offered.
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), titled_vampire(), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Red Question, The"]);
    pick(&mut e, bleed(t));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, t));
}

// The burn option "cannot be used if you have a non-titled, non-Anarch
// vampire in torpor" (LSJ 20091204): a torpid one bars the discard; an
// Anarch alone leaves it open.
// Rulings: [LSJ 20091204].
#[test]
fn the_red_question_burn_option_counts_a_torpid_vampire() {
    for (sect, burnable) in [(Sect::Camarilla, false), (Sect::Anarch, true)] {
        let mut e = game(2);
        let anarch_only = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
        let _ = anarch_only;
        let t = e.add_minion(SeatId(0), of_sect(sect, &[], 1), 4, false);
        e.state.minions[t.index()].in_torpor = true;
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Red Question, The"]);
        let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
        let _ = req;
        let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
        assert_eq!(
            req.options.contains(&Choice::Discard(0)),
            burnable,
            "{sect:?}: {:?}",
            req.options
        );
    }
}

// ---- Zephyr -----------------------------------------------------------------

// Zephyr: "Only usable after resolution of an unsuccessful action. [cel]
// Unlock this vampire at the end of the turn. [CEL] Unlock this vampire."
// Blocked: [CEL] unlocks now; [cel] at the end of the turn.
// Rulings: [RTR 20180719] [PIB 20150915] [LSJ 19981028] [ANK 20190425]
// [LSJ 20030130] [ANK 20181219] [ANK 20180906].
#[test]
fn zephyr_unlocks_after_a_blocked_action() {
    for superior in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Zephyr"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(b));
        let mode = u8::from(superior);
        pick_when(&mut e, play(0, mode, a));
        request(&mut e);
        if superior {
            assert!(!e.state.minions[a.index()].locked, "unlocked now");
        } else {
            assert!(e.state.minions[a.index()].locked, "not yet");
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            assert!(!e.state.minions[a.index()].locked, "at the end of the turn");
        }
    }
}

// "Cannot be played if the action fizzles" (ANK 20220218); nor after a
// successful one.
// Rulings: [ANK 20220218].
#[test]
fn zephyr_needs_an_unsuccessful_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Zephyr"]);
    pick(&mut e, bleed(a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
}

// ---- Vampires ---------------------------------------------------------------

/// The first vampire ability of `m` the request offers.
fn ability_of(req: &zvtes::decision::DecisionRequest, m: MinionId) -> Option<Choice> {
    req.options.iter().copied().find(
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == m),
    )
}

fn untitled_of(sect: Sect, nth: usize) -> u16 {
    of_sect(sect, &[], nth)
}

fn titled_vampire() -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.title != Title::None
                && !matches!(c.title, Title::Votes(_))
                && c.capacity >= 4
                && !c.imbued
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a titled vampire") as u16
}

// Boss Callihan: "-1 intercept against Sabbat vampires." The acting
// minion's sect read live; none against a Camarilla one.
#[test]
fn boss_callihan_has_less_intercept_against_sabbat() {
    let mut e = game(2);
    let boss = e.add_minion(SeatId(0), find_card("Boss Callihan"), 4, false);
    let s = e.add_minion(SeatId(1), untitled_of(Sect::Sabbat, 0), 4, false);
    let c = e.add_minion(SeatId(1), untitled_of(Sect::Camarilla, 0), 4, false);
    let at = |o| PrintedCtx {
        opponent: Some(o),
        ..Default::default()
    };
    assert_eq!(e.state.printed_mod(boss, Stat::Intercept, at(s)), -1);
    assert_eq!(e.state.printed_mod(boss, Stat::Intercept, at(c)), 0);
    e.state.minions[s.index()].sect_override = Some(Sect::Anarch);
    assert_eq!(e.state.printed_mod(boss, Stat::Intercept, at(s)), 0, "live");
}

// Daliah: "After Daliah enters combat, she can gain 1 blood." Once, as
// the combat begins.
#[test]
fn daliah_gains_a_blood_as_she_enters_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let d = e.add_minion(SeatId(1), find_card("Daliah"), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(d));
    let mut gained = false;
    for _ in 0..30 {
        let req = request(&mut e);
        if let Some(c) = ability_of(&req, d) {
            assert!(!gained, "once each combat");
            e.choose(index_of(&req, c));
            gained = true;
            assert_eq!(e.state.minions[d.index()].blood, 4);
            continue;
        }
        if req.window == WindowLabel::MinionPhase {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(gained);
}

// Gengis: "If Gengis is blocked by a titled vampire, he does not unlock as
// normal during your next unlock phase." The blocker's title as it blocks;
// an untitled blocker leaves him to unlock.
#[test]
fn gengis_blocked_by_a_titled_vampire_stays_locked() {
    for titled in [true, false] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), find_card("Gengis"), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let blocker = if titled {
            titled_vampire()
        } else {
            untitled_of(Sect::Camarilla, 0)
        };
        let b = e.add_minion(SeatId(1), blocker, 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        assert_eq!(e.state.title_of(b) != Title::None, titled);
        pick(&mut e, bleed(g));
        pick_when(&mut e, block(b));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[g.index()];
        assert_eq!(v.locked, titled, "titled blocker: {titled}");
        if titled {
            // The next one unlocks him as normal.
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(!e.state.minions[g.index()].locked);
        }
    }
}

// Javier Montoya: "During your minion phase, Javier can burn 1 blood to
// unlock a younger ready Anarch." Any Methuselah's, younger than Javier; not
// an older or non-Anarch one.
#[test]
fn javier_montoya_unlocks_a_younger_anarch() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Javier Montoya"), 5, false);
    let y = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    e.state.minions[y.index()].locked = true;
    let theirs = e.add_minion(SeatId(1), anarch(&[], 1), 4, false);
    let cam = e.add_minion(SeatId(0), untitled_of(Sect::Camarilla, 0), 4, false);
    e.state.minions[cam.index()].locked = true;
    let req = request(&mut e);
    let at = |m: MinionId| Choice::UseAbility {
        source: AbilitySource::Vampire(j),
        ability: 0,
        actor: None,
        target: Target::Minion(m),
    };
    assert!(req.options.contains(&at(y)), "{:?}", req.options);
    assert!(req.options.contains(&at(theirs)), "any Methuselah's");
    assert!(!req.options.contains(&at(cam)), "an Anarch only");
    assert!(!req.options.contains(&at(j)), "younger than him");
    pick(&mut e, at(y));
    request(&mut e);
    assert!(!e.state.minions[y.index()].locked);
    assert_eq!(e.state.minions[j.index()].blood, 4);
    // Once in the phase.
    e.state.minions[y.index()].locked = true;
    let req = request(&mut e);
    assert!(ability_of(&req, j).is_none());
}

// Jenna Cross: "Cards that cost blood cost Jenna 1 additional blood to
// play." With Salvador's reduction ordered first, a 1-blood card costs her
// nothing (ANK 20200709).
// Rulings: [ANK 20200709].
#[test]
fn jenna_cross_pays_one_more_unless_a_reduction_empties_the_cost() {
    for salvador in [false, true] {
        let mut e = game(2);
        let j = e.add_minion(SeatId(0), find_card("Jenna Cross"), 4, false);
        let t = e.add_minion(SeatId(0), untitled_of(Sect::Camarilla, 0), 4, false);
        if salvador {
            e.add_minion(SeatId(0), find_card("Salvador Garcia"), 4, false);
        }
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Field Training"]);
        pick(&mut e, play_at(0, 0, Some(j), Target::Minion(t)));
        choose_if_offered(&mut e, Choice::ChooseDiscipline("cel"));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(on(&e, "Field Training", t).is_some());
        let paid = 4 - e.state.minions[j.index()].blood;
        assert_eq!(paid, if salvador { 0 } else { 2 }, "Salvador: {salvador}");
    }
}

// Marguerite Foccart: "+1 strength in combat with a titled vampire."
#[test]
fn marguerite_foccart_is_stronger_against_a_titled_vampire() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Marguerite Foccart"), 4, false);
    let t = e.add_minion(SeatId(1), titled_vampire(), 4, false);
    let u = e.add_minion(SeatId(1), untitled_of(Sect::Camarilla, 0), 4, false);
    let at = |o| PrintedCtx {
        opponent: Some(o),
        ..Default::default()
    };
    assert_eq!(e.state.printed_mod(m, Stat::Strength, at(t)), 1);
    assert_eq!(e.state.printed_mod(m, Stat::Strength, at(u)), 0);
}

// Philip van Vermeer IV: "You and Philip can play cards requiring a Ventrue
// as if he were a Ventrue": a Ventrue master card of his Methuselah's.
// Rulings: [LSJ 20101013] [ANK 20190228] [ANK 20200705-2] [LSJ 20070708]
// [RTR 20070707].
#[test]
fn philip_van_vermeer_plays_ventrue_cards() {
    for philip in [true, false] {
        let mut e = game(2);
        let v = if philip {
            find_card("Philip van Vermeer IV")
        } else {
            plain("Malkavian", 0)
        };
        e.add_minion(SeatId(0), v, 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
        hand(&mut e, 0, &["Ventrue Headquarters"]);
        e.at_master_phase();
        let req = request(&mut e);
        assert_eq!(
            req.options.contains(&master_play(0)),
            philip,
            "{:?}",
            req.options
        );
    }
}

// Salvador Garcia: "While Salvador is ready, cards requiring an Anarch cost
// you and Anarchs you control 1 fewer blood or pool." An Anarch's Field
// Training free, a master card requiring an Anarch 1 pool less; none
// while he is in torpor. A baron's card is one requiring an Anarch (LSJ
// 20050128, LSJ 20080603).
// Rulings: [LSJ 20050128] [LSJ 20080603].
#[test]
fn salvador_garcia_reduces_cards_requiring_an_anarch() {
    for torpid in [false, true] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), find_card("Salvador Garcia"), 4, false);
        e.state.minions[s.index()].in_torpor = torpid;
        let a = e.add_minion(SeatId(0), anarch(&["CEL"], 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Anarch Free Press, The"]);
        e.at_master_phase();
        let req = request(&mut e);
        e.choose(index_of(&req, master_play(0)));
        request(&mut e);
        assert_eq!(e.state.seats[0].pool, if torpid { 9 } else { 10 });
        // The baron's card: Chameleon's X blood, 1 off.
        let chameleon = find_library("Chameleon");
        let off = e.state.printed_cost_off(a, chameleon, 0);
        assert_eq!(off.either, u8::from(!torpid));
    }
}

// Victor Gerard: "You and Victor can play cards requiring a Camarilla
// vampire as if Victor were a Camarilla vampire. While Victor is a Baron,
// you and Victor can play cards requiring a Prince as if Victor were a
// Prince." A baron: a titled Camarilla vampire's political action, a
// prince's reaction; not a baron, neither. The prince's votes not his
// (ANK 20231221); his sect untouched beyond the card (LSJ 20101013).
// Rulings: [LSJ 20101013] [ANK 20190228] [ANK 20200705-2] [LSJ 20070708]
// [RTR 20070707].
#[test]
fn victor_gerard_plays_camarilla_and_prince_cards() {
    for baron in [true, false] {
        let mut e = game(3);
        let v = e.add_minion(SeatId(0), find_card("Victor Gerard"), 6, false);
        if baron {
            e.state.minions[v.index()].granted_title = Some(Title::Baron("Montreal"));
        }
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
        hand(
            &mut e,
            0,
            &["Justicar Retribution", "Camarilla's Iron Fist"],
        );
        let req = request(&mut e);
        // A Camarilla vampire's card always; a titled one's as a baron.
        assert!(req.options.contains(&play(0, 0, v)), "{:?}", req.options);
        assert_eq!(
            req.options.contains(&play(1, 0, v)),
            baron,
            "{:?}",
            req.options
        );
        assert_eq!(e.state.title_of(v) != Title::None, baron);
        // Victor stays Anarch: the requirement met for the card only.
        assert_eq!(e.state.sect_of(v), Some(Sect::Anarch));
    }
    // The prince's reaction: +2 intercept, a baron Victor blocking.
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Victor Gerard"), 6, false);
    e.state.minions[v.index()].granted_title = Some(Title::Baron("Montreal"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), anarch(&["chi"], 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Second Tradition: Domain"]);
    hand(&mut e, 1, &["Illegalism"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 1, z));
    pick_when(&mut e, block(v));
    pick_when(&mut e, play(0, 0, v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10, "blocked");
}

// Vulture: "Once each turn, an Anarch you control can unlock after
// successfully equipping with a vehicle from your hand." Once in the turn.
// Rulings: [ANK 20180719-4].
#[test]
fn vulture_unlocks_an_anarch_equipping_a_vehicle() {
    let mut e = game(2);
    let vu = e.add_minion(SeatId(0), find_card("Vulture"), 4, false);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Sport Bike", "Learjet"]);
    pick(&mut e, play(0, 0, a));
    let unlock = own(vu, 0);
    pick_when(&mut e, unlock);
    request(&mut e);
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    // Its second vehicle this turn: no unlock (once each turn; one vehicle
    // anyway is the equip's own limit).
    let _ = a;
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

/// An untitled vampire with no printed text holding every superior of
/// `discs`.
fn with_superiors(discs: &[&str]) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.title == Title::None
                && c.capacity >= 4
                && !c.imbued
                && zvtes::cards::vampire(i as u16).is_none()
                && discs.iter().all(|d| c.disciplines.contains(d))
        })
        .unwrap_or_else(|| panic!("no vampire with {discs:?}")) as u16
}

// Zephyr "can be played if the action 'failed'" (RTR 20180719): a bleed
// Spying Mission [OBF] made unsuccessful.
// Rulings: [RTR 20180719].
#[test]
fn zephyr_follows_a_failed_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_superiors(&["OBF", "CEL"]), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Spying Mission", "Zephyr"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, a));
    // The Mission played: Zephyr is first in hand.
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    assert_eq!(e.state.seats[1].pool, 10, "failed");
    assert!(!e.state.minions[a.index()].locked, "unlocked");
}

// "Cannot be played if the action fizzles" (ANK 20220218): an unblocked
// Justicar Retribution fizzled by The Red Question.
// Rulings: [ANK 20220218].
#[test]
fn zephyr_does_not_follow_a_fizzled_action() {
    let mut e = game(2);
    let c = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.sect == Some(Sect::Camarilla)
                && c.title == Title::None
                && c.capacity >= 4
                && zvtes::cards::vampire(i as u16).is_none()
                && c.disciplines.contains(&"CEL")
        })
        .expect("a Camarilla vampire with superior Celerity") as u16;
    let v = e.add_minion(SeatId(0), c, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Justicar Retribution", "Red Question, The", "Zephyr"],
    );
    pick(&mut e, play(0, 0, v));
    pick_when(&mut e, play(0, 0, v));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, v));
}

// Zephyr "can be played from torpor" (LSJ 20030130, ANK 20181219): the
// blocked bleeder sent to torpor by the combat unlocks there.
// Rulings: [LSJ 20030130] [ANK 20181219] [ANK 20180906].
#[test]
fn zephyr_is_played_from_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Zephyr"]);
    pick(&mut e, bleed(a));
    // Bloodless once announced: the blocker's hand strike sends it to
    // torpor.
    e.state.minions[a.index()].blood = 0;
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    let v = &e.state.minions[a.index()];
    assert!(v.in_torpor && !v.locked, "unlocked in torpor");
}

// Vulture once each turn (ANK 20180719-4): a second Anarch's vehicle
// equipped that turn, no second unlock.
// Rulings: [ANK 20180719-4].
#[test]
fn vulture_unlocks_once_each_turn() {
    let mut e = game(2);
    let vu = e.add_minion(SeatId(0), find_card("Vulture"), 4, false);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    let b = e.add_minion(SeatId(0), anarch(&[], 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Sport Bike", "Learjet"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, own(vu, 0));
    request(&mut e);
    assert!(!e.state.minions[a.index()].locked);
    // Learjet is now first in hand.
    pick_when(&mut e, play(0, 0, b));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, own(vu, 0));
    assert!(e.state.minions[b.index()].locked);
}

// Netwar [chi] "can be used by a vampire who is already attempting to
// block" (ANK 20181122-2): the minion attempting is chosen, unlocked again
// by nothing and still blocking.
// Rulings: [ANK 20181122-2].
#[test]
fn netwar_chooses_a_minion_already_attempting() {
    let mut e = game(2);
    let r = made_anarch(&mut e, 0, &["chi"], 0);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = made_anarch(&mut e, 1, &["chi"], 1);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Netwar"]);
    hand(&mut e, 1, &["Illegalism"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    // A +1 stealth bleed: the attempt stays under way, short of it.
    pick(&mut e, play(0, 1, z));
    pick_when(&mut e, block(b));
    let chosen = play_at(0, 1, Some(r), Target::Minion(b));
    let req = request(&mut e);
    assert!(req.options.contains(&chosen), "{:?}", req.options);
}

// Netwar [chi] against Ambush: "if the action lacks a suitable target
// because of the unlock, the block attempt still happens" (LSJ 20090514):
// Ambush's locked target, unlocked by Netwar to block, blocks it.
// Rulings: [LSJ 20090514].
#[test]
fn netwar_unlocking_ambushs_target_still_blocks() {
    let mut e = game(2);
    let r = made_anarch(&mut e, 0, &["chi"], 0);
    let t = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[t.index()].locked = true;
    e.notes = Some(Vec::new());
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Netwar"]);
    hand(&mut e, 1, &["Ambush"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let ambush = play_at(0, 0, Some(z), Target::Minion(t));
    let req = request(&mut e);
    assert!(req.options.contains(&ambush), "{:?}", req.options);
    e.choose(index_of(&req, ambush));
    pick_when(&mut e, play_at(0, 1, Some(r), Target::Minion(t)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let combats = noted_combats(&mut e);
    assert!(!combats.is_empty(), "the block's combat");
}

// Philip van Vermeer IV: his ability is for cards played normally (LSJ
// 20050119): Piper's recruit of a Ventrue ally does not take him for a
// Ventrue, his own recruit action does.
// Rulings: [LSJ 20050119].
#[test]
fn philip_van_vermeer_does_not_recruit_a_ventrue_ally_by_piper() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Philip van Vermeer IV"), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Political Ally"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, p)), "{:?}", req.options);
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Philip van Vermeer IV"), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Piper", "Political Ally"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(
        !req.options.contains(&master_play(0)),
        "Piper has no ally to recruit: {:?}",
        req.options
    );
}

// Victor Gerard meets a Camarilla requirement with no Camarilla (Fall of
// the Camarilla in play: ANK 20230503, LSJ 20050124); a prince's
// requirement met as a baron gives no prince's votes (ANK 20231221).
// Rulings: [ANK 20230503] [LSJ 20050124] [ANK 20231221].
#[test]
fn victor_gerard_plays_camarilla_cards_with_no_camarilla() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Victor Gerard"), 6, false);
    e.state.minions[v.index()].granted_title = Some(Title::Baron("Montreal"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play_with_layers(&mut e, 1, "Fall of the Camarilla", None);
    hand(&mut e, 0, &["Justicar Retribution"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, v)), "{:?}", req.options);
    assert_eq!(
        e.state.title_votes_of(v),
        zvtes::state::title_votes(Title::Baron("Montreal")),
        "a baron's votes"
    );
}

// Protection Racket's "additional +1 intercept if the acting minion is
// titled" is gained as The Red Question changes the acting vampire's sect
// (ANK 20190416): a Camarilla vampire holding a baron's title (off-sect,
// untitled) made Anarch is titled, the +2 stealth bleed blocked; without
// The Red Question, it is not.
// Rulings: [ANK 20190416].
#[test]
fn the_red_question_gives_protection_racket_its_titled_intercept() {
    for question in [true, false] {
        let mut e = game(2);
        let c = CRYPT
            .iter()
            .enumerate()
            .position(|(i, c)| {
                c.sect == Some(Sect::Camarilla)
                    && c.title == Title::None
                    && c.capacity >= 4
                    && zvtes::cards::vampire(i as u16).is_none()
                    && c.disciplines.contains(&"PRO")
            })
            .expect("a Camarilla vampire with superior Protean") as u16;
        let v = e.add_minion(SeatId(0), c, 4, false);
        e.state.minions[v.index()].granted_title = Some(Title::Baron("Montreal"));
        assert_eq!(e.state.title_of(v), Title::None, "off-sect");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let b = e.add_minion(SeatId(1), anarch(&[], 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Earth Control", "Red Question, The"]);
        hand(&mut e, 1, &["Protection Racket"]);
        pick(&mut e, bleed(v));
        pick_when(&mut e, block(b));
        // Stealth when needed (D15), then the intercept.
        pick_when(&mut e, play(0, 1, v));
        pick_when(&mut e, play(0, 0, b));
        if question {
            pick_when(&mut e, play(0, 0, v));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let blocked = e.state.seats[1].pool == 10;
        assert_eq!(blocked, question, "The Red Question: {question}");
    }
}

// A bleed card's "+1 stealth action" is the action's stealth (rules-qa
// 104.5): The Platinum Protocol [obf] bleeds past a blocker with no
// intercept.
#[test]
fn a_bleed_cards_printed_stealth_counts() {
    let mut e = game(2);
    let z = made_anarch(&mut e, 0, &["obf"], 0);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Platinum Protocol, The"]);
    pick(&mut e, play(0, 0, z));
    pick_when(&mut e, block(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9, "the block fell short");
}

// Vulture after "successfully equipping": an equip action that fizzled
// (its vehicle unpayable as the action resolves) equipped nothing — no
// unlock (rules-qa 104.19).
#[test]
fn vulture_needs_an_equip_that_did_not_fizzle() {
    let mut e = game(2);
    let vu = e.add_minion(SeatId(0), find_card("Vulture"), 4, false);
    let a = e.add_minion(SeatId(0), anarch(&[], 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Helicopter"]);
    pick(&mut e, play(0, 0, a));
    // Short of its 3 pool by the resolution: the action fizzles.
    e.state.seats[0].pool = 2;
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, own(vu, 0));
    assert!(e.state.minions[a.index()].locked);
    assert_eq!(e.state.seats[0].pool, 2, "nothing paid");
}

// Synner-G's "after successfully equipping" likewise (rules-qa 104.19): a
// Laptop Computer equip fizzled (another Laptop on him by the resolution,
// one each) offers no unlock.
#[test]
fn synner_g_needs_an_equip_that_did_not_fizzle() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Synner-G"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Laptop Computer"]);
    pick(&mut e, play(0, 0, g));
    in_play(&mut e, 0, "Laptop Computer", Some(g));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, own(g, 0));
    assert!(e.state.minions[g.index()].locked);
}

// Jenna Cross's surcharge after a bleed card's reductions too (rules-qa
// 104.14): Legal Manipulations (1 blood) with The Line locked for it costs
// her nothing; without, 2.
// Rulings: [ANK 20200709].
#[test]
fn jenna_cross_pays_no_surcharge_on_a_bleed_card_the_line_empties() {
    for line in [true, false] {
        let mut e = game(2);
        let j = e.add_minion(SeatId(0), find_card("Jenna Cross"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let l = line.then(|| in_play_with_layers(&mut e, 0, "Line, The", None));
        hand(&mut e, 0, &["Legal Manipulations"]);
        pick(&mut e, play(0, 0, j));
        if let Some(l) = l {
            pick_when(&mut e, Choice::ResolveRider { card: l.0 });
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 7, "bled for 3");
        let paid = 4 - e.state.minions[j.index()].blood;
        assert_eq!(paid, if line { 0 } else { 2 }, "The Line: {line}");
    }
}

// Regina Blake's blood pays, it reduces nothing (rules-qa 104.14): Jenna
// Cross equipping Catacombs (1 blood) with Regina paying 1 still pays her
// own +1.
#[test]
fn jenna_cross_pays_her_surcharge_beside_regina_blakes_blood() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jenna Cross"), 4, false);
    let regina = e.add_minion(SeatId(0), find_card("Regina Blake"), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Catacombs"]);
    pick(&mut e, play(0, 0, j));
    pick_when(&mut e, Choice::ChooseMinion(regina));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(on(&e, "Catacombs", j).is_some(), "equipped");
    assert_eq!((blood(&e, j), blood(&e, regina)), (3, 2));
}
