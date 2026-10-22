//! Legacy cards, round 18 (`ADDED_BATCHES`, slice 16ad): the cards players
//! asked for (the server's /admin/unsupported counts) and the TWDA greedy's
//! steps 1-3 from them. Readings in rules-qa Batch 85. Windows with a
//! single option are elided, so the scripts only stop where a real
//! decision exists.

use zvtes::cards::{Stat, Target};
use zvtes::crypt::CRYPT;
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::{CardType, LIBRARY};
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

/// A crypt card of `clan` with capacity in `caps`, no printed-text
/// definition.
fn of_clan(clan: &str, caps: std::ops::RangeInclusive<u8>) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.clan == clan
                && caps.contains(&c.capacity)
                && c.title == zvtes::crypt::Title::None
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .unwrap_or_else(|| panic!("no {clan} of capacity {caps:?}")) as u16
}

/// A crypt card of `sect` with capacity in `caps`, untitled, no
/// printed-text definition.
fn of_sect(sect: zvtes::crypt::Sect, caps: std::ops::RangeInclusive<u8>) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.sect == Some(sect)
                && caps.contains(&c.capacity)
                && c.title == zvtes::crypt::Title::None
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .unwrap_or_else(|| panic!("no {sect:?} of capacity {caps:?}")) as u16
}

/// `m` gets `disc` at superior level (the fixture's Great Beast slot).
fn superior(e: &mut Engine, m: MinionId, disc: &'static str) {
    let slots = &mut e.state.minions[m.index()].beast_disciplines;
    let free = slots.iter().position(Option::is_none).expect("a free slot");
    slots[free] = Some(disc);
}

fn against(opponent: MinionId) -> PrintedCtx {
    PrintedCtx {
        opponent: Some(opponent),
        ..PrintedCtx::default()
    }
}

/// The first library card of only `ty` whose Disciplines include `discs`.
fn card_with(ty: CardType, discs: &[&str]) -> u16 {
    LIBRARY
        .iter()
        .position(|c| c.types == [ty] && discs.iter().all(|d| c.disciplines.contains(d)))
        .unwrap_or_else(|| panic!("no {ty:?} with {discs:?}")) as u16
}

fn playing(card: u16, mode: u8) -> PrintedCtx {
    PrintedCtx {
        card: Some((card, mode)),
        ..PrintedCtx::default()
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

/// `card`'s `ability` used by its Methuselah (`actor` the user, if any).
fn use_on(source: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(source),
        ability,
        actor,
        target,
    }
}

// ---- Vampires -----------------------------------------------------------------

// Antoinette DuChamp: "Action cards and strike cards that require Celerity
// cost Antoinette an additional blood." Action cards of every type
// (rules-qa 60.4), a dodge a strike card (42.2), the mode played (38.4):
// Side Strike's dodge yes, its additional strike no.
#[test]
fn antoinette_pays_more_for_celerity_actions_and_strikes() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Antoinette DuChamp"), 1, false);
    let st = &e.state;
    let side_strike = find_library("Side Strike");
    let more = |ctx| st.printed_mod(a, Stat::BloodCostMore, ctx);
    assert_eq!(more(playing(side_strike, 0)), 1, "a [cel] dodge");
    assert_eq!(more(playing(side_strike, 1)), 0, "an additional strike");
    let action = card_with(CardType::Action, &["cel"]);
    assert_eq!(more(playing(action, 0)), 1, "a [cel] action card");
    let modifier = card_with(CardType::ActionModifier, &["cel"]);
    assert_eq!(more(playing(modifier, 0)), 0, "not an action card");
    let pot_strike = find_library("Conflagration");
    assert_eq!(more(playing(pot_strike, 0)), 0, "not Celerity");
}

// Chandler: +1 intercept blocking another Gangrel; Christine: -1 intercept
// during bleed actions; Navar cannot block older vampires.
#[test]
fn chandler_christine_and_navar_printed_blocks() {
    let mut e = game(2);
    let chandler = e.add_minion(SeatId(1), find_card("Chandler Hungerford"), 3, false);
    let christine = e.add_minion(SeatId(1), find_card("Christine Boscacci"), 2, false);
    let navar = e.add_minion(SeatId(1), find_card("Navar McClaren"), 1, false);
    let gangrel = e.add_minion(SeatId(0), of_clan("Gangrel", 4..=6), 4, false);
    let toreador = e.add_minion(SeatId(0), of_clan("Toreador", 4..=6), 4, false);
    let st = &e.state;
    assert_eq!(
        st.printed_mod(chandler, Stat::Intercept, against(gangrel)),
        1
    );
    assert_eq!(
        st.printed_mod(chandler, Stat::Intercept, against(toreador)),
        0
    );
    let on = |kind| PrintedCtx {
        kind: Some(kind),
        ..PrintedCtx::default()
    };
    assert_eq!(
        st.printed_mod(christine, Stat::Intercept, on(ActionKind::Bleed)),
        -1
    );
    assert_eq!(
        st.printed_mod(christine, Stat::Intercept, on(ActionKind::Hunt)),
        0
    );
    assert!(st.cannot_block(navar, gangrel), "an older vampire");
    let younger = e.add_minion(SeatId(0), find_card("Smudge the Ignored"), 1, false);
    assert!(!e.state.cannot_block(navar, younger), "not an older one");
}

// Eurayle: "can unlock a minion as an action; that action gets +2 stealth
// if you control the minion." Another Methuselah's minion: directed at its
// controller, no stealth (PIB 20121210).
// [PIB 20121210]
#[test]
fn eurayle_unlocks_a_minion_with_stealth_for_her_own() {
    let mut e = game(2);
    let eurayle = e.add_minion(SeatId(0), find_card("Eurayle Gelasia Mylonas"), 6, false);
    let mine = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.state.minions[mine.index()].locked = true;
    let theirs = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    e.state.minions[theirs.index()].locked = true;
    pick(&mut e, own_at(eurayle, 0, Target::Minion(mine)));
    let a = e.action().expect("action");
    assert_eq!(a.stealth(&e.state), 2, "her own minion");
    assert_eq!(a.directed_seat(&e.state), None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[mine.index()].locked);
    let mut e2 = game(2);
    let eurayle = e2.add_minion(SeatId(0), find_card("Eurayle Gelasia Mylonas"), 6, false);
    e2.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let theirs = e2.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    e2.state.minions[theirs.index()].locked = true;
    pick(&mut e2, own_at(eurayle, 0, Target::Minion(theirs)));
    let a = e2.action().expect("action");
    assert_eq!(a.stealth(&e2.state), 0);
    assert_eq!(a.directed_seat(&e2.state), Some(SeatId(1)));
}

// Fergus: "can enter combat with a locked Sabbat vampire as a Ⓓ action":
// a ready locked Sabbat vampire, not an unlocked one nor a Camarilla one.
#[test]
fn fergus_enters_combat_with_a_locked_sabbat_vampire() {
    let mut e = game(2);
    let fergus = e.add_minion(SeatId(0), find_card("Fergus Alexander"), 3, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let sabbat = e.add_minion(
        SeatId(1),
        of_sect(zvtes::crypt::Sect::Sabbat, 4..=6),
        4,
        false,
    );
    let unlocked = e.add_minion(
        SeatId(1),
        of_sect(zvtes::crypt::Sect::Sabbat, 4..=6),
        4,
        false,
    );
    let cam = e.add_minion(
        SeatId(1),
        of_sect(zvtes::crypt::Sect::Camarilla, 4..=6),
        4,
        false,
    );
    e.state.minions[sabbat.index()].locked = true;
    e.state.minions[cam.index()].locked = true;
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&own_at(fergus, 0, Target::Minion(sabbat))));
    assert!(!req
        .options
        .contains(&own_at(fergus, 0, Target::Minion(unlocked))));
    assert!(!req
        .options
        .contains(&own_at(fergus, 0, Target::Minion(cam))));
}

// Julius: "If Julius goes into torpor, burn him." He goes there first
// (its torpor riders fire), then burns.
#[test]
fn julius_burns_once_in_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let julius = e.add_minion(SeatId(1), find_card("Julius"), 0, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(julius));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[julius.index()];
    assert!(!v.in_play, "burned");
}

// Smudge: "After Smudge diablerizes an older vampire for the first time, he
// gets +1 strength." For good, once.
#[test]
fn smudge_gets_strength_after_an_older_diablerie() {
    let mut e = game(2);
    let smudge = e.add_minion(SeatId(0), find_card("Smudge the Ignored"), 1, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let victim = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 2, true);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    assert_eq!(
        e.state
            .printed_mod(smudge, Stat::Strength, PrintedCtx::default()),
        0
    );
    pick(
        &mut e,
        Choice::TakeAction {
            minion: smudge,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[victim.index()].in_play);
    assert_eq!(
        e.state
            .printed_mod(smudge, Stat::Strength, PrintedCtx::default()),
        1
    );
}

// Greensleeves' "give Humo +1 bleed": Humo is not in the pool, the text
// inert (Alan Sovereign's precedent): no ability to offer.
#[test]
fn greensleeves_offers_nothing_without_humo() {
    let card = find_card("Greensleeves");
    let def = zvtes::cards::vampire(card).expect("defined");
    assert!(def.abilities.is_empty() && def.printed.is_empty());
    assert!(!CRYPT.iter().any(|c| c.name == "Humo"));
}

// ---- Titles -------------------------------------------------------------------

/// The `nth` untitled vampire matching `f`, no printed-text definition.
fn textless(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            f(c) && c.title == zvtes::crypt::Title::None
                && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a textless vampire")
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

// Crusade: Chicago: the Archbishop of Chicago on the Sabbat vampire calling
// it; not playable by a Camarilla one.
#[test]
fn crusade_chicago_titles_its_sabbat_caller() {
    use zvtes::crypt::{Sect, Title};
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Sabbat), 0),
        4,
        false,
    );
    let cam = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Camarilla), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Crusade: Chicago"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, cam)));
    pass_referendum(&mut e, a);
    request(&mut e);
    assert_eq!(e.state.title_of(a), Title::Archbishop("Chicago"));
    // Its player claiming the title: no second Crusade (rules-qa 76.2).
    hand(&mut e, 0, &["Crusade: Chicago"]);
    let b = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Sabbat), 1),
        4,
        false,
    );
    assert!(!request(&mut e).options.contains(&play(0, 0, b)));
    let on = e
        .state
        .cards
        .iter()
        .find(|c| c.card == find_library("Crusade: Chicago"))
        .expect("the card");
    assert!(on.in_play && on.host == Some(a) && !on.effect_only);
}

// Crusade: Istanbul / London: a Tzimisce (a Nosferatu antitribu) unlocks
// during the discard phase, its clan read then; another clan does not.
#[test]
fn crusades_unlock_their_clan_at_the_discard_phase() {
    use zvtes::crypt::Sect;
    for (card, clan, unlocks) in [
        ("Crusade: Istanbul", "Tzimisce", true),
        ("Crusade: London", "Nosferatu antitribu", true),
        ("Crusade: Istanbul", "Lasombra", false),
    ] {
        let mut e = game(2);
        let a = e.add_minion(
            SeatId(0),
            textless(|c| c.sect == Some(Sect::Sabbat) && c.clan == clan, 0),
            4,
            false,
        );
        e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
        e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
        hand(&mut e, 0, &[card]);
        pass_referendum(&mut e, a);
        // A card in hand keeps the discard phase from eliding.
        hand(&mut e, 0, &["Dodge"]);
        until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
        assert_eq!(!e.state.minions[a.index()].locked, unlocks, "{card} {clan}");
    }
}

// Nosferatu Justicar: a ready Camarilla Nosferatu chosen (LSJ 20041026),
// not a Sabbat Nosferatu antitribu; each Nosferatu +1 vote.
// [LSJ 20041026] [LSJ 20100129] [ANK 20191228]
#[test]
fn nosferatu_justicar_titles_a_camarilla_nosferatu() {
    use zvtes::crypt::{Sect, Title};
    let mut e = game(2);
    let n = e.add_minion(
        SeatId(0),
        textless(
            |c| c.clan == "Nosferatu" && c.sect == Some(Sect::Camarilla),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Nosferatu Justicar"]);
    pass_referendum(&mut e, n);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.title_of(n), Title::Justicar("Nosferatu"));
    // No Camarilla Nosferatu: not played (LSJ 20100129).
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Nosferatu antitribu", 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Nosferatu Justicar"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, a)));
}

// ---- Bleeds and the Edge -------------------------------------------------------

// Leverage: "Burn the Edge to get +1 bleed that does not count against the
// limit … If you would get the Edge, it is burned instead." Not without
// the Edge; the successful bleed gives it to no one.
#[test]
fn leverage_burns_the_edge_for_a_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Leverage"]);
    pick(&mut e, bleed(a));
    // Without the Edge: never offered.
    none_until(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
    assert!(e.state.seats[0].has_edge, "the bleed's Edge");
    let before = pool(&e)[1];
    let b = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    pick(&mut e, bleed(b));
    pick_when(&mut e, play(0, 0, b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], before - 2, "+1 bleed");
    assert!(e.state.seats.iter().all(|s| !s.has_edge), "burned instead");
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

fn pick_use_of(e: &mut Engine, source: CardId, ability: u8) {
    pick_matching(e, |o| {
        matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), ability: a, .. }
            if *c == source && *a == ability)
    });
}

// The Secret Library of Alexandria: "You may draw a card each time you
// successfully bleed your prey. Discard to your hand size afterward."
#[test]
fn secret_library_draws_on_a_successful_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Nosferatu", 4..=6), 4, false);
    e.add_minion(SeatId(0), of_clan("Nosferatu", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let lib = in_play(&mut e, 0, "Secret Library of Alexandria, The", None);
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(find_library("Dodge"));
    pick(&mut e, bleed(a));
    pick_use_of(&mut e, lib, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let s = &e.state.seats[0];
    assert!(s.library.is_empty(), "drawn");
    assert_eq!(s.ash.len(), 1, "discarded down");
    assert_eq!(e.state.hand_load(SeatId(0)), e.state.hand_size(SeatId(0)));
}

// Shanty Town Hunting Ground: 1 blood to a ready vampire during the unlock
// phase.
#[test]
fn shanty_town_feeds_a_ready_vampire() {
    let card = zvtes::cards::lookup(find_library("Shanty Town Hunting Ground")).expect("defined");
    assert_eq!(card.abilities.len(), 1);
    assert!(card.abilities[0]
        .effects
        .contains(&zvtes::cards::Effect::HuntingGroundBlood(1)));
}

// ---- Events ---------------------------------------------------------------------

// NSA Trio: "During the first combat between an acting vampire and a
// blocking vampire each turn, neither combatant may end combat as a strike
// in the first round." Form of Mist [PRO] not offered then; the turn's
// second such combat free (LSJ 20050215).
// [LSJ 20050215]
#[test]
fn nsa_trio_bars_combat_ends_in_the_first_block_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("PRO", 0), 6, false);
    let b = e.add_minion(SeatId(0), vampire_with("PRO", 1), 6, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 6, false);
    let y = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 6, false);
    in_play(&mut e, 1, "NSA Trio", None);
    hand(&mut e, 0, &["Form of Mist", "Form of Mist"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    none_until(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { mode: 1, .. })
    });
    let (before_b, before_y) = (blood(&e, b), blood(&e, y));
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(y));
    pick_when(&mut e, play(0, 1, b));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, b), before_b, "combat ended: no strike landed");
    assert_eq!(blood(&e, y), before_y);
}

// NSA Trio's burn: a discard phase action of a Methuselah whose minions
// announced no action this turn.
#[test]
fn nsa_trio_burns_for_a_methuselah_who_did_not_act() {
    for acted in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
        e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
        e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
        let trio = in_play(&mut e, 1, "NSA Trio", None);
        hand(&mut e, 0, &["Dodge"]);
        if acted {
            pick(&mut e, action(a, ActionKind::Hunt));
        }
        let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
        assert_eq!(uses(&req, trio.0).is_empty(), acted, "acted: {acted}");
        if !acted {
            pick(&mut e, uses(&req, trio.0)[0]);
            assert!(!e.state.cards[trio.index()].in_play);
        }
    }
}

// ---- Torpor -------------------------------------------------------------------

// Sacrificial Lamb [obf]: burn a vampire in torpor you control, gain blood
// equal to its capacity (excess ignored), its equipment taken if wished;
// not diablerie: no blood hunt, no Discipline.
#[test]
fn sacrificial_lamb_burns_a_torpid_vampire_for_its_capacity() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 6, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let t = e.add_minion(SeatId(0), of_clan("Brujah", 3..=3), 0, true);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let vest = CardId(e.state.cards.len() as u32);
    e.state.cards.push(CardInPlay::new(
        find_library("Kevlar Vest"),
        SeatId(0),
        Some(t),
        0,
    ));
    hand(&mut e, 0, &["Sacrificial Lamb"]);
    let cap = e.state.capacity_of(a).expect("capacity");
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Minion(t),
        },
    );
    pick_when(&mut e, Choice::ChooseCard(vest.0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_play, "burned");
    assert_eq!(e.state.cards[vest.index()].host, Some(a), "taken");
    assert_eq!(blood(&e, a), (6 - 3 + 3).min(cap), "its capacity's blood");
    assert!(
        e.state.minions[a.index()].diablerie_turn.is_none(),
        "not diablerie"
    );
}

// Sacrificial Lamb [OBF] Ⓓ: another Methuselah's vampire in torpor, a
// directed action; [obf] never offers one.
#[test]
fn sacrificial_lamb_superior_reaches_another_torpor_region() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("OBF", 0), 6, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let t = e.add_minion(SeatId(1), of_clan("Ventrue", 3..=6), 0, true);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Sacrificial Lamb"]);
    let at = |mode| Choice::Play {
        card: 0,
        mode,
        actor: Some(a),
        target: Target::Minion(t),
    };
    let req = request(&mut e);
    assert!(!req.options.contains(&at(0)));
    pick(&mut e, at(1));
    request(&mut e);
    assert_eq!(
        e.action().expect("action").directed_seat(&e.state),
        Some(SeatId(1))
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[t.index()].in_play);
}

// ---- Locations ------------------------------------------------------------------

// Storage Annex: not played as the only card in hand (LSJ 20100527); a hand
// card put face down on it as it is played, hidden from the others, seen by
// its controller; exchanged during a later master phase.
// [LSJ 20100527] [PIB 20150224]
#[test]
fn storage_annex_stores_a_card_and_exchanges_it() {
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    e.at_master_phase();
    hand(&mut e, 0, &["Storage Annex"]);
    // Its master phase elided: nothing to play.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    e.at_master_phase();
    hand(&mut e, 0, &["Storage Annex", "Dodge"]);
    e.state.seats[0].library.push(find_library("Leverage"));
    pick(&mut e, master_play(0));
    pick(&mut e, Choice::FindCard(find_library("Dodge")));
    let annex = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Storage Annex"))
        .expect("in play");
    assert_eq!(e.state.cards[annex].stored, vec![find_library("Dodge")]);
    assert!(
        e.state.seats[0]
            .hand
            .iter()
            .any(|&c| c == find_library("Leverage")),
        "replaced"
    );
    let theirs = e.state.legal_view(SeatId(1));
    assert_eq!(theirs.cards[annex].stored, vec![zvtes::state::HIDDEN]);
    let mine = e.state.legal_view(SeatId(0));
    assert_eq!(mine.cards[annex].stored, vec![find_library("Dodge")]);
    // The next master phase: Leverage for the Dodge.
    let req = to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let swap = uses(&req, annex as u32);
    assert_eq!(swap.len(), 1);
    // Leverage the one card in hand: picked without a decision.
    assert_eq!(e.state.seats[0].hand, vec![find_library("Leverage")]);
    pick(&mut e, swap[0]);
    request(&mut e);
    assert_eq!(e.state.cards[annex].stored, vec![find_library("Leverage")]);
    assert!(e.state.seats[0]
        .hand
        .iter()
        .any(|&c| c == find_library("Dodge")));
}

// The Admonitions: "During your discard phase, control of The Admonitions
// passes counter-clockwise until it is controlled by a Methuselah who
// controls a ready Sabbat vampire (or is burned if there are none)"; the
// only one keeps it (LSJ 20031121).
// [LSJ 20031121]
#[test]
fn the_admonitions_passes_to_the_next_sabbat_methuselah() {
    use zvtes::crypt::Sect;
    // Seat 0's predator is seat 2 (no Sabbat), then seat 1 (Sabbat).
    for (sabbat_seats, holder) in [
        (&[0usize, 1][..], Some(1u8)),
        (&[0][..], Some(0)),
        (&[][..], None),
    ] {
        let mut e = game(3);
        for s in 0..3 {
            let crypt = if sabbat_seats.contains(&s) {
                textless(|c| c.sect == Some(Sect::Sabbat), s)
            } else {
                textless(|c| c.sect == Some(Sect::Camarilla), s)
            };
            e.add_minion(SeatId(s as u8), crypt, 4, false);
            e.add_minion(SeatId(s as u8), of_clan("Ventrue", 4..=6), 4, false);
        }
        let adm = in_play(&mut e, 0, "Admonitions, The", None);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let c = &e.state.cards[adm.index()];
        match holder {
            Some(h) => {
                assert!(c.in_play);
                assert_eq!(e.state.card_controller(adm), SeatId(h), "{sabbat_seats:?}");
            }
            None => assert!(!c.in_play, "burned"),
        }
    }
}

// The Admonitions' locks: discard a card with a ready Sabbat vampire, draw
// two with a ready Black Hand one.
#[test]
fn the_admonitions_locks_need_sabbat_and_black_hand() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Sabbat) && !c.black_hand, 0),
        4,
        false,
    );
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let adm = in_play(&mut e, 0, "Admonitions, The", None);
    hand(&mut e, 0, &["Dodge"]);
    let req = request(&mut e);
    let offered: Vec<u8> = uses(&req, adm.0)
        .iter()
        .map(|o| match o {
            Choice::UseAbility { ability, .. } => *ability,
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(offered, vec![0], "no Black Hand: no draw");
    e.add_minion(SeatId(0), textless(|c| c.black_hand, 0), 4, false);
    let req = request(&mut e);
    assert_eq!(uses(&req, adm.0).len(), 2);
}

// Communal Haven: Cathedral: during the master phase, lock to move blood
// and equipment between two ready Sabbat vampires — no retainer, no
// non-Sabbat receiver.
#[test]
fn communal_haven_moves_between_sabbat_vampires() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let from = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity >= 4, 0),
        4,
        false,
    );
    let to = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity >= 4, 1),
        1,
        false,
    );
    let cam = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Camarilla), 0),
        1,
        false,
    );
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let haven = in_play(&mut e, 0, "Communal Haven: Cathedral", None);
    let vest = CardId(e.state.cards.len() as u32);
    e.state.cards.push(CardInPlay::new(
        find_library("Kevlar Vest"),
        SeatId(0),
        Some(from),
        0,
    ));
    e.at_master_phase();
    pick(&mut e, use_on(haven, 0, None, Target::Minion(from)));
    // The one Sabbat receiver, chosen without a decision: the blood next.
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&set_x(2)));
    assert!(
        !req.options
            .iter()
            .any(|o| matches!(o, Choice::ChooseMinion(_))),
        "not {cam:?}"
    );
    e.choose(index_of(&req, set_x(2)));
    pick(&mut e, Choice::ChooseCard(vest.0));
    request(&mut e);
    assert_eq!((blood(&e, from), blood(&e, to)), (2, 3));
    assert_eq!(e.state.cards[vest.index()].host, Some(to));
    assert!(e.state.cards[haven.index()].locked);
    // One ready Sabbat vampire: no pair, not offered.
    let mut e = game(2);
    let from = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity >= 4, 0),
        4,
        false,
    );
    e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Camarilla), 0),
        1,
        false,
    );
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let haven = in_play(&mut e, 0, "Communal Haven: Cathedral", None);
    e.at_master_phase();
    hand(&mut e, 0, &["Dodge"]);
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&use_on(haven, 0, None, Target::Minion(from))));
}

// ---- Combat -------------------------------------------------------------------

// Conflagration: [pot] 2R, [dai] 1R aggravated, [DAI] 2R aggravated — the
// mode its player's Disciplines allow.
// [LSJ 20011204-3] [PIB 20130704]
#[test]
fn conflagration_strikes_by_discipline() {
    use zvtes::cards::{Effect, StrikeKind};
    let def = zvtes::cards::lookup(find_library("Conflagration")).expect("defined");
    let strikes: Vec<_> = def
        .modes
        .iter()
        .map(|m| match m.effects {
            [Effect::Strike(StrikeKind::Damage {
                n,
                ranged,
                aggravated,
            })] => (*n, *ranged, *aggravated),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        strikes,
        vec![(2, true, false), (1, true, true), (2, true, true)]
    );
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pot", 0), 6, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Conflagration"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 0, a));
    assert!(!req.options.contains(&play(0, 1, a)), "no Daimoinon");
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 2, "2 damage");
}

// Dodge: "Do not replace until after combat. Strike: dodge."
#[test]
fn dodge_is_replaced_after_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 1, &["Dodge"]);
    fill_hand(&mut e, 1);
    e.state.seats[1].library.push(find_library("Leverage"));
    let leverage = |e: &Engine| e.state.seats[1].hand.contains(&find_library("Leverage"));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, z));
    assert!(!leverage(&e), "not replaced during combat");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 4, "dodged");
    assert!(leverage(&e), "replaced after combat");
}

// Hatch the Viper: 1 environmental damage each round at close range ([PRO]
// [SER]: 2), the employer preventing 1 each combat.
// [ANK 20180612] [ANK 20211127]
#[test]
fn hatch_the_viper_damages_at_close_range_and_prevents() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let hatch = in_play(&mut e, 1, "Hatch the Viper", Some(z));
    e.state.cards[hatch.index()].counters = 1;
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_use_of(&mut e, hatch, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1 - 1, "a hand strike and the Viper's 1");
    assert_eq!(blood(&e, z), 4, "the hand strike prevented");
}

// Eternal Vigilance: put on the acting titled Sabbat vampire; during an
// action it burns 1 blood to unlock and attempt to block (ANK
// 20181122-2: once each action); burned as its vampire goes to torpor.
#[test]
fn eternal_vigilance_unlocks_to_block_and_burns_in_torpor() {
    use zvtes::crypt::{Sect, Title};
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 6, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let v = e.add_minion(
        SeatId(1),
        textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity >= 3, 0),
        3,
        false,
    );
    e.state.minions[v.index()].granted_title = Some(Title::Archbishop("Montreal"));
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let ev = in_play(&mut e, 1, "Eternal Vigilance", Some(v));
    e.state.minions[v.index()].locked = true;
    pick(&mut e, bleed(a));
    pick_use_of(&mut e, ev, 0);
    assert!(!e.state.minions[v.index()].locked);
    assert_eq!(blood(&e, v), 2, "1 blood");
    e.state.minions[v.index()].blood = 0;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[v.index()].in_torpor);
    assert!(!e.state.cards[ev.index()].in_play, "burned in torpor");
}

// Eternal Vigilance played: a ready archbishop, priscus, cardinal or regent
// only; on the acting vampire itself.
#[test]
fn eternal_vigilance_goes_on_its_titled_actor() {
    use zvtes::crypt::{Sect, Title};
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Sabbat), 0),
        4,
        false,
    );
    let untitled = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Sabbat), 1),
        4,
        false,
    );
    e.state.minions[a.index()].granted_title = Some(Title::Priscus);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Eternal Vigilance"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, untitled)));
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = e
        .state
        .cards
        .iter()
        .find(|c| c.card == find_library("Eternal Vigilance"))
        .expect("the card");
    assert!(c.in_play && c.host == Some(a) && !c.effect_only);
    assert!(!e.state.seats[0]
        .ash
        .contains(&find_library("Eternal Vigilance")));
}

// Hidden Lurker: after the block's combat, another ready unlocked vampire
// of the acting Methuselah locks and enters combat with the blocker, which
// cannot strike in its first round; not against a blocker gone to torpor
// (LSJ 20100604-1).
// [LSJ 20100604-1]
#[test]
fn hidden_lurker_sends_another_vampire_at_the_blocker() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let l = e.add_minion(SeatId(0), vampire_with("obf", 0), 3, false);
    let z = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Hidden Lurker"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let lurk = play(0, 0, l);
    let req = offering(&mut e, lurk);
    assert_eq!(req.window, WindowLabel::ActionAfterCombat);
    e.choose(index_of(&req, lurk));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[l.index()].locked);
    assert_eq!(blood(&e, l), 3, "the blocker could not strike");
    assert_eq!(blood(&e, z), 4 - 2, "two combats' hands");
    // The blocker in torpor: not offered.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 6, false);
    let l = e.add_minion(SeatId(0), vampire_with("obf", 0), 6, false);
    let z = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 0, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Hidden Lurker"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    none_until(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == play(0, 0, l)
    });
    assert!(e.state.minions[z.index()].in_torpor);
}

// ---- Equipment ----------------------------------------------------------------

// Helicopter: equipped, it comes locked; after a successful action, its
// bearer may lock it to unlock. Taken from another minion, locked too
// (LSJ 20100119).
#[test]
fn helicopter_locks_as_equipped_and_unlocks_its_bearer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 6, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Helicopter"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let heli = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.card == find_library("Helicopter"))
            .expect("in play") as u32,
    );
    assert!(e.state.cards[heli.index()].locked, "locked as equipped");
    assert!(e.state.minions[a.index()].locked);
    // Unlocked (its unlock phase), it unlocks its bearer after a hunt.
    e.state.cards[heli.index()].locked = false;
    e.state.minions[a.index()].locked = false;
    pick(&mut e, action(a, ActionKind::Hunt));
    pick_use_of(&mut e, heli, 0);
    assert!(!e.state.minions[a.index()].locked);
    assert!(e.state.cards[heli.index()].locked);
}

// Improvised Flamethrower: damage from the opposing strike at long range
// burns it and deals its bearer 2 aggravated damage; once each combat.
#[test]
fn improvised_flamethrower_backfires_at_long_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 6, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let flame = in_play(&mut e, 0, "Improvised Flamethrower", Some(a));
    let magnum = in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Cold Aura"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_use_of(&mut e, flame, 0);
    pick_use_of(&mut e, magnum, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[flame.index()].in_play, "burned");
    assert!(
        e.state.minions[a.index()].in_torpor,
        "the backfire's aggravated damage"
    );
    assert!(e.state.minions[z.index()].in_torpor, "the Flamethrower's");
}

// Creation Rites: by a ready non-sterile archbishop, priscus, cardinal or
// regent; a 1-capacity Sabbat vampire of the actor's clan that cannot act
// this turn, up to 1 blood moved from the actor, decided as it resolves
// (PIB 20150418).
// [PIB 20150418]
#[test]
fn creation_rites_makes_a_sabbat_vampire_of_the_actors_clan() {
    use zvtes::crypt::{Sect, Title};
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless(
            |c| c.sect == Some(Sect::Sabbat) && c.clan == "Tzimisce" && !c.sterile,
            0,
        ),
        4,
        false,
    );
    e.state.minions[a.index()].granted_title = Some(Title::Archbishop("Montreal"));
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Creation Rites"]);
    let before = e.state.minions.len();
    request(&mut e);
    let blood_before = blood(&e, a);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, set_x(1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let new = MinionId(before as u32);
    let v = &e.state.minions[new.index()];
    assert!(v.in_play && v.is_vampire());
    assert_eq!(v.clan(), Some("Tzimisce"));
    assert_eq!(e.state.sect_of(new), Some(Sect::Sabbat));
    assert_eq!(e.state.capacity_of(new), Some(1));
    assert_eq!(blood(&e, new), 1, "the blood moved");
    assert_eq!(
        blood(&e, a),
        blood_before - 1 - 1,
        "its cost, the blood moved"
    );
}

// ---- Rulings ------------------------------------------------------------------

// [LSJ 20100709] The range set long before range is determined (Cold Aura,
// as Neutral Guard or Sniper Rifle would): Outside the Hourglass [TEM]'s
// damage then triggers the Flamethrower.
#[test]
fn outside_the_hourglass_triggers_the_flamethrower_at_long_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 6, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    superior(&mut e, z, "tem");
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let flame = in_play(&mut e, 0, "Improvised Flamethrower", Some(a));
    hand(&mut e, 0, &["Cold Aura"]);
    hand(&mut e, 1, &["Outside the Hourglass"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 3, z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[flame.index()].in_play, "burned");
    assert!(
        e.state.minions[a.index()].in_torpor,
        "its aggravated damage"
    );
}

// [LSJ 20040802] "even if it is prevented": a Kevlar Vest preventing the gun
// damage, the Flamethrower still burns. Close range: no backfire.
#[test]
fn improvised_flamethrower_backfires_through_a_prevention_not_at_close_range() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("nec", 0), 6, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let flame = in_play(&mut e, 0, "Improvised Flamethrower", Some(a));
    let vest = in_play(&mut e, 0, "Kevlar Vest", Some(a));
    let magnum = in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Cold Aura"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    pick_use_of(&mut e, magnum, 0);
    pick_use_of(&mut e, vest, 0);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        !e.state.cards[flame.index()].in_play,
        "burned though prevented"
    );
    // At close range, struck: kept.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let flame = in_play(&mut e, 0, "Improvised Flamethrower", Some(a));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.cards[flame.index()].in_play, "close range");
    assert_eq!(blood(&e, a), 3, "a hand strike only");
}

// Helicopter: equipped by Magic of the Smith, it comes locked (LSJ
// 20100119); taken from another minion, locked too.
// [LSJ 20100119]
#[test]
fn helicopter_comes_locked_however_equipped() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("tha", 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Magic of the Smith"]);
    fill_hand(&mut e, 0);
    // The replacement drawn from either end: a filler there.
    for c in ["Form of Mist", "Helicopter", "Form of Mist"] {
        e.state.seats[0].library.push(find_library(c));
    }
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::FindCard(find_library("Helicopter")));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let heli = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Helicopter"))
        .expect("equipped");
    assert!(e.state.cards[heli].locked, "Magic of the Smith");
    // Taken from another minion of its Methuselah.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let giver = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let heli = in_play(&mut e, 0, "Helicopter", Some(giver));
    pick(&mut e, take_from(a, giver));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[heli.index()].host, Some(a));
    assert!(e.state.cards[heli.index()].locked, "taken: equipped");
}

// Alastor: "put this card and the equipment on the chosen vampire" — put on,
// not equipped: the Helicopter unlocked (LSJ 20090415).
// [LSJ 20090415] [LSJ 20100119]
#[test]
fn alastor_puts_a_helicopter_on_unlocked() {
    use zvtes::crypt::{Sect, Title};
    let mut e = game(2);
    let j = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Camarilla), 0),
        4,
        false,
    );
    e.state.minions[j.index()].granted_title = Some(Title::Justicar("Ventrue"));
    let other = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Camarilla), 1),
        4,
        false,
    );
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Alastor"]);
    fill_hand(&mut e, 0);
    // The replacement drawn from either end: a filler there.
    for c in ["Form of Mist", "Helicopter", "Form of Mist"] {
        e.state.seats[0].library.push(find_library(c));
    }
    pick(&mut e, play(0, 0, j));
    pick_matching(&mut e, |o| *o == Choice::ChooseMinion(other));
    pick_matching(&mut e, |o| {
        matches!(
            o,
            Choice::CastVotes { favor: true, .. } | Choice::CastCardVote { favor: true }
        )
    });
    close_polling(&mut e);
    pick_when(&mut e, Choice::FindCard(find_library("Helicopter")));
    request(&mut e);
    let heli = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library("Helicopter"))
        .expect("put on");
    assert_eq!(e.state.cards[heli].host, Some(other));
    assert!(!e.state.cards[heli].locked, "put on, not equipped");
}

// Storage Annex stolen: the card on it goes with it, the new controller
// exchanging it into their own hand (LSJ 20001031, PIB 20121220); leaving
// play, the card on it burns to the ash heap of the Methuselah who put it
// there (LSJ 20021128) — Angelo burning the location.
// [LSJ 20001031] [PIB 20121220] [LSJ 20021128]
#[test]
fn storage_annex_goes_with_its_card_and_burns_it() {
    let mut e = game(2);
    let angelo = e.add_minion(SeatId(0), find_card("Angelo"), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let annex = in_play(&mut e, 0, "Storage Annex", None);
    e.state.cards[annex.index()].stored = vec![find_library("Dodge")].into();
    e.state.cards[annex.index()].stored_owner = Some(SeatId(0));
    pick(&mut e, own_at(angelo, 0, Target::Card(annex)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[annex.index()].in_play);
    assert!(
        e.state.seats[0].ash.contains(&find_library("Dodge")),
        "burned with it"
    );
    // Stolen: seat 1 exchanges its card for the stored one.
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let annex = in_play(&mut e, 0, "Storage Annex", None);
    e.state.cards[annex.index()].stored = vec![find_library("Dodge")].into();
    e.state.cards[annex.index()].controller = SeatId(1);
    hand(&mut e, 1, &["Leverage"]);
    let req = to_window(&mut e, SeatId(1), WindowLabel::MasterPhase);
    pick(&mut e, uses(&req, annex.0)[0]);
    request(&mut e);
    assert!(
        e.state.seats[1].hand.contains(&find_library("Dodge")),
        "its controller's hand"
    );
    assert_eq!(e.state.cards[annex.index()].stored_owner, Some(SeatId(1)));
}

// [LSJ 20100218] Leverage's unlimited bleed goes with a limited one, before
// or after.
// [LSJ 20100218] [RTR 20180511-2]
#[test]
fn leverage_goes_with_a_limited_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pre", 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Aire of Elation", "Leverage"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    let before = pool(&e)[1];
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(pool(&e)[1] <= before - 3, "1, +1 limited, +1 Leverage");
}

// [LSJ 19970516] A Soul Gem of Etrius on Sacrificial Lamb's target is not
// taken; [ANK 20210124] the target out of torpor by resolution: it fizzles.
// [ANK 20210124] [LSJ 20070411] [LSJ 20090514]
#[test]
fn sacrificial_lamb_leaves_a_soul_gem_and_fizzles_off_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let t = e.add_minion(SeatId(0), of_clan("Brujah", 3..=6), 0, true);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let gem = in_play(&mut e, 0, "Soul Gem of Etrius", Some(t));
    hand(&mut e, 0, &["Sacrificial Lamb"]);
    let lamb = Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(a),
        target: Target::Minion(t),
    };
    pick(&mut e, lamb);
    none_until(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        *o == Choice::ChooseCard(gem.0)
    });
    assert!(!e.state.minions[t.index()].in_play, "burned");
    assert_ne!(e.state.cards[gem.index()].host, Some(a), "not taken");
    // Out of torpor by resolution: no burn.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("obf", 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let t = e.add_minion(SeatId(0), of_clan("Brujah", 3..=6), 1, true);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    hand(&mut e, 0, &["Sacrificial Lamb"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Minion(t),
        },
    );
    request(&mut e);
    e.state.minions[t.index()].in_torpor = false;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].in_play, "fizzled");
}

// Hidden Lurker: by the acting minion's controller only (LSJ 19990425); not
// with a combat already queued (RTR 20020501): a second copy not offered;
// unable to strike, the blocker uses no weapon's maneuver (LSJ 20001126).
// [LSJ 20001126] [LSJ 20021028] [RTR 20020501] [LSJ 19990425]
#[test]
fn hidden_lurker_only_for_the_acting_methuselah_and_one_queued_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let l = e.add_minion(SeatId(0), vampire_with("obf", 0), 3, false);
    let m = e.add_minion(SeatId(0), vampire_with("obf", 1), 3, false);
    let z = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let theirs = e.add_minion(SeatId(1), vampire_with("obf", 2), 4, false);
    let magnum = in_play(&mut e, 1, ".44 Magnum", Some(z));
    hand(&mut e, 0, &["Hidden Lurker", "Hidden Lurker"]);
    hand(&mut e, 1, &["Hidden Lurker"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let req = offering(&mut e, play(0, 0, l));
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { actor: Some(x), .. } if *x == theirs)));
    e.choose(index_of(&req, play(0, 0, l)));
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, m)), "a combat queued");
    // The new combat's one round: neither the gun's strike nor its maneuver
    // for z.
    none_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == magnum),
    );
}

// Hatch the Viper [PRO][SER]: 2 damage at close range each round, through a
// dodge (RTR 19941109); the version as employed (ANK 20180612).
// [RTR 19941109] [TOM 19951216] [PIB 20140324] [ANK 20200422]; [RTR 19941109] [ANK 20180612]
#[test]
fn hatch_the_viper_superior_bites_through_a_dodge() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("pro", 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let z = e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let card = find_library("Hatch the Viper");
    e.state
        .cards
        .push(CardInPlay::new(card, SeatId(1), Some(z), 1));
    let hatch = CardId(e.state.cards.len() as u32 - 1);
    e.state.cards[hatch.index()].counters = 1;
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::RetainerDamageClose(2),
        source: hatch,
    });
    hand(&mut e, 0, &["Form of Mist"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 2, "dodged the strike, not the Viper");
}

// Fergus: the target unlocked by resolution, the action fizzles (ANK
// 20210124): no combat.
// [ANK 20210124] [LSJ 20070411] [LSJ 20090514]
#[test]
fn fergus_fizzles_on_an_unlocked_target() {
    let mut e = game(2);
    let fergus = e.add_minion(SeatId(0), find_card("Fergus Alexander"), 3, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    let s = e.add_minion(
        SeatId(1),
        of_sect(zvtes::crypt::Sect::Sabbat, 4..=6),
        4,
        false,
    );
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    e.state.minions[s.index()].locked = true;
    pick(&mut e, own_at(fergus, 0, Target::Minion(s)));
    // The block window, seat 1's other vampire's: the target unlocks there.
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.state.minions[s.index()].locked = false;
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, s), 4, "no combat");
    assert_eq!(blood(&e, fergus), 3);
}

// Eternal Vigilance keeps the blocking restrictions (ANK 20180623): with
// four Methuselahs, a vampire neither the bleed's target nor its acting
// Methuselah's prey or predator is never offered the unlock.
// [ANK 20180623]
#[test]
fn eternal_vigilance_keeps_the_block_restrictions() {
    use zvtes::crypt::{Sect, Title};
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    for s in [1u8, 3] {
        e.add_minion(SeatId(s), of_clan("Ventrue", 4..=6), 4, false);
    }
    let v = e.add_minion(
        SeatId(2),
        textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity >= 3, 0),
        3,
        false,
    );
    e.state.minions[v.index()].granted_title = Some(Title::Priscus);
    let ev = in_play(&mut e, 2, "Eternal Vigilance", Some(v));
    e.state.minions[v.index()].locked = true;
    pick(&mut e, bleed(a));
    none_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == ev),
    );
}

// The Admonitions: not usable as a card is played (RTR 20040501, The
// Barrens'); its two cards drawn as far as the library holds (LSJ
// 20060620).
// [LSJ 20060620] [RTR 20040501]
#[test]
fn the_admonitions_not_as_played_and_draws_what_there_is() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), textless(|c| c.black_hand, 0), 4, false);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let adm = in_play(&mut e, 0, "Admonitions, The", None);
    fill_hand(&mut e, 0);
    e.state.seats[0].library = vec![find_library("Dodge")].into();
    pick(&mut e, use_on(adm, 1, None, Target::None));
    request(&mut e);
    assert!(e.state.seats[0].library.is_empty(), "one card to draw");
    assert!(e.state.cards[adm.index()].locked);
    let _ = a;
    // Not in the as-played window of another's master card.
    let mut e = game(2);
    e.add_minion(SeatId(0), of_clan("Brujah", 4..=6), 4, false);
    e.add_minion(
        SeatId(1),
        textless(|c| c.sect == Some(zvtes::crypt::Sect::Sabbat), 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), of_clan("Ventrue", 4..=6), 4, false);
    let adm = in_play(&mut e, 1, "Admonitions, The", None);
    e.at_master_phase();
    hand(&mut e, 0, &["Villein"]);
    hand(&mut e, 1, &["Sudden Reversal", "Dodge"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(MinionId(0)),
        },
    );
    pick(&mut e, set_x(2));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert!(uses(&req, adm.0).is_empty());
    assert!(req.options.len() > 1);
}
