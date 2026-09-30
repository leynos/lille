//! Compiler-cache and resource-sampling contracts.
//!
//! sccache is the only owner of compiler output here, and it fails silently
//! when it is wired wrongly: a misconfigured backend reports a plausible
//! `Cache location` and caches nothing. These contracts pin the wiring that
//! makes it work, and the sampling that lets the runner shape be argued from
//! measurement rather than habit.

use rstest::rstest;

use crate::shared_action;
use crate::shell_reading::{samples, Measure};
use crate::workflow_assertions::{assert_input, job_named, step_using, workflows};
use crate::workflow_estate::{Workflow, BUILD_JOB_IDS};

/// The `expect-cache` each build job must declare. `build-test` has a fork
/// arm on a GitHub-hosted runner, so it takes whichever backend the runner
/// offers; `coverage-upload` runs only on Ubicloud and must get its proxy.
const EXPECTED_CACHE: [(&str, &str); 2] = [("build-test", "any"), ("coverage-upload", "ubicloud")];

/// The id the report step reads `setup-rust`'s outputs through.
const SETUP_RUST_ID: &str = "setup-rust";

/// Env that only a job which owns sccache by hand sets. `setup-rust` exports
/// the wrapper and selects the backend itself, so either surviving means the
/// job still carries a second owner.
const RETIRED_JOB_ENV: [&str; 3] = ["RUSTC_WRAPPER", "SCCACHE_GHA_ENABLED", "SCCACHE_CONF"];

/// `setup-rust` owns the compiler cache: it installs sccache, chooses the
/// backend by runner, starts the server with a 60 s startup timeout, falls
/// back to an uncached build if the server will not start, and names sccache
/// as the wrapper. Turning that off, or dropping the id the report reads its
/// outputs through, leaves the job compiling uncached or unreported.
#[rstest]
fn setup_rust_owns_the_compiler_cache(workflows: Vec<Workflow>) {
    for (id, expected) in EXPECTED_CACHE {
        let job = job_named(&workflows, id);
        let step = step_using(job, &shared_action("setup-rust"));
        assert_input(id, step, "cache-provider", "github");
        assert_input(id, step, "expect-cache", expected);
        let sccache = step.input("use-sccache");
        assert!(
            sccache.is_empty() || sccache == "true",
            "`{id}` must leave sccache to `setup-rust` (`use-sccache` is `{sccache}`)"
        );
        assert_eq!(
            step.id, SETUP_RUST_ID,
            "`{id}` `setup-rust` must carry the id `{SETUP_RUST_ID}`"
        );
    }
}

/// Only what `setup-rust` does not do stays at job level: sccache cannot cache
/// incremental compilation, and an incremental build would defeat every hit.
#[rstest]
fn the_job_disables_incremental_compilation(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        assert_eq!(
            job_named(&workflows, id).env("CARGO_INCREMENTAL"),
            "0",
            "`{id}` must export `CARGO_INCREMENTAL: 0` at job level"
        );
    }
}

/// The pieces `setup-rust` replaced must not survive beside it. Two owners
/// would start two servers, and the older one, which binds the wrong backend,
/// would win the race.
#[rstest]
fn no_hand_rolled_compiler_cache_survives_beside_setup_rust(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        for variable in RETIRED_JOB_ENV {
            assert!(
                job.env(variable).is_empty(),
                "`{id}` must not set `{variable}` at job level; `setup-rust` owns it"
            );
        }
        for (retired, needle) in [
            ("the cache proxy export", "actions/github-script"),
            ("a hand-installed sccache", "taiki-e/install-action"),
            ("a hand-started server", "sccache --zero-stats"),
            ("a hand-started server", "sccache --start-server"),
        ] {
            assert!(
                job.first_step_containing(needle).is_none(),
                "`{id}` must not carry {retired} (`{needle}`); `setup-rust` owns it"
            );
        }
    }
}

/// `setup-rust` must run before anything compiles, and the statistics must be
/// read after the build, or they measure nothing.
#[rstest]
fn the_compiler_cache_is_set_up_before_the_build_and_reported_after(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let stage = |needle: &str, what: &str| {
            let Some(at) = job.first_step_containing(needle) else {
                panic!("`{id}` must {what}");
            };
            at
        };
        let toolchain = stage("setup-rust", "set up Rust before anything compiles");
        let coverage = stage("generate-coverage", "build the workspace under coverage");
        let report = stage("sccache --show-stats", "report compiler-cache statistics");
        assert!(
            toolchain < coverage && coverage < report,
            "`{id}` must set up `setup-rust` (step {toolchain}) before the build (step \
             {coverage}) and report after it (step {report})"
        );
    }
}

/// The statistics step must read what `setup-rust` reports, and say which
/// backend it chose: `Cache location` reads `ghac` for the Ubicloud proxy and
/// for GitHub's own service alike, so it cannot tell them apart.
#[rstest]
fn compiler_cache_effectiveness_is_measured_and_names_its_backend(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let Some((_, report)) = job.first_step_with("sccache --show-stats") else {
            panic!("`{id}` must report compiler-cache statistics");
        };
        assert_eq!(
            report.env_value("SCCACHE_STATUS"),
            "${{ steps.setup-rust.outputs.sccache-status }}",
            "`{id}` must hand `setup-rust`'s `sccache-status` to the report"
        );
        assert_eq!(
            report.env_value("SCCACHE_BACKEND"),
            "${{ steps.setup-rust.outputs.cache-backend }}",
            "`{id}` must hand `setup-rust`'s `cache-backend` to the report"
        );
        let lines: Vec<&str> = report.run.lines().map(str::trim).collect();
        for wanted in [
            "if [[ \"${SCCACHE_STATUS}\" != started ]]; then",
            "printf 'backend: %s\\n' \"${SCCACHE_BACKEND}\"",
        ] {
            assert!(
                lines.contains(&wanted),
                "`{id}` statistics step must contain the line `{wanted}`"
            );
        }
        assert!(
            report.run.contains("GITHUB_STEP_SUMMARY"),
            "`{id}` must put the compiler-cache statistics in the job summary"
        );
        // The summary is not readable through the REST API, so a run whose
        // statistics went only there cannot be audited afterwards.
        assert!(
            report.run.contains("printf '%s\\n' \"$stats\""),
            "`{id}` must also print the compiler-cache statistics to the log"
        );
    }
}

/// The `ubicloud-standard-8` shape is inherited here, not measured. Sampling
/// memory and disk is what turns the next shape decision into evidence, and
/// disk is the one that has killed jobs silently elsewhere in this rollout.
#[rstest]
fn both_build_jobs_sample_and_report_their_resource_use(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let start = job
            .first_step_containing("sample-resources.sh")
            .unwrap_or_else(|| panic!("`{id}` must start a resource sampler"));
        let (report_at, report) = job
            .first_step_with("least free disk")
            .unwrap_or_else(|| panic!("`{id}` must report its peak resource use"));
        assert!(
            start < report_at,
            "`{id}` must start the sampler before it reports the peaks"
        );
        for measure in [Measure("free -m"), Measure("df -m")] {
            assert!(
                samples(job, measure),
                "`{id}` must sample `{measure}` in a form that runs; disk and \
                 memory are both needed, and a substring search alone would be \
                 satisfied by a sampler wrapped in `if false` or one whose \
                 failure is discarded with `|| true`"
            );
        }
        assert!(
            report.run.contains("peak used disk"),
            "`{id}` must report peak disk, not memory alone"
        );
    }
}
