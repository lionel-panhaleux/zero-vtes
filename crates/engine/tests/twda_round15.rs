//! Legacy cards for TWDA decks, round 15 (`ADDED_BATCHES`, slice 16aa): the
//! 2023-07-01 Promo set completed and the TWDA greedy's steps 1-8. Readings
//! in rules-qa Batch 78. Windows with a single option are elided, so the
//! scripts only stop where a real decision exists.

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
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

/// A vampire's own ability `ability` used by itself.
fn printed(m: MinionId, ability: u8, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor: Some(m),
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

fn stat(e: &Engine, m: MinionId, s: Stat) -> i32 {
    e.state.printed_mod(m, s, PrintedCtx::default())
}

/// The `nth` vampire with no printed text matching `f`.
fn textless(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| f(c) && zvtes::cards::vampire(*i as u16).is_none())
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a textless vampire")
}

fn pool(e: &Engine, seat: usize) -> i32 {
    e.state.seats[seat].pool
}

/// A master card played from hand at `target`.
fn master(card: u8, mode: u8, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: None,
        target,
    }
}

/// Through seat 0's next turn, taking every option `want` matches: how many.
fn next_turn_taking(e: &mut Engine, mut want: impl FnMut(&Choice) -> bool) -> usize {
    let start = e.state.turn;
    let mut n = 0;
    loop {
        let req = request(e);
        if req.seat == SeatId(0) && req.window == WindowLabel::MinionPhase && e.state.turn > start {
            return n;
        }
        // Every option seen by `want`, the first wanted taken.
        let wanted: Vec<bool> = req.options.iter().map(&mut want).collect();
        if let Some(i) = wanted.iter().position(|&w| w) {
            e.choose(i);
            n += 1;
            continue;
        }
        e.choose(fallback(&req));
    }
}

/// The option using vampire `m`'s printed ability `ability`, whoever acts.
fn vamp_ability(req: &DecisionRequest, m: MinionId, ability: u8) -> Option<Choice> {
    req.options.iter().copied().find(|o| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability: a, .. }
            if *v == m && *a == ability)
    })
}

/// Pass (or hand strike) until `m`'s ability `ability` is offered: take it.
fn use_vamp(e: &mut Engine, m: MinionId, ability: u8) {
    for _ in 0..300 {
        let req = request(e);
        if let Some(c) = vamp_ability(&req, m, ability) {
            pick(e, c);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("{m:?}'s ability {ability} never offered");
}

/// The `nth` textless untitled vampire of `sect` with capacity in `caps`.
fn of_sect(sect: Sect, caps: std::ops::RangeInclusive<u8>, nth: usize) -> u16 {
    textless(
        |c| c.sect == sect && c.title == Title::None && caps.contains(&c.capacity),
        nth,
    )
}

/// The first library card whose first mode requires `disc`.
fn requiring(disc: &str, kind: zvtes::library::CardType) -> u16 {
    LIBRARY
        .iter()
        .position(|c| {
            c.types.contains(&kind) && c.modes.first().is_some_and(|m| m.tags.contains(&disc))
        })
        .unwrap_or_else(|| panic!("no {kind:?} requiring {disc}")) as u16
}

fn ash_has(e: &Engine, seat: usize, name: &str) -> bool {
    e.state.seats[seat].ash.contains(&find_library(name))
}

// ---- Chunk A: vampires ---------------------------------------------------------

// The vampires printing only a sect or a title: no definition, their traits
// generated crypt data.
#[test]
fn the_round_15_text_only_vampires_carry_their_printed_traits() {
    use Sect::{Anarch, Camarilla, Sabbat};
    let cases: [(&str, Sect, Title); 9] = [
        ("Bloodfeud", Sabbat, Title::None),
        ("Denette Stensen", Sabbat, Title::None),
        ("Earl", Camarilla, Title::None),
        ("Frank Litzpar", Sabbat, Title::None),
        ("Jackson Asher", Camarilla, Title::None),
        ("Jephta Hester", Sabbat, Title::None),
        ("Lana Butcher", Camarilla, Title::None),
        ("Old Neddacka", Sabbat, Title::None),
        ("Bituin", Anarch, Title::Baron("Manila")),
    ];
    let mut e = game(2);
    for (name, sect, title) in cases {
        let card = find_card(name);
        assert!(
            zvtes::cards::vampire(card).is_none(),
            "{name}: no text to define"
        );
        let m = e.add_minion(SeatId(0), card, 1, false);
        assert_eq!(e.state.sect_of(m), Some(sect), "{name}");
        assert_eq!(e.state.title_of(m), title, "{name}");
        assert!(!e.state.is_black_hand(m), "{name}");
    }
}

// Appius Claudius Corvus: "Black Hand. Seraph. +1 stealth." The traits are
// generated [LSJ 20070322] [ANK 20180807].
#[test]
fn appius_is_a_black_hand_seraph_with_stealth() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Appius Claudius Corvus"), 4, false);
    assert_eq!(stat(&e, a, Stat::Stealth), 1);
    assert!(e.state.is_black_hand(a));
    assert!(e.state.minions[a.index()]
        .vampire()
        .is_some_and(|c| c.seraph));
    assert_eq!(
        e.state.title_of(a),
        Title::None,
        "a trait, not a title (rules-qa 78.1)"
    );
}

// Abdelsobek: "can unlock a vampire or mummy you control as a +1 stealth
// action." His own vampire, not another Methuselah's.
#[test]
fn abdelsobek_unlocks_a_vampire_of_his_controller() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Abdelsobek"), 4, false);
    let mine = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let theirs = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[mine.index()].locked = true;
    e.state.minions[theirs.index()].locked = true;
    let req = request(&mut e);
    assert!(!req.options.contains(&printed(a, 0, Target::Minion(theirs))));
    pick(&mut e, printed(a, 0, Target::Minion(mine)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[mine.index()].locked);
}

// Ghede: "can enter combat with a minion as a Ⓓ action. Combat cards
// requiring Serpentis [ser] cost him 1 fewer blood." The mode played
// [rules-qa 38.4].
#[test]
fn ghede_rushes_and_plays_serpentis_combat_cards_for_less() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Ghede"), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&printed(g, 0, Target::Minion(z))));
    let off = |card: u16| {
        e.state.printed_mod(
            g,
            Stat::BloodCostOff,
            PrintedCtx {
                card: Some((card, 0)),
                ..PrintedCtx::default()
            },
        )
    };
    use zvtes::library::CardType::Combat;
    assert_eq!(off(requiring("ser", Combat)), 1);
    assert_eq!(off(requiring("pot", Combat)), 0);
}

// Juniper: "may steal an ally controlled by your predator or prey as a +1
// stealth Ⓓ action." Not an ally across the table.
#[test]
fn juniper_steals_an_ally_of_her_predator_or_prey() {
    let mut e = game(4);
    let j = e.add_minion(SeatId(0), find_card("Juniper"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    for s in 1..4 {
        e.add_minion(SeatId(s), plain("Ventrue", s as usize), 4, false);
    }
    let prey = e.add_ally(SeatId(1), find_library("Political Ally"));
    let across = e.add_ally(SeatId(2), find_library("Gravebound Drone"));
    let req = request(&mut e);
    assert!(!req.options.contains(&printed(j, 0, Target::Minion(across))));
    pick(&mut e, printed(j, 0, Target::Minion(prey)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[prey.index()].owner, SeatId(0));
}

// Onaedo: "While /Otieno/ is ready, Onaedo gets +1 stealth during diablerie
// actions." No Otieno in the pool: never.
#[test]
fn onaedo_has_no_stealth_without_otieno() {
    let mut e = game(2);
    let o = e.add_minion(SeatId(0), find_card("Onaedo"), 4, false);
    let diablerie = PrintedCtx {
        kind: Some(ActionKind::Diablerie),
        ..PrintedCtx::default()
    };
    assert_eq!(e.state.printed_mod(o, Stat::Stealth, diablerie), 0);
}

// Saul Meira: "During your discard phase, Saul can burn 1 blood to unlock
// another ready Black Hand vampire." Any Methuselah's [rules-digest D11];
// not one that is not Black Hand.
#[test]
fn saul_meira_unlocks_a_black_hand_vampire_in_the_discard_phase() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Saul Meira"), 4, false);
    let bh = e.add_minion(SeatId(1), find_card("Appius Claudius Corvus"), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[bh.index()].locked = true;
    e.state.minions[other.index()].locked = true;
    let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::UseAbility { target: Target::Minion(x), .. } if *x == other)));
    let unlock = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::UseAbility { target: Target::Minion(x), .. } if *x == bh))
        .expect("on Appius");
    pick(&mut e, unlock);
    request(&mut e);
    assert!(!e.state.minions[bh.index()].locked);
    assert_eq!(blood(&e, s), 3);
}

// Tegyrius, Vizier: "2 votes (titled): While /Victoria Ash/ is in your ready
// region, Tegyrius gets +2 votes. +1 strength." Hers, read live.
#[test]
fn tegyrius_gets_two_votes_beside_victoria_ash() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tegyrius, Vizier"), 4, false);
    assert_eq!(e.state.title_of(t), Title::Votes(2));
    assert_eq!(stat(&e, t, Stat::Strength), 1);
    assert_eq!(stat(&e, t, Stat::Votes), 0);
    let theirs = e.add_minion(SeatId(1), find_card("Victoria Ash"), 4, false);
    assert_eq!(stat(&e, t, Stat::Votes), 0, "another Methuselah's");
    e.state.minions[theirs.index()].owner = SeatId(0);
    assert_eq!(stat(&e, t, Stat::Votes), 2);
}

// Feo Ramos: "During your unlock phase, older Sabbat vampires controlled by
// other Methuselahs can burn 1 blood to lock Feo." Not a younger one.
#[test]
fn feo_ramos_is_locked_by_an_older_sabbat_vampire() {
    let mut e = game(2);
    let f = e.add_minion(SeatId(0), find_card("Feo Ramos"), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let older = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 4..=6, 0), 3, false);
    let peer = CRYPT
        .iter()
        .position(|c| c.sect == Sect::Sabbat && c.capacity == 1 && c.name != "Feo Ramos")
        .expect("a 1-capacity Sabbat vampire") as u16;
    let younger = e.add_minion(SeatId(1), peer, 1, false);
    let lock = |m: MinionId| {
        move |o: &Choice| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), actor: Some(a), .. }
                if *v == f && *a == m)
        }
    };
    let mut seen_younger = false;
    let used = next_turn_taking(&mut e, |o| {
        seen_younger |= lock(younger)(o);
        lock(older)(o)
    });
    assert!(!seen_younger, "not older");
    assert_eq!((used, blood(&e, older)), (1, 2));
    assert!(e.state.minions[f.index()].locked);
}

// Shalmath: "Once each turn, Shalmath can unlock. He cannot use cards
// requiring Celerity [cel]." Once in each Methuselah's turn [ANK
// 20180719-4]; a card's [cel] mode barred [LSJ 20050322-2].
#[test]
fn shalmath_unlocks_once_each_turn_and_shuns_celerity() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Shalmath"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let barred = |e: &Engine, card: u16| {
        e.state.printed_mod(
            s,
            Stat::CannotPlay,
            PrintedCtx {
                card: Some((card, 0)),
                ..PrintedCtx::default()
            },
        )
    };
    use zvtes::library::CardType::Combat;
    assert_eq!(barred(&e, requiring("cel", Combat)), 1);
    assert_eq!(barred(&e, requiring("pot", Combat)), 0);
    let brawl = find_library("Line Brawl");
    let mode = |m: u8| {
        e.state.printed_mod(
            s,
            Stat::CannotPlay,
            PrintedCtx {
                card: Some((brawl, m)),
                ..PrintedCtx::default()
            },
        )
    };
    assert_eq!((mode(0), mode(1)), (1, 0), "[cel] barred, [pot] not");
    let req = request(&mut e);
    assert!(
        vamp_ability(&req, s, 0).is_none(),
        "unlocked: nothing to do"
    );
    e.state.minions[s.index()].locked = true;
    use_vamp(&mut e, s, 0);
    request(&mut e);
    assert!(!e.state.minions[s.index()].locked);
    e.state.minions[s.index()].locked = true;
    let req = request(&mut e);
    assert!(vamp_ability(&req, s, 0).is_none(), "once this turn");
    let again = Choice::UseAbility {
        source: AbilitySource::Vampire(s),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    assert!(
        offered_before(&mut e, again, SeatId(1), WindowLabel::MinionPhase),
        "again in the next turn"
    );
}

// Cybele: "If Cybele is unlocked at the start of your master phase, you get
// +1 master phase action. She can bleed with +2 bleed as a Ⓓ action that
// costs 1 blood. Infernal."
#[test]
fn cybele_gives_a_master_phase_action_and_bleeds_for_three() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Cybele"), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    assert!(e.state.is_infernal(c));
    let req = request(&mut e);
    let act = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(m), ability: 0, .. } if *m == c))
        .expect("her bleed");
    e.choose(act);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!((pool(&e, 1), blood(&e, c)), (10 - 3, 4));
    // Locked by her bleed and infernal: unpaid, she starts the master
    // phase locked (no extra action); unlocked, she gives one.
    let mpa = |e: &mut Engine| {
        until(e, SeatId(0), WindowLabel::MinionPhase);
        e.state.mpa_left(SeatId(0))
    };
    let locked = mpa(&mut e);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[c.index()].locked = false;
    assert_eq!((locked, mpa(&mut e)), (1, 2));
}

// César Holfield: "At the end of your minion phase, if there are any
// Gehenna events in play and César did not hunt, he burns 1 blood." Not
// without one; not after a hunt.
#[test]
fn cesar_holfield_burns_a_blood_under_gehenna_unless_he_hunted() {
    for (gehenna, hunt, burned) in [(true, true, 0), (false, false, 0), (true, false, 1)] {
        let mut e = game(2);
        let c = e.add_minion(
            SeatId(0),
            find_card("César Holfield"),
            u8::from(!hunt),
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if gehenna {
            in_play(&mut e, 1, "Fall of the Sabbat", None);
        }
        // With no blood, his one action is the hunt, taken.
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(1 - blood(&e, c), burned, "{gehenna} {hunt}");
    }
}

/// Kine Resources Contested called by `caller` of seat 0: `mine` points on
/// its Methuselah, `theirs` on seat 1, passed.
fn kine_resources(e: &mut Engine, caller: MinionId, mine: u8, theirs: u8) {
    hand(e, 0, &["Kine Resources Contested"]);
    pick(e, play_at(0, 0, caller, Target::None));
    pick(e, alloc(0, mine));
    if mine + theirs < 4 {
        pick(e, alloc(1, theirs));
    }
    let req = request(e);
    if req.options.contains(&votes(caller, true)) {
        pick(e, votes(caller, true));
    }
    close_polling(e);
}

// Donald Cargill: "Donald burns 1 blood after a referendum called by your
// predator passes." Not his prey's referendum; still when it ousts that
// predator [LSJ 20090115-2].
#[test]
fn donald_cargill_bleeds_after_his_predators_referendum() {
    let mut e = game(3);
    let caller = e.add_minion(SeatId(0), find_card("Andi Liu"), 4, false);
    let d = e.add_minion(SeatId(1), find_card("Donald Cargill"), 1, false);
    e.add_minion(SeatId(2), plain("Brujah", 1), 4, false);
    kine_resources(&mut e, caller, 0, 3);
    request(&mut e);
    assert_eq!(blood(&e, d), 0, "his predator's");

    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let caller = e.add_minion(SeatId(0), find_card("Andi Liu"), 4, false);
    let d = e.add_minion(SeatId(2), find_card("Donald Cargill"), 1, false);
    e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    kine_resources(&mut e, caller, 0, 3);
    request(&mut e);
    assert_eq!(blood(&e, d), 1, "his prey's");

    let mut e = game(3);
    let caller = e.add_minion(SeatId(0), find_card("Andi Liu"), 4, false);
    let d = e.add_minion(SeatId(1), find_card("Donald Cargill"), 1, false);
    e.add_minion(SeatId(2), plain("Brujah", 1), 4, false);
    e.state.seats[0].pool = 2;
    kine_resources(&mut e, caller, 2, 1);
    request(&mut e);
    assert!(e.state.seats[0].ousted);
    assert_eq!(blood(&e, d), 0, "the predator it ousted");
}

// Edward Vignes: "You may burn a card that requires Dominate [dom] from your
// hand to cause an action directed at Edward to fail." Not once a block is
// attempted [ANK 20260316] [LSJ 20041022]; the failed action was not
// blocked [LSJ 20070219].
#[test]
fn edward_vignes_fails_an_action_at_him_for_a_dominate_card() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Ghede"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ed = e.add_minion(SeatId(1), find_card("Edward Vignes"), 4, false);
    hand(&mut e, 1, &["Deflection", "Blood Doll"]);
    e.state.seats[1].library = vec![find_library("Blood Doll"); 9].into();
    pick(&mut e, printed(g, 0, Target::Minion(ed)));
    let fail = vamp_ability(&offering_vamp(&mut e, ed), ed, 0).expect("offered");
    // The Dominate card the only pick: elided.
    pick(&mut e, fail);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Deflection"));
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
    assert_eq!(
        e.state.seats[1].library.len(),
        9 - 1 - 5,
        "replaced, drawn up"
    );
}

/// Pass (or hand strike) until vampire `m`'s ability 0 is offered: that
/// request, unanswered.
fn offering_vamp(e: &mut Engine, m: MinionId) -> DecisionRequest {
    for _ in 0..300 {
        let req = request(e);
        if vamp_ability(&req, m, 0).is_some() {
            return req;
        }
        e.choose(fallback(&req));
    }
    panic!("{m:?}'s ability never offered");
}

// Iris Bennett: "If an older vampire blocks Iris, their controller can look
// at your hand before block resolution." Not a younger blocker's.
#[test]
fn iris_bennett_shows_her_hand_to_an_older_blocker() {
    let peer = CRYPT
        .iter()
        .position(|c| c.sect == Sect::Camarilla && c.capacity == 1 && c.name != "Iris Bennett")
        .expect("a 1-capacity Camarilla vampire") as u16;
    for (card, offered) in [(of_sect(Sect::Camarilla, 6..=8, 0), true), (peer, false)] {
        let mut e = game(2);
        let i = e.add_minion(SeatId(0), find_card("Iris Bennett"), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let b = e.add_minion(SeatId(1), card, 1, false);
        pick(&mut e, bleed(i));
        pick_when(&mut e, block(b));
        let req = request(&mut e);
        let peek = req.options.iter().copied().find(|o| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), actor: Some(a), .. }
                if *v == i && *a == b)
        });
        assert_eq!(peek.is_some(), offered, "{req:?}");
        if let Some(p) = peek {
            pick(&mut e, p);
            assert!(e.state.peeks.contains(&(SeatId(1), SeatId(0))));
        }
    }
}

// Katherine Stoddard: "Once during your turn, if Katherine is ready, you may
// discard a card that requires Auspex [aus] from your hand. Draw back up to
// your hand size afterward."
#[test]
fn katherine_stoddard_cycles_an_auspex_card_once_a_turn() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Katherine Stoddard"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Eyes of Argus", "Deflection"]);
    let doll = find_library("Blood Doll");
    e.state.seats[0].library = vec![doll; 5].into();
    // The Auspex card the only pick: elided.
    use_vamp(&mut e, k, 0);
    let req = request(&mut e);
    assert!(ash_has(&e, 0, "Eyes of Argus"));
    assert!(e.state.seats[0].library.is_empty(), "drawn back up");
    assert!(vamp_ability(&req, k, 0).is_none(), "once this turn");
}

// Lubomira Hradok: "If Lubomira successfully bleeds a Methuselah who controls
// a ready titled vampire, she does not unlock as normal during her next
// unlock phase." Lubomira, not the bled one [PIB 20110915-1].
#[test]
fn lubomira_stays_locked_after_bleeding_a_methuselah_with_a_titled_vampire() {
    for (titled, skip) in [(true, true), (false, false)] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), find_card("Lubomira Hradok"), 3, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let card = if titled {
            find_card("Andi Liu")
        } else {
            plain("Ventrue", 0)
        };
        let z = e.add_minion(SeatId(1), card, 2, false);
        e.state.minions[z.index()].locked = true;
        pick(&mut e, bleed(l));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[l.index()].skip_unlock, skip, "{titled}");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[l.index()].locked, skip, "{titled}");
    }
}

// Walker Grimes: "Master cards targeting Walker cost you 1 additional pool."
// His controller's only [LSJ 20081124-1].
#[test]
fn walker_grimes_taxes_his_own_masters() {
    let mut e = game(2);
    let w = e.add_minion(SeatId(0), find_card("Walker Grimes"), 1, false);
    let villein = find_library("Villein");
    let at = Target::Minion(w);
    let base = zvtes::cards::pool_cost(&e.state, SeatId(1), villein, at);
    assert_eq!(
        zvtes::cards::pool_cost(&e.state, SeatId(0), villein, at),
        base + 1
    );
}

// Skidmark: "When Skidmark is acting, you do not replace any cards you play
// until the end of the action." His action card too, and a modifier,
// replaced as the action ends.
#[test]
fn skidmark_replaces_his_cards_at_the_end_of_the_action() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Skidmark"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Restoration"]);
    e.state.seats[0].library = vec![find_library("Blood Doll"); 9].into();
    request(&mut e);
    let n = e.state.seats[0].hand.len();
    pick(&mut e, play_at(0, 0, s, Target::None));
    request(&mut e);
    assert!(e.action().is_some());
    assert_eq!(e.state.seats[0].hand.len(), n - 1, "held during the action");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].hand.len(), n, "replaced as it ends");
    // A modifier he plays as he bleeds: held too.
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Skidmark"), 4, false);
    superior(&mut e, s, "dom");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bonding"]);
    e.state.seats[0].library = vec![find_library("Blood Doll"); 9].into();
    pick(&mut e, bleed(s));
    let n = e.state.seats[0].hand.len();
    pick_when(&mut e, play(0, 0, s));
    request(&mut e);
    assert!(e.action().is_some());
    assert_eq!(e.state.seats[0].hand.len(), n - 1, "the modifier held");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.seats[0].hand.len(),
        n,
        "replaced as the action ends"
    );
}

// Franciscus: "Franciscus is burned (without going into torpor) if he takes
// more than 3 damage during a single combat." Four damage burn him; two
// send him to torpor.
#[test]
fn franciscus_burns_past_three_damage_in_a_combat() {
    for (bonus, burned) in [(3, true), (1, false)] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), find_card("Ghede"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let f = e.add_minion(SeatId(1), find_card("Franciscus"), 1, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.state.minions[g.index()].timed.push((
            zvtes::cards::Lasting::Strength(bonus),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        pick(&mut e, printed(g, 0, Target::Minion(f)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[f.index()];
        assert_eq!((v.in_play, v.in_torpor), (!burned, !burned), "{bonus}");
    }
}

// Bolesław Gutowski: "During an action he performs, Bolesław can burn 1
// blood to reveal the top card of your crypt, then shuffle your crypt. If it
// is a copy of Bolesław, he gets +1 stealth; otherwise, he gets +1
// strength." Usable before stealth is needed, the stealth then lost [ANK
// 20231031].
#[test]
fn boleslaw_reveals_his_crypt_for_strength_or_stealth() {
    for (copy, strength) in [(false, 1), (true, 0)] {
        let mut e = game(2);
        let b = e.add_minion(SeatId(0), find_card("Bolesław Gutowski"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let top = if copy {
            find_card("Bolesław Gutowski")
        } else {
            plain("Toreador", 0)
        };
        e.state.seats[0].crypt = vec![top].into();
        pick(&mut e, bleed(b));
        use_vamp(&mut e, b, 0);
        request(&mut e);
        let a = e.action().expect("action");
        assert_eq!(a.strength_bonus, strength, "{copy}");
        assert_eq!(a.stealth(&e.state), 0, "{copy}: no stealth needed yet");
        assert_eq!(blood(&e, b), 3);
    }
    // An empty crypt: nothing to reveal, no use (advanced rules §1.6.5).
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bolesław Gutowski"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].crypt = Vec::new().into();
    pick(&mut e, bleed(b));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        printed(b, 0, Target::None),
    );
}

// Helena (ADV): "Helena pays no blood costs for reaction cards requiring
// Auspex [aus] or Dominate [dom]. [MERGED] Helena is immune to damage from
// demons and gets +1 level of Daimoinon [dai] and Obtenebration [obt]." Any
// X free [LSJ 20051117]; merged, Independent [ANK 20180524].
#[test]
fn helena_adv_plays_auspex_reactions_free_and_merges() {
    let mut e = game(2);
    let adv = CRYPT
        .iter()
        .position(|c| c.name == "Helena" && c.adv)
        .expect("Helena (ADV)") as u16;
    let h = e.add_minion(SeatId(0), adv, 4, false);
    let off = |e: &Engine, card: u16, reacting: bool| {
        e.state.printed_mod(
            h,
            Stat::BloodCostOff,
            PrintedCtx {
                card: Some((card, 0)),
                reacting,
                ..PrintedCtx::default()
            },
        )
    };
    use zvtes::library::CardType::{Combat, Reaction};
    assert!(off(&e, requiring("aus", Reaction), true) >= 10);
    assert!(off(&e, requiring("dom", Reaction), true) >= 10);
    assert_eq!(off(&e, requiring("cel", Combat), false), 0);
    assert_eq!(e.state.discipline_level(h, "obt"), 0, "unmerged");
    e.state.minions[h.index()].merged = Some(find_card("Helena"));
    assert_eq!(e.state.sect_of(h), Some(Sect::Independent));
    assert_eq!(e.state.discipline_level(h, "obt"), 1);
    assert_eq!(e.state.discipline_level(h, "dai"), 2);
    assert_eq!(stat(&e, h, Stat::Bleed), 1, "the base card's +1 bleed");
}

// ---- Chunk B: library cards on existing primitives -------------------------

// Thrown Gate: "[POT] Strike: 2R damage, with an optional maneuver."
#[test]
fn thrown_gate_strikes_for_two_at_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    superior(&mut e, a, "POT");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Thrown Gate"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4 - 2);
}

// Typhonic Beast: "[POT][SER] +1 strength this combat, prevent 1 damage each
// round, with 1 optional press this round."
#[test]
fn typhonic_beast_strengthens_and_shields() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "SER");
    superior(&mut e, a, "POT");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Typhonic Beast"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 2, a));
    pick_when(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let hit = stat(&e, a, Stat::Strength) as u8 + 1 + 1;
    assert_eq!(blood(&e, z), 4 - hit, "+1 strength");
    assert_eq!(
        blood(&e, a),
        4 - 1,
        "its blood cost; the hand strike prevented"
    );
}

// Engling Fury: "[spi] This vampire gains 2 blood, and unlock this vampire
// during the next discard phase. [SPI] gains 2 blood and unlocks."
#[test]
fn engling_fury_feeds_and_unlocks() {
    for (mode, discipline) in [(1u8, "spi"), (2, "SPI")] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("for", 0), 1, false);
        superior(&mut e, a, if discipline == "SPI" { "SPI" } else { "spi" });
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Engling Fury"]);
        pick(&mut e, play_at(0, mode, a, Target::None));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), 3, "{discipline}");
        assert_eq!(e.state.minions[a.index()].locked, mode == 1, "{discipline}");
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(!e.state.minions[a.index()].locked, "{discipline}");
    }
}

// Flurry of Action: "[cel] Ⓓ Bleed. If the bleed is successful, draw two
// cards (discard afterward). [CEL] … this vampire unlocks." As much as the
// library holds [LSJ 20060620].
#[test]
fn flurry_of_action_draws_or_unlocks_after_a_successful_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("cel", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Flurry of Action"]);
    e.state.seats[0].library = vec![find_library("Blood Doll"); 1].into();
    request(&mut e);
    pick(&mut e, play_at(0, 0, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 10 - 1);
    assert!(e.state.seats[0].library.is_empty(), "one card left: drawn");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("cel", 0), 4, false);
    superior(&mut e, a, "CEL");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Flurry of Action"]);
    pick(&mut e, play_at(0, 1, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 10 - 1);
    assert!(
        !e.state.minions[a.index()].locked,
        "unlocked after the bleed"
    );
}

// Domain of Evernight: "[tem] If this action is blocked, all damage
// inflicted on vampires during the resulting combat is aggravated. [TEM]
// Only usable after resolution of a successful action. Unlock this
// vampire." After resolution, still during the action [LSJ 19981028] [ANK
// 20190425]; a vampire in torpor from aggravated damage burns blood.
#[test]
fn domain_of_evernight_aggravates_a_blocked_combat_and_unlocks_after_success() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    superior(&mut e, a, "tem");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    hand(&mut e, 0, &["Domain of Evernight"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[z.index()].in_play,
        "aggravated damage beyond its blood: burned"
    );

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    superior(&mut e, a, "TEM");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Domain of Evernight", "Domain of Evernight"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 2, a));
    let req = request(&mut e);
    assert!(!e.state.minions[a.index()].locked);
    assert!(
        !req.options.contains(&play(0, 2, a)),
        "one at superior each turn"
    );
}

// Hag's Wrinkles: "Only usable during an equip action. [thn] Unlock this
// vampire after action resolution if this action is successful. [THN] As
// [thn] above, but with +1 stealth": [THN] only while stealth is needed
// (rules-qa 8.1); Magic of the Smith an equip action (rules-qa 78.28).
#[test]
fn hags_wrinkles_unlocks_after_a_successful_equip() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    superior(&mut e, a, "THN");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Hag's Wrinkles", "Kevlar Vest"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play(0, 1, a)),
        "not outside an equip"
    );
    pick(&mut e, play_at(1, 0, a, Target::None));
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 1, a)));
    assert!(!req.options.contains(&play(0, 2, a)), "no stealth needed");
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked);
    // Magic of the Smith's search and equip is an equip action too (owner,
    // 2026-10-17).
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    superior(&mut e, a, "THN");
    superior(&mut e, a, "tha");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Hag's Wrinkles", "Magic of the Smith"]);
    e.state.seats[0].library = vec![find_library("Kevlar Vest"); 3].into();
    pick(&mut e, play_at(1, 0, a, Target::None));
    pick_when(&mut e, play(0, 1, a));
}

// Burnt Offerings: "[AUS] Only usable if a minion controlled by your
// predator successfully bleeds you. Your predator burns 1 pool."
#[test]
fn burnt_offerings_burns_the_predators_pool_after_a_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("aus", 0), 4, false);
    superior(&mut e, z, "AUS");
    hand(&mut e, 1, &["Burnt Offerings"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((pool(&e, 0), pool(&e, 1)), (10 - 1, 10 - 1));
    assert_eq!(blood(&e, z), 4 - 1, "its blood cost");
}

// Tattoo Signal: "+1 stealth action. Requires a Seraph. Put this card on
// this acting Seraph. This ready Seraph may lock this card to unlock
// another Black Hand vampire."
#[test]
fn tattoo_signal_lets_a_seraph_unlock_a_black_hand_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Appius Claudius Corvus"), 4, false);
    let s = e.add_minion(SeatId(0), find_card("Saul Meira"), 4, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Tattoo Signal"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play_at(0, 0, s, Target::None)),
        "not a Seraph"
    );
    pick(&mut e, play_at(0, 0, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let tattoo = find_library("Tattoo Signal");
    let id = e
        .state
        .cards
        .iter()
        .position(|c| c.card == tattoo && c.present())
        .expect("in play") as u32;
    assert!(!e.state.cards[id as usize].effect_only);
    assert_eq!(e.state.cards[id as usize].host, Some(a));
    assert!(!ash_has(&e, 0, "Tattoo Signal"));
    e.state.minions[s.index()].locked = true;
    e.state.minions[other.index()].locked = true;
    let req = request(&mut e);
    let unlock = |t: MinionId| Choice::UseAbility {
        source: AbilitySource::Card(CardId(id)),
        ability: 0,
        actor: None,
        target: Target::Minion(t),
    };
    assert!(!req.options.contains(&unlock(other)), "not Black Hand");
    pick(&mut e, unlock(s));
    request(&mut e);
    assert!(!e.state.minions[s.index()].locked);
}

// The burn option reads a card's printed requirement (rules-qa 33.8): a
// ready Seraph bars Tattoo Signal's [LSJ 20070410], a non-Seraph does not,
// nor a Seraph in torpor [LSJ 20091203]; a ready infernal vampire bars
// Unleash Hell's Fury's.
#[test]
fn the_burn_option_reads_the_cards_printed_requirement() {
    for (card, holder, torpid, offered) in [
        ("Tattoo Signal", "Appius Claudius Corvus", false, false),
        ("Tattoo Signal", "Appius Claudius Corvus", true, true),
        ("Tattoo Signal", "Saul Meira", false, true),
        ("Unleash Hell's Fury", "Cybele", false, false),
    ] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), find_card(holder), 4, torpid);
        let c = find_library(card);
        e.state.seats[1].hand = vec![c].into();
        e.state.seats[1].library = vec![find_library("Soak"); 6].into();
        pick(&mut e, Choice::Pass);
        let found = loop {
            let req = request(&mut e);
            if req.window == WindowLabel::UnlockAbilities && req.seat == SeatId(1) {
                break req.options.contains(&Choice::Discard(0));
            }
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break false;
            }
            e.choose(fallback(&req));
        };
        assert_eq!(found, offered, "{card} with {holder} (torpor {torpid})");
    }
}

// ---- Chunk C: library cards with new mechanisms ----------------------------

// Outside the Hourglass: "[TEM] Only usable before range is determined.
// This vampire inflicts 2 damage on the opposing minion. A vampire can play
// only one Outside the Hourglass at superior each round." The vampire's
// damage [PIB 20120426], mended as the before-range effects are done [ANK
// 20170930]: 2, then the hand strike.
#[test]
fn outside_the_hourglass_inflicts_two_before_range_once_a_round() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    superior(&mut e, a, "TEM");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    hand(
        &mut e,
        0,
        &["Outside the Hourglass", "Outside the Hourglass"],
    );
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 3, a));
    // A second copy barred, the window elides: the damage mended at once.
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play(0, 3, a)),
        "one at superior each round"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 5 - 2 - 1);
}

// Outside the Hourglass: "[obf] Strike: dodge." A vampire with Obfuscate and
// no Temporis may play it (rules-qa 25.2).
#[test]
fn outside_the_hourglass_dodges_without_temporis() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Outside the Hourglass"]);
    request(&mut e);
    let start = blood(&e, a);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 0, a));
    assert!(
        !req.options.contains(&play(0, 2, a)),
        "the [tem] strike needs Temporis"
    );
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), start, "dodged");
}

// Mythic Form: "[PRO] This combat, +1 strength, prevent 1 non-aggravated
// damage each round, 1 optional maneuver this round, and this vampire can
// strike: 2R aggravated damage once each round this combat." One each
// combat; the strike once, the round's.
#[test]
fn mythic_form_grants_an_aggravated_strike_and_a_shield() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    superior(&mut e, a, "PRO");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    hand(&mut e, 0, &["Mythic Form", "Mythic Form"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 1, a)), "one each combat");
    pick_when(&mut e, Choice::RoundStrike { at: None });
    pick_when(&mut e, Choice::UsePrevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Aggravated: the first point wounds, the second burns a blood.
    assert!(e.state.minions[z.index()].in_torpor, "2 aggravated");
    assert_eq!(blood(&e, z), 5 - 1);
    assert_eq!(blood(&e, a), 5 - 3, "its cost; the hand strike prevented");
}

// Stunt Cycle: "Only usable at long range. [pot] Strike: 3R damage, and
// prevent 1 damage from the opposing minion's strike during this strike
// resolution." The opposing 1R strike prevented [RTR 20041202].
#[test]
fn stunt_cycle_strikes_at_long_range_and_prevents_the_return() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    superior(&mut e, a, "pot");
    superior(&mut e, a, "OBF");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    superior(&mut e, z, "pot");
    hand(&mut e, 0, &["Stunt Cycle", "Swallowed by the Night"]);
    hand(&mut e, 1, &["Thrown Gate"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(1, 1, a));
    assert!(
        !req.options.contains(&play(0, 0, a)),
        "close range: no Stunt Cycle"
    );
    e.choose(index_of(&req, play(1, 1, a)));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 5 - 3);
    assert_eq!(blood(&e, a), 5, "the Thrown Gate's 1 prevented");
}

// Stunt Cycle: "If the opposing minion has Celerity [cel], he or she can
// burn 1 blood to prevent all damage from this strike."
#[test]
fn stunt_cycle_is_shrugged_off_with_celerity_for_a_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    superior(&mut e, a, "pot");
    superior(&mut e, a, "OBF");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    superior(&mut e, z, "cel");
    hand(&mut e, 0, &["Stunt Cycle", "Swallowed by the Night"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(1, 1, a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::ResolvePrinted { minion: z });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 5 - 1, "the burn, no damage");
}

// Mantle of the Bestial Majesty: "[PRO] +1 stealth action. The acting
// vampire unlocks. Until his or her next unlock phase, damage from this
// vampire's hand strikes is aggravated, and +1 strength." Two aggravated
// on an empty vampire burn it.
#[test]
fn mantle_of_the_bestial_majesty_aggravates_hand_strikes_until_unlock() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    superior(&mut e, a, "PRO");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    hand(&mut e, 0, &["Mantle of the Bestial Majesty"]);
    pick(&mut e, play_at(0, 1, a, Target::None));
    // Unlocked by it: his bleed offered in the same minion phase.
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_play, "2 aggravated: burned");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(stat(&e, a, Stat::Strength), 0, "until the unlock phase");
}

// Corporal Reservoir: "Put this card on any Black Hand vampire. This
// vampire may lock this card to prevent 1 damage in combat or to gain a
// blood. This card doesn't unlock as normal. This vampire may burn a blood
// to unlock this card during his or her unlock phase." Its controller's
// choice to use it [ANK 20210309-3].
#[test]
fn corporal_reservoir_locks_for_blood_and_stays_locked() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Saul Meira"), 3, false);
    let other = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let req = request(&mut e);
    let _ = req;
    let id = in_play(&mut e, 0, "Corporal Reservoir", Some(s));
    let _ = other;
    let use_on = |ai: u8| Choice::UseAbility {
        source: AbilitySource::Card(id),
        ability: ai,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, use_on(1));
    request(&mut e);
    assert_eq!(blood(&e, s), 4);
    assert!(e.state.cards[id.index()].locked);
    // Seat 0's next unlock phase: still locked, its unlock for a blood
    // offered.
    let mut offered = false;
    next_turn_taking(&mut e, |o| {
        offered |= *o == use_on(2);
        false
    });
    assert!(offered, "the burn to unlock");
    assert!(
        e.state.cards[id.index()].locked,
        "does not unlock as normal"
    );
}

// Into Thin Air: "[obf] +1 stealth. Once this action, this vampire may burn
// 1 blood to give an ally or younger vampire -1 intercept. Into Thin Air and
// Lost in Crowds cannot both be played on the same action."
#[test]
fn into_thin_air_strips_a_younger_blockers_intercept() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 1..=2, 0), 2, false);
    superior(&mut e, z, "aus");
    hand(&mut e, 0, &["Into Thin Air", "Lost in Crowds"]);
    hand(&mut e, 1, &["Eyes of Argus"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    // +1 stealth against 0 intercept; Eyes of Argus' +1 intercept back.
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, z));
    let req = request(&mut e);
    assert!(
        !plays(&req)
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. })),
        "not Lost in Crowds after it: {req:?}"
    );
    let burn = req
        .options
        .iter()
        .copied()
        .find(|o| {
            matches!(
                o,
                Choice::UseAbility {
                    source: AbilitySource::Card(_),
                    ..
                }
            )
        })
        .expect("its burn");
    let before = e.action().expect("action").intercept_of(&e.state, z);
    pick(&mut e, burn);
    let act = e.action().expect("action");
    assert_eq!(act.intercept_of(&e.state, z), before - 1);
    assert_eq!(blood(&e, a), 2);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Into Thin Air"), "burned as the action ends");
}

// Tye Cooper: "Tye can burn a non-ready minion as a Ⓓ action; if that
// action is successful, you can move a library card from your ash heap to
// your hand. He is immune to non-aggravated damage." The card picked on
// success; immune, no damage inflicted, even unpreventable [LSJ 20010627].
#[test]
fn tye_cooper_burns_a_torpid_minion_and_fetches_from_the_ash_heap() {
    let mut e = game(2);
    let t = e.add_ally(SeatId(0), find_library("Tye Cooper"));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let doll = find_library("Blood Doll");
    e.state.seats[0].ash = vec![doll].into();
    let req = request(&mut e);
    let burn = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(_), actor: Some(a), target: Target::Minion(x), .. } if *a == t && *x == z))
        .expect("his burn");
    pick(&mut e, burn);
    pick_when(&mut e, Choice::FindCard(doll));
    request(&mut e);
    assert!(!e.state.minions[z.index()].in_play, "burned");
    assert!(e.state.seats[0].hand.contains(&doll));
    assert!(!ash_has(&e, 0, "Blood Doll"));
    assert!(e.state.immune_non_aggravated(t));
    // Its target out of torpor by resolution: the burn fizzles, a success,
    // the ash card still fetched (owner, 2026-10-17; rules-qa 78.38).
    let mut e = game(2);
    let t = e.add_ally(SeatId(0), find_library("Tye Cooper"));
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[0].ash = vec![doll].into();
    let req = request(&mut e);
    let burn = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(_), actor: Some(a), target: Target::Minion(x), .. } if *a == t && *x == z))
        .expect("his burn");
    pick(&mut e, burn);
    e.state.minions[z.index()].in_torpor = false;
    pick_when(&mut e, Choice::FindCard(doll));
    request(&mut e);
    assert!(e.state.minions[z.index()].in_play, "not burned: fizzled");
    assert!(e.state.seats[0].hand.contains(&doll));
}

// Taking the Skin: Minion: "[abo] Skin. Play when this vampire burns a
// minion. Put this card on this vampire and unlock him or her. +1 bleed and
// +1 stealth when bleeding. Burn this card during your discard phase." In
// combat the opponent burned it [LSJ 20090922].
#[test]
fn taking_the_skin_follows_a_burned_minion_with_a_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    superior(&mut e, a, "abo");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Strength(3),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    hand(&mut e, 0, &["Taking the Skin: Minion"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(ally));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play);
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    let skin = find_library("Taking the Skin: Minion");
    let on = e
        .state
        .cards
        .iter()
        .find(|c| c.card == skin && c.present())
        .expect("on him");
    assert!(!on.effect_only && on.host == Some(a));
    // Unlocked, he bleeds again with +1 bleed.
    pick_when(&mut e, bleed(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 10 - 2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        ash_has(&e, 0, "Taking the Skin: Minion"),
        "burned in the discard phase"
    );
}

// Pocket Out of Time: "[TEM] Usable after combat. After each combat this
// action, this vampire can burn 1 blood to start a new combat with the
// opposing minion (if both combatants are still ready)."
#[test]
fn pocket_out_of_time_starts_another_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    superior(&mut e, a, "TEM");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    hand(&mut e, 0, &["Pocket Out of Time"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 2, a));
    let req = request(&mut e);
    let again = req
        .options
        .iter()
        .copied()
        .find(|o| {
            matches!(
                o,
                Choice::UseAbility {
                    source: AbilitySource::Card(_),
                    ..
                }
            )
        })
        .expect("its burn for a new combat");
    pick(&mut e, again);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 6 - 2, "two combats, two hand strikes");
    assert!(
        ash_has(&e, 0, "Pocket Out of Time"),
        "burned as the action ends"
    );
}

// Arcanum Chapterhouse, Alexandria: "Each of your predator and prey burns 1
// pool during his or her unlock phase for each Hunting Ground he or she
// controls."
#[test]
fn arcanum_chapterhouse_taxes_the_neighbours_hunting_grounds() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 0, "Arcanum Chapterhouse, Alexandria", None);
    in_play(&mut e, 1, "Base Hunting Ground", None);
    in_play(&mut e, 0, "Base Hunting Ground", None);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), 10 - 1, "the prey's hunting ground");
    assert_eq!(pool(&e, 2), 10, "the predator has none");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 0), 10, "not its controller");
}

// Eye of Hazimel: "The bearer gets +1 bleed and +1 strength … superior
// Potence [POT] and superior Chimerstry [CHI], and combat cards cost this
// vampire 1 fewer blood. While the bearer is ready, this equipment cannot be
// moved or stolen. If this equipment is burned, it inflicts 5 unpreventable
// environmental damage on the bearer. During your unlock phase, burn 1 pool
// or burn this equipment."
#[test]
fn eye_of_hazimel_empowers_and_costs_its_bearer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let eye = in_play(&mut e, 0, "Eye of Hazimel", Some(a));
    e.state.minions[a.index()].blood = 3;
    assert_eq!(e.state.bearer_mod(a, Stat::Strength, ActionKind::Bleed), 1);
    assert_eq!(e.state.bearer_mod(a, Stat::Bleed, ActionKind::Bleed), 1);
    assert_eq!(e.state.discipline_level(a, "chi"), 2);
    assert!(e.state.pinned(eye));
    use zvtes::library::CardType::Combat;
    let off = e.state.printed_cost_off(a, requiring("pot", Combat), 0);
    assert_eq!(off.blood, 1);
    // The unlock phase: the pool declined, the Eye burns and hits for 5.
    let mut seen = false;
    next_turn_taking(&mut e, |o| {
        if *o == (Choice::BurnInPlay { card: eye.0 }) {
            seen = true;
            return true;
        }
        false
    });
    assert!(seen, "pool or the Eye");
    assert!(!e.state.cards[eye.index()].present());
    // 3 blood mend 3 of the 5: the bearer wounded.
    assert!(e.state.minions[a.index()].in_torpor);
    assert_eq!(blood(&e, a), 0);
}

// Malkavian Dementia: "Take control of a ready Malkavian that another
// Methuselah controls until your next unlock phase." Back as that phase
// begins, before unlocking [ANK 20200629-2] [LSJ 20091208] [RTR 19951017],
// as it is [LSJ 20010119].
#[test]
fn malkavian_dementia_borrows_a_malkavian_until_the_unlock_phase() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let malk = {
        let m = e.add_minion(SeatId(1), plain("Malkavian", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        m
    };
    hand(&mut e, 0, &["Malkavian Dementia"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master(0, 0, Target::Minion(malk)));
    request(&mut e);
    assert_eq!(e.state.minions[malk.index()].owner, SeatId(0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[malk.index()].owner,
        SeatId(0),
        "through the turn's end"
    );
    e.state.minions[malk.index()].locked = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[malk.index()];
    assert_eq!(v.owner, SeatId(1), "back at the unlock phase");
    assert!(v.locked, "not unlocked by the phase it returned in");
}

// Derange: "+1 stealth action. Ⓓ Put this card on a younger vampire. The
// attached vampire is considered the same clan and sect as this acting
// vampire. The attached vampire does not unlock as normal." Back to its own
// as the card goes [RTR 20201130] [ANK 20200415]; a Malkavian antitribu
// plays it, either clan doing [ANK 20190203].
#[test]
fn derange_makes_a_younger_vampire_malkavian() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Malkavian antitribu", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let young = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 1..=3, 0), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let clan = e.state.minions[young.index()].clan();
    hand(&mut e, 0, &["Derange"]);
    pick(&mut e, play_at(0, 0, a, Target::Minion(young)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[young.index()].clan(),
        Some("Malkavian antitribu")
    );
    assert_eq!(e.state.sect_of(young), Some(Sect::Sabbat));
    let d = find_library("Derange");
    let id = e
        .state
        .cards
        .iter()
        .position(|c| c.card == d && c.present())
        .expect("on it");
    assert!(!e.state.cards[id].effect_only);
    e.state.minions[young.index()].locked = true;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[young.index()].locked,
        "does not unlock as normal"
    );
    e.burn_card(CardId(id as u32));
    assert_eq!(e.state.minions[young.index()].clan(), clan);
    assert_eq!(e.state.sect_of(young), Some(Sect::Camarilla));
}

// Diabolic Lure: "[DAI] Only usable if this vampire blocks, before block
// resolution. Put 4 pact counters on the acting minion; if the number of
// pact counters equals or exceeds their capacity or cost, burn those
// counters to make them infernal, and the acting minion's controller burns
// 1 pool." The pool whether or not [LSJ 20100205-2].
#[test]
fn diabolic_lure_makes_the_blocked_minion_infernal() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 1..=4, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let c = e.add_minion(SeatId(1), find_card("Cybele"), 5, false);
    superior(&mut e, c, "DAI");
    hand(&mut e, 1, &["Diabolic Lure"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(c));
    pick_when(&mut e, play(0, 2, c));
    request(&mut e);
    assert!(e.state.is_infernal(a), "capacity 4 or less: infernal");
    assert_eq!(e.state.minions[a.index()].pact, 0, "the counters burned");
    assert_eq!(pool(&e, 0), 10 - 1);
}

// Descent into Darkness: "[obt] Put this card on this vampire, then put this
// vampire and any other cards and counters on them out of play. During your
// influence phase, this vampire can gain 2 blood and you can choose to burn
// this card to return the vampire to play, locked." Its blood and cards
// remembered [RTR 20000501] [LSJ 20060908].
#[test]
fn descent_into_darkness_hides_a_vampire_and_returns_it() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Lasombra", 0), 1, false);
    superior(&mut e, a, "obt");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vest = in_play(&mut e, 0, "Kevlar Vest", Some(a));
    hand(&mut e, 0, &["Descent into Darkness"]);
    pick(&mut e, play_at(0, 0, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].present(), "out of play");
    assert!(!e.state.cards[vest.index()].present(), "its cards too");
    let descent = find_library("Descent into Darkness");
    let id = e
        .state
        .cards
        .iter()
        .position(|c| c.card == descent && c.in_play)
        .expect("in play") as u32;
    assert!(e.state.cards[id as usize].present(), "this card remains");
    let use_on = |ai: u8| Choice::UseAbility {
        source: AbilitySource::Card(CardId(id)),
        ability: ai,
        actor: None,
        target: Target::None,
    };
    // The blood and the return, both in the same influence phase.
    pick_when(&mut e, use_on(0));
    let turn = e.state.turn;
    let req = request(&mut e);
    assert_eq!(e.state.turn, turn);
    assert!(req.options.contains(&use_on(1)), "the same influence phase");
    pick(&mut e, use_on(1));
    request(&mut e);
    let v = &e.state.minions[a.index()];
    assert!(v.present() && v.locked);
    assert_eq!(v.blood, 2, "1 paid of its 1, then 2");
    assert!(e.state.cards[vest.index()].present());
    assert!(ash_has(&e, 0, "Descent into Darkness"));
}

// Dual Form: "[PRO] This acting vampire's capacity is reduced by 1 until the
// end of the game. Put this card in play, locked. It becomes a non-unique
// vampire with the same clan, sect, capacity and Disciplines as the acting
// vampire. If either minion leaves the ready region, the other is burned."
// The capacity first [LSJ 20050116], for good [LSJ 20051103]; the
// Disciplines as it resolves, an added one too [ANK 20170226]; the set
// burned once [LSJ 20050117].
#[test]
fn dual_form_makes_a_linked_copy() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Gangrel", 0), 5, false);
    superior(&mut e, a, "PRO");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let cap = e.state.capacity_of(a).unwrap();
    hand(&mut e, 0, &["Dual Form"]);
    pick(&mut e, play_at(0, 1, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.capacity_of(a), Some(cap - 1));
    let copy = MinionId(e.state.minions.len() as u32 - 1);
    let c = &e.state.minions[copy.index()];
    assert!(c.present() && c.locked && c.is_vampire());
    assert_eq!(e.state.capacity_of(copy), Some(cap - 1));
    assert_eq!(c.clan(), e.state.minions[a.index()].clan());
    assert_eq!(
        e.state.discipline_level(copy, "pro"),
        2,
        "the superior added too"
    );
    // The copy burned (its card): the original goes with it.
    let card = e
        .state
        .cards
        .iter()
        .position(|k| k.body == Some(copy))
        .expect("its card");
    e.burn_card(CardId(card as u32));
    assert!(!e.state.minions[copy.index()].in_play);
    assert!(!e.state.minions[a.index()].in_play, "the other burned");
}

// Dual Form: "[pro] Unlock this vampire. During this minion phase, you may
// unlock this vampire again."
#[test]
fn dual_form_unlocks_twice_in_the_minion_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Gangrel", 0), 5, false);
    superior(&mut e, a, "pro");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Dual Form"]);
    pick(&mut e, play_at(0, 0, a, Target::None));
    pick_when(&mut e, bleed(a));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].locked);
    let again = req
        .options
        .iter()
        .copied()
        .find(|o| {
            matches!(
                o,
                Choice::UseAbility {
                    source: AbilitySource::Card(_),
                    ..
                }
            )
        })
        .expect("the unlock again");
    pick(&mut e, again);
    request(&mut e);
    assert!(!e.state.minions[a.index()].locked);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Dual Form"), "gone as the minion phase ends");
}

// Madness Network: "Unlocked Malkavians can take actions in any Methuselah's
// minion phase (follow normal sequencing rules)." As that Methuselah ends
// their minion phase (rules-qa 49.6); in torpor too, to leave it [RTR
// 19950509].
#[test]
fn madness_network_lets_a_malkavian_act_in_anothers_minion_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let malk = e.add_minion(SeatId(1), plain("Malkavian", 0), 3, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let net = in_play(&mut e, 1, "Madness Network", None);
    let _ = a;
    pick(&mut e, Choice::Pass);
    let act = Choice::UseAbility {
        source: AbilitySource::Card(net),
        ability: 0,
        actor: Some(malk),
        target: Target::None,
    };
    let req = offering(&mut e, act);
    assert_eq!(req.seat, SeatId(1));
    e.choose(index_of(&req, act));
    pick(&mut e, action(malk, ActionKind::LeaveTorpor));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[malk.index()].in_torpor,
        "left torpor in seat 0's turn"
    );
}

// Madness Network: "Any minion can burn this card as an action that any
// unlocked Malkavian (in addition to the normally eligible blockers) can
// attempt to block." A Malkavian of a Methuselah across the table may block.
#[test]
fn any_malkavian_may_block_the_madness_networks_burn() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let across = e.add_minion(SeatId(2), plain("Malkavian", 0), 4, false);
    e.add_minion(SeatId(3), plain("Ventrue", 1), 4, false);
    let net = in_play(&mut e, 1, "Madness Network", None);
    let burn = Choice::UseAbility {
        source: AbilitySource::Card(net),
        ability: 1,
        actor: Some(a),
        target: Target::None,
    };
    pick(&mut e, burn);
    pick_when(&mut e, block(across));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        e.state.cards[net.index()].present(),
        "blocked by the Malkavian"
    );
}

// Curse of Nitocris: "Choose a Methuselah and put this card in play under
// the control of the chosen Methuselah. The controller of this card burns 1
// pool during each of his or her unlock phases. Each time a new Methuselah
// gets the Edge, the current controller of this card chooses any
// Methuselah, and that chosen Methuselah takes control of this card." Only
// a Methuselah new to the Edge [LSJ 19971006].
#[test]
fn curse_of_nitocris_passes_with_the_edge() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Curse of Nitocris"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master(0, 0, Target::Seat(SeatId(1))));
    request(&mut e);
    let curse = find_library("Curse of Nitocris");
    let id = e
        .state
        .cards
        .iter()
        .position(|c| c.card == curse && c.present())
        .expect("in play");
    assert_eq!(e.state.cards[id].controller, SeatId(1));
    // Seat 0 bleeds and gets the Edge anew: seat 1 gives the curse back.
    pick_when(&mut e, bleed(a));
    pick_when(&mut e, Choice::ChooseSeat(SeatId(0)));
    request(&mut e);
    assert_eq!(e.state.cards[id].controller, SeatId(0));
    // Seat 0 bleeding again, the Edge already its: no move.
    let b = MinionId(1);
    pick_when(&mut e, bleed(b));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::ChooseSeat(SeatId(1)),
    );
    assert_eq!(e.state.cards[id].controller, SeatId(0));
    let (before0, before1) = (pool(&e, 0), pool(&e, 1));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 1), before1, "not seat 1's any more");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e, 0), before0 - 1, "its controller's unlock phase");
}

// ---- Ledger: the rulings of the round's cards --------------------------------

// Derange: "The clan and sect assigned to Derange is set when the card is
// played and does not change when it is moved." [LSJ 20070707] Moved, the
// first vampire back to its own [RTR 20201130] [ANK 20200415]; a title off
// the new sect lost meanwhile [RTR 20201130] [LSJ 20060904]; over a sect
// change [LSJ 20100811] [ANK 20190619].
#[test]
fn derange_keeps_its_clan_and_sect_as_it_moves() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Malkavian antitribu", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let not_malk = |nth| {
        textless(
            |c| c.sect == Sect::Camarilla && c.capacity <= 3 && !c.clan.starts_with("Malkavian"),
            nth,
        )
    };
    let young = e.add_minion(SeatId(1), not_malk(0), 1, false);
    let next = e.add_minion(SeatId(1), not_malk(1), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Derange"]);
    pick(&mut e, play_at(0, 0, a, Target::Minion(young)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let d = find_library("Derange");
    let id = e
        .state
        .cards
        .iter()
        .position(|c| c.card == d && c.present())
        .unwrap() as u32;
    // The sect made Anarch underneath: still Sabbat while deranged.
    e.state.minions[young.index()].sect_override = Some(Sect::Anarch);
    assert_eq!(e.state.sect_of(young), Some(Sect::Sabbat));
    let mv = Choice::UseAbility {
        source: AbilitySource::Card(CardId(id)),
        ability: 1,
        actor: Some(young),
        target: Target::Minion(next),
    };
    pick_when(&mut e, mv);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[id as usize].host, Some(next));
    assert_eq!(
        e.state.minions[next.index()].clan(),
        Some("Malkavian antitribu")
    );
    assert_eq!(e.state.sect_of(next), Some(Sect::Sabbat));
    assert_eq!(
        e.state.sect_of(young),
        Some(Sect::Anarch),
        "its own, the change kept"
    );
}

// Derange: "During their unlock phase, they can burn 1 blood to unlock." A
// disease counter still burns as it unlocks [ANK 20221210-2]; infernal, its
// pool unlock still there [LSJ 20050114].
#[test]
fn derange_unlocks_for_a_blood_under_disease_or_as_infernal() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Malkavian", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let young = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 1..=3, 0), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Derange"]);
    pick(&mut e, play_at(0, 0, a, Target::Minion(young)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let d = find_library("Derange");
    let id = e
        .state
        .cards
        .iter()
        .position(|c| c.card == d && c.present())
        .unwrap() as u32;
    e.state.minions[young.index()].locked = true;
    e.state.minions[young.index()].disease = true;
    e.state.minions[young.index()].made_infernal = true;
    let start = blood(&e, young);
    let unlock = Choice::UseAbility {
        source: AbilitySource::Card(CardId(id)),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let mut infernal = false;
    let mut used = 0;
    next_turn_taking(&mut e, |o| {
        infernal |= matches!(o, Choice::UnlockInfernal { minion } if *minion == young);
        if *o == unlock {
            used += 1;
            return used == 1;
        }
        false
    });
    assert!(infernal, "the infernal unlock still offered");
    assert_eq!(start - blood(&e, young), 2, "the blood, and the disease's");
}

// Descent into Darkness: "breaking any temporary control effects": the
// vampire and the card to its permanent controller [LSJ 20040526]; Daring
// the Dawn's damage after resolution not applied [LSJ 20040616].
#[test]
fn descent_into_darkness_breaks_temporary_control_and_skips_the_dawn() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let a = e.add_minion(SeatId(1), plain("Lasombra", 0), 4, false);
    superior(&mut e, a, "obt");
    superior(&mut e, a, "for");
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[a.index()].owner = SeatId(0);
    e.state.minions[a.index()].temp_from = Some(SeatId(1));
    hand(&mut e, 0, &["Descent into Darkness", "Daring the Dawn"]);
    pick(&mut e, play_at(0, 0, a, Target::None));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[a.index()];
    assert_eq!((v.owner, v.temp_from), (SeatId(1), None));
    assert!(v.out_of_play);
    assert_eq!(v.blood, 4 - 1, "its cost only: no dawn damage");
    let descent = find_library("Descent into Darkness");
    let c = e
        .state
        .cards
        .iter()
        .find(|c| c.card == descent && c.in_play)
        .unwrap();
    assert_eq!(c.controller, SeatId(1));
}

// Descent into Darkness: "All cards on the vampire come back unlocked." [LSJ
// 20060908]; cards targeting it resume as it returns [TOM 19960211] [LSJ
// 20010809-2] [LSJ 20010809-3] [LSJ 20060908]: The Rack's 2 blood skipped
// while it is out of play. An ally never comes back
// [LSJ 20060124].
#[test]
fn descent_into_darkness_returns_its_cards_unlocked() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Lasombra", 0), 3, false);
    superior(&mut e, a, "OBT");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let vest = in_play(&mut e, 0, "Kevlar Vest", Some(a));
    e.state.cards[vest.index()].locked = true;
    let rack = in_play(&mut e, 0, "Rack, The", None);
    e.state.cards[rack.index()].chosen = vec![a];
    hand(&mut e, 0, &["Descent into Darkness"]);
    pick(&mut e, play_at(0, 1, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let blood_out = blood(&e, a);
    let descent = find_library("Descent into Darkness");
    let id = e
        .state
        .cards
        .iter()
        .position(|c| c.card == descent && c.in_play)
        .unwrap() as u32;
    let use_on = |ai: u8| Choice::UseAbility {
        source: AbilitySource::Card(CardId(id)),
        ability: ai,
        actor: None,
        target: Target::None,
    };
    // The Rack in the unlock phase: nothing for it out of play; then back
    // unlocked ([OBT]) in the influence phase.
    pick_when(&mut e, use_on(2));
    request(&mut e);
    assert_eq!(blood(&e, a), blood_out, "no Rack blood out of play");
    let v = &e.state.minions[a.index()];
    assert!(v.present() && !v.locked);
    assert!(
        !e.state.cards[vest.index()].locked,
        "its card back unlocked"
    );
    // An ally under a Descent stand-in: never back.
    let ally = e.add_ally(SeatId(0), find_library("Political Ally"));
    let fake = in_play(&mut e, 0, "Descent into Darkness", Some(ally));
    e.state.minions[ally.index()].out_of_play = true;
    let req = request(&mut e);
    assert!(!req.options.iter().any(
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == fake)
    ));
}

// Diabolic Lure: "[DAI] The opponent burns pool whether or not the pact
// counters reach capacity." [LSJ 20100205-2] [ANK 20221007] A 5-capacity
// acting vampire keeps its 4 counters.
#[test]
fn diabolic_lure_burns_pool_short_of_capacity_too() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let c = e.add_minion(SeatId(1), find_card("Cybele"), 5, false);
    superior(&mut e, c, "DAI");
    let cap = e.state.capacity_of(a).unwrap();
    assert!(cap > 4);
    hand(&mut e, 1, &["Diabolic Lure"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(c));
    pick_when(&mut e, play(0, 2, c));
    request(&mut e);
    assert!(!e.state.is_infernal(a));
    assert_eq!(e.state.minions[a.index()].pact, 4);
    assert_eq!(pool(&e, 0), 10 - 1);
}

// Domain of Evernight: "[tem] Only applies to damage done in combat, not
// damage done after combat like Catatonic Fear." [LSJ 20020314] [PIB
// 20130319] Catatonic Fear's after-combat 1 on a 1-blood vampire: mended,
// not aggravated.
#[test]
fn domain_of_evernight_spares_the_damage_after_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    superior(&mut e, a, "tem");
    superior(&mut e, a, "PRE");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, false);
    hand(&mut e, 0, &["Domain of Evernight", "Catatonic Fear"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert!(v.ready(), "normal damage mended by its blood");
    assert_eq!(v.blood, 0);
}

// Dual Form: "[PRO] A vampire's capacity can never be reduced below one."
// [RTR 19990712] "Only the clan, sect, capacity and disciplines are copied.
// Titles and other traits like Infernal or Scarce are not." [LSJ 20050116]
// "If they are more than two Dual Forms of the same vampires, all the
// others are burned when one leaves the ready region." [LSJ 20050117]
#[test]
fn dual_form_copies_no_title_or_trait_and_burns_its_whole_set() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Appius Claudius Corvus"), 10, false);
    superior(&mut e, a, "PRO");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Dual Form", "Dual Form"]);
    pick(&mut e, play_at(0, 1, a, Target::None));
    // The same action card once a turn: the second copy next turn.
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, play_at(0, 1, a, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let copies: Vec<MinionId> = (0..e.state.minions.len())
        .map(|i| MinionId(i as u32))
        .filter(|&m| e.state.minions[m.index()].dual_of == Some(a))
        .collect();
    assert_eq!(copies.len(), 2);
    for &c in &copies {
        assert!(!e.state.is_black_hand(c) && !e.state.is_seraph(c));
        assert_eq!(e.state.title_of(c), Title::None);
    }
    let first = e
        .state
        .cards
        .iter()
        .position(|k| k.body == Some(copies[0]))
        .unwrap();
    e.burn_card(CardId(first as u32));
    assert!(copies.iter().all(|&c| !e.state.minions[c.index()].in_play));
    assert!(!e.state.minions[a.index()].in_play, "the whole set");
    // Never below 1: a 1-capacity vampire (it cannot pay the 2 blood
    // itself) cut further stays at 1.
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Iris Bennett"), 1, false);
    e.state.minions[b.index()].capacity_less = 2;
    assert_eq!(e.state.capacity_of(b), Some(1));
}

// Outside the Hourglass: "The damage is applied even if the combat ends
// prematurely." [ANK 20180104] [LSJ 19971110] Alpha Glint [ANI][FOR]
// ending the combat before range: the 2 still land.
#[test]
fn outside_the_hourglass_lands_as_the_combat_ends_before_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, a, "TEM");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Cybele"), 6, false);
    superior(&mut e, z, "ANI");
    superior(&mut e, z, "FOR");
    hand(&mut e, 0, &["Outside the Hourglass"]);
    hand(&mut e, 1, &["Alpha Glint"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 3, a));
    pick_when(&mut e, play(0, 1, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 6 - 1 - 2, "Alpha Glint's blood, then the 2");
}

// Pocket Out of Time: "[TEM] Cannot be used to start a new combat if there
// is already a pending combat queued." [RTR 20020501] "… if the opponent is
// not ready." [ANK 20220429]
#[test]
fn pocket_out_of_time_needs_no_queued_combat_and_a_ready_opponent() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    superior(&mut e, a, "TEM");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    hand(&mut e, 0, &["Pocket Out of Time"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 2, a));
    let req = request(&mut e);
    let again = req
        .options
        .iter()
        .copied()
        .find(|o| {
            matches!(
                o,
                Choice::UseAbility {
                    source: AbilitySource::Card(_),
                    ..
                }
            )
        })
        .expect("its burn");
    pick(&mut e, again);
    assert!(e.action().expect("action").queued_combat.is_some());
    // The queued one fought: offered again after it, after each combat.
    let req = request(&mut e);
    assert!(req.options.contains(&again));
    // The opponent no longer ready after a combat: none.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    superior(&mut e, a, "TEM");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 0, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Pocket Out of Time"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 2, a));
    assert!(e.state.minions[z.index()].in_torpor);
}

// Taking the Skin: Minion: "If the minion is burned because of a referendum
// or as a side-effect of the action, this does not count as the acting
// minion burning him (War Ghoul when recruited)." [RTR 19960124] [LSJ
// 20090922] [ANK 20181022] [ANK 20220916]
#[test]
fn taking_the_skin_ignores_a_war_ghouls_entry_burn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Tzimisce", 0), 5, false);
    superior(&mut e, a, "abo");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_ally(SeatId(0), find_library("Political Ally"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["War Ghoul", "Taking the Skin: Minion"]);
    pick(&mut e, play_at(0, 0, a, Target::None));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
}

// Helena (ADV): "[MERGED] Helena is immune to damage from demons." Immune:
// none inflicted [LSJ 20010627] [LSJ 20010629] [LSJ 20090304-1]; Herald of
// Topheth, a demon ally, inflicts none on her.
#[test]
fn merged_helena_takes_no_damage_from_demons() {
    let mut e = game(2);
    let adv = CRYPT
        .iter()
        .position(|c| c.name == "Helena" && c.adv)
        .expect("Helena (ADV)") as u16;
    let h = e.add_minion(SeatId(0), adv, 4, false);
    e.state.minions[h.index()].merged = Some(find_card("Helena"));
    let demon = e.add_ally(SeatId(1), find_library("Herald of Topheth"));
    let other = e.add_ally(SeatId(1), find_library("Political Ally"));
    assert_eq!(e.state.damage_reduction(h, demon), u8::MAX);
    assert_eq!(e.state.damage_reduction(h, other), 0);
}

// Tye Cooper: "He is immune to non-aggravated damage." Immune means damage
// is not inflicted, even if non-preventable [LSJ 20010627] [LSJ 20010629]
// [LSJ 20090304-1]: a vampire's hand strike leaves him whole.
#[test]
fn tye_cooper_takes_no_normal_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Ghede"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_ally(SeatId(1), find_library("Tye Cooper"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, printed(a, 0, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[t.index()];
    assert!(v.in_play && v.blood == 1, "his 1 life kept");
}

// Bolesław Gutowski: "The ability can be used when in torpor." [LSJ
// 20011022] His leave-torpor action.
#[test]
fn boleslaw_reveals_from_torpor() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bolesław Gutowski"), 3, true);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].crypt = vec![plain("Toreador", 0)].into();
    pick(&mut e, action(b, ActionKind::LeaveTorpor));
    use_vamp(&mut e, b, 0);
    request(&mut e);
    assert_eq!(e.action().expect("action").strength_bonus, 1);
}

// Mythic Form: "Cannot be used to prevent aggravated damage." [LSJ
// 20040812-2] [PIB 20130327] Against aggravated hand strikes, no
// prevention offered.
#[test]
fn mythic_form_never_prevents_aggravated_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 5, false);
    superior(&mut e, a, "pro");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    e.state.minions[z.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    hand(&mut e, 0, &["Mythic Form"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::UsePrevent,
    );
}

// Stunt Cycle: "[POT] If fewer points of (preventable) damage are being
// resolved, then the effect prevents all of those points." [RTR 20041202]
// "Unused prevention points can't be carried over." [ANK 20200318] [LSJ
// 20001114] Its 2 against a 1R strike: the 1 prevented, nothing left
// after.
#[test]
fn stunt_cycle_superior_prevents_what_there_is() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 6, false);
    superior(&mut e, a, "POT");
    superior(&mut e, a, "OBF");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    superior(&mut e, z, "pot");
    hand(&mut e, 0, &["Stunt Cycle", "Swallowed by the Night"]);
    hand(&mut e, 1, &["Thrown Gate"]);
    request(&mut e);
    let (sa, sz) = (blood(&e, a), blood(&e, z));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(1, 1, a));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, z));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), sa, "the 1 prevented");
    assert_eq!(blood(&e, z), sz - 4);
    assert!(e.combat().is_none(), "nothing carried");
}

// Edward Vignes: "Cannot be used while a block is being attempted, or has
// already been successful." [ANK 20260316] [LSJ 20041022] "If the action
// fails, the action has reached resolution, so is subject to NRA." [ANK
// 20211015] [RTR 20180719]
#[test]
fn edward_vignes_cannot_fail_a_blocked_action_and_a_failed_one_is_spent() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Ghede"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ed = e.add_minion(SeatId(1), find_card("Edward Vignes"), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Deflection"]);
    e.state.seats[1].library = vec![find_library("Blood Doll"); 9].into();
    pick(&mut e, printed(g, 0, Target::Minion(ed)));
    pick_when(&mut e, block(z));
    let req = request(&mut e);
    assert!(vamp_ability(&req, ed, 0).is_none(), "under a block attempt");
    // Failed, the rush is spent for the turn.
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Ghede"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ed = e.add_minion(SeatId(1), find_card("Edward Vignes"), 4, false);
    hand(&mut e, 1, &["Deflection"]);
    e.state.seats[1].library = vec![find_library("Blood Doll"); 9].into();
    pick(&mut e, printed(g, 0, Target::Minion(ed)));
    let fail = vamp_ability(&offering_vamp(&mut e, ed), ed, 0).expect("offered");
    pick(&mut e, fail);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[g.index()].locked = false;
    let req2 = request(&mut e);
    let _ = req;
    assert!(
        !req2.options.contains(&printed(g, 0, Target::Minion(ed))),
        "NRA"
    );
}

// Madness Network: "A non-Malkavian cannot use Mask of a Thousand Faces to
// assume the action of a Malkavian on someone else's turn." [LSJ 19981007]
// "If a Malkavian has no blood, he may declare an action during another
// Methuselah's turn but, if he does, that action must be a hunt action."
// [LSJ 20060508]
#[test]
fn madness_network_actions_take_no_mask_and_starve_to_a_hunt() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let malk = e.add_minion(SeatId(1), plain("Malkavian", 0), 0, false);
    let other = e.add_minion(SeatId(1), vampire_with("obf", 0), 4, false);
    let net = in_play(&mut e, 1, "Madness Network", None);
    hand(&mut e, 1, &["Mask of a Thousand Faces"]);
    pick(&mut e, Choice::Pass);
    let act = Choice::UseAbility {
        source: AbilitySource::Card(net),
        ability: 0,
        actor: Some(malk),
        target: Target::None,
    };
    pick_when(&mut e, act);
    let req = request(&mut e);
    let kinds: Vec<ActionKind> = req
        .options
        .iter()
        .filter_map(|o| match *o {
            Choice::TakeAction { action, .. } => Some(action),
            _ => None,
        })
        .collect();
    assert_eq!(kinds, vec![ActionKind::Hunt], "no blood: a hunt");
    pick(&mut e, action(malk, ActionKind::Hunt));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        play(0, 0, other),
    );
}

// Madness Network: "Any locked Malkavian on the table can use a wake effect
// to attempt to block the action to burn the Madness Network." [LSJ
// 20041022-2]
#[test]
fn a_woken_malkavian_may_block_the_madness_networks_burn() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let across = e.add_minion(SeatId(2), plain("Malkavian", 0), 4, false);
    superior(&mut e, across, "AUS");
    e.state.minions[across.index()].locked = true;
    e.add_minion(SeatId(3), plain("Ventrue", 1), 4, false);
    let net = in_play(&mut e, 1, "Madness Network", None);
    hand(&mut e, 2, &["Eyes of Argus"]);
    let burn = Choice::UseAbility {
        source: AbilitySource::Card(net),
        ability: 1,
        actor: Some(a),
        target: Target::None,
    };
    pick(&mut e, burn);
    pick_when(&mut e, play(0, 1, across));
    pick_when(&mut e, block(across));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[net.index()].present(), "blocked");
}

// Derange: "If the target changes clan or sect to a clan or sect
// inappropriate for their title, they lose its benefit until their clan or
// sect is appropriate." [RTR 20201130] [LSJ 20060904] A Camarilla prince
// deranged Sabbat is untitled meanwhile.
#[test]
fn derange_suspends_a_title_off_its_new_sect() {
    let mut e = game(2);
    let prince = e.add_minion(SeatId(1), find_card("Andi Liu"), 4, false);
    let title = e.state.title_of(prince);
    assert_ne!(title, Title::None);
    e.state.minions[prince.index()].deranged =
        Some(("Malkavian antitribu", Sect::Sabbat, CardId(0)));
    assert_eq!(e.state.title_of(prince), Title::None);
    e.state.minions[prince.index()].deranged = None;
    assert_eq!(e.state.title_of(prince), title);
}

// Taking the Skin: Minion after a diablerie whose blood hunt burned the
// diablerist: a minion out of play plays no card, "this vampire" gone.
#[test]
fn taking_the_skin_is_not_played_by_a_diablerist_the_hunt_burned() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), plain("Tzimisce", 0), 4, false);
    superior(&mut e, d, "abo");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let victim = e.add_minion(SeatId(1), plain("Brujah", 2), 2, true);
    let prince = e.add_minion(
        SeatId(1),
        textless(
            |c| matches!(c.title, Title::Prince(_)) && c.sect == Sect::Camarilla,
            0,
        ),
        4,
        false,
    );
    hand(&mut e, 0, &["Taking the Skin: Minion"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    pick(
        &mut e,
        Choice::CastVotes {
            minion: prince,
            favor: true,
        },
    );
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 2, d));
    assert!(
        !e.state.minions[d.index()].in_play,
        "burned by the blood hunt"
    );
}

// Forced March after a diablerie whose blood hunt burned the diablerist
// (the owner's game, 2026-10-16: Mkhokeli still offered it): the acting
// minion gone, no card after resolution.
#[test]
fn forced_march_is_not_played_by_a_diablerist_the_hunt_burned() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    superior(&mut e, d, "cel");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let victim = e.add_minion(SeatId(1), plain("Brujah", 2), 2, true);
    let prince = e.add_minion(
        SeatId(1),
        textless(
            |c| matches!(c.title, Title::Prince(_)) && c.sect == Sect::Camarilla,
            0,
        ),
        4,
        false,
    );
    hand(&mut e, 0, &["Forced March"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    pick(
        &mut e,
        Choice::CastVotes {
            minion: prince,
            favor: true,
        },
    );
    assert!(!offered_before(
        &mut e.clone(),
        play(0, 0, d),
        SeatId(0),
        WindowLabel::MinionPhase
    ));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, d));
    assert!(
        !e.state.minions[d.index()].in_play,
        "burned by the blood hunt"
    );
}

// Malkavian Dementia returning its Malkavian to an ousted Methuselah: the
// minion is removed [ANK 20181110], the taker's own card on it burned
// (owner, 2026-09-06), not removed with it.
#[test]
fn malkavian_dementia_back_to_an_ousted_methuselah_burns_the_takers_card() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let malk = e.add_minion(SeatId(1), plain("Malkavian", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Malkavian Dementia"]);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master(0, 0, Target::Minion(malk)));
    request(&mut e);
    assert_eq!(e.state.minions[malk.index()].owner, SeatId(0));
    let jacket = in_play(&mut e, 0, "Flak Jacket", Some(malk));
    e.state.seats[1].pool = 1;
    pick_when(&mut e, bleed(malk));
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[malk.index()].in_play,
        "removed, not returned"
    );
    assert!(!e.state.cards[jacket.index()].in_play);
    assert!(ash_has(&e, 0, "Flak Jacket"), "the taker's card burned");
}

// A hand over size waits for no pending choice but a draw's own (rulebook
// "Drawing Cards"): Flurry of Action's draw is discarded down before Curse
// of Nitocris passes with the Edge (rules-qa 78.51).
#[test]
fn a_discard_down_comes_before_curse_of_nitocris_passes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("cel", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 1, "Curse of Nitocris", None);
    // Distinct cards: a discard of identical ones is elided.
    hand(
        &mut e,
        0,
        &[
            "Flurry of Action",
            "Blood Doll",
            "Kevlar Vest",
            "Flak Jacket",
            ".44 Magnum",
            "Deflection",
            "Eyes of Argus",
        ],
    );
    e.state.seats[0].library = vec![find_library("Blood Doll"); 5].into();
    pick(&mut e, play_at(0, 0, a, Target::None));
    for _ in 0..50 {
        let req = request(&mut e);
        if req.seat == SeatId(1) && req.options.contains(&Choice::ChooseSeat(SeatId(0))) {
            panic!("the curse's pick before the discard down");
        }
        if req.seat == SeatId(0) && req.options.iter().all(|o| matches!(o, Choice::Discard(_))) {
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("no discard down");
}

// Seat targets name no ousted Methuselah (rules-qa 78.52).
#[test]
fn curse_of_nitocris_names_no_ousted_methuselah() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    e.state.seats[2].ousted = true;
    e.state.seats[2].pool = 0;
    hand(&mut e, 0, &["Curse of Nitocris"]);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(req.options.contains(&master(0, 0, Target::Seat(SeatId(1)))));
    assert!(!req.options.contains(&master(0, 0, Target::Seat(SeatId(2)))));
}

// Dual Form played by a copy: "all the others are burned … whatever chain
// they form" [LSJ 20050117]; the copy's capacity reduced too, never below
// one [RTR 19990712] [LSJ 20051103].
#[test]
fn dual_form_played_by_a_copy_joins_the_set() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Appius Claudius Corvus"), 10, false);
    superior(&mut e, a, "PRO");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Dual Form", "Dual Form"]);
    pick(&mut e, play_at(0, 1, a, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let c1 = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(e.state.minions[c1.index()].dual_of, Some(a));
    assert_eq!(e.state.capacity_of(c1), Some(9));
    e.state.minions[c1.index()].blood = 4;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick_when(&mut e, play_at(0, 1, c1, Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let c2 = MinionId(e.state.minions.len() as u32 - 1);
    assert_ne!(c2, c1);
    assert_eq!(
        e.state.minions[c2.index()].dual_of,
        Some(a),
        "the original's set"
    );
    assert_eq!(
        e.state.capacity_of(c1),
        Some(8),
        "the copy's own capacity reduced"
    );
    assert_eq!(e.state.capacity_of(c2), Some(8));
    let mid = e
        .state
        .cards
        .iter()
        .position(|k| k.body == Some(c1))
        .unwrap();
    e.burn_card(CardId(mid as u32));
    assert!(
        !e.state.minions[c2.index()].in_play,
        "the copy's copy burned"
    );
    assert!(!e.state.minions[a.index()].in_play, "the original burned");
}

// Eye of Hazimel: "While the bearer is ready, this equipment cannot be
// moved or stolen": Heidelberg Castle moves the bearer's other equipment,
// never the Eye; once the bearer is in torpor, the Eye too.
#[test]
fn eye_of_hazimel_is_not_moved_off_a_ready_bearer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let eye = in_play(&mut e, 0, "Eye of Hazimel", Some(a));
    let candle = in_play(&mut e, 0, "Flaming Candle", Some(a));
    let castle = in_play(&mut e, 0, "Heidelberg Castle, Germany", None);
    let heidelberg = Choice::UseAbility {
        source: AbilitySource::Card(castle),
        ability: 0,
        actor: None,
        target: Target::Minion(a),
    };
    pick(&mut e, heidelberg);
    pick(&mut e, set_x(0));
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::ChooseCard(candle.0)));
    assert!(
        !req.options.contains(&Choice::ChooseCard(eye.0)),
        "pinned: {req:?}"
    );
    // Its bearer in torpor, the Eye is no longer pinned.
    e.state.minions[a.index()].in_torpor = true;
    assert!(!e.state.pinned(eye));
}

// Descent into Darkness: "If any card on the vampire was contested, it goes
// out of the contest until the vampire comes back." [LSJ 20080816] Its
// title likewise: out of the contest while away, contested again on its
// return (rules-qa 78.54).
#[test]
fn descent_into_darkness_leaves_and_reenters_its_contests() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Lasombra", 0), 1, false);
    superior(&mut e, a, "obt");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let paris = Title::Prince("Paris");
    for m in [a, z] {
        e.state.minions[m.index()].granted_title = Some(paris);
        e.state.minions[m.index()].title_contested = true;
    }
    let mine = in_play(&mut e, 0, "Eye of Hazimel", Some(a));
    let theirs = in_play(&mut e, 1, "Eye of Hazimel", Some(z));
    for c in [mine, theirs] {
        e.state.cards[c.index()].contested = true;
    }
    hand(&mut e, 0, &["Descent into Darkness"]);
    pick(&mut e, play_at(0, 0, a, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].present(), "out of play");
    assert!(
        !e.state.minions[z.index()].title_contested,
        "the rival's title alone"
    );
    assert!(!e.state.minions[a.index()].title_contested);
    assert!(!e.state.cards[mine.index()].contested && !e.state.cards[theirs.index()].contested);
    let descent = find_library("Descent into Darkness");
    let id = e
        .state
        .cards
        .iter()
        .position(|c| c.card == descent && c.in_play)
        .expect("in play") as u32;
    let use_on = |ai: u8| Choice::UseAbility {
        source: AbilitySource::Card(CardId(id)),
        ability: ai,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, use_on(1));
    request(&mut e);
    assert!(e.state.minions[a.index()].present(), "back");
    assert!(
        e.state.minions[a.index()].title_contested && e.state.minions[z.index()].title_contested
    );
    assert!(e.state.cards[mine.index()].contested && e.state.cards[theirs.index()].contested);
}

// Outside the Hourglass [TEM] with Weather Control: their pre-range damage
// lands in one damage window (owner, 2026-10-17; rules-qa 78.32), Weather
// Control's unpreventable.
#[test]
fn outside_the_hourglass_and_weather_control_land_together() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    superior(&mut e, a, "TEM");
    superior(&mut e, a, "tha");
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 5, false);
    hand(&mut e, 0, &["Outside the Hourglass", "Weather Control"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(1, 0, a));
    pick_when(&mut e, play(0, 3, a));
    // The combat elides whole: each lands its hand strike too.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, z),
        5 - 2 - 1 - 1,
        "the Hourglass, the weather, a strike"
    );
    // Alone, the Hourglass leaves it 2 (the opposing strike): the weather
    // once more, not twice.
    assert_eq!(blood(&e, a), 2 - 1, "the weather once");
}

// Maris Streck: "Maris can burn a blood to give a blocking minion +1
// intercept." [LSJ 20020814] [ANK 20180307-2] "Can use her ability multiple
// times each action." [ANK 20220705] "Can use her ability while locked."
// No "during": as often as she pays (rules-qa 79.3). Locked, for her own
// Methuselah's blocker or another's, she burns twice against a +2 stealth
// bleed: blocked.
#[test]
fn maris_streck_burns_blood_for_intercept_as_often_as_she_pays_while_locked() {
    for maris_seat in [1, 2] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), vampire_with("obt", 0), 4, false);
        superior(&mut e, a, "obt");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let m = e.add_minion(SeatId(maris_seat), find_card("Maris Streck"), 4, false);
        e.state.minions[m.index()].locked = true;
        hand(&mut e, 0, &["Shadow Play"]);
        let burn = Choice::UseAbility {
            source: AbilitySource::Vampire(m),
            ability: 0,
            actor: None,
            target: Target::None,
        };
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(z));
        pick_when(&mut e, play(0, 1, a)); // +2 stealth
        pick_when(&mut e, burn);
        pick_when(&mut e, burn);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.last_outcome,
            Some(zvtes::actions::Outcome::Blocked),
            "seat {maris_seat}"
        );
        assert_eq!(blood(&e, m), 4 - 2);
    }
}

// Maris Streck: "She can take a +1 stealth action to allow you to look at
// and rearrange the top 5 cards of your library." At resolution, as
// Eulogio's; she stays locked.
#[test]
fn maris_streck_rearranges_the_top_five_of_her_library() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Maris Streck"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let names = [
        "Glancing Blow",
        "Morbidity",
        "Obedience",
        "Heartrender",
        "Abbot",
        "Deflection",
    ];
    e.state.seats[0].library = names.iter().map(|n| find_library(n)).collect();
    fill_hand(&mut e, 0);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(m),
            ability: 1,
            actor: Some(m),
            target: Target::None,
        },
    );
    // The top five, from the top down: Deflection is the top card.
    pick_when(&mut e, Choice::FindCard(find_library("Morbidity")));
    pick_when(&mut e, Choice::FindCard(find_library("Deflection")));
    pick_when(&mut e, Choice::FindCard(find_library("Abbot")));
    pick_when(&mut e, Choice::FindCard(find_library("Obedience")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let top: Vec<u16> = e.state.seats[0].library.iter().rev().copied().collect();
    let want: Vec<u16> = [
        "Morbidity",
        "Deflection",
        "Abbot",
        "Obedience",
        "Heartrender",
        "Glancing Blow",
    ]
    .iter()
    .map(|n| find_library(n))
    .collect();
    assert_eq!(top, want);
    assert!(e.state.minions[m.index()].locked, "she acted");
    assert_eq!(blood(&e, m), 4);
}

// The intercept reach bound (`block_can_reach`): a third party's Maris
// Streck with blood can give the prey's blocker the +1 intercept a hunt's
// stealth needs (rules-qa 79.3); with none, the block is out of reach.
#[test]
fn a_third_partys_maris_streck_keeps_a_block_in_reach() {
    for maris_blood in [0, 2] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(2), find_card("Maris Streck"), maris_blood, false);
        pick(&mut e, action(a, ActionKind::Hunt));
        offering(&mut e, Choice::DeclareBlock { minion: z });
        assert_eq!(
            e.block_can_reach(SeatId(1), z),
            maris_blood > 0,
            "{maris_blood}"
        );
    }
}

// The reach bound: an unlocked Marciana Giovanni can still grant the
// blocker its burn (rules-qa 79.2), and the burn granted counts until it is
// used; a locked Marciana grants nothing.
#[test]
fn marcianas_grant_keeps_a_block_in_reach() {
    for locked in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let m = e.add_minion(
            SeatId(1),
            find_card("Marciana Giovanni, Investigator"),
            2,
            false,
        );
        e.state.minions[m.index()].locked = locked;
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        pick(&mut e, action(a, ActionKind::Hunt));
        offering(&mut e, Choice::DeclareBlock { minion: z });
        assert_eq!(e.block_can_reach(SeatId(1), z), !locked);
        if !locked {
            pick(
                &mut e,
                Choice::UseAbility {
                    source: AbilitySource::Vampire(m),
                    ability: 0,
                    actor: None,
                    target: Target::Minion(z),
                },
            );
            offering(&mut e, Choice::DeclareBlock { minion: z });
            assert!(e.state.minions[m.index()].locked);
            assert!(e.block_can_reach(SeatId(1), z), "the burn granted");
        }
    }
}

// The reach bound: a third party's unlocked The Rumor Mill can choose the
// prey's blocker to burn for intercept (ANK 20200607), keeping a hunt's
// block in reach; locked, it gives nothing.
#[test]
fn a_third_partys_rumor_mill_keeps_a_block_in_reach() {
    for locked in [false, true] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
        let mill = in_play(&mut e, 2, "Rumor Mill, Tabloid Newspaper, The", None);
        e.state.cards[mill.index()].locked = locked;
        pick(&mut e, action(a, ActionKind::Hunt));
        offering(&mut e, Choice::DeclareBlock { minion: z });
        assert_eq!(e.block_can_reach(SeatId(1), z), !locked);
    }
}
