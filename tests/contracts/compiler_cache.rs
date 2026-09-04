//! Compiler-cache and resource-sampling contracts.
//!
//! sccache is the only owner of compiler output here, and it fails silently
//! when it is wired wrongly: a misconfigured backend reports a plausible
//! `Cache location` and caches nothing. These contracts pin the wiring that
//! makes it work, and the sampling that lets the runner shape be argued from
//! measurement rather than habit.

use rstest::rstest;

use crate::shared_action;
use crate::workflow_assertions::{assert_input, job_named, step_using, workflows};
use crate::workflow_estate::{Workflow, BUILD_JOB_IDS};

/// Commit that every `actions/github-script` reference must pin (v8).
const GITHUB_SCRIPT_SHA: &str = "ed597411d8f924073f98dfc5c65a23a2325f34cd";

/// Variables sccache's GitHub Actions backend needs re-exported on Ubicloud.
const PROXY_VARIABLES: [&str; 3] = [
    "ACTIONS_CACHE_URL",
    "ACTIONS_RUNTIME_TOKEN",
    "ACTIONS_CACHE_SERVICE_V2",
];

#[rstest]
fn setup_rust_owns_the_registry_but_not_the_compiler_cache(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let step = step_using(job, &shared_action("setup-rust"));
        assert_input(id, step, "cache-provider", "github");
        // The action's sccache path runs the mozilla sccache-action, which
        // writes GitHub's v2 cache service back to `GITHUB_ENV` as its last
        // act, clobbering the proxy export for every later step. The job
        // installs and starts sccache itself instead.
        assert_input(id, step, "use-sccache", "false");
    }
}

/// The two variables that make the wrapper more than overhead.
///
/// `RUSTC_WRAPPER` engages sccache; `SCCACHE_GHA_ENABLED` selects the Actions
/// backend. Without the second, sccache writes to a local directory nothing
/// persists between runs, and every compilation misses.
#[rstest]
#[case::wrapper("RUSTC_WRAPPER", "sccache")]
#[case::backend("SCCACHE_GHA_ENABLED", "true")]
#[case::no_incremental("CARGO_INCREMENTAL", "0")]
fn the_compiler_cache_is_engaged_at_job_level(
    workflows: Vec<Workflow>,
    #[case] variable: &str,
    #[case] expected: &str,
) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        assert_eq!(
            job.env(variable),
            expected,
            "`{id}` must export `{variable}: {expected}` at job level"
        );
    }
}

/// The sccache server binds its backend once, when it starts, so the order of
/// these steps is the contract. Started before the export it binds GitHub's v2
/// service instead of Ubicloud's proxy; started after `setup-rust`, whose
/// sccache path rewrites the cache service back into `GITHUB_ENV`, it binds
/// whatever that left behind; reported before the build it measures nothing.
#[rstest]
fn the_compiler_cache_is_wired_in_the_only_order_that_works(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let stage = |needle: &str, what: &str| {
            job.first_step_containing(needle)
                .unwrap_or_else(|| panic!("`{id}` must {what}"))
        };
        let export = stage("actions/github-script", "export the Ubicloud cache proxy");
        let install = stage("taiki-e/install-action", "install a pinned sccache");
        let start = stage("sccache --zero-stats", "start the compiler cache");
        // `setup-rust` stands for the first step that could compile: it puts
        // the toolchain in place, and nothing before it runs cargo.
        let toolchain = stage("setup-rust", "set up Rust before anything compiles");
        let coverage = stage("generate-coverage", "build the workspace under coverage");
        let report = stage("sccache --show-stats", "report compiler-cache statistics");
        let order = [
            ("export the cache proxy", export),
            ("install sccache", install),
            ("start sccache", start),
            ("set up the toolchain", toolchain),
            ("build", coverage),
            ("report the statistics", report),
        ];
        for ((earlier, before), (later, after)) in order.iter().zip(order.iter().skip(1)) {
            assert!(
                before < after,
                "`{id}` must {earlier} (step {before}) before it can {later} (step {after})"
            );
        }
    }
}

#[rstest]
fn the_cache_proxy_export_is_pinned_and_names_every_variable(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let (export_at, export) = job
            .first_step_with("actions/github-script")
            .unwrap_or_else(|| panic!("`{id}` must export the Ubicloud cache proxy"));
        assert!(
            export.uses.ends_with(GITHUB_SCRIPT_SHA),
            "`{id}` must pin actions/github-script to {GITHUB_SCRIPT_SHA}"
        );
        let checkout_at = job
            .first_step_containing("actions/checkout")
            .unwrap_or_else(|| panic!("`{id}` must check out the repository"));
        assert!(
            checkout_at < export_at,
            "`{id}` must export the proxy after checkout"
        );
        let script = export.input("script");
        for variable in PROXY_VARIABLES {
            assert!(
                script.contains(variable),
                "`{id}` must export `{variable}` for sccache's backend"
            );
        }
        assert!(
            !script.contains("ACTIONS_RESULTS_URL"),
            "`{id}` must not export ACTIONS_RESULTS_URL; it does not route \
             through Ubicloud's cache proxy"
        );
    }
}

#[rstest]
fn compiler_cache_effectiveness_is_measured_around_the_build(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let zero_at = job
            .first_step_containing("sccache --zero-stats")
            .unwrap_or_else(|| panic!("`{id}` must reset the compiler-cache counters"));
        let (show_at, report) = job
            .first_step_with("sccache --show-stats")
            .unwrap_or_else(|| panic!("`{id}` must report compiler-cache statistics"));
        assert!(
            zero_at < show_at,
            "`{id}` must reset the counters before it reports them"
        );
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
        for measure in ["free -m", "df -m"] {
            assert!(
                job.steps.iter().any(|step| step.run.contains(measure)),
                "`{id}` must sample `{measure}`; disk and memory are both needed"
            );
        }
        assert!(
            report.run.contains("peak used disk"),
            "`{id}` must report peak disk, not memory alone"
        );
    }
}
