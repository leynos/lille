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
