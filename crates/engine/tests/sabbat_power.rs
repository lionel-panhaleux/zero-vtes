//! Sabbat V5, the Path of Power and the Inner Voice: the deck's new cards
//! (`ADDED_BATCHES`), the Sabbat titles (bishop, cardinal, priscus and the
//! prisci block, regent), the path layer (a path icon a play requirement,
//! rules-qa 49.7). Readings in rules-qa Batch 52. Windows with a single
//! option are elided, so the scripts only stop where a real decision
//! exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::decks::Deck;
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay};

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

fn pool(e: &Engine) -> Vec<i32> {
    e.state.seats.iter().map(|s| s.pool).collect()
}

fn use_at(card: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability,
        actor,
        target,
    }
}

fn vampire_ability(m: MinionId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// A card already in play for `seat`, on `host` if given, with its first
/// mode's layers (no play): its id.
fn in_play(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    let card = find_library(name);
    e.state
        .cards
        .push(CardInPlay::new(card, SeatId(seat), host, 0));
    let id = CardId(e.state.cards.len() as u32 - 1);
    for effect in cards::lookup(card).expect("definition").modes[0].effects {
        if let Effect::Layer(modifier) = effect {
            e.state.layers.push(Layer {
                scope: Scope::InPlay,
                modifier: *modifier,
                source: id,
            });
        }
    }
    id
}

/// The printed-name crypt card (the Sabbat names carry their epithets).
fn card(name: &str) -> u16 {
    find_card(name)
}

const AARADHYA: &str = "Aaradhya, The Callous Tyrant";
const VENCESLAVA: &str = "Věnceslava, The Implacable";
const REXTON: &str = "Rexton \"Savage\" Abernathy";
const POWER: &str = "Power and the Inner Voice";

/// Seat 0: `caller` (4 blood) and a spare; the others' given vampires.
fn table(caller: &str) -> (Engine, MinionId) {
    let mut e = game(3);
    let m = e.add_minion(SeatId(0), card(caller), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    (e, m)
}

// ---- The deck, titles, paths ---------------------------------------------------

// The Path of Power and the Inner Voice precon: 12 crypt and 77 library
// cards, every card defined but the vampires printing no text; drawn by its
// path for training (owner, 2026-10-05).
#[test]
fn the_path_of_power_deck_is_complete_and_drawn_by_its_path() {
    let deck = Deck::sabbat(0);
    assert_eq!(deck.name, "Path of Power and the Inner Voice (Sabbat V5)");
    assert_eq!(deck.crypt_cards().len(), 12);
    assert_eq!(deck.library_cards().len(), 77);
    assert_eq!(deck.path(), Some(POWER));
    assert_eq!(deck.category(), POWER);
    for c in deck.library_cards() {
        assert!(cards::lookup(c).is_some(), "{c}");
    }
    let defined = deck
        .crypt_cards()
        .iter()
        .filter(|&&c| cards::vampire(c).is_some())
        .count();
    assert!(defined >= 6, "the vampires printing text");
}

// Rulebook "Other Vampire Sects": bishop 1 vote, cardinal 3, regent 4 (the
// only unique Sabbat title), a priscus none but its prisci ballot; only
// while Sabbat.
#[test]
fn sabbat_titles_vote_and_only_the_regent_contests() {
    let mut e = game(3);
    let concordia = e.add_minion(SeatId(0), card("Concordia"), 4, false);
    let rasha = e.add_minion(SeatId(1), card("Rasha Tarabzouni"), 4, false);
    let aaradhya = e.add_minion(SeatId(0), card(AARADHYA), 4, false);
    let khin = e.add_minion(SeatId(2), card("Khin Aye"), 4, false);
    assert_eq!(e.state.title_of(concordia), Title::Bishop);
    assert_eq!(e.state.title_votes_of(concordia), 1);
    assert!(e.state.title_rivals(rasha).is_empty(), "bishops are many");
    assert_eq!(e.state.title_votes_of(aaradhya), 3);
    assert_eq!(e.state.title_of(khin), Title::Priscus);
    assert_eq!(e.state.title_votes_of(khin), 0);
    assert_eq!(zvtes::state::title_votes(Title::Regent), 4);
    assert_eq!(
        zvtes::state::title_key(Title::Regent),
        Some(zvtes::state::TitleKey::Named("Regent"))
    );
    e.state.minions[concordia.index()].sect_override = Some(Sect::Anarch);
    assert_eq!(e.state.title_of(concordia), Title::None);
}

// The path icon is a play requirement (rules-qa 49.7): Dominant
// Personality, a Path of Power and the Inner Voice minion card, is played
// by a vampire following that path — not a Camarilla vampire with the
// Discipline, nor a follower made non-Sabbat (the path's benefits only
// while Sabbat).
#[test]
fn a_path_icon_requires_a_vampire_following_the_path() {
    let mut e = game(3);
    let lika = e.add_minion(SeatId(0), card("Lika, The Purger"), 3, false);
    let other = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    assert_ne!(
        CRYPT[vampire_with("POT", 0) as usize].sect,
        Some(Sect::Sabbat)
    );
    assert_eq!(e.state.minions[lika.index()].path(), Some(POWER));
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Dominant Personality"]);
    let follower = Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(lika),
        target: Target::None,
    };
    let stranger = Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(other),
        target: Target::None,
    };
    pick(&mut e, bleed(other));
    pick(&mut e, block(z));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, stranger);
    e.state.minions[z.index()].locked = false;
    pick(&mut e, bleed(lika));
    pick(&mut e, block(z));
    let req = offering(&mut e, follower);
    assert_eq!(req.seat, SeatId(0));

    // Not Sabbat any more: no path benefit.
    let mut e = game(3);
    let lika = e.add_minion(SeatId(0), card("Lika, The Purger"), 3, false);
    e.state.minions[lika.index()].sect_override = Some(Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    assert!(!e.state.follows(lika, POWER));
    hand(&mut e, 0, &["Dominant Personality"]);
    pick(&mut e, bleed(lika));
    pick(&mut e, block(z));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(lika),
            target: Target::None,
        },
    );
}

// [LSJ 20011204-3] [PIB 20130704] "When played, a multi-Disciplines card
// counts as requiring the Discipline(s) being used": "[pot] or [pre]" is
// either Discipline (rules-qa 25.2) — Unthinkable Humiliation played by
// Suwira ([pre] only) and by Lika ([pot] only), not by a vampire with
// neither; side by side, "[pot][pre]" is a combo: Iron Glare's +2 votes
// need both.
#[test]
fn an_or_layout_card_is_played_with_either_discipline() {
    for name in ["Suwira", "Lika, The Purger"] {
        let mut e = game(3);
        let m = e.add_minion(SeatId(0), card(name), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
        hand(&mut e, 0, &["Unthinkable Humiliation"]);
        pick(&mut e, bleed(m));
        offering(&mut e, play(0, 0, m));
    }
    assert_eq!(
        zvtes::library::LIBRARY[find_library("Unthinkable Humiliation") as usize].modes[0].tags,
        &["pot", "pre"]
    );
    // Neither Discipline: not playable.
    let neither = CRYPT
        .iter()
        .position(|c| {
            !c.disciplines.is_empty()
                && c.disciplines
                    .iter()
                    .all(|d| !d.eq_ignore_ascii_case("pot") && !d.eq_ignore_ascii_case("pre"))
        })
        .expect("a vampire") as u16;
    let mut e = game(3);
    let m = e.add_minion(SeatId(0), neither, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Unthinkable Humiliation"]);
    pick(&mut e, bleed(m));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, m));

    // Iron Glare: Lika ([pot], no [pre]) cannot add its votes.
    let (mut e, lika) = table("Lika, The Purger");
    hand(&mut e, 0, &["Political Stranglehold", "Iron Glare"]);
    pick(&mut e, play(0, 0, lika));
    to_referendum(&mut e);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play(0, 0, lika),
    );
}

// Rulebook "The Prisci Block": each ready priscus casts a ballot in the
// prisci sub-referendum; the side with more ballots gets the block's 3
// votes. A priscus with no vote still casts its ballot.
#[test]
fn the_prisci_block_gives_its_3_votes_to_the_ballots_majority() {
    let (mut e, caller) = table("Concordia");
    let khin = e.add_minion(SeatId(1), card("Khin Aye"), 4, false);
    hand(&mut e, 0, &["Political Stranglehold"]);
    pick(&mut e, play(0, 0, caller));
    to_referendum(&mut e);
    pick_when(&mut e, votes(khin, false));
    let r = e.referendum().expect("referendum");
    assert_eq!(
        r.tally(&e.state),
        (0, 3),
        "Khin Aye's ballot: the block against"
    );
}

// "Sabbat cardinal: Aaradhya can unlock after performing a successful
// political action (even if the referendum failed). +1 bleed."
#[test]
fn aaradhya_unlocks_after_a_political_action_even_if_the_referendum_failed() {
    let (mut e, aaradhya) = table(AARADHYA);
    let against = e.add_minion(SeatId(1), card("Concordia"), 4, false);
    hand(&mut e, 0, &["Political Stranglehold"]);
    pick(&mut e, play(0, 0, aaradhya));
    to_referendum(&mut e);
    pick_when(&mut e, votes(against, false));
    close_polling(&mut e);
    pick_when(&mut e, vampire_ability(aaradhya, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[aaradhya.index()].locked);
}

// "Sabbat Archbishop of Pittsburgh: While you control a ready cardinal,
// Damian gets +1 bleed."
#[test]
fn damian_bleeds_harder_with_a_ready_cardinal() {
    let (mut e, damian) = table("Damian");
    pick(&mut e, bleed(damian));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9);
    e.add_minion(SeatId(0), card(AARADHYA), 4, false);
    e.state.minions[damian.index()].locked = false;
    e.state.minions[damian.index()].bled_this_turn = false;
    e.state.minions[damian.index()].acted.clear();
    pick(&mut e, bleed(damian));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9 - 2);
}

// [LSJ 20011022] Khin Aye "The passive ability applies even if the vampire
// is in torpor": +1 hand size while the predator controls more ready
// minions.
#[test]
fn khin_aye_gives_hand_size_while_the_predator_has_more_ready_minions() {
    let mut e = game(3);
    e.add_minion(SeatId(0), card("Khin Aye"), 4, true);
    e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
    assert_eq!(
        e.state.hand_size(SeatId(0)),
        8,
        "predator 1, seat 0 0 ready"
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    assert_eq!(e.state.hand_size(SeatId(0)), 7, "1 against 1");
}

// "Allies and vampires with capacity 3 or less cannot block Rexton" —
// a printed passive, read live as the blockers are offered.
#[test]
fn rexton_cannot_be_blocked_by_allies_or_small_vampires() {
    let mut e = game(3);
    let rexton = e.add_minion(SeatId(0), card(REXTON), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let deuce = e.add_ally(SeatId(1), find_library("Double Deuce"));
    let small = e.add_minion(SeatId(1), card("Suwira"), 2, false);
    let big = e.add_minion(SeatId(1), card("Concordia"), 4, false);
    pick(&mut e, bleed(rexton));
    let req = offering(&mut e, block(big));
    assert!(!req.options.contains(&block(deuce)));
    assert!(!req.options.contains(&block(small)));
}

// [LSJ 20011022] Rexton "The passive ability applies even if the vampire
// is in torpor": leaving it, Meaghan (capacity 3) is no blocker even with
// Eagle's Sight to reach the action's stealth; a bigger [aus] vampire is.
#[test]
fn rexton_bars_small_blockers_even_from_torpor() {
    let mut e = game(3);
    let rexton = e.add_minion(SeatId(0), card(REXTON), 4, true);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let small = e.add_minion(SeatId(1), card("Meaghan"), 3, false);
    let big_card = CRYPT
        .iter()
        .position(|c| c.capacity >= 4 && c.disciplines.contains(&"aus") && c.title == Title::None)
        .expect("an [aus] vampire") as u16;
    let big = e.add_minion(SeatId(1), big_card, 4, false);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, action(rexton, ActionKind::LeaveTorpor));
    let req = offering(&mut e, block(big));
    assert!(!req.options.contains(&block(small)));
}

// "Sabbat Archbishop of Budapest: While your prey has 10 or fewer pool,
// Üresség gets +1 vote and +1 bleed."
#[test]
fn uresseg_votes_and_bleeds_more_while_the_prey_is_low() {
    let (mut e, u) = table("Üresség");
    e.state.seats[1].pool = 11;
    assert_eq!(
        e.state
            .printed_mod(u, cards::Stat::Votes, Default::default()),
        0
    );
    e.state.seats[1].pool = 10;
    assert_eq!(
        e.state
            .printed_mod(u, cards::Stat::Votes, Default::default()),
        1
    );
    pick(&mut e, bleed(u));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
}

// "Once each turn, after resolution of an action performed by Věnceslava
// during which your prey burned 1 or more pool, you can gain 1 pool": a
// bleed, a referendum's burn (rules-qa 52.4), not a hunt nor a bleed
// deflected to another. [ANK 20220808] Not if her controller is ousted by
// the referendum effects.
#[test]
fn venceslava_gains_a_pool_after_an_action_her_prey_paid_for() {
    let (mut e, v) = table(VENCESLAVA);
    let gain = vampire_ability(v, 0);
    pick(&mut e, bleed(v));
    pick_when(&mut e, gain);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[..2], [11, 9]);
    // A hunt: the prey burns nothing.
    e.state.minions[v.index()].locked = false;
    e.state.minions[v.index()].blood = 2;
    pick(&mut e, action(v, ActionKind::Hunt));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, gain);

    // A referendum's burn: Empires Fall choosing the prey.
    let (mut e, v) = table(VENCESLAVA);
    hand(&mut e, 0, &["Empires Fall"]);
    pick(&mut e, play(0, 0, v));
    pick_when(&mut e, Choice::ChooseSeat(SeatId(1)));
    to_referendum(&mut e);
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    pick_when(&mut e, gain);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[..2], [11, 9]);

    // [ANK 20220808] "Their ability is not applied if his controller is
    // ousted by the referendum effects": Empires Fall's burn takes her
    // controller's last pool with the prey's.
    let (mut e, v) = table(VENCESLAVA);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
    e.state.seats[0].pool = 1;
    hand(&mut e, 0, &["Empires Fall"]);
    pick(&mut e, play(0, 0, v));
    pick_when(&mut e, Choice::ChooseSeat(SeatId(0)));
    pick_when(&mut e, Choice::ChooseSeat(SeatId(1)));
    to_referendum(&mut e);
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, gain);
    assert!(e.state.seats[0].ousted);

    // Deflected to the predator: not the prey's pool.
    let (mut e, v) = table(VENCESLAVA);
    let concordia = e.add_minion(SeatId(1), card("Concordia"), 4, false);
    hand(&mut e, 1, &["Deflection"]);
    pick(&mut e, bleed(v));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(concordia),
            target: Target::Seat(SeatId(2)),
        },
    );
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, gain);
    assert_eq!(pool(&e), vec![10, 10, 9]);
}

// ---- Library cards -------------------------------------------------------------

/// Seat 0's `a` bleeds, blocked by seat 1's `b`.
fn bleed_blocked(e: &mut Engine, a: MinionId, b: MinionId) {
    pick(e, bleed(a));
    pick(e, block(b));
}

// [LSJ 19980526] [RTR 20041202] [LSJ 20070124] Catatonic Fear [PRE]: "If
// the strike is dodged, no damage is done." [LSJ 20100204-2] "Damage is
// applied after combat, so cannot be prevented by combat cards": the
// opposing vampire's Indomitability is never offered for it. [LSJ
// 20031123] "Any blood loss occurs after combat": no Taste of Vitae
// gains from it.
#[test]
fn catatonic_fear_inflicts_its_damage_after_combat_unless_dodged() {
    let (mut e, aaradhya) = table(AARADHYA);
    let z = e.add_minion(SeatId(1), vampire_with("for", 0), 4, false);
    hand(&mut e, 0, &["Catatonic Fear", "Taste of Vitae"]);
    hand(&mut e, 1, &["Indomitability"]);
    bleed_blocked(&mut e, aaradhya, z);
    pick_when(&mut e, play(0, 1, aaradhya));
    // Taste of Vitae, if offered at all, finds no blood lost in the round.
    let taste = play(0, 0, aaradhya);
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 0, z)), "no prevention");
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        if req.options.contains(&taste) {
            pick(&mut e, taste);
        } else {
            e.choose(fallback(&req));
        }
    }
    assert_eq!(blood(&e, z), 3, "1 damage after combat");
    assert_eq!(
        blood(&e, aaradhya),
        4 - 1,
        "the card's blood, no Taste of Vitae"
    );

    // Dodged: nothing.
    let (mut e, aaradhya) = table(AARADHYA);
    let z = e.add_minion(SeatId(1), vampire_with("cel", 0), 4, false);
    hand(&mut e, 0, &["Catatonic Fear"]);
    hand(&mut e, 1, &["Side Strike"]);
    bleed_blocked(&mut e, aaradhya, z);
    pick_when(&mut e, play(0, 1, aaradhya));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4);
}

// [LSJ 20020314] [PIB 20130319] Dawn Operation "Only applies to damage
// done in combat, not damage done after combat like Catatonic Fear": the
// after-combat damage stays normal — a blood burned, no torpor.
#[test]
fn dawn_operation_does_not_aggravate_catatonic_fears_damage() {
    let (mut e, aaradhya) = table(AARADHYA);
    e.state.minions[aaradhya.index()].blood = 6;
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Dawn Operation", "Catatonic Fear"]);
    pick(&mut e, bleed(aaradhya));
    pick_when(&mut e, play(0, 1, aaradhya));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, aaradhya));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let ash = &e.state.seats[0].ash;
    assert!(
        ash.contains(&find_library("Dawn Operation"))
            && ash.contains(&find_library("Catatonic Fear"))
    );
    assert!(!e.state.minions[z.index()].in_torpor);
    assert_eq!(blood(&e, z), 3);
}

// [RTR 20020501] [LSJ 19980109] "If the combat continues or a new combat
// begins, the after combat effect does not happen": Telepathic Tracking
// starts a new round instead of ending the combat.
#[test]
fn catatonic_fear_does_nothing_if_the_combat_continues() {
    let (mut e, aaradhya) = table(AARADHYA);
    let z = e.add_minion(SeatId(1), vampire_with("AUS", 0), 5, false);
    hand(&mut e, 0, &["Catatonic Fear"]);
    hand(&mut e, 1, &["Telepathic Tracking"]);
    bleed_blocked(&mut e, aaradhya, z);
    pick_when(&mut e, play(0, 1, aaradhya));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Telepathic Tracking's blood, the second round's hand strike: no
    // damage after combat.
    assert_eq!(blood(&e, z), 5 - 1 - 1);
}

// [LSJ 19980107] Día de los Muertos: "During the referendum that passes
// automatically, no 'during a political action' and 'during a referendum'
// effect can be played and effects that operate on the margin get 0": no
// polling step, so no Iron Glare or Private Audience and no vote — [PIB
// 20150105] [LSJ 19980107] no card "usable during the polling step" (Awe,
// Business Pressure and the like); [LSJ 19980107] [PIB 20150105] "the vote
// is considered to have passed by 0 votes": Amici Noctis has no margin to
// distribute.
// The first referendum a Sabbat vampire calls this turn only (rules-qa
// 52.3): the second polls.
#[test]
fn dia_de_los_muertos_passes_the_first_referendum_without_a_polling_step() {
    let (mut e, damian) = table("Damian");
    let lika = e.add_minion(SeatId(0), card("Lika, The Purger"), 3, false);
    let against = e.add_minion(SeatId(1), card("Concordia"), 4, false);
    hand(
        &mut e,
        0,
        &[
            "Political Stranglehold",
            "Iron Glare",
            "Private Audience",
            "Amici Noctis",
            "Empires Fall",
            "Día de los Muertos",
        ],
    );
    e.at_master_phase();
    pick_when(&mut e, master_play(5));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, damian));
    // The hand shifts down: Iron Glare, Private Audience, Amici Noctis.
    let forbidden = [
        play(0, 0, damian),
        play(1, 0, damian),
        votes(against, false),
    ];
    let amici = play(2, 0, damian);
    loop {
        let req = request(&mut e);
        assert_ne!(
            req.window,
            WindowLabel::ReferendumPolling,
            "no polling step"
        );
        for f in &forbidden {
            assert!(!req.options.contains(f), "{f:?} in {:?}", req.window);
        }
        if req.options.contains(&amici) {
            break;
        }
        e.choose(fallback(&req));
    }
    let before = (
        pool(&e),
        e.state.minions.iter().map(|m| m.blood).collect::<Vec<_>>(),
    );
    pick(&mut e, amici);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let after = (
        pool(&e),
        e.state.minions.iter().map(|m| m.blood).collect::<Vec<_>>(),
    );
    assert_eq!(before, after, "a margin of 0");

    // The second referendum of the turn polls.
    pick(&mut e, play(2, 0, lika));
    to_referendum(&mut e);
    offering(&mut e, votes(against, false));
}

/// Under Siege in play for `seat`, with its 3 counters.
fn siege(e: &mut Engine, seat: u8) -> CardId {
    let c = in_play(e, seat, "Under Siege", None);
    e.state.cards[c.index()].counters = 3;
    c
}

// Under Siege: "a Sabbat vampire you control can burn 1 counter from this
// card to unlock and attempt to block with +1 intercept, even if intercept
// is not yet needed": a locked one blocks the bleed. [PIB 20150216] "Can be
// used for intercept even if the vampire is already unlocked." Once each
// action.
#[test]
fn under_siege_unlocks_a_sabbat_vampire_to_block() {
    for locked in [true, false] {
        let (mut e, aaradhya) = table(AARADHYA);
        let concordia = e.add_minion(SeatId(1), card("Concordia"), 4, false);
        e.state.minions[concordia.index()].locked = locked;
        let us = siege(&mut e, 1);
        let unlock = use_at(us, 0, Some(concordia), Target::None);
        pick(&mut e, bleed(aaradhya));
        pick_when(&mut e, unlock);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.cards[us.index()].counters, 2, "locked {locked}");
        // Blocked: no bleed, the combat's hand strikes.
        assert_eq!(e.state.seats[1].pool, 10, "locked {locked}");
        assert_eq!(blood(&e, aaradhya), 3);
    }
}

// [ANK 20181122-2] Under Siege "Can be used by a vampire who is already
// attempting to block": Concordia's attempt, met by Relentlessness's +1
// stealth, gets its +1 intercept.
#[test]
fn under_siege_helps_a_vampire_already_attempting() {
    let (mut e, aaradhya) = table(AARADHYA);
    e.state.minions[aaradhya.index()].blood = 6;
    let concordia = e.add_minion(SeatId(1), card("Concordia"), 4, false);
    let us = siege(&mut e, 1);
    hand(&mut e, 0, &["Relentlessness"]);
    pick(&mut e, bleed(aaradhya));
    pick(&mut e, block(concordia));
    pick_when(&mut e, play(0, 0, aaradhya));
    pick_when(&mut e, use_at(us, 0, Some(concordia), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "1 intercept against 1 stealth");
}

// [LSJ 20010714-2] Under Siege "Cannot be used if the vampire cannot
// attempt to block the action (eg. Daring the Dawn)". [ANK 20180623] "Does
// not ignore the normal prey, predator or target restrictions": a bleed
// of the next seat is not the third seat's to block.
#[test]
fn under_siege_keeps_the_blocking_restrictions() {
    let (mut e, aaradhya) = table(AARADHYA);
    let concordia = e.add_minion(SeatId(1), card("Concordia"), 4, false);
    let us = siege(&mut e, 1);
    hand(&mut e, 0, &["Daring the Dawn"]);
    pick(&mut e, bleed(aaradhya));
    pick_when(&mut e, play(0, 0, aaradhya));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_at(us, 0, Some(concordia), Target::None),
    );

    let mut e = game(4);
    let aaradhya = e.add_minion(SeatId(0), card(AARADHYA), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let across = e.add_minion(SeatId(2), card("Concordia"), 4, false);
    let us = siege(&mut e, 2);
    pick(&mut e, bleed(aaradhya));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_at(us, 0, Some(across), Target::None),
    );
}

// Daring the Dawn [FOR] "Vampires cannot block this action", played into a
// vampire's block attempt under way: the attempt fails (owner, sc-bot
// playtest: "the block happened it should not have"; rules-qa 92.1), the
// bleed lands and Aaradhya takes its damage after resolution. An ally's
// attempt is untouched: it blocks.
#[test]
fn daring_the_dawn_fails_a_vampires_attempt_under_way_not_an_allys() {
    for ally in [false, true] {
        let (mut e, aaradhya) = table(AARADHYA);
        let blocker = if ally {
            e.add_ally(SeatId(1), find_library("Political Ally"))
        } else {
            e.add_minion(SeatId(1), plain("Brujah", 1), 4, false)
        };
        hand(&mut e, 0, &["Daring the Dawn"]);
        pick(&mut e, bleed(aaradhya));
        pick_when(&mut e, Choice::DeclareBlock { minion: blocker });
        pick_when(&mut e, play(0, 1, aaradhya));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if ally {
            assert_eq!(e.state.seats[1].pool, 10, "the ally blocks");
            continue;
        }
        assert!(e.state.seats[1].pool < 10, "unblocked: the bleed lands");
        assert!(
            e.state.minions[aaradhya.index()].in_torpor,
            "Daring the Dawn's aggravated damage after resolution"
        );
    }
}

// Faceless Night [OBF] "minions who attempt to block this action and fail
// become locked before action resolution": an attempt failed by Daring the
// Dawn [FOR] played into it is one (rules-qa 92.2; LSJ 20081202): the
// blocker is locked.
#[test]
fn faceless_night_locks_a_vampire_whose_attempt_daring_the_dawn_fails() {
    let (mut e, sree) = table("Sreelekha");
    let blocker = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Faceless Night", "Daring the Dawn"]);
    pick(&mut e, bleed(sree));
    pick_when(&mut e, Choice::DeclareBlock { minion: blocker });
    pick_when(&mut e, play(0, 1, sree));
    pick_when(&mut e, play(0, 1, sree));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].pool < 10, "unblocked");
    assert!(e.state.minions[blocker.index()].locked, "a failed attempt");
}

// [LSJ 20091224] Black Forest Base "Cannot be attempted twice even if the
// first attempt was blocked and failed": used once, no other Sabbat
// vampire's this turn. [ANK 20180719-4] "'Once each turn' means once
// during each player's turn": the next Methuselah's Sabbat vampire uses it
// on their turn — another's location (any Methuselah's Sabbat vampire).
#[test]
fn black_forest_base_is_used_once_each_turn_by_any_sabbat_vampire() {
    let (mut e, concordia) = table("Concordia");
    let rasha = e.add_minion(SeatId(0), card("Rasha Tarabzouni"), 4, false);
    let khin = e.add_minion(SeatId(1), card("Khin Aye"), 4, false);
    let base = in_play(&mut e, 0, "Black Forest Base", None);
    let by = |m| use_at(base, 0, Some(m), Target::None);
    pick(&mut e, by(concordia));
    to_referendum(&mut e);
    pick_when(&mut e, votes(concordia, true));
    close_polling(&mut e);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 12, "Concordia's vote: 2 pool");
    assert!(!req.options.contains(&by(rasha)), "once this turn");
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&by(khin)), "the next turn");
}

// War of Ages: requires a titled Sabbat vampire; in play, the prey burns 1
// pool in their unlock phase; ousting them, the card burns for 3 more pool.
// [LSJ 20100129] [ANK 20191228] Unique, it cannot be played while in play
// (its effect could not happen).
#[test]
fn war_of_ages_drains_the_prey_and_pays_for_its_oust() {
    let (mut e, aaradhya) = table(AARADHYA);
    let concordia = e.add_minion(SeatId(0), card("Concordia"), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
    let war = find_library("War of Ages");
    hand(&mut e, 0, &["War of Ages", "War of Ages"]);
    pick(&mut e, play(0, 0, aaradhya));
    to_referendum(&mut e);
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards.iter().any(|c| c.in_play && c.card == war));
    assert!(
        !req.options.contains(&play(0, 0, concordia)),
        "unique, in play"
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9, "the prey's unlock phase");

    // The prey ousted by it: 3 more pool, the card burned.
    let (mut e, aaradhya) = table(AARADHYA);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(2), plain("Brujah", 2), 4, false);
    hand(&mut e, 0, &["War of Ages"]);
    pick(&mut e, play(0, 0, aaradhya));
    to_referendum(&mut e);
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.seats[1].pool = 1;
    let before = e.state.seats[0].pool;
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert_eq!(
        e.state.seats[0].pool,
        before + 6 + 3,
        "the oust's 6, the card's 3"
    );
    assert!(!e.state.cards.iter().any(|c| c.in_play && c.card == war));
}

// Cardinal Benediction: [LSJ 20100129] [ANK 20191228] not playable without
// a Sabbat vampire of capacity 7 or more to choose; passed, the chosen
// vampire is a cardinal. "Non-Sabbat vampires cannot cast votes or
// ballots": a Camarilla primogen's are not offered.
#[test]
fn cardinal_benediction_makes_a_big_sabbat_vampire_a_cardinal() {
    let (mut e, concordia) = table("Concordia");
    hand(&mut e, 0, &["Cardinal Benediction"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play(0, 0, concordia)),
        "no one to choose"
    );

    let (mut e, concordia) = table("Concordia");
    let hartmut = e.add_minion(SeatId(1), card("Hartmut Stover"), 4, false);
    let primogen = CRYPT
        .iter()
        .position(|c| c.title == Title::Primogen)
        .expect("a primogen") as u16;
    let p = e.add_minion(SeatId(2), primogen, 4, false);
    hand(&mut e, 0, &["Cardinal Benediction"]);
    // Hartmut, the only choice: elided.
    pick(&mut e, play(0, 0, concordia));
    to_referendum(&mut e);
    loop {
        let req = request(&mut e);
        if req.window != WindowLabel::ReferendumPolling {
            break;
        }
        assert!(!req.options.contains(&votes(p, false)), "not Sabbat");
        let mine = votes(concordia, true);
        if req.options.contains(&mine) {
            pick(&mut e, mine);
            continue;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(
        e.state.title_of(hartmut),
        Title::Cardinal,
        "Concordia's vote passes it"
    );
}

// Regent: [LSJ 20070808-2] "If the title is contested, the card is turned
// face down out of play and the provided action cannot be used": two
// regents contest; uncontested, another Methuselah's Sabbat vampire may
// enter combat with the regent.
#[test]
fn a_contested_regent_provides_no_action() {
    let mut e = game(3);
    let concordia = e.add_minion(SeatId(0), card("Concordia"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let hartmut = e.add_minion(SeatId(1), card("Hartmut Stover"), 4, false);
    let regent = in_play(&mut e, 1, "Regent", Some(hartmut));
    let enter = Choice::UseAbility {
        source: AbilitySource::Card(regent),
        ability: 0,
        actor: Some(concordia),
        target: Target::Minion(hartmut),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&enter), "{:?}", req.options);
    assert_eq!(e.state.title_of(hartmut), Title::Regent);

    let aaradhya = e.add_minion(SeatId(2), card(AARADHYA), 4, false);
    in_play(&mut e, 2, "Regent", Some(aaradhya));
    e.stage_contests(aaradhya);
    assert_eq!(e.state.title_of(hartmut), Title::None, "contested");
    let req = request(&mut e);
    assert!(!req.options.contains(&enter));
}

// No contest with oneself (advanced rules §1.13.1; rules-qa 100.1): Regent
// is not put on one's own vampire while another of one's own is the
// regent; it is when another Methuselah's vampire is (the two contest).
#[test]
fn regent_is_not_played_beside_its_methuselahs_own_regent() {
    let regents: Vec<u16> = CRYPT
        .iter()
        .enumerate()
        .filter(|(_, c)| c.sect == Some(Sect::Sabbat) && c.capacity >= 8 && c.title == Title::None)
        .map(|(i, _)| i as u16)
        .take(2)
        .collect();
    for holder in [0, 1] {
        let mut e = game(3);
        let t = e.add_minion(SeatId(0), regents[0], 4, false);
        let regent = e.add_minion(SeatId(holder), regents[1], 4, false);
        e.add_minion(SeatId(2), plain("Brujah", 0), 4, false);
        in_play(&mut e, holder, "Regent", Some(regent));
        assert_eq!(e.state.title_of(regent), Title::Regent);
        hand(&mut e, 0, &["Regent"]);
        e.at_master_phase();
        let second = Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(t),
        };
        let req = request(&mut e);
        assert_eq!(
            req.options.contains(&second),
            holder == 1,
            "regent held by seat {holder}: {:?}",
            req.options
        );
    }
}

// [ANK 20251125] [LSJ 20050221-2] Absolute Tyranny [POT][PRE]: "A vampire
// can have net negative votes, but net negative votes do not subtract from
// the total when votes are tallied": two copies (Aaradhya's modifier,
// Khin Aye's reaction) take Hartmut's 2 votes cast to 0 and a primogen's 1
// to -1, counted 0; the -1 reaches votes already cast (rules-qa 52.2).
#[test]
fn absolute_tyranny_reduces_votes_never_below_nothing() {
    let (mut e, aaradhya) = table(AARADHYA);
    let hartmut = e.add_minion(SeatId(1), card("Hartmut Stover"), 4, false);
    let khin = e.add_minion(SeatId(1), card("Khin Aye"), 4, false);
    let primogen = CRYPT
        .iter()
        .position(|c| c.title == Title::Primogen)
        .expect("a primogen") as u16;
    let p = e.add_minion(SeatId(2), primogen, 4, false);
    hand(&mut e, 0, &["Political Stranglehold", "Absolute Tyranny"]);
    hand(&mut e, 1, &["Absolute Tyranny"]);
    pick(&mut e, play(0, 0, aaradhya));
    to_referendum(&mut e);
    pick_when(&mut e, votes(hartmut, false));
    pick_when(&mut e, votes(p, false));
    assert_eq!(e.referendum().expect("referendum").tally(&e.state).1, 3);
    pick_when(&mut e, play(0, 1, aaradhya));
    pick_when(&mut e, play(0, 1, khin));
    request(&mut e);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, hartmut), 0);
    assert_eq!(r.tally(&e.state).1, 0, "-1 counted 0");
}

// [ANK 20251125] [LSJ 20050221-2] "A vampire can have net negative votes":
// the -1 is netted against the votes gained, never a floor at 0 first.
// Khin Aye's Absolute Tyranny [POT][PRE] gives Stavros (a priscus: no
// votes, a ballot) -1, then his Awe (X = 0) +1: 0 votes (rules-qa 90.3).
#[test]
fn absolute_tyranny_keeps_a_net_negative_against_later_votes() {
    let (mut e, stavros) = table("Stavros");
    let khin = e.add_minion(SeatId(1), card("Khin Aye"), 4, false);
    hand(&mut e, 0, &["Political Stranglehold", "Awe"]);
    hand(&mut e, 1, &["Absolute Tyranny"]);
    pick(&mut e, play(0, 0, stavros));
    to_referendum(&mut e);
    pick_when(&mut e, play(0, 1, khin));
    request(&mut e);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, stavros), 0, "-1, counted 0");
    pick_when(&mut e, play(0, 0, stavros));
    pick(&mut e, Choice::SetX(0));
    request(&mut e);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, stavros), 0, "-1 + 1, not 0 + 1");
}

// [LSJ 20020612] Unthinkable Humiliation "Can be played any time during
// the action before resolution; a block attempt is not required": played
// before any block, the blocker then burns 1 blood to attempt.
#[test]
fn unthinkable_humiliation_makes_blockers_pay_to_attempt() {
    let (mut e, aaradhya) = table(AARADHYA);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Unthinkable Humiliation"]);
    pick(&mut e, bleed(aaradhya));
    pick_when(&mut e, play(0, 0, aaradhya));
    pick_when(&mut e, block(z));
    assert_eq!(blood(&e, z), 3, "1 blood to attempt");
}

// Powerbase: Madrid: locked during a polling step, a titled Sabbat vampire
// gets +1 vote for each of its counters — any titled Sabbat vampire, its
// controller's to cast; a counter added in its controller's unlock phase
// while it has 3 or fewer.
#[test]
fn powerbase_madrid_gives_a_titled_sabbat_vampire_its_counters_in_votes() {
    let (mut e, concordia) = table("Concordia");
    let madrid = in_play(&mut e, 0, "Powerbase: Madrid", None);
    e.state.cards[madrid.index()].counters = 2;
    hand(&mut e, 0, &["Political Stranglehold"]);
    pick(&mut e, play(0, 0, concordia));
    to_referendum(&mut e);
    pick_when(&mut e, use_at(madrid, 1, None, Target::Minion(concordia)));
    request(&mut e);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, concordia), 1 + 2);
    assert!(e.state.cards[madrid.index()].locked);
}

// Privileged Position: after a referendum called by one's vampire passes,
// lock it to burn 1 pool from the prey.
#[test]
fn privileged_position_burns_a_prey_pool_after_a_passed_referendum() {
    let (mut e, concordia) = table("Concordia");
    let pp = in_play(&mut e, 0, "Privileged Position", None);
    hand(&mut e, 0, &["Political Stranglehold"]);
    pick(&mut e, play(0, 0, concordia));
    to_referendum(&mut e);
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    let lock = use_at(pp, 0, None, Target::None);
    offering(&mut e, lock);
    let before = e.state.seats[1].pool;
    pick(&mut e, lock);
    request(&mut e);
    assert_eq!(e.state.seats[1].pool, before - 1);
}

// Magnetic Authority [DOM] or [POT]: after a passed political action, 1
// blood to each Sabbat vampire in one's uncontrolled region — not another
// sect's.
#[test]
fn magnetic_authority_feeds_the_uncontrolled_sabbat_vampires() {
    let (mut e, concordia) = table("Concordia");
    for name in ["Lika, The Purger", "Suwira"] {
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card: card(name),
                copy: Default::default(),
                counters: 0,
                banished: None,
            });
    }
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: plain("Brujah", 2),
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    hand(&mut e, 0, &["Political Stranglehold", "Magnetic Authority"]);
    pick(&mut e, play(0, 0, concordia));
    to_referendum(&mut e);
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    pick_when(&mut e, play(0, 1, concordia));
    request(&mut e);
    let counters: Vec<u8> = e.state.seats[0]
        .uncontrolled
        .iter()
        .map(|u| u.counters)
        .collect();
    assert_eq!(counters, vec![1, 1, 0]);
}

// Unholy Sacrament: 3 blood to a titled Sabbat vampire in one's
// uncontrolled region — Concordia the bishop, not untitled Lika.
#[test]
fn unholy_sacrament_feeds_an_uncontrolled_titled_sabbat_vampire() {
    let (mut e, _) = table("Damian");
    for name in ["Concordia", "Lika, The Purger"] {
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card: card(name),
                copy: Default::default(),
                counters: 0,
                banished: None,
            });
    }
    hand(&mut e, 0, &["Unholy Sacrament"]);
    e.at_master_phase();
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let on = |i| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Uncontrolled(i),
    };
    assert!(!req.options.contains(&on(1)));
    pick(&mut e, on(0));
    request(&mut e);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 3);
}

// ---- Paths ------------------------------------------------------------------

// Black Chantry, 2025-06-12 (rules-qa 97.1, 97.2): in the influence phase,
// a Sabbat vampire following no path burns 1 blood to follow one of the
// four; a vampire following one, not Sabbat, or with no blood is offered
// none; once followed, it is offered none again.
#[test]
fn a_sabbat_vampire_following_no_path_follows_one_for_a_blood() {
    let mut e = game(3);
    let hartmut = e.add_minion(SeatId(0), find_card("Hartmut Stover"), 3, false);
    let pathed = e.add_minion(
        SeatId(0),
        find_card("Aaradhya, The Callous Tyrant"),
        3,
        false,
    );
    let ventrue = e.add_minion(SeatId(0), plain("Ventrue", 0), 3, false);
    let dry = e.add_minion(SeatId(0), find_card("Styles Margs"), 0, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 3, false);
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let offered: Vec<(MinionId, u8)> = req
        .options
        .iter()
        .filter_map(|o| match *o {
            Choice::FollowPath { minion, path } => Some((minion, path)),
            _ => None,
        })
        .collect();
    assert_eq!(offered, (0..4).map(|p| (hartmut, p)).collect::<Vec<_>>());
    for m in [pathed, ventrue, dry] {
        assert!(offered.iter().all(|&(o, _)| o != m), "{m:?}");
    }
    let power = zvtes::state::SABBAT_PATHS
        .iter()
        .position(|&p| p == "Power and the Inner Voice")
        .unwrap() as u8;
    pick(
        &mut e,
        Choice::FollowPath {
            minion: hartmut,
            path: power,
        },
    );
    assert_eq!(e.state.minions[hartmut.index()].blood, 2);
    assert!(e.state.follows(hartmut, "Power and the Inner Voice"));
    let req = request(&mut e);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::FollowPath { .. })));
}

// The four paths are the crypt's (Black Chantry, 2025-06-12).
#[test]
fn the_sabbat_paths_are_the_crypts() {
    let mut printed: Vec<&str> = CRYPT.iter().filter_map(|c| c.path).collect();
    printed.sort();
    printed.dedup();
    let mut paths = zvtes::state::SABBAT_PATHS.to_vec();
    paths.sort();
    assert_eq!(printed, paths);
}

// rules-qa 97.1, 97.4: a torpid Sabbat vampire with blood, and a library
// card as a Sabbat vampire, follow a path as any.
#[test]
fn a_torpid_or_created_sabbat_vampire_follows_a_path() {
    let mut e = game(3);
    let torpid = e.add_minion(SeatId(0), find_card("Styles Margs"), 2, true);
    let created = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    let childe = find_library("Childe of the Revolution");
    e.state
        .cards
        .push(CardInPlay::new(childe, SeatId(0), None, 0));
    e.state.minions[created.index()].kind = zvtes::state::MinionKind::Created {
        card: childe,
        id: CardId(e.state.cards.len() as u32 - 1),
        clan: Some("Brujah"),
        sect: Sect::Sabbat,
        born: &zvtes::state::NEWBORN,
    };
    e.add_minion(SeatId(1), plain("Ventrue", 1), 3, false);
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    for m in [torpid, created] {
        assert!(
            req.options
                .contains(&Choice::FollowPath { minion: m, path: 0 }),
            "{m:?}"
        );
    }
    pick(
        &mut e,
        Choice::FollowPath {
            minion: created,
            path: 1,
        },
    );
    assert!(e.state.follows(created, zvtes::state::SABBAT_PATHS[1]));
    let req = request(&mut e);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::FollowPath { minion, .. } if *minion == created)));
}
