//! Buffered damage events awaiting DBSP ingestion.

use bevy::prelude::Resource;

use crate::dbsp_circuit::DamageEvent;

#[derive(Resource, Default)]
pub struct DamageInbox {
    events: Vec<DamageEvent>,
}

impl DamageInbox {
    pub fn push(&mut self, event: DamageEvent) {
        self.events.push(event);
    }

    pub fn extend<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = DamageEvent>,
    {
        self.events.extend(events);
    }

    pub fn drain(&mut self) -> std::vec::Drain<'_, DamageEvent> {
        self.events.drain(..)
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dbsp_circuit::{DamageEvent, DamageSource};
    use rstest::rstest;

    fn sample_event(entity: u64, amount: u16, seq: Option<u32>) -> DamageEvent {
        DamageEvent {
            entity,
            amount,
            source: DamageSource::External,
            at_tick: 1,
            seq,
        }
    }

    #[rstest]
    fn push_appends_single_event() {
        let mut inbox = DamageInbox::default();
        let event = sample_event(1, 10, Some(1));
        assert!(inbox.is_empty());
        inbox.push(event);
        assert!(!inbox.is_empty());
        let drained: Vec<_> = inbox.drain().collect();
        assert_eq!(drained, vec![event]);
        assert!(inbox.is_empty());
    }

    #[rstest]
    fn extend_appends_multiple_events() {
        let mut inbox = DamageInbox::default();
        let first = sample_event(1, 5, Some(1));
        let second = sample_event(2, 7, None);
        inbox.extend(vec![first, second]);
        assert!(!inbox.is_empty());
        let drained: Vec<_> = inbox.drain().collect();
        assert_eq!(drained, vec![first, second]);
        assert!(inbox.is_empty());
    }
}
