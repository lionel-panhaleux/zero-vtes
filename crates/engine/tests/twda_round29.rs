//! Round 29 (`ADDED_BATCHES`): Keepers of Tradition and its 2018 reprint.
//! Windows with a single option are elided, so the scripts only stop where
//! a real decision exists.

use zvtes::cards::{AbilitySource, Stat, Target};
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId, CardInPlay, UncontrolledSlot};

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
#[track_caller]
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

/// The `nth` textless Camarilla vampire of `clan`.
#[track_caller]
fn camarilla(clan: &str, nth: usize) -> u16 {
    textless(|c| c.clan == clan && c.sect == Some(Sect::Camarilla), nth)
}

/// The `nth` textless vampire of capacity `cap` in `sect`.
#[track_caller]
fn of_cap(sect: Sect, cap: u8, nth: usize) -> u16 {
    textless(|c| c.sect == Some(sect) && c.capacity == cap, nth)
}

/// A Camarilla vampire of capacity 8: none is textless any more. Lazarus
/// James, "+1 bleed.", inert where no bleed of his succeeds.
fn elder() -> u16 {
    find_card("Lazarus James")
}

/// The crypt card `name` (not advanced).
fn find_card(name: &str) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && !c.adv)
        .unwrap_or_else(|| panic!("no {name}")) as u16
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

/// The card `name` entered play (itself, not a stand-in) on `host`, out of
/// every ash heap.
fn placed_on(e: &Engine, name: &str, host: Option<MinionId>) -> bool {
    let card = find_library(name);
    e.state
        .cards
        .iter()
        .any(|c| c.card == card && c.in_play && !c.effect_only && c.host == host)
        && e.state.seats.iter().all(|s| !s.ash.contains(&card))
}

/// Seat 0's `a` plays the political action card in hand slot 0, the terms
/// passed by their first option, and wins the referendum by its card vote.
fn pass_referendum(e: &mut Engine, a: MinionId) {
    pick(e, play(0, 0, a));
    pick_when(e, Choice::CastCardVote { favor: true });
    close_polling(e);
}

/// The referendum's polling step opened (the pre-vote stage over).
fn to_polling(e: &mut Engine) {
    to_referendum(e);
    for _ in 0..50 {
        let req = request(e);
        if req.window == WindowLabel::ReferendumPolling
            && req.options.contains(&Choice::CastCardVote { favor: true })
        {
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("no polling");
}

/// A card already in play for its host's owner on `host`: its id.
fn on(e: &mut Engine, name: &str, host: MinionId) -> CardId {
    let owner = e.state.minions[host.index()].owner;
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), owner, Some(host), 0));
    CardId(e.state.cards.len() as u32 - 1)
}

/// Seat 0's A and a second vampire, seat 1's X and a second one: plain
/// textless vampires of capacity 4 to 8, 4 blood each, A with `a_levels`
/// and X with `x_levels` gained.
fn duel(a_levels: &[&'static str], x_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let mid = |c: &zvtes::crypt::CryptCard| (4..=8).contains(&c.capacity);
    let a = e.add_minion(SeatId(0), textless(mid, 0), 4, false);
    e.add_minion(SeatId(0), textless(mid, 1), 4, false);
    let x = e.add_minion(SeatId(1), textless(mid, 2), 4, false);
    e.add_minion(SeatId(1), textless(mid, 3), 4, false);
    e.state.minions[a.index()].gained_levels = a_levels.to_vec();
    e.state.minions[x.index()].gained_levels = x_levels.to_vec();
    (e, a, x)
}

/// A bleeds, X blocks: their combat begins.
fn fight(e: &mut Engine, a: MinionId, x: MinionId) {
    pick(e, bleed(a));
    pick_when(e, block(x));
}

/// The strike of the weapon `w` (its ability `ability`) by its bearer.
fn weapon(w: CardId, ability: u8) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(w),
        ability,
        actor: None,
        target: Target::None,
    }
}

/// The card `card` of the hand played by `actor` in `mode` on `target`.
fn play_on(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

fn ash_has(e: &Engine, seat: usize, name: &str) -> bool {
    e.state.seats[seat].ash.contains(&find_library(name))
}

// ---- Title cards --------------------------------------------------------------

/// Brujah, Tremere and Ventrue Justicar: a ready Camarilla vampire of the
/// clan chosen, the title its card's; each ready vampire of the clan +1
/// vote in it, a torpid one none; with none of the clan in the Camarilla,
/// not played. Backs `[LSJ 20041026]` (group "Justicar title vote without
/// Camarilla") and `[LSJ 20100129] [ANK 20191228]` (group "Political
/// action with illegal terms").
#[test]
fn keepers_justicars_title_a_ready_camarilla_vampire_of_their_clan() {
    for (name, clan) in [
        ("Brujah Justicar", "Brujah"),
        ("Tremere Justicar", "Tremere"),
        ("Ventrue Justicar", "Ventrue"),
    ] {
        let mut e = game(2);
        let caller = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        let n = e.add_minion(SeatId(0), camarilla(clan, 0), 4, false);
        let foe = e.add_minion(SeatId(1), camarilla(clan, 1), 4, false);
        let torpid = e.add_minion(SeatId(1), camarilla(clan, 2), 4, true);
        hand(&mut e, 0, &[name]);
        pick(&mut e, play(0, 0, caller));
        let req = offering(&mut e, Choice::ChooseMinion(n));
        assert!(req.options.contains(&Choice::ChooseMinion(foe)), "{name}");
        assert!(!req.options.contains(&Choice::ChooseMinion(torpid)));
        assert!(!req.options.contains(&Choice::ChooseMinion(caller)));
        e.choose(index_of(&req, Choice::ChooseMinion(n)));
        to_polling(&mut e);
        let r = e.referendum().expect("referendum");
        assert_eq!(r.votes_of(&e.state, foe), 1, "{name}: the grant");
        assert_eq!(r.votes_of(&e.state, torpid), 0, "{name}: torpid");
        assert_eq!(r.votes_of(&e.state, caller), 0, "{name}: another clan");
        pick(&mut e, Choice::CastCardVote { favor: true });
        close_polling(&mut e);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.title_of(n), Title::Justicar(clan), "{name}");
        assert!(placed_on(&e, name, Some(n)), "{name}");
        // Only an antitribu and a non-Camarilla vampire of the clan: not
        // played.
        let mut e = game(2);
        let caller = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        e.add_minion(
            SeatId(0),
            textless(|c| c.clan == format!("{clan} antitribu"), 0),
            4,
            false,
        );
        e.add_minion(
            SeatId(1),
            textless(|c| c.clan == clan && c.sect != Some(Sect::Camarilla), 0),
            4,
            false,
        );
        hand(&mut e, 0, &[name]);
        assert!(
            !request(&mut e).options.contains(&play(0, 0, caller)),
            "{name}"
        );
    }
}

/// The five Praxis Seizures: the card on the Camarilla vampire calling it,
/// Prince of its city; a non-Camarilla vampire is not offered it.
#[test]
fn praxis_seizures_title_the_calling_camarilla_vampire() {
    for (name, city) in [
        ("Praxis Seizure: Atlanta", "Atlanta"),
        ("Praxis Seizure: Chicago", "Chicago"),
        ("Praxis Seizure: Houston", "Houston"),
        ("Praxis Seizure: Miami", "Miami"),
        ("Praxis Seizure: Washington, D.C.", "Washington, D.C."),
    ] {
        let mut e = game(2);
        let caller = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        let sabbat = e.add_minion(SeatId(0), of_cap(Sect::Sabbat, 4, 0), 4, false);
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        hand(&mut e, 0, &[name]);
        let req = request(&mut e);
        assert!(!req.options.contains(&play(0, 0, sabbat)), "{name}");
        pass_referendum(&mut e, caller);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.title_of(caller), Title::Prince(city), "{name}");
        assert!(placed_on(&e, name, Some(caller)), "{name}");
    }
}

/// Praxis Seizure: Atlanta beside another Methuselah's Prince of Atlanta:
/// the two contest the title (rules-qa 3.1).
#[test]
fn praxis_seizure_atlanta_contests_a_prince_of_atlanta_in_play() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let holder = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[holder.index()].granted_title = Some(Title::Prince("Atlanta"));
    hand(&mut e, 0, &["Praxis Seizure: Atlanta"]);
    pass_referendum(&mut e, caller);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    for m in [caller, holder] {
        assert_eq!(e.state.claimed_title(m), Title::Prince("Atlanta"));
        assert!(e.state.minions[m.index()].title_contested, "{m:?}");
    }
}

/// Ventrue Justicar on a printed prince: the card's title replaces the
/// printed one (rules-qa 19.2, the political chunk's J-5): a justicar's 3
/// votes, the prince's city gone.
#[test]
fn a_justicar_title_replaces_a_printed_princes_title() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let prince = CRYPT
        .iter()
        .enumerate()
        .position(|(i, c)| {
            c.clan == "Ventrue"
                && c.sect == Some(Sect::Camarilla)
                && matches!(c.title, Title::Prince(_))
                && !c.adv
                && zvtes::cards::vampire(i as u16).is_none()
        })
        .expect("a textless Ventrue prince") as u16;
    let n = e.add_minion(SeatId(0), prince, 4, false);
    assert!(matches!(e.state.title_of(n), Title::Prince(_)));
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Ventrue Justicar"]);
    pick(&mut e, play(0, 0, caller));
    pick_when(&mut e, Choice::ChooseMinion(n));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Ventrue Justicar", Some(n)));
    assert_eq!(e.state.title_of(n), Title::Justicar("Ventrue"));
}

/// Praxis Seizure: Atlanta, its caller no longer Camarilla before it
/// resolves (a sect change mid-action): the requirement lapsed, the action
/// fizzles (rules-digest D13, the chunk's P-2): no referendum, no title, the
/// card in its ash heap.
#[test]
fn praxis_seizure_fizzles_once_its_caller_leaves_the_camarilla() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Praxis Seizure: Atlanta"]);
    pick(&mut e, play(0, 0, caller));
    request(&mut e);
    e.state.minions[caller.index()].sect_override = Some(Sect::Anarch);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::CastCardVote { .. })
    });
    assert!(e.referendum().is_none());
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(e.state.title_of(caller), Title::None);
    assert!(ash_has(&e, 0, "Praxis Seizure: Atlanta"));
}

// ---- Pool and blood referendums -----------------------------------------------

/// Can't Take it with You: each Methuselah gains 1 pool, then burns 1 per
/// equipment, location and retainer card they control — the equipment and
/// retainers on their minions, a master location, a locquipment once; an
/// ally is none. Backs `[LSJ 20071005-2]`.
#[test]
fn cant_take_it_with_you_burns_a_pool_per_equipment_location_and_retainer() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let b = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let c = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), camarilla("Ventrue", 1), 4, false);
    in_play(&mut e, 0, "Flak Jacket", Some(a));
    in_play(&mut e, 0, "Kevlar Vest", Some(b));
    in_play(&mut e, 0, "Storage Annex", None);
    in_play(&mut e, 0, "Mr. Winthrop", Some(b));
    e.add_ally(SeatId(0), find_library("ECTU Operative"));
    in_play(&mut e, 1, "Catacombs", Some(c));
    hand(&mut e, 0, &["Can't Take it with You"]);
    pass_referendum(&mut e, a);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![7, 10, 11]);
}

/// Honor the Elders: each ready vampire of capacity 8 or more, locked too,
/// gains 1 blood, one in torpor not; each of capacity 8 or more in any
/// uncontrolled region gains 1.
#[test]
fn honor_the_elders_feeds_big_ready_and_uncontrolled_vampires() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let big = e.add_minion(SeatId(1), elder(), 3, false);
    e.state.minions[big.index()].locked = true;
    let small = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 0), 3, false);
    // Valerius (9), in torpor: his title steal is an action.
    let torpid = e.add_minion(SeatId(0), find_card("Valerius"), 3, true);
    // Gracetius (10), uncontrolled: his text reads only in play.
    for card in [find_card("Gracetius"), of_cap(Sect::Camarilla, 7, 1)] {
        e.state.seats[1].uncontrolled.push(UncontrolledSlot {
            card,
            copy: Default::default(),
            counters: 2,
            banished: None,
        });
    }
    hand(&mut e, 0, &["Honor the Elders"]);
    pass_referendum(&mut e, caller);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, big), 4);
    assert_eq!(blood(&e, small), 3);
    assert_eq!(blood(&e, torpid), 3);
    let slots: Vec<u8> = e.state.seats[1]
        .uncontrolled
        .iter()
        .map(|s| s.counters)
        .collect();
    assert_eq!(slots, vec![3, 2]);
}

/// Honor the Elders: a vampire already at its capacity gains nothing, one
/// below it gains 1 (rulebook: no blood beyond capacity; the chunk's H-4).
#[test]
fn honor_the_elders_feeds_no_vampire_beyond_its_capacity() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    // Lazarus James (8) full; Valerius (9) one short, no prince to steal from.
    let full = e.add_minion(SeatId(1), elder(), 8, false);
    let short = e.add_minion(SeatId(1), find_card("Valerius"), 8, false);
    hand(&mut e, 0, &["Honor the Elders"]);
    pass_referendum(&mut e, caller);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, full), 8);
    assert_eq!(blood(&e, short), 9);
}

// ---- National Guard Support ---------------------------------------------------

/// A titled vampire's National Guard Support in play: every minion's
/// directed action -1 stealth (a bleed 0 → -1), an undirected one not (a
/// hunt keeps its 1); a counter at its controller's unlock phase, none at
/// another's.
#[test]
fn national_guard_support_slows_directed_actions_and_counts_turns() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
    let b = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let h = e.add_minion(SeatId(0), camarilla("Toreador", 2), 1, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["National Guard Support"]);
    pass_referendum(&mut e, p);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "National Guard Support", None));
    let ngs = CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.card == find_library("National Guard Support"))
            .unwrap() as u32,
    );
    pick(&mut e, bleed(b));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), -1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, action(h, ActionKind::Hunt));
    request(&mut e);
    assert_eq!(e.action().expect("hunt").stealth(&e.state), 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[ngs.index()].counters, 0, "seat 1's unlock");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[ngs.index()].counters, 1, "its controller's");
}

/// Another Methuselah's National Guard Support in play: not played.
#[test]
fn national_guard_support_is_not_played_while_another_is_in_play() {
    let mut e = game(3);
    let p = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), camarilla("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["National Guard Support"]);
    assert!(request(&mut e).options.contains(&play(0, 0, p)));
    in_play(&mut e, 2, "National Guard Support", None);
    assert!(!request(&mut e).options.contains(&play(0, 0, p)));
}

/// Any vampire's +1 stealth political action to burn it, undirected (no
/// -1): each counter a vote in favor, the card's own. 3 counters beat a
/// prince's 2 votes against: burned to its owner's ash heap; 1 does not.
#[test]
fn national_guard_support_counter_votes_burn_it() {
    for (counters, burned) in [(3, true), (1, false)] {
        let mut e = game(2);
        let u = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        let p = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
        let ngs = in_play(&mut e, 1, "National Guard Support", None);
        e.state.cards[ngs.index()].counters = counters;
        pick(&mut e, granted(ngs.0, u));
        request(&mut e);
        assert_eq!(e.action().expect("political").stealth(&e.state), 1);
        to_referendum(&mut e);
        pick_when(&mut e, votes(p, false));
        close_polling(&mut e);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let card = find_library("National Guard Support");
        assert_eq!(!e.state.cards[ngs.index()].in_play, burned, "{counters}");
        assert_eq!(e.state.seats[1].ash.contains(&card), burned, "{counters}");
    }
}

// ---- Persona Non Grata ----------------------------------------------------------

/// Persona Non Grata on another Methuselah's untitled vampire: the card on
/// it; its bleed -1 stealth; no Praxis Seizure offered to it, its other
/// vampire's offered; a Ventrue Justicar's terms skip it ("prohibited by
/// card text").
#[test]
fn persona_non_grata_slows_its_vampire_and_bars_non_anarch_titles() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), camarilla("Toreador", 2), 4, false);
    let x = e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    let y = e.add_minion(SeatId(0), camarilla("Ventrue", 2), 4, false);
    hand(&mut e, 0, &["Persona Non Grata"]);
    hand(&mut e, 1, &["Praxis Seizure: Atlanta", "Ventrue Justicar"]);
    pick(&mut e, play(0, 0, p));
    pick(&mut e, Choice::ChooseMinion(v));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Persona Non Grata", Some(v)));
    assert!(!req.options.contains(&play(0, 0, v)));
    assert!(req.options.contains(&play(0, 0, w)));
    pick(&mut e, bleed(v));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), -1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, play(1, 0, w));
    let req = offering(&mut e, Choice::ChooseMinion(x));
    assert!(req.options.contains(&Choice::ChooseMinion(y)));
    assert!(!req.options.contains(&Choice::ChooseMinion(v)));
}

/// None to choose — every vampire titled and not Anarch, or bearing one
/// already — not played; an Anarch baron choosable. Backs `[LSJ
/// 20100129] [ANK 20191228]` (group "Political action with illegal
/// terms").
#[test]
fn persona_non_grata_needs_a_choosable_vampire() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
    let q = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[q.index()].granted_title = Some(Title::Prince("Chicago"));
    let marked = e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    in_play(&mut e, 0, "Persona Non Grata", Some(marked));
    hand(&mut e, 0, &["Persona Non Grata"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, p)));
    let baron = e.add_minion(SeatId(1), of_cap(Sect::Anarch, 5, 0), 4, false);
    e.state.minions[baron.index()].granted_title = Some(Title::Baron("Seattle"));
    assert!(request(&mut e).options.contains(&play(0, 0, p)));
    pick(&mut e, play(0, 0, p));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Persona Non Grata", Some(baron)));
}

/// Persona Non Grata on an untitled Anarch: a baron's is no non-Anarch
/// title (rules-qa 32.9, the chunk's G-4), so Fee Stake: Seattle titles
/// it; once titled, the card stays on it (D13: a card in play stays when
/// its requirement lapses; G-7).
#[test]
fn persona_non_grata_lets_its_anarch_become_a_baron_and_stays() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_cap(Sect::Anarch, 5, 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let png = in_play(&mut e, 1, "Persona Non Grata", Some(v));
    hand(&mut e, 0, &["Fee Stake: Seattle"]);
    pick(&mut e, play(0, 0, v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Fee Stake: Seattle", Some(v)));
    assert_eq!(e.state.title_of(v), Title::Baron("Seattle"));
    let card = &e.state.cards[png.index()];
    assert!(card.present() && card.host == Some(v), "stays on its baron");
}

/// Persona Non Grata on Horatio Ballard: his own referendum to become
/// Prince of Chicago passes, a legal no-op (the chunk's G-5: a title gained
/// without a card); without the card, he is the prince.
#[test]
fn persona_non_grata_makes_horatio_ballards_title_a_no_op() {
    for marked in [false, true] {
        let mut e = game(2);
        let p = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
        let h = e.add_minion(SeatId(0), find_card("Horatio Ballard"), 4, false);
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        if marked {
            in_play(&mut e, 1, "Persona Non Grata", Some(h));
        }
        let call = ability_of(&mut e, AbilitySource::Vampire(h), 0);
        pick(&mut e, call);
        to_referendum(&mut e);
        pick_when(&mut e, votes(p, true));
        close_polling(&mut e);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let want = if marked {
            Title::None
        } else {
            Title::Prince("Chicago")
        };
        assert_eq!(e.state.title_of(h), want, "marked {marked}");
    }
}

/// Persona Non Grata on Valerius: his steal of a prince's title gains him
/// none, and the prince keeps it (the chunk's G-5: the gain a no-op; the
/// steal is that gain); without the card, the title his.
#[test]
fn persona_non_grata_makes_valerius_steal_a_no_op() {
    for marked in [false, true] {
        let mut e = game(2);
        let val = e.add_minion(SeatId(0), find_card("Valerius"), 4, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        let p = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
        if marked {
            in_play(&mut e, 1, "Persona Non Grata", Some(val));
        }
        let steal = vampire_use_on(&mut e, val, 0, Target::Minion(p));
        pick(&mut e, steal);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let (thief, held) = if marked {
            (Title::None, Title::Prince("Atlanta"))
        } else {
            (Title::Prince("Atlanta"), Title::None)
        };
        assert_eq!(e.state.title_of(val), thief, "marked {marked}");
        assert_eq!(e.state.title_of(p), held, "marked {marked}");
    }
}

// ---- Scourge --------------------------------------------------------------------

/// Scourge: any vampire of the caller's sect, the caller and another
/// Methuselah's too, a Sabbat one not; the card on the chosen one, the
/// caller unlocked as it is.
#[test]
fn scourge_goes_on_a_same_sect_vampire_and_unlocks_its_caller() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
    let q = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let r = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let s = e.add_minion(SeatId(1), of_cap(Sect::Sabbat, 4, 0), 4, false);
    hand(&mut e, 0, &["Scourge"]);
    pick(&mut e, play(0, 0, p));
    let req = offering(&mut e, Choice::ChooseMinion(q));
    for m in [p, r] {
        assert!(req.options.contains(&Choice::ChooseMinion(m)), "{m:?}");
    }
    assert!(!req.options.contains(&Choice::ChooseMinion(s)));
    e.choose(index_of(&req, Choice::ChooseMinion(q)));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Scourge", Some(q)));
    assert!(!e.state.minions[p.index()].locked, "the caller unlocked");
}

/// Its vampire's +1 stealth action: combat with another Methuselah's ally
/// or vampire of capacity 4 or less, one of capacity 5 or an own minion
/// not.
#[test]
fn scourge_rushes_a_small_vampire_or_an_ally() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let own = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 3, 0), 3, false);
    let small = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 4, false);
    let big = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 5, 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("ECTU Operative"));
    let id = in_play(&mut e, 0, "Scourge", Some(b));
    let rush = |t: MinionId| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(id),
        ability: 0,
        actor: Some(b),
        target: Target::Minion(t),
    };
    let req = request(&mut e);
    assert!(req.options.contains(&rush(small)));
    assert!(req.options.contains(&rush(ally)));
    assert!(!req.options.contains(&rush(big)));
    assert!(!req.options.contains(&rush(own)));
    e.notes = Some(Vec::new());
    e.choose(index_of(&req, rush(small)));
    request(&mut e);
    assert_eq!(e.action().expect("rush").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!noted_combats(&mut e).is_empty(), "the rush's combat");
    // Hand strikes exchanged with the target.
    assert_eq!((blood(&e, b), blood(&e, small), blood(&e, big)), (3, 3, 4));
}

/// Scourge is no title (no "Title." printed; the chunk's SC-5): its
/// vampire stays untitled and cannot call Persona Non Grata ("Requires a
/// titled vampire"), a prince can.
#[test]
fn scourge_is_no_title() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let p = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    in_play(&mut e, 0, "Scourge", Some(s));
    hand(&mut e, 0, &["Persona Non Grata"]);
    assert_eq!(e.state.title_of(s), Title::None);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, s)));
    assert!(req.options.contains(&play(0, 0, p)));
}

// ---- Veles' Hunt --------------------------------------------------------------

/// A textless Gangrel of capacity `cap`.
#[track_caller]
fn gangrel(cap: u8, nth: usize) -> u16 {
    textless(|c| c.clan == "Gangrel" && c.capacity == cap, nth)
}

/// Veles' Hunt (3 pool, paid as it resolves): the prey's ally or younger
/// untitled vampire chosen — not one as old, older or titled, nor the
/// predator's; passed, burned to its owner's ash heap; Voter Captivation after
/// it (a political action like any other). None to choose: not played. Backs
/// `[LSJ 20081202-2]` (Voter Captivation) and `[LSJ 20100129] [ANK 20191228]`.
#[test]
fn veles_hunt_burns_an_ally_or_younger_untitled_vampire_of_the_prey() {
    let mut e = game(3);
    let g = e.add_minion(SeatId(0), gangrel(6, 0), 4, false);
    e.state.minions[g.index()].gained_levels = vec!["pre"];
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("ECTU Operative"));
    let y_card = of_cap(Sect::Camarilla, 4, 0);
    let y = e.add_minion(SeatId(1), y_card, 4, false);
    let same = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 6, 0), 4, false);
    let old = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 0), 4, false);
    let titled = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 1), 4, false);
    e.state.minions[titled.index()].granted_title = Some(Title::Prince("Atlanta"));
    let pred = e.add_minion(SeatId(2), of_cap(Sect::Camarilla, 3, 0), 3, false);
    hand(&mut e, 0, &["Veles' Hunt", "Voter Captivation"]);
    pick(&mut e, play(0, 0, g));
    let req = offering(&mut e, Choice::ChooseMinion(y));
    assert!(req.options.contains(&Choice::ChooseMinion(ally)));
    for m in [same, old, titled, pred] {
        assert!(!req.options.contains(&Choice::ChooseMinion(m)), "{m:?}");
    }
    e.choose(index_of(&req, Choice::ChooseMinion(y)));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    assert_eq!(e.state.seats[0].pool, 10 - 3, "its 3 pool paid");
    assert!(offered_before(
        &mut e,
        play(0, 0, g),
        SeatId(0),
        WindowLabel::MinionPhase
    ));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[y.index()].in_play);
    assert!(e.state.seats[1].ash_crypt.contains(&y_card));
    // Only an older vampire of the prey's: not played.
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), gangrel(6, 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 0), 4, false);
    hand(&mut e, 0, &["Veles' Hunt"]);
    assert!(!request(&mut e).options.contains(&play(0, 0, g)));
}

/// Canceled by Delaying Tactics in its polling: back in hand, no Veles'
/// Hunt is played again, the next turn neither, nor another Methuselah's;
/// failed, the same; blocked (no referendum), the next copy is played.
/// Backs `[LSJ 20081202-2]` (Delaying Tactics during the referendum).
#[test]
fn veles_hunt_canceled_or_failed_bars_every_copy_for_the_game() {
    let mut e = game(3);
    let g = e.add_minion(SeatId(0), gangrel(6, 0), 4, false);
    let g2 = e.add_minion(SeatId(0), gangrel(6, 1), 4, false);
    e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 4, false);
    let r = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 0), 4, false);
    let z = e.add_minion(SeatId(2), gangrel(6, 2), 4, false);
    e.add_minion(SeatId(2), of_cap(Sect::Camarilla, 3, 0), 3, false);
    hand(&mut e, 0, &["Veles' Hunt", "Veles' Hunt"]);
    hand(&mut e, 1, &["Delaying Tactics"]);
    hand(&mut e, 2, &["Veles' Hunt"]);
    pick(&mut e, play(0, 0, g));
    pick_when(&mut e, play(0, 0, r));
    let veles = find_library("Veles' Hunt");
    let gangrels = [g, g2, z];
    let bad = |o: &Choice| matches!(o, Choice::Play { actor: Some(m), .. } if gangrels.contains(m));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, bad);
    assert_eq!(
        e.state.seats[0]
            .hand
            .iter()
            .filter(|&&c| c == veles)
            .count(),
        2,
        "back in hand"
    );
    pick(&mut e, Choice::Pass);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, bad);
    // Failed: a prince's 2 votes against its card's 1.
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), gangrel(6, 0), 4, false);
    let g2 = e.add_minion(SeatId(0), gangrel(6, 1), 4, false);
    e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 4, false);
    let p = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
    hand(&mut e, 0, &["Veles' Hunt", "Veles' Hunt"]);
    pick(&mut e, play(0, 0, g));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    pick_when(&mut e, votes(p, false));
    close_polling(&mut e);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, g2)), "failed");
    // Blocked: no referendum, nothing barred.
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), gangrel(6, 0), 4, false);
    let g2 = e.add_minion(SeatId(0), gangrel(6, 1), 4, false);
    e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 4, false);
    // Kite: "+1 intercept", the card's +1 stealth met.
    let b = e.add_minion(SeatId(1), find_card("Kite"), 4, false);
    hand(&mut e, 0, &["Veles' Hunt", "Veles' Hunt"]);
    pick(&mut e, play(0, 0, g));
    pick(&mut e, block(b));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&play(0, 0, g2)), "blocked");
}

/// Rules-qa red (round 29): "Blood hunt referendum" — a blood hunt for the
/// blood-hunt readers: Karsh's +3 votes in it, Archon's host not
/// choosable, Genina to torpor instead of burned.
#[test]
fn veles_hunt_is_a_blood_hunt_referendum() {
    let mut e = game(2);
    let karsh = CRYPT
        .iter()
        .position(|c| c.name == "Karsh" && c.adv)
        .expect("Karsh (advanced)") as u16;
    let k = e.add_minion(SeatId(0), karsh, 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let genina = e.add_minion(SeatId(1), find_card("Genina, The Red Poet"), 4, false);
    let archon = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 4, false);
    in_play(&mut e, 1, "Archon", Some(archon));
    hand(&mut e, 0, &["Veles' Hunt"]);
    pick(&mut e, play(0, 0, k));
    to_polling(&mut e);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.terms.minions, vec![genina], "Archon's host not chosen");
    assert!(r.is_blood_hunt());
    assert_eq!(r.votes_cast(&e.state, k, true), 3);
    pick(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[genina.index()];
    assert!(v.in_play && v.in_torpor, "to torpor, not burned");
}

/// Veles' Hunt with Redline (V-5: "Blood hunt referendum" read as a blood
/// hunt by every reader): passed on seat 1's Y, seat 2's Redline ("Only
/// usable when a blood hunt referendum passes and would burn a vampire
/// controlled by another Methuselah. That vampire is not burned") saves Y
/// as the burn waits (`defer_hunt_burn`): Y stays in play, seat 2's,
/// Redline on it.
#[test]
fn redline_saves_a_veles_hunt_victim() {
    let mut e = game(3);
    let g = e.add_minion(SeatId(0), gangrel(6, 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let y = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 4, false);
    e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 1), 4, false);
    e.add_minion(SeatId(2), of_cap(Sect::Camarilla, 3, 0), 3, false);
    hand(&mut e, 0, &["Veles' Hunt"]);
    hand(&mut e, 2, &["Redline"]);
    pick(&mut e, play(0, 0, g));
    pick_when(&mut e, Choice::ChooseMinion(y));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    let redline = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(y),
    };
    pick_when(&mut e, redline);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[y.index()];
    assert!(v.in_play && !v.in_torpor, "not burned");
    assert_eq!(v.owner, SeatId(2));
    let card = find_library("Redline");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == card && c.present() && c.host == Some(y)));
}

/// Rules-qa red (round 29, V-5): Veles' Hunt read as a blood hunt by the
/// other blood-hunt cards. Called by Gwyedd (10): Ariadne ("Blood hunts
/// cannot be called on Ariadne") and Theo Bell G2 beside his Methuselah's
/// ready prince are not choosable; on Styles Margs, his +2 votes in favor
/// stand at once; Troius casts none against, his 2 in favor; Jason "Son"
/// Newberry 2 more against; on Ilyana Ravidovich, her 2 additional votes.
#[test]
fn veles_hunt_reads_every_blood_hunt_card() {
    let theo = CRYPT
        .iter()
        .position(|c| c.name == "Theo Bell" && c.group == 2 && !c.adv)
        .expect("Theo Bell G2") as u16;
    for on_styles in [true, false] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), find_card("Gwyedd"), 4, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        let styles = e.add_minion(SeatId(1), find_card("Styles Margs"), 4, false);
        let ilyana = e.add_minion(SeatId(1), find_card("Ilyana Ravidovich"), 4, false);
        let ariadne = e.add_minion(SeatId(1), find_card("Ariadne"), 4, false);
        let theo = e.add_minion(SeatId(1), theo, 4, false);
        let troius = e.add_minion(SeatId(1), find_card("Troius"), 4, false);
        let jason = e.add_minion(SeatId(1), find_card("Jason \"Son\" Newberry"), 4, false);
        hand(&mut e, 0, &["Veles' Hunt"]);
        pick(&mut e, play(0, 0, g));
        let req = offering(&mut e, Choice::ChooseMinion(styles));
        assert!(req.options.contains(&Choice::ChooseMinion(ilyana)));
        for m in [ariadne, theo, troius, jason] {
            assert!(!req.options.contains(&Choice::ChooseMinion(m)), "{m:?}");
        }
        let chosen = if on_styles { styles } else { ilyana };
        e.choose(index_of(&req, Choice::ChooseMinion(chosen)));
        to_polling(&mut e);
        let r = e.referendum().expect("referendum");
        assert!(r.is_blood_hunt());
        if on_styles {
            assert_eq!(r.tally(&e.state), (2, 0), "Styles' votes");
            assert_eq!(r.votes_of(&e.state, styles), 0);
        } else {
            assert_eq!(r.tally(&e.state), (0, 0));
            assert_eq!(r.votes_of(&e.state, ilyana), 2, "Ilyana's");
        }
        assert_eq!(r.votes_cast(&e.state, troius, false), 0, "Troius");
        assert_eq!(r.votes_cast(&e.state, troius, true), 2);
        assert_eq!(r.votes_cast(&e.state, jason, false), 1 + 2, "Jason");
        assert_eq!(r.votes_cast(&e.state, jason, true), 1);
    }
}

/// Veles' Hunt canceled as it is played (Direct Intervention) is no
/// action and holds no referendum; fizzled (its one choosable vampire
/// titled before it resolves: no legal terms, rules-qa 117.6, nothing
/// paid), no referendum either: neither "fails or is canceled", the next
/// copy is played (the chunk's V-6b, V-6c; rules-digest 3.2).
#[test]
fn veles_hunt_canceled_as_played_or_fizzled_bars_nothing() {
    let setup = || {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), gangrel(6, 0), 4, false);
        let g2 = e.add_minion(SeatId(0), gangrel(6, 1), 4, false);
        let y = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 4, false);
        e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 0), 4, false);
        hand(&mut e, 0, &["Veles' Hunt", "Veles' Hunt"]);
        (e, g, g2, y)
    };
    let (mut e, g, g2, _) = setup();
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, play(0, 0, g));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, master_play(0)));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Direct Intervention"));
    assert!(req.options.contains(&play(0, 0, g2)), "canceled as played");
    // Fizzled.
    let (mut e, g, g2, y) = setup();
    pick(&mut e, play(0, 0, g));
    request(&mut e);
    e.state.minions[y.index()].granted_title = Some(Title::Prince("Atlanta"));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::CastCardVote { .. })
    });
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(e.state.seats[0].pool, 10, "its 3 pool not paid");
    e.state.minions[y.index()].granted_title = None;
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, g2)), "fizzled");
}

/// Brinksmanship (1 pool), passed: as Veles' Hunt's 3, a political action
/// card's printed pool paid as the action resolves; the card itself in play.
#[test]
fn brinksmanship_pays_its_pool_and_enters_play() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Brinksmanship"]);
    pass_referendum(&mut e, a);
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 10 - 1, "its 1 pool paid");
    assert!(placed_on(&e, "Brinksmanship", None));
}

// ---- First Tradition: The Masquerade ----------------------------------------

/// Two minions each for `n` seats, First Tradition in play under seat 0.
fn tradition(n: usize) -> (Engine, CardId) {
    let mut e = game(n);
    for s in 0..n as u8 {
        e.add_minion(SeatId(s), camarilla("Toreador", 2 * s as usize), 4, false);
        e.add_minion(
            SeatId(s),
            camarilla("Toreador", 2 * s as usize + 1),
            4,
            false,
        );
    }
    let ft = in_play(&mut e, 0, "First Tradition: The Masquerade", None);
    (e, ft)
}

/// Every Methuselah pays 2 pool each unlock phase, its controller too; the
/// controller's sixth pool paid burns it, nothing paid after. Backs `[LSJ
/// 20031011]` (the pool condition met in full).
#[test]
fn first_tradition_burns_two_pool_each_unlock_and_itself_after_six() {
    let (mut e, ft) = tradition(2);
    for (round, left) in [(1, 8), (2, 6), (3, 4)] {
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, left, "seat 1, round {round}");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[0].pool, left, "seat 0, round {round}");
    }
    assert!(!e.state.cards[ft.index()].in_play, "6 pool paid");
    let card = find_library("First Tradition: The Masquerade");
    assert!(e.state.seats[0].ash.contains(&card));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::SkipTurn,
    );
    assert_eq!(e.state.seats[1].pool, 4);
}

/// The skip, asked before the turn: nothing of it happens (its locked
/// vampire stays locked, no pool paid, it never becomes the active seat,
/// the turn count does not advance: owner, 2026-11-12), the prey's turn
/// begins; two skips
/// and two paid turns leave it, the third skip burns it. Backs `[LSJ
/// 20031011]`.
#[test]
fn first_tradition_skipped_turns_burn_it_after_three() {
    let (mut e, ft) = tradition(2);
    e.state.seats[1].pool = 30;
    let a = MinionId(0);
    for (turn, skip) in [(1, true), (2, false), (3, true), (4, false)] {
        e.state.minions[a.index()].locked = true;
        let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
        assert_eq!(req.options, vec![Choice::Pass, Choice::SkipTurn], "{turn}");
        let pool = e.state.seats[0].pool;
        let (number, active) = (e.state.turn, e.state.active);
        assert_eq!(active, SeatId(1), "{turn}: asked before the turn begins");
        if skip {
            e.choose(1);
            let req = request(&mut e);
            assert_eq!(
                (req.seat, req.window),
                (SeatId(1), WindowLabel::ResolutionChoice)
            );
            assert!(e.state.minions[a.index()].locked, "{turn}: not unlocked");
            assert_eq!(e.state.seats[0].pool, pool, "{turn}: nothing paid");
            assert_eq!(
                (e.state.turn, e.state.active),
                (number, active),
                "{turn}: no turn begun, none counted"
            );
        } else {
            e.choose(0);
        }
        assert!(e.state.cards[ft.index()].in_play, "{turn}");
    }
    assert_eq!(e.state.cards[ft.index()].counters, 2);
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, Choice::SkipTurn));
    assert!(!e.state.cards[ft.index()].in_play, "3 skips");
}

/// Not skipped: the 2 pool is an unlock-phase due its payer orders, Vessel's
/// blood to pool first (2 + 1 - 2: not ousted). The skip is asked before
/// anything unlocks. Backs `[ANK 20210110]` and Vessel's mention of First
/// Tradition.
#[test]
fn first_tradition_skip_comes_before_the_unlock_and_vessel_orders_first() {
    let (mut e, ft) = tradition(2);
    let v = MinionId(2);
    let vessel = in_play(&mut e, 1, "Vessel", Some(v));
    e.state.minions[v.index()].locked = true;
    e.state.seats[1].pool = 2;
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    assert!(req.options.contains(&Choice::SkipTurn));
    assert!(e.state.minions[v.index()].locked, "asked before the unlock");
    e.choose(index_of(&req, Choice::Pass));
    let req = expect(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&Choice::ResolveRider { card: ft.0 }));
    let shuttle = *req
        .options
        .iter()
        .find(|o| {
            matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), ability: 0, .. } if *c == vessel)
        })
        .expect("Vessel's blood to pool");
    e.choose(index_of(&req, shuttle));
    pick(&mut e, Choice::ResolveRider { card: ft.0 });
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 1);
    assert!(!e.state.seats[1].ousted);
}

/// Two copies: 4 pool a turn; one skip of their controller counts on both.
/// Backs `[RTR 19941222]`.
#[test]
fn two_first_traditions_burn_four_and_a_skip_counts_on_both() {
    let (mut e, ft) = tradition(2);
    let ft2 = in_play(&mut e, 0, "First Tradition: The Masquerade", None);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 6);
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, Choice::SkipTurn));
    for c in [ft, ft2] {
        assert_eq!(e.state.cards[c.index()].counters, 1);
    }
}

/// Rules-qa red (round 29, FT-6): a skipped turn is none of its phases:
/// the Edge's holder gains nothing, an "until your next unlock phase"
/// effect (Juliet Parr's +2 hand size) keeps waiting and ends at the next
/// unlock phase its Methuselah plays.
#[test]
fn first_tradition_skipped_turn_ends_no_unlock_effect() {
    use zvtes::state::{Expiry, TimedStat};
    let (mut e, _) = tradition(2);
    e.state.seats[0].has_edge = true;
    e.state.seats[0]
        .timed
        .add(TimedStat::HandSize, Expiry::UnlockPhase, 2);
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    let pool = e.state.seats[0].pool;
    e.choose(index_of(&req, Choice::SkipTurn));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ResolutionChoice)
    );
    let s = &e.state.seats[0];
    assert_eq!(s.pool, pool, "no Edge pool");
    assert!(s.has_edge);
    assert_eq!(s.timed.of(TimedStat::HandSize, Expiry::UnlockPhase), 2);
    let req = until(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let s = &e.state.seats[0];
    assert_eq!(s.timed.of(TimedStat::HandSize, Expiry::UnlockPhase), 0);
}

/// A Methuselah short of the 2 pool, not skipping, burns what it has and is
/// ousted (an effect, not a cost: Anarch Revolt's precedent; the chunk's
/// FT-8); its predator takes the victory point, the next turn follows.
#[test]
fn first_tradition_ousts_a_methuselah_short_of_two_pool() {
    let (mut e, _) = tradition(3);
    e.state.seats[1].pool = 1;
    let req = until(&mut e, SeatId(1), WindowLabel::ResolutionChoice);
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert_eq!(e.state.seats[1].pool, 0);
    assert_eq!(e.state.seats[0].half_vps, 2, "its predator's point");
}

// ---- Mouthpiece -----------------------------------------------------------------

/// `m` gets `disc` at superior level (the fixture's Great Beast slot).
fn superior(e: &mut Engine, m: MinionId, disc: &'static str) {
    let slots = &mut e.state.minions[m.index()].beast_disciplines;
    let free = slots.iter().position(Option::is_none).expect("a free slot");
    slots[free] = Some(disc);
}

/// Mouthpiece [dom]: +1 bleed, limited.
#[test]
fn mouthpiece_inferior_adds_a_limited_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dom"];
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Mouthpiece"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").bleed_amount(&e.state), 2);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8);
}

/// Mouthpiece [DOM] by another, locked, vampire of the acting Methuselah:
/// the acting ally, then an acting vampire without Dominate, plays a basic
/// [dom] card (Bonding) as a vampire, never its superior, until the action
/// ends; the vampire does not have Dominate. Backs `[LSJ 19980210]`, `[ANK
/// 20180111]`, `[LSJ 19980303]` and the group on allies' sect.
#[test]
fn mouthpiece_superior_lets_the_acting_minion_play_basic_dominate() {
    let bonding = find_library("Bonding");
    for vampire in [false, true] {
        let mut e = game(2);
        let d = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        superior(&mut e, d, "dom");
        e.state.minions[d.index()].locked = true;
        let actor = if vampire {
            e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false)
        } else {
            e.add_ally(SeatId(0), find_library("ECTU Operative"))
        };
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Mouthpiece", "Bonding"]);
        pick(&mut e, bleed(actor));
        let req = offering(&mut e, play(0, 1, d));
        assert!(
            !req.options.contains(&play(1, 0, actor)),
            "{vampire}: before"
        );
        e.choose(index_of(&req, play(0, 1, d)));
        let req = offering(&mut e, play(0, 0, actor));
        assert!(
            !req.options.contains(&play(0, 1, actor)),
            "{vampire}: superior"
        );
        assert_eq!(e.state.discipline_level(actor, "dom"), 0);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(
            !zvtes::cards::has_disciplines(&e.state, actor, bonding, 0),
            "{vampire}: the action over"
        );
    }
}

/// Only the acting Methuselah's vampires play [DOM]; two of them each a
/// copy on one action. Backs `[LSJ 19990425]` and `[ANK 20200515]`.
#[test]
fn mouthpiece_superior_is_only_for_the_acting_methuselah_and_stacks_by_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let d1 = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let d2 = e.add_minion(SeatId(0), camarilla("Toreador", 2), 4, false);
    let foe = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    for m in [d1, d2, foe] {
        superior(&mut e, m, "dom");
    }
    hand(&mut e, 0, &["Mouthpiece", "Mouthpiece"]);
    hand(&mut e, 1, &["Mouthpiece"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, d1));
    let req = offering(&mut e, play(0, 1, d2));
    assert!(!req.options.contains(&play(0, 1, foe)));
    e.choose(index_of(&req, play(0, 1, d2)));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, foe));
}

/// A grant meets one missing requirement of a combo card (Obedient Flesh
/// [dom][pro]): with Protean, playable; with neither, two copies do not
/// meet both. Backs `[LSJ 20011217] [LSJ 20090508]` and `[LSJ
/// 20020218]`.
#[test]
fn mouthpiece_meets_one_requirement_of_a_multi_discipline_card() {
    let flesh = find_library("Obedient Flesh");
    for (pro, met) in [(true, true), (false, false)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        if pro {
            e.state.minions[a.index()].gained_levels = vec!["pro"];
        }
        let d1 = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        let d2 = e.add_minion(SeatId(0), camarilla("Toreador", 2), 4, false);
        superior(&mut e, d1, "dom");
        superior(&mut e, d2, "dom");
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Mouthpiece", "Mouthpiece"]);
        pick(&mut e, bleed(a));
        assert!(!zvtes::cards::has_disciplines(&e.state, a, flesh, 0));
        pick_when(&mut e, play(0, 1, d1));
        pick_when(&mut e, play(0, 1, d2));
        request(&mut e);
        assert_eq!(
            zvtes::cards::has_disciplines(&e.state, a, flesh, 0),
            met,
            "Protean: {pro}"
        );
    }
}

// ---- Portia -----------------------------------------------------------------------

/// Portia hunts at +1 stealth; blocked, 1 environmental damage on her in
/// the first round besides the blocker's hand strike (4 - 1 - 1).
#[test]
fn portia_hunts_with_stealth_and_takes_damage_when_blocked() {
    let mut e = game(2);
    let portia = e.add_minion(SeatId(0), find_card("Portia"), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let b = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    pick(&mut e, action(portia, ActionKind::Hunt));
    request(&mut e);
    assert_eq!(e.action().expect("hunt").stealth(&e.state), 2);
    // Sunset Strip's kind of hunt stealth, lowered for the block.
    e.state.seats[1].timed.add(
        zvtes::state::TimedStat::HuntStealth,
        zvtes::state::Expiry::EndOfTurn,
        -2,
    );
    pick(&mut e, block(b));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, portia), 2, "the hand strike and the environment");
    assert_eq!(blood(&e, b), 3);
}

/// Outside the Hourglass [TEM] by her blocker in the same before-range
/// step: her 1 environmental damage staged as the step opens, mended with
/// the Hourglass' 2 as it closes, then the strikes (4 - 1 - 2 - 1). Backs
/// `[ANK 20170930]` (group "Damage before range is determined").
#[test]
fn portias_damage_is_mended_with_other_pre_range_damage() {
    let mut e = game(2);
    let portia = e.add_minion(SeatId(0), find_card("Portia"), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let b = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    superior(&mut e, b, "tem");
    hand(&mut e, 1, &["Outside the Hourglass"]);
    pick(&mut e, action(portia, ActionKind::Hunt));
    request(&mut e);
    e.state.seats[1].timed.add(
        zvtes::state::TimedStat::HuntStealth,
        zvtes::state::Expiry::EndOfTurn,
        -2,
    );
    pick(&mut e, block(b));
    let req = offering(&mut e, play(0, 3, b));
    let c = e.combat().expect("combat");
    let i = (c.sides[1].minion == portia) as usize;
    assert_eq!(c.sides[i].pre_range_environmental, 1, "staged");
    assert_eq!(blood(&e, portia), 4, "not yet mended");
    e.choose(index_of(&req, play(0, 3, b)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, portia), 0);
}

/// An ally's own grant beside Mouthpiece's: never both for one combo card
/// — Nocturn [obt] and Shroud of Absence [dom][obt], Herald of Topheth
/// [pre] and Bliss [dom][pre] — each alone meeting its own. Backs `[LSJ
/// 20020218]` for Nocturn and Herald of Topheth.
#[test]
fn mouthpiece_and_an_allys_own_grant_never_meet_a_combo_card_together() {
    let bonding = find_library("Bonding");
    for (ally, combo) in [
        ("Nocturn", "Shroud of Absence"),
        ("Herald of Topheth", "Bliss"),
    ] {
        let mut e = game(2);
        let d = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        superior(&mut e, d, "dom");
        let a = e.add_ally(SeatId(0), find_library(ally));
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Mouthpiece"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, play(0, 1, d));
        request(&mut e);
        assert!(
            zvtes::cards::has_disciplines(&e.state, a, bonding, 0),
            "{ally}"
        );
        assert!(
            !zvtes::cards::has_disciplines(&e.state, a, find_library(combo), 0),
            "{ally}: {combo}"
        );
    }
}

/// The chosen vampire titled since the terms (a prince now): illegal as
/// the referendum passes, the card is not put on it (rules-digest D1:
/// re-checked, no effect) and goes to the ash heap.
#[test]
fn persona_non_grata_has_no_effect_on_a_vampire_titled_since_its_terms() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Persona Non Grata"]);
    pick(&mut e, play(0, 0, p));
    pick_when(&mut e, Choice::ChooseMinion(v));
    to_polling(&mut e);
    assert_eq!(e.referendum().expect("referendum").terms.minions, vec![v]);
    e.state.minions[v.index()].granted_title = Some(Title::Prince("Chicago"));
    pick(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let png = find_library("Persona Non Grata");
    assert!(!e.state.cards.iter().any(|c| c.card == png && c.in_play));
    assert!(e.state.seats[0].ash.contains(&png));
}

// ---- Action cards ---------------------------------------------------------------

/// Dark Mirror of the Mind: +2 pool for a capacity-8 vampire, not offered
/// to a capacity-7 one.
#[test]
fn dark_mirror_of_the_mind_gains_two_pool_for_an_elder() {
    let mut e = game(2);
    let old = e.add_minion(SeatId(0), elder(), 4, false);
    let young = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Dark Mirror of the Mind"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, young)));
    e.choose(index_of(&req, play(0, 0, old)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 12);
}

/// Forgery: a capacity-5 vampire's bleed of 1, successful: the prey burns
/// 1, the vampire gains 1 blood; a capacity-4 vampire is not offered it.
#[test]
fn forgery_bleeds_and_feeds_its_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 5, 0), 2, false);
    let small = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 4, 0), 2, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Forgery"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, small)));
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9);
    assert_eq!(blood(&e, a), 3);
}

/// Sixth Tradition: Destruction by a prince: a vampire who committed
/// diablerie since the end of the player's last turn is burned, one who
/// did before it is no target; an untitled vampire is not offered the
/// card. Backs `[LSJ 20020429]` (group "Since your last turn").
#[test]
fn sixth_tradition_destruction_burns_a_recent_diablerist() {
    let mut e = game(2);
    e.state.seats[0].previous_turn = 2;
    let p = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
    let plain_v = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let recent = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let old = e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    e.state.minions[recent.index()].diablerie_turn = Some(3);
    e.state.minions[old.index()].diablerie_turn = Some(2);
    hand(&mut e, 0, &["Sixth Tradition: Destruction"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play_on(0, 0, p, Target::Minion(old))));
    assert!(!plays(&req)
        .iter()
        .any(|c| matches!(c, Choice::Play { actor: Some(a), .. } if *a == plain_v)));
    e.choose(index_of(&req, play_on(0, 0, p, Target::Minion(recent))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[recent.index()].in_play);
    let card = e.state.minions[recent.index()]
        .crypt_card()
        .expect("vampire");
    assert!(e.state.seats[1].ash_crypt.contains(&card));
}

/// The Haunting [DEM] on the prey's vampire: the card itself on it; the
/// prey's unlock phase burns 1 of its blood; a second copy is no target at
/// it; the prey's minion burns it by an action.
#[test]
fn the_haunting_bleeds_its_minion_each_unlock_phase_and_can_be_burned() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    superior(&mut e, a, "dem");
    let b = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.state.minions[b.index()].gained_levels = vec!["dem"];
    let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Haunting, The", "Haunting, The"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(v)));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Haunting, The", Some(v)));
    assert!(!req.options.contains(&play_on(0, 0, b, Target::Minion(v))));
    let haunting = find_library("Haunting, The");
    let id = e
        .state
        .cards
        .iter()
        .position(|c| c.card == haunting && c.in_play && !c.effect_only && c.host == Some(v))
        .expect("on the vampire") as u32;
    assert!(!e.state.seats[0].ash.contains(&haunting));
    e.choose(fallback(&req));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 3, "its unlock phase");
    e.choose(index_of(&req, granted(id, v)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[id as usize].in_play);
    assert!(e.state.seats[0].ash.contains(&haunting));
}

/// New Management at the prey's 1-pool location: control changes; the
/// hand is replaced only once the action is over; a 2-pool location is
/// no target, one's own 1-pool one is. Backs `[TOM 19960226-1]`, `[LSJ
/// 20080803-2]`, `[LSJ 20001120]`, `[LSJ 20101021]` and `[RTR 19980623]`.
#[test]
fn new_management_takes_a_cheap_location() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let cheap = in_play(&mut e, 1, "Club Illusion", None);
    let dear = in_play(&mut e, 1, "Arcane Library", None);
    let own = in_play(&mut e, 0, "Market Square", None);
    for _ in 0..3 {
        e.state.seats[0].library.push(find_library("Form of Mist"));
    }
    hand(&mut e, 0, &["New Management", "Freak Drive"]);
    fill_hand(&mut e, 0);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play_on(0, 0, a, Target::Card(dear))));
    assert!(req.options.contains(&play_on(0, 0, a, Target::Card(own))));
    e.choose(index_of(&req, play_on(0, 0, a, Target::Card(cheap))));
    // Resolved, Freak Drive's window open: the location taken, the hand
    // not yet replaced.
    offering(&mut e, play(0, 0, a));
    assert_eq!(e.state.card_controller(cheap), SeatId(0));
    assert_eq!(e.state.seats[0].hand.len(), 6, "not replaced");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.seats[0].hand.len(),
        7,
        "replaced as the action ends"
    );
}

/// Blocked, New Management's card is replaced only once the action is
/// over, its combat included. Backs `[LSJ 20001120]` (group "Action not
/// replaced").
#[test]
fn new_management_is_not_replaced_when_blocked() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let b = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["pro"];
    let cheap = in_play(&mut e, 1, "Club Illusion", None);
    for _ in 0..3 {
        e.state.seats[0].library.push(find_library("Form of Mist"));
    }
    hand(&mut e, 0, &["New Management"]);
    fill_hand(&mut e, 0);
    pick(&mut e, play_on(0, 0, a, Target::Card(cheap)));
    pick(&mut e, block(b));
    for _ in 0..20 {
        if e.combat().is_some() {
            break;
        }
        let req = request(&mut e);
        e.choose(fallback(&req));
    }
    assert!(e.combat().is_some());
    assert_eq!(e.state.seats[0].hand.len(), 6, "not replaced in the combat");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.card_controller(cheap), SeatId(1));
    assert_eq!(e.state.seats[0].hand.len(), 7);
}

/// Psychic Veil: [obf] the player's later vampire actions this turn at +1
/// stealth, an ally's not; [OBF] the ally's too; neither the next turn.
#[test]
fn psychic_veil_cloaks_the_rest_of_the_turns_actions() {
    for superior_obf in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        if superior_obf {
            superior(&mut e, a, "obf");
        } else {
            e.state.minions[a.index()].gained_levels = vec!["obf"];
        }
        let v = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        let ally = e.add_ally(SeatId(0), find_library("ECTU Operative"));
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Psychic Veil"]);
        pick(&mut e, play(0, superior_obf as u8, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        pick(&mut e, bleed(v));
        request(&mut e);
        assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        pick(&mut e, bleed(ally));
        request(&mut e);
        assert_eq!(
            e.action().expect("bleed").stealth(&e.state),
            superior_obf as i32,
            "the ally, superior: {superior_obf}"
        );
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        pick(&mut e, bleed(a));
        request(&mut e);
        assert_eq!(e.action().expect("bleed").stealth(&e.state), 0);
    }
}

/// Brick Laying [POT] at the haven on the prey's younger vampire: the haven
/// burned, the vampire 1 blood down; an older vampire's haven is no
/// target, nor the older vampire.
#[test]
fn brick_laying_burns_a_haven_on_a_younger_vampire_and_wounds_it() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 6, 0), 4, false);
    superior(&mut e, a, "pot");
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let young = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 3, false);
    let old = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 0), 3, false);
    let haven = in_play(&mut e, 1, "Secure Haven", Some(young));
    let far = in_play(&mut e, 1, "Secure Haven", Some(old));
    hand(&mut e, 0, &["Brick Laying"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play_on(0, 2, a, Target::Card(far))));
    assert!(!req.options.contains(&play_on(0, 3, a, Target::Minion(old))));
    e.choose(index_of(&req, play_on(0, 2, a, Target::Card(haven))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[haven.index()].in_play);
    assert!(e.state.seats[1].ash.contains(&find_library("Secure Haven")));
    assert_eq!(blood(&e, young), 2);
}

/// Brick Laying [POT] at the prey's 1-life ally: locked, then burned by
/// the damage — a Ⓓ action burning an ally, Predator's Transformation
/// [ABO] answering it. Backs `[ANK 20220507]` for Brick Laying and
/// Predator's Transformation.
#[test]
fn brick_laying_locks_an_ally_and_its_damage_burns_it() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[a.index()].sect_override = Some(Sect::Anarch);
    superior(&mut e, a, "pot");
    superior(&mut e, a, "abo");
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Dark Web Procurer"));
    e.state.minions[ally.index()].blood = 1;
    hand(&mut e, 0, &["Brick Laying", "Predator's Transformation"]);
    pick(&mut e, play_on(0, 3, a, Target::Minion(ally)));
    let req = offering(&mut e, play(0, 3, a));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    e.choose(index_of(&req, play(0, 3, a)));
    request(&mut e);
    assert_eq!(pool(&e)[1], 10 - 2);
}

/// The card `name` in play, itself: its id.
fn card_in_play(e: &Engine, name: &str) -> CardId {
    let card = find_library(name);
    CardId(
        e.state
            .cards
            .iter()
            .position(|c| c.card == card && c.in_play && !c.effect_only)
            .unwrap_or_else(|| panic!("{name} not in play")) as u32,
    )
}

/// Anima Gathering [AUS]: the card on the acting vampire, the chosen other
/// vampire +2 intercept and +1 bleed; in the next unlock phase the bearer
/// may stay locked (the card kept), then unlocking burns it.
#[test]
fn anima_gathering_lends_intercept_while_its_vampire_stays_locked() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    superior(&mut e, a, "aus");
    let c = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Anima Gathering"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(c)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Anima Gathering", Some(a)));
    assert_eq!(e.state.bearer_mod(c, Stat::Intercept, ActionKind::Bleed), 2);
    assert_eq!(e.state.bearer_mod(c, Stat::Bleed, ActionKind::Bleed), 1);
    assert_eq!(e.state.bearer_mod(a, Stat::Intercept, ActionKind::Bleed), 0);
    let card = card_in_play(&e, "Anima Gathering");
    // Its controller's unlock phase: stay locked.
    let req = offering(&mut e, Choice::ResolvePrinted { minion: a });
    assert_eq!(req.seat, SeatId(0));
    e.choose(index_of(&req, Choice::Pass));
    request(&mut e);
    assert!(e.state.minions[a.index()].locked);
    assert!(e.state.cards[card.index()].in_play);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick_when(&mut e, Choice::ResolvePrinted { minion: a });
    request(&mut e);
    assert!(!e.state.minions[a.index()].locked);
    assert!(
        !e.state.cards[card.index()].in_play,
        "burned as it unlocked"
    );
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Anima Gathering")));
}

/// Anima Gathering's bearer unlocked by an effect (Freak Drive): the card
/// burns at once.
#[test]
fn anima_gathering_burns_when_its_vampire_unlocks_by_an_effect() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["aus", "for"];
    let c = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Anima Gathering", "Freak Drive"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(c)));
    let req = offering(&mut e, play(0, 0, a));
    assert!(placed_on(&e, "Anima Gathering", Some(a)));
    e.choose(index_of(&req, play(0, 0, a)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].locked);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Anima Gathering")));
}

/// Banishment on Anima Gathering's bearer, then influenced back: back
/// unlocked, which is no unlock — the card stays. Backs `[LSJ
/// 20060808-2] [LSJ 20060213]` (Anima Gathering, Banishment) and `[LSJ
/// 20060213] [LSJ 20060908]`.
#[test]
fn anima_gathering_survives_a_banishment() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 0), 6, false);
    e.state.minions[caller.index()].granted_title = Some(Title::Votes(3));
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let a = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 4, 0), 4, false);
    let c = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.state.minions[a.index()].locked = true;
    let card = in_play(&mut e, 0, "Anima Gathering", Some(a));
    e.state.cards[card.index()].chosen = vec![c];
    e.state.active = SeatId(1);
    hand(&mut e, 1, &["Banishment"]);
    pick(&mut e, play(0, 0, caller));
    pick_when(&mut e, Choice::ChooseMinion(a));
    pick_when(&mut e, votes(caller, true));
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert!(!e.state.minions[a.index()].in_play, "banished");
    let slot = e.state.seats[0]
        .uncontrolled
        .iter()
        .position(|u| u.banished.as_ref().is_some_and(|b| b.minion == a))
        .expect("banished slot") as u8;
    pick(&mut e, Choice::InfluenceOut { slot });
    request(&mut e);
    assert!(e.state.minions[a.index()].in_play);
    assert!(!e.state.minions[a.index()].locked);
    assert!(e.state.cards[card.index()].in_play, "not burned");
    assert_eq!(e.state.cards[card.index()].host, Some(a));
}

/// Blood of Sandman [tha] at the prey's ally: burned by a directed action.
#[test]
fn blood_of_sandman_burns_an_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tha"];
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Dark Web Procurer"));
    hand(&mut e, 0, &["Blood of Sandman"]);
    pick(&mut e, play_on(0, 0, a, Target::Minion(ally)));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play);
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Dark Web Procurer")));
}

/// Blood of Sandman [THA] on the prey's younger locked vampire: the card
/// itself on it; later, locked, it plays a reaction (Forced Awakening):
/// canceled as played, to the ash heap, no wake — its block never
/// offered. Backs `[ANK 20260216]` (group "Cancel, cost is still paid").
#[test]
fn blood_of_sandman_cancels_the_locked_vampires_reactions() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 4, false);
    superior(&mut e, a, "tha");
    let b = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let v = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 4, false);
    e.state.minions[v.index()].locked = true;
    let w = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[w.index()].locked = true;
    hand(&mut e, 0, &["Blood of Sandman"]);
    hand(&mut e, 1, &["Forced Awakening"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(v)));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 6);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Blood of Sandman", Some(v)));
    let card = card_in_play(&e, "Blood of Sandman");
    e.choose(index_of(&req, bleed(b)));
    pick_when(&mut e, play(0, 0, v));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(v));
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Forced Awakening")));
    assert_eq!(e.state.seats[1].pool, 9, "unblocked");
    assert!(e.state.cards[card.index()].in_play);
}

/// Blood of Sandman's vampire sent to torpor (a lost combat as it
/// blocks): the card burns.
#[test]
fn blood_of_sandman_burns_when_its_vampire_goes_to_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 0, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    let card = in_play(&mut e, 0, "Blood of Sandman", Some(v));
    e.state.cards[card.index()].mode = 1;
    pick(&mut e, bleed(a));
    pick(&mut e, block(v));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[v.index()].in_torpor);
    assert!(!e.state.cards[card.index()].in_play);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Blood of Sandman")));
}

/// Scrounging: the top three looked at (the fourth not offered), two moved
/// to hand, the hand then discarded down.
#[test]
fn scrounging_takes_two_of_the_top_three() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    // The top card replaces Scrounging; the next three are looked at.
    let [deep, c, b, top, replacement] =
        ["Bonding", "Forgery", "Leech", "Abomination", "Form of Mist"].map(find_library);
    for card in [deep, c, b, top, replacement] {
        e.state.seats[0].library.push(card);
    }
    hand(&mut e, 0, &["Scrounging"]);
    fill_hand(&mut e, 0);
    pick(&mut e, play(0, 0, a));
    let req = offering(&mut e, Choice::FindCard(top));
    let looked: Vec<u16> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::FindCard(c) => Some(*c),
            _ => None,
        })
        .collect();
    assert_eq!(looked.len(), 3, "{looked:?}");
    assert!(!looked.contains(&deep));
    e.choose(index_of(&req, Choice::FindCard(top)));
    pick(&mut e, Choice::FindCard(c));
    let hand_now = &e.state.seats[0].hand;
    assert!(hand_now.contains(&top) && hand_now.contains(&c));
    // Hand size + 2 - the played card: discarded down by one.
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::DiscardDown)
    );
}

/// Soul Scan by a capacity-6 vampire: [aus] finds a younger vampire with
/// no Obfuscate only; [AUS] also one with inferior Obfuscate, never one
/// with superior; the found one to the uncontrolled region, no blood.
#[test]
fn soul_scan_finds_a_younger_vampire_without_obfuscate() {
    let pick_crypt = |f: &dyn Fn(&zvtes::crypt::CryptCard) -> bool| {
        CRYPT
            .iter()
            .position(|c| !c.imbued && f(c))
            .expect("a crypt card") as u16
    };
    let none = pick_crypt(&|c| {
        c.capacity == 4 && !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("obf"))
    });
    let inferior = pick_crypt(&|c| c.capacity == 4 && c.disciplines.contains(&"obf"));
    let superior_obf = pick_crypt(&|c| c.capacity == 4 && c.disciplines.contains(&"OBF"));
    let older = pick_crypt(&|c| {
        c.capacity == 7 && !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("obf"))
    });
    for sup in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 6, 0), 4, false);
        if sup {
            superior(&mut e, a, "aus");
        } else {
            e.state.minions[a.index()].gained_levels = vec!["aus"];
        }
        e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        e.state.seats[0].crypt = vec![none, inferior, superior_obf, older].into();
        hand(&mut e, 0, &["Soul Scan"]);
        pick(&mut e, play(0, sup as u8, a));
        let req = offering(&mut e, Choice::AshVampire(none));
        let found: Vec<u16> = req
            .options
            .iter()
            .filter_map(|o| match o {
                Choice::AshVampire(c) => Some(*c),
                _ => None,
            })
            .collect();
        let mut want = vec![none];
        if sup {
            want.push(inferior);
        }
        want.sort();
        let mut found_sorted = found.clone();
        found_sorted.sort();
        assert_eq!(found_sorted, want, "superior: {sup}");
        e.choose(index_of(&req, Choice::AshVampire(none)));
        request(&mut e);
        let slot = e.state.seats[0].uncontrolled.last().expect("moved");
        assert_eq!((slot.card, slot.counters), (none, 0));
    }
}

/// Victim of Habit names a card of the prey's ash heap as played and
/// enters play; in another Methuselah's unlock phase its lock removes
/// three copies and the prey burns 1 pool; with fewer, they are removed
/// with no pool burned. Backs `[LSJ 20050408-2]`.
#[test]
fn victim_of_habit_burns_pool_for_three_copies() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let named = find_library("Bonding");
    for _ in 0..5 {
        e.state.seats[1].ash.push(named);
    }
    e.state.seats[1].ash.push(find_library("Forgery"));
    hand(&mut e, 0, &["Victim of Habit"]);
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::FindCard(named));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Victim of Habit", None));
    let card = card_in_play(&e, "Victim of Habit");
    assert_eq!(e.state.cards[card.index()].named.0, Some(named));
    let copies = |e: &Engine| e.state.seats[1].ash.iter().filter(|&&c| c == named).count();
    // The prey's unlock phase.
    until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
    pick(&mut e, ability(card.0, 0));
    request(&mut e);
    assert_eq!(copies(&e), 2);
    assert_eq!(e.state.seats[1].pool, 9);
    // Its own unlock phase, the card unlocked: two left, no pool.
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick(&mut e, ability(card.0, 0));
    request(&mut e);
    assert_eq!(copies(&e), 0);
    assert_eq!(e.state.seats[1].pool, 9);
    assert_eq!(
        e.state.seats[1]
            .removed
            .iter()
            .filter(|&&c| c == named)
            .count(),
        5
    );
}

/// Leech by a mortal ally at the prey's vampire: 1 blood stolen as 1 life,
/// the ally unlocked, then it meets basic Potence as a vampire — never
/// superior, never having Potence — until its controller's next unlock
/// phase. Backs `[LSJ 19980303]` (group "Meets missing disciplines").
#[test]
fn leech_steals_blood_as_life_and_lends_potence() {
    let brick = find_library("Brick Laying");
    let mut e = game(2);
    let ally = e.add_ally(SeatId(0), find_library("ECTU Operative"));
    let life = blood(&e, ally);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 3, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Leech"]);
    assert!(!zvtes::cards::has_disciplines(&e.state, ally, brick, 0));
    pick(&mut e, play_on(0, 0, ally, Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 2);
    assert_eq!(blood(&e, ally), life + 1);
    assert!(!e.state.minions[ally.index()].locked);
    assert!(zvtes::cards::has_disciplines(&e.state, ally, brick, 0));
    assert!(
        !zvtes::cards::has_disciplines(&e.state, ally, brick, 2),
        "never superior"
    );
    assert_eq!(e.state.discipline_level(ally, "pot"), 0);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        zvtes::cards::has_disciplines(&e.state, ally, brick, 0),
        "the prey's turn"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !zvtes::cards::has_disciplines(&e.state, ally, brick, 0),
        "its unlock phase"
    );
}

/// Leech at a vampire with no blood: playable, nothing stolen. Backs `[RTR
/// 20010711]`.
#[test]
fn leech_targets_a_vampire_with_no_blood() {
    let mut e = game(2);
    let ally = e.add_ally(SeatId(0), find_library("ECTU Operative"));
    let life = blood(&e, ally);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 0, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Leech"]);
    pick(&mut e, play_on(0, 0, ally, Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, v), blood(&e, ally)), (0, life));
    assert!(zvtes::cards::has_disciplines(
        &e.state,
        ally,
        find_library("Brick Laying"),
        0
    ));
}

/// A Ghouled ally (a ghoul, its own strike-only [pot] grant) plays Leech:
/// two Potence grants never make superior Potence. Backs `[ANK 20180111]`
/// for Leech and Ghouled.
#[test]
fn leech_and_ghouled_never_make_superior_potence() {
    let brick = find_library("Brick Laying");
    let mut e = game(2);
    let ally = e.add_ally(SeatId(0), find_library("ECTU Operative"));
    in_play(&mut e, 0, "Ghouled", Some(ally));
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 3, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Leech"]);
    pick(&mut e, play_on(0, 0, ally, Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(zvtes::cards::has_disciplines(&e.state, ally, brick, 0));
    assert!(!zvtes::cards::has_disciplines(&e.state, ally, brick, 2));
}

/// Leech by an imbued (a mortal): it unlocks, burns its convictions and
/// meets basic Potence until its next unlock phase (rules-qa round 29,
/// red).
#[test]
fn leech_by_an_imbued_burns_its_convictions() {
    let mut e = game(2);
    let imbued = CRYPT.iter().position(|c| c.imbued).expect("an imbued") as u16;
    let i = e.add_minion(SeatId(0), imbued, 3, false);
    let conviction = in_play(&mut e, 0, "Second Sight", Some(i));
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 3, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Leech"]);
    pick(&mut e, play_on(0, 0, i, Target::Minion(v)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, v), 2);
    assert!(!e.state.cards[conviction.index()].in_play);
    assert!(zvtes::cards::has_disciplines(
        &e.state,
        i,
        find_library("Brick Laying"),
        0
    ));
}

/// Blood Hunt by a prince on another Methuselah's untitled vampire: the
/// card itself on it; a third Methuselah's vampire may rush it at +1
/// stealth, its own Methuselah's may not; a prince is no target, and an
/// untitled vampire does not play the card.
#[test]
fn blood_hunt_lets_others_rush_the_hunted_vampire() {
    let mut e = game(3);
    let p = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
    let untitled = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let prince = e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    e.state.minions[prince.index()].granted_title = Some(Title::Prince("Chicago"));
    let hunter = e.add_minion(SeatId(2), camarilla("Brujah", 0), 4, false);
    e.add_minion(SeatId(2), camarilla("Brujah", 1), 4, false);
    hand(&mut e, 0, &["Blood Hunt"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req
        .options
        .contains(&play_on(0, 0, p, Target::Minion(prince))));
    assert!(!plays(&req)
        .iter()
        .any(|c| matches!(c, Choice::Play { actor: Some(a), .. } if *a == untitled)));
    e.choose(index_of(&req, play_on(0, 0, p, Target::Minion(v))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Blood Hunt", Some(v)));
    let card = card_in_play(&e, "Blood Hunt");
    let rush = |m| Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(card),
        ability: 0,
        actor: Some(m),
        target: Target::Minion(v),
    };
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!req.options.iter().any(|o| matches!(o, Choice::UseAbility { source: zvtes::cards::AbilitySource::Card(c), .. } if *c == card)));
    e.choose(fallback(&req));
    let req = until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    e.choose(index_of(&req, rush(hunter)));
    request(&mut e);
    let a = e.action().expect("the rush");
    assert_eq!(a.stealth(&e.state), 1);
    assert_eq!(a.directed_seat(&e.state), Some(SeatId(1)));
}

/// Karsh (advanced, untitled) plays Blood Hunt from his player's ash heap
/// as a prince: offered to him only, the card not replaced.
#[test]
fn karsh_plays_blood_hunt_from_his_ash_heap_as_a_prince() {
    let mut e = game(2);
    let karsh = CRYPT
        .iter()
        .position(|c| c.name == "Karsh" && c.adv)
        .expect("Karsh (ADV)") as u16;
    let k = e.add_minion(SeatId(0), karsh, 4, false);
    let other = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    e.state.seats[0].ash.push(find_library("Blood Hunt"));
    for _ in 0..3 {
        e.state.seats[0].library.push(find_library("Form of Mist"));
    }
    hand(&mut e, 0, &["Form of Mist"]);
    fill_hand(&mut e, 0);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let ash = e.state.seats[0].hand.len() as u8;
    assert!(!req
        .options
        .contains(&play_on(ash, 0, other, Target::Minion(v))));
    e.choose(index_of(&req, play_on(ash, 0, k, Target::Minion(v))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Blood Hunt", Some(v)));
    assert_eq!(e.state.seats[0].library.len(), 3, "not replaced");
}

/// Seat 0's `a` plays Abomination at a werewolf ally recruited this turn,
/// moving `moved` blood: the new vampire.
fn abomination(e: &mut Engine, a: MinionId, moved: u8) -> MinionId {
    let wolf = e.add_ally(SeatId(0), find_library("Tunnel Runner"));
    e.state.minions[wolf.index()].recruited = true;
    let before = e.state.minions.len();
    hand(e, 0, &["Abomination"]);
    let card = (e.state.seats[0].hand.len() - 1) as u8;
    pick(e, play_on(card, 0, a, Target::Minion(wolf)));
    pick_when(e, Choice::SetX(moved));
    request(e);
    assert!(
        !e.state.minions[wolf.index()].in_play,
        "the werewolf burned"
    );
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Tunnel Runner")));
    MinionId(before as u32)
}

/// Abomination burns a werewolf recruited this turn: the card itself in
/// play, a unique Independent Abomination with 4 capacity, superior
/// Protean, +1 strength, the blood moved; it rushes the prey's vampire,
/// its additional strike mandatory: 2 + 2 damage. Backs `[RTR 20180303]`
/// and `[PIB 20121028]` (group "Mandatory additional strike").
#[test]
fn abomination_burns_a_werewolf_and_becomes_a_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 6, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 5, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    let abo = abomination(&mut e, a, 4);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Abomination", None));
    let v = &e.state.minions[abo.index()];
    assert!(v.in_play && v.is_vampire());
    assert_eq!(v.clan(), Some("Abomination"));
    assert_eq!(e.state.sect_of(abo), Some(Sect::Independent));
    assert_eq!(e.state.capacity_of(abo), Some(4));
    assert_eq!(e.state.discipline_level(abo, "pro"), 2);
    assert_eq!((blood(&e, abo), blood(&e, a)), (4, 2));
    let card = card_in_play(&e, "Abomination");
    let rush = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(card),
        ability: 1,
        actor: Some(abo),
        target: Target::Minion(z),
    };
    pick(&mut e, rush);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, z), 5 - 2 - 2);
}

/// A second Abomination is not played while one is in play under its
/// player; one under another Methuselah is its rival as a unique vampire
/// (rules-qa round 29, red).
#[test]
fn abomination_is_a_unique_vampire_contested_across_methuselahs() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Tremere", 0), 8, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 5, false);
    let first = abomination(&mut e, a, 1);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.add_ally(SeatId(0), find_library("Tunnel Runner"));
    hand(&mut e, 0, &["Abomination"]);
    e.choose(fallback(&req));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(plays(&req).is_empty(), "{:?}", plays(&req));
    // The first now the prey's: a second is played, and contests it.
    e.state.minions[first.index()].take_for_good(SeatId(1));
    let card = card_in_play(&e, "Abomination");
    e.state.cards[card.index()].controller = SeatId(1);
    e.state.seats[0]
        .hand
        .retain(|&c| c != find_library("Abomination"));
    let second = abomination(&mut e, a, 1);
    assert_eq!(e.state.card_rivals(second), vec![first]);
}

/// Seat 0's `a` (a Toreador) plays The Becoming putting `masters` from hand
/// on the new vampire, 1 blood moved: the new vampire.
fn becoming(e: &mut Engine, a: MinionId, masters: &[&str]) -> MinionId {
    for _ in 0..4 {
        e.state.seats[0].library.push(find_library("Form of Mist"));
    }
    hand(e, 0, &["Becoming, The"]);
    hand(e, 0, masters);
    let before = e.state.minions.len();
    let card = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Becoming, The"))
        .expect("in hand") as u8;
    pick(e, play(card, 0, a));
    for m in masters {
        pick_when(
            e,
            Choice::TakeFrom {
                zone: zvtes::decision::Zone::Hand,
                card: find_library(m),
            },
        );
    }
    if masters.is_empty() {
        pick_when(e, Choice::Pass);
    } else {
        pick_when(e, Choice::SetX(1));
    }
    request(e);
    MinionId(before as u32)
}

/// The Becoming: a Toreador Camarilla vampire of 0 capacity +1 per master
/// Discipline card put on it from hand, 1 blood moved, idle this turn.
#[test]
fn the_becoming_makes_a_vampire_of_its_discipline_cards() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 3, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let n = becoming(&mut e, a, &["Celerity", "Dominate"]);
    let v = &e.state.minions[n.index()];
    assert!(v.in_play && v.is_vampire());
    assert_eq!(v.clan(), Some("Toreador"));
    assert_eq!(e.state.sect_of(n), Some(Sect::Camarilla));
    assert_eq!(e.state.capacity_of(n), Some(2));
    assert_eq!((blood(&e, n), blood(&e, a)), (1, 0), "3 - 2 cost - 1");
    assert!(placed_on(&e, "Becoming, The", None));
    assert!(e.state.minions[n.index()].recruited, "cannot act this turn");
    for m in ["Celerity", "Dominate"] {
        assert!(!e.state.seats[0].hand.contains(&find_library(m)));
        assert!(e
            .state
            .cards
            .iter()
            .any(|c| c.in_play && c.host == Some(n) && c.card == find_library(m)));
    }
}

/// The Becoming with no Discipline card put: the vampire burns at once, its
/// card a library card in the ash heap, no vampire there. Backs `[LSJ
/// 20020115]` (group "Action creating vampire").
#[test]
fn the_becoming_burns_without_discipline_cards() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 3, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let n = becoming(&mut e, a, &[]);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[n.index()].in_play);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Becoming, The")));
    assert!(e.state.seats[0].ash_crypt.is_empty());
}

/// The Becoming's vampires of two Methuselahs, each at 1 pool under
/// Tension in the Ranks ("after a ready minion is burned …, their
/// controller burns 1 pool"), left without Discipline cards in one step:
/// burned together, the acting Methuselah orders the burns (LSJ
/// 20090208-2; rules-qa 118.2).
#[test]
fn the_becoming_vampires_burned_together_have_their_burns_ordered() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 3, false);
    let b = e.add_minion(SeatId(0), camarilla("Toreador", 1), 3, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), camarilla("Ventrue", 1), 4, false);
    let n = becoming(&mut e, a, &["Celerity"]);
    let m = becoming(&mut e, b, &["Dominate"]);
    e.state.minions[m.index()].owner = SeatId(1);
    in_play(&mut e, 2, "Tension in the Ranks", None);
    e.state.seats[0].pool = 1;
    e.state.seats[1].pool = 1;
    for c in &mut e.state.cards {
        if c.host == Some(n) || c.host == Some(m) {
            c.out_of_play = true;
        }
    }
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::ResolutionChoice)
    );
    assert_eq!(
        req.options,
        vec![Choice::ChooseSeat(SeatId(0)), Choice::ChooseSeat(SeatId(1))]
    );
}

/// Created vampire `n` banished by hand (Banishment's move), with
/// `counters` blood: its slot.
fn banish(e: &mut Engine, n: MinionId, counters: u8) -> u8 {
    let card = e
        .state
        .cards
        .iter()
        .position(|c| c.body == Some(n))
        .expect("its card");
    let held: Vec<CardId> = e
        .state
        .cards
        .iter()
        .enumerate()
        .filter(|(_, c)| c.host == Some(n) && c.in_play)
        .map(|(i, _)| CardId(i as u32))
        .collect();
    e.state.minions[n.index()].in_play = false;
    e.state.minions[n.index()].blood = 0;
    e.state.cards[card].in_play = false;
    let mut cards = vec![CardId(card as u32)];
    cards.extend(held);
    e.state.seats[0].uncontrolled.push(UncontrolledSlot {
        card: zvtes::state::CREATED,
        copy: Default::default(),
        counters,
        banished: Some(zvtes::state::Banished { minion: n, cards }),
    });
    e.state.seats[0].uncontrolled.len() as u8 - 1
}

/// Banished, Abomination and The Becoming stay vampires and are influenced
/// again; The Becoming is a 1-capacity vampire there. Backs `[RTR
/// 19990712]` for both and `[LSJ 20090625]`.
#[test]
fn a_banished_created_vampire_can_be_influenced_again() {
    for abo in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), elder(), 8, false);
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        let n = if abo {
            abomination(&mut e, a, 1)
        } else {
            becoming(&mut e, a, &["Celerity"])
        };
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let cap = if abo { 4 } else { 1 };
        let slot = banish(&mut e, n, cap);
        assert_eq!(e.state.slot_vampire(SeatId(0), slot as usize).0, cap);
        until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        pick(&mut e, Choice::InfluenceOut { slot });
        request(&mut e);
        let v = &e.state.minions[n.index()];
        assert!(v.in_play && v.is_vampire(), "Abomination: {abo}");
    }
}

/// The Becoming in the uncontrolled region with no Discipline card on it:
/// a 1-capacity vampire, never 0. Backs `[LSJ 20090625]`.
#[test]
fn a_banished_becoming_is_a_one_capacity_vampire() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 3, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let n = becoming(&mut e, a, &["Celerity"]);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let master = e
        .state
        .cards
        .iter()
        .position(|c| c.host == Some(n) && c.in_play)
        .expect("the master card");
    e.state.cards[master].in_play = false;
    let slot = banish(&mut e, n, 0);
    assert_eq!(e.state.slot_vampire(SeatId(0), slot as usize).0, 1);
}

/// Kaymakli Nightmares moving Abomination or The Becoming to the crypt: it
/// is burned instead, its card to the ash heap. Backs `[LSJ 20041201-2]`
/// for both.
#[test]
fn a_created_vampire_going_to_the_crypt_is_burned() {
    for abo in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), elder(), 8, false);
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        let (n, name) = if abo {
            (abomination(&mut e, a, 1), "Abomination")
        } else {
            (becoming(&mut e, a, &["Celerity"]), "Becoming, The")
        };
        banish(&mut e, n, 1);
        hand(&mut e, 0, &["Kaymakli Nightmares"]);
        to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
        to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
        let k = e.state.seats[0]
            .hand
            .iter()
            .position(|&c| c == find_library("Kaymakli Nightmares"))
            .expect("in hand") as u8;
        pick(&mut e, master_play(k));
        request(&mut e);
        let s0 = &e.state.seats[0];
        assert!(s0.uncontrolled.is_empty(), "{name}: burned");
        assert!(s0.ash.contains(&find_library(name)));
        assert!(s0.crypt.is_empty());
    }
}

/// A burned Abomination (diablerized in torpor) is a library card in the
/// ash heap, no vampire. Backs `[LSJ 20020115]` (group "Action creating
/// vampire").
#[test]
fn a_burned_abomination_is_a_library_card_in_the_ash_heap() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), elder(), 8, false);
    let d = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let n = abomination(&mut e, a, 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[n.index()].in_torpor = true;
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(n),
        },
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[n.index()].in_play);
    assert!(e.state.seats[0].ash_crypt.is_empty());
    assert!(e.state.seats[0].ash.contains(&find_library("Abomination")));
}

/// Repair the Undead Flesh [for] on the acting vampire: struck for 1
/// aggravated (an aggravated hand strike), it heals it for 3 blood, no
/// wound; sent to torpor later, the card burns.
#[test]
fn repair_the_undead_flesh_heals_aggravated_damage_for_blood() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 7, false);
    e.state.minions[a.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    e.state.minions[z.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::Never,
        SeatId(1),
    ));
    hand(&mut e, 0, &["Repair the Undead Flesh"]);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Repair the Undead Flesh", Some(a)));
    let card = card_in_play(&e, "Repair the Undead Flesh");
    let heal = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(card),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    // The next turn: a bleeds, z blocks, strikes for 1 aggravated.
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, heal);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[a.index()];
    assert!(!v.in_torpor, "healed, not wounded");
    assert_eq!(blood(&e, a), 7 - 3);
    assert!(e.state.cards[card.index()].in_play);
}

/// Sword of Nuln's strike made aggravated (Sword of the Righteous [val]):
/// Repair [FOR] heals it at twice its rate, 4 blood. Backs `[LSJ 20031219]
/// [LSJ 20100527]` for Sword of Nuln and Repair the Undead Flesh.
#[test]
fn repair_the_undead_flesh_pays_double_against_sword_of_nuln() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), elder(), 8, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["val"];
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    let repair = in_play(&mut e, 0, "Repair the Undead Flesh", Some(a));
    e.state.cards[repair.index()].mode = 1;
    let sword = in_play(&mut e, 1, "Sword of Nuln", Some(z));
    hand(&mut e, 1, &["Sword of the Righteous"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 1,
            actor: Some(z),
            target: Target::Card(sword),
        },
    );
    pick_when(
        &mut e,
        Choice::UseAbility {
            source: zvtes::cards::AbilitySource::Card(sword),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    let heal = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(repair),
        ability: 1,
        actor: None,
        target: Target::None,
    };
    let req = offering(&mut e, heal);
    let pending = e
        .combat()
        .expect("combat")
        .sides
        .iter()
        .find(|s| s.minion == a)
        .map(|s| s.pending.aggravated)
        .unwrap_or(0);
    assert!(pending >= 1, "aggravated pending");
    e.choose(index_of(&req, heal));
    request(&mut e);
    assert_eq!(blood(&e, a), 8 - 4);
}

/// The Summoning [pre]: the search offers allies and retainers only; the
/// found one to hand.
#[test]
fn the_summoning_fetches_an_ally_or_retainer_to_hand() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Ventrue", 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["pre"];
    e.add_minion(SeatId(0), camarilla("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Toreador", 0), 4, false);
    let [ally, retainer, other] = ["ECTU Operative", "Ghoul Retainer", "Bonding"].map(find_library);
    for c in [ally, retainer, other, find_library("Form of Mist")] {
        e.state.seats[0].library.push(c);
    }
    hand(&mut e, 0, &["Summoning, The"]);
    fill_hand(&mut e, 0);
    pick(&mut e, play(0, 0, a));
    let req = offering(&mut e, Choice::FindCard(retainer));
    assert!(req.options.contains(&Choice::FindCard(ally)));
    assert!(!req.options.contains(&Choice::FindCard(other)));
    e.choose(index_of(&req, Choice::FindCard(retainer)));
    request(&mut e);
    assert!(e.state.seats[0].hand.contains(&retainer));
}

/// The Summoning [PRE] by a Ventrue: the library's allies it could recruit
/// as normal offered — not another clan's, not one it cannot pay for; the
/// found one recruited, unable to act this turn; Paths in Two Worlds not
/// played on it. Backs `[RBK recruit-ally]`, `[ANK 20250526]`, `[ANK
/// 20180817] [LSJ 20100303] [LSJ 20100725]` and `[LSJ 20080803]`.
#[test]
fn the_summoning_recruits_an_ally_from_the_library() {
    let mut e = game(2);
    e.state.seats[0].pool = 2;
    let a = e.add_minion(SeatId(0), camarilla("Ventrue", 0), 4, false);
    superior(&mut e, a, "pre");
    e.add_minion(SeatId(0), camarilla("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Toreador", 0), 4, false);
    let [cheap, clan, other_clan, dear] = [
        "Vagabond Mystic",
        "Political Ally",
        "Double Deuce",
        "Michael Harris, the Incinerator",
    ]
    .map(find_library);
    for c in [cheap, clan, other_clan, dear, find_library("Form of Mist")] {
        e.state.seats[0].library.push(c);
    }
    hand(&mut e, 0, &["Summoning, The", "Paths in Two Worlds"]);
    fill_hand(&mut e, 0);
    pick(&mut e, play(0, 1, a));
    // Paths in Two Worlds (hand slot 0 now): never offered on it.
    never_offered_where(&mut e, SeatId(0), WindowLabel::LibrarySearch, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
    let req = offering(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: cheap,
        },
    );
    let found: Vec<u16> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::TakeFrom { card, .. } => Some(*card),
            _ => None,
        })
        .collect();
    assert!(found.contains(&clan));
    assert!(!found.contains(&other_clan), "another clan's");
    assert!(!found.contains(&dear), "unaffordable");
    e.choose(index_of(
        &req,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: cheap,
        },
    ));
    request(&mut e);
    let m = e
        .state
        .minions
        .iter()
        .position(|v| v.in_play && v.kind.ally_card() == Some(cheap))
        .expect("recruited");
    assert!(e.state.minions[m].recruited, "cannot act this turn");
    assert!(e.state.seats[0]
        .hand
        .contains(&find_library("Paths in Two Worlds")));
}

/// The Summoning [PRE]'s recruit costs as normal, the recruiter's
/// surcharges too (rules-qa 116.76, 119.2): Darlene Killian ("Allies and
/// retainers cost Darlene 1 additional blood") pays 1 blood for a Vagabond
/// Mystic.
#[test]
fn the_summoning_charges_darlenes_surcharge() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Darlene Killian"), 2, false);
    superior(&mut e, d, "pre");
    e.add_minion(SeatId(0), camarilla("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Summoning, The"]);
    fill_hand(&mut e, 0);
    let mystic = find_library("Vagabond Mystic");
    // The Summoning's replacement drawn from the top: not the ally.
    for c in [mystic, find_library("Form of Mist")] {
        e.state.seats[0].library.push(c);
    }
    pick(&mut e, play(0, 1, d));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: mystic,
        },
    );
    request(&mut e);
    assert!(e
        .state
        .minions
        .iter()
        .any(|v| v.in_play && v.kind.ally_card() == Some(mystic)));
    assert_eq!(blood(&e, d), 2 - 1);
}

/// The Summoning [PRE]'s recruit pays the recruiter's cost increase too
/// (rules-qa 116.76, 119.2): Jenna Cross ("Cards that cost blood cost Jenna 1
/// additional blood to play") pays 1 + 1 for a Repo Man.
#[test]
fn the_summoning_charges_jenna_cross_her_blood() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Jenna Cross"), 4, false);
    superior(&mut e, j, "pre");
    e.add_minion(SeatId(0), camarilla("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Summoning, The"]);
    fill_hand(&mut e, 0);
    let repo = find_library("Repo Man");
    for c in [repo, find_library("Form of Mist")] {
        e.state.seats[0].library.push(c);
    }
    pick(&mut e, play(0, 1, j));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: repo,
        },
    );
    request(&mut e);
    assert!(e
        .state
        .minions
        .iter()
        .any(|v| v.in_play && v.kind.ally_card() == Some(repo)));
    assert_eq!(blood(&e, j), 4 - 1 - 1);
}

/// The Summoning [PRE] is a recruit action: Frau Schädel's "+1 stealth
/// during recruit and employ actions" applies. Backs `[TOM 19960130]`.
#[test]
fn the_summoning_is_a_recruit_action() {
    let mut e = game(2);
    let frau = e.add_minion(SeatId(0), find_card("Frau Schädel"), 4, false);
    superior(&mut e, frau, "pre");
    e.add_minion(SeatId(0), camarilla("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Summoning, The"]);
    pick(&mut e, play(0, 1, frau));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 2);
}

/// Will of the Council on a primogen: the card itself on it; a second not
/// played on it; the bearer burns it to play Sixth Tradition: Destruction
/// (a prince's card) as if a prince, the card to the ash heap.
#[test]
fn will_of_the_council_plays_a_prince_card() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Primogen);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let d = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Will of the Council"]);
    pick(&mut e, play(0, 0, p));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "Will of the Council", Some(p)));
    let will = card_in_play(&e, "Will of the Council");
    // The next turn: a second Will masked; Sixth Tradition as a prince.
    e.state.seats[0].previous_turn = e.state.turn;
    hand(
        &mut e,
        0,
        &["Will of the Council", "Sixth Tradition: Destruction"],
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[d.index()].diablerie_turn = Some(e.state.turn);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, p)), "one per vampire");
    e.choose(index_of(&req, play_on(1, 0, p, Target::Minion(d))));
    request(&mut e);
    assert!(!e.state.cards[will.index()].in_play, "burned to play it");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[d.index()].in_play, "burned by the action");
}

/// Will of the Council: in a political referendum its primogen burns 1
/// blood for 1 vote, once; as another Methuselah's prince goes to torpor,
/// the card burned, the primogen takes that title (contested).
#[test]
fn will_of_the_council_buys_a_vote_and_claims_a_torpid_princes_title() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let p = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.state.minions[p.index()].granted_title = Some(Title::Primogen);
    let will = in_play(&mut e, 0, "Will of the Council", Some(p));
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 0, false);
    e.state.minions[z.index()].granted_title = Some(Title::Prince("Chicago"));
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Praxis Seizure: Atlanta"]);
    pick(&mut e, play(0, 0, caller));
    to_polling(&mut e);
    let before = e.referendum().expect("referendum").tally(&e.state);
    let buy = Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(will),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick(&mut e, buy);
    let req = offering(
        &mut e,
        Choice::CastVotes {
            minion: p,
            favor: true,
        },
    );
    assert!(!req.options.contains(&buy), "once");
    e.choose(index_of(
        &req,
        Choice::CastVotes {
            minion: p,
            favor: true,
        },
    ));
    request(&mut e);
    assert_eq!(blood(&e, p), 3);
    let after = e.referendum().expect("referendum").tally(&e.state);
    assert!(after.0 > before.0, "{before:?} {after:?}");
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // The prince blocks and goes to torpor.
    pick(&mut e, bleed(p));
    pick_when(&mut e, block(z));
    pick_when(&mut e, Choice::ResolveRider { card: will.0 });
    request(&mut e);
    assert!(e.state.minions[z.index()].in_torpor);
    assert_eq!(e.state.claimed_title(p), Title::Prince("Chicago"));
    assert!(!e.state.cards[will.index()].in_play);
}

/// Will of the Council: "this primogen" is read live (rules-digest D31;
/// rules-qa 9.1, 70.20): its bearer's primogen title contested, it plays
/// no prince card, buys no vote and claims no torpid prince's title.
#[test]
fn will_of_the_council_needs_its_bearer_still_a_primogen() {
    let setup = |contested: bool| {
        let mut e = game(2);
        let caller = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        let p = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        e.state.minions[p.index()].granted_title = Some(Title::Primogen);
        let will = in_play(&mut e, 0, "Will of the Council", Some(p));
        let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 0, false);
        e.state.minions[z.index()].granted_title = Some(Title::Prince("Chicago"));
        e.state.minions[z.index()].diablerie_turn = Some(e.state.turn);
        e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
        e.state.minions[p.index()].title_contested = contested;
        (e, caller, p, will, z)
    };
    for contested in [false, true] {
        let (mut e, _, p, _, z) = setup(contested);
        hand(&mut e, 0, &["Sixth Tradition: Destruction"]);
        let req = request(&mut e);
        let sixth = play_on(0, 0, p, Target::Minion(z));
        assert_eq!(
            req.options.contains(&sixth),
            !contested,
            "contested: {contested}"
        );
    }
    let (mut e, caller, p, will, z) = setup(true);
    hand(&mut e, 0, &["Praxis Seizure: Atlanta"]);
    pick(&mut e, play(0, 0, caller));
    to_polling(&mut e);
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::UseAbility {
        source: AbilitySource::Card(will),
        ability: 0,
        actor: None,
        target: Target::None,
    }));
    close_polling(&mut e);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(p));
    pick_when(&mut e, block(z));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::ResolveRider { card: will.0 },
    );
    assert!(e.state.minions[z.index()].in_torpor);
    assert!(e.state.cards[will.index()].in_play, "not burned");
    assert_eq!(e.state.claimed_title(p), Title::Primogen);
    assert_eq!(e.state.claimed_title(z), Title::Prince("Chicago"));
}

/// Vidal Jarbeaux (a Toreador) by The Summoning [PRE]: an ally requiring a
/// Ventrue is not offered — his ability meets requirements only for cards
/// played normally. Backs `[LSJ 20050119]` for The Summoning (Vidal
/// Jarbeaux, group "Impersonating requirements").
#[test]
fn vidal_cannot_meet_a_summoned_allys_requirement() {
    let mut e = game(2);
    e.state.seats[0].pool = 5;
    let v = e.add_minion(SeatId(0), find_card("Vidal Jarbeaux"), 4, false);
    superior(&mut e, v, "pre");
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let [mystic, ventrue] = ["Vagabond Mystic", "Political Ally"].map(find_library);
    for c in [mystic, ventrue, find_library("Form of Mist")] {
        e.state.seats[0].library.push(c);
    }
    hand(&mut e, 0, &["Summoning, The"]);
    fill_hand(&mut e, 0);
    pick(&mut e, play(0, 1, v));
    let req = offering(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: mystic,
        },
    );
    assert!(!req.options.contains(&Choice::TakeFrom {
        zone: zvtes::decision::Zone::Library,
        card: ventrue,
    }));
}

/// Blood of Sandman [THA]: its target unlocked by resolution, the action
/// fizzles: no card put on it, the card to the ash heap, its 2 blood not
/// paid (a lost target, rules-qa 117.1); not fizzling, they are. Backs
/// `[ANK 20210124] [LSJ 20070411] [LSJ 20090514]` (group "Action targeting
/// locked minion").
#[test]
fn blood_of_sandman_fizzles_on_a_target_unlocked_by_resolution() {
    for fizzle in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 4, false);
        superior(&mut e, a, "tha");
        e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        let v = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 4, false);
        e.state.minions[v.index()].locked = true;
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Blood of Sandman"]);
        pick(&mut e, play_on(0, 1, a, Target::Minion(v)));
        request(&mut e);
        e.state.minions[v.index()].locked = !fizzle;
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if !fizzle {
            assert!(placed_on(&e, "Blood of Sandman", Some(v)));
            assert_eq!(blood(&e, a), 4 - 2, "paid");
            continue;
        }
        let card = find_library("Blood of Sandman");
        assert!(!e.state.cards.iter().any(|c| c.card == card && c.in_play));
        assert!(e.state.seats[0].ash.contains(&card));
        assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
        assert_eq!(blood(&e, a), 4, "not paid");
    }
}

/// After The Summoning [PRE], the vampire, unlocked, may still recruit an
/// ally from hand this turn. Backs `[ANK 20180817]` (group
/// "Equip/Employ/Recruit action card").
#[test]
fn the_summoning_does_not_stop_a_normal_recruit() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Ventrue", 0), 4, false);
    superior(&mut e, a, "pre");
    e.add_minion(SeatId(0), camarilla("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Toreador", 0), 4, false);
    let mystic = find_library("Vagabond Mystic");
    for c in [mystic, find_library("Form of Mist")] {
        e.state.seats[0].library.push(c);
    }
    hand(&mut e, 0, &["Summoning, The", "ECTU Operative"]);
    fill_hand(&mut e, 0);
    pick(&mut e, play(0, 1, a));
    pick_when(
        &mut e,
        Choice::TakeFrom {
            zone: zvtes::decision::Zone::Library,
            card: mystic,
        },
    );
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let _ = req;
    e.state.minions[a.index()].locked = false;
    let req = request(&mut e);
    assert!(
        plays(&req)
            .iter()
            .any(|c| matches!(c, Choice::Play { card: 0, actor: Some(m), .. } if *m == a)),
        "{:?}",
        plays(&req)
    );
}

/// Victim of Habit after its player's prey is ousted: the named card is
/// kept, the new prey's ash heap read. Backs `[ANK 20230103]`.
#[test]
fn victim_of_habit_reads_the_new_prey() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), camarilla("Ventrue", 1), 4, false);
    let named = find_library("Bonding");
    e.state.seats[1].ash.push(named);
    for _ in 0..3 {
        e.state.seats[2].ash.push(named);
    }
    hand(&mut e, 0, &["Victim of Habit"]);
    // The prey's one card: its name elided.
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = card_in_play(&e, "Victim of Habit");
    e.state.seats[1].ousted = true;
    until(&mut e, SeatId(2), WindowLabel::UnlockAbilities);
    pick(&mut e, ability(card.0, 0));
    request(&mut e);
    assert_eq!(e.state.cards[card.index()].named.0, Some(named));
    assert_eq!(
        e.state.seats[2].ash.iter().filter(|&&c| c == named).count(),
        0
    );
    assert_eq!(e.state.seats[2].pool, 9);
}

// ---- Masters and an event ---------------------------------------------------------

/// The Methuselah plays hand card `card` at `target`.
fn master_at(card: u8, target: Target) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: None,
        target,
    }
}

/// Ability `ability` of card `card` in play, by `actor` at `target`.
fn use_at(card: CardId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(card),
        ability,
        actor,
        target,
    }
}

/// Al's Army Apparatus: in its controller's minion phase, locked to search
/// a weapon to hand, not shown (no "show" printed: PIB 20121028); not in
/// the master phase.
#[test]
fn als_army_apparatus_searches_a_weapon_unseen_in_the_minion_phase() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let al = in_play(&mut e, 0, "Al's Army Apparatus", None);
    let magnum = find_library(".44 Magnum");
    fill_hand(&mut e, 0);
    for c in ["Form of Mist", ".44 Magnum", "Deflection"] {
        e.state.seats[0].library.push(find_library(c));
    }
    e.notes = Some(Vec::new());
    pick(&mut e, ability(al.0, 0));
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&Choice::FindCard(find_library("Deflection"))));
    e.choose(index_of(&req, Choice::FindCard(magnum)));
    request(&mut e);
    assert!(e.state.seats[0].hand.contains(&magnum));
    assert!(!e.state.seats[0].library.contains(&magnum));
    assert!(e.state.cards[al.index()].locked);
    assert!(
        noted_reveals(&mut e).iter().all(|r| r.card != magnum),
        "not shown"
    );
    // The master phase: not offered.
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let al = in_play(&mut e, 0, "Al's Army Apparatus", None);
    hand(&mut e, 0, &["Instability"]);
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(req.options.contains(&master_play(0)));
    assert!(!req.options.contains(&ability(al.0, 0)));
}

/// Elysium: The Arboretum: locked before range in a combat between its
/// controller's Camarilla vampire and another Camarilla vampire, the combat
/// ends, no strike; not offered against a Sabbat blocker. Backs `[RTR
/// 20001020] [LSJ 20001024]` (end-of-round cards still played after).
#[test]
fn elysium_the_arboretum_ends_a_camarilla_combat_before_range() {
    for sabbat in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        let b = if sabbat {
            e.add_minion(SeatId(1), of_cap(Sect::Sabbat, 4, 0), 4, false)
        } else {
            e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false)
        };
        e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
        let el = in_play(&mut e, 0, "Elysium: The Arboretum", None);
        hand(&mut e, 0, &["Taste of Vitae"]);
        let end = ability(el.0, 0);
        pick(&mut e, bleed(a));
        pick(&mut e, block(b));
        if sabbat {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, end);
            continue;
        }
        pick_when(&mut e, end);
        // The end of the round still comes: Taste of Vitae is played.
        pick_when(&mut e, play(0, 0, a));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.seats[0]
            .ash
            .contains(&find_library("Taste of Vitae")));
        assert_eq!((blood(&e, a), blood(&e, b)), (4, 4), "no strike");
        assert!(e.state.cards[el.index()].locked);
    }
}

/// Instability: with no Edge, the prey may take it, the player gains 2; a
/// second copy is not played that turn; not played while the player holds
/// the Edge, played (nothing to take) while the prey does.
#[test]
fn instability_offers_the_free_edge_to_the_prey_and_gains_two() {
    let mut e = game(3);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Instability", "Instability"]);
    e.at_master_phase();
    e.state.seats[0].mpa = 2;
    pick(&mut e, master_play(0));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(1));
    e.choose(index_of(&req, Choice::ChooseSeat(SeatId(1))));
    let req = request(&mut e);
    assert!(e.state.seats[1].has_edge);
    assert_eq!(e.state.seats[0].pool, 12);
    assert!(!req.options.contains(&master_play(0)), "one each turn");
    for (holder, offered) in [(0, false), (2, false), (1, true)] {
        let mut e = game(3);
        e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        e.state.seats[holder].has_edge = true;
        hand(&mut e, 0, &["Instability"]);
        e.at_master_phase();
        assert_eq!(
            request(&mut e).options.contains(&master_play(0)),
            offered,
            "the Edge at seat {holder}"
        );
        if offered {
            pick(&mut e, master_play(0));
            request(&mut e);
            assert!(e.state.seats[1].has_edge);
            assert_eq!(e.state.seats[0].pool, 12);
        }
    }
}

/// King's Rising: 5 pool or less gains 3, more gains 1, the card in play;
/// no transfer moves counters to or from the uncontrolled region (the
/// crypt draw stays); with the Edge in its controller's unlock phase it
/// burns.
#[test]
fn kings_rising_gains_by_pool_bars_transfer_moves_and_burns_with_the_edge() {
    for (before, after) in [(5, 8), (6, 7)] {
        let mut e = game(2);
        e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        e.state.seats[0].pool = before;
        e.state.seats[0].uncontrolled.push(UncontrolledSlot {
            card: camarilla("Ventrue", 1),
            copy: Default::default(),
            counters: 2,
            banished: None,
        });
        e.state.seats[0].crypt.push(camarilla("Ventrue", 2));
        hand(&mut e, 0, &["King's Rising"]);
        e.at_master_phase();
        pick(&mut e, master_play(0));
        request(&mut e);
        assert_eq!(e.state.seats[0].pool, after);
        let kr = card_in_play(&e, "King's Rising");
        let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        assert!(req.options.contains(&Choice::DrawCrypt));
        assert!(!req
            .options
            .iter()
            .any(|o| matches!(o, Choice::Transfer { .. } | Choice::Withdraw { .. })));
        // Seat 1's unlock phase with seat 0's Edge: kept.
        e.state.seats[0].has_edge = true;
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(e.state.cards[kr.index()].present());
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.cards[kr.index()].present(), "burned with the Edge");
    }
}

/// Nocturn Theater: in its controller's unlock phase, the card and a ready
/// vampire of theirs locked lock the prey's minion; a titled vampire may
/// burn it as an action, an untitled one not. Backs `[LSJ 20080604]`.
#[test]
fn nocturn_theater_locks_a_prey_minion_for_its_lock_and_a_vampires() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let t = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let titled = e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    e.state.minions[titled.index()].granted_title = Some(Title::Prince("Atlanta"));
    let nt = in_play(&mut e, 0, "Nocturn Theater", None);
    let lock = use_at(nt, 0, Some(v), Target::Minion(t));
    // Not in another Methuselah's unlock phase.
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, lock);
    let burn = |m| use_at(nt, 1, Some(m), Target::None);
    let req = request(&mut e);
    assert!(req.options.contains(&burn(titled)));
    assert!(!req.options.contains(&burn(t)), "untitled");
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick(&mut e, lock);
    request(&mut e);
    assert!(e.state.cards[nt.index()].locked);
    assert!(e.state.minions[v.index()].locked);
    assert!(e.state.minions[t.index()].locked);
}

/// Powerbase: Chicago: one of its two unlock abilities each unlock phase
/// (rules-qa 6.6); another Methuselah's vampire takes its blood to its
/// controller's pool as a directed action, the card staying.
#[test]
fn powerbase_chicago_banks_blood_and_another_vampire_takes_it() {
    let mut e = game(2);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    let pc = in_play(&mut e, 0, "Powerbase: Chicago", None);
    e.state.cards[pc.index()].counters = 1;
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    pick(&mut e, ability(pc.0, 0));
    let req = request(&mut e);
    assert!(!req.options.contains(&ability(pc.0, 1)), "one of the two");
    assert_eq!(e.state.cards[pc.index()].counters, 2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.notes = Some(Vec::new());
    pick(&mut e, use_at(pc, 2, Some(z), Target::None));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 12);
    assert_eq!(e.state.cards[pc.index()].counters, 0);
    assert!(e.state.cards[pc.index()].present());
    let notes = noted_actions(&mut e);
    let last = notes.last().expect("noted");
    assert_eq!(last.action.directed_seat(&e.state), Some(SeatId(0)));
}

/// Specialization: in its controller's unlock phase, locked with two copies
/// of one card discarded for 1 pool; a single copy is not named.
#[test]
fn specialization_trades_a_pair_for_a_pool() {
    let mut e = game(2);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let sp = in_play(&mut e, 0, "Specialization", None);
    until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    e.state.seats[0].hand.clear();
    hand(
        &mut e,
        0,
        &[
            "Deflection",
            "Deflection",
            "Form of Mist",
            "Form of Mist",
            "Bonding",
        ],
    );
    pick(&mut e, ability(sp.0, 0));
    let req = request(&mut e);
    assert_eq!(req.options, vec![Choice::Discard(0), Choice::Discard(2)]);
    e.choose(0);
    request(&mut e);
    let deflection = find_library("Deflection");
    let s = &e.state.seats[0];
    assert_eq!(s.pool, 11);
    assert_eq!(s.ash.iter().filter(|&&c| c == deflection).count(), 2);
    assert!(!s.hand.contains(&deflection));
    assert!(e.state.cards[sp.index()].locked);
}

/// Storm Sewers: locked as its controller's minion announces a bleed, the
/// block's combat is at close range with no determine range step. Backs
/// `[PIB 20120214]` (key) and `[RTR 19970630] [ANK 20180720]`.
#[test]
fn storm_sewers_closes_every_round_of_the_block() {
    for locked in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        // Flash [cel], a maneuver its blocker would play at the determine
        // range step.
        let b = e.add_minion(SeatId(1), find_card("Elisha Tucker"), 4, false);
        e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
        hand(&mut e, 1, &["Flash"]);
        let ss = in_play(&mut e, 0, "Storm Sewers", None);
        e.notes = Some(Vec::new());
        pick(&mut e, bleed(a));
        if locked {
            pick(&mut e, ability(ss.0, 0));
        }
        pick_when(&mut e, block(b));
        let mut range_step = false;
        loop {
            let req = request(&mut e);
            range_step |= req.window == WindowLabel::CombatDetermineRange;
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            e.choose(fallback(&req));
        }
        assert_eq!(range_step, !locked);
        if locked {
            let strikes: Vec<_> = noted_combats(&mut e)
                .into_iter()
                .filter_map(|n| match n {
                    zvtes::combat::CombatNote::Strikes { range, .. } => Some(range),
                    _ => None,
                })
                .collect();
            assert!(!strikes.is_empty());
            assert!(strikes.iter().all(|r| *r == zvtes::combat::Range::Close));
            assert!(e.state.cards[ss.index()].locked);
        }
    }
}

/// XTC-Laced Blood: played by another Methuselah after a vampire's hunt
/// gains blood, the card on the hunter; in its next minion phase it must
/// hunt (no Pass until it does). Not after a diseased hunter's hunt, which
/// gains nothing (`[LSJ 20050720-2]`), nor on the player's own turn
/// (rulebook "Out-of-turn").
#[test]
fn xtc_laced_blood_needs_a_hunt_that_gained_blood() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), camarilla("Toreador", 0), 2, false);
    let sick = e.add_minion(SeatId(0), camarilla("Toreador", 1), 2, false);
    e.state.minions[sick.index()].disease = true;
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["XTC-Laced Blood"]);
    hand(&mut e, 1, &["XTC-Laced Blood"]);
    let xtc = |m| master_at(0, Target::Minion(m));
    pick(&mut e, action(sick, ActionKind::Hunt));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, xtc(sick));
    pick(&mut e, action(h, ActionKind::Hunt));
    let req = offering(&mut e, xtc(h));
    assert_eq!(req.seat, SeatId(1), "not its player's own turn");
    e.choose(index_of(&req, xtc(h)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(placed_on(&e, "XTC-Laced Blood", Some(h)));
    assert!(req.options.contains(&Choice::Pass), "it hunted this turn");
    e.choose(index_of(&req, Choice::Pass));
    // Its next minion phase: the hunt is all it may do (elided), taken.
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[h.index()].hunted_turn, Some(e.state.turn));
    assert!(e.state.minions[h.index()].locked);
    assert!(req.options.contains(&Choice::Pass));
    assert!(e.state.minions[sick.index()].hunted_turn.is_none());
}

/// Special Report: another Methuselah's out-of-turn master unlocks their
/// locked vampire, which attempts to block; their 1 pool buys +1 intercept
/// once. Backs `[LSJ 20090514]`, `[ANK 20181122-2]`, `[LSJ 20010714-2]`,
/// `[ANK 20180623]` with the forced-block group tests.
#[test]
fn special_report_forces_a_block_and_buys_one_intercept_with_pool() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Stealth(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    let r = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[r.index()].locked = true;
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    hand(&mut e, 1, &["Special Report"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, master_at(0, Target::Minion(r)));
    request(&mut e);
    assert!(!e.state.minions[r.index()].locked, "unlocked");
    pick_when(&mut e, Choice::BurnPool);
    assert_eq!(e.state.seats[1].pool, 9);
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::BurnPool), "once");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9, "blocked: nothing bled");
}

/// Contingency Planning: as its player's minion bleeds, a Deflection played
/// against it is canceled, no cost paid (its blood not burned), the same
/// minion not offered it again; the bleed of 3 burns 1. Backs `[ANK
/// 20190104] [LSJ 19980212]` and `[LSJ 20061207] [LSJ 20061013]` (played at
/// once, nothing replaced in the window).
#[test]
fn contingency_planning_cancels_a_deflection_and_caps_the_bleed() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Bleed(2),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    let d = e.add_minion(SeatId(1), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(2), camarilla("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Contingency Planning"]);
    hand(&mut e, 1, &["Deflection", "Deflection"]);
    pick(&mut e, bleed(a));
    let deflect = play_on(0, 0, d, Target::Seat(SeatId(2)));
    pick_when(&mut e, deflect);
    let req = expect(&mut e, SeatId(0), WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, master_play(0)));
    let req = request(&mut e);
    assert!(!req.options.contains(&deflect), "the same reaction, once");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, d), 4, "no cost paid");
    assert_eq!(e.state.seats[1].pool, 9, "bled for 3, burns 1");
    assert_eq!(e.state.seats[2].pool, 10);
    assert_eq!(e.state.seats[0].pool, 9);
}

/// Zoning Board: in its controller's master phase, another Methuselah's
/// location chosen; that turn a minion moves it to its owner's library as
/// an action directed at its controller, who gains its pool cost; the same
/// minion may then call the burn referendum (`[LSJ 20100909]`), its
/// controller locking the card for a vote.
#[test]
fn zoning_board_grants_a_move_and_a_burn_both_by_one_minion() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let zb = in_play(&mut e, 0, "Zoning Board", None);
    let pc = in_play(&mut e, 1, "Powerbase: Chicago", None);
    fill_hand(&mut e, 1);
    let mark = use_at(zb, 0, None, Target::Card(pc));
    let mv = use_at(zb, 1, Some(a), Target::Card(pc));
    assert!(!request(&mut e).options.contains(&mv), "none chosen");
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, mark);
    to_minion_phase(&mut e, SeatId(0));
    e.notes = Some(Vec::new());
    pick(&mut e, mv);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let chicago = find_library("Powerbase: Chicago");
    assert!(!e.state.cards[pc.index()].in_play);
    assert!(e.state.seats[1].library.contains(&chicago));
    assert_eq!(e.state.seats[1].pool, 11, "its 1 pool cost");
    let notes = noted_actions(&mut e);
    let last = notes.last().expect("noted");
    assert_eq!(last.action.directed_seat(&e.state), Some(SeatId(1)));
    // The same minion, unlocked, calls the burn referendum.
    e.state.minions[a.index()].locked = false;
    pick(&mut e, use_at(zb, 2, Some(a), Target::None));
    pick_when(&mut e, ability(zb.0, 3));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.cards[zb.index()].present(),
        "its own vote passed it"
    );
}

/// High Museum of Art, Atlanta: played, +4; stolen by another Methuselah's
/// vampire, the loser burns 4 and the thief's controller gains 4; a second
/// copy played, +4, then both contested: each holder burns 4.
#[test]
fn high_museum_pays_four_and_costs_four_when_stolen_or_contested() {
    let mut e = game(2);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["High Museum of Art, Atlanta"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    assert_eq!(pool(&e), vec![14, 10]);
    let hm = card_in_play(&e, "High Museum of Art, Atlanta");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, granted(hm.0, z));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.card_controller(hm), SeatId(1));
    assert_eq!(pool(&e), vec![10, 14]);
    // Seat 0 plays another copy: contested with seat 1's.
    hand(&mut e, 0, &["High Museum of Art, Atlanta"]);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, master_play(0));
    request(&mut e);
    assert!(e.state.cards[hm.index()].contested);
    assert_eq!(pool(&e), vec![10 + 4 - 4, 14 - 4]);
}

/// High Museum of Art, Atlanta burned by Goblinism [pot]: its controller
/// burns 4.
#[test]
fn high_museum_costs_four_when_burned() {
    let mut e = game(2);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let p = e.add_minion(SeatId(1), vampire_with("pot", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    let hm = in_play(&mut e, 0, "High Museum of Art, Atlanta", None);
    hand(&mut e, 1, &["Goblinism"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, play_on(0, 0, p, Target::Card(hm)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[hm.index()].in_play);
    assert_eq!(e.state.seats[0].pool, 6);
}

/// Social Ladder: on its player's ready vampire; in their influence phase
/// (before they may pass) the vampire is removed from the game and its
/// blood goes to an older vampire of their uncontrolled region — none
/// older, the blood is lost (`[LSJ 20041015]`).
#[test]
fn social_ladder_removes_the_vampire_even_with_no_older_heir() {
    for older in [true, false] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 4, 0), 3, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        let heir = if older {
            of_cap(Sect::Camarilla, 7, 0)
        } else {
            of_cap(Sect::Camarilla, 3, 0)
        };
        e.state.seats[0].uncontrolled.push(UncontrolledSlot {
            card: heir,
            copy: Default::default(),
            counters: 1,
            banished: None,
        });
        hand(&mut e, 0, &["Social Ladder"]);
        e.at_master_phase();
        pick(&mut e, master_at(0, Target::Minion(v)));
        let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(placed_on(&e, "Social Ladder", Some(v)));
        e.choose(index_of(&req, Choice::Pass));
        let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        assert!(!req.options.contains(&Choice::Pass), "due first");
        let sl = card_in_play(&e, "Social Ladder");
        e.choose(index_of(&req, Choice::ResolveRider { card: sl.0 }));
        request(&mut e);
        assert!(!e.state.minions[v.index()].in_play);
        let s = &e.state.seats[0];
        assert!(s
            .removed_crypt
            .contains(&e.state.minions[v.index()].crypt_card().unwrap()));
        assert!(s.ash.contains(&find_library("Social Ladder")));
        assert_eq!(s.uncontrolled[0].counters, if older { 1 + 3 } else { 1 });
    }
}

/// White Nights Massacre: played in a discard phase, nothing due then; in
/// its player's next discard phase it burns, and they burn one torpid
/// vampire, or every one by discarding another copy (any Methuselah's).
#[test]
fn white_nights_massacre_burns_next_discard_phase_one_or_all_torpid() {
    for all in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        let t0 = e.add_minion(SeatId(0), camarilla("Toreador", 1), 0, true);
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        let t1 = e.add_minion(SeatId(1), camarilla("Ventrue", 1), 0, true);
        hand(&mut e, 0, &["White Nights Massacre"]);
        if all {
            hand(&mut e, 0, &["White Nights Massacre"]);
        }
        fill_hand(&mut e, 0);
        pick(&mut e, Choice::Pass);
        pick_when(&mut e, master_play(0));
        let wnm = find_library("White Nights Massacre");
        let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let id = card_in_play(&e, "White Nights Massacre");
        assert!(e.state.cards[id.index()].present(), "nothing due this turn");
        e.choose(index_of(&req, Choice::Pass));
        pick_when(&mut e, Choice::ResolveRider { card: id.0 });
        let req = request(&mut e);
        assert!(!e.state.cards[id.index()].present(), "burned");
        if all {
            let i = e.state.seats[0]
                .hand
                .iter()
                .position(|&c| c == wnm)
                .unwrap();
            e.choose(index_of(&req, Choice::Discard(i as u8)));
            request(&mut e);
            for t in [t0, t1] {
                assert!(!e.state.minions[t.index()].in_play, "{t:?}");
            }
        } else {
            assert!(!req.options.iter().any(|o| matches!(o, Choice::Discard(_))));
            e.choose(index_of(&req, Choice::ChooseMinion(t1)));
            request(&mut e);
            assert!(!e.state.minions[t1.index()].in_play);
            assert!(e.state.minions[t0.index()].in_play);
        }
    }
}

/// Bounty at X = 0 (`[LSJ 20030502]`): the card itself on another
/// Methuselah's vampire, 1 counter.
#[test]
fn bounty_at_x_zero_holds_one_counter() {
    let mut e = game(2);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bounty"]);
    e.at_master_phase();
    pick(&mut e, master_at(0, Target::Minion(v)));
    pick(&mut e, set_x(0));
    request(&mut e);
    assert!(placed_on(&e, "Bounty", Some(v)));
    let b = card_in_play(&e, "Bounty");
    assert_eq!(e.state.cards[b.index()].counters, 1);
    assert_eq!(e.state.seats[0].pool, 10);
}

/// Bounty: its vampire sent to torpor in combat, the opposing minion's
/// controller takes up to 2 counters (2 of 4 here), the card staying; then
/// diablerized, the diablerist's controller takes the rest, the card burned
/// with its vampire.
#[test]
fn bounty_pays_the_combatant_who_sends_it_to_torpor_and_the_diablerist() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 0, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    let bounty = in_play(&mut e, 1, "Bounty", Some(v));
    e.state.cards[bounty.index()].counters = 4;
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(v));
    let req = offering(&mut e, set_x(2));
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(req.options, vec![set_x(0), set_x(1), set_x(2)]);
    e.choose(index_of(&req, set_x(2)));
    request(&mut e);
    assert!(e.state.minions[v.index()].in_torpor);
    assert_eq!(e.state.seats[0].pool, 12);
    assert_eq!(e.state.cards[bounty.index()].counters, 2);
    assert!(e.state.cards[bounty.index()].present());
    // Next turn: diablerized by seat 0's other vampire.
    let d = e.add_minion(SeatId(0), camarilla("Toreador", 2), 4, false);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let before = e.state.seats[0].pool;
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(v),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[v.index()].in_play);
    assert_eq!(e.state.seats[0].pool, before + 2);
    assert!(e.state.seats[1].ash.contains(&find_library("Bounty")));
}

/// Malkavian Prank: each other Methuselah holds 1 to 4 in secret, then the
/// player guesses each: right, the player takes that much of their pool;
/// wrong, they gain the number they held from the bank.
#[test]
fn malkavian_prank_steals_on_a_right_guess_and_pays_from_the_bank_on_a_wrong_one() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Malkavian", 0), 4, false);
    hand(&mut e, 0, &["Malkavian Prank"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    for (seat, n) in [(1, 3), (2, 2)] {
        let req = request(&mut e);
        assert_eq!(req.seat, SeatId(seat));
        assert_eq!(req.options, (1..=4).map(set_x).collect::<Vec<_>>());
        e.choose(index_of(&req, set_x(n)));
    }
    for (of, guess) in [(1u8, 3), (2, 4)] {
        let req = request(&mut e);
        assert_eq!(req.seat, SeatId(0), "the guess at seat {of}");
        e.choose(index_of(&req, set_x(guess)));
    }
    request(&mut e);
    assert_eq!(pool(&e), vec![13, 7, 12]);
}

/// Hostile Takeover: an open auction from its player (`[LSJ 19990104]`):
/// seat 2 bids 3, the others pass; seat 2 pays seat 1, the vampire's
/// controller, and takes the vampire. Not playable on a capacity-7 one.
#[test]
fn hostile_takeover_auctions_control_and_pays_the_controller() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let v = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 5, 0), 4, false);
    let big = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 0), 4, false);
    e.add_minion(SeatId(2), camarilla("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Hostile Takeover"]);
    e.at_master_phase();
    let req = request(&mut e);
    assert!(!req.options.contains(&master_at(0, Target::Minion(big))));
    e.choose(index_of(&req, master_at(0, Target::Minion(v))));
    for (seat, bid) in [(0, None), (1, None), (2, Some(3)), (0, None), (1, None)] {
        let req = request(&mut e);
        assert_eq!(req.seat, SeatId(seat));
        e.choose(index_of(&req, bid.map_or(Choice::Pass, set_x)));
    }
    request(&mut e);
    assert_eq!(e.state.minions[v.index()].owner, SeatId(2));
    assert_eq!(pool(&e), vec![9, 13, 7]);
}

/// Hostile Takeover: the controller may bid up to twice their pool
/// (`[LSJ 19990825]`), others their pool; the controller winning at 7 keeps
/// the vampire and burns 4.
#[test]
fn hostile_takeover_lets_the_controller_bid_twice_their_pool() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let v = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 5, 0), 4, false);
    e.add_minion(SeatId(2), camarilla("Toreador", 0), 4, false);
    e.state.seats[1].pool = 5;
    hand(&mut e, 0, &["Hostile Takeover"]);
    e.at_master_phase();
    pick(&mut e, master_at(0, Target::Minion(v)));
    let req = request(&mut e);
    assert_eq!(
        req.options.last(),
        Some(&set_x(9)),
        "seat 0: its pool after the cost"
    );
    e.choose(index_of(&req, Choice::Pass));
    let req = request(&mut e);
    assert_eq!(req.options.last(), Some(&set_x(10)), "twice 5");
    e.choose(index_of(&req, set_x(7)));
    for seat in [2, 0] {
        let req = request(&mut e);
        assert_eq!(req.seat, SeatId(seat));
        e.choose(index_of(&req, Choice::Pass));
    }
    request(&mut e);
    assert_eq!(e.state.minions[v.index()].owner, SeatId(1));
    assert_eq!(e.state.seats[1].pool, 1);
}

/// Contingency Planning cancels a Deflection played from The Erciyes
/// Fragments as if from hand (`[LSJ 20060106]`): the bleed stays at its
/// target, 1 pool burned.
#[test]
fn contingency_planning_cancels_a_bounce_played_from_the_erciyes_fragments() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Bleed(2),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    let d = e.add_minion(SeatId(1), vampire_with("dom", 0), 4, false);
    e.add_minion(SeatId(2), camarilla("Ventrue", 1), 4, false);
    let frag = in_play(&mut e, 1, "Erciyes Fragments, The", None);
    e.state.cards[frag.index()].stored = vec![find_library("Deflection")].into();
    e.state.cards[frag.index()].stored_owner = Some(SeatId(1));
    hand(&mut e, 0, &["Contingency Planning"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_on(0, 0, d, Target::Seat(SeatId(2))));
    let req = expect(&mut e, SeatId(0), WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, master_play(0)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[frag.index()].stored.is_empty());
    assert_eq!(pool(&e), vec![9, 9, 10]);
}

/// Al's Army Apparatus with no weapon in the library: still offered, the
/// search finds nothing (rules-digest §19.4: a search may find nothing):
/// the card locked, the hand and library unchanged in content.
#[test]
fn als_army_apparatus_searches_in_vain_with_no_weapon() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let al = in_play(&mut e, 0, "Al's Army Apparatus", None);
    fill_hand(&mut e, 0);
    for c in ["Form of Mist", "Deflection"] {
        e.state.seats[0].library.push(find_library(c));
    }
    let hand_before = e.state.seats[0].hand.len();
    pick(&mut e, ability(al.0, 0));
    request(&mut e);
    assert!(e.state.cards[al.index()].locked);
    assert_eq!(e.state.seats[0].hand.len(), hand_before);
    assert_eq!(e.state.seats[0].library.len(), 2);
}

/// Elysium: The Arboretum's burn referendum: another Methuselah's
/// Camarilla vampire calls it as a +1 stealth political action, undirected
/// (rulebook: political actions are undirected; rules-qa 113.70); passed,
/// the card burned to its owner's ash heap. Their Sabbat vampire is not
/// offered it.
#[test]
fn elysium_the_arboretum_is_burned_by_an_undirected_camarilla_referendum() {
    let mut e = game(2);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let u = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[u.index()].granted_title = Some(Title::Prince("Atlanta"));
    let s = e.add_minion(SeatId(1), of_cap(Sect::Sabbat, 4, 0), 4, false);
    let el = in_play(&mut e, 0, "Elysium: The Arboretum", None);
    let call = |m| use_at(el, 1, Some(m), Target::None);
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&call(s)), "Sabbat");
    e.choose(index_of(&req, call(u)));
    request(&mut e);
    let act = e.action().expect("political");
    assert_eq!(act.stealth(&e.state), 1);
    assert_eq!(act.directed_seat(&e.state), None);
    to_referendum(&mut e);
    pick_when(&mut e, votes(u, true));
    close_polling(&mut e);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[el.index()].present());
    assert!(ash_has(&e, 0, "Elysium: The Arboretum"));
}

/// High Museum of Art, Atlanta moved to its owner's library by Zoning
/// Board's action: its controller burns 4 (losing control) and gains X = 0
/// (no pool cost printed).
#[test]
fn zoning_board_moves_a_high_museum_for_no_pool_and_it_costs_four() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let zb = in_play(&mut e, 0, "Zoning Board", None);
    let hm = in_play(&mut e, 1, "High Museum of Art, Atlanta", None);
    fill_hand(&mut e, 1);
    pick(&mut e, Choice::Pass);
    pick_when(&mut e, use_at(zb, 0, None, Target::Card(hm)));
    to_minion_phase(&mut e, SeatId(0));
    let before = e.state.seats[1].pool;
    pick(&mut e, use_at(zb, 1, Some(a), Target::Card(hm)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[hm.index()].in_play);
    assert!(e.state.seats[1]
        .library
        .contains(&find_library("High Museum of Art, Atlanta")));
    assert_eq!(e.state.seats[1].pool, before - 4);
}

/// Rules-qa red (round 29, master_event R3): the High Museum contest won
/// back — seat 1 yields its copy at its unlock phase; seat 0's copy, face
/// up again at its unlock phase, is control gained: +4.
#[test]
fn high_museum_pays_four_again_when_its_contest_is_won() {
    let mut e = game(2);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    let theirs = in_play(&mut e, 1, "High Museum of Art, Atlanta", None);
    hand(&mut e, 0, &["High Museum of Art, Atlanta"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    assert!(e.state.cards[theirs.index()].contested);
    assert_eq!(pool(&e), vec![10 + 4 - 4, 10 - 4]);
    let ours = CardId(e.state.cards.len() as u32 - 1);
    assert_ne!(ours, theirs);
    pick_when(
        &mut e,
        Choice::YieldContest(zvtes::game::Contest::Library(theirs)),
    );
    assert_eq!(e.state.seats[1].pool, 6, "nothing more");
    let before = e.state.seats[0].pool;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[ours.index()].contested);
    assert_eq!(e.state.seats[0].pool, before + 4);
}

/// Hostile Takeover: a torpid vampire and the player's own are chosen too
/// (rules-digest D11, D6). The Diamond Thunderbolt stops the change: the
/// vampire stays, gains 1 blood, and the winning bid is paid all the same
/// (LSJ 20040518: "Any bids made are paid").
#[test]
fn hostile_takeover_reaches_torpor_and_own_vampires_and_its_bids_stay_paid() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let own = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 5, 0), 4, false);
    let v = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 5, 1), 4, false);
    let torpid = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 0, true);
    e.add_minion(SeatId(2), camarilla("Toreador", 0), 4, false);
    hand(&mut e, 0, &["Hostile Takeover"]);
    hand(&mut e, 1, &["Diamond Thunderbolt, The"]);
    e.at_master_phase();
    let req = request(&mut e);
    for m in [own, torpid, v] {
        assert!(
            req.options.contains(&master_at(0, Target::Minion(m))),
            "{m:?}"
        );
    }
    e.choose(index_of(&req, master_at(0, Target::Minion(v))));
    for (seat, bid) in [(0, None), (1, None), (2, Some(3)), (0, None), (1, None)] {
        let req = request(&mut e);
        assert_eq!(req.seat, SeatId(seat));
        e.choose(index_of(&req, bid.map_or(Choice::Pass, set_x)));
    }
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(1));
    let dt = *plays(&req).first().expect("The Diamond Thunderbolt");
    e.choose(index_of(&req, dt));
    request(&mut e);
    assert_eq!(e.state.minions[v.index()].owner, SeatId(1));
    assert_eq!(blood(&e, v), 5);
    assert_eq!(pool(&e), vec![9, 13, 7]);
}

/// Malkavian Prank, a right guess of 4 against a Methuselah with 2 pool:
/// they lose all of it and are ousted, the player gaining the 2 (an
/// effect goes to 0, rules-digest D3).
#[test]
fn malkavian_prank_right_guess_takes_all_and_ousts() {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain("Malkavian", 0), 4, false);
    e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    e.state.seats[1].pool = 2;
    hand(&mut e, 0, &["Malkavian Prank"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    for (seat, n) in [(1, 4), (2, 1)] {
        let req = request(&mut e);
        assert_eq!(req.seat, SeatId(seat));
        e.choose(index_of(&req, set_x(n)));
    }
    for guess in [4, 2] {
        let req = request(&mut e);
        assert_eq!(req.seat, SeatId(0));
        e.choose(index_of(&req, set_x(guess)));
    }
    request(&mut e);
    assert!(e.state.seats[1].ousted);
    assert_eq!(e.state.seats[1].pool, 0);
    // 2 from seat 1, 6 for the oust (its predator).
    assert_eq!(e.state.seats[0].pool, 10 + 2 + 6);
}

/// Social Ladder's vampire in torpor by its influence phase: still removed
/// from the game, its blood to the older uncontrolled vampire (D13: a card
/// in play stays when its requirement lapses).
#[test]
fn social_ladder_removes_a_vampire_gone_to_torpor() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 4, 0), 3, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.seats[0].uncontrolled.push(UncontrolledSlot {
        card: of_cap(Sect::Camarilla, 7, 0),
        copy: Default::default(),
        counters: 1,
        banished: None,
    });
    hand(&mut e, 0, &["Social Ladder"]);
    e.at_master_phase();
    pick(&mut e, master_at(0, Target::Minion(v)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[v.index()].in_torpor = true;
    e.choose(index_of(&req, Choice::Pass));
    let req = until(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let sl = card_in_play(&e, "Social Ladder");
    e.choose(index_of(&req, Choice::ResolveRider { card: sl.0 }));
    request(&mut e);
    assert!(!e.state.minions[v.index()].in_play);
    let s = &e.state.seats[0];
    assert!(s
        .removed_crypt
        .contains(&e.state.minions[v.index()].crypt_card().unwrap()));
    assert_eq!(s.uncontrolled[0].counters, 1 + 3);
}

/// Special Report is out-of-turn: as its player's own minion bleeds, their
/// locked vampire is never offered it (rulebook "Out-of-turn").
#[test]
fn special_report_is_not_played_on_its_players_own_turn() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let r = e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.state.minions[r.index()].locked = true;
    e.add_minion(SeatId(0), camarilla("Toreador", 2), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Special Report"]);
    pick(&mut e, bleed(a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        master_at(0, Target::Minion(r)),
    );
    assert!(e.state.minions[r.index()].locked);
}

/// Storm Sewers closes the block's combat only: a second combat Psyche!
/// queues in the same action has its determine range step, its blocker's
/// Flash offered there.
#[test]
fn storm_sewers_leaves_a_second_combat_of_the_action_open() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    superior(&mut e, a, "cel");
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let b = e.add_minion(SeatId(1), find_card("Elisha Tucker"), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Psyche!"]);
    hand(&mut e, 1, &["Flash"]);
    let ss = in_play(&mut e, 0, "Storm Sewers", None);
    pick(&mut e, bleed(a));
    pick(&mut e, ability(ss.0, 0));
    pick_when(&mut e, block(b));
    let mut queued = false;
    let mut range_steps = (0, 0);
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        if req.window == WindowLabel::CombatDetermineRange {
            if queued {
                range_steps.1 += 1;
            } else {
                range_steps.0 += 1;
            }
        }
        if let Some(i) = req.options.iter().position(|o| *o == play(0, 1, a)) {
            queued = true;
            e.choose(i);
            continue;
        }
        e.choose(fallback(&req));
    }
    assert!(queued, "Psyche! played");
    assert_eq!(range_steps.0, 0, "the block's combat closed");
    assert!(range_steps.1 > 0, "the second combat's range step");
}

/// White Nights Massacre, "all" by locking a ready werewolf ally its
/// player controls (Renegade Garou): the ally locked, every torpid vampire
/// burned; a locked werewolf is not offered.
#[test]
fn white_nights_massacre_burns_all_by_locking_a_werewolf() {
    for locked in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        let t0 = e.add_minion(SeatId(0), camarilla("Toreador", 1), 0, true);
        let w = e.add_ally(SeatId(0), find_library("Renegade Garou"));
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        let t1 = e.add_minion(SeatId(1), camarilla("Ventrue", 1), 0, true);
        let id = in_play(&mut e, 0, "White Nights Massacre", None);
        fill_hand(&mut e, 0);
        let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        e.state.minions[w.index()].locked = locked;
        e.choose(index_of(&req, Choice::Pass));
        pick_when(&mut e, Choice::ResolveRider { card: id.0 });
        let req = request(&mut e);
        if locked {
            assert!(!req.options.contains(&Choice::ChooseMinion(w)));
            continue;
        }
        e.choose(index_of(&req, Choice::ChooseMinion(w)));
        request(&mut e);
        assert!(e.state.minions[w.index()].locked);
        for t in [t0, t1] {
            assert!(!e.state.minions[t.index()].in_play, "{t:?}");
        }
    }
}

/// Bounty on its player's own vampire (any ready vampire, rules-digest
/// D11): the card itself on it.
#[test]
fn bounty_goes_on_its_players_own_vampire() {
    let mut e = game(2);
    let own = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Bounty"]);
    e.at_master_phase();
    pick(&mut e, master_at(0, Target::Minion(own)));
    pick(&mut e, set_x(1));
    request(&mut e);
    assert!(placed_on(&e, "Bounty", Some(own)));
    assert_eq!(e.state.seats[0].pool, 9);
}

/// Bounty's vampire burned in combat (an aggravated hand strike of 2 on it at 0
/// blood: torpor, then burned): the opposing minion's controller takes every
/// counter, the card burned with it; emptied by the torpor payout (2 of 2), the
/// card burns.
#[test]
fn bounty_pays_all_for_a_burn_in_combat_and_burns_when_emptied() {
    for burned in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 0, false);
        e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
        let bounty = in_play(&mut e, 1, "Bounty", Some(v));
        e.state.cards[bounty.index()].counters = if burned { 3 } else { 2 };
        if burned {
            for l in [
                zvtes::cards::Lasting::AggravatedHands,
                zvtes::cards::Lasting::Strength(1),
            ] {
                e.state.minions[a.index()].timed.push((
                    l,
                    zvtes::state::Expiry::EndOfTurn,
                    SeatId(0),
                ));
            }
        }
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(v));
        if !burned {
            pick_when(&mut e, set_x(2));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let vamp = &e.state.minions[v.index()];
        assert_eq!(!vamp.in_play, burned, "burned {burned}");
        assert_eq!(e.state.seats[0].pool, 10 + if burned { 3 } else { 2 });
        assert!(!e.state.cards[bounty.index()].present());
        assert!(ash_has(&e, 1, "Bounty"));
    }
}

/// Contingency Planning played on its player's own turn is still an
/// out-of-turn master: it spends their next master phase action (advanced
/// rules §6.6.2), Information Highway not offered then; not played, it is.
#[test]
fn contingency_planning_on_its_own_turn_spends_the_next_master_phase_action() {
    for played in [true, false] {
        let mut e = game(3);
        let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        let d = e.add_minion(SeatId(1), vampire_with("dom", 0), 4, false);
        e.add_minion(SeatId(2), camarilla("Ventrue", 1), 4, false);
        hand(&mut e, 0, &["Contingency Planning", "Information Highway"]);
        hand(&mut e, 1, &["Deflection"]);
        pick(&mut e, bleed(a));
        let deflect = play_on(0, 0, d, Target::Seat(SeatId(2)));
        pick_when(&mut e, deflect);
        let req = expect(&mut e, SeatId(0), WindowLabel::CardAsPlayed);
        if played {
            e.choose(index_of(&req, master_play(0)));
        } else {
            e.choose(index_of(&req, Choice::Pass));
        }
        let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[0].out_of_turn.is_some(), played);
        e.choose(index_of(&req, Choice::Pass));
        let i = e.state.seats[0]
            .hand
            .iter()
            .position(|&c| c == find_library("Information Highway"))
            .unwrap() as u8;
        let offered = offered_before(&mut e, master_play(i), SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(offered, !played, "played {played}");
    }
}

// ---- Keepers of Tradition vampires, Aleister Crowley to Dr. John Dee -------

/// `m`'s printed ability `ai`, by `actor` at `target`.
fn vamp(m: MinionId, ai: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Vampire(m),
        ability: ai,
        actor,
        target,
    }
}

/// The round's printed traits (rules-qa 116.94): Allanyan Serata, Gracetius
/// ("+1 strength"), Beth Malcolm, Bloody Mary, Gustav Breidenstein, Jann Berger
/// ("+1 bleed"), Carlak's optional press, Viktor and Lord Fianna ("+1
/// stealth"); the titles generated (primogen, princes, Montecalme's justicar)
/// and Lord Fianna's Red List; the text-only vampires ("Camarilla.",
/// "Independent.") have no definition, their cards of their sect, untitled.
#[test]
fn keepers_vampires_carry_their_printed_stats_titles_and_sects() {
    let mut e = game(2);
    let stat = |e: &Engine, m: MinionId, s: Stat| e.state.printed_mod(m, s, Default::default());
    let add = |e: &mut Engine, name: &str| e.add_minion(SeatId(0), find_card(name), 4, false);
    for (name, s) in [
        ("Allanyan Serata", Stat::Strength),
        ("Gracetius", Stat::Strength),
        ("Beth Malcolm", Stat::Bleed),
        ("Bloody Mary", Stat::Bleed),
        ("Gustav Breidenstein", Stat::Bleed),
        ("Jann Berger", Stat::Bleed),
        ("Carlak", Stat::CombatPress),
        ("Viktor, The Night General", Stat::Stealth),
        ("Lord Fianna", Stat::Stealth),
    ] {
        let m = add(&mut e, name);
        assert_eq!(stat(&e, m, s), 1, "{name}");
    }
    for (name, title) in [
        ("Allanyan Serata", Title::Primogen),
        ("Bloody Mary", Title::Primogen),
        ("Gwyedd", Title::Primogen),
        ("Philippe de Marseilles", Title::Primogen),
        ("Carlak", Title::Prince("Prague")),
        ("Dr. John Dee", Title::Prince("London")),
        ("Baron Dieudonne", Title::Prince("Brussels")),
        ("Gustav Breidenstein", Title::Prince("Berlin")),
        ("Jann Berger", Title::Prince("Monaco")),
        ("Johannes Castelein", Title::Prince("Amsterdam")),
        ("Montecalme", Title::Justicar("Toreador")),
    ] {
        assert_eq!(CRYPT[find_card(name) as usize].title, title, "{name}");
    }
    assert!(CRYPT[find_card("Lord Fianna") as usize].red_list);
    for (name, sect) in [
        ("Don Cerro", Sect::Camarilla),
        ("Ezra Hawthorne", Sect::Camarilla),
        ("Florentina Lengauer", Sect::Camarilla),
        ("Foureyes", Sect::Camarilla),
        ("Garret", Sect::Camarilla),
        ("Honest Abe", Sect::Camarilla),
        ("Jack Tredegar", Sect::Camarilla),
        ("Lindsay Yates", Sect::Camarilla),
        ("Lisé", Sect::Independent),
        ("Masdela", Sect::Camarilla),
        ("Paul Calderone", Sect::Camarilla),
        ("Sean Andrews", Sect::Camarilla),
    ] {
        let card = find_card(name);
        let c = &CRYPT[card as usize];
        assert!(zvtes::cards::vampire(card).is_none(), "{name}");
        assert_eq!((c.sect, c.title), (Some(sect), Title::None), "{name}");
    }
}

/// Aleister Crowley: "Frenzy cards cannot be played on Aleister." Terror
/// Frenzy on the opposing minion: never offered against him.
#[test]
fn aleister_crowley_takes_no_frenzy_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Aleister Crowley"), 5, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("ani", 0), 6, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Terror Frenzy"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
    assert_eq!(e.state.seats[1].hand, vec![find_library("Terror Frenzy")]);
}

/// Alex Wilkins: "If Alex successfully bleeds, the target Methuselah can
/// burn 1 additional pool to steal him after resolution." The bled
/// Methuselah, after resolution: their pool down by the bleed and 1 more,
/// Alex theirs; blocked, nothing offered.
#[test]
fn alex_wilkins_is_stolen_by_the_methuselah_he_bleeds() {
    let mut e = game(2);
    let alex = e.add_minion(SeatId(0), find_card("Alex Wilkins"), 1, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    pick(&mut e, bleed(alex));
    let steal = vamp(alex, 0, None, Target::None);
    let req = offering(&mut e, steal);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(1), WindowLabel::ActionAfterResolution)
    );
    e.choose(index_of(&req, steal));
    request(&mut e);
    assert_eq!(e.state.minions[alex.index()].owner, SeatId(1));
    assert_eq!(e.state.seats[1].pool, 8);
    // Blocked: no successful bleed.
    let mut e = game(2);
    let alex = e.add_minion(SeatId(0), find_card("Alex Wilkins"), 1, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    pick(&mut e, bleed(alex));
    pick_when(&mut e, block(z));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, steal);
    assert_eq!(e.state.minions[alex.index()].owner, SeatId(0));
}

/// The Ankou: a vampire hunting successfully in its Methuselah's turn,
/// the Ankou ready and unlocked: as that Methuselah ends their minion
/// phase, the Ankou burns it as a Ⓓ action, and the phase goes on.
/// Rules-qa red (round 29): the moment is Enkil Cog's (rules-qa 49.6).
#[test]
fn the_ankou_burns_a_hunter_out_of_turn() {
    let mut e = game(2);
    let ankou = e.add_minion(SeatId(0), find_card("Ankou, The"), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let h_card = camarilla("Ventrue", 0);
    let h = e.add_minion(SeatId(1), h_card, 1, false);
    let other = e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let turn = e.state.turn;
    pick(&mut e, action(h, ActionKind::Hunt));
    // Not before they end their phase.
    expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, Choice::Pass);
    let burn = vamp(ankou, 0, Some(ankou), Target::Minion(h));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::PhaseImpulse)
    );
    assert_eq!(blood(&e, h), 2, "hunted");
    e.choose(index_of(&req, burn));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.turn, turn, "its phase goes on");
    assert!(req.options.contains(&action(other, ActionKind::Hunt)));
    assert!(!e.state.minions[h.index()].in_play);
    assert!(e.state.seats[1].ash_crypt.contains(&h_card));
    assert!(e.state.minions[ankou.index()].locked);
}

/// The Ankou locked as the hunt succeeds: no burn that turn, unlocked
/// afterwards neither; nor in his own next turn ("that turn").
#[test]
fn the_ankou_needs_to_be_ready_and_unlocked_as_the_hunt_succeeds() {
    let mut e = game(2);
    let ankou = e.add_minion(SeatId(0), find_card("Ankou, The"), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let h = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 1, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[ankou.index()].locked = true;
    pick(&mut e, action(h, ActionKind::Hunt));
    expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[ankou.index()].locked = false;
    let burn = vamp(ankou, 0, Some(ankou), Target::Minion(h));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, burn);
    assert_eq!(blood(&e, h), 2, "hunted");
    let req = request(&mut e);
    assert!(!req.options.contains(&burn));
}

/// Baron Dieudonne: after his successful action in his minion phase, 1
/// blood to unlock, once in the phase. His name is no baron's title.
/// Backs `[PIB 20151116]`.
#[test]
fn baron_dieudonne_unlocks_after_a_successful_action_once() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Baron Dieudonne"), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    assert!(!zvtes::cards::TitleClass::Baron.matches(e.state.title_of(d)));
    pick(&mut e, bleed(d));
    let unlock = vamp(d, 0, None, Target::None);
    let req = offering(&mut e, unlock);
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    e.choose(index_of(&req, unlock));
    request(&mut e);
    assert!(!e.state.minions[d.index()].locked);
    assert_eq!(blood(&e, d), 3);
    pick(&mut e, action(d, ActionKind::Hunt));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, unlock);
    assert!(e.state.minions[d.index()].locked);
}

/// Baron Dieudonne after a fizzled action: a rescue whose vampire left
/// torpor by resolution still lets him unlock. Backs `[ANK 20220218]`
/// (group "Requires successful action").
#[test]
fn baron_dieudonne_unlocks_after_a_fizzled_action() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), find_card("Baron Dieudonne"), 4, false);
    let t = e.add_minion(SeatId(0), camarilla("Toreador", 0), 1, true);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Rescue,
            target: Some(t),
        },
    );
    e.choose(0);
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.state.minions[t.index()].in_torpor = false;
    pick_when(&mut e, vamp(d, 0, None, Target::None));
    request(&mut e);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert!(!e.state.minions[d.index()].locked);
}

/// Bernard, the Scourge: his +1 stealth political action calls a blood
/// hunt on a vampire of capacity 4 or less, any Methuselah's; Delaying
/// Tactics cancels it in its polling (Bernard unlocked, no burn); passed,
/// the chosen vampire is burned and Voter Captivation follows. Backs
/// `[LSJ 20081202-2]` (Bernard's key row and its two "mention in" rows:
/// Delaying Tactics, Voter Captivation).
#[test]
fn bernard_calls_a_blood_hunt_as_a_political_action() {
    let setup = || {
        let mut e = game(3);
        let b = e.add_minion(SeatId(0), find_card("Bernard, the Scourge"), 4, false);
        let p = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 4, false);
        e.state.minions[p.index()].granted_title = Some(Title::Prince("Atlanta"));
        let small_card = of_cap(Sect::Camarilla, 4, 0);
        let small = e.add_minion(SeatId(1), small_card, 4, false);
        let big = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 6, 0), 4, false);
        let r = e.add_minion(SeatId(2), of_cap(Sect::Camarilla, 7, 1), 4, false);
        e.add_minion(SeatId(2), of_cap(Sect::Camarilla, 4, 1), 4, true);
        (e, b, p, small, small_card, big, r)
    };
    let (mut e, b, _, small, _, big, r) = setup();
    hand(&mut e, 2, &["Delaying Tactics"]);
    let hunt = vamp(b, 0, Some(b), Target::None);
    pick(&mut e, hunt);
    assert_eq!(e.action().expect("action").base_stealth, 1);
    let req = offering(&mut e, Choice::ChooseMinion(small));
    assert!(!req.options.contains(&Choice::ChooseMinion(big)));
    e.choose(index_of(&req, Choice::ChooseMinion(small)));
    pick_when(&mut e, play(0, 0, r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[small.index()].in_play, "canceled");
    assert!(!e.state.minions[b.index()].locked, "unlocked");
    // Passed: burned, Voter Captivation after it.
    let (mut e, b, p, small, small_card, _, _) = setup();
    hand(&mut e, 0, &["Voter Captivation"]);
    pick(&mut e, hunt);
    pick_when(&mut e, Choice::ChooseMinion(small));
    pick_when(&mut e, votes(p, true));
    close_polling(&mut e);
    assert!(offered_before(
        &mut e,
        play(0, 0, b),
        SeatId(0),
        WindowLabel::MinionPhase
    ));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[small.index()].in_play);
    assert!(e.state.seats[1].ash_crypt.contains(&small_card));
}

/// Bernard with no vampire of capacity 4 or less in play: not offered.
/// Backs `[LSJ 20100129] [ANK 20191228]` (group "Political action with
/// illegal terms").
#[test]
fn bernard_calls_no_blood_hunt_without_a_small_vampire() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bernard, the Scourge"), 4, false);
    e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 4, false);
    e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 6, 0), 4, false);
    let req = request(&mut e);
    assert!(!req.options.contains(&vamp(b, 0, Some(b), Target::None)));
    let small = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 4, false);
    e.state.minions[small.index()].in_torpor = true;
    let req = request(&mut e);
    assert!(req.options.contains(&vamp(b, 0, Some(b), Target::None)));
}

/// Bethany Ray: "Toreador and Toreador antitribu get +1 bleed against
/// you." A Toreador bleeding her controller for 2; a Toreador antitribu
/// +1 too, a Brujah not, nor a Toreador bleeding another Methuselah.
#[test]
fn bethany_ray_draws_toreador_bleeds() {
    let mut e = game(3);
    let t = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), find_card("Bethany Ray"), 4, true);
    e.add_minion(SeatId(2), camarilla("Ventrue", 0), 4, true);
    pick(&mut e, bleed(t));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8);
    let anti = e.add_minion(
        SeatId(2),
        textless(|c| c.clan == "Toreador antitribu", 0),
        4,
        false,
    );
    let brujah = e.add_minion(SeatId(2), camarilla("Brujah", 0), 4, false);
    let bleeding = |m: MinionId, s: u8| {
        e.state.printed_mod(
            m,
            Stat::Bleed,
            zvtes::state::PrintedCtx {
                kind: Some(ActionKind::Bleed),
                target: Some(SeatId(s)),
                ..Default::default()
            },
        )
    };
    assert_eq!(bleeding(anti, 1), 1);
    assert_eq!(bleeding(brujah, 1), 0);
    assert_eq!(bleeding(t, 2), 0);
}

/// Bulscu (base): with the Edge, he steals another Methuselah's location
/// or equipment costing 3 or less (Club Illusion, Helicopter), not
/// Flamethrower (4 pool); without the Edge, none.
#[test]
fn bulscu_steals_a_cheap_location_or_equipment_with_the_edge() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bulscu"), 4, false);
    assert!(!CRYPT[e.state.minions[b.index()].crypt_card().unwrap() as usize].adv);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    let club = in_play(&mut e, 1, "Club Illusion", None);
    let heli = in_play(&mut e, 1, "Helicopter", Some(z));
    let flame = in_play(&mut e, 1, "Flamethrower", Some(z));
    let at = |ai: u8, c: CardId| vamp(b, ai, Some(b), Target::Card(c));
    let req = request(&mut e);
    for o in [at(0, club), at(1, heli), at(1, flame)] {
        assert!(!req.options.contains(&o), "no Edge: {o:?}");
    }
    e.state.seats[0].has_edge = true;
    let req = request(&mut e);
    assert!(req.options.contains(&at(0, club)));
    assert!(!req.options.contains(&at(1, flame)));
    e.choose(index_of(&req, at(1, heli)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[heli.index()].host, Some(b));
}

/// Bulscu: "if you control the Edge" is read through resolution (rules-digest
/// D13, advanced rules §1.6.1): the Edge lost mid-action, the steal
/// fizzles, nothing paid (rules-qa 117.3), the Helicopter left on its
/// bearer.
#[test]
fn bulscu_fizzles_once_the_edge_is_lost_mid_action() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bulscu"), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    let heli = in_play(&mut e, 1, "Helicopter", Some(z));
    e.state.seats[0].has_edge = true;
    pick(&mut e, vamp(b, 1, Some(b), Target::Card(heli)));
    request(&mut e);
    assert!(e.action().is_some(), "under way");
    e.state.seats[0].has_edge = false;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[heli.index()].host, Some(z));
    assert_eq!(blood(&e, b), 4);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
}

/// A unique card is never taken by choice into a contest with oneself
/// (rulebook "Contested Cards": "you cannot voluntarily contest cards with
/// yourself"; advanced rules §1.13.1: "A play or choice that would do so
/// is illegal"): seat 0 controlling a Rowan Ring, Bulscu's steal of seat
/// 1's Rowan Ring is not offered (the Helicopter is), nor is Reformation
/// [chi]'s.
#[test]
fn a_unique_copy_one_controls_is_not_stolen_by_choice() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bulscu"), 4, false);
    let r = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Anarch), 0),
        4,
        false,
    );
    e.state.minions[r.index()].gained_levels = vec!["chi"];
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    in_play(&mut e, 0, "Rowan Ring", Some(r));
    let ring = in_play(&mut e, 1, "Rowan Ring", Some(z));
    let heli = in_play(&mut e, 1, "Helicopter", Some(z));
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Reformation"]);
    let req = request(&mut e);
    let steal = |c: CardId| vamp(b, 1, Some(b), Target::Card(c));
    let reform = |c: CardId| play_on(0, 0, r, Target::Card(c));
    assert!(req.options.contains(&steal(heli)) && req.options.contains(&reform(heli)));
    assert!(!req.options.contains(&steal(ring)), "Bulscu");
    assert!(!req.options.contains(&reform(ring)), "Reformation");

    // Seat 0's copy arriving mid-action: the chosen steal takes nothing,
    // the incoming copy not burned (burning is a forced contest's).
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bulscu"), 4, false);
    let r = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    let ring = in_play(&mut e, 1, "Rowan Ring", Some(z));
    e.state.seats[0].has_edge = true;
    pick(&mut e, vamp(b, 1, Some(b), Target::Card(ring)));
    request(&mut e);
    let own = in_play(&mut e, 0, "Rowan Ring", Some(r));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[ring.index()].in_play, "not burned");
    assert_eq!(e.state.cards[ring.index()].host, Some(z));
    assert_eq!(e.state.cards[own.index()].host, Some(r));
}

/// Bulscu at Ivan Krenyenko's Flamethrower (4 pool): its cost read with
/// Ivan's reduction, 2, so stolen. Backs Ivan Krenyenko's `[RTR
/// 19941109]` ("The reduction applies for other effects that check the
/// "cost" of equipment on him").
#[test]
fn bulscu_reads_the_cost_of_equipment_on_ivan_krenyenko() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bulscu"), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.seats[0].has_edge = true;
    let ivan = CRYPT
        .iter()
        .position(|c| c.name == "Ivan Krenyenko" && c.adv)
        .expect("Ivan Krenyenko (advanced)") as u16;
    let i = e.add_minion(SeatId(1), ivan, 4, false);
    e.state.minions[i.index()].locked = true;
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    let on_ivan = in_play(&mut e, 1, "Flamethrower", Some(i));
    let on_other = in_play(&mut e, 1, "Flamethrower", Some(z));
    let at = |c: CardId| vamp(b, 1, Some(b), Target::Card(c));
    let req = request(&mut e);
    assert!(!req.options.contains(&at(on_other)));
    e.choose(index_of(&req, at(on_ivan)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[on_ivan.index()].host, Some(b));
}

/// Bulscu merged: his base card's steal, the advanced card's +1 stealth at
/// a location and +1 strength, "[MERGED] Prince of Budapest".
#[test]
fn bulscu_merged_steals_and_sneaks() {
    let mut e = game(2);
    let adv = CRYPT
        .iter()
        .position(|c| c.name == "Bulscu" && c.adv)
        .expect("Bulscu (advanced)") as u16;
    let b = e.add_minion(SeatId(0), adv, 4, false);
    e.state.minions[b.index()].merged = Some(find_card("Bulscu"));
    let def = zvtes::cards::vampire_of(&e.state.minions[b.index()]).expect("merged");
    assert_eq!(def.abilities.len(), 2);
    assert_eq!(
        e.state.printed_mod(b, Stat::Strength, Default::default()),
        1
    );
    let at_location = zvtes::state::PrintedCtx {
        directed: Some(true),
        target_card: Some(find_library("Club Illusion")),
        ..Default::default()
    };
    assert_eq!(e.state.printed_mod(b, Stat::Stealth, at_location), 1);
    assert_eq!(e.state.title_of(b), Title::Prince("Budapest"));
}

/// Carlak: once a referendum is tallied, 1 blood for each Methuselah
/// casting only against: seat 1 (against only) counts, seat 2 (both ways)
/// and seat 0 (in favor) do not.
#[test]
fn carlak_feeds_on_the_opposition() {
    let mut e = game(3);
    let caller = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 4, false);
    let carlak = e.add_minion(SeatId(0), find_card("Carlak"), 4, false);
    let p1 = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 1), 4, false);
    let p2 = e.add_minion(SeatId(2), of_cap(Sect::Camarilla, 6, 0), 4, false);
    let q2 = e.add_minion(SeatId(2), of_cap(Sect::Camarilla, 6, 1), 4, false);
    for p in [p1, p2, q2] {
        e.state.minions[p.index()].granted_title = Some(Title::Primogen);
    }
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    to_polling(&mut e);
    pick(&mut e, Choice::CastCardVote { favor: true });
    pick_when(&mut e, votes(carlak, true));
    pick_when(&mut e, votes(p1, false));
    pick_when(&mut e, votes(p2, true));
    pick_when(&mut e, votes(q2, false));
    close_polling(&mut e);
    assert_eq!(blood(&e, carlak), 5);
}

/// Count Zaroff: in a blood hunt called on him, his Methuselah burns 1
/// pool to cancel it, nothing burned, and he goes to their uncontrolled
/// region with his blood as counters; again the same turn for another
/// blood hunt. Backs `[RTR 20180511]`.
#[test]
fn count_zaroff_cancels_blood_hunts_more_than_once_a_turn() {
    let mut e = game(2);
    let zaroff = find_card("Count Zaroff");
    let z = e.add_minion(SeatId(0), zaroff, 2, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let v1 = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 1, true);
    let v2 = e.add_minion(SeatId(1), camarilla("Ventrue", 1), 1, true);
    e.add_minion(SeatId(1), camarilla("Ventrue", 2), 4, false);
    for (n, v) in [(1, v1), (2, v2)] {
        let z = if n == 1 {
            z
        } else {
            e.add_minion(SeatId(0), zaroff, 2, false)
        };
        pick(
            &mut e,
            Choice::TakeAction {
                minion: z,
                action: ActionKind::Diablerie,
                target: Some(v),
            },
        );
        let cancel = vamp(z, 0, None, Target::None);
        let req = offering(&mut e, cancel);
        assert_eq!(req.window, WindowLabel::BloodHuntPolling);
        let held = blood(&e, z);
        e.choose(index_of(&req, cancel));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[0].pool, 10 - n);
        assert!(!e.state.minions[z.index()].in_play, "{n}");
        let slot = e.state.seats[0].uncontrolled.last().expect("uncontrolled");
        assert_eq!((slot.card, slot.counters), (zaroff, held));
    }
}

/// Count Zaroff chosen by Bernard's blood hunt: his Methuselah cancels it.
#[test]
fn count_zaroff_cancels_a_political_blood_hunt_on_him() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bernard, the Scourge"), 4, false);
    e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Count Zaroff"), 3, false);
    pick(&mut e, vamp(b, 0, Some(b), Target::None));
    pick_when(&mut e, vamp(z, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 9);
    assert!(!e.state.minions[z.index()].in_play);
    let slot = e.state.seats[1].uncontrolled.last().expect("uncontrolled");
    assert_eq!(slot.counters, 3);
}

/// Don Caravelli: once a combat of another vampire of his Methuselah's, 1
/// blood for its press, in the press step only; his own combat, no.
/// Backs `[PIB 20150121]`.
#[test]
fn don_caravelli_gives_a_press_in_the_press_step() {
    let mut e = game(2);
    let don = e.add_minion(SeatId(0), find_card("Don Caravelli"), 4, false);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    let press = vamp(don, 0, None, Target::None);
    let req = offering(&mut e, press);
    assert_eq!(req.window, WindowLabel::CombatPress);
    let before = blood(&e, z);
    e.choose(index_of(&req, press));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, don), 3);
    assert_eq!(before - blood(&e, z), 1, "a second round");
    // His own combat.
    let mut e = game(2);
    let don = e.add_minion(SeatId(0), find_card("Don Caravelli"), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    pick(&mut e, bleed(don));
    pick_when(&mut e, block(z));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, press);
}

/// Dr. John Dee: in any referendum's polling step, locked, he discards a
/// card requiring [tha] for +2 votes, once. Backs `[ANK 20220705]` (group
/// "Non-locking referendum ability").
#[test]
fn dr_john_dee_discards_sorcery_for_votes_while_locked_once() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 4, false);
    let john = e.add_minion(SeatId(0), find_card("Dr. John Dee"), 4, false);
    e.state.minions[john.index()].locked = true;
    e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 1), 4, false);
    hand(
        &mut e,
        0,
        &[
            "Kine Resources Contested",
            "Theft of Vitae",
            "Theft of Vitae",
        ],
    );
    pick(&mut e, play(0, 0, caller));
    to_polling(&mut e);
    let use_it = vamp(john, 0, None, Target::None);
    pick(&mut e, use_it);
    request(&mut e);
    assert_eq!(e.state.seats[0].ash, vec![find_library("Theft of Vitae")]);
    assert_eq!(
        e.referendum().expect("referendum").votes_of(&e.state, john),
        4
    );
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == use_it
    });
}

/// Dr. John Dee: a Ventrue in combat with him cannot strike: combat ends;
/// a Brujah can, a Ventrue against another vampire too.
#[test]
fn dr_john_dee_bars_combat_ends_to_ventrue() {
    let mut e = game(2);
    let john = e.add_minion(SeatId(0), find_card("Dr. John Dee"), 4, false);
    let other = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let v = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let br = e.add_minion(SeatId(1), camarilla("Brujah", 0), 4, false);
    assert!(e.state.combat_ends_barred(v, john));
    assert!(!e.state.combat_ends_barred(br, john));
    assert!(!e.state.combat_ends_barred(v, other));
}

/// Alex Wilkins bleeding a Methuselah down to 1 pool: the steal's 1 pool
/// still offered and paid down to 0 (an effect's price, rules-digest D3,
/// the chunk's AW3): they are ousted, the steal moot.
#[test]
fn alex_wilkins_steal_paid_with_the_last_pool_ousts() {
    let mut e = game(3);
    let alex = e.add_minion(SeatId(0), find_card("Alex Wilkins"), 1, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    e.add_minion(SeatId(2), camarilla("Ventrue", 1), 4, false);
    e.state.seats[1].pool = 2;
    pick(&mut e, bleed(alex));
    pick_when(&mut e, vamp(alex, 0, None, Target::None));
    request(&mut e);
    assert!(e.state.seats[1].ousted);
    assert_eq!(e.state.seats[1].pool, 0);
    assert_ne!(e.state.minions[alex.index()].owner, SeatId(1));
}

/// The Ankou's own Methuselah's hunter (the chunk's A4: "any vampire"), in
/// their own minion phase: an ordinary action, undirected, the hunter
/// burned.
#[test]
fn the_ankou_burns_his_own_methuselahs_hunter_undirected() {
    let mut e = game(2);
    let ankou = e.add_minion(SeatId(0), find_card("Ankou, The"), 4, false);
    let h_card = camarilla("Toreador", 0);
    let h = e.add_minion(SeatId(0), h_card, 1, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    pick(&mut e, action(h, ActionKind::Hunt));
    pick_when(&mut e, vamp(ankou, 0, Some(ankou), Target::Minion(h)));
    request(&mut e);
    assert_eq!(e.action().expect("the burn").directed_seat(&e.state), None);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[h.index()].in_play);
    assert!(e.state.seats[0].ash_crypt.contains(&h_card));
}

/// The Ankou's burn, its hunter gone to torpor before it resolves: still a
/// vampire in play, burned; gone from play, the action fizzles (rules-digest
/// D1; the chunk's A5).
#[test]
fn the_ankou_burns_a_torpid_hunter_and_fizzles_on_a_gone_one() {
    for gone in [false, true] {
        let mut e = game(2);
        let ankou = e.add_minion(SeatId(0), find_card("Ankou, The"), 4, false);
        let h = e.add_minion(SeatId(0), camarilla("Toreador", 0), 1, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        pick(&mut e, action(h, ActionKind::Hunt));
        pick_when(&mut e, vamp(ankou, 0, Some(ankou), Target::Minion(h)));
        request(&mut e);
        if gone {
            e.state.minions[h.index()].in_play = false;
        } else {
            e.state.minions[h.index()].in_torpor = true;
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        if gone {
            assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
        } else {
            assert!(!e.state.minions[h.index()].in_play, "burned from torpor");
            assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
        }
    }
}

/// Bulscu and a locquipment (Ruins of Villers Abbey, Belgium on another
/// Methuselah's vampire): in play a location (LSJ 20080114), stolen by his
/// location steal, not his equipment one; moved to a minion of his
/// controller's (RTR 19960112; rules-qa 88.9).
#[test]
fn bulscu_steals_a_locquipment_as_a_location() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bulscu"), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    e.state.seats[0].has_edge = true;
    let ruins = in_play(&mut e, 1, "Ruins of Villers Abbey, Belgium", Some(z));
    let at = |ai: u8| vamp(b, ai, Some(b), Target::Card(ruins));
    let req = request(&mut e);
    assert!(!req.options.contains(&at(1)), "no equipment in play");
    e.choose(index_of(&req, at(0)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.card_controller(ruins), SeatId(0));
    let host = e.state.cards[ruins.index()].host.expect("on a minion");
    assert_eq!(e.state.minions[host.index()].owner, SeatId(0));
}

/// Carlak: a referendum canceled by Delaying Tactics is not tallied, and
/// an automatic pass (Dia de los Muertos' kind: Cryptic Rider) has no
/// votes: no blood either way, though seat 1 cast only against in the
/// first (LSJ 20010209; the chunk's CA3).
#[test]
fn carlak_gains_nothing_from_a_canceled_or_automatic_referendum() {
    for automatic in [false, true] {
        let mut e = game(2);
        let caller = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 4, false);
        let carlak = e.add_minion(SeatId(0), find_card("Carlak"), 4, false);
        let p1 = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 1), 4, false);
        e.state.minions[p1.index()].granted_title = Some(Title::Primogen);
        hand(&mut e, 0, &["Kine Resources Contested"]);
        if automatic {
            e.state.seats[0].auto_pass_next = true;
            pick(&mut e, play(0, 0, caller));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(!e.state.seats[0].auto_pass_next, "it passed automatically");
        } else {
            hand(&mut e, 1, &["Delaying Tactics"]);
            pick(&mut e, play(0, 0, caller));
            to_polling(&mut e);
            pick_when(&mut e, votes(p1, false));
            pick_when(&mut e, play(0, 0, p1));
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(ash_has(&e, 1, "Delaying Tactics"));
        }
        assert_eq!(blood(&e, carlak), 4, "automatic {automatic}");
    }
}

/// Count Zaroff moved to the uncontrolled region by his cancel: the cards
/// on him go with him, as banished (rules-qa 27.7, 25.1; the chunk's CZ2):
/// his equipment no longer in play, kept on his slot, not in an ash heap.
#[test]
fn count_zaroff_keeps_his_cards_in_the_uncontrolled_region() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), find_card("Bernard, the Scourge"), 4, false);
    e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Count Zaroff"), 3, false);
    let vest = in_play(&mut e, 1, "Kevlar Vest", Some(z));
    pick(&mut e, vamp(b, 0, Some(b), Target::None));
    pick_when(&mut e, vamp(z, 0, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_play);
    assert!(!ash_has(&e, 1, "Kevlar Vest"), "kept");
    let card = &e.state.cards[vest.index()];
    assert!(!card.in_play);
    assert_eq!(card.host, Some(z), "with him");
}

/// Count Zaroff canceling a Veles' Hunt called on him: "If this
/// referendum fails or is canceled, no more Veles' Hunts can be played
/// this game" applies (the chunk's CZ4).
#[test]
fn count_zaroffs_cancel_of_veles_hunt_bars_it_for_the_game() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), gangrel(6, 0), 4, false);
    let g2 = e.add_minion(SeatId(0), gangrel(6, 1), 4, false);
    let z = e.add_minion(SeatId(1), find_card("Count Zaroff"), 3, false);
    e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 4, false);
    hand(&mut e, 0, &["Veles' Hunt", "Veles' Hunt"]);
    pick(&mut e, play(0, 0, g));
    pick_when(&mut e, Choice::ChooseMinion(z));
    pick_when(&mut e, vamp(z, 0, None, Target::None));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_play, "canceled, moved");
    assert!(!req.options.contains(&play(0, 0, g2)));
}

// ---- Keepers of Tradition vampires, Freddy Gage to Joseph DiGiaccomo -------

/// Freddy Gage: "Titled vampires get +1 bleed against you." A prince
/// (Jann Berger, +1 bleed of his own) bleeds Freddy's controller for 3; an
/// untitled vampire +0, nor a titled one bleeding another Methuselah.
#[test]
fn freddy_gage_draws_titled_bleeds() {
    let mut e = game(3);
    let jann = e.add_minion(SeatId(0), find_card("Jann Berger"), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(1), find_card("Freddy Gage"), 1, true);
    e.add_minion(SeatId(2), camarilla("Ventrue", 0), 4, true);
    pick(&mut e, bleed(jann));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 7);
    let untitled = e.add_minion(SeatId(2), camarilla("Brujah", 0), 4, false);
    let bleeding = |m: MinionId, s: u8| {
        e.state.printed_mod(
            m,
            Stat::Bleed,
            zvtes::state::PrintedCtx {
                kind: Some(ActionKind::Bleed),
                target: Some(SeatId(s)),
                ..Default::default()
            },
        )
    };
    assert_eq!(bleeding(untitled, 1), 0);
    assert_eq!(bleeding(jann, 2), 1, "his own +1 only");
}

/// Gotsdam, The Tired Warrior: an aggravated hand strike costs him 1 blood,
/// he stays ready; his "strike: combat ends" for 1 blood ends the combat
/// before the strikes resolve.
#[test]
fn gotsdam_ends_combat_for_blood_and_shrugs_off_aggravated() {
    for ends in [false, true] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), find_card("Gotsdam, The Tired Warrior"), 4, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        let b = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
        e.state.minions[b.index()].timed.push((
            zvtes::cards::Lasting::AggravatedHands,
            zvtes::state::Expiry::EndOfTurn,
            SeatId(1),
        ));
        pick(&mut e, bleed(g));
        pick_when(&mut e, block(b));
        let ability = vamp(g, 0, None, Target::None);
        let req = offering(&mut e, ability);
        assert_eq!(
            (req.seat, req.window),
            (SeatId(0), WindowLabel::CombatStrike)
        );
        if ends {
            e.choose(index_of(&req, ability));
        } else {
            e.choose(index_of(&req, Choice::HandStrike));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[g.index()];
        assert!(v.ready(), "never in torpor");
        assert_eq!(
            v.blood, 3,
            "the strike's blood, or the hand's damage as normal"
        );
        if ends {
            assert_eq!(e.state.minions[b.index()].blood, 4, "no strike resolved");
        } else {
            assert!(e.state.minions[b.index()].blood < 4);
        }
    }
}

/// Gracetius: "While you have the Edge, Gracetius can wake." Locked, his
/// Methuselah holding the Edge: not offered as a master card is played
/// (no action), offered as the acting Methuselah's Bonding [dom] is
/// played; woken, his block is offered, an option only — passed, the
/// bleed lands. Without the Edge, never offered. Backs `[ANK 20210817]`
/// and `[TOM 19951129]` (group "Wake").
#[test]
fn gracetius_wakes_with_the_edge_in_the_as_played_window_not_in_combat() {
    for edge in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 6, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        let g = e.add_minion(SeatId(1), find_card("Gracetius"), 4, false);
        e.state.minions[g.index()].locked = true;
        e.state.seats[1].has_edge = edge;
        let wake = vamp(g, 0, None, Target::None);
        e.at_master_phase();
        hand(&mut e, 0, &["Villein", "Bonding"]);
        pick(
            &mut e,
            Choice::Play {
                card: 0,
                mode: 0,
                actor: None,
                target: Target::Minion(a),
            },
        );
        pick(&mut e, Choice::SetX(2));
        if !edge {
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, wake);
            pick(&mut e, bleed(a));
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, wake);
            continue;
        }
        never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, wake);
        pick(&mut e, bleed(a));
        pick(&mut e, play(0, 0, a));
        let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
        e.choose(index_of(&req, wake));
        let req = offering(&mut e, block(g));
        assert!(!req.options.contains(&wake), "woken once");
        e.choose(index_of(&req, Choice::Pass));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.seats[1].pool < 10, "unblocked");
        assert!(e.state.minions[g.index()].locked);
    }
}

/// Gracetius in combat: another of his Methuselah's vampires blocks; as
/// cards are played in that combat, his wake is never offered (ANK
/// 20210817: "except in combat").
#[test]
fn gracetius_never_wakes_in_combat() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Gracetius"), 4, false);
    e.state.minions[g.index()].locked = true;
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Thrown Gate"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(z));
    pick_when(&mut e, play(0, 0, a));
    // Out of the combat, after it, he may (LSJ 20091123).
    loop {
        let req = request(&mut e);
        if e.combat().is_none() {
            break;
        }
        assert!(!req.options.contains(&vamp(g, 0, None, Target::None)));
        e.choose(fallback(&req));
    }
}

/// A wake "Can be played or used after action resolution (e.g. to use a
/// reaction card like Fast Reaction after combat)" [LSJ 20091123] (group
/// "Wake"). After a successful bleed, locked L plays Wake with Evening's
/// Freshness, then Touch of Pain (1 damage on the acting A), not offered
/// before the wake. After B's block, locked Gracetius (the Edge) wakes in
/// the after-combat window and plays Fast Reaction: locked, his combat
/// with A (his hand 2, B's 1 on A).
#[test]
fn a_wake_is_played_after_combat_or_resolution() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let l = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[l.index()].gained_levels = vec!["pot"];
    e.state.minions[l.index()].locked = true;
    let other = e.add_minion(SeatId(1), camarilla("Ventrue", 1), 4, false);
    e.state.minions[other.index()].locked = true;
    hand(
        &mut e,
        1,
        &["Wake with Evening's Freshness", "Touch of Pain"],
    );
    pick(&mut e, bleed(a));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionAfterResolution);
    assert!(!req.options.contains(&play(1, 0, l)), "locked");
    e.choose(index_of(&req, play(0, 0, l)));
    pick_when(&mut e, play(0, 0, l));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 1, "Touch of Pain");

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let b = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Gracetius"), 4, false);
    e.state.minions[g.index()].gained_levels = vec!["aus"];
    e.state.minions[g.index()].locked = true;
    e.state.seats[1].has_edge = true;
    hand(&mut e, 1, &["Fast Reaction"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(b));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionAfterCombat);
    assert!(!req.options.contains(&play(0, 0, g)), "locked");
    e.choose(index_of(&req, vamp(g, 0, None, Target::None)));
    pick_when(&mut e, play(0, 0, g));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[g.index()].locked);
    assert_eq!(blood(&e, a), 4 - 1 - 2, "B's hand, then Gracetius's");
}

/// Gustav Breidenstein: in a referendum's polling step he locks for 5
/// votes (2 as a prince, 7 in all), once.
#[test]
fn gustav_locks_for_five_votes() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 4, false);
    let gustav = e.add_minion(SeatId(0), find_card("Gustav Breidenstein"), 4, false);
    e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 1), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    to_polling(&mut e);
    let use_it = vamp(gustav, 0, None, Target::None);
    pick(&mut e, use_it);
    request(&mut e);
    assert!(e.state.minions[gustav.index()].locked);
    assert_eq!(
        e.referendum()
            .expect("referendum")
            .votes_of(&e.state, gustav),
        7
    );
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, use_it);
}

/// Gwyedd: his hunt (undirected) blocked in vain by a younger vampire — it
/// is locked as the after-resolution window opens; his bleed (directed),
/// at +1 stealth, blocked in vain likewise: nobody locked.
#[test]
fn gwyedd_locks_younger_failed_blockers_of_his_undirected_action() {
    for hunt in [true, false] {
        let mut e = game(2);
        let g = e.add_minion(SeatId(0), find_card("Gwyedd"), 4, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        let y = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 4, false);
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, true);
        if hunt {
            pick(&mut e, action(g, ActionKind::Hunt));
        } else {
            e.state.minions[g.index()].timed.push((
                zvtes::cards::Lasting::Stealth(1),
                zvtes::state::Expiry::EndOfTurn,
                SeatId(0),
            ));
            pick(&mut e, bleed(g));
        }
        pick_when(&mut e, block(y));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[y.index()].locked, hunt);
    }
}

/// Herbert Westin: after his successful equip action (Sport Bike from
/// hand), his controller may draw 1 card; after his hunt, not offered.
#[test]
fn herbert_westin_draws_after_a_successful_equip() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), find_card("Herbert Westin"), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.state.minions[z.index()].locked = true;
    hand(&mut e, 0, &["Sport Bike"]);
    e.state.seats[0].library.push(find_library("Villein"));
    let draw = vamp(h, 0, None, Target::None);
    pick(&mut e, play(0, 0, h));
    let req = offering(&mut e, draw);
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    e.choose(index_of(&req, draw));
    request(&mut e);
    assert!(e.state.seats[0].library.is_empty(), "drawn");
    e.state.minions[h.index()].locked = false;
    e.state.seats[0].library.push(find_library("Villein"));
    pick(&mut e, action(h, ActionKind::Hunt));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, draw);
}

/// Hiram "Hide" DeVries: an action to burn a location his controller
/// controls costs 1 more blood — Arson (no cost) by another Methuselah's
/// vampire, Maabara's granted burn, his own Arson at his Methuselah's
/// location — paid as it resolves, Hiram locked too (ready); not at
/// another Methuselah's location, nor with Hiram in torpor. At 0 blood, the
/// burns of his controller's location are not offered, the other is.
#[test]
fn hiram_devries_taxes_burning_his_controllers_locations() {
    // (Hiram's seat, Hiram locked, in torpor, Maabara's burn, at Hiram's
    // controller's location, the actor is Hiram, extra blood paid)
    for (seat, locked, torpor, granted, his, own, paid) in [
        (1, false, false, false, true, false, 1),
        (1, false, false, true, true, false, 1),
        (0, false, false, false, false, true, 0),
        (0, false, false, false, true, true, 1),
        (1, false, false, false, false, false, 0),
        (1, true, false, false, true, false, 1),
        (1, false, true, false, true, false, 0),
    ] {
        let mut e = game(3);
        let h = e.add_minion(SeatId(seat), find_card("Hiram \"Hide\" DeVries"), 3, torpor);
        e.state.minions[h.index()].locked = locked;
        let a = if own {
            h
        } else {
            e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false)
        };
        e.add_minion(SeatId(2), camarilla("Brujah", 0), 4, false);
        let maabara = in_play(&mut e, seat, "Maabara", None);
        let other = in_play(&mut e, 2, "Information Highway", None);
        let loc = if his { maabara } else { other };
        let choice = if granted {
            vamp_burn(maabara, a)
        } else {
            hand(&mut e, 0, &["Arson"]);
            play_on(0, 0, a, Target::Card(loc))
        };
        let before = blood(&e, a);
        pick(&mut e, choice);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.cards[loc.index()].in_play, "burned");
        assert_eq!(
            blood(&e, a),
            before - paid,
            "seat {seat}, locked {locked}, torpor {torpor}, granted {granted}, his {his}"
        );
    }
    // Themistocles with 1 blood: his burn (1 blood) of Hiram's
    // controller's location is not offered, another's is.
    let mut e = game(3);
    e.add_minion(SeatId(1), find_card("Hiram \"Hide\" DeVries"), 3, false);
    let t = e.add_minion(SeatId(0), find_card("Themistocles"), 1, false);
    e.add_minion(SeatId(2), camarilla("Brujah", 0), 4, false);
    let his = in_play(&mut e, 1, "Maabara", None);
    let other = in_play(&mut e, 2, "Information Highway", None);
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { target, .. } if *target == Target::Card(his))));
    assert!(req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { source: AbilitySource::Vampire(v), target, .. }
            if *v == t && *target == Target::Card(other))));
}

/// `actor`'s use of a location's "any vampire may burn this location"
/// action (Maabara's third ability).
fn vamp_burn(loc: CardId, actor: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(loc),
        ability: 2,
        actor: Some(actor),
        target: Target::None,
    }
}

/// Jann Berger blocking Lucian, the Perfect (his strength made 0): Thrown
/// Gate [pot] played as the strike is chosen costs him nothing (Lucian's
/// toll reduced); played at range for the maneuver, it costs him the toll,
/// 1 blood. Another Potence vampire pays the toll once either way. Backs
/// `[LSJ 20090529]` (and its mentions on Thrown Gate and Lucian).
#[test]
fn jann_berger_pays_lucians_toll_only_for_a_maneuvered_thrown_gate() {
    for (jann, at_range, paid) in [
        (true, false, 0),
        (true, true, 1),
        (false, false, 1),
        (false, true, 1),
    ] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), find_card("Lucian, the Perfect"), 6, false);
        let strength = e.state.base_strength(l) as i8;
        e.state.minions[l.index()].timed.push((
            zvtes::cards::Lasting::Strength(-strength),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        let card = if jann {
            find_card("Jann Berger")
        } else {
            vampire_with("POT", 0)
        };
        let j = e.add_minion(SeatId(1), card, 4, false);
        e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        hand(&mut e, 1, &["Thrown Gate"]);
        pick(&mut e, bleed(l));
        pick_when(&mut e, block(j));
        let gate = play(0, 0, j);
        let req = if at_range {
            offering(&mut e, gate)
        } else {
            until(&mut e, SeatId(1), WindowLabel::CombatStrike)
        };
        let window = if at_range {
            WindowLabel::CombatDetermineRange
        } else {
            WindowLabel::CombatStrike
        };
        assert_eq!(req.window, window);
        e.choose(index_of(&req, gate));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, j), 4 - paid, "Jann {jann}, at range {at_range}");
        assert_eq!(blood(&e, l), 5, "the strike made");
    }
}

/// Johan Wrede bleeding: an older vampire of the bled Methuselah burns 1 blood
/// to reduce his bleed by 1 — nothing bled; a vampire as young as he is (Freddy
/// Gage, capacity 1) is not offered it; his own controller's older vampire is
/// offered it too.
#[test]
fn johan_wrede_bleeds_less_for_older_blood() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Johan Wrede"), 1, false);
    let own = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 5, 0), 4, false);
    let old = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 5, 1), 4, false);
    // Capacity 1 too; his text bears on titled bleeders only.
    let young = e.add_minion(SeatId(1), find_card("Freddy Gage"), 1, false);
    pick(&mut e, bleed(j));
    let mine = vamp(j, 0, Some(own), Target::None);
    let req = offering(&mut e, mine);
    assert_eq!(req.seat, SeatId(0));
    let theirs = vamp(j, 0, Some(old), Target::None);
    let req = offering(&mut e, theirs);
    assert!(!req.options.contains(&vamp(j, 0, Some(young), Target::None)));
    e.choose(index_of(&req, theirs));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, old), 3);
    assert_eq!(e.state.seats[1].pool, 10, "bled for 0");
}

/// Johannes Castelein: his Political Stranglehold's referendum canceled by
/// Delaying Tactics — its player burns 2 pool, the card back in his hand;
/// the card canceled as played by Direct Intervention (1 pool) — 3 in all.
#[test]
fn johannes_castelein_charges_the_canceler() {
    for as_played in [false, true] {
        let mut e = game(2);
        let jo = e.add_minion(SeatId(0), find_card("Johannes Castelein"), 4, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        let z = e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
        hand(&mut e, 0, &["Political Stranglehold"]);
        pick(&mut e, play(0, 0, jo));
        if as_played {
            hand(&mut e, 1, &["Direct Intervention"]);
            let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
            e.choose(index_of(&req, master_play(0)));
            request(&mut e);
            assert_eq!(e.state.seats[1].pool, 10 - 1 - 2);
        } else {
            hand(&mut e, 1, &["Delaying Tactics"]);
            to_referendum(&mut e);
            pick_when(&mut e, play(0, 0, z));
            request(&mut e);
            assert_eq!(e.state.seats[1].pool, 8);
            assert_eq!(
                e.state.seats[0].hand,
                vec![find_library("Political Stranglehold")]
            );
        }
        assert_eq!(e.state.seats[0].pool, 10);
    }
}

/// Joseph DiGiaccomo, locked: unlocked as another Methuselah puts
/// Information Highway (a master location) in play; locked again, an
/// equipment entering play leaves him locked.
#[test]
fn joseph_digiaccomo_unlocks_as_a_master_location_enters() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    let j = e.add_minion(SeatId(1), find_card("Joseph DiGiaccomo"), 4, false);
    e.state.minions[j.index()].locked = true;
    e.at_master_phase();
    hand(&mut e, 0, &["Information Highway", "Sport Bike"]);
    pick(&mut e, master_play(0));
    request(&mut e);
    assert!(!e.state.minions[j.index()].locked);
    e.state.minions[j.index()].locked = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[j.index()].locked);
}

/// Freddy Gage and Bethany Ray under one Methuselah: a titled Toreador's
/// bleed against them gets both +1s, beside its own (Jann Berger is a
/// Brujah: a Toreador prince is used; the chunk's FG2).
#[test]
fn freddy_gage_stacks_with_bethany_ray() {
    let mut e = game(3);
    let t = e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.state.minions[t.index()].granted_title = Some(Title::Prince("Atlanta"));
    e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
    e.add_minion(SeatId(1), find_card("Freddy Gage"), 1, true);
    e.add_minion(SeatId(1), find_card("Bethany Ray"), 4, true);
    e.add_minion(SeatId(2), camarilla("Ventrue", 0), 4, true);
    pick(&mut e, bleed(t));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 1 - 1 - 1);
}

/// Gotsdam: "Frenzy cards cannot be played on him" — Terror Frenzy by his
/// blocker is never offered (rules-digest §3.19; the chunk's GO3).
#[test]
fn gotsdam_takes_no_frenzy_card() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Gotsdam, The Tired Warrior"), 4, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let z = e.add_minion(SeatId(1), vampire_with("ani", 0), 6, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    hand(&mut e, 1, &["Terror Frenzy"]);
    pick(&mut e, bleed(g));
    pick_when(&mut e, block(z));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::Play { card: 0, .. })
    });
    assert_eq!(e.state.seats[1].hand, vec![find_library("Terror Frenzy")]);
}

/// Gracetius woken with the Edge, the Edge then lost: he stays woken, his
/// block still offered (the Edge read as he wakes, D13, D33; the chunk's
/// GR3).
#[test]
fn gracetius_stays_woken_once_the_edge_is_lost() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 6, false);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Gracetius"), 4, false);
    e.state.minions[g.index()].locked = true;
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Bonding"]);
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    e.choose(index_of(&req, vamp(g, 0, None, Target::None)));
    e.state.seats[1].has_edge = false;
    e.state.seats[0].has_edge = true;
    let req = offering(&mut e, block(g));
    e.choose(index_of(&req, block(g)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

/// Gustav Breidenstein in torpor: he may lock in the polling step, but a
/// torpid vampire casts no votes (rules-digest D25; the chunk's GU3).
#[test]
fn gustav_in_torpor_locks_but_casts_no_votes() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 7, 0), 4, false);
    let gustav = e.add_minion(SeatId(0), find_card("Gustav Breidenstein"), 4, true);
    e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 7, 1), 4, false);
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    to_polling(&mut e);
    pick(&mut e, vamp(gustav, 0, None, Target::None));
    request(&mut e);
    assert!(e.state.minions[gustav.index()].locked);
    let r = e.referendum().expect("referendum");
    assert_eq!(r.votes_of(&e.state, gustav), 0);
}

/// Gwyedd's hunt: a younger vampire's attempt failed by Elder
/// Impersonation [OBF] ("That attempt fails"), then another's successful
/// block: the failed one is locked after resolution all the same (the
/// chunk's GW2: every failed attempt, a barred one beside a successful
/// block; rules-qa 63.4).
#[test]
fn gwyedd_locks_barred_and_outdone_blockers() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Gwyedd"), 4, false);
    superior(&mut e, g, "obf");
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    let y = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 4, 0), 4, false);
    // Kite: "+1 intercept", the card's +1 stealth met.
    let w = e.add_minion(SeatId(1), find_card("Kite"), 4, false);
    hand(&mut e, 0, &["Elder Impersonation"]);
    pick(&mut e, action(g, ActionKind::Hunt));
    pick_when(&mut e, block(y));
    pick_when(&mut e, play(0, 1, g));
    pick_when(&mut e, block(w));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
    assert!(e.state.minions[y.index()].locked, "barred, locked");
}

/// Johan Wrede bleeding for 3: two older vampires of the bled Methuselah
/// burn 1 blood each, one of them twice (no limit, rules-qa 79.3; the
/// chunk's JW2): bled for 0. Used as a reaction, and again once the blocks
/// are declined, as the bleed would succeed (JW4, LSJ 20081123).
#[test]
fn johan_wrede_is_reduced_as_often_as_paid() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), find_card("Johan Wrede"), 1, false);
    e.state.minions[j.index()].timed.push((
        zvtes::cards::Lasting::Bleed(2),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    e.add_minion(SeatId(0), of_cap(Sect::Camarilla, 5, 0), 4, false);
    let o1 = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 5, 1), 4, false);
    let o2 = e.add_minion(SeatId(1), of_cap(Sect::Camarilla, 6, 0), 4, false);
    pick(&mut e, bleed(j));
    let use_by = |m| vamp(j, 0, Some(m), Target::None);
    for _ in 0..2 {
        let req = offering(&mut e, use_by(o1));
        assert_eq!(req.window, WindowLabel::ActionBlock, "as a reaction");
        e.choose(index_of(&req, use_by(o1)));
    }
    // The reaction window passed: again as the bleed would succeed.
    let late = loop {
        let req = request(&mut e);
        if req.window != WindowLabel::ActionBlock && req.options.contains(&use_by(o2)) {
            break req;
        }
        e.choose(fallback(&req));
    };
    assert_eq!(late.window, WindowLabel::ActionBlocksDeclined);
    e.choose(index_of(&late, use_by(o2)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, o1), blood(&e, o2)), (2, 3));
    assert_eq!(e.state.seats[1].pool, 10, "bled for 0");
}

/// Joseph DiGiaccomo, locked: a master location canceled as it is played
/// (Sudden Reversal) was never put in play — he stays locked; a copy
/// entering play contested is put in play — he unlocks (the chunk's JD2).
#[test]
fn joseph_digiaccomo_unlocks_for_a_contested_location_not_a_canceled_one() {
    for canceled in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
        e.add_minion(SeatId(0), camarilla("Toreador", 1), 4, false);
        let j = e.add_minion(SeatId(1), find_card("Joseph DiGiaccomo"), 4, false);
        e.state.minions[j.index()].locked = true;
        if canceled {
            hand(&mut e, 1, &["Sudden Reversal"]);
        } else {
            in_play(&mut e, 1, "Information Highway", None);
        }
        e.at_master_phase();
        hand(&mut e, 0, &["Information Highway"]);
        pick(&mut e, master_play(0));
        if canceled {
            let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
            e.choose(index_of(&req, master_play(0)));
        }
        request(&mut e);
        assert_eq!(
            e.state.minions[j.index()].locked,
            canceled,
            "canceled {canceled}"
        );
    }
}

// ---- Armor of Vitality ------------------------------------------------------

/// Armor of Vitality [for]: "Prevent 3 damage." Fewer pending, all of it
/// prevented [RTR 20041202], the rest lost, not carried to the next damage
/// [ANK 20200318] [LSJ 20001114]: A's 1-damage hand strike prevented; A
/// presses (Relentless Pursuit [pot]) and its round-2 strike lands in full.
#[test]
fn armor_of_vitality_prevents_what_there_is() {
    let (mut e, a, x) = duel(&["pot"], &["for"]);
    hand(&mut e, 0, &["Relentless Pursuit"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    request(&mut e);
    assert_eq!(blood(&e, x), 4 - 1, "only the Armor's blood");
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1 - 1, "round 2's strike unprevented");
    assert!(ash_has(&e, 1, "Armor of Vitality"));
}

/// Armor of Vitality: "Cannot be used if there is no damage to prevent."
/// [LSJ 20001114] X dodges with Read Intentions [AUS]: no prevention
/// offered to X.
#[test]
fn armor_of_vitality_needs_damage_to_prevent() {
    let (mut e, a, x) = duel(&[], &["for", "aus", "aus"]);
    hand(&mut e, 1, &["Armor of Vitality", "Read Intentions"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(1, 1, x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert_eq!(blood(&e, x), 4);
}

/// Armor of Vitality [FOR]: "if any of the damage was from the opposing
/// minion's melee weapon, that weapon is destroyed." A's Baseball Bat
/// strike (2 damage) prevented and the bat burned; against a hand strike
/// no card burns.
#[test]
fn armor_of_vitality_superior_destroys_the_striking_melee_weapon() {
    let (mut e, a, x) = duel(&[], &["for", "for"]);
    let bat = on(&mut e, "Baseball Bat", a);
    hand(&mut e, 1, &["Armor of Vitality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(bat, 0));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[bat.index()].in_play, "the bat burned");
    assert!(ash_has(&e, 0, "Baseball Bat"));
    assert_eq!(blood(&e, x), 4 - 1, "the 2 prevented, the Armor's 1 paid");

    let (mut e, a, x) = duel(&[], &["for", "for"]);
    let bat = on(&mut e, "Baseball Bat", a);
    hand(&mut e, 1, &["Armor of Vitality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, Choice::HandStrike);
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        e.state.cards[bat.index()].in_play,
        "a hand strike: no weapon"
    );
}

/// Armor of Vitality [FOR] against Bundi's strike, "both a hand strike and
/// a melee weapon strike" [LSJ 20080702-1]: A's Bundi strike prevented and
/// Bundi burned.
#[test]
fn armor_of_vitality_superior_destroys_a_striking_bundi() {
    let (mut e, a, x) = duel(&[], &["for", "for"]);
    let bundi = equipped(&mut e, "Bundi", a);
    hand(&mut e, 1, &["Armor of Vitality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(bundi, 0));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[bundi.index()].in_play, "Bundi burned");
    assert!(ash_has(&e, 0, "Bundi"));
    assert_eq!(blood(&e, x), 4 - 1, "the 2 prevented, the Armor's 1 paid");
}

/// A textless untitled vampire of capacity `lo..=hi`, the `nth`.
#[track_caller]
fn in_caps(lo: u8, hi: u8, nth: usize) -> u16 {
    textless(|c| (lo..=hi).contains(&c.capacity), nth)
}

/// The press `seat`'s minion may use, offered at its press request.
fn offered_press(e: &mut Engine, seat: SeatId) -> bool {
    loop {
        let req = request(e);
        if req.window == WindowLabel::MinionPhase {
            return false;
        }
        if req.seat == seat
            && req.window == WindowLabel::CombatPress
            && req.options.contains(&Choice::UsePress)
        {
            return true;
        }
        let i = fallback(&req);
        e.choose(i);
    }
}

// ---- Backstep ---------------------------------------------------------------

/// Backstep: "Maneuver, only usable to go to long range. If the opposing
/// minion's strike successfully inflicts any damage on this minion this
/// round, the opposing minion gets an optional press." A goes long; X's
/// .44 Magnum (2R) lands: X is offered the press. A dodging it (Read
/// Intentions [AUS]): no press for X.
#[test]
fn backstep_goes_long_and_gives_a_press_if_hit() {
    let (mut e, a, x) = duel(&[], &[]);
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Backstep"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    request(&mut e);
    assert_eq!(
        e.combat().expect("combat").range,
        zvtes::combat::Range::Long
    );
    pick_when(&mut e, weapon(gun, 0));
    assert!(offered_press(&mut e, SeatId(1)));
    assert_eq!(blood(&e, a), 4 - 2);

    let (mut e, a, x) = duel(&["aus", "aus"], &[]);
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Backstep", "Read Intentions"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, weapon(gun, 0));
    assert!(!offered_press(&mut e, SeatId(1)));
    assert_eq!(blood(&e, a), 4);
}

/// Backstep's press "can only be used during the current round" [TOM
/// 19960521]: X declines it as A presses on (Relentless Pursuit [pot]);
/// hit again in round 2, X is offered none.
#[test]
fn backsteps_press_is_for_its_round_only() {
    let (mut e, a, x) = duel(&["pot"], &[]);
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Backstep", "Relentless Pursuit"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatPress);
    assert!(req.options.contains(&Choice::UsePress));
    e.choose(index_of(&req, Choice::Pass));
    request(&mut e);
    assert_eq!(e.combat().expect("round 2").round, 2);
    pick_when(&mut e, weapon(gun, 0));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::UsePress,
    );
}

/// Backstep, two copies (each card's clause its own, rules-digest D4): A
/// goes long, X's .44 Magnum maneuvers back to close, A goes long again;
/// the Magnum's 2R lands: X presses to continue, A's Relentless Pursuit
/// [pot] presses to end, X presses again with the second press.
#[test]
fn two_backsteps_give_two_presses() {
    let (mut e, a, x) = duel(&["pot"], &[]);
    let gun = equipped(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Backstep", "Backstep", "Relentless Pursuit"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, weapon(gun, 1));
    pick_when(&mut e, play(0, 0, a));
    // At long range the Magnum's strike is X's only one (elided).
    pick_when(&mut e, Choice::UsePress);
    assert_eq!(blood(&e, a), 4 - 2, "the Magnum's 2R landed");
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, Choice::UsePress);
    assert_eq!(req.seat, SeatId(1), "X's second press");
    e.choose(index_of(&req, Choice::UsePress));
    request(&mut e);
    assert_eq!(e.combat().expect("round 2").round, 2);
}

/// Backstep's "successfully inflicts" leaves out damage the minion is
/// immune to (rules-qa 116.138): Mictlantecuhtli goes long, X's .44
/// Magnum (2R) hits him, none landing: X gets no press.
#[test]
fn backstep_gives_no_press_for_damage_ignored_through_immunity() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mictlantecuhtli"), 6, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Backstep"]);
    fight(&mut e, m, x);
    pick_when(&mut e, play(0, 0, m));
    pick_when(&mut e, weapon(gun, 0));
    assert!(!offered_press(&mut e, SeatId(1)));
    assert_eq!(blood(&e, m), 6);
}

// ---- Brute Force ------------------------------------------------------------

/// Brute Force [pot]: "Strike: hand strike at +1 damage or use a melee
/// weapon strike at +2 damage", "Do not replace until after combat": the
/// hand strike deals 2 (seen held as X chooses its strike); the Baseball
/// Bat's strike 2 + 2.
#[test]
fn brute_force_strikes_by_hand_or_weapon() {
    let (mut e, a, x) = duel(&["pot"], &["aus", "aus"]);
    hand(&mut e, 0, &["Brute Force"]);
    hand(&mut e, 1, &["Read Intentions"]);
    e.state.seats[0].library = vec![find_library("Form of Mist"); 3].into();
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    expect(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert_eq!(e.state.seats[0].held[zvtes::state::Release::AfterCombat], 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2);
    assert_eq!(e.state.seats[0].held[zvtes::state::Release::AfterCombat], 0);

    let (mut e, a, x) = duel(&["pot"], &[]);
    let bat = on(&mut e, "Baseball Bat", a);
    hand(&mut e, 0, &["Brute Force"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play_on(0, 1, a, Target::Card(bat)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 0, "the bat's 2 and 2: 4 mended");
}

/// Brute Force [POT]: "Additional damage inherits all of the properties of
/// the base damage." [RTR 19970630] [TOM 19960225] Kali's Fang's aggravated
/// strike at +3: all 5 points aggravated, pending on X.
#[test]
fn brute_force_adds_to_the_strikes_kind() {
    let (mut e, a, x) = duel(&["pot", "pot"], &["for"]);
    let fang = on(&mut e, "Kali's Fang", a);
    hand(&mut e, 0, &["Brute Force"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play_on(0, 3, a, Target::Card(fang)));
    offering(&mut e, play(0, 0, x));
    let pending = e.combat().expect("combat").sides[1].pending;
    assert_eq!((pending.normal, pending.aggravated), (0, 5));
}

// ---- Burst of Sunlight ------------------------------------------------------

/// Burst of Sunlight: "Damage done to the striking minion is
/// environmental, it is inflicted regardless of the range." [LSJ 19970801]
/// [ANK 20221212] At long range (X's Backstep), [tha]: X takes 1R
/// aggravated, A its own 1 aggravated: both wounded to torpor.
#[test]
fn burst_of_sunlight_burns_the_striker_at_any_range() {
    let (mut e, a, x) = duel(&["tha"], &[]);
    hand(&mut e, 0, &["Burst of Sunlight"]);
    hand(&mut e, 1, &["Backstep"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert!(
        e.state.minions[a.index()].in_torpor,
        "its own aggravated damage"
    );
}

/// Burst of Sunlight dodged: the dodge protects the dodger only; the
/// striker still takes its 1 (the strike resolved, advanced rules §4.6.3).
#[test]
fn burst_of_sunlight_self_damage_survives_a_dodge() {
    let (mut e, a, x) = duel(&["tha"], &["aus", "aus"]);
    hand(&mut e, 0, &["Burst of Sunlight"]);
    hand(&mut e, 1, &["Read Intentions"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[x.index()].in_torpor);
    assert_eq!(blood(&e, x), 4);
    assert!(e.state.minions[a.index()].in_torpor);
}

/// Burst of Sunlight: the striker's damage comes with its strike's
/// resolution (advanced rules §4.6.3): X's Force of Personality [pre]
/// ends the combat first, A takes none and X none.
#[test]
fn burst_of_sunlight_hurts_nobody_once_combat_ends_first() {
    let (mut e, a, x) = duel(&["tha"], &["pre"]);
    hand(&mut e, 0, &["Burst of Sunlight"]);
    hand(&mut e, 1, &["Force of Personality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[a.index()].in_torpor);
    assert_eq!(blood(&e, a), 4, "no self-damage");
    assert!(!e.state.minions[x.index()].in_torpor);
    assert_eq!(blood(&e, x), 4 - 1, "its card's blood, no damage");
}

// ---- Dragon's Breath Rounds -------------------------------------------------

/// Dragon's Breath Rounds: "Does not make the gun base damage aggravated."
/// [LSJ 20030419-2] The .44 Magnum's 2 normal and 2 aggravated pending on
/// X; the gun burned after its strike resolution.
#[test]
fn dragons_breath_rounds_adds_aggravated_damage_and_burns_the_gun() {
    let (mut e, a, x) = duel(&[], &["for"]);
    let gun = on(&mut e, ".44 Magnum", a);
    hand(&mut e, 0, &["Dragon's Breath Rounds"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play(0, 0, a));
    offering(&mut e, play(0, 0, x));
    let pending = e.combat().expect("combat").sides[1].pending;
    assert_eq!((pending.normal, pending.aggravated), (2, 2));
    assert!(!e.state.cards[gun.index()].in_play);
    assert!(ash_has(&e, 0, ".44 Magnum"));
}

/// Dragon's Breath Rounds: "'Before resolution' means after strikes have
/// been declared but before they resolve." [RTR 19990105] "Cannot be
/// played on an opponent's weapon." [LSJ 20020425] Not offered to A before
/// its gun's strike is chosen, nor on X's gun while A hand strikes.
#[test]
fn dragons_breath_rounds_is_loaded_only_on_the_users_gun_after_strikes() {
    let (mut e, a, x) = duel(&[], &[]);
    on(&mut e, ".44 Magnum", a);
    let theirs = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Dragon's Breath Rounds"]);
    fight(&mut e, a, x);
    never_offered_where(&mut e, SeatId(0), WindowLabel::CombatStrike, |o| {
        *o == play(0, 0, a)
    });
    let req = request(&mut e);
    e.choose(index_of(&req, Choice::HandStrike));
    pick_when(&mut e, weapon(theirs, 0));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Dragon's Breath Rounds: "Does not burn the gun if combat ends before the
/// strike resolves (eg. Rötschreck or if the opponent uses a 'strike:
/// combat ends')." [LSJ 19981006] [LSJ 20001127-2] X's Force of
/// Personality [pre] ends the combat: the loaded gun stays; Rötschreck
/// played on X, the loaded strike now an aggravated one: the gun stays too.
#[test]
fn dragons_breath_rounds_spares_the_gun_when_combat_ends_first() {
    let (mut e, a, x) = duel(&[], &["pre"]);
    let gun = on(&mut e, ".44 Magnum", a);
    hand(&mut e, 0, &["Dragon's Breath Rounds"]);
    hand(&mut e, 1, &["Force of Personality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[gun.index()].in_play, "combat ended first");
    assert_eq!(blood(&e, x), 4 - 1, "its card's blood, no damage");

    let (mut e, a, x) = duel(&[], &[]);
    let gun = on(&mut e, ".44 Magnum", a);
    hand(&mut e, 0, &["Dragon's Breath Rounds"]);
    hand(&mut e, 1, &["Rötschreck"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play(0, 0, a));
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(x),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        e.state.cards[gun.index()].in_play,
        "Rötschreck ended it first"
    );
    assert!(
        e.state.minions[x.index()].in_torpor,
        "X's Rötschreck torpor"
    );
}

/// Dragon's Breath Rounds dodged: its strike resolved, the gun burns
/// (rules-qa 84.2).
#[test]
fn dragons_breath_rounds_burns_the_gun_though_dodged() {
    let (mut e, a, x) = duel(&[], &["aus", "aus"]);
    let gun = on(&mut e, ".44 Magnum", a);
    hand(&mut e, 0, &["Dragon's Breath Rounds"]);
    hand(&mut e, 1, &["Read Intentions"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[gun.index()].in_play);
    assert_eq!(blood(&e, x), 4);
}

// ---- Force of Personality ---------------------------------------------------

/// Force of Personality [PRE]: "Vampires must burn 1 blood to attempt to
/// block this action. Non-zombie allies cannot block this action." X pays
/// 1 blood to attempt; the mortal ally is offered no attempt, the zombie
/// one attempts free. As announced, before other modifiers [ANK 20190425],
/// the action card replaced first [ANK 20181208]: see
/// `as_announced_modifiers_precede_other_modifiers` and
/// `an_as_announced_modifier_drawn_as_the_replacement_is_playable`.
#[test]
fn force_of_personality_taxes_vampires_and_bars_non_zombie_allies() {
    let (mut e, a, x) = duel(&["pre", "pre"], &[]);
    let mortal = e.add_ally(SeatId(1), find_library("Political Ally"));
    let zombie = e.add_ally(SeatId(1), find_library("Gravebound Drone"));
    hand(&mut e, 0, &["Force of Personality"]);
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 1, a));
    let req = offering(&mut e, block(x));
    assert!(req.options.contains(&block(zombie)));
    assert!(!req.options.contains(&block(mortal)));
    e.choose(index_of(&req, block(x)));
    assert_eq!(blood(&e, x), 4 - 1);
}

/// Force of Personality [pre]: "Strike: combat ends." No damage either way.
#[test]
fn force_of_personality_ends_combat() {
    let (mut e, a, x) = duel(&["pre"], &[]);
    hand(&mut e, 0, &["Force of Personality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4));
}

// ---- Haymaker ---------------------------------------------------------------

/// Haymaker: A's initial strike is a hand strike at +1, X's initial strike
/// first: X's 2 on A's 1 blood sends A to torpor first, A never striking;
/// at 4 blood A strikes for 2 after X's 1.
#[test]
fn haymaker_commits_a_hand_strike_and_gives_first_strike() {
    let (mut e, a, x) = duel(&[], &[]);
    e.state.minions[a.index()].blood = 1;
    e.state.minions[x.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    hand(&mut e, 0, &["Haymaker"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[a.index()].in_torpor);
    assert_eq!(blood(&e, x), 4, "A never struck");

    let (mut e, a, x) = duel(&[], &[]);
    hand(&mut e, 0, &["Haymaker"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4 - 1, 4 - 2));
}

/// Haymaker: "Not usable if this minion played a Haymaker last round."
/// Played in round 1 (its press continues the combat), not offered in
/// round 2 (A presses on with Relentless Pursuit), offered in round 3.
#[test]
fn haymaker_is_not_played_two_rounds_running() {
    let mut e = game(2);
    // Capacity 9 or more, none textless: Grimgroth's "+2 bleed" is
    // blocked, Viktor's "+1 stealth" reads on his own actions.
    let a = e.add_minion(SeatId(0), find_card("Grimgroth"), 9, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), find_card("Viktor, The Night General"), 9, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["pot"];
    hand(&mut e, 0, &["Haymaker", "Haymaker", "Relentless Pursuit"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, Choice::UsePress);
    never_offered_until(&mut e, SeatId(0), WindowLabel::CombatPress, play(0, 0, a));
    assert_eq!(e.combat().expect("round 2").round, 2);
    pick(&mut e, play(1, 0, a));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(e.combat().expect("round 3").round, 3);
    assert_eq!(req.window, WindowLabel::CombatBeforeStrikes);
}

/// Haymaker, ruled (owner, 2026-11-12): "Inflict means not dodged, but is
/// counted even if prevented. Parenthesis text is reminder. Pulled Fangs
/// and Disarm use a different 'successfully inflicted' wording that means
/// not prevented." A hits X for 2, all prevented (Armor of Vitality), X
/// hits A first for 1: A gets the press, X none.
#[test]
fn haymaker_press_counts_prevented_damage() {
    let setup = || {
        let (mut e, a, x) = duel(&[], &["for"]);
        hand(&mut e, 0, &["Haymaker"]);
        hand(&mut e, 1, &["Armor of Vitality"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, 0, a));
        pick_when(&mut e, play(0, 0, x));
        (e, x)
    };
    let (mut e, _) = setup();
    assert!(offered_press(&mut e, SeatId(0)));
    let (mut e, x) = setup();
    assert!(!offered_press(&mut e, SeatId(1)));
    assert_eq!(blood(&e, x), 4 - 1, "the Armor's blood only");
}

/// Haymaker, prevented or not (rules-qa 116.44), with Rolling with the
/// Punches [FOR] ("prevent all damage from the opposing minion's strikes
/// this round"): X (strength 2) hits A first for 2, A's Haymaker hits X
/// for 2, X prevents it and the rest of the round; A's Blur [cel] strike
/// for 1, prevented too, still counts: 3 to 2, A gets the press.
#[test]
fn haymaker_counts_strikes_prevented_for_the_round() {
    let (mut e, a, x) = duel(&["cel"], &["for"]);
    e.state.minions[x.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    hand(&mut e, 0, &["Haymaker", "Blur"]);
    hand(&mut e, 1, &["Rolling with the Punches"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, play(0, 0, a));
    assert!(offered_press(&mut e, SeatId(0)));
    assert_eq!(blood(&e, x), 4 - 1, "Rolling with the Punches' blood only");
}

/// Haymaker: "Additional damage inherits all of the properties of the base
/// damage." [TOM 19960225] With aggravated hands, its hand strike's 1 + 1
/// aggravated, pending on X.
#[test]
fn haymaker_adds_to_an_aggravated_hand_strike() {
    let (mut e, a, x) = duel(&[], &["for"]);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    hand(&mut e, 0, &["Haymaker"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    offering(&mut e, play(0, 0, x));
    let pending = e.combat().expect("combat").sides[1].pending;
    assert_eq!((pending.normal, pending.aggravated), (0, 2));
}

/// Haymaker's "inflicts" counts damage the struck minion is immune to
/// (rules-qa 116.44, 116.138): Mictlantecuhtli's Haymaker hits X for 3; X
/// (strength 3) strikes first with a Baseball Bat for 4, none landing on
/// him: X gets the press, he none. Tye Cooper, immune to non-aggravated
/// damage, takes none of A's 2 and deals 0: A gets the press.
#[test]
fn haymaker_counts_damage_on_an_immune_minion() {
    let setup = || {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Mictlantecuhtli"), 6, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
        e.state.minions[x.index()].timed.push((
            zvtes::cards::Lasting::Strength(2),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(1),
        ));
        let bat = on(&mut e, "Baseball Bat", x);
        hand(&mut e, 0, &["Haymaker"]);
        fight(&mut e, m, x);
        pick_when(&mut e, play(0, 0, m));
        pick_when(&mut e, weapon(bat, 0));
        (e, m, x)
    };
    let (mut e, ..) = setup();
    assert!(offered_press(&mut e, SeatId(1)));
    let (mut e, m, x) = setup();
    assert!(!offered_press(&mut e, SeatId(0)));
    assert_eq!((blood(&e, m), blood(&e, x)), (6, 4 - 3));

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    let t = e.add_ally(SeatId(1), find_library("Tye Cooper"));
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    hand(&mut e, 0, &["Haymaker"]);
    fight(&mut e, a, t);
    pick_when(&mut e, play(0, 0, a));
    assert!(offered_press(&mut e, SeatId(0)));
    assert!(e.state.minions[t.index()].in_play, "his life kept");
}

// ---- High Ground ------------------------------------------------------------

/// High Ground: "When setting the range for the round, no other effect can
/// be used to reset the range. Skip the determine range step this round,
/// but other pre-range effects can be played afterwards." [RTR 19970630]
/// [ANK 20180720] A, with flight, against X without: long range, no
/// determine-range step (X's Backstep never offered), X's Aura Reading
/// still played before range.
#[test]
fn high_ground_sets_long_range_for_a_flier() {
    let (mut e, a, x) = duel(&["flight"], &["aus"]);
    hand(&mut e, 0, &["High Ground"]);
    hand(&mut e, 1, &["Aura Reading", "Backstep"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 0, x));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { card: 0, actor: Some(m), .. } if *m == x),
    );
    assert_eq!((blood(&e, a), blood(&e, x)), (4, 4), "no strike reached");
}

/// High Ground: "If another effect was already setting that round's range
/// (e.g. Neutral Guard), that effect has priority." [PIB 20120214] X's
/// Cold Aura [nec] first: High Ground's range set not offered after it.
#[test]
fn high_ground_yields_to_an_earlier_range_setter() {
    let (mut e, a, x) = duel(&["flight"], &["nec"]);
    hand(&mut e, 0, &["High Ground"]);
    hand(&mut e, 1, &["Cold Aura"]);
    fight(&mut e, a, x);
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    assert!(req.options.contains(&play(0, 1, a)));
    e.choose(index_of(&req, Choice::Pass));
    pick(&mut e, play(0, 0, x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
}

/// High Ground without flight: no range set; its maneuver only to long
/// range, one High Ground a round: after X's Flash back to close, A's
/// second copy is not offered.
#[test]
fn high_ground_maneuvers_only_to_long() {
    let (mut e, a, x) = duel(&["cel"], &["cel"]);
    hand(&mut e, 0, &["High Ground", "High Ground", "Flash"]);
    hand(&mut e, 1, &["Flash"]);
    fight(&mut e, a, x);
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert!(!req.options.contains(&play(0, 1, a)));
    e.choose(index_of(&req, play(0, 0, a)));
    pick(&mut e, play(0, 0, x));
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatDetermineRange);
    assert_eq!(
        e.combat().expect("combat").range,
        zvtes::combat::Range::Close
    );
    assert!(req.options.contains(&play(1, 0, a)), "its Flash");
    assert!(!req.options.contains(&play(0, 0, a)), "{:?}", req.options);
}

// ---- Read Intentions --------------------------------------------------------

/// Read Intentions [aus]: "Press, only usable to end combat." Not offered
/// with no press to continue standing; after A's press to continue, X's
/// ends the combat: no round 2.
#[test]
fn read_intentions_presses_only_to_end() {
    let (mut e, a, x) = duel(&["pot"], &["aus"]);
    hand(&mut e, 0, &["Relentless Pursuit"]);
    hand(&mut e, 1, &["Read Intentions"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    pick(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, a), blood(&e, x)),
        (4 - 1 - 1, 4 - 1),
        "one round"
    );

    let (mut e, a, x) = duel(&[], &["aus"]);
    hand(&mut e, 1, &["Read Intentions"]);
    fight(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
}

// ---- Relentless Pursuit -----------------------------------------------------

/// Relentless Pursuit [POT]: "Press, and if another round of combat starts,
/// you get +2 hand size for the remainder of combat." Continued: +2 in
/// round 2, gone after the combat; as X's press to continue ends the
/// combat instead, no hand size.
#[test]
fn relentless_pursuit_grows_the_hand_if_combat_goes_on() {
    let (mut e, a, x) = duel(&["pot", "pot"], &[]);
    let base = e.state.hand_size(SeatId(0));
    hand(&mut e, 0, &["Relentless Pursuit", "Brute Force"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    expect(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert_eq!(e.combat().expect("round 2").round, 2);
    assert_eq!(e.state.hand_size(SeatId(0)), base + 2);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.hand_size(SeatId(0)), base);

    let (mut e, a, x) = duel(&["pot", "pot"], &["pot"]);
    let base = e.state.hand_size(SeatId(0));
    hand(&mut e, 0, &["Relentless Pursuit"]);
    hand(&mut e, 1, &["Relentless Pursuit"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 1, a));
    assert_eq!(req.window, WindowLabel::CombatPress);
    e.choose(index_of(&req, Choice::Pass));
    pick(&mut e, play(0, 0, x));
    pick(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.hand_size(SeatId(0)), base);
    assert_eq!(blood(&e, x), 4 - 1 - 1, "one round");
}

/// Relentless Pursuit [POT], two copies (rules-digest D4) and the excess
/// discarded as the combat ends (rulebook "Drawing cards": "immediately
/// discard down to or draw up to match your hand size"; rules-qa 54.8): a
/// full hand of 7, +2 in round 2 and +2 more in round 3, drawn up to 11;
/// back to 7 as the combat ends, the 4 discarded.
#[test]
fn relentless_pursuits_stack_and_their_excess_is_discarded() {
    let (mut e, a, x) = duel(&["pot", "pot"], &[]);
    // A strike choice each round: a request in every strike step.
    on(&mut e, "Baseball Bat", a);
    let base = e.state.hand_size(SeatId(0));
    hand(
        &mut e,
        0,
        &[
            "Relentless Pursuit",
            "Relentless Pursuit",
            "Leverage",
            "Leverage",
            "Leverage",
            "Leverage",
            "Leverage",
        ],
    );
    stock(&mut e, 0, "Deflection", 10);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert_eq!(e.combat().expect("round 2").round, 2);
    assert_eq!(e.state.seats[0].hand.len(), base + 2);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert_eq!(e.combat().expect("round 3").round, 3);
    assert_eq!(e.state.hand_size(SeatId(0)), base + 4);
    assert_eq!(e.state.seats[0].hand.len(), base + 4, "drawn up");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].hand.len(), base, "discarded down");
    assert_eq!(
        e.state.seats[0].ash.len(),
        2 + 4,
        "the two cards and the excess"
    );
}

// ---- Shape Mastery ----------------------------------------------------------

/// An anarch textless vampire of capacity 4 to 8, the `nth`.
#[track_caller]
fn anarch(nth: usize) -> u16 {
    textless(
        |c| c.sect == Some(zvtes::crypt::Sect::Anarch) && (4..=8).contains(&c.capacity),
        nth,
    )
}

/// Shape Mastery [pro]: "Cancel a combat card requiring … Protean [pro] …
/// as it is played … The opposing minion cannot play that card again this
/// action." "The canceled card has still been played. … Any card which
/// text prohibits to play more than once cannot be played again." [ANK
/// 20190104] [LSJ 19980212] X's Donnybrook [pro] canceled; choosing its
/// strike again, X is offered no Donnybrook (its second copy) and hand
/// strikes.
#[test]
fn shape_mastery_cancels_and_bars_the_card_this_action() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(0), 4, false);
    e.add_minion(SeatId(0), anarch(1), 4, false);
    let x = e.add_minion(SeatId(1), anarch(2), 4, false);
    e.add_minion(SeatId(1), anarch(3), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["pro"];
    e.state.minions[x.index()].gained_levels = vec!["pro"];
    hand(&mut e, 0, &["Shape Mastery"]);
    hand(&mut e, 1, &["Donnybrook", "Donnybrook"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    pick(&mut e, play(0, 0, a));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == x),
    );
    assert!(ash_has(&e, 1, "Donnybrook"));
    assert_eq!(e.state.seats[1].hand.len(), 1, "the second copy kept");
    assert_eq!(blood(&e, a), 4 - 1, "X's hand strike instead");
}

/// Shape Mastery: "requiring" Protean is the Discipline the card was played
/// with (PIB 20130704's reading): X's Donnybrook played at [ser] is not
/// offered to it.
#[test]
fn shape_mastery_ignores_a_card_played_with_another_discipline() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(0), 4, false);
    e.add_minion(SeatId(0), anarch(1), 4, false);
    let x = e.add_minion(SeatId(1), anarch(2), 4, false);
    e.add_minion(SeatId(1), anarch(3), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["pro"];
    e.state.minions[x.index()].gained_levels = vec!["pro", "ser"];
    hand(&mut e, 0, &["Shape Mastery"]);
    hand(&mut e, 1, &["Donnybrook"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, a));
}

/// Shape Mastery [PRO]: "As above, but cancel an action modifier card
/// requiring any of those Disciplines": A's Invigorate [pro] canceled by
/// X's reaction, its "aggravated hands" gone.
#[test]
fn shape_mastery_superior_cancels_an_action_modifier() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(0), 4, false);
    e.add_minion(SeatId(0), anarch(1), 4, false);
    let x = e.add_minion(SeatId(1), anarch(2), 4, false);
    e.add_minion(SeatId(1), anarch(3), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["pro"];
    e.state.minions[x.index()].gained_levels = vec!["pro", "pro"];
    hand(&mut e, 0, &["Invigorate"]);
    hand(&mut e, 1, &["Shape Mastery"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 2, a));
    pick(&mut e, play(0, 1, x));
    request(&mut e);
    assert!(ash_has(&e, 0, "Invigorate"));
    assert!(!e.action().expect("the bleed").aggravated_hands);
}

/// Groundfighting "can cancel … Shape Mastery (when the latter is used to
/// cancel a strike)." [LSJ 20050224-2] A's Shape Mastery on X's Donnybrook
/// [pro] canceled by X's Groundfighting: the aggravated hand strike lands,
/// A to torpor.
#[test]
fn groundfighting_cancels_shape_mastery_canceling_its_strike() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), anarch(0), 4, false);
    e.add_minion(SeatId(0), anarch(1), 4, false);
    let x = e.add_minion(SeatId(1), anarch(2), 4, false);
    e.add_minion(SeatId(1), anarch(3), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["pro"];
    e.state.minions[x.index()].gained_levels = vec!["pro"];
    hand(&mut e, 0, &["Shape Mastery"]);
    hand(&mut e, 1, &["Groundfighting", "Donnybrook"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(1, 1, x));
    pick(&mut e, play(0, 0, a));
    pick(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Shape Mastery"));
    assert!(
        e.state.minions[a.index()].in_torpor,
        "the aggravated strike landed"
    );
}

// ---- Still the Mortal Flesh -------------------------------------------------

/// Still the Mortal Flesh [DOM]: "Strike: burn the opposing ally." The
/// blocking mortal ally burned; against a younger vampire not chosen.
#[test]
fn still_the_mortal_flesh_burns_an_opposing_ally() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(6, 7, 0), 6, false);
    e.add_minion(SeatId(0), in_caps(6, 7, 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.add_minion(SeatId(1), in_caps(4, 5, 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dom", "dom"];
    hand(&mut e, 0, &["Still the Mortal Flesh"]);
    fight(&mut e, a, ally);
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ally.index()].in_play);
    assert!(ash_has(&e, 1, "Political Ally"));

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(6, 7, 0), 6, false);
    e.add_minion(SeatId(0), in_caps(6, 7, 1), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 5, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 5, 1), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dom", "dom"];
    hand(&mut e, 0, &["Still the Mortal Flesh"]);
    fight(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, a));
}

/// Still the Mortal Flesh [dom]: "Only usable in combat with an ally or
/// younger vampire. Not usable against a mummy, wraith, or zombie." Its
/// maneuver offered against a younger vampire, not an older one, nor a
/// zombie ally.
#[test]
fn still_the_mortal_flesh_maneuvers_against_the_young_only() {
    for (x_cap, zombie, offered) in [
        ((4, 5), false, true),
        ((7, 7), false, false),
        ((4, 5), true, false),
    ] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), in_caps(6, 6, 0), 6, false);
        e.add_minion(SeatId(0), in_caps(6, 6, 1), 4, false);
        let x = if zombie {
            e.add_ally(SeatId(1), find_library("Gravebound Drone"))
        } else {
            e.add_minion(SeatId(1), in_caps(x_cap.0, x_cap.1, 0), 4, false)
        };
        e.add_minion(SeatId(1), in_caps(x_cap.0, x_cap.1, 1), 4, false);
        e.state.minions[a.index()].gained_levels = vec!["dom"];
        hand(&mut e, 0, &["Still the Mortal Flesh"]);
        fight(&mut e, a, x);
        assert_eq!(
            offered_before(&mut e, play(0, 0, a), SeatId(0), WindowLabel::MinionPhase),
            offered,
            "{x_cap:?} {zombie}"
        );
    }
}

// ---- Stutter-Step -----------------------------------------------------------

/// Stutter-Step [CEL]: "The dodge part protects from the opponent's strike
/// (even if done with First Strike), the hand strike part inflicts damage
/// during the normal strike resolution." [ANK 20180913-1] Against X's hand
/// strike, then X's Anesthetic Touch [OBE] with first strike: A untouched,
/// X takes A's 1 each time.
#[test]
fn stutter_step_dodges_and_strikes_by_hand() {
    for first in [false, true] {
        let (mut e, a, x) = duel(&["cel", "cel"], &["obe", "obe"]);
        hand(&mut e, 0, &["Stutter-Step"]);
        hand(&mut e, 1, &["Anesthetic Touch"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, 1, a));
        let req = expect(&mut e, SeatId(1), WindowLabel::CombatStrike);
        let x_strike = if first {
            play(0, 2, x)
        } else {
            Choice::HandStrike
        };
        e.choose(index_of(&req, x_strike));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!((blood(&e, a), blood(&e, x)), (4, 4 - 1), "{first}");
    }
}

/// Stutter-Step: "It is both a hand strike and a dodge. It cannot be used
/// if only hand strikes or only dodges are allowed." [LSJ 20010919] Under
/// X's Immortal Grapple neither [CEL] nor [cel]'s dodge is offered; at long
/// range (X's Backstep) [CEL] is not either.
#[test]
fn stutter_step_superior_needs_both_strikes() {
    let (mut e, a, x) = duel(&["cel", "cel"], &["pot"]);
    hand(&mut e, 0, &["Stutter-Step"]);
    hand(&mut e, 1, &["Immortal Grapple"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a),
    );

    let (mut e, a, x) = duel(&["cel", "cel"], &[]);
    hand(&mut e, 0, &["Stutter-Step"]);
    hand(&mut e, 1, &["Backstep"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&play(0, 0, a)));
    assert!(!req.options.contains(&play(0, 1, a)));
}

/// Stutter-Step: "The affected minion cannot play a card that provides an
/// additional strike, even if just to benefit from another effect (e.g.
/// Acrobatics)." [LSJ 20011214-5] After [CEL], A's Quickness is not
/// offered that round; in round 2 (A's Relentless Pursuit) it is.
#[test]
fn stutter_step_bars_additional_strikes_this_round() {
    let (mut e, a, x) = duel(&["cel", "cel", "pot"], &[]);
    hand(
        &mut e,
        0,
        &["Stutter-Step", "Quickness", "Relentless Pursuit"],
    );
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert_eq!(e.combat().expect("combat").round, 1);
    e.choose(index_of(&req, play(1, 0, a)));
    let req = offering(&mut e, play(0, 0, a));
    assert_eq!(req.window, WindowLabel::CombatAdditionalStrikes);
    assert_eq!(e.combat().expect("combat").round, 2);
}

/// Stutter-Step [CEL]: "the hand strike part inflicts damage during the
/// normal strike resolution" [ANK 20180913-1]: a hand-strike improver
/// applies to it (aggravated hands): X's pending damage 1 aggravated, A's
/// hand strike dodged.
#[test]
fn stutter_step_superior_hand_strike_takes_hand_strike_improvers() {
    let (mut e, a, x) = duel(&["cel", "cel"], &["for"]);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    hand(&mut e, 0, &["Stutter-Step"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 1));
    assert_eq!(pending(&e, 0), (0, 0), "X's strike dodged");
}

/// Stutter-Step [CEL]: "Not usable as an additional strike." A's
/// Quickness after the strikes: [cel]'s dodge is offered as its additional
/// strike, [CEL] not.
#[test]
fn stutter_step_superior_is_no_additional_strike() {
    let (mut e, a, x) = duel(&["cel", "cel"], &[]);
    hand(&mut e, 0, &["Quickness", "Stutter-Step"]);
    fight(&mut e, a, x);
    pick_when(&mut e, Choice::HandStrike);
    pick_when(&mut e, play(0, 0, a));
    let i = at(&e, 0, "Stutter-Step");
    let req = offering(&mut e, play(i, 0, a));
    assert_eq!(req.seat, SeatId(0));
    assert!(!req.options.contains(&play(i, 1, a)));
}

// ---- Target Retainer --------------------------------------------------------

/// A retainer with `life` on `host`.
fn retainer(e: &mut Engine, name: &str, host: MinionId, life: u8) -> CardId {
    let r = on(e, name, host);
    e.state.cards[r.index()].counters = life;
    r
}

/// Target Retainer: "You can target a retainer on the opposing minion with
/// this strike (instead of the opposing minion)." A's hand strike aimed at
/// X's Twisted Bloodhound: the hound loses 1 life, X untouched.
#[test]
fn target_retainer_strikes_a_retainer_at_close_range() {
    let (mut e, a, x) = duel(&[], &[]);
    let hound = retainer(&mut e, "Twisted Bloodhound", x, 3);
    hand(&mut e, 0, &["Target Retainer"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play_on(0, 0, a, Target::Card(hound)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[hound.index()].counters, 2);
    assert_eq!(blood(&e, x), 4);
}

/// Target Retainer: "if any damage from this strike is successfully
/// inflicted, this strike inflicts +1 damage." Of the strike's kind [RTR
/// 19970630] [TOM 19960225]: with aggravated hands, 2 aggravated pending on
/// X; "Adding damage to a strike that does not deal damage has no effect"
/// [RTR 19960221]: X dodging, X untouched.
#[test]
fn target_retainer_adds_one_to_a_landing_strike() {
    let (mut e, a, x) = duel(&[], &["for"]);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    hand(&mut e, 0, &["Target Retainer"]);
    hand(&mut e, 1, &["Armor of Vitality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    offering(&mut e, play(0, 0, x));
    let pending = e.combat().expect("combat").sides[1].pending;
    assert_eq!((pending.normal, pending.aggravated), (0, 2));

    let (mut e, a, x) = duel(&[], &["aus", "aus"]);
    hand(&mut e, 0, &["Target Retainer"]);
    hand(&mut e, 1, &["Read Intentions"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4);
}

/// Target Retainer canceled for two combat cards: "If it is canceled, the
/// strike resolves with the default target." [ANK 20200311] "If it is
/// canceled, any blood cost (eg. Terror Frenzy) has to be paid." [PIB
/// 20121031] Under X's Terror Frenzy [ANI], A pays 1 and its strike hits X
/// for 1.
#[test]
fn target_retainer_canceled_strikes_the_minion_its_cost_paid() {
    let (mut e, a, x) = duel(&[], &["ani", "ani"]);
    let hound = retainer(&mut e, "Twisted Bloodhound", x, 3);
    hand(&mut e, 0, &["Target Retainer"]);
    hand(
        &mut e,
        1,
        &["Terror Frenzy", "Side Strike", "Glancing Blow"],
    );
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    pick_when(&mut e, play_on(0, 0, a, Target::Card(hound)));
    pick_when(&mut e, Choice::PayToCancel);
    request(&mut e);
    assert_eq!(e.state.seats[1].hand.len(), 0, "the two combat cards");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[hound.index()].counters, 3);
    assert_eq!(blood(&e, x), 4 - 1 - 1, "its Terror Frenzy and A's strike");
    assert_eq!(blood(&e, a), 4 - 1 - 1, "its cost and X's strike");
}

/// Target Retainer: "Is played during the Strike phase even if you commit to
/// a strike in the Determine Range phase (eg. Thrown Gate). Cannot be played
/// if the strike cannot be chosen anymore during the Strike phase (eg.
/// Immortal Grapple)." [LSJ 20080409] X's .44 Magnum, committed by its
/// maneuver, takes the aim (2R + 1); barred by A's grapple, no aim.
#[test]
fn target_retainer_aims_a_committed_strike_unless_a_grapple_bars_it() {
    let (mut e, a, x) = duel(&[], &[]);
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 1, &["Target Retainer"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 1));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 4 - 3, "2R and the aim's 1");

    let (mut e, a, x) = duel(&["pot"], &[]);
    let a_gun = on(&mut e, ".44 Magnum", a);
    let x_gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Immortal Grapple"]);
    hand(&mut e, 1, &["Target Retainer"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(a_gun, 1));
    pick_when(&mut e, weapon(x_gun, 1));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 1, x));
    assert_eq!(e.state.seats[1].hand.len(), 1, "no aim played");
}

/// Lucian, the Perfect: "Strikes that target cards on him, are not affected
/// (eg. Target Retainer, Canine Horde)." [LSJ 20090528-2] X's hand strike
/// aimed at Lucian's retainer pays no toll.
#[test]
fn target_retainer_at_lucians_retainer_pays_no_toll() {
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lucian, the Perfect"), 6, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let strength = e.state.base_strength(l) as i8;
    e.state.minions[l.index()].timed.push((
        zvtes::cards::Lasting::Strength(-strength),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    let hound = retainer(&mut e, "Twisted Bloodhound", l, 3);
    hand(&mut e, 1, &["Target Retainer"]);
    fight(&mut e, l, x);
    pick_when(&mut e, play_on(0, 0, x, Target::Card(hound)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[hound.index()].counters, 2);
    assert_eq!(blood(&e, x), 4, "no toll");
}

// ---- Undying Tenacity -------------------------------------------------------

/// Undying Tenacity [for]: "This vampire will not go to torpor until after
/// combat ends." X, at 0 blood, hit: ready through the press step (A's
/// Relentless Pursuit still offered against a ready X), in torpor after.
#[test]
fn undying_tenacity_holds_torpor_until_the_combat_ends() {
    let (mut e, a, x) = duel(&["pot"], &["for"]);
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Relentless Pursuit"]);
    hand(&mut e, 1, &["Undying Tenacity"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    expect(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(!e.state.minions[x.index()].in_torpor);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
}

/// Undying Tenacity and the strikes sending a vampire to torpor: "The
/// opponent is wounded (if they play Undying Tenacity, they go to torpor
/// after combat)." [LSJ 20090622-2] Coma [dem], Entombment [OBT], Touch of
/// Oblivion [OBL]: X stays ready to the combat's end, then torpor.
#[test]
fn undying_tenacity_defers_the_torpor_a_strike_sends() {
    for (card, mode, levels) in [
        ("Coma", 0, &["dem"][..]),
        ("Entombment", 1, &["obt", "obt"][..]),
        ("Touch of Oblivion", 1, &["obl", "obl"][..]),
    ] {
        // Room for Coma's 3 blood and the press's 1.
        let mut e = game(2);
        // Grimgroth (10): his "+2 bleed" is blocked.
        let a = e.add_minion(SeatId(0), find_card("Grimgroth"), 8, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 2), 4, false);
        let x = e.add_minion(SeatId(1), in_caps(4, 8, 0), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        e.state.minions[a.index()].gained_levels = levels.to_vec();
        e.state.minions[a.index()].gained_levels.push("pot");
        e.state.minions[x.index()].gained_levels = vec!["for"];
        hand(&mut e, 0, &[card, "Relentless Pursuit"]);
        hand(&mut e, 1, &["Undying Tenacity"]);
        fight(&mut e, a, x);
        pick_when(&mut e, play(0, mode, a));
        pick_when(&mut e, play(0, 0, x));
        expect(&mut e, SeatId(0), WindowLabel::CombatPress);
        assert!(!e.state.minions[x.index()].in_torpor, "{card}");
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(e.state.minions[x.index()].in_torpor, "{card}");
    }
}

/// Seren Sukardi, his strike sending a vampire to torpor: "that vampire is
/// wounded (if they play Undying Tenacity, they go to torpor after
/// combat)." [LSJ 20090622-2] Round 1 at long range (X's Backstep), A
/// presses on; round 2 Seren's strike, X's Undying Tenacity.
#[test]
fn undying_tenacity_defers_serens_torpor() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), find_card("Seren Sukardi"), 6, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["pot"];
    e.state.minions[x.index()].gained_levels = vec!["for"];
    hand(&mut e, 0, &["Relentless Pursuit", "Relentless Pursuit"]);
    hand(&mut e, 1, &["Backstep", "Undying Tenacity"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play(0, 0, a));
    let seren = Choice::UseAbility {
        source: AbilitySource::Vampire(a),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    let req = offering(&mut e, seren);
    assert_eq!(e.combat().expect("round 2").round, 2);
    e.choose(index_of(&req, seren));
    pick_when(&mut e, play(0, 0, x));
    expect(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(!e.state.minions[x.index()].in_torpor);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
}

/// "If played first, effects that stop a vampire from going to torpor
/// would prevent the use of effects that are only usable on a vampire
/// going to torpor." [RTR 20010711] A's Decapitate [POT] first: X burned,
/// its Undying Tenacity never offered.
#[test]
fn decapitate_played_first_bars_undying_tenacity() {
    let (mut e, a, x) = duel(&["pot", "pot"], &["for"]);
    e.state.minions[x.index()].blood = 0;
    hand(&mut e, 0, &["Decapitate"]);
    hand(&mut e, 1, &["Undying Tenacity"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 2, a));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
    assert!(!e.state.minions[x.index()].in_play, "burned");
}

/// Undying Tenacity [FOR]: "Press or prevent 1 damage." In prevention, A's
/// 1 prevented; in the press step, a press to continue: round 2.
#[test]
fn undying_tenacity_superior_presses_or_prevents() {
    let (mut e, a, x) = duel(&[], &["for", "for"]);
    hand(&mut e, 1, &["Undying Tenacity"; 3]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    let req = offering(&mut e, play(0, 1, x));
    assert_eq!(req.window, WindowLabel::CombatPress);
    assert_eq!(blood(&e, x), 4, "the 1 prevented");
    e.choose(index_of(&req, play(0, 1, x)));
    offering(&mut e, play(0, 1, x));
    assert_eq!(e.combat().expect("round 2").round, 2);
}

// ---- Gun label ---------------------------------------------------------------

/// "Weapon, gun." on the type line is a gun (the generator's Gun label):
/// Desert Eagle loads Dragon's Breath Rounds.
#[test]
fn a_weapon_comma_gun_is_a_gun() {
    let (mut e, a, x) = duel(&[], &[]);
    let gun = on(&mut e, "Desert Eagle", a);
    hand(&mut e, 0, &["Dragon's Breath Rounds"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play(0, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !e.state.cards[gun.index()].in_play,
        "burned after its strike"
    );
}

// ---- The Oath ---------------------------------------------------------------

/// The Oath's lasting Ⓓ action on `giver` (as its play leaves it) and `n`
/// oath counters from it on `sworn`.
fn sworn(e: &mut Engine, giver: MinionId, sworn: MinionId, n: u8) -> CardId {
    let owner = e.state.minions[giver.index()].owner;
    let mut card = CardInPlay::new(find_library("Oath, The"), owner, Some(giver), 0);
    card.effect_only = true;
    e.state.cards.push(card);
    for _ in 0..n {
        e.state.minions[sworn.index()].add_oath(giver);
    }
    CardId(e.state.cards.len() as u32 - 1)
}

/// The Oath: "Play when a vampire … steals blood from this vampire. Put an
/// oath counter on that vampire." X's Theft of Vitae [tha] steals from A:
/// A's Oath puts a counter from A on X, the card to the ash heap and its
/// Ⓓ action left on A (an effect-only card).
#[test]
fn the_oath_answers_a_strike_steal() {
    let (mut e, a, x) = duel(&[], &["tha"]);
    hand(&mut e, 0, &["Oath, The"]);
    hand(&mut e, 1, &["Theft of Vitae"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play_on(0, 0, a, Target::Minion(x)));
    request(&mut e);
    assert_eq!(e.state.minions[x.index()].oaths_from(a), 1);
    assert!(ash_has(&e, 0, "Oath, The"));
    let oath = find_library("Oath, The");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.card == oath && c.in_play && c.effect_only && c.host == Some(a)));
}

/// The Oath: "Play when a vampire gains blood from a Taste of Vitae": X's
/// Taste of Vitae at the end of the round, on the blood A lost to its
/// strike: A's Oath at once.
#[test]
fn the_oath_answers_a_taste_of_vitae() {
    let (mut e, a, x) = duel(&[], &[]);
    hand(&mut e, 0, &["Oath, The"]);
    hand(&mut e, 1, &["Taste of Vitae"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play_on(0, 0, a, Target::Minion(x)));
    request(&mut e);
    assert_eq!(e.state.minions[x.index()].oaths_from(a), 1);
}

/// The Oath: "A vampire can play only one Oath each round." A holds two;
/// X's Theft of Vitae answered by the first, the second never offered in
/// the round though X still took A's blood.
#[test]
fn the_oath_is_played_once_a_round() {
    let (mut e, a, x) = duel(&[], &["tha"]);
    hand(&mut e, 0, &["Oath, The", "Oath, The"]);
    hand(&mut e, 1, &["Theft of Vitae"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, x));
    pick_when(&mut e, play_on(0, 0, a, Target::Minion(x)));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play_on(0, 0, a, Target::Minion(x)),
    );
    assert_eq!(e.state.minions[x.index()].oaths_from(a), 1);
}

/// The Oath: "This vampire may put another oath counter on that vampire as
/// a Ⓓ action." A's action at X, unblocked: 2 counters.
#[test]
fn the_oaths_action_adds_the_second_counter() {
    let (mut e, a, x) = duel(&[], &[]);
    let oath = sworn(&mut e, a, x, 1);
    let act = Choice::UseAbility {
        source: AbilitySource::Card(oath),
        ability: 0,
        actor: Some(a),
        target: Target::Minion(x),
    };
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&act), "{:?}", req.options);
    e.choose(index_of(&req, act));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[x.index()].oaths_from(a), 2);
}

/// The Oath: "A vampire with 2 oath counters from this vampire cannot
/// block or enter combat with this vampire." No combat between them in any
/// way (owner, 2026-11-12; LSJ 20090210: "entering combat is prohibited",
/// "A enters combat with B === B enters combat with A"): X never offered a
/// block of A's bleed (its other vampire is); neither's Bum's Rush at the
/// other is offered.
#[test]
fn a_sworn_vampire_cannot_block_its_giver() {
    let (mut e, a, x) = duel(&[], &[]);
    sworn(&mut e, a, x, 2);
    pick(&mut e, bleed(a));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(!req.options.contains(&block(x)));
    assert!(req.options.contains(&block(MinionId(3))));

    for (giver, on) in [(MinionId(0), MinionId(2)), (MinionId(2), MinionId(0))] {
        let (mut e, a, x) = duel(&[], &[]);
        sworn(&mut e, giver, on, 2);
        hand(&mut e, 0, &["Bum's Rush"]);
        let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!req.options.contains(&play_on(0, 0, a, Target::Minion(x))));
        assert!(req
            .options
            .contains(&play_on(0, 0, a, Target::Minion(MinionId(3)))));
    }
}

/// The Oath: the giver blocking its sworn vampire's action — "the action
/// ends with no combat": "Correct." (LSJ 20090210). A, sworn to X, bleeds;
/// X blocks: the bleed fails, no blood lost on either side.
#[test]
fn the_oaths_giver_blocks_its_sworn_vampire_with_no_combat() {
    let (mut e, a, x) = duel(&[], &[]);
    sworn(&mut e, x, a, 2);
    let pool = e.state.seats[1].pool;
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.combat().is_none());
    assert_eq!((blood(&e, a), blood(&e, x)), (4, 4));
    assert_eq!(e.state.seats[1].pool, pool, "the bleed failed");
    assert!(e.state.minions[x.index()].locked);
}

/// The Oath: a third party's effect cannot put them in combat ("Taunt the
/// Caged Beast, Blissful Agony … No.": LSJ 20090210). Seat 0's Taunt the
/// Caged Beast [ANI] at its prey's P: its predator's Q, sworn to P, is not
/// offered, its other vampires are.
#[test]
fn taunt_the_caged_beast_cannot_pit_a_sworn_pair() {
    let mut e = game(3);
    let a = e.add_minion(
        SeatId(0),
        textless(|c| c.disciplines.contains(&"ani"), 0),
        4,
        false,
    );
    superior(&mut e, a, "ani");
    let p = e.add_minion(SeatId(1), textless(|c| c.capacity >= 4, 1), 4, false);
    let q = e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 2), 4, false);
    let r = e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 3), 4, false);
    e.add_minion(SeatId(2), textless(|c| c.capacity >= 4, 4), 4, false);
    sworn(&mut e, p, q, 2);
    hand(&mut e, 0, &["Taunt the Caged Beast"]);
    pick(&mut e, play_on(0, 1, a, Target::Minion(p)));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ChooseMinion(q)), "{req:?}");
    assert!(req.options.contains(&Choice::ChooseMinion(r)), "{req:?}");
}

/// Dragon's Breath Rounds' aggravated points are gun-strike damage: X's
/// Kevlar Vest prevents 2 of them with its gun branch, the gun's normal 2
/// left pending (rules-qa 10.3: "damage from gun strikes").
#[test]
fn kevlar_vest_prevents_dragons_breath_aggravated_damage() {
    let (mut e, a, x) = duel(&[], &[]);
    let gun = on(&mut e, ".44 Magnum", a);
    let vest = on(&mut e, "Kevlar Vest", x);
    hand(&mut e, 0, &["Dragon's Breath Rounds"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play(0, 0, a));
    let use_vest = Choice::UseAbility {
        source: AbilitySource::Card(vest),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, use_vest);
    pick_when(&mut e, use_vest);
    let pending = e.combat().expect("combat").sides[1].pending;
    assert_eq!((pending.normal, pending.aggravated), (2, 0));
}

// ---- Reactions and action modifiers -----------------------------------------

/// The hand index of `name` in `seat`'s hand.
fn at(e: &Engine, seat: usize, name: &str) -> u8 {
    e.state.seats[seat]
        .hand
        .iter()
        .position(|&c| c == find_library(name))
        .unwrap_or_else(|| panic!("no {name} in hand")) as u8
}

/// The card `name` in play on `host`, the card itself (not an effect-only
/// stand-in).
fn card_on(e: &Engine, name: &str, host: MinionId) -> Option<CardId> {
    e.state
        .cards
        .iter()
        .position(|c| {
            c.in_play && !c.effect_only && c.host == Some(host) && c.card == find_library(name)
        })
        .map(|i| CardId(i as u32))
}

/// A textless untitled vampire of `sect`, capacity 4 to 8, the `nth`.
#[track_caller]
fn of_sect(sect: zvtes::crypt::Sect, nth: usize) -> u16 {
    textless(
        |c| c.sect == Some(sect) && (4..=8).contains(&c.capacity),
        nth,
    )
}

/// Seat 0's A (capacity 2 or 3, 2 blood) and a second vampire, seat 1's X
/// (capacity 6 to 8, older) and a second one, 4 blood, X with `x_levels`.
fn young_bleeder(x_levels: &[&'static str]) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(2, 3, 0), 2, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(6, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.state.minions[x.index()].gained_levels = x_levels.to_vec();
    (e, a, x)
}

// ---- Coterie Tactics --------------------------------------------------------

/// Seat 0's A bleeding at stealth 1 and a second vampire; seat 1's X and Y
/// (Camarilla, X with Animalism, Y with Protean) and Z (Sabbat), 4 blood
/// each.
fn coterie() -> (Engine, MinionId, MinionId, MinionId, MinionId) {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    let x = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 0), 4, false);
    let y = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 1), 4, false);
    let z = e.add_minion(SeatId(1), of_sect(Sect::Sabbat, 0), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["ani"];
    e.state.minions[y.index()].gained_levels = vec!["pro"];
    e.state.minions[a.index()].next_action_stealth = 1;
    (e, a, x, y, z)
}

/// Coterie Tactics: "Choose another ready unlocked vampire you control of
/// the same sect as this reacting vampire. These two vampires attempt to
/// block, using the sum of their intercept … If successful, both lock and
/// then you choose one to be the blocking vampire (and the other ceases to
/// be blocking)." X and Y (0 intercept each) attempt A's 1-stealth bleed;
/// Y's Sonar makes the sum 1: the block succeeds, both lock, seat 1 picks
/// Y among the two, Y fights A. Not a target: Z of another sect, nor a
/// locked one. [LSJ 20090309] "The vampire who ceases to block is still
/// considered to have blocked the action (e.g. they can play Cats'
/// Guidance)": X, locked, unlocks with it after the combat.
#[test]
fn coterie_tactics_blocks_with_the_sum_and_its_player_picks_the_blocker() {
    let (mut e, a, x, y, z) = coterie();
    let w = e.add_minion(
        SeatId(1),
        of_sect(zvtes::crypt::Sect::Camarilla, 2),
        4,
        false,
    );
    e.state.minions[w.index()].locked = true;
    hand(&mut e, 1, &["Coterie Tactics", "Sonar", "Cats' Guidance"]);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, play_on(0, 0, x, Target::Minion(y)));
    assert!(!req.options.contains(&play_on(0, 0, x, Target::Minion(z))));
    assert!(!req.options.contains(&play_on(0, 0, x, Target::Minion(w))));
    e.choose(index_of(&req, play_on(0, 0, x, Target::Minion(y))));
    let i = at(&e, 1, "Sonar");
    pick_when(&mut e, play(i, 0, y));
    let req = offering(&mut e, Choice::ChooseMinion(y));
    assert_eq!(
        req.options,
        vec![Choice::ChooseMinion(x), Choice::ChooseMinion(y)]
    );
    assert!(
        e.state.minions[x.index()].locked && e.state.minions[y.index()].locked,
        "both lock, then the pick"
    );
    e.choose(index_of(&req, Choice::ChooseMinion(y)));
    assert!(e.state.minions[x.index()].locked && e.state.minions[y.index()].locked);
    let i = at(&e, 1, "Cats' Guidance");
    pick_when(&mut e, play(i, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "blocked");
    assert_eq!(blood(&e, a), 4 - 1, "Y fought A");
    assert_eq!(blood(&e, y), 4 - 1);
    assert_eq!(blood(&e, x), 4, "X never fought");
    assert!(!e.state.minions[x.index()].locked, "X has blocked");
    assert!(e.state.minions[y.index()].locked);
}

/// Coterie Tactics: "Can be used by a vampire who is already attempting to
/// block." [ANK 20181122-2] X attempts, short of A's stealth, then plays
/// Coterie Tactics with Y: X's attempt goes on, joined by Y, no failed
/// attempt recorded.
#[test]
fn coterie_tactics_joins_an_attempt_under_way() {
    let (mut e, a, x, y, _) = coterie();
    hand(&mut e, 1, &["Coterie Tactics"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play_on(0, 0, x, Target::Minion(y)));
    request(&mut e);
    let act = e.action().expect("the bleed");
    assert_eq!(act.state, zvtes::actions::ActionState::Attempt(x));
    assert_eq!(act.joint, Some(y));
    assert!(act.failed_attempts.is_empty());
}

/// Coterie Tactics: "Cannot be used if the vampire cannot attempt to block
/// the action (eg. Daring the Dawn)." [LSJ 20010714-2] "Does not ignore
/// the normal prey, predator or target restrictions." [ANK 20180623] A's
/// Daring the Dawn [for]: never offered. A bleeds seat 1 in a 4-seat game:
/// seat 2's vampires are never offered it.
#[test]
fn coterie_tactics_is_not_offered_where_it_cannot_attempt() {
    let (mut e, a, x, y, _) = coterie();
    e.state.minions[a.index()].gained_levels = vec!["for"];
    hand(&mut e, 0, &["Daring the Dawn"]);
    hand(&mut e, 1, &["Coterie Tactics"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play_on(0, 0, x, Target::Minion(y)),
    );

    use zvtes::crypt::Sect;
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let x = e.add_minion(SeatId(2), of_sect(Sect::Camarilla, 0), 4, false);
    let y = e.add_minion(SeatId(2), of_sect(Sect::Camarilla, 1), 4, false);
    e.add_minion(SeatId(3), in_caps(4, 8, 3), 4, false);
    hand(&mut e, 2, &["Coterie Tactics"]);
    pick(&mut e, bleed(a));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        play_on(0, 0, x, Target::Minion(y)),
    );
}

/// Coterie Tactics, red reading: one attempt of two vampires; a bar on
/// either fails it whole. Daring the Dawn [FOR] played into the joint
/// attempt: back to no attempt, both barred, the ally left to block.
#[test]
fn coterie_tactics_joint_attempt_fails_with_its_partner() {
    let (mut e, a, x, y, _) = coterie();
    let ally = e.add_ally(SeatId(1), find_library("47th Street Royals"));
    e.state.minions[a.index()].gained_levels = vec!["for", "for"];
    hand(&mut e, 0, &["Daring the Dawn"]);
    hand(&mut e, 1, &["Coterie Tactics"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_on(0, 0, x, Target::Minion(y)));
    pick_when(&mut e, play(0, 1, a));
    let req = offering(&mut e, block(ally));
    assert!(!req.options.contains(&block(x)) && !req.options.contains(&block(y)));
    let act = e.action().expect("the bleed");
    assert_eq!(act.state, zvtes::actions::ActionState::Open);
    assert!(act.barred_blockers.contains(&x) && act.barred_blockers.contains(&y));
    assert_eq!(act.joint, None);
}

/// Coterie Tactics, red reading (one attempt of two; rules-qa 114.1): X
/// joins Y's attempt, then locks itself with Visions of Zapathasura
/// [OBF][PRE]: the joint attempt fails whole, Y never offered the block
/// again though nothing barred Y.
#[test]
fn coterie_tactics_fails_whole_once_one_partner_locks() {
    let (mut e, a, x, y, _) = coterie();
    e.state.minions[x.index()].gained_levels = vec!["obf", "obf", "pre", "pre"];
    e.state.minions[a.index()].next_action_stealth = 2;
    hand(&mut e, 1, &["Coterie Tactics", "Visions of Zapathasura"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_on(0, 0, y, Target::Minion(x)));
    let i = at(&e, 1, "Visions of Zapathasura");
    pick_when(&mut e, play(i, 1, x));
    // Seat 1 passes: the attempt resolves, X locked.
    pick(&mut e, Choice::Pass);
    request(&mut e);
    let act = e.action().expect("the bleed");
    assert_eq!(act.state, zvtes::actions::ActionState::Open);
    assert_eq!(act.joint, None);
    let failed = |m: MinionId| act.failed_attempts.iter().any(|&(f, _)| f == m);
    assert!(failed(y) && failed(x));
    assert!(!e.state.minions[y.index()].locked);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(y));
}

// ---- Day Operation ----------------------------------------------------------

/// Day Operation [for]: "Only usable as the action is announced. Vampires
/// cannot block this action. The acting vampire goes to torpor after
/// resolving the action." Seat 1's vampire is never offered a block, its
/// ally is; the bleed lands and A goes to torpor, its 1 blood paid.
#[test]
fn day_operation_bars_vampires_and_sends_the_actor_to_torpor() {
    let (mut e, a, x) = duel(&["for"], &[]);
    let ally = e.add_ally(SeatId(1), find_library("47th Street Royals"));
    hand(&mut e, 0, &["Day Operation"]);
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&block(ally)));
    assert!(!req.options.contains(&block(x)));
    assert!(!req.options.contains(&block(MinionId(3))));
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1, "the bleed landed");
    assert!(e.state.minions[a.index()].in_torpor);
    assert_eq!(blood(&e, a), 4 - 1);
}

/// Day Operation: torpor whenever the action reaches resolution, a block
/// by an ally and its combat included (rules-qa 23.3: "a blocked action
/// after combat"): seat 1's ally blocks, they fight, A goes to torpor.
#[test]
fn day_operation_sends_the_actor_to_torpor_after_an_allys_block() {
    let (mut e, a, _) = duel(&["for"], &[]);
    let ally = e.add_ally(SeatId(1), find_library("47th Street Royals"));
    hand(&mut e, 0, &["Day Operation"]);
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, block(ally));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "blocked");
    assert!(e.state.minions[a.index()].in_torpor);
}

/// Day Operation [FOR]: "As above, but usable if a minion is attempting to
/// block." Played into X's attempt, it fails it (rules-qa 92.1): the bleed
/// lands, A goes to torpor.
#[test]
fn day_operation_superior_fails_a_vampires_attempt_under_way() {
    let (mut e, a, x) = duel(&["for", "for"], &[]);
    hand(&mut e, 0, &["Day Operation"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 1, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1, "unblocked");
    assert!(e.state.minions[a.index()].in_torpor);
    assert_eq!(blood(&e, x), 4, "no combat");
}

/// Day Operation: "When played together with post-resolution, aggravated
/// damage effects (e.g. Force of Will), the acting Methuselah can resolve
/// these effects in any order." [LSJ 20040120-2] Both offered, no pass;
/// A has 1 blood left: the damage first (1 healed, to torpor, then the
/// torpor) leaves A in torpor; the torpor first, the aggravated damage
/// burns A.
#[test]
fn day_operation_and_force_of_will_resolve_in_either_order() {
    for damage_first in [true, false] {
        let (mut e, a, _) = duel(&["for"], &[]);
        e.state.minions[a.index()].locked = true;
        e.state.minions[a.index()].blood = 3;
        hand(&mut e, 0, &["Force of Will", "Day Operation"]);
        pick(&mut e, play(0, 0, a));
        pick_when(&mut e, play(0, 0, a));
        let req = offering(&mut e, Choice::TorporAfterResolution);
        assert!(req.options.contains(&Choice::TakeDamageAfterResolution));
        assert!(!req.options.contains(&Choice::Pass));
        if damage_first {
            e.choose(index_of(&req, Choice::TakeDamageAfterResolution));
        } else {
            e.choose(index_of(&req, Choice::TorporAfterResolution));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[a.index()];
        if damage_first {
            assert!(v.in_play && v.in_torpor, "to torpor by the damage");
        } else {
            assert!(!v.in_play, "aggravated damage in torpor burns it");
        }
    }
}

/// Day Operation is not circumvented by Eagle's Sight [AUS] [RTR 19950413]
/// [RTR 20020501] nor Anneke [RTR 20020501] [LSJ 20041203]: "must still
/// meet all other requirements to block … Day Operation … not
/// circumvented". In a 4-seat game, neither seat 2's vampire's Eagle's
/// Sight nor seat 3's Anneke's block is offered.
#[test]
fn day_operation_is_not_circumvented_by_eagles_sight_or_anneke() {
    let mut e = game(4);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let z = e.add_minion(SeatId(2), in_caps(4, 8, 3), 4, false);
    e.state.minions[z.index()].gained_levels = vec!["aus", "aus"];
    let anneke = e.add_minion(SeatId(3), find_card("Anneke"), 4, false);
    hand(&mut e, 0, &["Day Operation"]);
    hand(&mut e, 2, &["Eagle's Sight"]);
    pick(&mut e, bleed(a));
    pick(&mut e, play(0, 0, a));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == block(anneke) || *o == play(0, 1, z)
    });
    assert_eq!(pool(&e)[1], 10 - 1);
}

// ---- Elder Kindred Network --------------------------------------------------

/// Seat 0's caller (4 blood) and seat 1's two Ventrue reactors, Autarkis
/// Persecution called; seat 1 holds the Edge. `ekn`: how many of the
/// reactors play Elder Kindred Network first, one each; `card_vote`: the
/// caller casts the card's vote. Seat 1 burns the Edge against. Seat 0's
/// pool after.
fn ekn_referendum(ekn: usize, card_vote: bool, edge_against: bool) -> i32 {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Ventrue", 1), 4, false);
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Autarkis Persecution"]);
    hand(&mut e, 1, &["Elder Kindred Network"; 2]);
    pick(&mut e, play(0, 0, caller));
    if ekn == 0 {
        to_referendum(&mut e);
    }
    for m in [v, w].into_iter().take(ekn) {
        pick_when(&mut e, play(0, 0, m));
    }
    for _ in 0..200 {
        let req = request(&mut e);
        if e.referendum().is_none() {
            break;
        }
        let want = if req.seat == SeatId(0) && card_vote {
            Some(Choice::CastCardVote { favor: true })
        } else if req.seat == SeatId(1) && edge_against {
            Some(Choice::CastEdge { favor: false })
        } else {
            None
        };
        match want.and_then(|w| req.options.iter().position(|o| *o == w)) {
            Some(i) => e.choose(i),
            None => e.choose(fallback(&req)),
        }
    }
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pool(&e)[0]
}

/// Elder Kindred Network: "If the referendum fails, the Methuselah calling
/// the referendum burns 1 pool plus 1 additional pool for each vote
/// difference." Failed 0 to 1: 2; failed 1 to 1: 1; passed 1 to 0: none.
#[test]
fn elder_kindred_network_burns_the_caller_per_vote_difference() {
    for (card_vote, edge_against, burn) in [(false, true, 2), (true, true, 1), (true, false, 0)] {
        let without = ekn_referendum(0, card_vote, edge_against);
        let with = ekn_referendum(1, card_vote, edge_against);
        assert_eq!(
            without - with,
            burn,
            "card vote {card_vote}, edge {edge_against}"
        );
    }
}

/// Elder Kindred Network, two copies by two vampires (each play its own
/// burn, as Rant!'s; rules-digest D4): failed 0 to 1, the caller burns 2
/// for each.
#[test]
fn elder_kindred_network_copies_each_burn() {
    assert_eq!(
        ekn_referendum(0, false, true) - ekn_referendum(2, false, true),
        4
    );
}

/// Elder Kindred Network: "Cannot be used during a referendum that is
/// automatically passing." [PIB 20150105] [LSJ 19980107] Cryptic Rider's
/// passing referendum: never offered; not by a non-Ventrue either.
#[test]
fn elder_kindred_network_is_not_offered_on_an_automatic_pass() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let b = e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 2), 4, false);
    let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(1), plain("Brujah", 3), 4, false);
    hand(
        &mut e,
        0,
        &[
            "Autarkis Persecution",
            "Autarkis Persecution",
            "Cryptic Rider",
        ],
    );
    hand(&mut e, 1, &["Elder Kindred Network"]);
    pick(&mut e, play(0, 0, a));
    let req = offering(&mut e, play(0, 0, v));
    assert!(!req.options.contains(&play(0, 0, w)), "Ventrue only");
    e.choose(index_of(&req, Choice::Pass));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    pick_when(&mut e, play(1, 0, a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, b));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, v));
}

/// Elder Kindred Network, red reading: a referendum made to fail with the
/// votes for it failed by no difference: the caller burns 1. Concordia's
/// War of Ages ("Requires a titled Sabbat vampire") with the Edge for it,
/// failed by Confusion of the Eye [OBF].
#[test]
fn elder_kindred_network_forced_fail_burns_one() {
    let run = |ekn: bool| {
        let mut e = game(2);
        let c = e.add_minion(SeatId(0), find_card("Concordia"), 4, false);
        e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
        let v = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
        let z = e.add_minion(SeatId(1), in_caps(5, 5, 0), 4, false);
        e.state.minions[z.index()].gained_levels = vec!["obf", "obf"];
        e.state.seats[0].has_edge = true;
        hand(&mut e, 0, &["War of Ages"]);
        hand(
            &mut e,
            1,
            &["Confusion of the Eye", "Elder Kindred Network"],
        );
        pick(&mut e, play(0, 0, c));
        pick_when(&mut e, play(0, 1, z));
        if ekn {
            let i = at(&e, 1, "Elder Kindred Network");
            pick_when(&mut e, play(i, 0, v));
        }
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
                break;
            }
            let edge = Choice::CastEdge { favor: true };
            match req.options.iter().position(|o| *o == edge) {
                Some(i) => e.choose(i),
                None => e.choose(fallback(&req)),
            }
        }
        pool(&e)[0]
    };
    assert_eq!(run(false) - run(true), 1);
}

// ---- Eyes of the Beast ------------------------------------------------------

/// Eyes of the Beast [pro]: "+1 intercept, and put this card on this
/// vampire. This vampire gets -1 stealth. Burn this card during your
/// discard phase." X blocks A's 1-stealth bleed with it; the card itself
/// on X, past seat 0's discard phase; X's own bleed at -1 stealth; burned
/// in seat 1's discard phase.
#[test]
fn eyes_of_the_beast_stays_on_its_vampire_until_its_discard_phase() {
    let (mut e, a, x) = duel(&[], &["pro"]);
    e.state.minions[a.index()].next_action_stealth = 1;
    hand(&mut e, 1, &["Eyes of the Beast"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, x));
    request(&mut e);
    assert!(card_on(&e, "Eyes of the Beast", x).is_some());
    assert!(!ash_has(&e, 1, "Eyes of the Beast"));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "blocked");
    assert!(
        card_on(&e, "Eyes of the Beast", x).is_some(),
        "seat 0's discard phase"
    );
    pick(&mut e, bleed(x));
    let stealth = e.action().expect("X's bleed").stealth(&e.state);
    assert_eq!(stealth, -1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(card_on(&e, "Eyes of the Beast", x).is_none());
    assert!(ash_has(&e, 1, "Eyes of the Beast"));
}

/// Eyes of the Beast [PRO]: "The maneuver is only available during the
/// action in which Eyes of the Beast was played." [ANK 20191108] Played
/// in an attempt short of A's 2 stealth; X then blocks the next bleed: no
/// maneuver. "If the action is blocked but continues after combat (eg.
/// with Form of Mist) and the reacting vampire blocks again, he gets the
/// maneuver again." [ANK 20191108] X blocks with it (the maneuver
/// offered), A continues with Form of Mist [PRO], X unlocks (Cats'
/// Guidance) and blocks again with Sonar [PRO]: the maneuver again.
#[test]
fn eyes_of_the_beast_superior_maneuver_in_its_action_only() {
    let (mut e, a, x) = duel(&[], &["pro", "pro"]);
    e.state.minions[a.index()].next_action_stealth = 2;
    hand(&mut e, 1, &["Eyes of the Beast"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1, "short of the stealth");
    pick(&mut e, bleed(MinionId(1)));
    pick_when(&mut e, block(x));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        *o == Choice::UseManeuver
    });
    assert_eq!(pool(&e)[1], 10 - 1, "blocked");

    let (mut e, a, x) = duel(&["pro", "pro"], &["pro", "pro", "ani"]);
    e.state.minions[a.index()].next_action_stealth = 1;
    hand(&mut e, 0, &["Form of Mist"]);
    hand(&mut e, 1, &["Eyes of the Beast", "Cats' Guidance", "Sonar"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 1, x));
    let req = offering(&mut e, Choice::UseManeuver);
    assert_eq!(req.seat, SeatId(1));
    pick_when(&mut e, play(0, 1, a));
    pick_when(&mut e, Choice::ContinueAction);
    let i = at(&e, 1, "Cats' Guidance");
    pick_when(&mut e, play(i, 0, x));
    pick_when(&mut e, block(x));
    let i = at(&e, 1, "Sonar");
    pick_when(&mut e, play(i, 1, x));
    let req = offering(&mut e, Choice::UseManeuver);
    assert_eq!(req.seat, SeatId(1), "the second block's maneuver");
}

/// Eyes of the Beast, two copies (rules-digest D4): X plays one in each of
/// two failed attempts; both on X, its own bleed at -2 stealth.
#[test]
fn eyes_of_the_beast_copies_stack_on_their_vampire() {
    let (mut e, a, x) = duel(&[], &["pro"]);
    let b = MinionId(1);
    // A third: seat 0's minion phase stays open after the two bleeds.
    e.add_minion(SeatId(0), in_caps(4, 8, 5), 4, false);
    e.state.minions[a.index()].next_action_stealth = 2;
    e.state.minions[b.index()].next_action_stealth = 2;
    hand(&mut e, 1, &["Eyes of the Beast", "Eyes of the Beast"]);
    for m in [a, b] {
        pick(&mut e, bleed(m));
        pick_when(&mut e, block(x));
        pick_when(&mut e, play(0, 0, x));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    }
    assert_eq!(pool(&e)[1], 10 - 2, "both attempts short");
    let eyes = find_library("Eyes of the Beast");
    let on_x = |e: &Engine| {
        e.state
            .cards
            .iter()
            .filter(|c| c.in_play && !c.effect_only && c.host == Some(x) && c.card == eyes)
            .count()
    };
    assert_eq!(on_x(&e), 2);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(x));
    assert_eq!(e.action().expect("X's bleed").stealth(&e.state), -2);
}

// ---- Keep it Simple ---------------------------------------------------------

/// Keep it Simple: "Reduce a bleed against you by 1 for each point of
/// stealth the acting minion has when this card is played." (1 blood.) At
/// stealth 1, A's 1 bleed is reduced to none; at stealth 0, a legal no-op.
#[test]
fn keep_it_simple_reduces_a_bleed_by_the_stealth_as_played() {
    for stealth in [0u8, 1] {
        let (mut e, a, x) = duel(&[], &[]);
        e.state.minions[a.index()].next_action_stealth = stealth;
        hand(&mut e, 1, &["Keep it Simple"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, play(0, 0, x));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            pool(&e)[1],
            10 - 1 + i32::from(stealth),
            "stealth {stealth}"
        );
        assert_eq!(blood(&e, x), 4 - 1, "its cost");
    }
}

/// Keep it Simple: the reduction stays with the bleed after a redirect
/// (Deep Ecology's `bleed_reduce`, the grading's amber). A's 2 bleed at
/// stealth 1 at seat 1, reduced by 1, then Deflection [dom] sends it to
/// seat 2: seat 2 loses 1, seat 1 none. Its 1 blood: seat 1's ally is
/// never offered it (rules-digest §3.14).
#[test]
fn keep_it_simple_follows_a_redirected_bleed() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), elder(), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    let y = e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    e.state.minions[y.index()].gained_levels = vec!["dom"];
    let ally = e.add_ally(SeatId(1), find_library("47th Street Royals"));
    e.add_minion(SeatId(2), in_caps(4, 8, 3), 4, false);
    e.state.minions[a.index()].next_action_stealth = 1;
    hand(&mut e, 1, &["Keep it Simple", "Deflection"]);
    pick(&mut e, bleed(a));
    let req = offering(&mut e, play(0, 0, x));
    assert!(
        !req.options.contains(&play(0, 0, ally)),
        "an ally has no blood"
    );
    e.choose(index_of(&req, play(0, 0, x)));
    let i = at(&e, 1, "Deflection");
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlocksDeclined);
    e.choose(index_of(&req, play_on(i, 0, y, Target::Seat(SeatId(2)))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((pool(&e)[1], pool(&e)[2]), (10, 10 - 1));
}

// ---- Minor Irritation -------------------------------------------------------

/// Minor Irritation: "Only usable when this vampire successfully blocks an
/// ally or a younger vampire (play before combat, if any). This vampire
/// doesn't lock for successfully blocking." X blocks the younger A and
/// stays unlocked through the combat; against an older acting vampire it
/// is never offered.
#[test]
fn minor_irritation_keeps_its_blocker_unlocked() {
    let (mut e, a, x) = young_bleeder(&[]);
    hand(&mut e, 1, &["Minor Irritation"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 2 - 1, "the combat");
    assert!(!e.state.minions[x.index()].locked);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(6, 8, 0), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(2, 3, 0), 3, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    hand(&mut e, 1, &["Minor Irritation"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
}

/// Minor Irritation with Coterie Tactics: "The vampire who ceases to block
/// is still considered to have blocked the action" [LSJ 20090309]: X, not
/// picked, plays it and has its "both lock" undone; Y, the blocker, stays
/// locked.
#[test]
fn minor_irritation_unlocks_the_coterie_vampire_not_picked() {
    use zvtes::crypt::Sect;
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(2, 3, 0), 2, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 0), 4, false);
    let y = e.add_minion(SeatId(1), of_sect(Sect::Camarilla, 1), 4, false);
    e.state.minions[y.index()].gained_levels = vec!["pro"];
    e.state.minions[a.index()].next_action_stealth = 1;
    hand(&mut e, 1, &["Coterie Tactics", "Sonar", "Minor Irritation"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play_on(0, 0, x, Target::Minion(y)));
    let i = at(&e, 1, "Sonar");
    pick_when(&mut e, play(i, 0, y));
    pick_when(&mut e, Choice::ChooseMinion(y));
    let i = at(&e, 1, "Minor Irritation");
    pick_when(&mut e, play(i, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "blocked");
    assert!(!e.state.minions[x.index()].locked);
    assert!(e.state.minions[y.index()].locked);
}

// ---- Shattered Mirror -------------------------------------------------------

/// Shattered Mirror [dem]: "Put this card on the acting minion, lock this
/// vampire, and end the action. The attached minion gets -1 bleed. The
/// attached vampire can burn this card as an action that costs 1 blood."
/// "The action ends unsuccessfully immediately, no other action modifier
/// or reaction can be played afterwards." [RTR 20180719] [ANK 20200207] No
/// bleed, no combat, nothing more offered in the action; the card itself
/// on A, its Methuselah's; A's next bleed at 0; A burns it as an action for
/// 1 blood, to seat 1's ash heap.
#[test]
fn shattered_mirror_ends_the_bleed_and_stays_on_the_acting_minion() {
    let (mut e, a, x) = young_bleeder(&["dem"]);
    hand(&mut e, 1, &["Shattered Mirror"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, x));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::MinionPhase)
    );
    assert!(e.action().is_none(), "the action ended");
    assert_eq!(pool(&e)[1], 10, "no bleed");
    assert_eq!(blood(&e, a), 2, "no combat");
    assert!(e.state.minions[x.index()].locked);
    let mirror = card_on(&e, "Shattered Mirror", a).expect("on A");
    assert_eq!(e.state.card_controller(mirror), SeatId(0));
    assert!(!ash_has(&e, 1, "Shattered Mirror"));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(a));
    assert_eq!(e.action().expect("A's bleed").bleed_amount(&e.state), 0);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let burn = granted(mirror.0, a);
    assert!(req.options.contains(&burn), "{:?}", req.options);
    e.choose(index_of(&req, burn));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(card_on(&e, "Shattered Mirror", a).is_none());
    assert!(ash_has(&e, 1, "Shattered Mirror"));
    assert_eq!(blood(&e, a), 2 - 1);
}

/// Shattered Mirror [DEM]: "As above, but do not lock this vampire." "A
/// minion can have only one Shattered Mirror": X left unlocked; against A
/// holding one, never offered.
#[test]
fn shattered_mirror_superior_leaves_the_blocker_unlocked_and_only_one() {
    let (mut e, a, x) = young_bleeder(&["dem", "dem"]);
    hand(&mut e, 1, &["Shattered Mirror"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 1, x));
    request(&mut e);
    assert!(!e.state.minions[x.index()].locked);
    assert!(card_on(&e, "Shattered Mirror", a).is_some());

    let (mut e, a, x) = young_bleeder(&["dem", "dem"]);
    on(&mut e, "Shattered Mirror", a);
    hand(&mut e, 1, &["Shattered Mirror"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == x),
    );
}

/// Shattered Mirror: "The attached vampire can burn this card as an
/// action": an attached ally cannot (rules-digest D16). Seat 0's ally
/// holding one is never offered the burn, seat 0's vampire holding the
/// other is.
#[test]
fn shattered_mirror_on_an_ally_is_not_burned_by_it() {
    let (mut e, a, _) = duel(&[], &[]);
    let ally = e.add_ally(SeatId(0), find_library("47th Street Royals"));
    let on_ally = on(&mut e, "Shattered Mirror", ally);
    let on_a = on(&mut e, "Shattered Mirror", a);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&bleed(ally)), "the ally can act");
    assert!(!req.options.contains(&granted(on_ally.0, ally)));
    assert!(req.options.contains(&granted(on_a.0, a)));
}

// ---- Sonar ------------------------------------------------------------------

/// Sonar: "Do not replace until after the current turn. [pro] +1
/// intercept, only usable during a directed action. [PRO] +1 intercept."
/// Against A's undirected hunt, [pro] is never offered; against its bleed
/// X blocks with it, and its replacement waits for the turn's end.
#[test]
fn sonar_needs_a_directed_action_and_is_replaced_as_the_turn_ends() {
    let (mut e, a, x) = duel(&[], &["pro"]);
    e.state.minions[a.index()].next_action_stealth = 1;
    hand(&mut e, 1, &["Sonar"]);
    pick(
        &mut e,
        zvtes::decision::Choice::TakeAction {
            minion: a,
            action: zvtes::decision::ActionKind::Hunt,
            target: None,
        },
    );
    pick_when(&mut e, block(x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));

    let (mut e, a, x) = duel(&[], &["pro"]);
    e.state.minions[a.index()].next_action_stealth = 1;
    hand(&mut e, 1, &["Sonar"]);
    fill_hand(&mut e, 1);
    e.state.seats[1].library.push(find_library("Leverage"));
    let leverage = |e: &Engine| e.state.seats[1].hand.contains(&find_library("Leverage"));
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "blocked");
    assert!(!leverage(&e), "not replaced this turn");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(leverage(&e), "replaced after the turn");
}

// ---- Suppressing Fire -------------------------------------------------------

/// Suppressing Fire: "Only usable by a ready unlocked minion with a gun
/// other than the acting minion. The blocking minion gets -1 intercept."
/// "Can only be played by the controller of the acting minion." [LSJ
/// 19990425] Seat 0's G (a .44 Magnum) plays it on X's attempt at A's
/// 0-stealth bleed: X falls short, the bleed lands; seat 1's gun-holder is
/// never offered its copy.
#[test]
fn suppressing_fire_lowers_the_blockers_intercept() {
    let (mut e, a, x) = duel(&[], &[]);
    let g = MinionId(1);
    on(&mut e, ".44 Magnum", g);
    on(&mut e, ".44 Magnum", MinionId(3));
    hand(&mut e, 0, &["Suppressing Fire"]);
    hand(&mut e, 1, &["Suppressing Fire"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    let req = offering(&mut e, play(0, 0, g));
    assert!(
        !req.options.contains(&play(0, 0, a)),
        "not the acting minion"
    );
    e.choose(index_of(&req, play(0, 0, g)));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == MinionId(3)),
    );
    assert_eq!(pool(&e)[1], 10 - 1, "the attempt fell short");
}

/// Suppressing Fire: "Multiple copies can be played by different minions on
/// the same action." [ANK 20200515] G1 and G2 each play one on X's
/// attempt, X's Sonar between them: -2 in all, the bleed lands; G1 is not
/// offered a second.
#[test]
fn suppressing_fire_copies_by_different_minions() {
    let (mut e, a, x) = duel(&[], &["pro"]);
    let (g1, g2) = (
        MinionId(1),
        e.add_minion(SeatId(0), in_caps(4, 8, 5), 4, false),
    );
    on(&mut e, ".44 Magnum", g1);
    on(&mut e, ".44 Magnum", g2);
    hand(&mut e, 0, &["Suppressing Fire", "Suppressing Fire"]);
    hand(&mut e, 1, &["Sonar"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, g1));
    pick_when(&mut e, play(0, 0, x));
    let req = offering(&mut e, play(0, 0, g2));
    assert!(!req.options.contains(&play(0, 0, g1)), "one per minion");
    e.choose(index_of(&req, play(0, 0, g2)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10 - 1);
}

// ---- Voice of Madness -------------------------------------------------------

/// Voice of Madness [dem]: "Lock this vampire and end the action." "The
/// action ends unsuccessfully immediately, no other action modifier or
/// reaction can be played afterwards." [RTR 20180719] [ANK 20200207] No
/// bleed, no combat, the next request out of the action; [DEM] against an
/// acting ally with 1 life: "the acting minion burns 1 blood or life" — it
/// burns. Not against an older vampire.
#[test]
fn voice_of_madness_ends_the_action_and_burns_the_acting_minion() {
    let (mut e, a, x) = young_bleeder(&["dem"]);
    hand(&mut e, 1, &["Voice of Madness"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, x));
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::MinionPhase)
    );
    assert!(e.action().is_none());
    assert_eq!(pool(&e)[1], 10);
    assert_eq!(blood(&e, a), 2, "no combat");
    assert!(e.state.minions[x.index()].locked);
    assert_eq!(blood(&e, x), 4 - 1, "its cost");

    let (mut e, _, x) = young_bleeder(&["dem", "dem"]);
    let ally = e.add_ally(SeatId(0), find_library("47th Street Royals"));
    e.state.minions[ally.index()].blood = 1;
    hand(&mut e, 1, &["Voice of Madness"]);
    pick(&mut e, bleed(ally));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 1, x));
    request(&mut e);
    assert!(!e.state.minions[ally.index()].in_play, "burned");
    assert!(ash_has(&e, 0, "47th Street Royals"));
    assert!(e.state.minions[x.index()].locked);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(6, 8, 0), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(2, 3, 0), 3, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["dem"];
    hand(&mut e, 1, &["Voice of Madness"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, play(0, 0, x));
}

// ---- Minions and equipment -------------------------------------------------

/// An equipment already on `host`, its owner's, with its layers (no
/// play): its id.
fn equipped(e: &mut Engine, name: &str, host: MinionId) -> CardId {
    let seat = e.state.minions[host.index()].owner.0;
    in_play(e, seat, name, Some(host))
}

/// Seat 0's ally `name`, able to act: its minion and its card.
fn ally(e: &mut Engine, name: &str) -> (MinionId, CardId) {
    let m = e.add_ally(SeatId(0), find_library(name));
    (m, e.state.minions[m.index()].ally_id().expect("an ally"))
}

/// `seat`'s pending strike damage in the combat, (normal, aggravated).
fn pending(e: &Engine, side: usize) -> (u8, u8) {
    let p = e.combat().expect("combat").sides[side].pending;
    (p.normal, p.aggravated)
}

// ---- Agate Talisman ---------------------------------------------------------

/// Agate Talisman: "The bearer can lock this card during the polling step
/// of any referendum before votes and ballots are cast to get +1 vote." In
/// a blood hunt (any referendum): locked, B's votes one more; once B cast
/// its votes, no lock offered.
#[test]
fn agate_talisman_votes_in_a_blood_hunt_before_any_cast() {
    for lock_first in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        let b = e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
        e.state.minions[b.index()].granted_title = Some(Title::Votes(1));
        let victim = e.add_minion(SeatId(1), in_caps(4, 8, 2), 1, true);
        e.add_minion(SeatId(1), in_caps(4, 8, 3), 4, false);
        let talisman = equipped(&mut e, "Agate Talisman", b);
        pick(
            &mut e,
            Choice::TakeAction {
                minion: a,
                action: zvtes::decision::ActionKind::Diablerie,
                target: Some(victim),
            },
        );
        let lock = use_at(talisman, 0, None, Target::None);
        let req = until(&mut e, SeatId(0), WindowLabel::BloodHuntPolling);
        assert!(req.options.contains(&lock));
        let before = e
            .referendum()
            .expect("blood hunt")
            .votes_cast(&e.state, b, true);
        if lock_first {
            e.choose(index_of(&req, lock));
            let r = e.referendum().expect("blood hunt");
            assert_eq!(r.votes_cast(&e.state, b, true), before + 1);
            assert!(e.state.cards[talisman.index()].locked);
        } else {
            e.choose(index_of(&req, votes(b, true)));
            never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, lock);
        }
    }
}

/// Agate Talisman: "A vampire can have only one Agate Talisman": a
/// vampire holding one is not offered a second; an ally holding one is,
/// and has two (rules-digest D4: the cap reads on the word it qualifies;
/// rules-qa 17.4).
#[test]
fn agate_talisman_only_one_is_a_vampires() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let ally = e.add_ally(SeatId(0), find_library("Arms Dealer"));
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    equipped(&mut e, "Agate Talisman", v);
    equipped(&mut e, "Agate Talisman", ally);
    hand(&mut e, 0, &["Agate Talisman"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, v)), "the vampire's one");
    e.choose(index_of(&req, play(0, 0, ally)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let talisman = find_library("Agate Talisman");
    let held = e
        .state
        .cards
        .iter()
        .filter(|c| c.card == talisman && c.present() && c.host == Some(ally))
        .count();
    assert_eq!(held, 2);
}

/// Agate Talisman: "Can be equipped by a vampire with capacity less than 4,
/// although they cannot lock it to get a vote." [LSJ 20030607] A capacity
/// 3 vampire equips it from hand; on such a vampire, no lock in the blood
/// hunt.
#[test]
fn agate_talisman_on_a_young_vampire_gives_no_vote() {
    let mut e = game(2);
    let young = e.add_minion(SeatId(0), in_caps(1, 3, 0), 3, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    hand(&mut e, 0, &["Agate Talisman"]);
    pick(&mut e, play(0, 0, young));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.present() && c.card == find_library("Agate Talisman") && c.host == Some(young)));

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let young = e.add_minion(SeatId(0), in_caps(1, 3, 0), 3, false);
    e.state.minions[young.index()].granted_title = Some(Title::Votes(1));
    let victim = e.add_minion(SeatId(1), in_caps(4, 8, 2), 1, true);
    e.add_minion(SeatId(1), in_caps(4, 8, 3), 4, false);
    let talisman = equipped(&mut e, "Agate Talisman", young);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: zvtes::decision::ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::BloodHuntPolling);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_at(talisman, 0, None, Target::None),
    );
}

/// Agate Talisman: a torpid bearer may still lock it (rules-digest D25: a
/// printed ability with no "if ready" works in torpor; D14: a lock for a
/// vote one cannot cast is a legal no-op). B in torpor; seat 0 holds the
/// Edge (its polling request): the lock offered, the Talisman locked.
#[test]
fn agate_talisman_is_locked_by_a_torpid_bearer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let b = e.add_minion(SeatId(0), in_caps(4, 8, 1), 2, true);
    let victim = e.add_minion(SeatId(1), in_caps(4, 8, 2), 1, true);
    e.add_minion(SeatId(1), in_caps(4, 8, 3), 4, false);
    e.state.seats[0].has_edge = true;
    let talisman = equipped(&mut e, "Agate Talisman", b);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: zvtes::decision::ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    let lock = use_at(talisman, 0, None, Target::None);
    let req = until(&mut e, SeatId(0), WindowLabel::BloodHuntPolling);
    assert!(req.options.contains(&lock), "{:?}", req.options);
    e.choose(index_of(&req, lock));
    assert!(e.state.cards[talisman.index()].locked);
}

// ---- Arms Dealer, Loyal Street Gang -----------------------------------------

/// Arms Dealer: "may take a +1 stealth action to allow you to search your
/// library for a weapon and move it to your hand. Shuffle and discard
/// afterward." Unblocked: the Bang Nakh found, the hand discarded back to
/// its size.
#[test]
fn arms_dealer_fetches_a_weapon_and_discards_down() {
    let mut e = game(2);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let (dealer, id) = ally(&mut e, "Arms Dealer");
    fill_hand(&mut e, 0);
    let size = e.state.seats[0].hand.len();
    for name in [
        "Agate Talisman",
        "Bang Nakh — Tiger's Claws",
        "Agate Talisman",
    ] {
        let c = find_library(name);
        let s = &mut e.state.seats[0];
        s.library.put(c, zvtes::copies::CopyId::NONE);
    }
    pick(&mut e, use_at(id, 0, Some(dealer), Target::None));
    pick_when(
        &mut e,
        Choice::FindCard(find_library("Bang Nakh — Tiger's Claws")),
    );
    let req = request(&mut e);
    assert_eq!(req.window, WindowLabel::DiscardDown);
    assert_eq!(e.state.seats[0].hand.len(), size + 1);
    assert!(e.state.seats[0]
        .hand
        .contains(&find_library("Bang Nakh — Tiger's Claws")));
    e.choose(0);
    request(&mut e);
    assert_eq!(e.state.seats[0].hand.len(), size);
}

/// Loyal Street Gang: "Mortal with 2 life. 1 strength, 0 bleed." (Brujah.)
/// Recruited by a Brujah: it enters with 2 life; a Ventrue is offered no
/// recruit.
#[test]
fn loyal_street_gang_is_a_plain_brujah_ally() {
    let mut e = game(2);
    let brujah = e.add_minion(SeatId(0), textless(|c| c.clan == "Brujah", 0), 4, false);
    let ventrue = e.add_minion(SeatId(0), textless(|c| c.clan == "Ventrue", 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 0), 4, false);
    hand(&mut e, 0, &["Loyal Street Gang"]);
    let req = request(&mut e);
    assert!(!req.options.contains(&play(0, 0, ventrue)));
    e.choose(index_of(&req, play(0, 0, brujah)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let gang = e
        .state
        .minions
        .iter()
        .find(|m| m.in_play && m.kind.ally_card() == Some(find_library("Loyal Street Gang")))
        .expect("recruited");
    assert_eq!(gang.blood, 2);
}

// ---- Bang Nakh, Sengir Dagger, Mark V, Beretta 9mm --------------------------

/// Bang Nakh — Tiger's Claws: "Melee weapon. Strike: strength+2 damage." A
/// (strength 1) strikes X for 3.
#[test]
fn bang_nakh_strikes_for_strength_plus_two() {
    let (mut e, a, x) = duel(&[], &[]);
    let nakh = equipped(&mut e, "Bang Nakh — Tiger's Claws", a);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(nakh, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 3);
}

/// Sengir Dagger: "Strike: strength aggravated damage." A (strength 1): 1
/// aggravated pending on X.
#[test]
fn sengir_dagger_cuts_aggravated_by_strength() {
    let (mut e, a, x) = duel(&[], &["for"]);
    let dagger = equipped(&mut e, "Sengir Dagger", a);
    hand(&mut e, 1, &["Armor of Vitality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(dagger, 0));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 1));
}

/// Mark V: "Strike: 4R damage, only usable once each round and only at long
/// range." At close range not offered; at long range (Flash [cel]) it
/// strikes 4; its additional strike (Blur [cel]) cannot use it again, the
/// second copy can: "A second copy allows a second use in the same round."
/// [ANK 20230316]
#[test]
fn a_second_mark_v_fires_again_in_the_round() {
    let (mut e, a, x) = duel(&["cel"], &[]);
    let m1 = equipped(&mut e, "Mark V", a);
    fight(&mut e, a, x);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, weapon(m1, 0));

    let (mut e, a, x) = duel(&["cel"], &[]);
    let m1 = equipped(&mut e, "Mark V", a);
    let m2 = equipped(&mut e, "Mark V", a);
    hand(&mut e, 0, &["Blur", "Flash"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(1, 0, a));
    pick_when(&mut e, weapon(m1, 0));
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, weapon(m2, 0));
    assert!(!req.options.contains(&weapon(m1, 0)));
    e.choose(index_of(&req, weapon(m2, 0)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor, "4 and 4 on 4 blood");
}

/// Beretta 9mm: "Or, if bearer has another Beretta 9mm, strike: 2R damage,
/// with an optional maneuver each combat (a minion may use only one Beretta
/// 9mm to maneuver each round)." One Beretta: 1R only. Two: the 2R strike
/// offered; one copy's maneuver commits round 1 to it, the other's is not
/// offered that round, and is in round 2.
#[test]
fn beretta_pair_strikes_2r_and_maneuvers_once_a_round() {
    let (mut e, a, x) = duel(&[], &[]);
    let b = equipped(&mut e, "Beretta 9mm", a);
    fight(&mut e, a, x);
    let req = offering(&mut e, weapon(b, 0));
    assert!(!req.options.contains(&weapon(b, 1)), "no other Beretta");

    let (mut e, a, x) = duel(&["pot"], &[]);
    let b1 = equipped(&mut e, "Beretta 9mm", a);
    let b2 = equipped(&mut e, "Beretta 9mm", a);
    hand(&mut e, 0, &["Relentless Pursuit"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, weapon(b1, 2));
    assert!(req.options.contains(&weapon(b2, 2)));
    e.choose(index_of(&req, weapon(b1, 2)));
    // X may maneuver back; round 1's other maneuver is gone.
    loop {
        let req = request(&mut e);
        assert!(!req.options.contains(&weapon(b2, 2)), "round 1");
        if req.window == WindowLabel::CombatPress {
            break;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(blood(&e, x), 4 - 2, "the committed 2R strike");
    pick_when(&mut e, play(0, 0, a));
    let req = offering(&mut e, weapon(b2, 2));
    assert!(
        !req.options.contains(&weapon(b1, 2)),
        "b1's once each combat"
    );
}

/// Beretta 9mm: "The optional maneuver cannot be used if the strike cannot
/// be used (eg. Hidden Lurker)." [LSJ 20021028] Under X's One With the
/// Land [PRO] (no strike in the first round), neither the pair's strikes
/// nor their maneuvers are offered.
#[test]
fn beretta_maneuver_is_not_used_while_its_strike_is_barred() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    let tzimisce = |c: &zvtes::crypt::CryptCard| c.clan == "Tzimisce" && c.capacity >= 4;
    let x = e.add_minion(SeatId(1), textless(tzimisce, 0), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["pro", "pro"];
    e.state.minions[x.index()].locked = true;
    let b1 = equipped(&mut e, "Beretta 9mm", a);
    let b2 = equipped(&mut e, "Beretta 9mm", a);
    hand(&mut e, 1, &["One With the Land"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 1, x));
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::UseManeuver)
            || matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. }
                if *c == b1 || *c == b2)
    });
}

/// Beretta 9mm, Desert Eagle: "Weapon, gun." (the comma form) are guns:
/// Kevlar Vest's gun branch prevents 2 of their strike's damage.
#[test]
fn a_comma_gun_is_a_gun_for_kevlar_vest() {
    for gun in ["Beretta 9mm", "Desert Eagle"] {
        assert!(zvtes::cards::is_gun(find_library(gun)), "{gun}");
        let (mut e, a, x) = duel(&[], &[]);
        let g = equipped(&mut e, gun, a);
        let vest = equipped(&mut e, "Kevlar Vest", x);
        fight(&mut e, a, x);
        pick_when(&mut e, weapon(g, 0));
        let req = offering(&mut e, use_at(vest, 0, None, Target::None));
        assert_eq!(req.seat, SeatId(1), "{gun}");
    }
}

// ---- Childling Muse, Veneficorum Artum Sanguis ------------------------------

/// Childling Muse: "This Malkavian gets +1 bleed." Employed by a Malkavian:
/// its bleed costs 2 pool; on a Brujah it adds nothing.
#[test]
fn childling_muse_gives_its_malkavian_a_bleed() {
    for (clan, bled) in [("Malkavian", 2), ("Brujah", 1)] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), textless(|c| c.clan == clan, 0), 4, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
        equipped(&mut e, "Childling Muse", m);
        pick(&mut e, bleed(m));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, 10 - bled, "{clan}");
    }
}

/// Veneficorum Artum Sanguis: "The vampire with this equipment has superior
/// Blood Sorcery [THA]. Any Tremere or Tremere antitribu may enter combat
/// with the minion with this equipment as a Ⓓ action." The bearer has
/// superior Thaumaturgy; seat 1's Tremere is offered the rush at it, its
/// Ventrue not; the rush resolves into combat.
#[test]
fn veneficorum_gives_superior_thaumaturgy_and_draws_tremere() {
    let mut e = game(2);
    let bearer = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    let tremere = e.add_minion(SeatId(1), textless(|c| c.clan == "Tremere", 0), 4, false);
    let ventrue = e.add_minion(SeatId(1), textless(|c| c.clan == "Ventrue", 0), 4, false);
    let v = equipped(&mut e, "Veneficorum Artum Sanguis", bearer);
    assert!(e.state.has_discipline(bearer, "THA"));
    let rush = |m| use_at(v, 0, Some(m), Target::Minion(bearer));
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&rush(ventrue)));
    e.choose(index_of(&req, rush(tremere)));
    expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        (blood(&e, bearer), blood(&e, tremere)),
        (3, 3),
        "hand strikes"
    );
}

// ---- Muddled Vampire Hunter, Wendell Delburton ------------------------------

/// Muddled Vampire Hunter: "strikes with first strike. He may enter combat
/// with a ready vampire controlled by another Methuselah as a Ⓓ action."
/// His rush on X (3 blood): his 4 lands first, X goes to torpor and its
/// strike never resolves (the 1-life hunter survives).
#[test]
fn muddled_vampire_hunter_rushes_and_strikes_first() {
    let mut e = game(2);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 3, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let (hunter, id) = ally(&mut e, "Muddled Vampire Hunter");
    pick(&mut e, use_at(id, 0, Some(hunter), Target::Minion(x)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    let h = &e.state.minions[hunter.index()];
    assert!(h.in_play && h.blood == 1, "X's strike never resolved");
}

/// Wendell Delburton: "He inflicts +2 damage with melee weapons."
/// "Additional damage inherits all of the properties of the base damage."
/// [RTR 19970630] [TOM 19960225] With Bang Nakh, 0+2+2 = 4; with Sengir
/// Dagger, 2 aggravated pending. "He can enter combat with any minion as a
/// Ⓓ action."
#[test]
fn wendell_hits_harder_with_melee_weapons() {
    let mut e = game(2);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let (w, id) = ally(&mut e, "Wendell Delburton");
    let nakh = equipped(&mut e, "Bang Nakh — Tiger's Claws", w);
    pick(&mut e, use_at(id, 2, Some(w), Target::Minion(x)));
    pick_when(&mut e, weapon(nakh, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 0);
    assert!(!e.state.minions[x.index()].in_torpor, "exactly 4");

    let mut e = game(2);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["for"];
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    hand(&mut e, 1, &["Armor of Vitality"]);
    let (w, id) = ally(&mut e, "Wendell Delburton");
    let dagger = equipped(&mut e, "Sengir Dagger", w);
    pick(&mut e, use_at(id, 2, Some(w), Target::Minion(x)));
    pick_when(&mut e, weapon(dagger, 0));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 2));
}

/// Wendell Delburton: "If Wendell has 2 or less life during your unlock
/// phase, he gains 1 life. During your unlock phase, Wendell inflicts 2
/// unpreventable damage on a ready vampire you control." Carolina Vález:
/// "She is immune to unlock damage from allies like Abyssal Hunter and
/// Wendell Delburton" [ANK 20210228]: picked, she takes none; A picked
/// takes 2.
#[test]
fn carolina_takes_no_unlock_damage_from_wendell() {
    for pick_carolina in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        let carolina = e.add_minion(SeatId(0), find_card("Carolina Vález"), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
        let (w, _) = ally(&mut e, "Wendell Delburton");
        e.state.minions[w.index()].blood = 2;
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let victim = if pick_carolina { carolina } else { a };
        pick_when(&mut e, Choice::ChooseMinion(victim));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[w.index()].blood, 3, "1 life gained");
        let expect = if pick_carolina { 4 } else { 2 };
        assert_eq!(blood(&e, victim), expect);
    }
}

/// Wendell Delburton: "Wendell can strike: 2R damage. … He gets 1 optional
/// maneuver each combat." Rushing X, he maneuvers to long range once and
/// strikes 2R there.
#[test]
fn wendell_rushes_and_maneuvers() {
    let mut e = game(2);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let (w, id) = ally(&mut e, "Wendell Delburton");
    pick(&mut e, use_at(id, 2, Some(w), Target::Minion(x)));
    pick_when(&mut e, use_at(id, 1, None, Target::None));
    pick_when(&mut e, use_at(id, 0, None, Target::None));
    assert_eq!(
        e.combat().expect("combat").range,
        zvtes::combat::Range::Long
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2);
    assert_eq!(
        e.state.minions[w.index()].blood,
        3,
        "X's hand strike at long range"
    );
}

// ---- Blade of Enoch, Bomb ---------------------------------------------------

/// Blade of Enoch: "Damage this weapon inflicts on Brujah or Brujah
/// antitribu is aggravated. Frenzy cards cannot be used on this vampire
/// with capacity 6 or more. Strike: strength+1 damage." A (capacity 6+) is
/// shielded from X's Terror Frenzy [ani] and cuts the Brujah X for 2
/// aggravated. Dam the Heart's River: "The opponent is the one affected by
/// frenzy (eg. for Blade of Enoch)." [ANK 20220409] A, the bearer, may play
/// it.
#[test]
fn blade_of_enoch_cuts_brujah_aggravated_and_shields_from_frenzy() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(6, 11, 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["qui"];
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let brujah = |c: &zvtes::crypt::CryptCard| c.clan == "Brujah" && c.capacity >= 4;
    let x = e.add_minion(SeatId(1), textless(brujah, 0), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["ani", "for"];
    e.add_minion(SeatId(1), textless(brujah, 1), 4, false);
    let blade = equipped(&mut e, "Blade of Enoch", a);
    hand(&mut e, 0, &["Dam the Heart's River"]);
    hand(&mut e, 1, &["Terror Frenzy", "Armor of Vitality"]);
    fight(&mut e, a, x);
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatBeforeRange);
    assert!(
        req.options.contains(&play(0, 0, a)),
        "Dam the Heart's River"
    );
    e.choose(index_of(&req, Choice::Pass));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(
        e.state.seats[1]
            .hand
            .contains(&find_library("Terror Frenzy")),
        "not played"
    );
    e.choose(index_of(&req, weapon(blade, 0)));
    offering(&mut e, play(1, 0, x));
    assert_eq!(pending(&e, 1), (0, 2));
}

/// Blade of Enoch: "Can be equipped by a vampire with capacity less than 6,
/// although he cannot use it to strike and does not gain immunity to
/// frenzy cards." [LSJ 20030607] On a capacity 5 vampire: no strike with
/// it, X's Terror Frenzy offered.
#[test]
fn blade_of_enoch_on_a_young_vampire_neither_strikes_nor_shields() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 5, 0), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["ani"];
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let blade = equipped(&mut e, "Blade of Enoch", a);
    hand(&mut e, 1, &["Terror Frenzy"]);
    fight(&mut e, a, x);
    let req = offering(&mut e, play(0, 0, x));
    assert_eq!(req.window, WindowLabel::CombatBeforeRange);
    e.choose(index_of(&req, Choice::Pass));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        weapon(blade, 0),
    );
}

/// Bomb: "Strike: 5R damage. If the bomb is used in combat, the bearer
/// takes 5 damage as well. … Burn the Bomb after use." "Damage done to the
/// bearer is environmental." [LSJ 19970801] A (6 blood) bombs X (4 blood):
/// X to torpor, A takes 5 and X's hand strike's 1, the Bomb burned.
#[test]
fn bomb_strikes_5r_burns_and_hurts_its_bearer() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(6, 11, 0), 6, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let bomb = equipped(&mut e, "Bomb", a);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(bomb, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert_eq!(blood(&e, a), 6 - 5 - 1);
    assert!(!e.state.cards[bomb.index()].present());
    assert!(ash_has(&e, 0, "Bomb"));
}

/// Bomb: "When used as a strike, it does not burn if combat ends before it
/// resolves (eg. if the opponent uses a 'strike: combat ends')." [LSJ
/// 19981006] [LSJ 20001127-2] [LSJ 20010806-1] X's Majesty [pre]: the Bomb
/// stays, its bearer unhurt.
#[test]
fn bomb_is_kept_when_combat_ends_before_its_strike() {
    let (mut e, a, x) = duel(&[], &["pre"]);
    let bomb = equipped(&mut e, "Bomb", a);
    hand(&mut e, 1, &["Majesty"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(bomb, 0));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[bomb.index()].present());
    assert_eq!(
        (blood(&e, a), blood(&e, x)),
        (4, 4 - 1),
        "Majesty's cost only"
    );
}

/// The Channel 10 location for seat 1: its id.
fn channel_10(e: &mut Engine) -> CardId {
    e.state.cards.push(CardInPlay::new(
        find_library("Channel 10"),
        SeatId(1),
        None,
        0,
    ));
    CardId(e.state.cards.len() as u32 - 1)
}

/// Bomb: "The minion with this weapon may burn a location as a Ⓓ action."
/// "An action directed at a card controlled by another Methuselah is
/// directed at that Methuselah." [LSJ 20090324] Seat 1 is offered the
/// block; unblocked, its location and the Bomb burn.
#[test]
fn bomb_burns_a_location_directed_at_its_controller() {
    let (mut e, a, _) = duel(&[], &[]);
    let bomb = equipped(&mut e, "Bomb", a);
    let ch = channel_10(&mut e);
    pick(&mut e, use_at(bomb, 1, Some(a), Target::Card(ch)));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[ch.index()].present());
    assert!(!e.state.cards[bomb.index()].present());
    assert!(ash_has(&e, 0, "Bomb"));
}

/// Bomb: "Using the provided action does not force the minion to strike
/// with it if he is blocked." [LSJ 19971215] Blocked, A strikes by hand:
/// the Bomb and the location stay.
#[test]
fn bomb_action_blocked_keeps_the_bomb() {
    let (mut e, a, x) = duel(&[], &[]);
    let bomb = equipped(&mut e, "Bomb", a);
    let ch = channel_10(&mut e);
    pick(&mut e, use_at(bomb, 1, Some(a), Target::Card(ch)));
    pick_when(&mut e, block(x));
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatStrike);
    e.choose(index_of(&req, Choice::HandStrike));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[bomb.index()].present());
    assert!(e.state.cards[ch.index()].present());
}

/// Bomb: "The action provided by the equipment to the attached minion
/// fizzles (resolves successfully without effect) if the equipment is not
/// attached to the acting minion when the action resolves unblocked." [RTR
/// 19960221] The Bomb moved off A: the location stays.
#[test]
fn bomb_action_fizzles_without_the_bomb() {
    let (mut e, a, _) = duel(&[], &[]);
    let other = MinionId(1);
    let bomb = equipped(&mut e, "Bomb", a);
    let ch = channel_10(&mut e);
    pick(&mut e, use_at(bomb, 1, Some(a), Target::Card(ch)));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.state.cards[bomb.index()].host = Some(other);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[ch.index()].present());
    assert!(e.state.cards[bomb.index()].present());
}

/// Bomb: its action fizzling on a location gone by resolution is not paid
/// (rules-qa 117.1, cause 1/: "the action is not paid"); "Burn the Bomb
/// after use" is its effect, not a cost (rules-digest D20): the Bomb stays
/// on A.
#[test]
fn bomb_action_fizzles_on_a_gone_location_and_keeps_the_bomb() {
    let (mut e, a, _) = duel(&[], &[]);
    let bomb = equipped(&mut e, "Bomb", a);
    let ch = channel_10(&mut e);
    pick(&mut e, use_at(bomb, 1, Some(a), Target::Card(ch)));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.burn_card(ch);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert!(e.state.cards[bomb.index()].present());
    assert_eq!(e.state.cards[bomb.index()].host, Some(a));
    assert!(!ash_has(&e, 0, "Bomb"));
}

// ---- Rowan Ring, Wooden Stake -----------------------------------------------

/// Seat 1's `y` rescues `x` from torpor.
fn rescue(e: &mut Engine, y: MinionId, x: MinionId) {
    pick(
        e,
        Choice::TakeAction {
            minion: y,
            action: zvtes::decision::ActionKind::Rescue,
            target: Some(x),
        },
    );
}

/// `card` is in play on `m`, the card itself (no stand-in), out of the ash
/// heap.
fn on_itself(e: &Engine, card: CardId, m: MinionId, seat: usize, name: &str) -> bool {
    let c = &e.state.cards[card.index()];
    c.present() && c.host == Some(m) && !c.effect_only && !ash_has(e, seat, name)
}

/// Rowan Ring: "Strike: Send the opposing vampire to torpor. This card is
/// transferred to that vampire, and he or she doesn't unlock as normal
/// during the unlock phase as long as he or she remains in torpor." "Adding
/// damage to the Ring strike will not deal damage." [RTR 19960221] "Only
/// usable at close range. Damage prevention cannot be used against it."
/// [LSJ 19980219] X to torpor unhurt (no Armor of Vitality offered), the
/// Ring on X; X stays locked through seat 1's unlock phase; rescued, it
/// unlocks at the next. "If the strike is inflicted, the opponent is
/// wounded (if they play Undying Tenacity, they go to torpor after
/// combat)." [LSJ 20090622-2]
#[test]
fn rowan_ring_torpors_and_follows_its_victim() {
    let (mut e, a, x) = duel(&[], &["for"]);
    let y = MinionId(3);
    let ring = equipped(&mut e, "Rowan Ring", a);
    hand(&mut e, 1, &["Armor of Vitality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(ring, 0));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, x));
    let v = &e.state.minions[x.index()];
    assert!(v.in_torpor && v.locked && v.blood == 4, "no damage, held");
    assert!(on_itself(&e, ring, x, 0, "Rowan Ring"));
    assert_eq!(e.state.card_controller(ring), SeatId(1));
    rescue(&mut e, y, x);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[x.index()].in_torpor);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[x.index()].locked, "out of torpor: unlocks");

    let (mut e, a, x) = duel(&["pot"], &["for"]);
    let ring = equipped(&mut e, "Rowan Ring", a);
    hand(&mut e, 0, &["Relentless Pursuit"]);
    hand(&mut e, 1, &["Undying Tenacity"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(ring, 0));
    pick_when(&mut e, play(0, 0, x));
    expect(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(!e.state.minions[x.index()].in_torpor);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert!(on_itself(&e, ring, x, 0, "Rowan Ring"));
}

/// Rowan Ring: "If the opponent dodges or ends combat, they avoid the
/// effect and the ring stays." [LSJ 19980219] [ANK 20220923] X dodges (Read
/// Intentions [AUS]): X ready, the Ring on A.
#[test]
fn rowan_ring_is_dodged_and_stays() {
    let (mut e, a, x) = duel(&[], &["aus", "aus"]);
    let ring = equipped(&mut e, "Rowan Ring", a);
    hand(&mut e, 1, &["Read Intentions"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(ring, 0));
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[x.index()].in_torpor);
    assert_eq!(e.state.cards[ring.index()].host, Some(a));
}

/// Rowan Ring: "If placed on a minion that cannot have equipment, the Ring
/// is burned. The 'does not unlock as normal' effect still applies." "The
/// 'does not unlock as normal' effect continues to apply if the Ring is
/// burned or moved." [LSJ 19980831] The Ring burned off torpid X: X held
/// through its unlock phase; Helen Fairchild ("cannot have or use
/// equipment"): the Ring burned at the transfer, Helen held.
#[test]
fn rowan_ring_hold_outlives_the_ring() {
    let (mut e, a, x) = duel(&[], &[]);
    let ring = equipped(&mut e, "Rowan Ring", a);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(ring, 0));
    request(&mut e);
    e.burn_card(ring);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let v = &e.state.minions[x.index()];
    assert!(v.in_torpor && v.locked);

    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    let helen = e.add_minion(SeatId(1), find_card("Helen Fairchild"), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let ring = equipped(&mut e, "Rowan Ring", a);
    fight(&mut e, a, helen);
    pick_when(&mut e, weapon(ring, 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Rowan Ring"));
    let v = &e.state.minions[helen.index()];
    assert!(v.in_torpor && v.locked);
}

/// "Prevent normal unlock": "The 'does not unlock as normal' effect is
/// redundant with being infernal. If the minion is infernal, his controller
/// can still pay a pool to unlock him." [LSJ 20050114] Rowan Ring's and
/// Wooden Stake's infernal victim is offered the paid unlock.
#[test]
fn rowan_ring_victim_infernal_still_pays_to_unlock() {
    for weapon_name in ["Rowan Ring", "Wooden Stake"] {
        let (mut e, a, x) = duel(&[], &[]);
        e.state.minions[x.index()].made_infernal = true;
        e.state.minions[x.index()].blood = 1;
        let w = equipped(&mut e, weapon_name, a);
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::Strength(1),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        fight(&mut e, a, x);
        pick_when(&mut e, weapon(w, 0));
        let unlock = Choice::UnlockInfernal { minion: x };
        let req = until(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
        let v = &e.state.minions[x.index()];
        assert!(v.in_torpor && v.locked, "{weapon_name}");
        assert!(req.options.contains(&unlock), "{weapon_name}");
        e.choose(index_of(&req, unlock));
        assert!(!e.state.minions[x.index()].locked, "{weapon_name}");
    }
}

/// Rowan Ring transferred to a victim whose Methuselah already controls a
/// Rowan Ring: a forced take, the incoming copy burns (advanced rules
/// §1.13.1: "Where an effect forces the contest instead, the incoming copy
/// is burned"; rules-qa 58.19); the hold stays on X.
#[test]
fn rowan_ring_burns_as_it_reaches_a_holder_of_another() {
    let (mut e, a, x) = duel(&[], &[]);
    let ring = equipped(&mut e, "Rowan Ring", a);
    let theirs = equipped(&mut e, "Rowan Ring", MinionId(3));
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(ring, 0));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[ring.index()].present());
    assert!(ash_has(&e, 0, "Rowan Ring"));
    assert!(e.state.cards[theirs.index()].present());
    assert!(!e.state.cards[theirs.index()].contested);
    let v = &e.state.minions[x.index()];
    assert!(
        v.in_torpor && v.locked,
        "held through seat 1's unlock phase"
    );
}

/// Wooden Stake: "Strike: strength damage. If more than 1 damage is
/// inflicted on an opposing vampire by this weapon in a given combat, that
/// vampire is sent to torpor. In that case, this card is transferred to
/// that vampire". A (strength 1) stakes X twice (a press): 1 then 2, X
/// with blood left goes to torpor, the stake on X. The first dodged (Read
/// Intentions [AUS]): not inflicted (advanced rules §4.5.5: "Damage ...
/// dodged is not inflicted at all"), no torpor, the stake stays.
#[test]
fn wooden_stake_stakes_after_two_damage_in_a_combat() {
    for dodge in [false, true] {
        let (mut e, a, x) = duel(&["pot"], &["aus", "aus"]);
        let stake = equipped(&mut e, "Wooden Stake", a);
        hand(&mut e, 0, &["Relentless Pursuit"]);
        if dodge {
            hand(&mut e, 1, &["Read Intentions"]);
        }
        fight(&mut e, a, x);
        pick_when(&mut e, weapon(stake, 0));
        if dodge {
            pick_when(&mut e, play(0, 1, x));
        }
        pick_when(&mut e, play(0, 0, a));
        pick_when(&mut e, weapon(stake, 0));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let v = &e.state.minions[x.index()];
        if dodge {
            assert!(!v.in_torpor && v.blood == 3, "1 inflicted");
            assert_eq!(e.state.cards[stake.index()].host, Some(a));
        } else {
            assert!(v.in_torpor && v.blood == 2, "2 inflicted, blood left");
            assert!(on_itself(&e, stake, x, 0, "Wooden Stake"));
        }
    }
}

/// Wooden Stake: prevented damage is inflicted, not successfully (advanced
/// rules §4.5.5: "An effect keyed to damage inflicted fires anyway"; owner,
/// 2026-11-12, rules-qa 116.74). A (strength 2) stakes X, X prevents all 2
/// (Armor of Vitality [for]): X to torpor all the same, the stake on X.
#[test]
fn wooden_stake_counts_prevented_damage() {
    let (mut e, a, x) = duel(&[], &["for"]);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    let stake = equipped(&mut e, "Wooden Stake", a);
    hand(&mut e, 1, &["Armor of Vitality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(stake, 0));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[x.index()];
    assert!(
        v.in_torpor && v.blood == 4 - 1,
        "none landed, the Armor's 1 paid"
    );
    assert!(on_itself(&e, stake, x, 0, "Wooden Stake"));
}

/// Wooden Stake: damage Mictlantecuhtli is immune to is inflicted, not
/// successfully (rules-qa 116.74, 116.138). X (strength 2) stakes him: none
/// lands, he goes to torpor all the same, the stake on him.
#[test]
fn wooden_stake_counts_damage_its_target_is_immune_to() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mictlantecuhtli"), 6, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    e.state.minions[x.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(1),
    ));
    let stake = equipped(&mut e, "Wooden Stake", x);
    fight(&mut e, m, x);
    pick_when(&mut e, weapon(stake, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[m.index()];
    assert!(v.in_torpor && v.blood == 6, "none landed");
    assert!(on_itself(&e, stake, m, 0, "Wooden Stake"));
}

/// Wooden Stake: "If placed on a minion that cannot have equipment, the
/// Stake is burned. The 'does not unlock as normal' effect still applies."
/// "… continues to apply if the Stake is burned or moved." [LSJ 19980831]
/// A (strength 2) stakes X: the stake on torpid X burned, X held through
/// its unlock phase; Helen Fairchild: the stake burned, Helen held.
#[test]
fn wooden_stake_hold_outlives_the_stake() {
    for helen in [false, true] {
        let (mut e, a, mut x) = duel(&[], &[]);
        if helen {
            x = e.add_minion(SeatId(1), find_card("Helen Fairchild"), 4, false);
        }
        e.state.minions[a.index()].timed.push((
            zvtes::cards::Lasting::Strength(1),
            zvtes::state::Expiry::EndOfTurn,
            SeatId(0),
        ));
        let stake = equipped(&mut e, "Wooden Stake", a);
        fight(&mut e, a, x);
        pick_when(&mut e, weapon(stake, 0));
        request(&mut e);
        if !helen {
            assert!(on_itself(&e, stake, x, 0, "Wooden Stake"));
            e.burn_card(stake);
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert!(ash_has(&e, 0, "Wooden Stake"));
        let v = &e.state.minions[x.index()];
        assert!(v.in_torpor && v.locked, "helen: {helen}");
    }
}

// ---- Winchester Mansion -----------------------------------------------------

/// Winchester Mansion: "The vampire with this location has superior
/// Dementation [DEM]. At the end of your master phase, if you did not play
/// a master card from your hand this phase, this vampire burns 1 blood." A
/// master phase with none played: 1 burned; one with Channel 10 played:
/// none.
#[test]
fn winchester_mansion_bleeds_a_vampire_whose_master_phase_played_no_master() {
    for played in [false, true] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 3), 4, false);
        equipped(&mut e, "Winchester Mansion", a);
        assert!(e.state.has_discipline(a, "DEM"));
        if played {
            hand(&mut e, 0, &["Channel 10"]);
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        if played {
            pick_when(&mut e, master_play(0));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, a), if played { 4 } else { 3 }, "played: {played}");
    }
}

/// Winchester Mansion "does not count as equipment while in play": a
/// location, no equipment bar takes its Discipline (advanced rules §1.3.1;
/// rules-qa 94.4); Veneficorum Artum Sanguis, equipment, loses its own
/// (rules-qa 76.16).
#[test]
fn winchester_mansion_keeps_dementation_under_an_equipment_bar() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    equipped(&mut e, "Winchester Mansion", a);
    equipped(&mut e, "Veneficorum Artum Sanguis", a);
    e.state.equipment_barred.push(a);
    assert!(e.state.has_discipline(a, "DEM"));
    assert!(!e.state.has_discipline(a, "tha"));
}

/// Winchester Mansion on an ally (rules-digest D16: "this vampire" gives
/// an ally nothing): no Dementation, and its master phase with no master
/// played burns no life.
#[test]
fn winchester_mansion_does_nothing_for_an_ally() {
    let mut e = game(2);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let ally = e.add_ally(SeatId(0), find_library("47th Street Royals"));
    e.state.minions[ally.index()].blood = 3;
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 3), 4, false);
    equipped(&mut e, "Winchester Mansion", ally);
    assert!(!e.state.has_discipline(ally, "DEM"));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, ally), 3);
}

// ---- Phased Motion Detector -------------------------------------------------

/// Phased Motion Detector: "if a vampire plays an action modifier card
/// requiring … Obfuscate [obf] …, the bearer gets +1 intercept". "Grants
/// intercept even if the bearer is not blocking yet." [LSJ 20100329] "Can
/// provide intercept only once during each action." [ANK 20230116] A's
/// Cloak the Gathering gives X's bearer 1 (its block holds); Faceless Night
/// then gives none (the bleed lands). Without it, one Cloak is enough.
#[test]
fn phased_motion_detector_gives_intercept_on_an_obfuscate_modifier() {
    for (detector, cloaks, blocked) in [(true, 1, true), (true, 2, false), (false, 1, false)] {
        let (mut e, a, x) = duel(&["obf"], &[]);
        if detector {
            equipped(&mut e, "Phased Motion Detector", x);
        }
        hand(&mut e, 0, &["Cloak the Gathering", "Faceless Night"]);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(x));
        for _ in 0..cloaks {
            pick_when(&mut e, play(0, 0, a));
            let given = e.action().expect("the bleed").given_intercept(x);
            assert_eq!(given, detector as i32, "{detector} {cloaks}");
        }
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.seats[1].pool,
            if blocked { 10 } else { 9 },
            "{detector} {cloaks}"
        );
    }
}

/// Phased Motion Detector: "if a vampire plays an action modifier card
/// requiring … Obtenebration [obt]": an ally playing it as a vampire is
/// none (rules-digest D16). Nocturn's Shadow Play [obt] in X's attempt: X
/// given nothing; a vampire's: 1.
#[test]
fn phased_motion_detector_ignores_an_ally_playing_as_a_vampire() {
    for by_ally in [true, false] {
        let (mut e, a, x) = duel(&["obt"], &[]);
        let actor = if by_ally {
            let n = e.add_ally(SeatId(0), find_library("Nocturn"));
            e.state.minions[n.index()].blood = 3;
            n
        } else {
            a
        };
        equipped(&mut e, "Phased Motion Detector", x);
        hand(&mut e, 0, &["Shadow Play"]);
        pick(&mut e, bleed(actor));
        pick_when(&mut e, block(x));
        pick_when(&mut e, play(0, 0, actor));
        let given = e.action().expect("the bleed").given_intercept(x);
        assert_eq!(given, i32::from(!by_ally), "by ally {by_ally}");
    }
}

// ---- Pier 13, Port of Baltimore ---------------------------------------------

/// Pass until an ability of `source` (`ability`) is offered: that option.
fn ability_of(e: &mut Engine, source: AbilitySource, ability: u8) -> Choice {
    for _ in 0..300 {
        let req = request(e);
        let found = req.options.iter().copied().find(|o| {
            matches!(o, Choice::UseAbility { source: s, ability: a, .. }
                if *s == source && *a == ability)
        });
        if let Some(c) = found {
            return c;
        }
        e.choose(fallback(&req));
    }
    panic!("{source:?}'s ability {ability} never offered");
}

/// Pier 13 used in seat 0's influence phase on the hand card `name` (the
/// terms elided when it is the only card offered).
fn pier_equips(e: &mut Engine, pier: CardId, name: &str) {
    let use_pier = ability_of(e, AbilitySource::Card(pier), 0);
    pick(e, use_pier);
    let req = request(e);
    if req.window == WindowLabel::PlayTerms {
        let card = find_library(name);
        let want = req
            .options
            .iter()
            .copied()
            .find(|o| matches!(o, Choice::EmployFromHand { card: c, .. } if *c == card))
            .expect("offered");
        e.choose(index_of(&req, want));
    }
}

/// Pier 13: "During your influence phase, this minion may equip with a
/// non-location, non-unique equipment card from your hand (requirements and
/// cost apply as normal). This is not an action and cannot be blocked." In
/// seat 0's influence phase A equips the .44 Magnum (2 pool paid), not the
/// unique Sengir Dagger nor the unique location Winchester Mansion; once
/// that phase. Locquipments: "An action targeting it (eg. Conceal) is
/// directed at the Methuselah controlling it." [LSJ 20090324]
#[test]
fn pier_13_equips_its_host_in_the_influence_phase() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 3), 4, false);
    let pier = equipped(&mut e, "Pier 13, Port of Baltimore", a);
    hand(
        &mut e,
        0,
        &[".44 Magnum", "Sengir Dagger", "Winchester Mansion"],
    );
    pier_equips(&mut e, pier, ".44 Magnum");
    let hand0 = &e.state.seats[0].hand;
    assert!(hand0.contains(&find_library("Sengir Dagger")), "unique");
    assert!(
        hand0.contains(&find_library("Winchester Mansion")),
        "a location"
    );
    let magnum = find_library(".44 Magnum");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.present() && c.card == magnum && c.host == Some(a)));
    assert_eq!(e.state.seats[0].pool, 10 - 2);
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == pier),
    );
}

/// Pier 13: a torpid host may still equip (rules-digest D25: a printed
/// ability with no "if ready" works in torpor): A in torpor takes the .44
/// Magnum in seat 0's influence phase.
#[test]
fn pier_13_equips_a_torpid_host() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 2, true);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 3), 4, false);
    let pier = equipped(&mut e, "Pier 13, Port of Baltimore", a);
    hand(&mut e, 0, &[".44 Magnum"]);
    pier_equips(&mut e, pier, ".44 Magnum");
    let magnum = find_library(".44 Magnum");
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.present() && c.card == magnum && c.host == Some(a)));
    assert_eq!(e.state.seats[0].pool, 10 - 2);
}

/// Synner-G: "His ability applies when equipping in a non-normal way (eg.
/// Pier 13, Port of Baltimore)." [PIB 20150105-2] Synner-G, locked, equips
/// a Laptop Computer through Pier 13: his unlock is offered, once.
#[test]
fn synner_g_unlocks_after_a_pier_13_equip() {
    let mut e = game(2);
    let g = e.add_minion(SeatId(0), find_card("Synner-G"), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let pier = equipped(&mut e, "Pier 13, Port of Baltimore", g);
    hand(&mut e, 0, &["Laptop Computer"]);
    pick(&mut e, bleed(g));
    pier_equips(&mut e, pier, "Laptop Computer");
    assert!(e.state.minions[g.index()].locked);
    let unlock = ability_of(&mut e, AbilitySource::Vampire(g), 1);
    pick(&mut e, unlock);
    assert!(!e.state.minions[g.index()].locked);
    // Once for that equip: locked again in the phase, not offered.
    e.state.minions[g.index()].locked = true;
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, unlock);
}

/// Vulture: "His ability applies when equipping in a non-normal way (eg.
/// Pier 13, Port of Baltimore)." [PIB 20150105-2] Vulture, an Anarch,
/// locked, equips a Sport Bike through Pier 13: unlocked by his ability.
#[test]
fn vulture_unlocks_an_anarch_after_a_pier_13_vehicle() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Vulture"), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let pier = equipped(&mut e, "Pier 13, Port of Baltimore", v);
    hand(&mut e, 0, &["Sport Bike"]);
    pick(&mut e, bleed(v));
    pier_equips(&mut e, pier, "Sport Bike");
    assert!(e.state.minions[v.index()].locked);
    let unlock = ability_of(&mut e, AbilitySource::Vampire(v), 1);
    assert_eq!(
        unlock,
        Choice::UseAbility {
            source: AbilitySource::Vampire(v),
            ability: 1,
            actor: None,
            target: Target::Minion(v),
        }
    );
    pick(&mut e, unlock);
    assert!(!e.state.minions[v.index()].locked);
}

// ---- Arcane Appraiser -------------------------------------------------------

/// Seat 0's Arcane Appraiser, seat 0's A, seat 1's torpid T bearing `name`
/// and a ready vampire: (engine, appraiser, its card, A, T, the equipment).
fn appraisal(name: &str) -> (Engine, MinionId, CardId, MinionId, MinionId, CardId) {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let t = e.add_minion(SeatId(1), in_caps(4, 8, 1), 2, true);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let gear = on(&mut e, name, t);
    let (app, id) = ally(&mut e, "Arcane Appraiser");
    (e, app, id, a, t, gear)
}

/// Arcane Appraiser: "can move an equipment from a vampire in torpor to a
/// ready minion you control as a +1 stealth Ⓓ action." At seat 1's torpid
/// T's .44 Magnum: directed at seat 1 (its block window), A chosen as the
/// action is announced; unblocked, the Magnum on A, seat 0's.
#[test]
fn arcane_appraiser_moves_equipment_off_a_torpid_vampire() {
    let (mut e, app, id, a, _, gun) = appraisal(".44 Magnum");
    pick(&mut e, use_at(id, 0, Some(app), Target::Card(gun)));
    let req = expect(&mut e, SeatId(0), WindowLabel::ResolutionChoice);
    assert!(
        req.options.contains(&Choice::ChooseMinion(app)),
        "itself too"
    );
    e.choose(index_of(&req, Choice::ChooseMinion(a)));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[gun.index()].host, Some(a));
    assert_eq!(e.state.card_controller(gun), SeatId(0));
}

/// Arcane Appraiser: "If a second copy of a unique equipment you control
/// would enter play, instead it is burned." (Rulebook "Contested Cards": a
/// contest with oneself burns the incoming copy.) A holds a Sengir Dagger;
/// the Appraiser moves T's onto A: that copy burns to seat 1's ash heap,
/// no contest.
#[test]
fn arcane_appraiser_burns_a_second_unique_copy_it_moves() {
    let (mut e, app, id, a, _, dagger) = appraisal("Sengir Dagger");
    let own = on(&mut e, "Sengir Dagger", a);
    pick(&mut e, use_at(id, 0, Some(app), Target::Card(dagger)));
    pick_when(&mut e, Choice::ChooseMinion(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[dagger.index()].present());
    assert!(ash_has(&e, 1, "Sengir Dagger"));
    assert_eq!(e.state.cards[own.index()].host, Some(a));
    assert!(!e.state.cards[own.index()].contested);
}

/// Arcane Appraiser: a locquipment on a torpid vampire is "just a location
/// while in play" [LSJ 20080114]: T's Winchester Mansion is no target, its
/// .44 Magnum is; the Appraiser may take the Magnum itself (rules-digest
/// D28: the actor among the chosen).
#[test]
fn arcane_appraiser_takes_no_locquipment_and_may_take_for_itself() {
    let (mut e, app, id, _, t, gun) = appraisal(".44 Magnum");
    let mansion = on(&mut e, "Winchester Mansion", t);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req
        .options
        .contains(&use_at(id, 0, Some(app), Target::Card(mansion))));
    e.choose(index_of(&req, use_at(id, 0, Some(app), Target::Card(gun))));
    pick_when(&mut e, Choice::ChooseMinion(app));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[gun.index()].host, Some(app));
    assert_eq!(e.state.cards[mansion.index()].host, Some(t));
}

/// Ghoul: "Ghouls are monsters, not mortal." [ANK 20200203-2] [LSJ
/// 20060515] The Arcane Appraiser is no mortal.
#[test]
fn arcane_appraiser_is_no_mortal() {
    let mut e = game(2);
    let (app, _) = ally(&mut e, "Arcane Appraiser");
    assert!(!e.state.is_mortal(app));
}

// ---- Martinelli's Ring ------------------------------------------------------

/// Martinelli's Ring: "The bearer cannot have or be the target of cards
/// requiring Malkavian, Malkavian antitribu, Dementation [dem] or Serpentis
/// [ser]". A's Sleep of Reason [dem] may target the young Y, not the young
/// X bearing the Ring; "The action provided by this card is considered to
/// be an action requiring the listed clan" [LSJ 20080604]: the Muddled
/// Vampire Hunter (Malkavian) may rush Y, not X.
#[test]
fn martinellis_ring_shields_its_bearer_from_dementation_cards() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(7, 7, 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dem"];
    let x = e.add_minion(SeatId(1), in_caps(1, 3, 0), 1, false);
    let y = e.add_minion(SeatId(1), in_caps(1, 3, 1), 1, false);
    equipped(&mut e, "Martinelli's Ring", x);
    let (hunter, id) = ally(&mut e, "Muddled Vampire Hunter");
    hand(&mut e, 0, &["Sleep of Reason"]);
    let req = request(&mut e);
    let sleep = |t| play_on(0, 0, a, Target::Minion(t));
    let rush = |t| use_at(id, 0, Some(hunter), Target::Minion(t));
    assert!(req.options.contains(&sleep(y)) && !req.options.contains(&sleep(x)));
    assert!(req.options.contains(&rush(y)) && !req.options.contains(&rush(x)));
}

/// Martinelli's Ring: "(any of these are burned)". On a Malkavian, the
/// Childling Muse (a Malkavian retainer) burns as the Ring is equipped;
/// another employed later burns as it arrives.
#[test]
fn martinellis_ring_burns_a_malkavian_retainer_on_its_bearer() {
    let mut e = game(2);
    let malk = |c: &zvtes::crypt::CryptCard| c.clan == "Malkavian" && c.capacity >= 4;
    let m = e.add_minion(SeatId(0), textless(malk, 0), 4, false);
    let other = e.add_minion(SeatId(0), textless(malk, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 0), 4, false);
    let muse = equipped(&mut e, "Childling Muse", m);
    hand(&mut e, 0, &["Martinelli's Ring", "Childling Muse"]);
    pick(&mut e, play(0, 0, m));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[muse.index()].present());
    assert!(ash_has(&e, 0, "Childling Muse"));
    let _ = other;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, m));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let muse = find_library("Childling Muse");
    assert!(!e.state.cards.iter().any(|c| c.present() && c.card == muse));
}

/// Martinelli's Ring: "The bearer cannot use [ser] or [dem] cards on
/// themselves (eg. Mummify)." [LSJ 20080212] The bearer's [dem] card aimed
/// at another (Sleep of Reason at the young Y) stays playable; at his own
/// young bearer of the Ring, B, not; his [dem] modifier (Mind Tricks) during
/// his own action played.
#[test]
fn martinellis_ring_bars_mummify_but_not_a_dementation_bleed() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(7, 7, 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["dem"];
    let b = e.add_minion(SeatId(0), in_caps(1, 3, 2), 1, false);
    let y = e.add_minion(SeatId(1), in_caps(1, 3, 1), 1, false);
    equipped(&mut e, "Martinelli's Ring", b);
    hand(&mut e, 0, &["Sleep of Reason"]);
    let req = request(&mut e);
    assert!(req.options.contains(&play_on(0, 0, a, Target::Minion(y))));
    assert!(!req.options.contains(&play_on(0, 0, a, Target::Minion(b))));

    // The bearer's own [dem] modifier during his action: played.
    let (mut e, a, x) = duel(&["dem"], &[]);
    equipped(&mut e, "Martinelli's Ring", a);
    hand(&mut e, 0, &["Mind Tricks"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(0, 0, a));
    assert!(!e.state.seats[0].hand.contains(&find_library("Mind Tricks")));
}

// ---- Bundi ------------------------------------------------------------------

/// Bundi: "Strike: hand strike at +1 damage. (This strike is both a hand
/// strike and a melee weapon strike)." "Inherits all of the properties of
/// the base hand strike." [TOM 19960225] [LSJ 20071020] "Effects that
/// affect or improve hand strikes can be used on its strike." [LSJ
/// 20071020] "… can be use it with a strike card that reads 'strike: make
/// a weapon strike' (eg. Brute Force). In that case, the strike is still
/// considered a hand strike (ie. can be chosen if Immortal Grapple has been
/// played)" [LSJ 20080702-1] [LSJ 20090114]. Under A's Immortal Grapple
/// [pot], Bundi's strike is offered (the Bang Nakh's not), and Brute
/// Force's weapon strike with it: 1 + 1 + 2. Aggravated hands make it
/// aggravated; Wendell's melee +2 adds (0 + 1 + 2).
#[test]
fn bundi_strikes_as_a_hand_and_a_melee_weapon_strike() {
    let (mut e, a, x) = duel(&["pot"], &[]);
    let bundi = equipped(&mut e, "Bundi", a);
    let nakh = equipped(&mut e, "Bang Nakh — Tiger's Claws", a);
    hand(&mut e, 0, &["Immortal Grapple", "Brute Force"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&weapon(bundi, 0)));
    assert!(
        !req.options.contains(&weapon(nakh, 0)),
        "a melee weapon strike only"
    );
    e.choose(index_of(&req, play_on(0, 1, a, Target::Card(bundi))));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 0, "1 + 1 + 2 on 4");

    let (mut e, a, x) = duel(&[], &["for"]);
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    let bundi = equipped(&mut e, "Bundi", a);
    hand(&mut e, 1, &["Armor of Vitality"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(bundi, 0));
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (0, 2));

    let mut e = game(2);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let (w, id) = ally(&mut e, "Wendell Delburton");
    let bundi = equipped(&mut e, "Bundi", w);
    pick(&mut e, use_at(id, 2, Some(w), Target::Minion(x)));
    pick_when(&mut e, weapon(bundi, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 3);
}

/// Bundi: "Bearer may prevent 1 damage from each melee weapon strike made
/// against him or her." "Preventing damage with it does not force the
/// minion to strike with it." [LSJ 19971215] X, striking by hand, prevents
/// 1 of A's Bang Nakh in round 1 and again in round 2 (A's press); against
/// a gun strike it is not offered.
#[test]
fn bundi_prevents_one_from_each_melee_weapon_strike() {
    let (mut e, a, x) = duel(&["pot"], &[]);
    let nakh = equipped(&mut e, "Bang Nakh — Tiger's Claws", a);
    let bundi = equipped(&mut e, "Bundi", x);
    let prevent = use_at(bundi, 1, None, Target::None);
    hand(&mut e, 0, &["Relentless Pursuit"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(nakh, 0));
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatStrike);
    e.choose(index_of(&req, Choice::HandStrike));
    pick_when(&mut e, prevent);
    expect(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert_eq!(blood(&e, x), 4 - 2, "3, 1 prevented");
    pick(&mut e, play(0, 0, a));
    pick_when(&mut e, weapon(nakh, 0));
    pick_when(&mut e, prevent);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[x.index()];
    assert!(v.blood == 0 && !v.in_torpor, "round 2: 3, 1 prevented");

    let (mut e, a, x) = duel(&[], &[]);
    let gun = equipped(&mut e, ".44 Magnum", a);
    let bundi = equipped(&mut e, "Bundi", x);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 0));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_at(bundi, 1, None, Target::None),
    );
}

/// Bundi, two copies: each prevents its 1 (ANK 20230316, groups "Weapon
/// once per round/combat": "A second copy allows a second use"). A's Bang
/// Nakh's 3 on X: 2 prevented, 1 lands.
#[test]
fn two_bundis_prevent_one_each() {
    let (mut e, a, x) = duel(&[], &[]);
    let nakh = equipped(&mut e, "Bang Nakh — Tiger's Claws", a);
    let b1 = equipped(&mut e, "Bundi", x);
    let b2 = equipped(&mut e, "Bundi", x);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(nakh, 0));
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatStrike);
    e.choose(index_of(&req, Choice::HandStrike));
    pick_when(&mut e, use_at(b1, 1, None, Target::None));
    pick_when(&mut e, use_at(b2, 1, None, Target::None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1);
}

/// Bundi: a Ghoul Retainer's use of its employer's melee weapon "is not a
/// strike" [LSJ 20080210]: X's Bundi prevents none of it.
#[test]
fn bundi_prevents_nothing_of_a_ghoul_retainers_weapon_use() {
    let (mut e, a, x) = duel(&[], &[]);
    let nakh = equipped(&mut e, "Bang Nakh — Tiger's Claws", a);
    let ghoul = ghoul_on(&mut e, a);
    let bundi = equipped(&mut e, "Bundi", x);
    ghoul_fight(&mut e, a, x, ghoul, nakh);
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        use_at(bundi, 1, None, Target::None),
    );
    assert_eq!(
        blood(&e, x),
        4 - 3 - 1,
        "the ghoul's 1 + 2, A's hand 1: none prevented"
    );
}

// ---- Ghoul Retainer ---------------------------------------------------------

/// The Ghoul Retainer's use of weapon `w` (its controller's choice in the
/// strike step), found among the options as offered.
fn ghoul_use(e: &mut Engine, ghoul: CardId, w: CardId) -> Choice {
    for _ in 0..100 {
        let req = request(e);
        let found = req.options.iter().copied().find(|o| {
            matches!(o, Choice::UseAbility { source: AbilitySource::Card(g), target: Target::Card(t), .. }
                if *g == ghoul && *t == w)
        });
        if let Some(c) = found {
            return c;
        }
        e.choose(fallback(&req));
    }
    panic!("the Ghoul Retainer's use of {w:?} never offered");
}

/// A bleed by A blocked by X, the ghoul's use of A's weapon `w` taken in
/// A's strike step: A's strike is then its hand strike, the weapon's no
/// longer offered (elided).
fn ghoul_fight(e: &mut Engine, a: MinionId, x: MinionId, ghoul: CardId, w: CardId) {
    fight(e, a, x);
    let u = ghoul_use(e, ghoul, w);
    pick(e, u);
}

/// A Ghoul Retainer on `host`, with its layers, 2 life.
fn ghoul_on(e: &mut Engine, host: MinionId) -> CardId {
    let g = equipped(e, "Ghoul Retainer", host);
    e.state.cards[g.index()].counters = 2;
    g
}

/// Ghoul Retainer: "inflicts 1 damage … during the initial strike
/// resolution". "The 1 Damage is not inflicted at long range." [ANK
/// 20210929] "The damage is environmental, the retainer is the damage
/// source." [ANK 20180612] [ANK 20211127] "Damage is inflicted even against
/// a dodge." [RTR 19941109] [TOM 19951216] [PIB 20140324] [ANK 20200422]
/// At close range X, dodging A's hand strike
/// (Read Intentions [AUS]), still takes the ghoul's 1; at long range (X's
/// .44 Magnum maneuver), only the gun's 2 on A, the ghoul's 1 not inflicted.
#[test]
fn ghoul_retainer_bites_at_close_range_only() {
    let (mut e, a, x) = duel(&[], &["aus", "aus"]);
    ghoul_on(&mut e, a);
    hand(&mut e, 1, &["Read Intentions"]);
    fight(&mut e, a, x);
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1);

    let (mut e, a, x) = duel(&[], &[]);
    ghoul_on(&mut e, a);
    let gun = on(&mut e, ".44 Magnum", x);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 1));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4, "no bite at long range");
    assert_eq!(blood(&e, a), 4 - 2);
}

/// Ghoul Retainer: "The Ghoul is not the bearer of the weapon, the employer
/// is (eg. … Zip Gun's damage go to the employer)." [TOM 19951114] [LSJ
/// 20080210] "The Ghoul uses weapon as if making the strike, including all
/// side-effects (eg. Zip Gun's damage …)" [LSJ 20080210]. The ghoul uses
/// A's Zip Gun: X takes its 1 and A's hand 1, no bite; A takes the Zip
/// Gun's 1 and X's hand 1; A cannot strike with it that round.
#[test]
fn ghoul_retainer_fires_the_employers_zip_gun() {
    let (mut e, a, x) = duel(&[], &[]);
    let ghoul = ghoul_on(&mut e, a);
    let zip = on(&mut e, "Zip Gun", a);
    ghoul_fight(&mut e, a, x, ghoul, zip);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 1 - 1);
    assert_eq!(blood(&e, a), 4 - 1 - 1);
}

/// Ghoul Retainer: "… is not limited to damage dealing strikes (eg. Rowan
/// Ring sends to torpor, Flash Grenade ends combat)." [LSJ 20080210] The
/// ghoul uses A's Rowan Ring: X to torpor, the Ring on X. With A's Flash
/// Grenade: the combat ends, no damage; X stunned (locked, no next
/// unlock), the Grenade burned.
#[test]
fn ghoul_retainer_uses_rowan_ring_and_flash_grenade() {
    let (mut e, a, x) = duel(&[], &[]);
    let ghoul = ghoul_on(&mut e, a);
    let ring = on(&mut e, "Rowan Ring", a);
    ghoul_fight(&mut e, a, x, ghoul, ring);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert!(on_itself(&e, ring, x, 0, "Rowan Ring"));

    let (mut e, a, x) = duel(&[], &[]);
    let ghoul = ghoul_on(&mut e, a);
    let grenade = on(&mut e, "Flash Grenade", a);
    ghoul_fight(&mut e, a, x, ghoul, grenade);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!((blood(&e, a), blood(&e, x)), (4, 4), "no damage");
    let v = &e.state.minions[x.index()];
    assert!(v.locked && v.skip_unlock, "stunned");
    assert!(ash_has(&e, 0, "Flash Grenade"));
}

/// Ghoul Retainer with A's Weighted Walking Stick: "including all
/// side-effects (eg. … Weighted Walking Stick tokens)" [LSJ 20080210]: its
/// 1 + 1 damage burns 2 counters.
#[test]
fn ghoul_retainer_spends_weighted_walking_stick_counters() {
    let (mut e, a, x) = duel(&[], &[]);
    let ghoul = ghoul_on(&mut e, a);
    let stick = on(&mut e, "Weighted Walking Stick", a);
    e.state.cards[stick.index()].counters = 5;
    ghoul_fight(&mut e, a, x, ghoul, stick);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[stick.index()].counters, 3);
    assert_eq!(blood(&e, x), 4 - 2 - 1);
}

/// Ghoul Retainer: "It is not a strike and cannot provide a maneuver, press
/// or additional strike to the employer (eg. Meat Hook)." [LSJ 20080210]
/// The ghoul's Meat Hook hit gives A no press.
#[test]
fn ghoul_retainers_meat_hook_gives_no_extra_strike() {
    let (mut e, a, x) = duel(&[], &[]);
    let ghoul = ghoul_on(&mut e, a);
    let hook = equipped(&mut e, "Meat Hook", a);
    ghoul_fight(&mut e, a, x, ghoul, hook);
    assert!(!offered_press(&mut e, SeatId(0)));
    assert_eq!(blood(&e, x), 4 - 2 - 1, "the hook's 1 + 1, A's hand 1");
}

/// Ghoul Retainer: "The Ghoul cannot play ammo cards (not even with a
/// Magazine), although it can benefit from ammo cards played by the
/// employer in an earlier round (eg. Scattershot)." [LSJ 20080210] A loads
/// its .44 Magnum with Scattershot in round 1 (2 + 2 at close range); in
/// round 2 (A's press) the ghoul's use of it deals 4 too, X (cap 9+, 8
/// blood) down 4 + 1 + 4 + 1.
#[test]
fn ghoul_retainer_plays_no_ammo_but_uses_loaded_scattershot() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["pot"];
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    // Viktor (9): "+1 stealth.", on his own actions.
    let x = e.add_minion(SeatId(1), find_card("Viktor, The Night General"), 8, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let ghoul = ghoul_on(&mut e, a);
    let gun = on(&mut e, ".44 Magnum", a);
    hand(&mut e, 0, &["Scattershot", "Relentless Pursuit"]);
    fight(&mut e, a, x);
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, play(0, 0, a));
    let u = ghoul_use(&mut e, ghoul, gun);
    pick(&mut e, u);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[x.index()];
    assert!(v.in_torpor, "4 and 4 from the loaded gun: {}", v.blood);
}

/// Jar of Skin Eaters: "Ghoul Retainer is not the bearer so he can use the
/// Jar even if it has no counter on it." [ANK 20171003] A's empty Jar: A
/// cannot strike with it, the ghoul uses it: 3 aggravated pending on X, the
/// Jar burned after.
#[test]
fn ghoul_retainer_uses_an_empty_jar_of_skin_eaters() {
    let (mut e, a, x) = duel(&[], &["for"]);
    let ghoul = ghoul_on(&mut e, a);
    let jar = on(&mut e, "Jar of Skin Eaters", a);
    hand(&mut e, 1, &["Armor of Vitality"]);
    ghoul_fight(&mut e, a, x, ghoul, jar);
    offering(&mut e, play(0, 0, x));
    assert_eq!(pending(&e, 1), (1, 3), "A's hand 1, the Jar's 3 aggravated");
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 0, "Jar of Skin Eaters"));
}

/// Red reading: the ghoul's weapon use is no strike, its damage the
/// retainer's environmental damage, so X's dodge does not stop it (group
/// "Environmental damage": "Damage is inflicted even against a dodge.").
/// X dodges (Read Intentions [AUS]); the ghoul's .44 Magnum still deals 2.
#[test]
fn ghoul_retainers_weapon_use_goes_through_a_dodge() {
    let (mut e, a, x) = duel(&[], &["aus", "aus"]);
    let ghoul = ghoul_on(&mut e, a);
    let gun = on(&mut e, ".44 Magnum", a);
    hand(&mut e, 1, &["Read Intentions"]);
    ghoul_fight(&mut e, a, x, ghoul, gun);
    pick_when(&mut e, play(0, 1, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2);
}

/// Ghoul Retainer with A's Wooden Stake: "The Ghoul uses weapon as if
/// making the strike, including all side-effects" [LSJ 20080210]: its 1
/// counts on the stake's tally. Round 1 the ghoul stakes X (1, A's hand
/// 1), round 2 (a press) A stakes X (1, the ghoul's bite 1): 2 by the
/// stake, X to torpor with blood left, the stake on X.
#[test]
fn ghoul_retainers_wooden_stake_damage_counts_for_the_stake() {
    let (mut e, a, x) = duel(&["pot"], &[]);
    let cap = e.state.capacity_of(x).unwrap();
    assert!(cap > 4, "blood left after 4");
    e.state.minions[x.index()].blood = cap;
    let ghoul = ghoul_on(&mut e, a);
    let stake = equipped(&mut e, "Wooden Stake", a);
    hand(&mut e, 0, &["Relentless Pursuit"]);
    ghoul_fight(&mut e, a, x, ghoul, stake);
    pick_when(&mut e, play(0, 0, a));
    pick_when(&mut e, weapon(stake, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[x.index()];
    assert!(
        v.in_torpor && v.blood == cap - 4,
        "2 by the stake, blood left"
    );
    assert!(on_itself(&e, stake, x, 0, "Wooden Stake"));
}

/// Red reading: "1 strength" printed on the Ghoul Retainer is the strength
/// of its weapon use: A (strength 3) has the ghoul swing its Bang Nakh for
/// 1 + 2, not 3 + 2.
#[test]
fn ghoul_retainer_swings_a_melee_weapon_with_its_own_strength() {
    let (mut e, a, x) = duel(&[], &[]);
    e.state.minions[x.index()].blood = 8;
    e.state.minions[a.index()].timed.push((
        zvtes::cards::Lasting::Strength(2),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    let ghoul = ghoul_on(&mut e, a);
    let nakh = on(&mut e, "Bang Nakh — Tiger's Claws", a);
    let cap = e.state.capacity_of(x).unwrap();
    ghoul_fight(&mut e, a, x, ghoul, nakh);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        blood(&e, x),
        cap.min(8) - 3 - 3,
        "the ghoul's 3, A's hand 3"
    );
}

/// Zip Gun: "Ammo cards cannot be used with this gun." (Its "no ammo card
/// in the pool" was stale: Scattershot is.) A strikes with its Zip Gun: no
/// Scattershot offered; with a .44 Magnum, offered.
#[test]
fn zip_gun_takes_no_ammo() {
    for (gun, offered) in [("Zip Gun", false), (".44 Magnum", true)] {
        let (mut e, a, x) = duel(&[], &[]);
        let g = on(&mut e, gun, a);
        hand(&mut e, 0, &["Scattershot"]);
        fight(&mut e, a, x);
        pick_when(&mut e, weapon(g, 0));
        let mut seen = false;
        loop {
            let req = request(&mut e);
            seen |= req.options.contains(&play(0, 0, a));
            if req.window == WindowLabel::MinionPhase {
                break;
            }
            e.choose(fallback(&req));
        }
        assert_eq!(seen, offered, "{gun}");
    }
}

/// Wendell Delburton: "Adding damage to a strike that does not deal damage
/// has no effect, so the strike still will not deal damage (eg. … Rowan
/// Ring)." [RTR 19960221] His Rowan Ring sends X to torpor, X unhurt.
#[test]
fn wendell_adds_nothing_to_rowan_ring() {
    let mut e = game(2);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let (w, id) = ally(&mut e, "Wendell Delburton");
    let ring = equipped(&mut e, "Rowan Ring", w);
    pick(&mut e, use_at(id, 2, Some(w), Target::Minion(x)));
    pick_when(&mut e, weapon(ring, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[x.index()];
    assert!(v.in_torpor && v.blood == 4);
}

// ---- Keepers of Tradition vampires, Kalila to Rathmere ---------------------

/// Whether `o` uses vampire `m`'s printed ability `ability`.
fn uses_vampire(o: &Choice, m: MinionId, ability: u8) -> bool {
    matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability: i, .. }
        if *v == m && *i == ability)
}

/// A card in play for `seat`, on no minion (a location, an event).
fn in_play_for(e: &mut Engine, seat: u8, name: &str) -> CardId {
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), SeatId(seat), None, 0));
    CardId(e.state.cards.len() as u32 - 1)
}

fn diablerize(m: MinionId, victim: MinionId) -> Choice {
    Choice::TakeAction {
        minion: m,
        action: zvtes::decision::ActionKind::Diablerie,
        target: Some(victim),
    }
}

/// Kalila: "+1 stealth during actions directed at a location." Her Bomb's
/// Ⓓ action at another Methuselah's location: stealth 1; at her own
/// Methuselah's location the action is undirected (rules-qa 17.2): 0.
#[test]
fn kalila_gets_stealth_on_an_action_at_another_location() {
    for (own, stealth) in [(false, 1), (true, 0)] {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), find_card("Kalila"), 3, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        let loc = in_play_for(&mut e, u8::from(!own), "Powerbase: Madrid");
        let bomb = on(&mut e, "Bomb", k);
        pick(
            &mut e,
            Choice::UseAbility {
                source: AbilitySource::Card(bomb),
                ability: 1,
                actor: Some(k),
                target: Target::Card(loc),
            },
        );
        request(&mut e);
        let a = e.action().expect("the Bomb's action");
        assert_eq!(a.stealth(&e.state), stealth, "own location {own}");
    }
}

/// Kalila: a locquipment is "a location once in play" (rules-qa 94.4): her
/// Bomb's Ⓓ action at seat 1's vampire's Winchester Mansion has stealth 1.
#[test]
fn kalila_gets_stealth_on_an_action_at_a_locquipment() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), find_card("Kalila"), 3, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    let mansion = on(&mut e, "Winchester Mansion", x);
    let bomb = on(&mut e, "Bomb", k);
    pick(&mut e, use_at(bomb, 1, Some(k), Target::Card(mansion)));
    request(&mut e);
    assert_eq!(e.action().expect("the Bomb's action").stealth(&e.state), 1);
}

/// Keller Thiel: "If Keller is ready and unlocked, minions burn 1 blood or
/// life after bleeding you successfully." A's bleed lands: A burns 1 blood;
/// Keller locked: nothing; a blocked bleed: nothing; Political Ally (1
/// life) bleeding lands: it burns its life and is burned.
#[test]
fn keller_thiel_makes_a_successful_bleeder_burn_blood() {
    for (locked, blocked, burned) in [(false, false, 1), (true, false, 0), (false, true, 0)] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
        let k = e.add_minion(SeatId(1), find_card("Keller Thiel"), 4, false);
        let x = e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
        e.state.minions[k.index()].locked = locked;
        pick(&mut e, bleed(a));
        if blocked {
            pick_when(&mut e, block(x));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, if blocked { 10 } else { 9 });
        // Blocked: X's hand strike's 1 only.
        let struck = u8::from(blocked);
        assert_eq!(blood(&e, a), 4 - struck - burned, "locked {locked}");
    }
    let mut e = game(2);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let ally = e.add_ally(SeatId(0), find_library("Political Ally"));
    e.add_minion(SeatId(1), find_card("Keller Thiel"), 4, false);
    pick(&mut e, bleed(ally));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 3);
    assert!(!e.state.minions[ally.index()].in_play, "its life burned");
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Political Ally")));
}

/// Keller Thiel: "a bleed of 0 … triggers no 'after a successful bleed'"
/// (rules-qa 24.4): A's 1 bleed at stealth 1 reduced to 0 by X's Keep it
/// Simple: A burns nothing.
#[test]
fn keller_thiel_takes_nothing_for_a_bleed_of_zero() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), find_card("Keller Thiel"), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    e.state.minions[a.index()].next_action_stealth = 1;
    hand(&mut e, 1, &["Keep it Simple"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "a bleed of 0");
    assert_eq!(blood(&e, a), 4);
}

/// Keller Thiel: a bleed ousting his Methuselah sets nothing off (the
/// engine board's "An oust happens as pool reaches 0", owner, 2026-11-11):
/// seat 1 at 1 pool ousted by A's bleed, A burns nothing.
#[test]
fn keller_thiel_takes_nothing_for_a_bleed_that_ousts_his_methuselah() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), find_card("Keller Thiel"), 4, false);
    e.add_minion(SeatId(2), in_caps(4, 8, 2), 4, false);
    e.state.seats[1].pool = 1;
    pick(&mut e, bleed(a));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[1].ousted);
    assert_eq!(blood(&e, a), 4);
}

/// Lord Fianna: "Allies cannot block Lord Fianna. Red List. +1 stealth."
/// His bleed has stealth 1; seat 1's vampire is offered the block, its
/// ally (Political Ally) never.
#[test]
fn lord_fianna_cannot_be_blocked_by_allies() {
    let mut e = game(2);
    let f = e.add_minion(SeatId(0), find_card("Lord Fianna"), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    assert!(e.state.is_red_list(f));
    pick(&mut e, bleed(f));
    let req = offering(&mut e, block(x));
    assert!(!req.options.contains(&block(ally)));
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
    never_offered_until(&mut e, SeatId(0), WindowLabel::MinionPhase, block(ally));
}

/// Lynn Thompson: "Lynn cannot have a haven (instead any haven that she
/// would receive is burned)." Her equip action with Body Bag succeeds: the
/// Body Bag burned; another vampire of hers keeps one. A Secure Haven put
/// on her in the master phase is burned too, its pool paid.
#[test]
fn lynn_thompson_burns_a_haven_she_receives() {
    for lynn in [true, false] {
        let mut e = game(2);
        let l = e.add_minion(SeatId(0), find_card("Lynn Thompson"), 3, false);
        let o = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        hand(&mut e, 0, &["Body Bag"]);
        let m = if lynn { l } else { o };
        pick(&mut e, play(0, 0, m));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let bag = find_library("Body Bag");
        let on_m = e
            .state
            .cards
            .iter()
            .any(|c| c.in_play && c.card == bag && c.host == Some(m));
        assert_eq!(on_m, !lynn, "Lynn {lynn}");
        assert_eq!(e.state.seats[0].ash.contains(&bag), lynn);
    }
    let mut e = game(2);
    let l = e.add_minion(SeatId(0), find_card("Lynn Thompson"), 3, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    hand(&mut e, 0, &["Secure Haven"]);
    e.at_master_phase();
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(l),
        },
    );
    request(&mut e);
    let haven = find_library("Secure Haven");
    assert!(!e.state.cards.iter().any(|c| c.in_play && c.card == haven));
    assert!(e.state.seats[0].ash.contains(&haven));
    assert_eq!(e.state.seats[0].pool, 10 - 1);
}

/// MacAlister Marshall: "Other Tremere get +1 intercept during actions
/// directed at MacAlister." A diablerie aimed at him in torpor: seat 1's
/// Tremere +1, its other vampire 0, MacAlister himself 0; aimed at another
/// torpid vampire of seat 1's: 0.
#[test]
fn macalister_gives_other_tremere_intercept_on_actions_at_him() {
    for at_him in [true, false] {
        let mut e = game(2);
        let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
        let mac = e.add_minion(SeatId(1), find_card("MacAlister Marshall"), 2, true);
        let other = e.add_minion(SeatId(1), in_caps(4, 8, 2), 2, true);
        let tremere = e.add_minion(
            SeatId(1),
            textless(|c| c.clan == "Tremere" && (4..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        let x = e.add_minion(SeatId(1), in_caps(4, 8, 3), 4, false);
        pick(&mut e, diablerize(a, if at_him { mac } else { other }));
        offering(&mut e, block(tremere));
        let act = e.action().expect("diablerie");
        let ctx = act.printed_ctx(&e.state, Some(a));
        let icpt = |m: MinionId| e.state.printed_mod(m, zvtes::cards::Stat::Intercept, ctx);
        assert_eq!(icpt(tremere), i32::from(at_him), "at him {at_him}");
        assert_eq!(icpt(x), 0);
        assert_eq!(icpt(mac), 0, "other Tremere only");
    }
}

/// Mictlantecuhtli: "immune to damage from weapons … +1 strength." [LSJ
/// 20010627] [LSJ 20010629] [LSJ 20090304-1] Immune: none lands (inflicted,
/// never successfully, rules-qa 116.138). X's .44 Magnum strike deals him
/// nothing; X's hand strike still deals 1; his own hand strike deals 2.
#[test]
fn mictlantecuhtli_takes_no_weapon_damage() {
    for gun in [true, false] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Mictlantecuhtli"), 6, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
        let magnum = on(&mut e, ".44 Magnum", x);
        pick(&mut e, bleed(m));
        pick_when(&mut e, block(x));
        if gun {
            pick_when(&mut e, weapon(magnum, 0));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, m), if gun { 6 } else { 6 - 1 }, "gun {gun}");
        assert_eq!(blood(&e, x), 4 - 2, "his strength 2");
    }
}

/// Talbot's Chainsaw: "The damage inflicted during the unlock phase is
/// weapon damage … Mictlantecuhtli can be chosen and would take no damage."
/// [LSJ 20090304-1] [LSJ 20100514]
#[test]
fn mictlantecuhtli_is_chosen_for_the_chainsaw_and_takes_nothing() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mictlantecuhtli"), 6, false);
    let b = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    on(&mut e, "Talbot's Chainsaw", b);
    let req = offering(&mut e, Choice::ChooseMinion(m));
    assert_eq!(req.seat, SeatId(0));
    pick(&mut e, Choice::ChooseMinion(m));
    request(&mut e);
    assert_eq!(blood(&e, m), 6);
    assert!(e.state.minions[m.index()].ready());
}

/// Mictlantecuhtli: "Each of your other ready vampires gains 1 blood after
/// he commits diablerie (before the blood hunt is called)." His ready
/// vampire +1 as the blood hunt opens, his torpid one +0, he gets the
/// victim's 2.
#[test]
fn mictlantecuhtli_feeds_his_ready_vampires_after_diablerie() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mictlantecuhtli"), 4, false);
    let ready = e.add_minion(SeatId(0), in_caps(4, 8, 0), 2, false);
    let torpid = e.add_minion(SeatId(0), in_caps(4, 8, 1), 2, true);
    let v = e.add_minion(SeatId(1), in_caps(4, 8, 2), 2, true);
    e.add_minion(SeatId(1), in_caps(4, 8, 3), 4, false);
    // A vote to cast: the blood hunt stops for a decision.
    e.state.seats[1].has_edge = true;
    pick(&mut e, diablerize(m, v));
    to_referendum(&mut e);
    assert_eq!(blood(&e, ready), 3);
    assert_eq!(blood(&e, torpid), 2);
    assert_eq!(blood(&e, m), 4 + 2);
}

/// Mictlantecuhtli: "Damage done to the bearer is environmental." [LSJ
/// 19970801] (Zip Gun, Bomb): his own Bomb's 5 hits him; X's hand strike's
/// 1 too.
#[test]
fn mictlantecuhtli_takes_his_own_bombs_damage() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mictlantecuhtli"), 7, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let bomb = equipped(&mut e, "Bomb", m);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(x));
    pick_when(&mut e, weapon(bomb, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, m), 7 - 5 - 1);
}

/// An "inflicts" counts damage Mictlantecuhtli is immune to (rules-qa
/// 116.138): X's Weighted Walking Stick (strength+1) hits him for 2, none
/// landing, and burns 2 counters; X's Ghoul Retainer swinging it (its use
/// carries "all side-effects", LSJ 20080210) burns 2 too, only X's hand
/// strike landing; at long range (his Backstep) X's .44 Magnum hits him,
/// none landing, and his Improvised Flamethrower backfires ("even if it is
/// prevented or ignored", LSJ 20040802): burned, its 2 aggravated sending
/// him to torpor.
#[test]
fn mictlantecuhtlis_immunity_still_counts_as_inflicted() {
    let setup = || {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Mictlantecuhtli"), 6, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
        (e, m, x)
    };
    let (mut e, m, x) = setup();
    let stick = equipped(&mut e, "Weighted Walking Stick", x);
    e.state.cards[stick.index()].counters = 5;
    fight(&mut e, m, x);
    pick_when(&mut e, weapon(stick, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, m), 6);
    assert_eq!(e.state.cards[stick.index()].counters, 5 - 2);

    let (mut e, m, x) = setup();
    let stick = equipped(&mut e, "Weighted Walking Stick", x);
    e.state.cards[stick.index()].counters = 5;
    let ghoul = ghoul_on(&mut e, x);
    fight(&mut e, m, x);
    let u = ghoul_use(&mut e, ghoul, stick);
    pick(&mut e, u);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, m), 6 - 1, "X's hand strike only");
    assert_eq!(e.state.cards[stick.index()].counters, 5 - 2);

    let (mut e, m, x) = setup();
    let flame = equipped(&mut e, "Improvised Flamethrower", m);
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 0, &["Backstep"]);
    fight(&mut e, m, x);
    pick_when(&mut e, play(0, 0, m));
    pick_when(&mut e, weapon(gun, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[flame.index()].in_play, "burned");
    assert!(e.state.minions[m.index()].in_torpor, "its 2 aggravated");
}

/// Carolina Vález's immunity to allies is no reduction (rules-qa 116.138):
/// a blocking Aggressive Corpse's Weighted Walking Stick hits her for 3,
/// none landing, and burns 3 counters all the same.
#[test]
fn carolinas_immunity_still_counts_as_inflicted() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), find_card("Carolina Vález"), 6, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Aggressive Corpse"));
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    let stick = equipped(&mut e, "Weighted Walking Stick", ally);
    e.state.cards[stick.index()].counters = 5;
    fight(&mut e, c, ally);
    pick_when(&mut e, weapon(stick, 0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, c), 6);
    assert_eq!(
        e.state.cards[stick.index()].counters,
        5 - 3,
        "strength 2 + 1"
    );
}

/// Mictlantecuhtli: "Additional damage inherits all of the properties of
/// the base damage" [TOM 19960225]: Dragon's Breath Rounds' 2 aggravated
/// on X's .44 Magnum is weapon damage, none lands on him (116.138); X's Ghoul
/// Retainer swinging X's Bang Nakh at him deals none either (this round's
/// Ghoul Retainer reading: its weapon use is the weapon's).
#[test]
fn mictlantecuhtli_takes_no_added_or_retainer_weapon_damage() {
    let setup = || {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), find_card("Mictlantecuhtli"), 6, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
        (e, m, x)
    };
    let (mut e, m, x) = setup();
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 1, &["Dragon's Breath Rounds"]);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(x));
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play(0, 0, x));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(ash_has(&e, 1, "Dragon's Breath Rounds"), "played");
    assert_eq!(blood(&e, m), 6);
    assert!(!e.state.minions[m.index()].in_torpor);

    let (mut e, m, x) = setup();
    let nakh = equipped(&mut e, "Bang Nakh — Tiger's Claws", x);
    let ghoul = ghoul_on(&mut e, x);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(x));
    let u = ghoul_use(&mut e, ghoul, nakh);
    pick(&mut e, u);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, m), 6 - 1, "X's hand strike only");
}

/// Mictlantecuhtli's immunity is his own: X's .44 Magnum aimed at his
/// Twisted Bloodhound (Target Retainer) takes its 2 life.
#[test]
fn mictlantecuhtlis_retainer_takes_weapon_damage() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mictlantecuhtli"), 6, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let hound = retainer(&mut e, "Twisted Bloodhound", m, 3);
    let gun = on(&mut e, ".44 Magnum", x);
    hand(&mut e, 1, &["Target Retainer"]);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(x));
    pick_when(&mut e, weapon(gun, 0));
    pick_when(&mut e, play_on(0, 0, x, Target::Card(hound)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[hound.index()].counters, 3 - 2);
    assert_eq!(blood(&e, m), 6);
}

/// Mictlantecuhtli: an unsuccessful diablerie (Byzar's) opens no
/// after-diablerie window (rules-qa 113.9): his ready vampire gains
/// nothing.
#[test]
fn mictlantecuhtli_feeds_nobody_after_an_unsuccessful_diablerie() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Mictlantecuhtli"), 4, false);
    let ready = e.add_minion(SeatId(0), in_caps(4, 8, 0), 2, false);
    let by = e.add_minion(SeatId(1), find_card("Byzar"), 3, true);
    e.add_minion(SeatId(1), in_caps(4, 8, 3), 4, false);
    pick(&mut e, diablerize(m, by));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[by.index()].in_play, "Byzar survives");
    assert_eq!(blood(&e, ready), 2);
    assert_eq!(blood(&e, m), 4);
}

/// Miguel Cordovera: "During your predator's minion phase, they can burn 1
/// pool to unlock and take control of Miguel until the end of the minion
/// phase. +1 bleed." Seat 0 (seat 1's predator) burns 1 pool: Miguel,
/// locked, is unlocked and seat 0's; he bleeds seat 1 for 2; back to seat 1
/// as seat 0's minion phase ends, before its turn's end.
#[test]
fn miguel_cordovera_is_borrowed_by_the_predator_for_a_minion_phase() {
    let mut e = game(3);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Miguel Cordovera"), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(2), in_caps(4, 8, 2), 4, false);
    e.state.minions[g.index()].locked = true;
    // Something to influence: the influence phase stops for a decision.
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: in_caps(4, 8, 3),
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let borrow = *req
        .options
        .iter()
        .find(|o| uses_vampire(o, g, 0))
        .expect("offered to the predator");
    e.choose(index_of(&req, borrow));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10 - 1);
    let v = &e.state.minions[g.index()];
    assert!(!v.locked);
    assert_eq!(v.owner, SeatId(0));
    assert!(!req.options.iter().any(|o| uses_vampire(o, g, 0)));
    e.choose(index_of(&req, bleed(g)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    e.choose(index_of(&req, Choice::Pass));
    expect(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert_eq!(e.state.minions[g.index()].owner, SeatId(1));
    assert!(e.state.minions[g.index()].temp_from.is_none());
}

/// Miguel Cordovera: no "if ready" (rules-digest D25): his predator may
/// take him in torpor, as he is.
#[test]
fn miguel_cordovera_can_be_taken_from_torpor() {
    let mut e = game(3);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Miguel Cordovera"), 2, true);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(2), in_caps(4, 8, 2), 4, false);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let borrow = *req
        .options
        .iter()
        .find(|o| uses_vampire(o, g, 0))
        .expect("offered");
    e.choose(index_of(&req, borrow));
    request(&mut e);
    let v = &e.state.minions[g.index()];
    assert_eq!(v.owner, SeatId(0));
    assert!(v.in_torpor);
}

/// Miguel Cordovera: the predator burning their last pool for him is
/// ousted (rules-digest D3) and takes no control: Miguel stays seat 1's.
#[test]
fn miguel_cordovera_borrowed_with_the_last_pool_stays_put() {
    let mut e = game(3);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let g = e.add_minion(SeatId(1), find_card("Miguel Cordovera"), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(2), in_caps(4, 8, 2), 4, false);
    e.state.minions[g.index()].locked = true;
    e.state.seats[0].pool = 1;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let borrow = *req
        .options
        .iter()
        .find(|o| uses_vampire(o, g, 0))
        .expect("offered to the predator");
    e.choose(index_of(&req, borrow));
    request(&mut e);
    assert!(e.state.seats[0].ousted);
    let v = &e.state.minions[g.index()];
    assert_eq!(v.owner, SeatId(1));
    assert!(v.temp_from.is_none());
}

/// Pass the political action's referendum, `caller` voting for.
fn pass_vote(e: &mut Engine, caller: MinionId) {
    for _ in 0..200 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(|o| *o == votes(caller, true)) {
            e.choose(i);
            close_polling(e);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("no vote offered");
}

/// Montecalme: "can call a referendum to burn any location as a +1 stealth
/// political action." The action has stealth 1; any location may be chosen,
/// his Methuselah's too; passed, the chosen one of seat 1's burns.
#[test]
fn montecalme_burns_a_location_by_referendum() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Montecalme"), 6, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    let loc = in_play_for(&mut e, 1, "Powerbase: Madrid");
    let own = in_play_for(&mut e, 0, "Academic Hunting Ground");
    let call = ability_of(&mut e, AbilitySource::Vampire(m), 0);
    pick(&mut e, call);
    request(&mut e);
    assert_eq!(e.action().expect("political").stealth(&e.state), 1);
    let req = offering(&mut e, Choice::ChooseCard(loc.0));
    assert!(
        req.options.contains(&Choice::ChooseCard(own.0)),
        "his own too"
    );
    e.choose(index_of(&req, Choice::ChooseCard(loc.0)));
    pass_vote(&mut e, m);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[loc.index()].in_play);
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Powerbase: Madrid")));
}

/// "Cannot be used or played if the conditions for the terms of the
/// referendum cannot be met (e.g. no legal selection …)." [LSJ 20100129]
/// [ANK 20191228] No location in play: Montecalme's action not offered.
#[test]
fn montecalme_cannot_call_with_no_location_in_play() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Montecalme"), 6, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.iter().any(|o| uses_vampire(o, m, 0)));
}

/// Montecalme: the referendum passed with the chosen location gone does
/// nothing (rules-digest D1): seat 1's Powerbase: Madrid burned meanwhile,
/// his own location stays, the action over.
#[test]
fn montecalme_burns_nothing_once_the_location_is_gone() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), find_card("Montecalme"), 6, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    let loc = in_play_for(&mut e, 1, "Powerbase: Madrid");
    let own = in_play_for(&mut e, 0, "Academic Hunting Ground");
    let call = ability_of(&mut e, AbilitySource::Vampire(m), 0);
    pick(&mut e, call);
    pick_when(&mut e, Choice::ChooseCard(loc.0));
    e.burn_card(loc);
    pass_vote(&mut e, m);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.action().is_none());
    assert!(e.state.cards[own.index()].in_play);
    let madrid = find_library("Powerbase: Madrid");
    assert_eq!(
        e.state.seats[1]
            .ash
            .iter()
            .filter(|&&c| c == madrid)
            .count(),
        1
    );
}

/// Pedrag Hasek: "Pedrag cannot commit diablerie." No diablerie action
/// offered him (another vampire's is); blocking a leave-torpor action
/// (Eagle's Sight's intercept), no diablerie offered.
#[test]
fn pedrag_hasek_cannot_diablerize() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Pedrag Hasek"), 4, false);
    let o = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let v = e.add_minion(SeatId(1), in_caps(4, 8, 1), 2, true);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&diablerize(o, v)));
    assert!(!req.options.contains(&diablerize(p, v)));

    let mut e = game(2);
    let t = e.add_minion(SeatId(0), in_caps(4, 8, 0), 2, true);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    let p = e.add_minion(SeatId(1), find_card("Pedrag Hasek"), 4, false);
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick(&mut e, action(t, zvtes::decision::ActionKind::LeaveTorpor));
    pick_when(&mut e, block(p));
    pick_when(&mut e, play(0, 0, p));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        Choice::Diablerise,
    );
    let v = &e.state.minions[t.index()];
    assert!(v.in_play && v.in_torpor, "blocked, not diablerized");
}

/// Philippe de Marseilles: "If there are any Gehenna cards in play during
/// your unlock phase, you can remove Philippe from the game to gain 6
/// pool." Used from torpor: removed (not burned), +6 pool; with no Gehenna
/// card in play, never offered.
#[test]
fn philippe_de_marseilles_leaves_the_game_for_six_pool() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Philippe de Marseilles"), 3, true);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    in_play_for(&mut e, 1, "Unmasking, The");
    let u = ability_of(&mut e, AbilitySource::Vampire(p), 0);
    let before = e.state.seats[0].pool;
    pick(&mut e, u);
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, before + 6);
    assert!(!e.state.minions[p.index()].in_play);
    let card = find_card("Philippe de Marseilles");
    assert!(e.state.seats[0].removed_crypt.contains(&card));
    assert!(!e.state.seats[0].ash_crypt.contains(&card));

    let mut e = game(2);
    let p = e.add_minion(SeatId(0), find_card("Philippe de Marseilles"), 3, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, Choice::Pass);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_vampire(o, p, 0)
    });
}

/// Rathmere: "+1 strength in combat with a Sabbat vampire." His hand strike
/// deals 2 to a Sabbat blocker, 1 to a Camarilla one.
#[test]
fn rathmere_strikes_harder_against_sabbat() {
    use zvtes::crypt::Sect;
    for (sect, dealt) in [(Sect::Sabbat, 2), (Sect::Camarilla, 1)] {
        let mut e = game(2);
        let r = e.add_minion(SeatId(0), find_card("Rathmere"), 4, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        let x = e.add_minion(
            SeatId(1),
            textless(|c| c.sect == Some(sect) && (4..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        pick(&mut e, bleed(r));
        pick_when(&mut e, block(x));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, x), 4 - dealt, "{sect:?}");
    }
}

// ---- Keepers of Tradition vampires, Viktor to Zelios -----------------------

/// The first offer of vampire `m`'s ability `ability` at `target`, the
/// fallback taken until then.
fn vampire_use_on(e: &mut Engine, m: MinionId, ability: u8, target: Target) -> Choice {
    for _ in 0..300 {
        let req = request(e);
        if let Some(o) = req.options.iter().find(|o| {
            uses_vampire(o, m, ability)
                && matches!(o, Choice::UseAbility { target: t, .. } if *t == target)
        }) {
            return *o;
        }
        e.choose(fallback(&req));
    }
    panic!("{m:?}'s ability {ability} at {target:?} never offered");
}

/// `n` library cards for `seat`, to replace from.
fn stock(e: &mut Engine, seat: usize, name: &str, n: usize) {
    for _ in 0..n {
        e.state.seats[seat]
            .library
            .put(find_library(name), zvtes::copies::CopyId::NONE);
    }
}

/// Sheva Carr: "+1 strength in combat with a Lasombra." Her hand strike
/// deals 2 to a Lasombra blocker, 1 to a Toreador one.
#[test]
fn sheva_carr_strikes_harder_against_lasombra() {
    for (clan, dealt) in [("Lasombra", 2), ("Toreador", 1)] {
        let mut e = game(2);
        let v = e.add_minion(SeatId(0), find_card("Sheva Carr"), 4, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        let x = e.add_minion(
            SeatId(1),
            textless(|c| c.clan == clan && (4..=8).contains(&c.capacity), 0),
            4,
            false,
        );
        e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        pick(&mut e, bleed(v));
        pick_when(&mut e, block(x));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(blood(&e, x), 4 - dealt, "{clan}");
    }
}

/// Themistocles: "can burn a location as a Ⓓ action that costs 1 blood."
/// At seat 1's location: unblocked, it is in its owner's ash heap and he
/// paid 1; blocked, the location stays and nothing is paid (rules-digest
/// D20).
#[test]
fn themistocles_burns_a_location_for_one_blood() {
    for blocked in [false, true] {
        let mut e = game(2);
        let t = e.add_minion(SeatId(0), find_card("Themistocles"), 4, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
        let loc = in_play_for(&mut e, 1, "Powerbase: Madrid");
        let u = vampire_use_on(&mut e, t, 0, Target::Card(loc));
        pick(&mut e, u);
        if blocked {
            pick_when(&mut e, block(x));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.cards[loc.index()].in_play, blocked);
        assert_eq!(ash_has(&e, 1, "Powerbase: Madrid"), !blocked);
        // Blocked: X's hand strike's 1.
        assert_eq!(blood(&e, t), 4 - 1, "blocked {blocked}");
    }
}

/// Themistocles' 1 blood on a fizzle (rules-qa 117.1): the location gone
/// by resolution (cause 1/), "the action is not paid": his 4 blood kept;
/// his blood gone by resolution (cause 3/), he "must pay as much as
/// possible, and the action fizzles": nothing to pay, the location stays.
#[test]
fn themistocles_fizzles_unpaid_on_a_gone_location_and_short_on_blood() {
    for gone in [true, false] {
        let mut e = game(2);
        let t = e.add_minion(SeatId(0), find_card("Themistocles"), 4, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        let loc = in_play_for(&mut e, 1, "Powerbase: Madrid");
        let u = vampire_use_on(&mut e, t, 0, Target::Card(loc));
        pick(&mut e, u);
        expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
        if gone {
            e.burn_card(loc);
        } else {
            e.state.minions[t.index()].blood = 0;
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.last_outcome,
            Some(zvtes::actions::Outcome::Fizzled),
            "gone {gone}"
        );
        if gone {
            assert_eq!(blood(&e, t), 4, "not paid");
        } else {
            assert_eq!(blood(&e, t), 0);
            assert!(e.state.cards[loc.index()].in_play, "no effect");
        }
    }
}

/// Talbot: "You can discard a master card to have Talbot attempt to enter
/// combat with a minion as a +1 stealth Ⓓ action." The discard is the
/// action's cost, paid as it resolves (rules-digest D20: "An action's costs
/// are paid as it resolves, never at announcement"; rules-qa 59.1): the
/// master still in hand during the action, discarded at resolution and the
/// combat entered; blocked, the card kept; the master gone by resolution,
/// the action fizzles, no combat (rules-qa 117.1). No master in hand: not
/// offered.
#[test]
fn talbot_discards_a_master_to_enter_combat() {
    let madrid = find_library("Powerbase: Madrid");
    for case in ["unblocked", "blocked", "gone"] {
        let mut e = game(2);
        let t = e.add_minion(SeatId(0), find_card("Talbot"), 4, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        // Kite: "+1 intercept", Talbot's +1 stealth met.
        let y = e.add_minion(SeatId(1), find_card("Kite"), 4, false);
        hand(&mut e, 0, &["Powerbase: Madrid"]);
        let u = vampire_use_on(&mut e, t, 0, Target::Minion(x));
        pick(&mut e, u);
        request(&mut e);
        assert_eq!(e.action().expect("Talbot's action").stealth(&e.state), 1);
        assert!(
            e.state.seats[0].hand.contains(&madrid),
            "{case}: not paid yet"
        );
        match case {
            "blocked" => pick_when(&mut e, block(y)),
            "gone" => {
                let k = e.state.seats[0]
                    .hand
                    .iter()
                    .position(|&c| c == madrid)
                    .unwrap();
                e.state.seats[0].hand.take(k);
            }
            _ => {}
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        match case {
            "unblocked" => {
                assert!(ash_has(&e, 0, "Powerbase: Madrid"));
                assert!(!e.state.seats[0].hand.contains(&madrid));
                assert_eq!(blood(&e, x), 4 - 1, "Talbot's hand strike");
            }
            "blocked" => {
                assert!(e.state.seats[0].hand.contains(&madrid), "kept");
                assert!(!ash_has(&e, 0, "Powerbase: Madrid"));
                assert_eq!((blood(&e, x), blood(&e, y)), (4, 4 - 1));
            }
            _ => {
                assert_eq!(blood(&e, x), 4, "no combat");
                assert_eq!(blood(&e, t), 4);
                assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
            }
        }
    }

    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Talbot"), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    hand(&mut e, 0, &["Leverage"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.iter().any(|o| uses_vampire(o, t, 0)));
}

/// Shiloh Marie: "can enter combat with a younger vampire as a Ⓓ action."
/// Seat 1's younger vampire is offered, an older one and one of equal
/// capacity are not (rules-digest D30).
#[test]
fn shiloh_marie_enters_combat_with_a_younger_vampire() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Shiloh Marie, Vengeance"), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let young = e.add_minion(SeatId(1), in_caps(3, 5, 0), 3, false);
    let equal = e.add_minion(SeatId(1), in_caps(6, 6, 0), 4, false);
    let old = e.add_minion(SeatId(1), in_caps(7, 7, 0), 4, false);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let at = |m: MinionId| {
        req.options.iter().any(|o| {
            uses_vampire(o, s, 0)
                && matches!(o, Choice::UseAbility { target: t, .. } if *t == Target::Minion(m))
        })
    };
    assert!(at(young));
    assert!(!at(equal));
    assert!(!at(old));
}

/// Shiloh Marie: "If she is ready during your unlock phase, another ready
/// vampire you control takes 1 unpreventable damage." Her controller picks
/// one of the two others (never her): it loses 1 blood. In torpor, nothing.
#[test]
fn shiloh_marie_hurts_another_of_her_vampires_each_unlock() {
    for torpor in [false, true] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), find_card("Shiloh Marie, Vengeance"), 4, torpor);
        let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        let b = e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
        pick(&mut e, Choice::Pass);
        if torpor {
            never_offered_until(
                &mut e,
                SeatId(0),
                WindowLabel::MinionPhase,
                Choice::ChooseMinion(a),
            );
            assert_eq!((blood(&e, a), blood(&e, b)), (4, 4));
            continue;
        }
        let req = offering(&mut e, Choice::ChooseMinion(a));
        assert_eq!(req.seat, SeatId(0));
        assert!(req.options.contains(&Choice::ChooseMinion(b)));
        assert!(!req.options.contains(&Choice::ChooseMinion(s)));
        assert!(!req.options.contains(&Choice::Pass), "mandatory");
        e.choose(index_of(&req, Choice::ChooseMinion(b)));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!((blood(&e, a), blood(&e, b), blood(&e, s)), (4, 3, 4));
    }
}

/// William Thorbecke: "During your discard phase, William can burn 2 blood
/// to unlock a minion you control." His locked vampire: unlocked, he burns
/// 2; not offered again that phase; an unlocked one is not offered (rules-qa
/// 48.7); from torpor too (rules-digest D25).
#[test]
fn william_thorbecke_unlocks_a_minion_in_the_discard_phase() {
    for torpor in [false, true] {
        let mut e = game(2);
        let w = e.add_minion(SeatId(0), find_card("William Thorbecke"), 5, torpor);
        let o = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        let u = e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
        pick(&mut e, Choice::Pass);
        e.state.minions[o.index()].locked = true;
        let req = until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
        assert!(!req.options.iter().any(|c| uses_vampire(c, w, 0)
            && matches!(c, Choice::UseAbility { target: t, .. } if *t == Target::Minion(u))));
        let unlock = vampire_use_on(&mut e, w, 0, Target::Minion(o));
        pick(&mut e, unlock);
        // A target left: the once a phase holds, not the lack of one.
        e.state.minions[u.index()].locked = true;
        let req = request(&mut e);
        assert!(!e.state.minions[o.index()].locked, "torpor {torpor}");
        assert_eq!(blood(&e, w), 5 - 2);
        assert!(!req.options.iter().any(|c| uses_vampire(c, w, 0)));
    }
}

/// "The cards must all be discarded at once, then replaced." [LSJ 19981005]
/// [ANK 20230404] Ruxandra locks and spends the discard phase action: three
/// discarded one pick at a time, none replaced until done, then all.
#[test]
fn ruxandra_discards_any_number_then_replaces() {
    ruxandra_discards(false);
}

/// "The ability can be used when in torpor." [LSJ 20011022]
#[test]
fn ruxandra_discards_from_torpor() {
    ruxandra_discards(true);
}

fn ruxandra_discards(torpor: bool) {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Ruxandra"), 4, torpor);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    hand(&mut e, 0, &["Leverage"; 7]);
    stock(&mut e, 0, "Deflection", 5);
    pick(&mut e, Choice::Pass);
    let u = ability_of(&mut e, AbilitySource::Vampire(r), 0);
    pick(&mut e, u);
    for _ in 0..3 {
        pick(&mut e, Choice::Discard(0));
    }
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::Pass));
    assert_eq!(e.state.seats[0].hand.len(), 4, "none replaced yet");
    e.choose(index_of(&req, Choice::Pass));
    request(&mut e);
    let deflections = e.state.seats[0]
        .hand
        .iter()
        .filter(|&&c| c == find_library("Deflection"))
        .count();
    assert_eq!((e.state.seats[0].hand.len(), deflections), (7, 3));
    assert!(e.state.minions[r.index()].locked);
    assert_eq!(
        e.state.seats[0]
            .ash
            .iter()
            .filter(|&&c| c == find_library("Leverage"))
            .count(),
        3
    );
}

/// "The passive ability applies even if the vampire is in torpor." [LSJ
/// 20011022] Ruxandra's leave-torpor action: a Ventrue blocker's intercept
/// is 1 lower, a Toreador's not.
#[test]
fn ruxandra_lowers_ventrue_intercept_from_torpor() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Ruxandra"), 2, true);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let clan =
        |c: &'static str, n| textless(move |v| v.clan == c && (4..=8).contains(&v.capacity), n);
    let ventrue = e.add_minion(SeatId(1), clan("Ventrue", 0), 4, false);
    let toreador = e.add_minion(SeatId(1), clan("Toreador", 0), 4, false);
    pick(&mut e, action(r, zvtes::decision::ActionKind::LeaveTorpor));
    request(&mut e);
    let a = e.action().expect("leave torpor");
    assert_eq!(a.intercept_of(&e.state, ventrue), -1);
    assert_eq!(a.intercept_of(&e.state, toreador), 0);
}

/// Ruxandra: "any number" includes none (rules-digest D14): used, then
/// passed at once: locked, the discard phase action spent (no discard
/// offered), the hand untouched.
#[test]
fn ruxandra_may_discard_no_card() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), find_card("Ruxandra"), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    hand(&mut e, 0, &["Leverage"; 7]);
    stock(&mut e, 0, "Deflection", 5);
    pick(&mut e, Choice::Pass);
    let u = ability_of(&mut e, AbilitySource::Vampire(r), 0);
    pick(&mut e, u);
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::Discard(0)));
    e.choose(index_of(&req, Choice::Pass));
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::Discard(0)), "spent");
    assert!(e.state.minions[r.index()].locked);
    assert_eq!(e.state.seats[0].hand.len(), 7);
    assert!(e.state.seats[0].ash.is_empty());
}

/// The first play of hand card `card` offered now (any mode or target).
fn play_of(e: &mut Engine, card: u8) -> Choice {
    let req = request(e);
    *req.options
        .iter()
        .find(|o| matches!(o, Choice::Play { card: c, .. } if *c == card))
        .unwrap_or_else(|| panic!("card {card} not playable in {:?}", req.options))
}

/// "The additional blood cost is not paid if the action does not
/// succeed." [PIB 20121031] Tryphosa (seat 1) ready: seat 0's Malkavian
/// bleeds with stealth 1 and pays 1 blood as it succeeds; blocked, it pays
/// nothing (the blocker's hand strike's 1 only); Tryphosa in torpor:
/// neither. Her own bleed too.
#[test]
fn tryphosa_taxes_and_hides_every_malkavian_bleed() {
    let malk = textless(
        |c| c.clan == "Malkavian" && (4..=8).contains(&c.capacity),
        0,
    );
    for (torpor, blocked) in [(false, false), (false, true), (true, false)] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), malk, 4, false);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        e.add_minion(SeatId(1), find_card("Tryphosa"), 4, torpor);
        // +1 intercept: the stealth-1 bleed blocked.
        let x = e.add_minion(SeatId(1), find_card("Sergei Voshkov, The Eye"), 4, false);
        pick(&mut e, bleed(m));
        offering(&mut e, block(x));
        let taxed = !torpor;
        assert_eq!(
            e.action().expect("bleed").stealth(&e.state),
            i32::from(taxed),
            "torpor {torpor}"
        );
        if blocked {
            pick(&mut e, block(x));
        }
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let paid = u8::from(taxed && !blocked);
        assert_eq!(
            blood(&e, m),
            4 - paid - u8::from(blocked),
            "{torpor} {blocked}"
        );
        assert_eq!(e.state.seats[1].pool, if blocked { 10 } else { 9 });
    }
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), find_card("Tryphosa"), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    pick(&mut e, bleed(t));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, t), 4 - 1);
}

/// Tryphosa: an action's added cost is "payable … as the action is
/// announced (not offered otherwise)" (rules-qa 42.1): seat 0's Malkavian
/// with 1 blood is offered Govern the Unaligned [dom] (1 blood), not once
/// Tryphosa's +1 makes it 2.
#[test]
fn tryphosa_bars_a_bleed_its_malkavian_cannot_pay() {
    for tryphosa in [false, true] {
        let mut e = game(2);
        let m = e.add_minion(
            SeatId(0),
            textless(
                |c| c.clan == "Malkavian" && (4..=8).contains(&c.capacity),
                0,
            ),
            1,
            false,
        );
        e.state.minions[m.index()].gained_levels = vec!["dom"];
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        let other = if tryphosa {
            find_card("Tryphosa")
        } else {
            in_caps(4, 8, 1)
        };
        e.add_minion(SeatId(1), other, 4, false);
        hand(&mut e, 0, &["Govern the Unaligned"]);
        let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let governs = req
            .options
            .iter()
            .any(|o| matches!(o, Choice::Play { card: 0, actor: Some(a), .. } if *a == m));
        assert_eq!(governs, !tryphosa, "Tryphosa {tryphosa}");
        assert!(req.options.contains(&bleed(m)), "the plain bleed costs 1");
    }
}

/// Tryphosa's added blood is the bleed's cost (rules-qa 117.4: "the
/// surcharges on an action are its cost"; on cause 3/ "they are paid first
/// (42.6), each as far as it goes, then the action's own cost likewise"):
/// the Malkavian's blood gone by resolution, its plain bleed fizzles, seat
/// 1 losing nothing; its Govern the Unaligned [dom] (1 blood) left with 1
/// blood pays Tryphosa's 1, not the card's, and fizzles.
#[test]
fn tryphosas_blood_is_paid_first_as_far_as_it_goes_on_a_fizzle() {
    for card in [false, true] {
        let mut e = game(2);
        let m = e.add_minion(
            SeatId(0),
            textless(
                |c| c.clan == "Malkavian" && (4..=8).contains(&c.capacity),
                0,
            ),
            3,
            false,
        );
        e.state.minions[m.index()].gained_levels = vec!["dom"];
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        e.add_minion(SeatId(1), find_card("Tryphosa"), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        if card {
            hand(&mut e, 0, &["Govern the Unaligned"]);
            pick(&mut e, play(0, 0, m));
        } else {
            pick(&mut e, bleed(m));
        }
        expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
        e.state.minions[m.index()].blood = u8::from(card);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(
            e.last_outcome,
            Some(zvtes::actions::Outcome::Fizzled),
            "card {card}"
        );
        assert_eq!(e.state.seats[1].pool, 10, "no bleed");
        assert_eq!(blood(&e, m), 0, "card {card}: as far as it goes");
    }
}

/// Santaleous: "He can burn 3 blood to cancel a master card as it is
/// played." "Can use his ability to cancel a master card multiple times
/// each turn." [ANK 20200420-2] "When using his ability, the cost of the
/// canceled card must still be paid." [LSJ 20090601-2] "The cost for a
/// canceled non-action-card is still paid" [ANK 20260216]. Seat 0's two
/// masters canceled: both in its ash heap, their pool paid, Santaleous -6.
#[test]
fn santaleous_cancels_master_cards_paying_three_blood_each() {
    let mut e = game(2);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let s = e.add_minion(SeatId(1), find_card("Santaleous"), 8, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    hand(&mut e, 0, &["Channel 10", "Powerbase: Madrid"]);
    e.at_master_phase();
    e.state.seats[0].mpa = 2;
    pick(&mut e, master_play(0));
    let cancel = ability_of(&mut e, AbilitySource::Vampire(s), 0);
    pick(&mut e, cancel);
    pick(&mut e, master_play(0));
    let cancel = ability_of(&mut e, AbilitySource::Vampire(s), 0);
    pick(&mut e, cancel);
    request(&mut e);
    assert_eq!(blood(&e, s), 8 - 6);
    assert_eq!(e.state.seats[0].pool, 10 - 2 - 1);
    assert!(ash_has(&e, 0, "Channel 10"));
    assert!(ash_has(&e, 0, "Powerbase: Madrid"));
    assert!(!e.state.cards.iter().any(|c| c.in_play));
}

/// Santaleous cancels from torpor (rules-digest D25: no "if ready"):
/// seat 0's Channel 10 canceled, its pool paid, his 3 blood burned.
#[test]
fn santaleous_cancels_from_torpor() {
    let mut e = game(2);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let s = e.add_minion(SeatId(1), find_card("Santaleous"), 5, true);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    hand(&mut e, 0, &["Channel 10"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    let cancel = ability_of(&mut e, AbilitySource::Vampire(s), 0);
    pick(&mut e, cancel);
    request(&mut e);
    assert_eq!(blood(&e, s), 5 - 3);
    assert!(ash_has(&e, 0, "Channel 10"));
    assert!(!e.state.cards.iter().any(|c| c.in_play));
    assert_eq!(e.state.seats[0].pool, 10 - 2);
}

/// Santaleous: "+1 bleed for each unique hunting ground in play." Two of
/// two Methuselahs' and a non-unique one (Poacher's Hunting Ground): +2.
#[test]
fn santaleous_bleeds_more_per_unique_hunting_ground() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), find_card("Santaleous"), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 0), 4, false);
    in_play_for(&mut e, 0, "Academic Hunting Ground");
    in_play_for(&mut e, 1, "Zoo Hunting Ground");
    in_play_for(&mut e, 1, "Poacher's Hunting Ground");
    let bleed = e
        .state
        .printed_mod(s, zvtes::cards::Stat::Bleed, Default::default());
    assert_eq!(bleed, 2);
}

/// Reiner Stoschka: "While Reiner is ready, master discipline cards can be
/// played as trifles." Another Methuselah's Reiner: seat 0's first master
/// Discipline card leaves its master phase action (a trifle), the second
/// spends it; Reiner in torpor, the first spends it.
#[test]
fn reiner_stoschka_makes_master_discipline_cards_trifles() {
    for torpor in [false, true] {
        let mut e = game(2);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        e.add_minion(SeatId(1), find_card("Reiner Stoschka"), 4, torpor);
        e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        hand(&mut e, 0, &["Auspex", "Celerity"]);
        e.at_master_phase();
        let p = play_of(&mut e, 0);
        pick(&mut e, p);
        request(&mut e);
        assert_eq!(e.state.seats[0].mpa, u8::from(!torpor), "torpor {torpor}");
        if !torpor {
            let p = play_of(&mut e, 0);
            pick(&mut e, p);
            request(&mut e);
            assert_eq!(e.state.seats[0].mpa, 0, "one trifle a master phase");
        }
    }
}

/// Tomaine: "While Tomaine is ready and your prey controls a ready Sabbat
/// vampire, you[r] prey gets +1 hand size." Seat 1, his controller's prey,
/// with a ready Sabbat vampire: 8, the others 7; the Sabbat vampire or
/// Tomaine in torpor: 7.
#[test]
fn tomaine_raises_his_preys_hand_size_while_they_hold_a_sabbat() {
    let sabbat = textless(
        |c| c.sect == Some(zvtes::crypt::Sect::Sabbat) && (4..=8).contains(&c.capacity),
        0,
    );
    for (tomaine_torpid, sabbat_torpid, size) in
        [(false, false, 8), (false, true, 7), (true, false, 7)]
    {
        let mut e = game(3);
        e.add_minion(SeatId(0), find_card("Tomaine"), 4, tomaine_torpid);
        e.add_minion(SeatId(1), sabbat, 4, sabbat_torpid);
        e.add_minion(SeatId(2), sabbat, 4, false);
        assert_eq!(
            e.state.hand_size(SeatId(1)),
            size,
            "{tomaine_torpid} {sabbat_torpid}"
        );
        assert_eq!(e.state.hand_size(SeatId(0)), 7);
        assert_eq!(e.state.hand_size(SeatId(2)), 7);
    }
}

/// "When using his ability, you do not get to see the card drawn as
/// replacement." [ANK 20211020] [PIB 20121031] Sergei blocks seat 0's bleed:
/// the look offered; the discard (1 blood) offers the action card only,
/// leaves it in seat 0's ash heap, the hand replaced, no look at it after.
#[test]
fn sergei_voshkov_discards_an_action_card_from_the_blocked_hand() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 1), 4, false);
    let s = e.add_minion(SeatId(1), find_card("Sergei Voshkov, The Eye"), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    hand(&mut e, 0, &["Deflection", "Bum's Rush", "Computer Hacking"]);
    stock(&mut e, 0, "Leverage", 6);
    pick(&mut e, bleed(a));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").intercept_of(&e.state, s), 1);
    pick(&mut e, block(s));
    let look = ability_of(&mut e, AbilitySource::Vampire(s), 0);
    pick(&mut e, look);
    let discard = ability_of(&mut e, AbilitySource::Vampire(s), 1);
    e.notes = Some(Vec::new());
    pick(&mut e, discard);
    let req = request(&mut e);
    let rush = find_library("Bum's Rush");
    assert!(req
        .options
        .contains(&Choice::FindCard(find_library("Computer Hacking"))));
    assert!(!req
        .options
        .contains(&Choice::FindCard(find_library("Deflection"))));
    e.choose(index_of(&req, Choice::FindCard(rush)));
    assert_eq!(blood(&e, s), 4 - 1);
    assert!(ash_has(&e, 0, "Bum's Rush"));
    assert!(!e.state.seats[0].hand.contains(&rush));
    assert!(e.state.seats[0].hand.contains(&find_library("Leverage")));
    request(&mut e);
    let looks = noted_looks(&mut e);
    assert!(
        !looks
            .iter()
            .any(|l| l.viewer == SeatId(1) && l.seat == SeatId(0)),
        "{looks:?}"
    );
}

/// Zelios: "If Zelios is ready, you can lock him to pay the pool cost of a
/// location with his blood instead." A 2-pool location: Zelios
/// locks, -2 blood, the pool untouched. With 1 blood, locked or in torpor:
/// not offered (the pool paid).
#[test]
fn zelios_pays_a_locations_pool_cost_with_his_blood() {
    for (bl, locked, torpor, offered) in [
        (3, false, false, true),
        (1, false, false, false),
        (3, true, false, false),
        (3, false, true, false),
    ] {
        let mut e = game(2);
        let z = e.add_minion(SeatId(0), find_card("Zelios"), bl, torpor);
        e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
        e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
        e.state.minions[z.index()].locked = locked;
        hand(&mut e, 0, &["Channel 10"]);
        e.at_master_phase();
        pick(&mut e, master_play(0));
        let req = request(&mut e);
        let lock = Choice::ResolvePrinted { minion: z };
        assert_eq!(
            req.options.contains(&lock),
            offered,
            "{bl} {locked} {torpor}"
        );
        if offered {
            e.choose(index_of(&req, lock));
            request(&mut e);
            assert_eq!(e.state.seats[0].pool, 10);
            assert_eq!(blood(&e, z), bl - 2);
            assert!(e.state.minions[z.index()].locked);
        } else {
            request(&mut e);
            assert_eq!(e.state.seats[0].pool, 10 - 2);
        }
        let loc = find_library("Channel 10");
        assert!(e.state.cards.iter().any(|c| c.in_play && c.card == loc));
    }
}

/// Zelios: "only with that much blood, his last included" (rules-digest
/// D3): at 2 blood he pays Channel 10's 2, left at 0, still ready.
#[test]
fn zelios_pays_a_location_with_his_last_blood() {
    let mut e = game(2);
    let z = e.add_minion(SeatId(0), find_card("Zelios"), 2, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    hand(&mut e, 0, &["Channel 10"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    pick(&mut e, Choice::ResolvePrinted { minion: z });
    request(&mut e);
    assert_eq!(e.state.seats[0].pool, 10);
    let v = &e.state.minions[z.index()];
    assert!(v.blood == 0 && !v.in_torpor && v.locked);
}

/// Zelios: "+1 strength." His hand strike deals 2.
#[test]
fn zelios_strikes_for_two() {
    let mut e = game(2);
    let z = e.add_minion(SeatId(0), find_card("Zelios"), 4, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    fight(&mut e, z, x);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4 - 2);
}

// ---- Vidal Jarbeaux ---------------------------------------------------------

/// Vidal (6 blood) and a plain vampire for seat 0, two for seat 1.
fn vidal_table() -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Vidal Jarbeaux"), 6, false);
    e.add_minion(SeatId(0), in_caps(4, 8, 0), 4, false);
    let x = e.add_minion(SeatId(1), in_caps(4, 8, 1), 4, false);
    let y = e.add_minion(SeatId(1), in_caps(4, 8, 2), 4, false);
    (e, v, x, y)
}

/// The requirements Vidal has met so far.
fn met(e: &Engine, v: MinionId) -> Vec<zvtes::cards::MetKey> {
    e.state.minions[v.index()]
        .met_once
        .0
        .iter()
        .map(|&(k, _)| k)
        .collect()
}

/// Hand card `card` is offered to `m` in `req`, any mode or target.
fn plays_card(req: &zvtes::decision::DecisionRequest, card: u8, m: Option<MinionId>) -> bool {
    req.options
        .iter()
        .any(|o| matches!(o, Choice::Play { card: c, actor, .. } if *c == card && *actor == m))
}

/// "When meeting a requirement to play a card, he is treated as such for
/// all the effects of the card, and only them … not … for effects the card
/// has from being in play." [RTR 20070707] [LSJ 20070708] [ANK 20200705-2]
/// [ANK 20190228] [LSJ 20101013] "If the contract choice is an effect of
/// the card in play (eg. Clandestine Contract), he's not chosen." [LSJ
/// 20070708] [LSJ 20071109] Vidal plays Clandestine Contract (Banu Haqim):
/// it resolves and lands on X, not choosing him; the clan spent, the
/// second copy is not offered him at his next minion phase.
#[test]
fn vidal_meets_a_clan_requirement_once_a_game() {
    use zvtes::cards::MetKey;
    let (mut e, v, x, _) = vidal_table();
    hand(&mut e, 0, &["Clandestine Contract", "Clandestine Contract"]);
    let contract = Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(v),
        target: Target::Minion(x),
    };
    pick(&mut e, contract);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(met(&e, v), vec![MetKey::Clan("Banu Haqim")]);
    let cc = find_library("Clandestine Contract");
    let on_x = e
        .state
        .cards
        .iter()
        .find(|c| c.in_play && c.card == cc && c.host == Some(x))
        .expect("the contract on X");
    assert!(on_x.chosen.is_empty(), "not chosen by the card in play");
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!plays_card(&req, 0, Some(v)));
}

/// "He can meet the requirements for his controller to play any card,
/// including Master cards." [LSJ 20091015] [PIB 20130128] A Tremere master
/// with ready Vidal and no Tremere: played, the clan spent; Vidal in
/// torpor: not offered.
#[test]
fn vidal_meets_his_methuselahs_master_requirement() {
    use zvtes::cards::MetKey;
    for torpor in [false, true] {
        let (mut e, v, _, _) = vidal_table();
        e.state.minions[v.index()].in_torpor = torpor;
        hand(&mut e, 0, &["Academic Hunting Ground"]);
        e.at_master_phase();
        let req = request(&mut e);
        assert_eq!(
            req.options.contains(&master_play(0)),
            !torpor,
            "torpor {torpor}"
        );
        if !torpor {
            e.choose(index_of(&req, master_play(0)));
            request(&mut e);
            assert_eq!(met(&e, v), vec![MetKey::Clan("Tremere")]);
            assert!(e
                .state
                .cards
                .iter()
                .any(|c| c.in_play && c.card == find_library("Academic Hunting Ground")));
        }
    }
}

/// "If a card he plays is canceled as it is played, he is still considered
/// having used the requirement, if any." [ANK 20200710] Santaleous cancels
/// the Tremere master: the clan spent, a second copy not offered.
#[test]
fn vidal_spends_a_requirement_on_a_canceled_card() {
    use zvtes::cards::MetKey;
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), find_card("Vidal Jarbeaux"), 6, false);
    let s = e.add_minion(SeatId(1), find_card("Santaleous"), 6, false);
    hand(
        &mut e,
        0,
        &["Academic Hunting Ground", "Academic Hunting Ground"],
    );
    e.at_master_phase();
    e.state.seats[0].mpa = 2;
    pick(&mut e, master_play(0));
    let cancel = ability_of(&mut e, AbilitySource::Vampire(s), 0);
    pick(&mut e, cancel);
    let req = request(&mut e);
    assert!(ash_has(&e, 0, "Academic Hunting Ground"));
    assert_eq!(met(&e, v), vec![MetKey::Clan("Tremere")]);
    assert!(!req.options.contains(&master_play(0)), "{:?}", req.options);
}

/// "Vampire traits: titles, "Red List", "Flight", non-requirements … are
/// vampire traits." [LSJ 20081120-1] [LSJ 20091016] [LSJ 20100226] Vidal
/// plays Fiendish Tongue (requires a Sabbat vampire): it does not fizzle,
/// bleeding for 3 (his +1); at his next minion phase the second copy is
/// not offered, Recruitment Exercise (requires a Black Hand vampire) is,
/// and Dive Bomb (requires flight).
#[test]
fn vidal_meets_a_sect_and_a_trait_requirement_once_each() {
    use zvtes::cards::MetKey;
    let (mut e, v, _, _) = vidal_table();
    hand(
        &mut e,
        0,
        &[
            "Fiendish Tongue",
            "Fiendish Tongue",
            "Recruitment Exercise",
            "Dive Bomb",
        ],
    );
    let p = play_of(&mut e, 0);
    pick(&mut e, p);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 3);
    assert_eq!(met(&e, v), vec![MetKey::Sect(zvtes::crypt::Sect::Sabbat)]);
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let hand0 = &e.state.seats[0].hand;
    let at = |n: &str| hand0.iter().position(|&c| c == find_library(n)).unwrap() as u8;
    assert!(!plays_card(&req, at("Fiendish Tongue"), Some(v)));
    assert!(plays_card(&req, at("Recruitment Exercise"), Some(v)));
    assert!(plays_card(&req, at("Dive Bomb"), Some(v)));
    let rx = at("Recruitment Exercise");
    let p = *req
        .options
        .iter()
        .find(|o| matches!(o, Choice::Play { card, actor: Some(a), .. } if *card == rx && *a == v))
        .unwrap();
    e.choose(index_of(&req, p));
    request(&mut e);
    assert_eq!(met(&e, v)[1], MetKey::BlackHand);

    // Sabbat paths, slave and circle are vampire traits (owner,
    // 2026-11-12): a Seraph's Tattoo Signal, a slave's Raw Recruit (X in
    // torpor) and the Path of Caine's Augury of Doom offered him; Augury
    // played, the path spent, its second copy no longer offered; with
    // Seraph and slave spent, nor theirs.
    let (mut e, v, x, _) = vidal_table();
    e.state.minions[x.index()].in_torpor = true;
    let names = [
        "Tattoo Signal",
        "Raw Recruit",
        "Augury of Doom",
        "Augury of Doom",
    ];
    hand(&mut e, 0, &names);
    let req = request(&mut e);
    for card in 0..3 {
        assert!(plays_card(&req, card, Some(v)), "{}", names[card as usize]);
    }
    let p = play_of(&mut e, 2);
    pick(&mut e, p);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(met(&e, v), vec![MetKey::Path("Caine")]);
    e.state.minions[v.index()]
        .met_once
        .0
        .extend([MetKey::Seraph, MetKey::Slave].map(|k| (k, 0)));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { actor: Some(a), .. } if *a == v)));
}

/// "Non-requirements (eg. non-Red List) are vampire traits." [LSJ 20081120-1]
/// [LSJ 20091016] [LSJ 20100226] A primogen Vidal plays The Red Question
/// ("Requires a non-titled non-Anarch vampire"): the untitled trait spent, its
/// second copy no longer offered him.
#[test]
fn vidal_meets_a_non_requirement_once() {
    use zvtes::cards::MetKey;
    let (mut e, v, _, _) = vidal_table();
    e.state.minions[v.index()].granted_title = Some(Title::Primogen);
    hand(&mut e, 0, &["Red Question, The", "Red Question, The"]);
    pick(&mut e, bleed(v));
    pick_when(&mut e, play(0, 0, v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(met(&e, v), vec![MetKey::Untitled]);
    // A Camarilla primogen again (the card burned with the change).
    e.state.minions[v.index()].sect_override = Some(Sect::Camarilla);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick(&mut e, bleed(v));
    never_offered_until(&mut e, SeatId(1), WindowLabel::MinionPhase, play(0, 0, v));
}

/// Inside an either/or requirement too (rules-qa 116.130): a primogen
/// Vidal plays Mozambique Allure ("a Laibon or non-titled vampire"), his
/// player choosing Laibon or non-titled; non-titled chosen and spent, he
/// becomes Laibon. A Camarilla primogen again, his crypt empty (Laibon
/// short of it too): the second copy is not offered him.
#[test]
fn vidal_meets_an_either_or_requirement_once() {
    use zvtes::cards::MetKey;
    let (mut e, v, _, _) = vidal_table();
    e.state.minions[v.index()].granted_title = Some(Title::Primogen);
    e.state.seats[0].crypt.push(in_caps(4, 8, 3));
    hand(&mut e, 0, &["Mozambique Allure", "Mozambique Allure"]);
    pick(&mut e, play(0, 0, v));
    let req = request(&mut e);
    let meet = |key| Choice::MeetAs { minion: v, key };
    let untitled = meet(MetKey::Untitled);
    assert_eq!(
        req.options,
        vec![meet(MetKey::Sect(Sect::Laibon)), untitled]
    );
    e.choose(index_of(&req, untitled));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(met(&e, v), vec![MetKey::Untitled]);
    assert_eq!(e.state.sect_of(v), Some(Sect::Laibon));
    e.state.minions[v.index()].sect_override = Some(Sect::Camarilla);
    e.state.seats[0].crypt.take_top();
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!plays_card(&req, 0, Some(v)));
}

/// "When meeting a requirement to play a card, he is treated as such for
/// all the effects of the card." [RTR 20070707] [LSJ 20101013] A primogen
/// Vidal, non-titled spent, plays Mozambique Allure through Laibon, the one
/// alternative left, spent unasked; Laibon for the card, he draws from his
/// crypt and stays Camarilla.
#[test]
fn vidal_meets_the_one_alternative_left_unasked() {
    use zvtes::cards::MetKey;
    let (mut e, v, _, _) = vidal_table();
    e.state.minions[v.index()].granted_title = Some(Title::Primogen);
    e.state.minions[v.index()]
        .met_once
        .0
        .push((MetKey::Untitled, 0));
    e.state.seats[0].crypt.push(in_caps(4, 8, 3));
    let drawn = e.state.seats[0].uncontrolled.len();
    hand(&mut e, 0, &["Mozambique Allure"]);
    pick(&mut e, play(0, 0, v));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        met(&e, v),
        vec![MetKey::Untitled, MetKey::Sect(Sect::Laibon)]
    );
    assert_eq!(e.state.seats[0].uncontrolled.len(), drawn + 1);
    assert_eq!(e.state.sect_of(v), Some(Sect::Camarilla));
}

/// Vidal on a requirement listing alternatives: his player chooses which
/// he meets as the card is played (owner, 2026-11-12). Parity Shift
/// ("prince or justicar"): justicar chosen, prince left for later. "He
/// must choose a title when playing a card required a "titled" vampire …,
/// but can only choose each title once. He can choose the non-descript "X
/// votes" title, but only once." [LSJ 20050128] [PIB 20150306] [PIB
/// 20150307] [ANK 20211019] Reinforcements ("a titled vampire") offers
/// every title unspent and the votes title; votes spent, not again.
#[test]
fn vidal_chooses_which_alternative_he_meets() {
    use zvtes::cards::{MetKey, TitleClass};
    let (mut e, v, _, _) = vidal_table();
    e.state.seats[1].pool = 15;
    hand(&mut e, 0, &["Parity Shift"]);
    let p = play_of(&mut e, 0);
    pick(&mut e, p);
    let req = request(&mut e);
    let as_title = |t| Choice::MeetAs {
        minion: v,
        key: MetKey::Title(t),
    };
    assert_eq!(
        req.options,
        vec![as_title(TitleClass::Prince), as_title(TitleClass::Justicar)]
    );
    e.choose(index_of(&req, as_title(TitleClass::Justicar)));
    request(&mut e);
    assert_eq!(met(&e, v), vec![MetKey::Title(TitleClass::Justicar)]);

    for votes_spent in [false, true] {
        let (mut e, v, _, _) = vidal_table();
        if votes_spent {
            e.state.minions[v.index()]
                .met_once
                .0
                .push((MetKey::Votes, 0));
        }
        hand(&mut e, 0, &["Reinforcements"]);
        let p = play_of(&mut e, 0);
        pick(&mut e, p);
        let req = request(&mut e);
        let votes = Choice::MeetAs {
            minion: v,
            key: MetKey::Votes,
        };
        assert_eq!(req.options.len(), 11 + usize::from(!votes_spent));
        assert!(req.options.contains(&as_title(TitleClass::Magaji)));
        assert_eq!(req.options.contains(&votes), !votes_spent);
        if !votes_spent {
            e.choose(index_of(&req, votes));
            request(&mut e);
            assert_eq!(met(&e, v), vec![MetKey::Votes]);
        }
    }
}

/// "Titles: When meeting a title requirement, he is considered a member of
/// the underlying sect. He can meet a given generic title (eg. Prince) only
/// once…" [LSJ 20050128] [PIB 20150306] [PIB 20150307] [ANK 20211019] "When
/// meeting a title requirement for a political action, the vampire does
/// not get the title's votes for the subsequent referendum." [ANK
/// 20231221] Parity Shift (prince or justicar): prince chosen, Vidal
/// untitled in its referendum, casting no votes; with prince spent the card
/// is still offered (justicar), with both not.
#[test]
fn vidal_meets_a_title_once_each_without_its_votes() {
    use zvtes::cards::{MetKey, TitleClass};
    let (mut e, v, _, _) = vidal_table();
    e.state.seats[1].pool = 15;
    hand(&mut e, 0, &["Parity Shift"]);
    let p = play_of(&mut e, 0);
    pick(&mut e, p);
    pick(
        &mut e,
        Choice::MeetAs {
            minion: v,
            key: MetKey::Title(TitleClass::Prince),
        },
    );
    to_referendum(&mut e);
    assert_eq!(met(&e, v), vec![MetKey::Title(TitleClass::Prince)]);
    assert_eq!(e.state.title_of(v), Title::None);
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::CastVotes { minion, .. } if *minion == v),
    );

    for (spent, offered) in [
        (&[TitleClass::Prince][..], true),
        (&[TitleClass::Prince, TitleClass::Justicar][..], false),
    ] {
        let (mut e, v, _, _) = vidal_table();
        e.state.seats[1].pool = 15;
        let card = find_library("Parity Shift");
        e.state.minions[v.index()].met_once.0 =
            spent.iter().map(|&t| (MetKey::Title(t), card)).collect();
        hand(&mut e, 0, &["Parity Shift"]);
        let req = request(&mut e);
        assert_eq!(plays_card(&req, 0, Some(v)), offered, "{spent:?}");
    }
}

/// "Can meet a sect-specific title (e.g. Justicar) requirement even if the
/// sect does not exist anymore (eg. Fall of the Camarilla)." [ANK 20230503]
/// [LSJ 20050124] Archon (prince or justicar) after Fall of the Camarilla:
/// offered to Vidal, who may choose himself (he is of the title's sect for
/// it).
#[test]
fn vidal_plays_a_title_card_after_the_fall() {
    let (mut e, v, _, _) = vidal_table();
    in_play_for(&mut e, 0, "Fall of the Camarilla");
    hand(&mut e, 0, &["Archon"]);
    let req = request(&mut e);
    assert!(plays_card(&req, 0, Some(v)));
}

/// "If his capacity is … less [than 11 playing Call the Great Beast], the
/// card is put on him with tokens equal to his capacity, but he will not be
/// able to add counters to it later on." [ANK 20180723] Vidal (capacity 8)
/// as a Baali: the card on him with 8 counters; its counter action never
/// offered him.
#[test]
fn vidal_plays_call_the_great_beast_as_a_baali() {
    let (mut e, v, _, _) = vidal_table();
    hand(&mut e, 0, &["Call the Great Beast"]);
    let p = play_of(&mut e, 0);
    pick(&mut e, p);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let beast = find_library("Call the Great Beast");
    let c = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == beast && c.host == Some(v))
        .expect("on Vidal");
    assert_eq!(e.state.cards[c].counters, 8);
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(id), .. } if id.index() == c),
    );
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| matches!(o,
        Choice::UseAbility { source: AbilitySource::Card(id), .. } if id.index() == c)));
}

/// "His ability do not let him fulfill card requirements if they are not
/// played normally, like when using Concealed Weapon for an equipment or
/// Piper or The Summoning for an ally." [LSJ 20050119] Piper (requires an
/// anarch) naming Political Ally (Ventrue): not offered him; naming a
/// clanless ally: offered. Political Ally from hand: his to recruit.
#[test]
fn vidal_meets_no_requirement_for_a_card_played_by_another() {
    for (ally, offered) in [("Political Ally", false), ("Vagabond Mystic", true)] {
        let (mut e, v, _, _) = vidal_table();
        hand(&mut e, 0, &["Piper", ally]);
        e.at_master_phase();
        let req = request(&mut e);
        assert_eq!(plays_card(&req, 0, Some(v)), offered, "{ally}");
    }
    let (mut e, v, _, _) = vidal_table();
    hand(&mut e, 0, &["Political Ally"]);
    let req = request(&mut e);
    assert!(plays_card(&req, 0, Some(v)));
    // Concealed Weapon naming Righteous Blade (Salubri): not offered him;
    // a .44 Magnum: offered.
    for (weapon, offered) in [("Righteous Blade", false), (".44 Magnum", true)] {
        let (mut e, v, x, _) = vidal_table();
        hand(&mut e, 0, &["Concealed Weapon", weapon]);
        fight(&mut e, v, x);
        let mut seen = false;
        loop {
            let req = request(&mut e);
            seen |= plays_card(&req, 0, Some(v));
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break;
            }
            e.choose(fallback(&req));
        }
        assert_eq!(seen, offered, "{weapon}");
    }
}

/// "Orun and Aye cards cannot be placed on him as long as he's not a
/// Laibon." [LSJ 20110611] Orun (put on a Laibon), at a real master phase:
/// offered on the Laibon, never on Vidal.
#[test]
fn vidal_takes_no_orun() {
    let (mut e, v, _, _) = vidal_table();
    let l = e.add_minion(
        SeatId(0),
        textless(|c| c.sect == Some(Sect::Laibon), 0),
        4,
        false,
    );
    hand(&mut e, 0, &["Orun"]);
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let on = |m: MinionId| {
        req.options.iter().any(|o| {
            matches!(o,
            Choice::Play { card: 0, target: Target::Minion(t), .. } if *t == m)
        })
    };
    assert!(on(l));
    assert!(!on(v));
}

/// "You can still use the burn options of cards with a requirement he
/// could meet when you play them, but does not meet while in play." [LSJ
/// 20050225] Sight Beyond Sight (Salubri): with Vidal ready, its burn
/// option is still offered in the unlock phase.
#[test]
fn vidal_leaves_burn_options_open() {
    let (mut e, _, _, _) = vidal_table();
    hand(&mut e, 0, &["Sight Beyond Sight"]);
    stock(&mut e, 0, "Leverage", 2);
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&Choice::Discard(0)));
}

// ---- Charming Lobby and Minor Boon: helpers --------------------------------

/// The `nth` textless vampire of capacity 4 or more, not advanced,
/// matching `f`.
#[track_caller]
fn textless_basic(f: impl Fn(&zvtes::crypt::CryptCard) -> bool, nth: usize) -> u16 {
    textless(|c| !c.adv && c.capacity >= 4 && f(c), nth)
}

/// The `nth` textless vampire with Discipline `disc` as printed (lower
/// case basic, upper case superior).
fn with(disc: &'static str, nth: usize) -> u16 {
    textless_basic(|c| c.disciplines.contains(&disc), nth)
}

/// The `nth` textless vampire without Presence.
fn other(nth: usize) -> u16 {
    textless_basic(
        |c| !c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("pre")),
        nth,
    )
}

/// The hand card `name` named by Charming Lobby's terms, mode 0.
fn named(name: &str) -> Choice {
    Choice::EmployFromHand {
        card: find_library(name),
        mode: 0,
    }
}

const CTIWY: &str = "Can't Take it with You";

// ---- Charming Lobby -----------------------------------------------------------

/// Charming Lobby [pre]: the political action card in hand named as the
/// action is announced, kept in hand through the action; at success it is
/// played (out of hand, its caller's card ballot offered), its referendum
/// called; Charming Lobby itself no political action card (no ballot from
/// a copy in hand). It passing, the next referendum another Methuselah's
/// vampire calls passes with no polling: no ballot offered for the
/// political action card in hand that a polling would. Backs `[RTR
/// 19991001] [LSJ 20101210]`, `[ANK 20220704]` ("played" at resolution),
/// `[PIB 20110719]`, `[LSJ 19980107]` and `[ANK 20220704] [LSJ 20070927]
/// [LSJ 20050407]`.
#[test]
fn charming_lobby_calls_a_hand_political_action_and_the_next_passes() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), with("pre", 0), 4, false);
    e.add_minion(SeatId(0), other(0), 4, false);
    let c = e.add_minion(SeatId(1), other(1), 4, false);
    e.add_minion(SeatId(1), other(2), 4, false);
    e.add_minion(SeatId(2), other(3), 4, false);
    hand(&mut e, 0, &["Charming Lobby", CTIWY, "Brinksmanship"]);
    hand(&mut e, 1, &[CTIWY]);
    hand(&mut e, 2, &["Charming Lobby", CTIWY]);
    let (lobby, ctiwy) = (find_library("Charming Lobby"), find_library(CTIWY));
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(req.options, vec![named(CTIWY), named("Brinksmanship")]);
    e.choose(0);
    request(&mut e);
    assert!(e.state.seats[0].hand.contains(&ctiwy), "kept in hand");
    to_referendum(&mut e);
    assert!(!e.state.seats[0].hand.contains(&ctiwy), "played");
    let req = offering(&mut e, Choice::CastCardVote { favor: true });
    assert_eq!(req.seat, SeatId(0));
    assert!(
        !req.options.contains(&votes(a, true)),
        "no votes of its own"
    );
    e.choose(index_of(&req, Choice::CastCardVote { favor: true }));
    // Seat 2's ballots: its political action card, not its Lobby.
    let req = offering(
        &mut e,
        Choice::BurnForVote {
            card: 1,
            favor: true,
        },
    );
    assert_eq!(req.seat, SeatId(2));
    assert!(!req.options.contains(&Choice::BurnForVote {
        card: 0,
        favor: true
    }));
    close_polling(&mut e);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![11, 11, 11]);
    for card in [lobby, ctiwy] {
        assert!(e.state.seats[0].ash.contains(&card));
    }
    assert_eq!(e.state.auto_pass_any.0, 1);
    pick(&mut e, play(0, 0, c));
    never_offered_where(&mut e, SeatId(2), WindowLabel::MinionPhase, |o| {
        matches!(o, Choice::BurnForVote { .. } | Choice::CastCardVote { .. })
    });
    assert_eq!(pool(&e), vec![12, 12, 12], "passed with no vote");
    assert_eq!(e.state.auto_pass_any.0, 0, "spent");
}

/// Charming Lobby with no referendum to call (no political action card in
/// hand, none a card in play allows): not offered.
#[test]
fn charming_lobby_needs_a_referendum_to_name() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with("pre", 0), 4, false);
    e.add_minion(SeatId(0), other(0), 4, false);
    e.add_minion(SeatId(1), other(1), 4, false);
    hand(&mut e, 0, &["Charming Lobby"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, a)));
    hand(&mut e, 0, &["Charming Lobby", CTIWY]);
    let req = request(&mut e);
    assert!(req.options.contains(&play(0, 0, a)));
}

/// Firebrand named: its Anarch requirement applies to the acting vampire
/// but is not the action's, so CrimethInc. ("after a successful action
/// requiring an Anarch") is not offered after it. Backs `[LSJ 20100725]
/// [LSJ 20100303] [ANK 20180817]` (the group row).
#[test]
fn charming_lobby_named_requirements_are_not_the_actions() {
    let mut e = game(2);
    let a = e.add_minion(
        SeatId(0),
        textless_basic(
            |c| c.sect == Some(Sect::Anarch) && c.disciplines.contains(&"pre"),
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), other(0), 4, false);
    e.add_minion(SeatId(1), other(1), 4, false);
    hand(
        &mut e,
        0,
        &["Charming Lobby", "Firebrand", CTIWY, "CrimethInc."],
    );
    pick(&mut e, play(0, 0, a));
    pick(&mut e, named("Firebrand"));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { card: 1, actor: Some(m), .. } if *m == a),
    );
    assert!(placed_on(&e, "Firebrand", Some(a)));
}

/// Charming Lobby [PRE]: the acting vampire casts 2 votes in its
/// referendum.
#[test]
fn charming_lobby_superior_gives_two_votes_in_its_referendum() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with("PRE", 0), 4, false);
    e.add_minion(SeatId(1), other(0), 4, false);
    hand(&mut e, 0, &["Charming Lobby", CTIWY, "Brinksmanship"]);
    pick(&mut e, play(0, 1, a));
    pick(&mut e, named(CTIWY));
    pick_when(&mut e, votes(a, true));
    close_polling(&mut e);
    assert!(
        e.state.cast_votes.1.contains(&(a, 2)),
        "{:?}",
        e.state.cast_votes
    );
    assert_eq!(pool(&e), vec![11, 11]);
}

/// Charming Lobby canceled as it is played (Direct Intervention): its
/// named card stays in hand, unpaid. Backs `[ANK 20220704] [ANK
/// 20180925-2] [LSJ 20101210] [LSJ 20100130]`.
#[test]
fn charming_lobby_named_card_stays_in_hand_when_canceled() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with("pre", 0), 4, false);
    e.add_minion(SeatId(0), other(0), 4, false);
    e.add_minion(SeatId(1), other(1), 4, false);
    hand(&mut e, 0, &["Charming Lobby", "Brinksmanship", CTIWY]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, named("Brinksmanship"));
    pick_when(&mut e, master_play(0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let s = &e.state.seats[0];
    assert!(s.hand.contains(&find_library("Brinksmanship")));
    assert!(s.ash.contains(&find_library("Charming Lobby")));
    assert_eq!(s.pool, 10);
    assert!(e.referendum().is_none());
}

/// The named card is played without an as-played window: Direct
/// Intervention, offered for Charming Lobby as it is played, never for
/// the card it names (its pool cost paid at resolution). Backs `[LSJ
/// 20090113-3]` (Charming Lobby's row and Direct Intervention's).
#[test]
fn charming_lobby_named_card_cannot_be_canceled() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with("pre", 0), 4, false);
    e.add_minion(SeatId(0), other(0), 4, false);
    e.add_minion(SeatId(1), other(1), 4, false);
    hand(&mut e, 0, &["Charming Lobby", "Brinksmanship", CTIWY]);
    hand(&mut e, 1, &["Direct Intervention"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, named("Brinksmanship"));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert!(req.options.contains(&master_play(0)));
    e.choose(index_of(&req, Choice::Pass));
    never_offered_until(
        &mut e,
        SeatId(0),
        WindowLabel::ReferendumPolling,
        master_play(0),
    );
    assert_eq!(e.state.seats[0].pool, 9, "Brinksmanship's 1 pool");
    assert!(e.state.seats[1]
        .hand
        .contains(&find_library("Direct Intervention")));
}

/// My Kin Against the World named: X chosen as the action succeeds, paid
/// in the acting Ravnos's blood — Ravnos Carnival's counters, which pay
/// action cards' blood, never offered for it. Backs `[ANK 20220704] [LSJ
/// 20100819]` and `[LSJ 20091021]`.
#[test]
fn charming_lobby_pays_its_cards_x_at_resolution_never_by_ravnos_carnival() {
    let mut e = game(2);
    let ravnos = |c: &zvtes::crypt::CryptCard| c.clan == "Ravnos";
    let a = e.add_minion(
        SeatId(0),
        textless_basic(
            |c| {
                ravnos(c)
                    && c.capacity >= 6
                    && c.disciplines.iter().any(|d| d.eq_ignore_ascii_case("pre"))
            },
            0,
        ),
        4,
        false,
    );
    e.add_minion(SeatId(0), other(0), 4, false);
    e.add_minion(
        SeatId(1),
        textless_basic(|c| ravnos(c) && c.capacity <= 5, 0),
        4,
        false,
    );
    let carnival = in_play(&mut e, 0, "Ravnos Carnival", None);
    e.state.cards[carnival.index()].counters = 2;
    hand(
        &mut e,
        0,
        &["Charming Lobby", "My Kin Against the World", CTIWY],
    );
    pick(&mut e, play(0, 0, a));
    pick(&mut e, named("My Kin Against the World"));
    let req = offering(&mut e, set_x(2));
    assert_eq!(req.window, WindowLabel::ResolutionChoice);
    assert_eq!(req.options, (0..=4).map(set_x).collect::<Vec<_>>());
    e.choose(index_of(&req, set_x(2)));
    request(&mut e);
    assert_eq!(blood(&e, a), 2);
    assert_eq!(e.state.cards[carnival.index()].counters, 2);
}

/// Delaying Tactics on Charming Lobby's referendum: the named card back
/// to hand, Charming Lobby to the ash heap; the acting Methuselah's other
/// vampire is not offered Charming Lobby this turn. Backs `[ANK
/// 20201029-2] [LSJ 20030426]` (both cards' rows).
#[test]
fn delaying_tactics_returns_charming_lobbys_named_card() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with("pre", 0), 4, false);
    let b = e.add_minion(SeatId(0), with("pre", 1), 4, false);
    let d = e.add_minion(SeatId(1), other(0), 4, false);
    hand(
        &mut e,
        0,
        &["Charming Lobby", CTIWY, "Brinksmanship", "Charming Lobby"],
    );
    hand(&mut e, 1, &["Delaying Tactics"]);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, named(CTIWY));
    to_referendum(&mut e);
    pick_when(&mut e, play(0, 0, d));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let s = &e.state.seats[0];
    assert!(s.hand.contains(&find_library(CTIWY)));
    assert!(s.ash.contains(&find_library("Charming Lobby")));
    assert_eq!(pool(&e), vec![10, 10]);
    let at = |name: &str| {
        s.hand
            .iter()
            .position(|&c| c == find_library(name))
            .unwrap() as u8
    };
    assert!(req.options.contains(&play(at(CTIWY), 0, b)));
    assert!(!req.options.contains(&play(at("Charming Lobby"), 0, b)));
}

/// Raising the Portcullis in play: Charming Lobby names its burn
/// referendum; passing burns the card, with no caller's card ballot.
/// Charming Lobby is no political action card. Backs `[ANK 20220704] [LSJ
/// 20070927] [LSJ 20050407]` with the hand test.
#[test]
fn charming_lobby_calls_a_referendum_a_card_in_play_allows() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), with("pre", 0), 4, false);
    e.add_minion(SeatId(0), other(0), 4, false);
    e.add_minion(SeatId(1), other(1), 4, false);
    e.state.seats[0].has_edge = true;
    let portcullis = in_play(&mut e, 1, "Raising the Portcullis", None);
    hand(&mut e, 0, &["Charming Lobby", CTIWY]);
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    let use_it = Choice::UseAbility {
        source: AbilitySource::Card(portcullis),
        ability: 0,
        actor: Some(a),
        target: Target::None,
    };
    assert_eq!(req.options, vec![named(CTIWY), use_it]);
    e.choose(1);
    let req = offering(&mut e, Choice::CastEdge { favor: true });
    assert!(!req.options.contains(&Choice::CastCardVote { favor: true }));
    e.choose(index_of(&req, Choice::CastEdge { favor: true }));
    close_polling(&mut e);
    request(&mut e);
    assert!(!e.state.cards[portcullis.index()].present());
    assert!(e.state.seats[1]
        .ash
        .contains(&find_library("Raising the Portcullis")));
}

/// A blood hunt is no referendum a vampire calls: it does not spend the
/// pass Charming Lobby left. Backs `[TOM 19950921]`.
#[test]
fn a_blood_hunt_does_not_spend_charming_lobbys_pass() {
    let mut e = game(2);
    let d = e.add_minion(SeatId(0), other(0), 4, false);
    e.add_minion(SeatId(0), other(1), 4, false);
    let v = e.add_minion(SeatId(1), other(2), 2, true);
    e.state.seats[1].has_edge = true;
    e.state.auto_pass_any.0 = 1;
    pick(
        &mut e,
        Choice::TakeAction {
            minion: d,
            action: ActionKind::Diablerie,
            target: Some(v),
        },
    );
    let req = until(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
    assert!(req.options.contains(&Choice::CastEdge { favor: true }));
    assert_eq!(e.state.auto_pass_any.0, 1);
}

/// Herald of Topheth, playing Charming Lobby as a vampire, names a
/// political action card in hand (an ally plays none otherwise). Backs
/// `[RTR 20020501]` (both cards' rows).
#[test]
fn herald_of_topheth_calls_a_referendum_with_charming_lobby() {
    let mut e = game(2);
    let h = e.add_ally(SeatId(0), find_library("Herald of Topheth"));
    e.add_minion(SeatId(0), other(0), 4, false);
    e.add_minion(SeatId(1), other(1), 4, false);
    hand(&mut e, 0, &[CTIWY, "Charming Lobby", "Brinksmanship"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&play(0, 0, h)), "no political card");
    pick(&mut e, play(1, 0, h));
    pick(&mut e, named(CTIWY));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    close_polling(&mut e);
    request(&mut e);
    assert_eq!(pool(&e), vec![11, 11]);
}

/// Epikasta Rigatos recycles Charming Lobby, her action's card, whatever
/// it called (here a card in play's referendum). Backs `[LSJ 20090430]
/// [ANK 20180628]` (Charming Lobby's row and Epikasta's).
#[test]
fn epikasta_retrieves_charming_lobby_whatever_it_called() {
    let mut e = game(2);
    let epi = e.add_minion(SeatId(0), find_card("Epikasta Rigatos"), 6, false);
    e.add_minion(SeatId(0), other(0), 4, false);
    e.add_minion(SeatId(1), other(1), 4, false);
    let portcullis = in_play(&mut e, 1, "Raising the Portcullis", None);
    hand(&mut e, 0, &["Charming Lobby", CTIWY]);
    pick(&mut e, play(0, 0, epi));
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(portcullis),
            ability: 0,
            actor: Some(epi),
            target: Target::None,
        },
    );
    pick_when(&mut e, votes(epi, true));
    let recycle = Choice::UseAbility {
        source: AbilitySource::Vampire(epi),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick_when(&mut e, recycle);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let s = &e.state.seats[0];
    let lobby = find_library("Charming Lobby");
    assert!(!s.ash.contains(&lobby));
    assert!(s.library.contains(&lobby) || s.hand.contains(&lobby));
    assert!(s.hand.contains(&find_library(CTIWY)));
    assert!(!e.state.cards[portcullis.index()].present());
    // A hand card named: played by Charming Lobby, to the ash heap.
    let mut e = game(2);
    let epi = e.add_minion(SeatId(0), find_card("Epikasta Rigatos"), 6, false);
    e.add_minion(SeatId(0), other(0), 4, false);
    e.add_minion(SeatId(1), other(1), 4, false);
    hand(&mut e, 0, &["Charming Lobby", CTIWY, "Brinksmanship"]);
    pick(&mut e, play(0, 0, epi));
    pick(&mut e, named(CTIWY));
    pick_when(&mut e, recycle);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let s = &e.state.seats[0];
    assert!(!s.ash.contains(&lobby));
    assert!(s.ash.contains(&find_library(CTIWY)));
}

/// Two Blood Brothers of one circle with no printed-text definition.
fn circle_mates() -> (u16, u16) {
    let bb = |i: usize, c: &zvtes::crypt::CryptCard| {
        c.clan == "Blood Brother" && c.circle.is_some() && zvtes::cards::vampire(i as u16).is_none()
    };
    let first = (0..CRYPT.len())
        .find(|&i| bb(i, &CRYPT[i]))
        .expect("a Blood Brother");
    let mate = (0..CRYPT.len())
        .find(|&i| {
            i != first
                && bb(i, &CRYPT[i])
                && CRYPT[i].circle == CRYPT[first].circle
                && CRYPT[i].name != CRYPT[first].name
        })
        .expect("a circle-mate");
    (first as u16, mate as u16)
}

/// Hay Ride after Charming Lobby: the action's card, bound for the ash
/// heap, is Charming Lobby, the circle-mate's to replay — not the card it
/// named.
#[test]
fn hay_ride_replays_charming_lobby_not_its_named_card() {
    let (x, y) = circle_mates();
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), x, 4, false);
    e.state.minions[a.index()].gained_levels = vec!["san", "pre"];
    let b = e.add_minion(SeatId(0), y, 4, false);
    e.add_minion(SeatId(1), other(0), 4, false);
    hand(
        &mut e,
        0,
        &["Charming Lobby", CTIWY, "Brinksmanship", "Hay Ride"],
    );
    pick(&mut e, play(0, 0, a));
    pick(&mut e, named(CTIWY));
    pick_when(&mut e, Choice::CastCardVote { favor: true });
    let hay = Choice::Play {
        card: 1,
        mode: 0,
        actor: Some(a),
        target: Target::Minion(b),
    };
    pick_when(&mut e, hay);
    request(&mut e);
    let next = e.state.seats[0].next_from_ash.0;
    assert_eq!(next, Some((b, find_library("Charming Lobby"))));
}

// ---- Minor Boon ---------------------------------------------------------------

/// Seat 0's `b` bleeds; seat 1's `v`, with no blood, blocks and goes to
/// torpor in the combat; seat 2 holds Minor Boon.
fn boon_combat() -> (Engine, MinionId, MinionId) {
    let mut e = game(3);
    let b = e.add_minion(SeatId(0), other(0), 4, false);
    e.add_minion(SeatId(0), other(1), 4, false);
    let v = e.add_minion(SeatId(1), other(2), 0, false);
    e.add_minion(SeatId(1), other(3), 4, false);
    e.add_minion(SeatId(2), other(4), 4, false);
    hand(&mut e, 2, &["Minor Boon"]);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(v));
    (e, b, v)
}

/// Minor Boon: asked of seat 2 once the combat that sends the vampire to
/// torpor ends; played, the vampire stays ready, the card itself on it.
/// Backs `[RTR 19950509-2]`.
#[test]
fn minor_boon_keeps_a_vampire_out_of_torpor_after_combat() {
    let (mut e, _, v) = boon_combat();
    let save = Choice::Play {
        card: 0,
        mode: 0,
        actor: None,
        target: Target::Minion(v),
    };
    let req = offering(&mut e, save);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(2), WindowLabel::ResolutionChoice)
    );
    e.choose(index_of(&req, save));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[v.index()].ready());
    assert!(placed_on(&e, "Minor Boon", Some(v)));
}

/// Minor Boon declined: the torpor lands with its riders, Bounty's still
/// reading the combat it came from (its vanquisher's counters).
#[test]
fn minor_boon_declined_lets_the_torpor_and_its_riders_land() {
    let (mut e, _, v) = boon_combat();
    let bounty = in_play(&mut e, 1, "Bounty", Some(v));
    e.state.cards[bounty.index()].counters = 2;
    let req = expect_after_combat(&mut e);
    e.choose(index_of(&req, Choice::Pass));
    let req = offering(&mut e, set_x(2));
    assert_eq!(req.seat, SeatId(0));
    e.choose(index_of(&req, set_x(2)));
    request(&mut e);
    assert!(e.state.minions[v.index()].in_torpor);
    assert_eq!(e.state.seats[0].pool, 12);
}

/// Minor Boon declined for two vampires sent to torpor together (seat 0's
/// empty hunter, seat 1's empty blocker): their torpors land together
/// once the last is declined, so under Tension in the Ranks, both
/// Methuselahs at 1 pool, the acting one orders the burns (LSJ
/// 20090208-2; rules-qa 116.58, 118.2).
#[test]
fn minor_boon_declined_twice_lands_the_torpors_together() {
    let mut e = game(3);
    let b = e.add_minion(SeatId(0), other(0), 0, false);
    e.add_minion(SeatId(0), other(1), 4, false);
    let v = e.add_minion(SeatId(1), other(2), 0, false);
    e.state.minions[v.index()].gained_levels = vec!["aus"];
    e.add_minion(SeatId(1), other(3), 4, false);
    e.add_minion(SeatId(2), other(4), 4, false);
    hand(&mut e, 2, &["Minor Boon"]);
    in_play(&mut e, 2, "Tension in the Ranks", None);
    e.state.seats[0].pool = 1;
    e.state.seats[1].pool = 1;
    // The hunt's stealth met: [aus] Eagle's Sight, +1 intercept.
    hand(&mut e, 1, &["Eagle's Sight"]);
    pick_when(&mut e, block(v));
    pick_when(&mut e, play(0, 0, v));
    for m in [b, v] {
        let req = expect_after_combat(&mut e);
        assert!(req
            .options
            .iter()
            .any(|o| matches!(o, Choice::Play { target, .. } if *target == Target::Minion(m))));
        e.choose(index_of(&req, Choice::Pass));
    }
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(0), WindowLabel::ResolutionChoice)
    );
    assert_eq!(
        req.options,
        vec![Choice::ChooseSeat(SeatId(0)), Choice::ChooseSeat(SeatId(1))]
    );
}

/// The Minor Boon request, seat 2's.
fn expect_after_combat(e: &mut Engine) -> zvtes::decision::DecisionRequest {
    for _ in 0..100 {
        let req = request(e);
        if req.seat == SeatId(2) && req.window == WindowLabel::ResolutionChoice {
            return req;
        }
        e.choose(fallback(&req));
    }
    panic!("no Minor Boon request");
}

/// Minor Boon is not played on its player's own turn (out-of-turn, no
/// "usable during your turn"; owner's red R3): the vampire goes to torpor
/// at once.
#[test]
fn minor_boon_is_not_played_on_its_players_own_turn() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), other(0), 4, false);
    e.add_minion(SeatId(0), other(1), 4, false);
    let v = e.add_minion(SeatId(1), other(2), 0, false);
    e.add_minion(SeatId(1), other(3), 4, false);
    hand(&mut e, 0, &["Minor Boon"]);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(v));
    never_offered_where(
        &mut e,
        SeatId(0),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::Play { target: Target::Minion(t), .. } if *t == v),
    );
    assert!(e.state.minions[v.index()].in_torpor);
    assert_eq!(e.state.torpor_wait.0, None);
}

/// Minor Boon's vampire: not offered a bleed at its player, the prey; a
/// bleed at another not redirected to its player. Backs `[PIB
/// 20130711]`.
#[test]
fn minor_boon_bars_bleeding_and_redirecting_to_its_player() {
    // The player is the vampire's prey: no bleed.
    let mut e = game(3);
    let v = e.add_minion(SeatId(0), other(0), 4, false);
    let w = e.add_minion(SeatId(0), other(1), 4, false);
    e.add_minion(SeatId(1), other(2), 4, false);
    e.add_minion(SeatId(2), other(3), 4, false);
    in_play(&mut e, 1, "Minor Boon", Some(v));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&bleed(v)));
    assert!(req.options.contains(&bleed(w)));
    // Another player's: the bleed at the prey not redirected to them.
    let mut e = game(4);
    let v = e.add_minion(SeatId(0), other(0), 4, false);
    e.add_minion(SeatId(0), other(1), 4, false);
    let r = e.add_minion(SeatId(1), with("dom", 0), 4, false);
    e.add_minion(SeatId(2), other(2), 4, false);
    e.add_minion(SeatId(3), other(3), 4, false);
    in_play(&mut e, 2, "Minor Boon", Some(v));
    hand(&mut e, 1, &["Deflection"]);
    pick(&mut e, bleed(v));
    let to = |s: u8| Choice::Play {
        card: 0,
        mode: 0,
        actor: Some(r),
        target: Target::Seat(SeatId(s)),
    };
    let req = offering(&mut e, to(3));
    assert!(!req.options.contains(&to(2)));
}

/// Minor Boon burns as its vampire rescues a vampire of its player's from
/// torpor, not another Methuselah's.
#[test]
fn minor_boon_burns_when_its_vampire_rescues_the_players_vampire() {
    let mut e = game(3);
    let v = e.add_minion(SeatId(0), other(0), 4, false);
    let u = e.add_minion(SeatId(0), other(1), 4, false);
    let mine = e.add_minion(SeatId(1), other(2), 2, true);
    let theirs = e.add_minion(SeatId(2), other(3), 2, true);
    let boon_v = in_play(&mut e, 1, "Minor Boon", Some(v));
    let boon_u = in_play(&mut e, 1, "Minor Boon", Some(u));
    let rescue = |m: MinionId, t: MinionId| Choice::TakeAction {
        minion: m,
        action: ActionKind::Rescue,
        target: Some(t),
    };
    pick(&mut e, rescue(u, theirs));
    pick_when(&mut e, rescue(v, mine));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[mine.index()].in_torpor);
    assert!(!e.state.cards[boon_v.index()].present());
    assert!(e.state.seats[1].ash.contains(&find_library("Minor Boon")));
    assert!(!e.state.minions[theirs.index()].in_torpor);
    assert!(e.state.cards[boon_u.index()].present());
}

/// High Museum of Art, Atlanta's toll to 0 pool ousts: seat 1's copy
/// contested by seat 0's, seat 1 burns 4 of its 3 pool and is ousted at
/// once (a fuzz seed left it at -1, still in the game).
#[test]
fn high_museum_toll_to_zero_ousts_its_holder() {
    let mut e = game(3);
    e.add_minion(SeatId(0), camarilla("Toreador", 0), 4, false);
    e.add_minion(SeatId(1), camarilla("Ventrue", 0), 4, false);
    e.add_minion(SeatId(2), camarilla("Ventrue", 1), 4, false);
    e.state.seats[1].pool = 3;
    let theirs = in_play(&mut e, 1, "High Museum of Art, Atlanta", None);
    hand(&mut e, 0, &["High Museum of Art, Atlanta"]);
    e.at_master_phase();
    pick(&mut e, master_play(0));
    request(&mut e);
    assert!(e.state.cards[theirs.index()].contested);
    assert!(e.state.seats[1].ousted);
    assert_eq!(e.state.seats[1].pool, 0);
}
