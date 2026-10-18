//! Legacy cards for TWDA decks, round 16 (`ADDED_BATCHES`, slice 16ab): the
//! TWDA greedy's steps 1-6 and five promo sets completed. Readings in
//! rules-qa Batch 80. Windows with a single option are elided, so the
//! scripts only stop where a real decision exists.

use zvtes::cards::{Stat, Target};
use zvtes::crypt::CRYPT;
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

/// A crypt card of `clan` with capacity in `caps`, no printed-text
/// definition.
fn of_clan(clan: &str, caps: std::ops::RangeInclusive<u8>) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.clan == clan
                && caps.contains(&c.capacity)
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .unwrap_or_else(|| panic!("no {clan} of capacity {caps:?}")) as u16
}

/// A Sabbat crypt card, no printed-text definition, capacity 4 to 7.
fn sabbat() -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.sect == zvtes::crypt::Sect::Sabbat
                && (4..=7).contains(&c.capacity)
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a plain Sabbat vampire") as u16
}

fn against(opponent: MinionId) -> PrintedCtx {
    PrintedCtx {
        opponent: Some(opponent),
        ..PrintedCtx::default()
    }
}

// ---- Vampires -----------------------------------------------------------------

// Aisling Sturbridge: "Younger Tremere cannot block Aisling." A younger
// Tremere is no candidate; an older one and a younger non-Tremere are.
#[test]
fn aisling_sturbridge_cannot_be_blocked_by_younger_tremere() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Aisling Sturbridge"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let young = e.add_minion(SeatId(1), find_card("Almiro Suarez"), 2, false);
    let old = e.add_minion(SeatId(1), of_clan("Tremere", 6..=11), 6, false);
    let other = e.add_minion(SeatId(1), of_clan("Ventrue", 2..=4), 2, false);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, block(old));
    assert!(req.options.contains(&block(other)));
    assert!(!req.options.contains(&block(young)));
}

// Blythe Candeleria: -1 strength in combat with a Malkavian; Kite +1
// intercept; Rodolfo +1 bleed; Richard Tauber -1 intercept against
// Lasombra and Tzimisce; Yazid Tamari +1 stealth on directed actions.
#[test]
fn round_sixteen_printed_stats() {
    let mut e = game(2);
    let blythe = e.add_minion(SeatId(0), find_card("Blythe Candeleria"), 3, false);
    let kite = e.add_minion(SeatId(0), find_card("Kite"), 7, false);
    let rodolfo = e.add_minion(SeatId(0), find_card("Rodolfo"), 8, false);
    let richard = e.add_minion(
        SeatId(0),
        find_card("Richard Tauber, Ayelea's Puppet"),
        4,
        false,
    );
    let yazid = e.add_minion(SeatId(0), find_card("Yazid Tamari"), 8, false);
    let malk = e.add_minion(SeatId(1), plain("Malkavian", 0), 4, false);
    let lasombra = e.add_minion(SeatId(1), plain("Lasombra", 0), 4, false);
    let st = &e.state;
    let d = PrintedCtx::default();
    assert_eq!(st.printed_mod(blythe, Stat::Strength, against(malk)), -1);
    assert_eq!(st.printed_mod(blythe, Stat::Strength, against(lasombra)), 0);
    assert_eq!(st.printed_mod(kite, Stat::Intercept, d), 1);
    assert_eq!(st.printed_mod(rodolfo, Stat::Bleed, d), 1);
    assert_eq!(
        st.printed_mod(richard, Stat::Intercept, against(lasombra)),
        -1
    );
    assert_eq!(st.printed_mod(richard, Stat::Intercept, against(malk)), 0);
    let directed = |on| PrintedCtx {
        kind: Some(ActionKind::Bleed),
        directed: Some(on),
        ..PrintedCtx::default()
    };
    assert_eq!(st.printed_mod(yazid, Stat::Stealth, directed(true)), 1);
    assert_eq!(st.printed_mod(yazid, Stat::Stealth, directed(false)), 0);
}

// Karsh (ADV): "He cannot block infernal vampires": an infernal acting
// vampire's bleed offers him no block.
#[test]
fn karsh_cannot_block_an_infernal_vampire() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nergal"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let adv = CRYPT
        .iter()
        .position(|c| c.name == "Karsh" && c.adv)
        .expect("Karsh (ADV)") as u16;
    let k = e.add_minion(SeatId(1), adv, 6, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(n));
    let req = offering(&mut e, block(z));
    assert!(!req.options.contains(&block(k)));
}

/// A vampire's own ability `ability` (its text), used by its Methuselah.
fn own(m: MinionId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(m),
        ability,
        actor: None,
        target: Target::None,
    }
}

// Qawiyya el-Ghaduba: "can burn 2 blood to unlock during your minion
// phase": once in it (ANK 20190207, rules-qa 6.6); +2 strength.
// [ANK 20190207] Once per minion phase.
#[test]
fn qawiyya_unlocks_once_in_her_minion_phase() {
    let mut e = game(2);
    let q = e.add_minion(SeatId(0), find_card("Qawiyya el-Ghaduba"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[q.index()].locked = true;
    assert_eq!(
        e.state
            .printed_mod(q, Stat::Strength, PrintedCtx::default()),
        2
    );
    pick(&mut e, own(q, 0));
    assert!(!e.state.minions[q.index()].locked);
    assert_eq!(blood(&e, q), 4);
    e.state.minions[q.index()].locked = true;
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, own(q, 0));
}

// Okulos: "Once each turn, Okulos can burn 1 blood to draw 1 card (discard
// down afterward)": once in a turn, the hand back to its size.
#[test]
fn okulos_draws_once_each_turn() {
    let mut e = game(2);
    let o = e.add_minion(SeatId(0), find_card("Okulos"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].library = (0..10).map(|_| find_library("Form of Mist")).collect();
    fill_hand(&mut e, 0);
    let lib = e.state.seats[0].library.len();
    pick(&mut e, own(o, 0));
    assert_eq!(blood(&e, o), 4);
    assert_eq!(e.state.seats[0].library.len(), lib - 1, "a card drawn");
    let req = request(&mut e);
    assert!(!req.options.contains(&own(o, 0)), "once this turn");
}

// Len Konopka: "If Len is ready at the start of your influence phase, you
// get +1 transfer": none from torpor.
#[test]
fn len_konopka_gives_a_transfer_while_ready() {
    let mut e = game(2);
    let len = e.add_minion(SeatId(0), find_card("Len Konopka"), 8, false);
    let d = PrintedCtx::default();
    assert_eq!(e.state.printed_mod(len, Stat::Transfers, d), 1);
    e.state.minions[len.index()].in_torpor = true;
    assert_eq!(e.state.printed_mod(len, Stat::Transfers, d), 0);
}

// Marie Faucigny: "Non-weapon equipment cost her -1 blood or pool to
// equip": a Laptop Computer free, a .44 Magnum at full cost; a
// locquipment's blood too (Palatial Estate, LSJ 20080109).
// [LSJ 20080109] Locations that are equipment included (Palatial Estate).
#[test]
fn marie_faucigny_equips_non_weapons_cheaper() {
    for (card, pool_after, blood_after) in [
        ("Laptop Computer", 10, 7),
        (".44 Magnum", 8, 7),
        ("Palatial Estate", 10, 6),
    ] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Marie Faucigny"), 7, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &[card]);
        pick(&mut e, play(0, 0, m));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[0].pool, pool_after, "{card}");
        assert_eq!(blood(&e, m), blood_after, "{card}");
    }
}

// Menele (ADV): "While Helena is ready, minion cards cost Menele 1 fewer
// blood": any Methuselah's Helena.
#[test]
fn menele_plays_cheaper_while_helena_is_ready() {
    for helena in [false, true] {
        let mut e = game(2);
        let adv = CRYPT
            .iter()
            .position(|c| c.name == "Menele" && c.adv)
            .expect("Menele (ADV)") as u16;
        let m = e.add_minion(SeatId(0), adv, 8, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if helena {
            e.add_minion(SeatId(1), find_card("Helena"), 8, false);
        }
        let pctx = PrintedCtx {
            card: Some((find_library("Majesty"), 0)),
            ..PrintedCtx::default()
        };
        let off = e.state.printed_mod(m, Stat::BloodCostOff, pctx);
        assert_eq!(off, i32::from(helena), "Helena ready: {helena}");
    }
}

// Fiorenza Savona: "If you burn the Edge for a vote, you get +1 vote. If
// you burn a political action card for a vote, you get +1 vote." Praxis
// Seizure: York: "Successful referendum means this card is put on this
// vampire to represent the unique Camarilla title of Prince of York" —
// the calling vampire's.
#[test]
fn fiorenza_savona_doubles_edge_and_card_votes_and_praxis_seizure_titles_her() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), of_clan("Ventrue", 4..=6), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), find_card("Fiorenza Savona"), 7, false);
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Praxis Seizure: York"]);
    hand(&mut e, 1, &["Praxis Seizure: York"]);
    pick(&mut e, play(0, 0, caller));
    to_referendum(&mut e);
    pick_when(&mut e, Choice::CastEdge { favor: false });
    pick_when(
        &mut e,
        Choice::BurnForVote {
            card: 0,
            favor: false,
        },
    );
    let r = e.referendum().expect("referendum");
    let theirs: Vec<u8> = r
        .records
        .iter()
        .filter(|v| v.seat == SeatId(1))
        .map(|v| v.n)
        .collect();
    assert_eq!(theirs, vec![2, 2], "the Edge and the card, +1 each");
    // Seat 0's own vote alone does not carry it; without the card vote
    // either, the referendum fails, the card to the ash heap.
    close_polling(&mut e);
    request(&mut e);
    assert_ne!(
        e.state.title_of(caller),
        zvtes::crypt::Title::Prince("York")
    );
}

// Praxis Seizure: York: "Successful referendum means this card is put on
// this vampire to represent the unique Camarilla title of Prince of York":
// on the calling vampire.
#[test]
fn praxis_seizure_york_titles_the_calling_vampire() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), of_clan("Ventrue", 4..=6), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Praxis Seizure: York"]);
    pick(&mut e, play(0, 0, caller));
    to_referendum(&mut e);
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    request(&mut e);
    assert_eq!(
        e.state.title_of(caller),
        zvtes::crypt::Title::Prince("York")
    );
}

// Dr. Solomon Grey: "Burn 1 pool as Solomon enters play": influenced out,
// before any contest (LSJ 20080526).
// [LSJ 20080526] The pool burned before any contest.
#[test]
fn dr_solomon_grey_burns_a_pool_as_he_enters_play() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: find_card("Dr. Solomon Grey"),
            copy: Default::default(),
            counters: 2,
            banished: None,
        });
    loop {
        let req = request(&mut e);
        if req.seat == SeatId(0) && req.window == WindowLabel::InfluenceTransfers {
            break;
        }
        e.choose(0);
    }
    let pool = e.state.seats[0].pool;
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    assert_eq!(e.state.seats[0].pool, pool - 1);
    assert!(e
        .state
        .minions
        .iter()
        .any(|m| m.present() && m.clan() == Some("Caitiff")));
}

// ---- Library ------------------------------------------------------------------

// Corpse Minion: "Vampire with this retainer may burn X blood to get +X
// intercept for the current action" [TOM 19960109] "May be used any number
// of times during a single action." Against a +2 stealth bleed, its
// vampire burns twice and blocks.
#[test]
fn corpse_minion_buys_intercept_as_often_as_paid() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
    superior(&mut e, a, "obt");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    let minion = in_play(&mut e, 1, "Corpse Minion", Some(z));
    e.state.cards[minion.index()].counters = 1;
    hand(&mut e, 0, &["Shadow Play"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, ability(minion.0, 0));
    pick_when(&mut e, ability(minion.0, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

// Kindred Intelligence: "Move the top card from your crypt to your
// uncontrolled region" [RTR 20000501] [LSJ 20070411-2] "Cannot be played
// when the target crypt is empty."
#[test]
fn kindred_intelligence_moves_the_crypt_top_and_needs_a_crypt() {
    for crypt in [true, false] {
        let mut e = game(2);
        let n = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[0].crypt = if crypt {
            vec![plain("Brujah", 2)].into_iter().collect()
        } else {
            Default::default()
        };
        hand(&mut e, 0, &["Kindred Intelligence"]);
        let req = request(&mut e);
        assert_eq!(req.options.contains(&play(0, 0, n)), crypt);
        if crypt {
            pick(&mut e, play(0, 0, n));
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[0].uncontrolled.len(), 1);
        }
    }
}

// Night Moves: "Bleed your predator or prey at +3 stealth. If more than 1
// pool is bled with this action, ignore the excess." A bleed for 2 burns 1.
#[test]
fn night_moves_burns_one_pool_at_most() {
    let mut e = game(3);
    let n = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, true);
    e.state.minions[n.index()].bleed_bonus_turn = 1;
    hand(&mut e, 0, &["Night Moves"]);
    pick(&mut e, play_at(0, 0, n, Target::Seat(SeatId(1))));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1, "the excess ignored");
}

// Rumble: "Enter combat with a ready minion controlled by another
// Methuselah. In that combat, your hand size is increased by one. If this
// acting vampire is not ready at the end of the action, you burn 1 pool."
#[test]
fn rumble_burns_a_pool_if_its_vampire_is_down() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), sabbat(), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Rumble"]);
    pick(&mut e, play_at(0, 0, r, Target::Minion(z)));
    // No blood left to take the hit: torpor.
    e.state.minions[r.index()].blood = 0;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[r.index()].ready(), "sent to torpor");
    assert_eq!(pool(&e)[0], 10 - 1);
}

// Wrong and Crosswise: "[dem] Reduce a bleed against you by 2."
#[test]
fn wrong_and_crosswise_reduces_a_bleed_by_two() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let d = e.add_minion(SeatId(1), vampire_with("dem", 0), 4, false);
    e.state.minions[a.index()].bleed_bonus_turn = 2;
    hand(&mut e, 1, &["Wrong and Crosswise"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, d));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1, "3 less 2");
}

// Death Seeker: "Cancel a combat card played by the opposing minion as it
// is played, and its cost is not paid." (1 blood.)
#[test]
fn death_seeker_cancels_an_opposing_combat_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Qawiyya el-Ghaduba"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("POT", 0), 4, false);
    hand(&mut e, 0, &["Death Seeker"]);
    hand(&mut e, 1, &["Immortal Grapple"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Immortal Grapple")));
    assert!(e.state.seats[0].ash.contains(&find_library("Death Seeker")));
}

// Righteous Aura: "Only usable if this vampire is ready and the opposing
// minion is not. Put this card on this vampire. This vampire gets +2
// bleed." At the end of the round its opponent went to torpor in.
#[test]
fn righteous_aura_goes_on_its_vampire_over_a_downed_opponent() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Qawiyya el-Ghaduba"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Righteous Aura"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    e.state.minions[z.index()].blood = 0;
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let aura = e
        .state
        .cards
        .iter()
        .position(|c| c.card == find_library("Righteous Aura"))
        .expect("in play");
    assert_eq!(e.state.cards[aura].host, Some(a));
    assert!(!e.state.cards[aura].effect_only);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Righteous Aura")));
    assert_eq!(e.state.bearer_mod(a, Stat::Bleed, ActionKind::Bleed), 2);
}

// Dark Web Procurer: "can search your library (shuffle afterward) for a
// non-unique equipment card costing 1 or fewer pool and equip with it as a
// +1 stealth action": a 2-pool .44 Magnum and a unique Seal of Veddartha
// are not found, a Kevlar Vest is.
// [TOM 19960130] Its special is an equip action.
// [ANK 20180817] [LSJ 20100303] [LSJ 20100725] Requirements apply; the ally may still equip directly (not the same action).
#[test]
fn dark_web_procurer_finds_only_cheap_non_unique_equipment() {
    let mut e = game(2);
    let p = e.add_ally(SeatId(0), find_library("Dark Web Procurer"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].library = vec![
        find_library(".44 Magnum"),
        find_library("Seal of Veddartha"),
        find_library("Kevlar Vest"),
    ]
    .into();
    fill_hand(&mut e, 0);
    let card = e
        .state
        .cards
        .iter()
        .position(|c| c.body == Some(p))
        .expect("its card") as u32;
    pick(&mut e, granted(card, p));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::FindCard(find_library("Kevlar Vest"))]
    );
}

// NRA PAC: "an equip action" read as Hag's Wrinkles' (rules-qa 80.44,
// 78.28): the equip from another minion moves equipment and is not one; a
// granted search equipping from the library (Dark Web Procurer) is.
#[test]
fn nra_pac_unlocks_a_search_equip_but_not_a_move_from_another_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_ally(SeatId(0), find_library("Dark Web Procurer"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "NRA PAC", None);
    let vest = in_play(&mut e, 0, "Kevlar Vest", Some(a));
    e.state.seats[0].library = vec![find_library("Kevlar Vest")].into();
    fill_hand(&mut e, 0);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: b,
            action: ActionKind::EquipFrom,
            target: Some(a),
        },
    );
    decline_futile(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[vest.index()].host, Some(b));
    assert!(!e.state.minions[b.index()].unlock_end_of_turn);
    let card = e
        .state
        .cards
        .iter()
        .position(|c| c.body == Some(p))
        .expect("its card") as u32;
    pick(&mut e, granted(card, p));
    decline_futile(&mut e);
    pick_when(&mut e, Choice::FindCard(find_library("Kevlar Vest")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[p.index()].unlock_end_of_turn);
}

// Smash and Grab: "[ani] Burn 2 life from an ally or retainer. [pot] Burn a
// location controlled by your predator or prey." An ally of 2 life burned;
// a retainer of 3 life left with 1.
// [LSJ 20090324] Directed at the card's controller.
#[test]
fn smash_and_grab_burns_life_from_an_ally_or_a_retainer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ani", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    let ally = e.add_ally(SeatId(1), find_library("Dark Web Procurer"));
    e.state.minions[ally.index()].blood = 2;
    let retainer = in_play(&mut e, 1, "Corpse Minion", Some(z));
    e.state.cards[retainer.index()].counters = 3;
    hand(&mut e, 0, &["Smash and Grab"]);
    pick(&mut e, play_at(0, 0, a, Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    let a2 = e.add_minion(SeatId(0), vampire_with("ani", 1), 4, false);
    e.state.minions[a2.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    hand(&mut e, 0, &["Smash and Grab"]);
    pick(&mut e, play_at(0, 1, a2, Target::Card(retainer)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[retainer.index()].counters, 1);
}

// Ponticulus: "He can lock to give any Tremere +1 intercept … He cannot
// act and he cannot have or use equipment." No action offered him; locked,
// he gives a Tremere attempting to block the intercept it needs.
// [LSJ 20010627] [LSJ 20010629] [LSJ 20090304-1] Immune: the damage is not inflicted.
#[test]
fn ponticulus_cannot_act_but_locks_to_give_a_tremere_intercept() {
    let mut e = game(2);
    let pont = e.add_ally(SeatId(0), find_library("Ponticulus"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| matches!(o,
        Choice::TakeAction { minion, .. } if *minion == pont)));
    // Seat 1 bleeds seat 0 at +1 stealth: seat 0's Tremere blocks with it.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), of_clan("Tremere", 4..=11), 5, false);
    let pont = e.add_ally(SeatId(1), find_library("Ponticulus"));
    let card = e
        .state
        .cards
        .iter()
        .position(|c| c.body == Some(pont))
        .expect("its card") as u32;
    hand(&mut e, 0, &["Lost in Crowds"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(t));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, ability(card, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert!(e.state.minions[pont.index()].locked);
}

// Tremere Convocation: "Tremere get +1 stealth during actions requiring
// Blood Sorcery. Vampires with basic Blood Sorcery can play combat cards
// requiring Blood Sorcery as if they had superior Blood Sorcery."
#[test]
fn tremere_convocation_gives_stealth_and_superior_combat_cards() {
    // A Tremere with basic Blood Sorcery, no text.
    let tremere = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.clan == "Tremere"
                && c.disciplines.contains(&"tha")
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a Tremere with basic tha") as u16;
    let library = |name| &zvtes::library::LIBRARY[find_library(name) as usize];
    let superior = library("Theft of Vitae")
        .modes
        .iter()
        .position(|m| m.tags.contains(&"THA"))
        .expect("a [THA] mode") as u8;
    let mut stealth = Vec::new();
    for conv in [false, true] {
        let mut e = game(2);
        if conv {
            in_play(&mut e, 1, "Tremere Convocation", None);
        }
        let t = e.add_minion(SeatId(0), tremere, 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[0].library = vec![find_library("Kevlar Vest"); 3].into();
        hand(&mut e, 0, &["Magic of the Smith"]);
        pick(&mut e, play(0, 0, t));
        request(&mut e);
        stealth.push(e.action().expect("action").stealth(&e.state));
        // The strike: Theft of Vitae's [THA] mode at basic.
        let mut e = game(2);
        if conv {
            in_play(&mut e, 1, "Tremere Convocation", None);
        }
        let t = e.add_minion(SeatId(0), tremere, 5, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Theft of Vitae"]);
        pick(&mut e, bleed(t));
        pick_when(&mut e, block(z));
        let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
        assert_eq!(req.options.contains(&play(0, superior, t)), conv);
    }
    assert_eq!(stealth[1], stealth[0] + 1, "Tremere +1 stealth");
}

// Malkavian Rider Clause: "Only usable during the polling step of a
// political action before votes and ballots are cast. If the referendum
// passes, the next referendum a vampire you control calls passes
// automatically." No "this turn": it waits for that referendum; none if
// this one fails.
// [LSJ 19980107] No polling-step effect in the automatic pass.
#[test]
fn malkavian_rider_clause_passes_the_next_referendum_if_this_one_passes() {
    for passes in [true, false] {
        let mut e = game(2);
        let caller = e.add_minion(SeatId(0), of_clan("Ventrue", 4..=6), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let malk = e.add_minion(SeatId(1), plain("Malkavian", 0), 4, false);
        hand(&mut e, 0, &["Praxis Seizure: York"]);
        hand(&mut e, 1, &["Malkavian Rider Clause"]);
        pick(&mut e, play(0, 0, caller));
        to_referendum(&mut e);
        pick_when(&mut e, play(0, 0, malk));
        pick_when(&mut e, Choice::CastCardVote { favor: passes });
        close_polling(&mut e);
        request(&mut e);
        assert_eq!(e.state.seats[1].auto_pass_lasting, passes);
    }
}

// Vaticination: "Ⓓ Look at each Methuselah's hand and discard a card of
// your choice from among all those hands (including your own hand)."
// Directed at every other Methuselah (owner, 2026-10-18): the predator may
// block too; the hand chosen, then the card, replaced unseen.
// [PIB 20121031] The replacement card is not seen.
#[test]
fn vaticination_discards_from_any_hand_and_is_directed_at_all() {
    let mut e = game(3);
    let t = e.add_minion(SeatId(0), find_card("Nehemiah"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let pred = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Vaticination"]);
    hand(&mut e, 1, &["Form of Mist"]);
    hand(&mut e, 2, &["Kevlar Vest"]);
    pick(&mut e, play(0, 0, t));
    let req = offering(&mut e, block(pred));
    assert_eq!(req.seat, SeatId(2), "the predator may block");
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, Choice::ChooseSeat(SeatId(2)));
    request(&mut e);
    assert!(e.state.seats[2].ash.contains(&find_library("Kevlar Vest")));
}

// Fast Reaction: "Only usable after a combat between a blocking minion you
// control other than this vampire and the acting minion. [aus] Lock this
// vampire. This vampire enters combat with the acting minion. The first
// round of this new combat, the acting minion cannot strike."
#[test]
fn fast_reaction_sends_a_second_vampire_at_the_acting_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let v = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    hand(&mut e, 1, &["Fast Reaction"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let fast = play(0, 0, v);
    let req = offering(&mut e, fast);
    assert_eq!(req.window, WindowLabel::ActionAfterCombat);
    pick(&mut e, fast);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[v.index()].locked);
    assert_eq!(blood(&e, v), 4, "the acting minion could not strike");
    assert_eq!(blood(&e, a), 4 - 2, "both combats' hands");
}

// Decapitate: "[POT] Only usable when the opposing vampire is going into
// torpor … Burn the opposing vampire instead of sending that vampire to
// torpor." Before torpor lands (owner, 2026-10-18): burned, never in
// torpor; unplayed, it goes to torpor.
#[test]
fn decapitate_burns_a_vampire_instead_of_torpor() {
    for decapitate in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Decapitate"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        e.state.minions[z.index()].blood = 0;
        if decapitate {
            pick_when(&mut e, play(0, 2, a));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[z.index()];
        assert_eq!(v.in_play, !decapitate, "burned: {decapitate}");
        assert_eq!(v.in_torpor, !decapitate);
    }
}

// The going-to-torpor step offers what that moment allows only: Decapitate,
// not The Erciyes Fragments' any-impulse lock (rules-qa 80.43), which the
// prevention window just offered.
#[test]
fn decapitate_step_offers_no_any_impulse_ability() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let frag = in_play(&mut e, 0, "Erciyes Fragments, The", None);
    e.state.seats[1].ash.push(find_library("Kevlar Vest"));
    hand(&mut e, 0, &["Decapitate"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    e.state.minions[z.index()].blood = 0;
    let req = offering(&mut e, play(0, 2, a));
    assert!(
        card_ability(&mut e, frag, 0).is_none(),
        "no any-impulse ability at the step: {:?}",
        req.options
    );
}

// Gran Madre di Dio, Italy: "When your predator or prey puts a minion in
// play in any phase except the unlock phase, that minion is locked. If that
// minion is a younger vampire, he or she burns 1 blood." Younger than its
// bearer (owner, 2026-10-18).
// [RTR 20111202] [LSJ 20020726] A unique equipment while not in play.
#[test]
fn gran_madre_locks_a_minion_its_prey_puts_in_play() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), of_clan("Ventrue", 6..=11), 6, false);
    let gran = in_play(&mut e, 1, "Gran Madre di Dio, Italy", Some(b));
    let _ = gran;
    let young = of_clan("Brujah", 2..=3);
    let cap = CRYPT[young as usize].capacity;
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: young,
            copy: Default::default(),
            counters: cap,
            banished: None,
        });
    loop {
        let req = request(&mut e);
        if req.seat == SeatId(0) && req.window == WindowLabel::InfluenceTransfers {
            break;
        }
        e.choose(0);
    }
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let m = e
        .state
        .minions
        .iter()
        .position(|v| {
            v.present() && matches!(v.kind, zvtes::state::MinionKind::Vampire(c) if c == young)
        })
        .expect("in play");
    let v = &e.state.minions[m];
    assert!(v.locked, "locked as it enters");
    assert_eq!(v.blood, cap - 1, "younger than the bearer: 1 blood");
}

// Gran Madre di Dio on a vampire put in play by Soul Gem of Etrius (rules-qa
// 80.41): the Gem's vampire enters play as any, locked, and burns 1 blood
// younger than the bearer.
#[test]
fn gran_madre_locks_a_soul_gem_vampire() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Hardestadt"), 6, false);
    superior(&mut e, h, "ani");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    in_play(&mut e, 0, "Gran Madre di Dio, Italy", Some(h));
    let zc = plain("Ventrue", 0);
    let z = e.add_minion(SeatId(1), zc, 1, false);
    in_play(&mut e, 1, "Soul Gem of Etrius", Some(z));
    let drawn = of_clan("Ventrue", 1..=CRYPT[zc as usize].capacity - 1);
    e.state.seats[1].crypt = vec![drawn].into();
    hand(&mut e, 0, &["Drawing Out the Beast", "Wolf Claws"]);
    pick(&mut e, bleed(h));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, h));
    pick_when(&mut e, play(0, 0, h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = e
        .state
        .minions
        .iter()
        .find(|v| {
            v.present() && matches!(v.kind, zvtes::state::MinionKind::Vampire(c) if c == drawn)
        })
        .expect("the Gem's vampire");
    assert!(v.locked, "locked as it enters");
    assert_eq!(v.blood, CRYPT[drawn as usize].capacity - 1);
}

// Light Intensifying Goggles: "You can move any aim card the bearer uses to
// this card, face up and out of play (even if it is canceled)." Asked each
// time (owner, 2026-10-18): Target Vitals, used, moves onto them.
#[test]
fn light_intensifying_goggles_keep_an_aim_card_its_bearer_used() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let goggles = in_play(&mut e, 0, "Light Intensifying Goggles", Some(a));
    hand(&mut e, 0, &["Target Vitals"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ResolveRider { card: goggles.0 });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let vitals = find_library("Target Vitals");
    assert!(e.state.cards[goggles.index()].stored.contains(&vitals));
    assert!(!e.state.seats[0].ash.contains(&vitals));
}

// Light Intensifying Goggles: "The bearer can play an aim card on this
// equipment as if from your hand; it is then removed from the game": the
// bearer only.
// [LSJ 20081126] Played from it: no replacement.
#[test]
fn light_intensifying_goggles_bearer_plays_a_stored_aim_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let z2 = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let goggles = in_play(&mut e, 0, "Light Intensifying Goggles", Some(a));
    let vitals = find_library("Target Vitals");
    e.state.cards[goggles.index()].stored = vec![vitals].into();
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(1, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[goggles.index()].stored.is_empty());
    assert!(
        !e.state.seats[0].ash.contains(&vitals),
        "removed from the game"
    );
    // Another of their vampires cannot.
    e.state.cards[goggles.index()].stored = vec![vitals].into();
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(z2));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(1, 0, b));
}

// Light Intensifying Goggles [LSJ 20021128]: "If it leaves play, the card
// on it is burned": its bearer decapitated, the goggles and the stored aim
// card go to the ash heap.
#[test]
fn light_intensifying_goggles_burn_their_stored_card_as_they_leave_play() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let goggles = in_play(&mut e, 1, "Light Intensifying Goggles", Some(z));
    let vitals = find_library("Target Vitals");
    e.state.cards[goggles.index()].stored = vec![vitals].into();
    hand(&mut e, 0, &["Decapitate"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    e.state.minions[z.index()].blood = 0;
    pick_when(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[goggles.index()].in_play);
    assert!(e.state.cards[goggles.index()].stored.is_empty());
    assert!(e.state.seats[1].ash.contains(&vitals));
}

// Nosferatu Bestial: "it becomes a 2-capacity unique Nosferatu of the same
// sect as the acting Nosferatu with basic Animalism [ani]. Nosferatu
// Bestial cannot perform non-hunt actions, cast votes or ballots, or have
// or use equipment."
#[test]
fn nosferatu_bestial_becomes_a_hunting_vampire() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Nosferatu Bestial"]);
    pick(&mut e, play(0, 0, n));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = find_library("Nosferatu Bestial");
    let beast = MinionId(
        e.state
            .minions
            .iter()
            .position(|v| matches!(v.kind, zvtes::state::MinionKind::Created { card: c, .. } if c == card))
            .expect("the Bestial") as u32,
    );
    let st = &e.state;
    assert!(st.minions[beast.index()].present() && st.minions[beast.index()].is_vampire());
    assert_eq!(st.minions[beast.index()].clan(), Some("Nosferatu"));
    assert_eq!(st.capacity_of(beast), Some(2));
    assert_eq!(st.discipline_level(beast, "ani"), 1);
    assert_eq!(st.sect_of(beast), st.sect_of(n));
    // With no blood it hunted at once, this turn (nothing bars it, owner);
    // with blood and unlocked, the hunt is still its only action.
    assert!(e.state.minions[beast.index()].locked, "it hunted");
    e.state.minions[beast.index()].blood = 2;
    e.state.minions[beast.index()].locked = false;
    let req = request(&mut e);
    let kinds: Vec<ActionKind> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::TakeAction { minion, action, .. } if *minion == beast => Some(*action),
            _ => None,
        })
        .collect();
    assert_eq!(kinds, vec![ActionKind::Hunt]);
}

// Nosferatu Bestial: "During your unlock phase, Nosferatu Bestial can
// employ a retainer requiring Animalism from your hand (requirements and
// cost apply as normal; this is not an action and cannot be blocked)."
// [LSJ 20080803] The version by its own Disciplines (basic Animalism).
// [LSJ 20100725] [LSJ 20100303] [ANK 20180817] Requirements and cost apply.
#[test]
fn nosferatu_bestial_employs_an_animal_retainer_in_the_unlock_phase() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Nosferatu Bestial", "Raven Spy"]);
    pick(&mut e, play(0, 0, n));
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let bestial = CardId(0);
    let card = find_library("Nosferatu Bestial");
    let beast = MinionId(
        e.state
            .minions
            .iter()
            .position(|v| matches!(v.kind, zvtes::state::MinionKind::Created { card: c, .. } if c == card))
            .expect("the Bestial") as u32,
    );
    e.state.minions[beast.index()].blood = 2;
    pick(&mut e, use_on(bestial, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let spy = find_library("Raven Spy");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == spy && c.in_play && c.host == Some(beast)));
    assert_eq!(blood(&e, beast), 1, "its cost paid");
}

// Seal of Veddartha: "The first 2 counters each grant this vampire one
// level of Dominate [dom]. The next 2 each grant this vampire one level of
// Fortitude [for]. Remove all counters from this card if another minion
// gains control of it."
#[test]
fn seal_of_veddartha_counters_grant_levels_and_clear_on_a_new_bearer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Toreador", 6..=11), 6, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let seal = in_play(&mut e, 0, "Seal of Veddartha", Some(a));
    let (dom0, for0) = (
        e.state.discipline_level(a, "dom"),
        e.state.discipline_level(a, "for"),
    );
    e.state.cards[seal.index()].counters = 3;
    assert_eq!(e.state.discipline_level(a, "dom"), (dom0 + 2).min(2));
    assert_eq!(e.state.discipline_level(a, "for"), (for0 + 1).min(2));
    e.state.set_host(seal, b);
    assert_eq!(e.state.cards[seal.index()].counters, 0, "another minion");
}

// Vivienne Géroux: "Vampires you control get -1 stealth during political
// actions." Her controller's vampires, political actions only.
#[test]
fn vivienne_geroux_costs_her_vampires_political_stealth() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_ally(SeatId(0), find_library("Vivienne Géroux"));
    let theirs = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let st = &e.state;
    assert_eq!(
        st.bearer_mod_in(a, Stat::Stealth, ActionKind::Political, false),
        -1
    );
    assert_eq!(
        st.bearer_mod_in(a, Stat::Stealth, ActionKind::Bleed, false),
        0
    );
    assert_eq!(
        st.bearer_mod_in(theirs, Stat::Stealth, ActionKind::Political, false),
        0
    );
}

// Seal of Veddartha: "This vampire can bleed at +1 bleed as a Ⓓ action"
// (capacity above 5 only); "during each of your unlock phases, put a
// counter on this card if it has 3 or fewer counters."
// [LSJ 20070320-2] Counters whatever the bearer's Disciplines.
// [LSJ 20030607] [ANK 20180801] A bearer of capacity 5 or less: no bleed, the levels still.
#[test]
fn seal_of_veddartha_bleeds_above_capacity_five_and_gains_counters_to_four() {
    let mut e = game(2);
    let big = e.add_minion(SeatId(0), of_clan("Toreador", 6..=11), 6, false);
    let small = e.add_minion(SeatId(0), of_clan("Toreador", 1..=5), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let seal = in_play(&mut e, 0, "Seal of Veddartha", Some(big));
    assert!(
        card_ability(&mut e, seal, 0).is_some(),
        "capacity 6 or more"
    );
    e.state.set_host(seal, small);
    assert!(
        card_ability(&mut e, seal, 0).is_none(),
        "capacity 5 or less"
    );
    // [LSJ 20030607] [ANK 20180801] Its levels still apply.
    let dom = e.state.discipline_level(small, "dom");
    e.state.cards[seal.index()].counters = 1;
    assert_eq!(e.state.discipline_level(small, "dom"), (dom + 1).min(2));
    e.state.cards[seal.index()].counters = 3;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.cards[seal.index()].counters,
        4,
        "one more, then no more"
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[seal.index()].counters, 4);
}

// Wrong and Crosswise [DEM]: "Only usable during the polling step of a
// political action. This vampire gets +4 votes against the referendum."
// [LSJ 20031119] The votes force its vote against.
#[test]
fn wrong_and_crosswise_superior_casts_four_votes_against() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let d = e.add_minion(SeatId(1), vampire_with("dem", 0), 4, false);
    superior(&mut e, d, "DEM");
    hand(&mut e, 0, &["Kine Resources Contested"]);
    hand(&mut e, 1, &["Wrong and Crosswise"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, d));
    let _ = request(&mut e);
    // Not cast yet: the votes wait, and force its cast against.
    let r = e.referendum().expect("the referendum");
    assert!(r.bonus.contains(&(d, 4)) && r.forced_against.contains(&d));
}

// Vivienne Géroux [ANK 20230417]: "Is not Anarch (eg. does not benefit
// from An Anarch Manifesto)": Anarch only to play her cards.
#[test]
fn vivienne_geroux_gets_nothing_from_an_anarch_manifesto() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_ally(SeatId(0), find_library("Vivienne Géroux"));
    in_play(&mut e, 0, "Anarch Manifesto, An", Some(v));
    assert_eq!(e.state.anarch_action_stealth(v), 0);
}

// Vivienne Géroux: "A vampire you control can give another Methuselah
// control of Vivienne as a Ⓓ action. A vampire controlled by another
// Methuselah can steal Vivienne as a Ⓓ action": not by her controller's.
#[test]
fn vivienne_geroux_is_given_and_stolen_only_across_the_table() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_ally(SeatId(0), find_library("Vivienne Géroux"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let card = CardId(0);
    let req = request(&mut e);
    assert!(
        !req.options
            .contains(&use_on(card, 1, Some(a), Target::None)),
        "her own"
    );
    pick(&mut e, use_on(card, 0, Some(a), Target::Seat(SeatId(1))));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[v.index()].owner, SeatId(1));
    assert_eq!(e.state.card_controller(card), SeatId(1));
    // Back: seat 0 steals her.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, use_on(card, 1, Some(a), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[v.index()].owner, SeatId(0));
}

// Karsh (ADV): "He gets +3 votes in blood hunt referendums": untitled, he
// casts 3, either side.
#[test]
fn karsh_casts_three_votes_in_a_blood_hunt() {
    let mut e = game(2);
    let diablerist = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let victim = e.add_minion(SeatId(1), plain("Brujah", 1), 2, true);
    let adv = CRYPT
        .iter()
        .position(|c| c.name == "Karsh" && c.adv)
        .expect("Karsh (ADV)") as u16;
    let k = e.add_minion(SeatId(1), adv, 6, false);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: diablerist,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    pick(&mut e, votes(k, false));
    let r = e.referendum().expect("blood hunt");
    assert_eq!(r.records.last().map(|v| v.n), Some(3));
}

// General Perfidio Díos: "can burn 1 blood to unlock and attempt to block
// an action directed at a location you control." Locked, he blocks a
// Conceal [OBF] at his Methuselah's location.
#[test]
fn general_perfidio_dios_unlocks_to_guard_a_location() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("OBF", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("General Perfidio Díos"), 5, false);
    e.state.minions[g.index()].locked = true;
    let loc = in_play(&mut e, 1, "Institution Hunting Ground", None);
    hand(&mut e, 0, &["Conceal"]);
    pick(&mut e, play_at(0, 1, a, Target::Card(loc)));
    pick_when(&mut e, own(g, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert!(e.state.cards[loc.index()].present(), "the location saved");
}

// Nefertiti (ADV): "can search your crypt for a card named Aabbt Kindred
// (shuffle afterward), reveal it and move it to your uncontrolled region
// as a +1 stealth action": none in the pool, the search finds nothing, a
// legal action — not with an empty crypt (rules-qa 78.20).
#[test]
fn nefertiti_searches_her_crypt_but_not_an_empty_one() {
    for crypt in [true, false] {
        let mut e = game(2);
        let adv = CRYPT
            .iter()
            .position(|c| c.name == "Nefertiti" && c.adv)
            .expect("Nefertiti (ADV)") as u16;
        let n = e.add_minion(SeatId(0), adv, 8, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if crypt {
            e.state.seats[0].crypt = vec![plain("Brujah", 2)].into_iter().collect();
        }
        let req = request(&mut e);
        let search = req.options.iter().find(|o| {
            matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Vampire(m), .. } if *m == n)
        });
        assert_eq!(search.is_some(), crypt);
        if let Some(&search) = search {
            pick(&mut e, search);
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            assert_eq!(e.state.seats[0].crypt.len(), 1, "nothing found");
            assert!(e.state.seats[0].uncontrolled.is_empty());
        }
    }
}

/// An ability of the card in play `source`, with its target.
fn use_on(source: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(source),
        ability,
        actor,
        target,
    }
}

/// The card ability `ability` of `source` among the options, if offered.
fn card_ability(e: &mut Engine, source: CardId, ability: u8) -> Option<Choice> {
    request(e).options.into_iter().find(|o| {
        matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), ability: a, .. }
            if *c == source && *a == ability)
    })
}

// Institution Hunting Ground: "During your unlock phase, a ready vampire you
// control can gain 1 blood" (the hunting-ground family, rules-qa 6.5).
#[test]
fn institution_hunting_ground_feeds_a_ready_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let hg = in_play(&mut e, 0, "Institution Hunting Ground", None);
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick(&mut e, use_on(hg, 0, None, Target::Minion(a)));
    assert_eq!(blood(&e, a), 5);
}

// Nosferatu Kingdom: "During your influence phase, lock to move 1 blood
// from the blood bank to a Nosferatu antitribu in your uncontrolled
// region": not another clan's.
#[test]
fn nosferatu_kingdom_feeds_an_uncontrolled_nosferatu_antitribu() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let slot = |card| zvtes::state::UncontrolledSlot {
        card,
        copy: Default::default(),
        counters: 0,
        banished: None,
    };
    e.state.seats[0].uncontrolled = vec![
        slot(of_clan("Nosferatu antitribu", 1..=11)),
        slot(of_clan("Nosferatu", 1..=11)),
    ];
    let k = in_play(&mut e, 0, "Nosferatu Kingdom", None);
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let add = |i| use_on(k, 0, None, Target::Uncontrolled(i));
    assert!(!req.options.contains(&add(1)), "not a Nosferatu antitribu");
    pick(&mut e, add(0));
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 1);
    assert!(e.state.cards[k.index()].locked);
}

// Fee Stake: Perth: the Baron of Perth, no clan's calling vote; its burn
// referendum makes non-Anarch titles worth 1 vote less.
#[test]
fn fee_stake_perth_titles_an_anarch_and_its_burn_weakens_other_titles() {
    use zvtes::crypt::{Sect, Title};
    let pick_crypt = |f: &dyn Fn(&zvtes::crypt::CryptCard) -> bool| {
        CRYPT
            .iter()
            .enumerate()
            .position(|(i, c)| f(c) && zvtes::cards::vampire(i as u16).is_none())
            .expect("a crypt card") as u16
    };
    let mut e = game(3);
    let t = e.add_minion(
        SeatId(0),
        pick_crypt(&|c| c.sect == Sect::Anarch && c.capacity >= 5 && c.title == Title::None),
        4,
        false,
    );
    let p = e.add_minion(
        SeatId(1),
        pick_crypt(&|c| matches!(c.title, Title::Prince(_))),
        4,
        false,
    );
    e.add_minion(SeatId(2), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Fee Stake: Perth"]);
    pick(&mut e, play(0, 0, t));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.title_of(t), Title::Baron("Perth"));
    assert_eq!(e.state.calling_votes(t), 0, "no clan's vote");
    let fee = CardId(e.state.cards.len() as u32 - 1);
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, use_on(fee, 0, Some(p), Target::None));
    to_referendum(&mut e);
    let r = e.referendum().expect("a referendum");
    assert_eq!(r.votes_of(&e.state, p), 1, "a prince, -1");
    assert_eq!(r.votes_of(&e.state, t), 2, "a baron: an Anarch title");
}

// IR Goggles: "This minion gets an optional maneuver each combat."
#[test]
fn ir_goggles_give_a_maneuver_each_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    let g = in_play(&mut e, 0, "IR Goggles", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(card_ability(&mut e, g, 0).is_some(), "the maneuver");
}

// Patagia: put on the acting Nosferatu; "once each combat, this vampire may
// burn 1 blood to gain flight for the remainder of the round" (a legal
// no-op, nothing reads flight yet).
// [ANK 20221021] Only in its own vampire's combat.
#[test]
fn patagia_goes_on_its_nosferatu_and_buys_flight_once_a_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 5, false);
    e.add_minion(SeatId(0), plain("Nosferatu", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    hand(&mut e, 0, &["Patagia: Flaps Allowing Limited Flight"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let pat = CardId(e.state.cards.len() as u32 - 1);
    let c = &e.state.cards[pat.index()];
    assert!(c.in_play && !c.effect_only && c.host == Some(a));
    let mut e2 = game(2);
    let a = e2.add_minion(SeatId(0), plain("Nosferatu", 0), 5, false);
    e2.add_minion(SeatId(0), plain("Nosferatu", 1), 4, false);
    let z2 = e2.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    let _ = z;
    let pat = in_play(
        &mut e2,
        0,
        "Patagia: Flaps Allowing Limited Flight",
        Some(a),
    );
    pick(&mut e2, bleed(a));
    pick_when(&mut e2, block(z2));
    until(&mut e2, SeatId(0), WindowLabel::CombatBeforeRange);
    let fly = card_ability(&mut e2, pat, 1).expect("flight offered");
    let before = blood(&e2, a);
    pick(&mut e2, fly);
    assert_eq!(blood(&e2, a), before - 1);
    assert!(card_ability(&mut e2, pat, 1).is_none(), "once a combat");
}

// Patagia's flight lasts "for the remainder of the round": a combat Psyche!
// queues starts without it.
// [LSJ 20021213] Psyche!'s combat is a new combat.
#[test]
fn patagia_flight_ends_with_the_combat_psyche_replaces() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    let pat = in_play(&mut e, 0, "Patagia: Flaps Allowing Limited Flight", Some(a));
    hand(&mut e, 0, &["Psyche!"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    let fly = card_ability(&mut e, pat, 1).expect("flight offered");
    pick(&mut e, fly);
    let flying = |e: &Engine| {
        e.state.minions[a.index()]
            .timed
            .iter()
            .any(|&(l, ..)| l == zvtes::cards::Lasting::Superior("flight"))
    };
    assert!(flying(&e));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    assert_eq!(e.combat().expect("the queued combat").round, 1);
    assert!(!flying(&e), "the round's flight is gone");
}

// Sense Death: [ani] enters combat with a younger vampire only; [val] with
// any minion.
#[test]
fn sense_death_ani_needs_a_younger_vampire_val_any_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Gangrel", 6..=6), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "ANI");
    superior(&mut e, a, "VAL");
    let young = e.add_minion(SeatId(1), of_clan("Ventrue", 3..=3), 3, false);
    let older = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| c.capacity >= 9 && zvtes::cards::vampire(i as u16).is_none())
        .expect("an older vampire") as u16;
    let old = e.add_minion(SeatId(1), older, 5, false);
    hand(&mut e, 0, &["Sense Death"]);
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&play_at(0, 0, a, Target::Minion(young))));
    assert!(!req.options.contains(&play_at(0, 0, a, Target::Minion(old))));
    assert!(req.options.contains(&play_at(0, 1, a, Target::Minion(old))));
    pick(&mut e, play_at(0, 1, a, Target::Minion(old)));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(
        req.options.contains(&Choice::UsePress),
        "the optional press"
    );
}

// Death Seeker [LSJ 20100206]: "If used to cancel a strike, another strike
// is chosen": Torrent [CEL]'s strike canceled, its player strikes again.
#[test]
fn death_seeker_cancels_a_strike_and_another_is_chosen() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Qawiyya el-Ghaduba"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("CEL", 0), 4, false);
    hand(&mut e, 0, &["Death Seeker"]);
    hand(&mut e, 1, &["Torrent", "Torrent"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, z));
    pick_when(&mut e, play(0, 0, a));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::CombatStrike)
    );
    assert!(req.options.contains(&play(0, 1, z)), "the other Torrent");
}

// Fast Reaction [LSJ 19980109] [RTR 20020501]: the first combat's "the
// action continues as if unblocked" (Torrent [CEL]) is lost to the new
// combat; "only usable after a combat between a blocking minion you
// control … and the acting minion": not after its own combat, so a second
// Fast Reaction is not offered (rules-qa 80.37).
#[test]
fn fast_reaction_loses_the_first_combats_continuation_and_follows_only_the_blocks_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let v = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    let w = e.add_minion(SeatId(1), vampire_with("aus", 1), 4, false);
    hand(&mut e, 0, &["Torrent"]);
    hand(&mut e, 1, &["Fast Reaction", "Fast Reaction"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, v));
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play(0, 0, w)),
        "not after its own combat"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "no continuation: blocked");
}

// ---- Found on the way ---------------------------------------------------------

// Dual Form [PRO] played by the Great Beast (Call the Great Beast's
// clanless vampire, p13's crash at 331ce66): "a non-unique vampire with
// the same clan … as the acting vampire" — clanless as it is.
#[test]
fn dual_form_on_the_great_beast_makes_a_clanless_copy() {
    let mut e = game(2);
    let call = in_play(&mut e, 0, "Call the Great Beast", None);
    let beast = MinionId(e.state.minions.len() as u32);
    e.state.minions.push(zvtes::state::Minion::new(
        SeatId(0),
        zvtes::state::MinionKind::GreatBeast {
            card: find_library("Call the Great Beast"),
            id: call,
        },
        9,
    ));
    superior(&mut e, beast, "PRO");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Dual Form"]);
    pick(&mut e, play_at(0, 1, beast, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let copy = MinionId(e.state.minions.len() as u32 - 1);
    let c = &e.state.minions[copy.index()];
    assert!(c.present() && c.is_vampire() && c.dual_of == Some(beast));
    assert_eq!(c.clan(), None);
}
