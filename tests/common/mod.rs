//! Shared test fixtures and helpers for DBSP integration tests.

use anyhow::{ensure, Context, Result};
use bevy::prelude::*;
use lille::dbsp_sync::DbspState;
use lille::{DamageInbox, DbspPlugin, DdlogId, Health};

/// Builder for test `App` instances configured for DBSP scenarios.
pub struct DbspTestAppBuilder {
    app: App,
}

impl DbspTestAppBuilder {
    /// Create a new test app with `MinimalPlugins` and `DbspPlugin` installed.
    #[must_use]
    pub fn new() -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(DbspPlugin);
        Self { app }
    }

    /// Spawn an entity with `DdlogId`, `Transform`, and `Health` components.
    #[must_use]
    pub fn spawn_entity_with_health(
        mut self,
        ddlog_id: i64,
        current: u16,
        max: u16,
    ) -> (Self, Entity) {
        let entity = self
            .app
            .world_mut()
            .spawn((
                DdlogId(ddlog_id),
                Transform::default(),
                Health { current, max },
            ))
            .id();
        (self, entity)
    }

    /// Run one update cycle to prime the DBSP circuit and resources.
    #[must_use]
    pub fn prime(mut self) -> Self {
        self.app.update();
        self
    }

    /// Build and return the configured `App`.
    #[must_use]
    pub fn build(self) -> App {
        self.app
    }

    /// Build and return both the `App` and a tracked entity.
    #[must_use]
    pub fn build_with_entity(self, entity: Entity) -> (App, Entity) {
        (self.build(), entity)
    }
}

impl Default for DbspTestAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Assertion helpers for common DBSP state checks.
pub struct DbspAssertions;

impl DbspAssertions {
    /// Fetch the `DbspState` non-send resource.
    pub fn get_state(app: &App) -> Result<&DbspState> {
        app.world()
            .get_non_send_resource::<DbspState>()
            .context("DbspState non-send resource missing")
    }

    /// Fetch the `Health` component for an entity.
    pub fn get_health(app: &App, entity: Entity) -> Result<&Health> {
        app.world()
            .get::<Health>(entity)
            .context("entity missing Health component")
    }

    /// Fetch the `DamageInbox` resource.
    pub fn get_damage_inbox(app: &App) -> Result<&DamageInbox> {
        app.world()
            .get_resource::<DamageInbox>()
            .context("DamageInbox resource missing")
    }

    /// Assert the current health equals `expected`.
    pub fn assert_health_current(app: &App, entity: Entity, expected: u16) -> Result<()> {
        let health = Self::get_health(app, entity)?;
        ensure!(
            health.current == expected,
            "Expected health.current = {expected}, got {}",
            health.current
        );
        Ok(())
    }

    /// Assert the duplicate counter equals `expected`.
    pub fn assert_duplicate_count(app: &App, expected: u64) -> Result<()> {
        let state = Self::get_state(app)?;
        ensure!(
            state.applied_health_duplicates() == expected,
            "Expected {expected} duplicates, got {}",
            state.applied_health_duplicates()
        );
        Ok(())
    }

    /// Assert the damage inbox is empty.
    pub fn assert_inbox_empty(app: &App) -> Result<()> {
        let inbox = Self::get_damage_inbox(app)?;
        ensure!(inbox.is_empty(), "DamageInbox should be empty");
        Ok(())
    }
}
