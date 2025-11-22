//! Tests for incremental maintenance of the entity ID mapping.

use anyhow::{ensure, Context, Result};
use bevy::prelude::*;
use rstest::rstest;

use lille::components::DdlogId;
use lille::dbsp_sync::{cache_state_for_dbsp_system, init_dbsp_system, DamageInbox, DbspState};
use lille::init_world_handle_system;

/// Returns an [`App`] with the DBSP cache system wired.
///
/// # Examples
///
/// ```
/// # use anyhow::Result;
/// # fn demo() -> Result<()> {
/// let mut app = build_app()?;
/// # Ok(())
/// # }
/// ```
fn build_app() -> Result<App> {
    let mut app = App::new();
    init_dbsp_system(app.world_mut()).context("failed to initialise DbspState")?;
    app.world_mut().init_resource::<DamageInbox>();
    app.add_systems(Startup, init_world_handle_system);
    app.add_systems(Update, cache_state_for_dbsp_system);
    Ok(app)
}

#[rstest]
fn removes_entity_from_id_map_when_ddlog_id_removed() -> Result<()> {
    let mut app = build_app()?;
    let entity = app
        .world_mut()
        .spawn((DdlogId(1), Transform::default()))
        .id();

    app.update();
    {
        let state = app.world().non_send_resource::<DbspState>();
        ensure!(
            state.entity_for_id(1) == Some(entity),
            "expected entity 1 to be registered"
        );
    }

    app.world_mut().entity_mut(entity).remove::<DdlogId>();
    app.update();

    let state = app.world().non_send_resource::<DbspState>();
    ensure!(
        state.entity_for_id(1).is_none(),
        "expected entity 1 to be removed from map"
    );
    Ok(())
}

#[rstest]
fn updates_id_map_when_ddlog_id_changed() -> Result<()> {
    let mut app = build_app()?;
    let entity = app
        .world_mut()
        .spawn((DdlogId(1), Transform::default()))
        .id();

    app.update();

    app.world_mut().entity_mut(entity).insert(DdlogId(2));

    app.update();

    let state = app.world().non_send_resource::<DbspState>();
    ensure!(
        state.entity_for_id(1).is_none(),
        "old identifier should be removed"
    );
    ensure!(
        state.entity_for_id(2) == Some(entity),
        "updated identifier should map to entity"
    );
    Ok(())
}
