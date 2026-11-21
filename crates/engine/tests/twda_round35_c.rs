//! Round 35 (`ADDED_BATCHES`): Bloodlines and Final Nights completed,
//! track C: the vampires Aabbt Kindred to Bajazet, then the next ones.
//! Windows with a single option are elided, so the scripts only stop where
//! a real decision exists.

use zvtes::cards::{AbilitySource, MinionCounter, Target};
use zvtes::copies::CopyId;
use zvtes::crypt::{Sect, Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_library, CardId, CardInPlay};

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

/// The `nth` untitled fixture vampire of capacity 4 or more.
fn plain4(nth: usize) -> u16 {
    textless(|c| c.capacity >= 4, nth)
}

/// The crypt card `name`, its base card (`adv` false) or its advanced one.
#[track_caller]
fn card_of(name: &str, adv: bool) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && c.adv == adv)
        .unwrap_or_else(|| panic!("no {name} (adv {adv})")) as u16
}

/// The base crypt card `name`.
#[track_caller]
fn base(name: &str) -> u16 {
    card_of(name, false)
}

/// Vampire `m`'s printed ability `ability`, used by `actor` at `target`.
fn printed_use(m: MinionId, ability: u8, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Vampire(m),
        ability,
        actor,
        target,
    }
}

/// Any use of vampire `m`'s printed ability `ability`.
fn uses_printed(o: &Choice, m: MinionId, ability: u8) -> bool {
    matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(v), ability: i, .. }
        if *v == m && *i == ability)
}

/// Pass (or strike with the hand) until a use of vampire `m`'s printed
/// ability `ability` is offered, and take it.
#[track_caller]
fn pick_use(e: &mut Engine, m: MinionId, ability: u8) {
    for _ in 0..300 {
        let req = request(e);
        if let Some(i) = req.options.iter().position(|o| uses_printed(o, m, ability)) {
            e.choose(i);
            return;
        }
        e.choose(fallback(&req));
    }
    panic!("{m:?}'s ability {ability} never offered");
}

/// The card `name` of `seat` in play, on `host` if given (no play).
fn put(e: &mut Engine, seat: u8, name: &str, host: Option<MinionId>) -> CardId {
    e.state
        .cards
        .push(CardInPlay::new(find_library(name), SeatId(seat), host, 0));
    CardId(e.state.cards.len() as u32 - 1)
}

fn torpid(e: &Engine, m: MinionId) -> bool {
    let v = &e.state.minions[m.index()];
    v.in_play && v.in_torpor
}

/// The first ready minion of `card` in play.
fn in_play_of(e: &Engine, card: u16) -> Option<MinionId> {
    e.state
        .minions
        .iter()
        .position(|v| v.in_play && v.crypt_card() == Some(card))
        .map(|i| MinionId(i as u32))
}

/// A rescue of `t` by `m`.
fn rescue(m: MinionId, t: MinionId) -> Choice {
    Choice::TakeAction {
        minion: m,
        action: ActionKind::Rescue,
        target: Some(t),
    }
}

/// The votes `m` casts, ready and alone with the Edge-less table: its
/// printed and title votes (`GameState::votes_of`).
fn votes_of(e: &Engine, m: MinionId) -> i32 {
    e.state.votes_held(m) as i32
}

// ---- Aabbt Kindred ----------------------------------------------------------

/// Aabbt Kindred: "cannot perform directed actions unless Nefertiti is
/// ready": no bleed with no Nefertiti, or one in torpor; another
/// Methuselah's ready Nefertiti lets her bleed; so does a Spell of Life
/// mummy made of Nefertiti, [LSJ 20070928-2] [LSJ 20071001-2] "If one is
/// Nefertiti, then Aabbt Kindred will be able to take directed actions".
#[test]
fn aabbt_kindred_acts_directed_only_while_nefertiti_is_ready() {
    for nefertiti in [None, Some(false), Some(true)] {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), base("Aabbt Kindred"), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        if let Some(torpor) = nefertiti {
            e.add_minion(SeatId(1), base("Nefertiti"), 4, torpor);
        }
        let req = request(&mut e);
        assert_eq!(
            req.options.contains(&bleed(k)),
            nefertiti == Some(false),
            "{nefertiti:?}"
        );
    }
    // The mummy: seat 0's Ministry vampire burns five Spells of Life on a
    // Nefertiti of its uncontrolled region.
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), base("Aabbt Kindred"), 4, false);
    e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Ministry" && c.capacity >= 4, 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: base("Nefertiti"),
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    let spells: Vec<CardId> = (0..5)
        .map(|_| in_play(&mut e, 0, "Spell of Life", None))
        .collect();
    let req = request(&mut e);
    assert!(!req.options.contains(&bleed(k)));
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(spells[0]),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    pick(&mut e, Choice::Slot(0));
    let req = request(&mut e);
    assert!(req.options.contains(&bleed(k)), "the mummy named Nefertiti");
}

/// [LSJ 20081120] (an Aabbt action become directed: "See Ambush vs. a
/// minion who untaps", it fizzles) [LSJ 20090518] "While Nefertiti remains
/// in play and ready": Nefertiti sent to torpor during Aabbt's bleed, the
/// bleed fizzles (seat 1 keeps its pool); still ready, it lands.
#[test]
fn aabbt_kindred_bleed_fizzles_when_nefertiti_leaves() {
    for leaves in [false, true] {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), base("Aabbt Kindred"), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        let n = e.add_minion(SeatId(1), base("Nefertiti"), 4, false);
        pick(&mut e, bleed(k));
        request(&mut e);
        assert!(e.action().is_some());
        e.state.minions[n.index()].in_torpor = leaves;
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(
            e.state.seats[1].pool,
            if leaves { 10 } else { 9 },
            "{leaves}"
        );
    }
}

/// Nefertiti (ADV): "can search your crypt for a card named Aabbt Kindred
/// (shuffle afterward), reveal it and move it to your uncontrolled region
/// as a +1 stealth action": the Aabbt Kindred of her crypt moved there.
#[test]
fn nefertiti_finds_an_aabbt_kindred_in_her_crypt() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), card_of("Nefertiti", true), 8, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let k = base("Aabbt Kindred");
    e.state.seats[0].crypt = vec![plain4(2), k].into_iter().collect();
    pick_use(&mut e, n, 0);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].crypt.to_cards(), vec![plain4(2)]);
    let slots: Vec<u16> = e.state.seats[0]
        .uncontrolled
        .iter()
        .map(|u| u.card)
        .collect();
    assert_eq!(slots, vec![k]);
}

/// Spell of Life: [LSJ 20070928-2] "The mummies do not contest non-unique
/// vampires (eg. Aabbt Kindred)": seat 0's mummy of an Aabbt Kindred
/// beside seat 1's Aabbt Kindred, neither contested.
#[test]
fn aabbt_kindred_mummy_does_not_contest_aabbt_kindred() {
    let k = base("Aabbt Kindred");
    let mut e = game(2);
    e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Ministry" && c.capacity >= 4, 0),
        4,
        false,
    );
    let theirs = e.add_minion(SeatId(1), k, 1, false);
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: k,
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    let spells: Vec<CardId> = (0..5)
        .map(|_| in_play(&mut e, 0, "Spell of Life", None))
        .collect();
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(spells[0]),
            ability: 0,
            actor: None,
            target: Target::None,
        },
    );
    pick(&mut e, Choice::Slot(0));
    let mummy = e
        .state
        .minions
        .iter()
        .position(|m| {
            m.in_play
                && matches!(m.kind, zvtes::state::MinionKind::Represented { card, crypt: true, .. } if card == k)
        })
        .expect("a mummy");
    assert!(!e.state.minions[mummy].contested);
    assert!(!e.state.minions[theirs.index()].contested);
}

/// Aabbt Kindred: "can prevent 1 damage each combat": blocking a bleed,
/// the hand strike's 1 damage prevented.
#[test]
fn aabbt_kindred_prevents_one_damage_each_combat() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let k = e.add_minion(SeatId(0), base("Aabbt Kindred"), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(k));
    pick_use(&mut e, k, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, k), 4, "the hit prevented");
}

/// Chain of Command: [LSJ 20040518-2] "If none of the vampires can bleed
/// (eg. Aabbt Kindred), they all go to the bottom of the crypt": the
/// commanded Aabbt Kindred, no Nefertiti in play, is back at once.
#[test]
fn chain_of_command_returns_aabbt_kindred_who_cannot_bleed() {
    let mut e = game(2);
    let elder = e.add_minion(SeatId(0), textless(|c| c.capacity >= 9, 0), 6, false);
    e.state.minions[elder.index()].gained_levels = vec!["dom"];
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let k = base("Aabbt Kindred");
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: k,
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    hand(&mut e, 0, &["Chain of Command"]);
    fill_hand(&mut e, 0);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(elder),
            target: Target::None,
        },
    );
    pick(&mut e, Choice::Slot(0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(in_play_of(&e, k).is_none(), "returned");
    assert_eq!(e.state.seats[0].crypt.first(), Some(&k), "the bottom");
}

// ---- Agaitas ----------------------------------------------------------------

/// Seat 0's Agaitas and a second vampire A with Computer Hacking and a full
/// hand, its library one card (copy 500); seat 1's vampire and library top
/// `prey_top` (copy 600).
fn agaitas_game(prey_top: &str) -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let ag = e.add_minion(
        SeatId(0),
        base("Agaitas, The Scholar of Antiquities"),
        4,
        false,
    );
    let a = e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Computer Hacking"]);
    fill_hand(&mut e, 0);
    fill_hand(&mut e, 1);
    e.state.seats[0]
        .library
        .put(find_library("Leverage"), CopyId(500));
    e.state.seats[1]
        .library
        .put(find_library(prey_top), CopyId(600));
    (e, ag, a)
}

/// Agaitas: "When you play a card, you may draw its replacement from your
/// prey's library instead of your own. Put the card drawn face-up …": the
/// played Computer Hacking's replacement from seat 1's library, face up
/// (seat 1 sees it in seat 0's hand); played in turn, it goes to its
/// owner's ash heap, [RTR 19970425] [LSJ 20060921] "Cards from his ability
/// that go into an ash heap … always go into their owner's".
#[test]
fn agaitas_draws_a_replacement_from_the_prey_face_up() {
    let (mut e, _, a) = agaitas_game("Computer Hacking");
    pick(&mut e, play(0, 0, a));
    let req = expect(&mut e, SeatId(0), WindowLabel::Draw);
    assert_eq!(
        req.options,
        vec![Choice::DrawLibrary, Choice::ChooseSeat(SeatId(1))]
    );
    e.choose(index_of(&req, Choice::ChooseSeat(SeatId(1))));
    let hand = &e.state.seats[0].hand;
    let i = hand
        .copies()
        .iter()
        .position(|&c| c == CopyId(600))
        .expect("the prey's card in hand");
    assert_eq!(hand.to_cards()[i], find_library("Computer Hacking"));
    assert!(e.state.seats[1].library.is_empty());
    assert_eq!(
        e.state.seats[0].library.len(),
        1,
        "nothing drawn from his own"
    );
    let seen = e.state.legal_view(SeatId(1));
    let shown: Vec<u16> = seen.seats[0]
        .hand
        .to_cards()
        .into_iter()
        .filter(|&c| c != zvtes::state::HIDDEN)
        .collect();
    assert_eq!(shown, vec![find_library("Computer Hacking")], "face up");
    // The bleed resolves; the next minion phase, A plays the prey's card.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let i = e.state.seats[0]
        .hand
        .copies()
        .iter()
        .position(|&c| c == CopyId(600))
        .expect("still in hand") as u8;
    pick(&mut e, play(i, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(
        e.state.seats[1].ash.copies().contains(&CopyId(600)),
        "its owner's ash heap"
    );
    assert!(!e.state.seats[0].ash.copies().contains(&CopyId(600)));
}

/// [LSJ 20100304] "If Agaitas is burned while cards owned by your prey are
/// still face-up in front of you, you can still play and discard them as
/// normal, but you cannot draw in your prey's library until you control
/// Agaitas anew": Agaitas burned after the draw, the face-up card stays
/// in hand, face up; the next play is replaced from the own library.
#[test]
fn agaitas_burned_leaves_the_face_up_cards_but_no_more_prey_draws() {
    let (mut e, ag, a) = agaitas_game("Computer Hacking");
    e.state.seats[1]
        .library
        .put(find_library("Dodge"), CopyId(601));
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ChooseSeat(SeatId(1)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.minions[ag.index()].in_play = false;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let i = e.state.seats[0]
        .hand
        .copies()
        .iter()
        .position(|&c| c == CopyId(601))
        .expect("still in hand") as u8;
    assert!(e.state.seats[0].face_up.0.contains(&CopyId(601)));
    let card = e.state.seats[0].hand.to_cards()[i as usize];
    assert_eq!(card, find_library("Dodge"));
    let k = e.state.seats[0]
        .hand
        .to_cards()
        .iter()
        .position(|&c| c == find_library("Computer Hacking"));
    assert!(k.is_none());
    hand(&mut e, 0, &["Computer Hacking"]);
    e.state.seats[0].hand.take(0);
    let at = e.state.seats[0].hand.len() as u8 - 1;
    pick(&mut e, play(at, 0, a));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::ChooseSeat(SeatId(1)),
    );
}

/// [LSJ 20011202] "Additional draws are not subject to his ability (eg.
/// Infernal Pursuit, Aura Reading, Learjet)": Learjet's bearer plays
/// Computer Hacking; the replacement from the prey, the additional card
/// from the own library.
#[test]
fn agaitas_extra_draws_come_from_his_own_library() {
    let (mut e, _, a) = agaitas_game("Dodge");
    put(&mut e, 0, "Learjet", Some(a));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::ExtraDrawOnReplace,
        source: CardId(e.state.cards.len() as u32 - 1),
    });
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ChooseSeat(SeatId(1)));
    let req = expect(&mut e, SeatId(0), WindowLabel::Draw);
    assert_eq!(req.options, vec![Choice::Pass, Choice::DrawLibrary]);
    e.choose(index_of(&req, Choice::DrawLibrary));
    let copies = e.state.seats[0].hand.copies().to_vec();
    assert!(copies.contains(&CopyId(600)), "the prey's card");
    assert!(
        copies.contains(&CopyId(500)),
        "the extra from his own library"
    );
}

/// [LSJ 20020718] "His ability only applies when the card is played. Any
/// delay in the replacement prevents it (eg. The Meddling of Semsith,
/// Bum's Rush)": Bum's Rush, replaced after its action, from the own
/// library only; under The Meddling of Semsith, a card played in the
/// minion phase alike.
#[test]
fn agaitas_gets_no_prey_draw_for_a_delayed_replacement() {
    let (mut e, _, a) = agaitas_game("Dodge");
    let x = MinionId(2);
    e.state.seats[0].hand.take(0);
    hand(&mut e, 0, &["Bum's Rush"]);
    let at = e.state.seats[0].hand.len() as u8 - 1;
    pick(
        &mut e,
        Choice::Play {
            card: at,
            mode: 0,
            actor: Some(a),
            target: Target::Minion(x),
        },
    );
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::ChooseSeat(SeatId(1)),
    );
    assert!(e.state.seats[0].hand.copies().contains(&CopyId(500)));
    assert_eq!(e.state.seats[1].library.len(), 1, "the prey's untouched");
    // The Meddling of Semsith on seat 0.
    let (mut e, _, a) = agaitas_game("Dodge");
    let m = put(&mut e, 1, "Meddling of Semsith, The", None);
    e.state.cards[m.index()].against = zvtes::ids::SeatSet::single(SeatId(0));
    e.state.layers.push(zvtes::cards::Layer {
        scope: zvtes::cards::Scope::InPlay,
        modifier: zvtes::cards::Modifier::NoReplaceInMinionPhaseForChosen,
        source: m,
    });
    pick(&mut e, play(0, 0, a));
    never_offered_until(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        Choice::ChooseSeat(SeatId(1)),
    );
    assert_eq!(e.state.seats[1].library.len(), 1, "the prey's untouched");
}

/// [LSJ 20050913] "A weapon available from his ability can be chosen for a
/// Concealed Weapon": the prey's .44 Magnum drawn face up is offered to
/// Concealed Weapon and equipped, its owner's card in play.
#[test]
fn agaitas_face_up_weapon_serves_concealed_weapon() {
    let (mut e, _, a) = agaitas_game(".44 Magnum");
    let x = MinionId(2);
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ChooseSeat(SeatId(1)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.seats[0].hand.take(0);
    hand(&mut e, 0, &["Concealed Weapon"]);
    let at = e.state.seats[0].hand.len() as u8 - 1;
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(x));
    pick_when(&mut e, play(at, 0, a));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let magnum = find_library(".44 Magnum");
    let c = e
        .state
        .cards
        .iter()
        .find(|c| c.in_play && c.card == magnum)
        .expect("equipped");
    assert_eq!(c.host, Some(a));
    assert_eq!(c.owner, SeatId(1), "its owner's card");
}

/// Heart of Nizchetus: [RTR 19970425] [LSJ 20060921] Agaitas's cards
/// "that go into an ash heap, hand, or library always go into their
/// owner's": a face-up card put on the library by its effect lands in its
/// owner's library.
#[test]
fn agaitas_face_up_card_on_heart_of_nizchetus_goes_to_its_owners_library() {
    let (mut e, _, a) = agaitas_game("Dodge");
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ChooseSeat(SeatId(1)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // The Heart's move, its hand card to the library's bottom.
    let i = e.state.seats[0]
        .hand
        .copies()
        .iter()
        .position(|&c| c == CopyId(600))
        .expect("in hand");
    let (card, copy) = e.state.seats[0].hand.take(i);
    e.state.seats[0].library.put_at(0, card, copy);
    request(&mut e);
    assert!(e.state.seats[1].library.copies().contains(&CopyId(600)));
    assert!(!e.state.seats[0].library.copies().contains(&CopyId(600)));
}

/// Storage Annex: [PIB 20150224] "The card on it goes to the controller's
/// hand, regardless of where the new card comes from (eg. Agaitas
/// face-up cards)": Agaitas's prey card exchanged onto the Annex, the
/// Annex's card to seat 0's hand; the prey's card on it stays the prey's.
#[test]
fn storage_annex_returns_its_card_to_the_controller_for_an_agaitas_card() {
    let (mut e, _, a) = agaitas_game("Dodge");
    pick(&mut e, play(0, 0, a));
    pick(&mut e, Choice::ChooseSeat(SeatId(1)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let annex = put(&mut e, 0, "Storage Annex", None);
    e.state.cards[annex.index()]
        .stored
        .put(find_library("Leverage"), CopyId(700));
    e.state.cards[annex.index()].stored_owner = Some(SeatId(0));
    // Only the prey's Dodge and the Annex's card in hand.
    let keep = e.state.seats[0]
        .hand
        .copies()
        .iter()
        .position(|&c| c == CopyId(600))
        .expect("in hand");
    let (card, copy) = e.state.seats[0].hand.take(keep);
    e.state.seats[0].hand.clear();
    e.state.seats[0].library.clear();
    e.state.seats[0].hand.put(card, copy);
    let req = to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let swap = uses(&req, annex.0);
    pick(&mut e, swap[0]);
    request(&mut e);
    assert!(e.state.seats[0].hand.copies().contains(&CopyId(700)));
    assert_eq!(
        e.state.cards[annex.index()].stored_owner,
        Some(SeatId(1)),
        "the prey's card on it"
    );
}

// ---- Aisha ------------------------------------------------------------------

/// Aisha: "Once each turn, Aisha can burn 1 blood to get +1 intercept for
/// the current action": [ANK 20180719-4] "'Once each turn' means once
/// during each player's turn": used against a hunt, not again that turn.
#[test]
fn aisha_burns_for_intercept_once_each_turn() {
    use zvtes::decision::ActionKind::Hunt;
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain4(0), 4, false);
    let b = e.add_minion(SeatId(0), plain4(1), 4, false);
    let n = e.add_minion(SeatId(1), base("Aisha"), 6, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    pick(&mut e, action(a, Hunt));
    pick_when(&mut e, block(n));
    pick_use(&mut e, n, 0);
    assert_eq!(blood(&e, n), 5);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    e.state.minions[n.index()].locked = false;
    pick(&mut e, action(b, Hunt));
    pick_when(&mut e, block(n));
    let req = request(&mut e);
    assert!(
        !req.options.iter().any(|o| uses_printed(o, n, 0)),
        "this turn"
    );
}

// ---- Al-Ashrad ----------------------------------------------------------------

/// Al-Ashrad: "can burn a location requiring a clan as a Ⓓ action that
/// costs 2 blood. … +1 bleed.": seat 1's Alamut (Banu Haqim) offered, its
/// Information Highway (no clan) not; burned, 2 blood paid. Merged with
/// his advanced card: the Banu Haqim Inner Circle, Camarilla, +2 bleed, the
/// burn still offered.
#[test]
fn al_ashrad_burns_a_location_requiring_a_clan() {
    let mut e = game(2);
    let al = e.add_minion(SeatId(0), base("Al-Ashrad, Amr of Alamut"), 6, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let alamut = put(&mut e, 1, "Alamut", None);
    let highway = put(&mut e, 1, "Information Highway", None);
    let req = request(&mut e);
    let at = |c| printed_use(al, 0, Some(al), Target::Card(c));
    assert!(req.options.contains(&at(alamut)));
    assert!(!req.options.contains(&at(highway)));
    e.choose(index_of(&req, at(alamut)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[alamut.index()].in_play, "burned");
    assert!(e.state.seats[1].ash.contains(&find_library("Alamut")));
    assert_eq!(blood(&e, al), 4);
    // Merged.
    let mut e = game(2);
    let al = e.add_minion(
        SeatId(0),
        card_of("Al-Ashrad, Amr of Alamut", true),
        6,
        false,
    );
    e.state.minions[al.index()].merged = Some(base("Al-Ashrad, Amr of Alamut"));
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let alamut = put(&mut e, 1, "Alamut", None);
    assert_eq!(e.state.title_of(al), Title::InnerCircle("Banu Haqim"));
    assert_eq!(e.state.sect_of(al), Some(Sect::Camarilla));
    assert!(e.state.sorcery_first_round(al));
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&printed_use(al, 0, Some(al), Target::Card(alamut))));
    pick(&mut e, bleed(al));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 3, "1 + 2 bleed");
}

// ---- Ambrogino Giovanni -------------------------------------------------------

/// Ambrogino Giovanni: "1 vote (titled): +1 bleed."; merged with his
/// advanced card ("+1 bleed. [MERGED] +1 stealth"): both cards' bleed, the
/// stealth, the vote, the ash-heap trade still offered.
#[test]
fn ambrogino_giovanni_bleeds_and_merged_bleeds_for_three_with_stealth() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), base("Ambrogino Giovanni"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    assert_eq!(e.state.title_of(m), Title::Votes(1));
    pick(&mut e, bleed(m));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 2);
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), card_of("Ambrogino Giovanni", true), 4, false);
    e.state.minions[m.index()].merged = Some(base("Ambrogino Giovanni"));
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    assert_eq!(e.state.title_of(m), Title::Votes(1));
    assert_eq!(
        e.state
            .printed_mod(m, zvtes::cards::Stat::Stealth, Default::default()),
        1
    );
    pick(&mut e, bleed(m));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10 - 3);
}

/// Personal titles: [LSJ 20010714] "Keeps their title and votes if they
/// changes sects": Ambrogino, Amisa, Augustus and Aziz made Anarch keep
/// their titled votes.
#[test]
fn personal_vote_titles_survive_a_sect_change_round_35() {
    for (name, votes) in [
        ("Ambrogino Giovanni", 1),
        ("Amisa", 2),
        ("Augustus Giovanni", 2),
        ("Aziz, Dammar of Istanbul", 2),
    ] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), base(name), 4, false);
        e.state.minions[m.index()].sect_override = Some(Sect::Anarch);
        assert_eq!(e.state.title_of(m), Title::Votes(votes), "{name}");
        assert_eq!(votes_of(&e, m), votes as i32, "{name}");
    }
}

// ---- Amisa ------------------------------------------------------------------

/// Amisa: "can lock a vampire with capacity 8 or more as a Ⓓ action":
/// seat 1's capacity-8+ vampire offered, its smaller one not; locked.
#[test]
fn amisa_locks_a_vampire_of_capacity_eight_or_more() {
    let mut e = game(2);
    let am = e.add_minion(SeatId(0), base("Amisa"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let big = e.add_minion(SeatId(1), textless(|c| c.capacity >= 8, 0), 4, false);
    let small = e.add_minion(
        SeatId(1),
        textless(|c| c.capacity <= 7 && c.capacity >= 4, 1),
        4,
        false,
    );
    let req = request(&mut e);
    let at = |m| printed_use(am, 0, Some(am), Target::Minion(m));
    assert!(req.options.contains(&at(big)));
    assert!(!req.options.contains(&at(small)));
    e.choose(index_of(&req, at(big)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[big.index()].locked, "locked");
    assert!(e.state.minions[am.index()].locked, "acted");
}

// ---- Andrea Giovanni ----------------------------------------------------------

/// Andrea Giovanni: "+1 stealth during actions requiring Necromancy
/// [nec]": read on an action whose mode requires it, not on a bleed.
#[test]
fn andrea_giovanni_sneaks_on_necromancy_actions() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), base("Andrea Giovanni"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Divine Sign"]);
    pick_when(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(m),
            target: Target::Minion(x),
        },
    );
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 2);
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), base("Andrea Giovanni"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, bleed(m));
    request(&mut e);
    assert_eq!(e.action().expect("action").stealth(&e.state), 0);
}

// ---- Angela Preston ---------------------------------------------------------

/// Angela Preston: "During your influence phase, Angela can lock to unlock
/// another vampire": [LSJ 20011022] in torpor too; once in the phase.
#[test]
fn angela_preston_locks_to_unlock_another_vampire_from_torpor() {
    let mut e = game(2);
    let ang = e.add_minion(SeatId(0), base("Angela Preston"), 2, true);
    let v = e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.minions[v.index()].locked = true;
    e.state.minions[ang.index()].locked = false;
    let req = to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let at = |o: &Choice| {
        uses_printed(o, ang, 0)
            && matches!(o, Choice::UseAbility { target, .. } if *target == Target::Minion(v))
    };
    let i = req
        .options
        .iter()
        .position(at)
        .unwrap_or_else(|| panic!("{:?}", req.options));
    e.choose(i);
    assert!(!e.state.minions[v.index()].locked);
    assert!(e.state.minions[ang.index()].locked);
    assert!(torpid(&e, ang));
}

/// [RTR 20180303] "When she is influenced out, her ability to unlock a
/// vampire can be used during the ongoing influence phase."
#[test]
fn angela_preston_unlocks_the_phase_she_is_influenced_out() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let angela = base("Angela Preston");
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: angela,
            copy: Default::default(),
            counters: CRYPT[angela as usize].capacity,
            banished: None,
        });
    e.state.minions[v.index()].locked = true;
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    e.state.minions[v.index()].locked = true;
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let ang = in_play_of(&e, angela).expect("influenced out");
    let req = request(&mut e);
    assert!(
        req.options.iter().any(|o| uses_printed(o, ang, 0)
            && matches!(o, Choice::UseAbility { target, .. } if *target == Target::Minion(v))),
        "{:?}",
        req.options
    );
}

/// Angela Preston: "She must burn 1 blood to attempt to block an older
/// vampire": blocking a younger one costs nothing, an older one 1 blood.
#[test]
fn angela_preston_pays_to_block_an_older_vampire() {
    for older in [false, true] {
        let mut e = game(2);
        let cap = |c: &zvtes::crypt::CryptCard| {
            if older {
                c.capacity >= 7
            } else {
                c.capacity <= 4
            }
        };
        let x = e.add_minion(SeatId(0), textless(cap, 0), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let ang = e.add_minion(SeatId(1), base("Angela Preston"), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        pick(&mut e, action(x, ActionKind::Hunt));
        pick_when(&mut e, block(ang));
        request(&mut e);
        assert_eq!(blood(&e, ang), if older { 3 } else { 4 }, "older {older}");
    }
}

// ---- Anisa Marianna Lopez ---------------------------------------------------

/// Anisa Marianna Lopez: "can burn a vampire with a capacity less than 4
/// controlled by your predator or prey as a Ⓓ action that costs 1 pool":
/// four seats; the prey's capacity-3 vampire offered, its capacity-4 one
/// not, nor the cross-table seat's; burned, 1 pool paid.
#[test]
fn anisa_burns_a_small_vampire_of_her_predator_or_prey() {
    let mut e = game(4);
    let an = e.add_minion(SeatId(0), base("Anisa Marianna Lopez"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let small = |n| textless(|c| c.capacity <= 3, n);
    let prey_small = e.add_minion(SeatId(1), small(0), 2, false);
    let prey_big = e.add_minion(SeatId(1), textless(|c| c.capacity == 4, 0), 2, false);
    let across = e.add_minion(SeatId(2), small(1), 2, false);
    e.add_minion(SeatId(3), plain4(1), 2, false);
    let req = request(&mut e);
    let at = |m| printed_use(an, 0, Some(an), Target::Minion(m));
    assert!(req.options.contains(&at(prey_small)));
    assert!(!req.options.contains(&at(prey_big)));
    assert!(!req.options.contains(&at(across)));
    e.choose(index_of(&req, at(prey_small)));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.minions[prey_small.index()].in_play, "burned");
    assert_eq!(e.state.seats[0].pool, 9);
}

// ---- Ankla Hotep ----------------------------------------------------------------

/// Ankla Hotep: "Rescuing Ankla from torpor costs 2 fewer blood. He gets 1
/// optional press each combat.": rescued for nothing, no blood moving.
#[test]
fn ankla_hotep_is_rescued_for_free() {
    let mut e = game(2);
    let ankla = e.add_minion(SeatId(0), base("Ankla Hotep"), 1, true);
    let r = e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    pick(&mut e, rescue(r, ankla));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[ankla.index()].ready(), "rescued");
    assert_eq!(blood(&e, r), 4);
    assert_eq!(blood(&e, ankla), 1);
    assert_eq!(e.state.combat_presses_of(ankla, x), 1);
}

/// Mitchell, The Headhunter: "Rescuing Mitchell from torpor costs +1
/// blood" — the rescue's cost as announced (rules-qa 48.8): 3 blood, not
/// offered with 2 between the rescuer and him.
#[test]
fn mitchell_is_not_offered_a_rescue_his_blood_cannot_pay() {
    for have in [2, 3] {
        let mut e = game(2);
        let mitchell = e.add_minion(SeatId(0), base("Mitchell, The Headhunter"), 0, true);
        let r = e.add_minion(SeatId(0), plain4(0), have, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        let req = request(&mut e);
        assert_eq!(
            req.options.contains(&rescue(r, mitchell)),
            have == 3,
            "{have} blood"
        );
    }
}

// ---- Augustus Giovanni ----------------------------------------------------------

/// Augustus Giovanni: "If a vampire you control has been burned since your
/// last turn, Augustus can move that vampire from your ash heap to the
/// ready region (with no blood) as an action that costs 1 pool": seat 0's
/// vampire burned in seat 1's turn returned ready with no blood, 1 pool
/// paid; one burned before seat 0's last turn not offered.
#[test]
fn augustus_giovanni_returns_a_vampire_burned_since_last_turn() {
    let mut e = game(2);
    let au = e.add_minion(SeatId(0), base("Augustus Giovanni"), 4, false);
    let old = e.add_minion(SeatId(0), plain4(0), 4, false);
    let v = e.add_minion(SeatId(0), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    e.state.seats[0].this_turn = 4;
    let gone = |e: &mut Engine, m: MinionId| {
        let v = &mut e.state.minions[m.index()];
        v.in_play = false;
        let (card, copy) = (v.crypt_card().expect("a vampire"), *v.copy);
        let turn = e.state.turn;
        e.state.seats[0].burn_vampire(card, copy, Some((turn, SeatId(0))));
    };
    gone(&mut e, old);
    let req = request(&mut e);
    assert!(
        req.options.iter().any(|o| uses_printed(o, au, 0)),
        "this turn's burn"
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let req = request(&mut e);
    assert!(
        !req.options.iter().any(|o| uses_printed(o, au, 0)),
        "burned before the last turn"
    );
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    gone(&mut e, v);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    pick_use(&mut e, au, 0);
    pick(&mut e, Choice::AshVampire(plain4(1)));
    let back = loop {
        let req = request(&mut e);
        if let Some(m) = in_play_of(&e, plain4(1)) {
            break m;
        }
        e.choose(fallback(&req));
    };
    assert!(e.state.minions[back.index()].ready());
    assert_eq!(blood(&e, back), 0);
    assert_eq!(e.state.seats[0].pool, 9);
    assert!(!e.state.seats[0].ash_crypt.contains(&plain4(1)));
}

/// Augustus Giovanni: "If a vampire you control has been burned": the
/// controller at the burn [ANK 20210813]: seat 0's own vampire burned
/// while seat 1 controlled it (in seat 0's ash heap) is not offered; one
/// seat 0 controlled is.
#[test]
fn augustus_giovanni_skips_a_vampire_burned_under_another() {
    for stolen in [false, true] {
        let mut e = game(2);
        let au = e.add_minion(SeatId(0), base("Augustus Giovanni"), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(2), 4, false);
        let v = if stolen {
            let v = e.add_minion(SeatId(1), plain4(1), 4, false);
            e.state.minions[v.index()].origin = Some(SeatId(0));
            v
        } else {
            e.add_minion(SeatId(0), plain4(1), 4, false)
        };
        e.state.seats[0].this_turn = 4;
        zvtes::copies::number_unnumbered(&mut e);
        e.burn(v);
        assert!(e.state.seats[0].ash_crypt.contains(&plain4(1)));
        let req = request(&mut e);
        assert_eq!(
            req.options.iter().any(|o| uses_printed(o, au, 0)),
            !stolen,
            "{stolen}"
        );
    }
}

// ---- Aziz, Dammar of Istanbul ---------------------------------------------------

/// Aziz: "Put 1 status counter on Aziz after a vampire opposing him is
/// burned in combat. He gets +1 vote for each status counter on him. …
/// +1 strength.": his opponent of 1 blood takes 3 aggravated damage and
/// burns: 1 counter, 3 votes; an opponent sent to torpor: none.
#[test]
fn aziz_gets_a_status_counter_for_a_vampire_burned_in_combat() {
    for burn in [false, true] {
        let mut e = game(2);
        let aziz = e.add_minion(SeatId(0), base("Aziz, Dammar of Istanbul"), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let x = e.add_minion(SeatId(1), plain4(1), 0, false);
        e.add_minion(SeatId(1), plain4(2), 4, false);
        if burn {
            e.state.minions[aziz.index()].timed.push((
                zvtes::cards::Lasting::AggravatedHands,
                zvtes::state::Expiry::EndOfTurn,
                SeatId(0),
            ));
        }
        pick(&mut e, bleed(aziz));
        pick_when(&mut e, block(x));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(!e.state.minions[x.index()].in_play, burn);
        let n = e.state.minions[aziz.index()].counter(MinionCounter::Status);
        assert_eq!(n, burn as u8, "burn {burn}");
        assert_eq!(votes_of(&e, aziz), 2 + burn as i32);
    }
}

// ---- Bajazet al-Nasir ---------------------------------------------------------

/// Bajazet al-Nasir: "Once each combat, Bajazet can burn 1 blood to make the
/// damage from his hand strikes aggravated for the current round": [RTR
/// 19960112] "Can be played after both strikes have been chosen": used
/// before resolution, his hand strike's 1 damage aggravated; once.
#[test]
fn bajazet_aggravates_his_hand_strike_once_each_combat() {
    let mut e = game(2);
    let b = e.add_minion(SeatId(0), base("Bajazet al-Nasir"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    pick(&mut e, bleed(b));
    pick_when(&mut e, block(x));
    loop {
        let req = request(&mut e);
        if req.window == WindowLabel::CombatBeforeResolution
            && req.options.iter().any(|o| uses_printed(o, b, 0))
        {
            let i = req
                .options
                .iter()
                .position(|o| uses_printed(o, b, 0))
                .expect("offered");
            e.choose(i);
            break;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(blood(&e, b), 3);
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| uses_printed(o, b, 0)), "once");
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 4, "aggravated: no blood mends it");
    assert!(torpid(&e, x));
}

// ---- Text-only vampires -------------------------------------------------------

/// Abd al-Rashid, Anwar and Badr al-Budur ("Independent: Blood cursed."):
/// no definition, the trait generated.
#[test]
fn the_round_35_blood_cursed_banu_haqim_carry_their_trait() {
    for name in ["Abd al-Rashid", "Anwar", "Badr al-Budur"] {
        let card = base(name);
        assert!(zvtes::cards::vampire(card).is_none(), "{name}");
        assert!(CRYPT[card as usize].blood_cursed, "{name}");
        assert_eq!(CRYPT[card as usize].sect, Some(Sect::Independent));
    }
}

// ---- High Priest Angra Mainyu -----------------------------------------------

/// Angra: "While Angra is ready, your prey gets -1 hand size": three seats,
/// the prey's hand size 6, the predator's 7; Angra in torpor, 7.
#[test]
fn angra_mainyu_shrinks_his_prey_hand_while_ready() {
    for torpor in [false, true] {
        let mut e = game(3);
        e.add_minion(SeatId(0), base("High Priest Angra Mainyu"), 4, torpor);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        assert_eq!(e.state.hand_size(SeatId(1)), if torpor { 7 } else { 6 });
        assert_eq!(e.state.hand_size(SeatId(2)), 7);
        assert_eq!(e.state.hand_size(SeatId(0)), 7);
    }
}

/// Enticement: [LSJ 20100527] "If you use High Priest Angra Mainyu to get
/// +1 stealth, and the action is unblocked, your prey burns no pool, but
/// you still cannot bleed": Angra burns the Edge for the stealth seat 1's
/// intercept needs; the Edge gone, no pool burned, his controller's other
/// vampire not offered a bleed. Not offered on a bleed.
#[test]
fn angra_mainyu_burns_the_edge_on_enticement_and_still_cannot_bleed() {
    let mut e = game(3);
    let a = e.add_minion(SeatId(0), base("High Priest Angra Mainyu"), 4, false);
    let b = e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(2), plain4(2), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["aus"];
    hand(&mut e, 0, &["Enticement", "Eagle's Sight"]);
    hand(&mut e, 1, &["Eagle's Sight"]);
    e.state.seats[0].has_edge = true;
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(a),
            target: Target::Seat(SeatId(1)),
        },
    );
    pick_when(&mut e, block(x));
    pick_use(&mut e, a, 0);
    assert!(!e.state.seats[0].has_edge, "the Edge burned");
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 10, "no pool burned");
    assert!(!req.options.contains(&bleed(b)));
    // A bleed: never offered.
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), base("High Priest Angra Mainyu"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.minions[x.index()].gained_levels = vec!["aus"];
    e.state.seats[0].has_edge = true;
    pick(&mut e, bleed(a));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_printed(o, a, 0)
    });
}

// ---- Howler ------------------------------------------------------------------

/// Howler: "She cannot use weapons. +1 strength." [LSJ 20100321] "She can
/// equip weapons, but cannot use them": her .44 Magnum's strike never
/// offered, her hand strike for 2 lands.
#[test]
fn howler_equips_a_weapon_but_never_strikes_with_it() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), base("Howler"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(h));
    pick(&mut e, bleed(h));
    pick_when(&mut e, block(x));
    never_offered_where(
        &mut e,
        SeatId(1),
        WindowLabel::MinionPhase,
        |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == gun),
    );
    assert_eq!(blood(&e, x), 4 - 2, "her hand strike");
}

/// Howler: [LSJ 20040608] the Chainsaw's press, prevention and action "are
/// using the weapon"; [PIB 20110928] "Howler doesn't get a press from the
/// Chainsaw ... Howler can hunt": bearing Talbot's Chainsaw, her hunt is
/// offered and its action is not; a plain bearer's hunt is barred.
#[test]
fn howler_takes_no_passive_bonus_from_her_weapon() {
    for howler in [true, false] {
        let mut e = game(2);
        let h = if howler {
            e.add_minion(SeatId(0), base("Howler"), 4, false)
        } else {
            e.add_minion(SeatId(0), plain4(3), 4, false)
        };
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        let saw = in_play(&mut e, 0, "Talbot's Chainsaw", Some(h));
        let req = request(&mut e);
        assert_eq!(
            req.options.contains(&action(h, ActionKind::Hunt)),
            howler,
            "{howler}"
        );
        assert_eq!(
            req.options.iter().any(
                |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == saw)
            ),
            !howler,
            "{howler}"
        );
    }
}

/// Count Germaine: "Minions opposing Germaine in combat cannot use
/// weapons", a weapon's passive bonus included (Howler's words, LSJ
/// 20040608; rules-qa 111.44 corrected): his bleed blocked by a bearer of
/// Talbot's Chainsaw, no mandatory press: one round of hand strikes; the
/// same bearer opposing a plain vampire: two.
#[test]
fn count_germaine_bars_his_opponents_weapon_bonuses() {
    for germaine in [true, false] {
        let mut e = game(2);
        let g = if germaine {
            e.add_minion(
                SeatId(0),
                zvtes::state::find_card("Count Germaine"),
                4,
                false,
            )
        } else {
            e.add_minion(SeatId(0), plain4(3), 4, false)
        };
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let b = e.add_minion(SeatId(1), textless(|c| c.capacity >= 7, 0), 6, false);
        e.add_minion(SeatId(1), plain4(2), 4, false);
        in_play(&mut e, 1, "Talbot's Chainsaw", Some(b));
        pick(&mut e, bleed(g));
        pick_when(&mut e, block(b));
        loop {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break;
            }
            let i = req
                .options
                .iter()
                .position(|o| *o == Choice::HandStrike)
                .unwrap_or_else(|| fallback(&req));
            e.choose(i);
        }
        let rounds = if germaine { 1 } else { 2 };
        assert_eq!(blood(&e, g), 4 - rounds, "{germaine}: b's hand strikes");
    }
}

/// Ghoul Retainer: [LSJ 20040607] "A Ghoul Retainer can use a weapon on
/// Howler": the retainer fires her .44 Magnum.
#[test]
fn ghoul_retainer_on_howler_uses_her_weapon() {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), base("Howler"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 7, 0), 6, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    let ghoul = in_play(&mut e, 0, "Ghoul Retainer", Some(h));
    e.state.cards[ghoul.index()].counters = 2;
    let gun = in_play(&mut e, 0, ".44 Magnum", Some(h));
    pick(&mut e, bleed(h));
    pick_when(&mut e, block(x));
    let fire = |o: &Choice| {
        matches!(o, Choice::UseAbility { source: AbilitySource::Card(g), target: Target::Card(t), .. }
            if *g == ghoul && *t == gun)
    };
    for _ in 0..100 {
        let req = request(&mut e);
        if let Some(i) = req.options.iter().position(fire) {
            e.choose(i);
            break;
        }
        e.choose(fallback(&req));
    }
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 6 - 2 - 2, "the Magnum's 2, her hand's 2");
}

// ---- Huitzilopochtli -----------------------------------------------------------

/// Huitzilopochtli: "If Huitzilopochtli is ready during your master phase,
/// you can burn 1 pool to get +1 master phase action. … +1 bleed.": once,
/// 1 pool, a second master played; in torpor not offered.
#[test]
fn huitzilopochtli_buys_one_more_master_phase_action_once() {
    for torpor in [false, true] {
        let mut e = game(2);
        let h = e.add_minion(SeatId(0), base("Huitzilopochtli"), 4, torpor);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        hand(&mut e, 0, &["Information Highway", "Rack, The"]);
        fill_hand(&mut e, 0);
        let req = to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
        let offered = req.options.iter().any(|o| uses_printed(o, h, 0));
        assert_eq!(offered, !torpor);
        if torpor {
            continue;
        }
        e.choose(
            req.options
                .iter()
                .position(|o| uses_printed(o, h, 0))
                .unwrap(),
        );
        assert_eq!(e.state.seats[0].pool, 9);
        pick(&mut e, master_play(0));
        let req = request(&mut e);
        assert!(!req.options.iter().any(|o| uses_printed(o, h, 0)), "once");
        assert!(
            req.options
                .iter()
                .any(|o| matches!(o, Choice::Play { card: 0, .. })),
            "a second master"
        );
    }
}

// ---- Husamettin -------------------------------------------------------------

/// Husamettin bleeds with 2 aggravated hand damage; seat 1's vampire (0
/// blood, a Contract on it if `contract`) blocks and burns.
fn husamettin_burns(contract: bool) -> (Engine, MinionId) {
    let mut e = game(2);
    let h = e.add_minion(SeatId(0), base("Husamettin"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let v = e.add_minion(SeatId(1), plain4(1), 0, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    if contract {
        put(&mut e, 0, "Contract", Some(v));
    }
    e.state.minions[h.index()].timed.push((
        zvtes::cards::Lasting::AggravatedHands,
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    e.state.minions[h.index()].timed.push((
        zvtes::cards::Lasting::Strength(1),
        zvtes::state::Expiry::EndOfTurn,
        SeatId(0),
    ));
    fill_hand(&mut e, 0);
    fill_hand(&mut e, 1);
    e.state.seats[0].library.push(find_library("Potence"));
    e.state.seats[0].library.push(find_library("Quietus"));
    pick(&mut e, bleed(h));
    pick_when(&mut e, block(v));
    (e, h)
}

/// Husamettin: "After Husamettin burns a vampire with a contract on them,
/// you can search your library for a master Discipline card and put it on
/// him": the Potence master put on him, not the Quietus he has superior
/// (LSJ 20040730).
#[test]
fn husamettin_takes_a_master_after_burning_a_contracted_vampire() {
    let (mut e, h) = husamettin_burns(true);
    pick_use(&mut e, h, 0);
    let req = expect(&mut e, SeatId(0), WindowLabel::LibrarySearch);
    let potence = Choice::TakeFrom {
        zone: zvtes::decision::Zone::Library,
        card: find_library("Potence"),
    };
    let quietus = Choice::TakeFrom {
        zone: zvtes::decision::Zone::Library,
        card: find_library("Quietus"),
    };
    assert!(req.options.contains(&potence));
    assert!(!req.options.contains(&quietus), "superior already");
    e.choose(index_of(&req, potence));
    let on = e
        .state
        .cards
        .iter()
        .find(|c| c.in_play && c.card == find_library("Potence"))
        .expect("on him");
    assert_eq!(on.host, Some(h));
    assert!(!on.effect_only);
}

/// No contract on the burned vampire: never offered.
#[test]
fn husamettin_needs_the_contract() {
    let (mut e, h) = husamettin_burns(false);
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_printed(o, h, 0) || uses_printed(o, h, 1)
    });
}

// ---- Intisar ------------------------------------------------------------------

/// Intisar: "If Intisar is ready and you have the Edge during your unlock
/// phase, you can give another Methuselah 1 pool to lock a minion they
/// control." [LSJ 20010716] one pool, one minion: seat 1's vampire locked,
/// 1 pool to seat 1, the Edge kept; her own minion no target.
#[test]
fn intisar_pays_a_rival_to_lock_one_of_their_minions() {
    let mut e = game(2);
    let i = e.add_minion(SeatId(0), base("Intisar"), 4, false);
    let own = e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.seats[0].has_edge = true;
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let at = |m| {
        req.options.iter().copied().find(|o| {
            uses_printed(o, i, 0)
                && matches!(o, Choice::UseAbility { target: Target::Minion(t), .. } if *t == m)
        })
    };
    assert!(at(own).is_none());
    let lock = at(x).expect("offered");
    e.choose(index_of(&req, lock));
    assert!(e.state.minions[x.index()].locked);
    assert_eq!((e.state.seats[0].pool, e.state.seats[1].pool), (9, 11));
    assert!(e.state.seats[0].has_edge);
}

// ---- Ivan Krenyenko -------------------------------------------------------------

/// Ivan Krenyenko: "If Ivan is ready during your unlock phase, you can
/// search your library for a non-location equipment card, reveal it and
/// move it to your hand. +1 strength.": the Kevlar Vest found, not the
/// location; merged, both cards' text (+2 strength, +1 bleed, the search).
#[test]
fn ivan_krenyenko_searches_an_equipment_and_merges() {
    let mut e = game(2);
    let ivan = e.add_minion(SeatId(0), base("Ivan Krenyenko"), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(find_library("Kevlar Vest"));
    e.state.seats[0]
        .library
        .push(find_library("Information Highway"));
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let i = req
        .options
        .iter()
        .position(|o| uses_printed(o, ivan, 0))
        .expect("the search");
    e.choose(i);
    let req = request(&mut e);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::FindCard(c) if *c == find_library("Information Highway"))));
    pick(&mut e, Choice::FindCard(find_library("Kevlar Vest")));
    assert!(e.state.seats[0].hand.contains(&find_library("Kevlar Vest")));
    use zvtes::cards::Stat;
    assert_eq!(
        e.state
            .printed_mod(ivan, Stat::Strength, Default::default()),
        1
    );
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), card_of("Ivan Krenyenko", true), 4, false);
    e.state.minions[m.index()].merged = Some(base("Ivan Krenyenko"));
    assert_eq!(
        e.state.printed_mod(m, Stat::Strength, Default::default()),
        2
    );
    assert_eq!(e.state.printed_mod(m, Stat::Bleed, Default::default()), 1);
    let def = zvtes::cards::vampire_of(&e.state.minions[m.index()]).expect("merged");
    assert_eq!(def.abilities.len(), 1, "the base card's search");
}

// ---- Jack Dawson ------------------------------------------------------------------

/// Jack Dawson: "When you move Jack from your uncontrolled region to your
/// ready region, you may equip him with a weapon card from your hand (pay
/// the cost to equip as normal)." Influenced out: the .44 Magnum offered,
/// not Kali's Fang (Banu Haqim; LSJ 20011214 "Equip"); equipped, 2 pool
/// paid, the card in play on him, replaced.
#[test]
fn jack_dawson_equips_a_weapon_as_he_is_influenced_out() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let jack = base("Jack Dawson");
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: jack,
            copy: Default::default(),
            counters: CRYPT[jack as usize].capacity,
            banished: None,
        });
    hand(&mut e, 0, &[".44 Magnum", "Kali's Fang"]);
    fill_hand(&mut e, 0);
    e.state.seats[0].library.push(find_library("Leverage"));
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let req = expect(&mut e, SeatId(0), WindowLabel::EntryEffect);
    let magnum = Choice::EmployFromHand {
        card: find_library(".44 Magnum"),
        mode: 0,
    };
    assert!(req.options.contains(&magnum));
    assert!(!req.options.iter().any(|o| matches!(o,
        Choice::EmployFromHand { card, .. } if *card == find_library("Kali's Fang"))));
    let pool = e.state.seats[0].pool;
    e.choose(index_of(&req, magnum));
    let j = in_play_of(&e, jack).expect("in play");
    let c = e
        .state
        .cards
        .iter()
        .find(|c| c.in_play && c.card == find_library(".44 Magnum"))
        .expect("equipped");
    assert_eq!(c.host, Some(j));
    assert!(!c.effect_only);
    assert_eq!(e.state.seats[0].pool, pool - 2);
    assert!(!e.state.seats[0].hand.contains(&find_library(".44 Magnum")));
    assert!(
        e.state.seats[0].hand.contains(&find_library("Leverage")),
        "replaced"
    );
}

/// [LSJ 20051223] Merging is no move out of the uncontrolled region: the
/// advanced Jack merged with his base card asks nothing; merged, both
/// cards' text.
#[test]
fn jack_dawson_merge_equips_nothing() {
    let mut e = game(2);
    let adv = e.add_minion(SeatId(0), card_of("Jack Dawson", true), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let jack = base("Jack Dawson");
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: jack,
            copy: Default::default(),
            counters: 0,
            banished: None,
        });
    hand(&mut e, 0, &[".44 Magnum"]);
    fill_hand(&mut e, 0);
    let req = to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    let merge = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::Merge { .. }))
        .unwrap_or_else(|| panic!("a merge: {:?}", req.options));
    e.choose(merge);
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::EntryEffect);
    assert_eq!(e.state.minions[adv.index()].merged, Some(jack));
    let def = zvtes::cards::vampire_of(&e.state.minions[adv.index()]).expect("merged");
    assert_eq!(def.riders.len(), 1);
    assert_eq!(def.abilities.len(), 1);
}

// ---- Jalal Sayad -------------------------------------------------------------------

/// Jalal Sayad: "Once each turn after combat ends, Jalal can burn 1 blood
/// to start a new combat with the opposing minion." [ANK 20180719-4] Akram's
/// test: 1 blood, a new combat; not again that turn.
#[test]
fn jalal_sayad_begins_another_combat_after_his_own() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(0), plain4(1), 4, false);
    let k = e.add_minion(SeatId(1), base("Jalal Sayad"), 6, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(k));
    pick_use(&mut e, k, 0);
    assert_eq!(blood(&e, k), 6 - 1 - 1, "the first combat's 1, his cost 1");
    request(&mut e);
    assert_eq!([blood(&e, a), blood(&e, k)], [4 - 2, 6 - 3]);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_printed(o, k, 0)
    });
}

/// [ANK 20170905] Akram and Jalal "work the same as Psyche!": after a
/// combat no block began (his own Bum's Rush, at resolution), Jalal and
/// Akram are offered the new combat as it would end, and it is fought: 1
/// blood paid, a second round of hand strikes.
#[test]
fn jalal_sayad_begins_another_combat_outside_an_action() {
    for name in ["Jalal Sayad", "Akram"] {
        let mut e = game(2);
        let k = e.add_minion(SeatId(0), base(name), 6, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let t = e.add_minion(SeatId(1), plain4(1), 4, false);
        e.add_minion(SeatId(1), plain4(2), 4, false);
        hand(&mut e, 0, &["Bum's Rush"]);
        pick(
            &mut e,
            Choice::Play {
                card: 0,
                mode: 0,
                actor: Some(k),
                target: Target::Minion(t),
            },
        );
        let req = loop {
            let req = request(&mut e);
            if req.options.iter().any(|o| uses_printed(o, k, 0)) {
                break req;
            }
            assert!(e.action().is_some(), "{name}: never offered");
            e.choose(fallback(&req));
        };
        assert_eq!(req.window, WindowLabel::CombatEndOfRound, "{name}");
        assert!(e.combat().is_some(), "{name}: as the combat would end");
        let i = req
            .options
            .iter()
            .position(|o| uses_printed(o, k, 0))
            .unwrap();
        e.choose(i);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(
            [blood(&e, k), blood(&e, t)],
            [6 - 1 - 1 - 1, 4 - 2],
            "{name}"
        );
    }
}

/// [LSJ 19980109] [RTR 20020501] Torrent ends the first combat; Jalal's new
/// combat: the bleed stays blocked.
#[test]
fn jalal_sayad_loses_the_first_combats_continuation() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), vampire_with("CEL", 0), 6, false);
    e.add_minion(SeatId(0), plain4(1), 4, false);
    let k = e.add_minion(SeatId(1), base("Jalal Sayad"), 6, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 0, &["Torrent"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(k));
    pick_when(&mut e, play(0, 1, a));
    pick_use(&mut e, k, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e)[1], 10, "no continuation: blocked");
}

/// [RTR 20020501] Pocket Out of Time's combat queued: not offered.
#[test]
fn jalal_sayad_needs_no_queued_combat_and_a_ready_opponent() {
    let mut e = game(2);
    let a = e.add_minion(SeatId(0), plain4(0), 6, false);
    e.add_minion(SeatId(0), plain4(1), 4, false);
    let k = e.add_minion(SeatId(1), base("Jalal Sayad"), 6, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    e.state.minions[a.index()].gained_levels = vec!["tem", "tem"];
    hand(&mut e, 0, &["Pocket Out of Time"]);
    pick(&mut e, bleed(a));
    pick_when(&mut e, block(k));
    pick_when(&mut e, play(0, 1, a));
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| uses_printed(o, k, 0)), "queued");
}

// ---- Joaquina Amaya, Jorge De La Muerte, Julian Sanders ---------------------------

/// Joaquina Amaya: "+1 stealth during equip actions": an equip from hand
/// at stealth 2 (the rulebook's +1 and hers); a bleed at 0.
#[test]
fn joaquina_amaya_gets_stealth_on_equip_actions() {
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), base("Joaquina Amaya"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Kevlar Vest"]);
    pick(&mut e, play(0, 0, j));
    request(&mut e);
    assert_eq!(e.action().expect("equip").stealth(&e.state), 2);
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), base("Joaquina Amaya"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, bleed(j));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 0);
}

/// Jorge De La Muerte: "+1 bleed when bleeding a Methuselah who controls a
/// ready Sabbat vampire": 2 against one, 1 when it is in torpor.
#[test]
fn jorge_de_la_muerte_bleeds_more_against_a_sabbat_controller() {
    for torpor in [false, true] {
        let mut e = game(2);
        let j = e.add_minion(SeatId(0), base("Jorge De La Muerte, The Agent"), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let sabbat = textless(|c| c.sect == Some(Sect::Sabbat) && c.capacity >= 4, 0);
        e.add_minion(SeatId(1), sabbat, 4, torpor);
        e.add_minion(
            SeatId(1),
            textless(|c| c.sect != Some(Sect::Sabbat) && c.capacity <= 3, 0),
            2,
            false,
        );
        pick(&mut e, bleed(j));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[1].pool, if torpor { 9 } else { 8 });
    }
}

// ---- Kalinda (G2) --------------------------------------------------------------------

/// Kalinda (G2): "can bleed with +1 bleed as a +1 stealth Ⓓ action that
/// costs 2 blood": seat 1 bled for 2, her 2 blood paid, stealth 1; the G6
/// primogen's Edge unlock is not hers.
#[test]
fn kalinda_g2_bleeds_for_two_as_a_stealthy_action() {
    let g2 = CRYPT
        .iter()
        .position(|c| c.name == "Kalinda" && c.group == 2)
        .expect("Kalinda G2") as u16;
    let g6 = CRYPT
        .iter()
        .position(|c| c.name == "Kalinda" && c.group == 6)
        .expect("Kalinda G6") as u16;
    assert_ne!(
        zvtes::cards::vampire(g2).map(|d| d.group),
        zvtes::cards::vampire(g6).map(|d| d.group)
    );
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), g2, 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick_use(&mut e, k, 0);
    request(&mut e);
    assert_eq!(e.action().expect("the bleed").stealth(&e.state), 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8);
    assert_eq!(blood(&e, k), 2);
}

// ---- Kanimana Belghazi ---------------------------------------------------------------

/// Kanimana Belghazi: "may put a research counter on herself as a +1
/// stealth action that costs 1 blood. While she is ready, your hand size is
/// +X": 1 blood, 1 counter, hand size 8; in torpor 7.
#[test]
fn kanimana_belghazi_studies_for_hand_size() {
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), base("Kanimana Belghazi"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick_use(&mut e, k, 0);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[k.index()].research.0, 1);
    assert_eq!(blood(&e, k), 3);
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
    e.state.minions[k.index()].in_torpor = true;
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
}

// ---- Text and traits ----------------------------------------------------------------

/// Isabel Giovanni ("Independent."), Juanita Santiago ("Sabbat: Sterile.")
/// and Kanya Akhtar ("Independent: Blood cursed."): no definition; Howler
/// sterile, Angra and Huitzilopochtli infernal, Husamettin, Jalal and
/// Kalinda (G2) blood cursed, Kanimana scarce, generated; Julian Sanders
/// presses once each combat.
#[test]
fn the_round_35_vampires_angra_to_kanya_carry_their_printed_traits() {
    for name in ["Isabel Giovanni", "Juanita Santiago", "Kanya Akhtar"] {
        assert!(zvtes::cards::vampire(base(name)).is_none(), "{name}");
    }
    let c = |name: &str| &CRYPT[base(name) as usize];
    assert!(c("Juanita Santiago").sterile && c("Howler").sterile);
    assert!(c("Kanya Akhtar").blood_cursed);
    assert!(c("High Priest Angra Mainyu").infernal && c("Huitzilopochtli").infernal);
    assert!(c("Husamettin").blood_cursed && c("Jalal Sayad").blood_cursed);
    assert!(c("Kanimana Belghazi").scarce);
    let mut e = game(2);
    let j = e.add_minion(SeatId(0), base("Julian Sanders"), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    assert_eq!(e.state.combat_presses_of(j, x), 1);
}

// ---- Synesios ---------------------------------------------------------------------

/// Seat 0 (at its master phase, `seats` seats) plays Information Highway;
/// seat 1's Synesios and a second vampire, Sudden Reversal in seat 1's
/// hand and a master to keep its own master phase open.
fn synesios_table(seats: usize) -> (Engine, MinionId) {
    let mut e = game(seats);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let s = e.add_minion(SeatId(1), base("Synesios"), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    for t in 2..seats {
        e.add_minion(SeatId(t as u8), plain4(t), 4, false);
    }
    hand(&mut e, 0, &["Information Highway"]);
    hand(&mut e, 1, &["Sudden Reversal", "Rack, The"]);
    for t in 0..seats {
        fill_hand(&mut e, t as u8);
    }
    e.at_master_phase();
    pick(&mut e, master_play(0));
    (e, s)
}

/// Synesios: "You can burn 1 pool as you play an out-of-turn master card to
/// get +1 master phase action at the start of your next master phase.":
/// announced with Sudden Reversal, 1 pool burned; [ANK 20191004] "His
/// ability do not allow his owner to play more than one Master: Out of Turn
/// between two master phases" (the out-of-turn master still held); his next
/// master phase: 1 action (1 + 1, less the out-of-turn master's).
#[test]
fn synesios_buys_a_master_phase_action_with_an_out_of_turn_master() {
    for buy in [false, true] {
        let (mut e, s) = synesios_table(2);
        pick(&mut e, master_play(0));
        let announce = Choice::ResolvePrinted { minion: s };
        let req = offering(&mut e, announce);
        assert_eq!(req.seat, SeatId(1));
        e.choose(index_of(&req, if buy { announce } else { Choice::Pass }));
        request(&mut e);
        assert_eq!(e.state.seats[1].pool, if buy { 9 } else { 10 });
        assert_eq!(e.state.seats[1].mpa_next, buy as u8);
        assert!(e.state.seats[1].out_of_turn.is_some(), "one between phases");
        if buy {
            let turn = e.state.turn;
            to_window(&mut e, SeatId(1), WindowLabel::MasterPhase);
            assert_eq!(e.state.turn, turn + 1, "his next master phase");
            assert_eq!(e.state.mpa_left(SeatId(1)), 1);
        }
    }
}

/// An out-of-turn master canceled as played (rules-qa Batch 127): Synesios's
/// pool not burned, no master phase action bought — seat 2's Sudden
/// Reversal on seat 1's.
#[test]
fn synesios_canceled_master_buys_nothing() {
    let (mut e, s) = synesios_table(3);
    hand(&mut e, 2, &["Sudden Reversal"]);
    e.state.seats[2].hand.take(0);
    pick(&mut e, master_play(0));
    pick(&mut e, Choice::ResolvePrinted { minion: s });
    let at = e.state.seats[2].hand.len() as u8 - 1;
    pick_when(&mut e, master_play(at));
    request(&mut e);
    assert_eq!(e.state.seats[1].pool, 10, "nothing paid");
    assert_eq!(e.state.seats[1].mpa_next, 0, "nothing bought");
}

/// Chas Giovanni Tello's discard is no play of an out-of-turn master: [LSJ
/// 20020419] Synesios's controller discarding a master to fail Chas's bleed
/// is never offered Synesios's purchase; nothing paid, nothing bought.
#[test]
fn chas_giovanni_tellos_discard_buys_nothing_with_synesios() {
    let mut e = game(2);
    let c = e.add_minion(SeatId(0), base("Chas Giovanni Tello"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let s = e.add_minion(SeatId(1), base("Synesios"), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 1, &["Blood Doll"]);
    let fail = Choice::UseAbility {
        source: AbilitySource::Vampire(c),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick(&mut e, bleed(c));
    pick_when(&mut e, fail);
    request(&mut e);
    assert!(
        e.state.seats[1].out_of_turn.is_some(),
        "the master discarded"
    );
    for _ in 0..100 {
        let req = request(&mut e);
        assert!(
            !req.options.contains(&Choice::ResolvePrinted { minion: s }),
            "{req:?}"
        );
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(e.state.seats[1].pool, 10, "nothing paid, not bled");
    assert_eq!(e.state.seats[1].mpa_next, 0, "nothing bought");
}

/// Synesios: "cannot use cards requiring Celerity [cel]": [LSJ
/// 20050322-2] "Can use a multi-discipline card with a potential (unused)
/// Celerity [cel] effect": Line Brawl's [cel] mode barred, its [pot] not.
#[test]
fn synesios_shuns_celerity() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), base("Synesios"), 4, false);
    let brawl = find_library("Line Brawl");
    let barred = |m: u8| {
        e.state.printed_mod(
            s,
            zvtes::cards::Stat::CannotPlay,
            zvtes::state::PrintedCtx {
                card: Some((brawl, m)),
                ..Default::default()
            },
        )
    };
    assert_eq!((barred(0), barred(1)), (1, 0));
}

// ---- Tariq, The Silent ------------------------------------------------------------

/// Tariq (base): "He can enter combat with a vampire as a Ⓓ action. Blood
/// hunts cannot be called on him.": the rush offered at seat 1's vampire,
/// not its ally; capacity 3 in play.
#[test]
fn tariq_base_rushes_a_vampire_and_escapes_blood_hunts() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), base("Tariq, The Silent"), 3, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("47th Street Royals"));
    let req = request(&mut e);
    assert!(req
        .options
        .contains(&printed_use(t, 0, Some(t), Target::Minion(x))));
    assert!(!req
        .options
        .contains(&printed_use(t, 0, Some(t), Target::Minion(ally))));
    assert_eq!(e.state.capacity_of(t), Some(3));
    assert_eq!(
        e.state
            .printed_mod(t, zvtes::cards::Stat::NoBloodHunt, Default::default()),
        1
    );
}

/// [ANK 20190522] "His capacity is reduced immediately when he get
/// controlled, before it can be merged"; [ANK 20180523] "[MERGED] When he
/// merges (in whatever way), he becomes independent and any Anarch token is
/// burned": the base Tariq influenced out with 7 is drained to 3; merged
/// (Gift of Experience) while Anarch: capacity 7, blood 3, Independent.
#[test]
fn tariq_base_drains_as_controlled_and_merges_at_seven() {
    let mut e = game(2);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let tariq = base("Tariq, The Silent");
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card: tariq,
            copy: Default::default(),
            counters: 7,
            banished: None,
        });
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let t = in_play_of(&e, tariq).expect("in play");
    request(&mut e);
    assert_eq!(blood(&e, t), 3, "drained as controlled");
    e.state.minions[t.index()].sect_override = Some(Sect::Anarch);
    e.state.seats[0]
        .crypt
        .put(card_of("Tariq, The Silent", true), CopyId::NONE);
    hand(&mut e, 0, &["Gift of Experience"]);
    let card = e.state.seats[0].hand.len() as u8 - 1;
    pick_when(
        &mut e,
        Choice::Play {
            card,
            mode: 0,
            actor: None,
            target: Target::Minion(t),
        },
    );
    request(&mut e);
    let m = &e.state.minions[t.index()];
    assert!(m.merged.is_some(), "merged");
    assert_eq!(e.state.capacity_of(t), Some(7));
    assert_eq!(blood(&e, t), 3);
    assert_eq!(e.state.sect_of(t), Some(Sect::Independent));
}

// ---- Tegyrius, Vizier (G2) --------------------------------------------------------

/// Seat 0's Tegyrius (G2) and a Banu Haqim of seat 1, Independent.
fn tegyrius_table() -> (Engine, MinionId, MinionId) {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), base_g("Tegyrius, Vizier", 2), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let bh = e.add_minion(
        SeatId(1),
        textless(
            |c| c.clan == "Banu Haqim" && c.sect == Some(Sect::Independent) && c.capacity >= 4,
            0,
        ),
        4,
        false,
    );
    (e, t, bh)
}

/// Tegyrius (G2): "If Tegyrius is ready, any Banu Haqim can put 1
/// allegiance counter on themselves as a +1 stealth action. Any Banu Haqim
/// with any allegiance counters is considered Camarilla." Seat 1's
/// Independent Banu Haqim takes it: 1 counter, Camarilla; [LSJ 20080604]
/// the action requires the clan: a Brujah not offered.
#[test]
fn tegyrius_g2_lets_a_banu_haqim_swear_allegiance() {
    let (mut e, t, bh) = tegyrius_table();
    let brujah = e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Brujah" && c.capacity >= 4, 0),
        4,
        false,
    );
    let req = to_window(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let sworn = printed_use(t, 0, Some(bh), Target::None);
    assert!(req.options.contains(&sworn), "{:?}", req.options);
    assert!(!req
        .options
        .contains(&printed_use(t, 0, Some(brujah), Target::None)));
    e.choose(index_of(&req, sworn));
    request(&mut e);
    assert_eq!(e.action().expect("the action").stealth(&e.state), 1);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let n = e.state.minions[bh.index()].counter(MinionCounter::Allegiance);
    assert_eq!(n, 1);
    assert_eq!(e.state.sect_of(bh), Some(Sect::Camarilla));
}

/// [LSJ 20010616] [LSJ 20030522-1] "Allegiance counters continue to
/// function even if Tegyrius leaves play"; [ANK 20180626-2] "an Assamite
/// with it stays Camarilla even if he changes his underlying sect": the
/// counter on, Tegyrius gone, the vampire made Anarch: still Camarilla; a
/// clan changed away from Banu Haqim, not.
#[test]
fn allegiance_counters_outlive_tegyrius_and_a_sect_change() {
    let (mut e, t, bh) = tegyrius_table();
    e.state.add_minion_counter(bh, MinionCounter::Allegiance);
    e.state.minions[t.index()].in_play = false;
    assert_eq!(e.state.sect_of(bh), Some(Sect::Camarilla));
    e.state.minions[bh.index()].sect_override = Some(Sect::Anarch);
    assert_eq!(e.state.sect_of(bh), Some(Sect::Camarilla));
    e.state.minions[bh.index()].clan_change = Some("Brujah");
    assert_eq!(e.state.sect_of(bh), Some(Sect::Anarch));
}

/// Tegyrius's groups: G6 keeps Victoria Ash's votes, G2 his allegiance;
/// merged G2, the grant beside the advanced card's polling votes.
#[test]
fn tegyrius_g2_binds_his_own_text() {
    let g2 = zvtes::cards::vampire(base_g("Tegyrius, Vizier", 2)).expect("G2");
    let g6 = zvtes::cards::vampire(base_g("Tegyrius, Vizier", 6)).expect("G6");
    assert_eq!(g2.abilities.len(), 1);
    assert!(g2
        .printed
        .iter()
        .all(|p| p.stat != zvtes::cards::Stat::Votes));
    assert!(g6
        .printed
        .iter()
        .any(|p| p.stat == zvtes::cards::Stat::Votes));
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), card_of("Tegyrius, Vizier", true), 4, false);
    e.state.minions[m.index()].merged = Some(base_g("Tegyrius, Vizier", 2));
    let d = zvtes::cards::vampire_of(&e.state.minions[m.index()]).expect("merged");
    assert_eq!(d.abilities.len(), 2);
    assert_eq!(
        e.state
            .printed_mod(m, zvtes::cards::Stat::Strength, Default::default()),
        1
    );
}

/// The base crypt card `name` of group `g`.
fn base_g(name: &str, g: u8) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name && !c.adv && c.group == g)
        .unwrap_or_else(|| panic!("no {name} G{g}")) as u16
}

// ---- Theron -------------------------------------------------------------------------

/// Theron: "can put 1 derangement counter on a vampire as a Ⓓ action. If
/// the number of derangement counters equals or exceeds the vampire's
/// capacity, that vampire is considered to be Malkavian antitribu." Seat
/// 1's vampire with capacity−1 counters gets one more: Malkavian antitribu;
/// [LSJ 20060209] a capacity raised (a Potence master on it): its own clan
/// again; Theron gone, the counters still at work.
#[test]
fn theron_deranges_a_vampire_at_its_capacity() {
    let mut e = game(2);
    let th = e.add_minion(SeatId(0), base("Theron"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let v = e.add_minion(SeatId(1), plain4(1), 4, false);
    let cap = e.state.capacity_of(v).expect("a vampire");
    let clan = e.state.minions[v.index()].clan();
    for _ in 1..cap {
        e.state.add_minion_counter(v, MinionCounter::Derangement);
    }
    assert_eq!(e.state.minions[v.index()].clan(), clan, "one short");
    let req = request(&mut e);
    let at = printed_use(th, 0, Some(th), Target::Minion(v));
    e.choose(index_of(&req, at));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[v.index()].clan(),
        Some("Malkavian antitribu")
    );
    e.state.minions[th.index()].in_play = false;
    in_play(&mut e, 1, "Potence", Some(v));
    request(&mut e);
    assert_eq!(e.state.capacity_of(v), Some(cap + 1));
    assert_eq!(e.state.minions[v.index()].clan(), clan, "capacity raised");
}

/// A card's clan over Theron's counters (rules-qa Batch 127): Clan
/// Impersonation's clan governs a vampire with as many counters as its
/// capacity.
#[test]
fn theron_derangement_yields_to_a_card_clan_override() {
    let mut e = game(2);
    e.add_minion(SeatId(0), base("Theron"), 4, false);
    let v = e.add_minion(SeatId(1), plain4(1), 4, false);
    let cap = e.state.capacity_of(v).expect("a vampire");
    for _ in 0..cap {
        e.state.add_minion_counter(v, MinionCounter::Derangement);
    }
    assert_eq!(
        e.state.minions[v.index()].clan(),
        Some("Malkavian antitribu")
    );
    let card = put(&mut e, 1, "Clan Impersonation", Some(v));
    e.state.minions[v.index()]
        .clan_overrides
        .0
        .push(("Tzimisce", card));
    request(&mut e);
    assert_eq!(e.state.minions[v.index()].clan(), Some("Tzimisce"));
}

// ---- Thetmes, Thomas Steed, Ublo-Satha, Unre -------------------------------------

/// Thetmes: "Damage from Thetmes's hand strikes is aggravated.": his
/// blocker of 4 blood to torpor by his 1 aggravated.
#[test]
fn thetmes_hand_strikes_aggravated() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), base("Thetmes, Caliph of Alamut"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    pick(&mut e, bleed(t));
    pick_when(&mut e, block(x));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(torpid(&e, x));
    assert_eq!(blood(&e, x), 4);
}

/// Thomas Steed: "may search your library for a melee weapon and move it to
/// your hand as a +1 stealth action": the Bang Nakh found, not the gun.
#[test]
fn thomas_steed_fetches_a_melee_weapon() {
    let mut e = game(2);
    let t = e.add_minion(SeatId(0), base("Thomas Steed, The Angry"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    fill_hand(&mut e, 0);
    fill_hand(&mut e, 1);
    e.state.seats[0].library.push(find_library(".44 Magnum"));
    e.state.seats[0]
        .library
        .push(find_library("Bang Nakh — Tiger's Claws"));
    pick_use(&mut e, t, 0);
    let claws = Choice::FindCard(find_library("Bang Nakh — Tiger's Claws"));
    let req = offering(&mut e, claws);
    assert!(!req
        .options
        .contains(&Choice::FindCard(find_library(".44 Magnum"))));
    e.choose(index_of(&req, claws));
    request(&mut e);
    assert!(e.state.seats[0]
        .hand
        .contains(&find_library("Bang Nakh — Tiger's Claws")));
}

/// Ublo-Satha: "may prevent 1 damage each combat. Brujah get +1 bleed when
/// bleeding Ublo-Satha's controller.": seat 1's Brujah bleeds seat 0 for 2,
/// its Ventrue for 1; Ublo blocking prevents the hit.
#[test]
fn ublo_satha_prevents_once_and_feeds_brujah_bleeds() {
    for brujah in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), base("Ublo-Satha"), 4, true);
        e.add_minion(SeatId(0), plain4(0), 4, true);
        let clan = if brujah { "Brujah" } else { "Ventrue" };
        let x = e.add_minion(
            SeatId(1),
            textless(|c| c.clan == clan && c.capacity >= 4, 0),
            4,
            false,
        );
        e.add_minion(SeatId(1), plain4(2), 4, false);
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, bleed(x));
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[0].pool, if brujah { 8 } else { 9 }, "{clan}");
    }
    let mut e = game(2);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let u = e.add_minion(SeatId(0), base("Ublo-Satha"), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, bleed(x));
    pick_when(&mut e, block(u));
    pick_use(&mut e, u, 1);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, u), 4, "the hit prevented");
}

/// Unre: "If a minion opposing Unre in combat is burned, put a trap counter
/// on her. Unre gets +1 intercept for each trap counter she has.": seat 1's
/// ally of 1 life burned by her hand strike: 1 counter, +1 intercept; a
/// vampire sent to torpor, none.
#[test]
fn unre_traps_a_burned_opponent() {
    for ally in [true, false] {
        let mut e = game(2);
        let u = e.add_minion(SeatId(0), base("Unre, Keeper of Golgotha"), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let x = if ally {
            let a = e.add_ally(SeatId(1), find_library("47th Street Royals"));
            e.state.minions[a.index()].blood = 1;
            a
        } else {
            e.add_minion(SeatId(1), plain4(1), 0, false)
        };
        e.add_minion(SeatId(1), plain4(2), 4, false);
        pick(&mut e, bleed(u));
        pick_when(&mut e, block(x));
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[x.index()].in_play, !ally);
        let n = e.state.minions[u.index()].counter(MinionCounter::Trap);
        assert_eq!(n, ally as u8, "ally {ally}");
        assert_eq!(
            e.state
                .printed_mod(u, zvtes::cards::Stat::Intercept, Default::default()),
            ally as i32
        );
    }
}

// ---- Ur-Shulgi, Vaclav, Wolf, Yusuf, Zahir --------------------------------------------

/// Ur-Shulgi: "If Ur-Shulgi is ready during your unlock phase, you can
/// search your library for a card named Tajdid, reveal it, and move it to
/// your hand.": Tajdid to hand.
#[test]
fn ur_shulgi_fetches_tajdid_in_the_unlock_phase() {
    let mut e = game(2);
    let u = e.add_minion(SeatId(0), base("Ur-Shulgi, The Shepherd"), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    fill_hand(&mut e, 0);
    fill_hand(&mut e, 1);
    e.state.seats[0].library.push(find_library("Tajdid"));
    e.state.seats[0].library.push(find_library("Leverage"));
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let i = req
        .options
        .iter()
        .position(|o| uses_printed(o, u, 0))
        .expect("the search");
    e.choose(i);
    pick(&mut e, Choice::FindCard(find_library("Tajdid")));
    assert!(e.state.seats[0].hand.contains(&find_library("Tajdid")));
}

/// Vaclav Petalengro: "+1 intercept against Gangrel": against a Gangrel's
/// action, not a Gangrel antitribu's.
#[test]
fn vaclav_intercepts_gangrel() {
    let mut e = game(2);
    let v = e.add_minion(SeatId(0), base("Vaclav Petalengro"), 4, false);
    let g = e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Gangrel" && c.capacity >= 4, 0),
        4,
        false,
    );
    let ga = e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Gangrel antitribu" && c.capacity >= 4, 0),
        4,
        false,
    );
    let vs = |o: MinionId| {
        e.state.printed_mod(
            v,
            zvtes::cards::Stat::Intercept,
            zvtes::state::PrintedCtx {
                opponent: Some(o),
                ..Default::default()
            },
        )
    };
    assert_eq!((vs(g), vs(ga)), (1, 0));
}

/// Wolf Valentine: "1 optional press per combat, only usable to continue
/// combat, when in combat with a Tremere": offered against a Tremere, not a
/// Ventrue.
#[test]
fn wolf_valentine_presses_to_continue_against_a_tremere() {
    for tremere in [true, false] {
        let mut e = game(2);
        let w = e.add_minion(SeatId(0), base("Wolf Valentine"), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let clan = if tremere { "Tremere" } else { "Ventrue" };
        let x = e.add_minion(
            SeatId(1),
            textless(|c| c.clan == clan && c.capacity >= 4, 0),
            4,
            false,
        );
        e.add_minion(SeatId(1), plain4(2), 4, false);
        pick(&mut e, bleed(w));
        pick_when(&mut e, block(x));
        let mut seen = false;
        for _ in 0..100 {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break;
            }
            seen |= req.options.iter().any(|o| uses_printed(o, w, 0));
            e.choose(fallback(&req));
        }
        assert_eq!(seen, tremere, "{clan}");
    }
}

/// Yusuf: "If Yusuf performs a successful action to equip with an
/// equipment requiring an Banu Haqim, he can unlock during the discard
/// phase.": Kali's Fang equipped by action, the unlock offered in the
/// discard phase; a .44 Magnum, not. [ANK 20220218] the outcome read as
/// Szechenyi Jolán's (`after_resolution`: a fizzled action too).
#[test]
fn yusuf_unlocks_after_equipping_kalis_fang() {
    for (weapon, pool, offered) in [("Kali's Fang", 10, true), (".44 Magnum", 10, false)] {
        let mut e = game(2);
        let y = e.add_minion(SeatId(0), base("Yusuf, Scribe of Alamut"), 4, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        hand(&mut e, 0, &[weapon]);
        fill_hand(&mut e, 0);
        pick(&mut e, play(0, 0, y));
        e.state.seats[0].pool = pool;
        let mut seen = false;
        for _ in 0..100 {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break;
            }
            seen |= req.window == WindowLabel::DiscardPhase
                && req.options.iter().any(|o| uses_printed(o, y, 0));
            e.choose(fallback(&req));
        }
        assert_eq!(seen, offered, "{weapon} at {pool} pool");
    }
}

/// Yusuf: Vast Wealth's "+1 stealth equip action" (TOM 19960130: "an
/// 'equip action'") that finds Kali's Fang in his library is "an action to
/// equip with an equipment requiring an Banu Haqim": his discard-phase
/// unlock offered.
#[test]
fn yusuf_unlocks_after_vast_wealth_finds_kalis_fang() {
    let mut e = game(2);
    let y = e.add_minion(SeatId(0), base("Yusuf, Scribe of Alamut"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let wealth = put(&mut e, 0, "Vast Wealth", Some(y));
    e.state.seats[0].library = vec![find_library("Kali's Fang")].into();
    fill_hand(&mut e, 0);
    let req = request(&mut e);
    let i = req
        .options
        .iter()
        .position(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == wealth))
        .expect("Vast Wealth's equip action");
    e.choose(i);
    let mut seen = false;
    for _ in 0..100 {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        seen |= req.window == WindowLabel::DiscardPhase
            && req.options.iter().any(|o| uses_printed(o, y, 0));
        e.choose(fallback(&req));
    }
    assert!(seen, "the unlock offered");
}

/// Zahir: "can search your library for a contract card, reveal it, and move
/// it to your hand as a +1 stealth action": the Contract found.
#[test]
fn zahir_fetches_a_contract() {
    let mut e = game(2);
    let z = e.add_minion(SeatId(0), base("Zahir, Hand of the Silsila"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    fill_hand(&mut e, 0);
    fill_hand(&mut e, 1);
    e.state.seats[0].library.push(find_library("Contract"));
    e.state.seats[0].library.push(find_library("Leverage"));
    pick_use(&mut e, z, 0);
    let req = offering(&mut e, Choice::FindCard(find_library("Contract")));
    assert!(!req
        .options
        .contains(&Choice::FindCard(find_library("Leverage"))));
    e.choose(index_of(&req, Choice::FindCard(find_library("Contract"))));
    request(&mut e);
    assert!(e.state.seats[0].hand.contains(&find_library("Contract")));
}

/// Tansu Bekir ("Independent: Blood cursed.") and Vittorio Giovanni
/// ("Independent."): no definition, traits generated; Synesios, Thetmes and
/// Ur-Shulgi keep their titled votes through a sect change [LSJ 20010714].
#[test]
fn the_round_35_vampires_synesios_to_zahir_carry_their_printed_traits() {
    for name in ["Tansu Bekir", "Vittorio Giovanni"] {
        assert!(zvtes::cards::vampire(base(name)).is_none(), "{name}");
    }
    assert!(CRYPT[base("Tansu Bekir") as usize].blood_cursed);
    for name in [
        "Synesios",
        "Thetmes, Caliph of Alamut",
        "Ur-Shulgi, The Shepherd",
    ] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), base(name), 4, false);
        e.state.minions[m.index()].sect_override = Some(Sect::Anarch);
        assert_eq!(e.state.title_of(m), Title::Votes(2), "{name}");
        assert_eq!(votes_of(&e, m), 2, "{name}");
    }
}

// ---- Melisande, Miriam Benyona ---------------------------------------------------------

/// Melisande: "gets an optional additional strike (limited) on the first
/// round of combat": offered after round 1's strikes, not in round 2.
#[test]
fn melisande_strikes_again_in_the_first_round_only() {
    let mut e = game(2);
    let m = e.add_minion(SeatId(0), base("Melisande"), 6, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 7, 0), 7, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    pick(&mut e, bleed(m));
    pick_when(&mut e, block(x));
    let mut rounds = Vec::new();
    for _ in 0..200 {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        if let Some(i) = req.options.iter().position(|o| uses_printed(o, m, 0)) {
            rounds.push(e.combat().expect("combat").round);
            e.choose(i);
            continue;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(rounds, vec![1]);
}

/// Miriam Benyona: "Rescuing a vampire from torpor costs Miriam -1 blood.
/// During your unlock phase, you can move 1 blood from her to a ready
/// vampire." [PIB 20110918] [LSJ 20020304-2] "The discount applies to the
/// action, regardless of who pays": her rescue costs 1, paid by the
/// rescued; in the unlock phase, 1 blood to another Methuselah's ready
/// vampire, not to a torpid one.
#[test]
fn miriam_benyona_rescues_for_less_and_feeds_a_ready_vampire() {
    let mut e = game(2);
    let mi = e.add_minion(SeatId(0), base("Miriam Benyona"), 1, false);
    let t = e.add_minion(SeatId(0), plain4(0), 2, true);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, rescue(mi, t));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.minions[t.index()].ready(), "rescued");
    assert_eq!(blood(&e, t) + blood(&e, mi), 1 + 2 - 1, "1 blood paid");
    let mut e = game(2);
    let mi = e.add_minion(SeatId(0), base("Miriam Benyona"), 3, false);
    let ready = e.add_minion(SeatId(1), plain4(1), 2, false);
    let torpid = e.add_minion(SeatId(1), plain4(2), 2, true);
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let at = |m: MinionId| {
        req.options.iter().copied().find(|o| {
            uses_printed(o, mi, 0)
                && matches!(o, Choice::UseAbility { target: Target::Minion(t), .. } if *t == m)
        })
    };
    assert!(at(torpid).is_none());
    let give = at(ready).expect("offered");
    e.choose(index_of(&req, give));
    assert_eq!((blood(&e, mi), blood(&e, ready)), (2, 3));
}

// ---- Nefertiti --------------------------------------------------------------------------

/// Nefertiti: "can put 1 corruption counter on a minion controlled by your
/// prey as a +1 stealth Ⓓ action that costs 1 blood. +1 bleed.": three
/// seats; the prey's minion offered, the predator's not; 1 of seat 0's
/// counters on it, 1 blood paid; she bleeds for 2.
#[test]
fn nefertiti_corrupts_a_minion_of_her_prey_for_a_blood() {
    let mut e = game(3);
    let n = e.add_minion(SeatId(0), base("Nefertiti"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let prey = e.add_minion(SeatId(1), plain4(1), 4, false);
    let pred = e.add_minion(SeatId(2), plain4(2), 4, false);
    let req = request(&mut e);
    let at = |m| printed_use(n, 0, Some(n), Target::Minion(m));
    assert!(req.options.contains(&at(prey)));
    assert!(!req.options.contains(&at(pred)));
    e.choose(index_of(&req, at(prey)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[prey.index()].corruption_of(SeatId(0)), 1);
    assert_eq!(blood(&e, n), 3);
    assert_eq!(
        e.state
            .printed_mod(n, zvtes::cards::Stat::Bleed, Default::default()),
        1
    );
}

/// Merged Nefertiti: her [MERGED] unlock offered once in her minion phase
/// while locked; the base card's action and the search both hers.
#[test]
fn nefertiti_merged_unlocks_in_her_minion_phase_and_searches() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), card_of("Nefertiti", true), 4, false);
    e.state.minions[n.index()].merged = Some(base("Nefertiti"));
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let def = zvtes::cards::vampire_of(&e.state.minions[n.index()]).expect("merged");
    assert_eq!(def.abilities.len(), 3);
    e.state.minions[n.index()].locked = true;
    pick_use(&mut e, n, 2);
    assert!(!e.state.minions[n.index()].locked);
    e.state.minions[n.index()].locked = true;
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| uses_printed(o, n, 2)), "once");
}

// ---- Nehsi, Nettie Hale -----------------------------------------------------------------

/// Nehsi: "2 votes (titled): Nehsi gets 1 optional maneuver each combat."
/// [LSJ 20010714] his votes kept through a sect change; the maneuver once.
#[test]
fn nehsi_maneuvers_once_and_keeps_his_votes() {
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), base("Nehsi"), 4, false);
    e.state.minions[n.index()].sect_override = Some(Sect::Anarch);
    assert_eq!(e.state.title_of(n), Title::Votes(2));
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 7, 0), 7, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    pick(&mut e, bleed(n));
    pick_when(&mut e, block(x));
    let mut used = 0;
    for _ in 0..200 {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        if let Some(i) = req.options.iter().position(|o| uses_printed(o, n, 0)) {
            used += 1;
            e.choose(i);
            continue;
        }
        e.choose(fallback(&req));
    }
    assert_eq!(used, 1);
}

/// Nettie Hale: "+1 stealth when employing retainers and recruiting
/// allies": on a recruit, not on a bleed.
#[test]
fn nettie_hale_sneaks_on_recruits() {
    use zvtes::cards::Requirement as R;
    let def = zvtes::cards::vampire(base("Nettie Hale")).expect("defined");
    assert_eq!(def.printed[0].requires, &[R::IsRecruitOrEmploy]);
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), base("Nettie Hale"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Mylan Horseed"]);
    pick(&mut e, play(0, 0, n));
    request(&mut e);
    let with = e.action().expect("recruit").stealth(&e.state);
    let mut e = game(2);
    let n = e.add_minion(SeatId(0), base("Nettie Hale"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, bleed(n));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 0);
    assert!(with >= 1);
}

// ---- Parmenides -------------------------------------------------------------------------

/// Three seats; seat 0 influences Parmenides out: his own minion and its
/// other; seats 1 and 2 a vampire each.
fn parmenides_table() -> (Engine, u16) {
    let mut e = game(3);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(2), plain4(2), 4, false);
    (e, base("Parmenides"))
}

fn influence_parmenides(e: &mut Engine, card: u16) -> MinionId {
    e.state.seats[0]
        .uncontrolled
        .push(zvtes::state::UncontrolledSlot {
            card,
            copy: Default::default(),
            counters: CRYPT[card as usize].capacity,
            banished: None,
        });
    to_window(e, SeatId(0), WindowLabel::InfluenceTransfers);
    let slot = e.state.seats[0].uncontrolled.len() as u8 - 1;
    pick(e, Choice::InfluenceOut { slot });
    MinionId(e.state.minions.len() as u32 - 1)
}

/// Parmenides: "As Parmenides enters play, your predator takes control of
/// him until your next unlock phase." Seat 2 (seat 0's predator) holds
/// him through seats 1 and 2's turns; [LSJ 20010622] "He comes back as the
/// unlock phase begins and unlocks as normal": locked while held, back
/// and unlocked in seat 0's unlock phase.
#[test]
fn parmenides_enters_under_the_predators_control_and_returns_at_unlock() {
    let (mut e, card) = parmenides_table();
    let p = influence_parmenides(&mut e, card);
    request(&mut e);
    let v = &e.state.minions[p.index()];
    assert_eq!((v.owner, v.temp_from), (SeatId(2), Some(SeatId(0))));
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[p.index()].owner, SeatId(2), "still held");
    e.state.minions[p.index()].locked = true;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[p.index()];
    assert_eq!((v.owner, v.temp_from), (SeatId(0), None));
    assert!(!v.locked, "unlocked as normal");
}

/// Parmenides, two copies influenced in one turn (owner 2026-11-20: LSJ
/// 20030304 #m9 "Reverses the earlier ruling about contesting-shifting",
/// LSJ 20030305 #m19, over [LSJ 20010623-2], the KRCG ruling to correct): "P1 is
/// contested, so Denise's temporary control is broken ... Since both are
/// simultaneously 'incoming' to Abby, she chooses which is burned. She keeps
/// the other face up." The second copy influenced: seat 0 offered exactly
/// its two copies, burns the first; the second stays seat 0's for good,
/// seat 2 holding none.
#[test]
fn a_second_parmenides_breaks_the_hold_and_its_owner_burns_one() {
    let (mut e, card) = parmenides_table();
    for _ in 0..2 {
        e.state.seats[0]
            .uncontrolled
            .push(zvtes::state::UncontrolledSlot {
                card,
                copy: Default::default(),
                counters: CRYPT[card as usize].capacity,
                banished: None,
            });
    }
    to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let first = MinionId(e.state.minions.len() as u32 - 1);
    assert_eq!(
        e.state.minions[first.index()].owner,
        SeatId(2),
        "to the predator"
    );
    pick(&mut e, Choice::InfluenceOut { slot: 0 });
    let second = MinionId(e.state.minions.len() as u32 - 1);
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(0));
    let mut options = req.options.clone();
    options.sort_by_key(|o| format!("{o:?}"));
    assert_eq!(
        options,
        vec![Choice::ChooseMinion(first), Choice::ChooseMinion(second)]
    );
    pick(&mut e, Choice::ChooseMinion(first));
    request(&mut e);
    assert!(!e.state.minions[first.index()].in_play, "burned");
    assert!(e.state.seats[0].ash_crypt.contains(&card));
    let v = &e.state.minions[second.index()];
    assert!(v.in_play);
    assert_eq!((v.owner, v.temp_from), (SeatId(0), None));
    assert_eq!(
        e.state
            .minions_of(SeatId(2))
            .filter(|&m| e.state.minions[m.index()].name() == v.name())
            .count(),
        0
    );
}

/// Augustus Giovanni's "a vampire you control": seat 0's Parmenides, held by
/// seat 2 (its predator) and burned in seat 2's turn, was not seat 0's to
/// control at the burn [ANK 20210813]: at seat 0's next turn Augustus is
/// not offered it; a vampire seat 0 controlled, burned in that same turn,
/// is.
#[test]
fn augustus_giovanni_skips_parmenides_burned_under_the_predator() {
    for held in [true, false] {
        let (mut e, card) = parmenides_table();
        let au = e.add_minion(SeatId(0), base("Augustus Giovanni"), 4, false);
        e.state.seats[0].this_turn = e.state.turn;
        let p = influence_parmenides(&mut e, card);
        let own = e.add_minion(SeatId(0), plain4(3), 4, false);
        zvtes::copies::number_unnumbered(&mut e);
        until(&mut e, SeatId(2), WindowLabel::MinionPhase);
        assert_eq!(e.state.minions[p.index()].owner, SeatId(2));
        e.burn(if held { p } else { own });
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        let req = request(&mut e);
        assert_eq!(
            req.options.iter().any(|o| uses_printed(o, au, 0)),
            !held,
            "{held}"
        );
    }
}

/// [LSJ 20021111] "If the owner is ousted before Parmenides comes back, the
/// predator keeps control of him indefinitely."
#[test]
fn parmenides_stays_with_the_predator_if_his_owner_is_ousted() {
    let (mut e, card) = parmenides_table();
    let p = influence_parmenides(&mut e, card);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    e.state.seats[0].pool = 0;
    e.state.seats[0].ousted = true;
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    let v = &e.state.minions[p.index()];
    assert!(v.in_play);
    assert_eq!(v.owner, SeatId(2));
}

/// A contest breaks temporary control (advanced rules §6.4.1; LSJ
/// 20030305 "P1 is contested, so Denise's temporary control is broken"):
/// seat 1 holds a Parmenides; seat 0's goes to seat 2, is contested, and
/// is seat 0's in the contest, not seat 2's.
#[test]
fn a_contest_breaks_parmenides_temporary_control() {
    let (mut e, card) = parmenides_table();
    let theirs = e.add_minion(SeatId(1), card, 4, false);
    let p = influence_parmenides(&mut e, card);
    request(&mut e);
    let v = &e.state.minions[p.index()];
    assert!(v.contested, "contested");
    assert_eq!((v.owner, v.temp_from), (SeatId(0), None));
    assert!(e.state.minions[theirs.index()].contested);
}

// ---- Pochtli -------------------------------------------------------------------------------

/// Pochtli: "can move up to two library cards from your ash heap to your
/// library as a +1 stealth action (shuffle afterward)." [LSJ 20010627] the
/// cards named as the action is declared: two copies of Leverage named,
/// both in the library on success, the Dodge left in the ash heap.
#[test]
fn pochtli_returns_two_named_ash_cards_to_the_library() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), base("Pochtli"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    fill_hand(&mut e, 0);
    fill_hand(&mut e, 1);
    for c in ["Leverage", "Leverage", "Dodge"] {
        e.state.seats[0].ash.push(find_library(c));
    }
    pick_use(&mut e, p, 0);
    let leverage = Choice::FindCard(find_library("Leverage"));
    let req = request(&mut e);
    assert!(req.options.contains(&Choice::Pass), "up to two");
    e.choose(index_of(&req, leverage));
    let req = request(&mut e);
    assert!(req.options.contains(&leverage), "its second copy");
    e.choose(index_of(&req, leverage));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let lib = e.state.seats[0].library.to_cards();
    assert_eq!(
        lib.iter()
            .filter(|&&c| c == find_library("Leverage"))
            .count(),
        2
    );
    assert_eq!(e.state.seats[0].ash.to_cards(), vec![find_library("Dodge")]);
}

/// Pochtli: not offered with an empty ash heap; naming none, nothing
/// moves.
#[test]
fn pochtli_names_nothing_or_needs_an_ash_heap() {
    let mut e = game(2);
    let p = e.add_minion(SeatId(0), base("Pochtli"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| uses_printed(o, p, 0)));
    e.state.seats[0].ash.push(find_library("Dodge"));
    pick_use(&mut e, p, 0);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].ash.to_cards(), vec![find_library("Dodge")]);
}

// ---- Pugfar ------------------------------------------------------------------------------

/// Pugfar: "can burn one blood to give a Tremere antitribu you control a
/// press." [PIB 20150121] "only … during the press step": his controller's
/// Tremere antitribu in combat gets the press for 1 of his blood; a
/// Ventrue combatant, none.
#[test]
fn pugfar_burns_blood_to_give_a_tremere_antitribu_a_press() {
    for tremere in [true, false] {
        let mut e = game(2);
        let pug = e.add_minion(SeatId(0), base("Pugfar"), 4, false);
        let clan = if tremere {
            "Tremere antitribu"
        } else {
            "Ventrue"
        };
        let a = e.add_minion(
            SeatId(0),
            textless(|c| c.clan == clan && c.capacity >= 4, 0),
            4,
            false,
        );
        let x = e.add_minion(SeatId(1), textless(|c| c.capacity >= 7, 0), 7, false);
        e.add_minion(SeatId(1), plain4(2), 4, false);
        pick(&mut e, bleed(a));
        pick_when(&mut e, block(x));
        let mut windows = Vec::new();
        for _ in 0..200 {
            let req = request(&mut e);
            if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
                break;
            }
            if let Some(i) = req.options.iter().position(|o| uses_printed(o, pug, 1)) {
                windows.push(req.window);
                e.choose(i);
                break;
            }
            e.choose(fallback(&req));
        }
        if tremere {
            assert_eq!(windows, vec![WindowLabel::CombatPress]);
            assert_eq!(blood(&e, pug), 3);
        } else {
            assert!(windows.is_empty(), "{clan}");
        }
    }
}

// ---- Qadir ul-Ghani, Qufur am-Heru --------------------------------------------------------

/// Qadir ul-Ghani: "can change his clan to any other clan as a +1 stealth
/// action that costs 1 blood.": Toreador named as declared (his own Banu
/// Haqim not offered), his clan for good, 1 blood paid.
#[test]
fn qadir_ul_ghani_changes_his_clan_for_a_blood() {
    let mut e = game(2);
    let q = e.add_minion(SeatId(0), base("Qadir ul-Ghani"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick_use(&mut e, q, 0);
    let req = request(&mut e);
    assert!(!req.options.contains(&Choice::ChooseClan("Banu Haqim")));
    e.choose(index_of(&req, Choice::ChooseClan("Toreador")));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[q.index()].clan(), Some("Toreador"));
    assert_eq!(blood(&e, q), 3);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[q.index()].clan(),
        Some("Toreador"),
        "for good"
    );
}

/// Qufur am-Heru: "If Qufur is burned from play, you can move him from your
/// ash heap to your uncontrolled region during your next influence phase."
/// Burned in seat 1's turn: offered in seat 0's next influence phase,
/// moved with no counters; declined, not offered the turn after.
#[test]
fn qufur_returns_from_the_ash_heap_in_the_next_influence_phase() {
    for take in [true, false] {
        let mut e = game(2);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        let q = e.add_minion(SeatId(0), base("Qufur am-Heru"), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        e.state.seats[0].this_turn = 4;
        until(&mut e, SeatId(1), WindowLabel::MinionPhase);
        let card = base("Qufur am-Heru");
        let v = &mut e.state.minions[q.index()];
        v.in_play = false;
        let copy = *v.copy;
        let turn = e.state.turn;
        e.state.seats[0].burn_vampire(card, copy, Some((turn, SeatId(0))));
        let req = to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        let back = Choice::AshVampire(card);
        assert!(req.options.contains(&back));
        if take {
            e.choose(index_of(&req, back));
            assert!(!e.state.seats[0].ash_crypt.contains(&card));
            let slot = e.state.seats[0].uncontrolled.last().expect("moved");
            assert_eq!((slot.card, slot.counters), (card, 0));
        } else {
            e.choose(index_of(&req, Choice::Pass));
            until(&mut e, SeatId(1), WindowLabel::MinionPhase);
            until(&mut e, SeatId(0), WindowLabel::MinionPhase);
            assert!(e.state.ash_returns(SeatId(0)).is_empty(), "lapsed");
            assert!(e.state.seats[0].ash_crypt.contains(&card));
        }
    }
}

/// Qufur am-Heru: burned in seat 0's own minion phase, "your next
/// influence phase" is that same turn's: offered there; declined, not the
/// next turn.
#[test]
fn qufur_burned_before_his_owners_influence_returns_that_turn() {
    let mut e = game(2);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let q = e.add_minion(SeatId(0), base("Qufur am-Heru"), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.seats[0].this_turn = 4;
    zvtes::copies::number_unnumbered(&mut e);
    e.burn(q);
    let card = base("Qufur am-Heru");
    let req = to_window(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
    assert_eq!(e.state.turn, 4, "this turn's influence phase");
    assert!(req.options.contains(&Choice::AshVampire(card)));
    e.choose(index_of(&req, Choice::Pass));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.ash_returns(SeatId(0)).is_empty(), "spent");
}

/// Muse ("-1 intercept"), Natalia ("+1 bleed"), Parnassus and Parmenides
/// ("+1 strength"), Qadir ("+1 bleed"); Nepata, Paul DiCarlo and Obsidian
/// print no more than traits; Obsidian's Flight, Pugfar the Tremere
/// antitribu's slave, Miriam scarce, Nettie sterile, generated.
#[test]
fn the_round_35_vampires_melisande_to_qufur_carry_their_printed_traits() {
    use zvtes::cards::Stat;
    for name in ["Nepata", "Paul DiCarlo, The Alpha", "Obsidian"] {
        assert!(zvtes::cards::vampire(base(name)).is_none(), "{name}");
    }
    let c = |name: &str| &CRYPT[base(name) as usize];
    assert!(c("Obsidian").disciplines.contains(&"FLIGHT"));
    assert_eq!(c("Pugfar").slave, Some("Tremere antitribu"));
    assert!(c("Miriam Benyona").scarce && c("Nettie Hale").sterile);
    assert!(c("Parmenides").blood_cursed && c("Parnassus").blood_cursed);
    let mut e = game(2);
    for (name, stat, n) in [
        ("Muse", Stat::Intercept, -1),
        ("Natalia", Stat::Bleed, 1),
        ("Parnassus", Stat::Strength, 1),
        ("Parmenides", Stat::Strength, 1),
        ("Qadir ul-Ghani", Stat::Bleed, 1),
    ] {
        let m = e.add_minion(SeatId(0), base(name), 2, false);
        assert_eq!(
            e.state.printed_mod(m, stat, Default::default()),
            n,
            "{name}"
        );
    }
}

// ---- Raful al-Zarqa, Reg Driscoll, Regina Giovanni -------------------------------------

/// Raful al-Zarqa: "Once each action, Raful can burn a blood to get +1
/// stealth.": against a blocker, once, 1 blood; the block fails.
#[test]
fn raful_al_zarqa_burns_for_stealth_once_an_action() {
    let mut e = game(2);
    let r = e.add_minion(SeatId(0), base("Raful al-Zarqa"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    pick(&mut e, bleed(r));
    pick_when(&mut e, block(x));
    pick_use(&mut e, r, 0);
    assert_eq!(blood(&e, r), 3);
    never_offered_where(&mut e, SeatId(0), WindowLabel::MinionPhase, |o| {
        uses_printed(o, r, 0)
    });
    assert_eq!(e.state.seats[1].pool, 10 - 1, "the block failed");
}

/// Reg Driscoll: "If Reg performs a successful equip action, you may move
/// the equipment to any ready minion you control." [LSJ 20020211] after he
/// equips: his .44 Magnum moved to his other ready vampire, not to a torpid
/// one; a move, no cost; his Dagger held before the action stays: [LSJ
/// 20050217] [LSJ 20090622] "You may move all or none the equipment he gets
/// to just one minion".
#[test]
fn reg_driscoll_moves_the_equipment_he_equips_to_a_ready_minion() {
    let mut e = game(2);
    let reg = e.add_minion(SeatId(0), base("Reg Driscoll"), 4, false);
    let v = e.add_minion(SeatId(0), plain4(0), 4, false);
    let torpid = e.add_minion(SeatId(0), plain4(1), 2, true);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    let dagger = in_play(&mut e, 0, "Dagger", Some(reg));
    hand(&mut e, 0, &[".44 Magnum"]);
    pick(&mut e, play(0, 0, reg));
    let req = offering(&mut e, printed_use(reg, 0, None, Target::Minion(v)));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    assert!(!req
        .options
        .contains(&printed_use(reg, 0, None, Target::Minion(torpid))));
    let pool = e.state.seats[0].pool;
    e.choose(index_of(&req, printed_use(reg, 0, None, Target::Minion(v))));
    let magnum = e
        .state
        .cards
        .iter()
        .position(|c| c.in_play && c.card == find_library(".44 Magnum"))
        .expect("in play");
    assert_eq!(e.state.cards[magnum].host, Some(v));
    assert_eq!(e.state.cards[dagger.index()].host, Some(reg), "held before");
    assert_eq!(e.state.seats[0].pool, pool, "a move");
}

/// [LSJ 20020211] [LSJ 20060221] "He cannot use his ability to move the
/// Eye of Hazimel, or an equipment that is a location while in play": the
/// Eye equipped, nothing offered.
#[test]
fn reg_driscoll_moves_all_or_none_and_never_the_eye_or_a_location() {
    let mut e = game(2);
    let reg = e.add_minion(SeatId(0), base("Reg Driscoll"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    hand(&mut e, 0, &["Eye of Hazimel"]);
    pick(&mut e, play(0, 0, reg));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_printed(o, reg, 0)
    });
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| c.in_play && c.card == find_library("Eye of Hazimel") && c.host == Some(reg)));
}

/// Regina Giovanni: "Allies and retainers cost Regina 1 fewer blood or
/// pool.": Mylan Horseed (1 pool) recruited for nothing.
#[test]
fn regina_giovanni_recruits_cheaper() {
    let mut e = game(2);
    let r = e.add_minion(
        SeatId(0),
        base("Regina Giovanni, The Right Hand of Augustus"),
        4,
        false,
    );
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 0, &["Mylan Horseed"]);
    pick(&mut e, play(0, 0, r));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 10);
    assert!(e
        .state
        .minions
        .iter()
        .any(|m| m.in_play && m.kind.ally_card() == Some(find_library("Mylan Horseed"))));
}

/// Personal titles [LSJ 20010714]: Regina and Sutekh keep their 2 votes
/// through a sect change.
#[test]
fn regina_and_sutekh_keep_their_votes_through_a_sect_change() {
    for name in [
        "Regina Giovanni, The Right Hand of Augustus",
        "Sutekh, The Dark God",
    ] {
        let mut e = game(2);
        let m = e.add_minion(SeatId(0), base(name), 4, false);
        e.state.minions[m.index()].sect_override = Some(Sect::Anarch);
        assert_eq!(e.state.title_of(m), Title::Votes(2), "{name}");
        assert_eq!(votes_of(&e, m), 2, "{name}");
    }
}

// ---- Samat, Saqqaf, Sargon --------------------------------------------------------------

/// Samat Ramal-Ra: "Blood hunts cannot be called on Samat.": his diablerie
/// of seat 1's torpid vampire calls none (seat 1's Edge there to vote).
#[test]
fn samat_ramal_ra_is_never_blood_hunted() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), base("Samat Ramal-Ra, Archon"), 2, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let t = e.add_minion(SeatId(1), plain4(1), 2, true);
    e.add_minion(SeatId(1), plain4(2), 4, false);
    e.state.seats[1].has_edge = true;
    pick(
        &mut e,
        Choice::TakeAction {
            minion: s,
            action: ActionKind::Diablerie,
            target: Some(t),
        },
    );
    loop {
        let req = request(&mut e);
        assert_ne!(req.window, WindowLabel::BloodHuntPolling);
        if (req.seat, req.window) == (SeatId(1), WindowLabel::MinionPhase) {
            break;
        }
        e.choose(fallback(&req));
    }
    assert!(!e.state.minions[t.index()].in_play, "diablerized");
    assert!(e.state.minions[s.index()].in_play);
}

/// Saqqaf: "Once each turn, you can gain 1 pool after Saqqaf performs a
/// successful action requiring Serpentis [ser].": Enticement [ser], 1 pool
/// after resolution; a bleed, nothing; [ANK 20220218] a fizzled action
/// counting (`Requirement::ActionSuccessful`).
#[test]
fn saqqaf_gains_a_pool_after_a_serpentis_action_once_a_turn() {
    let mut e = game(2);
    let q = e.add_minion(
        SeatId(0),
        base("Saqqaf, Keeper of the Grand Temple of Set"),
        6,
        false,
    );
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.seats[0].has_edge = true;
    hand(&mut e, 0, &["Enticement"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(q),
            target: Target::Seat(SeatId(1)),
        },
    );
    let req = offering(&mut e, printed_use(q, 0, None, Target::None));
    assert_eq!(req.window, WindowLabel::ActionAfterResolution);
    let before = e.state.seats[0].pool;
    e.choose(index_of(&req, printed_use(q, 0, None, Target::None)));
    assert_eq!(e.state.seats[0].pool, before + 1);
    let mut e = game(2);
    let q = e.add_minion(
        SeatId(0),
        base("Saqqaf, Keeper of the Grand Temple of Set"),
        6,
        false,
    );
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, bleed(q));
    never_offered_where(&mut e, SeatId(1), WindowLabel::MinionPhase, |o| {
        uses_printed(o, q, 0)
    });
}

/// Sargon: "After Sargon performs a successful action, you gain the Edge.",
/// the gain optional since 2023-03-06 (Baali and Daughters POD: "Gaining
/// the Edge is now optional."): after his hunt resolves (the prey holding
/// the Edge) the gain is offered to his controller; declined, the prey
/// keeps it; taken, it is his controller's. [ANK 20220218] a fizzled
/// action counts too (`Requirement::ActionSuccessful`).
#[test]
fn sargon_may_gain_the_edge_after_a_successful_action() {
    for take in [false, true] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), base("Sargon"), 2, false);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        e.state.seats[1].has_edge = true;
        pick(&mut e, action(s, ActionKind::Hunt));
        let req = loop {
            let req = request(&mut e);
            if req.options.iter().any(|o| uses_printed(o, s, 0)) {
                break req;
            }
            assert!(e.action().is_some(), "the gain never offered");
            e.choose(fallback(&req));
        };
        assert_eq!(req.window, WindowLabel::ActionAfterResolution);
        assert_eq!(req.seat, SeatId(0));
        assert!(
            e.state.seats[1].has_edge,
            "nothing gained before the choice"
        );
        let i = if take {
            req.options
                .iter()
                .position(|o| uses_printed(o, s, 0))
                .unwrap()
        } else {
            req.options.iter().position(|o| *o == Choice::Pass).unwrap()
        };
        e.choose(i);
        until(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.seats[0].has_edge, take);
        assert_eq!(e.state.seats[1].has_edge, !take);
    }
}

/// [LSJ 20011222] [ANK 20221028-2] "If it is a referendum, he gets the
/// Edge after the referendum is concluded": in the polling, still seat
/// 1's; after it, offered and taken, seat 0's.
#[test]
fn sargon_gains_the_edge_after_the_referendum_concludes() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), base("Sargon"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    e.state.seats[1].has_edge = true;
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, s));
    pick(&mut e, alloc(0, 3));
    let mut polled = false;
    loop {
        let req = request(&mut e);
        if req.window == WindowLabel::ReferendumPolling {
            polled = true;
        }
        if let Some(i) = req.options.iter().position(|o| uses_printed(o, s, 0)) {
            assert!(polled, "offered after the referendum");
            assert!(!e.state.seats[0].has_edge);
            e.choose(i);
            break;
        }
        assert!(
            (req.seat, req.window) != (SeatId(0), WindowLabel::MinionPhase),
            "the gain never offered"
        );
        e.choose(fallback(&req));
    }
    assert!(e.state.seats[0].has_edge);
}

// ---- Sarrasine ------------------------------------------------------------------------------

/// Sarrasine: "If Sarrasine is ready during your master phase, you can burn
/// the Edge to get +1 master phase action.": the Edge burned, 2 actions
/// left, not offered again; without the Edge or in torpor, not offered.
#[test]
fn sarrasine_burns_the_edge_for_a_master_phase_action() {
    for (edge, torpor) in [(true, false), (false, false), (true, true)] {
        let mut e = game(2);
        let s = e.add_minion(SeatId(0), base("Sarrasine"), 4, torpor);
        e.add_minion(SeatId(0), plain4(0), 4, false);
        e.add_minion(SeatId(1), plain4(1), 4, false);
        e.state.seats[0].has_edge = edge;
        hand(&mut e, 0, &["Information Highway"]);
        fill_hand(&mut e, 0);
        let req = to_window(&mut e, SeatId(0), WindowLabel::MasterPhase);
        let i = req.options.iter().position(|o| uses_printed(o, s, 0));
        assert_eq!(i.is_some(), edge && !torpor, "edge {edge} torpor {torpor}");
        if let Some(i) = i {
            e.choose(i);
            assert!(!e.state.seats[0].has_edge);
            assert_eq!(e.state.mpa_left(SeatId(0)), 2);
            let req = request(&mut e);
            assert!(!req.options.iter().any(|o| uses_printed(o, s, 0)), "once");
        }
    }
}

/// Merged Sarrasine: [ANK 20180524] Camarilla; [ANK 20200925] [ANK
/// 20211105] "Putting a corruption counter on a minion does not give you an
/// opportunity to steal it": in his unlock phase 1 of seat 0's counters on
/// the prey's minion, still seat 1's.
#[test]
fn sarrasine_merged_is_camarilla_and_corrupts_a_preys_minion() {
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), card_of("Sarrasine", true), 4, false);
    e.state.minions[s.index()].merged = Some(base("Sarrasine"));
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), textless(|c| c.capacity <= 3, 0), 1, false);
    assert_eq!(e.state.sect_of(s), Some(Sect::Camarilla));
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let at = req
        .options
        .iter()
        .position(|o| {
            uses_printed(o, s, 1)
                && matches!(o, Choice::UseAbility { target: Target::Minion(t), .. } if *t == x)
        })
        .expect("offered");
    e.choose(at);
    request(&mut e);
    assert_eq!(e.state.minions[x.index()].corruption_of(SeatId(0)), 1);
    assert_eq!(e.state.minions[x.index()].owner, SeatId(1), "no steal");
}

// ---- The Siamese, Silvia, Spider-Killer, Stefano, Sutekh --------------------------------------

/// The Siamese ("+1 bleed"): bleeds for 2. Spider-Killer ("+1 stealth"):
/// his bleed at stealth 1. Silvia Giovanni ("1 optional press each combat.
/// +1 strength."): her printed press and strength. Saxum, Master of Slaves
/// ("Camarilla Tremere Slave: Flight"): the slave's fight; Rudolfo, Sarisha
/// and Sir Marriot print no text.
#[test]
fn the_round_35_vampires_raful_to_sutekh_carry_their_printed_traits() {
    use zvtes::cards::Stat;
    for name in ["Rudolfo Giovanni", "Sarisha Veliku", "Sir Marriot D'Urban"] {
        assert!(zvtes::cards::vampire(base(name)).is_none(), "{name}");
    }
    assert_eq!(
        CRYPT[base("Saxum, Master of Slaves") as usize].slave,
        Some("Tremere")
    );
    assert!(CRYPT[base("Siamese, The") as usize].sterile);
    let mut e = game(2);
    let s = e.add_minion(SeatId(0), base("Siamese, The"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, bleed(s));
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, 8);
    let mut e = game(2);
    let k = e.add_minion(SeatId(0), base("Spider-Killer"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let x = e.add_minion(SeatId(1), plain4(1), 4, false);
    pick(&mut e, bleed(k));
    request(&mut e);
    assert_eq!(e.action().expect("bleed").stealth(&e.state), 1);
    let sv = e.add_minion(SeatId(0), base("Silvia Giovanni"), 4, false);
    assert_eq!(e.state.combat_presses_of(sv, x), 1);
    assert_eq!(
        e.state.printed_mod(sv, Stat::Strength, Default::default()),
        1
    );
}

/// Stefano Giovanni: "During the polling step of a political action,
/// Stefano can burn 1 blood to get +1 vote." [ANK 20220705] "The ability
/// can be used while locked, only once per referendum": locked, once.
#[test]
fn stefano_giovanni_buys_a_vote_once_while_locked() {
    let mut e = game(2);
    let caller = e.add_minion(SeatId(0), plain4(0), 4, false);
    e.add_minion(SeatId(0), plain4(1), 4, false);
    let st = e.add_minion(SeatId(1), base("Stefano Giovanni"), 3, false);
    e.state.minions[st.index()].locked = true;
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, caller));
    pick(&mut e, alloc(0, 3));
    pick_use(&mut e, st, 0);
    assert_eq!(blood(&e, st), 2);
    let req = request(&mut e);
    assert!(!req.options.iter().any(|o| uses_printed(o, st, 0)), "once");
}

/// Sutekh: "You can gain 1 pool after resolution of any successful action
/// to recruit a mummy ally.": seat 1's Ministry recruits Saatet-ta, seat 0
/// gains 1 pool; [ANK 20220218] a fizzled action counting
/// (`Requirement::ActionSuccessful`).
#[test]
fn sutekh_gains_a_pool_after_any_mummy_recruit() {
    let mut e = game(2);
    let su = e.add_minion(SeatId(0), base("Sutekh, The Dark God"), 4, false);
    e.add_minion(SeatId(0), plain4(0), 4, false);
    let m = e.add_minion(
        SeatId(1),
        textless(|c| c.clan == "Ministry" && c.capacity >= 4, 0),
        4,
        false,
    );
    e.add_minion(SeatId(1), plain4(1), 4, false);
    hand(&mut e, 1, &["Saatet-ta"]);
    until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    pick(&mut e, play(0, 0, m));
    pick_use(&mut e, su, 0);
    assert_eq!(e.state.seats[0].pool, 11);
}

/// Sutekh: "During your unlock phase, you can move 1 blood between Sutekh
/// and another Ministry you control.": one move to his Ministry, then no
/// more that phase; a non-Ministry never a target.
#[test]
fn sutekh_moves_one_blood_with_another_ministry_each_unlock_phase() {
    let mut e = game(2);
    let su = e.add_minion(SeatId(0), base("Sutekh, The Dark God"), 4, false);
    let min = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Ministry" && c.capacity >= 4, 0),
        2,
        false,
    );
    let other = e.add_minion(
        SeatId(0),
        textless(|c| c.clan == "Brujah" && c.capacity >= 4, 0),
        2,
        false,
    );
    e.add_minion(SeatId(1), plain4(1), 4, false);
    let req = to_window(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    let to = |m: MinionId, i: u8| {
        req.options.iter().copied().find(|o| {
            uses_printed(o, su, i)
                && matches!(o, Choice::UseAbility { target: Target::Minion(t), .. } if *t == m)
        })
    };
    assert!(to(other, 1).is_none() && to(other, 2).is_none());
    let give = to(min, 1).expect("offered");
    e.choose(index_of(&req, give));
    assert_eq!((blood(&e, su), blood(&e, min)), (3, 3));
    let req = request(&mut e);
    assert!(!req
        .options
        .iter()
        .any(|o| uses_printed(o, su, 1) || uses_printed(o, su, 2)));
}
