//! The events a seat is told (docs/protocol.md §9), checked against the
//! views it is sent: each decision's view is the last one with the
//! events since applied (`reduce`); an action is told ended before the
//! next is announced, a combat begun, its strikes, then ended. Every copy
//! an event names is numbered; English only when the settings ask, and
//! then what `zvtes_text` renders from the event's data (the renderer's
//! oracle).

use std::collections::HashMap;

use zvtes_api::event::{Event, EventKind};
use zvtes_api::ids::{MinionKind, OptionKind};
use zvtes_api::spec::Views;
use zvtes_api::view::{Card, TableView};
use zvtes_api::{Command, GameSpec, In, Message, Out, Player, SeatSpec, Settings};
use zvtes_host::{Bots, Clock, Game};
use zvtes_text::Names;

mod reduce;

/// The first seed a sweep plays (`HOST_SEED_FROM`, to look at one).
fn from() -> u32 {
    std::env::var("HOST_SEED_FROM").map_or(1, |s| s.parse().unwrap())
}

/// Seeds past the default a deeper sweep failed on (`HOST_SEEDS=400`), kept: a
/// banished created vampire (70), Mask of a Thousand Faces (132), Victor
/// Pelletier's price (172), the strikes shown after an undo (219), Spiritual
/// Guidance's play from the ash heap (346); and the table kept from the events:
/// a vampire banished and back between two views (45), one influenced into a
/// contest (81), a unique ally played into one (95), a crypt card come with
/// counters (111), a contested minion's cards (118), a merge (119), training
/// counters (152), a vampire moved within its uncontrolled region (342), two
/// Methuselahs ousted at once (239), a unique card put into a contest (299), a
/// vampire's excess drained in one (380), an action's intercept changed between
/// two views (103), a play canceled as it is announced (88: Andrew Stuart), a
/// cost reduced by a lasting effect whose card is in the ash heap (104: Inner
/// Essence); and the causes a deeper sweep found missing: Camarilla Threat's
/// discard toll (294), blood to an uncontrolled vampire (375), a diablerie
/// in a torpor block (411), Yawp Court's yawp (591), a burn to prevent
/// (728), a rider fired as its bearer blocks (913), a diseased vampire's
/// paid infernal unlock (504), a discard-phase rider's damage (579); and
/// the view paths the events missed: an ally stolen for good (99, 300:
/// Entrancement), a contested vampire's capacity (114, 286), Call the Great
/// Beast become a vampire (300), a card burned to an ousted owner's ash heap
/// (474, 933: Disarm), an open hand (1111: Aura Reading), cards into one
/// (106), one closed with a card leaving it in the same step (910, 929).
const KEPT: [u32; 37] = [
    45, 70, 81, 88, 95, 99, 103, 104, 106, 111, 114, 118, 119, 132, 152, 172, 219, 239, 286, 294,
    299, 300, 342, 346, 375, 380, 411, 474, 504, 579, 591, 728, 910, 913, 929, 933, 1111,
];

/// The sweep's seeds: `HOST_SEED_FROM` to `HOST_SEEDS` (40), the kept
/// ones past it, the imbued and the two-player ones.
fn seeds() -> Vec<u32> {
    let to: u32 = std::env::var("HOST_SEEDS").map_or(40, |s| s.parse().unwrap());
    let kept = KEPT.into_iter().filter(|&s| s > to && from() == 1);
    let imbued = IMBUED.into_iter().filter(|_| from() == 1);
    let two = TWO.filter(|_| from() == 1);
    (from()..=to).chain(kept).chain(imbued).chain(two).collect()
}

/// Seeds whose seat after the human's plays the imbued: one
/// incapacitated, burned or recovering between two views.
const IMBUED: [u32; 4] = [1001, 1002, 1003, 1004];

/// Seeds played by two seats (VEKN's two-player variant, its suggested
/// lists), both the same list every other one: crypt cards contested in
/// play.
const TWO: std::ops::RangeInclusive<u32> = 2001..=2040;

/// Every imbued, every power and conviction.
fn imbued() -> zvtes_api::DeckList {
    use zvtes::crypt::CRYPT;
    use zvtes::library::{CardType, LIBRARY};
    let crypt = CRYPT.iter().filter(|c| c.imbued).map(|c| (c.id, 1));
    let library = LIBRARY
        .iter()
        .filter(|c| {
            c.types
                .iter()
                .any(|t| matches!(t, CardType::Power | CardType::Conviction))
        })
        .map(|c| {
            (
                c.id,
                if c.types.contains(&CardType::Conviction) {
                    8
                } else {
                    3
                },
            )
        });
    crypt
        .chain(library)
        .map(|(id, n)| (id.to_string(), n))
        .collect()
}

struct NoBots;

impl Bots for NoBots {
    fn profiles(&self) -> Vec<String> {
        vec![]
    }
    fn make(
        &self,
        _: &str,
        _: u64,
    ) -> Option<(Box<dyn zvtes::agent::Agent>, zvtes_api::record::BotRecord)> {
        None
    }
}

struct Ticks(std::cell::Cell<u64>);

impl Clock for Ticks {
    fn now(&self) -> u64 {
        self.0.set(self.0.get() + 1);
        self.0.get()
    }
}

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n
    }
}

/// The precon with the most political action cards.
fn political(decks: &[(String, zvtes_api::DeckList)]) -> usize {
    use zvtes::library::{CardType, LIBRARY};
    let politics = |list: &zvtes_api::DeckList| -> u32 {
        list.iter()
            .filter(|(id, _)| {
                LIBRARY.iter().any(|c| {
                    c.id.to_string() == **id && c.types.contains(&CardType::PoliticalAction)
                })
            })
            .map(|(_, &n)| n)
            .sum()
    };
    (0..decks.len())
        .max_by_key(|&i| politics(&decks[i].1))
        .unwrap()
}

fn game(seed: u64, settings: Settings) -> Game {
    let n = if TWO.contains(&(seed as u32)) { 2 } else { 4 };
    // Two seats play VEKN's suggested lists, the variant's decks.
    let decks = match n {
        2 => zvtes_host::two_player_catalog(),
        _ => zvtes_host::catalog(),
    };
    let styles = ["aggro_bleed", "wall", "random"];
    let human = seed as usize % n;
    let spec = GameSpec {
        seed: Some(seed),
        time_limit: Some(40),
        client: Some("tests".into()),
        seats: (0..n)
            .map(|i| {
                // Every other game, the human plays the most political
                // precon: referendums, their allocations and votes.
                let pick = if n == 2 && seed.is_multiple_of(2) {
                    seed as usize % decks.len()
                } else if i == human && seed.is_multiple_of(2) {
                    political(&decks)
                } else {
                    (seed as usize + i * 5) % decks.len()
                };
                let (name, list) = if IMBUED.contains(&(seed as u32)) && i == (human + 1) % n {
                    ("Imbued".to_string(), imbued())
                } else {
                    decks[pick].clone()
                };
                SeatSpec {
                    name,
                    deck: list,
                    player: if i == human {
                        Player::Human { settings }
                    } else {
                        Player::Bot {
                            profile: styles[(seed as usize + i) % 3].into(),
                        }
                    },
                }
            })
            .collect(),
    };
    Game::new(
        format!("g{seed}"),
        spec,
        &NoBots,
        Box::new(Ticks(Default::default())),
    )
    .unwrap()
}

/// What the events told since the last view.
struct Stream {
    seed: u32,
    text: bool,
    /// A combat told begun, not yet told ended; unknown after an undo.
    in_combat: Option<bool>,
    told: usize,
    /// The plays told resolved and canceled.
    played: [usize; 2],
    /// The bleeds told started or ended, each with its target.
    bleeds: usize,
    /// Views showing a minion contested in play (two seats).
    contested: usize,
    texts: usize,
    names: Names<'static>,
    /// Each decision's view as first sent, and the decision an undo
    /// asks again: its view must be the same.
    asked: HashMap<u64, TableView>,
    again: Option<u64>,
    reasked: usize,
    /// Decisions an undo went back to that were never asked.
    unasked: Vec<u64>,
    /// The legacy lines the renderer told otherwise: (legacy, rendered).
    misses: Vec<(String, String)>,
    /// Each minion's `numbered`, as the views and its entering or leaving
    /// play told it (forgotten at an undo: it takes back those given
    /// since).
    name_numbers: HashMap<u32, Option<u8>>,
    /// The action told announced and not yet ended (seat, acting), by the
    /// events alone; unknown after an undo, until a view shows the table.
    open: Option<Option<(u8, u32)>>,
    /// The seat of the last decision, until the action it announced or
    /// the next turn.
    decided: Option<u8>,
    /// Actions announced and ended within one message, and announced with
    /// no decision before (a forced action).
    within: usize,
    undecided: usize,
    started: bool,
    /// The turn's active seat, as its `turn` told it.
    active: Option<u8>,
    /// The acting minion of the action ended last, until the next one,
    /// decision or turn: none of its effects told after its end.
    closed: Option<Closed>,
    /// Pool and blood events, each with its cause.
    caused: usize,
    /// The source and purpose each decision asked was told with.
    sources: HashMap<
        u64,
        (
            Option<zvtes_api::option::CardRef>,
            Option<zvtes_api::ids::Purpose>,
        ),
    >,
    /// Markers the views showed, and combats a decision's view did.
    markers: usize,
    combats: usize,
    /// Cards told moving, among them as backs, and within an uncontrolled
    /// region (a reshuffle).
    moved: [usize; 3],
    /// Each copy's KRCG id, as the messages first showed it: an undo
    /// forgets none (a number never names another card).
    faces: HashMap<u32, u64>,
    /// Messages seen; each copy's first message; each decision's message,
    /// as last asked (what an undo to it restores); the copies first sent
    /// after a decision an undo went back to, never sent again (§3: never
    /// given twice).
    seq: usize,
    born: HashMap<u32, usize>,
    asked_at: HashMap<u64, usize>,
    dead: std::collections::HashSet<u32>,
    /// The viewing seat, and the copies told played into limbo (since an
    /// undo).
    viewer: Option<u8>,
    played_in: std::collections::HashSet<u32>,
    /// The phase the events told last (a turn's: its unlock phase) and
    /// the transfers left (unknown after an undo).
    phase: Option<zvtes_api::ids::Phase>,
    transfers: Option<u8>,
    /// The last decision's view.
    prev: Option<TableView>,
    /// The table kept from the events since the last decision's view
    /// (none after an undo, until a view shows the table), the paths it
    /// missed with an example, and the places in another order.
    table: Option<reduce::Table>,
    unreduced: std::collections::BTreeMap<String, (usize, String)>,
    unordered: std::collections::BTreeMap<&'static str, usize>,
    reduced: usize,
    /// Cards told moving into a hand open to the viewer (`Table::into_open`).
    into_open: usize,
    /// Per kind of state, its changes between two decisions' views.
    changes: std::collections::BTreeMap<String, usize>,
}

/// The action ended last: its seat, acting minion and card's copy.
#[derive(Clone, Copy)]
struct Closed {
    seat: u8,
    acting: u32,
    card: Option<u32>,
}

/// An option names no copy the viewer has not seen (`u32::MAX`), in its
/// key, effects or user: an uncontrolled vampire by its `slot`, its copy
/// only to who sees it.
fn sees_its_copies(o: &zvtes_api::option::OptionView) -> bool {
    use zvtes_api::option::User;
    use zvtes_api::view::Target;
    let k = &o.key;
    let copies = [
        k.actor,
        k.minion,
        k.card.as_ref().and_then(|c| c.copy),
        k.card.as_ref().and_then(|c| c.from),
        match k.target {
            Some(Target::Card { copy }) => Some(copy),
            _ => None,
        },
        match o.user {
            Some(User::Minion { copy }) => Some(copy),
            _ => None,
        },
    ];
    copies
        .into_iter()
        .chain(o.effects.iter().map(|e| e.copy))
        .all(|c| c != Some(u32::MAX))
}

/// A slot the view shows as a back: the option names no card or copy of
/// it.
fn keeps_backs(o: &zvtes_api::option::OptionView, v: &TableView) -> bool {
    let Some(r) = o.key.slot else {
        return true;
    };
    let back = v
        .seats
        .iter()
        .find(|s| s.seat == r.seat)
        .and_then(|s| s.uncontrolled.get(r.slot as usize))
        .is_none_or(|u| u.card.id.is_none());
    !back || (o.key.card.is_none() && o.key.target.is_none())
}

fn numbered(c: &Card) -> bool {
    c.copy.is_some() && c.id.is_some() && c.nth.is_some()
}

impl Stream {
    /// A pool or blood event's cause: there; and another seat's pool or
    /// another minion's blood the action ended just before moved (a
    /// bleed's, by its acting minion or its card) is told within it, not
    /// after (the acting minion's own riders may follow on itself: César
    /// Holfield's minion phase end).
    fn cause(&mut self, cause: Option<&zvtes_api::event::Cause>, e: &Event) {
        let seed = self.seed;
        let c = cause.unwrap_or_else(|| panic!("seed {seed}: no cause: {e:?}"));
        use zvtes_api::event::{CauseKind, CauseRule as R};
        let (pool, delta) = match e.kind {
            EventKind::Pool { delta, .. } => (true, delta),
            EventKind::Blood { delta, .. } => (false, delta),
            _ => unreachable!("a pool or blood event"),
        };
        // A cost is paid, never gained.
        assert!(
            !(c.kind == CauseKind::Cost && delta > 0),
            "seed {seed}: a cost gained: {e:?}"
        );
        // A rule moves what it can: a transfer 1 of the active seat's pool
        // (to an uncontrolled vampire, or back), combat blood (damage, a
        // strike stealing it), a disease or capacity blood lost, the Edge's
        // pool gained, a vote's price pool.
        let fits = match c.rule {
            None => c.card.is_some(),
            Some(R::Transfer) => {
                let active = |s: u8| self.active.is_none_or(|a| a == s);
                pool && delta.abs() == 1
                    && matches!(e.kind, EventKind::Pool { seat, .. } if active(seat))
            }
            Some(R::Strikes) => !pool,
            Some(R::Disease | R::Capacity) => !pool && delta < 0,
            Some(R::Edge) => pool && delta > 0,
            Some(R::Vote) => pool && delta < 0,
            Some(R::Oust | R::Withdrawal) => pool,
            Some(R::Contest) => delta < 0,
            Some(R::Diablerie) => true,
            Some(R::Unknown) => false,
        };
        assert!(fits, "seed {seed}: a cause that cannot be: {e:?}");
        if let Some(a) = self.closed {
            let by = c.card.as_ref().and_then(|c| c.copy);
            let of_action = by.is_some() && (by == Some(a.acting) || by == a.card);
            let other = match e.kind {
                EventKind::Pool { seat, .. } => seat != a.seat,
                EventKind::Blood { minion, .. } => minion != a.acting,
                _ => false,
            };
            assert!(
                !(c.kind == CauseKind::Effect && of_action && other),
                "seed {seed}: told after its action: {e:?}"
            );
        }
        self.caused += 1;
    }

    fn event(&mut self, e: &Event) {
        let seed = self.seed;
        if let EventKind::Decided {
            seat,
            choice,
            source,
            purpose,
            decision,
            ..
        } = &e.kind
        {
            self.decided = Some(*seat);
            assert!(sees_its_copies(choice), "seed {seed}: {e:?}");
            assert!(
                source.as_ref().and_then(|c| c.copy) != Some(u32::MAX),
                "seed {seed}: {e:?}"
            );
            // An uncontrolled vampire named only as the viewer has seen
            // it (a transfer's target, a slot choice's card, a play's).
            if choice.key.slot.is_some() {
                use zvtes_api::view::Target;
                let named = match choice.key.target {
                    Some(Target::Card { copy }) => Some(copy),
                    _ => None,
                };
                let card = (choice.key.kind == OptionKind::Slot)
                    .then_some(choice.key.card.as_ref())
                    .flatten();
                assert!(
                    named.is_none_or(|c| self.names.knows(c))
                        && card.is_none_or(|c| c.copy.is_some_and(|c| self.names.knows(c))),
                    "seed {seed}: a face down vampire named: {e:?}"
                );
            }
            // As the decision told its seat.
            if let Some(told) = self.sources.remove(decision) {
                assert_eq!(told, (*source, *purpose), "seed {seed}: {e:?}");
            }
        }
        let decided = self.decided;
        self.told += 1;
        // An acting minion named before it acts: in a view, or entering
        // play first (a vampire created and hunting in one run).
        if let EventKind::ActionStarts { acting, .. } = &e.kind {
            assert!(
                self.names.is_minion(acting),
                "seed {seed}: acting before entering play: {e:?}"
            );
        }
        self.names.hear(&e.kind);
        if let Some(t) = &mut self.table {
            t.apply(&e.kind);
        }
        if std::env::var_os("HOST_TRACE").is_some() {
            eprintln!("event {seed} {:?} {:?}", e.kind, e.text);
        }
        if let Some(t) = &e.text {
            assert!(self.text, "seed {seed}: text unasked: {e:?}");
            assert!(!t.is_empty(), "seed {seed}");
            self.texts += 1;
            let rendered = match e.kind {
                // Its option's line (`zvtes_text::option`).
                EventKind::Decided { .. } => t.clone(),
                _ => zvtes_text::event(&e.kind, &self.names).unwrap_or_default(),
            };
            if &rendered != t {
                self.misses.push((t.clone(), rendered));
            }
        }
        // A minion entering or leaving play, or acting: its name's number
        // as the views give it (docs/protocol.md §3), a ghoul token's none.
        if let EventKind::EntersPlay {
            card,
            minion: Some(kind),
            ..
        }
        | EventKind::LeavesPlay {
            card,
            minion: Some(kind),
            ..
        }
        | EventKind::ActionStarts {
            acting: card,
            minion: kind,
            ..
        }
        | EventKind::ActingChanges {
            acting: card,
            minion: kind,
            ..
        } = &e.kind
        {
            if let Some(c) = card.copy {
                if *kind == MinionKind::Token {
                    assert_eq!(card.numbered, None, "seed {seed}: a token numbered");
                }
                self.number(c, card.numbered, "an event");
            }
        }
        // A play finished: the card played numbered for the viewer, each
        // minion or card its effects name too.
        match &e.kind {
            EventKind::Resolved { card, effects, .. } => {
                assert!(card.copy.is_some(), "seed {seed}: {e:?}");
                // Its copy told as it was played (docs/protocol.md §9).
                let c = card.copy.unwrap_or_default();
                assert!(
                    self.played_in.contains(&c),
                    "seed {seed}: played untold: {e:?}"
                );
                // What it moves at once told by its events, not twice.
                use zvtes_api::option::EffectKind as K;
                assert!(
                    !effects
                        .iter()
                        .any(|f| f.when.is_none() && matches!(f.kind, K::Pool | K::Blood | K::Life)),
                    "seed {seed}: told twice: {e:?}"
                );
                for c in effects.iter().filter_map(|f| f.copy) {
                    assert_ne!(c, u32::MAX, "seed {seed}: an unnumbered copy: {e:?}");
                }
                self.played[0] += 1;
            }
            EventKind::Canceled { card, .. } => {
                assert!(card.copy.is_some(), "seed {seed}: {e:?}");
                let c = card.copy.unwrap_or_default();
                assert!(
                    self.played_in.contains(&c),
                    "seed {seed}: played untold: {e:?}"
                );
                self.played[1] += 1;
            }
            _ => {}
        }
        // A bleed told with the Methuselah it bleeds (docs/protocol.md §9).
        if let EventKind::ActionStarts { action, .. } | EventKind::ActionEnds { action, .. } =
            &e.kind
        {
            if action.kind == zvtes_api::ids::ActionKind::Bleed {
                assert!(
                    matches!(action.target, Some(zvtes_api::view::Target::Seat { .. })),
                    "seed {seed}: a bleed told without its target: {e:?}"
                );
                self.bleeds += 1;
            }
        }
        let copy = |c: u32| assert_ne!(c, u32::MAX, "seed {seed}: an unnumbered minion: {e:?}");
        match &e.kind {
            EventKind::Turn { active, .. } => {
                self.active = Some(*active);
                self.closed = None;
                self.decided = None;
                self.phase = Some(zvtes_api::ids::Phase::Unlock);
                self.transfers = None;
            }
            // A phase begun: a new one, the game's end its `over` message.
            EventKind::Phase { phase } => {
                assert_ne!(*phase, zvtes_api::ids::Phase::Over, "seed {seed}: {e:?}");
                assert_ne!(self.phase, Some(*phase), "seed {seed}: told again: {e:?}");
                self.phase = Some(*phase);
                self.transfers = None;
            }
            // A card as the viewer may know it: a back where neither place
            // shows it (a library, a crypt, another's face-down vampires),
            // its face where one plainly does.
            EventKind::CardMoves { card, from, to } => {
                use zvtes_api::ids::Zone as Z;
                self.moved[0] += 1;
                self.moved[1] += usize::from(card.id.is_none());
                let region = from.zone == Z::Uncontrolled && to.zone == Z::Uncontrolled;
                self.moved[2] += usize::from(region && from.seat == to.seat);
                let v = self.viewer;
                let hidden =
                    |r: &zvtes_api::event::ZoneRef| matches!(r.zone, Z::Library | Z::Crypt);
                let shown = |r: &zvtes_api::event::ZoneRef| match r.zone {
                    Z::AshHeap | Z::Removed | Z::Limbo | Z::InPlay | Z::Ready | Z::Torpor => true,
                    Z::Hand | Z::Uncontrolled => Some(r.seat) == v,
                    _ => false,
                };
                if hidden(from) && hidden(to) {
                    assert!(
                        card.id.is_none() && card.copy.is_none(),
                        "seed {seed}: {e:?}"
                    );
                }
                if shown(from) || shown(to) {
                    assert!(card.id.is_some(), "seed {seed}: a face withheld: {e:?}");
                }
                if to.zone == Z::Limbo {
                    self.played_in
                        .insert(card.copy.expect("a card into limbo numbered"));
                }
            }
            // At each change: never the same twice in a row.
            EventKind::Transfers { left } => {
                assert_ne!(
                    self.transfers,
                    Some(*left),
                    "seed {seed}: told again: {e:?}"
                );
                self.transfers = Some(*left);
            }
            // An uncontrolled vampire's counters: a transfer, or a card.
            EventKind::Counters {
                slot: Some(_),
                cause,
                ..
            } => {
                assert!(cause.is_some(), "seed {seed}: no cause: {e:?}");
            }
            EventKind::Decided { .. } => self.closed = None,
            EventKind::Undone { to, .. } => {
                self.again = Some(*to);
                // Back in another turn, perhaps: the next view tells it.
                self.active = None;
                self.sources.clear();
                self.open = None;
                self.in_combat = None;
                self.name_numbers.clear();
                self.prev = None;
                self.table = None;
                self.phase = None;
                self.transfers = None;
                self.played_in.clear();
            }
            EventKind::CombatBegins { minions } => {
                minions.iter().for_each(|&m| copy(m));
                assert_ne!(self.in_combat, Some(true), "seed {seed}: begun twice");
                self.in_combat = Some(true);
            }
            EventKind::Strikes { sides, .. } => {
                sides.iter().for_each(|s| copy(s.minion));
                for s in sides {
                    assert!(
                        s.strike_damage <= s.damage,
                        "seed {seed}: a strike's share beyond the damage: {s:?}"
                    );
                }
                assert_ne!(
                    self.in_combat,
                    Some(false),
                    "seed {seed}: strikes out of combat"
                );
            }
            EventKind::StrikeDeclared { minion, .. } => {
                copy(*minion);
                assert_ne!(
                    self.in_combat,
                    Some(false),
                    "seed {seed}: a strike out of combat"
                );
            }
            EventKind::CombatEnds => {
                assert_ne!(
                    self.in_combat,
                    Some(false),
                    "seed {seed}: an untold combat ended"
                );
                self.in_combat = Some(false);
            }
            EventKind::ActionStarts { seat, acting, .. } => {
                let acting = &acting.copy.expect("the acting minion's copy");
                copy(*acting);
                // A forced action (no decision of its seat before it: a
                // hunt) is the active Methuselah's: told after its turn's
                // `turn`. One decided may be another's (a minion phase a
                // card grants).
                if let (Some(active), true) = (self.active, decided != Some(*seat)) {
                    assert_eq!(active, *seat, "seed {seed}: forced before its turn: {e:?}");
                }
                self.closed = None;
                if let Some(open) = self.open {
                    assert_eq!(open, None, "seed {seed}: announced while one is open");
                }
                self.open = Some(Some((*seat, *acting)));
                self.undecided += usize::from(decided != Some(*seat));
                self.decided = None;
                self.started = true;
            }
            // Another minion now acting: within an action of its seat.
            EventKind::ActingChanges { seat, acting, .. } => {
                let acting = acting.copy.expect("the acting minion's copy");
                copy(acting);
                if let Some(open) = self.open {
                    let Some((s, _)) = open else {
                        panic!("seed {seed}: acting changes with no action");
                    };
                    assert_eq!(s, *seat, "seed {seed}: another seat's action");
                    self.open = Some(Some((s, acting)));
                }
            }
            EventKind::ActionEnds {
                seat,
                acting,
                blocker,
                action,
                ..
            } => {
                copy(*acting);
                blocker.iter().for_each(|&m| copy(m));
                if let Some(open) = self.open {
                    assert_eq!(
                        open,
                        Some((*seat, *acting)),
                        "seed {seed}: ended unannounced"
                    );
                }
                self.open = Some(None);
                self.closed = Some(Closed {
                    seat: *seat,
                    acting: *acting,
                    card: action.card.as_ref().and_then(|c| c.copy),
                });
                self.within += usize::from(self.started);
            }
            EventKind::ReferendumEnds { action, .. } => {
                // A political action's referendum ends within it.
                if let (Some(open), Some(_)) = (self.open, action) {
                    assert!(
                        open.is_some(),
                        "seed {seed}: a referendum ended after its action"
                    );
                }
            }
            EventKind::Pool { cause, .. } => self.cause(cause.as_ref(), e),
            EventKind::Blood { minion, cause, .. } => {
                copy(*minion);
                self.cause(cause.as_ref(), e);
            }
            EventKind::Torpor { minion, .. } => copy(*minion),
            EventKind::EntersPlay {
                card, on, effect, ..
            }
            | EventKind::Moves { card, on, effect } => {
                assert!(numbered(card) != *effect, "seed {seed}: {e:?}");
                on.iter().for_each(|&m| copy(m));
            }
            EventKind::LeavesPlay { card, effect, .. } => {
                assert!(numbered(card) != *effect, "seed {seed}: {e:?}");
            }
            EventKind::BlockFails { minion } => {
                copy(*minion);
                if let Some(open) = self.open {
                    assert!(
                        open.is_some(),
                        "seed {seed}: a block failed after its action"
                    );
                }
            }
            _ => {}
        }
    }

    /// `c`'s name number, `n`, as one message tells it: the same as every
    /// other told it since the last undo.
    fn number(&mut self, c: u32, n: Option<u8>, what: &str) {
        let seed = self.seed;
        let was = *self.name_numbers.entry(c).or_insert(n);
        assert_eq!(was, n, "seed {seed}: {c}'s number in {what}");
    }

    fn view(&mut self, v: &TableView, at: &str) {
        let seed = self.seed;
        self.viewer = v.viewer;
        let in_play = |m: &zvtes_api::view::MinionView| {
            m.markers
                .iter()
                .any(|k| k.kind == zvtes_api::ids::MarkerKind::Contested)
        };
        if v.seats.iter().flat_map(|s| &s.ready).any(in_play) {
            self.contested += 1;
        }
        // A card in limbo at the first view, or the view an undo goes back
        // to, was played before (the events since tell the rest).
        if self.prev.is_none() {
            let limbo = v.seats.iter().flat_map(|s| &s.limbo).filter_map(|c| c.copy);
            self.played_in.extend(limbo);
        }
        self.names.see(v);
        // Minions of one name told apart, each as told before.
        let mut names: Vec<(&str, Option<u8>)> = Vec::new();
        for m in v.seats.iter().flat_map(|s| s.minions()) {
            if let Some(c) = m.card.copy {
                self.number(c, m.card.numbered, at);
            }
            if m.kind == MinionKind::Token {
                continue;
            }
            let n = (m.name.as_str(), m.card.numbered);
            assert!(!names.contains(&n), "seed {seed} {at}: two {n:?}");
            names.push(n);
        }
        // A combat's strikes in English exactly when the settings ask.
        for s in v
            .combat
            .iter()
            .flat_map(|c| &c.sides)
            .filter_map(|s| s.strike.as_ref())
        {
            let want = self
                .text
                .then(|| zvtes_text::strike_declared(s, &self.names));
            assert_eq!(s.text, want, "seed {seed} {at}: a strike's text");
        }
        // Markers in English likewise, from their data.
        for k in v
            .seats
            .iter()
            .flat_map(|s| s.minions())
            .flat_map(|m| &m.markers)
        {
            let want = self.text.then(|| zvtes_text::marker(k, &self.names));
            assert_eq!(k.text, want, "seed {seed} {at}: a marker's text");
            self.markers += 1;
        }
        self.shows(v, at);
        if at.starts_with("decision") && v.combat.is_some() {
            self.combats += 1;
        }
        // The kinds of state a view changed, each told (`reduce`).
        if let Some(p) = self.prev.take() {
            for (kind, _) in changed(&p, v) {
                *self.changes.entry(kind).or_default() += 1;
            }
        }
        self.prev = Some(v.clone());
    }

    /// The table the events kept since the last decision's view is this
    /// one (`over`'s: the phase over); then this view is the table.
    fn reduce(&mut self, v: &TableView, at: &str, over: bool) {
        if std::env::var_os("HOST_TRACE").is_some() {
            eprintln!("view {} {at}", self.seed);
        }
        let mut open = Default::default();
        if let Some(mut t) = self.table.take() {
            if over {
                t.v.phase = zvtes_api::ids::Phase::Over;
                t.v.transfers = None;
            }
            let missed = reduce::check(&mut t.v, v, &mut self.unordered);
            self.reduced += 1;
            self.into_open += t.into_open;
            for (path, example) in missed {
                if std::env::var_os("HOST_REDUCE").is_some() {
                    eprintln!("seed {} {at}: {path}: {example}", self.seed);
                }
                let e = self.unreduced.entry(path).or_insert((0, String::new()));
                e.0 += 1;
                if e.1.is_empty() {
                    e.1 = format!("seed {} {at}: {example}", self.seed);
                }
            }
            open = t.open;
        }
        self.table = Some(reduce::Table::new(v.clone(), &open));
    }

    /// The view shows the action the events told under way, if any.
    fn shows(&mut self, v: &TableView, at: &str) {
        let action = v.action.as_ref().map(|a| (a.seat, a.acting));
        if let Some(open) = self.open {
            assert_eq!(open, action, "seed {} {at}: the action told", self.seed);
        }
        self.open = Some(action);
    }

    /// Every card `x` names by its copy and id (a view's card, an
    /// option's or an event's): the card its copy first named.
    fn faces(&mut self, x: &impl serde::Serialize) {
        fn walk(x: &serde_json::Value, out: &mut Vec<(u64, u64)>) {
            match x {
                serde_json::Value::Object(o) => {
                    if let (Some(c), Some(id)) = (
                        o.get("copy").and_then(|c| c.as_u64()),
                        o.get("id").and_then(|c| c.as_u64()),
                    ) {
                        out.push((c, id));
                    }
                    o.values().for_each(|x| walk(x, out));
                }
                serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
                _ => {}
            }
        }
        let mut cards = Vec::new();
        walk(&serde_json::to_value(x).expect("serializes"), &mut cards);
        for (c, id) in cards {
            let c = c as u32;
            let was = *self.faces.entry(c).or_insert(id);
            assert_eq!(was, id, "seed {}: copy {c} names another card", self.seed);
            assert!(
                !self.dead.contains(&c),
                "seed {}: copy {c} given again",
                self.seed
            );
            self.born.entry(c).or_insert(self.seq);
        }
    }

    fn see(&mut self, outs: &[Out]) {
        for o in outs {
            self.seq += 1;
            if let Message::Events { events, .. } = &o.message {
                for e in events {
                    if let EventKind::Undone { to, .. } = e.kind {
                        if let Some(&at) = self.asked_at.get(&to) {
                            let since = self.born.iter().filter(|&(_, &b)| b > at);
                            self.dead.extend(since.map(|(&c, _)| c));
                        }
                    }
                }
            }
            if let Message::Decision(d) = &o.message {
                self.asked_at.insert(d.decision, self.seq);
            }
            self.faces(&o.message);
            self.started = false;
            match &o.message {
                // A step's view is the table after the step, or after the
                // whole allocation an `allocate` answered: checked where
                // everything is told (decisions, the end).
                Message::Events { events, view } => {
                    events.iter().for_each(|e| self.event(e));
                    if let Some(v) = view {
                        self.names.see(v);
                        self.active = Some(v.active);
                        self.shows(v, "a step");
                    }
                }
                Message::Decision(d) => {
                    self.active = Some(d.view.active);
                    for o in &d.options {
                        assert!(sees_its_copies(o), "seed {}: {o:?}", self.seed);
                        assert!(keeps_backs(o, &d.view), "seed {}: {o:?}", self.seed);
                    }
                    self.sources.insert(d.decision, (d.source, d.purpose));
                    if let Some(to) = self.again.take() {
                        assert_eq!(d.decision, to, "seed {}: asked again", self.seed);
                        let first = self.asked.get(&to);
                        if first.is_none() {
                            self.unasked.push(to);
                        }
                        if let Some(first) = first {
                            assert_eq!(&d.view, first, "seed {}: the view asked again", self.seed);
                            self.reasked += 1;
                        }
                    }
                    self.asked.insert(d.decision, d.view.clone());
                    let at = format!("decision {}", d.decision);
                    self.reduce(&d.view, &at, false);
                    self.view(&d.view, &at)
                }
                Message::Over(over) => {
                    self.reduce(&over.view, "over", true);
                    self.view(&over.view, "over")
                }
                Message::Joined(j) => {
                    self.table = Some(reduce::Table::new(j.view.clone(), &Default::default()));
                    self.view(&j.view, "joined")
                }
                _ => {}
            }
        }
    }
}

/// Amounts giving all the allocation's points, within each cap, to at
/// least `min_nonzero` recipients.
fn amounts(a: &zvtes_api::option::Allocation, rng: &mut Rng) -> Option<Vec<u8>> {
    let n = a.recipients.len();
    let cap = |i: usize| a.recipients[i].cap.unwrap_or(a.points);
    let mut out = vec![0u8; n];
    let mut left = a.points;
    for (i, o) in out.iter_mut().enumerate().take(a.min_nonzero as usize) {
        if left > 0 && cap(i) > 0 {
            *o = 1;
            left -= 1;
        }
    }
    while left > 0 {
        let open: Vec<usize> = (0..n).filter(|&i| out[i] < cap(i)).collect();
        if open.is_empty() {
            return None;
        }
        out[open[rng.below(open.len())]] += 1;
        left -= 1;
    }
    Some(out)
}

fn cmd(command: Command) -> In {
    In { id: None, command }
}

#[test]
fn the_events_tell_what_the_views_show() {
    let seeds = seeds();
    let (mut told, mut texts, mut undone, mut allocated, mut voted) = (0, 0, 0, 0, 0);
    let mut played = [0, 0];
    let mut bleeds = 0;
    let mut contested = 0;
    let (mut within, mut undecided, mut caused, mut markers) = (0, 0, 0, 0);
    let mut combats = 0;
    let mut moved = [0; 3];
    let mut reasked = 0;
    let mut incapacitated = 0;
    let mut into_open = 0;
    let (mut unreduced, mut unordered, mut reduced) = (
        std::collections::BTreeMap::new(),
        std::collections::BTreeMap::new(),
        0,
    );
    let mut changes: std::collections::BTreeMap<String, usize> = Default::default();
    let mut misses = Vec::new();
    for &seed in &seeds {
        let settings = Settings {
            opportunities: seed % 3 != 0,
            views: Views::Steps,
            text: seed.is_multiple_of(2),
        };
        let mut g = game(seed.into(), settings);
        let human = g.human();
        let mut rng = Rng(seed as u64 * 7 + 1);
        let mut s = Stream {
            seed,
            text: settings.text,
            in_combat: Some(false),
            told: 0,
            texts: 0,
            names: Names::new(zvtes_host::cards::cards()),
            misses: Vec::new(),
            asked: HashMap::new(),
            again: None,
            reasked: 0,
            unasked: Vec::new(),
            name_numbers: HashMap::new(),
            played: [0, 0],
            bleeds: 0,
            contested: 0,
            open: Some(None),
            decided: None,
            within: 0,
            undecided: 0,
            started: false,
            active: None,
            closed: None,
            caused: 0,
            sources: HashMap::new(),
            markers: 0,
            combats: 0,
            moved: [0; 3],
            faces: HashMap::new(),
            seq: 0,
            born: HashMap::new(),
            asked_at: HashMap::new(),
            dead: Default::default(),
            viewer: None,
            played_in: Default::default(),
            phase: None,
            transfers: None,
            prev: None,
            table: None,
            unreduced: Default::default(),
            unordered: Default::default(),
            reduced: 0,
            into_open: 0,
            changes: Default::default(),
        };
        let mut outs = g.handle(
            human,
            cmd(Command::Join {
                token: None,
                after: None,
            }),
        );
        s.see(&outs);
        let mut steps = 0;
        while !g.is_over() {
            steps += 1;
            assert!(steps < 20_000, "seed {seed}: no end");
            let d = outs
                .iter()
                .rev()
                .find_map(|o| match &o.message {
                    Message::Decision(d) => Some(d.clone()),
                    _ => None,
                })
                .expect("a decision until the end");
            let c = if let Some(a) = d.allocation.as_ref().filter(|_| rng.below(2) == 0) {
                let Some(amounts) = amounts(a, &mut rng) else {
                    panic!("seed {seed}: an allocation with no way to give its points: {a:?}");
                };
                // Too many points: refused, nothing changes.
                let mut bad = amounts.clone();
                bad[0] += 1;
                let refused = g.handle(
                    human,
                    cmd(Command::Allocate {
                        decision: d.decision,
                        amounts: bad,
                    }),
                );
                assert!(
                    matches!(
                        refused[..],
                        [Out {
                            message: Message::Error(_),
                            ..
                        }]
                    ),
                    "seed {seed}: {refused:?}"
                );
                allocated += 1;
                Command::Allocate {
                    decision: d.decision,
                    amounts,
                }
            } else if let Some(v) = d
                .votes
                .filter(|v| v.favor + v.against > 0 && rng.below(3) == 0)
            {
                voted += 1;
                Command::VoteAll {
                    decision: d.decision,
                    favor: v.favor > 0 && (v.against == 0 || rng.below(2) == 0),
                }
            } else if d.undo && rng.below(6) == 0 {
                undone += 1;
                Command::Undo {
                    decision: d.decision,
                }
            } else {
                // Cards played half the time: referendums (allocations,
                // votes) come of political actions.
                let plays: Vec<_> = d
                    .options
                    .iter()
                    .filter(|o| o.key.kind == OptionKind::Play)
                    .collect();
                let o = if !plays.is_empty() && rng.below(2) == 0 {
                    plays[rng.below(plays.len())]
                } else {
                    &d.options[rng.below(d.options.len())]
                };
                Command::Answer {
                    decision: d.decision,
                    option: o.index,
                }
            };
            outs = g.handle(human, cmd(c));
            s.see(&outs);
            for o in &outs {
                if let Message::Events { events, .. } = &o.message {
                    incapacitated += events
                        .iter()
                        .filter(|e| matches!(e.kind, EventKind::Incapacitated { .. }))
                        .count();
                }
            }
        }
        assert!(!settings.text || s.texts > 0, "seed {seed}: no text");
        reasked += s.reasked;
        assert!(
            s.unasked.is_empty(),
            "seed {seed}: undone to {:?}, never asked",
            s.unasked
        );
        told += s.told;
        texts += s.texts;
        within += s.within;
        undecided += s.undecided;
        caused += s.caused;
        markers += s.markers;
        combats += s.combats;
        for (m, n) in moved.iter_mut().zip(s.moved) {
            *m += n;
        }
        for (k, n) in s.changes {
            *changes.entry(k).or_default() += n;
        }
        played[0] += s.played[0];
        played[1] += s.played[1];
        bleeds += s.bleeds;
        contested += s.contested;
        misses.extend(s.misses.into_iter().map(|m| (seed, m)));
        reduced += s.reduced;
        if s.into_open > 0 && std::env::var_os("HOST_REDUCE").is_some() {
            eprintln!("seed {seed}: {} into an open hand", s.into_open);
        }
        into_open += s.into_open;
        for (k, (n, ex)) in s.unreduced {
            let e = unreduced.entry(k).or_insert((0, ex));
            e.0 += n;
        }
        for (k, n) in s.unordered {
            *unordered.entry(k).or_insert(0) += n;
        }
    }
    for (seed, (legacy, rendered)) in misses.iter().take(20) {
        eprintln!("seed {seed}: {legacy:?} rendered {rendered:?}");
    }
    assert!(
        misses.is_empty(),
        "{} events rendered otherwise",
        misses.len()
    );
    assert!(bleeds > 0, "no bleed told");
    eprintln!(
        "{} games: {told} events ({texts} with text), {undone} undos, \
         {allocated} allocations, {voted} votes cast at once, {reasked} views asked again alike",
        seeds.len()
    );
    assert!(
        contested > 0 || !seeds.iter().any(|s| TWO.contains(s)),
        "no two-player contest in view"
    );
    assert!(
        allocated > 0 && voted > 0,
        "allocations and votes at once untried"
    );
    eprintln!("plays told: {} resolved, {} canceled", played[0], played[1]);
    assert!(played[0] > 0 && played[1] > 0, "plays told: {played:?}");
    // The actions no view between two messages shows, and those the
    // engine announces itself, followed all the same.
    eprintln!("actions: {within} announced and ended in one message, {undecided} with no decision");
    assert!(within > 0 && undecided > 0, "actions untried");
    eprintln!("{caused} pool and blood events, each with its cause; {markers} markers told");
    assert!(caused > 0, "no pool or blood told");
    assert!(markers > 0, "no marker seen");
    eprintln!("{incapacitated} imbued told entering or leaving incapacitation");
    assert!(incapacitated > 0, "no imbued incapacitated");
    assert!(combats > 0, "no combat a decision's view showed");
    // Each kind of the table's state changes at least once between two
    // decisions' views: the reducer's check below meets it.
    for kind in [
        "phase",
        "transfers",
        "minion locked",
        "card locked",
        "card counters",
        "slot counters",
        "card controller",
        "minion capacity",
        "minion title",
        "minion markers",
        "action.step",
        "action.stealth",
        "action.bleed",
        "action.intercept",
        "referendum.votes",
        "referendum.terms",
        "referendum.step",
    ] {
        assert!(
            changes.get(kind).is_some_and(|&n| n > 0),
            "no {kind} change between two views"
        );
    }
    let n: usize = changes.values().sum();
    eprintln!("{n} changes of the table's state between decisions");
    eprintln!("{} cards told moving, {} as backs", moved[0], moved[1]);
    // Each decision's view is the last one with the events since applied
    // (`HOST_REDUCE=1` names each miss), a place's order the view's.
    eprintln!("{reduced} views kept from the events; places in another order: {unordered:?}");
    // A card into a hand open to the viewer keeps its face (`hand`).
    assert!(into_open > 0 || from() != 1, "no card into an open hand");
    // An uncontrolled region's order told too (`slot`): the counters an
    // event names by place are on the vampire the view shows there.
    assert!(
        !unordered.contains_key("uncontrolled"),
        "an uncontrolled region in another order: {unordered:?}"
    );
    assert!(moved[2] > 0, "no vampire moved within its region");
    for (path, (n, ex)) in &unreduced {
        eprintln!("  {path}: {n}, as {ex}");
    }
    assert!(
        unreduced.is_empty(),
        "{} paths the events miss",
        unreduced.len()
    );
    assert!(
        moved[0] > 0 && moved[1] > 0,
        "no card moves told: {moved:?}"
    );
}

/// Every option a seat is shown is told in English with the text
/// setting, every decision taken too (a pass where it does something),
/// and no line falls back to an unnamed card or minion: a name the client
/// was never shown is a field missing. A pass reads the same without the
/// options beside it and the action or referendum under way: what it
/// does is in its data, never only in its English (docs/protocol.md §7).
#[test]
fn every_option_and_choice_is_told() {
    let seeds = seeds();
    let (mut options, mut decided, mut prompts) = (0, 0, 0);
    let mut unnamed: std::collections::BTreeMap<String, (usize, String)> = Default::default();
    let decks = zvtes_host::catalog();
    for &seed in &seeds {
        let settings = Settings {
            opportunities: seed % 3 != 0,
            views: Views::Decisions,
            text: true,
        };
        let mut g = game(seed.into(), settings);
        let human = g.human();
        let mut rng = Rng(seed as u64 * 11 + 3);
        let mut outs = g.handle(
            human,
            cmd(Command::Join {
                token: None,
                after: None,
            }),
        );
        // The names a client knows, with no action or referendum seen.
        let mut bare = Names::new(zvtes_host::cards::cards());
        let see = |names: &mut Names, v: &TableView| {
            let mut v = v.clone();
            v.action = None;
            v.referendum = None;
            names.see(&v);
        };
        let mut check = |what: String, text: &str| {
            // A gap the words leave (a name of nothing); the decks' own
            // names aside.
            let words = decks.iter().fold(text.to_string(), |t, (name, _)| {
                t.replace(name.as_str(), "Deck")
            });
            assert!(
                words == words.trim() && !words.contains("  "),
                "seed {seed}: {what}: a gap in {text:?}"
            );
            // What the table may not know (another seat's library search,
            // a card put under the library or drawn from a store) and
            // words that name no card ("equip from a minion").
            let hidden = ["Bottom in", "DrawStored in", "in LibrarySearch"];
            if hidden.iter().any(|h| what.contains(h)) {
                return;
            }
            let text = text
                .replace("from a minion", "")
                .replace("a card requiring", "")
                .replace("with a card in your hand", "")
                .replace("a minion of", "")
                .replace("bottom a card", "")
                .replace("a card in your library", "")
                .replace("Discard a card from", "");
            for bare in ["a card", "a minion"] {
                if text.contains(bare) {
                    let e = unnamed
                        .entry(what.clone())
                        .or_insert((0, format!("seed {seed}: {text}")));
                    e.0 += 1;
                }
            }
        };
        loop {
            for o in &outs {
                if let Message::Events { events, view, .. } = &o.message {
                    for e in events {
                        bare.hear(&e.kind);
                        if let EventKind::Decided { choice, window, .. } = &e.kind {
                            let t = e.text.as_deref();
                            // A pass told only where it does something.
                            let told =
                                choice.key.kind != OptionKind::Pass || !choice.effects.is_empty();
                            assert_eq!(
                                t.is_some_and(|t| !t.is_empty()),
                                told,
                                "seed {seed}: a decision told {t:?}: {e:?}"
                            );
                            if let Some(t) = t {
                                decided += 1;
                                check(format!("decided {:?} in {window:?}", choice.key.kind), t);
                            }
                        } else if let Some(t) = &e.text {
                            check(format!("{:?}", std::mem::discriminant(&e.kind)), t);
                        }
                    }
                    if let Some(v) = view {
                        see(&mut bare, v);
                    }
                }
            }
            if g.is_over() {
                break;
            }
            let d = outs
                .iter()
                .rev()
                .find_map(|o| match &o.message {
                    Message::Decision(d) => Some(d.clone()),
                    _ => None,
                })
                .unwrap();
            see(&mut bare, &d.view);
            if let Some(p) = &d.prompt {
                prompts += 1;
                check(format!("prompt in {:?} for {:?}", d.window, d.purpose), p);
            }
            for o in &d.options {
                let t = o.text.as_deref();
                assert!(
                    t.is_some_and(|t| !t.is_empty()),
                    "seed {seed}: an option untold in {:?} for {:?}: {o:?}",
                    d.window,
                    d.purpose
                );
                options += 1;
                check(format!("{:?} in {:?}", o.key.kind, d.window), t.unwrap());
                if o.key.kind == OptionKind::Pass {
                    let alone = zvtes_text::option(
                        o,
                        d.window,
                        d.purpose,
                        d.about.as_ref(),
                        d.seat,
                        &[],
                        &bare,
                    );
                    assert_eq!(
                        alone.as_deref(),
                        t,
                        "seed {seed}: a pass in {:?} for {:?} told beyond its data: {o:?}",
                        d.window,
                        d.purpose
                    );
                }
            }
            // The options of one decision read apart, unless they do the
            // same (a card's line split into modes alike in effect).
            for (i, a) in d.options.iter().enumerate() {
                for b in &d.options[i + 1..] {
                    assert!(
                        a.text != b.text || a.text.is_none() || a.effects == b.effects,
                        "seed {seed}: two options read {:?} in {:?}",
                        a.text,
                        d.window
                    );
                }
            }
            let plays: Vec<_> = d
                .options
                .iter()
                .filter(|o| o.key.kind == OptionKind::Play)
                .collect();
            let o = if !plays.is_empty() && rng.below(2) == 0 {
                plays[rng.below(plays.len())]
            } else {
                &d.options[rng.below(d.options.len())]
            };
            outs = g.handle(
                human,
                cmd(Command::Answer {
                    decision: d.decision,
                    option: o.index,
                }),
            );
        }
    }
    let mut unnamed: Vec<_> = unnamed.into_iter().collect();
    unnamed.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    for (what, (n, eg)) in &unnamed {
        eprintln!("unnamed: {n:6} {what} (e.g. {eg:?})");
    }
    eprintln!(
        "{} games: {options} options, {decided} decisions, {prompts} prompts told",
        seeds.len()
    );
    assert!(
        unnamed.is_empty(),
        "{} kinds of lines name nothing",
        unnamed.len()
    );
}

/// The table's state a view shows (the zones' cards aside: the minions'
/// and cards' own state, the phase, the action, combat and referendum
/// under way), each entry keyed by its kind and what it is about.
fn state(v: &TableView) -> std::collections::BTreeMap<(String, String), String> {
    let mut m = std::collections::BTreeMap::new();
    let mut put = |kind: &str, of: String, x: &dyn std::fmt::Debug| {
        m.insert((kind.to_string(), of), format!("{x:?}"));
    };
    put("phase", String::new(), &v.phase);
    put("transfers", String::new(), &v.transfers);
    for s in &v.seats {
        let n = s.seat;
        put("edge", n.to_string(), &s.edge);
        put("withdrawal", n.to_string(), &(s.withdrawing, s.withdrew));
        put("vp_halves", n.to_string(), &s.vp_halves);
        put("prey", n.to_string(), &(s.prey, s.predator));
        for (i, u) in s.uncontrolled.iter().enumerate() {
            // By its place (`seat:slot`), and its copy if the viewer sees
            // it (`seat:slot:copy`): a slot leaving shifts the rest.
            let of = u
                .card
                .copy
                .map_or(format!("{n}:{i}"), |c| format!("{n}:{i}:{c}"));
            put("slot counters", of.clone(), &u.counters);
            put("slot banished", of, &u.banished);
        }
        let mut cards: Vec<&zvtes_api::view::InPlayView> = s.in_play.iter().collect();
        for mi in s.minions() {
            let Some(c) = mi.card.copy else { continue };
            let c = c.to_string();
            put("minion contested", c.clone(), &mi.contested);
            put("minion name", c.clone(), &(&mi.name, mi.kind));
            // A contested minion is out of play: its state told as it
            // enters play again.
            if !mi.contested {
                put("minion locked", c.clone(), &mi.locked);
                put("minion capacity", c.clone(), &mi.capacity);
                put("minion title", c.clone(), &mi.title);
                let markers: Vec<_> = mi
                    .markers
                    .iter()
                    .map(|k| (k.kind, k.seat, k.n, k.card))
                    .collect();
                put("minion markers", c.clone(), &markers);
                put("minion traits", c.clone(), &mi.traits);
                put("minion owner", c.clone(), &mi.owner);
                put("minion controller", c, &n);
            }
            cards.extend(&mi.cards);
        }
        for c in cards {
            let Some(k) = c.card.copy else { continue };
            let k = k.to_string();
            put("card contested", k.clone(), &c.contested);
            // A contested card is out of play: its state told as it
            // enters play again.
            if !c.contested {
                put("card locked", k.clone(), &c.locked);
                put("card counters", k.clone(), &c.counters);
                put("card controller", k, &c.controller);
            }
        }
    }
    let mut obj = |what: &str, of: String, x: serde_json::Value| {
        if let serde_json::Value::Object(o) = x {
            for (f, x) in o {
                m.insert((format!("{what}.{f}"), of.clone()), x.to_string());
            }
        }
    };
    if let Some(a) = &v.action {
        obj(
            "action",
            format!("{}:{}", a.seat, a.acting),
            serde_json::to_value(a).unwrap(),
        );
    }
    if let Some(c) = &v.combat {
        obj(
            "combat",
            format!("{:?}", c.minions),
            serde_json::to_value(c).unwrap(),
        );
    }
    if let Some(r) = &v.referendum {
        obj(
            "referendum",
            r.caller.to_string(),
            serde_json::to_value(r).unwrap(),
        );
    }
    m
}

/// What changed between two views: the entries in both, unequal.
fn changed(a: &TableView, b: &TableView) -> Vec<(String, String)> {
    let (a, b) = (state(a), state(b));
    a.iter()
        .filter(|&(k, x)| b.get(k).is_some_and(|y| y != x))
        .map(|(k, _)| k.clone())
        .collect()
}
