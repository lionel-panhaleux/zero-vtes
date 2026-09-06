//! Slice 1 of the effects framework: the master phase as an offer window,
//! terms at announcement, the as-played cancel window, cost layers, trifle
//! and out-of-turn accounting, the discard phase. Cards: Villein, Sudden
//! Reversal.

use zvtes::cards::{self, Target, Window};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::state::{find_card, find_library};

fn request(e: &mut Engine) -> DecisionRequest {
    match e.run() {
        Step::Decision(req) => req,
        Step::Over(_) => panic!("game over while expecting a decision"),
    }
}

fn expect(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    let req = request(e);
    assert_eq!((req.seat, req.window), (seat, window), "{:?}", req.options);
    req
}

fn pick(e: &mut Engine, want: Choice) {
    let req = request(e);
    let idx = req
        .options
        .iter()
        .position(|o| *o == want)
        .unwrap_or_else(|| panic!("option {want:?} not in {:?}", req.options));
    e.choose(idx);
}

fn play(card: u8, target: MinionId) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: None,
        target: Target::Minion(target),
    }
}

/// 3 seats at seat 0's master phase on turn 4. Seat 0 has two ready
/// vampires (0: blood 5, 1: blood 1); seat 1 has one (2).
fn table() -> Engine {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), find_card("Abraham DuSable"), 5, false);
    e.add_minion(SeatId(0), find_card("Abraham Mellon"), 1, false);
    e.add_minion(SeatId(1), find_card("Abaddon"), 5, false);
    e.state.turn = 4;
    e.at_master_phase();
    e
}

#[test]
fn implemented_definitions_cover_the_slice() {
    let implemented: Vec<&str> = LIBRARY
        .iter()
        .enumerate()
        .filter(|(i, _)| cards::lookup(*i as u16).is_some())
        .map(|(_, c)| c.name)
        .collect();
    assert_eq!(
        implemented,
        vec![
            "Academic Hunting Ground",
            "Anarch Revolt",
            "Anarch Salon",
            "Anarchist Uprising",
            "Ancilla Empowerment",
            "Archon Investigation",
            "Asylum Hunting Ground",
            "Banishment",
            "Bewitching Oration",
            "Biotech Company Hunting Ground",
            "Blood Doll",
            "Blur",
            "Body Flare",
            "Bonding",
            "Camarilla's Iron Fist",
            "Carfax Abbey",
            "Cats' Guidance",
            "Claws of the Dead",
            "Closed Session",
            "Conditioning",
            "Consanguineous Boon",
            "Conservative Agitation",
            "Consign to Oblivion",
            "Creeping Sabotage",
            "Deflection",
            "Delaying Tactics",
            "Domain Challenge",
            "Earth Control",
            "Expulsion",
            "Eyes of Argus",
            "Flash",
            "Form of Mist",
            "Form of the Cobra",
            "Guardian Angel",
            "Hidden Strength",
            "Homunculus",
            "Immortal Grapple",
            "Indomitability",
            "Kine Resources Contested",
            "Library Hunting Ground",
            "Majesty",
            "Neonate Breach",
            "Old Friends",
            "Ominous Chorus",
            "Papillon",
            "Parity Shift",
            "Park Hunting Ground",
            "Party Out Of Bounds",
            "Perfect Paragon",
            "Political Hunting Ground",
            "Protected District",
            "Reckless Agitation",
            "Roundhouse",
            "Scalpel Tongue",
            "Scorn of Adonis",
            "Side Strike",
            "Slum Hunting Ground",
            "Soak",
            "Society Hunting Ground",
            "Sudden Reversal",
            "Taste of Vitae",
            "Telepathic Tracking",
            "Telepathic Vote Counting",
            "Temple Hunting Ground",
            "Theft of Vitae",
            "Torn Signpost",
            "Underworld Hunting Ground",
            "Uptown Hunting Ground",
            "Vessel",
            "Villein",
            "Voter Captivation",
            "Warzone Hunting Ground",
            "Zoo Hunting Ground",
        ]
    );
    // Definition modes line up with the generated per-mode static half:
    // same count, and each mode's windows are exactly the reviewed
    // matrix's (the matrix is authoritative; a definition may only add the
    // engine's own hooks, none so far).
    for (i, c) in LIBRARY.iter().enumerate() {
        if let Some(def) = cards::lookup(i as u16) {
            assert_eq!(def.modes.len(), c.modes.len(), "{}", c.name);
            for (mi, (dm, lm)) in def.modes.iter().zip(c.modes).enumerate() {
                let mut want: Vec<Window> = lm
                    .windows
                    .iter()
                    .map(|w| Window::from_name(w).unwrap())
                    .collect();
                let mut have: Vec<Window> = dm.windows.to_vec();
                want.sort_by_key(|w| *w as u8);
                have.sort_by_key(|w| *w as u8);
                assert_eq!(have, want, "{} mode {mi}", c.name);
            }
        }
    }
}

#[test]
fn villein_targets_terms_blood_move_layers_and_trifle() {
    let mut e = table();
    let villein = find_library("Villein");
    e.state.seats[0].hand.push(villein);
    // Targets: seat 0's vampires with any blood; not seat 1's.
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(
        req.options,
        vec![Choice::Pass, play(0, MinionId(0)), play(0, MinionId(1))]
    );
    pick(&mut e, play(0, MinionId(0)));
    // Terms are announced at play: 2 to 5 (rules-qa 3.5).
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(
        req.options,
        vec![
            Choice::SetX(2),
            Choice::SetX(3),
            Choice::SetX(4),
            Choice::SetX(5)
        ]
    );
    pick(&mut e, Choice::SetX(4));
    // Nobody holds a cancel: the as-played window elides; the trifle refunds
    // the master phase action, but the hand is empty, so the master phase
    // elides too and the minion phase opens.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[0].blood, 1);
    assert_eq!(e.state.seats[0].pool, 34);
    assert_eq!(e.state.seats[0].mpa, 1);
    assert!(e.state.seats[0].trifle_used);
    assert_eq!(e.state.cards.len(), 1);
    assert_eq!(e.state.cards[0].host, Some(MinionId(0)));
    assert_eq!(e.state.layers.len(), 2);
    assert!(e.state.seats[0].ash.is_empty());
}

#[test]
fn villein_costs_one_more_on_a_vampire_that_already_has_one() {
    let mut e = table();
    let villein = find_library("Villein");
    e.state.seats[0].hand.extend([villein, villein]);
    pick(&mut e, play(0, MinionId(0)));
    pick(&mut e, Choice::SetX(2));
    // Trifle refunded: a second Villein this master phase. With 0 pool the
    // copy on the same vampire (cost 1) is masked out; another vampire is
    // free.
    expect(&mut e, SeatId(0), WindowLabel::MasterPhase); // first play settled
    e.state.seats[0].pool = 0;
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(req.options, vec![Choice::Pass, play(0, MinionId(1))]);
    e.state.seats[0].pool = 1;
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(req.options.contains(&play(0, MinionId(0))));
    pick(&mut e, play(0, MinionId(0)));
    pick(&mut e, Choice::SetX(3));
    // Cost 1 paid, the 3 blood left taken (pool 1 - 1 + 3): the second
    // trifle gives nothing back this master phase.
    // (Drained to 0, the vampire's mandatory hunt is elided at once.)
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 3);
    assert!(e.state.minions[0].blood == 1 && e.state.minions[0].locked);
    assert_eq!(e.state.seats[0].mpa, 0);
}

#[test]
fn sudden_reversal_cancels_a_master_and_its_cost_and_trifle_refund() {
    let mut e = table();
    e.state.seats[0].hand.push(find_library("Villein"));
    e.state.seats[1].hand.push(find_library("Sudden Reversal"));
    pick(&mut e, play(0, MinionId(0)));
    pick(&mut e, Choice::SetX(5));
    // As-played window: seat 1 may cancel (out of turn).
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::Play {
                card: 0,
                mode: 0,
                actor: None,
                target: Target::None
            }
        ]
    );
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::None,
        },
    );
    // The cancel's own as-played window has no takers; both settle.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[0].blood, 5);
    assert_eq!(e.state.seats[0].pool, 30);
    assert_eq!(e.state.seats[0].ash, vec![find_library("Villein")]);
    assert_eq!(e.state.seats[1].ash, vec![find_library("Sudden Reversal")]);
    assert!(e.state.cards.is_empty() && e.state.layers.is_empty());
    // A canceled trifle refunds nothing; the master phase action is spent.
    assert_eq!(e.state.seats[0].mpa, 0);
    assert!(e.state.seats[1].out_of_turn.is_some());
}

#[test]
fn discard_phase_replaces_from_the_library() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), find_card("Abraham DuSable"), 5, false);
    e.add_minion(SeatId(0), find_card("Abraham Mellon"), 5, false);
    let a = find_library(".44 Magnum");
    let b = find_library("AK-47");
    e.state.seats[0].hand.push(a);
    e.state.seats[0].library.push(b);
    e.state.turn = 4;
    pick(&mut e, Choice::Pass); // minion phase; influence elides
    let req = expect(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert_eq!(req.options, vec![Choice::Pass, Choice::Discard(0)]);
    pick(&mut e, Choice::Discard(0));
    assert_eq!(e.state.seats[0].ash, vec![a]);
    assert_eq!(e.state.seats[0].hand, vec![b]);
    // One discard phase action: the phase elides and the turn passes; the
    // two empty seats elide their whole turns.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.turn, 7);
}

#[test]
fn cards_without_a_definition_are_not_playable() {
    let mut e = table();
    e.state.seats[0].hand.push(find_library(".44 Magnum"));
    // Master phase with only an equipment card: Pass alone, elided.
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&Choice::TakeAction {
        minion: MinionId(0),
        action: ActionKind::Bleed,
        target: None
    }));
}
