//! Observer regions and enter/leave event tracking.
//!
//! An [`Observer`] is a query shape that the world remembers across
//! frames. After every [`AoiWorld::update_observer`] call (or at the
//! moment of [`AoiWorld::create_observer`]), the world re-runs the
//! underlying spatial query and **diffs** the new hit set against the
//! previous frame's set:
//!
//! - Ids in the new set but not the old → [`AoiEvent::Enter`]
//! - Ids in the old set but not the new → [`AoiEvent::Leave`]
//!
//! Events accumulate inside the observer's `pending` queue until the
//! game pulls them out via [`AoiWorld::drain_events`], which clears the
//! queue. Events do **not** persist across `drain_events` calls — if
//! you skip a frame, you lose the per-frame events but the next diff
//! still observes the cumulative set transition.

use std::collections::HashSet;

use crate::world::EntityId;

/// Stable handle for an observer inside an [`crate::AoiWorld`]. Like
/// [`EntityId`], may be recycled after [`crate::AoiWorld::remove_observer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "vdp", derive(serde::Serialize))]
#[cfg_attr(feature = "vdp", serde(transparent))]
pub struct ObserverId(pub u32);

/// A transition event reported by an observer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AoiEvent {
    /// Entity entered this observer's region since the last
    /// `update_observer`.
    Enter(EntityId),
    /// Entity left this observer's region since the last
    /// `update_observer`.
    Leave(EntityId),
}

// Hand-rolled serialization so the wire format is a flat
// `{"type": "enter", "id": 7}` instead of serde's default newtype-variant
// shape `{"Enter": 7}`. Going through `derive(Serialize)` would conflict
// with `EntityId`'s `#[serde(transparent)]` because `tag = "..."` requires
// the inner value to be a struct/map.
#[cfg(feature = "vdp")]
impl serde::Serialize for AoiEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let (kind, id) = match self {
            AoiEvent::Enter(id) => ("enter", id),
            AoiEvent::Leave(id) => ("leave", id),
        };
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("type", kind)?;
        map.serialize_entry("id", id)?;
        map.end()
    }
}

pub(crate) struct Observer {
    pub current: HashSet<EntityId>,
    pub pending: Vec<AoiEvent>,
    /// Optional per-observer filter, applied to every candidate hit
    /// before the diff. See [`crate::AoiFilter`] for the call signature.
    ///
    /// **Critical invariant**: the filter is part of the observer's
    /// persistent state. If the game replaces it with
    /// [`crate::AoiWorld::set_observer_filter`], the *next*
    /// `update_observer` will diff against a different effective hit
    /// set and may emit Enter/Leave events for entities that haven't
    /// physically moved — those events truthfully reflect the
    /// observer's *visibility* changing, which is exactly what LOD
    /// systems want.
    pub filter: Option<Box<crate::world::AoiFilter>>,
}

impl Observer {
    pub fn new() -> Self {
        Self {
            current: HashSet::new(),
            pending: Vec::new(),
            filter: None,
        }
    }

    /// Replace `current` with `new_hits`, queueing Enter/Leave events
    /// for the symmetric difference.
    pub fn diff_into_pending(&mut self, new_hits: HashSet<EntityId>) {
        for &id in new_hits.difference(&self.current) {
            self.pending.push(AoiEvent::Enter(id));
        }
        for &id in self.current.difference(&new_hits) {
            self.pending.push(AoiEvent::Leave(id));
        }
        self.current = new_hits;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(slice: &[u32]) -> HashSet<EntityId> {
        slice.iter().copied().map(EntityId).collect()
    }

    #[test]
    fn first_diff_emits_only_enters() {
        let mut o = Observer::new();
        o.diff_into_pending(ids(&[1, 2, 3]));
        assert_eq!(o.pending.len(), 3);
        assert!(o.pending.iter().all(|e| matches!(e, AoiEvent::Enter(_))));
    }

    #[test]
    fn no_change_emits_nothing() {
        let mut o = Observer::new();
        o.diff_into_pending(ids(&[1, 2]));
        o.pending.clear();
        o.diff_into_pending(ids(&[1, 2]));
        assert!(o.pending.is_empty());
    }

    #[test]
    fn departures_emit_leave() {
        let mut o = Observer::new();
        o.diff_into_pending(ids(&[1, 2, 3]));
        o.pending.clear();
        o.diff_into_pending(ids(&[1])); // 2, 3 left
        let leaves: Vec<_> = o
            .pending
            .iter()
            .filter_map(|e| match e {
                AoiEvent::Leave(EntityId(i)) => Some(*i),
                _ => None,
            })
            .collect();
        assert_eq!(leaves.len(), 2);
        assert!(leaves.contains(&2));
        assert!(leaves.contains(&3));
    }

    #[test]
    fn mixed_enter_and_leave_in_one_diff() {
        let mut o = Observer::new();
        o.diff_into_pending(ids(&[1, 2]));
        o.pending.clear();
        o.diff_into_pending(ids(&[2, 3])); // 1 left, 3 entered
        assert!(o.pending.contains(&AoiEvent::Leave(EntityId(1))));
        assert!(o.pending.contains(&AoiEvent::Enter(EntityId(3))));
        assert_eq!(o.pending.len(), 2);
    }
}
