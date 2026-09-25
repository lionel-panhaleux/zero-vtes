//! Slice 10: allies — a minion in its own right (rulebook "Ally cards"):
//! recruited as a +1 stealth undirected action paid at resolution, unable
//! to act the turn it is recruited; its printed life, strength and bleed;
//! life, not blood (advanced rules 5.5): no hunt, no blood cost, no torpor
//! — damage burns life and the last one burns the ally. Windows with a
//! single option are elided, so the scripts only stop where a real
//! decision exists.

use zvtes::cards::{self, AbilitySource, Effect, Layer, Scope, Stat, Target};
use zvtes::crypt::{Title, CRYPT};
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::{Engine, Step};
use zvtes::ids::{MinionId, SeatId};
use zvtes::state::{find_card, find_library, CardId, CardInPlay, PrintedCtx};

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
        .unwrap_or_else(|| {
            panic!(
                "option {want:?} not in {:?} ({:?})",
                req.options, req.window
            )
        });
    e.choose(idx);
}

/// Pass (or hand strike) through every decision until `seat`'s `window`.
fn until(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
    loop {
        let req = request(e);
        if (req.seat, req.window) == (seat, window) {
            return req;
        }
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
            .unwrap_or(0);
        e.choose(i);
    }
}

fn hand(e: &mut Engine, seat: u8, cards: &[&str]) {
    for c in cards {
        e.state.seats[seat as usize].hand.push(find_library(c));
    }
}

fn play(card: u8, actor: MinionId) -> Choice {
    Choice::Play {
        card,
        mode: 0,
        actor: Some(actor),
        target: Target::None,
    }
}

fn action(minion: MinionId, action: ActionKind) -> Choice {
    Choice::TakeAction {
        minion,
        action,
        target: None,
    }
}

/// The `nth` untitled V5 vampire of `clan` with no printed-text definition
/// and capacity at least 4.
fn plain(clan: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.clan == clan
                && c.title == Title::None
                && c.capacity >= 4
                && cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no plain {clan} vampire"))
}

/// The actions `m` is offered in the current request.
fn actions_of(req: &DecisionRequest, m: MinionId) -> Vec<Choice> {
    req.options
        .iter()
        .copied()
        .filter(|o| match *o {
            Choice::TakeAction { minion, .. } => minion == m,
            Choice::Play { actor, .. } | Choice::UseAbility { actor, .. } => actor == Some(m),
            _ => false,
        })
        .collect()
}

#[test]
fn recruit_ally_is_a_plus_one_stealth_undirected_action_paid_at_resolution() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    // A second minion keeps the minion phase open after the action.
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Political Ally"]);
    let pool = e.state.seats[0].pool;
    pick(&mut e, play(0, v));
    // +1 stealth against no intercept: nobody may block; the ally is in
    // play before the next decision.
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::ActionBlock, "{req:?}");
    assert_eq!(
        e.state.seats[0].pool,
        pool - 2,
        "2 pool, paid at resolution"
    );
    let ally = MinionId(e.state.minions.len() as u32 - 1);
    let a = &e.state.minions[ally.index()];
    assert!(a.is_ally() && a.present() && !a.locked && a.recruited);
    assert_eq!(
        (a.owner, a.blood, a.name()),
        (SeatId(0), 1, "Political Ally")
    );
    let id = a.ally_id().expect("an ally's card");
    assert_eq!(e.state.cards[id.index()].body, Some(ally));
    assert_eq!(e.state.card_controller(id), SeatId(0));
    assert!(e.state.seats[0].ash.is_empty(), "the card entered play");
    // It cannot act this turn (rulebook "Recruit Ally").
    assert_eq!(req.window, WindowLabel::MinionPhase);
    assert!(actions_of(&req, ally).is_empty(), "{req:?}");
}

#[test]
fn an_ally_bleeds_for_its_printed_bleed_and_cannot_hunt() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    let ally = e.add_ally(SeatId(0), find_library("Political Ally"));
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        actions_of(&req, ally),
        vec![action(ally, ActionKind::Bleed)]
    );
    let pool = e.state.seats[1].pool;
    pick(&mut e, action(ally, ActionKind::Bleed));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, pool - 3, "0 strength, 3 bleed");
}

#[test]
fn a_unique_ally_another_methuselah_controls_is_contested_and_ones_own_copy_bars_a_second() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    let theirs = e.add_ally(SeatId(1), find_library("Political Ally"));
    hand(&mut e, 0, &["Political Ally", "Political Ally"]);
    pick(&mut e, play(0, v));
    let req = request(&mut e);
    let mine = MinionId(e.state.minions.len() as u32 - 1);
    for m in [mine, theirs] {
        let a = &e.state.minions[m.index()];
        assert!(a.contested && !a.present(), "{m:?}");
        let id = a.ally_id().unwrap();
        assert!(e.state.cards[id.index()].contested);
    }
    // "You cannot voluntarily contest cards with yourself."
    assert!(!req
        .options
        .iter()
        .any(|o| matches!(o, Choice::Play { card: 0, .. })));
}

#[test]
fn damage_burns_an_allys_life_and_its_last_life_burns_the_ally_and_its_card() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    pick(&mut e, action(v, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: ally });
    // Hand strikes: 1 damage burns its only life; its 0 strength deals none.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let a = &e.state.minions[ally.index()];
    assert!(!a.in_play && !a.in_torpor);
    assert_eq!(e.state.seats[1].ash, vec![find_library("Political Ally")]);
    let id = a.ally_id().unwrap();
    assert!(!e.state.cards[id.index()].in_play);
    assert_eq!(e.state.minions[v.index()].blood, 4);
}

/// The `nth` untitled V5 vampire printing the Discipline `tag` exactly
/// (`ani` basic, `ANI` superior), with no printed-text definition.
fn with_discipline(tag: &str, nth: usize) -> u16 {
    of_clan_with("", tag, nth)
}

/// The `nth` untitled Anarch vampire with no printed-text definition.
fn anarch(nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.sect == zvtes::crypt::Sect::Anarch
                && c.title == Title::None
                && cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .expect("a plain Anarch")
}

/// As `with_discipline`, of `clan` (any clan if empty).
fn of_clan_with(clan: &str, tag: &str, nth: usize) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            (clan.is_empty() || c.clan == clan)
                && c.disciplines.contains(&tag)
                && c.title == Title::None
                && cards::vampire(*i as u16).is_none()
        })
        .nth(nth)
        .map(|(i, _)| i as u16)
        .unwrap_or_else(|| panic!("no plain vampire with {tag}"))
}

fn play_mode(card: u8, mode: u8, actor: MinionId) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target: Target::None,
    }
}

/// The retainer `card` (a LIBRARY index) in play on `host`, if any.
fn retainer_on(e: &Engine, card: u16, host: MinionId) -> Option<&CardInPlay> {
    e.state
        .cards
        .iter()
        .find(|c| c.in_play && c.card == card && c.host == Some(host))
}

// [RTR 19941109] [ANK 20180612] "The discipline level used to recruit it
// decides which version it is (inferior or superior). This does not change
// once it is in play."
#[test]
fn raven_spy_is_the_version_it_was_employed_at_and_gives_the_employer_intercept() {
    let mut e = Engine::bare(3);
    let sup = e.add_minion(SeatId(0), with_discipline("ANI", 0), 4, false);
    let basic = e.add_minion(SeatId(0), with_discipline("ani", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Raven Spy", "Raven Spy"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(
        !req.options.contains(&play_mode(0, 1, basic)),
        "[ANI] needs superior"
    );
    pick(&mut e, play_mode(0, 1, sup));
    let pool = e.state.seats[0].pool;
    pick(&mut e, play_mode(0, 0, basic));
    request(&mut e);
    let spy = find_library("Raven Spy");
    assert_eq!(retainer_on(&e, spy, sup).map(|c| c.counters), Some(2));
    assert_eq!(retainer_on(&e, spy, basic).map(|c| c.counters), Some(1));
    assert_eq!(e.state.minions[sup.index()].blood, 3, "1 blood each");
    assert_eq!(e.state.seats[0].pool, pool);
    for m in [sup, basic] {
        assert_eq!(e.state.bearer_mod(m, Stat::Intercept, ActionKind::Bleed), 1);
    }
}

#[test]
fn revenant_is_employed_by_a_tzimisce_only() {
    let mut e = Engine::bare(3);
    let tzimisce = e.add_minion(SeatId(0), plain("Tzimisce", 0), 4, false);
    let ventrue = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Revenant"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&play(0, tzimisce)));
    assert!(!req.options.contains(&play(0, ventrue)));
}

/// A retainer already on `host` in the version `mode`, with its life and
/// layers (no play).
fn retainer(e: &mut Engine, name: &str, host: MinionId, mode: usize) -> CardId {
    let card = find_library(name);
    let owner = e.state.minions[host.index()].owner;
    let def = cards::lookup(card).expect("definition");
    let life = def.modes[mode]
        .effects
        .iter()
        .find_map(|x| match x {
            Effect::EmployRetainer { life } => Some(*life),
            _ => None,
        })
        .expect("a retainer");
    e.state.cards.push(CardInPlay {
        card,
        owner,
        controller: owner,
        host: Some(host),
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: life,
        stored: Vec::new(),
    });
    let id = CardId(e.state.cards.len() as u32 - 1);
    for effect in def.modes[mode].effects {
        if let Effect::Layer(modifier) = effect {
            e.state.layers.push(Layer {
                scope: Scope::InPlay,
                modifier: *modifier,
                source: id,
            });
        }
    }
    id
}

/// Seat 0's Brujah `a` with Murder of Crows bleeds; seat 1's `z` blocks.
fn crows_bleed_blocked_by(z_card: u16, z_hand: &str) -> (Engine, MinionId, MinionId) {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), z_card, 4, false);
    retainer(&mut e, "Murder of Crows", a, 0);
    hand(&mut e, 1, &[z_hand]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    (e, a, z)
}

// [RTR 19941109] [TOM 19951216] [PIB 20140324] [ANK 20200422] Environmental
// damage "is inflicted even against a dodge. A strike: combat ends ends
// combat before damage is applied."
#[test]
fn murder_of_crows_damages_the_opposing_minion_each_round_even_against_a_dodge() {
    let (mut e, a, z) = crows_bleed_blocked_by(with_discipline("cel", 0), "Side Strike");
    // A's hand strike is its only strike (elided).
    pick(&mut e, play_mode(0, 0, z)); // dodge
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[z.index()].blood, 3, "1R from the crows");
    assert_eq!(e.state.minions[a.index()].blood, 4);
}

#[test]
fn murder_of_crows_inflicts_nothing_once_a_combat_ends_strike_ended_the_combat() {
    let (mut e, _, z) = crows_bleed_blocked_by(with_discipline("pre", 0), "Majesty");
    pick(&mut e, play_mode(0, 0, z)); // combat ends
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[z.index()].blood,
        3,
        "Majesty's 1 blood only"
    );
}

// [RTR 19941109] A retainer that strikes "does not gain additional strikes
// from its owner": the crows' damage comes with the normal strikes only.
#[test]
fn murder_of_crows_does_not_repeat_on_additional_strikes() {
    let (mut e, a, z) = crows_bleed_blocked_by(with_discipline("CEL", 0), "Side Strike");
    expect(&mut e, SeatId(1), WindowLabel::CombatBeforeStrikes);
    pick(&mut e, play_mode(0, 1, z)); // an additional strike
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Normal pair: hand strikes both ways and the crows; then Z's
    // additional hand strike alone.
    assert_eq!(e.state.minions[z.index()].blood, 2);
    assert_eq!(e.state.minions[a.index()].blood, 2);
}

// Rulebook "Leave Torpor": a blocked leave-torpor has no combat; "if they
// choose not to [diablerise], or if they are an ally, then the action
// simply fails".
#[test]
fn an_ally_blocking_a_leave_torpor_action_has_no_diablerie_to_offer() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), plain("Brujah", 0), 3, true);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    // Mr. Winthrop's +1 intercept meets the action's +1 stealth.
    retainer(&mut e, "Mr. Winthrop", ally, 0);
    pick(&mut e, action(v, ActionKind::LeaveTorpor));
    pick(&mut e, Choice::DeclareBlock { minion: ally });
    let req = request(&mut e);
    assert_ne!(req.window, WindowLabel::TorporBlock, "{req:?}");
    let m = &e.state.minions[v.index()];
    assert!(m.in_play && m.in_torpor);
    assert_eq!(m.blood, 3, "no cost paid");
}

// Printed text naming allies: Ragnar Nordstrom "gets +1 strength in combat
// with an ally or younger vampire"; Adisa "can enter combat with an ally or
// younger vampire as a Ⓓ action".
#[test]
fn ragnar_and_adisa_read_an_ally_as_their_texts_say() {
    let mut e = Engine::bare(3);
    let ragnar = e.add_minion(SeatId(0), find_card("Ragnar Nordstrom"), 4, false);
    let adisa = e.add_minion(SeatId(0), find_card("Adisa"), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    let ctx = PrintedCtx {
        opponent: Some(ally),
        ..PrintedCtx::default()
    };
    assert_eq!(e.state.printed_mod(ragnar, Stat::Strength, ctx), 1);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&Choice::UseAbility {
        source: AbilitySource::Vampire(adisa),
        ability: 0,
        actor: Some(adisa),
        target: Target::Minion(ally),
    }));
}

// [PIB 20121028] Taste of Vitae "cannot be played against an ally" (it
// names the opposing vampire; advanced rules 5.5.1).
#[test]
fn taste_of_vitae_cannot_be_played_against_an_ally() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    // War Ghoul: 5 life, 4 strength; its entry burn takes a retainer.
    let v = e.add_minion(SeatId(1), plain("Tzimisce", 0), 4, false);
    retainer(&mut e, "Revenant", v, 0);
    let ghoul = e.add_ally(SeatId(1), find_library("War Ghoul"));
    hand(&mut e, 0, &["Taste of Vitae"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: ghoul });
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        assert!(!req.options.contains(&play(0, a)), "{req:?}");
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
            .unwrap_or(0);
        e.choose(i);
    }
    // Both still ready at the end of the round, A mended its 4 damage.
    assert_eq!(e.state.minions[ghoul.index()].blood, 4, "1 of its 5 life");
    let v = &e.state.minions[a.index()];
    assert!(v.blood == 0 && v.ready());
    assert_eq!(e.state.seats[0].hand, vec![find_library("Taste of Vitae")]);
}

// The cost is paid at resolution (spec §1.7.1): unpayable by then, the
// recruit fizzles and the card is discarded (rules-qa 3.6).
#[test]
fn a_recruit_the_methuselah_cannot_pay_at_resolution_fizzles_and_the_card_is_discarded() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    e.add_minion(SeatId(0), plain("Ventrue", 1), 4, false);
    // Seat 1's vampire has the intercept to try a block: the action waits.
    let z = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    retainer(&mut e, "Mr. Winthrop", z, 0);
    hand(&mut e, 0, &["Political Ally"]);
    pick(&mut e, play(0, v));
    expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    e.state.seats[0].pool = 1;
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, 1);
    assert!(e.state.minions.iter().all(|m| !m.is_ally()));
    assert_eq!(e.state.seats[0].ash, vec![find_library("Political Ally")]);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
}

// ---- Slice 10b: the pre-built-deck allies' and retainers' text -----------

fn ability_of(source: CardId, ability: u8, actor: Option<MinionId>) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(source),
        ability,
        actor,
        target: Target::None,
    }
}

#[test]
fn forty_seventh_street_royals_burns_to_reduce_a_bleed_against_you_by_3() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let royals = e.add_ally(SeatId(1), find_library("47th Street Royals"));
    let id = e.state.minions[royals.index()].ally_id().unwrap();
    let pool = e.state.seats[1].pool;
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, ability_of(id, 0, None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, pool, "a bleed of 1 reduced by 3");
    assert!(!e.state.minions[royals.index()].in_play);
    assert_eq!(
        e.state.seats[1].ash,
        vec![find_library("47th Street Royals")]
    );
}

// [ANK 20180517] Allies with "lock this ally to" abilities "can lock to use
// their ability the turn they were recruited".
#[test]
fn city_star_taxi_locks_to_give_the_acting_ravnos_stealth_when_needed() {
    let mut e = Engine::bare(3);
    let r = e.add_minion(SeatId(0), plain("Ravnos", 0), 4, false);
    let taxi = e.add_ally(SeatId(0), find_library("City Star Taxi"));
    // Recruited this turn.
    e.state.minions[taxi.index()].recruited = true;
    let id = e.state.minions[taxi.index()].ally_id().unwrap();
    let z = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    pick(&mut e, action(r, ActionKind::Bleed));
    // Not needed yet: no stealth before an attempt needs it (0 against 0).
    pick(&mut e, Choice::DeclareBlock { minion: z });
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert!(req.options.contains(&ability_of(id, 2, None)), "{req:?}");
    pick(&mut e, ability_of(id, 2, None));
    assert!(
        e.state.minions[taxi.index()].locked,
        "the ally locks, not its card"
    );
    let pool = e.state.seats[1].pool;
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, pool - 1, "the attempt failed: bled");
}

#[test]
fn city_star_taxi_strikes_for_1r_and_maneuvers_once_each_combat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let taxi = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    let id = e.state.minions[taxi.index()].ally_id().unwrap();
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: taxi });
    let req = until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    assert!(req.options.contains(&ability_of(id, 1, None)), "{req:?}");
    pick(&mut e, ability_of(id, 1, None)); // to long range
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    let _ = req;
    pick(&mut e, ability_of(id, 0, None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Long range: the hand strike misses, the 1R hits.
    assert_eq!(e.state.minions[a.index()].blood, 3);
    assert_eq!(e.state.minions[taxi.index()].blood, 2);
}

#[test]
fn double_deuce_has_stealth_and_heals_to_3_in_its_unlock_phase() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Gangrel", 0), 4, false);
    let dd = e.add_ally(SeatId(0), find_library("Double Deuce"));
    assert_eq!(e.state.bearer_mod(dd, Stat::Stealth, ActionKind::Bleed), 1);
    e.state.minions[dd.index()].blood = 2;
    // Around the table to seat 0's next turn (nobody else has a minion).
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[dd.index()].blood, 3);
    // At 3 life, no more.
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[dd.index()].blood, 3);
}

#[test]
fn dog_pack_bars_combat_ends_strikes_against_its_employer() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_discipline("pre", 0), 3, false);
    retainer(&mut e, "Dog Pack", a, 0);
    hand(&mut e, 1, &["Majesty"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    // Majesty is Z's only card: with it barred, the strike window elides.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].hand, vec![find_library("Majesty")]);
    assert_eq!(
        e.state.minions[z.index()].blood,
        2,
        "hit by the hand strike"
    );
}

// [RTR 19941109] [ANK 20180612] "The discipline level used to recruit it
// decides which version it is": the [ani] unlock waits for the discard
// phase, the [ANI] one comes after resolution.
#[test]
fn feral_hound_unlocks_its_employer_at_resolution_or_in_the_discard_phase() {
    // [ANI]: after resolution.
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), of_clan_with("Ravnos", "ANI", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Feral Hound"]);
    pick(&mut e, play_mode(0, 1, v));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[v.index()].locked);
    // [ani]: locked until the discard phase.
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), of_clan_with("Ravnos", "ani", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Feral Hound"]);
    pick(&mut e, play_mode(0, 0, v));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[v.index()].locked);
    // A card in hand keeps the discard phase from eliding.
    hand(&mut e, 0, &["Majesty"]);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::DiscardPhase);
    assert!(!e.state.minions[v.index()].locked);
}

#[test]
fn feral_hound_locks_to_give_its_employer_intercept() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Gangrel", 0), 4, false);
    let hound = retainer(&mut e, "Feral Hound", z, 0);
    // A hunt (+1 stealth): Z needs the hound to block.
    pick(&mut e, action(a, ActionKind::Hunt));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, ability_of(hound, 0, None));
    assert!(e.state.cards[hound.index()].locked);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

// A failed action "has reached resolution, so is subject to NRA" [ANK
// 20211015] [RTR 20180719]; "if an action fails, it was not blocked" [LSJ
// 20070219].
#[test]
fn szlachta_bodyguard_burns_to_fail_an_action_directed_at_your_minion() {
    let mut e = Engine::bare(3);
    let adisa = e.add_minion(SeatId(0), find_card("Adisa"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    let guard = retainer(&mut e, "Szlachta Bodyguard", ally, 0);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(adisa),
            ability: 0,
            actor: Some(adisa),
            target: Target::Minion(ally),
        },
    );
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&ability_of(guard, 1, None)), "{req:?}");
    pick(&mut e, ability_of(guard, 1, None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Failed, not blocked; it reached resolution (NRA).
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
    assert!(!e.state.minions[adisa.index()].acted.is_empty());
    assert!(e.state.minions[ally.index()].in_play, "no combat");
    assert!(!e.state.cards[guard.index()].in_play);
}

#[test]
fn szlachta_bodyguard_locks_to_prevent_1_damage_to_its_employer() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    let guard = retainer(&mut e, "Szlachta Bodyguard", a, 0);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    until(&mut e, SeatId(0), WindowLabel::CombatPrevention);
    pick(&mut e, ability_of(guard, 0, None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.cards[guard.index()].locked);
    assert_eq!(e.state.minions[a.index()].blood, 4);
    assert_eq!(e.state.minions[z.index()].blood, 3);
}

// [ANK 20260316] [LSJ 20041022] Failing an action "cannot be used while a
// block is being attempted, or has already been successful."
#[test]
fn szlachta_bodyguard_cannot_fail_an_action_under_a_block_attempt() {
    let mut e = Engine::bare(3);
    let adisa = e.add_minion(SeatId(0), find_card("Adisa"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    let guard = retainer(&mut e, "Szlachta Bodyguard", ally, 0);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(adisa),
            ability: 0,
            actor: Some(adisa),
            target: Target::Minion(ally),
        },
    );
    pick(&mut e, Choice::DeclareBlock { minion: ally });
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        assert!(
            !req.options.contains(&ability_of(guard, 1, None)),
            "{req:?}"
        );
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
            .unwrap_or(0);
        e.choose(i);
    }
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Blocked));
}

// [ANK 20190412] Dog Pack put on a minion after the opponent declared a
// strike: combat ends — that strike has no effect.
#[test]
fn a_combat_ends_strike_has_no_effect_once_dog_pack_is_on_its_opponent() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), with_discipline("pre", 0), 3, false);
    hand(&mut e, 1, &["Majesty"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    pick(&mut e, play_mode(0, 0, z));
    // (As Pack Alpha would, before strike resolution.)
    retainer(&mut e, "Dog Pack", a, 0);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // Majesty paid, then the hand strike landed: the combat did not end.
    assert_eq!(e.state.minions[z.index()].blood, 1);
    assert_eq!(e.state.seats[1].ash, vec![find_library("Majesty")]);
}

/// Seat 0's War Ghoul (in play, able to act) beside a Tzimisce.
fn war_ghoul() -> (Engine, MinionId, CardId) {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), plain("Tzimisce", 0), 4, false);
    retainer(&mut e, "Revenant", v, 0);
    let ghoul = e.add_ally(SeatId(0), find_library("War Ghoul"));
    request(&mut e); // the entry burn: the Revenant, its only choice
    let id = e.state.minions[ghoul.index()].ally_id().unwrap();
    (e, ghoul, id)
}

// War Ghoul burns an ally or retainer its controller controls as it enters
// play, however it enters [ANK 20170309-2] — itself when there is no other
// [RTR 19961113].
#[test]
fn war_ghoul_burns_an_ally_or_retainer_on_entry_or_itself() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), plain("Tzimisce", 0), 4, false);
    e.add_minion(SeatId(0), plain("Tzimisce", 1), 4, false);
    let rev = retainer(&mut e, "Revenant", v, 0);
    let pa = e.add_ally(SeatId(0), find_library("Political Ally"));
    let pa_card = e.state.minions[pa.index()].ally_id().unwrap();
    hand(&mut e, 0, &["War Ghoul"]);
    pick(&mut e, play(0, v));
    let req = expect(&mut e, SeatId(0), WindowLabel::EntryEffect);
    assert_eq!(
        req.options,
        vec![
            Choice::BurnInPlay { card: rev.0 },
            Choice::BurnInPlay { card: pa_card.0 },
        ]
    );
    pick(&mut e, Choice::BurnInPlay { card: pa_card.0 });
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[pa.index()].in_play);
    assert!(e.state.cards[rev.index()].in_play);
    let ghoul = MinionId(e.state.minions.len() as u32 - 1);
    assert!(e.state.minions[ghoul.index()].in_play);

    // Nothing else to burn: itself.
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), plain("Tzimisce", 0), 4, false);
    e.add_minion(SeatId(0), plain("Tzimisce", 1), 4, false);
    hand(&mut e, 0, &["War Ghoul"]);
    pick(&mut e, play(0, v));
    expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions.iter().all(|m| !m.is_ally() || !m.in_play));
    assert_eq!(e.state.seats[0].ash, vec![find_library("War Ghoul")]);

    // Put in play other than by recruiting: the entry burn all the same.
    let (e, _, _) = war_ghoul();
    assert!(e
        .state
        .cards
        .iter()
        .any(|c| { c.card == find_library("Revenant") && !c.in_play }));
}

#[test]
fn war_ghoul_enters_combat_with_a_vampire_and_prevents_1_damage_each_round() {
    let (mut e, ghoul, id) = war_ghoul();
    let z = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    let rush = |m| Choice::UseAbility {
        source: AbilitySource::Card(id),
        ability: 1,
        actor: Some(ghoul),
        target: Target::Minion(m),
    };
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&rush(z)));
    assert!(!req.options.contains(&rush(ally)), "a vampire only");
    pick(&mut e, rush(z));
    until(&mut e, SeatId(0), WindowLabel::CombatPrevention);
    pick(&mut e, ability_of(id, 0, None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[ghoul.index()].blood,
        5,
        "1 damage prevented"
    );
    assert_eq!(e.state.minions[z.index()].blood, 0, "4 strength");
}

// [ANK 20180517] Allies with "lock this ally to" abilities "can lock to use
// their ability the turn they were recruited".
#[test]
fn war_ghoul_locks_and_burns_itself_to_burn_a_location() {
    let (mut e, ghoul, id) = war_ghoul();
    e.state.minions[ghoul.index()].recruited = true;
    e.state.cards.push(CardInPlay {
        card: find_library("Academic Hunting Ground"),
        owner: SeatId(1),
        controller: SeatId(1),
        host: None,
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
    });
    let hg = CardId(e.state.cards.len() as u32 - 1);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(id),
            ability: 2,
            actor: None,
            target: Target::Card(hg),
        },
    );
    request(&mut e);
    assert!(!e.state.cards[hg.index()].in_play);
    assert!(!e.state.minions[ghoul.index()].in_play);
    assert!(e.state.seats[0].ash.contains(&find_library("War Ghoul")));
}

#[test]
fn vozhd_of_sofia_discards_an_ally_or_retainer_card_for_2_life() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), plain("Tzimisce", 0), 4, false);
    retainer(&mut e, "Revenant", v, 0);
    let vozhd = e.add_ally(SeatId(0), find_library("Vozhd of Sofia, The"));
    let id = e.state.minions[vozhd.index()].ally_id().unwrap();
    e.state.minions[vozhd.index()].blood = 2;
    hand(&mut e, 0, &["Political Ally", "Majesty"]);
    // Around to seat 0's next unlock phase.
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&ability_of(id, 2, None)), "{req:?}");
    pick(&mut e, ability_of(id, 2, None));
    // The discard: the ally card, the only one that qualifies.
    request(&mut e);
    assert_eq!(e.state.minions[vozhd.index()].blood, 4);
    let pa = find_library("Political Ally");
    assert!(e.state.seats[0].ash.contains(&pa));
    assert!(!e.state.seats[0].hand.contains(&pa));
}

// [ANK 20180517] Allies with "lock this ally to" abilities "can lock to use
// their ability the turn they were recruited".
#[test]
fn vagabond_mystic_heals_a_hurt_ally_and_cannot_block_vampires() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), plain("Gangrel", 0), 4, false);
    let pa = e.add_ally(SeatId(0), find_library("Political Ally"));
    let mystic = e.add_ally(SeatId(1), find_library("Vagabond Mystic"));
    let deuce = e.add_ally(SeatId(1), find_library("Double Deuce"));
    let id = e.state.minions[mystic.index()].ally_id().unwrap();
    e.state.minions[mystic.index()].recruited = true;
    e.state.minions[deuce.index()].blood = 1;
    // A vampire's bleed: the mystic may not block it.
    pick(&mut e, action(v, ActionKind::Bleed));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(!req
        .options
        .contains(&Choice::DeclareBlock { minion: mystic }));
    assert!(req
        .options
        .contains(&Choice::DeclareBlock { minion: deuce }));
    // Only the hurt ally is a target (Double Deuce below its 3 life).
    let heal = |t| Choice::UseAbility {
        source: AbilitySource::Card(id),
        ability: 0,
        actor: None,
        target: Target::Minion(t),
    };
    assert!(req.options.contains(&heal(deuce)));
    assert!(!req.options.contains(&heal(mystic)));
    pick(&mut e, heal(deuce));
    assert_eq!(e.state.minions[deuce.index()].blood, 2);
    assert!(e.state.minions[mystic.index()].locked);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // An ally's bleed: it may block (unlocked again next turn).
    e.state.minions[mystic.index()].locked = false;
    pick(&mut e, action(pa, ActionKind::Bleed));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req
        .options
        .contains(&Choice::DeclareBlock { minion: mystic }));
}

#[test]
fn underbridge_stray_superior_has_2_life_and_1_strength() {
    let mut e = Engine::bare(3);
    let basic = e.add_ally(SeatId(0), find_library("Underbridge Stray"));
    let sup = e.add_ally_version(SeatId(0), find_library("Underbridge Stray"), 1);
    assert_eq!(e.state.minions[basic.index()].blood, 1);
    assert_eq!(e.state.minions[sup.index()].blood, 2);
    assert_eq!(e.state.base_strength(basic), 0);
    assert_eq!(e.state.base_strength(sup), 1);
}

// [ANK 20180517] "He can burn 1 life to give a minion you control a press
// the turn he is recruited."
#[test]
fn underbridge_stray_burns_life_for_a_press_for_its_controllers_combatant() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    let stray = e.add_ally(SeatId(1), find_library("Underbridge Stray"));
    e.state.minions[stray.index()].recruited = true;
    let id = e.state.minions[stray.index()].ally_id().unwrap();
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    until(&mut e, SeatId(1), WindowLabel::CombatPress);
    pick(&mut e, ability_of(id, 0, None));
    // Its last life: burned; a second round follows.
    assert!(!e.state.minions[stray.index()].in_play);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[a.index()].blood, 2);
    assert_eq!(e.state.minions[z.index()].blood, 2);
}

#[test]
fn underbridge_stray_burns_to_unlock_a_minion_during_an_action_directed_at_you() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    e.state.minions[z.index()].locked = true;
    let stray = e.add_ally(SeatId(1), find_library("Underbridge Stray"));
    let id = e.state.minions[stray.index()].ally_id().unwrap();
    pick(&mut e, action(a, ActionKind::Bleed));
    let unlock = Choice::UseAbility {
        source: AbilitySource::Card(id),
        ability: 1,
        actor: None,
        target: Target::Minion(z),
    };
    pick(&mut e, unlock);
    assert!(!e.state.minions[z.index()].locked);
    assert!(!e.state.minions[stray.index()].in_play);
    // Z can now block.
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&Choice::DeclareBlock { minion: z }));
}

// Crypt's Sons continues the action as if unblocked, before block
// resolution: no combat, the blocker stays locked [LSJ 20080611] [ANK
// 20210131] — so no reaction pertaining to that combat [ANK 20210131]; the
// action's modifiers and reactions stand [RTR 19941109] [ANK 20190116].
#[test]
fn crypts_sons_burn_a_life_to_continue_a_blocked_action_as_if_unblocked() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let sons = retainer(&mut e, "Crypt's Sons", a, 0);
    let pool = e.state.seats[1].pool;
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlocked);
    assert_eq!(req.options, vec![Choice::Pass, ability_of(sons, 0, None)]);
    pick(&mut e, ability_of(sons, 0, None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, pool - 1, "bled");
    assert!(e.state.minions[z.index()].locked);
    assert_eq!(e.state.minions[z.index()].blood, 4, "no combat");
    assert_eq!(e.state.cards[sons.index()].counters, 2);
}

#[test]
fn szlachta_assistant_takes_2_off_a_tzimisce_ghoul_ally_recruit() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), plain("Tzimisce", 0), 4, false);
    e.add_minion(SeatId(0), plain("Tzimisce", 1), 4, false);
    let assistant = retainer(&mut e, "Szlachta Assistant", v, 0);
    retainer(&mut e, "Revenant", v, 0); // for War Ghoul's entry burn
                                        // Seat 1's intercept keeps the action open for the acting seat.
    let z = e.add_minion(SeatId(1), plain("Brujah", 0), 4, false);
    retainer(&mut e, "Mr. Winthrop", z, 0);
    hand(&mut e, 0, &["War Ghoul"]);
    let pool = e.state.seats[0].pool;
    pick(&mut e, play(0, v));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionBlock);
    assert!(
        req.options.contains(&ability_of(assistant, 0, None)),
        "{req:?}"
    );
    pick(&mut e, ability_of(assistant, 0, None));
    // Seat 1 lets it through; with the assistant gone, the Revenant is the
    // entry burn's only choice.
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[0].pool, pool - 1, "3 pool less 2");
    assert!(!e.state.cards[assistant.index()].in_play);
    let ghoul = MinionId(e.state.minions.len() as u32 - 1);
    assert!(e.state.minions[ghoul.index()].in_play);
}

// [ANK 20180517] "A copy that was recruited this turn can be locked to
// grant the bleed bonus." [ANK 20231124] [LSJ 19970718] The bleed increase
// is used during the bleed action, before its resolution.
#[test]
fn bone_shambler_bleeds_more_with_another_copy_and_at_obl_a_hecatas_blood() {
    let mut e = Engine::bare(3);
    let h = e.add_minion(SeatId(0), plain("Hecata", 0), 4, false);
    let a = e.add_ally_version(SeatId(0), find_library("Bone Shambler"), 1);
    let b = e.add_ally(SeatId(0), find_library("Bone Shambler"));
    e.state.minions[b.index()].recruited = true;
    let id = e.state.minions[a.index()].ally_id().unwrap();
    // It cannot have or use equipment.
    hand(&mut e, 0, &["Sport Bike"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&play(0, h)));
    assert!(!req.options.contains(&play(0, a)) && !req.options.contains(&play(0, b)));
    let pool = e.state.seats[1].pool;
    pick(&mut e, action(a, ActionKind::Bleed));
    let with = |ability, t| Choice::UseAbility {
        source: AbilitySource::Card(id),
        ability,
        actor: None,
        target: Target::Minion(t),
    };
    pick(&mut e, with(0, b));
    pick(&mut e, with(1, h));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, pool - 3);
    assert!(e.state.minions[b.index()].locked);
    assert_eq!(e.state.minions[h.index()].blood, 3);
}

#[test]
fn crypts_sons_is_employed_by_an_anarch_only() {
    let mut e = Engine::bare(3);
    let anarch = e.add_minion(SeatId(0), anarch(0), 4, false);
    let other = e.add_minion(SeatId(0), plain("Ventrue", 0), 4, false);
    assert_ne!(
        CRYPT[plain("Ventrue", 0) as usize].sect,
        zvtes::crypt::Sect::Anarch
    );
    hand(&mut e, 0, &["Crypt's Sons"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&play(0, anarch)));
    assert!(!req.options.contains(&play(0, other)));
}

// [ANK 20190116] "If the action continues as if unblocked, any Methuselahs
// that have declined to block before do not get another opportunity."
#[test]
fn crypts_sons_continuing_keeps_the_declines() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch(0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    let w = e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    for m in [z, w] {
        retainer(&mut e, "Mr. Winthrop", m, 0);
    }
    let sons = retainer(&mut e, "Crypt's Sons", a, 0);
    // A hunt: undirected, prey and predator may block.
    pick(&mut e, action(a, ActionKind::Hunt));
    let mut asked = Vec::new();
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        asked.push((req.seat, req.window));
        // Seat 1 (the prey, first) declines; seat 2 blocks.
        let choice = if req.options.contains(&Choice::DeclareBlock { minion: w }) {
            Choice::DeclareBlock { minion: w }
        } else if req.options.contains(&ability_of(sons, 0, None)) {
            ability_of(sons, 0, None)
        } else {
            Choice::Pass
        };
        let i = req.options.iter().position(|o| *o == choice).unwrap();
        e.choose(i);
    }
    // Seat 1 declined once, before the block, and was not asked again.
    let seat1 = asked
        .iter()
        .filter(|&&(s, w)| s == SeatId(1) && w == WindowLabel::ActionBlock)
        .count();
    assert_eq!(seat1, 1, "{asked:?}");
    assert_eq!(e.state.minions[a.index()].blood, 2, "the hunt succeeded");
    assert_eq!(
        e.state.cards[sons.index()].counters,
        2,
        "once for the block"
    );
    let _ = z;
}

// [LSJ 20001111] Vagabond Mystic's "ability cannot be used in the middle of
// a single damage resolution step".
#[test]
fn vagabond_mystic_is_not_used_during_damage_resolution() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let mystic = e.add_ally(SeatId(1), find_library("Vagabond Mystic"));
    let deuce = e.add_ally(SeatId(1), find_library("Double Deuce"));
    e.state.minions[deuce.index()].blood = 2;
    let id = e.state.minions[mystic.index()].ally_id().unwrap();
    pick(&mut e, action(a, ActionKind::Bleed));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::DeclareBlock { minion: deuce });
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let heal = req.options.iter().any(
            |o| matches!(o, Choice::UseAbility { source: AbilitySource::Card(c), .. } if *c == id),
        );
        assert!(
            !(heal && req.window == WindowLabel::CombatPrevention),
            "{req:?}"
        );
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
            .unwrap_or(0);
        e.choose(i);
    }
}

#[test]
fn screamer_burns_for_minus_1_stealth_on_a_predator_or_prey_action() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 1, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Hecata", 0), 4, false);
    let screamer = e.add_ally(SeatId(1), find_library("Screamer"));
    let id = e.state.minions[screamer.index()].ally_id().unwrap();
    // A hunt (+1 stealth): no block possible for Z until the -1.
    pick(&mut e, action(a, ActionKind::Hunt));
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionAsAnnounced);
    assert!(req.options.contains(&ability_of(id, 0, None)), "{req:?}");
    pick(&mut e, ability_of(id, 0, None));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&Choice::DeclareBlock { minion: z }));
    assert!(!e.state.minions[screamer.index()].in_play);
}

// Failing an action as announced: after the acting Methuselah's own
// as-announced effects [LSJ 20081105], before anything else [ANK 20190425];
// then the action has failed, not been blocked [LSJ 20070219], and reached
// resolution [ANK 20211015] [RTR 20180719].
#[test]
fn screamer_burns_to_fail_an_action_directed_at_your_ally_as_announced() {
    let mut e = Engine::bare(3);
    let adisa = e.add_minion(SeatId(0), find_card("Adisa"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let screamer = e.add_ally(SeatId(1), find_library("Screamer"));
    let pa = e.add_ally(SeatId(1), find_library("Political Ally"));
    let id = e.state.minions[screamer.index()].ally_id().unwrap();
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(adisa),
            ability: 0,
            actor: Some(adisa),
            target: Target::Minion(pa),
        },
    );
    let req = expect(&mut e, SeatId(1), WindowLabel::ActionAsAnnounced);
    assert!(req.options.contains(&ability_of(id, 1, None)), "{req:?}");
    pick(&mut e, ability_of(id, 1, None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Failed));
    assert!(!e.state.minions[adisa.index()].acted.is_empty());
    assert!(e.state.minions[pa.index()].in_play);
}

// [LSJ 20030530] Raptor [ANI]: the opponent discards down for the combat
// and draws back up as it ends.
#[test]
fn raptor_superior_takes_1_hand_size_from_the_opponent_while_in_combat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    retainer(&mut e, "Raptor", a, 1);
    hand(&mut e, 1, &["Majesty"; 7]);
    e.state.seats[1].library = vec![find_library("Majesty"); 5];
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    let req = until(&mut e, SeatId(1), WindowLabel::DiscardDown);
    assert_eq!(e.state.hand_size(SeatId(1)), 6);
    let _ = req;
    pick(&mut e, Choice::Discard(0));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.hand_size(SeatId(1)), 7);
    assert_eq!(e.state.seats[1].hand.len(), 7, "drawn back up");
}

#[test]
fn vozhd_of_szczecin_unlocks_for_a_tzimisces_blood_in_another_minion_phase() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Tzimisce", 0), 4, false);
    retainer(&mut e, "Revenant", t, 0); // the entry burn
    let vozhd = e.add_ally(SeatId(1), find_library("Vozhd of Szczecin, The"));
    request(&mut e); // (the entry burn elides onto the Revenant)
    let id = e.state.minions[vozhd.index()].ally_id().unwrap();
    e.state.minions[vozhd.index()].locked = true;
    e.state.minions[t.index()].locked = true;
    pick(&mut e, action(a, ActionKind::Bleed));
    let unlock = Choice::UseAbility {
        source: AbilitySource::Card(id),
        ability: 2,
        actor: None,
        target: Target::Minion(t),
    };
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert!(req.options.contains(&unlock), "{req:?}");
    pick(&mut e, unlock);
    assert!(!e.state.minions[vozhd.index()].locked);
    assert_eq!(e.state.minions[t.index()].blood, 3);
    let req = request(&mut e);
    assert!(
        req.options
            .contains(&Choice::DeclareBlock { minion: vozhd }),
        "{req:?}"
    );
}

#[test]
fn vozhd_of_szczecin_discards_a_protean_card_to_prevent_2_once_each_combat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let t = e.add_minion(SeatId(1), plain("Tzimisce", 0), 4, false);
    retainer(&mut e, "Revenant", t, 0);
    let vozhd = e.add_ally(SeatId(1), find_library("Vozhd of Szczecin, The"));
    request(&mut e);
    let id = e.state.minions[vozhd.index()].ally_id().unwrap();
    hand(&mut e, 1, &["Form of Mist", "Form of Mist"]);
    hand(&mut e, 0, &["Roundhouse"]);
    let _ = t;
    pick(&mut e, action(a, ActionKind::Bleed));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::DeclareBlock { minion: vozhd });
    let req = until(&mut e, SeatId(1), WindowLabel::CombatPrevention);
    assert!(req.options.contains(&ability_of(id, 1, None)), "{req:?}");
    pick(&mut e, ability_of(id, 1, None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].hand.len(), 1, "one Form of Mist discarded");
}

// Seraphina "can add 2 blood or life to a minion you control …, not to
// exceed starting life": an ally stops at its starting life.
#[test]
fn seraphina_adds_life_to_an_ally_up_to_its_starting_life() {
    let mut e = Engine::bare(3);
    let s = e.add_minion(SeatId(0), find_card("Seraphina"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let deuce = e.add_ally(SeatId(0), find_library("Double Deuce"));
    e.state.minions[deuce.index()].blood = 2;
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Vampire(s),
            ability: 0,
            actor: Some(s),
            target: Target::Minion(deuce),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[deuce.index()].blood,
        3,
        "its 3 starting life"
    );
}

// Rowena "can strike: combat ends in combat with a werewolf" (Double Deuce).
#[test]
fn rowena_strikes_combat_ends_against_a_werewolf_only() {
    let mut e = Engine::bare(3);
    let rowena = e.add_minion(SeatId(0), find_card("Rowena"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let deuce = e.add_ally(SeatId(1), find_library("Double Deuce"));
    let ends = Choice::UseAbility {
        source: AbilitySource::Vampire(rowena),
        ability: 1,
        actor: None,
        target: Target::None,
    };
    pick(&mut e, action(rowena, ActionKind::Bleed));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::DeclareBlock { minion: deuce });
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&ends), "{req:?}");
    pick(&mut e, ends);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[rowena.index()].blood, 4, "no damage");

    // Against a vampire: no such strike.
    let mut e = Engine::bare(3);
    let rowena = e.add_minion(SeatId(0), find_card("Rowena"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    pick(&mut e, action(rowena, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        assert!(!req.options.contains(&ends), "{req:?}");
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
            .unwrap_or(0);
        e.choose(i);
    }
}

fn play_at(card: u8, mode: u8, actor: MinionId, target: Target) -> Choice {
    Choice::Play {
        card,
        mode,
        actor: Some(actor),
        target,
    }
}

#[test]
fn entrancement_steals_an_ally_with_its_retainers_directed_at_its_controller() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), with_discipline("PRE", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let pa = e.add_ally(SeatId(2), find_library("Political Ally"));
    let winthrop = retainer(&mut e, "Mr. Winthrop", pa, 0);
    // Seat 1's intercept: only the target Methuselah may block.
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    retainer(&mut e, "Mr. Winthrop", z, 0);
    hand(&mut e, 0, &["Entrancement"]);
    pick(&mut e, play_at(0, 1, v, Target::Minion(pa)));
    // Seat 2, not seat 1, holds the block window.
    let req = request(&mut e);
    assert_eq!(
        (req.seat, req.window),
        (SeatId(2), WindowLabel::ActionBlock)
    );
    assert_eq!(
        e.action().and_then(|a| a.directed_seat(&e.state)),
        Some(SeatId(2))
    );
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        assert_ne!(
            (req.seat, req.window),
            (SeatId(1), WindowLabel::ActionBlock)
        );
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass))
            .unwrap_or(0);
        e.choose(i);
    }
    assert_eq!(e.state.minions[pa.index()].owner, SeatId(0));
    assert_eq!(e.state.card_controller(winthrop), SeatId(0));
    let id = e.state.minions[pa.index()].ally_id().unwrap();
    assert_eq!(e.state.card_controller(id), SeatId(0));
    assert_eq!(e.state.seats[0].ash, vec![find_library("Entrancement")]);
}

#[test]
fn entrancement_bleeds_at_plus_1_and_not_after_a_bleed() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), with_discipline("pre", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    hand(&mut e, 0, &["Entrancement", "Entrancement"]);
    let pool = e.state.seats[1].pool;
    pick(&mut e, play_mode(0, 0, v));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, pool - 2);
    e.state.minions[v.index()].locked = false;
    let _ = req;
    let req = request(&mut e);
    assert!(
        !req.options.contains(&play_mode(0, 0, v)),
        "one bleed a turn"
    );
}

// [TOM 19960226-1] "Requirements do not apply when taking control of a
// card already in play": a Revenant (requires a Tzimisce) goes to any
// vampire.
#[test]
fn far_mastery_steals_a_retainer_onto_the_acting_vampire_or_an_ally() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), with_discipline("DOM", 0), 4, false);
    assert_ne!(CRYPT[with_discipline("DOM", 0) as usize].clan, "Tzimisce");
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Tzimisce", 1), 4, false);
    let spy = retainer(&mut e, "Revenant", z, 0);
    let pa = e.add_ally(SeatId(1), find_library("Political Ally"));
    hand(&mut e, 0, &["Far Mastery", "Far Mastery"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&play_at(0, 1, v, Target::Minion(pa))));
    let pool = e.state.seats[0].pool;
    pick(&mut e, play_at(0, 0, v, Target::Card(spy)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.cards[spy.index()].host, Some(v));
    assert_eq!(e.state.card_controller(spy), SeatId(0));
    assert_eq!(e.state.minions[v.index()].blood, 3, "1 blood");
    assert_eq!(e.state.seats[0].pool, pool);
}

#[test]
fn aggressive_corpse_is_no_target_of_directed_dominate_or_presence_actions() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), with_discipline("DOM", 0), 4, false);
    let w = e.add_minion(SeatId(0), with_discipline("PRE", 0), 4, false);
    let corpse = e.add_ally(SeatId(1), find_library("Aggressive Corpse"));
    let pa = e.add_ally(SeatId(1), find_library("Political Ally"));
    hand(&mut e, 0, &["Far Mastery", "Entrancement"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&play_at(0, 1, v, Target::Minion(pa))));
    assert!(!req
        .options
        .contains(&play_at(0, 1, v, Target::Minion(corpse))));
    assert!(req.options.contains(&play_at(1, 1, w, Target::Minion(pa))));
    assert!(!req
        .options
        .contains(&play_at(1, 1, w, Target::Minion(corpse))));
}

// [LSJ 20030902-2] [LSJ 20060808-1] A strike that cannot be dodged "does not
// prevent the opponent from dodging, the dodge just has no effect."
#[test]
fn aggressive_corpse_strikes_through_a_dodge_and_gains_no_life() {
    let mut e = Engine::bare(3);
    let z = e.add_minion(SeatId(0), with_discipline("cel", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let corpse = e.add_ally(SeatId(1), find_library("Aggressive Corpse"));
    let mystic = e.add_ally(SeatId(1), find_library("Vagabond Mystic"));
    let mystic_id = e.state.minions[mystic.index()].ally_id().unwrap();
    hand(&mut e, 0, &["Side Strike"]);
    pick(&mut e, action(z, ActionKind::Bleed));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::DeclareBlock { minion: corpse });
    until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    pick(&mut e, play_mode(0, 0, z)); // dodge
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[z.index()].blood,
        2,
        "2 strength through the dodge"
    );
    assert_eq!(
        e.state.minions[corpse.index()].blood,
        3,
        "Z's dodge: no hit"
    );
    // Hurt, it gains nothing.
    e.state.minions[corpse.index()].blood = 1;
    pick(&mut e, Choice::Pass);
    let heal = Choice::UseAbility {
        source: AbilitySource::Card(mystic_id),
        ability: 0,
        actor: None,
        target: Target::Minion(corpse),
    };
    let req = until(&mut e, SeatId(1), WindowLabel::MinionPhase);
    assert!(req.options.contains(&heal), "{req:?}");
    pick(&mut e, heal);
    assert_eq!(e.state.minions[corpse.index()].blood, 1);
    assert!(e.state.minions[mystic.index()].locked);
}

// [PIB 20121031] War Ghoul may lock and burn itself during an action that
// would steal it: "the action will have no effect but is still successful
// if unblocked and must then be paid". [LSJ 20020725] A target no longer
// legal: "the action fizzles (ends successfully with no effect)".
#[test]
fn entrancement_steal_of_an_ally_gone_before_resolution_does_nothing() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), with_discipline("PRE", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Tzimisce", 0), 4, false);
    retainer(&mut e, "Revenant", t, 0);
    let ghoul = e.add_ally(SeatId(1), find_library("War Ghoul"));
    request(&mut e); // the entry burn: the Revenant
    let id = e.state.minions[ghoul.index()].ally_id().unwrap();
    e.state.cards.push(CardInPlay {
        card: find_library("Academic Hunting Ground"),
        owner: SeatId(2),
        controller: SeatId(2),
        host: None,
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
    });
    let hg = CardId(e.state.cards.len() as u32 - 1);
    hand(&mut e, 0, &["Entrancement"]);
    pick(&mut e, play_at(0, 1, v, Target::Minion(ghoul)));
    let burn = Choice::UseAbility {
        source: AbilitySource::Card(id),
        ability: 2,
        actor: None,
        target: Target::Card(hg),
    };
    let req = until(&mut e, SeatId(1), WindowLabel::ActionAsAnnounced);
    assert!(req.options.contains(&burn), "{req:?}");
    pick(&mut e, burn);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[ghoul.index()].in_play);
    assert_eq!(e.state.minions[ghoul.index()].owner, SeatId(1));
    // A target no longer legal: the action fizzles, "ends successfully with
    // no effect" [LSJ 20020725].
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Fizzled));
    assert_eq!(e.state.seats[0].ash, vec![find_library("Entrancement")]);
}

// Rulebook "Retainers and Combat": at long range a ranged strike may be
// aimed at one of the opposing minion's retainers, announced with it; a
// dodge would not protect the retainer.
#[test]
fn a_ranged_strike_at_long_range_may_be_aimed_at_a_retainer() {
    let mut e = Engine::bare(3);
    let z = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let spy = retainer(&mut e, "Raven Spy", z, 0);
    let taxi = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    let id = e.state.minions[taxi.index()].ally_id().unwrap();
    pick(&mut e, action(z, ActionKind::Bleed));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::DeclareBlock { minion: taxi });
    until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    pick(&mut e, ability_of(id, 1, None)); // to long range
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    let aimed = Choice::UseAbility {
        source: AbilitySource::Card(id),
        ability: 0,
        actor: None,
        target: Target::Card(spy),
    };
    assert!(req.options.contains(&aimed), "{req:?}");
    pick(&mut e, aimed);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[spy.index()].in_play, "its 1 life burned");
    assert_eq!(e.state.minions[z.index()].blood, 4, "Z unhurt");
}

// ---- Allies cannot block ----------------------------------------------

fn blocks(req: &DecisionRequest) -> Vec<MinionId> {
    req.options
        .iter()
        .filter_map(|o| match *o {
            Choice::DeclareBlock { minion } => Some(minion),
            _ => None,
        })
        .collect()
}

#[test]
fn visions_of_gehenna_keeps_allies_from_blocking_the_action() {
    let mut e = Engine::bare(3);
    let r = e.add_minion(SeatId(0), of_clan_with("Ravnos", "pre", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    hand(&mut e, 0, &["Visions of Gehenna"]);
    pick(&mut e, action(r, ActionKind::Bleed));
    pick(&mut e, play_mode(0, 0, r));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(blocks(&req), vec![z]);
    let _ = ally;
}

// [LSJ 20011204-3] [PIB 20130704] Played, a multi-Discipline card counts
// as requiring the Discipline used: each mode wants its own.
#[test]
fn invigorate_gives_strength_bars_allies_or_aggravates_hand_strikes() {
    // [dom]: no ally block.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch_with("dom"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.add_ally(SeatId(1), find_library("Political Ally"));
    hand(&mut e, 0, &["Invigorate"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, play_mode(0, 1, a));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(blocks(&req), vec![z]);

    // [ani]: +1 strength this action.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch_with("ani"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Invigorate"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, play_mode(0, 0, a));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::DeclareBlock { minion: z });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[z.index()].blood, 2);

    // [pro]: an aggravated hand strike wounds even a vampire with blood.
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch_with("pro"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    hand(&mut e, 0, &["Invigorate"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, play_mode(0, 2, a));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::DeclareBlock { minion: z });
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let v = &e.state.minions[z.index()];
    assert!(
        v.in_torpor && v.blood == 4,
        "aggravated: wounded, blood kept"
    );
}

#[test]
fn libertas_bars_allies_and_taxes_dominate_and_presence_around_its_anarch() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), anarch(0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), with_discipline("pre", 0), 1, false);
    e.add_ally(SeatId(1), find_library("Political Ally"));
    e.state.cards.push(CardInPlay {
        card: find_library("Libertas"),
        owner: SeatId(0),
        controller: SeatId(0),
        host: Some(a),
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
    });
    let id = CardId(e.state.cards.len() as u32 - 1);
    for m in [
        cards::Modifier::AlliesCannotBlockBearer,
        cards::Modifier::DisciplineSurchargeAround(&["dom", "pre"], 1),
    ] {
        e.state.layers.push(Layer {
            scope: Scope::InPlay,
            modifier: m,
            source: id,
        });
    }
    hand(&mut e, 1, &["Majesty"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(blocks(&req), vec![z]);
    pick(&mut e, Choice::DeclareBlock { minion: z });
    // Majesty costs Z 2 blood beside the Anarch: with 1, not offered.
    loop {
        let req = request(&mut e);
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        assert!(!req.options.contains(&play_mode(0, 0, z)), "{req:?}");
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
            .unwrap_or(0);
        e.choose(i);
    }
}

/// An untitled Anarch with the Discipline `tag`, no printed text.
fn anarch_with(tag: &str) -> u16 {
    CRYPT
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            c.sect == zvtes::crypt::Sect::Anarch
                && c.disciplines.contains(&tag)
                && c.title == Title::None
                && c.capacity >= 4
                && cards::vampire(*i as u16).is_none()
        })
        .map(|(i, _)| i as u16)
        .next()
        .expect("a plain Anarch")
}

// [LSJ 20080818] Libertas' "additional cost does not apply once the minion
// has failed to block, so it does not apply to the cost of the action
// card".
#[test]
fn libertas_does_not_tax_the_action_card_once_its_anarchs_block_failed() {
    let mut e = Engine::bare(3);
    let abraham = e.add_minion(SeatId(0), find_card("Abraham DuSable"), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let a = e.add_minion(SeatId(1), anarch(0), 4, false);
    retainer(&mut e, "Mr. Winthrop", a, 0);
    let ally = e.add_ally(SeatId(1), find_library("Political Ally"));
    e.state.cards.push(CardInPlay {
        card: find_library("Libertas"),
        owner: SeatId(1),
        controller: SeatId(1),
        host: Some(a),
        body: None,
        mode: 0,
        by: None,
        in_play: true,
        contested: false,
        locked: false,
        counters: 0,
        stored: Vec::new(),
    });
    let id = CardId(e.state.cards.len() as u32 - 1);
    e.state.layers.push(Layer {
        scope: Scope::InPlay,
        modifier: cards::Modifier::DisciplineSurchargeAround(&["dom", "pre"], 1),
        source: id,
    });
    let tha = zvtes::library::LIBRARY
        .iter()
        .position(|c| c.disciplines == ["tha"])
        .unwrap() as u16;
    e.state.seats[0].hand = vec![find_library("Far Mastery"), tha];
    pick(&mut e, play_at(0, 1, abraham, Target::Minion(ally)));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::DeclareBlock { minion: a });
    // Abraham discards a Blood Sorcery card for +1 stealth: the attempt fails.
    let req = request(&mut e);
    let stealth = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::UseAbility { source: AbilitySource::Vampire(m), .. } if *m == abraham))
        .expect("Abraham's stealth");
    pick(&mut e, stealth);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.minions[ally.index()].owner, SeatId(0), "stolen");
    assert_eq!(
        e.state.minions[abraham.index()].blood,
        3,
        "1 blood, untaxed"
    );
}

#[test]
fn slaughtering_the_herd_bleeds_at_plus_2_and_allies_cannot_block() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), with_discipline("dom", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let z = e.add_minion(SeatId(1), plain("Brujah", 1), 4, false);
    e.add_ally(SeatId(1), find_library("Political Ally"));
    hand(&mut e, 0, &["Slaughtering the Herd"]);
    let pool = e.state.seats[1].pool;
    pick(&mut e, play_mode(0, 0, v));
    let req = until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    assert_eq!(blocks(&req), vec![z]);
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(e.state.seats[1].pool, pool - 3);
    assert_eq!(e.state.minions[v.index()].blood, 2, "2 blood");
}

/// Seat 0's DOM vampire has put Slaughtering the Herd on its predator's
/// (seat 2's) vampire: returns (engine, slaughterer, attached, card).
fn slaughtered() -> (Engine, MinionId, MinionId, CardId) {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), with_discipline("DOM", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let p = e.add_minion(SeatId(2), plain("Ventrue", 0), 5, false);
    e.add_minion(SeatId(2), plain("Ventrue", 1), 4, false);
    hand(&mut e, 0, &["Slaughtering the Herd"]);
    pick(&mut e, play_at(0, 1, v, Target::Minion(p)));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let card = (0..e.state.cards.len())
        .map(|i| CardId(i as u32))
        .find(|c| e.state.cards[c.index()].card == find_library("Slaughtering the Herd"))
        .expect("in play");
    assert_eq!(e.state.cards[card.index()].host, Some(p));
    (e, v, p, card)
}

#[test]
fn slaughtering_the_herd_drains_a_blood_each_action_of_its_host() {
    let (mut e, v, p, _) = slaughtered();
    assert_eq!(e.state.minions[v.index()].blood, 2, "2 blood paid");
    pick(&mut e, Choice::Pass);
    until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    pick(&mut e, action(p, ActionKind::Hunt));
    assert_eq!(e.state.minions[p.index()].blood, 4);
    assert_eq!(e.state.minions[v.index()].blood, 3);
}

#[test]
fn slaughtering_the_herds_host_burns_4_blood_in_its_minion_phase_to_burn_it() {
    let (mut e, _, p, card) = slaughtered();
    let burn = Choice::UseAbility {
        source: AbilitySource::Card(card),
        ability: 0,
        actor: None,
        target: Target::None,
    };
    pick(&mut e, Choice::Pass);
    let req = until(&mut e, SeatId(2), WindowLabel::MinionPhase);
    assert!(req.options.contains(&burn), "{req:?}");
    pick(&mut e, burn);
    assert!(!e.state.cards[card.index()].in_play);
    assert_eq!(e.state.minions[p.index()].blood, 1);
}

#[test]
fn slaughtering_the_herd_burns_once_its_vampire_leaves_the_ready_region() {
    let (mut e, v, _, card) = slaughtered();
    // Next turn, V (2 blood) bleeds; a strong blocker sends it to torpor.
    e.state.minions[v.index()].blood = 1;
    let big = e.add_minion(SeatId(1), plain("Brujah", 2), 4, false);
    hand(&mut e, 1, &["Roundhouse"]);
    let _ = big;
    e.state.minions[v.index()].locked = false;
    pick(&mut e, action(v, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: big });
    let req = until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    let rh = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::Play { .. }))
        .expect("Roundhouse");
    pick(&mut e, rh);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.minions[v.index()].in_torpor);
    assert!(!e.state.cards[card.index()].in_play);
}

// [LSJ 20030701-2] Rotting Behemoth's controller "can choose not to remove
// a minion from his or her ash heap and thus burn the Rotting Behemoth".
// Its entry effect applies however it enters [ANK 20170309-2] (here put in
// play, not recruited).
#[test]
fn rotting_behemoth_removes_an_ally_or_vampire_from_the_ash_heap_or_burns() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Hecata", 0), 4, false);
    let vamp = plain("Brujah", 0);
    e.state.seats[0].ash = vec![find_library("Political Ally"), find_library("Majesty")];
    e.state.seats[0].ash_crypt = vec![vamp];
    let rb = e.add_ally(SeatId(0), find_library("Rotting Behemoth"));
    let req = expect(&mut e, SeatId(0), WindowLabel::EntryEffect);
    assert_eq!(
        req.options,
        vec![
            Choice::Pass,
            Choice::FindCard(find_library("Political Ally")),
            Choice::AshVampire(vamp),
        ]
    );
    pick(&mut e, Choice::AshVampire(vamp));
    request(&mut e);
    assert!(e.state.minions[rb.index()].in_play);
    assert!(e.state.seats[0].ash_crypt.is_empty());
    assert_eq!(e.state.seats[0].removed_crypt, vec![vamp]);

    // Declined: burned.
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Hecata", 0), 4, false);
    e.state.seats[0].ash = vec![find_library("Political Ally")];
    let rb = e.add_ally(SeatId(0), find_library("Rotting Behemoth"));
    pick(&mut e, Choice::Pass);
    request(&mut e);
    assert!(!e.state.minions[rb.index()].in_play);
}

#[test]
fn a_burned_vampire_goes_to_the_ash_heap() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, true);
    pick(
        &mut e,
        Choice::TakeAction {
            minion: a,
            action: ActionKind::Diablerie,
            target: Some(z),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[z.index()].in_play);
    assert_eq!(e.state.seats[1].ash_crypt, vec![plain("Ventrue", 0)]);
}

// [LSJ 20080512] "When paying their last life for the action, the action
// is still considered successful, but combat immediately ends."
#[test]
fn rotting_behemoth_paying_its_last_life_for_its_rush_burns_without_combat() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Hecata", 0), 4, false);
    e.state.seats[0].ash = vec![find_library("Political Ally")];
    let rb = e.add_ally(SeatId(0), find_library("Rotting Behemoth"));
    pick(&mut e, Choice::FindCard(find_library("Political Ally")));
    let id = e.state.minions[rb.index()].ally_id().unwrap();
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    e.state.minions[rb.index()].blood = 1;
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(id),
            ability: 0,
            actor: Some(rb),
            target: Target::Minion(z),
        },
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[rb.index()].in_play);
    assert_eq!(e.state.minions[z.index()].blood, 4, "no combat");
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
}

#[test]
fn rotting_behemoth_superior_burns_life_for_a_press_in_the_first_round() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Hecata", 0), 4, false);
    e.state.seats[0].ash = vec![find_library("Political Ally")];
    let rb = e.add_ally_version(SeatId(0), find_library("Rotting Behemoth"), 1);
    pick(&mut e, Choice::FindCard(find_library("Political Ally")));
    let id = e.state.minions[rb.index()].ally_id().unwrap();
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 8, false);
    pick(
        &mut e,
        Choice::UseAbility {
            source: AbilitySource::Card(id),
            ability: 0,
            actor: Some(rb),
            target: Target::Minion(z),
        },
    );
    let req = until(&mut e, SeatId(0), WindowLabel::CombatPress);
    assert!(req.options.contains(&ability_of(id, 1, None)), "{req:?}");
    pick(&mut e, ability_of(id, 1, None));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    // 6 life, 1 for the rush, 1 for the press, 1 per round from Z.
    assert_eq!(e.state.minions[rb.index()].blood, 2);
}

#[test]
fn vozhd_of_gravesend_burns_life_to_cancel_an_opposing_strike_card() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), with_discipline("pot", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let t = e.add_minion(SeatId(1), plain("Tzimisce", 0), 4, false);
    retainer(&mut e, "Revenant", t, 0);
    let vozhd = e.add_ally(SeatId(1), find_library("Vozhd of Gravesend, The"));
    request(&mut e); // the entry burn: the Revenant
    let id = e.state.minions[vozhd.index()].ally_id().unwrap();
    hand(&mut e, 0, &["Roundhouse"]);
    pick(&mut e, action(a, ActionKind::Bleed));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::DeclareBlock { minion: vozhd });
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    let rh = req
        .options
        .iter()
        .copied()
        .find(|o| matches!(o, Choice::Play { .. }))
        .expect("Roundhouse");
    pick(&mut e, rh);
    let req = expect(&mut e, SeatId(1), WindowLabel::CardAsPlayed);
    assert!(req.options.contains(&ability_of(id, 2, None)), "{req:?}");
    pick(&mut e, ability_of(id, 2, None));
    // Canceled, unpaid: A chooses a strike again (its hand strike, elided).
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(e.state.seats[0].ash.contains(&find_library("Roundhouse")));
    assert_eq!(
        e.state.minions[a.index()].blood,
        0,
        "4 strength, none prevented"
    );
    // 5 life: 1 for the cancel, 1 from A's hand strike.
    assert_eq!(e.state.minions[vozhd.index()].blood, 3);
}

// ---- Allies playing cards "as a vampire" ------------------------------

/// Seat 0's Vozhd of Juiz de Fora (a Tzimisce beside it, whose Revenant
/// took the entry burn).
fn juiz() -> (Engine, MinionId, MinionId, CardId) {
    let mut e = Engine::bare(3);
    let t = e.add_minion(SeatId(0), plain("Tzimisce", 0), 4, false);
    retainer(&mut e, "Revenant", t, 0);
    let vozhd = e.add_ally(SeatId(0), find_library("Vozhd of Juiz de Fora, The"));
    request(&mut e);
    let id = e.state.minions[vozhd.index()].ally_id().unwrap();
    (e, t, vozhd, id)
}

// [LSJ 20011217] [LSJ 20090508] Meeting a Discipline requirement as a
// vampire meets one requirement, at basic level [ANK 20180111].
#[test]
fn vozhd_of_juiz_de_fora_employs_an_animalism_retainer_as_a_vampire_paying_life() {
    let (mut e, _, vozhd, _) = juiz();
    hand(&mut e, 0, &["Raven Spy", "Feral Hound"]);
    let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(req.options.contains(&play_mode(0, 0, vozhd)));
    assert!(
        !req.options.contains(&play_mode(0, 1, vozhd)),
        "[ANI] needs superior"
    );
    assert!(
        !req.options.contains(&play_mode(1, 0, vozhd)),
        "Feral Hound needs a Ravnos"
    );
    pick(&mut e, play_mode(0, 0, vozhd));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert_eq!(
        e.state.minions[vozhd.index()].blood,
        4,
        "1 blood paid in life"
    );
    let spy = find_library("Raven Spy");
    assert!(retainer_on(&e, spy, vozhd).is_some());
}

// [LSJ 20080512] "If they use their last life to pay for an action, the
// action still resolves fully."
#[test]
fn an_ally_paying_its_last_life_as_a_vampire_resolves_the_action_then_burns() {
    let (mut e, _, vozhd, _) = juiz();
    e.state.minions[vozhd.index()].blood = 1;
    hand(&mut e, 0, &["Raven Spy"]);
    pick(&mut e, play_mode(0, 0, vozhd));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[vozhd.index()].in_play);
    assert!(
        e.state.seats[0].ash.contains(&find_library("Raven Spy")),
        "burned with it"
    );
    assert_eq!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
}

#[test]
fn vozhd_of_juiz_de_fora_takes_a_tzimisces_blood_as_life_up_to_its_starting_life() {
    let (mut e, t, vozhd, id) = juiz();
    e.state.minions[vozhd.index()].blood = 3;
    pick(&mut e, Choice::Pass);
    let steal = Choice::UseAbility {
        source: AbilitySource::Card(id),
        ability: 1,
        actor: None,
        target: Target::Minion(t),
    };
    let req = until(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
    assert!(req.options.contains(&steal), "{req:?}");
    pick(&mut e, steal);
    let req = request(&mut e);
    assert_eq!(req.options, vec![Choice::SetX(1), Choice::SetX(2)]);
    pick(&mut e, Choice::SetX(2));
    assert_eq!(e.state.minions[vozhd.index()].blood, 5);
    assert_eq!(e.state.minions[t.index()].blood, 2);
}

// [ANK 20200930] [ANK 20200813-3] Spectral Servitor acts the turn it is
// recruited, in its Methuselah's minion phase.
#[test]
fn spectral_servitor_acts_at_once_unlocks_its_recruiter_and_plays_oblivion_modifiers() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), with_discipline("obl", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    hand(
        &mut e,
        0,
        &["Spectral Servitor", "Consign to Oblivion", "Bone Shambler"],
    );
    pick(&mut e, play_mode(0, 0, v));
    let req = until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    let servitor = MinionId(e.state.minions.len() as u32 - 1);
    assert!(
        !e.state.minions[v.index()].locked,
        "its first recruit this turn"
    );
    assert!(req.options.contains(&action(servitor, ActionKind::Bleed)));
    // Recruiting a Bone Shambler [obl] is an action: not as a vampire.
    let shambler = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Bone Shambler"))
        .unwrap() as u8;
    assert!(!req.options.contains(&play_mode(shambler, 0, servitor)));
    // Its bleed: it may play Consign to Oblivion as a vampire.
    pick(&mut e, action(servitor, ActionKind::Bleed));
    let req = expect(&mut e, SeatId(0), WindowLabel::ActionAsAnnounced);
    let consign = e.state.seats[0]
        .hand
        .iter()
        .position(|&c| c == find_library("Consign to Oblivion"))
        .unwrap() as u8;
    assert!(
        req.options.contains(&play_mode(consign, 0, servitor)),
        "{req:?}"
    );
}

#[test]
fn spectral_servitor_burns_in_its_unlock_phase_unless_obl_pays_a_pool() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Hecata", 0), 4, false);
    let basic = e.add_ally(SeatId(0), find_library("Spectral Servitor"));
    let sup = e.add_ally_version(SeatId(0), find_library("Spectral Servitor"), 1);
    let sup_id = e.state.minions[sup.index()].ally_id().unwrap();
    let pool = e.state.seats[0].pool;
    pick(&mut e, Choice::Pass);
    loop {
        let req = request(&mut e);
        if req.window == WindowLabel::RiderTerms {
            assert_eq!(
                req.options,
                vec![Choice::BurnPool, Choice::BurnInPlay { card: sup_id.0 }]
            );
            pick(&mut e, Choice::BurnPool);
            continue;
        }
        if (req.seat, req.window) == (SeatId(0), WindowLabel::MinionPhase) {
            break;
        }
        let i = req
            .options
            .iter()
            .position(|o| matches!(o, Choice::Pass | Choice::ResolveRider { .. }))
            .unwrap_or(0);
        e.choose(i);
    }
    assert!(!e.state.minions[basic.index()].in_play);
    assert!(e.state.minions[sup.index()].in_play);
    assert_eq!(e.state.seats[0].pool, pool - 1);
}

// [ANK 20180928-1] "If they use their last life to play an action
// modifier, the action fails."
#[test]
fn spectral_servitor_paying_its_last_life_for_a_modifier_burns_and_the_action_fails() {
    let mut e = Engine::bare(3);
    e.add_minion(SeatId(0), plain("Hecata", 0), 4, false);
    let servitor = e.add_ally(SeatId(0), find_library("Spectral Servitor"));
    hand(&mut e, 0, &["Consign to Oblivion"]);
    let pool = e.state.seats[1].pool;
    pick(&mut e, action(servitor, ActionKind::Bleed));
    pick(&mut e, play_mode(0, 0, servitor));
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.minions[servitor.index()].in_play);
    assert_eq!(e.state.seats[1].pool, pool, "no bleed");
    assert_ne!(e.last_outcome, Some(zvtes::actions::Outcome::Successful));
}

// [LSJ 19980303] "While a minion has an effect that allows them to play
// cards that require Discipline(s), they do not actually have those
// Discipline(s)."
#[test]
fn an_ally_playing_as_a_vampire_does_not_have_the_discipline() {
    let (e, _, vozhd, _) = juiz();
    assert!(!cards::holds_for_host(
        cards::Requirement::HostDiscipline("ani"),
        &e.state,
        vozhd
    ));
}

#[test]
fn owl_companion_opens_the_opposing_controllers_hand_during_the_combat() {
    let mut e = Engine::bare(3);
    let a = e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    e.add_minion(SeatId(0), plain("Brujah", 1), 4, false);
    let z = e.add_minion(SeatId(1), plain("Ventrue", 0), 4, false);
    retainer(&mut e, "Owl Companion", a, 0);
    hand(&mut e, 1, &["Majesty"]);
    assert!(!e.state.hand_visible_to(SeatId(1), SeatId(0)));
    pick(&mut e, action(a, ActionKind::Bleed));
    pick(&mut e, Choice::DeclareBlock { minion: z });
    until(&mut e, SeatId(1), WindowLabel::CombatStrike);
    assert!(e.state.hand_visible_to(SeatId(1), SeatId(0)));
    assert!(e.state.hand_visible_to(SeatId(1), SeatId(2)), "open to all");
    assert!(
        !e.state.hand_visible_to(SeatId(0), SeatId(1)),
        "not the employer's"
    );
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.hand_visible_to(SeatId(1), SeatId(0)));
}

// A strike card from hand may be aimed at a retainer too; life stolen from
// it becomes the striker's blood (rulebook "Steal Blood").
#[test]
fn a_ranged_strike_card_may_be_aimed_at_a_retainer_and_steals_its_life() {
    let mut e = Engine::bare(3);
    let v = e.add_minion(SeatId(0), with_discipline("tha", 0), 2, false);
    e.add_minion(SeatId(0), plain("Brujah", 0), 4, false);
    let taxi = e.add_ally(SeatId(1), find_library("City Star Taxi"));
    let taxi_id = e.state.minions[taxi.index()].ally_id().unwrap();
    let spy = retainer(&mut e, "Raven Spy", taxi, 0);
    hand(&mut e, 0, &["Theft of Vitae"]);
    pick(&mut e, action(v, ActionKind::Bleed));
    until(&mut e, SeatId(1), WindowLabel::ActionBlock);
    pick(&mut e, Choice::DeclareBlock { minion: taxi });
    until(&mut e, SeatId(1), WindowLabel::CombatDetermineRange);
    pick(&mut e, ability_of(taxi_id, 1, None)); // to long range
    let aimed = play_at(0, 0, v, Target::Card(spy));
    let req = until(&mut e, SeatId(0), WindowLabel::CombatStrike);
    assert!(req.options.contains(&aimed), "{req:?}");
    pick(&mut e, aimed);
    until(&mut e, SeatId(0), WindowLabel::MinionPhase);
    assert!(!e.state.cards[spy.index()].in_play, "its 1 life stolen");
    assert_eq!(e.state.minions[taxi.index()].blood, 2, "the taxi untouched");
    // 2 blood, +1 stolen (the taxi's hand strike misses at long range).
    assert_eq!(e.state.minions[v.index()].blood, 3);
}
