//! Slice 5 of the effects framework: a master card in play with no host,
//! a mandatory unlock-phase rider fired for every Methuselah at their own
//! unlock phase, and an action granted by a card in play (the stress set's
//! "ongoing global" card, Anarch Revolt). Windows with a single option are
//! elided, so the scripts only stop where a real decision exists.

use zvtes::actions::Outcome;
use zvtes::cards::{Layer, Modifier, Scope, Target};
use zvtes::crypt::CRYPT;
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, ActionId, CardId, CardInPlay};

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

fn master_play(card: u8) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: None,
        target: Target::None,
    }
}

fn granted(card: u32, actor: MinionId) -> Choice {
    Choice::UseAbility {
        card,
        ability: 0,
        actor: Some(actor),
        target: Target::None,
    }
}

fn votes(minion: MinionId, favor: bool) -> Choice {
    Choice::CastVotes { minion, favor }
}

fn hand(e: &mut Engine, seat: u8, cards: &[&str]) {
    for c in cards {
        e.state.seats[seat as usize].hand.push(find_library(c));
    }
}

fn pool(e: &Engine) -> Vec<i32> {
    e.state.seats.iter().map(|s| s.pool).collect()
}

/// Anarch Revolt in play under `owner`, with no play (the master phase
/// path is `anarch_revolt_enters_play_with_no_host`).
fn revolt_in_play(e: &mut Engine, owner: u8) -> u32 {
    e.state.cards.push(CardInPlay {
        card: find_library("Anarch Revolt"),
        owner: SeatId(owner),
        host: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
    });
    e.state.cards.len() as u32 - 1
}

/// Pass every window until `seat`'s minion phase opens (with empty hands
/// the master phase has only Pass, so it is elided); pending mandatory
/// unlock-phase riders are resolved in offer order.
fn to_minion_phase(e: &mut Engine, seat: SeatId) -> DecisionRequest {
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, WindowLabel::MinionPhase) {
            return req;
        }
        assert!(
            matches!(req.options[0], Choice::Pass | Choice::ResolveRider { .. }),
            "{:?}",
            req
        );
        e.choose(0);
    }
}

/// Everyone passes the polling step until it closes.
fn close_polling(e: &mut Engine) {
    loop {
        match e.run() {
            Step::Decision(req) if req.window == WindowLabel::ReferendumPolling => {
                e.choose(0);
            }
            _ => return,
        }
    }
}

/// 3 seats at seat 0's master phase, turn 4, 10 pool each. Seat 0: Alexa
/// Draper (Camarilla Prince of Melbourne, 2 votes); seat 1: Kevin Jackson
/// (Camarilla Prince of Chicago, 2 votes) and Anita Wainwright (Anarch,
/// capacity 3); seat 2: Lenny Burkhead (Camarilla primogen, 1 vote).
struct Table {
    e: Engine,
    alexa: MinionId,
    kevin: MinionId,
    anita: MinionId,
    lenny: MinionId,
}

fn table() -> Table {
    let mut e = Engine::bare(3);
    let alexa = e.add_minion(SeatId(0), find_card("Alexa Draper"), 5, false);
    let kevin = e.add_minion(SeatId(1), find_card("Kevin Jackson"), 5, false);
    let anita = e.add_minion(SeatId(1), find_card("Anita Wainwright"), 3, false);
    let lenny = e.add_minion(SeatId(2), find_card("Lenny Burkhead"), 4, false);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    e.at_master_phase();
    Table {
        e,
        alexa,
        kevin,
        anita,
        lenny,
    }
}

// "Put this card in play": a master card with no host, kept in play (not
// discarded), one master phase action.
#[test]
fn anarch_revolt_enters_play_with_no_host() {
    let Table { mut e, .. } = table();
    hand(&mut e, 0, &["Anarch Revolt"]);
    pick(&mut e, master_play(0));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let c = &e.state.cards[0];
    assert!(c.in_play && c.host.is_none() && c.owner == SeatId(0));
    assert_eq!(c.card, find_library("Anarch Revolt"));
    assert!(e.state.seats[0].hand.is_empty() && e.state.seats[0].ash.is_empty());
}

// "A Methuselah not controlling a ready Anarch burns 1 pool during their
// unlock phase": every Methuselah at their own unlock phase, the card's
// controller included; a ready Anarch exempts (locked at the turn's start,
// she unlocks before the check); a torpid one does not (spec §5.2.2: the
// condition is checked when the phase ends).
#[test]
fn anarch_revolt_burns_pool_at_each_unlock_phase_without_a_ready_anarch() {
    let Table { mut e, anita, .. } = table();
    revolt_in_play(&mut e, 0);
    e.state.minions[anita.index()].locked = true;
    to_minion_phase(&mut e, SeatId(1));
    // Seat 1 controls a ready Anarch: no burn.
    assert_eq!(pool(&e), vec![10, 10, 10]);
    to_minion_phase(&mut e, SeatId(2));
    assert_eq!(pool(&e), vec![10, 10, 9]);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e), vec![9, 10, 9]);
    // Anita in torpor: seat 1 burns too.
    e.state.minions[anita.index()].in_torpor = true;
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(pool(&e), vec![9, 9, 9]);
}

// Spec §1.15: each copy in play burns on its own.
#[test]
fn anarch_revolt_copies_burn_cumulatively() {
    let Table { mut e, .. } = table();
    revolt_in_play(&mut e, 0);
    revolt_in_play(&mut e, 2);
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(2));
    assert_eq!(pool(&e), vec![10, 10, 8]);
}

// Pool reaching 0 in the unlock phase: ousted at once, the turn passes; the
// predator scores the victory point and the 6 pool.
#[test]
fn anarch_revolt_ousts_at_unlock_and_the_turn_passes() {
    let Table { mut e, .. } = table();
    revolt_in_play(&mut e, 0);
    e.state.seats[2].pool = 1;
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(0));
    assert!(e.state.seats[2].ousted);
    assert_eq!(pool(&e), vec![9, 16, 0]);
    assert_eq!(e.state.seats[1].half_vps, 2);
}

// "Vampires can call a referendum to burn this card as a +1 stealth
// political action": granted to any Methuselah's ready unlocked vampire
// during their minion phase, a political action (one per minion per turn)
// with no calling card to vote with; a passed referendum burns the card to
// its owner's ash heap.
#[test]
fn anarch_revolt_referendum_burns_it() {
    let Table {
        mut e,
        alexa,
        kevin,
        anita,
        lenny,
    } = table();
    let revolt = revolt_in_play(&mut e, 0);
    let req = to_minion_phase(&mut e, SeatId(1));
    let granted_options: Vec<Choice> = req
        .options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::UseAbility { .. }))
        .collect();
    assert_eq!(
        granted_options,
        vec![granted(revolt, kevin), granted(revolt, anita)]
    );
    pick(&mut e, granted(revolt, kevin));
    assert!(e.state.minions[kevin.index()].locked);
    // Polling: no calling card, so no card vote for the caller.
    let req = expect(&mut e, SeatId(1), WindowLabel::ReferendumPolling);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::CastCardVote { .. })));
    pick(&mut e, votes(kevin, true));
    close_polling(&mut e);
    expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(!e.state.cards[revolt as usize].in_play);
    assert_eq!(e.state.seats[0].ash, vec![find_library("Anarch Revolt")]);
    assert!(e.state.minions[kevin.index()].political_this_turn);
    assert_eq!(pool(&e), vec![10, 10, 10]);
    // Gone: no Methuselah burns pool at their unlock phase any more.
    to_minion_phase(&mut e, SeatId(2));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e), vec![10, 10, 10]);
    let _ = (alexa, lenny);
}

// A failed referendum leaves the card in play; the acting vampire has used
// its one political action this turn.
#[test]
fn anarch_revolt_failed_referendum_keeps_it() {
    let Table {
        mut e,
        alexa,
        kevin,
        anita,
        ..
    } = table();
    let revolt = revolt_in_play(&mut e, 0);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, granted(revolt, kevin));
    pick(&mut e, votes(kevin, true));
    pick(&mut e, Choice::Pass); // seat 2 (Lenny)
    pick(&mut e, votes(alexa, false));
    close_polling(&mut e);
    let req = expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.cards[revolt as usize].in_play);
    // Kevin, unlocked again, cannot call it a second time this turn; Anita
    // still can.
    e.state.minions[kevin.index()].locked = false;
    let req2 = expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let _ = req;
    let granted_options: Vec<Choice> = req2
        .options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::UseAbility { .. }))
        .collect();
    assert_eq!(granted_options, vec![granted(revolt, anita)]);
}

// Delaying Tactics on a granted referendum: nothing returns to hand (there
// is no calling card), the acting vampire unlocks, and the seat's minions
// cannot call the same card's referendum again this turn.
#[test]
fn anarch_revolt_referendum_canceled_by_delaying_tactics() {
    let Table {
        mut e,
        alexa,
        kevin,
        anita,
        ..
    } = table();
    let revolt = revolt_in_play(&mut e, 0);
    hand(&mut e, 0, &["Delaying Tactics"]);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, granted(revolt, kevin));
    pick(&mut e, votes(kevin, true));
    pick(&mut e, Choice::Pass); // seat 2 (Lenny)
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: Some(alexa),
            target: Target::None,
        },
    );
    let req = expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(e.state.cards[revolt as usize].in_play);
    assert!(e.state.seats[1].hand.is_empty());
    assert!(!e.state.minions[kevin.index()].locked);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::UseAbility { .. })));
    let _ = anita;
}

// rules-qa 6.1: the active Methuselah orders the mandatory unlock-phase
// effects among their own (two copies: either first, no Pass until both are
// resolved), and resolves them before the impulse passes to other players'
// unlock-phase abilities (seat 0's Homunculus sees the pool already burned).
#[test]
fn anarch_revolt_burn_is_ordered_by_the_active_methuselah_before_the_impulse_passes() {
    let Table { mut e, alexa, .. } = table();
    let first = revolt_in_play(&mut e, 0);
    let second = revolt_in_play(&mut e, 2);
    e.state.cards.push(CardInPlay {
        card: find_library("Homunculus"),
        owner: SeatId(0),
        host: Some(alexa),
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
    });
    e.state.minions[alexa.index()].locked = true;
    // Seat 0's own turn ends; seat 1 has a ready Anarch, so its two riders
    // resolve to nothing (still ordered: two options, no Pass).
    let req = to_unlock_abilities(&mut e, SeatId(1));
    assert_eq!(
        req.options,
        vec![
            Choice::ResolveRider { card: first },
            Choice::ResolveRider { card: second }
        ]
    );
    pick(&mut e, Choice::ResolveRider { card: second });
    // The last one elides; then seat 0 may use Homunculus (Alexa locked).
    let req = expect(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert_eq!(pool(&e), vec![10, 10, 10]);
    assert_eq!(req.options.len(), 2);
    pick(&mut e, Choice::Pass);
    to_minion_phase(&mut e, SeatId(1));
    // Seat 2's unlock phase: no Anarch; both burns are resolved before
    // seat 0's Homunculus window opens.
    e.state.minions[alexa.index()].locked = true;
    let req = to_unlock_abilities(&mut e, SeatId(2));
    pick(&mut e, req.options[0]);
    expect(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert_eq!(pool(&e), vec![10, 10, 8]);
}

/// Pass every window until `seat`'s unlock-phase abilities window opens
/// for `seat` itself.
fn to_unlock_abilities(e: &mut Engine, seat: SeatId) -> DecisionRequest {
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, WindowLabel::UnlockAbilities) {
            return req;
        }
        assert_eq!(req.options[0], Choice::Pass, "{:?}", req);
        e.choose(0);
    }
}

// rules-qa 1.4 / 6.2: NRA for a granted action is per copy of the card in
// play — and a political action counts against the one-per-minion cap
// besides. Kevin calls the first copy's referendum (it fails); the record
// names that copy. With the political cap lifted by hand (not a legal
// state: it isolates the copy rule), the other copy is still offered and
// the first is not.
#[test]
fn granted_action_nra_is_per_copy() {
    let Table {
        mut e,
        alexa,
        kevin,
        anita,
        ..
    } = table();
    let first = revolt_in_play(&mut e, 0);
    let second = revolt_in_play(&mut e, 2);
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, granted(first, kevin));
    pick(&mut e, votes(kevin, true));
    pick(&mut e, Choice::Pass); // seat 2 (Lenny)
    pick(&mut e, votes(alexa, false));
    close_polling(&mut e);
    expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[kevin.index()].acted,
        vec![ActionId::InPlay(CardId(first))]
    );
    e.state.minions[kevin.index()].locked = false;
    e.state.minions[kevin.index()].political_this_turn = false;
    let req = expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
    let granted_options: Vec<Choice> = req
        .options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::UseAbility { .. }))
        .collect();
    assert_eq!(
        granted_options,
        vec![
            granted(first, anita),
            granted(second, kevin),
            granted(second, anita)
        ]
    );
}

// ---- Guardian Angel, Creeping Sabotage ---------------------------------

fn vampire_with(disc: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(_, c)| c.disciplines.contains(&disc))
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no vampire with {disc}"))
}

fn vampire_of_clan(clan: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(_, c)| c.clan == clan)
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no {clan}"))
}

fn bleed(minion: MinionId) -> Choice {
    Choice::TakeAction {
        minion,
        action: ActionKind::Bleed,
        target: None,
    }
}

fn block(minion: MinionId) -> Choice {
    Choice::DeclareBlock { minion }
}

fn card_play(card: u8, mode: u8, actor: MinionId) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target: Target::None,
    }
}

/// A Guardian Angel already in play on `host` (its intercept layer
/// included, as the play would have pushed it).
fn guardian_angel_on(e: &mut Engine, owner: u8, host: MinionId) -> u32 {
    e.state.cards.push(CardInPlay {
        card: find_library("Guardian Angel"),
        owner: SeatId(owner),
        host: Some(host),
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
    });
    let id = e.state.cards.len() as u32 - 1;
    e.state.layers.push(Layer {
        scope: Scope::InPlay,
        modifier: Modifier::HostInterceptOnBleedAtYou(1),
        source: CardId(id),
    });
    id
}

fn blood(e: &Engine, m: MinionId) -> u8 {
    e.state.minions[m.index()].blood
}

/// 3 seats at seat 0's minion phase, turn 4, 10 pool each: seat 0 has a
/// (DOM, 6 blood) and a second vampire (so its minion phase stays a real
/// decision once a is locked), seat 1 x (4 blood), seat 2 z (4 blood).
fn bleed_table() -> (Engine, MinionId, MinionId, MinionId) {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 6, false);
    e.add_minion(SeatId(0), vampire_with("CEL", 0), 6, false);
    let x = e.add_minion(SeatId(1), vampire_with("FOR", 0), 4, false);
    let z = e.add_minion(SeatId(2), vampire_with("PRO", 0), 4, false);
    for s in &mut e.state.seats {
        s.pool = 10;
    }
    e.state.turn = 4;
    (e, a, x, z)
}

// "This vampire gets +1 intercept during bleed actions directed at you":
// with Guardian Angel on the prey's vampire, Bonding's +1 stealth no longer
// beats the block at closure (1 intercept ≥ 1 stealth); the card also lets
// the host prevent 1 damage as an in-play ability in the prevention window
// (no as-played window).
#[test]
fn guardian_angel_intercept_on_bleeds_at_its_controller_and_prevention() {
    let (mut e, a, x, _z) = bleed_table();
    let angel = guardian_angel_on(&mut e, 1, x);
    hand(&mut e, 0, &["Bonding"]);
    pick(&mut e, bleed(a));
    pick(&mut e, Choice::Pass); // seat 0: no modifier before an attempt
    pick(&mut e, block(x));
    pick(&mut e, card_play(0, 1, a)); // [DOM] +1 stealth, +1 bleed
                                      // Blocked all the same: combat; a's hand strike puts 1 damage on x.
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::UseAbility {
                card: angel,
                ability: 0,
                actor: None,
                target: Target::None
            }
        ]
    );
    pick(&mut e, req.options[1]);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert_eq!((blood(&e, a), blood(&e, x)), (5, 4));
    assert!(e.state.cards[angel as usize].in_play);
}

// Guardian Angel gives no intercept outside bleed actions directed at its
// controller: against a hunt (undirected, +1 stealth) the host cannot reach
// 1 intercept, so no attempt is even offered (reachability) and the hunt
// resolves.
#[test]
fn guardian_angel_gives_nothing_outside_bleeds_at_its_controller() {
    let (mut e, a, x, _z) = bleed_table();
    guardian_angel_on(&mut e, 1, x);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Hunt,
            target: None,
        },
    );
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    assert_eq!(blood(&e, a), 7);
}

// "Can prevent 1 damage each combat" is per copy: against a 3-damage
// Roundhouse, the first Guardian Angel prevents 1, the window reopens with
// only the second copy offered (the first is spent), the second prevents 1,
// and the last point lands.
#[test]
fn guardian_angel_prevents_once_per_combat_per_copy() {
    let (mut e, _a, x, _z) = bleed_table();
    let a = e.add_minion(SeatId(0), vampire_with("POT", 0), 6, false);
    let first = guardian_angel_on(&mut e, 1, x);
    let second = guardian_angel_on(&mut e, 1, x);
    hand(&mut e, 0, &["Roundhouse"]);
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    pick(&mut e, card_play(0, 0, a)); // [pot] hand strike +2: 3 damage
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::UseAbility {
                card: first,
                ability: 0,
                actor: None,
                target: Target::None
            },
            Choice::UseAbility {
                card: second,
                ability: 0,
                actor: None,
                target: Target::None
            }
        ]
    );
    pick(&mut e, req.options[1]);
    let req = expect(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::UseAbility {
                card: second,
                ability: 0,
                actor: None,
                target: Target::None
            }
        ]
    );
    pick(&mut e, req.options[1]);
    // 1 damage left and nothing to prevent it with: the window elides.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, x), 3);
    assert!(e.state.cards[first as usize].in_play && e.state.cards[second as usize].in_play);
}

// Guardian Angel played: on a ready controlled vampire only (a torpid one
// is not offered), 2 pool, the card on the host with its intercept layer.
#[test]
fn guardian_angel_play_puts_it_on_a_ready_vampire_with_its_layer() {
    let Table { mut e, alexa, .. } = table();
    let torpid = e.add_minion(SeatId(0), find_card("Abraham DuSable"), 3, true);
    hand(&mut e, 0, &["Guardian Angel"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let targets: Vec<Target> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::Play { target, .. } => Some(*target),
            _ => None,
        })
        .collect();
    assert_eq!(targets, vec![Target::Minion(alexa)]);
    let _ = torpid;
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(alexa),
        },
    );
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 8);
    assert_eq!(e.state.cards[0].host, Some(alexa));
    assert_eq!(
        e.state.layers,
        vec![Layer {
            scope: Scope::InPlay,
            modifier: Modifier::HostInterceptOnBleedAtYou(1),
            source: CardId(0),
        }]
    );
}

// "If this vampire is in torpor, burn this card": the card leaves play as
// the host goes to torpor, inside the combat.
#[test]
fn guardian_angel_burned_when_its_host_goes_to_torpor() {
    let (mut e, a, x, _z) = bleed_table();
    e.state.minions[x.index()].blood = 0;
    let angel = guardian_angel_on(&mut e, 1, x);
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    expect(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    pick(&mut e, Choice::Pass); // x takes the hand strike at 0 blood
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert!(!e.state.cards[angel as usize].in_play);
    assert_eq!(e.state.seats[1].ash, vec![find_library("Guardian Angel")]);
}

fn fame_on(e: &mut Engine, owner: u8, host: MinionId) -> u32 {
    e.state.cards.push(CardInPlay {
        card: find_library("Fame"),
        owner: SeatId(owner),
        host: Some(host),
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
    });
    e.state.cards.len() as u32 - 1
}

// "Put this card on a ready vampire": any controller's — the master phase
// offers every ready vampire on the table, and the card sits on the prey's
// vampire under its player's ownership, with no layer.
#[test]
fn fame_goes_on_any_ready_vampire() {
    let Table {
        mut e,
        alexa: a,
        kevin: x,
        lenny: z,
        ..
    } = table();
    e.state.minions[z.index()].in_torpor = true;
    hand(&mut e, 0, &["Fame"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let targets: Vec<Target> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::Play { target, .. } => Some(*target),
            _ => None,
        })
        .collect();
    assert!(targets.contains(&Target::Minion(a)));
    assert!(targets.contains(&Target::Minion(x)));
    assert!(!targets.contains(&Target::Minion(z)));
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(x),
        },
    );
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[0].host, Some(x));
    assert_eq!(e.state.cards[0].owner, SeatId(0));
    assert!(e.state.layers.is_empty());
}

// "During each Methuselah's unlock phase, if this vampire is in torpor,
// that Methuselah burns 1 pool": every Methuselah, on their own unlock
// phase, while the host is in torpor; nothing once it is ready. The rider
// is ordered by the active Methuselah among their unlock effects and
// resolved before the impulse passes (LSJ 20010121; rules-qa 6.1).
#[test]
fn fame_burns_1_pool_from_each_methuselah_while_its_host_is_in_torpor() {
    let (mut e, _a, x, _z) = bleed_table();
    fame_on(&mut e, 0, x);
    e.state.minions[x.index()].in_torpor = true;
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(pool(&e), vec![10, 9, 10]);
    to_minion_phase(&mut e, SeatId(2));
    assert_eq!(pool(&e), vec![10, 9, 9]);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e), vec![9, 9, 9]);
    e.state.minions[x.index()].in_torpor = false;
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(2));
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e), vec![9, 9, 9]);
}

// "After this vampire goes to torpor, their controller burns 3 pool":
// inside the combat, from the host's controller (rules-qa 6.4).
#[test]
fn fame_burns_3_pool_from_the_host_controller_as_it_goes_to_torpor() {
    let (mut e, a, x, _z) = bleed_table();
    e.state.minions[x.index()].blood = 0;
    fame_on(&mut e, 0, x);
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert_eq!(pool(&e), vec![10, 7, 10]);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
}

// The famous vampire is the acting one and its Methuselah has 3 pool: the
// burn ousts them mid-combat — their predator scores at once (VP and 6
// pool), the combat and the action are torn down, and the turn passes to
// their prey with no after-combat window. On a host that left the game
// the unlock rider is inert (cards on a minion leaving play are not burned
// yet — DESIGN §11 slice 5d).
#[test]
fn fame_ousts_the_active_methuselah_mid_combat_and_the_turn_passes() {
    let (mut e, a, x, _z) = bleed_table();
    e.state.seats[0].pool = 3;
    let fame = fame_on(&mut e, 1, a);
    pick(&mut e, bleed(a));
    // Bled at 0 blood: the hand strike sends a to torpor.
    e.state.minions[a.index()].blood = 0;
    pick(&mut e, block(x));
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(1));
    assert!(e.state.seats[0].ousted);
    assert_eq!(pool(&e), vec![0, 10, 16]);
    let vps: Vec<u32> = e.state.seats.iter().map(|s| s.half_vps).collect();
    assert_eq!(vps, vec![0, 0, 2]);
    let _ = fame;
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(2));
    assert_eq!(pool(&e), vec![0, 10, 16]);
}

// The famous vampire is the blocker and its Methuselah has 3 pool: ousted
// mid-combat, their minion leaves the game with them, the combat ends at
// once (no end-of-round window for the survivor) and the blocked action
// ends; the acting Methuselah, their predator, scores. The ousted seat gets
// no decision.
#[test]
fn fame_ousts_the_blocking_methuselah_mid_combat_and_the_action_ends() {
    let (mut e, a, x, _z) = bleed_table();
    e.state.minions[x.index()].blood = 0;
    e.state.seats[1].pool = 3;
    fame_on(&mut e, 1, x);
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.seat != SeatId(1));
    assert!(e.state.seats[1].ousted);
    assert!(!e.state.minions[x.index()].in_play);
    assert_eq!(e.last_outcome, Some(Outcome::Blocked));
    assert_eq!(pool(&e), vec![16, 0, 10]);
    let vps: Vec<u32> = e.state.seats.iter().map(|s| s.half_vps).collect();
    assert_eq!(vps, vec![2, 0, 0]);
    assert_eq!(e.state.prey(SeatId(0)), SeatId(2));
}

// Two Methuselahs left: the mid-combat oust ends the game — the survivor
// scores the oust and the last-standing VP.
#[test]
fn fame_oust_mid_combat_can_end_the_game() {
    let mut e = Engine::bare(2);
    let a = e.add_minion(SeatId(0), vampire_with("DOM", 0), 6, false);
    e.add_minion(SeatId(0), vampire_with("CEL", 0), 6, false);
    let x = e.add_minion(SeatId(1), vampire_with("FOR", 0), 0, false);
    e.state.seats[0].pool = 10;
    e.state.seats[1].pool = 3;
    e.state.turn = 4;
    fame_on(&mut e, 0, x);
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    match e.run() {
        Step::Over(_) => {}
        Step::Decision(req) => panic!("expected the game over, got {req:?}"),
    }
    let vps: Vec<u32> = e.state.seats.iter().map(|s| s.half_vps).collect();
    assert_eq!(vps, vec![4, 0]);
    assert_eq!(e.state.game_win(), Some(SeatId(0)));
}

// Creeping Sabotage: "+1 stealth action" (blocks pruned with no intercept),
// "X is the number of copies you control" read when the cost is paid (the
// second copy costs 1), "Put this card in play" (no host, not discarded),
// and "during your unlock phase, your prey burns 1 pool" per copy, as the
// controller's mandatory riders — nothing on other Methuselahs' turns.
#[test]
fn creeping_sabotage_x_cost_enters_play_and_prey_burns_at_unlock() {
    let (mut e, _a, x, _z) = bleed_table();
    // The card requires a Nosferatu (clan icon).
    let a = e.add_minion(SeatId(0), vampire_of_clan("Nosferatu", 0), 6, false);
    hand(&mut e, 0, &["Creeping Sabotage", "Creeping Sabotage"]);
    pick(&mut e, card_play(0, 0, a));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    assert_eq!(blood(&e, a), 6);
    assert!(e.state.cards[0].in_play && e.state.cards[0].host.is_none());
    assert!(e.state.seats[0].ash.is_empty());
    // NRA: a acted with Creeping Sabotage (name identity) this turn.
    e.state.minions[a.index()].locked = false;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { actor: Some(m), .. } if *m == a)));
    e.state.minions[a.index()].acted.clear();
    pick(&mut e, card_play(0, 0, a));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(blood(&e, a), 5);
    assert_eq!(e.state.cards.len(), 2);
    // Seat 1's and seat 2's unlock phases: nothing; seat 0's: prey burns 2.
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(2));
    assert_eq!(pool(&e), vec![10, 10, 10]);
    let _ = x;
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e), vec![10, 8, 10]);
}

// "Minions can burn this card as a Ⓓ action": directed at the card's
// controller (they may block), NRA per copy; for the controller's own
// minion it is undirected (prey and predator may block). No stealth either
// way: the card prints none (rules-qa 6.2, 6.8).
#[test]
fn creeping_sabotage_burn_action_is_directed_at_its_controller() {
    let (mut e, a, x, _z) = bleed_table();
    let first = revolt_like_in_play(&mut e, "Creeping Sabotage", 0);
    let second = revolt_like_in_play(&mut e, "Creeping Sabotage", 0);
    // Seat 0's own vampire burns one: undirected; prey and predator may
    // attempt to block (stealth 0) and decline.
    pick(&mut e, granted(first, a));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(2), WindowLabel::ActionBlock);
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[first as usize].in_play);
    assert_eq!(
        e.state.seats[0].ash,
        vec![find_library("Creeping Sabotage")]
    );
    assert_eq!(
        e.state.minions[a.index()].acted,
        vec![ActionId::InPlay(CardId(first))]
    );
    // Seat 1's x burns the other: seat 0 may block with a (unlocked).
    e.state.minions[a.index()].locked = false;
    to_minion_phase(&mut e, SeatId(1));
    pick(&mut e, granted(second, x));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert_eq!(req.options[..2], [Choice::Pass, block(a)]);
    pick(&mut e, Choice::Pass);
    let _ = request(&mut e); // seat 1's phase elides (x locked): seat 2's
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    assert!(!e.state.cards[second as usize].in_play);
    assert_eq!(e.state.seats[0].ash.len(), 2);
}

fn revolt_like_in_play(e: &mut Engine, name: &str, owner: u8) -> u32 {
    e.state.cards.push(CardInPlay {
        card: find_library(name),
        owner: SeatId(owner),
        host: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
    });
    e.state.cards.len() as u32 - 1
}

// ---- Blood Doll, Vessel ---------------------------------------------------

/// Pass every window until `seat`'s master phase opens as a real decision.
fn to_master_phase(e: &mut Engine, seat: SeatId) -> DecisionRequest {
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, WindowLabel::MasterPhase) {
            return req;
        }
        assert!(
            matches!(req.options[0], Choice::Pass | Choice::ResolveRider { .. }),
            "{:?}",
            req
        );
        e.choose(0);
    }
}

fn ability(card: u32, ability: u8) -> Choice {
    Choice::UseAbility {
        card,
        ability,
        actor: None,
        target: Target::None,
    }
}

fn abilities(req: &DecisionRequest) -> Vec<Choice> {
    req.options
        .iter()
        .copied()
        .filter(|o| matches!(o, Choice::UseAbility { .. }))
        .collect()
}

// Blood Doll: on a vampire you control (others' vampires not offered);
// "during their master phase, this vampire's controller can move 1 blood"
// either way — a host ability costing no master phase action, once per
// master phase (the "during X" template, rules-qa 6.6), again next turn.
#[test]
fn blood_doll_moves_blood_either_way_once_per_master_phase() {
    let Table { mut e, alexa, .. } = table();
    hand(&mut e, 0, &["Blood Doll"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let targets: Vec<Target> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::Play { target, .. } => Some(*target),
            _ => None,
        })
        .collect();
    assert_eq!(targets, vec![Target::Minion(alexa)]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(alexa),
        },
    );
    // Same master phase, no action left: the Doll's two directions.
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(e.state.seats[0].mpa, 0);
    assert_eq!(abilities(&req), vec![ability(0, 0), ability(0, 1)]);
    pick(&mut e, ability(0, 0));
    assert_eq!((blood(&e, alexa), e.state.seats[0].pool), (4, 11));
    // Used this phase (either direction spends the copy): nothing left in
    // the master phase, which elides.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Seat 0's next master phase: pool to the vampire this time.
    let req = to_master_phase(&mut e, SeatId(0));
    assert_eq!(abilities(&req), vec![ability(0, 0), ability(0, 1)]);
    pick(&mut e, ability(0, 1));
    assert_eq!((blood(&e, alexa), e.state.seats[0].pool), (5, 10));
}

// The Doll works with its host in torpor (PIB 20150522).
#[test]
fn blood_doll_usable_with_the_host_in_torpor() {
    let Table { mut e, alexa, .. } = table();
    e.state.cards.push(CardInPlay {
        card: find_library("Blood Doll"),
        owner: SeatId(0),
        host: Some(alexa),
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
    });
    e.state.minions[alexa.index()].in_torpor = true;
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(abilities(&req), vec![ability(0, 0), ability(0, 1)]);
}

// Vessel: a trifle put on any vampire (another Methuselah's included);
// "you can burn a card named Blood Doll (if any)" — any Blood Doll in play
// (rules-qa 6.7), announced with the play as its terms (known in the
// as-played window), burned at resolution, elided when there is none; the
// ability is the host's controller's, at their own unlock phase, once per
// phase.
#[test]
fn vessel_on_any_vampire_burns_any_blood_doll_and_is_the_controllers_to_use() {
    let Table {
        mut e,
        alexa,
        kevin,
        anita,
        lenny,
    } = table();
    let doll = e.state.cards.len() as u32;
    e.state.cards.push(CardInPlay {
        card: find_library("Blood Doll"),
        owner: SeatId(2),
        host: Some(lenny),
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
    });
    hand(&mut e, 0, &["Vessel", "Vessel"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    let targets: Vec<Target> = req
        .options
        .iter()
        .filter_map(|o| match o {
            Choice::Play {
                card: 0, target, ..
            } => Some(*target),
            _ => None,
        })
        .collect();
    assert_eq!(
        targets,
        vec![
            Target::Minion(alexa),
            Target::Minion(kevin),
            Target::Minion(anita),
            Target::Minion(lenny)
        ]
    );
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(kevin),
        },
    );
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(
        req.options,
        vec![Choice::Pass, Choice::BurnInPlay { card: doll }]
    );
    pick(&mut e, Choice::BurnInPlay { card: doll });
    // Announced, not yet burned: the as-played window (elided here) comes
    // first; resolution burns it.
    let _ = request(&mut e);
    assert!(!e.state.cards[doll as usize].in_play);
    assert_eq!(e.state.seats[2].ash, vec![find_library("Blood Doll")]);
    // Trifle: a master phase action is left (the second Vessel is offered);
    // the Blood Doll is gone, so its burn choice elides this time.
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(e.state.seats[0].pool, 9);
    assert!(req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { card: 0, .. })));
    pick(&mut e, Choice::Pass);
    // Seat 1's unlock phase: Kevin's controller may shuttle blood; seat 0
    // (the card's owner) never gets the option.
    let req = to_unlock_abilities(&mut e, SeatId(1));
    let vessel = doll + 1;
    assert_eq!(
        abilities(&req),
        vec![ability(vessel, 0), ability(vessel, 1)]
    );
    pick(&mut e, ability(vessel, 0));
    assert_eq!((blood(&e, kevin), e.state.seats[1].pool), (4, 11));
    let req = request(&mut e);
    assert!(abilities(&req).is_empty(), "{req:?}");
}
