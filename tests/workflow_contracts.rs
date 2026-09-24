//! Structural contracts over the repository's GitHub Actions workflows.
//!
//! These encode the Ubicloud adoption rules a reviewer would otherwise re-check
//! by hand on every workflow edit. They read the files directly, so they fail
//! on the change that introduces a violation rather than on the CI run that
//! suffers from it.
//!
//! This file is the harness. The rules live in twelve modules, split by the
//! question each asks: `coverage_boundary` for what a pull-request lane may
//! not publish or hold, `coverage_reach` for the closure of local calls and
//! the loader beneath it, `coverage_publisher` for the one lane that may
//! publish, `supply_chain` for what the estate will execute, `placement` for
//! what it costs and who owns each cache, `compiler_cache` for the sccache
//! wiring and the resource sampling, `concurrency` for which runs a newer
//! push may cancel, `sampler_reading` for the cases that hold
//! the `shell_reading` support module to what a shell would actually run,
//! `codescene_uploader` for the coverage uploader's pin and the deprecated
//! checksum inputs it rejects,
//! `timeouts` for the ordering of the timers that can end a run,
//! `timeout_budgets` for the arithmetic that ordering rests on, and `parsing`
//! for the loader itself.

#[path = "support/coverage_boundary.rs"]
mod coverage_boundary;
#[path = "support/coverage_publisher.rs"]
mod coverage_publisher;
#[path = "support/coverage_reach.rs"]
mod coverage_reach;
#[path = "support/shell_reading.rs"]
mod shell_reading;
#[path = "support/workflow_assertions.rs"]
mod workflow_assertions;
#[path = "support/workflow_cache_owners.rs"]
mod workflow_cache_owners;
#[path = "support/workflow_concurrency.rs"]
mod workflow_concurrency;
#[path = "support/workflow_config.rs"]
mod workflow_config;
#[path = "support/workflow_estate.rs"]
mod workflow_estate;
#[path = "support/workflow_loader.rs"]
mod workflow_loader;
#[path = "support/workflow_model.rs"]
mod workflow_model;
#[path = "support/workflow_triggers.rs"]
mod workflow_triggers;

#[path = "contracts/codescene_uploader.rs"]
mod codescene_uploader;
#[path = "contracts/compiler_cache.rs"]
mod compiler_cache;
#[path = "contracts/concurrency.rs"]
mod concurrency;
#[path = "contracts/coverage_boundary.rs"]
mod coverage_boundary_contract;
#[path = "contracts/coverage_publisher.rs"]
mod coverage_publisher_contract;
#[path = "contracts/coverage_reach.rs"]
mod coverage_reach_contract;
#[path = "contracts/parsing.rs"]
mod parsing;
#[path = "contracts/placement.rs"]
mod placement;
#[path = "contracts/sampler_reading.rs"]
mod sampler_reading;
#[path = "contracts/supply_chain.rs"]
mod supply_chain;
#[path = "contracts/timeout_budgets.rs"]
mod timeout_budgets;
#[path = "contracts/timeouts.rs"]
mod timeouts;

use workflow_estate::SHARED_ACTIONS_OWNER;

/// Full coordinate of a shared composite action this repository calls.
#[must_use]
pub fn shared_action(name: &str) -> String {
    format!("{SHARED_ACTIONS_OWNER}/.github/actions/{name}")
}
