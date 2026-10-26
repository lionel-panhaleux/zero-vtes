//! Legacy cards for TWDA decks, round 17 (`ADDED_BATCHES`, slice 16ac):
//! twenty promo sets completed and the TWDA greedy's steps 1-6. Readings in
//! rules-qa Batch 84. Windows with a single option are elided, so the
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

/// An anarch with Celerity, no printed-text definition.
fn anarch_with_celerity() -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.sect == Some(zvtes::crypt::Sect::Anarch)
                && c.disciplines.contains(&"cel")
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("an anarch with Celerity") as u16
}

/// `m` gets `disc` at superior level (the fixture's Great Beast slot).
fn superior(e: &mut Engine, m: MinionId, disc: &'static str) {
    let slots = &mut e.state.minions[m.index()].beast_disciplines;
    let free = slots.iter().position(Option::is_none).expect("a free slot");
    slots[free] = Some(disc);
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

/// A crypt card of `sect` with capacity in `caps`, no printed-text
/// definition.
fn of_sect(sect: zvtes::crypt::Sect, caps: std::ops::RangeInclusive<u8>) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.sect == Some(sect)
                && caps.contains(&c.capacity)
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .unwrap_or_else(|| panic!("no {sect:?} of capacity {caps:?}")) as u16
}

/// The crypt card of `name`, advanced or not, of `group`.
fn crypt_card(name: &str, adv: bool, group: u8) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.adv == adv && c.group == group)
        .unwrap_or_else(|| panic!("no {name} (adv {adv}, G{group})")) as u16
}

fn against(opponent: MinionId) -> PrintedCtx {
    PrintedCtx {
        opponent: Some(opponent),
        ..PrintedCtx::default()
    }
}

fn acting(kind: ActionKind, directed: bool) -> PrintedCtx {
    PrintedCtx {
        kind: Some(kind),
        directed: Some(directed),
        ..PrintedCtx::default()
    }
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

/// A vampire's own action `ability` at `target`.
fn own_at(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(m),
        ability,
        actor: Some(m),
        target,
    }
}

// ---- Vampires -----------------------------------------------------------------

// Kisha Bhimji -1 stealth on non-hunt actions; Reverend Adams -1 on
// political actions; Genina +1 on diablerie and undirected actions, once
// for an undirected diablerie; Hesha (G2) +1 stealth hunting, +1 hunt;
// Mitchell's rescue +1 blood.
#[test]
fn round_seventeen_printed_action_stats() {
    let mut e = game(2);
    let kisha = e.add_minion(SeatId(0), find_card("Kisha Bhimji"), 5, false);
    let adams = e.add_minion(SeatId(0), find_card("Reverend Adams"), 3, false);
    let genina = e.add_minion(SeatId(0), find_card("Genina, The Red Poet"), 8, false);
    let hesha = e.add_minion(SeatId(0), crypt_card("Hesha Ruhadze", false, 2), 6, false);
    let mitchell = e.add_minion(SeatId(0), find_card("Mitchell, The Headhunter"), 2, false);
    let st = &e.state;
    let s = Stat::Stealth;
    assert_eq!(
        st.printed_mod(kisha, s, acting(ActionKind::Bleed, true)),
        -1
    );
    assert_eq!(st.printed_mod(kisha, s, acting(ActionKind::Hunt, false)), 0);
    assert_eq!(
        st.printed_mod(adams, s, acting(ActionKind::Political, false)),
        -1
    );
    assert_eq!(st.printed_mod(adams, s, acting(ActionKind::Bleed, true)), 0);
    assert_eq!(
        st.printed_mod(genina, s, acting(ActionKind::Diablerie, true)),
        1
    );
    assert_eq!(
        st.printed_mod(genina, s, acting(ActionKind::Diablerie, false)),
        1
    );
    assert_eq!(
        st.printed_mod(genina, s, acting(ActionKind::Hunt, false)),
        1
    );
    assert_eq!(
        st.printed_mod(genina, s, acting(ActionKind::Bleed, true)),
        0
    );
    assert_eq!(st.printed_mod(hesha, s, acting(ActionKind::Hunt, false)), 1);
    assert_eq!(st.printed_mod(hesha, Stat::Hunt, PrintedCtx::default()), 1);
    assert_eq!(st.printed_mod(hesha, Stat::Bleed, PrintedCtx::default()), 0);
    assert_eq!(
        st.printed_mod(mitchell, Stat::RescueCostMore, PrintedCtx::default()),
        1
    );
}

// Hesha Ruhadze's two groups: G6's bleed per unique equipment no longer
// binds the G2 card (each definition of its group).
#[test]
fn hesha_ruhadze_groups_keep_their_own_text() {
    let g2 = zvtes::cards::vampire(crypt_card("Hesha Ruhadze", false, 2)).expect("G2");
    let g6 = zvtes::cards::vampire(crypt_card("Hesha Ruhadze", false, 6)).expect("G6");
    assert!(g2.printed.iter().any(|p| p.stat == Stat::Hunt));
    assert!(g6.printed.iter().all(|p| p.stat == Stat::Bleed));
}

// Echo +1 strength against a Red List minion; Lucita -1 intercept for a
// younger Sabbat vampire only.
#[test]
fn round_seventeen_printed_combat_stats() {
    let mut e = game(2);
    let echo = e.add_minion(SeatId(0), find_card("Echo"), 7, false);
    let lucita = e.add_minion(SeatId(0), crypt_card("Lucita", false, 2), 8, false);
    let red = e.add_minion(SeatId(1), find_card("Genina, The Red Poet"), 8, false);
    let plain_one = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let young_sabbat = e.add_minion(
        SeatId(1),
        of_sect(zvtes::crypt::Sect::Sabbat, 2..=5),
        4,
        false,
    );
    let old_sabbat = e.add_minion(
        SeatId(1),
        of_sect(zvtes::crypt::Sect::Sabbat, 8..=11),
        8,
        false,
    );
    let st = &e.state;
    assert_eq!(st.printed_mod(echo, Stat::Strength, against(red)), 1);
    assert_eq!(st.printed_mod(echo, Stat::Strength, against(plain_one)), 0);
    let ia = Stat::InterceptAgainst;
    assert_eq!(st.printed_mod(lucita, ia, against(young_sabbat)), -1);
    assert_eq!(st.printed_mod(lucita, ia, against(old_sabbat)), 0);
    assert_eq!(st.printed_mod(lucita, ia, against(plain_one)), 0);
}

// Porphyry +2 bleed against a Methuselah controlling a location, not one
// contested; Kestrelle Hayes +1 stealth on actions directed at a
// Methuselah with no ready Sabbat vampire.
// [PIB 20150526] [RTR 20080808] Directed at several Methuselahs
// (Vaticination): +1 if any of them controls no ready Sabbat vampire.
#[test]
fn porphyry_and_kestrelle_read_the_target_methuselah() {
    let mut e = game(3);
    let p = e.add_minion(SeatId(0), find_card("Porphyry"), 4, false);
    let k = e.add_minion(SeatId(0), find_card("Kestrelle Hayes"), 6, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    e.add_minion(
        SeatId(2),
        of_sect(zvtes::crypt::Sect::Sabbat, 2..=6),
        4,
        false,
    );
    let at = |s: u8| PrintedCtx {
        kind: Some(ActionKind::Bleed),
        directed: Some(true),
        target: Some(SeatId(s)),
        ..PrintedCtx::default()
    };
    assert_eq!(e.state.printed_mod(p, Stat::Bleed, at(1)), 0);
    let loc = in_play(&mut e, 1, "Institution Hunting Ground", None);
    assert_eq!(e.state.printed_mod(p, Stat::Bleed, at(1)), 2);
    e.state.cards[loc.index()].contested = true;
    assert_eq!(e.state.printed_mod(p, Stat::Bleed, at(1)), 0, "contested");
    assert_eq!(e.state.printed_mod(k, Stat::Stealth, at(1)), 1);
    assert_eq!(e.state.printed_mod(k, Stat::Stealth, at(2)), 0);
    let both = PrintedCtx {
        targets: zvtes::ids::SeatSet::single(SeatId(2)).with(SeatId(1)),
        ..at(2)
    };
    assert_eq!(e.state.printed_mod(k, Stat::Stealth, both), 1);
}

// Nu, The Pillar: "During your minion phase, Nu can burn 1 blood to
// unlock", once; "cannot use cards requiring Celerity".
// [LSJ 20020309] He can unlock during an action (Domain Challenge's
// polling).
#[test]
fn nu_unlocks_once_in_his_minion_phase() {
    let mut e = game(2);
    let nu = e.add_minion(SeatId(0), find_card("Nu, The Pillar"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[nu.index()].locked = true;
    pick(&mut e, own(nu, 0));
    assert!(!e.state.minions[nu.index()].locked);
    assert_eq!(blood(&e, nu), 5);
    e.state.minions[nu.index()].locked = true;
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, own(nu, 0));
}

// Hannibal: "During each Methuselah's minion phase, Hannibal can burn 2
// blood to unlock": in another Methuselah's minion phase too.
#[test]
fn hannibal_unlocks_in_another_methuselahs_minion_phase() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let h = e.add_minion(SeatId(1), find_card("Hannibal"), 8, false);
    e.state.minions[h.index()].locked = true;
    pick_when(&mut e, own(h, 1));
    assert!(!e.state.minions[h.index()].locked);
    assert_eq!(blood(&e, h), 6);
}

// Xaviar: "He can prevent 1 damage each combat. Once each combat, before
// range is determined, he may burn 2 blood to get +2 strength for the
// remainder of combat."
#[test]
fn xaviar_buys_strength_before_range() {
    let mut e = game(2);
    let x = e.add_minion(SeatId(0), crypt_card("Xaviar", false, 3), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    pick(&mut e, own(x, 1));
    assert_eq!(blood(&e, x), 6);
    assert_eq!(e.combat().expect("combat").sides[0].strength_mods, 2);
    let req = request(&mut e);
    assert!(!req.options.contains(&own(x, 1)), "once each combat");
}

// Anu Diptinatpa: "During a bleed action, Anu can burn 1 blood to give the
// acting minion +1 bleed": another Methuselah's bleeding minion too.
// [ANK 20220502] Only on an acting bleeding minion.
// [ANK 20231124] [LSJ 19970718] During a bleed action, before resolution.
#[test]
fn anu_diptinatpa_gives_any_bleeding_minion_a_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let anu = e.add_minion(SeatId(1), find_card("Anu Diptinatpa"), 7, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, own(anu, 0));
    assert_eq!(blood(&e, anu), 6);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8, "bled for 2");
}

// Joe "Boot" Hill: "can enter combat with a werewolf as a +1 stealth Ⓓ
// action": a werewolf ally is a target, a vampire is not.
#[test]
fn joe_boot_hill_hunts_werewolves() {
    let mut e = game(2);
    let joe = e.add_minion(SeatId(0), find_card("Joe \"Boot\" Hill"), 7, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let wolf = e.add_ally(SeatId(1), find_library("Renegade Garou"));
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&own_at(joe, 0, Target::Minion(wolf))));
    assert!(!req.options.contains(&own_at(joe, 0, Target::Minion(v))));
    assert_eq!(
        e.state
            .printed_mod(joe, Stat::Strength, PrintedCtx::default()),
        1
    );
}

// Claudio Severino (base): "During your unlock phase, burn 1 pool if there
// are no Gehenna cards in play"; combat cards cost an opposing vampire an
// additional blood.
#[test]
fn claudio_severino_burns_a_pool_without_gehenna() {
    let mut e = game(2);
    e.state.turn = 3;
    e.state.active = SeatId(1);
    e.add_minion(
        SeatId(0),
        crypt_card("Claudio Severino", false, 5),
        9,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.seats[0].pool, 9);
}

// Martino della Passaglia: "Equipment costs Martino 1 fewer blood or pool
// to equip": a .44 Magnum for 1 pool.
#[test]
fn martino_equips_for_one_less() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Martino della Passaglia"), 7, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &[".44 Magnum"]);
    pick(&mut e, play(0, 0, m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 9);
}

// Verbruch: "As Verbruch enters play, you can search your crypt for a
// Tremere ... and move them to your uncontrolled region."
// [LSJ 20080526] Before he enters contest.
#[test]
fn verbruch_searches_the_crypt_as_he_enters() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let tremere = of_clan("Tremere", 2..=11);
    e.state.seats[0].crypt = [plain("Brujah", 2), tremere].into_iter().collect();
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: find_card("Verbruch"),
            copy: Default::default(),
            counters: 5,
            banished: None,
        });
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    pick(&mut e, Choice::AshVampire(tremere));
    assert!(e.state.seats[0]
        .uncontrolled
        .iter()
        .any(|u| u.card == tremere));
    assert_eq!(e.state.seats[0].crypt.len(), 1);
}

// Yong-Sun: "may burn the top card of your prey's crypt as a Ⓓ action".
#[test]
fn yong_sun_burns_the_preys_crypt_top() {
    let mut e = game(2);
    let y = e.add_minion(SeatId(0), find_card("Yong-Sun, Harmonist"), 9, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let top = plain("Ventrue", 1);
    e.state.seats[1].crypt = [plain("Ventrue", 2), top].into_iter().collect();
    pick(&mut e, own_at(y, 0, Target::Seat(SeatId(1))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].crypt.len(), 1);
    assert_eq!(e.state.seats[1].ash_crypt.len(), 1);
}

// Lambach: "can search your library for a card named Eye of Hazimel and
// equip it at no cost as a +1 stealth action".
// [TOM 19960130] The provided action is an equip action.
#[test]
fn lambach_equips_the_eye_of_hazimel_at_no_cost() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lambach"), 10, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let eye = find_library("Eye of Hazimel");
    fill_hand(&mut e, 0);
    e.state.seats[0].library = std::iter::repeat_n(find_library("Conceal"), 6)
        .chain([eye])
        .collect();
    e.state.seats[0].pool = 1;
    pick(&mut e, own_at(l, 0, Target::None));
    pick_when(&mut e, Choice::FindCard(eye));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 1, "no cost");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == eye && c.in_play && c.host == Some(l)));
}

// Reverend Adams: "Older vampires do not lock for successfully blocking
// Reverend Adams"; a younger one does.
#[test]
fn reverend_adams_blockers_older_stay_unlocked() {
    for (caps, locked) in [(4u8..=11, false), (1u8..=2, true)] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), find_card("Reverend Adams"), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), of_clan("Brujah", caps.clone()), 4, false);
        pick(&mut e, bleed(r));
        pick_when(&mut e, block(z));
        request(&mut e);
        assert_eq!(
            e.state.minions[z.index()].locked,
            locked,
            "capacities {caps:?}"
        );
    }
}

// Mitchell: "Rescuing Mitchell from torpor costs +1 blood": 3 blood.
#[test]
fn rescuing_mitchell_costs_three_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let m = e.add_minion(SeatId(0), find_card("Mitchell, The Headhunter"), 0, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Rescue,
            target: Some(m),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[m.index()].in_torpor);
    assert_eq!(blood(&e, a), 1);
}

// Genina: "If a blood hunt is successfully called on Genina, she goes to
// torpor instead of being burned."
#[test]
fn genina_goes_to_torpor_for_a_blood_hunt() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Genina, The Red Poet"), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let victim = e.add_minion(SeatId(1), plain("Brujah", 1), 2, true);
    e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    // A vote to pass the blood hunt.
    e.state.seats[1].has_edge = true;
    pick(
        &mut e,
        Choice::TakeAction {
            minion: g,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    to_referendum(&mut e);
    loop {
        let req = request(&mut e);
        if req.window != WindowLabel::BloodHuntPolling {
            break;
        }
        let i = req
            .options
            .iter()
            .position(|c| matches!(c, Choice::CastEdge { favor: true }))
            .unwrap_or(0);
        e.choose(i);
    }
    let v = &e.state.minions[g.index()];
    assert!(v.in_play && v.in_torpor, "to torpor, not burned");
}

// Guillaume Giovanni: "While Guillaume is ready, you get +1 hand size for
// each location you control."
// [LSJ 20080107] A unique location played for contest does not count.
#[test]
fn guillaume_counts_his_locations_in_hand_size() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Guillaume Giovanni"), 9, false);
    let base = e.state.hand_size(SeatId(0));
    in_play(&mut e, 0, "Institution Hunting Ground", None);
    let c = in_play(&mut e, 0, "Palatial Estate", Some(g));
    assert_eq!(e.state.hand_size(SeatId(0)), base + 2);
    e.state.cards[c.index()].contested = true;
    assert_eq!(e.state.hand_size(SeatId(0)), base + 1, "contested");
    e.state.minions[g.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), base);
}

// Jessica (ADV): "+1 stealth on political actions"; merged with her base
// card, Archbishop of Brussels.
#[test]
fn jessica_advanced_stealth_on_political_actions() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), crypt_card("Jessica", true, 2), 8, false);
    let p = acting(ActionKind::Political, false);
    assert_eq!(e.state.printed_mod(j, Stat::Stealth, p), 1);
}

// White Lily: "Once each combat, before range is determined, White Lily
// can equip with a weapon from your hand (requirements and cost apply as
// normal)": the .44 Magnum named and paid; not offered with no weapon in
// hand.
// [ANK 20221021] Only in her own combat.
// [LSJ 20100130] [ANK 20180925-2] [LSJ 19991207] Named in hand as used.
#[test]
fn white_lily_equips_a_weapon_from_hand_before_range() {
    let mut e = game(2);
    let w = e.add_minion(SeatId(0), find_card("White Lily"), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &[".44 Magnum", "Conceal"]);
    pick(&mut e, bleed(w));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    // The one weapon in hand: named at once (elided).
    pick(&mut e, own(w, 0));
    let req = request(&mut e);
    let magnum = find_library(".44 Magnum");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == magnum && c.in_play && c.host == Some(w)));
    assert_eq!(e.state.seats[0].pool, 8, "its cost paid");
    assert!(!req.options.contains(&own(w, 0)), "once each combat");
}

#[test]
fn fatima_needs_a_weapon_in_hand() {
    let mut e = game(2);
    let f = e.add_minion(SeatId(0), find_card("Fatima al-Faqadi"), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Conceal"]);
    pick(&mut e, bleed(f));
    pick_when(&mut e, block(z));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, own(f, 0));
}

// Tori Longwood: "After your minion phase ends, lock Tori if she did not
// perform an action this turn": locked when she did not act, in torpor
// too; not after an action of hers.
// [LSJ 20011022] [LSJ 20100902-2] Her ability still applies in torpor.
#[test]
fn tori_longwood_locks_when_she_did_not_act() {
    for torpor in [false, true] {
        let mut e = game(2);
        let t = e.add_minion(SeatId(0), find_card("Tori Longwood"), 3, torpor);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(e.state.minions[t.index()].locked, "torpor {torpor}");
    }
    // She hunted: the lock is the action's, nothing more; unlocked after,
    // she stays unlocked.
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tori Longwood"), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, action(t, ActionKind::Hunt));
    request(&mut e);
    e.state.minions[t.index()].locked = false;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].locked);
}

// Sascha Vykos: "Once each turn, after a vampire in combat with Sascha goes
// to torpor, you can get +2 hand size until the end of the turn."
#[test]
fn sascha_vykos_gains_hand_size_as_his_opponent_goes_to_torpor() {
    let mut e = game(2);
    let s = e.add_minion(
        SeatId(0),
        find_card("Sascha Vykos, The Angel of Caine"),
        8,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    let size = e.state.hand_size(SeatId(0));
    pick(&mut e, bleed(s));
    pick_when(&mut e, block(z));
    pick_when(&mut e, Choice::ResolvePrinted { minion: s });
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(e.state.hand_size(SeatId(0)), size + 2);
}

/// Seat 0 influences out `card` (its uncontrolled slot full) in its
/// influence phase.
fn influence_out(e: &mut Engine, card: u16, counters: u8) {
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card,
            copy: Default::default(),
            counters,
            banished: None,
        });
    until(e, SeatId(0), WindowLabel::InfluenceTransfers);
    let slot = e.state.seats[0].uncontrolled.len() as u8 - 1;
    pick(e, Choice::InfluenceOut { slot });
}

// Victor Pelletier: "When Victor enters play, you may pay ... 2 pool to
// make him Prince of Paris"; "if Victor's title would be contested, he
// immediately yields instead of contesting": against a Prince of Paris
// already in play, he yields it at once.
// [LSJ 20080526] [LSJ 20100902] Before he enters contest, not after.
#[test]
fn victor_pelletier_buys_a_title_and_yields_a_contest() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    influence_out(&mut e, find_card("Victor Pelletier"), 5);
    let pool = e.state.seats[0].pool;
    let req = request(&mut e);
    assert!(
        req.options.contains(&Choice::SetX(1)),
        "primogen beside an untitled vampire: {:?}",
        req.options
    );
    pick(&mut e, Choice::SetX(2));
    let v = e
        .state
        .minions_of(SeatId(0))
        .find(|&m| e.state.minions[m.index()].name() == "Victor Pelletier")
        .expect("Victor");
    assert_eq!(e.state.seats[0].pool, pool - 2);
    assert_eq!(e.state.title_of(v), zvtes::crypt::Title::Prince("Paris"));
    // A rival Prince of Paris arrives: Victor yields, no contest.
    let rival = e.add_minion(SeatId(1), plain("Toreador", 0), 4, false);
    e.state.minions[rival.index()].granted_title = Some(zvtes::crypt::Title::Prince("Paris"));
    e.stage_contests(rival);
    assert_eq!(e.state.title_of(v), zvtes::crypt::Title::None);
    assert!(!e.state.minions[rival.index()].title_contested);
}

// Baldesar Rossellini: "Once each turn, if Baldesar is ready, he can reduce
// the cost of a location or of an equipment you play by 1 blood or pool":
// announced with a .44 Magnum another vampire equips; once in the turn.
// [ANK 20180719-4] Once during each player's turn.
#[test]
fn baldesar_takes_one_off_an_equipment_once_each_turn() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Baldesar Rossellini"), 8, false);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &[".44 Magnum", ".44 Magnum"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ResolvePrinted { minion: b });
    pick_when(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 9, "2 pool, 1 off");
    let req = request(&mut e);
    let again = play(0, 0, b);
    assert!(req.options.contains(&again));
    pick(&mut e, again);
    assert!(
        !request(&mut e)
            .options
            .contains(&Choice::ResolvePrinted { minion: b }),
        "once each turn"
    );
}

// Barbaro Lucchese: "can change his clan to Tremere antitribu as +1 stealth
// action"; "put 1 taint counter on a minion as a +1 stealth Ⓓ action; if
// the minion has 2 taint counters, burn them and the minion becomes
// infernal". Both the same turn.
// [LSJ 20100909] The two actions provided are not cross-restrictive.
#[test]
fn barbaro_lucchese_taints_and_changes_clan() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Barbaro Lucchese"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[z.index()].taint = 1;
    pick(&mut e, own_at(b, 1, Target::Minion(z)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert_eq!(v.taint, 0);
    assert!(e.state.is_infernal(z));
    e.state.minions[b.index()].locked = false;
    pick(&mut e, own_at(b, 0, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[b.index()].clan(), Some("Tremere antitribu"));
}

/// A Blood Brother crypt card of `circle`.
fn blood_brother(circle: &str) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.clan == "Blood Brother" && c.circle == Some(circle))
        .unwrap_or_else(|| panic!("no {circle} Blood Brother")) as u16
}

// Angelo: "All Blood Brothers may play cards that require Sanguinus as if
// Angelo were a member of any circle": of another Blood Brother's circle
// for its Sanguinus play, his own circle of one otherwise; "he cannot have
// or use retainers"; "can burn a location as a Ⓓ action".
// [LSJ 20020609] [ANK 20211113] A vampire with no circle is in his own
// circle of one.
// [LSJ 20050315] [LSJ 20010210] He cannot employ a retainer.
// [LSJ 20101013] [ANK 20190228] [ANK 20200705-2] [LSJ 20070708] [RTR 20070707]
// Treated as of the circle for all the card's effects: the one comparison
// every circle effect reads.
#[test]
fn angelo_joins_any_circle_for_sanguinus_and_takes_no_retainers() {
    let mut e = game(2);
    let angelo = e.add_minion(SeatId(0), find_card("Angelo"), 7, false);
    let bb = e.add_minion(SeatId(0), blood_brother("Kiev"), 4, false);
    let other = e.add_minion(SeatId(0), blood_brother("Chicago"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let st = &e.state;
    assert!(!st.same_circle(bb, angelo), "his own circle of one");
    assert!(st.same_circle_for_play(bb, angelo));
    assert!(st.same_circle_for_play(other, angelo));
    assert!(!st.same_circle_for_play(angelo, bb), "not as the player");
    assert!(!st.same_circle_for_play(bb, other));
    hand(&mut e, 0, &["Mr. Winthrop"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, angelo)), "no employ");
    assert!(req.options.contains(&play(0, 0, bb)), "{:?}", req.options);
    let loc = in_play(&mut e, 1, "Institution Hunting Ground", None);
    let req = request(&mut e);
    assert!(req.options.contains(&own_at(angelo, 0, Target::Card(loc))));
}

// ---- Library ------------------------------------------------------------------

fn use_on(source: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(source),
        ability,
        actor,
        target,
    }
}

/// Pick, once offered, ability `ability` of card `source`, whoever uses it.
fn pick_use_of(e: &mut Engine, source: CardId, ability: u8) {
    loop {
        let req = request(e);
        if let Some(i) = req.options.iter().position(|o| {
            matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), ability: a, .. }
                if *c == source && *a == ability)
        }) {
            e.choose(i);
            return;
        }
        let i = fallback(&req);
        e.choose(i);
    }
}

/// The first ability use offered now, if any.
fn any_use(e: &mut Engine) -> Option<Choice> {
    request(e)
        .options
        .into_iter()
        .find(|o| matches!(o, Choice::UseAbility { .. }))
}

// Command of the Beast: "[dom] +1 bleed (limited). [DOM] +1 bleed that does
// not count against the limit." With Conditioning's limited +2 after it:
// 4.
// [LSJ 20100218] [RTR 20180511-2] A limited bleed modifier before or after.
#[test]
fn command_of_the_beast_stacks_with_a_limited_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Command of the Beast", "Conditioning"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 6);
}

// Crematorium: "Lock during your unlock phase to burn a vampire in torpor
// with no blood": any Methuselah's; not one with blood.
#[test]
fn crematorium_burns_an_empty_torpid_vampire() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let empty = e.add_minion(SeatId(1), plain("Ventrue", 1), 0, true);
    let full = e.add_minion(SeatId(1), plain("Ventrue", 2), 1, true);
    let c = in_play(&mut e, 0, "Crematorium", None);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(!req
        .options
        .contains(&use_on(c, 0, None, Target::Minion(full))));
    pick(&mut e, use_on(c, 0, None, Target::Minion(empty)));
    assert!(!e.state.minions[empty.index()].in_play);
    assert!(e.state.cards[c.index()].locked);
}

// Dis Pater: "Once each action, if a Giovanni you control is about to
// successfully bleed your prey, you can remove seven cards in your ash
// heap from the game to give that action +1 bleed."
// [LSJ 20050422] Only "would be successful" effects can follow.
#[test]
fn dis_pater_adds_a_bleed_for_seven_ash_cards() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), of_clan("Giovanni", 4..=11), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].ash = std::iter::repeat_n(find_library("Conceal"), 7).collect();
    let d = in_play(&mut e, 0, "Dis Pater", None);
    pick(&mut e, bleed(g));
    pick_when(&mut e, use_on(d, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8);
    assert_eq!(e.state.seats[0].ash.len(), 0);
    assert_eq!(e.state.seats[0].removed.len(), 7);
}

// Black Sunrise: "Only usable by a locked vampire. [qui] Only usable during
// an action directed at you ... This vampire unlocks and attempts to
// block."
// [LSJ 20010714-2] Cannot be used by a vampire who cannot attempt to block.
#[test]
fn black_sunrise_unlocks_a_locked_vampire_to_block() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let q = e.add_minion(SeatId(1), vampire_with("QUI", 0), 5, false);
    e.state.minions[q.index()].locked = true;
    hand(&mut e, 1, &["Black Sunrise"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, q));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "blocked");
}

// Giulia Giovanni Abruzzina: "If a Giovanni you control is in combat, you
// can lock Giulia before range is determined to set the range for the
// round to close."
#[test]
fn giulia_locks_to_set_close_range() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), of_clan("Giovanni", 4..=11), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let giulia = e.add_ally(SeatId(0), find_library("Giulia Giovanni Abruzzina"));
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(g));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    let u = any_use(&mut e).expect("Giulia's lock offered");
    pick(&mut e, u);
    assert!(e.state.minions[giulia.index()].locked);
    assert_eq!(
        e.combat().expect("combat").range,
        zvtes::combat::Range::Close
    );
}

// Morgue Hunting Ground: the hunting-ground family (rules-qa 6.5).
#[test]
fn morgue_hunting_ground_feeds_a_ready_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let hg = in_play(&mut e, 0, "Morgue Hunting Ground", None);
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick(&mut e, use_on(hg, 0, None, Target::Minion(a)));
    assert_eq!(blood(&e, a), 5);
}

// Spiritual Intervention [NEC]: "Strike: combat ends"; Stonestrength [VIS]:
// "Prevent 2 damage".
#[test]
fn spiritual_intervention_ends_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("NEC", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Spiritual Intervention"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 6, "no damage taken");
}

// As the Crow: "Only usable after resolution of a successful action ...
// [FLIGHT] Unlock this minion": Alabástrom flies (generated Flight); a
// vampire without it cannot play it.
// [PIB 20150915] [LSJ 19981028] [ANK 20190425] After resolution, still
// during the action.
#[test]
fn as_the_crow_unlocks_a_flying_minion() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Alabástrom"), 4, false);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["As the Crow"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, a)));
    pick(&mut e, bleed(p));
    pick_when(&mut e, play(0, 0, p));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[p.index()].locked);
}

// Shakar: "This Black Hand vampire can enter combat with any younger
// non-Black Hand vampire as a Ⓓ action": a younger non-Black Hand target,
// not an older one nor a Black Hand one.
// [LSJ 20080604] The provided action requires a Black Hand vampire.
#[test]
fn shakar_rushes_younger_non_black_hand_vampires() {
    let mut e = game(2);
    let bh = e.add_minion(SeatId(0), find_card("Reza Fatir, The Dark Angel"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let young = e.add_minion(SeatId(1), of_clan("Ventrue", 2..=4), 3, false);
    let old = e.add_minion(
        SeatId(1),
        of_sect(zvtes::crypt::Sect::Camarilla, 8..=11),
        8,
        false,
    );
    let young_bh = e.add_minion(SeatId(1), find_card("Djuhah, The Bronze Bow"), 5, false);
    let s = in_play(&mut e, 0, "Shakar", Some(bh));
    let req = request(&mut e);
    let rush = |t| use_on(s, 0, Some(bh), Target::Minion(t));
    assert!(req.options.contains(&rush(young)));
    assert!(!req.options.contains(&rush(old)));
    assert!(!req.options.contains(&rush(young_bh)));
}

// Waxen Poetica: "Strike: 2R aggravated damage. Not usable against a
// vampire with Celerity, an ally, or a retainer. Burn after use."
// [LSJ 20001127-2] [LSJ 20010806-1] [LSJ 19981006] Not burned if combat
// ends before it resolves; dodged, it is used and burns (owner: rules-qa
// 84.2).
#[test]
fn waxen_poetica_burns_after_use_and_spares_celerity() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Ventrue", 2..=11), 4, false);
    let w = in_play(&mut e, 0, "Waxen Poetica", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_use_of(&mut e, w, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[w.index()].in_play, "burned after use");
    // Dodged (Behind You! [OBF]): used, burned.
    let obf = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.disciplines.contains(&"OBF")
                && !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("cel"))
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a superior Obfuscate vampire without Celerity") as u16;
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), obf, 8, false);
    let w = in_play(&mut e, 0, "Waxen Poetica", Some(a));
    hand(&mut e, 1, &["Behind You!"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_use_of(&mut e, w, 0);
    pick_matching(
        &mut e,
        |o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == z),
    );
    let before = blood(&e, z);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), before, "dodged");
    assert!(!e.state.cards[w.index()].in_play, "used, burned");
    // Against a vampire with Celerity, not offered.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("cel", 0), 4, false);
    let w = in_play(&mut e, 0, "Waxen Poetica", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    loop {
        let req = request(&mut e);
        if req.seat == SeatId(1) && req.window == WindowLabel::MinionPhase {
            break;
        }
        assert!(!req.options.iter().any(|o| matches!(o,
            Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == w)));
        let i = fallback(&req);
        e.choose(i);
    }
}

// Spontaneous Power: "Put this card on a vampire and choose a Discipline.
// This vampire gains 1 level of the chosen Discipline."
#[test]
fn spontaneous_power_grants_a_chosen_discipline() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let before = e.state.discipline_level(a, "obf");
    hand(&mut e, 0, &["Spontaneous Power"]);
    e.at_master_phase();
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(a),
        },
    );
    pick_when(&mut e, Choice::ChooseDiscipline("obf"));
    request(&mut e);
    assert_eq!(e.state.discipline_level(a, "obf"), before + 1);
    let c = e
        .state
        .cards
        .iter()
        .find(|c| c.host == Some(a))
        .expect("on her");
    assert!(c.in_play && !c.effect_only);
}

// Thin Blood: "Only usable at close range before strikes are chosen ...
// [QUI] The opposing vampire burns 2 blood."
#[test]
fn thin_blood_burns_the_opponents_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("QUI", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Thin Blood"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    request(&mut e);
    assert!(blood(&e, z) <= 2, "2 burned before strikes");
}

// Cold Aura [NEC]: "Set the range for this round to long, with 1 optional
// press, only usable to end combat": the press offered once the opponent
// presses to continue (Apportation), and it ends the combat.
// [TOM 19960521] The optional press can only be used this round.
#[test]
fn cold_aura_sets_long_range_with_an_ending_press() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("NEC", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("tha", 0), 4, false);
    hand(&mut e, 0, &["Cold Aura"]);
    hand(&mut e, 1, &["Apportation"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, z));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::CombatPress)
    );
    assert_eq!(
        e.combat().expect("combat").range,
        zvtes::combat::Range::Long
    );
    pick(&mut e, Choice::UsePress);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 5, "its 1 blood; nothing landed at long range");
}

// Argent Baton: "If the opposing minion is a werewolf, the bearer can make
// a ranged strike to burn 4 life from that werewolf": Renegade Garou (4
// life) burned.
#[test]
fn argent_baton_burns_a_werewolfs_life() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let wolf = e.add_ally(SeatId(1), find_library("Renegade Garou"));
    let life = e.state.minions[wolf.index()].blood;
    let b = in_play(&mut e, 0, "Argent Baton", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(wolf));
    pick_use_of(&mut e, b, 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[wolf.index()];
    assert!(life <= 4);
    assert!(!v.in_play, "burned at 0 life");
}

// Earthshock: "[pot] Strike: strength ranged damage. This strike cannot be
// dodged. Not usable against a minion with flight."
// [LSJ 20060509] Usable at long range to target a retainer even if their
// employer has flight.
#[test]
fn earthshock_is_not_usable_against_flight() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let f = e.add_minion(SeatId(1), find_card("Alabástrom"), 6, false);
    hand(&mut e, 0, &["Earthshock"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(f));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, a));
    // Not against a ground minion: offered, and its strength lands at range.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Earthshock"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(blood(&e, z) < 4);
}

// Centralized Background Check: "Weapons cost an additional pool": a .44
// Magnum for 3 pool, any Methuselah's.
// [LSJ 20040701] [LSJ 20080125] [ANK 20181215] It makes a .44 Magnum too
// costly for Concealed Weapon (3 pool).
#[test]
fn centralized_background_check_raises_weapon_costs() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Centralized Background Check", None);
    let magnum = find_library(".44 Magnum");
    assert_eq!(
        zvtes::cards::pool_cost(&e.state, SeatId(0), magnum, Target::Minion(a)),
        3
    );
    hand(&mut e, 0, &[".44 Magnum"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 7);
}

// Therbold Realty: "Locations cost you 1 less blood or pool": a location
// master card's pool; an equipment that is a location's blood.
// [LSJ 20080107] The reduction applies to equipments that are locations.
#[test]
fn therbold_realty_discounts_locations() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Therbold Realty", None);
    let ground = find_library("Institution Hunting Ground");
    let printed = zvtes::cards::pool_cost(&Engine::bare(2).state, SeatId(0), ground, Target::None);
    assert_eq!(
        zvtes::cards::pool_cost(&e.state, SeatId(0), ground, Target::None),
        printed - 1
    );
    assert_eq!(
        e.state.equip_discount(a, find_library("Palatial Estate")),
        1
    );
    assert_eq!(e.state.equip_discount(a, find_library(".44 Magnum")), 0);
}

// Ensemble: "+X bleed (limited), where X is the number of ready unlocked
// Daughters of Cacophony you control. Lock all ready Daughters of
// Cacophony you control."
#[test]
fn ensemble_bleeds_for_each_unlocked_daughter_and_locks_them() {
    let mut e = game(2);
    let scout = find_card("Scout Youngwood");
    let a = e.add_minion(SeatId(0), scout, 8, false);
    let b = e.add_minion(SeatId(0), scout, 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ensemble"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[b.index()].locked);
    assert!(e.state.seats[1].pool <= 8, "bled for 1 + X");
}

// Dive Bomb: "If this action is blocked, the blocking minion gets 1
// optional press during the first round of the resulting combat."
#[test]
fn dive_bomb_gives_its_blocker_a_press() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Alabástrom"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    // Kite's +1 intercept meets the +1 stealth.
    let kite = e.add_minion(SeatId(1), find_card("Kite"), 7, false);
    hand(&mut e, 0, &["Dive Bomb"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Minion(t),
        },
    );
    pick_when(&mut e, block(kite));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPress);
    assert!(req.options.contains(&Choice::UsePress));
}

// Infamous Insurgent: "Put this card on a Red List Anarch vampire. This
// vampire is infernal ... and gets +2 votes."
// [ANK 20220718] It does not require an Anarch.
#[test]
fn infamous_insurgent_makes_its_bearer_infernal() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Genina, The Red Poet"), 8, false);
    e.state.minions[g.index()].sect_override = Some(zvtes::crypt::Sect::Anarch);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    assert!(!e.state.is_infernal(g));
    in_play(&mut e, 0, "Infamous Insurgent", Some(g));
    assert!(e.state.is_infernal(g));
    assert!(e.state.unlocks_as_infernal(g));
}

// Lapse [TEM]: "This round, the opposing minion cannot strike during the
// initial strike step": the opponent's hand strike never lands.
// [LSJ 20001126] The opponent cannot use the maneuver from a strike.
#[test]
fn lapse_bars_the_opponents_initial_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Nu, The Pillar"), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Qawiyya el-Ghaduba"), 6, false);
    hand(&mut e, 0, &["Lapse"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 6, "2 paid, no damage");
}

// Thoughts Betrayed [DOM]: "Opposing minion cannot play any strike cards
// for the duration of this combat."
// [ANK 20210928-2] Cards that merely provide the ability to strike are
// allowed; strike cards played at Determine Range for their maneuver are
// not.
#[test]
fn thoughts_betrayed_bars_strike_cards_for_the_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("cel", 0), 6, false);
    hand(&mut e, 0, &["Thoughts Betrayed"]);
    hand(&mut e, 1, &["Side Strike"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, z));
}

// Rockheart [vis]: "Prevent half the damage (round up) done by the
// opponent's strike": Porphyry blocking, of Qawiyya's 3, 2 prevented.
// [LSJ 20001114] Cannot be used if there is no damage to prevent.
#[test]
fn rockheart_prevents_half_a_strike() {
    let mut e = game(2);
    let q = e.add_minion(SeatId(0), find_card("Qawiyya el-Ghaduba"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), find_card("Porphyry"), 4, false);
    hand(&mut e, 1, &["Rockheart"]);
    pick(&mut e, bleed(q));
    pick_when(&mut e, block(p));
    pick_when(&mut e, play(0, 1, p));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, p), 3, "1 of 3 taken");
}

// Ruins of Villers Abbey: "The vampire with this location may burn X blood
// before range is determined on the first round of a combat to get X
// optional maneuvers for that combat": 1 blood, 1 maneuver, again.
#[test]
fn ruins_of_villers_abbey_buys_maneuvers_for_the_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let r = in_play(&mut e, 0, "Ruins of Villers Abbey, Belgium", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_use_of(&mut e, r, 0);
    let before = blood(&e, a) + 1;
    pick_use_of(&mut e, r, 0);
    assert_eq!(blood(&e, a), before - 2);
    let c = e.combat().expect("combat");
    assert_eq!(c.sides[0].combat_maneuvers, 2);
}

// Port Authority: "When a Methuselah uses a discard phase action to discard
// a card, he or she doesn't draw to replace that card until his or her
// next unlock phase."
// [ANK 20200629-2] [LSJ 20091208] Replaced before unlocking cards.
#[test]
fn port_authority_delays_the_discard_phase_replacement() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Port Authority", None);
    e.state.seats[0].library = std::iter::repeat_n(find_library("Conceal"), 10).collect();
    fill_hand(&mut e, 0);
    let size = e.state.seats[0].hand.len();
    until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, Choice::Discard(0));
    request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), size - 1, "not replaced yet");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.seats[0].hand.len(),
        size,
        "replaced at the unlock phase"
    );
}

// Prophecies of Gehenna: "During your master phase, you may lock this card
// to look at your prey's hand. If you do so, your prey takes control of
// the Prophecies of Gehenna."
#[test]
fn prophecies_of_gehenna_looks_and_passes_to_the_prey() {
    let mut e = game(3);
    for s in 0..3 {
        e.add_minion(SeatId(s), plain("Brujah", s as usize), 4, false);
    }
    let p = in_play(&mut e, 0, "Prophecies of Gehenna", None);
    e.at_master_phase();
    pick_when(&mut e, use_on(p, 0, None, Target::Seat(SeatId(1))));
    request(&mut e);
    assert!(e.state.peeks.contains(&(SeatId(0), SeatId(1))));
    assert_eq!(e.state.card_controller(p), SeatId(1));
    assert!(e.state.cards[p.index()].locked);
}

// The Path of Bone: "Cards requiring Necromancy cost Giovanni 1 fewer
// blood. Any minion can burn this card as a Ⓓ action that inflicts 1
// unpreventable environmental damage on acting vampires."
#[test]
fn the_path_of_bone_discounts_necromancy_and_bites_its_burner() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), of_clan("Giovanni", 4..=11), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let p = in_play(&mut e, 0, "Path of Bone, The", None);
    let card = find_library("Spiritual Intervention");
    assert_eq!(e.state.printed_cost_off(g, card, 0).blood, 1);
    assert_eq!(
        e.state.printed_cost_off(z, card, 0).blood,
        0,
        "not a Giovanni"
    );
    // Burned by another Methuselah's vampire, who is damaged.
    e.state.active = SeatId(1);
    let req = to_minion_phase(&mut e, SeatId(1));
    let burn = req
        .options
        .iter()
        .find(|o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), actor: Some(m), .. } if *c == p && *m == z))
        .copied()
        .expect("the burn offered");
    let before = blood(&e, z);
    pick(&mut e, burn);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[p.index()].in_play);
    assert_eq!(blood(&e, z), before - 1);
}

// The Path of the Scorched Heart: "While this True Brujah is acting or in
// combat, minions controlled by other Methuselahs cannot play cards
// requiring Presence."
#[test]
fn the_path_of_the_scorched_heart_bars_presence_around_its_vampire() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Nu, The Pillar"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("PRE", 0), 6, false);
    in_play(&mut e, 0, "Path of the Scorched Heart, The", Some(t));
    hand(&mut e, 1, &["Majesty"]);
    pick(&mut e, bleed(t));
    pick_when(&mut e, block(z));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, z));
}

// Retribution: "Choose a minion who successfully bled you for more than 1
// pool since your last turn. The chosen minion takes 3 unpreventable
// damage. A Methuselah can play only 1 Retribution in a game."
// [LSJ 20020429] "Since your last turn": your current turn counts.
#[test]
fn retribution_strikes_back_at_a_bleeder_once_a_game() {
    let mut e = game(2);
    e.state.active = SeatId(1);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    let small = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let turn = e.state.turn;
    e.state.seats[0].previous_turn = turn - 2;
    e.state.minions[b.index()]
        .bleeds
        .push((turn - 1, SeatId(0), 2));
    e.state.minions[small.index()]
        .bleeds
        .push((turn - 1, SeatId(0), 1));
    e.state.active = SeatId(0);
    hand(&mut e, 0, &["Retribution", "Retribution"]);
    e.at_master_phase();
    let at = |m| Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(m),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&at(small)), "bled for 1 only");
    pick(&mut e, at(b));
    request(&mut e);
    assert!(blood(&e, b) <= 2, "3 damage");
    assert!(
        !request(&mut e)
            .options
            .iter()
            .any(|o| matches!(o, Choice::Play { .. })),
        "once a game"
    );
}

// Public Enemy: "Choose a non-titled, non-Red List minion who has bled for
// 3 or more pool since your last turn or is infernal or anarch. The chosen
// minion becomes Red List."
// [LSJ 20020429] "Since your last turn" means between the end of your last
// turn and now: a bleed of 3 the turn before qualifies, one before that
// does not.
#[test]
fn public_enemy_red_lists_an_anarch() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        of_sect(zvtes::crypt::Sect::Camarilla, 4..=11),
        6,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let anarch = e.add_minion(
        SeatId(1),
        of_sect(zvtes::crypt::Sect::Anarch, 2..=6),
        4,
        false,
    );
    let untitled = |nth| {
        CRYPT
            .iter()
            .enumerate()
            .filter(|(i, c)| {
                c.sect == Some(zvtes::crypt::Sect::Camarilla)
                    && c.title == zvtes::crypt::Title::None
                    && zvtes::cards::vampire(*i as u16).is_none()
            })
            .nth(nth)
            .expect("an untitled Camarilla vampire")
            .0 as u16
    };
    let cam = e.add_minion(SeatId(1), untitled(0), 4, false);
    let recent = e.add_minion(SeatId(1), untitled(1), 4, false);
    let turn = e.state.turn;
    e.state.seats[0].previous_turn = turn - 2;
    e.state.minions[cam.index()]
        .bleeds
        .push((turn - 3, SeatId(0), 3));
    e.state.minions[recent.index()]
        .bleeds
        .push((turn - 1, SeatId(0), 3));
    hand(&mut e, 0, &["Public Enemy"]);
    let at = |m| Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(a),
        target: Target::Minion(m),
    };
    let req = request(&mut e);
    assert!(
        !req.options.contains(&at(cam)),
        "bled before your last turn"
    );
    assert!(
        req.options.contains(&at(recent)),
        "bled since your last turn"
    );
    // Ⓓ is indicative (rules-qa 20.1): one's own anarch too.
    let untitled_anarch = CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.sect == Some(zvtes::crypt::Sect::Anarch)
                && c.title == zvtes::crypt::Title::None
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(1)
        .expect("an untitled anarch")
        .0 as u16;
    let own = e.add_minion(SeatId(0), untitled_anarch, 4, false);
    assert!(
        request(&mut e).options.contains(&at(own)),
        "one's own minion"
    );
    pick(&mut e, at(anarch));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.is_red_list(anarch));
}

// Hand Contract: "Put this card on any ready minion and choose a Black
// Hand vampire. That Black Hand vampire may enter combat with the minion
// with this card as a +1 stealth Ⓓ action": another Methuselah's Black
// Hand vampire chosen.
#[test]
fn hand_contract_lets_its_chosen_black_hand_rush() {
    let mut e = game(2);
    let bh = e.add_minion(SeatId(0), find_card("Reza Fatir, The Dark Angel"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Hand Contract"]);
    e.at_master_phase();
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(z),
        },
    );
    // The one Black Hand vampire: chosen at once (elided).
    let req = to_minion_phase(&mut e, SeatId(0));
    let hc = e
        .state
        .cards
        .iter()
        .position(|c| c.host == Some(z) && c.in_play)
        .map(|i| CardId(i as u32))
        .expect("on the minion");
    assert_eq!(e.state.cards[hc.index()].chosen, vec![bh]);
    assert!(req
        .options
        .contains(&use_on(hc, 0, Some(bh), Target::Minion(z))));
}

// Provision of the Silsila: after a combat with the minion holding the
// contract that chose this Assamite, that minion not ready: blood to
// capacity, the contract burned.
// [PIB 20150512] Usable even if the target is burned or the contract
// burned as the target leaves the ready region.
#[test]
fn provision_of_the_silsila_fills_the_contracts_assassin() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Banu Haqim", 5..=8), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    // No blood: the hand strike sends it to torpor.
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let pc = in_play(&mut e, 0, "Priority Contract", Some(z));
    e.state.cards[pc.index()].chosen = vec![a];
    hand(&mut e, 0, &["Provision of the Silsila"]);
    pick(&mut e, use_on(pc, 0, Some(a), Target::Minion(z)));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(Some(blood(&e, a)), e.state.capacity_of(a));
    assert!(!e.state.cards[pc.index()].in_play);
}

// Promise of 1528: "Only usable if this Giovanni is about to enter combat
// with a Camarilla vampire ... End the action. (Do not lock this vampire if
// they are blocking.)": a Giovanni blocking a Camarilla bleeder ends it,
// unlocked, no combat.
// [RTR 20180719] [ANK 20200207] The action ends unsuccessfully immediately.
#[test]
fn promise_of_1528_ends_a_camarilla_vampires_action() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        of_sect(zvtes::crypt::Sect::Camarilla, 4..=8),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let g = e.add_minion(SeatId(1), of_clan("Giovanni", 4..=11), 5, false);
    hand(&mut e, 1, &["Promise of 1528"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(g));
    pick_when(&mut e, play(0, 0, g));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[g.index()].locked,
        "not locked for the block"
    );
    assert_eq!(e.state.seats[1].pool, 10, "no bleed");
    assert_eq!(blood(&e, a), 5, "no combat");
}

// Promise of 1528 before the combat Psyche! [CEL] queues (rules-qa 88.11,
// by LSJ 19991025): the Giovanni "about to enter combat with a Camarilla
// vampire" again ends the action — the locked Giovanni blocker against the
// Camarilla bleeder's Psyche!, or the acting Giovanni against the Camarilla
// blocker after his own Psyche!. No second combat; the action blocked.
#[test]
fn promise_of_1528_ends_the_action_before_psyches_combat() {
    for giovanni_acts in [false, true] {
        let mut e = game(2);
        let (cam, gio) = (
            of_sect(zvtes::crypt::Sect::Camarilla, 4..=8),
            of_clan("Giovanni", 4..=11),
        );
        let (first, second) = if giovanni_acts {
            (gio, cam)
        } else {
            (cam, gio)
        };
        let a = e.add_minion(SeatId(0), first, 6, false);
        superior(&mut e, a, "CEL");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), second, 6, false);
        let g = if giovanni_acts { a } else { z };
        if giovanni_acts {
            hand(&mut e, 0, &["Psyche!", "Promise of 1528"]);
        } else {
            hand(&mut e, 0, &["Psyche!"]);
            hand(&mut e, 1, &["Promise of 1528"]);
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        let req = offering(&mut e, play(0, 1, a));
        e.choose(index_of(&req, play(0, 1, a)));
        let promise = if giovanni_acts {
            play(0, 1, g)
        } else {
            play(0, 2, g)
        };
        let req = offering(&mut e, promise);
        assert_eq!(req.window, WindowLabel::ActionAfterCombat);
        assert!(e.combat().is_none(), "between the combats");
        e.choose(index_of(&req, promise));
        assert!(e.combat().is_none(), "no second combat");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
        assert_eq!(e.state.seats[1].pool, 10, "no bleed");
    }
}

// Increased Strength [POT]: "For the remainder of combat, all damaging
// strikes that require Potence made by this vampire inflict +2 damage": a
// Potence strike card's damage, not the hand strike's.
// [LSJ 20030611] A strike requiring Potence counts.
#[test]
fn increased_strength_raises_potence_strikes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 7, false);
    hand(&mut e, 0, &["Increased Strength", "Earthshock"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let before = blood(&e, z);
    // Earthshock [POT]: strength+1 ranged, +2 more.
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let strength = 1;
    assert_eq!(blood(&e, z), before - (strength + 1 + 2));
}

// Call the Wild Hunt [ANI][PRO]: "burn X animals from your hand to inflict
// X ranged damage on the opposing minion": X = 2, two Dog Packs discarded,
// 2 environmental damage.
// [LSJ 20060215] X chosen, then that many animals discarded.
// [LSJ 20060217] The damage is environmental.
#[test]
fn call_the_wild_hunt_burns_animals_for_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ANI", 0), 8, false);
    superior(&mut e, a, "pro");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 7, false);
    hand(&mut e, 0, &["Call the Wild Hunt", "Dog Pack", "Dog Pack"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let before = blood(&e, z);
    // Two alike Dog Packs: each discard a lone option (elided).
    pick_when(&mut e, Choice::SetX(2));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(blood(&e, z) <= before - 2);
    let dog = find_library("Dog Pack");
    assert_eq!(
        e.state.seats[0].ash.iter().filter(|&&c| c == dog).count(),
        2
    );
}

// Gianna di Canneto: "If another minion you control is in combat, Gianna
// can lock and burn herself to burn an equipment on the opposing minion,
// or lock during normal strike resolution to inflict 1 unpreventable
// damage on the opposing minion."
// [ANK 20180517] They can lock to use their ability the turn recruited.
#[test]
fn gianna_burns_the_opponents_equipment_or_hurts_it() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let gianna = e.add_ally(SeatId(0), find_library("Gianna di Canneto"));
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(z));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let burn = loop {
        let req = request(&mut e);
        if let Some(o) = req.options.iter().find(|o| {
            matches!(o,
            Choice::UseAbility { target: Target::Card(c), .. } if *c == gun)
        }) {
            break *o;
        }
        let i = fallback(&req);
        e.choose(i);
    };
    pick(&mut e, burn);
    request(&mut e);
    assert!(!e.state.cards[gun.index()].in_play, "the gun burned");
    assert!(
        !e.state.minions[gianna.index()].in_play,
        "she burned herself"
    );
}

// Botched Move: "If another copy of the same combat card is revealed, the
// played card is canceled (cost is still paid), the opposing minion takes 1
// unpreventable damage, and all revealed copies are burned."
// [RTR 19970630] Damage is environmental.
#[test]
fn botched_move_cancels_a_card_whose_copy_is_revealed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("cel", 0), 5, false);
    hand(&mut e, 0, &["Botched Move"]);
    hand(&mut e, 1, &["Side Strike"]);
    fill_hand(&mut e, 1);
    let side = find_library("Side Strike");
    e.state.seats[1].library = [find_library("Conceal"), side].into_iter().collect();
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let before = blood(&e, z);
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(
        e.state.seats[1].ash.iter().filter(|&&c| c == side).count(),
        2
    );
    assert!(blood(&e, z) < before, "1 unpreventable damage");
}

// Dam the Heart's River [QUI]: "each strike ... against the opposing
// minion inflicts +1 damage ... strikes cost the opposing minion 1
// additional blood or life, and they can choose not to strike": the
// opponent's hand strike costs it a blood; ours hits for 2.
// [LSJ 20090703] Strikes are paid at declaration.
// [ANK 20220409] The opponent is the one affected by frenzy.
#[test]
fn dam_the_hearts_river_taxes_and_hurts_the_opponent() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("QUI", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    hand(&mut e, 0, &["Dam the Heart's River"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(
        req.options.contains(&Choice::Pass),
        "may choose not to strike"
    );
    let before = blood(&e, z);
    pick(&mut e, Choice::HandStrike);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), before - 1 - 2, "the toll, then 1+1 damage");
}

// Alvusia: "During an action, you can burn 1 counter from this card to give
// a slave Gargoyle you control +1 bleed"; "Add 1 counter on this card after
// a Tremere or Gargoyle enters play during your influence phase".
// [ANK 20231124] [LSJ 19970718] Only during a bleed action before
// resolution.
#[test]
fn alvusia_gives_a_slave_gargoyle_a_bleed() {
    let mut e = game(2);
    let tremere = e.add_minion(SeatId(0), of_clan("Tremere", 2..=11), 4, false);
    let g = e.add_minion(SeatId(0), find_card("Porphyry"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let _ = tremere;
    let al = in_play(&mut e, 0, "Alvusia", None);
    e.state.cards[al.index()].counters = 2;
    pick(&mut e, bleed(g));
    pick_when(&mut e, use_on(al, 0, Some(g), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[al.index()].counters, 1);
    assert_eq!(e.state.seats[1].pool, 8, "bled for 1 + 1");
    // A Tremere influenced out: 1 counter more.
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let al = in_play(&mut e, 0, "Alvusia", None);
    e.state.cards[al.index()].counters = 1;
    let tremere = of_clan("Tremere", 2..=4);
    influence_out(
        &mut e,
        tremere,
        zvtes::crypt::CRYPT[tremere as usize].capacity,
    );
    request(&mut e);
    assert_eq!(e.state.cards[al.index()].counters, 2);
}

// Filchware's Pawn Shop: "Any minion may equip with any non-location
// equipment card in any Methuselah's ash heap as an undirected +1 stealth
// equip action ... costs 1 additional pool if ... his or her controller's
// ash heap. Otherwise, the action costs 1 less pool or blood": a .44
// Magnum from the other ash heap for 1 pool.
#[test]
fn filchwares_pawn_shop_equips_from_an_ash_heap() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let f = in_play(&mut e, 1, "Filchware's Pawn Shop", None);
    let magnum = find_library(".44 Magnum");
    e.state.seats[1].ash.push(magnum);
    pick(&mut e, use_on(f, 0, Some(a), Target::None));
    // One ash heap, one card: both named at once (elided).
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let gun = e
        .state
        .cards
        .iter()
        .position(|c| c.card == magnum && c.in_play && c.host == Some(a))
        .map(|i| CardId(i as u32))
        .expect("equipped");
    assert_eq!(e.state.seats[0].pool, 9, "2 pool, 1 off");
    assert!(e.state.seats[1].ash.is_empty());
    // Still the other Methuselah's card, controlled with its bearer: burned,
    // it returns to their ash heap.
    assert_eq!(e.state.card_controller(gun), SeatId(0));
    e.burn_card(gun);
    assert_eq!(
        e.state.seats[1].ash.iter().copied().collect::<Vec<_>>(),
        vec![magnum]
    );
}

// Guide and Mentor: "search your library for a master: archetype and move 1
// blood from the bank and that card to himself or herself as a +1 stealth
// action that costs X pool where X is the number of copies of that card on
// minions you control": a Perfectionist, one already on another minion of
// ours, for 1 pool.
// [PIB 20110914] Usable even if the vampire already has an archetype.
#[test]
fn guide_and_mentor_searches_an_archetype_onto_its_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let gm = in_play(&mut e, 0, "Guide and Mentor", None);
    in_play(&mut e, 0, "Perfectionist", Some(b));
    let perfectionist = find_library("Perfectionist");
    fill_hand(&mut e, 0);
    e.state.seats[0].library = std::iter::repeat_n(find_library("Conceal"), 5)
        .chain([perfectionist])
        .collect();
    let before = blood(&e, a);
    pick(&mut e, use_on(gm, 0, Some(a), Target::None));
    pick_when(&mut e, Choice::FindCard(perfectionist));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), before + 1);
    assert_eq!(e.state.seats[0].pool, 9, "X = 1");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == perfectionist && c.in_play && c.host == Some(a)));
}

// Legion: "This non-Sterile vampire can put a Master Discipline card from
// your hand or ash heap in play to represent a fledgling with 1 level of
// that Discipline and discard the rest of your hand as a +2 stealth action
// ... The fledgling cannot act or block or play cards if this vampire is
// not ready."
#[test]
fn legion_makes_a_fledgling_from_a_master_discipline() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        of_sect(zvtes::crypt::Sect::Camarilla, 8..=11),
        8,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let l = in_play(&mut e, 0, "Legion", Some(a));
    hand(&mut e, 0, &["Celerity", "Conceal"]);
    e.state.seats[0].library = std::iter::repeat_n(find_library("Conceal"), 10).collect();
    pick(&mut e, use_on(l, 0, Some(a), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let f = e
        .state
        .minions
        .iter()
        .position(|m| m.in_play && m.tethered_to == Some(a))
        .map(|i| MinionId(i as u32))
        .expect("the fledgling");
    assert_eq!(e.state.discipline_level(f, "cel"), 1);
    assert_eq!(e.state.capacity_of(f), Some(1));
    assert_eq!(
        e.state.minions[f.index()].clan(),
        e.state.minions[a.index()].clan()
    );
    assert!(
        e.state.seats[0].ash.contains(&find_library("Conceal")),
        "the rest discarded"
    );
    // Its vampire in torpor: it cannot act.
    e.state.minions[a.index()].in_torpor = true;
    assert!(!e.state.tether_holds(f));
}

// Raw Recruit: "Put this card in play and move a vampire in torpor to this
// card ... A vampire you control to whom this Gargoyle is enslaved can
// remove that vampire from the game as a +1 stealth action to turn this
// card into a Gargoyle with the same capacity ..."
// [LSJ 20100221] The created vampire is not unique.
// [LSJ 20100302-2] Blood, equipments and other cards on the target burn.
#[test]
fn raw_recruit_turns_a_torpid_vampire_into_a_gargoyle() {
    let mut e = game(2);
    let tremere = e.add_minion(SeatId(0), of_clan("Tremere", 4..=8), 6, false);
    let p = e.add_minion(SeatId(0), find_card("Porphyry"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let victim_card = of_clan("Ventrue", 6..=6);
    let victim = e.add_minion(SeatId(1), victim_card, 2, true);
    hand(&mut e, 0, &["Raw Recruit"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(p),
            target: Target::Minion(victim),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let rr = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.held_vampire.is_some())
        .map(|i| CardId(i as u32))
        .expect("the vampire held");
    assert!(!e.state.minions[victim.index()].in_play);
    pick(&mut e, use_on(rr, 0, Some(tremere), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let g = e.state.cards[rr.index()].body.expect("a Gargoyle");
    assert_eq!(e.state.capacity_of(g), Some(6));
    assert_eq!(e.state.minions[g.index()].clan(), Some("Gargoyle"));
    assert_eq!(e.state.slave_clan(g), Some("Tremere"));
    assert_eq!(e.state.discipline_level(g, "flight"), 2);
    assert!(e.state.seats[1].removed_crypt.contains(&victim_card));
    // A slave: as a Tremere of its controller's is blocked, it may fight
    // in its place (advanced rules §5.7.2).
    e.state.minions[g.index()].locked = false;
    e.state.minions[tremere.index()].locked = false;
    let z = e.state.minions_of(SeatId(1)).next().expect("a blocker");
    e.state.minions[z.index()].locked = false;
    pick(&mut e, bleed(tremere));
    pick_when(&mut e, block(z));
    offering(&mut e, own(g, 0));
}

// Leadership Vacuum: "Only usable when a ready titled vampire controlled by
// another Methuselah is burned or sent to torpor ... put X counters on this
// card where X is the number of votes that vampire has. That Methuselah
// burns X pool during each of his or her unlock phases."
// [ANK 20211009] [LSJ 20040518-2] Bonus votes from cards count.
// [ANK 20221024] Changing the counters does not change X.
#[test]
fn leadership_vacuum_taxes_a_methuselah_who_lost_a_titled_vampire() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let prince = e.add_minion(SeatId(1), of_clan("Ventrue", 2..=5), 3, false);
    e.state.minions[prince.index()].granted_title = Some(zvtes::crypt::Title::Prince("Paris"));
    e.add_minion(SeatId(1), of_clan("Ventrue", 2..=4), 3, false);
    let votes = e.state.votes_held(prince);
    hand(&mut e, 0, &["Leadership Vacuum"]);
    e.burn(prince);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Seat(SeatId(1)),
        },
    );
    request(&mut e);
    let lv = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Leadership Vacuum"))
        .map(|i| CardId(i as u32))
        .expect("in play");
    assert_eq!(e.state.cards[lv.index()].fixed_x, votes);
    // The card itself in play, not a stand-in.
    assert!(!e.state.cards[lv.index()].effect_only);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Leadership Vacuum")));
    e.state.cards[lv.index()].counters = 9;
    let pool = e.state.seats[1].pool;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.seats[1].pool,
        pool - votes as i32,
        "X, not the counters"
    );
}

// ---- Ledger rows ----------------------------------------------------------------

/// Pick, once offered, the first option `want` accepts.
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

/// The card in play that is ally `m`.
fn body_card(e: &Engine, m: MinionId) -> CardId {
    CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.body == Some(m))
            .expect("its card") as u32,
    )
}

/// Fallback choices to `seat`'s `window`, asserting `banned` never offered.
fn none_until(e: &mut Engine, seat: SeatId, window: WindowLabel, banned: impl Fn(&Choice) -> bool) {
    for _ in 0..300 {
        let req = request(e);
        if (req.seat, req.window) == (seat, window) {
            return;
        }
        assert!(!req.options.iter().any(&banned), "{req:?}");
        let i = fallback(&req);
        e.choose(i);
    }
    panic!("{window:?} never reached");
}

// [ANK 20210627] Promise of 1528 after a Banshee Ironwail block: "the blood
// is burned before any other effect can be used": the acting vampire has
// burned its blood as the Promise is offered.
#[test]
fn promise_of_1528_comes_after_the_banshee_ironwails_burn() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        of_sect(zvtes::crypt::Sect::Camarilla, 4..=8),
        5,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let g = e.add_minion(SeatId(1), of_clan("Giovanni", 4..=11), 5, false);
    in_play(&mut e, 1, "Banshee Ironwail", Some(g));
    hand(&mut e, 1, &["Promise of 1528"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(g));
    offering(&mut e, play(0, 0, g));
    assert_eq!(blood(&e, a), 4, "burned as the block succeeds");
    pick(&mut e, play(0, 0, g));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4, "no combat");
}

// [LSJ 20070214] Botched Move "Can be played against any combat card, even
// those played at the end of the round": against Apportation's press.
#[test]
fn botched_move_answers_a_press_at_the_end_of_the_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("tha", 0), 5, false);
    hand(&mut e, 0, &["Botched Move"]);
    hand(&mut e, 1, &["Apportation"]);
    fill_hand(&mut e, 1);
    let app = find_library("Apportation");
    e.state.seats[1].library = [find_library("Conceal"), app].into_iter().collect();
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(
        e.state.seats[1].ash.iter().filter(|&&c| c == app).count(),
        2
    );
}

// [LSJ 20081120-2] Botched Move "Is not affected by a Dark Influences in
// play": the copy revealed still cancels, Dark Influences stays.
#[test]
fn botched_move_is_not_caught_by_dark_influences() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("cel", 0), 5, false);
    let di = in_play(&mut e, 1, "Dark Influences", None);
    hand(&mut e, 0, &["Botched Move"]);
    hand(&mut e, 1, &["Side Strike"]);
    fill_hand(&mut e, 1);
    let side = find_library("Side Strike");
    e.state.seats[1].library = [find_library("Conceal"), side].into_iter().collect();
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(
        e.state.seats[1].ash.iter().filter(|&&c| c == side).count(),
        2
    );
    assert!(e.state.cards[di.index()].in_play);
}

// [LSJ 20040210] Call the Wild Hunt "Does not render immune to a
// previously-played Frenzy card": the opponent's Dam the Heart's River,
// played first, still adds its damage.
#[test]
fn call_the_wild_hunt_keeps_an_earlier_frenzy_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ani", 0), 6, false);
    superior(&mut e, a, "pro");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("QUI", 0), 6, false);
    hand(&mut e, 0, &["Call the Wild Hunt"]);
    hand(&mut e, 1, &["Dam the Heart's River"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 0, a));
    let before = blood(&e, a);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), before - 2, "the hand strike, +1");
}

// Dam the Heart's River [ANK 20181128] applies to environmental damage
// inflicted to the opposing minion, [ANK 20170111] [LSJ 20071011] to damage
// played in combat resolved after combat, [ANK 20170111] [PIB 20130319]
// damage after combat ends included: Thoughts Betrayed's environmental 1 is
// 2; Catatonic Fear's after-combat 1 is 2.
#[test]
fn dam_the_hearts_river_adds_to_environmental_and_after_combat_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("dom", 0), 8, false);
    superior(&mut e, a, "qui");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    hand(&mut e, 0, &["Dam the Heart's River", "Thoughts Betrayed"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    let before = blood(&e, z);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, z),
        before - 2 - 2,
        "the hand strike 1+1, the environmental 1+1"
    );

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("QUI", 0), 8, false);
    superior(&mut e, a, "pre");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    hand(&mut e, 0, &["Dam the Heart's River", "Catatonic Fear"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    let before = blood(&e, z);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), before - 2, "after combat, 1+1");
}

// [LSJ 20071017] Dam the Heart's River [QUI] after a Sniper Rifle set the
// range: the bearer "may choose not to strike if another card gives the
// option not to strike".
#[test]
fn dam_lets_a_sniper_rifles_bearer_choose_not_to_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("QUI", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    let rifle = in_play(&mut e, 1, "Sniper Rifle", Some(z));
    hand(&mut e, 0, &["Dam the Heart's River"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_on(rifle, 1, None, Target::None));
    pick_when(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(req.options.contains(&Choice::Pass), "{req:?}");
}

// [PIB 20141026] Gianna di Canneto burning an equipment its bearer
// committed a strike to "(eg. by using the maneuver of a .44 Magnum) ...
// the opponent strike has no effect."
#[test]
fn gianna_burns_a_committed_gun_and_its_strike_fails() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_ally(SeatId(0), find_library("Gianna di Canneto"));
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(z));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, use_on(gun, 1, None, Target::None));
    pick_matching(
        &mut e,
        |o| matches!(o, Choice::UseAbility { target: Target::Card(c), .. } if *c == gun),
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[gun.index()].in_play);
    assert_eq!(blood(&e, a), 5, "no strike landed");
}

// [ANK 20211130] Gianna "cannot use her ability to inflict damage if a
// 'Strike: combat ends' was used": Staredown ends the combat first.
#[test]
fn gianna_inflicts_nothing_after_a_combat_ending_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRE", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let gianna = e.add_ally(SeatId(0), find_library("Gianna di Canneto"));
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    let g = body_card(&e, gianna);
    hand(&mut e, 0, &["Staredown"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    none_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), ability: 1, .. } if *c == g),
    );
    assert_eq!(blood(&e, z), 6);
}

// [LSJ 20050224-2] Groundfighting "Can cancel superior Thoughts Betrayed":
// the anarch blocker cancels it as it is played, then strikes with a card.
#[test]
fn groundfighting_cancels_superior_thoughts_betrayed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let anarch = anarch_with_celerity();
    let z = e.add_minion(SeatId(1), anarch, 6, false);
    hand(&mut e, 0, &["Thoughts Betrayed"]);
    hand(&mut e, 1, &["Groundfighting", "Side Strike"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, z));
    request(&mut e);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Thoughts Betrayed")));
    offering(&mut e, play(0, 0, z));
}

// A canceled card whose cost is still paid takes the reductions announced
// with it as it is paid (rules-qa 101.1): superior Thoughts Betrayed (2
// blood), announced with Focus the Blood and canceled by Groundfighting,
// costs 1 and burns the Focus.
#[test]
fn a_canceled_card_still_paid_takes_its_announced_reduction() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Banu Haqim", 6..=11), 3, false);
    superior(&mut e, a, "DOM");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let anarch = anarch_with_celerity();
    let z = e.add_minion(SeatId(1), anarch, 6, false);
    hand(&mut e, 0, &["Focus the Blood", "Thoughts Betrayed"]);
    hand(&mut e, 1, &["Groundfighting", "Side Strike"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    let focus = e
        .state
        .cards
        .iter()
        .position(|c| c.card == find_library("Focus the Blood") && c.in_play)
        .map(|i| zvtes::state::CardId(i as u32))
        .expect("Focus the Blood in play");
    assert_eq!(blood(&e, a), 4);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::ResolveRider { card: focus.0 });
    pick_when(&mut e, play(0, 1, z));
    request(&mut e);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Thoughts Betrayed")));
    assert_eq!(blood(&e, a), 3, "2 blood, 1 off");
    assert!(!e.state.cards[focus.index()].in_play, "the Focus burned");
}

// Not The Shard: announced with superior Thoughts Betrayed and the card
// canceled by Groundfighting, its cost paid in full and the Shard left
// unlocked — never locked by a card canceled as played (rules-qa 101.1,
// 36.1; ANK 20230620).
#[test]
fn a_canceled_card_still_paid_locks_no_shard() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Banu Haqim", 6..=11), 4, false);
    superior(&mut e, a, "DOM");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let anarch = anarch_with_celerity();
    let z = e.add_minion(SeatId(1), anarch, 6, false);
    let shard = in_play(&mut e, 0, "Shard, London, The", None);
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Thoughts Betrayed"]);
    hand(&mut e, 1, &["Groundfighting", "Side Strike"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::ResolveRider { card: shard.0 });
    pick_when(&mut e, play(0, 1, z));
    request(&mut e);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Thoughts Betrayed")));
    assert_eq!(blood(&e, a), 2, "2 blood, the Shard not taken");
    assert!(!e.state.cards[shard.index()].locked);
}

// [LSJ 20081120-2] Heroic Might's "strikes it provides are Potence
// strikes": Increased Strength adds 1 to its superior 2R strike.
#[test]
fn increased_strength_raises_heroic_mights_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    e.state.cards.push(CardInPlay::new(
        find_library("Heroic Might"),
        SeatId(0),
        Some(a),
        1,
    ));
    let hm = CardId(e.state.cards.len() as u32 - 1);
    hand(&mut e, 0, &["Increased Strength"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    let before = blood(&e, z);
    pick_use_of(&mut e, hm, 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), before - 3, "2R, +1");
}

// [ANK 20230314] Provision of the Silsila "Is used after combat effectively
// ends and can be ordered before or after other cards and effects played
// after combat" (Monster, Shadow Boxing): all three offered at once.
#[test]
fn provision_of_the_silsila_orders_with_other_after_combat_effects() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Banu Haqim", 5..=8), 4, false);
    superior(&mut e, a, "obf");
    superior(&mut e, a, "pot");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let pc = in_play(&mut e, 0, "Priority Contract", Some(z));
    e.state.cards[pc.index()].chosen = vec![a];
    let monster = in_play(&mut e, 0, "Monster", Some(a));
    hand(&mut e, 0, &["Provision of the Silsila", "Shadow Boxing"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 0, a));
    assert!(req.options.contains(&play(1, 2, a)), "{req:?}");
    assert!(req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == monster)));
}

// [LSJ 20021128] Raw Recruit: "If it leaves play, the card on it is
// burned": the vampire it holds goes to its owner's ash heap.
#[test]
fn raw_recruit_leaving_play_burns_its_vampire() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan("Tremere", 4..=8), 6, false);
    let p = e.add_minion(SeatId(0), find_card("Porphyry"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let victim_card = of_clan("Ventrue", 6..=6);
    let victim = e.add_minion(SeatId(1), victim_card, 2, true);
    hand(&mut e, 0, &["Raw Recruit"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(p),
            target: Target::Minion(victim),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let rr = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.held_vampire.is_some())
        .map(|i| CardId(i as u32))
        .expect("the vampire held");
    // "A vampire ... to whom this Gargoyle is enslaved", read live: the
    // Gargoyle burned, no master's action.
    e.burn(p);
    let req = request(&mut e);
    assert!(
        !req.options.iter().any(|o| matches!(o,
            Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == rr)),
        "{req:?}"
    );
    e.burn_card(rr);
    assert!(e.state.seats[1].ash_crypt.contains(&victim_card));
}

// [PIB 20120116] Rockheart [VIS] "still prevents only half of the current
// strike": of Qawiyya's 3, 2 prevented.
#[test]
fn superior_rockheart_prevents_half_the_current_strike() {
    let mut e = game(2);
    let q = e.add_minion(SeatId(0), find_card("Qawiyya el-Ghaduba"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(1), find_card("Porphyry"), 4, false);
    superior(&mut e, p, "vis");
    hand(&mut e, 1, &["Rockheart"]);
    pick(&mut e, bleed(q));
    pick_when(&mut e, block(p));
    pick_when(&mut e, play(0, 2, p));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, p), 3, "1 of 3 taken");
}

// Thoughts Betrayed [dom]: [RTR 19970630] [ANK 20200422] the damage is
// environmental, [RTR 19941109] [TOM 19951216] [PIB 20140324] [ANK 20200422]
// inflicted even against a dodge; a "strike: combat ends" ends combat
// before it.
#[test]
fn thoughts_betrayeds_damage_lands_through_a_dodge_not_a_combat_end() {
    for (disc, card, mode, taken) in [("cel", "Side Strike", 0, 1), ("PRE", "Staredown", 1, 0)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("dom", 0), 8, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), vampire_with(disc, 0), 6, false);
        hand(&mut e, 0, &["Thoughts Betrayed"]);
        hand(&mut e, 1, &[card]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 0, a));
        let before = blood(&e, z);
        pick_when(&mut e, play(0, mode, z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), before - taken, "{card}");
    }
}

// [ANK 20221021] Fatima al-Faqadi "Can only use her ability when she is in
// combat herself": another of her Methuselah's vampires in combat, her
// weapon stays in hand.
#[test]
fn fatima_equips_only_in_her_own_combat() {
    let mut e = game(2);
    let f = e.add_minion(SeatId(0), find_card("Fatima al-Faqadi"), 8, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &[".44 Magnum"]);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(z));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, own(f, 0));
}

// [ANK 20211009] [LSJ 20041025] Leadership Vacuum: "Priscus ballots are not
// taken into account": a priscus burned, X is its votes, its ballot not
// counted.
#[test]
fn leadership_vacuum_does_not_count_a_priscus_ballot() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let priscus = e.add_minion(
        SeatId(1),
        of_sect(zvtes::crypt::Sect::Sabbat, 2..=5),
        3,
        false,
    );
    e.state.minions[priscus.index()].granted_title = Some(zvtes::crypt::Title::Priscus);
    e.add_minion(SeatId(1), of_clan("Ventrue", 2..=4), 3, false);
    assert_eq!(e.state.ballots(priscus), 1);
    let votes = e.state.votes_held(priscus);
    hand(&mut e, 0, &["Leadership Vacuum"]);
    e.burn(priscus);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Seat(SeatId(1)),
        },
    );
    request(&mut e);
    let lv = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Leadership Vacuum"))
        .expect("in play");
    assert_eq!(e.state.cards[lv].fixed_x, votes);
}

// Hand Contract "choose a Black Hand vampire": with none in play, nothing
// to choose, not offered.
#[test]
fn hand_contract_needs_a_black_hand_vampire_in_play() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Hand Contract"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. })),
        "{req:?}"
    );
}

// Dam the Heart's River [QUI] on strike cards: [LSJ 20090529] a strike an
// effect grants afterwards (Aid from Bats) pays the extra blood; [LSJ
// 20070829-1] it is a card cost, reductions apply: a Giovanni's Breath of
// Thanatos under The Path of Bone costs 1 + 1 - 1. The Aid from Bats played
// for its maneuver, at long range; the Breath taking the hand strike's 1 + 1.
#[test]
fn dam_the_hearts_river_taxes_strike_cards_reductions_included() {
    let giovanni = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.clan == "Giovanni"
                && c.disciplines.contains(&"NEC")
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a Giovanni with superior Necromancy") as u16;
    for (striker, card, mode, hit) in [
        (vampire_with("ani", 0), "Aid from Bats", 0, 0),
        (giovanni, "Breath of Thanatos", 1, 2),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("QUI", 0), 8, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), striker, 6, false);
        if card == "Breath of Thanatos" {
            in_play(&mut e, 1, "Path of Bone, The", None);
        }
        hand(&mut e, 0, &["Dam the Heart's River"]);
        hand(&mut e, 1, &[card]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 1, a));
        let before = blood(&e, z);
        pick_when(&mut e, play(0, mode, z));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, z), before - 1 - hit, "{card}");
    }
}

// Dam the Heart's River [QUI]: [LSJ 20090703] strikes paid at declaration,
// so an opponent with no blood to pay has no strike; [ANK 20181128] the
// +1 reaches a retainer's damage and Botched Move's.
#[test]
fn dam_the_hearts_river_bars_an_unpaid_strike_and_raises_every_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("QUI", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    hand(&mut e, 0, &["Dam the Heart's River"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let before = blood(&e, a);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), before, "no strike against it");

    // A Murder of Crows' damage, +1.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("QUI", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    in_play(&mut e, 0, "Murder of Crows", Some(a));
    hand(&mut e, 0, &["Dam the Heart's River"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let before = blood(&e, z);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, z),
        before - 1 - 2 - 2,
        "its toll, hand strike 1+1, the crows 1+1"
    );

    // Botched Move's 1 unpreventable against Flash's maneuver, +1: read at
    // the opponent's strike, before any strike resolves.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("QUI", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("cel", 0), 8, false);
    hand(&mut e, 0, &["Dam the Heart's River", "Botched Move"]);
    hand(&mut e, 1, &["Flash"]);
    fill_hand(&mut e, 1);
    let flash = find_library("Flash");
    e.state.seats[1].library = [find_library("Conceal"), flash].into_iter().collect();
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    // [QUI]: its strike then a choice, a request to read at.
    pick_when(&mut e, play(0, 1, a));
    let before = blood(&e, z);
    pick_when(&mut e, play(0, 0, z));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert_eq!(blood(&e, z), before - 2, "1 unpreventable, +1");
}

// Waxen Poetica: "Not usable against a vampire with Celerity, an ally, or a
// retainer": at long range, not aimed at the opposing vampire's retainer.
#[test]
fn waxen_poetica_is_not_aimed_at_a_retainer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Nosferatu", 0), 6, false);
    let wax = in_play(&mut e, 0, "Waxen Poetica", Some(a));
    // A second ranged strike: a real choice, not elided.
    in_play(&mut e, 0, ".44 Magnum", Some(a));
    let dog = in_play(&mut e, 1, "Dog Pack", Some(z));
    hand(&mut e, 0, &["Cold Aura"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == wax)));
    assert!(!req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(s), target: Target::Card(c), .. }
            if *s == wax && *c == dog)), "{req:?}");
}

// Baldesar Rossellini on equipment another card plays (rules-qa 41.13,
// 36.5): Contraband's .44 Magnum costs 1 less.
#[test]
fn baldesar_reduces_contrabands_equipment() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Baldesar Rossellini"), 8, false);
    let ravnos = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.clan == "Ravnos"
                && c.disciplines.contains(&"OBF")
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a Ravnos with superior Obfuscate") as u16;
    let a = e.add_minion(SeatId(0), ravnos, 5, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    hand(&mut e, 0, &["Contraband", ".44 Magnum"]);
    let p0 = e.state.seats[0].pool;
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::SetX(0));
    pick_when(&mut e, Choice::ResolvePrinted { minion: b });
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, p0 - 1, "2 pool, 1 off");
}

// Filchware's Pawn Shop: a cost the equipping Methuselah cannot pay (rules-qa
// 38.5): no effect, the card left in its ash heap.
#[test]
fn filchwares_pawn_shop_fizzles_on_an_unpayable_cost() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let f = in_play(&mut e, 0, "Filchware's Pawn Shop", None);
    let magnum = find_library(".44 Magnum");
    e.state.seats[0].ash.push(magnum);
    e.state.seats[0].pool = 2;
    pick(&mut e, use_on(f, 0, Some(a), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 2, "2 pool, 1 more: unpayable");
    assert!(e.state.seats[0].ash.contains(&magnum));
}

// Thoughts Betrayed [dom] twice (no limit printed): 2 additional damage.
#[test]
fn thoughts_betrayed_twice_adds_two() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("dom", 0), 9, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    hand(&mut e, 0, &["Thoughts Betrayed", "Thoughts Betrayed"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    let before = blood(&e, z);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), before - 1 - 2, "the hand strike, 1 + 1");
}

// Victor Pelletier: not offered a title his Methuselah already holds — no
// contest with oneself (rules-qa 76.2).
#[test]
fn victor_pelletier_is_not_offered_a_title_already_held() {
    let mut e = game(2);
    let prince = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[prince.index()].granted_title = Some(zvtes::crypt::Title::Prince("Paris"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    influence_out(&mut e, find_card("Victor Pelletier"), 5);
    assert!(!request(&mut e).options.contains(&Choice::SetX(2)));
}

// Rules-qa 100.2 (owner, 2026-10-25): no vampire printed with a unique
// title his Methuselah claims already is influenced out — no voluntary
// contest with oneself (advanced rules §1.13.1); another city's prince is.
#[test]
fn a_prince_of_a_city_already_held_is_not_influenced_out() {
    use zvtes::crypt::Title;
    let card = CRYPT
        .iter()
        .position(|c| c.title == Title::Prince("Paris"))
        .expect("a Prince of Paris") as u16;
    for (held, contested, offered) in [
        (Title::Baron("Paris"), false, false),
        (Title::Baron("Paris"), true, false),
        (Title::Prince("Berlin"), false, true),
    ] {
        let mut e = game(2);
        let p = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[p.index()].granted_title = Some(held);
        // A contested copy still claims it as it returns (rules-qa 100.4).
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.state.minions[p.index()].contested = contested;
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card,
                copy: Default::default(),
                counters: CRYPT[card as usize].capacity,
                banished: None,
            });
        until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        let slot = e.state.seats[0].uncontrolled.len() as u8 - 1;
        let req = request(&mut e);
        assert_eq!(
            req.options.contains(&Choice::InfluenceOut { slot }),
            offered,
            "{held:?}, contested {contested}"
        );
    }
}

// Flight is read wherever it was gained (rules-qa 84.37): a Nosferatu's
// flight bought with Patagia bars the opponent's Earthshock.
#[test]
fn patagias_flight_bars_earthshock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Nosferatu", 0), 6, false);
    let patagia = in_play(&mut e, 1, "Patagia: Flaps Allowing Limited Flight", Some(z));
    hand(&mut e, 0, &["Earthshock"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_use_of(&mut e, patagia, 1);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

// Leadership Vacuum: "Only usable when a ready titled vampire ... is burned
// or sent to torpor": banished, it offers nothing (rules-qa 84.15).
#[test]
fn leadership_vacuum_is_not_offered_for_a_banished_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ventrue", 7..=11), 6, false);
    e.state.minions[a.index()].granted_title = Some(zvtes::crypt::Title::Prince("Paris"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), of_clan("Toreador", 2..=5), 3, false);
    e.state.minions[t.index()].granted_title = Some(zvtes::crypt::Title::Prince("Berlin"));
    e.add_minion(SeatId(1), plain("Ventrue", 1), 3, false);
    hand(&mut e, 0, &["Banishment", "Leadership Vacuum"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ChooseMinion(t));
    pick_when(&mut e, votes(a, true));
    close_polling(&mut e);
    request(&mut e);
    assert!(!e.state.minions[t.index()].in_play, "banished");
    assert!(
        e.state.titled_left.is_empty(),
        "not burned nor sent to torpor"
    );
}

// Leadership Vacuum: "During his or her unlock phase, if he or she controls
// a ready vampire with a capacity of 8 or more or with a title, burn this
// card" — read before the pool burn: a titled vampire of theirs, no pool
// lost (rules-qa 84.15).
#[test]
fn leadership_vacuum_burns_before_taxing_a_titled_methuselah() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let titled = e.add_minion(SeatId(1), of_clan("Ventrue", 2..=4), 3, false);
    e.state.minions[titled.index()].granted_title = Some(zvtes::crypt::Title::Prince("Paris"));
    let lv = in_play(&mut e, 0, "Leadership Vacuum", None);
    e.state.cards[lv.index()].against = zvtes::ids::SeatSet::single(SeatId(1));
    e.state.cards[lv.index()].fixed_x = 2;
    let pool = e.state.seats[1].pool;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[lv.index()].in_play, "burned");
    assert_eq!(e.state.seats[1].pool, pool, "no pool burned");
}

// Angelo is of any circle only for a card requiring Sanguinus (rules-qa
// 84.5, LSJ 20101013): not for Pavlo's ability, which feeds a Kiev Blood
// Brother but not Angelo.
#[test]
fn angelos_circle_is_not_a_vampire_abilitys() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Pavlo"), 3, false);
    let angelo = e.add_minion(SeatId(0), find_card("Angelo"), 4, false);
    let kiev = e.add_minion(SeatId(0), blood_brother("Kiev"), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let give = |t| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(p),
        ability: 0,
        actor: None,
        target: Target::Minion(t),
    };
    let req = offering(&mut e, give(kiev));
    assert!(!req.options.contains(&give(angelo)), "{req:?}");
}

// The `sanguinus` flag of a circle requirement is the card's Sanguinus
// requirement (rules-qa 84.5): set exactly on cards requiring it, never on
// a vampire's ability; the circle readers with no flag on Sanguinus cards
// only.
#[test]
fn circle_requirements_read_angelo_only_on_sanguinus_cards() {
    for def in zvtes::cards::defs::DEFS {
        let text = format!("{def:?}");
        let san = zvtes::library::LIBRARY[find_library(def.name) as usize]
            .disciplines
            .contains(&"san");
        assert!(!text.contains("sanguinus: true") || san, "{}", def.name);
        assert!(!text.contains("sanguinus: false") || !san, "{}", def.name);
        // The other circle readers count Angelo always: Sanguinus cards only.
        for reader in [
            "SameCircleAsActing",
            "ControlsOtherReadyUnlockedSameCircle",
            "CirclePicks",
            "GainPerUnlockedCircleMember",
            "CircleIntercept",
        ] {
            assert!(!text.contains(reader) || san, "{} reads {reader}", def.name);
        }
    }
    for def in zvtes::cards::vampires::VAMPIRES {
        assert!(
            !format!("{def:?}").contains("sanguinus: true"),
            "{}",
            def.name
        );
    }
}

// An ally's printed flight is read too (rules-qa 84.37): Earthshock is not
// usable against a blocking Herald of Topheth.
#[test]
fn herald_of_tophets_flight_bars_earthshock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let herald = e.add_ally(SeatId(1), find_library("Herald of Topheth"));
    hand(&mut e, 0, &["Earthshock"]);
    assert_eq!(e.state.discipline_level(herald, "flight"), 2);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(herald));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

// Dam the Heart's River [qui]: [RTR 19960221] a strike that deals no
// damage gets no +1: Heroic Might's "strike: burn equipment" burns the gun,
// no damage.
#[test]
fn dam_the_hearts_river_adds_nothing_to_a_non_damaging_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("QUI", 0), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(z));
    e.state.cards.push(CardInPlay::new(
        find_library("Heroic Might"),
        SeatId(0),
        Some(a),
        0,
    ));
    let hm = CardId(e.state.cards.len() as u32 - 1);
    hand(&mut e, 0, &["Dam the Heart's River"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    let before = blood(&e, z);
    pick_matching(&mut e, |o| {
        matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(s), target: Target::Card(c), .. }
            if *s == hm && *c == gun)
    });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[gun.index()].in_play, "the gun burned");
    assert_eq!(blood(&e, z), before, "no damage, no +1");
}

// Leadership Vacuum: "burned or sent to torpor" — a blocking prince sent
// to torpor in combat offers it, X its votes (rules-qa 84.15).
#[test]
fn leadership_vacuum_follows_a_titled_vampire_into_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let prince = e.add_minion(SeatId(1), of_clan("Ventrue", 2..=5), 0, false);
    e.state.minions[prince.index()].granted_title = Some(zvtes::crypt::Title::Prince("Paris"));
    e.add_minion(SeatId(1), of_clan("Ventrue", 2..=4), 3, false);
    let votes = e.state.votes_held(prince);
    hand(&mut e, 0, &["Leadership Vacuum"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(prince));
    let lv = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Seat(SeatId(1)),
    };
    pick_when(&mut e, lv);
    request(&mut e);
    assert!(e.state.minions[prince.index()].in_torpor);
    let card = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Leadership Vacuum"))
        .expect("in play");
    assert_eq!(e.state.cards[card].fixed_x, votes);
}

// Alvusia (owner, rules-qa 84.4): the bleed to a slave Gargoyle bleeding,
// after blocks are declined too; the strength to a slave Gargoyle of yours
// at any moment of an action, a blocking one in combat included.
#[test]
fn alvusia_reaches_after_blocks_and_a_blocking_gargoyle() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan("Tremere", 2..=11), 4, false);
    let g = e.add_minion(SeatId(0), find_card("Porphyry"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let al = in_play(&mut e, 0, "Alvusia", None);
    e.state.cards[al.index()].counters = 2;
    pick(&mut e, bleed(g));
    let req = until(&mut e, SeatId(0), WindowLabel::ActionBlocksDeclined);
    assert!(
        req.options.contains(&use_on(al, 0, Some(g), Target::None)),
        "{req:?}"
    );

    // Seat 1 bleeds; Porphyry blocks and, in combat, takes +1 strength.
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan("Tremere", 2..=11), 4, false);
    let g = e.add_minion(SeatId(0), find_card("Porphyry"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let al = in_play(&mut e, 0, "Alvusia", None);
    e.state.cards[al.index()].counters = 2;
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(g));
    pick_matching(&mut e, |o| {
        matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), ability: 1, actor: Some(m), .. }
            if *c == al && *m == g)
    });
    let before = blood(&e, z);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[al.index()].counters, 1);
    assert_eq!(blood(&e, z), before - 2, "1 strength + 1");
}
