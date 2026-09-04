//! Slice 4 of the effects framework: politics (rulebook "Political Action",
//! "The Referendum"). The political action as an action, terms chosen
//! stepwise once it succeeds (rules-qa 3.5), the polling step with votes,
//! ballots and card plays interleaved, the tally, simultaneous ousts, and
//! the after-resolution window. Windows with a single option are elided,
//! so the scripts only stop where a real decision exists.

use zvtes::actions::Outcome;
use zvtes::cards::Target;
use zvtes::decision::{Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library};

fn request(e: &mut Engine) -> DecisionRequest {
    match e.run() {
        Step::Decision(req) => {
            if std::env::var("TRACE").is_ok() {
                eprintln!("REQ {:?} {:?} {:?}", req.seat, req.window, req.options);
            }
            req
        }
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

fn play(card: u8, mode: u8, actor: MinionId) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target: Target::None,
    }
}

fn play_on(card: u8, mode: u8, actor: MinionId, target: MinionId) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target: Target::Minion(target),
    }
}

fn plays(req: &DecisionRequest) -> Vec<Choice> {
    req.options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::Play { .. }))
        .collect()
}

fn votes(minion: MinionId, favor: bool) -> Choice {
    Choice::CastVotes { minion, favor }
}

fn alloc(seat: u8, n: u8) -> Choice {
    Choice::AllocateTo {
        seat: SeatId(seat),
        n,
    }
}

fn hand(e: &mut Engine, seat: u8, cards: &[&str]) {
    for c in cards {
        e.state.seats[seat as usize].hand.push(find_library(c));
    }
}

fn pool(e: &Engine) -> Vec<i32> {
    e.state.seats.iter().map(|s| s.pool).collect()
}

/// 3 seats at seat 0's minion phase, turn 4, 10 pool each. Seat 0: Alexa
/// Draper (Prince of Melbourne, 2 votes, PRE), Mateo García (capacity 2,
/// pre; keeps seat 0's minion phase a real decision once Alexa is locked);
/// seat 1: Kevin Jackson (Prince of Chicago, 2 votes), Bret Stryker (CEL
/// PRE, capacity 4); seat 2: Lenny Burkhead (primogen, 1 vote, pre).
struct Table {
    e: Engine,
    alexa: MinionId,
    mateo: MinionId,
    kevin: MinionId,
    bret: MinionId,
    lenny: MinionId,
}

fn table() -> Table {
    let mut e = Engine::bare(3);
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    let mateo = e.add_minion(SeatId(0), find_card("Mateo García"), 2, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    let bret = e.add_minion(SeatId(1), find_card("Bret Stryker"), 4, false);
    let lenny = e.add_minion(SeatId(2), find_card("Lenny Burkhead"), 4, false);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    Table {
        e,
        alexa,
        mateo,
        kevin,
        bret,
        lenny,
    }
}

/// Everyone passes the polling step until it closes.
fn close_polling(e: &mut Engine) {
    loop {
        match e.run() {
            Step::Decision(req)
                if matches!(
                    req.window,
                    WindowLabel::ReferendumPolling | WindowLabel::BloodHuntPolling
                ) =>
            {
                e.choose(0);
            }
            _ => return,
        }
    }
}

// rules-qa 3.5 — the terms of a political action are chosen once the action
// succeeds, before any vote is cast; the allocation is stepwise, seat by
// seat clockwise from the caller, and the mask keeps "two or more
// Methuselahs" satisfiable (the last seat is forced, so elided).
#[test]
fn referendum_terms_before_votes() {
    let Table { mut e, alexa, .. } = table();
    hand(&mut e, 0, &["Kine Resources Contested"]);
    pick(&mut e, play(0, 0, alexa));
    // +1 stealth, nothing to intercept with: the block loop elides.
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert_eq!(
        req.options,
        vec![alloc(0, 0), alloc(0, 1), alloc(0, 2), alloc(0, 3)]
    );
    pick(&mut e, alloc(0, 0));
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    // Nothing to seat 1 would leave a single recipient; all 4 likewise.
    assert_eq!(req.options, vec![alloc(1, 1), alloc(1, 2), alloc(1, 3)]);
    pick(&mut e, alloc(1, 3));
    // Seat 2 gets the remaining point without a decision; polling opens at
    // the caller with its prince and the calling card's own vote.
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            votes(alexa, true),
            votes(alexa, false),
            Choice::CastCardVote { favor: true },
            Choice::CastCardVote { favor: false },
        ]
    );
    pick(&mut e, votes(alexa, true));
    close_polling(&mut e);
    assert_eq!(pool(&e), vec![10, 7, 9]);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    // The card went to the ash heap; the prince is locked and has taken its
    // political action for the turn.
    assert_eq!(
        e.state.seats[0].ash,
        vec![find_library("Kine Resources Contested")]
    );
    assert!(e.state.minions[alexa.index()].locked);
    assert!(e.state.minions[alexa.index()].political_this_turn);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
}

// Rulebook "Resolve the Referendum": tied referendums fail.
#[test]
fn tied_referendum_fails() {
    let Table {
        mut e,
        alexa,
        kevin,
        ..
    } = table();
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, alexa));
    // No terms: straight to polling.
    expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    pick(&mut e, votes(alexa, true));
    pick(&mut e, votes(kevin, false));
    close_polling(&mut e);
    assert_eq!(pool(&e), vec![10, 10, 10]);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
}

// Rulebook "Gaining Votes": the calling card's 1 vote and a political action
// card burned from hand are Methuselah ballots, at most one per Methuselah
// per referendum; the burned card is replaced.
#[test]
fn political_card_votes_once_per_methuselah() {
    let Table {
        mut e,
        alexa,
        kevin,
        lenny,
        ..
    } = table();
    hand(&mut e, 0, &["Ancilla Empowerment", "Domain Challenge"]);
    hand(&mut e, 2, &["Neonate Breach"]);
    e.state.seats[2].library.push(find_library("Villein"));
    pick(&mut e, play(0, 0, alexa));
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    // The caller may use the calling card's vote or burn the other one.
    assert!(req.options.contains(&Choice::CastCardVote { favor: true }));
    assert!(req.options.contains(&Choice::BurnForVote {
        card: 0,
        favor: true
    }));
    pick(&mut e, Choice::CastCardVote { favor: true });
    // Seat 1: no political card, its prince only.
    let req = expect(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert_eq!(
        req.options,
        vec![Choice::Pass, votes(kevin, true), votes(kevin, false)]
    );
    pick(&mut e, votes(kevin, false));
    // Seat 2 burns Neonate Breach for a vote: to the ash heap, replaced.
    pick(
        &mut e,
        Choice::BurnForVote {
            card: 0,
            favor: true,
        },
    );
    assert_eq!(e.state.seats[2].ash, vec![find_library("Neonate Breach")]);
    assert_eq!(e.state.seats[2].hand, vec![find_library("Villein")]);
    // Back at the caller: the card vote is spent, the prince remains.
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert_eq!(
        req.options,
        vec![Choice::Pass, votes(alexa, true), votes(alexa, false)]
    );
    pick(&mut e, votes(alexa, true));
    // Seat 1 has nothing left (elided); seat 2's primogen votes.
    pick(&mut e, votes(lenny, true));
    close_polling(&mut e);
    // 1 + 1 + 2 for, 2 against: passes; each Methuselah burns 1 per minion.
    assert_eq!(pool(&e), vec![8, 8, 9]);
}

// Bewitching Oration during polling: votes gained before casting fold into
// the vampire's source; gained after casting they join the cast on the same
// side (all votes of a source agree).
#[test]
fn bewitching_oration_adds_votes_before_or_after_casting() {
    let Table {
        mut e,
        alexa,
        kevin,
        lenny,
        ..
    } = table();
    hand(&mut e, 0, &["Ancilla Empowerment", "Bewitching Oration"]);
    pick(&mut e, play(0, 0, alexa));
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    // The replacement draw is empty (no library): the Oration is hand 0.
    assert_eq!(plays(&req), vec![play(0, 0, alexa), play(0, 1, alexa)]);
    pick(&mut e, votes(alexa, true));
    pick(&mut e, votes(kevin, false));
    pick(&mut e, Choice::Pass); // seat 2
                                // The play is a commitment: it reopens the rotation — seat 1 has
                                // nothing left (elided), seat 2's primogen is asked again.
    pick(&mut e, play(0, 1, alexa));
    let req = expect(&mut e, SeatId(2), WindowLabel::ReferendumPolling);
    assert_eq!(
        req.options,
        vec![Choice::Pass, votes(lenny, true), votes(lenny, false)]
    );
    close_polling(&mut e);
    // 2 + 4 for, 2 against: passes.
    assert_eq!(pool(&e), vec![8, 8, 9]);
}

// Scalpel Tongue: targets are vampires who have cast votes; the chosen
// vampire is locked, its votes are canceled, and at superior it burns 1
// blood. Playable by a reacting minion of another Methuselah.
#[test]
fn scalpel_tongue_cancels_cast_votes() {
    let Table {
        mut e, alexa, bret, ..
    } = table();
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    hand(&mut e, 1, &["Scalpel Tongue"]);
    pick(&mut e, play(0, 0, alexa));
    // Before any vote is cast there is no legal target.
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    pick(&mut e, votes(alexa, true));
    let req2 = expect(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert_eq!(
        plays(&req2),
        vec![play_on(0, 0, bret, alexa), play_on(0, 1, bret, alexa)]
    );
    assert!(plays(&req).is_empty());
    pick(&mut e, play_on(0, 1, bret, alexa));
    close_polling(&mut e);
    assert!(e.state.minions[alexa.index()].locked);
    assert_eq!(e.state.minions[alexa.index()].blood, 4);
    assert_eq!(e.state.minions[bret.index()].blood, 3);
    // 0 for, 0 against: fails.
    assert_eq!(pool(&e), vec![10, 10, 10]);
}

// Neonate Breach: "choose one or more Methuselahs" as stepwise seat choices
// (Pass once at least one is chosen); the young-vampire bonus is read when
// the effect applies.
#[test]
fn neonate_breach_chooses_seats() {
    let Table { mut e, alexa, .. } = table();
    hand(&mut e, 0, &["Neonate Breach"]);
    pick(&mut e, play(0, 0, alexa));
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert_eq!(
        req.options,
        vec![
            Choice::ChooseSeat(SeatId(0)),
            Choice::ChooseSeat(SeatId(1)),
            Choice::ChooseSeat(SeatId(2)),
        ]
    );
    pick(&mut e, Choice::ChooseSeat(SeatId(1)));
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::ChooseSeat(SeatId(0)),
            Choice::ChooseSeat(SeatId(2)),
        ]
    );
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    pick(&mut e, votes(alexa, true));
    close_polling(&mut e);
    // Seat 1 controls Bret Stryker (capacity 4): 1 + 3.
    assert_eq!(pool(&e), vec![10, 6, 10]);
}

// Domain Challenge counts locked minions when the effect applies: the
// acting vampire is locked by its own action.
#[test]
fn domain_challenge_counts_locked_minions_at_resolution() {
    let Table { mut e, alexa, .. } = table();
    hand(&mut e, 0, &["Domain Challenge"]);
    e.state.minions[MinionId(2).index()].locked = true; // Bret
    pick(&mut e, play(0, 0, alexa));
    pick(&mut e, votes(alexa, true));
    close_polling(&mut e);
    assert_eq!(pool(&e), vec![9, 9, 10]);
}

// Rulebook "Political Action": one political action per minion per turn,
// whichever card provides it; another minion may still act.
#[test]
fn one_political_action_per_minion_per_turn() {
    let Table {
        mut e,
        alexa,
        mateo,
        ..
    } = table();
    hand(&mut e, 0, &["Ancilla Empowerment", "Domain Challenge"]);
    pick(&mut e, play(0, 0, alexa));
    close_polling(&mut e);
    // Unlock Alexa as a card would: the per-turn cap still holds.
    e.state.minions[alexa.index()].locked = false;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(plays(&req), vec![play(0, 0, mateo)]);
}

// Spec §6.5.4 — pool losses of a referendum are simultaneous: every
// Methuselah at 0 is ousted at once; each predator scores the victory
// point even when ousted in the same batch, and only a surviving predator
// takes the 6 pool.
#[test]
fn simultaneous_ousts_score_predators_and_pay_survivors_only() {
    let Table { mut e, alexa, .. } = table();
    e.state.seats[0].pool = 2;
    e.state.seats[1].pool = 2;
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    pick(&mut e, play(0, 0, alexa));
    pick(&mut e, votes(alexa, true));
    close_polling(&mut e);
    // Seats 0 and 1 (2 minions each) are ousted together; seat 2 is the
    // last one standing.
    let vps = match e.run() {
        Step::Over(v) => v,
        Step::Decision(req) => panic!("expected the game to end, got {req:?}"),
    };
    // Seat 2: its prey (seat 0) ousted, +6 pool, and the survivor's VP.
    // Seat 0: its prey (seat 1) ousted — the VP, no pool.
    assert_eq!(vps, vec![2, 0, 4]);
    assert_eq!(pool(&e), vec![0, 0, 15]);
}

// A blocked political action: combat, no referendum, and the political
// action is still spent for the turn (NRA on resolution, rules-qa 1.4).
#[test]
fn blocked_political_action_calls_no_referendum() {
    let Table { mut e, alexa, .. } = table();
    // Seat 1 (prey) blocks with an ANI vampire and Cats' Guidance [ANI] for
    // the intercept.
    let ani = e.add_minion(SeatId(1), find_card("Trung Chau Pham"), 3, false);
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    hand(&mut e, 1, &["Cats' Guidance"]);
    pick(&mut e, play(0, 0, alexa));
    pick(&mut e, Choice::DeclareBlock { minion: ani });
    pick(&mut e, play(0, 1, ani));
    // One round of hand strikes, then the action ends blocked.
    while let Step::Decision(req) = e.run() {
        if req.window == WindowLabel::MinionPhase {
            break;
        }
        e.choose(0);
    }
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert_eq!(pool(&e), vec![10, 10, 10]);
    assert!(e.state.minions[alexa.index()].political_this_turn);
}

fn set_x(n: u8) -> Choice {
    Choice::SetX(n)
}

// Parity Shift: the chosen Methuselah must have more pool than the caller;
// the allocation runs over everyone but the chosen one, and a shortfall at
// resolution is re-allocated by the caller within the terms.
#[test]
fn parity_shift_chooses_a_richer_methuselah_and_reallocates_a_shortfall() {
    let Table { mut e, alexa, .. } = table();
    e.state.seats[0].pool = 5;
    e.state.seats[2].pool = 4;
    hand(&mut e, 0, &["Parity Shift"]);
    pick(&mut e, play(0, 0, alexa));
    // Only seat 1 (10 pool) has more than the caller (5): forced, elided.
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert_eq!(
        req.options,
        vec![alloc(0, 0), alloc(0, 1), alloc(0, 2), alloc(0, 3)]
    );
    pick(&mut e, alloc(0, 2));
    // Seat 2 gets the last point (elided); polling.
    expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    pick(&mut e, votes(alexa, true));
    // Pool moving during the referendum does not change the terms, but
    // only what is there moves: seat 1 is down to 2 when it passes.
    e.state.seats[1].pool = 2;
    close_polling(&mut e);
    // Shortfall: 2 points among seat 0 (at most 2) and seat 2 (at most 1).
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert_eq!(req.options, vec![alloc(0, 1), alloc(0, 2)]);
    pick(&mut e, alloc(0, 1));
    close_polling(&mut e);
    // Seat 1 is ousted by the move; seat 0 (its predator) scores and takes
    // 6 pool: 5 + 1 + 6.
    assert!(e.state.seats[1].ousted);
    assert_eq!(pool(&e), vec![12, 0, 5]);
    assert_eq!(e.state.seats[0].half_vps, 2);
}

// Camarilla's Iron Fist: the chosen Methuselah receives no points, so with
// two opponents only the caller can be chosen (ANK 20241106).
#[test]
fn iron_fist_chosen_methuselah_receives_no_points() {
    let Table { mut e, alexa, .. } = table();
    hand(&mut e, 0, &["Camarilla's Iron Fist"]);
    pick(&mut e, play(0, 0, alexa));
    // Choosing seat 0 is the only feasible choice: elided. Allocation over
    // seats 1 and 2, both receiving.
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert_eq!(
        req.options,
        vec![alloc(1, 1), alloc(1, 2), alloc(1, 3), alloc(1, 4)]
    );
    pick(&mut e, alloc(1, 2));
    expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    pick(&mut e, votes(alexa, true));
    close_polling(&mut e);
    // Cost 1 blood at resolution; seat 0 gains 1.
    assert_eq!(e.state.minions[alexa.index()].blood, 4);
    assert_eq!(pool(&e), vec![11, 8, 7]);
}

// Reckless Agitation: an Independent or Anarch of capacity 5+, and at least
// two opponents to allocate among (LSJ 20100129) — the play is masked, not
// the terms.
#[test]
fn reckless_agitation_needs_two_opponents() {
    let Table { mut e, .. } = table();
    let saku = e.add_minion(SeatId(0), find_card("Saku Pihlajamäki"), 5, false);
    hand(&mut e, 0, &["Reckless Agitation"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(plays(&req), vec![play(0, 0, saku)]);
    e.state.seats[2].ousted = true;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(plays(&req).is_empty());
}

// Banishment: a ready vampire younger than the acting one goes to its
// controller's uncontrolled region with its blood.
#[test]
fn banishment_sends_a_younger_vampire_to_the_uncontrolled_region() {
    let Table {
        mut e,
        alexa,
        mateo,
        kevin,
        bret,
        lenny,
    } = table();
    hand(&mut e, 0, &["Banishment"]);
    pick(&mut e, play(0, 0, alexa));
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumTerms);
    assert_eq!(
        req.options,
        vec![
            Choice::ChooseMinion(mateo),
            Choice::ChooseMinion(kevin),
            Choice::ChooseMinion(bret),
            Choice::ChooseMinion(lenny),
        ]
    );
    pick(&mut e, Choice::ChooseMinion(bret));
    pick(&mut e, votes(alexa, true));
    close_polling(&mut e);
    assert!(!e.state.minions[bret.index()].in_play);
    let slot = &e.state.seats[1].uncontrolled[0];
    assert_eq!((slot.card, slot.counters), (find_card("Bret Stryker"), 4));
}

// Expulsion: up to two minions; the chosen ones cannot block, react or
// vote this turn.
#[test]
fn expulsion_bars_blocks_reactions_and_votes_this_turn() {
    let Table {
        mut e,
        alexa,
        mateo,
        kevin,
        bret,
        ..
    } = table();
    hand(&mut e, 0, &["Expulsion", "Ancilla Empowerment"]);
    hand(&mut e, 1, &["Scalpel Tongue"]);
    pick(&mut e, play(0, 0, alexa));
    pick(&mut e, Choice::ChooseMinion(kevin));
    pick(&mut e, Choice::ChooseMinion(bret));
    pick(&mut e, votes(alexa, true));
    close_polling(&mut e);
    // Mateo calls a referendum: seat 1 has no vote and no reaction, so
    // the rotation skips it.
    pick(&mut e, play(0, 0, mateo));
    pick(&mut e, votes(alexa, true));
    expect(&mut e, SeatId(2), WindowLabel::ReferendumPolling);
    // Unlock Mateo as a card would: seat 1 cannot block his bleed either
    // (no candidate: the block loop elides).
    e.state.minions[mateo.index()].locked = false;
    close_polling(&mut e);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: mateo,
            action: zvtes::decision::ActionKind::Bleed,
            target: None,
        },
    );
    let _ = request(&mut e);
    assert_eq!(pool(&e), vec![8, 7, 9]);
}

// Delaying Tactics: the referendum is canceled without a tally, the acting
// vampire unlocks, the political action card returns to hand (discard down
// afterward), and the acting Methuselah's minions cannot perform that
// political action again this turn — the acting vampire has still taken
// its political action.
#[test]
fn delaying_tactics_cancels_returns_the_card_and_forbids_a_repeat() {
    let Table {
        mut e,
        alexa,
        mateo,
        bret,
        ..
    } = table();
    hand(
        &mut e,
        0,
        &[
            "Kine Resources Contested",
            "Ancilla Empowerment",
            "Villein",
            "Villein",
            "Villein",
            "Villein",
            "Villein",
        ],
    );
    e.state.seats[0].library.push(find_library("Villein"));
    hand(&mut e, 1, &["Delaying Tactics"]);
    pick(&mut e, play(0, 0, alexa));
    pick(&mut e, alloc(0, 0));
    pick(&mut e, alloc(1, 3));
    pick(&mut e, votes(alexa, true));
    pick(&mut e, play(0, 0, bret));
    // Discard down: 8 cards in hand, the returned card last.
    let req = expect(&mut e, SeatId(0), WindowLabel::DiscardDown);
    assert_eq!(req.options.len(), 8);
    assert_eq!(
        e.state.seats[0].hand[7],
        find_library("Kine Resources Contested")
    );
    pick(&mut e, Choice::Discard(2));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10, 10]);
    assert_eq!(e.state.minions[bret.index()].blood, 3);
    assert!(!e.state.minions[alexa.index()].locked);
    assert!(e.state.minions[alexa.index()].political_this_turn);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    // Mateo may still call Ancilla Empowerment (hand 0), not the returned
    // card (hand 6).
    assert_eq!(e.state.seats[0].hand.len(), 7);
    assert_eq!(plays(&req), vec![play(0, 0, mateo)]);
}

// Telepathic Vote Counting [AUS]: a vampire is forced to abstain — before
// it votes, it never gets to.
#[test]
fn telepathic_vote_counting_forces_abstention() {
    let Table { mut e, kevin, .. } = table();
    let silverson = e.add_minion(SeatId(0), find_card("Alexander Silverson"), 5, false);
    hand(
        &mut e,
        0,
        &["Ancilla Empowerment", "Telepathic Vote Counting"],
    );
    pick(&mut e, play(0, 0, silverson));
    pick(&mut e, play_on(0, 1, silverson, kevin));
    // Seat 1 has nothing left to cast (elided): seat 2 is asked.
    let req = expect(&mut e, SeatId(2), WindowLabel::ReferendumPolling);
    assert!(!req.options.contains(&votes(kevin, false)));
    pick(&mut e, Choice::Pass);
    pick(&mut e, votes(silverson, true));
    close_polling(&mut e);
    // Unopposed: passes; seat 0 controls three minions.
    assert_eq!(pool(&e), vec![7, 8, 9]);
}

// Closed Session: before any vote is cast, and then non-Camarilla vampires
// cast nothing.
#[test]
fn closed_session_bars_non_camarilla_votes() {
    let Table { mut e, alexa, .. } = table();
    let saku = e.add_minion(SeatId(1), find_card("Saku Pihlajamäki"), 5, false);
    hand(
        &mut e,
        0,
        &["Ancilla Empowerment", "Closed Session", "Closed Session"],
    );
    pick(&mut e, play(0, 0, alexa));
    pick(&mut e, play(0, 0, alexa));
    let req = expect(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert!(!req.options.contains(&votes(saku, true)));
    pick(&mut e, Choice::Pass);
    pick(&mut e, Choice::Pass); // seat 2
                                // Votes cast: the second copy is no longer usable.
    pick(&mut e, votes(alexa, true));
    pick(&mut e, Choice::Pass); // seat 1 (Kevin abstains)
    pick(&mut e, Choice::Pass); // seat 2
    let req = expect(&mut e, SeatId(0), WindowLabel::ReferendumPolling);
    assert!(plays(&req).is_empty());
    close_polling(&mut e);
    assert_eq!(pool(&e), vec![8, 7, 9]);
}

// Protected District during polling: +3 votes against, which forces the
// primogen's cast against; unusable once it voted in favor.
#[test]
fn protected_district_votes_against() {
    let Table {
        mut e,
        alexa,
        lenny,
        ..
    } = table();
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    hand(&mut e, 2, &["Protected District"]);
    pick(&mut e, play(0, 0, alexa));
    pick(&mut e, votes(alexa, true));
    pick(&mut e, Choice::Pass); // seat 1
    pick(&mut e, play(0, 0, lenny));
    pick(&mut e, Choice::Pass); // seat 0 keeps its card vote
    pick(&mut e, Choice::Pass); // seat 1 (Kevin abstains)
                                // Lenny again, against only.
    let req = expect(&mut e, SeatId(2), WindowLabel::ReferendumPolling);
    assert_eq!(req.options, vec![Choice::Pass, votes(lenny, false)]);
    pick(&mut e, votes(lenny, false));
    close_polling(&mut e);
    // 2 for, 1 + 3 against.
    assert_eq!(pool(&e), vec![10, 10, 10]);
}

// Protected District during a bleed against its controller: -3 bleed.
#[test]
fn protected_district_reduces_a_bleed() {
    let Table { mut e, mateo, .. } = table();
    let chrys = e.add_minion(SeatId(1), find_card("Chrysanthemum"), 3, false);
    hand(&mut e, 1, &["Protected District"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: mateo,
            action: zvtes::decision::ActionKind::Bleed,
            target: None,
        },
    );
    pick(&mut e, play(0, 0, chrys));
    pick(&mut e, Choice::Pass); // no block
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10, 10]);
}

// Scorn of Adonis reaches votes cast before it was played: 1 pool per
// Methuselah who voted against, at the tally, before the effects.
#[test]
fn scorn_of_adonis_burns_pool_of_those_who_voted_against() {
    let Table { mut e, kevin, .. } = table();
    let catalina = e.add_minion(SeatId(0), find_card("Catalina Vega"), 5, false);
    hand(&mut e, 0, &["Ancilla Empowerment", "Scorn of Adonis"]);
    pick(&mut e, play(0, 0, catalina));
    pick(&mut e, votes(catalina, true));
    pick(&mut e, votes(kevin, false));
    pick(&mut e, Choice::Pass); // seat 2
    pick(&mut e, play(0, 0, catalina));
    close_polling(&mut e);
    // Tied: fails; seat 1 still burns 1 for voting against.
    assert_eq!(pool(&e), vec![10, 9, 10]);
}

// Old Friends: "do not replace until your next unlock phase" — the hand
// stays one short until then, and the card is replaced before unlocking.
#[test]
fn old_friends_is_replaced_at_the_next_unlock_phase() {
    let Table { mut e, .. } = table();
    let silverson = e.add_minion(SeatId(0), find_card("Alexander Silverson"), 5, false);
    // A full hand: the played card is not replaced until the unlock phase.
    hand(&mut e, 0, &["Old Friends"]);
    for _ in 0..6 {
        e.state.seats[0].hand.push(find_library("Villein"));
    }
    for _ in 0..3 {
        e.state.seats[0].library.push(find_library("Villein"));
    }
    pick(
        &mut e,
        Choice::TakeAction {
            minion: silverson,
            action: zvtes::decision::ActionKind::Bleed,
            target: None,
        },
    );
    pick(&mut e, play(0, 0, silverson));
    pick(&mut e, Choice::Pass); // seat 1 declines to block
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 8, 10]);
    assert_eq!(e.state.seats[0].hand.len(), 6);
    assert_eq!(e.state.minions[silverson.index()].blood, 4);
    // Through the other seats' turns: still not replaced.
    let turn = e.state.turn;
    while e.state.turn < turn + 3 {
        match e.run() {
            Step::Decision(_) => e.choose(0),
            Step::Over(_) => panic!("game over"),
        }
    }
    assert_eq!(e.state.seats[0].hand.len(), 7);
    assert_eq!(e.state.seats[0].library.len(), 2);
}

// Party Out Of Bounds [obf]: an Anarch reduces a bleed against its
// controller by 2.
#[test]
fn party_out_of_bounds_reduces_a_bleed() {
    let Table { mut e, mateo, .. } = table();
    let sergio = e.add_minion(SeatId(1), find_card("Sergio Bueno"), 5, false);
    hand(&mut e, 1, &["Party Out Of Bounds"]);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: mateo,
            action: zvtes::decision::ActionKind::Bleed,
            target: None,
        },
    );
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(plays(&req), vec![play(0, 0, sergio)]);
    pick(&mut e, play(0, 0, sergio));
    pick(&mut e, Choice::Pass); // no block
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 10, 10]);
}

// Perfect Paragon [PRE]: younger vampires get -1 intercept for the rest of
// the action — an attempt that can no longer reach the stealth is pruned.
#[test]
fn perfect_paragon_penalizes_younger_intercept() {
    let Table { mut e, alexa, .. } = table();
    let trung = e.add_minion(SeatId(1), find_card("Trung Chau Pham"), 3, false);
    hand(&mut e, 0, &["Ancilla Empowerment", "Perfect Paragon"]);
    hand(&mut e, 1, &["Cats' Guidance"]);
    pick(&mut e, play(0, 0, alexa));
    // Block loop: the acting Methuselah first (the block attempt is
    // reachable with Cats' Guidance, so the window is real).
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert_eq!(plays(&req), vec![play(0, 1, alexa)]);
    pick(&mut e, play(0, 1, alexa));
    // Seat 1: Trung's +1 intercept minus 1 can no longer reach +1 stealth.
    let req = request(&mut e);
    assert!(!req
        .options
        .contains(&Choice::DeclareBlock { minion: trung }));
}

// Voter Captivation after a passed referendum: 1 blood per vote of the
// margin; at superior, up to 2 of them to the pool, chosen as terms.
#[test]
fn voter_captivation_gains_blood_by_the_margin() {
    let Table { mut e, alexa, .. } = table();
    hand(&mut e, 0, &["Ancilla Empowerment", "Voter Captivation"]);
    pick(&mut e, play(0, 0, alexa));
    pick(&mut e, votes(alexa, true));
    close_polling(&mut e);
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionAfterResolution);
    assert_eq!(plays(&req), vec![play(0, 0, alexa), play(0, 1, alexa)]);
    pick(&mut e, play(0, 1, alexa));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(req.options, vec![set_x(0), set_x(1), set_x(2)]);
    pick(&mut e, set_x(1));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[alexa.index()].blood, 6);
    assert_eq!(pool(&e), vec![9, 8, 9]);
}

// Consanguineous Boon (a clan chosen as terms) and Anarch Salon (requires
// an Anarch).
#[test]
fn consanguineous_boon_and_anarch_salon() {
    let Table { mut e, alexa, .. } = table();
    let saku = e.add_minion(SeatId(0), find_card("Saku Pihlajamäki"), 5, false);
    hand(&mut e, 0, &["Consanguineous Boon", "Anarch Salon"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Anarch Salon needs the Anarch; the Boon anyone.
    assert!(plays(&req).contains(&play(1, 0, saku)));
    assert!(!plays(&req).contains(&play(1, 0, alexa)));
    pick(&mut e, play(0, 0, alexa));
    pick(&mut e, Choice::ChooseClan("Ventrue"));
    pick(&mut e, votes(alexa, true));
    close_polling(&mut e);
    // Alexa and Kevin are Ventrue.
    assert_eq!(pool(&e), vec![11, 11, 10]);
    pick(&mut e, play(0, 0, saku));
    pick(&mut e, votes(saku, true));
    close_polling(&mut e);
    assert_eq!(pool(&e), vec![12, 11, 10]);
    assert_eq!(e.state.minions[saku.index()].blood, 6);
}

// Ominous Chorus: a Lasombra only, acting or reacting.
#[test]
fn ominous_chorus_requires_a_lasombra() {
    let Table { mut e, alexa, .. } = table();
    let kamal = e.add_minion(SeatId(1), find_card("Kamaluddin"), 4, false);
    hand(&mut e, 0, &["Ancilla Empowerment"]);
    hand(&mut e, 1, &["Ominous Chorus"]);
    pick(&mut e, play(0, 0, alexa));
    pick(&mut e, votes(alexa, true));
    let req = expect(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert_eq!(plays(&req), vec![play(0, 0, kamal)]);
    pick(&mut e, play(0, 0, kamal));
    pick(&mut e, Choice::Pass); // seat 2
    pick(&mut e, Choice::Pass); // seat 0 keeps its card vote
    pick(&mut e, votes(kamal, false));
    close_polling(&mut e);
    // 2 for, 1 + 3 against: fails.
    assert_eq!(pool(&e), vec![10, 10, 10]);
}
