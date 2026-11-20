//! Round 32 (`ADDED_BATCHES`): Legacies of Blood completed, its action cards
//! and vampires (Iniko to Olugbenga). Windows with a single option are elided,
//! so the scripts only stop where a real decision exists.

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay, UncontrolledSlot};

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

/// The `nth` untitled fixture vampire matching `f` (`fixture_vampire`).
#[track_caller]
fn textless(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    fixture_vampire(|c| f(c) && c.title == Title::None, nth)
}

/// The `nth` textless vampire of `clan` with a capacity in `caps`.
#[track_caller]
fn of_clan(clan: &str, caps: std::ops::RangeInclusive<u8>, nth: usize) -> u16 {
    textless(|c| c.clan == clan && caps.contains(&c.capacity), nth)
}

/// The `nth` textless vampire of `sect` with a capacity in `caps`.
#[track_caller]
fn of_sect(sect: Sect, caps: std::ops::RangeInclusive<u8>, nth: usize) -> u16 {
    textless(|c| c.sect == Some(sect) && caps.contains(&c.capacity), nth)
}

/// The card `name` entered play (itself, not a stand-in) on `host`, out of
/// every ash heap: its id.
#[track_caller]
fn placed_on(e: &Engine, name: &str, host: Option<MinionId>) -> CardId {
    let card = find_library(name);
    assert!(
        e.state.seats.iter().all(|s| !s.ash.contains(&card)),
        "{name} in an ash heap"
    );
    let i = e
        .state
        .cards
        .iter()
        .position(|c| c.card == card && c.in_play && !c.effect_only && c.host == host)
        .unwrap_or_else(|| panic!("{name} not in play on {host:?}"));
    CardId(i as u32)
}

fn ash_has(e: &Engine, seat: usize, name: &str) -> bool {
    e.state.seats[seat].ash.contains(&find_library(name))
}

/// The card in hand slot `card` played by `actor` in `mode` on `target`.
fn act(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

/// The ability `ability` of the card in play `source`, used by `actor` on
/// `target`.
fn use_on(source: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(source),
        ability,
        actor,
        target,
    }
}

/// An uncontrolled vampire `card` of `seat` with `counters`: its slot.
fn uncontrolled(e: &mut Engine, seat: usize, card: u16, counters: u8) -> u8 {
    let region = &mut e.state.seats[seat].uncontrolled;
    region.push(UncontrolledSlot {
        card,
        copy: Default::default(),
        counters,
        banished: None,
    });
    (region.len() - 1) as u8
}

// ---- Cards on the acting minion -------------------------------------------------

/// Enforcer: "+1 stealth action. Put this card on the acting Osebo. This
/// Osebo gets +1 strength. A minion can have only one Enforcer." The card
/// itself on the actor, +1 strength; a second not offered to its bearer
/// (rules-qa 99.1).
#[test]
fn enforcer_gives_its_osebo_strength() {
    let mut e = game(2);
    let o = e.add_minion(SeatId(0), of_clan("Osebo", 4..=7, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Enforcer", "Enforcer"]);
    assert_eq!(e.state.bearer_mod(o, Stat::Strength, ActionKind::Bleed), 0);
    pick(&mut e, play(0, 0, o));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Enforcer", Some(o));
    assert_eq!(e.state.bearer_mod(o, Stat::Strength, ActionKind::Bleed), 1);
    e.state.minions[o.index()].locked = false;
    let req = if req.options.contains(&play(0, 0, o)) {
        req
    } else {
        request(&mut e)
    };
    assert!(!req.options.contains(&play(0, 0, o)), "only one");
}

/// Belonging Grants Protection: "+1 stealth action. Requires a ready
/// magaji. Unlock another ready Laibon or move 2 blood from the blood bank
/// to a younger Laibon in your uncontrolled region." A magaji unlocks
/// another Methuselah's locked Laibon (directed at them), or feeds a
/// younger uncontrolled Laibon 2 blood; an untitled Laibon does not play
/// it.
#[test]
fn belonging_grants_protection_unlocks_another_laibon_or_feeds_an_uncontrolled_one() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 6..=8, 0), 4, false);
    e.state.minions[m.index()].granted_title = Some(Title::Magaji);
    let u = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 6..=8, 1), 4, false);
    let l = e.add_minion(SeatId(1), of_sect(Sect::Laibon, 4..=8, 2), 4, false);
    e.state.minions[l.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let young = of_sect(Sect::Laibon, 3..=5, 0);
    let slot = uncontrolled(&mut e, 0, young, 1);
    hand(
        &mut e,
        0,
        &["Belonging Grants Protection", "Belonging Grants Protection"],
    );
    let req = request(&mut e);
    assert!(
        !req.options.contains(&act(0, 0, u, Target::Minion(l))),
        "no magaji"
    );
    pick(&mut e, act(0, 0, m, Target::Minion(l)));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[l.index()].locked, "unlocked");
    // Another magaji: the same action card once a turn each (NRA).
    e.state.minions[u.index()].granted_title = Some(Title::Magaji);
    pick(&mut e, act(0, 1, u, Target::Uncontrolled(slot)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled[slot as usize].counters, 3);
}

/// Legend of the Leopard: "+1 stealth action. Ⓓ Inflict 1 damage on a
/// vampire controlled by your predator or prey or take control of an ally
/// controlled by your predator or prey." [TOM 19960226-1] "Requirements do
/// not apply when taking control of a card already in play": the prey's
/// vampire burns 1 blood; the prey's Screamer (requiring a Malkavian) comes
/// to an Osebo's Methuselah.
#[test]
fn legend_of_the_leopard_wounds_or_takes_an_ally_whatever_its_requirements() {
    let mut e = game(3);
    let o = e.add_minion(SeatId(0), of_clan("Osebo", 4..=7, 0), 4, false);
    let o2 = e.add_minion(SeatId(0), of_clan("Osebo", 4..=7, 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let s = e.add_ally(SeatId(1), find_library("Screamer"));
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Legend of the Leopard", "Legend of the Leopard"],
    );
    pick(&mut e, act(0, 0, o, Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 3);
    assert_eq!(blood(&e, o), 3, "1 blood paid");
    pick(&mut e, act(0, 1, o2, Target::Minion(s)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[s.index()].owner, SeatId(0));
}

/// Healing Touch: "+1 stealth action. [cel] Unlock a younger vampire you
/// control. [obe] Add up to 2 life to an ally, not to exceed their starting
/// life. [OBE] Rescue a vampire from torpor. The rescued vampire gains 1
/// blood." [cel] unlocks the younger vampire; [obe] heals an ally to its
/// starting life, no more.
#[test]
fn healing_touch_unlocks_or_heals() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["cel", "obe"];
    let y = e.add_minion(SeatId(0), textless(|c| c.capacity <= 3, 0), 1, false);
    e.state.minions[y.index()].locked = true;
    let ally = e.add_ally(SeatId(0), find_library("Screamer"));
    let start = blood(&e, ally);
    e.state.minions[ally.index()].blood = start - 1;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Healing Touch", "Healing Touch"]);
    pick(&mut e, act(0, 0, a, Target::Minion(y)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[y.index()].locked);
    let h = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[h.index()].gained_levels = vec!["obe"];
    pick(&mut e, act(0, 1, h, Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ally), start, "not above its starting life");
}

/// Healing Touch [OBE]: [ANK 20181017] "Is not the default action, no blood
/// is paid for the rescue": the card's 1 blood only; the rescued vampire
/// leaves torpor with 1 blood more.
#[test]
fn healing_touch_superior_rescues_without_paying_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Healing Touch"]);
    pick(&mut e, act(0, 2, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_torpor);
    assert_eq!(blood(&e, t), 2);
    assert_eq!(blood(&e, a), 3, "the card's 1 blood, no rescue cost");
}

/// Healing Touch [OBE]: [ANK 20210124] [LSJ 20070411] [LSJ 20090514] "If
/// the action is unblocked when it resolves and the target is no longer in
/// torpor, the action 'fizzles'": the vampire, rescued meanwhile, gains no
/// blood; its cost not paid (rules-digest §10.2, rules-qa 117.3).
#[test]
fn healing_touch_superior_fizzles_off_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Healing Touch"]);
    pick(&mut e, act(0, 2, a, Target::Minion(t)));
    request(&mut e);
    e.state.minions[t.index()].in_torpor = false;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, t), 1, "no blood");
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(blood(&e, a), 4, "not paid");
}

/// Fire on the Mountain: "+1 stealth action. Requires a ready Laibon. Ⓓ
/// Diablerize a locked vampire in torpor. In the resulting blood hunt
/// referendum, this vampire gets 2 additional votes." The Laibon
/// diablerizes the prey's locked torpid vampire; the blood hunt counts its
/// 2 more votes; a non-Laibon does not play it.
#[test]
fn fire_on_the_mountain_diablerizes_with_two_more_votes() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 5..=8, 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
    e.state.minions[t.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Fire on the Mountain"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&act(0, 0, b, Target::Minion(t))),
        "no Laibon"
    );
    pick(&mut e, act(0, 0, l, Target::Minion(t)));
    for _ in 0..20 {
        if e.referendum().is_some() || !e.state.minions[t.index()].in_play {
            break;
        }
        let req = request(&mut e);
        e.choose(fallback(&req));
    }
    assert!(!e.state.minions[t.index()].in_play, "diablerized");
    request(&mut e);
    let r = e.referendum().expect("the blood hunt");
    assert!(r.bonus.contains(&(l, 2)), "{:?}", r.bonus);
}

/// Fire on the Mountain: [ANK 20210124] [LSJ 20070411] [LSJ 20090514] the
/// action fizzles if, unblocked as it resolves, its target is unlocked, or
/// no longer in torpor: no diablerie.
#[test]
fn fire_on_the_mountain_fizzles_on_an_unlocked_or_rescued_target() {
    for rescued in [false, true] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 5..=8, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 2, true);
        e.state.minions[t.index()].locked = true;
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Fire on the Mountain"]);
        pick(&mut e, act(0, 0, l, Target::Minion(t)));
        request(&mut e);
        if rescued {
            e.state.minions[t.index()].in_torpor = false;
        } else {
            e.state.minions[t.index()].locked = false;
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[t.index()].in_play, "not diablerized");
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    }
}

/// Clotho's Gift [obf]: "Draw 1 card from your crypt." [LSJ 20060620] "You
/// can draw more cards than are in the library or crypt (even from an empty
/// library or crypt); draw as much as you can": the crypt's card to the
/// uncontrolled region; with the crypt empty, still played, drawing nothing.
#[test]
fn clothos_gift_draws_from_the_crypt_even_empty() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    let b = e.add_minion(SeatId(0), vampire_with("obf", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let top = plain("Ventrue", 3);
    e.state.seats[0].crypt.put(top, Default::default());
    hand(&mut e, 0, &["Clotho's Gift", "Clotho's Gift"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.seats[0].uncontrolled.last().map(|u| u.card),
        Some(top)
    );
    let n = e.state.seats[0].uncontrolled.len();
    assert!(e.state.seats[0].crypt.is_empty());
    pick(&mut e, play(0, 0, b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].uncontrolled.len(), n, "nothing drawn");
    assert!(ash_has(&e, 0, "Clotho's Gift"));
}

/// Clotho's Gift [TEM]: "Put this card on this vampire, locked. During your
/// minion phase, this vampire can lock this card and burn 1 blood to
/// unlock." The card on its vampire (not a stand-in), locked; at its next
/// minion phase the locked vampire locks it for 1 blood and unlocks; not
/// offered again with the card locked.
#[test]
fn clothos_gift_superior_lets_its_vampire_unlock_for_a_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tem", "tem"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Clotho's Gift"]);
    pick(&mut e, play(0, 2, a));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = placed_on(&e, "Clotho's Gift", Some(a));
    assert!(e.state.cards[c.index()].locked, "locked");
    assert!(uses(&req, c.0).is_empty(), "the card locked");
    e.state.cards[c.index()].locked = false;
    let req = request(&mut e);
    let u = uses(&req, c.0);
    assert_eq!(u.len(), 1, "{:?}", req.options);
    pick(&mut e, u[0]);
    request(&mut e);
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    assert_eq!(blood(&e, a), 2, "1 for the card, 1 to unlock");
    assert!(e.state.cards[c.index()].locked);
}

/// Choir [mel]: "Put this card in play. Burn this card during your influence
/// phase." In play (itself) through the minion phase; in the influence
/// phase a due its controller orders before passing (rules-qa 28.9), then
/// in the ash heap.
#[test]
fn choir_stays_until_the_influence_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["mel"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    // A transfer to make: the influence phase is a decision.
    uncontrolled(&mut e, 0, plain("Ventrue", 3), 0);
    hand(&mut e, 0, &["Choir"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = placed_on(&e, "Choir", None);
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert!(!req.options.contains(&Choice::Pass), "a due first");
    pick(&mut e, Choice::ResolveRider { card: c.0 });
    assert!(!e.state.cards[c.index()].in_play);
    assert!(ash_has(&e, 0, "Choir"));
}

/// Choir [MEL]: "Ⓓ Your prey burns 2 pool for each copy of this card you
/// control (not counting this one). Burn all those copies." Two copies in
/// play: the prey burns 4, both to the ash heap; the other Methuselah's
/// copy counts for nothing and stays.
#[test]
fn choir_superior_burns_two_per_choir_and_the_choirs() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["mel", "mel"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    let c1 = in_play(&mut e, 0, "Choir", None);
    let c2 = in_play(&mut e, 0, "Choir", None);
    let other = in_play(&mut e, 2, "Choir", None);
    hand(&mut e, 0, &["Choir"]);
    pick(&mut e, act(0, 1, a, Target::Seat(SeatId(1))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 6, 10]);
    assert!(!e.state.cards[c1.index()].in_play && !e.state.cards[c2.index()].in_play);
    assert!(e.state.cards[other.index()].in_play);
}

/// Choir [MEL], its prey ousted before it resolves: "your prey" read as it
/// resolves: the new prey burns 4 and both Choirs burn (owner, rules-qa
/// 124.42).
#[test]
fn choir_burns_the_new_prey_once_its_prey_is_ousted() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["mel", "mel"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 0), 4, false);
    let c1 = in_play(&mut e, 0, "Choir", None);
    let c2 = in_play(&mut e, 0, "Choir", None);
    hand(&mut e, 0, &["Choir"]);
    pick(&mut e, act(0, 1, a, Target::Seat(SeatId(1))));
    request(&mut e);
    e.state.seats[1].pool = 0;
    e.state.seats[1].ousted = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[2].pool, 6);
    assert!(!e.state.cards[c1.index()].in_play && !e.state.cards[c2.index()].in_play);
}

/// Harmony: "Your prey burns 3 pool for each Choir you control" (rules-qa
/// 113.94): with two Choirs, 6 pool, the Choirs burned.
#[test]
fn harmony_burns_three_per_choir() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[d.index()].clan_change = Some("Daughter of Cacophony");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c1 = in_play(&mut e, 0, "Choir", None);
    let c2 = in_play(&mut e, 0, "Choir", None);
    hand(&mut e, 0, &["Harmony"]);
    pick(&mut e, act(0, 0, d, Target::Seat(SeatId(1))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 4]);
    assert!(!e.state.cards[c1.index()].in_play && !e.state.cards[c2.index()].in_play);
}

/// Hanging Fermata: "Out-of-turn. Only usable during your influence phase.
/// Not usable if you played a Hanging Fermata during your last turn. No
/// cards named Choir are burned during this influence phase." Played in
/// its own influence phase, before the Choirs' dues: no Choir burns, the
/// phase passes with both in play; counted against its player's next
/// master phase (advanced rules §6.6.2).
#[test]
fn hanging_fermata_spares_the_choirs_this_influence_phase() {
    let mut e = game(2);
    let doc = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[doc.index()].clan_change = Some("Daughter of Cacophony");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c1 = in_play(&mut e, 0, "Choir", None);
    let c2 = in_play(&mut e, 0, "Choir", None);
    uncontrolled(&mut e, 0, plain("Ventrue", 3), 0);
    hand(&mut e, 0, &["Hanging Fermata"]);
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert!(req.options.contains(&Choice::ResolveRider { card: c1.0 }));
    pick(&mut e, master_play(0));
    let req = expect(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert!(req.options.contains(&Choice::Pass), "nothing due");
    assert!(!req.options.contains(&Choice::ResolveRider { card: c1.0 }));
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, SeatId(1));
    assert!(e.state.cards[c1.index()].in_play && e.state.cards[c2.index()].in_play);
    assert!(e.state.seats[0].out_of_turn.is_some());
}

/// Hanging Fermata: not usable if its player played one during their last
/// turn; the turn after, usable again.
#[test]
fn hanging_fermata_not_two_turns_running() {
    for (played, offered) in [(2, false), (1, true)] {
        let mut e = game(2);
        let doc = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
        e.state.minions[doc.index()].clan_change = Some("Daughter of Cacophony");
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        in_play(&mut e, 0, "Choir", None);
        uncontrolled(&mut e, 0, plain("Ventrue", 3), 0);
        e.state.seats[0].previous_turn = 2;
        e.state.seats[0]
            .played_at
            .0
            .push((find_library("Hanging Fermata"), played));
        hand(&mut e, 0, &["Hanging Fermata"]);
        let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        assert_eq!(req.options.contains(&master_play(0)), offered, "{played}");
    }
}

/// Masai Blood Milk: "+1 stealth action. Requires a ready Laibon. Unique.
/// Put this card on the acting Laibon. This Laibon may burn this card as a
/// +1 stealth action to get +1 strength for the remainder of the game. Burn
/// this card during the discard phase of your next turn." The card on its
/// Laibon (itself), kept through this turn's discard phase, burned in the
/// next one's; the granted burn action: the card burned, +1 strength that
/// outlasts it (D33).
#[test]
fn masai_blood_milk_burns_for_lasting_strength_and_expires() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 4..=8, 0), 4, false);
    let k = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 4..=8, 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Masai Blood Milk"]);
    pick(&mut e, play(0, 0, l));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = placed_on(&e, "Masai Blood Milk", Some(l));
    assert!(!req.options.contains(&granted(c.0, l)), "its Laibon locked");
    // This turn's discard phase: kept.
    to_minion_phase(&mut e, SeatId(1));
    assert!(e.state.cards[c.index()].in_play);
    // The next: burned.
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.cards[c.index()].in_play);
    to_minion_phase(&mut e, SeatId(1));
    assert!(!e.state.cards[c.index()].in_play);
    assert!(ash_has(&e, 0, "Masai Blood Milk"));
    // The granted action.
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = in_play(&mut e, 0, "Masai Blood Milk", Some(k));
    let k = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 4..=8, 1), 4, false);
    e.state.cards[c.index()].host = Some(k);
    pick(&mut e, granted(c.0, k));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[c.index()].in_play);
    assert_eq!(e.state.lasting(k).1, 1);
    e.at_next_turn();
    e.at_next_turn();
    assert_eq!(e.state.lasting(k).1, 1, "for the remainder of the game");
}

/// Masai Blood Milk: [LSJ 20080604] "The action provided by this card is
/// considered to be an action requiring the listed clan/sect/capacity/
/// title": its burn action is not offered to a non-Laibon bearer.
#[test]
fn masai_blood_milk_action_needs_a_laibon_bearer() {
    for laibon in [true, false] {
        let mut e = game(2);
        let b = if laibon {
            e.add_minion(SeatId(0), of_sect(Sect::Laibon, 4..=8, 0), 4, false)
        } else {
            e.add_minion(SeatId(0), plain("Brujah", 0), 4, false)
        };
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let c = in_play(&mut e, 0, "Masai Blood Milk", Some(b));
        let req = request(&mut e);
        assert_eq!(req.options.contains(&granted(c.0, b)), laibon);
    }
}

/// Concert Tour: "+1 stealth action. Put this card on this Daughter of
/// Cacophony. During your unlock phase, burn this card to have this vampire
/// gain enough blood to reach full capacity." On its vampire (itself); its
/// controller's next unlock phase fills the vampire and burns the card.
#[test]
fn concert_tour_fills_its_daughter_in_the_next_unlock_phase() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), plain("Nosferatu", 0), 3, false);
    e.state.minions[d.index()].clan_change = Some("Daughter of Cacophony");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Concert Tour"]);
    pick(&mut e, play(0, 0, d));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = placed_on(&e, "Concert Tour", Some(d));
    assert_eq!(blood(&e, d), 2);
    to_minion_phase(&mut e, SeatId(1));
    assert!(
        e.state.cards[c.index()].in_play,
        "not in another's unlock phase"
    );
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(Some(blood(&e, d)), e.state.capacity_of(d));
    assert!(!e.state.cards[c.index()].in_play);
    assert!(ash_has(&e, 0, "Concert Tour"));
}

/// Earth-Feeder: "+2 stealth hunt action. Requires a ready Laibon. Hunt. If
/// the hunt is successful, put this card on this acting Laibon. This Laibon
/// gets +1 stealth when hunting. A vampire can have only one Earth-Feeder."
/// The hunt's blood; the card on the hunter (itself), +1 stealth on its
/// hunts; at capacity the hunt is still successful (rules-digest §11.2,
/// advanced rules §3.7.2.1), the card put on; a second played by its bearer
/// hunts, not put on (rules-qa 99.2).
#[test]
fn earth_feeder_hunts_and_stays_on_a_successful_hunter() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 5..=8, 0), 2, false);
    let full = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 4..=4, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Earth-Feeder", "Earth-Feeder", "Earth-Feeder"]);
    pick(&mut e, play(0, 0, l));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, l), 3);
    placed_on(&e, "Earth-Feeder", Some(l));
    assert_eq!(e.state.bearer_mod(l, Stat::Stealth, ActionKind::Hunt), 1);
    assert_eq!(e.state.bearer_mod(l, Stat::Stealth, ActionKind::Bleed), 0);
    pick(&mut e, play(0, 0, full));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!ash_has(&e, 0, "Earth-Feeder"), "successful at capacity");
    assert_eq!(e.state.bearer_mod(full, Stat::Stealth, ActionKind::Hunt), 1);
    // Its bearer, a turn later: hunts again, the second card not put on.
    e.at_next_turn();
    e.at_next_turn();
    e.state.minions[l.index()].locked = false;
    let before = blood(&e, l);
    pick_when(&mut e, play(0, 0, l));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, l), before + 1);
    let feeder = find_library("Earth-Feeder");
    let on = e
        .state
        .cards
        .iter()
        .filter(|c| c.card == feeder && c.present() && c.host == Some(l))
        .count();
    assert_eq!(on, 1, "only one");
}

/// Earth-Feeder: [RBK action-card-or-card-in-play] "A minion can only
/// perform this special hunt once per turn."
#[test]
fn earth_feeder_is_a_hunt_once_each_turn() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 5..=8, 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Earth-Feeder", "Earth-Feeder"]);
    pick(&mut e, play(0, 0, l));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, l)), "locked");
    e.state.minions[l.index()].locked = false;
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, l)), "once each turn");
}

// ---- Frozen Object, Kiss of Lachesis ------------------------------------------

/// Frozen Object [ser]: "Ⓓ Look at a Methuselah's hand and discard all
/// equipment cards from it." The prey's equipment discarded, its other
/// cards kept.
#[test]
fn frozen_object_discards_the_equipment_of_a_hand() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Frozen Object"]);
    hand(&mut e, 1, &[".44 Magnum", "Deer Rifle", "Screamer"]);
    pick(&mut e, act(0, 0, a, Target::Seat(SeatId(1))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, ".44 Magnum") && ash_has(&e, 1, "Deer Rifle"));
    assert!(e.state.seats[1].hand.contains(&find_library("Screamer")));
}

/// Frozen Object [tem]: "Put this card on this vampire. If this vampire
/// blocks a directed action, you can burn this card to inflict 2
/// unpreventable environmental damage on the acting minion before block
/// resolution." [RBK resolve-the-action] "If an acting minion leaves the
/// ready region before block resolution, the action ends immediately and
/// the blocking minion is not locked": the acting ally burned, no combat,
/// the blocker unlocked.
#[test]
fn frozen_object_burns_an_acting_ally_before_block_resolution() {
    let mut e = game(2);
    let ally = e.add_ally(SeatId(0), find_library("Screamer"));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let fo = in_play_mode(&mut e, 1, "Frozen Object", Some(z), 1);
    pick(&mut e, bleed(ally));
    pick(&mut e, block(z));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlocked);
    let u = uses(&req, fo.0);
    assert_eq!(u.len(), 1, "{:?}", req.options);
    pick(&mut e, u[0]);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    assert!(!e.state.minions[z.index()].locked, "the blocker not locked");
    assert!(!e.state.cards[fo.index()].in_play);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
}

/// Frozen Object [TEM]: "Put this card in play. You can burn this card
/// after a minion successfully bleeds you to inflict 2 unpreventable
/// environmental damage on them." After the bleed: the bleeder burns 2
/// blood, the card burned. "A Methuselah can have only one Frozen Object":
/// with one in play, its [tem] and [TEM] are not offered, its [ser] is.
#[test]
fn frozen_object_superior_damages_a_successful_bleeder() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let fo = in_play_mode(&mut e, 1, "Frozen Object", None, 2);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, use_on(fo, 1, None, Target::None));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    pick(&mut e, use_on(fo, 1, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 2);
    assert!(!e.state.cards[fo.index()].in_play);
}

/// Frozen Object: "A Methuselah can have only one Frozen Object": its
/// player controlling one, the modes putting another in play are not
/// offered; the [ser] mode, which puts none, is.
#[test]
fn frozen_object_is_one_per_methuselah() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tem", "tem", "ser"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Frozen Object"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 1, a)) && req.options.contains(&play(0, 2, a)));
    in_play_mode(&mut e, 0, "Frozen Object", None, 2);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 1, a)) && !req.options.contains(&play(0, 2, a)));
    assert!(req.options.contains(&act(0, 0, a, Target::Seat(SeatId(1)))));
}

/// Kiss of Lachesis [ser]: "Ⓓ Burn a retainer or non-unique equipment."
/// [tem]: "Ⓓ Burn an ally, retainer or non-unique equipment." The prey's
/// retainer and gun burned; an ally only at [tem].
#[test]
fn kiss_of_lachesis_burns_a_retainer_equipment_or_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("ser", 0), 4, false);
    let t = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[t.index()].gained_levels = vec!["tem"];
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Screamer"));
    let r = in_play(&mut e, 1, "Mr. Winthrop", Some(v));
    let gun = in_play(&mut e, 1, ".44 Magnum", Some(v));
    hand(&mut e, 0, &["Kiss of Lachesis", "Kiss of Lachesis"]);
    let req = request(&mut e);
    assert!(req.options.contains(&act(0, 0, a, Target::Card(r))));
    assert!(req.options.contains(&act(0, 0, a, Target::Card(gun))));
    assert!(
        !req.options.contains(&act(0, 2, a, Target::Minion(ally))),
        "[ser]: no ally"
    );
    pick(&mut e, act(0, 0, a, Target::Card(r)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[r.index()].in_play);
    pick(&mut e, act(0, 2, t, Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play);
}

/// Kiss of Lachesis [TEM]: "Move a non-unique equipment card from an ash
/// heap to this vampire. If the equipment card is moved from your ash heap,
/// pay half the cost rounded down; otherwise, pay no cost." From its
/// player's heap, the Helicopter (3 pool) for 1 pool; from another's, the
/// Jar of Skin Eaters (requires a Tremere) free, staying its owner's.
/// [LSJ 20100204] [LSJ 20040518-2] [LSJ 20100302-1] "Requirements do not
/// apply." [LSJ 20010627] [ANK 20180202] "Is an undirected action, even
/// when targeting another Methuselah's ash heap."
#[test]
fn kiss_of_lachesis_superior_takes_equipment_from_any_ash_heap() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tem", "tem"];
    let b = e.add_minion(SeatId(0), plain("Nosferatu", 1), 4, false);
    e.state.minions[b.index()].gained_levels = vec!["tem", "tem"];
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].ash.push(find_library("Helicopter"));
    e.state.seats[1]
        .ash
        .push(find_library("Jar of Skin Eaters"));
    zvtes::copies::number_unnumbered(&mut e);
    hand(&mut e, 0, &["Kiss of Lachesis", "Kiss of Lachesis"]);
    pick(&mut e, play(0, 3, a));
    pick(&mut e, Choice::ChooseSeat(SeatId(0)));
    request(&mut e);
    assert_eq!(e.action().expect("action").directed_seat(&e.state), None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let heli = placed_on(&e, "Helicopter", Some(a));
    assert_eq!(e.state.seats[0].pool, 9, "half of 3, rounded down");
    assert_eq!(blood(&e, a), 3, "the Kiss's blood");
    assert!(!e.state.cards[heli.index()].effect_only);
    // The only heap left, named without a choice.
    pick(&mut e, play(0, 3, b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let jar = placed_on(&e, "Jar of Skin Eaters", Some(b));
    assert_eq!(e.state.seats[0].pool, 9, "free");
    assert_eq!(e.state.cards[jar.index()].owner, SeatId(1));
}

/// Kiss of Lachesis [TEM] from its player's own ash heap: the half cost
/// takes the reductions (rules-qa 36.5, 41.13): Anne-Marie Bourgeois
/// ("Equipment cost Anne-Marie -1 pool") puts on the Helicopter (3 pool)
/// for nothing, half of 3 being 1.
#[test]
fn kiss_of_lachesis_half_cost_takes_reductions() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        find_card("Anne-Marie Bourgeois, Inconnu Recruiter"),
        4,
        false,
    );
    e.state.minions[a.index()].gained_levels = vec!["tem", "tem"];
    e.add_minion(SeatId(0), plain("Nosferatu", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[0].ash.push(find_library("Helicopter"));
    zvtes::copies::number_unnumbered(&mut e);
    hand(&mut e, 0, &["Kiss of Lachesis"]);
    pick(&mut e, play(0, 3, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Helicopter", Some(a));
    assert_eq!(e.state.seats[0].pool, 10, "1, less her 1");
}

/// Kiss of Lachesis [TEM]: [LSJ 20060209] a Flaming Candle played already
/// may still be put in play by an effect that is no equip action. And [ANK
/// 20200420-3]: not playable with no non-unique equipment in any ash heap.
#[test]
fn kiss_of_lachesis_moves_a_flaming_candle_played_this_game() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tem", "tem"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Kiss of Lachesis"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 3, a)), "nothing to move");
    let candle = find_library("Flaming Candle");
    e.state.once_played.push(candle);
    e.state.seats[1].ash.push(candle);
    zvtes::copies::number_unnumbered(&mut e);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    pick(&mut e, play(0, 3, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Flaming Candle", Some(a));
}

// ---- Ahrimane Protectorate, Armor of Terra, Art's Traumatic Essence ------------

/// Ahrimane Protectorate: "+1 stealth action. Unique. Put this card in
/// play, and this Ahrimane may burn 1 blood to unlock at the end of the
/// action." The card in play (itself); after resolution its Ahrimane burns
/// 1 blood to unlock.
#[test]
fn ahrimane_protectorate_enters_play_and_may_unlock_its_ahrimane() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ahrimane", 4..=9, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ahrimane Protectorate"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::UnlockForBlood { minion: a });
    request(&mut e);
    placed_on(&e, "Ahrimane Protectorate", None);
    assert!(!e.state.minions[a.index()].locked);
    assert_eq!(blood(&e, a), 3);
}

/// Ahrimane Protectorate: "You may burn this card to cause an action
/// directed at an ally or location you control to fail." Another
/// Methuselah's action at its controller's ally fails, the card burned;
/// the ally untouched.
#[test]
fn ahrimane_protectorate_burns_to_fail_an_action_at_your_ally() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[h.index()].gained_levels = vec!["tem"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Screamer"));
    let ap = in_play(&mut e, 1, "Ahrimane Protectorate", None);
    hand(&mut e, 0, &["Kiss of Lachesis"]);
    pick(&mut e, act(0, 2, h, Target::Minion(ally)));
    pick_when(&mut e, use_on(ap, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[ally.index()].in_play);
    assert!(!e.state.cards[ap.index()].in_play);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
}

/// Ahrimane Protectorate: "You may burn this card during an action against
/// you to give an Ahrimane you control +1 intercept." Its Ahrimane
/// attempting to block a +1 stealth action at its Methuselah gets it and
/// blocks; a non-Ahrimane blocker is not offered it.
#[test]
fn ahrimane_protectorate_gives_an_attempting_ahrimane_intercept() {
    for ahrimane in [true, false] {
        let mut e = game(2);
        let o = e.add_minion(SeatId(0), of_clan("Osebo", 4..=7, 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = if ahrimane {
            e.add_minion(SeatId(1), of_clan("Ahrimane", 4..=9, 0), 4, false)
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false)
        };
        let ap = in_play(&mut e, 1, "Ahrimane Protectorate", None);
        hand(&mut e, 0, &["Legend of the Leopard"]);
        pick(&mut e, act(0, 0, o, Target::Minion(z)));
        let boost = use_on(ap, 1, None, Target::None);
        if !ahrimane {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, boost);
            continue;
        }
        pick(&mut e, block(z));
        pick_when(&mut e, boost);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
        assert!(!e.state.cards[ap.index()].in_play);
    }
}

/// Seat 0's `a` (with an Ivory Bow: 1R aggravated) bleeds, blocked by
/// seat 1's Gargoyle `z` bearing Armor of Terra; seat 1 holds Rötschreck.
fn armored_block() -> (Engine, MinionId, MinionId, CardId, CardId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Gargoyle", 4..=8, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let bow = in_play(&mut e, 0, "Ivory Bow", Some(a));
    let armor = in_play(&mut e, 1, "Armor of Terra", Some(z));
    hand(&mut e, 1, &["Rötschreck"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(z));
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    pick(&mut e, use_on(bow, 0, None, Target::None));
    (e, a, z, bow, armor)
}

/// Armor of Terra: "The Gargoyle with this card treats aggravated damage as
/// normal damage. Burn this card if this Gargoyle goes to torpor." The
/// bow's aggravated point mended with 1 blood (aggravated, it would have
/// sent the Gargoyle to torpor); then 4 normal damage out of combat send it
/// to torpor, the card burned.
#[test]
fn armor_of_terra_turns_aggravated_damage_normal_and_burns_in_torpor() {
    let (mut e, _, z, _, armor) = armored_block();
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_torpor, "as normal");
    assert!(blood(&e, z) < 4, "mended with blood");
    assert!(e.state.cards[armor.index()].in_play);
    let left = blood(&e, z);
    e.damage(z, left + 1);
    assert!(e.state.minions[z.index()].in_torpor);
    assert!(!e.state.cards[armor.index()].in_play);
    assert!(ash_has(&e, 1, "Armor of Terra"));
}

/// Rötschreck: [ANK 20200130-1] "Can be played and is effective even if the
/// opponent treats aggravated damage as normal": against the bow's strike,
/// the Armor of Terra Gargoyle's Methuselah plays it; the Gargoyle goes to
/// torpor, its Armor burned.
#[test]
fn rotschreck_is_played_against_an_opponent_taking_aggravated_as_normal() {
    let (mut e, _, z, _, armor) = armored_block();
    let rot = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(z),
    };
    let req = offering(&mut e, rot);
    assert_eq!(req.seat, SeatId(1));
    pick(&mut e, rot);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_torpor);
    assert!(!e.state.cards[armor.index()].in_play);
}

/// Armor of Terra [tha]: "Put this card on a slave Gargoyle you control";
/// [vis] "on this acting Gargoyle". [LSJ 20011204-2] "[vis], [VIS] Can be
/// used by a non-slave Gargoyle." A non-slave Gargoyle plays [vis] on
/// itself (the card itself, not a stand-in); [tha] targets only a slave
/// Gargoyle of its player's; a Gargoyle bearing one is offered no second.
#[test]
fn armor_of_terra_vis_goes_on_a_non_slave_gargoyle() {
    let mut e = game(2);
    let gargoyle = |slave: bool| {
        CRYPT
            .iter()
            .position(|c| {
                c.clan == "Gargoyle" && c.slave.is_some() == slave && !c.adv && c.capacity >= 4
            })
            .expect("a Gargoyle") as u16
    };
    let g = e.add_minion(SeatId(0), gargoyle(false), 4, false);
    e.state.minions[g.index()].gained_levels = vec!["vis", "tha"];
    let slave = e.add_minion(SeatId(0), gargoyle(true), 3, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Armor of Terra", "Armor of Terra"]);
    let req = request(&mut e);
    assert!(req.options.contains(&act(0, 0, g, Target::Minion(slave))));
    assert!(
        !req.options.contains(&act(0, 0, g, Target::Minion(g))),
        "not a slave"
    );
    pick(&mut e, play(0, 1, g));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Armor of Terra", Some(g));
    e.state.minions[slave.index()].gained_levels = vec!["tha", "vis"];
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&act(0, 0, slave, Target::Minion(slave))));
    assert!(req.options.contains(&play(0, 1, slave)));
    in_play(&mut e, 0, "Armor of Terra", Some(slave));
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let req = request(&mut e);
    assert!(
        !req.options
            .contains(&act(0, 0, slave, Target::Minion(slave))),
        "only one"
    );
    assert!(!req.options.contains(&play(0, 1, slave)), "only one");
}

/// Armor of Terra [VIS]: "the Gargoyle with this card may prevent 1 damage
/// each combat": its bearer prevents the bow's point once; the [vis] card
/// offers no prevention.
#[test]
fn armor_of_terra_superior_prevents_one_damage_each_combat() {
    for superior in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let z = e.add_minion(SeatId(1), of_clan("Gargoyle", 4..=8, 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let bow = in_play(&mut e, 0, "Ivory Bow", Some(a));
        let armor = in_play_mode(
            &mut e,
            1,
            "Armor of Terra",
            Some(z),
            if superior { 2 } else { 1 },
        );
        pick(&mut e, bleed(a));
        pick(&mut e, block(z));
        until(&mut e, SeatId(0), WindowLabel::CombatStrike);
        pick(&mut e, use_on(bow, 0, None, Target::None));
        let prevent = offered_before(
            &mut e,
            use_on(armor, 0, None, Target::None),
            SeatId(0),
            WindowLabel::MinionPhase,
        );
        assert_eq!(prevent, superior);
        if superior {
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert_eq!(blood(&e, z), 4, "prevented");
        }
    }
}

/// Art's Traumatic Essence [mel]: "Ⓓ As [aus] above, and the target minion
/// burns 1 blood or life": the prey's vampire locked, 1 blood burned.
#[test]
fn arts_traumatic_essence_locks_and_burns_a_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["mel"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Art's Traumatic Essence"]);
    pick(&mut e, act(0, 1, a, Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[v.index()].locked);
    assert_eq!(blood(&e, v), 3);
}

/// Art's Traumatic Essence: [ANK 20210124] [LSJ 20070411] [LSJ 20090514]
/// "If the action is unblocked when it resolves and the target is locked,
/// the action 'fizzles'": no blood burned.
#[test]
fn arts_traumatic_essence_fizzles_on_a_target_locked_by_resolution() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["mel"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Art's Traumatic Essence"]);
    pick(&mut e, act(0, 1, a, Target::Minion(v)));
    request(&mut e);
    e.state.minions[v.index()].locked = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 4);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
}

/// Art's Traumatic Essence [MEL]: "Ⓓ Put this card on a ready unlocked
/// minion and lock them. The attached minion burns 1 blood or life each
/// time they announce an action or attempt to block." On the prey's
/// vampire (itself), locked; as its bleed is announced, 1 blood; as its
/// attempt to block is made, 1 more; an ally attached with 1 life burned
/// as it attempts, the attempt failing (rules-qa 41.4).
#[test]
fn arts_traumatic_essence_superior_taxes_actions_and_blocks() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["mel", "mel"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Art's Traumatic Essence"]);
    pick(&mut e, act(0, 2, a, Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Art's Traumatic Essence", Some(v));
    assert!(e.state.minions[v.index()].locked);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, bleed(v));
    assert_eq!(blood(&e, v), 3, "as announced");
    // Unlocked, it attempts to block.
    to_minion_phase(&mut e, SeatId(0));
    e.state.minions[v.index()].locked = false;
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    pick(&mut e, bleed(b));
    pick(&mut e, block(v));
    assert_eq!(blood(&e, v), 2, "as it attempts to block");
    // An ally with 1 life.
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Screamer"));
    e.state.minions[ally.index()].blood = 1;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play_mode(&mut e, 0, "Art's Traumatic Essence", Some(ally), 2);
    pick(&mut e, bleed(b));
    pick(&mut e, block(ally));
    request(&mut e);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    assert_eq!(
        e.action().map(|a| a.blocker),
        Some(None),
        "the attempt failed"
    );
}

/// Art's Traumatic Essence [MEL]: "During their master phase, they can lock
/// and burn 1 pool to burn this card." The attached minion's Methuselah, in
/// their master phase: locked, 1 pool, the card burned (to its owner's ash
/// heap).
#[test]
fn arts_traumatic_essence_is_burned_in_the_master_phase_for_a_lock_and_a_pool() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let c = in_play_mode(&mut e, 0, "Art's Traumatic Essence", Some(v), 2);
    let req = until(&mut e, SeatId(1), WindowLabel::MasterPhase);
    let u = uses(&req, c.0);
    assert_eq!(u.len(), 1, "{:?}", req.options);
    pick(&mut e, u[0]);
    request(&mut e);
    assert!(e.state.minions[v.index()].locked);
    assert_eq!(e.state.seats[1].pool, 9);
    assert!(!e.state.cards[c.index()].in_play);
    assert!(ash_has(&e, 0, "Art's Traumatic Essence"));
}

// ---- Picks, contests, the ash heap ------------------------------------------------

/// Blessed Audience [aus]: "Ⓓ Lock X-1 allies or younger vampires
/// controlled by your predator and/or prey." An ally and a younger vampire
/// of the prey picked, the older not offered; both locked, X = 3 blood.
#[test]
fn blessed_audience_locks_x_minus_one_of_the_neighbours() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["aus"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity == 3, 0), 3, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity == 8, 0), 4, false);
    hand(&mut e, 0, &["Blessed Audience"]);
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(req.options.contains(&Choice::ChooseMinion(ally)));
    assert!(req.options.contains(&Choice::ChooseMinion(young)));
    assert!(!req.options.contains(&Choice::ChooseMinion(old)));
    pick(&mut e, Choice::ChooseMinion(ally));
    pick(&mut e, Choice::ChooseMinion(young));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[ally.index()].locked && e.state.minions[young.index()].locked);
    assert!(!e.state.minions[old.index()].locked);
    assert_eq!(blood(&e, a), 3, "X = 3");
}

/// Blessed Audience [MEL]: "this vampire can increase their capacity for
/// purposes of choosing the targets of this effect. This action costs +1
/// blood for each point of capacity gained (chosen as the action is
/// announced)." A capacity-6 vampire picks a capacity-8 one: 3 points
/// bought, X = 2 + 3 = 5 blood.
#[test]
fn blessed_audience_superior_buys_capacity_to_pick_an_older_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 6, false);
    e.state.minions[a.index()].gained_levels = vec!["mel", "mel"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity == 8, 0), 4, false);
    let huge = e.add_minion(SeatId(1), textless(|c| c.capacity >= 10, 0), 4, false);
    hand(&mut e, 0, &["Blessed Audience"]);
    pick(&mut e, play(0, 2, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert!(req.options.contains(&Choice::ChooseMinion(old)));
    assert!(
        !req.options.contains(&Choice::ChooseMinion(huge)),
        "beyond its blood"
    );
    pick(&mut e, Choice::ChooseMinion(old));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[old.index()].locked);
    assert_eq!(blood(&e, a), 1, "2 for the pick, 3 for the capacity");
}

/// Cleansing Ritual: "+1 stealth action. [dom] Add 1 blood to a vampire you
/// control. [obe] Burn a card on an ally or younger vampire in your ready
/// region." [obe]: the retainer on a younger vampire of its player's
/// burned; not one on an older vampire.
#[test]
fn cleansing_ritual_feeds_or_burns_a_card_off_a_younger_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 3, false);
    e.state.minions[a.index()].gained_levels = vec!["dom", "obe"];
    let b = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 1), 3, false);
    e.state.minions[b.index()].gained_levels = vec!["obe"];
    let young = e.add_minion(SeatId(0), textless(|c| c.capacity == 3, 0), 3, false);
    let old = e.add_minion(SeatId(0), textless(|c| c.capacity == 8, 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let r = in_play(&mut e, 0, "Mr. Winthrop", Some(young));
    let r2 = in_play(&mut e, 0, "Mr. Winthrop", Some(old));
    hand(&mut e, 0, &["Cleansing Ritual", "Cleansing Ritual"]);
    pick(&mut e, act(0, 0, a, Target::Minion(young)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, young), 3, "capped at capacity 3");
    let req = request(&mut e);
    assert!(
        !req.options.contains(&act(0, 1, b, Target::Card(r2))),
        "older"
    );
    pick(&mut e, act(0, 1, b, Target::Card(r)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[r.index()].in_play);
}

/// Cleansing Ritual [OBE]: "As [obe] above, and unlock the target minion
/// during the next discard phase." The card burned; its locked vampire
/// unlocked as this turn's discard phase comes.
#[test]
fn cleansing_ritual_superior_unlocks_the_minion_in_the_discard_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.capacity == 6, 0), 3, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    let young = e.add_minion(SeatId(0), textless(|c| c.capacity == 3, 0), 3, false);
    e.state.minions[young.index()].locked = true;
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let r = in_play(&mut e, 0, "Mr. Winthrop", Some(young));
    hand(&mut e, 0, &["Cleansing Ritual"]);
    pick(&mut e, act(0, 2, a, Target::Card(r)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[r.index()].in_play);
    assert!(e.state.minions[young.index()].locked, "not yet");
    to_minion_phase(&mut e, SeatId(1));
    assert!(!e.state.minions[young.index()].locked, "the discard phase");
}

/// Two copies of one vampire contested by seats 0 and `rival`.
fn contested_pair(e: &mut Engine, rival: u8) -> (MinionId, MinionId) {
    let card = plain("Ventrue", 5);
    let own = e.add_minion(SeatId(0), card, 3, false);
    let other = e.add_minion(SeatId(rival), card, 3, false);
    e.state.minions[own.index()].contested = true;
    e.state.minions[other.index()].contested = true;
    (own, other)
}

/// Clio's Kiss [dom]: "Ⓓ Burn 1 pool from a Methuselah who is contesting a
/// card with you." The Methuselah contesting a vampire burns 1 pool; one
/// not contesting is no target.
#[test]
fn clios_kiss_burns_a_pool_from_a_contesting_methuselah() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Toreador", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 1), 4, false);
    contested_pair(&mut e, 2);
    hand(&mut e, 0, &["Clio's Kiss"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 0, a, Target::Seat(SeatId(1)))));
    pick(&mut e, act(0, 0, a, Target::Seat(SeatId(2))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10, 9]);
}

/// Clio's Kiss [tem]: "Exchange a card from your hand for a non-unique
/// non-master library card in your ash heap." [LSJ 20000222] [LSJ
/// 20011023] "Both the card in the ash heap and the card in hand have to be
/// named when announcing the action": named as announced (a unique card or
/// a master in the ash heap not offered), exchanged as it resolves.
#[test]
fn clios_kiss_names_both_cards_as_announced() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tem"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for c in ["Deer Rifle", "Helicopter", "Ivory Bow", "Villein"] {
        e.state.seats[0].ash.push(find_library(c));
    }
    hand(&mut e, 0, &["Clio's Kiss", "Screamer"]);
    zvtes::copies::number_unnumbered(&mut e);
    pick(&mut e, play(0, 1, a));
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::ActionExchange);
    let mut named = req.options.clone();
    named.sort_by_key(|o| match o {
        Choice::FindCard(c) => *c,
        _ => u16::MAX,
    });
    let mut want = vec![
        Choice::FindCard(find_library("Deer Rifle")),
        Choice::FindCard(find_library("Helicopter")),
    ];
    want.sort_by_key(|o| match o {
        Choice::FindCard(c) => *c,
        _ => u16::MAX,
    });
    assert_eq!(named, want, "non-unique, non-master");
    // The one other hand card, named with no choice.
    pick(&mut e, Choice::FindCard(find_library("Deer Rifle")));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").exchange,
        Some((find_library("Deer Rifle"), find_library("Screamer")))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].hand.contains(&find_library("Deer Rifle")));
    assert!(ash_has(&e, 0, "Screamer"));
}

/// Clio's Kiss [TEM]: "Ⓓ Choose a Methuselah who is contesting a vampire
/// with you. That Methuselah yields their copy. If no other Methuselah is
/// contesting the vampire, turn the vampire face up (unlocked)." The
/// rival's copy burned as yielded; the player's, alone, uncontested at
/// once, unlocked.
#[test]
fn clios_kiss_superior_makes_the_rival_yield_and_turns_yours_up() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tem", "tem"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Toreador", 0), 4, false);
    e.add_minion(SeatId(2), plain("Toreador", 1), 4, false);
    let (own, other) = contested_pair(&mut e, 2);
    e.state.minions[own.index()].locked = true;
    hand(&mut e, 0, &["Clio's Kiss"]);
    pick(&mut e, act(0, 2, a, Target::Seat(SeatId(2))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[other.index()].in_play, "yielded");
    let v = &e.state.minions[own.index()];
    assert!(v.in_play && !v.contested && !v.locked);
}

/// Clio's Kiss [TEM], a titled vampire turned up: a contested copy, out of
/// play, contested no title, so the yield leaves none contested; another
/// vampire in play holding the title contests it as the copy turns up
/// (owner, rules-qa 124.46).
#[test]
fn clios_kiss_turns_up_a_title_contested_only_by_another_vampire() {
    let princes: Vec<u16> = (0..CRYPT.len() as u16)
        .filter(|&i| {
            matches!(CRYPT[i as usize].title, Title::Prince(_)) && CRYPT[i as usize].capacity <= 9
        })
        .collect();
    let (card, rival) = princes
        .iter()
        .find_map(|&a| {
            let b = princes.iter().find(|&&b| {
                b != a
                    && CRYPT[b as usize].title == CRYPT[a as usize].title
                    && CRYPT[b as usize].name != CRYPT[a as usize].name
            })?;
            Some((a, *b))
        })
        .expect("two princes of one city");
    for other_prince in [false, true] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["tem", "tem"];
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Toreador", 0), 4, false);
        e.add_minion(SeatId(2), plain("Toreador", 1), 4, false);
        let own = e.add_minion(SeatId(0), card, 1, false);
        let copy = e.add_minion(SeatId(2), card, 1, false);
        e.state.minions[own.index()].contested = true;
        e.state.minions[copy.index()].contested = true;
        let held = other_prince.then(|| e.add_minion(SeatId(1), rival, 1, false));
        hand(&mut e, 0, &["Clio's Kiss"]);
        pick(&mut e, act(0, 2, a, Target::Seat(SeatId(2))));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[own.index()];
        assert!(v.in_play && !v.contested);
        assert_eq!(v.title_contested, other_prince);
        if let Some(h) = held {
            assert!(e.state.minions[h.index()].title_contested);
        }
    }
}

/// Emergency Rations: "+1 stealth action. Remove a mortal (ally or
/// retainer) in any Methuselah's ash heap from the game to put this card on
/// the acting Nagaraja." [LSJ 20010627] [ANK 20180202] "Is an undirected
/// action, even when targeting another Methuselah's ash heap": the prey's
/// mortal ally removed from the game, the card on the Nagaraja (itself).
#[test]
fn emergency_rations_eats_a_mortal_from_any_ash_heap() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[n.index()].clan_change = Some("Nagaraja");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let taxi = find_library("City Star Taxi");
    e.state.seats[1].ash.push(taxi);
    zvtes::copies::number_unnumbered(&mut e);
    hand(&mut e, 0, &["Emergency Rations"]);
    pick(&mut e, play(0, 0, n));
    request(&mut e);
    assert_eq!(e.action().expect("action").directed_seat(&e.state), None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].removed.contains(&taxi));
    placed_on(&e, "Emergency Rations", Some(n));
}

/// Emergency Rations: [ANK 20200420-3] "Cannot be played if no target is
/// available in any Methuselah's ash heap": a ghoul retainer is no mortal
/// (ANK 20200203-2); a burned imbued is one (rules-digest §3.6).
#[test]
fn emergency_rations_needs_a_mortal_in_an_ash_heap() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[n.index()].clan_change = Some("Nagaraja");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].ash.push(find_library("Revenant"));
    hand(&mut e, 0, &["Emergency Rations"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, n)), "no mortal");
    let imbued = CRYPT.iter().position(|c| c.imbued).expect("an imbued") as u16;
    e.state.seats[1].ash_crypt.push(imbued);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, n)), "an imbued");
}

/// Emergency Rations: "During your unlock phase you may burn this card to
/// restore this ready Nagaraja to full capacity with blood from the blood
/// bank." Its controller's unlock phase: offered, the Nagaraja filled, the
/// card burned.
#[test]
fn emergency_rations_refills_its_ready_nagaraja() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), plain("Nosferatu", 0), 1, false);
    e.state.minions[n.index()].clan_change = Some("Nagaraja");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = in_play(&mut e, 0, "Emergency Rations", Some(n));
    let ration = use_on(c, 0, None, Target::None);
    let req = offering(&mut e, ration);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::UnlockAbilities)
    );
    pick(&mut e, ration);
    request(&mut e);
    assert_eq!(Some(blood(&e, n)), e.state.capacity_of(n));
    assert!(!e.state.cards[c.index()].in_play);
}

/// Founders of the Ebony Kingdom: "+1 stealth action. Put this card on a
/// younger Laibon in your uncontrolled region and move 4 blood from the
/// blood bank to that vampire. (This card remains in play.) … Burn this
/// card if the vampire leaves the uncontrolled region." On the slot
/// (itself), 4 counters; the vampire influenced out, the card burned, not
/// carried.
#[test]
fn founders_of_the_ebony_kingdom_stays_on_the_slot_and_burns_as_it_leaves() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), of_clan("Guruhi", 7..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let young = of_sect(Sect::Laibon, 4..=5, 0);
    let slot = uncontrolled(&mut e, 0, young, 0);
    hand(&mut e, 0, &["Founders of the Ebony Kingdom"]);
    pick(&mut e, act(0, 0, g, Target::Uncontrolled(slot)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let founders = find_library("Founders of the Ebony Kingdom");
    let c = e
        .state
        .cards
        .iter()
        .position(|c| c.card == founders && c.in_play && !c.effect_only)
        .map(|i| CardId(i as u32))
        .expect("in play");
    assert_eq!(e.state.cards[c.index()].on_slot, Some((SeatId(0), slot)));
    assert_eq!(e.state.seats[0].uncontrolled[slot as usize].counters, 4);
    let cap = CRYPT[young as usize].capacity;
    e.state.seats[0].uncontrolled[slot as usize].counters = cap;
    pick_when(&mut e, Choice::InfluenceOut { slot });
    request(&mut e);
    assert!(!e.state.cards[c.index()].in_play);
    assert!(ash_has(&e, 0, "Founders of the Ebony Kingdom"));
}

/// Ghost-Eater: "+1 stealth hunt action. Choose a minion or retainer in any
/// ash heap who was burned from play since your last turn. Remove the
/// chosen minion or retainer from the game to add X blood to this Harbinger
/// of Skulls, where X is the capacity of the chosen vampire …" A vampire of
/// capacity 3 burned this turn: removed from the game, 3 blood (a hunt).
/// [LSJ 20010627] [ANK 20180202] "Is an undirected action, even when
/// targeting another Methuselah's ash heap."
#[test]
fn ghost_eater_eats_a_vampire_burned_since_your_last_turn() {
    let mut e = game(2);
    let h = e.add_minion(
        SeatId(0),
        of_clan("Harbinger of Skulls", 7..=11, 0),
        1,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let victim = textless(|c| c.capacity == 3, 0);
    let v = e.add_minion(SeatId(1), victim, 1, false);
    zvtes::copies::number_unnumbered(&mut e);
    e.burn(v);
    hand(&mut e, 0, &["Ghost-Eater"]);
    pick(&mut e, play(0, 0, h));
    request(&mut e);
    assert_eq!(e.action().expect("action").directed_seat(&e.state), None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, h), 4);
    assert!(e.state.seats[1].removed_crypt.contains(&victim));
}

/// Ghost-Eater: a merged vampire burned from play puts both its cards in
/// the ash heap burned from play (rulebook "Advanced"), each a choice.
#[test]
fn ghost_eater_offers_both_cards_of_a_burned_merged_vampire() {
    let mut e = game(2);
    let h = e.add_minion(
        SeatId(0),
        of_clan("Harbinger of Skulls", 7..=11, 0),
        1,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let adv = crypt_card("Kamiri wa Itherero", true);
    let base = crypt_card("Kamiri wa Itherero", false);
    let k = e.add_minion(SeatId(1), adv, 1, false);
    e.state.minions[k.index()].merged = Some(base);
    zvtes::copies::number_unnumbered(&mut e);
    e.burn(k);
    hand(&mut e, 0, &["Ghost-Eater"]);
    pick(&mut e, play(0, 0, h));
    let req = request(&mut e);
    for c in [base, adv] {
        assert!(
            req.options.contains(&Choice::AshVampire(c)),
            "{c}: {:?}",
            req.options
        );
    }
}

/// Ghost-Eater: [LSJ 20020429] "'Since your last turn' means between the
/// end of your last turn and now. Your last turn does not qualify, your
/// current turn does." A vampire burned in the player's last turn is no
/// choice; one burned since is. [ANK 20200420-3] "Cannot be played if no
/// target is available": nothing burned, not offered.
#[test]
fn ghost_eater_needs_a_minion_burned_since_your_last_turn() {
    for (burned_at, offered) in [(2, false), (3, true)] {
        let mut e = game(2);
        let h = e.add_minion(
            SeatId(0),
            of_clan("Harbinger of Skulls", 7..=11, 0),
            1,
            false,
        );
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let v = e.add_minion(SeatId(1), textless(|c| c.capacity == 3, 0), 1, false);
        hand(&mut e, 0, &["Ghost-Eater"]);
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 0, h)), "nothing burned");
        zvtes::copies::number_unnumbered(&mut e);
        e.state.turn = burned_at;
        e.burn(v);
        e.state.turn = 4;
        e.state.seats[0].previous_turn = 2;
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let req = request(&mut e);
        assert_eq!(req.options.contains(&play(0, 0, h)), offered, "{burned_at}");
    }
}

/// Ghost-Eater: [RBK action-card-or-card-in-play] "A minion can only
/// perform this special hunt once per turn."
#[test]
fn ghost_eater_is_a_hunt_once_each_turn() {
    let mut e = game(2);
    let h = e.add_minion(
        SeatId(0),
        of_clan("Harbinger of Skulls", 7..=11, 0),
        1,
        false,
    );
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let a = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    let b = e.add_ally(SeatId(1), find_library("Screamer"));
    zvtes::copies::number_unnumbered(&mut e);
    e.burn(a);
    e.burn(b);
    hand(&mut e, 0, &["Ghost-Eater", "Ghost-Eater"]);
    pick(&mut e, play(0, 0, h));
    pick_when(&mut e, Choice::FindCard(find_library("City Star Taxi")));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, h)), "locked");
    e.state.minions[h.index()].locked = false;
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, h)), "once each turn");
}

/// Hourglass of the Mind [pot]: "Draw 3 cards (discard down afterward)."
#[test]
fn hourglass_of_the_mind_draws_three() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let lib = ["Deer Rifle", ".44 Magnum", "Screamer", "Helicopter"];
    for c in lib {
        e.state.seats[0].library.push(find_library(c));
    }
    hand(&mut e, 0, &["Hourglass of the Mind"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    for c in &lib[1..] {
        assert!(e.state.seats[0].hand.contains(&find_library(c)), "{c}");
    }
}

/// Hourglass of the Mind [tem]: "Draw 4 cards without discarding, then
/// move four cards from your hand to the top of your library in any order.
/// Unlock this vampire." [LSJ 20060620] draws what the library holds: from
/// a library of 2, both drawn; then the hand's three cards back on top one
/// at a time, the last on top; the vampire unlocked.
#[test]
fn hourglass_of_the_mind_draws_four_then_tops_four_and_unlocks() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tem"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for c in ["Deer Rifle", ".44 Magnum"] {
        e.state.seats[0].library.push(find_library(c));
    }
    hand(&mut e, 0, &["Hourglass of the Mind", "Screamer"]);
    fill_hand(&mut e, 0);
    let held = e.state.seats[0].hand.len();
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::BottomCards);
    assert_eq!(e.state.seats[0].hand.len(), held + 1, "both drawn");
    assert!(e.state.seats[0].library.is_empty());
    let at = |e: &Engine, name: &str| {
        let c = find_library(name);
        Choice::Bottom(
            e.state.seats[0]
                .hand
                .iter()
                .position(|&h| h == c)
                .expect("in hand") as u8,
        )
    };
    // Each on top of the one before; the fourth (a Form of Mist, the only
    // card left of its kind) moves with no choice.
    for name in ["Screamer", "Deer Rifle", ".44 Magnum"] {
        let o = at(&e, name);
        pick(&mut e, o);
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    // Drawn up again from the top: the first moved is the one left.
    let library: Vec<u16> = e.state.seats[0].library.iter().copied().collect();
    assert_eq!(library, vec![find_library("Screamer")]);
}

/// Hourglass of the Mind [TEM]: "Put this card on this vampire. During your
/// unlock phase, this vampire can burn 2 blood to add 1 counter to this
/// card. While this vampire is ready, you get +1 hand size for each counter
/// on this card." 2 counters: +2 hand size, none while in torpor; in the
/// unlock phase, 2 blood for a third.
#[test]
fn hourglass_of_the_mind_superior_grows_hand_size_while_ready() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let base = e.state.hand_size(SeatId(0));
    let c = in_play_mode(&mut e, 0, "Hourglass of the Mind", Some(a), 2);
    e.state.cards[c.index()].counters = 2;
    assert_eq!(e.state.hand_size(SeatId(0)), base + 2);
    e.state.minions[a.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), base, "not ready");
    e.state.minions[a.index()].in_torpor = false;
    let counter = use_on(c, 0, None, Target::None);
    let req = offering(&mut e, counter);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::UnlockAbilities)
    );
    pick(&mut e, counter);
    request(&mut e);
    assert_eq!(e.state.cards[c.index()].counters, 3);
    assert_eq!(blood(&e, a), 2);
}

/// Kiss of Lachesis [TEM], Alastor: "Cards requiring a discipline come in
/// play at the inferior version" (RBK equip): Tinglestripe ([myt] or
/// [MYT]) moved or found by a vampire with no Mytherceria comes in at
/// [myt], its first version. [RBK equip] [RBK recruit-ally] [RBK
/// employ-retainer]
#[test]
fn an_equipment_put_in_play_ignoring_requirements_is_inferior() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Nosferatu", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tem", "tem"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.seats[1].ash.push(find_library("Tinglestripe"));
    zvtes::copies::number_unnumbered(&mut e);
    hand(&mut e, 0, &["Kiss of Lachesis"]);
    pick(&mut e, play(0, 3, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let t = placed_on(&e, "Tinglestripe", Some(a));
    assert_eq!(e.state.cards[t.index()].mode, 0, "[myt]");
    // Alastor's search.
    let mut e = game(2);
    let justicar = CRYPT
        .iter()
        .position(|c| matches!(c.title, Title::Justicar(_)) && !c.adv && c.capacity >= 6)
        .expect("a justicar") as u16;
    let j = e.add_minion(SeatId(0), justicar, 6, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Alastor"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(find_library("Tinglestripe"));
    for _ in 0..5 {
        e.state.seats[0].library.push(find_library("Deflection"));
    }
    pick(&mut e, play(0, 0, j));
    pick_when(&mut e, votes(j, true));
    pick_when(&mut e, Choice::FindCard(find_library("Tinglestripe")));
    request(&mut e);
    let t = placed_on(&e, "Tinglestripe", Some(j));
    assert_eq!(e.state.cards[t.index()].mode, 0, "[myt]");
}

// ---- Action cards, Panacea to Unburdening -----------------------------------

/// Panacea [aus]: "Burn 1 blood to unlock a younger vampire." [ANK 20210226]
/// "The burn blood effect is not reduced by effects that reduce the cost of
/// the card … The card cannot be played if the minion cannot afford to burn
/// the blood": the actor's blood burned, another Methuselah's younger
/// vampire unlocked (directed at them).
#[test]
fn panacea_auspex_burns_a_blood_to_unlock_a_younger_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    e.state.minions[a.index()].gained_levels = vec!["aus"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let y = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 1, false);
    e.state.minions[y.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Panacea"]);
    pick(&mut e, act(0, 0, a, Target::Minion(y)));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[y.index()].locked);
    assert_eq!(blood(&e, a), 2, "the effect's blood");
}

/// Panacea [OBE]: "As [obe] above, and unlock that ally during the next
/// discard phase." The hurt ally gains 1 life; still locked after the
/// action, unlocked once the discard phase came.
#[test]
fn panacea_superior_heals_a_hurt_ally_and_unlocks_it_at_discard() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(0), find_library("Screamer"));
    let start = blood(&e, ally);
    e.state.minions[ally.index()].blood = start - 1;
    e.state.minions[ally.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Panacea"]);
    pick(&mut e, act(0, 2, a, Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ally), start);
    assert!(e.state.minions[ally.index()].locked, "not yet");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].locked, "the discard phase");
}

/// Rayzeel's Song [ani]: "This vampire burns 1 blood to unlock an ally."
/// [ANK 20210226] the blood is an effect, not a cost; the ally another
/// Methuselah's (directed). [VAL]: "+1 stealth action. Add 1 life from the
/// blood bank to an ally that has fewer life than his or her starting
/// amount."
#[test]
fn rayzeels_song_unlocks_an_ally_or_heals_a_hurt_one() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 3, false);
    e.state.minions[a.index()].gained_levels = vec!["ani", "val", "val"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[b.index()].gained_levels = vec!["val", "val"];
    let ally = e.add_ally(SeatId(1), find_library("Screamer"));
    e.state.minions[ally.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Rayzeel's Song", "Rayzeel's Song"]);
    pick(&mut e, act(0, 0, a, Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].locked);
    assert_eq!(blood(&e, a), 2);
    let start = blood(&e, ally);
    e.state.minions[ally.index()].blood = start - 1;
    pick(&mut e, act(0, 2, b, Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ally), start);
}

/// Sense Vitality: "[for] This vampire gains 2 blood. [val] Rescue a vampire
/// from torpor. [VAL] As [val] above, and the rescued vampire gains 1
/// blood." [ANK 20181017] "[val], [VAL] Is not the default action, no blood
/// is paid for the rescue."
#[test]
fn sense_vitality_rescues_without_rulebook_blood_or_feeds_its_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["val", "val", "for"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 2, false);
    e.state.minions[b.index()].gained_levels = vec!["for"];
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Sense Vitality", "Sense Vitality"]);
    pick(&mut e, act(0, 2, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_torpor);
    assert_eq!(blood(&e, t), 2);
    assert_eq!(blood(&e, a), 4, "no rescue cost");
    pick(&mut e, play(0, 0, b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), 4);
}

/// Sense Vitality [val] is a rescue to every rescue reader, its cost (none)
/// aside (owner, rules-qa 124.204): Ritual Goblet's "Once each turn, this
/// vampire can unlock after rescuing a vampire from torpor" offered,
/// Humanitas' "+1 stealth when attempting to rescue" counted, Boss
/// Callihan ("cannot rescue other vampires") offered no such play.
#[test]
fn sense_vitality_is_a_rescue_to_rescue_readers() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["val"];
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 1, true);
    let base = CRYPT
        .iter()
        .position(|c| c.name == "Boss Callihan" && !c.adv)
        .expect("Boss Callihan") as u16;
    let c = e.add_minion(SeatId(0), base, 4, false);
    e.state.minions[c.index()].gained_levels = vec!["val"];
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let goblet = in_play(&mut e, 0, "Ritual Goblet", Some(a));
    in_play(&mut e, 0, "Humanitas", Some(a));
    hand(&mut e, 0, &["Sense Vitality"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&act(0, 1, c, Target::Minion(t))));
    assert!(req.options.contains(&act(0, 1, a, Target::Minion(t))));
    pick(&mut e, act(0, 1, a, Target::Minion(t)));
    request(&mut e);
    assert_eq!(e.action().expect("rescue").stealth(&e.state), 2);
    pick_when(&mut e, use_on(goblet, 0, None, Target::None));
    assert!(!e.state.minions[t.index()].in_torpor);
    assert!(!e.state.minions[a.index()].locked, "unlocked");
    assert_eq!(blood(&e, a), 4, "no rescue cost");
}

/// Sense Vitality [val] by a Salubri with Saulot's Healing Touch ("if the
/// action is successful, the rescued vampire gains 1 blood"): a rescue, so
/// the rescued vampire gains it (owner, rules-qa 124.204).
#[test]
fn sense_vitality_rescue_takes_saulots_healing_touch_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["val"];
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Saulot's Healing Touch", Some(a));
    hand(&mut e, 0, &["Sense Vitality"]);
    pick(&mut e, act(0, 1, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_torpor);
    assert_eq!(blood(&e, t), 2);
}

/// Healing Touch [OBE], a card rescue with a cost: Ritual Goblet's
/// "Rescuing a vampire from torpor costs this vampire -1 blood" takes its 1
/// blood off (owner, rules-qa 124.204).
#[test]
fn healing_touch_superior_rescue_takes_the_rescue_discount() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 1, true);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Ritual Goblet", Some(a));
    hand(&mut e, 0, &["Healing Touch"]);
    pick(&mut e, act(0, 2, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_torpor);
    assert_eq!(blood(&e, a), 4, "the card's 1 blood taken off");
}

/// Psychic Projection [aus]: "Put this card on a minion you control. The
/// minion with this card gets +2 intercept. Burn this card during your
/// unlock phase." [AUS]: "Each of your minions gets +1 intercept." The card
/// itself on the minion; burned in its controller's next unlock phase.
#[test]
fn psychic_projection_gives_intercept_until_the_unlock_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["aus", "aus"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Psychic Projection", "Psychic Projection"]);
    pick(&mut e, act(0, 0, a, Target::Minion(b)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Psychic Projection", Some(b));
    assert_eq!(e.state.bearer_mod(b, Stat::Intercept, ActionKind::Bleed), 2);
    let c = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.state.minions[c.index()].gained_levels = vec!["aus", "aus"];
    pick(&mut e, play(0, 1, c));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Psychic Projection", None);
    assert_eq!(e.state.bearer_mod(b, Stat::Intercept, ActionKind::Bleed), 3);
    assert_eq!(e.state.bearer_mod(a, Stat::Intercept, ActionKind::Bleed), 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.bearer_mod(b, Stat::Intercept, ActionKind::Bleed), 3);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.bearer_mod(b, Stat::Intercept, ActionKind::Bleed), 0);
    assert_eq!(
        e.state.seats[0]
            .ash
            .iter()
            .filter(|&&c| c == find_library("Psychic Projection"))
            .count(),
        2
    );
}

/// Reindoctrination: "Choose a younger Gargoyle. That Gargoyle loses any
/// existing slave status and becomes a slave to this acting vampire's clan.
/// Take control of the Gargoyle if another Methuselah controls him or her,
/// move the Gargoyle to your ready region, and unlock him or her." [ANK
/// 20190203] "Only requires one clan or the other (Tremere or Tremere
/// antitribu)": a Tremere antitribu takes another's torpid Gargoyle, now a
/// slave to the Tremere antitribu, ready and unlocked.
#[test]
fn reindoctrination_steals_and_enslaves_another_methuselahs_gargoyle() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), of_clan("Tremere antitribu", 6..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let g = e.add_minion(SeatId(1), of_clan("Gargoyle", 1..=5, 0), 1, true);
    e.state.minions[g.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Reindoctrination"]);
    pick(&mut e, act(0, 0, t, Target::Minion(g)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[g.index()];
    assert_eq!(v.owner, SeatId(0));
    assert!(!v.in_torpor && !v.locked);
    assert_eq!(e.state.slave_clan(g), Some("Tremere antitribu"));
}

/// Riddle Phantastique: "Put X riddle counters on this card, where X is half
/// the capacity of the acting vampire (round down). The minion with this
/// card burns 1 riddle counter instead of unlocking during his or her
/// unlock phase. Burn this card when it has no counters." A capacity-5
/// Kiasyd: 2 counters, the minion held locked two unlock phases, the card
/// then burned. Not played while another is in play.
#[test]
fn riddle_phantastique_holds_its_minion_locked_for_half_the_capacity() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), of_clan("Kiasyd", 5..=5, 0), 5, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 1, false);
    e.state.minions[y.index()].locked = true;
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Riddle Phantastique", "Riddle Phantastique"]);
    pick(&mut e, act(0, 0, k, Target::Minion(y)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = placed_on(&e, "Riddle Phantastique", Some(y));
    assert_eq!(e.state.cards[c.index()].counters, 2);
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. })),
        "another in play"
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[y.index()].locked);
    assert_eq!(e.state.cards[c.index()].counters, 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        e.state.minions[y.index()].locked,
        "the last counter instead"
    );
    assert!(!e.state.cards[c.index()].in_play);
    assert!(ash_has(&e, 0, "Riddle Phantastique"));
}

/// Riddle Phantastique: [LSJ 20070307] "If put into play with no counters,
/// it burns immediately": from a capacity-1 vampire, burned as it enters.
#[test]
fn riddle_phantastique_from_a_capacity_one_vampire_burns_at_once() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), of_clan("Kiasyd", 4..=4, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let y = e.add_ally(SeatId(1), find_library("Screamer"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Riddle Phantastique"]);
    pick(&mut e, act(0, 0, k, Target::Minion(y)));
    let req = request(&mut e);
    // Its capacity down to 1 as the action resolves (the blood not drained
    // yet).
    let down = in_play(&mut e, 0, "Barrenness", Some(k));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::HostCapacityDown(3),
        source: down,
    });
    assert_eq!(e.state.capacity_of(k), Some(1));
    e.choose(fallback(&req));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Riddle Phantastique"));
    assert!(!e
        .state
        .cards
        .iter()
        .any(|c| c.card == find_library("Riddle Phantastique") && c.in_play));
}

/// Riddle Phantastique: [ANK 20240610] "The burn option can only be used if
/// you do not control a ready Kiasyd (no matter the presence or absence of
/// a valid target)": with a ready Kiasyd, no burn option; with none, the
/// card burned for its replacement, though no target exists.
#[test]
fn riddle_phantastique_burn_option_closed_by_a_ready_kiasyd() {
    for kiasyd in [true, false] {
        let mut e = game(2);
        if kiasyd {
            e.add_minion(SeatId(0), of_clan("Kiasyd", 4..=11, 0), 4, false);
        }
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Malkavian", 0), 4, false);
        hand(&mut e, 0, &["Riddle Phantastique"]);
        pick(&mut e, Choice::Pass);
        let mut offered = false;
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            offered |= req.window == WindowLabel::UnlockAbilities
                && req.seat == SeatId(0)
                && req.options.contains(&Choice::Discard(0));
            e.choose(fallback(&req));
        }
        assert_eq!(offered, !kiasyd);
    }
}

/// Wise Spider: "Put this card on this acting Akunanse. If this action is
/// successful, unlock the acting vampire at the end of the turn. While this
/// Akunanse is ready, your hand size is one card larger." The card on the
/// Akunanse, +1 hand size; the Akunanse unlocked as the turn ends; in
/// torpor, no larger hand.
#[test]
fn wise_spider_raises_hand_size_while_its_akunanse_is_ready() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Akunanse", 4..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Wise Spider", "Wise Spider"]);
    let size = e.state.hand_size(SeatId(0));
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Wise Spider", Some(a));
    assert_eq!(e.state.hand_size(SeatId(0)), size + 1);
    assert!(e.state.minions[a.index()].locked);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[a.index()].locked,
        "unlocked at the end of the turn"
    );
    e.state.minions[a.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), size);
}

/// Sires Command, Childer Inherit: "Move 1 blood from the blood bank to any
/// other Laibon. Alternatively, if this Laibon is a magaji, Ⓓ lock two
/// ready Laibon who belong to the same clan (as each other) and are
/// controlled by the same Methuselah (as each other)." A Laibon feeds
/// another Methuselah's Laibon; the magaji's lock not offered with no such
/// pair, then locks the pair of one clan and one controller.
#[test]
fn sires_command_feeds_a_laibon_or_locks_a_pair_of_one_clan() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), of_clan("Osebo", 5..=11, 0), 4, false);
    let l = e.add_minion(SeatId(0), of_clan("Osebo", 5..=11, 1), 4, false);
    let a = e.add_minion(SeatId(1), of_clan("Guruhi", 4..=11, 0), 2, false);
    let b = e.add_minion(SeatId(1), of_clan("Akunanse", 4..=11, 0), 4, false);
    hand(
        &mut e,
        0,
        &[
            "Sires Command, Childer Inherit",
            "Sires Command, Childer Inherit",
        ],
    );
    pick(&mut e, act(0, 0, l, Target::Minion(a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 3);
    e.state.minions[m.index()].granted_title = Some(Title::Magaji);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&act(0, 1, m, Target::Minion(a))),
        "no pair of one clan"
    );
    let c = e.add_minion(SeatId(1), of_clan("Guruhi", 4..=11, 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    // The one pair's second pick elided.
    pick(&mut e, act(0, 1, m, Target::Minion(a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].locked && e.state.minions[c.index()].locked);
    assert!(!e.state.minions[b.index()].locked);
}

/// Steal the Mind [MYT], two copies on one minion: each copy's -1 bleed
/// (advanced rules §1.15: numeric bonuses from copies add).
#[test]
fn steal_the_mind_copies_add_up() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    for _ in 0..2 {
        let c = in_play_mode(&mut e, 0, "Steal the Mind", Some(t), 2);
        e.state.cards[c.index()].by = Some(a);
    }
    assert_eq!(e.state.bleed_against_mod(t, SeatId(0)), -2);
}

/// Steal the Mind [myt]: "Put this card on a ready minion. The acting
/// vampire gets +2 intercept when attempting to block that minion." [MYT]:
/// "the minion with this card gets -1 bleed when bleeding this acting
/// vampire's controller." The card on another's minion; its vampire's +2
/// intercept against it (the attempt reaching its stealth); the bearer's
/// bleed of that controller 1 less.
#[test]
fn steal_the_mind_gives_its_vampire_intercept_against_the_bearer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["myt", "myt"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Steal the Mind"]);
    let bleed_before = e.state.current_bleed(t);
    pick(&mut e, act(0, 2, a, Target::Minion(t)));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Steal the Mind", Some(t));
    assert_eq!(e.state.bleed_against_mod(t, SeatId(0)), -1);
    assert_eq!(e.state.current_bleed(t) as i32, bleed_before as i32 - 1);
    e.state.minions[a.index()].locked = false;
    pick(&mut e, Choice::Pass);
    pick(&mut e, Choice::Pass);
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, bleed(t));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert!(
        e.block_can_reach(SeatId(0), a),
        "+2 intercept against 1 stealth"
    );
    e.choose(index_of(&req, block(a)));
}

/// Steal the Mind: "Any minion can burn this card as an action." Another
/// Methuselah's minion burns it from its own vampire.
#[test]
fn steal_the_mind_burned_by_any_minion_as_an_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = in_play_mode(&mut e, 1, "Steal the Mind", Some(a), 1);
    e.state.cards[c.index()].by = Some(t);
    pick(&mut e, use_on(c, 0, Some(a), Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[c.index()].in_play);
}

/// Steal the Mind [dom]: "Ⓓ Lock a minion."
#[test]
fn steal_the_mind_dominate_locks_a_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Steal the Mind"]);
    pick(&mut e, act(0, 0, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].locked);
}

/// Travelers Obey the Tenets: "When a vampire successfully takes an action
/// directed at you, that vampire takes 1 unpreventable damage (after
/// resolving the action)." [ANK 20180206] "The acting player decides the
/// order of (and can play other) 'after resolving the action' effects":
/// the prey's vampire bleeding its controller takes 1 damage after
/// resolution; an ally's bleed none; only one per Methuselah.
#[test]
fn travelers_obey_the_tenets_damages_a_vampire_after_its_directed_action() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 5..=11, 0), 4, false);
    e.state.minions[m.index()].granted_title = Some(Title::Magaji);
    let m2 = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 5..=11, 1), 4, false);
    e.state.minions[m2.index()].granted_title = Some(Title::Magaji);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    hand(
        &mut e,
        0,
        &["Travelers Obey the Tenets", "Travelers Obey the Tenets"],
    );
    pick(&mut e, play(0, 0, m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Travelers Obey the Tenets", None);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, m2)), "only one");
    // Seat 1's turn: its vampire bleeds seat 0.
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, bleed(v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 3, "1 damage");
    let life = blood(&e, ally);
    pick(&mut e, bleed(ally));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ally), life, "an ally");
}

/// Travelers Obey the Tenets: "Any vampire may burn this card (and take no
/// damage from it) as a Ⓓ action." Another Methuselah's vampire burns it,
/// unharmed.
#[test]
fn travelers_obey_the_tenets_burned_without_damage() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let c = in_play(&mut e, 1, "Travelers Obey the Tenets", None);
    pick(&mut e, use_on(c, 0, Some(a), Target::None));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[c.index()].in_play);
    assert_eq!(blood(&e, a), 4, "no damage");
}

/// Vulture's Buffet: "+1 stealth action. If this hunt is successful, unlock
/// this vampire. [obf] Remove a minion or retainer in any Methuselah's ash
/// heap from the game to hunt." [ANK 20200421] "Is a hunt action." [LSJ
/// 20010627] [ANK 20180202] undirected at another's heap: the retainer
/// removed from the game, the vampire gains 1 blood and unlocks.
#[test]
fn vultures_buffet_removes_an_ash_minion_to_hunt_and_unlocks() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let spy = find_library("Raven Spy");
    e.state.seats[1].ash.push(spy);
    zvtes::copies::number_unnumbered(&mut e);
    hand(&mut e, 0, &["Vulture's Buffet"]);
    pick(&mut e, play(0, 0, a));
    request(&mut e);
    let act_ctx = e.action().expect("action");
    assert_eq!(act_ctx.directed_seat(&e.state), None);
    assert!(act_ctx.is_hunt());
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].removed.contains(&spy));
    assert_eq!(blood(&e, a), 3);
    assert!(!e.state.minions[a.index()].locked);
}

/// Vulture's Buffet: [ANK 20200420-3] "Cannot be played if no target is
/// available in any Methuselah's ash heap"; [RBK action-card-or-card-in-play]
/// "A minion can only perform this special hunt once per turn"; [SPI] at +1
/// hunt and 1 pool.
#[test]
fn vultures_buffet_superior_hunts_two_once_each_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("SPI", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Vulture's Buffet", "Vulture's Buffet"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 2, a)), "empty heaps");
    let victim = textless(|c| c.capacity == 3, 0);
    e.state.seats[1].ash_crypt.push(victim);
    e.state.seats[0].ash.push(find_library("Raven Spy"));
    zvtes::copies::number_unnumbered(&mut e);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    pick(&mut e, play(0, 2, a));
    pick(&mut e, Choice::ChooseSeat(SeatId(1)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].removed_crypt.contains(&victim));
    assert_eq!(blood(&e, a), 3);
    assert_eq!(e.state.seats[0].pool, 11);
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { actor: Some(x), card: 0, .. } if *x == a)),
        "once each turn"
    );
}

/// Resurrection [obe]: "Move the retainer card from an ash heap to this
/// vampire, with life equal to its starting life. If it requires a
/// Discipline, use the basic level." [LSJ 20010627] [ANK 20180202] "Is an
/// undirected action, even when targeting another Methuselah's ash heap";
/// [RBK equip] [RBK recruit-ally] [RBK employ-retainer] "Cards requiring a
/// discipline come in play at the inferior version"; [LSJ 20100204]
/// requirements do not apply: another
/// Methuselah's Raven Spy, burned this turn, on the acting vampire at 1
/// life, still that Methuselah's card.
#[test]
fn resurrection_takes_a_retainer_burned_since_your_last_turn_from_another_heap() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let h = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let spy = in_play_mode(&mut e, 1, "Raven Spy", Some(h), 1);
    e.state.cards[spy.index()].counters = 2;
    zvtes::copies::number_unnumbered(&mut e);
    e.burn(h);
    hand(&mut e, 0, &["Resurrection"]);
    pick(&mut e, play(0, 1, a));
    request(&mut e);
    assert_eq!(e.action().expect("action").directed_seat(&e.state), None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let back = placed_on(&e, "Raven Spy", Some(a));
    let c = &e.state.cards[back.index()];
    assert_eq!((c.mode, c.counters), (0, 1), "basic, its starting life");
    assert_eq!(c.owner, SeatId(1));
}

/// Resurrection: [LSJ 20011216-1] "Can only resurrect allies or retainers
/// burned from play. Discards and blocked actions do not qualify." [LSJ
/// 20020429] "Your last turn does not qualify, your current turn does." A
/// discarded ally offers nothing ([ANK 20200420-3] "Cannot be played if no
/// target is available"); another Methuselah's ally burned this turn goes to
/// the ready region unlocked ([OBE]): [LSJ 20100204] [LSJ 20040518-2] [LSJ
/// 20100302-1] "Requirements do not apply … that minion can act this
/// turn".
#[test]
fn resurrection_needs_a_card_burned_from_play_since_your_last_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["val", "obe", "obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let taxi = find_library("City Star Taxi");
    e.state.seats[0].ash.push(taxi);
    hand(&mut e, 0, &["Resurrection"]);
    let req = request(&mut e);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. })),
        "a discard"
    );
    let ally = e.add_ally(SeatId(1), taxi);
    zvtes::copies::number_unnumbered(&mut e);
    e.burn(ally);
    e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    pick(&mut e, play(0, 2, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let back = e
        .state
        .minions
        .iter()
        .position(|m| m.in_play && m.kind.ally_card() == Some(taxi))
        .map(|i| MinionId(i as u32))
        .expect("the ally in play");
    assert_eq!(e.state.minions[back.index()].owner, SeatId(0));
    assert!(!e.state.minions[back.index()].locked);
    assert!(e.state.seats[0].ash.contains(&taxi), "the discard stays");
}

/// Charnas the Imp, Ilomba: [LSJ 20090824] [LSJ 20051121] "The option to
/// place the card on another minion can only be used when playing the card
/// normally; for any other action or put into play effect, the card is
/// placed on the playing minion": put in play by Resurrection [obe], each
/// goes on the acting vampire.
#[test]
fn resurrection_puts_charnas_on_the_acting_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[b.index()].gained_levels = vec!["obe"];
    let h = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let imp = in_play(&mut e, 0, "Charnas the Imp", Some(h));
    e.state.cards[imp.index()].counters = 1;
    let ilomba = in_play(&mut e, 0, "Ilomba", Some(h));
    e.state.cards[ilomba.index()].counters = 1;
    zvtes::copies::number_unnumbered(&mut e);
    e.burn(h);
    hand(&mut e, 0, &["Resurrection", "Resurrection"]);
    pick_when(&mut e, play(0, 1, a));
    pick(&mut e, Choice::FindCard(find_library("Charnas the Imp")));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Charnas the Imp", Some(a));
    pick(&mut e, play(0, 1, b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Ilomba", Some(b));
}

/// Putrescent Servitude [thn]: "put this card on a mortal or ghoul ally you
/// control. This ally gains 1 life, and he or she may play cards that
/// require basic Potence [pot] as a vampire." [LSJ 19980303] it does not
/// have Potence; [ANK 20180111] basic only. Also "Move any mortal or ghoul
/// retainer you control to this vampire."
#[test]
fn putrescent_servitude_feeds_an_ally_basic_potence_or_moves_a_retainer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["thn"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.state.minions[b.index()].gained_levels = vec!["thn"];
    let h = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    let ally = e.add_ally(SeatId(0), find_library("City Star Taxi"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ghoul = in_play(&mut e, 0, "Ghoul Retainer", Some(h));
    let life = blood(&e, ally);
    hand(&mut e, 0, &["Putrescent Servitude", "Putrescent Servitude"]);
    pick(&mut e, act(0, 1, a, Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Putrescent Servitude", Some(ally));
    assert_eq!(blood(&e, ally), life + 1);
    assert!(!e.state.has_discipline(ally, "pot"));
    let grapple = find_library("Immortal Grapple");
    assert!(zvtes::cards::plays_card_as_vampire(&e.state, ally, grapple, 0).is_some());
    assert!(
        zvtes::cards::plays_card_as_vampire(&e.state, ally, grapple, 1).is_none(),
        "basic only"
    );
    pick(&mut e, act(0, 0, b, Target::Card(ghoul)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[ghoul.index()].host, Some(b));
}

/// Putrescent Servitude [THN]: "put this card on any mortal or ghoul ally
/// and take control of that ally." [TOM 19960226-1] "Requirements do not
/// apply when taking control of a card already in play."
#[test]
fn putrescent_servitude_superior_takes_an_ally_ignoring_its_requirements() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["thn", "thn"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Putrescent Servitude"]);
    pick(&mut e, act(0, 3, a, Target::Minion(ally)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[ally.index()].owner, SeatId(0));
    placed_on(&e, "Putrescent Servitude", Some(ally));
}

/// Unburdening the Bestial Soul [obe]: "Move 1 or more blood from this
/// vampire to another vampire … and unlock this acting vampire." The amount
/// announced (3 of 4), moved at resolution; the actor unlocked.
#[test]
fn unburdening_moves_announced_blood_to_another_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe"];
    let t = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Unburdening the Bestial Soul"]);
    pick(&mut e, act(0, 1, a, Target::Minion(t)));
    let req = request(&mut e);
    assert_eq!(req.options.len(), 4, "1 to 4: {:?}", req.options);
    e.choose(index_of(&req, set_x(3)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, t)), (1, 4));
    assert!(!e.state.minions[a.index()].locked);
}

/// Unburdening the Bestial Soul [OBE]: "Put this card on an ally or younger
/// vampire. The attached minion cannot perform actions, block or play
/// reaction cards. During the attached minion's unlock phase, burn this
/// card unless this acting vampire burns 2 blood." Its actions withheld;
/// in its controller's unlock phase the acting vampire's player pays 2
/// blood, then declines, the card burned.
#[test]
fn unburdening_superior_idles_its_bearer_while_its_player_pays() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let y = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 2, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Unburdening the Bestial Soul"]);
    pick(&mut e, act(0, 2, a, Target::Minion(y)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = placed_on(&e, "Unburdening the Bestial Soul", Some(y));
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, Choice::BurnBloodToAvoid));
    assert_eq!(blood(&e, a), 2);
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::TakeAction { minion, .. } if *minion == y)),
        "no actions"
    );
    assert!(req.options.contains(&bleed(v)));
    e.choose(index_of(&req, Choice::Pass));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, Choice::Pass));
    request(&mut e);
    assert!(!e.state.cards[c.index()].in_play);
    assert_eq!(blood(&e, a), 2);
}

/// The Wildebeest: "Put this card on the acting Ahrimane. This Ahrimane gets
/// +1 strength and -1 stealth … He or she cannot use equipment and cannot
/// have retainers (any retainers on this vampire are burned)." Its Ghoul
/// Retainer burned, the card on it, +1 strength and -1 stealth, no employ
/// offered; its Sport Bike's +1 intercept not its, an equip still offered
/// (the red reading: equipping allowed, use barred).
#[test]
fn the_wildebeest_burns_the_retainers_and_bars_equipment_use_not_equipping() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Ahrimane", 3..=11, 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ghoul = in_play(&mut e, 0, "Ghoul Retainer", Some(a));
    e.state.cards[ghoul.index()].counters = 2;
    in_play(&mut e, 0, "Sport Bike", Some(a));
    assert_eq!(e.state.bearer_mod(a, Stat::Intercept, ActionKind::Bleed), 1);
    hand(
        &mut e,
        0,
        &["Wildebeest, The", "Ghoul Retainer", "Leather Jacket"],
    );
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Wildebeest, The", Some(a));
    assert!(!e.state.cards[ghoul.index()].in_play, "retainer burned");
    assert!(ash_has(&e, 0, "Ghoul Retainer"));
    assert_eq!(e.state.bearer_mod(a, Stat::Strength, ActionKind::Bleed), 1);
    assert_eq!(e.state.bearer_mod(a, Stat::Stealth, ActionKind::Bleed), -1);
    assert_eq!(
        e.state.bearer_mod(a, Stat::Intercept, ActionKind::Bleed),
        0,
        "the bike's intercept not used"
    );
    e.state.minions[a.index()].locked = false;
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, a)), "employ barred");
    assert!(req.options.contains(&play(1, 0, a)), "equipping allowed");
}

/// No Secrets From the Magaji: "Put this card on this magaji and unlock him
/// or her. The magaji with this card gets +1 intercept when attempting to
/// block vampires. If this magaji is locked, he or she may attempt to block
/// a vampire as if unlocked." [LSJ 20070208] "It does not provide the
/// ability to play reaction cards while locked (this is not a wake)." The
/// card on the magaji, unlocked; locked again, it is offered the attempt
/// against a vampire with +1 intercept, no reaction; against an ally, no
/// attempt.
#[test]
fn no_secrets_lets_a_locked_magaji_block_a_vampire_not_an_ally() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 5..=11, 0), 4, false);
    e.state.minions[m.index()].granted_title = Some(Title::Magaji);
    e.state.minions[m.index()].gained_levels = vec!["aus"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    hand(&mut e, 0, &["No Secrets From the Magaji", "Eagle's Sight"]);
    pick(&mut e, play(0, 0, m));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "No Secrets From the Magaji", Some(m));
    assert!(!e.state.minions[m.index()].locked, "unlocked");
    e.state.minions[m.index()].locked = true;
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, bleed(ally));
    let req = request(&mut e);
    assert!(!req.options.contains(&block(m)), "an ally acting");
    e.choose(fallback(&req));
    pick_when(&mut e, bleed(v));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(m)));
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, .. })),
        "no reaction while locked"
    );
    assert_eq!(
        e.action().expect("action").host_intercept(&e.state, m),
        1,
        "+1 against a vampire"
    );
    e.choose(index_of(&req, block(m)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10, "blocked");
    placed_on(&e, "No Secrets From the Magaji", Some(m));
}

/// No Secrets From the Magaji: "Burn this card if this magaji attempts to
/// block a vampire but is not successful." [LSJ 20090810] [LSJ 20100305]
/// "It still burns if a block is attempted and then an effect prevents the
/// block (eg. Daring the Dawn, Psalm of the Damned)": the bleeder's Daring
/// the Dawn fails the attempt, the card burned; so does a [MAL] Psalm of
/// the Damned on the magaji.
#[test]
fn daring_the_dawn_failing_the_magajis_attempt_burns_no_secrets() {
    for (card, mode, disc) in [
        ("Daring the Dawn", 0, "for"),
        ("Psalm of the Damned", 2, "MAL"),
    ] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 5..=11, 0), 4, false);
        e.state.minions[m.index()].granted_title = Some(Title::Magaji);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let c = in_play(&mut e, 0, "No Secrets From the Magaji", Some(m));
        let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if disc == "MAL" {
            e.state.minions[v.index()].made_infernal = true;
            in_play(&mut e, 1, "Maleficia", Some(v));
            in_play(&mut e, 1, "Maleficia", Some(v));
        } else {
            e.state.minions[v.index()].gained_levels = vec![disc];
        }
        hand(&mut e, 1, &[card]);
        pick(&mut e, Choice::Pass);
        pick_when(&mut e, bleed(v));
        pick_when(&mut e, block(m));
        let target = if mode == 2 {
            Target::Minion(m)
        } else {
            Target::None
        };
        pick_when(&mut e, act(0, mode, v, target));
        request(&mut e);
        assert!(!e.state.cards[c.index()].in_play, "{card}");
        assert!(ash_has(&e, 0, "No Secrets From the Magaji"), "{card}");
    }
}

/// Vlad Tepes: [ANK 20210903] "He can use No Secrets From the Magaji to
/// attempt to block a vampire acting with no stealth": locked, he is
/// offered the attempt.
#[test]
fn vlad_tepes_blocks_a_vampire_with_no_stealth_through_no_secrets_while_locked() {
    let mut e = game(2);
    let vlad = e.add_minion(SeatId(0), find_card("Vlad Tepes"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    in_play(&mut e, 0, "No Secrets From the Magaji", Some(vlad));
    e.state.minions[vlad.index()].locked = true;
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, bleed(v));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(vlad)));
}

/// `seat`'s library: `cards` under `n` Deflections (the replacements).
fn stack(e: &mut Engine, seat: usize, cards: &[&str], n: usize) {
    for c in cards {
        e.state.seats[seat].library.push(find_library(c));
    }
    for _ in 0..n {
        e.state.seats[seat].library.push(find_library("Deflection"));
    }
}

/// Muricia's Call [ani]: "Search your library for a retainer that requires
/// Animalism. This vampire employs that retainer (pay cost as normal).
/// Shuffle your library afterward." [TOM 19960130] "Is an action of the
/// appropriate type for the chosen card (recruit/employ/equip)": an employ
/// action; the Raven Spy found as it resolves, on the vampire, its blood
/// paid; a Feral Hound (requiring a Ravnos) not offered to a non-Ravnos
/// (requirements apply: ANK 20180817).
#[test]
fn muricias_call_employs_an_animalism_retainer_from_the_library() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["ani"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Muricia's Call"]);
    fill_hand(&mut e, 0);
    stack(&mut e, 0, &["Raven Spy", "Feral Hound"], 3);
    pick(&mut e, play(0, 0, a));
    request(&mut e);
    assert!(e.action().expect("action").is_employ_action(&e.state));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    let spy = find_library("Raven Spy");
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::TakeFrom {
                zone: zvtes::decision::Zone::Library,
                card: spy,
            }
        ]
    );
    e.choose(1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = placed_on(&e, "Raven Spy", Some(a));
    assert_eq!(e.state.cards[c.index()].counters, 1, "the basic version");
    assert_eq!(blood(&e, a), 3, "its cost");
    assert_eq!(
        e.state.seats[0].library.len(),
        3,
        "the spy taken, a card drawn"
    );
    assert!(e.state.seats[0]
        .library
        .contains(&find_library("Feral Hound")));
}

/// Muricia's Call: the found retainer's "cost as normal" takes cost
/// reductions, asked once it is chosen, as a searched ally's (LSJ
/// 20090920-2): The Shard pays the Raven Spy's 1 blood.
#[test]
fn muricias_call_retainer_takes_cost_reductions() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["ani"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let shard = in_play(&mut e, 0, "Shard, London, The", None);
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Muricia's Call"]);
    fill_hand(&mut e, 0);
    stack(&mut e, 0, &["Raven Spy"], 3);
    pick(&mut e, play(0, 0, a));
    let req = until(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    e.choose(index_of(
        &req,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: find_library("Raven Spy"),
        },
    ));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    e.choose(index_of(&req, Choice::ResolveRider { card: shard.0 }));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Raven Spy", Some(a));
    assert!(e.state.cards[shard.index()].locked);
    assert_eq!(blood(&e, a), 4, "no blood paid");
}

/// A searched retainer pays its searcher's surcharge on any blood left
/// (rules-qa 119.2; ANK 20200709): Jenna Cross ("Cards that cost blood cost
/// Jenna 1 additional blood to play") pays 1 + 1 for Muricia's Call's Raven
/// Spy.
#[test]
fn muricias_call_charges_jenna_cross_her_blood() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jenna Cross"), 4, false);
    e.state.minions[j.index()].gained_levels = vec!["ani"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Muricia's Call"]);
    fill_hand(&mut e, 0);
    stack(&mut e, 0, &["Raven Spy"], 3);
    pick(&mut e, play(0, 0, j));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: find_library("Raven Spy"),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    placed_on(&e, "Raven Spy", Some(j));
    assert_eq!(blood(&e, j), 4 - 1 - 1);
}

/// Jenna Cross at 1 blood: the Raven Spy would cost her 1 + 1, so Muricia's
/// Call's search does not offer it (the employ gate counts her surcharge,
/// rules-qa 119.2, 124.147).
#[test]
fn muricias_call_offers_jenna_cross_no_retainer_she_cannot_pay() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jenna Cross"), 1, false);
    e.state.minions[j.index()].gained_levels = vec!["ani"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Muricia's Call"]);
    fill_hand(&mut e, 0);
    stack(&mut e, 0, &["Raven Spy"], 3);
    pick(&mut e, play(0, 0, j));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: find_library("Raven Spy"),
        },
    );
    assert_eq!(blood(&e, j), 1);
}

/// Muricia's Call [ANI][SPI]: "the retainer has an additional life … and
/// unlock this acting vampire at the end of the turn." [LSJ 20080803]
/// "When choosing a card whose effect depend on Discipline level, the
/// vampire decides in respect to his disciplines the normal way": a
/// superior vampire's Raven Spy taken at [ANI] (its player's choice,
/// rules-qa 125.17), its 2 life plus 1; the vampire unlocked
/// as the turn ends. [ANK 20180817] "Does not prevent the acting vampire
/// from playing the equipment/retainer/ally directly this turn if he
/// unlocks": unlocked, it employs another Raven Spy from hand.
#[test]
fn muricias_call_superior_raven_spy_has_three_life_and_unlocks_at_end_of_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["ani", "ani", "spi", "spi"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Muricia's Call", "Raven Spy"]);
    fill_hand(&mut e, 0);
    stack(&mut e, 0, &["Raven Spy"], 3);
    pick(&mut e, play(0, 2, a));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: find_library("Raven Spy"),
        },
    );
    pick(
        &mut e,
        Choice::Version {
            card: find_library("Raven Spy"),
            mode: 1,
        },
    );
    let c = placed_on(&e, "Raven Spy", Some(a));
    assert_eq!(e.state.cards[c.index()].mode, 1);
    assert_eq!(e.state.cards[c.index()].counters, 3);
    e.state.minions[a.index()].locked = false;
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        req.options.contains(&play(0, 1, a)),
        "employed from hand too"
    );
    e.state.minions[a.index()].locked = true;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[a.index()].locked,
        "unlocked at the end of the turn"
    );
}

/// Muricia's Call [ani][spi]'s "additional life" is a starting life, as
/// Tainted Spring [ANI]'s (rules-qa 74.12): its Raven Spy [ANI] at 3 life,
/// down to 1, is restored to 3 by Renewed Vigor [obe] ("enough life to …
/// reach their starting life"); Tainted Spring [ANI]'s Murder of Crows
/// likewise to 3.
#[test]
fn an_additional_life_raises_a_retainers_starting_life() {
    let heal = |e: &mut Engine, b: MinionId, c: CardId| {
        e.state.cards[c.index()].counters = 1;
        pick_when(
            e,
            Choice::Play {
                card: 0,
                mode: 3,
                actor: Some(b),
                target: Target::Card(c),
            },
        );
        until(e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.cards[c.index()].counters, 3, "its starting life");
    };
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["ani", "ani", "spi"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[b.index()].gained_levels = vec!["obe"];
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Muricia's Call", "Renewed Vigor"]);
    fill_hand(&mut e, 0);
    stack(&mut e, 0, &["Raven Spy"], 3);
    pick(&mut e, play(0, 1, a));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: find_library("Raven Spy"),
        },
    );
    pick(
        &mut e,
        Choice::Version {
            card: find_library("Raven Spy"),
            mode: 1,
        },
    );
    let c = placed_on(&e, "Raven Spy", Some(a));
    assert_eq!(e.state.cards[c.index()].counters, 3);
    heal(&mut e, b, c);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["ani", "ani"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[b.index()].gained_levels = vec!["obe"];
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Murder of Crows", "Tainted Spring", "Renewed Vigor"],
    );
    pick(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = placed_on(&e, "Murder of Crows", Some(a));
    assert_eq!(e.state.cards[c.index()].counters, 3);
    heal(&mut e, b, c);
}

/// Muricia's Call: [LSJ 20080205-2] [LSJ 20060417] "meeting the [ani]
/// requirement for Pack Alpha or Muricia's Call also meets the [ani]
/// requirement for the Raven Spy employed by that effect": The Vozhd of
/// Juiz de Fora, playing [ani] cards as a vampire, plays it and employs the
/// Raven Spy. Not offered with an empty library (rules-qa 122.213).
#[test]
fn vozhd_of_juiz_de_fora_plays_muricias_call_and_meets_animalism_for_its_raven_spy() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    in_play(&mut e, 0, "Revenant", Some(b));
    let vozhd = e.add_ally(SeatId(0), find_library("Vozhd of Juiz de Fora, The"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    request(&mut e); // its entry burns the Revenant
    hand(&mut e, 0, &["Muricia's Call"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play(0, 0, vozhd)),
        "an empty library"
    );
    fill_hand(&mut e, 0);
    stack(&mut e, 0, &["Raven Spy"], 3);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    pick(&mut e, play(0, 0, vozhd));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: find_library("Raven Spy"),
        },
    );
    request(&mut e);
    placed_on(&e, "Raven Spy", Some(vozhd));
}

/// Pandora's Whisper [nec]: "Move one library card from your ash heap to
/// your hand (discard down afterward)." [LSJ 20010627] "The card to
/// retrieve must be announced when declaring the action"; "Do not replace
/// until your unlock phase" [ANK 20200629-2] [LSJ 20091208] [RTR 19951017]
/// "Is replaced before unlocking cards": the named card reaches the hand,
/// the Whisper unreplaced until the next unlock phase.
#[test]
fn pandoras_whisper_necromancy_takes_the_named_ash_card_and_waits_to_replace() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["nec"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Pandora's Whisper"]);
    fill_hand(&mut e, 0);
    stack(&mut e, 0, &[], 4);
    let deflection = find_library("Deflection");
    e.state.seats[0].ash.push(find_library("Bum's Rush"));
    e.state.seats[0]
        .ash
        .push(find_library("Govern the Unaligned"));
    zvtes::copies::number_unnumbered(&mut e);
    pick(&mut e, play(0, 0, a));
    pick(
        &mut e,
        Choice::FindCard(find_library("Govern the Unaligned")),
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0]
        .hand
        .contains(&find_library("Govern the Unaligned")));
    assert!(ash_has(&e, 0, "Bum's Rush"));
    let held = |e: &Engine| e.state.seats[0].held.total();
    assert_eq!(held(&e), 1, "not replaced yet");
    let library = e.state.seats[0].library.len();
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(held(&e), 0, "replaced in the unlock phase");
    assert_eq!(e.state.hand_load(SeatId(0)), e.state.hand_size(SeatId(0)));
    assert!(e.state.seats[0].library.len() <= library);
    let _ = deflection;
}

/// Pandora's Whisper [myt]: "Each other Methuselah can draw 2 cards. You
/// draw 5 cards (discard down afterward)." Read all or none (the red
/// reading): one other Methuselah draws both, one passes; then, in the
/// printed order, the acting Methuselah draws 5 and discards down. [LSJ
/// 20060620] "draw as much as you can": a library of 3 gives 3.
#[test]
fn pandoras_whisper_others_may_draw_two() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["myt"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Pandora's Whisper"]);
    fill_hand(&mut e, 0);
    stack(&mut e, 0, &[], 3);
    for s in [1, 2] {
        fill_hand(&mut e, s as u8);
        stack(&mut e, s, &[], 4);
    }
    pick(&mut e, play(0, 1, a));
    let mut drew = Vec::new();
    loop {
        let req = request(&mut e);
        match (req.seat, req.window) {
            (SeatId(0), WindowLabel::MinionPhase) => break,
            (s, WindowLabel::Draw) => {
                assert_eq!(req.options, vec![Choice::Pass, Choice::DrawLibrary]);
                assert_eq!(e.state.seats[0].library.len(), 3, "the others first");
                drew.push(s);
                let want = if s == SeatId(1) {
                    Choice::DrawLibrary
                } else {
                    Choice::Pass
                };
                e.choose(index_of(&req, want));
            }
            _ => e.choose(if req.window == WindowLabel::DiscardDown {
                0
            } else {
                fallback(&req)
            }),
        }
    }
    assert_eq!(drew, vec![SeatId(1), SeatId(2)], "the prey first");
    assert_eq!(e.state.seats[1].library.len(), 2, "both drawn");
    assert_eq!(e.state.seats[2].library.len(), 4, "passed");
    assert_eq!(e.state.seats[0].library.len(), 0, "all it held");
    let size = e.state.hand_size(SeatId(0));
    assert_eq!(e.state.hand_load(SeatId(0)), size, "discarded down");
}

/// Pandora's Whisper [MYT]: "you can put any of the cards you discard on
/// the top of your library instead of in your ash heap": the discards down
/// offered, one at a time; the one taken on top of the library.
#[test]
fn pandoras_whisper_superior_puts_discards_on_the_library_top() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["myt", "myt"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Pandora's Whisper"]);
    fill_hand(&mut e, 0);
    stack(&mut e, 0, &["Bum's Rush"], 4);
    let rush = find_library("Bum's Rush");
    pick(&mut e, play(0, 2, a));
    let req = loop {
        let req = request(&mut e);
        if req.window == WindowLabel::DiscardDown {
            let i = e.state.seats[0]
                .hand
                .iter()
                .position(|&c| c == rush)
                .unwrap_or(0);
            let i = req
                .options
                .iter()
                .position(|o| *o == Choice::Discard(i as u8))
                .unwrap_or(0);
            e.choose(i);
            continue;
        }
        if req.options.contains(&Choice::FindCard(rush)) {
            break req;
        }
        e.choose(fallback(&req));
    };
    assert!(
        req.options
            .iter()
            .all(|o| matches!(o, Choice::Pass | Choice::FindCard(_))),
        "{:?}",
        req.options
    );
    e.choose(index_of(&req, Choice::FindCard(rush)));
    let req = request(&mut e);
    e.choose(index_of(&req, Choice::Pass));
    assert_eq!(e.state.seats[0].library.last(), Some(&rush));
    assert!(!ash_has(&e, 0, "Bum's Rush"));
}

/// Persistent Echo [mel]: "Put this card in play with one card requiring
/// Melpominee from your hand on it … A vampire you control can burn this
/// card to play the card on it as if from your hand and as if they had
/// basic Melpominee." [LSJ 20051114] "A vampire can use any effect on the
/// stored card, if they otherwise fulfill its requirements": a Gangrel
/// without Melpominee plays the stored Virtuosa's [mel] +2 bleed, not its
/// [pre] nor its [MEL]; the Echo burned. [LSJ 20081126] "Ignores all 'do
/// not replace' clauses and special replacement effects on cards played
/// from it": not replaced. [LSJ 20050118] "The card on it cannot be
/// discarded or burned to satisfy another effect": out of the hand.
#[test]
fn persistent_echo_stores_a_melpominee_card_a_vampire_plays_as_if_basic() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["mel"];
    let g = e.add_minion(SeatId(0), plain("Gangrel", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Persistent Echo", "Virtuosa"]);
    fill_hand(&mut e, 0);
    stack(&mut e, 0, &[], 4);
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let echo = placed_on(&e, "Persistent Echo", None);
    let virtuosa = find_library("Virtuosa");
    assert!(e.state.cards[echo.index()].stored.contains(&virtuosa));
    assert!(!e.state.seats[0].hand.contains(&virtuosa));
    let library = e.state.seats[0].library.len();
    pick(&mut e, bleed(g));
    let req = request(&mut e);
    let from_echo = |m: u8| {
        req.options.iter().any(|o| {
            matches!(o, Choice::Play { card, mode, actor: Some(x), .. }
                if *x == g && *mode == m && e.state.seats[0].hand.len() <= *card as usize)
        })
    };
    assert!(from_echo(2), "[mel] +2 bleed: {:?}", req.options);
    assert!(!from_echo(0), "[pre] its own requirement");
    assert!(!from_echo(3), "[MEL] beyond basic");
    let i = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::Play { mode: 2, actor: Some(x), .. } if *x == g))
        .expect("the stored card");
    e.choose(i);
    request(&mut e);
    assert!(!e.state.cards[echo.index()].in_play, "the Echo burned");
    assert!(ash_has(&e, 0, "Persistent Echo"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].library.len(), library, "not replaced");
    assert!(ash_has(&e, 0, "Virtuosa"));
}

/// Persistent Echo: [LSJ 20021128] "If it leaves play, the card on it is
/// burned." [MEL]: the stored card played as if superior.
#[test]
fn persistent_echo_superior_lends_superior_and_burns_its_card_leaving_play() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), plain("Gangrel", 0), 4, false);
    e.add_minion(SeatId(0), plain("Gangrel", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let echo = in_play_mode(&mut e, 0, "Persistent Echo", None, 2);
    let virtuosa = find_library("Virtuosa");
    e.state.cards[echo.index()].stored.push(virtuosa);
    pick(&mut e, bleed(g));
    let req = request(&mut e);
    assert!(
        req.options.iter().any(|o| matches!(o,
            Choice::Play { mode: 3, actor: Some(x), .. } if *x == g)),
        "[MEL] as if superior: {:?}",
        req.options
    );
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Gangrel", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let echo = in_play_mode(&mut e, 0, "Persistent Echo", None, 1);
    e.state.cards[echo.index()].stored.push(virtuosa);
    e.burn_card(echo);
    assert!(ash_has(&e, 0, "Virtuosa"));
    assert!(ash_has(&e, 0, "Persistent Echo"));
}

/// Persistent Echo [aus]: "Unlock a vampire with Melpominee [mel]."
#[test]
fn persistent_echo_auspex_unlocks_a_melpominee_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["aus"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Toreador", 0), 4, false);
    e.state.minions[t.index()].gained_levels = vec!["mel"];
    e.state.minions[t.index()].locked = true;
    let n = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[n.index()].locked = true;
    hand(&mut e, 0, &["Persistent Echo"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&act(0, 0, a, Target::Minion(n))),
        "no Melpominee"
    );
    pick(&mut e, act(0, 0, a, Target::Minion(t)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].locked);
}

/// Shadow of Taint [dom]: "Ⓓ Move a card played by another Methuselah on an
/// ally in your ready region to another ally." The prey's Steal the Mind on
/// the player's ally goes to the prey's own ally, chosen as played (the
/// action directed at the prey); a card the player played is no target.
#[test]
fn shadow_of_taint_moves_anothers_card_from_an_own_ally_to_another_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let mine = e.add_ally(SeatId(0), find_library("City Star Taxi"));
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let theirs = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    let c = in_play_mode(&mut e, 1, "Steal the Mind", Some(mine), 1);
    e.state.cards[c.index()].by = Some(v);
    let own = in_play_mode(&mut e, 0, "Steal the Mind", Some(mine), 1);
    hand(&mut e, 0, &["Shadow of Taint"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&act(0, 0, a, Target::Card(own))),
        "played by its player"
    );
    pick(&mut e, act(0, 0, a, Target::Card(c)));
    request(&mut e);
    assert_eq!(e.state.cards[c.index()].host, Some(mine), "not yet");
    let directed = e.action().expect("action").directed_seats(&e.state);
    assert!(directed.contains(SeatId(1)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[c.index()].host, Some(theirs));
    assert_eq!(e.state.cards[c.index()].by, Some(v));
}

/// Shadow of Taint [val]: "move a card played by another Methuselah on a
/// vampire in your ready region to another minion on whom the card could be
/// played. That minion cannot be a vampire older than the vampire with the
/// card." The red reading: the card's played mode's target filter, read for
/// its player as if its minion played it now: a Puppet Master [DOM] ("a
/// younger vampire") on the player's capacity-5 vampire, played by a
/// capacity-8 one, goes to the prey's capacity-4 vampire, not to an ally nor
/// to an older vampire. [VAL] at +1 stealth.
#[test]
fn shadow_of_taint_moves_a_card_where_it_could_be_played() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["val", "val"];
    let host = e.add_minion(SeatId(0), textless(|c| c.capacity == 5, 0), 4, false);
    let ally = e.add_ally(SeatId(0), find_library("City Star Taxi"));
    let by = e.add_minion(SeatId(1), textless(|c| c.capacity == 8, 0), 4, false);
    let young = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 4, false);
    let old = e.add_minion(SeatId(1), textless(|c| c.capacity == 7, 0), 4, false);
    let c = in_play_mode(&mut e, 1, "Puppet Master", Some(host), 1);
    e.state.cards[c.index()].by = Some(by);
    hand(&mut e, 0, &["Shadow of Taint"]);
    pick(&mut e, act(0, 4, a, Target::Card(c)));
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::ChooseMinion(young)));
    assert!(
        !req.options.contains(&Choice::ChooseMinion(ally)),
        "no vampire"
    );
    assert!(
        !req.options.contains(&Choice::ChooseMinion(old)),
        "older than its host"
    );
    e.choose(index_of(&req, Choice::ChooseMinion(young)));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[c.index()].host, Some(young));
}

/// Spirit Marionette [obe]: "Ⓓ Bleed with +1 bleed and lock a ready minion
/// controlled by your prey (even if the target of the bleed is changed)."
/// The bleed for 2, then a ready minion of the prey locked, chosen as it
/// resolves; a torpid one no choice.
#[test]
fn spirit_marionette_obedience_bleeds_and_locks_a_prey_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let u = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let torpid = e.add_minion(SeatId(1), plain("Ventrue", 2), 1, true);
    hand(&mut e, 0, &["Spirit Marionette"]);
    pick(&mut e, play(0, 1, a));
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(
        !req.options.contains(&Choice::ChooseMinion(torpid)),
        "ready only"
    );
    assert!(!req.options.contains(&Choice::Pass), "mandatory");
    e.choose(index_of(&req, Choice::ChooseMinion(u)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[u.index()].locked);
    assert!(!e.state.minions[t.index()].locked);
    assert_eq!(e.state.seats[1].pool, 8, "bled for 2");
}

/// Spirit Marionette [OBE]: "Ⓓ Take control of a ready unlocked minion until
/// the end of the next action. That minion must bleed your prey unless they
/// must hunt." Its forced bleed the next action here: back after it.
/// [LSJ 20011216-2] "The mandatory bleed action must be performed before
/// any non-mandatory actions": no Pass and no other action offered;
/// [LSJ 20021121-2] "can be performed by playing a card (eg. Computer
/// Hacking)": offered; [LSJ 20030219] "After resolution" effects (the prey's
/// Travelers Obey the Tenets) before its return; [TOM 19960226-1]
/// requirements do not apply to the minion taken.
#[test]
fn spirit_marionette_control_lasts_until_its_own_bleed_ends() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Travelers Obey the Tenets", None);
    hand(&mut e, 0, &["Spirit Marionette", "Computer Hacking"]);
    pick(&mut e, act(0, 2, a, Target::Minion(v)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[v.index()].owner, SeatId(0), "taken");
    assert!(
        !req.options.contains(&Choice::Pass),
        "the forced bleed first"
    );
    assert!(
        !req.options.contains(&bleed(b)),
        "no other action before it"
    );
    assert!(req.options.contains(&bleed(v)));
    assert!(
        req.options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, actor: Some(x), .. } if *x == v)),
        "Computer Hacking: {:?}",
        req.options
    );
    e.choose(index_of(&req, bleed(v)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[v.index()].owner,
        SeatId(1),
        "back after its bleed"
    );
    assert_eq!(blood(&e, v), 3, "the after-resolution damage first");
    assert!(req.options.contains(&bleed(b)), "the phase goes on");
}

/// Spirit Marionette [OBE], "until the end of the next action": any
/// minion's next action; the forced bleed, mandatory, offered beside the
/// player's own empty vampire's hunt, in the player's order: the hunt
/// taken first, the minion goes back as it ends, unbled (owner, rules-qa
/// 124.218).
#[test]
fn spirit_marionette_control_ends_with_the_next_action_any_minions() {
    for hunt_first in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
        let b = e.add_minion(SeatId(0), plain("Brujah", 1), 1, false);
        let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Spirit Marionette", "Computer Hacking"]);
        pick(&mut e, act(0, 2, a, Target::Minion(v)));
        request(&mut e);
        // Empty by the action's end: it must hunt.
        e.state.minions[b.index()].blood = 0;
        let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[v.index()].owner, SeatId(0), "taken");
        let hunt = Choice::TakeAction {
            minion: b,
            action: ActionKind::Hunt,
            target: None,
        };
        assert!(req.options.contains(&hunt) && req.options.contains(&bleed(v)));
        assert!(!req.options.contains(&Choice::Pass));
        e.choose(index_of(&req, if hunt_first { hunt } else { bleed(v) }));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[v.index()].owner, SeatId(1), "back");
        assert_eq!(e.state.seats[1].pool, if hunt_first { 10 } else { 9 });
    }
}

/// Spirit Marionette [OBE]: [ANK 20210124] [LSJ 20070411] [LSJ 20090514]
/// "If the action is unblocked when it resolves and the target is locked,
/// the action 'fizzles'"; [LSJ 20031010] [LSJ 20090319] [ANK 20180907]
/// "Actions that choose the vampire cannot be played against him (eg. …
/// Spirit Marionette)": a Secure Haven's vampire no target.
#[test]
fn spirit_marionette_fizzles_on_a_locked_target_and_spares_a_haven() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let h = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    in_play(&mut e, 1, "Secure Haven", Some(h));
    hand(&mut e, 0, &["Spirit Marionette"]);
    let req = request(&mut e);
    assert!(
        !req.options.contains(&act(0, 2, a, Target::Minion(h))),
        "Secure Haven"
    );
    pick(&mut e, act(0, 2, a, Target::Minion(v)));
    request(&mut e);
    e.state.minions[v.index()].locked = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(e.state.minions[v.index()].owner, SeatId(1));
}

/// Spirit Marionette [OBE]: [TOM 19960226-1] "Requirements do not apply
/// when taking control of a card already in play": the prey's Screamer
/// (requiring a Malkavian) taken by a Brujah's Methuselah.
#[test]
fn spirit_marionette_takes_an_ally_whatever_its_requirements() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let s = e.add_ally(SeatId(1), find_library("Screamer"));
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Spirit Marionette"]);
    pick(&mut e, act(0, 2, a, Target::Minion(s)));
    // Its forced bleed, the one option, elided: it bled its own Methuselah
    // and went back.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9, "bled by its taker's");
    assert_eq!(e.state.minions[s.index()].owner, SeatId(1));
}

/// Spirit Marionette [OBE]: [RTR 20020501] "If the bleed is canceled, the
/// vampire still returns to its original controller": its Computer Hacking
/// canceled as played (Evil Eye [MAL]), it goes back.
#[test]
fn spirit_marionette_canceled_bleed_still_returns_the_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.minions[w.index()].made_infernal = true;
    in_play(&mut e, 1, "Maleficia", Some(w));
    in_play(&mut e, 1, "Maleficia", Some(w));
    hand(&mut e, 0, &["Spirit Marionette", "Computer Hacking"]);
    hand(&mut e, 1, &["Evil Eye"]);
    pick(&mut e, act(0, 2, a, Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[v.index()].owner, SeatId(0));
    pick(&mut e, play(0, 0, v));
    pick_when(&mut e, act(0, 1, w, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[v.index()].owner, SeatId(1), "back");
    assert!(ash_has(&e, 0, "Computer Hacking"), "canceled");
}

/// Spirit Marionette [OBE], Temporary control: [ANK 20181110] "The stolen
/// minion is removed from the game when he should return to a Methuselah
/// who has been ousted": its bleed ousts its own Methuselah, then it is
/// removed.
#[test]
fn spirit_marionette_minion_removed_when_its_methuselah_is_ousted() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["obe", "obe"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Spirit Marionette"]);
    e.state.seats[1].pool = 1;
    pick(&mut e, act(0, 2, a, Target::Minion(v)));
    // Its forced bleed, the one option, elided.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert!(!e.state.minions[v.index()].in_play, "removed");
}

/// Approximation of Loyalty [PRE], Lucita, Mithras: [LSJ 20060106] "A card
/// played 'as if from hand' can be canceled 'as played'": a reaction played
/// from The Erciyes Fragments (as if from hand) offers each its cancel.
#[test]
fn a_reaction_played_from_the_erciyes_fragments_is_canceled_as_played() {
    for canceller in ["Approximation of Loyalty", "Lucita", "Mithras"] {
        let mut e = game(3);
        e.add_minion(SeatId(2), plain("Ventrue", 2), 4, false);
        let a = if canceller == "Approximation of Loyalty" {
            let a = e.add_minion(SeatId(0), textless(|c| c.capacity >= 7, 0), 4, false);
            e.state.minions[a.index()].gained_levels = vec!["pre", "pre"];
            a
        } else {
            // Lucita's advanced card, Mithras's G3: their cancels.
            let card = CRYPT
                .iter()
                .position(|c| {
                    c.name == canceller && (c.adv || canceller == "Mithras" && c.group == 3)
                })
                .expect("the canceller") as u16;
            e.add_minion(SeatId(0), card, 4, false)
        };
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let r = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 3, false);
        e.state.minions[r.index()].gained_levels = vec!["dom"];
        let frag = in_play(&mut e, 1, "Erciyes Fragments, The", None);
        e.state.cards[frag.index()]
            .stored
            .push(find_library("Deflection"));
        hand(&mut e, 0, &["Approximation of Loyalty"]);
        pick(&mut e, bleed(a));
        let stored = |o: &Choice| matches!(o, Choice::Play { actor: Some(x), .. } if *x == r);
        for _ in 0..20 {
            let req = request(&mut e);
            if let Some(i) = req.options.iter().position(stored) {
                e.choose(i);
                break;
            }
            e.choose(fallback(&req));
        }
        let req = request(&mut e);
        let cancel = req.options.iter().any(|o| match o {
            Choice::Play {
                card: 0, mode: 1, ..
            } => canceller == "Approximation of Loyalty",
            Choice::UseAbility {
                source: AbilitySource::Vampire(x),
                ..
            } => *x == a,
            _ => false,
        });
        assert!(cancel, "{canceller}: {:?} {:?}", req.window, req.options);
    }
}

/// Persistent Echo, Evil Eye: [LSJ 20060106] "A card played 'as if from
/// hand' can be canceled 'as played'": a Choir played from the Echo is
/// canceled by Evil Eye [MAL] as it is played, the Echo burned all the same.
#[test]
fn persistent_echo_stored_card_canceled_as_played() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), plain("Gangrel", 0), 4, false);
    e.add_minion(SeatId(0), plain("Gangrel", 1), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[w.index()].made_infernal = true;
    in_play(&mut e, 1, "Maleficia", Some(w));
    in_play(&mut e, 1, "Maleficia", Some(w));
    let echo = in_play_mode(&mut e, 0, "Persistent Echo", None, 1);
    e.state.cards[echo.index()]
        .stored
        .push(find_library("Choir"));
    hand(&mut e, 1, &["Evil Eye"]);
    let req = request(&mut e);
    let i = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::Play { mode: 0, actor: Some(x), .. } if *x == g))
        .expect("the stored Choir");
    e.choose(i);
    pick_when(&mut e, act(0, 1, w, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Choir"), "canceled");
    assert!(ash_has(&e, 0, "Persistent Echo"));
    assert!(!e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == find_library("Choir")));
}

/// Bollix, Charismatic Aura, Fae Contortion, Sword of the Archangel, Hide
/// the Mind, Bliss, Predator's Transformation, Taking the Skin: Minion,
/// Meditative Grove: [LSJ 20060106] "A card played 'as if from hand' can be
/// canceled 'as played'": the acting vampire's combat card played from The
/// Erciyes Fragments (a grapple, a card requiring Auspex, a frenzy card on
/// the blocker) offers the blocker's side each its cancel.
#[test]
fn a_combat_card_played_from_the_erciyes_fragments_is_canceled_as_played() {
    for (canceller, stored, disc) in [
        ("Bollix", "Immortal Grapple", "pot"),
        ("Charismatic Aura", "Immortal Grapple", "pot"),
        ("Fae Contortion", "Immortal Grapple", "pot"),
        ("Sword of the Archangel", "Immortal Grapple", "pot"),
        ("Hide the Mind", "Psychic Assault", "aus"),
        ("Bliss", "Terror Frenzy", "ani"),
        ("Predator's Transformation", "Terror Frenzy", "ani"),
        ("Taking the Skin: Minion", "Terror Frenzy", "ani"),
        ("Meditative Grove", "Terror Frenzy", "ani"),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec![disc];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let b = match canceller {
            "Bollix" => e.add_minion(SeatId(1), of_sect(Sect::Anarch, 4..=11, 0), 4, false),
            "Sword of the Archangel" | "Meditative Grove" => {
                e.add_minion(SeatId(1), plain("Salubri", 0), 4, false)
            }
            _ => e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false),
        };
        e.state.minions[b.index()].gained_levels = vec!["pre", "aus", "myt", "obf", "dom", "abo"];
        let in_play_canceller = match canceller {
            "Sword of the Archangel" => Some(in_play(&mut e, 1, canceller, Some(b))),
            "Meditative Grove" => Some(in_play(&mut e, 1, canceller, None)),
            _ => {
                hand(&mut e, 1, &[canceller]);
                None
            }
        };
        let frag = in_play(&mut e, 0, "Erciyes Fragments, The", None);
        e.state.cards[frag.index()]
            .stored
            .push(find_library(stored));
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(b));
        let from_store = |o: &Choice| matches!(o, Choice::Play { actor: Some(x), card, .. } if *x == a && *card == 0);
        let mut played = false;
        for _ in 0..30 {
            let req = request(&mut e);
            if let Some(i) = req.options.iter().position(from_store) {
                e.choose(i);
                played = true;
                break;
            }
            e.choose(fallback(&req));
        }
        assert!(played, "{canceller}: the stored {stored}");
        let req = request(&mut e);
        let cancel = req.options.iter().any(|o| match o {
            Choice::Play { actor, .. } => *actor == Some(b) && in_play_canceller.is_none(),
            Choice::UseAbility {
                source: AbilitySource::Card(c),
                ..
            } => Some(*c) == in_play_canceller,
            _ => false,
        });
        assert!(cancel, "{canceller}: {:?} {:?}", req.window, req.options);
    }
}

/// Sudden Reversal, Wash, Santaleous: [LSJ 20060106] "A card played 'as if
/// from hand' can be canceled 'as played'": a master card played from The
/// Erciyes Fragments offers each its cancel.
#[test]
fn a_master_played_from_the_erciyes_fragments_is_canceled_as_played() {
    for canceller in ["Sudden Reversal", "Wash", "Santaleous"] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let s = if canceller == "Santaleous" {
            Some(e.add_minion(SeatId(1), find_card("Santaleous"), 6, false))
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
            hand(&mut e, 1, &[canceller]);
            None
        };
        let frag = in_play(&mut e, 0, "Erciyes Fragments, The", None);
        e.state.cards[frag.index()]
            .stored
            .push(find_library("Minion Tap"));
        e.at_master_phase();
        let req = request(&mut e);
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Play { target: Target::Minion(x), .. } if *x == a))
            .unwrap_or_else(|| panic!("{canceller}: the stored master {:?}", req.options));
        e.choose(i);
        let req = request(&mut e);
        let req = if req.window == WindowLabel::PlayTerms {
            e.choose(0);
            request(&mut e)
        } else {
            req
        };
        let cancel = req.options.iter().any(|o| match o {
            Choice::Play { actor: None, .. } => s.is_none() && req.seat == SeatId(1),
            Choice::UseAbility {
                source: AbilitySource::Vampire(x),
                ..
            } => Some(*x) == s,
            _ => false,
        });
        assert!(cancel, "{canceller}: {:?} {:?}", req.window, req.options);
    }
}

/// Emergency Preparations, Jan Pieterzoon: [LSJ 20060106] "A card played
/// 'as if from hand' can be canceled 'as played'": a Gehenna card (Thirst)
/// played from The Erciyes Fragments offers each's cancel of a Gehenna
/// card as it is played.
#[test]
fn a_gehenna_card_played_from_the_erciyes_fragments_is_canceled_as_played() {
    for canceller in ["Emergency Preparations", "Jan Pieterzoon"] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let jan = if canceller == "Jan Pieterzoon" {
            Some(e.add_minion(SeatId(1), crypt_card("Jan Pieterzoon", true), 6, false))
        } else {
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
            hand(&mut e, 1, &[canceller]);
            None
        };
        let frag = in_play(&mut e, 0, "Erciyes Fragments, The", None);
        let thirst = find_library("Thirst");
        e.state.cards[frag.index()].stored.push(thirst);
        let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Play { actor: None, .. }))
            .unwrap_or_else(|| panic!("{canceller}: the stored Thirst {:?}", req.options));
        e.choose(i);
        let req = request(&mut e);
        let req = if req.window == WindowLabel::PlayTerms {
            e.choose(0);
            request(&mut e)
        } else {
            req
        };
        let cancel = req.options.iter().any(|o| match o {
            Choice::Play { actor: None, .. } => jan.is_none() && req.seat == SeatId(1),
            Choice::UseAbility {
                source: AbilitySource::Vampire(x),
                ..
            } => Some(*x) == jan,
            _ => false,
        });
        assert!(cancel, "{canceller}: {:?} {:?}", req.window, req.options);
    }
}

/// Raw Recruit: [LSJ 20011212] [LSJ 20060815] "The action 'to remove that
/// vampire from the game' is an action to put a vampire into play, a sterile
/// vampire cannot do it": a Tremere made sterile (Muricia's `made_sterile`, as
/// Barrenness makes any vampire sterile) is not offered it; another Tremere is.
#[test]
fn raw_recruit_is_no_action_for_a_sterile_vampire() {
    let mut e = game(2);
    let tremere = |n| textless(|c| c.clan == "Tremere" && (4..=8).contains(&c.capacity), n);
    let t = e.add_minion(SeatId(0), tremere(0), 6, false);
    let u = e.add_minion(SeatId(0), tremere(1), 6, false);
    e.state.minions[t.index()].made_sterile = zvtes::state::Sparse(1);
    let p = e.add_minion(SeatId(0), find_card("Porphyry"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let victim = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=8, 0), 2, true);
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
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let rr = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.held_vampire.is_some())
        .map(|i| CardId(i as u32))
        .expect("the vampire held");
    assert!(
        !req.options.contains(&use_on(rr, 0, Some(t), Target::None)),
        "sterile"
    );
    assert!(req.options.contains(&use_on(rr, 0, Some(u), Target::None)));
}

/// Esprit de Corps [POT][PRE], Andrew Stuart: [LSJ 20060106] "A card played
/// 'as if from hand' can be canceled 'as played'": an action card from The
/// Erciyes Fragments (Steal the Mind [dom] at the prey's minion) offers the
/// prey's Esprit de Corps its cancel; Andrew Stuart's Theft of Vitae from
/// it is canceled by his own reveal of a Blood Sorcery card.
#[test]
fn an_action_card_played_from_the_erciyes_fragments_is_canceled_as_played() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let esprit = in_play_mode(&mut e, 1, "Esprit de Corps", None, 1);
    let frag = in_play(&mut e, 0, "Erciyes Fragments, The", None);
    e.state.cards[frag.index()]
        .stored
        .push(find_library("Steal the Mind"));
    let req = request(&mut e);
    let i = req
        .options
        .iter()
        .position(|o| {
            matches!(o, Choice::Play { mode: 0, actor: Some(x), target: Target::Minion(y), .. }
                if *x == a && *y == t)
        })
        .expect("the stored Steal the Mind");
    e.choose(i);
    let req = request(&mut e);
    assert!(
        req.options.contains(&use_on(esprit, 0, None, Target::None)),
        "{:?} {:?}",
        req.window,
        req.options
    );
    // Andrew Stuart.
    let mut e = game(2);
    let andrew = e.add_minion(SeatId(0), find_card("Andrew Stuart"), 4, false);
    e.state.minions[andrew.index()].gained_levels = vec!["tha"];
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    fill_hand(&mut e, 0);
    stack(&mut e, 0, &["Theft of Vitae"], 0);
    let frag = in_play(&mut e, 0, "Erciyes Fragments, The", None);
    let hand_ = find_library("Rutor's Hand");
    e.state.cards[frag.index()].stored.push(hand_);
    let req = request(&mut e);
    let i = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::Play { actor: Some(x), .. } if *x == andrew))
        .expect("the stored Rutor's Hand");
    e.choose(i);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, andrew), 3, "canceled, his blood burned");
    assert!(
        !e.state.cards.iter().any(|c| c.in_play && c.card == hand_),
        "canceled"
    );
}

/// Anonymous Freight, Children of Stone [tha], Magic of the Smith, Delivery
/// Truck: [LSJ 20080803] "When choosing a card whose effect depend on
/// Discipline level, the vampire decides in respect to his disciplines the
/// normal way": Tinglestripe ([myt], [MYT]) equipped by a vampire with
/// superior Mytherceria at [MYT], his player's choice (rules-qa 125.17), by
/// one with basic at [myt] only, the found card's version never asked.
#[test]
fn an_equip_action_takes_tinglestripes_version_by_the_vampires_disciplines() {
    let tingle = find_library("Tinglestripe");
    let superior_version = |o: &Choice| {
        matches!(o, Choice::EmployFromHand { card, mode: 1 } | Choice::Version { card, mode: 1 }
            if *card == tingle)
    };
    for superior in [false, true] {
        let levels: &[&str] = if superior {
            &["myt", "myt", "tha"]
        } else {
            &["myt", "tha"]
        };
        for card in [
            "Anonymous Freight",
            "Children of Stone",
            "Magic of the Smith",
        ] {
            let mut e = game(2);
            let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
            e.state.minions[a.index()].gained_levels = levels.to_vec();
            e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
            e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
            if card == "Magic of the Smith" {
                hand(&mut e, 0, &[card]);
                fill_hand(&mut e, 0);
                stack(&mut e, 0, &["Tinglestripe"], 2);
            } else {
                hand(&mut e, 0, &[card, "Tinglestripe"]);
                fill_hand(&mut e, 0);
                stack(&mut e, 0, &[], 2);
            }
            pick(&mut e, play(0, 0, a));
            let mut offered = false;
            loop {
                let req = request(&mut e);
                if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                    break;
                }
                if let Some(i) = req.options.iter().position(superior_version) {
                    offered = true;
                    e.choose(i);
                } else if let Some(i) = req
                    .options
                    .iter()
                    .position(|o| *o == Choice::FindCard(tingle))
                {
                    e.choose(i);
                } else {
                    e.choose(fallback(&req));
                }
            }
            assert_eq!(offered, superior, "{card}");
            let t = placed_on(&e, "Tinglestripe", Some(a));
            assert_eq!(e.state.cards[t.index()].mode, u8::from(superior), "{card}");
        }
        // Delivery Truck: the card on it equipped as an action.
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = levels.to_vec();
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let h = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
        let truck = in_play(&mut e, 0, "Delivery Truck", Some(h));
        e.state.cards[truck.index()].stored.push(tingle);
        let req = request(&mut e);
        let i = req
            .options
            .iter()
            .position(|o| {
                matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), actor: Some(x), .. }
                    if *c == truck && *x == a)
            })
            .expect("the truck's equip");
        e.choose(i);
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            match req.options.iter().position(superior_version) {
                Some(i) => e.choose(i),
                None => e.choose(fallback(&req)),
            }
        }
        let t = placed_on(&e, "Tinglestripe", Some(a));
        assert_eq!(
            e.state.cards[t.index()].mode,
            u8::from(superior),
            "Delivery Truck"
        );
    }
}

/// Rules-qa 125.17 (owner, 2026-11-19): a card a search or a storage card
/// puts in play enters at the version its player chooses among those the
/// vampire's Disciplines meet, as from hand: with superior Mytherceria, a
/// Tinglestripe found by Magic of the Smith or on Delivery Truck, with
/// superior Animalism a Raven Spy found by Muricia's Call, each offered at
/// both versions and taken at the basic one.
#[test]
fn a_searched_card_enters_at_the_version_its_player_chooses() {
    /// Pass until the version request of `card`, take its basic version,
    /// then pass to the minion phase.
    fn choose_basic(e: &mut Engine, card: u16, find: Choice) {
        let both = vec![
            Choice::Version { card, mode: 0 },
            Choice::Version { card, mode: 1 },
        ];
        loop {
            let req = request(e);
            assert_ne!(req.window, WindowLabel::MinionPhase, "no version asked");
            if req.options == both {
                e.choose(0);
                break;
            }
            match req.options.iter().position(|o| *o == find) {
                Some(i) => e.choose(i),
                None => e.choose(fallback(&req)),
            }
        }
        until(e, SeatId(0), WindowLabel::MinionPhase);
    }
    let tingle = find_library("Tinglestripe");
    for truck in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["myt", "myt"];
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        if truck {
            let h = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
            let t = in_play(&mut e, 0, "Delivery Truck", Some(h));
            e.state.cards[t.index()].stored.push(tingle);
            let req = request(&mut e);
            let i = req
                .options
                .iter()
                .position(|o| {
                    matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), actor: Some(x), .. }
                        if *c == t && *x == a)
                })
                .expect("the truck's equip");
            e.choose(i);
        } else {
            hand(&mut e, 0, &["Magic of the Smith"]);
            fill_hand(&mut e, 0);
            stack(&mut e, 0, &["Tinglestripe"], 2);
            pick(&mut e, play(0, 0, a));
        }
        choose_basic(&mut e, tingle, Choice::FindCard(tingle));
        let t = placed_on(&e, "Tinglestripe", Some(a));
        assert_eq!(e.state.cards[t.index()].mode, 0, "truck {truck}");
    }
    let spy = find_library("Raven Spy");
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["ani", "ani"];
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Muricia's Call"]);
    fill_hand(&mut e, 0);
    stack(&mut e, 0, &["Raven Spy"], 3);
    pick(&mut e, play(0, 0, a));
    let take = Choice::TakeFrom {
        zone: zvtes::decision::Zone::Library,
        card: spy,
    };
    choose_basic(&mut e, spy, take);
    let c = placed_on(&e, "Raven Spy", Some(a));
    assert_eq!(
        (
            e.state.cards[c.index()].mode,
            e.state.cards[c.index()].counters
        ),
        (0, 1)
    );
}

// ---- Round 32 vampires, Iniko to Olugbenga ----------------------------------

/// The crypt card `name`, advanced or not.
#[track_caller]
fn crypt_card(name: &str, adv: bool) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.adv == adv)
        .unwrap_or_else(|| panic!("{name}")) as u16
}

/// The abilities of vampire `m` offered.
fn vuses(req: &zvtes::decision::DecisionRequest, m: MinionId) -> Vec<Choice> {
    req.options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == m))
        .collect()
}

/// Pass (or hand strike) until vampire `m`'s ability `ability` is offered,
/// and take it.
fn pick_vuse(e: &mut Engine, m: MinionId, ability: u8) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(|o| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability: a, .. }
                if *v == m && *a == ability)
        }) {
            e.choose(i);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("ability {ability} of {m:?} never offered");
}

/// Any ability of vampire `m`.
fn is_vuse(o: &Choice, m: MinionId) -> bool {
    matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), .. } if *v == m)
}

/// A weapon already on `host` (no play): its id.
fn equip(e: &mut Engine, name: &str, host: MinionId) -> CardId {
    let owner = e.state.minions[host.index()].owner;
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), owner, Some(host), 0));
    CardId(e.state.cards.len() as u32 - 1)
}

/// The traits of round 32's vampires Iniko to Olugbenga: the text-only seven
/// ("Laibon.") have no definition, Laibon and untitled; Nkule Galadima and
/// Olugbenga are magaji, Mr. Noir scarce, Muricia and Maria Stone sterile,
/// The Kikiyaon has Flight.
#[test]
fn the_round_32_c_vampires_carry_their_printed_traits() {
    let mut e = game(2);
    for name in [
        "Ismitta",
        "Kamaria",
        "Kamau Jafari",
        "Kenyatta",
        "Lumumba",
        "Meno Ngari",
        "Misrak",
    ] {
        let card = find_card(name);
        assert!(zvtes::cards::vampire(card).is_none(), "{name}: no text");
        let m = e.add_minion(SeatId(0), card, 1, false);
        assert_eq!(e.state.sect_of(m), Some(Sect::Laibon), "{name}");
        assert_eq!(e.state.title_of(m), Title::None, "{name}");
    }
    for name in ["Nkule Galadima", "Olugbenga"] {
        let m = e.add_minion(SeatId(0), find_card(name), 1, false);
        assert_eq!(e.state.title_of(m), Title::Magaji, "{name}");
    }
    assert!(CRYPT[find_card("Mr. Noir") as usize].scarce);
    for card in [find_card("Muricia"), crypt_card("Maria Stone", true)] {
        let m = e.add_minion(SeatId(0), card, 1, false);
        assert!(e.state.is_sterile(m), "{}", CRYPT[card as usize].name);
    }
    let k = e.add_minion(SeatId(0), find_card("Kikiyaon, The"), 1, false);
    assert!(e.state.has_discipline(k, "flight"));
}

/// Iniko, The Black Lion: "Laibon: +1 stealth." On every action of his.
#[test]
fn iniko_acts_with_plus_one_stealth() {
    let mut e = game(2);
    let i = e.add_minion(SeatId(0), find_card("Iniko, The Black Lion"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(i));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
}

/// Jack Dawson's fixture: Jack (6 blood) bearing a Blade of Bellona bleeds,
/// a 6-blood vampire blocks.
fn jack_game() -> (Engine, MinionId, MinionId, CardId) {
    let mut e = game(2);
    let jack = e.add_minion(SeatId(0), crypt_card("Jack Dawson", true), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    let blade = equip(&mut e, "Blade of Bellona", jack);
    pick(&mut e, bleed(jack));
    pick_when(&mut e, block(z));
    (e, jack, z, blade)
}

/// Jack Dawson: "Jack can burn 1 blood to get 1 additional strike
/// (limited), only usable to strike with the a weapon he used this round."
/// Struck with his Blade of Bellona: for 1 blood, one more strike, the
/// Blade's or none (no hand strike); limited, not offered again.
#[test]
fn jack_dawson_strikes_again_with_his_weapon() {
    let (mut e, jack, z, blade) = jack_game();
    let req = loop {
        let req = request(&mut e);
        if req.options.iter().any(|o| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 0, .. } if *c == blade)
        }) {
            break req;
        }
        e.choose(fallback(&req));
    };
    let i = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), ability: 0, .. } if *c == blade))
        .expect("the Blade's strike");
    e.choose(i);
    pick_vuse(&mut e, jack, 0);
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(req.window, WindowLabel::CombatStrike);
    assert!(
        !req.options.contains(&Choice::HandStrike),
        "{:?}",
        req.options
    );
    assert!(req.options.contains(&Choice::Pass));
    let i = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == blade))
        .expect("the Blade's strike again");
    e.choose(i);
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        is_vuse(o, jack)
    });
    // 1 blood for the strike, 1 mended from Z's hand strike; Z took 2 + 2.
    assert_eq!((blood(&e, jack), blood(&e, z)), (6 - 1 - 1, 6 - 2 - 2));
}

/// Jack Dawson (rules-qa 124.122): a weapon "used" is one struck with: his
/// Blade borne but a hand strike made, the ability is never offered.
#[test]
fn jack_dawson_gets_no_extra_strike_after_a_hand_strike() {
    let (mut e, jack, _, _) = jack_game();
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        is_vuse(o, jack)
    });
    assert_eq!(blood(&e, jack), 6 - 1);
}

/// Jack Dawson, "a weapon he used this round": any use (owner, rules-qa
/// 124.122): his Bundi borne, a hand strike made, Bundi preventing 1 of
/// the opposing Bang Nakh's strike: his ability offered. A vampire that
/// cannot use equipment (The Wildebeest on him) cannot use Bundi's
/// prevention either: neither is offered.
#[test]
fn jack_dawson_strikes_again_with_a_weapon_used_to_prevent() {
    for barred in [false, true] {
        let mut e = game(2);
        let jack = e.add_minion(SeatId(0), crypt_card("Jack Dawson", true), 6, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
        let bundi = equip(&mut e, "Bundi", jack);
        let nakh = equip(&mut e, "Bang Nakh — Tiger's Claws", z);
        if barred {
            in_play(&mut e, 0, "Wildebeest, The", Some(jack));
        }
        pick(&mut e, bleed(jack));
        pick_when(&mut e, block(z));
        let used = |o: &Choice, c: CardId, a: u8| matches!(o, Choice::UseAbility { source: AbilitySource::Card(x), ability, .. } if *x == c && *ability == a);
        let (mut prevented, mut offered) = (false, false);
        for _ in 0..60 {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            let pick_at = |f: &dyn Fn(&Choice) -> bool| req.options.iter().position(f);
            let i = if let Some(i) = pick_at(&|o| is_vuse(o, jack)) {
                offered = true;
                i
            } else if let Some(i) = pick_at(&|o| used(o, bundi, 1)) {
                prevented = true;
                i
            } else if req.seat == SeatId(1) && req.window == WindowLabel::CombatStrike {
                pick_at(&|o| used(o, nakh, 0)).expect("the Nakh's strike")
            } else if req.seat == SeatId(0) && req.window == WindowLabel::CombatStrike && !offered {
                index_of(&req, Choice::HandStrike)
            } else {
                fallback(&req)
            };
            e.choose(i);
        }
        assert_eq!((prevented, offered), (!barred, !barred), "{barred}");
    }
}

/// Jack Dawson: "He gets first strike in combat with a werewolf." A
/// 1-life Renegade Garou blocking him burns before its strike lands.
#[test]
fn jack_dawson_strikes_first_against_a_werewolf() {
    let mut e = game(2);
    let jack = e.add_minion(SeatId(0), crypt_card("Jack Dawson", true), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let garou = e.add_ally(SeatId(1), find_library("Renegade Garou"));
    e.state.minions[garou.index()].blood = 1;
    pick(&mut e, bleed(jack));
    pick_when(&mut e, block(garou));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[garou.index()].in_play, "burned");
    assert_eq!(blood(&e, jack), 4, "its strike never landed");
}

/// Jibade el-Bahrawi: "Jibade gets +1 stealth when recruiting, employing,
/// or taking an action to put a vampire in play. Allies and retainers cost
/// him 1 less blood or pool." The Summoning [PRE] (+1 stealth action) at
/// stealth 2, its ECTU Operative recruited for 1 pool.
/// [LSJ 20091207] "Gets +1 stealth when using The Summoning at superior
/// [PRE]." (ledger rows 2115, 2116)
#[test]
fn jibade_el_bahrawi_sneaks_the_summoning_and_recruits_cheaper() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), crypt_card("Jibade el-Bahrawi", false), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Summoning, The"]);
    fill_hand(&mut e, 0);
    let ectu = find_library("ECTU Operative");
    for c in [ectu, find_library("Form of Mist")] {
        e.state.seats[0].library.push(c);
    }
    pick(&mut e, play(0, 1, j));
    request(&mut e);
    assert_eq!(e.action().expect("the Summoning").stealth(&e.state), 2);
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: ectu,
        },
    );
    request(&mut e);
    assert!(e
        .state
        .minions
        .iter()
        .any(|v| v.in_play && v.kind.ally_card() == Some(ectu)));
    assert_eq!(e.state.seats[0].pool, 10 - (2 - 1));
}

/// Jibade el-Bahrawi: The Embrace (a +1 stealth action putting a vampire
/// in play) at stealth 2.
#[test]
fn jibade_el_bahrawi_embraces_with_stealth() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), crypt_card("Jibade el-Bahrawi", false), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Embrace, The"]);
    pick(&mut e, play(0, 0, j));
    request(&mut e);
    assert_eq!(e.action().expect("the Embrace").stealth(&e.state), 2);
}

/// Jibade el-Bahrawi merged (rulebook "Advanced"): the base card's
/// recruit stealth with the advanced card's Aye shield, in play. The
/// Embrace at stealth 2; his ally's damage prevented by an Aye locked.
#[test]
fn jibade_merged_keeps_both_texts() {
    let merged = |e: &mut Engine| {
        let j = e.add_minion(SeatId(0), crypt_card("Jibade el-Bahrawi", true), 5, false);
        e.state.minions[j.index()].merged = Some(crypt_card("Jibade el-Bahrawi", false));
        assert_eq!(e.state.sect_of(j), Some(Sect::Laibon));
        j
    };
    let mut e = game(2);
    let j = merged(&mut e);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Embrace, The"]);
    pick(&mut e, play(0, 0, j));
    request(&mut e);
    assert_eq!(e.action().expect("the Embrace").stealth(&e.state), 2);

    let mut e = game(2);
    let j = merged(&mut e);
    let ally = e.add_ally(SeatId(0), find_library("Gravebound Drone"));
    let b = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let aye = in_play(&mut e, 0, "Aye", Some(j));
    let life = blood(&e, ally);
    pick_when(&mut e, bleed(ally));
    pick_when(&mut e, block(b));
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(j),
            ability: 1,
            actor: None,
            target: Target::Minion(ally),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ally), life, "prevented");
    assert!(e.state.cards[aye.index()].locked);
    assert_eq!(blood(&e, j), 5 - 1);
}

/// Jubal: "Jubal gets +1 intercept when attempting to block a Ministry."
/// Against a Ministry's bleed 1, a Brujah's 0.
#[test]
fn jubal_intercepts_the_ministry() {
    for (clan, n) in [("Ministry", 1), ("Brujah", 0)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan(clan, 4..=8, 0), 4, false);
        e.add_minion(SeatId(0), plain("Toreador", 0), 4, false);
        let jubal = e.add_minion(SeatId(1), find_card("Jubal"), 4, false);
        pick(&mut e, bleed(a));
        request(&mut e);
        let act = e.action().expect("bleed");
        assert_eq!(act.intercept_of(&e.state, jubal), n, "{clan}");
    }
}

/// Kamiri wa Itherero: "Younger vampires must burn 1 blood to attempt to
/// block Kamiri." A younger vampire pays 1; one as old pays nothing.
#[test]
fn kamiri_tolls_younger_blockers() {
    for (old, paid) in [(false, 1), (true, 0)] {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), crypt_card("Kamiri wa Itherero", false), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let card = if old {
            fixture_vampire(|c| c.capacity >= 10, 0)
        } else {
            of_sect(Sect::Camarilla, 4..=6, 0)
        };
        let z = e.add_minion(SeatId(1), card, 4, false);
        pick(&mut e, bleed(k));
        pick_when(&mut e, block(z));
        assert_eq!(blood(&e, z), 4 - paid, "older: {old}");
    }
}

/// Kamiri wa Itherero: "If an ally blocks Kamiri and combat occurs, burn
/// the ally at the end of the action." Renegade Garou blocks, survives the
/// combat, burns as the action ends — not burned by Kamiri: his Aye stays,
/// Taking the Skin: Minion is never offered. A vampire blocking is not
/// burned.
/// [RTR 19960124] [LSJ 20090922] [ANK 20181022] [ANK 20220916] "If the
/// minion is burned because of a referendum or as a side-effect of the
/// action, this does not count as the acting minion burning him (eg. …
/// Kamiri wa Itherero's ability)." (ledger row 1641)
#[test]
fn kamiri_burns_a_blocking_ally_after_combat() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), crypt_card("Kamiri wa Itherero", false), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let aye = in_play(&mut e, 0, "Aye", Some(k));
    let garou = e.add_ally(SeatId(1), find_library("Renegade Garou"));
    hand(&mut e, 0, &["Taking the Skin: Minion"]);
    fill_hand(&mut e, 0);
    pick(&mut e, bleed(k));
    pick_when(&mut e, block(garou));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
    assert!(!e.state.minions[garou.index()].in_play, "burned");
    assert!(ash_has(&e, 1, "Renegade Garou"));
    assert!(e.state.cards[aye.index()].in_play, "not Kamiri's burn");
    assert!(e.state.seats[0]
        .hand
        .contains(&find_library("Taking the Skin: Minion")));
}

/// Kamiri wa Itherero: the blocking ally that fought him burns as the
/// action ends though Kamiri was burned in that combat: triggered as the
/// combat occurs, not lost with its source (D33, rules-qa 74.2).
#[test]
fn kamiri_burned_in_combat_still_burns_the_blocking_ally() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), crypt_card("Kamiri wa Itherero", false), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let garou = e.add_ally(SeatId(1), find_library("Renegade Garou"));
    e.state.minions[garou.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    e.state.minions[garou.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    pick(&mut e, bleed(k));
    pick_when(&mut e, block(garou));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.minions[k.index()].in_play,
        "Kamiri burned in combat"
    );
    assert!(!e.state.minions[garou.index()].in_play, "the ally burned");
    assert!(ash_has(&e, 1, "Renegade Garou"));
}

/// Kamiri wa Itherero: a vampire blocking him is not burned.
#[test]
fn kamiri_spares_a_blocking_vampire() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), crypt_card("Kamiri wa Itherero", false), 8, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(k));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[z.index()].in_play);
}

/// Kamiri wa Itherero merged: "[MERGED] Once each action, Kamiri may burn
/// 1 blood to give an acting Laibon +1 stealth." Another Laibon of his
/// bleeds, a block attempt needs stealth: 1 blood, +1 stealth; the block
/// fails. Not for a Camarilla acting vampire.
#[test]
fn kamiri_merged_gives_an_acting_laibon_stealth() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), crypt_card("Kamiri wa Itherero", true), 4, false);
    e.state.minions[k.index()].merged = Some(crypt_card("Kamiri wa Itherero", false));
    let laibon = e.add_minion(SeatId(0), of_sect(Sect::Laibon, 4..=8, 0), 4, false);
    let cam = e.add_minion(SeatId(0), of_sect(Sect::Camarilla, 4..=8, 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(laibon));
    pick_when(&mut e, block(z));
    pick_vuse(&mut e, k, 1);
    assert_eq!(blood(&e, k), 4 - 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1, "the block failed");
    pick(&mut e, bleed(cam));
    pick_when(&mut e, block(z));
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability: 1, .. } if *v == k),
    );
}

/// The Kikiyaon: "Once each combat, The Kikiyaon may burn a blood to get a
/// maneuver." Used: 1 blood; not offered again.
#[test]
fn the_kikiyaon_burns_blood_to_maneuver() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Kikiyaon, The"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(k));
    pick_when(&mut e, block(z));
    pick_vuse(&mut e, k, 0);
    assert_eq!(blood(&e, k), 4 - 1);
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        is_vuse(o, k)
    });
}

/// Maria Stone (advanced): "-1 strength." Her hand strike does no damage.
#[test]
fn maria_stone_strikes_weaker() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), crypt_card("Maria Stone", true), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4, "strength 0");
}

/// Maskini: "If Maskini attempts an unsuccessful action, … he burns 1
/// blood at the end of that action." Blocked: 1 more than the combat cost
/// him; a successful bleed: none.
#[test]
fn maskini_burns_after_a_blocked_action() {
    for blocked in [true, false] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Maskini"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        pick(&mut e, bleed(m));
        if blocked {
            pick_when(&mut e, block(z));
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let want = if blocked { 4 - 1 - 1 } else { 4 };
        assert_eq!(blood(&e, m), want, "blocked: {blocked}");
    }
}

/// Maskini: "… or attempts to block but is not successful, he burns 1
/// blood at the end of that action." His attempt short of a hunt's
/// stealth: 1 blood as the action ends, not before.
#[test]
fn maskini_burns_after_a_failed_block() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let m = e.add_minion(SeatId(1), find_card("Maskini"), 4, false);
    e.notes = Some(Vec::new());
    let before = e.table();
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_when(&mut e, block(m));
    assert_eq!(blood(&e, m), 4, "not yet");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, m), 4 - 1);
    // Told with its cause, Maskini.
    let burns: Vec<_> = noted_changes(&mut e, before)
        .into_iter()
        .filter(|&(_, at, n)| at == Err(m) && n < 0)
        .map(|(cause, ..)| cause)
        .collect();
    let by = zvtes::notes::Cause::Effect(zvtes::notes::Source::Minion(m));
    assert_eq!(burns, [Some(by)]);
}

/// Matata: "Matata gets an optional press each combat. +1 strength." One
/// press without a card, not offered again in that combat; hand strikes
/// of 2.
#[test]
fn matata_presses_once_and_hits_harder() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Matata"), 6, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 6, false);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(z));
    pick_when(&mut e, Choice::UsePress);
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::UsePress,
    );
    assert_eq!(blood(&e, z), 6 - 2 - 2, "two rounds of 2");
}

/// Mr. Noir: "After Mr. Noir is burned in combat, you gain 2 pool." With
/// 1 blood, blocking Matata's 3-aggravated Kali's Fang, he burns (rulebook
/// "Wounded"): his controller gains 2, the other Methuselah nothing.
#[test]
fn mr_noir_pays_his_controller_when_burned_in_combat() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Mr. Noir"), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Matata"), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    let fang = equip(&mut e, "Kali's Fang", z);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(z));
    pick_when(&mut e, block(n));
    let fang_strike = |o: &Choice| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == fang);
    let req = loop {
        let req = request(&mut e);
        if req.options.iter().any(fang_strike) {
            break req;
        }
        e.choose(fallback(&req));
    };
    e.choose(req.options.iter().position(fang_strike).expect("the Fang"));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[n.index()].in_play, "burned");
    assert_eq!(pool(&e), vec![10 + 2, 10]);
}

/// Mr. Noir burned out of combat (his contest yielded): no pool.
#[test]
fn mr_noir_burned_out_of_combat_pays_nothing() {
    let mut e = game(2);
    let card = find_card("Mr. Noir");
    let n = e.add_minion(SeatId(0), card, 2, false);
    let rival = e.add_minion(SeatId(1), card, 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.state.minions[n.index()].contested = true;
    e.state.minions[rival.index()].contested = true;
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let before = e.state.seats[0].pool;
    pick(&mut e, Choice::YieldContest(zvtes::game::Contest::Card(n)));
    assert!(!e.state.minions[n.index()].in_play, "burned");
    assert_eq!(e.state.seats[0].pool, before);
}

/// Muricia: "During your unlock phase, a Gangrel antitribu you control may
/// burn 1 blood to become a sterile Ahrimane and gain 1 level of Spiritus
/// [spi]. +1 strength." Her controller's Gangrel antitribu: 1 blood, an
/// Ahrimane, sterile (The Embrace no longer offered to it), inferior
/// Spiritus; once that unlock phase; another Methuselah's never a target.
#[test]
fn muricia_makes_a_gangrel_antitribu_a_sterile_ahrimane() {
    let mut e = game(2);
    let mu = e.add_minion(SeatId(0), find_card("Muricia"), 4, false);
    let g = e.add_minion(SeatId(0), of_clan("Gangrel antitribu", 4..=8, 0), 4, false);
    let g2 = e.add_minion(SeatId(0), of_clan("Gangrel antitribu", 4..=8, 1), 4, false);
    let theirs = e.add_minion(SeatId(1), of_clan("Gangrel antitribu", 4..=8, 2), 4, false);
    assert_eq!(
        e.state.printed_mod(mu, Stat::Strength, Default::default()),
        1
    );
    let on = |t: MinionId| Choice::UseAbility {
        source: AbilitySource::Vampire(mu),
        ability: 0,
        actor: None,
        target: Target::Minion(t),
    };
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&on(g2)), "{:?}", req.options);
    assert!(!req.options.contains(&on(theirs)));
    pick(&mut e, on(g));
    assert_eq!(blood(&e, g), 4 - 1);
    assert_eq!(e.state.minions[g.index()].clan(), Some("Ahrimane"));
    assert!(e.state.is_sterile(g));
    assert_eq!(e.state.discipline_level(g, "spi"), 1);
    let req = request(&mut e);
    assert!(vuses(&req, mu).is_empty(), "once: {:?}", req.options);
    to_minion_phase(&mut e, SeatId(0));
    hand(&mut e, 0, &["Embrace, The"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, g)), "sterile");
    assert!(req.options.contains(&play(0, 0, g2)));
}

/// Muricia (rules-qa 124.146): used from torpor (D25); a
/// Gangrel antitribu with inferior Spiritus raised to superior (capped
/// there); the clan, sterility and level outliving Muricia (D33).
#[test]
fn muricia_from_torpor_raises_spiritus_for_good() {
    let mut e = game(2);
    let mu = e.add_minion(SeatId(0), find_card("Muricia"), 4, true);
    let g = e.add_minion(SeatId(0), of_clan("Gangrel antitribu", 4..=8, 0), 4, false);
    e.state.minions[g.index()].gained_levels = vec!["spi"];
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(mu),
            ability: 0,
            actor: None,
            target: Target::Minion(g),
        },
    );
    assert_eq!(e.state.discipline_level(g, "spi"), 2, "superior");
    e.burn(mu);
    request(&mut e);
    assert_eq!(e.state.minions[g.index()].clan(), Some("Ahrimane"));
    assert!(e.state.is_sterile(g));
    assert_eq!(e.state.discipline_level(g, "spi"), 2);
}

/// Nangila Were: "Nangila may enter combat with a minion controlled by
/// another Methuselah as a Ⓓ action. While Nangila is ready, your hand
/// size is one card larger." Another Methuselah's minion only; hand size 8,
/// 7 with her in torpor.
#[test]
fn nangila_were_rushes_and_widens_the_hand() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), find_card("Nangila Were"), 4, false);
    let own = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let rush = |t| Choice::UseAbility {
        source: AbilitySource::Vampire(n),
        ability: 0,
        actor: Some(n),
        target: Target::Minion(t),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&rush(z)), "{:?}", req.options);
    assert!(!req.options.contains(&rush(own)));
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
    e.state.minions[n.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
}

/// Nkule Galadima: "Lasombra cannot cast votes or ballots in referendums
/// called by Nkule. While he is ready, if you burn the Edge for a vote, you
/// gain an additional 2 votes." His referendum: no Lasombra votes, his
/// controller's own included; his Edge vote 3. Called by another vampire:
/// Lasombra vote; the Edge counts 3 with him ready in any referendum (rules-qa
/// 124.151, 80.10), 1 with him in torpor.
#[test]
fn nkule_bars_lasombra_votes_and_prizes_the_edge() {
    for (nkule_calls, torpid, lasombra_vote, edge_votes) in [
        (true, false, false, 3),
        (false, true, true, 1),
        (false, false, true, 3),
    ] {
        let mut e = game(2);
        let nk = e.add_minion(SeatId(0), find_card("Nkule Galadima"), 6, torpid);
        let caller = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        e.state.minions[caller.index()].granted_title = Some(Title::Votes(1));
        let mine = e.add_minion(SeatId(0), of_clan("Lasombra", 4..=8, 0), 4, false);
        e.state.minions[mine.index()].granted_title = Some(Title::Votes(1));
        let theirs = e.add_minion(SeatId(1), of_clan("Lasombra", 4..=8, 1), 4, false);
        e.state.minions[theirs.index()].granted_title = Some(Title::Votes(1));
        e.state.seats[0].has_edge = true;
        hand(&mut e, 0, &["Ancilla Empowerment"]);
        pick(&mut e, play(0, 0, if nkule_calls { nk } else { caller }));
        to_referendum(&mut e);
        let mut seen = false;
        let mut edge = None;
        for _ in 0..40 {
            let req = request(&mut e);
            if req.window != WindowLabel::ReferendumPolling {
                break;
            }
            seen |= req.options.iter().any(|o| {
                [mine, theirs]
                    .iter()
                    .any(|&l| *o == votes(l, true) || *o == votes(l, false))
            });
            if let Some(i) = req
                .options
                .iter()
                .position(|o| *o == Choice::CastEdge { favor: true })
            {
                e.choose(i);
                edge = e.referendum().and_then(|r| r.records.last().map(|v| v.n));
                continue;
            }
            e.choose(fallback(&req));
        }
        let case = (nkule_calls, torpid);
        assert_eq!(seen, lasombra_vote, "Lasombra votes {case:?}");
        assert_eq!(edge, Some(edge_votes), "the Edge {case:?}");
    }
}

/// Olugbenga: "If you are contesting a vampire, you can use the blood on
/// that vampire or on Olugbenga as pool to pay for that contest." His
/// controller's contest: 1 pool, 1 blood from the contested vampire or 1
/// from him; paid from him, the pool unchanged. The rival, without him,
/// pays pool, unasked.
#[test]
fn olugbenga_pays_a_contest_with_blood() {
    let mut e = game(2);
    let o = e.add_minion(SeatId(0), find_card("Olugbenga"), 4, false);
    let (own, _) = contested_pair(&mut e, 1);
    pick(&mut e, Choice::Pass);
    never_offered_where(&mut e, SeatId(0), WindowLabel::UnlockAbilities, |c| {
        matches!(c, Choice::BurnBloodFrom { .. })
    });
    assert_eq!(e.state.seats[1].pool, 10 - 1, "the rival paid pool");
    pick(&mut e, Choice::PayContest(zvtes::game::Contest::Card(own)));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(
        req.options,
        vec![
            Choice::BurnPool,
            Choice::BurnBloodFrom { minion: own },
            Choice::BurnBloodFrom { minion: o },
        ]
    );
    e.choose(index_of(&req, Choice::BurnBloodFrom { minion: o }));
    assert_eq!(e.state.seats[0].pool, 10);
    assert_eq!((blood(&e, o), blood(&e, own)), (4 - 1, 3));
}

/// Olugbenga (rules-qa 124.155): a unit of blood from a vampire with none is
/// not offered: the contested vampire empty, pool or Olugbenga's blood only;
/// Olugbenga empty too, pool only (paid unasked).
#[test]
fn olugbenga_offers_no_blood_where_there_is_none() {
    let mut e = game(2);
    let o = e.add_minion(SeatId(0), find_card("Olugbenga"), 4, false);
    let (own, _) = contested_pair(&mut e, 1);
    e.state.minions[own.index()].blood = 0;
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, Choice::PayContest(zvtes::game::Contest::Card(own)));
    let req = request(&mut e);
    assert_eq!(
        req.options,
        vec![Choice::BurnPool, Choice::BurnBloodFrom { minion: o }]
    );
}

/// Olugbenga: "He gets +1 intercept during bleed actions." Any
/// Methuselah's bleed: 1; a hunt: 0.
#[test]
fn olugbenga_intercepts_bleeds() {
    for (bleeds, n) in [(true, 1), (false, 0)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
        e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
        let o = e.add_minion(SeatId(1), find_card("Olugbenga"), 4, false);
        pick(
            &mut e,
            if bleeds {
                bleed(a)
            } else {
                action(a, ActionKind::Hunt)
            },
        );
        request(&mut e);
        assert_eq!(e.action().expect("action").intercept_of(&e.state, o), n);
    }
}
