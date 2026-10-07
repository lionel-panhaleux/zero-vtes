//! Copy identity (docs/human-play.md "Design items"): a stable id per
//! physical card across steps, as clients key their sprites by card copy.
//!
//! The engine identifies only what is in play (the minion and card arenas,
//! never reused); its zones are lists of card *types*. The tracker follows
//! one viewer's views step to step: arena entries keep their copy; each
//! zone keeps the copies that stayed (matched in order); a copy that
//! arrives somewhere claims one that left elsewhere — the same card first,
//! then a hidden card of the same kind (a draw: a hidden library card left,
//! a known hand card arrived); what claims nothing is a fresh copy (a
//! shuffled region is re-identified, as Succubus Club's own server does).
//! It reads only the viewer's legal view, so the ids it assigns carry no
//! hidden information.

use std::collections::{BTreeMap, HashMap};

/// What a copy shows: a crypt or library card, or a hidden one of either.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Face {
    Crypt(u16),
    Library(u16),
    HiddenCrypt,
    HiddenLibrary,
}

impl Face {
    fn is_crypt(self) -> bool {
        matches!(self, Face::Crypt(_) | Face::HiddenCrypt)
    }

    fn hidden(self) -> bool {
        matches!(self, Face::HiddenCrypt | Face::HiddenLibrary)
    }
}

/// An arena entry: a minion (a vampire; a token ally, which has no card
/// in play), or a library card in play (an ally or created vampire is its
/// card).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    Minion(u32),
    Card(u32),
}

/// A zone: a list of copies, per seat (or per card in play, for the cards
/// stored on it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ZoneId {
    Seat(u8, SeatZone),
    /// Cards stored on a card in play (arena index).
    Stored(u32),
    /// The cards a banished vampire (arena index) carries in the
    /// uncontrolled region, out of play until it returns (rules-qa 25.1).
    Banished(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SeatZone {
    Crypt,
    Library,
    Hand,
    AshHeap,
    Removed,
    Uncontrolled,
    /// Cards the seat is playing, not resolved yet.
    Limbo,
}

/// Where everything is, as one view shows it: arena entries (with their
/// face and seat) and zones (faces in order; the seat each belongs to).
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub arena: BTreeMap<Key, (Face, u8)>,
    pub zones: BTreeMap<ZoneId, (Vec<Face>, u8)>,
}

/// The copies assigned to a layout.
#[derive(Clone, Debug, Default)]
pub struct Copies {
    pub arena: HashMap<Key, u32>,
    pub zones: HashMap<ZoneId, Vec<u32>>,
}

impl Copies {
    pub fn arena(&self, k: Key) -> u32 {
        self.arena[&k]
    }

    pub fn zone(&self, z: ZoneId) -> &[u32] {
        self.zones.get(&z).map_or(&[], |v| v.as_slice())
    }
}

#[derive(Clone, Debug, Default)]
pub struct Tracker {
    prev: Layout,
    copies: Copies,
    next: u32,
}

/// A copy that left its place this step, and the seat it belonged to.
struct Left {
    copy: u32,
    face: Face,
    seat: u8,
    /// The zone it left (`None`: the arena).
    zone: Option<ZoneId>,
    taken: bool,
}

/// A place a copy arrived at this step.
#[derive(Clone, Copy)]
enum Arrival {
    Arena(Key),
    Zone(ZoneId, usize),
}

impl Arrival {
    fn zone(self) -> Option<ZoneId> {
        match self {
            Arrival::Arena(_) => None,
            Arrival::Zone(z, _) => Some(z),
        }
    }
}

impl Tracker {
    /// Assign copies to `now`, continuing from the previous layout.
    pub fn update(&mut self, now: &Layout) -> &Copies {
        let mut copies = Copies::default();
        let mut left: Vec<Left> = Vec::new();
        let mut arrivals: Vec<(Arrival, Face, u8)> = Vec::new();

        for (&k, &(face, seat)) in &self.prev.arena {
            if !now.arena.contains_key(&k) {
                left.push(Left {
                    copy: self.copies.arena[&k],
                    face,
                    seat,
                    zone: None,
                    taken: false,
                });
            }
        }
        for (&k, &(face, seat)) in &now.arena {
            match self.prev.arena.get(&k) {
                Some(_) => {
                    copies.arena.insert(k, self.copies.arena[&k]);
                }
                None => arrivals.push((Arrival::Arena(k), face, seat)),
            }
        }

        let empty = (Vec::new(), 0);
        let mut zone_ids: Vec<ZoneId> = self.prev.zones.keys().copied().collect();
        zone_ids.extend(now.zones.keys().copied());
        zone_ids.sort();
        zone_ids.dedup();
        for z in zone_ids {
            let (before, seat_before) = self.prev.zones.get(&z).unwrap_or(&empty);
            let before_copies = self.copies.zone(z);
            let (after, seat) = now.zones.get(&z).unwrap_or(&empty);
            let mut kept = vec![None; after.len()];
            let mut stayed = vec![false; before.len()];
            // Order-preserving greedy match: each face takes the first
            // unmatched equal face at or after the last match.
            let mut from = 0;
            for (j, f) in after.iter().enumerate() {
                if let Some(i) = (from..before.len()).find(|&i| before[i] == *f) {
                    kept[j] = Some(before_copies[i]);
                    stayed[i] = true;
                    from = i + 1;
                }
            }
            for (i, &s) in stayed.iter().enumerate() {
                if !s {
                    left.push(Left {
                        copy: before_copies[i],
                        face: before[i],
                        seat: *seat_before,
                        zone: Some(z),
                        taken: false,
                    });
                }
            }
            let mut list = Vec::with_capacity(after.len());
            for (j, k) in kept.into_iter().enumerate() {
                match k {
                    Some(c) => list.push(c),
                    None => {
                        list.push(u32::MAX);
                        arrivals.push((Arrival::Zone(z, j), after[j], *seat));
                    }
                }
            }
            if !list.is_empty() {
                copies.zones.insert(z, list);
            }
        }

        // Claims, most specific first: the same card from the same seat,
        // from any seat, then a hidden card of the same kind (or a known
        // card into a hidden place) from the same seat.
        type Rule = fn(&Left, Face, u8) -> bool;
        let rules: [Rule; 3] = [
            |l, f, s| l.face == f && l.seat == s,
            |l, f, _| l.face == f,
            |l, f, s| {
                l.seat == s && l.face.is_crypt() == f.is_crypt() && (l.face.hidden() || f.hidden())
            },
        ];
        let mut claimed: Vec<Option<u32>> = vec![None; arrivals.len()];
        for rule in rules {
            for (a, &(at, face, seat)) in arrivals.iter().enumerate() {
                if claimed[a].is_some() {
                    continue;
                }
                let Some(l) = left.iter_mut().find(|l| !l.taken && rule(l, face, seat)) else {
                    continue;
                };
                l.taken = true;
                claimed[a] = Some(l.copy);
                // A known card arriving from a hidden library while the
                // seat's hidden hand kept its size: it was played or
                // discarded from the hand and replaced — the hand's copy
                // arrives, the library's joins the hand. (A card milled
                // looks the same; the copies are hidden, only the path
                // shown differs.)
                let hand = ZoneId::Seat(l.seat, SeatZone::Hand);
                let from_library = l.zone == Some(ZoneId::Seat(l.seat, SeatZone::Library));
                if from_library && l.face.hidden() && !face.hidden() && at.zone() != Some(hand) {
                    let faces = now
                        .zones
                        .get(&hand)
                        .map(|(f, _)| f.as_slice())
                        .unwrap_or(&[]);
                    let held = copies.zones.get_mut(&hand).and_then(|c| {
                        (0..c.len())
                            .rev()
                            .find(|&j| faces[j].hidden() && c[j] != u32::MAX)
                            .map(|j| &mut c[j])
                    });
                    if let Some(held) = held {
                        claimed[a] = Some(std::mem::replace(held, l.copy));
                    }
                }
            }
        }
        for (a, &(at, _, _)) in arrivals.iter().enumerate() {
            let copy = claimed[a].unwrap_or_else(|| {
                self.next += 1;
                self.next
            });
            match at {
                Arrival::Arena(k) => {
                    copies.arena.insert(k, copy);
                }
                Arrival::Zone(z, j) => copies.zones.get_mut(&z).expect("zone")[j] = copy,
            }
        }

        self.prev = now.clone();
        self.copies = copies;
        &self.copies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(seat: u8, z: SeatZone) -> ZoneId {
        ZoneId::Seat(seat, z)
    }

    fn layout(zones: &[(ZoneId, &[Face])], arena: &[(Key, Face)]) -> Layout {
        Layout {
            arena: arena.iter().map(|&(k, f)| (k, (f, 0))).collect(),
            zones: zones.iter().map(|&(z, f)| (z, (f.to_vec(), 0))).collect(),
        }
    }

    #[test]
    fn a_draw_moves_the_top_library_copy_to_hand() {
        let mut t = Tracker::default();
        let lib = zone(0, SeatZone::Library);
        let hand = zone(0, SeatZone::Hand);
        let h = Face::HiddenLibrary;
        let c0 = t
            .update(&layout(
                &[(lib, &[h, h, h]), (hand, &[Face::Library(5)])],
                &[],
            ))
            .clone();
        let c1 = t
            .update(&layout(
                &[
                    (lib, &[h, h]),
                    (hand, &[Face::Library(5), Face::Library(9)]),
                ],
                &[],
            ))
            .clone();
        assert_eq!(c1.zone(lib), &c0.zone(lib)[..2]);
        assert_eq!(c1.zone(hand), &[c0.zone(hand)[0], c0.zone(lib)[2]]);
    }

    #[test]
    fn a_played_card_keeps_its_copy_into_play_and_the_ash_heap() {
        let mut t = Tracker::default();
        let hand = zone(0, SeatZone::Hand);
        let ash = zone(0, SeatZone::AshHeap);
        let x = Face::Library(3);
        let c0 = t
            .update(&layout(&[(hand, &[Face::Library(1), x])], &[]))
            .clone();
        let copy = c0.zone(hand)[1];
        let c1 = t
            .update(&layout(
                &[(hand, &[Face::Library(1)])],
                &[(Key::Card(0), x)],
            ))
            .clone();
        assert_eq!(c1.arena(Key::Card(0)), copy);
        assert_eq!(c1.zone(hand), &c0.zone(hand)[..1]);
        let c2 = t
            .update(&layout(&[(hand, &[Face::Library(1)]), (ash, &[x])], &[]))
            .clone();
        assert_eq!(c2.zone(ash), &[copy]);
    }

    #[test]
    fn an_opponents_discard_leaves_from_the_hand() {
        let mut t = Tracker::default();
        let lib = zone(0, SeatZone::Library);
        let hand = zone(0, SeatZone::Hand);
        let ash = zone(0, SeatZone::AshHeap);
        let h = Face::HiddenLibrary;
        let c0 = t
            .update(&layout(&[(lib, &[h, h, h]), (hand, &[h, h])], &[]))
            .clone();
        // A card discarded and replaced: the hand keeps its size.
        let c1 = t
            .update(&layout(
                &[(lib, &[h, h]), (hand, &[h, h]), (ash, &[Face::Library(4)])],
                &[],
            ))
            .clone();
        assert_eq!(c1.zone(ash), &[c0.zone(hand)[1]]);
        assert_eq!(c1.zone(hand), &[c0.zone(hand)[0], c0.zone(lib)[2]]);
    }

    #[test]
    fn copies_are_unique() {
        let mut t = Tracker::default();
        let lib = zone(0, SeatZone::Library);
        let h = Face::HiddenLibrary;
        t.update(&layout(&[(lib, &[h; 5])], &[]));
        let c = t.update(&layout(
            &[(lib, &[h; 7])],
            &[(Key::Minion(0), Face::Crypt(2))],
        ));
        let mut all: Vec<u32> = c.zone(lib).to_vec();
        all.push(c.arena(Key::Minion(0)));
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n);
    }
}
