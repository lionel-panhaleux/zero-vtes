//! Slice 5 of the effects framework: a master card in play with no host,
//! a mandatory unlock-phase rider fired for every Methuselah at their own
//! unlock phase, and an action granted by a card in play (the stress set's
//! "ongoing global" card, Anarch Revolt). Windows with a single option are
//! elided, so the scripts only stop where a real decision exists.

use zvtes::actions::Outcome;
use zvtes::cards::AbilitySource;
use zvtes::cards::{Layer, Modifier, Scope, Target};
use zvtes::crypt::{Sect, CRYPT};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, ActionId, CardId, CardInPlay, UncontrolledSlot};

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
        source: AbilitySource::Card(CardId(card)),
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
        controller: SeatId(owner),
        host: None,
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
        effect_only: false,
        against: Default::default(),
        entered: 0,
        chosen: Vec::new(),
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
// Rulings: [LSJ 19980225].
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
// Rulings: [LSJ 20041130] [LSJ 20100527].
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
// Rulings: [LSJ 20100723].
#[test]
fn anarch_revolt_burn_is_ordered_by_the_active_methuselah_before_the_impulse_passes() {
    let Table { mut e, alexa, .. } = table();
    let first = revolt_in_play(&mut e, 0);
    let second = revolt_in_play(&mut e, 2);
    e.state.cards.push(CardInPlay {
        card: find_library("Homunculus"),
        owner: SeatId(0),
        controller: SeatId(0),
        host: Some(alexa),
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
        effect_only: false,
        against: Default::default(),
        entered: 0,
        chosen: Vec::new(),
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
        vec![ActionId::InPlay(CardId(first), 0)]
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

/// The `nth` V5 vampire with `disc` and no printed text implemented (a plain
/// fixture: printed abilities would add decisions to the script).
fn vampire_with(disc: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.disciplines.contains(&disc) && zvtes::cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no vampire with {disc}"))
}

fn vampire_of_clan(clan: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| c.clan == clan && zvtes::cards::vampire(*i as u16).is_none())
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
        controller: SeatId(owner),
        host: Some(host),
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
        effect_only: false,
        against: Default::default(),
        entered: 0,
        chosen: Vec::new(),
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
    e.add_minion(SeatId(0), find_card("Farah Sarroub"), 6, false);
    let x = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 4, false);
    let z = e.add_minion(SeatId(2), find_card("Crystal Lynn"), 4, false);
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
                source: AbilitySource::Card(CardId(angel)),
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
                source: AbilitySource::Card(CardId(first)),
                ability: 0,
                actor: None,
                target: Target::None
            },
            Choice::UseAbility {
                source: AbilitySource::Card(CardId(second)),
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
                source: AbilitySource::Card(CardId(second)),
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
        controller: SeatId(owner),
        host: Some(host),
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
        effect_only: false,
        against: Default::default(),
        entered: 0,
        chosen: Vec::new(),
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
// Rulings: [LSJ 20010121].
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
// their prey with no after-combat window. The oust removes what they
// controlled (spec §6.1.2): the famous vampire and their own Tension in
// the Ranks — removed, not burned (no ash heap, no rider); the other
// Methuselah's Fame on the removed vampire is burned to its owner's ash
// heap (owner, 2026-09-06).
#[test]
fn fame_ousts_the_active_methuselah_mid_combat_and_the_turn_passes() {
    let (mut e, a, x, _z) = bleed_table();
    e.state.seats[0].pool = 3;
    let fame = fame_on(&mut e, 1, a);
    let tension = revolt_like_in_play(&mut e, "Tension in the Ranks", 0);
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
    assert!(!e.state.cards[fame as usize].in_play);
    assert!(!e.state.cards[tension as usize].in_play);
    assert_eq!(e.state.seats[1].ash, vec![find_library("Fame")]);
    assert!(e.state.seats[0].ash.is_empty());
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
    e.add_minion(SeatId(0), find_card("Farah Sarroub"), 6, false);
    let x = e.add_minion(SeatId(1), find_card("Monica Giovanni"), 0, false);
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

// Tension in the Ranks: "After a ready minion is burned or sent to torpor,
// their controller burns 1 pool" — the blocker's torpor costs its
// Methuselah 1 pool inside the combat, whoever controls the card; with Fame
// on the same vampire both riders land at once (4 pool).
#[test]
fn tension_in_the_ranks_burns_1_pool_from_the_controller_of_a_minion_sent_to_torpor() {
    let (mut e, a, x, _z) = bleed_table();
    revolt_like_in_play(&mut e, "Tension in the Ranks", 2);
    e.state.minions[x.index()].blood = 0;
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[x.index()].in_torpor);
    assert_eq!(pool(&e), vec![10, 9, 10]);

    let (mut e, a, x, _z) = bleed_table();
    revolt_like_in_play(&mut e, "Tension in the Ranks", 2);
    fame_on(&mut e, 0, x);
    e.state.minions[x.index()].blood = 0;
    pick(&mut e, bleed(a));
    pick(&mut e, block(x));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(pool(&e), vec![10, 6, 10]);
}

// "A ready minion is burned": a diablerie's victim is burned from torpor —
// no pool (and the cards on it are burned, spec §1.12.1); the diablerist
// burned by the blood hunt was ready — its controller burns 1 pool.
#[test]
fn tension_in_the_ranks_burned_ready_minion_but_not_a_torpor_victim() {
    let Table {
        mut e,
        alexa,
        kevin,
        lenny,
        ..
    } = table();
    revolt_like_in_play(&mut e, "Tension in the Ranks", 2);
    let victim = e.add_minion(SeatId(1), find_card("Abraham DuSable"), 2, true);
    let angel = guardian_angel_on(&mut e, 1, victim);
    to_minion_phase(&mut e, SeatId(0));
    pick(
        &mut e,
        Choice::TakeAction {
            minion: alexa,
            action: ActionKind::Diablerie,
            target: Some(victim),
        },
    );
    pick(&mut e, Choice::Pass); // seat 1 declines to block
    let req = expect(&mut e, SeatId(0), WindowLabel::BloodHuntPolling);
    assert!(!e.state.minions[victim.index()].in_play);
    assert!(!e.state.cards[angel as usize].in_play);
    assert_eq!(e.state.seats[1].ash, vec![find_library("Guardian Angel")]);
    assert_eq!(pool(&e), vec![10, 10, 10]);
    let _ = req;
    pick(&mut e, Choice::Pass);
    pick(&mut e, votes(kevin, true));
    pick(&mut e, votes(lenny, true));
    // The rotation closes on consecutive passes.
    while request(&mut e).window == WindowLabel::BloodHuntPolling {
        pick(&mut e, Choice::Pass);
    }
    // Seat 0 has no minion left: the rest of its turn elides.
    let _ = request(&mut e);
    assert!(!e.state.minions[alexa.index()].in_play);
    assert_eq!(pool(&e), vec![9, 10, 10]);
}

// "Methuselahs can use a master phase action and discard two master cards
// to burn this card": any Methuselah in their own master phase, with a
// master phase action left and two master cards in hand; the two discards
// are announced with the use (one at a time), the card goes to its owner's
// ash heap.
#[test]
fn tension_in_the_ranks_burned_by_a_master_phase_action_and_two_master_discards() {
    let Table { mut e, .. } = table();
    let t = revolt_like_in_play(&mut e, "Tension in the Ranks", 1);
    hand(&mut e, 0, &["Bonding", "Villein"]);
    // One master card in hand: not offered.
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::UseAbility { .. })));
    e.state.seats[0].hand.push(find_library("Blood Doll"));
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(req.options.contains(&ability(t, 0)));
    pick(&mut e, ability(t, 0));
    let req = expect(&mut e, SeatId(0), WindowLabel::PlayTerms);
    assert_eq!(req.options, vec![Choice::Discard(1), Choice::Discard(2)]);
    pick(&mut e, Choice::Discard(2));
    // The second pick has a single candidate left: it elides.
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].hand, vec![find_library("Bonding")]);
    assert_eq!(
        e.state.seats[0].ash,
        vec![find_library("Blood Doll"), find_library("Villein")]
    );
    assert_eq!(e.state.seats[0].mpa, 0);
    assert!(!e.state.cards[t as usize].in_play);
    assert_eq!(
        e.state.seats[1].ash,
        vec![find_library("Tension in the Ranks")]
    );
}

// No master phase action left: the burn is not offered — nothing is, so
// the master phase elides.
#[test]
fn tension_in_the_ranks_burn_needs_a_master_phase_action() {
    let Table { mut e, .. } = table();
    revolt_like_in_play(&mut e, "Tension in the Ranks", 1);
    hand(&mut e, 0, &["Villein", "Blood Doll", "Bonding"]);
    e.state.seats[0].mpa = 0;
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
}

/// Pass every window until `seat` faces `window` (a mandatory rider's
/// payment window, say); single-option windows elide on the way.
fn to_window(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, window) {
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

// Smiling Jack: "During your unlock phase, move 1 counter from your pool
// to this card" — the controller's mandatory rider, one pool a turn onto
// the card; the last pool moved ousts them on their own unlock phase and
// the turn passes.
#[test]
fn smiling_jack_moves_1_pool_to_a_counter_each_own_unlock_phase_and_can_oust() {
    let (mut e, _a, _x, _z) = bleed_table();
    let jack = revolt_like_in_play(&mut e, "Smiling Jack, The Anarch", 0);
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(2));
    assert_eq!(pool(&e), vec![10, 10, 10]);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e), vec![9, 10, 10]);
    assert_eq!(e.state.cards[jack as usize].counters, 1);
    e.state.seats[0].pool = 1;
    // No counters were paid by the others so far: the first counter landed
    // on seat 0's turn; seats 1 and 2 now pay 1 each on theirs (a vampire's
    // blood or a pool: they have vampires with blood, so a window opens).
    let req = to_window(&mut e, SeatId(1), WindowLabel::RiderTerms);
    assert_eq!(req.options[0], Choice::BurnPool);
    pick(&mut e, Choice::BurnPool);
    let _ = to_window(&mut e, SeatId(2), WindowLabel::RiderTerms);
    pick(&mut e, Choice::BurnPool);
    // Seat 0's unlock: the pool goes onto the card and seat 0 is ousted.
    to_minion_phase(&mut e, SeatId(1));
    assert!(e.state.seats[0].ousted);
    assert_eq!(pool(&e), vec![0, 9, 15]);
    assert!(!e.state.cards[jack as usize].in_play);
}

// "During each other Methuselah's unlock phase, for each counter on this
// card, that Methuselah burns 1 pool or burns 1 blood from a vampire they
// control": each unit is chosen as it is paid, any mix (ANK 20181006); a
// vampire with no blood is no source (LSJ 19971212), so with only empty
// vampires the pool is the only way and the window elides.
// Rulings: [LSJ 19971212]; [ANK 20181006].
#[test]
fn smiling_jack_each_other_methuselah_pays_pool_or_blood_per_counter() {
    let (mut e, a, x, z) = bleed_table();
    let jack = revolt_like_in_play(&mut e, "Smiling Jack, The Anarch", 0);
    e.state.cards[jack as usize].counters = 2;
    e.state.minions[z.index()].blood = 0;
    let req = to_window(&mut e, SeatId(1), WindowLabel::RiderTerms);
    assert_eq!(
        req.options,
        vec![Choice::BurnPool, Choice::BurnBloodFrom { minion: x }]
    );
    pick(&mut e, Choice::BurnPool);
    let req = expect(&mut e, SeatId(1), WindowLabel::RiderTerms);
    assert_eq!(
        req.options,
        vec![Choice::BurnPool, Choice::BurnBloodFrom { minion: x }]
    );
    pick(&mut e, Choice::BurnBloodFrom { minion: x });
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(pool(&e), vec![10, 9, 10]);
    assert_eq!(blood(&e, x), 3);
    // Seat 2: z is empty, both units come from the pool with no decision
    // (and z's forced hunt elides its minion phase). Seat 0 then pays
    // nothing but its own counter.
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e), vec![9, 9, 8]);
    assert_eq!(e.state.cards[jack as usize].counters, 3);
    let _ = a;
}

// A paying Methuselah at 1 pool who pays the first unit from the pool is
// ousted on the spot: the remaining units lapse, the turn passes, their
// predator scores.
#[test]
fn smiling_jack_payment_from_the_last_pool_ousts_the_payer() {
    let (mut e, _a, x, _z) = bleed_table();
    let jack = revolt_like_in_play(&mut e, "Smiling Jack, The Anarch", 0);
    e.state.cards[jack as usize].counters = 2;
    e.state.seats[1].pool = 1;
    let req = to_window(&mut e, SeatId(1), WindowLabel::RiderTerms);
    assert_eq!(
        req.options,
        vec![Choice::BurnPool, Choice::BurnBloodFrom { minion: x }]
    );
    pick(&mut e, Choice::BurnPool);
    let req = request(&mut e);
    assert_eq!(req.seat, SeatId(2));
    assert!(e.state.seats[1].ousted);
    assert_eq!(pool(&e), vec![16, 0, 10]);
    assert_eq!(e.state.seats[0].half_vps, 2);
}

// "Vampires can burn this card as a Ⓓ action": directed at its controller,
// who may block; on success the card is burned to its owner's ash heap.
#[test]
fn smiling_jack_can_be_burned_as_a_directed_action() {
    let (mut e, a, x, _z) = bleed_table();
    let jack = revolt_like_in_play(&mut e, "Smiling Jack, The Anarch", 1);
    pick(&mut e, granted(jack, a));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(req.options[..2], [Choice::Pass, block(x)]);
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    assert!(!e.state.cards[jack as usize].in_play);
    assert_eq!(
        e.state.seats[1].ash,
        vec![find_library("Smiling Jack, The Anarch")]
    );
}

fn uses(req: &DecisionRequest, card: u32) -> Vec<Choice> {
    req.options
        .iter()
        .filter(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(CardId(c)), .. } if *c == card))
        .copied()
        .collect()
}

// Dreams of the Sphinx: "You can lock this card to get +2 hand size until
// the end of the turn" — the hand is drawn up to 9 at once, the card locks
// and gains a counter, and stays locked until its controller's next unlock
// phase; the bonus ends with the turn (LSJ 20100812), and seat 0 discards
// down to 7 at once (rulebook "Drawing Cards": "whenever an effect changes
// your hand size …, immediately discard down").
#[test]
fn dreams_of_the_sphinx_hand_size_lock_lasts_the_turn() {
    let Table { mut e, .. } = table();
    let dreams = revolt_like_in_play(&mut e, "Dreams of the Sphinx", 0);
    hand(&mut e, 0, &["Bonding"]);
    e.state.seats[0].library = vec![find_library("Villein"); 10];
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert_eq!(uses(&req, dreams), vec![ability(dreams, 0)]);
    pick(&mut e, ability(dreams, 0));
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(uses(&req, dreams).is_empty()); // locked
    assert_eq!(e.state.seats[0].hand.len(), 9);
    assert_eq!(e.state.seats[0].library.len(), 2);
    assert!(e.state.cards[dreams as usize].locked);
    assert_eq!(e.state.cards[dreams as usize].counters, 1);
    let req = loop {
        let req = request(&mut e);
        if req.window == WindowLabel::DiscardDown {
            break req;
        }
        assert_eq!(req.options[0], Choice::Pass, "{req:?}");
        e.choose(0);
    };
    assert_eq!(req.seat, SeatId(0));
    assert_eq!(e.state.seats[0].hand_bonus, 0);
    assert_eq!(e.state.active, SeatId(1));
    e.choose(0);
    expect(&mut e, SeatId(0), WindowLabel::DiscardDown);
    e.choose(0);
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(e.state.seats[0].hand.len(), 7);
    // Seat 0's next turn: the card unlocked, the hand full.
    let req = to_master_phase(&mut e, SeatId(0));
    assert!(!e.state.cards[dreams as usize].locked);
    assert_eq!(uses(&req, dreams), vec![ability(dreams, 0)]);
    assert_eq!(e.state.seats[0].hand.len(), 7);
    assert_eq!(e.state.seats[0].library.len(), 2);
}

// "You can lock this card during your unlock phase to gain 1 pool if you
// have the Edge": an own-unlock-phase ability gated on the Edge, offered
// among the unlock-phase effects; the untimed hand-size lock is offered
// there too.
#[test]
fn dreams_of_the_sphinx_edge_lock_in_own_unlock_phase() {
    let (mut e, _a, _x, _z) = bleed_table();
    let dreams = revolt_like_in_play(&mut e, "Dreams of the Sphinx", 0);
    e.state.seats[0].has_edge = true;
    // In the other Methuselahs' unlock rotations seat 0 is offered the
    // untimed lock only; on its own unlock phase the Edge lock too.
    let req = loop {
        let req = to_unlock_abilities(&mut e, SeatId(0));
        if e.state.active == SeatId(0) {
            break req;
        }
        assert_eq!(uses(&req, dreams), vec![ability(dreams, 0)]);
        e.choose(0);
    };
    assert_eq!(
        uses(&req, dreams),
        vec![ability(dreams, 1), ability(dreams, 0)]
    );
    pick(&mut e, ability(dreams, 1));
    assert_eq!(pool(&e), vec![11, 10, 10]);
    assert_eq!(e.state.cards[dreams as usize].counters, 1);
    assert!(e.state.cards[dreams as usize].locked);
    // The Edge, kept for the lock, is burned afterward in the same unlock
    // phase: +2 pool in all.
    let req = expect(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&Choice::BurnEdge));
    pick(&mut e, Choice::BurnEdge);
    assert_eq!(pool(&e), vec![12, 10, 10]);
    assert!(!e.state.seats[0].has_edge);
}

// "You can lock this card to add 1 blood to a vampire in your uncontrolled
// region" and "burn this card if it has 3 counters": the third lock's
// effect lands, then the card burns.
#[test]
fn dreams_of_the_sphinx_blood_to_uncontrolled_and_burn_at_three_counters() {
    let Table { mut e, .. } = table();
    let dreams = revolt_like_in_play(&mut e, "Dreams of the Sphinx", 0);
    e.state.cards[dreams as usize].counters = 2;
    e.state.seats[0].uncontrolled.push(UncontrolledSlot {
        card: find_card("Abraham DuSable"),
        counters: 0,
        banished: None,
    });
    let pick_slot = Choice::UseAbility {
        source: AbilitySource::Card(CardId(dreams)),
        ability: 2,
        actor: None,
        target: Target::Uncontrolled(0),
    };
    let req = expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    assert!(req.options.contains(&pick_slot));
    pick(&mut e, pick_slot);
    assert_eq!(e.state.seats[0].uncontrolled[0].counters, 1);
    assert!(!e.state.cards[dreams as usize].in_play);
    assert_eq!(
        e.state.seats[0].ash,
        vec![find_library("Dreams of the Sphinx")]
    );
}

// The Barrens: "You can lock this card to discard a card (draw up
// afterward)" — usable whenever its controller holds the impulse, another
// Methuselah's action included (ANK 20200616); the card to discard is
// announced with the use; locked, it is no longer offered.
// Rulings: [ANK 20200616].
#[test]
fn the_barrens_discards_a_chosen_card_at_any_impulse() {
    let (mut e, a, _x, _z) = bleed_table();
    let barrens = revolt_like_in_play(&mut e, "Barrens, The", 1);
    hand(&mut e, 1, &["Villein", "Bonding"]);
    e.state.seats[1].library = vec![find_library("Villein"); 3];
    pick(&mut e, bleed(a));
    // Offered as the action is announced already (a window that elides
    // without it).
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionAsAnnounced);
    assert_eq!(uses(&req, barrens), vec![ability(barrens, 0)]);
    pick(&mut e, ability(barrens, 0));
    let req = expect(&mut e, SeatId(1), WindowLabel::PlayTerms);
    assert_eq!(req.options, vec![Choice::Discard(0), Choice::Discard(1)]);
    pick(&mut e, Choice::Discard(1));
    assert_eq!(e.state.seats[1].ash, vec![find_library("Bonding")]);
    assert_eq!(e.state.seats[1].hand.len(), 4);
    assert!(e.state.cards[barrens as usize].locked);
    let req = to_window(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(uses(&req, barrens).is_empty());
}

// A starving vampire's forced hunt does not pre-empt an untimed ability:
// the controller may use it first (owner, 2026-09-06).
#[test]
fn untimed_ability_offered_alongside_a_forced_hunt() {
    let (mut e, a, _x, _z) = bleed_table();
    let barrens = revolt_like_in_play(&mut e, "Barrens, The", 0);
    hand(&mut e, 0, &["Bonding"]);
    for m in &mut e.state.minions {
        if m.owner == SeatId(0) {
            m.blood = 0;
        }
    }
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&Choice::Pass));
    assert!(req.options.iter().any(|o| matches!(
        o,
        Choice::TakeAction {
            minion,
            action: ActionKind::Hunt,
            ..
        } if *minion == a
    )));
    assert_eq!(uses(&req, barrens), vec![ability(barrens, 0)]);
}

// rules-qa 7.9: the active Methuselah's phases end on an impulse rotation
// over the other seats — seat 1 uses The Barrens as seat 0 passes out of
// its master phase, and the impulse returns to seat 0 in that phase.
#[test]
fn the_barrens_usable_in_the_rotation_closing_another_methuselahs_phase() {
    let Table { mut e, .. } = table();
    let barrens = revolt_like_in_play(&mut e, "Barrens, The", 1);
    hand(&mut e, 0, &["Villein"]);
    hand(&mut e, 1, &["Bonding", "Villein"]);
    pick(&mut e, Choice::Pass); // seat 0 passes its master phase
    let req = expect(&mut e, SeatId(1), WindowLabel::PhaseImpulse);
    assert_eq!(uses(&req, barrens), vec![ability(barrens, 0)]);
    pick(&mut e, ability(barrens, 0));
    pick(&mut e, Choice::Discard(0));
    // Back to seat 0's master phase (still a real decision: its master
    // phase action is unspent), which it passes again; nothing left for
    // the others (The Barrens is locked): the phase ends.
    expect(&mut e, SeatId(0), WindowLabel::MasterPhase);
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].ash, vec![find_library("Bonding")]);
}

// Not in the as-played window: only cancels and wakes there (rules-qa
// 2.4; LSJ 20061207).
// Rulings: [RTR 20040501]; [LSJ 20061207] [LSJ 20061013].
#[test]
fn the_barrens_is_not_usable_in_the_as_played_window() {
    let Table { mut e, .. } = table();
    let barrens = revolt_like_in_play(&mut e, "Barrens, The", 1);
    hand(&mut e, 0, &["Villein"]);
    hand(&mut e, 1, &["Sudden Reversal", "Bonding"]);
    pick(
        &mut e,
        Choice::Play {
            card: 0,
            mode: 0,
            actor: None,
            target: Target::Minion(MinionId(0)),
        },
    );
    pick(&mut e, Choice::SetX(2));
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert!(uses(&req, barrens).is_empty());
    assert!(req.options.len() > 1);
}

fn coven_use(card: u32, target: MinionId) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(CardId(card)),
        ability: 0,
        actor: None,
        target: Target::Minion(target),
    }
}

// The Coven: "You can lock this card to add 2 blood to a ready vampire you
// control" — the controller's own ready vampires only (not torpid ones, not
// other Methuselahs'), capped at capacity; usable before a starving
// vampire's forced hunt, which then is no longer forced (owner, 2026-09-06).
#[test]
fn the_coven_feeds_a_ready_vampire_you_control_before_the_forced_hunt() {
    let (mut e, a, x, _z) = bleed_table();
    let coven = revolt_like_in_play(&mut e, "Coven, The", 0);
    let cel = MinionId(1);
    e.state.minions[a.index()].blood = 0;
    e.state.minions[cel.index()].in_torpor = true;
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&Choice::Pass));
    assert_eq!(uses(&req, coven), vec![coven_use(coven, a)]);
    let _ = x;
    pick(&mut e, coven_use(coven, a));
    assert_eq!(blood(&e, a), 2);
    assert!(e.state.cards[coven as usize].locked);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&Choice::Pass));
    assert!(req.options.contains(&bleed(a)));
    assert!(uses(&req, coven).is_empty());
}

// "During your discard phase, your predator takes control of The Coven":
// fired as its controller passes out of their discard phase (LSJ
// 20100723); the new controller unlocks it on their unlock phase, uses it,
// and passes it on to their own predator in turn. Ownership never moves.
// Rulings: [LSJ 20100723] [PIB 20111002].
#[test]
fn the_coven_goes_to_the_predator_at_the_end_of_each_controllers_discard_phase() {
    let Table {
        mut e,
        alexa,
        lenny,
        ..
    } = table();
    let coven = revolt_like_in_play(&mut e, "Coven, The", 0);
    hand(&mut e, 0, &["Bonding"]);
    hand(&mut e, 2, &["Bonding"]);
    pick(&mut e, coven_use(coven, alexa));
    let cap = e.state.capacity_of(alexa).unwrap();
    assert_eq!(blood(&e, alexa), 7.min(cap));
    let req = to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(uses(&req, coven).is_empty()); // locked
    assert_eq!(e.state.cards[coven as usize].controller, SeatId(0));
    pick(&mut e, Choice::Pass);
    // The predator takes it as they get the impulse in the rotation
    // closing the phase (rules-qa 7.9) — a mandatory choice that elides.
    let _ = request(&mut e);
    assert_eq!(e.state.cards[coven as usize].controller, SeatId(2));
    assert_eq!(e.state.cards[coven as usize].owner, SeatId(0));
    // Seat 2, its new controller: unlocked on its unlock phase, usable on
    // its own vampire only.
    let req = to_master_phase(&mut e, SeatId(2));
    assert!(!e.state.cards[coven as usize].locked);
    assert_eq!(uses(&req, coven), vec![coven_use(coven, lenny)]);
    let req = to_window(&mut e, SeatId(2), WindowLabel::DiscardPhase);
    assert!(!uses(&req, coven).is_empty());
    pick(&mut e, Choice::Pass);
    let _ = request(&mut e);
    assert_eq!(e.state.cards[coven as usize].controller, SeatId(1));
}

// The predator already controls a Coven: the incoming copy would contest
// involuntarily, so it is burned instead (spec §1.13.1), to its owner's ash.
#[test]
fn a_coven_moving_onto_a_second_copy_is_burned() {
    let Table { mut e, .. } = table();
    let mine = revolt_like_in_play(&mut e, "Coven, The", 0);
    let theirs = revolt_like_in_play(&mut e, "Coven, The", 2);
    hand(&mut e, 0, &["Bonding"]);
    to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, Choice::Pass);
    let _ = request(&mut e);
    assert!(!e.state.cards[mine as usize].in_play);
    assert_eq!(e.state.seats[0].ash, vec![find_library("Coven, The")]);
    assert!(e.state.cards[theirs as usize].present());
    assert_eq!(e.state.cards[theirs as usize].controller, SeatId(2));
}

// A contested Coven is out of play: its discard-phase rider does not fire.
#[test]
fn a_contested_coven_does_not_move() {
    let Table { mut e, .. } = table();
    let coven = revolt_like_in_play(&mut e, "Coven, The", 0);
    e.state.cards[coven as usize].contested = true;
    hand(&mut e, 0, &["Bonding"]);
    to_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, Choice::Pass);
    let _ = request(&mut e);
    assert_eq!(e.state.cards[coven as usize].controller, SeatId(0));
}

// Control, not ownership, decides an oust (spec §6.1.2): the owner ousted,
// The Coven their predator controls stays in play.
#[test]
fn the_coven_taken_survives_its_owners_oust() {
    let (mut e, _a, _x, _z) = bleed_table();
    let coven = revolt_like_in_play(&mut e, "Coven, The", 0);
    e.state.cards[coven as usize].controller = SeatId(2);
    let jack = revolt_like_in_play(&mut e, "Smiling Jack, The Anarch", 0);
    e.state.seats[0].pool = 1;
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(2));
    to_minion_phase(&mut e, SeatId(1));
    assert!(e.state.seats[0].ousted);
    assert!(!e.state.cards[jack as usize].in_play);
    assert!(e.state.cards[coven as usize].in_play);
    // Seat 2 passed it on at its own discard phase meanwhile.
    assert_eq!(e.state.card_controller(CardId(coven)), SeatId(1));
    assert_eq!(e.state.cards[coven as usize].owner, SeatId(0));
}

fn vampire_of_sect(sect: Sect, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| c.sect == sect && zvtes::cards::vampire(*i as u16).is_none())
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no {sect:?} vampire"))
}

// Constant Revolution: "+1 stealth action. Requires an Anarch. Unique. Put
// this card in play with 1 counter. During your unlock phase, add 1
// counter to this card." — offered to an Anarch only, enters with a
// counter (blocks pruned by the stealth), one more each own unlock phase.
#[test]
fn constant_revolution_needs_an_anarch_and_counts_up() {
    let (mut e, a, _x, _z) = bleed_table();
    let anarch = e.add_minion(SeatId(0), vampire_of_sect(Sect::Anarch, 0), 6, false);
    hand(&mut e, 0, &["Constant Revolution"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!req.options.contains(&card_play(0, 0, a)));
    assert!(req.options.contains(&card_play(0, 0, anarch)));
    pick(&mut e, card_play(0, 0, anarch));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    let cr = e.state.cards.len() as u32 - 1;
    assert!(e.state.cards[cr as usize].present());
    assert_eq!(e.state.cards[cr as usize].counters, 1);
    to_minion_phase(&mut e, SeatId(1));
    to_minion_phase(&mut e, SeatId(2));
    assert_eq!(e.state.cards[cr as usize].counters, 1);
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(e.state.cards[cr as usize].counters, 2);
}

// "During each other Methuselah's unlock phase, that Methuselah burns X
// pool and/or cards at random from their hand": the count of cards is
// announced (at most the hand, LSJ 20090618), those are burned at random
// at once, the rest is pool; an empty hand leaves only the pool (elided).
// Rulings: [LSJ 20090618].
#[test]
fn constant_revolution_others_burn_pool_or_random_cards() {
    let (mut e, _a, _x, _z) = bleed_table();
    let cr = revolt_like_in_play(&mut e, "Constant Revolution", 0);
    e.state.cards[cr as usize].counters = 2;
    hand(&mut e, 1, &["Villein", "Bonding", "Flash"]);
    let req = to_window(&mut e, SeatId(1), WindowLabel::RiderTerms);
    assert_eq!(
        req.options,
        vec![Choice::SetX(0), Choice::SetX(1), Choice::SetX(2)]
    );
    pick(&mut e, Choice::SetX(1));
    to_minion_phase(&mut e, SeatId(1));
    assert_eq!(pool(&e), vec![10, 9, 10]);
    assert_eq!(e.state.seats[1].hand.len(), 2);
    assert_eq!(e.state.seats[1].ash.len(), 1);
    // Seat 2, no hand: 2 pool with no decision; seat 0 pays nothing.
    to_minion_phase(&mut e, SeatId(0));
    assert_eq!(pool(&e), vec![10, 9, 8]);
    assert_eq!(e.state.cards[cr as usize].counters, 3);
}

// "Vampires can burn this card as a Ⓓ action that costs 1 pool": directed
// at its controller, the pool paid at resolution.
#[test]
fn constant_revolution_burn_action_costs_1_pool() {
    let (mut e, a, x, _z) = bleed_table();
    let cr = revolt_like_in_play(&mut e, "Constant Revolution", 1);
    pick(&mut e, granted(cr, a));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(req.options[..2], [Choice::Pass, block(x)]);
    pick(&mut e, Choice::Pass);
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    assert_eq!(pool(&e), vec![9, 10, 10]);
    assert!(!e.state.cards[cr as usize].in_play);
    assert_eq!(
        e.state.seats[1].ash,
        vec![find_library("Constant Revolution")]
    );
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
    // 6 blood, capped at its capacity.
    let start = e.state.capacity_of(a).unwrap().min(6);
    hand(&mut e, 0, &["Creeping Sabotage", "Creeping Sabotage"]);
    pick(&mut e, card_play(0, 0, a));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(Outcome::Successful));
    assert_eq!(blood(&e, a), start);
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
    assert_eq!(blood(&e, a), start - 1);
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
        vec![ActionId::InPlay(CardId(first), 0)]
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
        controller: SeatId(owner),
        host: None,
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
        effect_only: false,
        against: Default::default(),
        entered: 0,
        chosen: Vec::new(),
    });
    e.state.cards.len() as u32 - 1
}

// ---- Replacement and hand size --------------------------------------------

/// Pass (or take the first option) until `seat`'s `window` opens on
/// `seat`'s own turn.
fn to_own_window(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, window) && e.state.active == seat {
            return req;
        }
        let i = req
            .options
            .iter()
            .position(|o| *o == Choice::Pass)
            .unwrap_or(0);
        e.choose(i);
    }
}

// Elder Library: "Unique location. +1 hand size." Played from a full hand,
// its replacement draws the hand up to 8.
#[test]
fn elder_library_gives_its_controller_one_more_hand_size() {
    let Table { mut e, .. } = table();
    hand(&mut e, 0, &["Elder Library"]);
    hand(&mut e, 0, &["Bonding"; 6]);
    e.state.seats[0].library = vec![find_library("Villein"); 3];
    pick(&mut e, master_play(0));
    let _ = request(&mut e);
    assert_eq!(e.state.hand_size(SeatId(0)), 8);
    assert_eq!(e.state.seats[0].hand.len(), 8);
    assert_eq!(e.state.seats[0].library.len(), 1);
    assert_eq!(e.state.hand_size(SeatId(1)), 7);
}

// Visit from the Capuchin: "Put this card in play with 4 counters. You get
// +1 hand size for each counter on this card. Each time you would replace
// a card other than this card, instead burn 1 counter from this card. Burn
// this card if it has no counters." Its own replacement draws the hand up
// to 11; a discard phase action's replacement burns a counter instead; a
// delayed replacement coming due at the unlock phase burns the last one,
// and the card with it.
#[test]
fn visit_from_the_capuchin_burns_a_counter_instead_of_each_replacement() {
    let Table { mut e, .. } = table();
    hand(&mut e, 0, &["Visit from the Capuchin"]);
    hand(&mut e, 0, &["Bonding"; 6]);
    e.state.seats[0].library = vec![find_library("Villein"); 10];
    pick(&mut e, master_play(0));
    let _ = request(&mut e);
    assert_eq!(e.state.cards[0].counters, 4);
    assert_eq!(e.state.hand_size(SeatId(0)), 11);
    assert_eq!(e.state.seats[0].hand.len(), 11);
    assert_eq!(e.state.seats[0].library.len(), 5);
    to_own_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, Choice::Discard(0));
    assert_eq!(e.state.cards[0].counters, 3);
    assert_eq!(e.state.seats[0].hand.len(), 10);
    assert_eq!(e.state.seats[0].library.len(), 5);
    // One counter left (hand size 8), one card held back until the unlock
    // phase: its replacement burns the last counter and the card.
    e.state.cards[0].counters = 1;
    e.state.seats[0].hand.truncate(7);
    e.state.seats[0].held.unlock = 1;
    to_own_window(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[0].in_play);
    assert!(e.state.seats[0]
        .ash
        .contains(&find_library("Visit from the Capuchin")));
    assert_eq!(e.state.hand_size(SeatId(0)), 7);
    assert_eq!(e.state.seats[0].hand.len(), 7);
    assert_eq!(e.state.seats[0].library.len(), 5);
}

// Rules-qa 8.3: with an empty library a card is not replaced, so Visit from
// the Capuchin has no replacement to replace — no counter burns, and the
// hand stays short.
#[test]
fn visit_from_the_capuchin_burns_nothing_with_an_empty_library() {
    let Table { mut e, .. } = table();
    let capuchin = revolt_like_in_play(&mut e, "Visit from the Capuchin", 0);
    e.state.cards[capuchin as usize].counters = 2;
    e.state.layers.push(Layer {
        scope: Scope::InPlay,
        modifier: Modifier::HandSizePerCounter,
        source: CardId(capuchin),
    });
    hand(&mut e, 0, &["Bonding"; 9]);
    to_own_window(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    pick(&mut e, Choice::Discard(0));
    assert_eq!(e.state.cards[capuchin as usize].counters, 2);
    assert_eq!(e.state.hand_size(SeatId(0)), 9);
    assert_eq!(e.state.seats[0].hand.len(), 8);
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
        source: AbilitySource::Card(CardId(card)),
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
// Rulings: [ANK 20210309-3].
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
// Rulings: [PIB 20150522].
#[test]
fn blood_doll_usable_with_the_host_in_torpor() {
    let Table { mut e, alexa, .. } = table();
    e.state.cards.push(CardInPlay {
        card: find_library("Blood Doll"),
        owner: SeatId(0),
        controller: SeatId(0),
        host: Some(alexa),
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
        effect_only: false,
        against: Default::default(),
        entered: 0,
        chosen: Vec::new(),
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
// Rulings: [ANK 20210309-3].
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
        controller: SeatId(2),
        host: Some(lenny),
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
        effect_only: false,
        against: Default::default(),
        entered: 0,
        chosen: Vec::new(),
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
