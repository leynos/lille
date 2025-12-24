#![cfg_attr(
    feature = "test-support",
    doc = "Behavioural tests for Block and `BlockSlope` attachment using rust-rspec."
)]
#![cfg_attr(
    not(feature = "test-support"),
    doc = "Behavioural tests require `test-support`."
)]
#![cfg(feature = "test-support")]
//! Behavioural test: `Block` and `BlockSlope` components are attached to tiles.
//!
//! This file contains a single test because it ticks the Bevy app under
//! `--all-features`, which initializes a render device and uses process-global
//! renderer state.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

#[path = "support/thread_safe_app.rs"]
mod thread_safe_app;

#[path = "support/rspec_runner.rs"]
mod rspec_runner;

#[path = "support/map_fixture.rs"]
mod map_fixture;

use std::sync::MutexGuard;

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TilePos;
use lille::components::{Block, BlockSlope};
use lille::map::{Collidable, LilleMapError, LilleMapSettings, MapAssetPath, SlopeProperties};
use lille::{DbspPlugin, LilleMapPlugin};
use map_test_plugins::CapturedMapErrors;
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::ThreadSafeApp;

const CUSTOM_PROPERTIES_MAP_PATH: &str = "maps/primary-isometric-custom-properties.tmx";
const MAX_LOAD_TICKS: usize = 100;

/// The fixture map uses a 2x2 tile grid; every tile carries `Collidable`.
/// All tiles also have `SlopeProperties` with `grad_x=0.25` and `grad_y=0.5`.
const EXPECTED_COLLIDABLE_COUNT: usize = 4;

#[derive(Debug, Clone)]
struct BlockAttachmentFixture {
    base: map_fixture::MapPluginFixtureBase,
}

impl BlockAttachmentFixture {
    fn bootstrap() -> Self {
        let mut app = App::new();
        map_test_plugins::add_map_test_plugins(&mut app);
        app.add_plugins(DbspPlugin);

        map_test_plugins::install_map_error_capture(&mut app);
        app.insert_resource(LilleMapSettings {
            primary_map: MapAssetPath::from(CUSTOM_PROPERTIES_MAP_PATH),
            should_spawn_primary_map: true,
            should_bootstrap_camera: false,
        });
        app.add_plugins(LilleMapPlugin);

        Self {
            base: map_fixture::MapPluginFixtureBase::new(app),
        }
    }

    fn app_guard(&self) -> MutexGuard<'_, ThreadSafeApp> {
        self.base.app_guard()
    }

    fn tick(&self) {
        self.base.tick();
    }

    fn tick_until_blocks_attached(&self, max_ticks: usize) -> bool {
        for _ in 0..max_ticks {
            self.tick();
            if self.blocks_ready() {
                return true;
            }
            if !self.captured_map_errors().is_empty() {
                return false;
            }
        }

        false
    }

    fn blocks_ready(&self) -> bool {
        self.block_count() == EXPECTED_COLLIDABLE_COUNT
    }

    fn collidable_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&Collidable>();
        query.iter(world).count()
    }

    fn block_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&Block>();
        query.iter(world).count()
    }

    fn blocks_with_collidable_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<(&Block, &Collidable)>();
        query.iter(world).count()
    }

    fn blocks_without_collidable_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query_filtered::<&Block, Without<Collidable>>();
        query.iter(world).count()
    }

    fn block_coordinates(&self) -> Vec<(i32, i32)> {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&Block>();
        query.iter(world).map(|b| (b.x, b.y)).collect()
    }

    fn tile_positions(&self) -> Vec<(u32, u32)> {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query_filtered::<&TilePos, With<Collidable>>();
        query.iter(world).map(|t| (t.x, t.y)).collect()
    }

    fn block_ids(&self) -> Vec<i64> {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&Block>();
        query.iter(world).map(|b| b.id).collect()
    }

    fn captured_map_errors(&self) -> Vec<LilleMapError> {
        let app = self.app_guard();
        app.world()
            .get_resource::<CapturedMapErrors>()
            .map(|errors| errors.0.clone())
            .unwrap_or_default()
    }

    fn all_blocks_have_z_zero(&self) -> bool {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&Block>();
        query.iter(world).all(|block| block.z == 0)
    }

    fn coordinates_match(&self) -> bool {
        let blocks = self.block_coordinates();
        let tiles = self.tile_positions();

        if blocks.len() != tiles.len() {
            return false;
        }

        #[expect(
            clippy::cast_possible_wrap,
            reason = "Test tile coordinates fit in i32."
        )]
        tiles
            .iter()
            .all(|(tx, ty)| blocks.contains(&(*tx as i32, *ty as i32)))
    }

    // --- BlockSlope query methods ---

    fn block_slope_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&BlockSlope>();
        query.iter(world).count()
    }

    fn blocks_with_slopes_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<(&Block, &BlockSlope)>();
        query.iter(world).count()
    }

    fn slopes_without_blocks_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query_filtered::<&BlockSlope, Without<Block>>();
        query.iter(world).count()
    }

    fn slope_properties_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&SlopeProperties>();
        query.iter(world).count()
    }

    fn block_slope_ids_match_block_ids(&self) -> bool {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<(&Block, &BlockSlope)>();
        query.iter(world).all(|(b, s)| b.id == s.block_id)
    }

    fn all_slopes_have_expected_gradients(&self, expected_x: f64, expected_y: f64) -> bool {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&BlockSlope>();
        query.iter(world).all(|s| {
            (s.grad_x.into_inner() - expected_x).abs() < f64::EPSILON
                && (s.grad_y.into_inner() - expected_y).abs() < f64::EPSILON
        })
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "rspec-style tests with before_each and multiple then clauses are inherently verbose"
)]
fn map_plugin_attaches_blocks_to_collidable_tiles() {
    let fixture = BlockAttachmentFixture::bootstrap();

    run_serial(&rspec::given(
        "LilleMapPlugin attaches Block to Collidable tiles",
        fixture,
        |scenario: &mut Scenario<BlockAttachmentFixture>| {
            scenario.when("the app ticks until blocks are attached", |ctx| {
                ctx.before_each(|state| {
                    let attached = state.tick_until_blocks_attached(MAX_LOAD_TICKS);
                    let map_errors = state.captured_map_errors();
                    assert!(
                        attached,
                        "expected blocks to be attached within {MAX_LOAD_TICKS} ticks; \
                         map errors: {map_errors:?}"
                    );
                });

                ctx.then("all collidable tiles receive Block components", |state| {
                    assert_eq!(
                        state.blocks_with_collidable_count(),
                        EXPECTED_COLLIDABLE_COUNT,
                        "expected exactly {EXPECTED_COLLIDABLE_COUNT} entities"
                    );
                });

                ctx.then("block count matches collidable count", |state| {
                    assert_eq!(state.block_count(), state.collidable_count());
                });

                ctx.then("collidable count matches expected fixture count", |state| {
                    assert_eq!(
                        state.collidable_count(),
                        EXPECTED_COLLIDABLE_COUNT,
                        "map fixture has changed: expected {EXPECTED_COLLIDABLE_COUNT} \
                         Collidable tiles, found {}",
                        state.collidable_count()
                    );
                });

                ctx.then("no blocks exist without Collidable", |state| {
                    assert_eq!(state.blocks_without_collidable_count(), 0);
                });

                ctx.then("block coordinates match tile positions", |state| {
                    assert!(
                        state.coordinates_match(),
                        "block and tile positions should match"
                    );
                });

                ctx.then("block IDs are unique", |state| {
                    let ids = state.block_ids();
                    let mut unique_ids = ids.clone();
                    unique_ids.sort_unstable();
                    unique_ids.dedup();
                    assert_eq!(
                        ids.len(),
                        unique_ids.len(),
                        "all block IDs should be unique"
                    );
                });

                ctx.then("all blocks have z=0", |state| {
                    assert!(state.all_blocks_have_z_zero(), "all blocks should have z=0");
                });

                ctx.then("no map errors are emitted", |state| {
                    assert!(state.captured_map_errors().is_empty());
                });

                // --- BlockSlope assertions ---

                ctx.then("sloped tiles receive BlockSlope components", |state| {
                    assert_eq!(
                        state.block_slope_count(),
                        EXPECTED_COLLIDABLE_COUNT,
                        "expected {EXPECTED_COLLIDABLE_COUNT} BlockSlope components"
                    );
                });

                ctx.then(
                    "block slope count matches slope properties count",
                    |state| {
                        assert_eq!(
                            state.block_slope_count(),
                            state.slope_properties_count(),
                            "BlockSlope count should match SlopeProperties count"
                        );
                    },
                );

                ctx.then("all BlockSlope IDs match their parent Block IDs", |state| {
                    assert!(
                        state.block_slope_ids_match_block_ids(),
                        "all BlockSlope.block_id values should match Block.id"
                    );
                });

                ctx.then("no BlockSlope exists without a Block", |state| {
                    assert_eq!(
                        state.slopes_without_blocks_count(),
                        0,
                        "no BlockSlope should exist without a matching Block"
                    );
                });

                ctx.then("all slopes have blocks", |state| {
                    assert_eq!(
                        state.blocks_with_slopes_count(),
                        state.block_slope_count(),
                        "all BlockSlopes should be paired with Blocks"
                    );
                });

                ctx.then("slope gradients match fixture values", |state| {
                    // The fixture map has grad_x=0.25, grad_y=0.5 on all tiles.
                    assert!(
                        state.all_slopes_have_expected_gradients(0.25, 0.5),
                        "gradients should match fixture values (0.25, 0.5)"
                    );
                });
            });
        },
    ));
}
