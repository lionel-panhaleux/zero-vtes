//! Legacy cards for TWDA decks, round 11 (`ADDED_BATCHES`, the
//! twenty-first batch, slice 16w): the cards that bring the most 2025-2026
//! TWDA decks into the kept corpus. Readings in rules-qa Batch 71. Windows
//! with a single option are elided, so the scripts only stop where a real
//! decision exists.

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
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

fn use_at(card: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability,
        actor,
        target,
    }
}

/// A vampire's own ability `ability` used by itself.
fn printed(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: Some(m),
        target,
    }
}

/// A vampire's own ability `ability` used outside its actions (no actor).
fn own_use(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: None,
        target,
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
    for effect in zvtes::cards::lookup(card).expect("definition").modes[0].effects {
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

/// `m` gets `disc` at superior level (the fixture's Great Beast slot).
fn superior(e: &mut Engine, m: MinionId, disc: &'static str) {
    let slots = &mut e.state.minions[m.index()].beast_disciplines;
    let free = slots.iter().position(Option::is_none).expect("a free slot");
    slots[free] = Some(disc);
}

/// A card play by `actor` at `target`.
fn play_at(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

/// The `nth` plain vampire (no printed text, untitled, capacity 4 or more)
/// with `disc` (any, if empty), of `sect` if given.
fn with_disc(disc: &str, sect: Option<Sect>, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            sect.is_none_or(|s| c.sect == s)
                && (disc.is_empty() || c.disciplines.contains(&disc))
                && c.title == Title::None
                && c.capacity >= 4
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no plain vampire with {disc}"))
}

/// An uncontrolled vampire of `card`, no blood.
fn uncontrolled(card: u16) -> zvtes::state::UncontrolledSlot {
    zvtes::state::UncontrolledSlot {
        card,
        copy: Default::default(),
        counters: 0,
        banished: None,
    }
}

fn stat(e: &Engine, m: MinionId, s: Stat) -> i32 {
    e.state.printed_mod(m, s, PrintedCtx::default())
}

// ---- Vampires -----------------------------------------------------------------

// Gustaphe Brunnelle (G4): "Camarilla primogen: Gustaphe can enter combat
// with a minion controlled by your predator or prey as a Ⓓ action." Not a
// minion across the table.
#[test]
fn gustaphe_enters_combat_with_his_predators_or_preys_minion() {
    let mut e = game(4);
    let g = e.add_minion(SeatId(0), find_card("Gustaphe Brunnelle"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let prey = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let across = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let pred = e.add_minion(SeatId(3), plain("Toreador", 0), 4, false);
    let req = request(&mut e);
    let at = |m: MinionId| printed(g, 0, Target::Minion(m));
    assert!(req.options.contains(&at(prey)));
    assert!(req.options.contains(&at(pred)));
    assert!(!req.options.contains(&at(across)));
}

// Venere Carboni (G3): "Sabbat priscus: If Venere is in torpor during your
// unlock phase, he may burn 1 blood to move to your ready region."
#[test]
fn venere_carboni_leaves_torpor_for_a_blood() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let v = e.add_minion(SeatId(1), find_card("Venere Carboni"), 3, true);
    let up = own_use(v, 0, Target::None);
    pick_when(&mut e, up);
    request(&mut e);
    let m = &e.state.minions[v.index()];
    assert!(!m.in_torpor);
    assert_eq!(m.blood, 2);
}

// Josef von Bauren (G4): "Camarilla Nosferatu Inner Circle: You can use a
// discard phase action to discard 1 card at random from your predator or
// prey's hand. +1 bleed. +1 stealth." [ANK 20190725] "His ability can be
// used multiple times if one has multiple discard phase actions": a
// discard phase action each use; the prey draws back up.
#[test]
fn josef_von_bauren_discards_at_random_per_discard_phase_action() {
    let mut e = game(3);
    let j = e.add_minion(SeatId(0), find_card("Josef von Bauren"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    assert_eq!(stat(&e, j, Stat::Bleed), 1);
    assert_eq!(stat(&e, j, Stat::Stealth), 1);
    fill_hand(&mut e, 1);
    e.state.seats[1].library = vec![find_library("Conceal"); 4].into();
    let at = |s: u8| own_use(j, 0, Target::Seat(SeatId(s)));
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(req.options.contains(&at(1)) && req.options.contains(&at(2)));
    e.state.seats[0].dpa = 2;
    let ash = e.state.seats[1].ash.len();
    pick(&mut e, at(1));
    pick(&mut e, at(1));
    let req = request(&mut e);
    assert!(
        !req.options.contains(&at(1)),
        "no discard phase action left"
    );
    assert_eq!(e.state.seats[1].ash.len(), ash + 2);
    assert_eq!(e.state.hand_load(SeatId(1)), e.state.hand_size(SeatId(1)));
}

// Gabrielle di Righetti (G5): "Camarilla Tremere Justicar: Gabrielle can
// steal 2 blood or life (becoming blood) from a ready minion as a +1
// stealth Ⓓ action." An ally with 2 life left burns; she gains the 2.
#[test]
fn gabrielle_steals_an_allys_last_life_and_burns_it() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Gabrielle di Righetti"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[a.index()].blood = 2;
    pick(&mut e, printed(g, 0, Target::Minion(a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_play);
    assert_eq!(blood(&e, g), 6);
}

// Harold Tanner (G4): "Camarilla: The controller of the minion in combat
// with Harold plays with an open hand." Seat 1 sees seat 0's hand while
// Harold blocks seat 0's minion, not before.
#[test]
fn harold_tanner_opens_his_opponents_hand_in_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), find_card("Harold Tanner"), 4, false);
    hand(&mut e, 0, &["Immortal Grapple"]);
    assert!(!e.state.hand_visible_to(SeatId(0), SeatId(1)));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(h));
    request(&mut e);
    assert!(e.combat().is_some());
    assert!(e.state.hand_visible_to(SeatId(0), SeatId(1)));
}

// Giotto Verducci (G2): "Independent: During another Methuselah's discard
// phase, you can discard one card (draw up afterward). Vampires must burn 1
// blood to attempt to block Giotto. Infernal." The discard in that phase,
// once; the toll a vampire's, not an ally's.
#[test]
fn giotto_discards_in_others_discard_phases_and_tolls_vampire_blockers() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(1), find_card("Giotto Verducci"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    fill_hand(&mut e, 1);
    e.state.seats[1].library = vec![find_library("Conceal"); 4].into();
    let discard = own_use(g, 0, Target::None);
    let req = offering(&mut e, discard);
    assert_eq!(req.seat, SeatId(1));
    assert_eq!(e.state.active, SeatId(0));
    let ash = e.state.seats[1].ash.len();
    // The cards alike: the one to discard needs no choosing.
    e.choose(index_of(&req, discard));
    let req = request(&mut e);
    assert!(!req.options.contains(&discard), "once in that phase");
    assert_eq!(e.state.seats[1].ash.len(), ash + 1);
    assert_eq!(e.state.hand_load(SeatId(1)), e.state.hand_size(SeatId(1)));
}

// Giotto Verducci: the toll on a vampire attempting to block him, not on an
// ally.
#[test]
fn giotto_tolls_vampire_blockers_only() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(1), find_card("Giotto Verducci"), 4, false);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a = e.add_ally(SeatId(0), find_library("Political Ally"));
    let ctx = PrintedCtx {
        opponent: Some(g),
        ..Default::default()
    };
    assert_eq!(e.state.printed_mod(v, Stat::BlockToll, ctx), 1);
    assert_eq!(e.state.printed_mod(a, Stat::BlockToll, ctx), 0);
}

// Leandro (G2): "Camarilla Malkavian Inner Circle: During each other
// Methuselah's unlock phase, if Leandro is ready, that Methuselah chooses
// either to burn 1 pool or to lose all transfers during his or her next
// influence phase. +2 bleed." [LSJ 20021210-1] "Choice is mandatory, there
// is no default": both offered, nothing else.
#[test]
fn leandro_makes_others_burn_a_pool_or_lose_their_transfers() {
    for burn in [true, false] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), find_card("Leandro"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        assert_eq!(stat(&e, l, Stat::Bleed), 2);
        e.state.seats[1]
            .uncontrolled
            .push(uncontrolled(plain("Ventrue", 1)));
        // The rider their only pending effect: straight to the choice.
        let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
        assert_eq!(req.options, vec![Choice::BurnPool, Choice::Pass]);
        e.choose(index_of(
            &req,
            if burn { Choice::BurnPool } else { Choice::Pass },
        ));
        let mut left = None;
        loop {
            let req = request(&mut e);
            if req.window == WindowLabel::InfluenceTransfers {
                left = e.transfers_left();
            }
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            e.choose(fallback(&req));
        }
        if burn {
            assert_eq!(pool(&e)[1], 10 - 1);
            assert!(left.is_some_and(|n| n > 0));
        } else {
            assert_eq!(pool(&e)[1], 10);
            assert!(left.is_none_or(|n| n == 0), "{left:?}");
            assert!(!e.state.seats[1].no_transfers_next, "spent");
        }
    }
}

// Nergal (G5 ADV): "Independent: While Nergal is acting, reaction cards
// cost +1 blood. During an action, he can burn 1 blood to get +1 stealth.
// Infernal." [PIB 20150924-2] "Card with multiple types (eg. Reaction/Action
// Modifier) are considered of the type they are played as, not both": a
// reaction costs +1 while he acts, not before.
#[test]
fn nergal_advanced_taxes_reactions_while_he_acts() {
    let mut e = game(2);
    let adv = CRYPT
        .iter()
        .position(|c| c.name == "Nergal" && c.adv)
        .expect("Nergal (ADV)") as u16;
    let n = e.add_minion(SeatId(0), adv, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), with_disc("aus", None, 0), 4, false);
    let cost = |e: &Engine| {
        e.state.printed_mod(
            z,
            Stat::BloodCostMore,
            PrintedCtx {
                card: Some((find_library("Telepathic Counter"), 0)),
                reacting: true,
                ..Default::default()
            },
        )
    };
    assert_eq!(cost(&e), 0);
    pick(&mut e, bleed(n));
    request(&mut e);
    assert_eq!(cost(&e), 1);
}

// Nergal (G5 ADV): his stealth, burned for as needed.
#[test]
fn nergal_advanced_burns_blood_for_stealth_when_needed() {
    let mut e = game(2);
    let adv = CRYPT
        .iter()
        .position(|c| c.name == "Nergal" && c.adv)
        .expect("Nergal (ADV)") as u16;
    let n = e.add_minion(SeatId(0), adv, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(n));
    let stealth = own_use(n, 0, Target::None);
    let req = offering(&mut e, Choice::DeclareBlock { minion: z });
    assert!(!req.options.contains(&stealth));
    e.choose(index_of(&req, block(z)));
    pick_when(&mut e, own_use(n, 0, Target::None));
    assert_eq!(blood(&e, n), 3);
}

// ---- Library cards ------------------------------------------------------------

/// A crypt card of `clan` with capacity in `caps`, any text.
fn of_clan(clan: &str, caps: std::ops::RangeInclusive<u8>) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.clan == clan && caps.contains(&c.capacity))
        .unwrap_or_else(|| panic!("no {clan} of capacity {caps:?}")) as u16
}

/// Mesu Bedshet played by `m` (seat 0) with `top` on its crypt (none: an
/// empty crypt); the minion phase after it.
fn mesu(top: Option<u16>) -> (Engine, MinionId) {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), plain("Ministry", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    e.state.seats[0].crypt = top.into_iter().chain([plain("Brujah", 2)]).rev().collect();
    if top.is_none() {
        e.state.seats[0].crypt = Default::default();
    }
    hand(&mut e, 0, &["Mesu Bedshet"]);
    (e, m)
}

// Mesu Bedshet: "+1 stealth action. Reveal the top card of your crypt. If
// it is a Follower of Set, draw it and add 2 blood to it if it is younger.
// Otherwise, shuffle it back into your crypt." [RTR 20000501] [LSJ
// 20070411-2] "Cannot be played when the target crypt is empty." A younger
// Follower of Set drawn with 2 blood, an older one with none, another clan
// shuffled back.
#[test]
fn mesu_bedshet_draws_a_follower_of_set_with_blood_if_younger() {
    let cap = CRYPT[plain("Ministry", 0) as usize].capacity;
    for (top, drawn, b) in [
        (of_clan("Ministry", 1..=cap - 1), true, 2),
        (of_clan("Ministry", cap + 1..=11), true, 0),
        (plain("Brujah", 1), false, 0),
    ] {
        let (mut e, m) = mesu(Some(top));
        pick(&mut e, play(0, 0, m));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let s = &e.state.seats[0];
        if drawn {
            assert_eq!(s.uncontrolled.len(), 1);
            assert_eq!(
                (s.uncontrolled[0].card, s.uncontrolled[0].counters),
                (top, b)
            );
            assert_eq!(s.crypt.len(), 1);
        } else {
            assert!(s.uncontrolled.is_empty());
            assert_eq!(s.crypt.len(), 2, "shuffled back");
            assert!(s.crypt.contains(&top));
        }
    }
    let (mut e, m) = mesu(None);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, m)), "an empty crypt");
}

/// Kindred Segregation passed by seat 0's vampire.
fn segregate(e: &mut Engine, a: MinionId) {
    hand(e, 0, &["Kindred Segregation"]);
    let card = (e.state.seats[0].hand.len() - 1) as u8;
    pick(e, play(card, 0, a));
    to_referendum(e);
    pick_when(e, Choice::CastCardVote { favor: true });
    close_polling(e);
}

// Kindred Segregation: "Successful referendum means that all allies are
// burned. Any Methuselah can keep an ally or allies he or she controls by
// repaying their pool cost to recruit." [RTR 20010711] "Only requires the
// pool cost to be paid in order to keep the ally. Allies with blood costs
// can be kept by paying 0 pool." [ANK 20190724] "One can decide to burn an
// ally with no pool cost (not paying zero)." Seat 0 keeps its Political
// Ally for 2 pool; seat 1 keeps a Gravebound Drone for nothing and lets
// the other go.
#[test]
fn kindred_segregation_keeps_allies_for_their_pool_cost() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let pa = e.add_ally(SeatId(0), find_library("Political Ally"));
    let d1 = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
    let d2 = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    segregate(&mut e, a);
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert_eq!(req.options, vec![Choice::Pass, Choice::ChooseMinion(pa)]);
    // Kept, nothing left to choose: on to seat 1.
    e.choose(index_of(&req, Choice::ChooseMinion(pa)));
    pick(&mut e, Choice::ChooseMinion(d1));
    let req = expect(&mut e, SeatId(1), WindowLabel::ReferendumTerms);
    assert!(
        req.options.contains(&Choice::ChooseMinion(d2)),
        "0 to keep, still a choice"
    );
    e.choose(index_of(&req, Choice::Pass));
    request(&mut e);
    let alive = |m: MinionId| e.state.minions[m.index()].in_play;
    assert_eq!([pa, d1, d2].map(alive), [true, true, false]);
    assert_eq!(pool(&e), vec![10 - 2, 10]);
}

// Kindred Segregation: [RTR 20010711] "Can be called even if there are no
// allies in play."
#[test]
fn kindred_segregation_is_callable_without_allies() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    hand(&mut e, 0, &["Kindred Segregation"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, a)));
}

// Legendary Vampire: "Unique. Put this card on a vampire with capacity 7 or
// more who entered your ready region during your last influence phase. This
// vampire gets +2 bleed and +2 votes." [LSJ 20030519-2] "Cannot be used on
// a vampire who merged last turn: merging is not entering play." Seat 0
// brings out a 7+ vampire in its influence phase: the next master phase it
// is the only one eligible — not one in play before, merged since.
#[test]
fn legendary_vampire_goes_on_a_vampire_from_the_last_influence_phase() {
    let mut e = game(2);
    // Big vampires all print some text: one with no ability to add a
    // decision.
    let quiet = |i: u16| {
        zvtes::cards::vampire(i).is_none_or(|d| d.abilities.is_empty() && d.riders.is_empty())
    };
    let bigs: Vec<u16> = (0..CRYPT.len() as u16)
        .filter(|&i| {
            CRYPT[i as usize].capacity >= 7 && CRYPT[i as usize].title == Title::None && quiet(i)
        })
        .take(2)
        .collect();
    let (big, other) = (bigs[0], bigs[1]);
    let cap = CRYPT[big as usize].capacity;
    let old = e.add_minion(SeatId(0), other, 7, false);
    e.state.minions[old.index()].merged = Some(0);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            counters: cap,
            ..uncontrolled(big)
        });
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    e.choose(index_of(&req, Choice::InfluenceOut { slot: 0 }));
    let fresh = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(e.state.minions[fresh.index()].crypt_card(), Some(big));
    hand(&mut e, 0, &["Legendary Vampire"]);
    let on = |m: MinionId| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(m),
    };
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(req.options.contains(&on(fresh)));
    assert!(
        !req.options.contains(&on(old)),
        "in play before, merged since"
    );
    e.choose(index_of(&req, on(fresh)));
    request(&mut e);
    assert_eq!(e.state.bearer_mod(fresh, Stat::Bleed, ActionKind::Bleed), 2);
    assert_eq!(e.state.host_votes(fresh), 2);
}

// Propaganda of the Deed: "Requires an Anarch. Only usable after resolution
// of a successful directed action. Put this card in play. You can burn this
// card during the polling step of a political action to get +2 votes."
// After an Anarch's successful bleed it enters play, not the ash heap; in a
// later political action's polling step it burns for 2 votes.
#[test]
fn propaganda_of_the_deed_enters_play_then_burns_for_two_votes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 0), 4, false);
    let b = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    hand(
        &mut e,
        0,
        &["Propaganda of the Deed", "Ancilla Empowerment"],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    let id = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.card == find_library("Propaganda of the Deed"))
            .expect("in play") as u32,
    );
    assert!(e.state.cards[id.index()].in_play);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Propaganda of the Deed")));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Ancilla Empowerment"))
        .expect("in hand") as u8;
    let _ = req;
    pick(&mut e, play(card, 0, b));
    to_referendum(&mut e);
    pick_when(&mut e, ability(id.0, 0));
    assert!(!e.state.cards[id.index()].in_play);
}

// Deer Rifle: "Weapon: gun. 1R damage each strike, with two optional
// maneuvers each combat." Its bearer, acting, maneuvers; the blocker's
// gun answers; the Rifle maneuvers again.
#[test]
fn deer_rifle_maneuvers_twice_a_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let rifle = in_play(&mut e, 0, "Deer Rifle", Some(a));
    let magnum = in_play(&mut e, 1, ".44 Magnum", Some(z));
    let rifle_move = use_at(rifle, 1, None, Target::None);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, rifle_move);
    pick_when(&mut e, use_at(magnum, 1, None, Target::None));
    let req = request(&mut e);
    assert!(req.options.contains(&rifle_move), "a second maneuver");
    // Used: both strikes committed to the guns, the combat resolves with
    // no further decision — the Rifle's 1R, the Magnum's 2R.
    e.choose(index_of(&req, rifle_move));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, z), blood(&e, a)), (4 - 1, 4 - 2));
}

// Groundfighting: "Requires a ready anarch. Do not replace until after
// combat. Maneuver or press or burn 1 blood to cancel a combat card played
// by the opposing minion that would restrict this anarch's choice of
// strikes this round as it is played." Immortal Grapple ("strikes that are
// not hand strikes cannot be used this round") canceled for 1 blood: the
// anarch's gun strike is there again; the grapple stays played, in the ash
// heap, its cost paid all the same [ANK 20260216] (none here).
#[test]
fn groundfighting_cancels_a_grapple_for_a_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_disc("pot", None, 0), 4, false);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(a));
    hand(&mut e, 0, &["Groundfighting"]);
    hand(&mut e, 1, &["Immortal Grapple"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    let cancel = play(0, 1, a);
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, cancel));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&use_at(gun, 0, None, Target::None)));
    assert_eq!(blood(&e, a), 3);
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Immortal Grapple")));
}

// Groundfighting: [LSJ 20050221] "Can cancel cards preventing the use of
// equipment if the target has a weapon": Drawing Out the Beast ("cannot use
// equipment") against an armed anarch, not an unarmed one.
#[test]
fn groundfighting_cancels_an_equipment_bar_only_with_a_weapon() {
    for armed in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), with_disc("ani", None, 0), 4, false);
        if armed {
            in_play(&mut e, 0, ".44 Magnum", Some(a));
        }
        hand(&mut e, 0, &["Groundfighting"]);
        hand(&mut e, 1, &["Drawing Out the Beast"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 0, z));
        let req = request(&mut e);
        assert_eq!(
            req.options.contains(&play(0, 1, a)),
            armed,
            "{:?}",
            req.options
        );
    }
}

// Groundfighting: [LSJ 20050221] "Cannot cancel a card that destroys or
// steals an equipment or weapon": Fast Hands' strike is no restriction.
#[test]
fn groundfighting_does_not_cancel_a_weapon_steal() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_disc("cel", None, 0), 4, false);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(a));
    hand(&mut e, 0, &["Groundfighting"]);
    hand(&mut e, 1, &["Fast Hands"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play_at(0, 0, z, Target::Card(gun)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
}

/// The Status Perfectus in play for seat 0, with two of its anarchs.
fn perfectus() -> (Engine, CardId, MinionId, MinionId) {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 0), 4, false);
    let y = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 1), 2, false);
    let id = in_play(&mut e, 0, "Status Perfectus, The", None);
    (e, id, x, y)
}

// The Status Perfectus: "During your master phase, you may move 1 blood
// from a ready anarch you control to another ready anarch." Once in the
// phase.
#[test]
fn status_perfectus_moves_a_blood_between_anarchs_in_the_master_phase() {
    let (mut e, id, x, y) = perfectus();
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let from_x = use_at(id, 0, None, Target::Minion(x));
    e.choose(index_of(&req, from_x));
    request(&mut e);
    assert_eq!((blood(&e, x), blood(&e, y)), (3, 3));
    let req = request(&mut e);
    assert!(!req.options.contains(&from_x), "once in the phase");
}

// The Status Perfectus: "When a blocking anarch has just completed combat
// with an acting minion, you may lock this card and a ready unlocked
// anarch you control other than the blocking anarch. That anarch enters
// combat with the acting minion. The acting minion cannot use any strikes
// in the first round of that combat." [LSJ 20001126] "Minions that cannot
// strike, cannot play or use the maneuver from a strike" — the acting
// minion's gun offers no maneuver; [LSJ 19970814-2] its strike is none. Y
// hits it once, unanswered: the whole combat needs no decision.
#[test]
fn status_perfectus_sends_a_second_anarch_the_acting_minion_cannot_strike() {
    let (mut e, id, x, y) = perfectus();
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, ".44 Magnum", Some(v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(v));
    pick_when(&mut e, block(x));
    let send = use_at(id, 1, None, Target::Minion(y));
    let req = offering(&mut e, send);
    assert!(!req
        .options
        .contains(&use_at(id, 1, None, Target::Minion(x))));
    let (before, y_blood) = (blood(&e, v), blood(&e, y));
    e.choose(index_of(&req, send));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::MinionPhase)
    );
    assert!(e.state.cards[id.index()].locked);
    assert!(e.state.minions[y.index()].locked);
    assert_eq!(blood(&e, y), y_blood, "no strike from the acting minion");
    assert_eq!(blood(&e, v), before - 1);
}

/// The first crypt card matching `f` whose printed text adds no decision.
fn quiet_where(f: impl Fn(&zvtes::crypt::CryptCard) -> bool) -> u16 {
    (0..CRYPT.len() as u16)
        .find(|&i| {
            f(&CRYPT[i as usize])
                && zvtes::cards::vampire(i)
                    .is_none_or(|d| d.abilities.is_empty() && d.riders.is_empty())
        })
        .expect("a quiet vampire")
}

// Petra Resonance: "+1 stealth action. Not usable if any Methuselah's crypt
// is empty. Each Methuselah reveals the top card of his or her crypt. If
// there is a tie for highest or lowest cost among the revealed cards, every
// Methuselah burns 1 pool. Otherwise, the Methuselah revealing the highest
// cost steals 3 pool from the Methuselah revealing the lowest cost. All
// Methuselahs shuffle their crypts afterward." [RTR 20000501] [LSJ
// 20070411-2] "Cannot be played when the target crypt is empty."
#[test]
fn petra_resonance_steals_for_the_highest_or_taxes_a_tie() {
    let cap = |c: u8| quiet_where(|v| v.capacity == c);
    for (tops, want) in [([5, 3, 8], [10, 7, 13]), ([5, 8, 8], [9, 9, 9])] {
        let mut e = game(3);
        let m = e.add_minion(SeatId(0), plain("Malkavian", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
        e.add_minion(SeatId(2), plain("Ventrue", 1), 4, true);
        for (s, &t) in tops.iter().enumerate() {
            e.state.seats[s].crypt = vec![plain("Brujah", 2), cap(t)].into();
        }
        hand(&mut e, 0, &["Petra Resonance"]);
        pick(&mut e, play(0, 0, m));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e), want.to_vec(), "{tops:?}");
        assert!(e.state.seats.iter().all(|s| s.crypt.len() == 2));
    }
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), plain("Malkavian", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    e.state.seats[0].crypt = vec![cap(5)].into();
    hand(&mut e, 0, &["Petra Resonance"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play(0, 0, m)),
        "seat 1's crypt is empty"
    );
}

// Waiting Game: "Transient. Put this card in play with 10 counters.
// Whenever a non-Anarch vampire takes an action, burn a counter from this
// card. Burn this card when it has no counters." An Anarch's bleed costs no
// counter; a Camarilla vampire's its last one.
#[test]
fn waiting_game_counts_down_non_anarch_actions() {
    let mut e = game(2);
    let anarch = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 0), 4, false);
    let cam = e.add_minion(SeatId(0), with_disc("", Some(Sect::Camarilla), 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let wg = in_play(&mut e, 1, "Waiting Game", None);
    e.state.cards[wg.index()].counters = 1;
    pick(&mut e, bleed(anarch));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[wg.index()].counters, 1);
    pick(&mut e, bleed(cam));
    request(&mut e);
    assert!(!e.state.cards[wg.index()].in_play);
}

// Waiting Game: "During your unlock phase, each anarch burns 1 blood or
// becomes Camarilla, and each ally burns 1 life." Every Methuselah's: an
// anarch with blood chooses, one without becomes Camarilla, a 1-life ally
// burns.
#[test]
fn waiting_game_bleeds_anarchs_or_makes_them_camarilla() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let rich = e.add_minion(SeatId(1), with_disc("", Some(Sect::Anarch), 0), 3, false);
    let poor = e.add_minion(SeatId(1), with_disc("", Some(Sect::Anarch), 1), 0, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.state.minions[ally.index()].blood = 1;
    let wg = in_play(&mut e, 1, "Waiting Game", None);
    e.state.cards[wg.index()].counters = 10;
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert_eq!(req.options, vec![Choice::Pass, Choice::BurnBloodToAvoid]);
    e.choose(index_of(&req, Choice::BurnBloodToAvoid));
    request(&mut e);
    assert_eq!(blood(&e, rich), 2);
    assert_eq!(e.state.sect_of(rich), Some(Sect::Anarch));
    assert_eq!(e.state.sect_of(poor), Some(Sect::Camarilla));
    assert!(!e.state.minions[ally.index()].in_play);
}

// Waiting Game: [ANK 20210124] "If an action is cancelled after it has
// been played (e.g. Tangle Atropos' Hand but not Direct Intervention) it
// still counts as taken": an action card canceled as it is played (Direct
// Intervention) costs no counter.
#[test]
fn waiting_game_does_not_count_an_action_card_canceled_as_played() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), plain("Ministry", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let wg = in_play(&mut e, 1, "Waiting Game", None);
    e.state.cards[wg.index()].counters = 10;
    e.state.seats[0].crypt = vec![plain("Brujah", 2)].into();
    hand(&mut e, 0, &["Mesu Bedshet"]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, play(0, 0, m));
    pick_when(&mut e, master_play(0));
    request(&mut e);
    assert_eq!(e.state.cards[wg.index()].counters, 10);
}

/// A plain untitled vampire of capacity 2 or 3, no text.
fn small_plain() -> u16 {
    (0..CRYPT.len() as u16)
        .find(|&i| {
            let c = &CRYPT[i as usize];
            (2..=3).contains(&c.capacity)
                && c.title == Title::None
                && zvtes::cards::vampire(i).is_none()
        })
        .expect("a small plain vampire")
}

/// A plain vampire with `disc`, of `clan` if given, capacity in `caps`.
fn vamp(disc: &str, clan: Option<&str>, caps: std::ops::RangeInclusive<u8>, nth: usize) -> u16 {
    (0..CRYPT.len() as u16)
        .filter(|&i| {
            let c = &CRYPT[i as usize];
            c.disciplines.contains(&disc)
                && clan.is_none_or(|k| c.clan == k)
                && caps.contains(&c.capacity)
                && c.title == Title::None
                && zvtes::cards::vampire(i).is_none()
        })
        .nth(nth)
        .unwrap_or_else(|| panic!("no {disc} {clan:?} {caps:?}"))
}

// Mental Maze: "Only usable if this vampire blocks an ally or younger
// vampire, before block resolution. [obf] Lock this vampire and end the
// action. This card costs Followers of Set 1 fewer blood. [OBF] As above,
// but do not lock this vampire." [RTR 20180719] [ANK 20200207] "The action
// ends unsuccessfully immediately, no other action modifier or reaction can
// be played afterwards": no combat, no bleed; the blocker stays locked
// ([obf]) or unlocks ([OBF]); a Follower of Set pays nothing for it.
#[test]
fn mental_maze_ends_the_action_its_blocker_locked_or_not() {
    for (superior_, ministry) in [(false, false), (true, false), (false, true)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), small_plain(), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let card = if ministry {
            vamp("obf", Some("Ministry"), 4..=11, 0)
        } else {
            (0..)
                .map(|n| vamp("obf", None, 4..=11, n))
                .find(|&c| CRYPT[c as usize].clan != "Ministry")
                .expect("a non-Ministry obf vampire")
        };
        let z = e.add_minion(SeatId(1), card, 4, false);
        if superior_ {
            superior(&mut e, z, "obf");
        }
        hand(&mut e, 1, &["Mental Maze"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, u8::from(superior_), z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e)[1], 10, "no bleed");
        assert_eq!(blood(&e, a), 3, "no combat");
        assert_eq!(e.state.minions[z.index()].locked, !superior_);
        assert_eq!(blood(&e, z), if ministry { 4 } else { 3 });
    }
}

// Mental Maze: not against an older vampire.
#[test]
fn mental_maze_is_not_played_against_an_older_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vamp("obf", None, 1..=3, 0), 3, false);
    assert!(e.state.capacity_of(z) < e.state.capacity_of(a));
    hand(&mut e, 1, &["Mental Maze"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, z));
}

// Maabara: "Master: unique location. You may lock this location to move a
// library card from your ash heap to this location, face down. You may use
// a master phase action to move a card from this location to the top of
// your library. Any vampire may burn this location as a Ⓓ action." [ANK
// 20200523] "All other players can see which card is selected from the ash
// heap": the pick a public choice of the card.
#[test]
fn maabara_keeps_an_ash_card_for_the_library_top() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let id = in_play(&mut e, 0, "Maabara", None);
    fill_hand(&mut e, 0);
    let (conceal, magnum) = (find_library("Conceal"), find_library(".44 Magnum"));
    e.state.seats[0].ash = vec![conceal, magnum].into();
    pick_when(&mut e, use_at(id, 0, None, Target::None));
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&Choice::FindCard(conceal)));
    assert!(req.options.contains(&Choice::FindCard(magnum)));
    e.choose(index_of(&req, Choice::FindCard(magnum)));
    request(&mut e);
    assert!(e.state.cards[id.index()].locked);
    assert_eq!(e.state.cards[id.index()].stored.to_vec(), vec![magnum]);
    let back = use_at(id, 1, None, Target::None);
    pick_when(&mut e, back);
    request(&mut e);
    assert_eq!(e.state.seats[0].library.last(), Some(&magnum));
    // Seat 1's vampire burns it as a directed action.
    let burn = use_at(id, 2, Some(z), Target::None);
    pick_when(&mut e, burn);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[id.index()].in_play);
}

// An Anarch Manifesto: "The anarch with this equipment gets +1 stealth on
// actions that require an anarch. Titled non-Anarch vampires get +1
// strength in combat with this minion." Against Ian Carfax (+1 intercept):
// Constant Revolution (requiring an Anarch, +1 stealth) goes at 2 stealth,
// unblockable by him; a bleed at none, blocked.
#[test]
fn anarch_manifesto_stealths_actions_requiring_an_anarch() {
    for revolution in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let ian = e.add_minion(SeatId(1), find_card("Ian Carfax"), 4, false);
        in_play(&mut e, 0, "Anarch Manifesto, An", Some(a));
        hand(&mut e, 0, &["Constant Revolution"]);
        pick(&mut e, if revolution { play(0, 0, a) } else { bleed(a) });
        let blocked = offered_before(&mut e, block(ian), SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blocked, !revolution);
    }
}

// An Anarch Manifesto: a titled non-Anarch vampire (a prince) blocking its
// bearer hits for 2.
#[test]
fn anarch_manifesto_strengthens_titled_non_anarch_foes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let p = e.add_minion(
        SeatId(1),
        quiet_where(|c| matches!(c.title, Title::Prince(_)) && c.sect == Sect::Camarilla),
        4,
        false,
    );
    in_play(&mut e, 0, "Anarch Manifesto, An", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(p));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 2);
}

/// A quiet Anarch or Independent of capacity 9 or 10 (5 points).
fn anarch_of_nine() -> u16 {
    quiet_where(|c| {
        matches!(c.sect, Sect::Anarch | Sect::Independent) && (9..=10).contains(&c.capacity)
    })
}

// Free States Rant: "Requires an Independent or Anarch vampire. Allocate X
// points among one or more ready vampires, where X is half the capacity of
// the acting vampire (rounded up). No more than 3 points can be allocated
// to each vampire. Successful referendum means each vampire burns 1 blood
// for each point allocated." An Anarch of capacity 9: 5 points, 3 at most
// on one vampire — the rest on another; they burn as many.
#[test]
fn free_states_rant_allocates_half_capacity_three_at_most_each() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch_of_nine(), 9, false);
    let x = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Free States Rant"]);
    pick(&mut e, play(0, 0, a));
    for _ in 0..3 {
        pick(&mut e, Choice::ChooseMinion(x));
    }
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ChooseMinion(x)), "3 at most");
    e.choose(index_of(&req, Choice::ChooseMinion(y)));
    pick(&mut e, Choice::ChooseMinion(y));
    to_referendum(&mut e);
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, x), blood(&e, y)), (4 - 3, 4 - 2));
}

// Free States Rant: [LSJ 20010810] "If there are insuffient targets to
// which to allocate all of the points allowed, then the card cannot be
// played." [LSJ 20100129] [ANK 20191228]: 5 points, two ready vampires (6
// at most) — playable; one (3 at most) — not.
#[test]
fn free_states_rant_needs_room_for_every_point() {
    for others in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), anarch_of_nine(), 9, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
        if others {
            e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        }
        hand(&mut e, 0, &["Free States Rant"]);
        let req = request(&mut e);
        assert_eq!(req.options.contains(&play(0, 0, a)), others);
    }
}

/// The Gargoyle with superior Visceratika (Alabástrom: his ability adds no
/// decision here).
fn gargoyle() -> u16 {
    find_card("Alabástrom")
}

// Raking Talons: "Only usable before range is determined. For the remainder
// of combat, damage from this Gargoyle's hand strikes is aggravated. A
// vampire may play only one Raking Talons each combat." Played before range,
// its hand damage aggravated for the combat; the second copy not offered.
#[test]
fn raking_talons_makes_hand_strikes_aggravated_once_a_combat() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), gargoyle(), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Raking Talons", "Raking Talons"]);
    pick(&mut e, bleed(g));
    pick_when(&mut e, block(z));
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, g)));
    e.choose(index_of(&req, play(0, 0, g)));
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, g)), "one each combat");
    let c = e.combat().expect("combat");
    let i = c.side_of_minion(g).expect("in combat");
    assert!(c.sides[i].aggravated_hands_combat);
}

// Brick by Brick: "[tha] Maneuver. [vis] Strike: 2R damage, with 1 optional
// maneuver. [VIS] Only usable before range is determined. This vampire
// sets the range for this round." [RTR 19970630] [ANK 20180720] "Once the
// range is set, no other effect can be used to reset the range that round,
// but other pre-range effects can be played afterwards. Skip the Determine
// Range step for that round": set to long range, no maneuver offered, the
// [vis] strike a ranged one.
#[test]
fn brick_by_brick_sets_the_range() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), gargoyle(), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Brick by Brick", "Brick by Brick"]);
    pick(&mut e, bleed(g));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 3, g));
    let req = request(&mut e);
    assert_eq!(
        req.window,
        WindowLabel::CombatStrike,
        "no determine range step"
    );
    assert_eq!(
        e.combat().expect("combat").range,
        zvtes::combat::Range::Long
    );
    assert!(!req.options.contains(&play(0, 2, g)), "set once");
    let _ = gun;
}

// Fortschritt Library: "Master. Search your library for a Gehenna event
// card. Show it to all players and add it to your hand. Discard down to
// your hand size afterward. Only one Fortschritt Library may be played per
// game." The Gehenna events of the library offered (not another event),
// the library shuffled; a second copy not playable.
#[test]
fn fortschritt_library_finds_a_gehenna_event_once_a_game() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let (blood_weakens, conceal) = (find_library("Blood Weakens"), find_library("Conceal"));
    e.state.seats[0].library = vec![conceal, blood_weakens, conceal].into();
    hand(&mut e, 0, &["Fortschritt Library", "Fortschritt Library"]);
    fill_hand(&mut e, 0);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    e.choose(index_of(&req, master_play(0)));
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::FindCard(blood_weakens)]
    );
    e.choose(1);
    let req = request(&mut e);
    assert!(e.state.seats[0].hand.contains(&blood_weakens));
    assert_eq!(e.state.seats[0].library.len(), 2);
    assert!(!req.options.contains(&master_play(0)), "once a game");
}

// Nose of the Hound: "+1 stealth action. [aus] Ⓓ Enter combat with a ready
// locked minion controlled by another Methuselah. … [SPI] As [spi] above,
// but enter combat with any ready minion controlled by another Methuselah."
// [aus]: a locked minion only; [SPI]: an unlocked one too.
#[test]
fn nose_of_the_hound_needs_a_locked_minion_below_superior_spiritus() {
    for spi in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_disc("aus", None, 0), 4, false);
        if spi {
            superior(&mut e, a, "spi");
        }
        let locked = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[locked.index()].locked = true;
        let free = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Nose of the Hound"]);
        let req = request(&mut e);
        let at = |mode: u8, m: MinionId| play_at(0, mode, a, Target::Minion(m));
        assert!(req.options.contains(&at(0, locked)));
        assert!(!req.options.contains(&at(0, free)));
        assert_eq!(req.options.contains(&at(2, free)), spi);
    }
}

/// Seat 0's Animalism vampire, seat 1's (its prey) and seat 2's (its
/// predator) ready vampires.
fn taunt_table() -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), with_disc("ani", None, 0), 4, false);
    superior(&mut e, a, "ani");
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let q = e.add_minion(SeatId(2), plain("Toreador", 0), 0, false);
    hand(&mut e, 0, &["Taunt the Caged Beast"]);
    (e, a, p, q)
}

// Taunt the Caged Beast [ANI]: "Choose a ready vampire controlled by your
// predator and another controlled by your prey (not usable when only one
// other Methuselah is in the game). The chosen vampires enter combat. If
// only one is ready at the end of that combat, he or she gains 4 blood."
// [LSJ 20070319] "The prey goes first in the combat (for declaring
// maneuvers, strikes, presses)": the prey's vampire the first side; the
// predator's, with no blood, sent to torpor — the prey's gains 4. Both its
// targets: the action is directed at both Methuselahs (owner, 2026-10-14:
// rules-qa 71.21), each of whom may block.
#[test]
fn taunt_the_caged_beast_pits_prey_against_predator() {
    let (mut e, a, p, q) = taunt_table();
    e.add_minion(SeatId(2), plain("Toreador", 1), 4, false);
    in_play(&mut e, 2, ".44 Magnum", Some(q));
    // Two ready vampires of the predator's: the pick asked.
    pick(&mut e, play_at(0, 1, a, Target::Minion(p)));
    pick(&mut e, Choice::ChooseMinion(q));
    request(&mut e);
    let directed = e.action().expect("action").directed_seats(&e.state);
    assert!(directed.contains(SeatId(1)) && directed.contains(SeatId(2)));
    assert!(!directed.contains(SeatId(0)));
    let mut blockers = Vec::new();
    loop {
        let req = request(&mut e);
        if req.window != WindowLabel::ActionBlock {
            break;
        }
        blockers.push(req.seat);
        e.choose(fallback(&req));
    }
    blockers.sort();
    assert_eq!(blockers, vec![SeatId(1), SeatId(2)], "both may block");
    let req = until(&mut e, SeatId(2), WindowLabel::CombatDetermineRange);
    let c = e.combat().expect("combat");
    assert_eq!((c.sides[0].minion, c.sides[1].minion), (p, q));
    e.choose(fallback(&req));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[q.index()].in_torpor);
    let cap = e.state.capacity_of(p).unwrap();
    assert_eq!(blood(&e, p), (4 - 1 + 4).min(cap));
}

// Taunt the Caged Beast [ANI]: not with only one other Methuselah.
#[test]
fn taunt_the_caged_beast_needs_a_predator_besides_the_prey() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_disc("ani", None, 0), 4, false);
    superior(&mut e, a, "ani");
    let p = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Taunt the Caged Beast"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play_at(0, 1, a, Target::Minion(p))));
    assert!(
        req.options.contains(&play_at(0, 0, a, Target::Minion(p))),
        "[ani] still"
    );
}

// Big Game: "Do not replace until after this action. Ⓓ Enter combat with a
// ready minion controlled by another Methuselah. If that minion is a Red
// List minion, this minion gets an optional maneuver or press in that
// combat and unlocks at the end of the action if he or she is still ready."
// [ANK 20180518] "'that minion' refers to the target and 'this minion' to
// the acting minion": against Enkidu (Red List) the acting vampire unlocks
// after the combat; against another, it stays locked.
#[test]
fn big_game_unlocks_after_a_red_list_hunt() {
    for red in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let t = e.add_minion(
            SeatId(1),
            if red {
                find_card("Enkidu, The Noah")
            } else {
                plain("Ventrue", 0)
            },
            4,
            false,
        );
        hand(&mut e, 0, &["Big Game"]);
        pick(&mut e, play_at(0, u8::from(red), a, Target::Minion(t)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[a.index()];
        assert!(v.ready());
        assert_eq!(v.locked, !red);
    }
}

/// Seat 0's `bearer` with the Kaymakli Fragment.
fn kaymakli(bearer: u16) -> (Engine, MinionId, CardId) {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), bearer, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let id = in_play(&mut e, 0, "Kaymakli Fragment", Some(m));
    (e, m, id)
}

// Kaymakli Fragment: "This Sabbat vampire can draw 5 cards as an action
// (discard down afterward)." [LSJ 20060620] "You can draw more cards than
// are in the library or crypt (even from an empty library or crypt); draw
// as much as you can": three cards left, three drawn. [PIB 20150809] "Does
// not make the vampire Sabbat nor infernal": a Camarilla bearer has no
// such action.
#[test]
fn kaymakli_fragment_draws_for_a_sabbat_bearer() {
    let (mut e, m, id) = kaymakli(find_card("Midget"));
    e.state.seats[0].library = vec![find_library("Conceal"); 3].into();
    pick(&mut e, use_at(id, 0, Some(m), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].hand.len(), 3);
    assert!(e.state.seats[0].library.is_empty());
    let (mut e, m, id) = kaymakli(with_disc("", Some(Sect::Camarilla), 0));
    let req = request(&mut e);
    assert!(!req.options.contains(&use_at(id, 0, Some(m), Target::None)));
}

// Kaymakli Fragment: "This infernal minion can add 3 blood to a vampire in
// your uncontrolled region as a +1 stealth action." [LSJ 20100909] "The two
// actions provided are not cross-restrictive. Each can be performed once
// during the same turn by the same minion": after the draw, Midget unlocked
// again adds the blood.
#[test]
fn kaymakli_fragment_adds_blood_for_an_infernal_bearer() {
    let (mut e, m, id) = kaymakli(find_card("Midget"));
    e.state.seats[0]
        .uncontrolled
        .push(uncontrolled(plain("Brujah", 2)));
    e.state.seats[0].library = vec![find_library("Conceal"); 5].into();
    pick(&mut e, use_at(id, 0, Some(m), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[m.index()].locked = false;
    pick(&mut e, use_at(id, 1, Some(m), Target::Uncontrolled(0)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 3);
}

// Kaymakli Fragment: "An opposing minion can strike: steal this equipment."
// Its bearer's blocker takes it, at close range.
#[test]
fn kaymakli_fragment_is_stolen_by_a_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let id = in_play(&mut e, 0, "Kaymakli Fragment", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let steal = use_at(id, 2, Some(z), Target::Card(id));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(req.options.contains(&steal));
    e.choose(index_of(&req, steal));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[id.index()].host, Some(z));
}

/// Pass until `m` is offered a card play, and take the first one.
fn pick_play_by(e: &mut Engine, m: MinionId) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(i) = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Play { actor: Some(x), .. } if *x == m))
        {
            e.choose(i);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("no play by {m:?}");
}

/// Whether `m` is offered a card play before seat 0's next minion phase
/// decision (passing all the way).
fn plays_offered(e: &mut Engine, m: MinionId) -> bool {
    loop {
        let req = request(e);
        if req
            .options
            .iter()
            .any(|o| matches!(o, Choice::Play { actor: Some(x), .. } if *x == m))
        {
            return true;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            return false;
        }
        e.choose(fallback(&req));
    }
}

// Perfect Clarity: "Only usable when the action is announced. [tha]
// Reaction cards that require Dominate [dom] or Presence [pre] do not
// affect this vampire for the duration of this action." [LSJ 20010621]
// [LSJ 20010830] "Does prevent bleed redirection (eg. Deflection)."
#[test]
fn perfect_clarity_shields_a_bleed_from_deflection() {
    for clarity in [false, true] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), with_disc("tha", None, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let d = e.add_minion(SeatId(1), with_disc("dom", None, 0), 4, false);
        e.add_minion(SeatId(2), plain("Toreador", 0), 4, true);
        hand(&mut e, 0, &["Perfect Clarity"]);
        hand(&mut e, 1, &["Deflection"]);
        pick(&mut e, bleed(a));
        if clarity {
            pick_when(&mut e, play(0, 0, a));
        }
        let deflected = plays_offered(&mut e, d);
        assert_eq!(deflected, !clarity);
    }
}

// Perfect Clarity [THA]: "… minions opposing this vampire in combat cannot
// play cards that require Dominate or Presence": its blocker's Majesty
// [pre] not offered.
#[test]
fn perfect_clarity_superior_bars_presence_from_its_opponent() {
    for clarity in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_disc("tha", None, 0), 4, false);
        superior(&mut e, a, "tha");
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), with_disc("pre", None, 0), 4, false);
        hand(&mut e, 0, &["Perfect Clarity"]);
        hand(&mut e, 1, &["Majesty"]);
        pick(&mut e, bleed(a));
        if clarity {
            pick_when(&mut e, play(0, 1, a));
        }
        pick_when(&mut e, block(z));
        let majesty = plays_offered(&mut e, z);
        assert_eq!(majesty, !clarity);
    }
}

// Legal Manipulations: "[pre] Ⓓ Bleed with +2 bleed. [PRE] As above, and
// gain 1 pool if the bleed is successful (for 1 or more)."
#[test]
fn legal_manipulations_bleeds_for_three_and_gains_a_pool_superior() {
    for sup in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_disc("pre", None, 0), 4, false);
        if sup {
            superior(&mut e, a, "pre");
        }
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
        hand(&mut e, 0, &["Legal Manipulations"]);
        pick(&mut e, play(0, u8::from(sup), a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e), vec![10 + u8::from(sup) as i32, 10 - 3]);
    }
}

/// A vampire with Dementation whose text adds no decision.
fn dementation() -> u16 {
    quiet_where(|c| c.disciplines.contains(&"dem") && c.title == Title::None)
}

// Touch of Clarity: "Usable by a ready unlocked vampire other than the
// acting minion. [dem] Cancel a non-combat card requiring Dementation
// [dem], Dominate [dom] or Presence [pre] as it is played, and its cost is
// not paid. Lock this vampire." [ANK 20200813-2] "Can be used to cancel all
// action cards": Legal Manipulations canceled, its blood unpaid, the
// canceler locked; the action not taken, its vampire free to act again
// [RBK cancel-a-card].
#[test]
fn touch_of_clarity_cancels_a_presence_action_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_disc("pre", None, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), dementation(), 4, false);
    hand(&mut e, 0, &["Legal Manipulations"]);
    hand(&mut e, 1, &["Touch of Clarity"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, t));
    let req = request(&mut e);
    assert_eq!(pool(&e)[1], 10);
    assert_eq!(blood(&e, a), 4, "unpaid");
    assert!(e.state.minions[t.index()].locked);
    assert!(req.options.contains(&bleed(a)), "free to act again");
}

// Touch of Clarity: [LSJ 20090425] [LSJ 19990425] "Can be used as an action
// modifier by a non-acting ready unlocked minion controlled by the
// controller of the acting minion": seat 0's other vampire cancels the
// Deflection against its bleed.
#[test]
fn touch_of_clarity_by_the_acting_methuselahs_other_vampire() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(0), dementation(), 4, false);
    let d = e.add_minion(SeatId(1), with_disc("dom", None, 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, true);
    hand(&mut e, 0, &["Touch of Clarity"]);
    hand(&mut e, 1, &["Deflection"]);
    pick(&mut e, bleed(a));
    pick_play_by(&mut e, d);
    pick_when(&mut e, play(0, 1, t));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1, "the bleed not redirected");
}

// The Art of Pain: "Unique master. Put this card on a ready Toreador
// antitribu. This vampire gets +2 strength." One of the player's (rules-qa
// 32.7), not another clan.
#[test]
fn the_art_of_pain_strengthens_a_toreador_antitribu() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), plain("Toreador antitribu", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    hand(&mut e, 0, &["Art of Pain, The"]);
    let on = |m: MinionId| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(m),
    };
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(!req.options.contains(&on(b)));
    e.choose(index_of(&req, on(t)));
    request(&mut e);
    assert_eq!(e.state.bearer_mod(t, Stat::Strength, ActionKind::Bleed), 2);
}

// An Anarch Manifesto: [ANK 20230417] "Is not Anarch (eg. does not benefit
// from An Anarch Manifesto)": an ally bearer (Grey Thorne's, Vivienne
// Géroux's kind) gets no stealth from it.
#[test]
fn anarch_manifesto_gives_an_ally_nothing() {
    let mut e = game(2);
    let ally = e.add_ally(SeatId(0), find_library("Political Ally"));
    let a = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 0), 4, false);
    in_play(&mut e, 0, "Anarch Manifesto, An", Some(ally));
    in_play(&mut e, 0, "Anarch Manifesto, An", Some(a));
    assert_eq!(e.state.anarch_action_stealth(ally), 0);
    assert_eq!(e.state.anarch_action_stealth(a), 1);
}

// Groundfighting: [LSJ 20050221] "Cannot cancel maneuvering, setting the
// range or strikes that 'cannot be dodge'": the opposing Gargoyle's Brick by
// Brick [VIS], setting the range, is no restriction to cancel.
#[test]
fn groundfighting_does_not_cancel_a_range_setting() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_disc("", Some(Sect::Anarch), 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let g = e.add_minion(SeatId(1), gargoyle(), 4, false);
    hand(&mut e, 0, &["Groundfighting"]);
    hand(&mut e, 1, &["Brick by Brick"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(g));
    pick_when(&mut e, play(0, 3, g));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
}

// Leandro: [LSJ 20100527] "When a Methuselah decides to get no transfer,
// he can still use other effects that put blood on his uncontrolled
// vampires (eg. [DOM] Govern the Unaligned)": after losing the transfers,
// seat 1's Govern the Unaligned [DOM] still adds 3 blood.
#[test]
fn leandro_leaves_other_blood_for_the_uncontrolled() {
    let mut e = game(2);
    e.add_minion(SeatId(0), find_card("Leandro"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let d = e.add_minion(SeatId(1), with_disc("dom", None, 0), 4, false);
    superior(&mut e, d, "dom");
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[1]
        .uncontrolled
        .push(uncontrolled(small_plain()));
    hand(&mut e, 1, &["Govern the Unaligned"]);
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, Choice::Pass));
    pick_when(&mut e, play_at(0, 1, d, Target::Uncontrolled(0)));
    request(&mut e);
    assert!(e.state.seats[1].no_transfers_next);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].uncontrolled[0].counters, 3);
}
