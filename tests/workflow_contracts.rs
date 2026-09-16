//! Structural contracts over the repository's GitHub Actions workflows.
//!
//! These encode the Ubicloud adoption rules a reviewer would otherwise re-check
//! by hand on every workflow edit. They read the files directly, so they fail
//! on the change that introduces a violation rather than on the CI run that
//! suffers from it.
//!
//! This file is the harness. The rules live in seven modules, split by the
//! question each asks: `supply_chain` for what the estate will execute,
//! `placement` for what it costs and who owns each cache, `compiler_cache` for
//! the sccache wiring and the resource sampling, `sampler_reading` for the
//! cases that hold the `shell_reading` support module to what a shell
//! would actually run, `timeouts` for the
//! ordering of the timers that can end a run, `timeout_budgets` for the
//! arithmetic that ordering rests on, and `parsing` for the loader itself.

#[path = "support/placement_expression.rs"]
mod placement_expression;
#[path = "support/shell_reading.rs"]
mod shell_reading;
#[path = "support/workflow_assertions.rs"]
mod workflow_assertions;
#[path = "support/workflow_cache_owners.rs"]
mod workflow_cache_owners;
#[path = "support/workflow_config.rs"]
mod workflow_config;
#[path = "support/workflow_estate.rs"]
mod workflow_estate;
#[path = "support/workflow_loader.rs"]
mod workflow_loader;
#[path = "support/workflow_model.rs"]
mod workflow_model;

#[path = "contracts/compiler_cache.rs"]
mod compiler_cache;
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
