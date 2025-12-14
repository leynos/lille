//! Observer-driven routes for DBSP-facing events.
//!
//! This module is a feature-gated spike evaluating Bevy's Observers V1 API for
//! routing DBSP ingress events. The intent is to reduce boilerplate by allowing
//! systems to emit events without taking `ResMut` handles directly, while still
//! keeping the DBSP circuit authoritative.
//!
//! The spike currently covers only damage ingress and keeps the existing
//! `DamageInbox`-driven DBSP tick behaviour unchanged.

use bevy::ecs::prelude::On;
use bevy::prelude::*;

use crate::dbsp_circuit::DamageEvent;

use super::DamageInbox;

/// Event used to enqueue a [`DamageEvent`] for DBSP ingestion.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbspDamageIngress {
    event: DamageEvent,
}

impl DbspDamageIngress {
    /// Wrap a [`DamageEvent`] for observer-based routing.
    #[must_use]
    pub const fn new(event: DamageEvent) -> Self {
        Self { event }
    }

    /// Returns the wrapped [`DamageEvent`].
    #[must_use]
    pub const fn damage(&self) -> DamageEvent {
        self.event
    }
}

impl From<DamageEvent> for DbspDamageIngress {
    fn from(event: DamageEvent) -> Self {
        Self::new(event)
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Observer systems must take On<T> by value for Events V2."
)]
pub(crate) fn buffer_damage_ingress(event: On<DbspDamageIngress>, mut inbox: ResMut<DamageInbox>) {
    inbox.push(event.event().damage());
}
