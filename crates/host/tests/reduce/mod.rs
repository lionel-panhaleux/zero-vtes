//! A client's table kept from the events alone (docs/protocol.md §9):
//! the last view, each event applied as it comes, is the next view; a
//! place's order the next view's to show, and a search's cards in its
//! seat's library the decision's.

use std::collections::{BTreeMap, HashMap};

use zvtes_api::event::{EventKind, ZoneRef};
use zvtes_api::ids::{MinionKind, Phase, Zone};
use zvtes_api::view::{Card, InPlayView, MinionView, SlotView, TableView};

/// A card as it sits in its place: a slot keeps its counters, a card in
/// play its state, a minion its own.
enum Item {
    Card(Card),
    Slot(SlotView),
    InPlay(InPlayView),
    Minion(MinionView),
}

impl Item {
    fn card(&self) -> Card {
        match self {
            Item::Card(c) => *c,
            Item::Slot(s) => s.card,
            Item::InPlay(c) => c.card,
            Item::Minion(m) => m.card,
        }
    }
}

pub struct Table {
    pub v: TableView,
    /// Each minion's kind and name's number, as its `enters_play` told
    /// them (docs/protocol.md §3: `numbered` on a minion entering play).
    kinds: HashMap<u32, (MinionKind, Option<u8>)>,
    /// Cards told onto a minion or a banished vampire not on the table
    /// yet (it comes next in the same step).
    waiting: HashMap<u32, Vec<Item>>,
    cards: &'static dyn zvtes_text::Cards,
}

/// A card as a place hiding it shows it: a back.
fn back(c: Card) -> Card {
    Card {
        copy: None,
        id: None,
        crypt: c.crypt,
        nth: None,
        numbered: None,
    }
}

/// Whether two cards are the same where a place holds `had`: by copy,
/// else by face, else any back.
fn same(had: &Card, told: &Card) -> bool {
    match (had.copy, told.copy) {
        (Some(a), Some(b)) => a == b,
        (Some(_), None) => false,
        (None, _) => had.crypt == told.crypt && (had.id.is_none() || had.id == told.id),
    }
}

impl Table {
    pub fn new(v: TableView) -> Table {
        Table {
            v,
            kinds: HashMap::new(),
            waiting: HashMap::new(),
            cards: zvtes_host::cards::cards(),
        }
    }

    fn hides(&self, r: &ZoneRef) -> bool {
        match r.zone {
            Zone::Library | Zone::Crypt => true,
            Zone::Hand | Zone::Uncontrolled => r.on.is_none() && Some(r.seat) != self.v.viewer,
            _ => false,
        }
    }

    fn minion_mut(&mut self, copy: u32) -> Option<&mut MinionView> {
        self.v
            .seats
            .iter_mut()
            .flat_map(|s| s.minions_mut())
            .find(|m| m.card.copy == Some(copy))
    }

    /// A card in play (on no minion, on one, in training).
    fn in_play_mut(&mut self, copy: u32) -> Option<&mut InPlayView> {
        fn find(v: &mut [InPlayView], copy: u32) -> Option<&mut InPlayView> {
            v.iter_mut().find(|c| c.card.copy == Some(copy))
        }
        let seats = &mut self.v.seats;
        for s in seats.iter_mut() {
            if let Some(c) = find(&mut s.in_play, copy) {
                return Some(c);
            }
            if let Some(c) = find(&mut s.training, copy) {
                return Some(c);
            }
            // Field by field: `minions_mut` would borrow all of `s` past
            // the returns above.
            for m in s
                .ready
                .iter_mut()
                .chain(&mut s.torpor)
                .chain(&mut s.incapacitated)
            {
                if let Some(c) = find(&mut m.cards, copy) {
                    return Some(c);
                }
            }
        }
        None
    }

    fn take(&mut self, r: &ZoneRef, told: &Card) -> Option<Item> {
        fn pick<T>(v: &mut Vec<T>, told: &Card, card: impl Fn(&T) -> Card) -> Option<T> {
            let i = v.iter().rposition(|x| same(&card(x), told))?;
            Some(v.remove(i))
        }
        let s = r.seat as usize;
        match (r.zone, r.on) {
            (Zone::Ready | Zone::Torpor | Zone::Incapacitated, None) => {
                let seat = &mut self.v.seats[s];
                let m = pick(&mut seat.ready, told, |m| m.card)
                    .or_else(|| pick(&mut seat.torpor, told, |m| m.card))
                    .or_else(|| pick(&mut seat.incapacitated, told, |m| m.card))?;
                Some(Item::Minion(m))
            }
            (Zone::Ready | Zone::Torpor | Zone::Incapacitated, Some(_)) => None,
            (Zone::InPlay, None) => {
                pick(&mut self.v.seats[s].in_play, told, |c| c.card).map(Item::InPlay)
            }
            (Zone::InPlay, Some(m)) => {
                let m = self.minion_mut(m)?;
                pick(&mut m.cards, told, |c| c.card).map(Item::InPlay)
            }
            (Zone::Training, _) => {
                pick(&mut self.v.seats[s].training, told, |c| c.card).map(Item::InPlay)
            }
            (Zone::Stored, Some(c)) => {
                let c = self.in_play_mut(c)?;
                pick(&mut c.stored, told, |c| *c).map(Item::Card)
            }
            (Zone::Uncontrolled, None) => {
                let region = &mut self.v.seats[s].uncontrolled;
                match r.slot.map(usize::from) {
                    Some(i) if region.get(i).is_some_and(|u| same(&u.card, told)) => {
                        Some(Item::Slot(region.remove(i)))
                    }
                    _ => pick(region, told, |u| u.card).map(Item::Slot),
                }
            }
            (Zone::Uncontrolled, Some(v)) => {
                let slot = self
                    .v
                    .seats
                    .iter_mut()
                    .flat_map(|s| s.uncontrolled.iter_mut())
                    .find(|u| u.card.copy == Some(v))?;
                pick(&mut slot.cards, told, |c| *c).map(Item::Card)
            }
            (z, _) => {
                let seat = &mut self.v.seats[s];
                let v = match z {
                    Zone::Hand => &mut seat.hand,
                    Zone::Library => &mut seat.library,
                    Zone::Crypt => &mut seat.crypt,
                    Zone::AshHeap => &mut seat.ash_heap,
                    Zone::Removed => &mut seat.removed,
                    Zone::Limbo => &mut seat.limbo,
                    _ => return None,
                };
                pick(v, told, |c| *c).map(Item::Card)
            }
        }
    }

    fn put(&mut self, from: &ZoneRef, r: &ZoneRef, told: Card, item: Option<Item>) {
        let card = if self.hides(r) { back(told) } else { told };
        let s = r.seat as usize;
        let in_play = |item: Option<Item>| match item {
            Some(Item::InPlay(c)) => InPlayView { card, ..c },
            // Its place's holder's, else told by `controller` after.
            _ => InPlayView {
                card,
                controller: r.seat,
                locked: false,
                counters: 0,
                contested: false,
                stored: vec![],
            },
        };
        match (r.zone, r.on) {
            (Zone::Ready | Zone::Torpor, None) => {
                let m = match item {
                    Some(Item::Minion(m)) => {
                        // Taken by another Methuselah: its owner kept.
                        let owner = m.owner.unwrap_or(from.seat);
                        MinionView {
                            card,
                            owner: (owner != r.seat).then_some(owner),
                            ..m
                        }
                    }
                    other => {
                        let (blood, cards) = match other {
                            Some(Item::Slot(u)) => (u.counters, vec![]),
                            _ => (0, vec![]),
                        };
                        let told = card.copy.and_then(|c| self.kinds.get(&c).copied());
                        let kind = told.map_or(
                            if card.crypt {
                                MinionKind::Vampire
                            } else {
                                MinionKind::Ally
                            },
                            |(k, _)| k,
                        );
                        let card = Card {
                            numbered: card.numbered.or(told.and_then(|(_, n)| n)),
                            ..card
                        };
                        let name = card
                            .id
                            .and_then(|id| self.cards.name(id))
                            .unwrap_or_default()
                            .to_string();
                        MinionView {
                            card,
                            kind,
                            name,
                            blood,
                            capacity: None,
                            locked: false,
                            contested: false,
                            title: None,
                            owner: None,
                            cards,
                            markers: vec![],
                            traits: vec![],
                        }
                    }
                };
                let mut m = m;
                for w in m
                    .card
                    .copy
                    .and_then(|c| self.waiting.remove(&c))
                    .unwrap_or_default()
                {
                    if let Item::InPlay(c) = w {
                        m.cards.push(c);
                    }
                }
                let seat = &mut self.v.seats[s];
                match r.zone {
                    Zone::Torpor => seat.torpor.push(m),
                    Zone::Incapacitated => seat.incapacitated.push(m),
                    _ => seat.ready.push(m),
                }
            }
            // A merge: the base vampire's card onto the minion it is now,
            // the advanced card's (its counters burned), with the base's
            // own state and cards.
            (Zone::Ready | Zone::Torpor, Some(on)) => {
                if let Some(Item::Minion(m)) = item {
                    if let Some(to) = self.minion_mut(on) {
                        *to = MinionView {
                            card: to.card,
                            name: to.name.clone(),
                            ..m
                        };
                    }
                }
            }
            (Zone::InPlay, None) => {
                let c = in_play(item);
                self.v.seats[s].in_play.push(c);
            }
            (Zone::InPlay, Some(m)) => {
                let c = in_play(item);
                match self.minion_mut(m) {
                    Some(m) => m.cards.push(c),
                    None => self.waiting.entry(m).or_default().push(Item::InPlay(c)),
                }
            }
            (Zone::Training, _) => {
                let c = in_play(item);
                self.v.seats[s].training.push(c);
            }
            (Zone::Stored, Some(on)) => {
                if let Some(c) = self.in_play_mut(on) {
                    c.stored.push(card);
                }
            }
            (Zone::Uncontrolled, None) => {
                let (counters, banished, mut cards) = match item {
                    Some(Item::Slot(u)) => (u.counters, u.banished, u.cards),
                    Some(Item::Minion(m)) => (m.blood, true, vec![]),
                    _ => (0, false, vec![]),
                };
                let waiting = card.copy.and_then(|c| self.waiting.remove(&c));
                cards.extend(waiting.unwrap_or_default().iter().map(Item::card));
                // A banished vampire face up to every seat.
                let region = &mut self.v.seats[s].uncontrolled;
                let at = r.slot.map_or(region.len(), |i| region.len().min(i.into()));
                let slot = SlotView {
                    card: if banished { told } else { card },
                    counters,
                    banished,
                    cards,
                };
                region.insert(at, slot);
            }
            (Zone::Uncontrolled, Some(v)) => {
                if let Some(u) = self
                    .v
                    .seats
                    .iter_mut()
                    .flat_map(|s| s.uncontrolled.iter_mut())
                    .find(|u| u.card.copy == Some(v))
                {
                    u.cards.push(card);
                } else {
                    self.waiting.entry(v).or_default().push(Item::Card(card));
                }
            }
            (z, _) => {
                let seat = &mut self.v.seats[s];
                let v = match z {
                    Zone::Hand => &mut seat.hand,
                    Zone::Library => &mut seat.library,
                    Zone::Crypt => &mut seat.crypt,
                    Zone::AshHeap => &mut seat.ash_heap,
                    Zone::Removed => &mut seat.removed,
                    Zone::Limbo => &mut seat.limbo,
                    _ => return,
                };
                v.push(card);
            }
        }
    }

    /// `seat` out of the game, its table with it (an oust, a withdrawal).
    fn leaves(&mut self, seat: u8) {
        let gone = &mut self.v.seats[seat as usize];
        gone.ousted = true;
        for z in [
            &mut gone.crypt,
            &mut gone.library,
            &mut gone.hand,
            &mut gone.ash_heap,
            &mut gone.removed,
            &mut gone.limbo,
        ] {
            z.clear();
        }
        gone.uncontrolled.clear();
        gone.training.clear();
        gone.ready.clear();
        gone.torpor.clear();
        gone.incapacitated.clear();
        gone.in_play.clear();
        // The cards it controlled on others' minions, gone with it.
        for s in &mut self.v.seats {
            for m in s.minions_mut() {
                m.cards.retain(|c| c.controller != seat);
            }
        }
        for i in 0..self.v.seats.len() as u8 {
            let (prey, predator) = (self.seat_from(i, 1), self.seat_from(i, -1));
            let s = &mut self.v.seats[i as usize];
            (s.prey, s.predator) = (prey, predator);
        }
    }

    /// The next seat not ousted from `s`, either way.
    fn seat_from(&self, s: u8, step: i32) -> u8 {
        let n = self.v.seats.len() as i32;
        let mut i = s as i32;
        loop {
            i = (i + step).rem_euclid(n);
            if !self.v.seats[i as usize].ousted || i == s as i32 {
                return i as u8;
            }
        }
    }

    pub fn apply(&mut self, e: &EventKind) {
        let v = &mut self.v;
        match e {
            EventKind::Decided { decision, .. } | EventKind::AutoPassed { decision, .. } => {
                v.decision = decision + 1
            }
            EventKind::Turn { turn, active } => {
                v.turn = *turn;
                v.active = *active;
                v.phase = Phase::Unlock;
                v.transfers = None;
            }
            EventKind::Phase { phase } => {
                v.phase = *phase;
                v.transfers = None;
            }
            EventKind::Transfers { left } => v.transfers = Some(*left),
            EventKind::Pool { seat, pool, .. } => v.seats[*seat as usize].pool = *pool,
            EventKind::Blood { minion, blood, .. } => {
                if let Some(m) = self.minion_mut(*minion) {
                    m.blood = *blood;
                }
            }
            EventKind::EntersPlay {
                card,
                minion: Some(kind),
                ..
            } => {
                if let Some(c) = card.copy {
                    self.kinds.insert(c, (*kind, card.numbered));
                    // Back from a contest: unlocked, else told so.
                    if let Some(m) = self.minion_mut(c) {
                        m.kind = *kind;
                        m.card.numbered = card.numbered;
                        m.contested = false;
                        m.locked = false;
                    }
                }
            }
            EventKind::EntersPlay {
                card: Card { copy: Some(c), .. },
                effect: false,
                ..
            } => {
                if let Some(x) = self.in_play_mut(*c) {
                    (x.contested, x.locked) = (false, false);
                }
            }
            EventKind::CardMoves { card, from, to } => {
                let item = self.take(from, card);
                self.put(from, to, *card, item);
            }
            // Out of play in a contest: a minion with no title, a card on
            // a contested minion with it (`contested` its own contest's).
            EventKind::LeavesPlay {
                card: card @ Card { copy: Some(c), .. },
                contested: true,
                ..
            } => {
                let on_contested = self.v.seats.iter().any(|s| {
                    s.minions()
                        .any(|m| m.contested && m.cards.iter().any(|x| x.card.copy == Some(*c)))
                });
                if let Some(m) = self.minion_mut(*c) {
                    m.contested = true;
                    m.title = None;
                    m.card.numbered = card.numbered;
                } else if let Some(x) = self.in_play_mut(*c).filter(|_| !on_contested) {
                    x.contested = true;
                }
            }
            EventKind::Torpor { minion, enters } => {
                for s in &mut self.v.seats {
                    let (from, to) = if *enters {
                        (&mut s.ready, &mut s.torpor)
                    } else {
                        (&mut s.torpor, &mut s.ready)
                    };
                    if let Some(i) = from.iter().position(|m| m.card.copy == Some(*minion)) {
                        to.push(from.remove(i));
                    }
                }
            }
            EventKind::Incapacitated { minion, enters } => {
                for s in &mut self.v.seats {
                    let (from, to) = if *enters {
                        (&mut s.ready, &mut s.incapacitated)
                    } else {
                        (&mut s.incapacitated, &mut s.ready)
                    };
                    if let Some(i) = from.iter().position(|m| m.card.copy == Some(*minion)) {
                        to.push(from.remove(i));
                    }
                }
            }
            EventKind::Locked { copy, locked } => {
                if let Some(m) = self.minion_mut(*copy) {
                    m.locked = *locked;
                } else if let Some(c) = self.in_play_mut(*copy) {
                    c.locked = *locked;
                }
            }
            EventKind::Counters {
                copy,
                slot,
                counters,
                ..
            } => {
                if let Some(c) = copy {
                    if let Some(x) = self.in_play_mut(*c) {
                        x.counters = *counters;
                    }
                } else if let Some(r) = slot {
                    if let Some(u) = self.v.seats[r.seat as usize]
                        .uncontrolled
                        .get_mut(r.slot as usize)
                    {
                        u.counters = *counters;
                    }
                }
            }
            EventKind::Controller { copy, seat } => {
                if let Some(c) = self.in_play_mut(*copy) {
                    c.controller = *seat;
                }
            }
            EventKind::Capacity { minion, capacity } => {
                if let Some(m) = self.minion_mut(*minion) {
                    m.capacity = *capacity;
                }
            }
            EventKind::Title { minion, title } => {
                if let Some(m) = self.minion_mut(*minion) {
                    m.title = title.clone();
                }
            }
            EventKind::Markers { minion, markers } => {
                if let Some(m) = self.minion_mut(*minion) {
                    m.markers = markers.clone();
                }
            }
            EventKind::Traits { minion, traits } => {
                if let Some(m) = self.minion_mut(*minion) {
                    m.traits = traits.clone();
                }
            }
            EventKind::VictoryPoints { seat, vp_halves } => {
                self.v.seats[*seat as usize].vp_halves = *vp_halves
            }
            EventKind::Ousted { seat } => self.leaves(*seat),
            EventKind::WithdrawalAnnounced { seat } => {
                self.v.seats[*seat as usize].withdrawing = true
            }
            EventKind::WithdrawalBroken { seat } => {
                self.v.seats[*seat as usize].withdrawing = false
            }
            EventKind::Withdrew { seat } => {
                self.leaves(*seat);
                let gone = &mut self.v.seats[*seat as usize];
                (gone.withdrawing, gone.withdrew) = (false, true);
            }
            EventKind::Edge { seat } => {
                for s in &mut self.v.seats {
                    s.edge = Some(s.seat) == *seat;
                }
            }
            EventKind::ActingChanges { acting, .. } => {
                if let (Some(a), Some(c)) = (self.v.action.as_mut(), acting.copy) {
                    a.acting = c;
                }
            }
            EventKind::ActionState { action } => self.v.action = Some((**action).clone()),
            EventKind::ActionEnds { .. } => self.v.action = None,
            EventKind::ReferendumState { referendum } => {
                self.v.referendum = Some((**referendum).clone())
            }
            EventKind::ReferendumEnds { .. } => self.v.referendum = None,
            EventKind::CombatState { combat } => self.v.combat = Some((**combat).clone()),
            EventKind::CombatEnds => self.v.combat = None,
            _ => {}
        }
    }
}

/// A copy, for the places' cards: what orders a place.
trait Copied: PartialEq + Clone {
    fn copy(&self) -> Option<u32>;
}

impl Copied for Card {
    fn copy(&self) -> Option<u32> {
        self.copy
    }
}

impl Copied for SlotView {
    fn copy(&self) -> Option<u32> {
        self.card.copy
    }
}

impl Copied for InPlayView {
    fn copy(&self) -> Option<u32> {
        self.card.copy
    }
}

impl Copied for MinionView {
    fn copy(&self) -> Option<u32> {
        self.card.copy
    }
}

/// `a` in `b`'s order: each card where `b` shows its equal, else its
/// copy; counted by place when only the order differed.
fn like<T: Copied>(
    a: &mut Vec<T>,
    b: &[T],
    what: &'static str,
    unordered: &mut BTreeMap<&'static str, usize>,
) {
    if a.as_slice() == b {
        return;
    }
    let mut rest = std::mem::take(a);
    let mut equal = true;
    for x in b {
        let i = rest.iter().position(|y| y == x).or_else(|| {
            equal = false;
            x.copy()
                .and_then(|c| rest.iter().position(|y| y.copy() == Some(c)))
        });
        if let Some(i) = i {
            a.push(rest.remove(i));
        }
    }
    if equal && rest.is_empty() {
        *unordered.entry(what).or_default() += 1;
    }
    a.extend(rest);
}

/// `a`'s places in `b`'s order.
fn order_as(a: &mut TableView, b: &TableView, unordered: &mut BTreeMap<&'static str, usize>) {
    for (s, t) in a.seats.iter_mut().zip(&b.seats) {
        like(&mut s.crypt, &t.crypt, "crypt", unordered);
        like(&mut s.library, &t.library, "library", unordered);
        like(&mut s.hand, &t.hand, "hand", unordered);
        like(&mut s.ash_heap, &t.ash_heap, "ash_heap", unordered);
        like(&mut s.removed, &t.removed, "removed", unordered);
        like(&mut s.limbo, &t.limbo, "limbo", unordered);
        like(
            &mut s.uncontrolled,
            &t.uncontrolled,
            "uncontrolled",
            unordered,
        );
        for (u, w) in s.uncontrolled.iter_mut().zip(&t.uncontrolled) {
            like(&mut u.cards, &w.cards, "slot cards", unordered);
        }
        like(&mut s.ready, &t.ready, "ready", unordered);
        like(&mut s.torpor, &t.torpor, "torpor", unordered);
        like(
            &mut s.incapacitated,
            &t.incapacitated,
            "incapacitated",
            unordered,
        );
        like(&mut s.in_play, &t.in_play, "in_play", unordered);
        like(&mut s.training, &t.training, "training", unordered);
        let inner = |c: &mut InPlayView, d: &InPlayView, unordered: &mut BTreeMap<_, _>| {
            like(&mut c.stored, &d.stored, "stored", unordered);
        };
        for (m, n) in s.minions_mut().zip(t.minions()) {
            like(&mut m.cards, &n.cards, "minion cards", unordered);
            for (c, d) in m.cards.iter_mut().zip(&n.cards) {
                inner(c, d, unordered);
            }
        }
        for (c, d) in s.in_play.iter_mut().zip(&t.in_play) {
            inner(c, d, unordered);
        }
    }
}

/// Where two JSON values differ: each path, numbers as `#`, with both
/// values.
fn diff(a: &serde_json::Value, b: &serde_json::Value, at: String, out: &mut Vec<(String, String)>) {
    use serde_json::Value as V;
    match (a, b) {
        (V::Object(x), V::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let (p, q) = (x.get(k).unwrap_or(&V::Null), y.get(k).unwrap_or(&V::Null));
                diff(p, q, format!("{at}.{k}"), out);
            }
        }
        (V::Array(x), V::Array(y)) if x.len() == y.len() => {
            // An element by its copy, if it shows one (`HOST_REDUCE`).
            let copy = |v: &V| {
                let c = v.get("card").unwrap_or(v).get("copy")?.as_u64()?;
                Some(format!("c{c}"))
            };
            for (p, q) in x.iter().zip(y) {
                let c = if std::env::var_os("HOST_REDUCE").is_some() {
                    copy(q).unwrap_or_default()
                } else {
                    String::new()
                };
                diff(p, q, format!("{at}[{c}]"), out);
            }
        }
        _ if a != b => {
            let s = |v: &V| {
                let s = v.to_string();
                s.chars().take(300).collect::<String>()
            };
            out.push((at, format!("{} -> {}", s(a), s(b))));
        }
        _ => {}
    }
}

/// The reduced table against the view: the paths that differ (none:
/// the same), each place in the view's order first.
pub fn check(
    reduced: &mut TableView,
    view: &TableView,
    unordered: &mut BTreeMap<&'static str, usize>,
) -> Vec<(String, String)> {
    order_as(reduced, view, unordered);
    // A search's cards shown in its seat's library: the decision's (the
    // viewer's own, another's never shown).
    for (s, t) in reduced.seats.iter_mut().zip(&view.seats) {
        if Some(s.seat) == view.viewer && s.library.len() == t.library.len() {
            for (a, b) in s.library.iter_mut().zip(&t.library) {
                a.id = b.id;
            }
        }
    }
    if reduced == view {
        return vec![];
    }
    let mut out = Vec::new();
    let (a, b) = (
        serde_json::to_value(&*reduced).unwrap(),
        serde_json::to_value(view).unwrap(),
    );
    diff(&a, &b, String::new(), &mut out);
    out
}
