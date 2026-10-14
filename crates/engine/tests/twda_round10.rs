//! Legacy cards for TWDA decks, round 10 (`ADDED_BATCHES`, the twentieth
//! batch, slice 16v): the 2025-2026 TWDA decks the version cap keeps.
//! Readings in rules-qa Batch 70. Windows with a single option are elided,
//! so the scripts only stop where a real decision exists.

use zvtes::actions::Outcome;
use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::CRYPT;
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay, PrintedCtx, Release};

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

/// The crypt card of this name and group (Victoria Ash G2 and G7).
fn crypt_g(name: &str, group: u8) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.group == group)
        .unwrap_or_else(|| panic!("no {name} G{group}")) as u16
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

fn stealth(e: &Engine, m: MinionId, kind: Option<ActionKind>) -> i32 {
    e.state.printed_mod(
        m,
        Stat::Stealth,
        PrintedCtx {
            kind,
            ..Default::default()
        },
    )
}

// ---- Vampires -----------------------------------------------------------------

// Annabelle Triabell (G6): "Camarilla primogen: While Annabelle is locked,
// other Toreador you control get +1 stealth." [LSJ 20011022] "The passive
// ability applies even if the vampire is in torpor": locked in torpor, she
// still gives it; not to herself, nor to a Brujah.
#[test]
fn annabelle_gives_other_toreador_stealth_while_locked_in_torpor_too() {
    let mut e = game(2);
    let ann = e.add_minion(SeatId(0), find_card("Annabelle Triabell"), 4, false);
    let tor = e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
    let bru = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let theirs = e.add_minion(SeatId(1), plain("Toreador", 1), 4, false);
    assert_eq!(stealth(&e, tor, None), 0, "unlocked");
    e.state.minions[ann.index()].locked = true;
    assert_eq!(
        [tor, bru, ann, theirs].map(|m| stealth(&e, m, None)),
        [1, 0, 0, 0]
    );
    e.state.minions[ann.index()].in_torpor = true;
    assert_eq!(stealth(&e, tor, None), 1, "in torpor too");
}

// Dark Selina (G5): "Sabbat cardinal: Selina gets +1 stealth during
// political actions."
#[test]
fn dark_selina_gets_stealth_on_political_actions() {
    let e = {
        let mut e = game(2);
        e.add_minion(SeatId(0), find_card("Dark Selina"), 4, false);
        e
    };
    let s = MinionId(0);
    assert_eq!(stealth(&e, s, Some(ActionKind::Political)), 1);
    assert_eq!(stealth(&e, s, Some(ActionKind::Bleed)), 0);
}

// Genevieve (G2): "Sabbat priscus: Genevieve gets one additional vote during
// each political action." [LSJ 20051113-2] [RBK gaining-votes] "Her votes
// and ballots must be cast on the same side": one cast carries her vote and
// her prisci ballot.
#[test]
fn genevieve_casts_her_vote_with_her_ballot() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Genevieve"), 4, false);
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, a));
    to_referendum(&mut e);
    let req = offering(&mut e, votes(g, false));
    assert!(!req.options.contains(&votes(g, true)) || req.seat == SeatId(1));
    e.choose(index_of(&req, votes(g, false)));
    request(&mut e);
    let r = e.referendum().expect("referendum");
    let cast: Vec<(u8, u8, bool)> = r
        .records
        .iter()
        .map(|v| (v.n, v.ballots, v.favor))
        .collect();
    assert_eq!(cast, vec![(1, 1, false)]);
}

// Drozodny (G5): "Sabbat bishop: Drozodny can move a political action card
// from your ash heap to your hand as a +1 stealth action (discard down
// afterward)." [LSJ 20010627] "When using his ability, the card to retrieve
// must be announced when declaring the action": named as he declares, among
// the political action cards only; it moves on success.
#[test]
fn drozodny_names_the_political_card_as_he_declares() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Drozodny"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let (krc, anc, conceal) = (
        find_library("Kine Resources Contested"),
        find_library("Ancilla Empowerment"),
        find_library("Conceal"),
    );
    e.state.seats[0].ash = vec![krc, conceal, anc].into();
    pick(&mut e, printed(d, 0, Target::None));
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    let mut want = vec![Choice::FindCard(krc), Choice::FindCard(anc)];
    want.sort_by_key(|c| match c {
        Choice::FindCard(x) => *x,
        _ => 0,
    });
    assert_eq!(req.options, want);
    e.choose(index_of(&req, Choice::FindCard(anc)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].hand.contains(&anc));
    assert_eq!(e.state.seats[0].ash, vec![krc, conceal]);
}

// Malabranca (G4): "Sabbat Archbishop of Rome: If your prey has more pool
// than you, Malabranca may move 1 counter from your prey's pool to yours as
// a +1 stealth Ⓓ action." Not offered with a poorer prey.
#[test]
fn malabranca_takes_a_pool_from_a_richer_prey() {
    for richer in [true, false] {
        let mut e = game(2);
        e.state.seats[0].pool = if richer { 6 } else { 10 };
        let m = e.add_minion(SeatId(0), find_card("Malabranca"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let take = printed(m, 0, Target::Seat(SeatId(1)));
        if !richer {
            let req = request(&mut e);
            assert!(!req.options.contains(&take));
            continue;
        }
        pick(&mut e, take);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(pool(&e)[..2], [7, 9]);
    }
}

// Hardestadt (G4): "+1 bleed. +2 strength." His bleed burns 2 pool.
#[test]
fn hardestadt_bleeds_for_two() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hardestadt"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    pick(&mut e, bleed(h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 2);
    assert_eq!(
        e.state
            .printed_mod(h, Stat::Strength, PrintedCtx::default()),
        2
    );
}

// Ian Carfax (G7): "Once each turn after a combat involving Ian ends, he can
// unlock if the opposing minion is not ready. +1 intercept." He blocks a
// 1-life ally and burns it: he unlocks after the combat.
#[test]
fn ian_carfax_unlocks_after_a_combat_his_opponent_left() {
    let mut e = game(2);
    let a = e.add_ally(SeatId(0), find_library("Political Ally"));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ian = e.add_minion(SeatId(1), find_card("Ian Carfax"), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ian));
    pick_when(&mut e, own_use(ian, 0, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_play, "burned");
    assert!(!e.state.minions[ian.index()].locked);
}

// Victoria Ash (G7): "While Victoria is ready, you can discard 1 card during
// a bleed action to give an acting Assamite +1 bleed. +1 bleed." [ANK
// 20231124] [LSJ 19970718] "The ability to increase bleed can only be used
// during a bleed action before Action Resolution": discarded for during the
// bleed, not after it resolved.
#[test]
fn victoria_ash_gives_an_acting_banu_haqim_bleed_before_resolution() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), plain("Banu Haqim", 0), 4, false);
    let v = e.add_minion(SeatId(0), crypt_g("Victoria Ash", 7), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    hand(&mut e, 0, &["Conceal"]);
    pick(&mut e, bleed(b));
    pick_when(&mut e, own_use(v, 0, Target::None));
    let req = request(&mut e);
    assert!(
        !abilities(&req).contains(&own_use(v, 0, Target::None)),
        "once each action"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 2);
    assert!(e.state.seats[0].ash.contains(&find_library("Conceal")));
}

/// The `nth` plain vampire (no printed text, untitled, capacity 4 or more)
/// with `disc`, Anarch if `anarch`.
fn with_disc(disc: &str, anarch: bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            (!anarch || c.sect == zvtes::crypt::Sect::Anarch)
                && c.disciplines.contains(&disc)
                && c.title == zvtes::crypt::Title::None
                && c.capacity >= 4
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no plain vampire with {disc}"))
}

fn anarch_with(disc: &str, nth: usize) -> u16 {
    with_disc(disc, true, nth)
}

// ---- Anarch modifiers -------------------------------------------------------

// Power of One [pre]: "Only usable during a bleed action. +1 bleed
// (limited), or +1 stealth and +1 bleed (limited)." [ANK 20171023] [RTR
// 19941109] "[pre] Can only be used during a bleed action": not during a
// hunt; during a bleed, the bleed alone (the stealth one waits for a block
// attempt to need it).
#[test]
fn power_of_one_pre_is_only_for_a_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch_with("pre", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    hand(&mut e, 0, &["Power of One"]);
    pick(&mut e, action(a, ActionKind::Hunt));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
    e.state.minions[a.index()].locked = false;
    pick(&mut e, bleed(a));
    let req = offering(&mut e, play(0, 2, a));
    assert!(!req.options.contains(&play(0, 3, a)), "no stealth needed");
    e.choose(index_of(&req, play(0, 2, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 2);
}

// Power of One [pot]: "Only usable during a bleed action. +1 bleed that does
// not count against the limit." [LSJ 20100218] [RTR 20180511-2] "[pot] A
// limited bleed modifier can be played before or after": Adhocracy [ani]'s
// limited +1 first, then Power of One [pot].
#[test]
fn power_of_one_pot_goes_with_a_limited_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch_with("pot", 0), 4, false);
    superior(&mut e, a, "ani");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    hand(&mut e, 0, &["Adhocracy", "Power of One"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 3);
}

// Power of One [obt]: "Only usable if this Anarch is blocked, before block
// resolution. The blocking minion burns 2 blood or life." An ally blocker
// with 2 life burns.
#[test]
fn power_of_one_obt_costs_the_blocker_two() {
    for ally in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), anarch_with("pot", 0), 4, false);
        superior(&mut e, a, "obt");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = if ally {
            e.add_ally(SeatId(1), find_library("Gravebound Drone"))
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false)
        };
        hand(&mut e, 0, &["Power of One"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if ally {
            assert!(!e.state.minions[z.index()].in_play, "burned");
        } else {
            // Its 2, then the acting Anarch's hand strike.
            assert_eq!(blood(&e, z), 4 - 2 - 1);
        }
    }
}

// Adhocracy: "Requires an Anarch. [ani] Only usable during a bleed action.
// +1 bleed (limited). [chi] Only usable during the polling step of a
// political action. This Anarch gets +3 votes. [pot] If this Anarch is
// blocked, they get +1 strength during the resulting combat." [chi]: its
// three votes, cast with this Anarch's.
#[test]
fn adhocracy_gives_this_anarch_three_votes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch_with("pot", 0), 4, false);
    superior(&mut e, a, "chi");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ancilla Empowerment", "Adhocracy"]);
    pick(&mut e, play(0, 0, a));
    to_referendum(&mut e);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, votes(a, true));
    request(&mut e);
    let r = e.referendum().expect("referendum");
    assert!(r.records.iter().any(|v| v.favor && v.n == 3));
}

// Adhocracy [pot]: blocked, this Anarch hits with +1 strength.
#[test]
fn adhocracy_pot_strength_if_blocked() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Adhocracy"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 2);
}

// Ni Dieu ni Maître: "Requires an Anarch. Only usable as the action is
// announced. More than one Discipline can be used to play this card. [ani]
// If this action is blocked, the opposing minion cannot maneuver during the
// resulting combat. [for] Vampires must burn 1 blood to attempt to block
// this action. [vic] If this action is blocked, this Anarch gets +1 strength
// during the resulting combat." [for]: a vampire pays its blood to attempt,
// an ally attempts for free.
#[test]
fn ni_dieu_ni_maitre_tolls_vampires_not_allies() {
    for ally in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), anarch_with("pot", 0), 4, false);
        superior(&mut e, a, "for");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = if ally {
            e.add_ally(SeatId(1), find_library("Gravebound Drone"))
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false)
        };
        let life = blood(&e, z);
        hand(&mut e, 0, &["Ni Dieu ni Maître"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, play(0, 1, a));
        pick_when(&mut e, block(z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        // The toll, then the acting Anarch's hand strike.
        assert_eq!(blood(&e, z), if ally { life - 1 } else { life - 1 - 1 });
    }
}

// Ni Dieu ni Maître [ani][vic]: both Disciplines used, one card; blocked,
// the opposing minion cannot maneuver (Fake Out never offered) and this
// Anarch hits with +1 strength.
#[test]
fn ni_dieu_ni_maitre_ani_vic_bars_maneuvers_and_strengthens() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch_with("ani", 0), 4, false);
    superior(&mut e, a, "vic");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ni Dieu ni Maître"]);
    hand(&mut e, 1, &["Fake Out"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 4, a));
    pick_when(&mut e, block(z));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == z),
    );
    assert_eq!(blood(&e, z), 4 - 2);
}

// Marked Path: "Only usable after a successful directed action. [obf] Put
// this card on the acting vampire. During an action directed at the same
// Methuselah (or same set of Methuselahs), this vampire may burn this card
// to get +1 stealth." [LSJ 20100527] "If the set of Methuselahs includes
// the controller of the vampire this card is on, the controller of the
// vampire is not considered as part of that set": the set remembered is the
// bled Methuselah alone; against them again, the card burns for +1 stealth;
// not against another.
#[test]
fn marked_path_burns_for_stealth_against_the_same_methuselah() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    e.add_minion(SeatId(2), vampire_with("aus", 1), 4, false);
    hand(&mut e, 0, &["Marked Path"]);
    hand(&mut e, 1, &["Enhanced Senses"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let mp = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.card == find_library("Marked Path") && c.in_play)
            .expect("on the acting vampire") as u32,
    );
    assert_eq!(e.state.cards[mp.index()].host, Some(a));
    assert!(e.state.cards[mp.index()].against.contains(SeatId(1)));
    // Its next turn: one bleed a turn.
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(2));
    to_minion_phase(&mut e, SeatId(0));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(y));
    pick_when(&mut e, use_at(mp, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[mp.index()].in_play, "burned for the stealth");
    assert_eq!(pool(&e)[1], 10 - 2, "both bleeds landed");
}

// ---- Masters --------------------------------------------------------------------

// Vulnerability: "Master. Burn a vampire in torpor." Any Methuselah's; not a
// ready one.
#[test]
fn vulnerability_burns_a_vampire_in_torpor() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ready = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 1), 1, true);
    hand(&mut e, 0, &["Vulnerability"]);
    e.at_master_phase();
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let burn = |m: MinionId| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(m),
    };
    assert!(!req.options.contains(&burn(ready)));
    e.choose(index_of(&req, burn(t)));
    request(&mut e);
    assert!(!e.state.minions[t.index()].in_play);
    assert_eq!(pool(&e)[0], 10 - 1);
}

// Redeem the Lost Soul: "Master. Choose a vampire in your ash heap. Gain X
// pool, where X is half of the capacity of that vampire (round down). Remove
// that vampire from the game." Chosen as it is played: a capacity 7, 3 pool.
#[test]
fn redeem_the_lost_soul_gains_half_a_burned_vampires_capacity() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let seven = CRYPT.iter().position(|c| c.capacity == 7).expect("a 7") as u16;
    let four = CRYPT.iter().position(|c| c.capacity == 4).expect("a 4") as u16;
    e.state.seats[0].ash_crypt = vec![four, seven].into();
    hand(&mut e, 0, &["Redeem the Lost Soul"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    e.choose(index_of(&req, Choice::AshVampire(seven)));
    request(&mut e);
    assert_eq!(pool(&e)[0], 10 + 3);
    assert_eq!(e.state.seats[0].ash_crypt, vec![four]);
    assert!(e.state.seats[0].removed_crypt.contains(&seven));
}

// ---- Blocks and combat -----------------------------------------------------------

// Horrific Countenance: "Only usable when this vampire is blocked. [pro]
// Unlock the blocking minion. This action is not blocked, and it is now
// unblockable." (4 blood.) The bleed lands; seat 1's other vampire never
// gets to attempt.
#[test]
fn horrific_countenance_unblocks_the_action_for_good() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_disc("pro", false, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Horrific Countenance"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(y));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(w));
    assert!(!e.state.minions[y.index()].locked, "unlocked");
    assert_eq!(pool(&e)[1], 10 - 1);
    assert_eq!(blood(&e, a), 0);
}

// Fast Hands: "[cel] Strike: steal weapon. [CEL] Strike: steal weapon with
// first strike." [RTR 19980928] "Cannot be used if there is no weapon to
// steal": not offered against an unarmed vampire; against one with a .44
// Magnum, the gun moves to this vampire.
#[test]
fn fast_hands_steals_a_weapon_and_needs_one() {
    for armed in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("cel", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let gun = armed.then(|| in_play(&mut e, 1, ".44 Magnum", Some(z)));
        hand(&mut e, 0, &["Fast Hands"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        let Some(g) = gun else {
            never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
                matches!(o, Choice::Play { card: 0, .. })
            });
            continue;
        };
        pick_when(&mut e, play_at(0, 0, a, Target::Card(g)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.cards[g.index()].host, Some(a));
    }
}

// Orgy of Blood: "Only usable if the opposing ally is being burned. Each
// vampire you control gains X blood, where X is the cost to recruit the
// ally." [LSJ 20100721] "Can be used by a vampire going to torpor or about
// to be burned": a blocker with no blood burns the Spectral Servitor and
// goes to torpor in the same strikes; it plays the card at the end of the
// round, and every vampire of its controller's gains the ally's cost.
#[test]
fn orgy_of_blood_by_a_vampire_going_to_torpor() {
    let mut e = game(2);
    let s = e.add_ally(SeatId(0), find_library("Spectral Servitor"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    let other = e.add_minion(SeatId(1), plain("Ventrue", 1), 1, false);
    hand(&mut e, 1, &["Orgy of Blood"]);
    let lib = &zvtes::library::LIBRARY[find_library("Spectral Servitor") as usize];
    let x = match (lib.pool_cost, lib.blood_cost) {
        (zvtes::library::Cost::Fixed(n), _) | (_, zvtes::library::Cost::Fixed(n)) => n,
        other => panic!("{other:?}"),
    };
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(blood(&e, z), x);
    assert_eq!(blood(&e, other), 1 + x);
}

// Drawing Out the Beast: "Only usable before range is determined on the
// first round. Frenzy. [ani] During this combat, opposing vampire gets +1
// strength, but he or she cannot use maneuvers to maneuver to long range,
// cannot use presses to end combat and cannot use equipment. [ANI] As above,
// and the opposing vampire takes 1 unpreventable damage during the press
// step each round." [PIB 20121028] "Cannot be played against an ally."
#[test]
fn drawing_out_the_beast_is_not_played_against_an_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ani", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
    hand(&mut e, 0, &["Drawing Out the Beast"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
}

// Drawing Out the Beast [ANI]: [RTR 19970630] "The additional damage done
// during the press step is environmental" — on the opposing vampire, +1
// strength to it besides. [LSJ 20010312] [LSJ 20001027] "The effect in the
// press step does not happen if the combat ends prematurely (e.g. from
// torpor or 'strike: combat ends')": Majesty ends the combat — no damage.
#[test]
fn drawing_out_the_beast_damages_in_the_press_step_unless_combat_ended() {
    for ends in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), with_disc("ani", false, 0), 4, false);
        superior(&mut e, a, "ani");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), with_disc("pre", false, 0), 4, false);
        hand(&mut e, 0, &["Drawing Out the Beast"]);
        hand(&mut e, 1, &["Majesty"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 1, a));
        if ends {
            pick_when(&mut e, play(0, 0, z));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if ends {
            assert_eq!((blood(&e, a), blood(&e, z)), (4, 4 - 1));
        } else {
            // z's hand strike at strength 2; a's at 1, then the press step.
            assert_eq!((blood(&e, a), blood(&e, z)), (4 - 2, 4 - 1 - 1));
        }
    }
}

// Approximation of Loyalty: "Requires a vampire with capacity 7 or more.
// [pre] Only usable as the action is announced. Choose a vampire with
// capacity 6 or less or an ally. That minion cannot block this action."
#[test]
fn approximation_of_loyalty_bars_a_small_blocker() {
    let mut e = game(2);
    let big = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.capacity >= 7
                && c.disciplines.contains(&"pre")
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a big presence vampire") as u16;
    let a = e.add_minion(SeatId(0), big, 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let small = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Approximation of Loyalty"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_at(0, 0, a, Target::Minion(small)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(small));
    assert_eq!(pool(&e)[1], 10 - 1);
}

// ---- Political actions and cards in play -----------------------------------------

/// The plain prince of the crypt with no printed text, `nth`.
fn prince(nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            matches!(c.title, zvtes::crypt::Title::Prince(_))
                && c.capacity >= 4
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a plain prince")
}

// Rumors of Gehenna: "Choose one or more Methuselahs. If this referendum is
// successful, put this card into play. Each chosen Methuselah receives two
// master phase actions during his or her master phase (instead of one)."
// Passed for seat 1: the card remembers seat 1 alone; a second copy adds
// nothing ("instead of one").
#[test]
fn rumors_of_gehenna_gives_the_chosen_two_master_phase_actions() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Rumors of Gehenna"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ChooseSeat(SeatId(1)));
    pick_when(&mut e, Choice::Pass);
    to_referendum(&mut e);
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let rumors = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.card == find_library("Rumors of Gehenna") && c.in_play)
            .expect("in play") as u32,
    );
    assert!(e.state.cards[rumors.index()].against.contains(SeatId(1)));
    assert!(!e.state.cards[rumors.index()].against.contains(SeatId(0)));
    let base = |e: &Engine, s: u8| e.state.seats[s as usize].mpa;
    assert_eq!(e.state.mpa_left(SeatId(1)), base(&e, 1) + 1);
    assert_eq!(e.state.mpa_left(SeatId(2)), base(&e, 2));
    let second = in_play(&mut e, 2, "Rumors of Gehenna", None);
    e.state.cards[second.index()].against = zvtes::ids::SeatSet::single(SeatId(1));
    assert_eq!(
        e.state.mpa_left(SeatId(1)),
        base(&e, 1) + 1,
        "two, not three"
    );
}

// Camarilla Threat: "Each Methuselah must pay an additional pool to use a
// discard phase action to discard a card." Each copy its pool.
#[test]
fn camarilla_threat_charges_a_pool_per_discard() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Camarilla Threat", None);
    in_play(&mut e, 1, "Camarilla Threat", None);
    hand(&mut e, 0, &["Conceal"]);
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    e.choose(index_of(&req, Choice::Discard(0)));
    request(&mut e);
    assert_eq!(pool(&e)[0], 10 - 2);
}

/// The Meddling of Semsith in play for `seat`, its chosen Methuselah `of`.
fn meddling(e: &mut Engine, seat: u8, of: u8) -> CardId {
    let id = in_play(e, seat, "Meddling of Semsith, The", None);
    e.state.cards[id.index()].against = zvtes::ids::SeatSet::single(SeatId(of));
    id
}

// The Meddling of Semsith: "You get -1 hand size. The chosen Methuselah does
// not replace cards during any Methuselah's minion phase." [ANK 20180318]
// [ANK 20231229] "Cards that are not replaced count against the hand size":
// seat 1 plays Power of One in its minion phase; its hand stays short until
// the phase ends, the card held counting against its hand size (no draw
// up), then replaced. Seat 0 is at 6 cards.
#[test]
fn meddling_holds_replacements_through_the_minion_phase() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let y = e.add_minion(SeatId(1), anarch_with("pot", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    meddling(&mut e, 0, 1);
    assert_eq!(e.state.hand_size(SeatId(0)), 6);
    hand(&mut e, 1, &["Power of One"]);
    fill_hand(&mut e, 1);
    e.state.seats[1].library = vec![find_library("Conceal"); 3].into();
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, bleed(y));
    pick_when(&mut e, play(0, 1, y));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].hand.len(), 6, "not replaced");
    assert_eq!(e.state.seats[1].held[Release::MinionPhaseEnd], 1);
    assert_eq!(e.state.seats[1].library.len(), 3, "no draw up either");
    e.choose(index_of(&req, Choice::Pass));
    request(&mut e);
    assert_eq!(e.state.seats[1].hand.len(), 7, "replaced as the phase ends");
    assert!(e.state.seats[1].hand.contains(&find_library("Conceal")));
}

// [LSJ 20070320-1] The Meddling of Semsith: "The 'do not replace' effect
// stacks with whatever 'do not replace' clause the card itself has, so that
// the longest one wins out": Fake Out ("do not replace until after
// combat"), played by the chosen Methuselah in a combat of the minion phase,
// waits for the phase's end, not the combat's.
#[test]
fn meddling_outlasts_a_cards_own_delay() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    meddling(&mut e, 0, 1);
    hand(&mut e, 1, &["Fake Out"]);
    fill_hand(&mut e, 1);
    e.state.seats[1].library = vec![find_library("Conceal"); 3].into();
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].hand.len(), 6, "not replaced after combat");
    assert_eq!(e.state.seats[1].held[Release::MinionPhaseEnd], 1);
}

// [LSJ 20021008] The Meddling of Semsith: "Stays in play and still reduces
// your handsize when the target is ousted." "Is removed from play and its
// effect cancelled if the Methuselah who played it is ousted."
#[test]
fn meddling_survives_its_target_not_its_player() {
    // The chosen Methuselah ousted: it stays.
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    let m = meddling(&mut e, 0, 1);
    e.state.seats[1].pool = 1;
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert!(e.state.cards[m.index()].in_play);
    assert_eq!(e.state.hand_size(SeatId(0)), 6);
    // Its player ousted: gone, the chosen one replaces again.
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, true);
    let b = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let m = meddling(&mut e, 0, 1);
    e.state.seats[0].pool = 1;
    to_minion_phase(&mut e, SeatId(2));
    pick(&mut e, bleed(b));
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].ousted);
    assert!(!e.state.cards[m.index()].present());
    assert!(!e.state.meddled(SeatId(1)));
}

// Eat the Rich: "Requires an Anarch. Choose one or more Methuselahs.
// Successful referendum means each chosen Methuselah burns 1 pool, plus 3
// additional pool if they control a ready titled vampire. In this
// referendum, each priscus gets -1 ballot, each non-priscus titled vampire
// gets -1 vote, and burning the Edge is worth 1 additional vote." Seat 1's
// prince casts 1 vote, Genevieve her 1 vote and no ballot; seat 0's Edge
// is worth 2; seat 1, chosen, burns 4.
#[test]
fn eat_the_rich_taxes_titles_and_doubles_the_edge() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), prince(0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Genevieve"), 4, false);
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Eat the Rich"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ChooseSeat(SeatId(1)));
    pick_when(&mut e, Choice::Pass);
    to_referendum(&mut e);
    pick_when(&mut e, Choice::CastEdge { favor: true });
    pick_when(&mut e, votes(p, false));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    pick_when(&mut e, votes(g, false));
    let r = e.referendum().expect("referendum");
    let cast: Vec<(u8, u8, bool)> = r
        .records
        .iter()
        .map(|v| (v.n, v.ballots, v.favor))
        .collect();
    assert_eq!(
        cast,
        vec![(2, 0, true), (1, 0, false), (1, 0, true), (1, 0, false)]
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 4);
}

// Anathema: "If the attached vampire's blood is reduced to 0 in combat, the
// attached vampire is burned and the Methuselah controlling the opposing
// minion gains pool equal to the attached vampire's capacity." [RTR
// 19980623] "Burns the target vampire when the target is reduced to zero
// blood in combat, regardless of the source of the loss." [RTR 19980623]
// "Will not burn a vampire for entering a combat with zero blood: there
// must be an actual blood loss": a blocker at 1 blood loses it to a hand
// strike and burns, seat 0 gaining its capacity; one at 0 goes to torpor.
#[test]
fn anathema_burns_a_vampire_reduced_to_zero_not_one_at_zero() {
    for start in [1u8, 0] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), start, false);
        in_play(&mut e, 1, "Anathema", Some(z));
        let cap = e.state.capacity_of(z).unwrap() as i32;
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if start == 1 {
            assert!(!e.state.minions[z.index()].in_play, "burned");
            assert_eq!(pool(&e)[0], 10 + cap);
        } else {
            assert!(e.state.minions[z.index()].in_torpor, "torpor, not burned");
            assert_eq!(pool(&e)[0], 10);
        }
    }
}

// [LSJ 20021120] Anathema: "Does not trigger if the vampire is burned by
// aggravated damage during damage resolution": Wolf Claws' aggravated hand
// strike, at Hardestadt's 3 strength, burns the 1-blood blocker — no pool.
#[test]
fn anathema_does_not_pay_for_an_aggravated_burn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Hardestadt"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    in_play(&mut e, 1, "Anathema", Some(z));
    hand(&mut e, 0, &["Wolf Claws"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_play, "burned by the damage");
    assert_eq!(pool(&e)[0], 10);
}

// [LSJ 19980225] Anathema: "Damage done by combat effects after combat ends
// do not trigger Anathema": Oubliette [OBT]'s blood, burned after combat,
// takes the last one — no burn.
#[test]
fn anathema_ignores_a_loss_after_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_disc("obt", false, 0), 4, false);
    superior(&mut e, a, "obt");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    in_play(&mut e, 1, "Anathema", Some(z));
    hand(&mut e, 0, &["Oubliette"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 0);
    assert!(e.state.minions[z.index()].in_play, "not burned");
}

// [LSJ 20021117] Anathema: "Multiple Anathemas don't multiply the pool gain.
// Once one Anathema resolves it burns the vampire and the others are burned
// before they get to resolve." [PIB 20121028] "It is controlled by the
// controller of the minion it's on - it is not removed when another
// Methuselah is ousted": both cards played by seat 2, ousted before.
#[test]
fn anathema_pays_once_and_belongs_to_its_hosts_controller() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, true);
    let x = in_play(&mut e, 2, "Anathema", Some(z));
    let y = in_play(&mut e, 2, "Anathema", Some(z));
    assert_eq!(e.state.card_controller(x), SeatId(1));
    e.state.seats[2].ousted = true;
    let cap = e.state.capacity_of(z).unwrap() as i32;
    assert!(e.state.cards[x.index()].in_play && e.state.cards[y.index()].in_play);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_play);
    assert_eq!(pool(&e)[0], 10 + cap, "once");
    assert!(!e.state.cards[x.index()].in_play && !e.state.cards[y.index()].in_play);
}

// [LSJ 20021122-2] [ANK 20211112] Anathema: "Diablerie does not trigger
// Anathema": the diablerised vampire carrying one burns, no pool for it.
#[test]
fn anathema_is_not_triggered_by_diablerie() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let victim = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Anathema", Some(victim));
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    for _ in 0..60 {
        let req = request(&mut e);
        if req.window == WindowLabel::MinionPhase && req.seat == SeatId(0) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(!e.state.minions[victim.index()].in_play);
    assert_eq!(pool(&e)[0], 10, "no Anathema gain");
}

// Protected Resources: "If you would burn more than 2 pool to a bleed, you
// burn 2 pool instead. After a minion you control successfully bleeds, burn
// this card." [TOM 19960413] "All cards which modify the bleed amount are
// applied before Protected Resources's effect." [LSJ 20030211] "Does not
// alter the bleed amount: it merely limits the maximum amount of pool the
// target of the bleed burns for the successful bleed": a bleed of 3 (two
// modifiers) burns 2; the bleed still succeeds — the Edge moves.
#[test]
fn protected_resources_caps_the_pool_burned_not_the_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch_with("pot", 0), 4, false);
    superior(&mut e, a, "ani");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let pr = in_play(&mut e, 1, "Protected Resources", None);
    hand(&mut e, 0, &["Adhocracy", "Power of One"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 2);
    assert!(e.state.seats[0].has_edge, "a successful bleed");
    assert!(
        e.state.cards[pr.index()].in_play,
        "not its controller's bleed"
    );
}

// [LSJ 20030701-1] Protected Resources: "If you perform a bleed action that
// resolves as a bleed for 0, you don't burn your Protected Resources": a
// 0-bleed ally's bleed keeps it; a vampire's bleed of 1 burns it.
#[test]
fn protected_resources_burns_after_its_controllers_successful_bleed() {
    let mut e = game(2);
    let dog = e.add_ally(SeatId(0), find_library("Dog Pack"));
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let pr = in_play(&mut e, 0, "Protected Resources", None);
    pick(&mut e, bleed(dog));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[pr.index()].in_play, "a bleed for 0");
    pick(&mut e, bleed(a));
    request(&mut e);
    assert!(!e.state.cards[pr.index()].in_play, "burned");
}

// Antediluvian Awakening: "Unique. Put this card in play. During each
// Methuselah's unlock phase, that Methuselah burns 1 pool unless they burn
// a vampire they control with capacity 4 or more to burn this card." Seat 1
// pays its pool; seat 0 burns a vampire, and the card.
#[test]
fn antediluvian_awakening_costs_a_pool_or_a_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let aa = in_play(&mut e, 1, "Antediluvian Awakening", None);
    let req = until(&mut e, SeatId(1), WindowLabel::RiderTerms);
    e.choose(index_of(&req, Choice::BurnPool));
    let req = until(&mut e, SeatId(0), WindowLabel::RiderTerms);
    assert_eq!(pool(&e)[1], 10 - 1);
    e.choose(index_of(&req, Choice::ChooseMinion(a)));
    request(&mut e);
    assert!(!e.state.minions[a.index()].in_play);
    assert!(!e.state.cards[aa.index()].in_play);
    assert_eq!(pool(&e)[0], 10);
}

// Third Tradition: Progeny: "+1 stealth action. Requires a non-sterile
// prince or justicar. Put this card in play. It becomes a 1-capacity
// (non-unique) Camarilla vampire of the same clan as the acting vampire and
// cannot perform actions this turn. … Move up to 2 blood from the acting
// vampire to this new vampire." [PIB 20150418] "The decision to move blood
// on the Progeny or not is taken upon resolution": 0 to 2, asked then.
// (`progeny` plays it; the test below checks it.)
fn progeny(e: &mut Engine, p: MinionId, moved: u8) -> MinionId {
    let before = e.state.minions.len();
    hand(e, 0, &["Third Tradition: Progeny"]);
    let card = (e.state.seats[0].hand.len() - 1) as u8;
    pick(e, play(card, 0, p));
    let req = offering(e, Choice::SetX(moved));
    assert_eq!(
        req.options,
        vec![Choice::SetX(0), Choice::SetX(1), Choice::SetX(2)]
    );
    e.choose(index_of(&req, Choice::SetX(moved)));
    request(e);
    MinionId(before as u32)
}

// [PIB 20150418] Third Tradition: Progeny: the blood moved decided upon
// resolution; the progeny a Camarilla vampire of the prince's clan that
// cannot act this turn.
#[test]
fn third_tradition_progeny_moves_blood_decided_at_resolution() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), prince(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let n = progeny(&mut e, p, 2);
    let v = &e.state.minions[n.index()];
    assert!(
        v.in_play && v.is_vampire() && v.recruited,
        "cannot act this turn"
    );
    assert_eq!(e.state.sect_of(n), Some(zvtes::crypt::Sect::Camarilla));
    assert_eq!(
        e.state.minions[n.index()].clan(),
        e.state.minions[p.index()].clan()
    );
    assert_eq!(blood(&e, p), 4 - 1 - 2, "its 1 blood, then the 2 moved");
    // The 2 moved, the excess over its capacity drained.
    assert_eq!(blood(&e, n), 1);
}

// Kaymakli Nightmares: "All Methuselahs move any crypt cards in their
// uncontrolled regions to their crypts. Any blood counters on them are moved
// to the Methuselah's pool, and any cards on them are burned. Each
// Methuselah shuffles his or her crypt and then moves as many cards from his
// or her crypt to his or her uncontrolled region as he or she shuffled in."
// [LSJ 20080229] "'Any crypt cards' means all of them": both of seat 1's go,
// their 3 blood to its pool, two come back out. [LSJ 20041201] "Crypt cards
// cannot be put in the library and library cards cannot be put in the
// crypt." [LSJ 20041201-2] A created vampire: "If it would be put into the
// crypt for any reason, it is burned instead": seat 0's banished progeny
// burns, its card to the ash heap, and none comes out for it.
#[test]
fn kaymakli_nightmares_reshuffles_every_uncontrolled_crypt_card() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), prince(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let n = progeny(&mut e, p, 0);
    let card = e
        .state
        .cards
        .iter()
        .position(|c| c.body == Some(n))
        .expect("its card");
    e.state.minions[n.index()].in_play = false;
    e.state.cards[card].in_play = false;
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: zvtes::state::CREATED,
            copy: Default::default(),
            counters: 1,
            banished: Some(zvtes::state::Banished {
                minion: n,
                cards: vec![CardId(card as u32)],
            }),
        });
    let crypt: Vec<u16> = (10..15).collect();
    e.state.seats[1].crypt = crypt[2..].to_vec().into();
    for (k, c) in [(2u8, crypt[0]), (1, crypt[1])] {
        e.state.seats[1]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card: c,
                copy: Default::default(),
                counters: k,
                banished: None,
            });
    }
    let pools = pool(&e);
    hand(&mut e, 0, &["Kaymakli Nightmares"]);
    to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let k = (e.state.seats[0].hand.len() - 1) as u8;
    pick(&mut e, master_play(k));
    request(&mut e);
    let s1 = &e.state.seats[1];
    assert_eq!(s1.uncontrolled.len(), 2);
    assert!(s1.uncontrolled.iter().all(|u| u.counters == 0));
    assert_eq!(s1.crypt.len(), 3);
    assert_eq!(pool(&e)[1], pools[1] + 3);
    let s0 = &e.state.seats[0];
    assert!(s0.uncontrolled.is_empty(), "burned, none drawn for it");
    assert!(s0.ash.contains(&find_library("Third Tradition: Progeny")));
    assert_eq!(pool(&e)[0], pools[0] + 1, "its counter to the pool");
}

// Approximation of Loyalty [PRE]: "Cancel a reaction card played by a
// vampire with capacity 6 or less or an ally as it is played (no cost is
// paid). No more action modifiers can be played on this action." Enhanced
// Senses canceled; Aire of Elation, a bleed modifier, is no longer offered.
#[test]
fn approximation_of_loyalty_cancels_a_small_reaction_and_closes_modifiers() {
    let mut e = game(2);
    let big = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.capacity >= 7
                && c.disciplines.contains(&"pre")
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a big presence vampire") as u16;
    let a = e.add_minion(SeatId(0), big, 6, false);
    superior(&mut e, a, "pre");
    superior(&mut e, a, "obf");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), with_disc("aus", false, 0), 4, false);
    hand(
        &mut e,
        0,
        &[
            "Approximation of Loyalty",
            "Lost in Crowds",
            "Aire of Elation",
        ],
    );
    hand(&mut e, 1, &["Enhanced Senses"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(1, 0, a));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 1, a));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a),
    );
    assert_eq!(pool(&e)[1], 10 - 1, "the block failed");
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Enhanced Senses")));
}

// Zaire River Ferry: "Lock as an action directed at this Lasombra is
// announced to have that action fail. Lock to give this Lasombra +1
// stealth." [LSJ 20081105] "The acting player has the opportunity to play
// 'as announced' effects first (eg. Seduction)." [ANK 20190425] "The ability
// to fail an action must be used before action modifiers or reactions that
// are not explicitly played 'as the action is announced'." [ANK 20211015]
// [RTR 20180719] "If the action fails, the action has reached resolution, so
// is subject to NRA (Non Repeatable Action) rules." [LSJ 20070219] "If an
// action fails, it was not blocked": seat 0 plays Seduction as announced
// first, then the Ferry fails the Bum's Rush — no combat, no block, and no
// second Bum's Rush this turn.
#[test]
fn zaire_river_ferry_fails_an_action_at_its_lasombra() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_disc("dom", false, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let l = e.add_minion(SeatId(1), plain("Lasombra", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Lasombra", 1), 4, false);
    let ferry = in_play(&mut e, 1, "Zaire River Ferry", Some(l));
    hand(&mut e, 0, &["Bum's Rush", "Seduction", "Bum's Rush"]);
    let rush = play_at(0, 0, a, Target::Minion(l));
    pick(&mut e, rush);
    let req = offering(&mut e, use_at(ferry, 0, None, Target::None));
    assert_eq!(req.seat, SeatId(1));
    // Seduction was offered to seat 0 first, as announced.
    e.choose(index_of(&req, use_at(ferry, 0, None, Target::None)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(w));
    assert_eq!(e.last_outcome, Some(Outcome::Failed));
    assert!(e.state.cards[ferry.index()].locked);
    assert_eq!((blood(&e, a), blood(&e, l)), (4, 4), "no combat");
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a)),
        "a's actions are done: NRA"
    );
}

// Zaire River Ferry's +1 stealth: its Lasombra bleeds, a blocker attempts —
// the Ferry locks, the block fails.
#[test]
fn zaire_river_ferry_gives_its_lasombra_stealth() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), plain("Lasombra", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ferry = in_play(&mut e, 0, "Zaire River Ferry", Some(l));
    pick(&mut e, bleed(l));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_at(ferry, 1, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1);
}

// The Ankara Citadel, Turkey: "Non-action modifier cards costing blood cost
// this vampire half as much blood to play, rounded down (multiplication and
// division are handled first, followed by addition and subtraction)." [RTR
// 20070707] "Concerning the cost of the cards, multiplications and
// divisions are applied first, followed by additions and subtractions":
// Wolf Claws (1 blood) costs its bearer 0; Aire of Elation, an action
// modifier, its 1.
#[test]
fn ankara_citadel_halves_its_bearers_non_modifier_costs() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with_disc("pro", false, 0), 4, false);
    superior(&mut e, a, "pre");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Ankara Citadel, Turkey, The", Some(a));
    hand(&mut e, 0, &["Aire of Elation", "Wolf Claws"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(blood(&e, a), 4 - 1, "an action modifier, paid in full");
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Wolf Claws free; the blocker's hand strike took one.
    assert_eq!(blood(&e, a), 4 - 1 - 1);
}

/// The plain baron of the largest capacity.
fn big_baron() -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            matches!(c.title, zvtes::crypt::Title::Baron(_))
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .max_by_key(|(_, c)| c.capacity)
        .map(|(i, _)| i as u16)
        .expect("a plain baron")
}

/// The `nth` plain untitled non-Anarch vampire of capacity 2 or 3.
fn small(nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            (2..=3).contains(&c.capacity)
                && c.title == zvtes::crypt::Title::None
                && c.sect != zvtes::crypt::Sect::Anarch
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a small plain vampire")
}

// Chameleon: "+1 stealth action. Requires a baron. Ⓓ Take control of a
// younger vampire with capacity of X who entered play since your last
// minion phase. That vampire becomes Anarch." (X blood.) [PIB 20150728]
// "Cannot be used on a vampire who merged last turn: merging is not
// entering play": one merged since is no target, one that entered since is
// — it changes hands and becomes Anarch, for its capacity in blood.
#[test]
fn chameleon_steals_a_vampire_that_entered_not_one_that_merged() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), big_baron(), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let merged = e.add_minion(SeatId(1), small(0), 4, false);
    request(&mut e);
    let clock = e.state.entry_clock;
    e.state.seats[0].minion_phase_end = clock;
    e.state.minions[merged.index()].merged = Some(0);
    let fresh = e.add_minion(SeatId(1), small(1), 4, false);
    hand(&mut e, 0, &["Chameleon"]);
    let req = request(&mut e);
    let on = |m: MinionId| play_at(0, 0, b, Target::Minion(m));
    assert!(
        !req.options.contains(&on(merged)),
        "merging is not entering play"
    );
    e.choose(index_of(&req, on(fresh)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let cap = e.state.capacity_of(fresh).unwrap();
    assert_eq!(e.state.minions[fresh.index()].owner, SeatId(0));
    assert_eq!(e.state.sect_of(fresh), Some(zvtes::crypt::Sect::Anarch));
    assert_eq!(blood(&e, b), e.state.capacity_of(b).unwrap().min(8) - cap);
}

// [LSJ 20080803-2] Chameleon: "Can be used on one of your own vampire to
// make it Anarch."
#[test]
fn chameleon_makes_ones_own_vampire_anarch() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), big_baron(), 8, false);
    let own = e.add_minion(SeatId(0), small(0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Chameleon"]);
    pick(&mut e, play_at(0, 0, b, Target::Minion(own)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[own.index()].owner, SeatId(0));
    assert_eq!(e.state.sect_of(own), Some(zvtes::crypt::Sect::Anarch));
}

// Infernal Servitor: "Requires an infernal vampire. [dai] Infernal Servitor
// can perform actions the turn it is recruited. Remove it from the game if
// you control no infernal vampires, or after it performs a successful
// action." Recruited by Arishat, it bleeds at once, then leaves the game; a
// second one leaves as Arishat burns.
#[test]
fn infernal_servitor_leaves_after_success_or_without_an_infernal() {
    let mut e = game(2);
    let ar = e.add_minion(SeatId(0), find_card("Arishat"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    hand(&mut e, 0, &["Infernal Servitor"]);
    pick_when(&mut e, play(0, 0, ar));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let s = MinionId(e.state.minions.len() as u32 - 1);
    assert!(e.state.minions[s.index()].is_ally());
    e.choose(index_of(&req, bleed(s)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 3);
    assert!(
        !e.state.minions[s.index()].in_play,
        "removed after its success"
    );
    assert!(e.state.seats[0]
        .removed
        .contains(&find_library("Infernal Servitor")));
    // A second one, its infernal vampire burned.
    let t = e.add_ally(SeatId(0), find_library("Infernal Servitor"));
    e.burn(ar);
    request(&mut e);
    assert!(!e.state.minions[t.index()].in_play);
}

// [LSJ 20010729] Soul Gem of Etrius: "Functions even if the bearer (now
// burned) was prohibited from using equipments (eg. because of Drawing Out
// the Beast)": the bearer burned in a combat under Drawing Out the Beast,
// the Gem still draws.
#[test]
fn soul_gem_works_under_drawing_out_the_beast() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hardestadt"), 6, false);
    superior(&mut e, h, "ani");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    in_play(&mut e, 1, "Soul Gem of Etrius", Some(z));
    e.state.seats[1].crypt = vec![plain("Ventrue", 3)].into();
    hand(&mut e, 0, &["Drawing Out the Beast", "Wolf Claws"]);
    pick(&mut e, bleed(h));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, h));
    pick_when(&mut e, play(0, 0, h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_play, "burned");
    assert!(e.state.seats[1].crypt.is_empty(), "the Gem drew");
}

// [RTR 19960112] A location moved onto another minion: Zaire River Ferry on a
// Brujah — "this Lasombra" no longer — offers neither ability.
#[test]
fn zaire_river_ferry_needs_a_lasombra_bearer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let ferry = in_play(&mut e, 1, "Zaire River Ferry", Some(b));
    hand(&mut e, 0, &["Bum's Rush"]);
    pick(&mut e, play_at(0, 0, a, Target::Minion(b)));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == ferry),
    );
}

// The Ankara Citadel, Turkey (owner, 2026-10-14: rules-qa 70.17): a dual
// Action Modifier/Reaction card played as a reaction is halved — Scalpel
// Tongue (1 blood) costs a reacting bearer 0 — but not played as an action
// modifier by the acting bearer, who pays its 1.
#[test]
fn ankara_citadel_halves_a_dual_card_played_as_a_reaction() {
    for reacting in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), prince(0), 4, false);
        superior(&mut e, a, "pre");
        superior(&mut e, a, "cel");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let r = e.add_minion(SeatId(1), prince(1), 4, false);
        superior(&mut e, r, "pre");
        superior(&mut e, r, "cel");
        let (bearer, seat, voter) = if reacting { (r, 1, a) } else { (a, 0, r) };
        in_play(&mut e, seat, "Ankara Citadel, Turkey, The", Some(bearer));
        hand(&mut e, seat, &["Scalpel Tongue"]);
        hand(&mut e, 0, &["Ancilla Empowerment"]);
        let call = (e.state.seats[0].hand.len() - 1) as u8;
        pick(&mut e, play(call, 0, a));
        to_referendum(&mut e);
        pick_when(&mut e, votes(voter, true));
        pick_when(&mut e, play_at(0, 0, bearer, Target::Minion(voter)));
        request(&mut e);
        assert_eq!(blood(&e, bearer), if reacting { 4 } else { 4 - 1 });
    }
}

// The Ankara Citadel, Turkey (owner, 2026-10-14: rules-qa 70.17): an X cost
// is halved once X is chosen — Chameleon at X 3 costs its bearer 1.
#[test]
fn ankara_citadel_halves_an_x_cost() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), big_baron(), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let three = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.capacity == 3
                && c.title == zvtes::crypt::Title::None
                && c.sect != zvtes::crypt::Sect::Anarch
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a capacity 3") as u16;
    let t = e.add_minion(SeatId(1), three, 3, false);
    in_play(&mut e, 0, "Ankara Citadel, Turkey, The", Some(b));
    hand(&mut e, 0, &["Chameleon"]);
    request(&mut e);
    let before = blood(&e, b);
    pick(&mut e, play_at(0, 0, b, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[t.index()].owner, SeatId(0));
    assert_eq!(blood(&e, b), before - 1);
}

// Eat the Rich (owner, 2026-10-14: rules-qa 70.6): "each priscus gets -1
// ballot" — of its ballots in all: Gratiano, a priscus with "+1 ballot",
// casts 1.
#[test]
fn eat_the_rich_leaves_gratiano_one_ballot() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Gratiano"), 4, false);
    hand(&mut e, 0, &["Eat the Rich"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ChooseSeat(SeatId(1)));
    pick_when(&mut e, Choice::Pass);
    to_referendum(&mut e);
    pick_when(&mut e, votes(g, false));
    let r = e.referendum().expect("referendum");
    let mine: Vec<u8> = r
        .records
        .iter()
        .filter(|v| v.source == zvtes::referendum::Source::Vampire(g))
        .map(|v| v.ballots)
        .collect();
    assert_eq!(mine, vec![1]);
}
