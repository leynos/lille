//! Structural contracts over the repository's GitHub Actions workflows.
//!
//! These encode the Ubicloud adoption rules a reviewer would otherwise re-check
//! by hand on every workflow edit: no tool is built from source, every cached
//! path has one owner, references are pinned, API-bound jobs stay
//! GitHub-hosted, and an installer precedes the first use of what it installs.
//! They also pin the inputs that make those rules true, so a workflow cannot
//! keep the shape of the policy while dropping its substance. They read the
//! files directly, so they fail on the change that introduces a violation
//! rather than on the CI run that suffers from it.

#[path = "support/workflow_assertions.rs"]
mod workflow_assertions;
#[path = "support/workflow_cache_owners.rs"]
mod workflow_cache_owners;
#[path = "support/workflow_estate.rs"]
mod workflow_estate;
#[path = "support/workflow_loader.rs"]
mod workflow_loader;
#[path = "support/workflow_model.rs"]
mod workflow_model;

use camino::Utf8Path;
use rstest::rstest;

use workflow_assertions::{assert_input, job_named, jobs, step_using, workflows};
use workflow_estate::{
    Workflow, WorkflowSource, BUILD_JOB_IDS, CACHE_ACTION_SHA, SHARED_ACTIONS_OWNER,
    SHARED_ACTIONS_SHA, UBICLOUD_LABEL,
};
use workflow_loader::{all_steps, load_workflows_in, parse_workflow, read_repository_file};

/// Full coordinate of a shared composite action this repository calls.
fn shared_action(name: &str) -> String {
    format!("{SHARED_ACTIONS_OWNER}/.github/actions/{name}")
}

/// Fragments that mark a step as building a tool from source.
///
/// `cargo binstall` is included because it compiles whenever its default
/// strategies fall through to `compile`; the estate's rule is to install from
/// a verified release archive instead.
const SOURCE_BUILD_FRAGMENTS: [&str; 3] = ["cargo install", "cargo-binstall ", "cargo binstall"];

/// Commands that would run the test suite a second time in a build job.
const REPEAT_TEST_COMMANDS: [&str; 4] = ["cargo test", "cargo nextest", "make test", "make all"];

/// Commit that every `actions/github-script` reference must pin (v8).
const GITHUB_SCRIPT_SHA: &str = "ed597411d8f924073f98dfc5c65a23a2325f34cd";

/// Pinned prebuilt sccache the build jobs install.
const SCCACHE_TOOL: &str = "sccache@0.16.0";

/// Variables sccache's GitHub Actions backend needs re-exported on Ubicloud.
const PROXY_VARIABLES: [&str; 3] = [
    "ACTIONS_CACHE_URL",
    "ACTIONS_RUNTIME_TOKEN",
    "ACTIONS_CACHE_SERVICE_V2",
];

/// Expression fragments the uv tool-layer cache key must carry.
const UV_CACHE_KEY_FRAGMENTS: [&str; 4] = [
    "runner.os",
    "runner.arch",
    "runner.environment",
    "hashFiles(",
];

#[rstest]
fn every_cache_reference_is_pinned_to_v6_1_0(workflows: Vec<Workflow>) {
    let unpinned: Vec<String> = all_steps(&workflows)
        .into_iter()
        .filter(|(_, _, step)| step.uses.starts_with("actions/cache"))
        .filter(|(_, _, step)| !step.uses.ends_with(CACHE_ACTION_SHA))
        .map(|(file, job, step)| format!("{file}:{job}: {}", step.uses))
        .collect();
    assert!(
        unpinned.is_empty(),
        "every actions/cache reference must pin {CACHE_ACTION_SHA} (v6.1.0): {unpinned:?}"
    );
}

#[rstest]
fn no_workflow_uses_the_ubicloud_cache_fork(workflows: Vec<Workflow>) {
    let forks: Vec<String> = all_steps(&workflows)
        .into_iter()
        .filter(|(_, _, step)| step.uses.starts_with("ubicloud/cache"))
        .map(|(file, job, step)| format!("{file}:{job}: {}", step.uses))
        .collect();
    assert!(
        forks.is_empty(),
        "the deprecated ubicloud/cache fork must not be used: {forks:?}"
    );
}

#[rstest]
fn every_shared_action_reference_is_pinned(workflows: Vec<Workflow>) {
    let mut references: Vec<String> = all_steps(&workflows)
        .into_iter()
        .map(|(file, job, step)| (file, job, step.uses))
        .chain(
            jobs(&workflows)
                .into_iter()
                .map(|(file, job)| (file, job.id.clone(), job.uses)),
        )
        .filter(|(_, _, uses)| uses.starts_with("leynos/shared-actions"))
        .filter(|(_, _, uses)| !uses.ends_with(SHARED_ACTIONS_SHA))
        .map(|(file, job, uses)| format!("{file}:{job}: {uses}"))
        .collect();
    references.sort();
    assert!(
        references.is_empty(),
        "every leynos/shared-actions reference must pin {SHARED_ACTIONS_SHA}: {references:?}"
    );
}

#[rstest]
fn no_step_builds_a_tool_from_source(workflows: Vec<Workflow>) {
    let offenders: Vec<String> = all_steps(&workflows)
        .into_iter()
        .filter(|(_, _, step)| {
            SOURCE_BUILD_FRAGMENTS
                .iter()
                .any(|fragment| step.run.contains(fragment))
        })
        .map(|(file, job, step)| format!("{file}:{job}: {}", step.name))
        .collect();
    assert!(
        offenders.is_empty(),
        "tools must be installed from verified prebuilt releases, not compiled: {offenders:?}"
    );
}

#[rstest]
fn install_action_fails_closed_rather_than_compiling(workflows: Vec<Workflow>) {
    let permissive: Vec<String> = all_steps(&workflows)
        .into_iter()
        .filter(|(_, _, step)| step.uses.starts_with("taiki-e/install-action"))
        .filter(|(_, _, step)| step.input("fallback") != "none")
        .map(|(file, job, step)| format!("{file}:{job}: {}", step.name))
        .collect();
    assert!(
        permissive.is_empty(),
        "taiki-e/install-action must set `fallback: none` so it cannot compile: {permissive:?}"
    );
}

#[rstest]
fn each_cached_path_has_exactly_one_owner(workflows: Vec<Workflow>) {
    let clashes: Vec<String> = jobs(&workflows)
        .into_iter()
        .flat_map(|(file, job)| {
            workflow_cache_owners::duplicated_paths(&job)
                .into_iter()
                .map(move |(path, owners)| format!("{file}:{}: {path} owned by {owners:?}", job.id))
        })
        .collect();
    assert!(
        clashes.is_empty(),
        "each cached path must have one owner: {clashes:?}"
    );
}

#[rstest]
fn non_build_jobs_stay_on_github_hosted_runners(workflows: Vec<Workflow>) {
    let misplaced: Vec<String> = jobs(&workflows)
        .into_iter()
        .filter(|(_, job)| job.runs_on.names_a_runner())
        .filter(|(_, job)| !BUILD_JOB_IDS.contains(&job.id.as_str()))
        .filter(|(_, job)| !job.is_github_hosted())
        .map(|(file, job)| format!("{file}:{}: {}", job.id, job.runs_on))
        .collect();
    assert!(
        misplaced.is_empty(),
        "delayed-comment, metadata, and other API-bound jobs must stay GitHub-hosted: {misplaced:?}"
    );
}

/// The measured bounds for each build job's `timeout-minutes`.
///
/// The lower bound keeps the timeout above the observed median so a normal run
/// cannot be killed; the upper bound keeps a hung run from billing for hours.
#[rstest]
#[case::build_test("build-test", 45, 120)]
#[case::coverage_upload("coverage-upload", 30, 90)]
fn build_jobs_keep_their_label_and_a_bounded_timeout(
    workflows: Vec<Workflow>,
    #[case] id: &str,
    #[case] lowest: u64,
    #[case] highest: u64,
) {
    let job = job_named(&workflows, id);
    assert_eq!(
        job.runs_on.labels(),
        [UBICLOUD_LABEL],
        "`{id}` must keep its measured runner label"
    );
    let timeout = job
        .timeout_minutes
        .unwrap_or_else(|| panic!("`{id}` bills by the minute and must declare timeout-minutes"));
    assert!(
        (lowest..=highest).contains(&timeout),
        "`{id}` timeout-minutes {timeout} must lie between {lowest} and {highest}"
    );
}

/// A warm run has to be triggerable without pushing a commit, so the runner
/// and cache changes can be measured on an unchanged tree.
#[rstest]
fn the_pull_request_workflow_accepts_a_warm_run_dispatch(workflows: Vec<Workflow>) {
    let Some(ci) = workflows.iter().find(|workflow| workflow.file == "ci.yml") else {
        panic!("the estate must define ci.yml")
    };
    assert!(
        ci.has_trigger("workflow_dispatch"),
        "ci.yml must accept `workflow_dispatch` so a warm run can be measured \
         on demand; it declares {:?}",
        ci.triggers
    );
}

#[rstest]
fn every_runner_label_is_registered_with_actionlint(workflows: Vec<Workflow>) {
    let text = read_repository_file(".github/actionlint.yaml")
        .unwrap_or_else(|err| panic!("actionlint configuration must be readable: {err}"));
    let unregistered: Vec<String> = jobs(&workflows)
        .into_iter()
        .filter(|(_, job)| job.runs_on.names_a_runner() && !job.is_github_hosted())
        .filter(|(_, job)| {
            !job.runs_on
                .labels()
                .iter()
                .all(|label| text.contains(label.as_str()))
        })
        .map(|(file, job)| format!("{file}:{}: {}", job.id, job.runs_on))
        .collect();
    assert!(
        unregistered.is_empty(),
        "every self-hosted label must appear in .github/actionlint.yaml: {unregistered:?}"
    );
}

#[rstest]
#[case::rust_toolchain("setup-rust", "cargo")]
#[case::whitaker_suite("install-whitaker", "whitaker ")]
fn an_installer_precedes_the_first_use_of_its_tool(
    workflows: Vec<Workflow>,
    #[case] installer: &str,
    #[case] first_use: &str,
) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let Some(use_index) = job.first_step_containing(first_use) else {
            continue;
        };
        let install_index = job
            .first_step_containing(installer)
            .unwrap_or_else(|| panic!("`{id}` uses `{first_use}` without a `{installer}` step"));
        assert!(
            install_index < use_index,
            "`{id}` must run `{installer}` before step {use_index} uses `{first_use}`"
        );
    }
}

#[rstest]
fn coverage_is_the_only_test_execution(workflows: Vec<Workflow>) {
    let duplicates: Vec<String> = all_steps(&workflows)
        .into_iter()
        .filter(|(_, job, _)| BUILD_JOB_IDS.contains(&job.as_str()))
        .filter(|(_, _, step)| {
            REPEAT_TEST_COMMANDS
                .iter()
                .any(|command| step.run.contains(command))
        })
        .map(|(file, job, step)| format!("{file}:{job}: {}", step.label()))
        .collect();
    assert!(
        duplicates.is_empty(),
        "the instrumented coverage run is the only test execution; drop the repeat: {duplicates:?}"
    );
}

#[rstest]
fn setup_rust_owns_the_registry_but_not_the_compiler_cache(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let step = step_using(job, &shared_action("setup-rust"));
        assert_input(id, step, "cache-provider", "github");
        // The action would start the sccache server inside an action step,
        // where the Ubicloud runner re-injects its own cache variables and the
        // server binds GitHub's v2 service instead of the local proxy. The job
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

#[rstest]
fn sccache_is_installed_from_a_pinned_prebuilt_release(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let step = step_using(job, "taiki-e/install-action");
        assert_input(id, step, "tool", SCCACHE_TOOL);
        assert_input(id, step, "fallback", "none");
    }
}

/// The sccache server binds its backend once, when it starts, so the order of
/// these steps is the contract. Started before the export it binds GitHub's v2
/// service instead of Ubicloud's proxy; started after the toolchain is in
/// place it can miss the first compilation; reported before the build it
/// measures nothing.
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

#[rstest]
fn coverage_runs_the_whole_suite_once_and_owns_no_cargo_cache(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let step = step_using(job, &shared_action("generate-coverage"));
        for flag in ["all-features", "all-targets", "doctests"] {
            assert_input(id, step, flag, "true");
        }
        assert_input(id, step, "cache-provider", "external");
    }
}

#[rstest]
fn whitaker_is_installed_from_a_pinned_prebuilt_release(workflows: Vec<Workflow>) {
    let job = job_named(&workflows, "build-test");
    let step = step_using(job, &shared_action("install-whitaker"));
    assert_input("build-test", step, "installer-version", "0.2.7");
    assert_input("build-test", step, "cache-provider", "github");
}

#[rstest]
fn the_uv_cache_names_its_layers_and_keys_them_by_runner(workflows: Vec<Workflow>) {
    let job = job_named(&workflows, "build-test");
    let cache = job
        .steps
        .iter()
        .find(|step| step.cache_paths().iter().any(|path| path == ".uv-cache"));
    let Some(step) = cache else {
        panic!("`build-test` must cache the uv download layer")
    };
    assert_eq!(
        step.cache_paths(),
        vec![".uv-cache".to_owned(), ".uv-tools".to_owned()],
        "the uv cache must own both the download store and the tool store"
    );
    let key = step.input("key");
    for fragment in UV_CACHE_KEY_FRAGMENTS {
        assert!(
            key.contains(fragment),
            "the uv cache key `{key}` must vary with `{fragment}`"
        );
    }
}

#[rstest]
#[case::not_a_workflow("scratch.yml", "steps: []")]
#[case::mistyped_runner("scratch.yml", "jobs:\n  a:\n    runs-on: {group: [g]}\n")]
#[case::mistyped_runner_label("scratch.yml", "jobs:\n  a:\n    runs-on: [a, [b]]\n")]
#[case::groupless_runner_mapping("scratch.yml", "jobs:\n  a:\n    runs-on: {labels: [a]}\n")]
#[case::placeless_job("scratch.yml", "jobs:\n  a:\n    steps: []\n")]
#[case::mistyped_steps("scratch.yml", "jobs:\n  a:\n    runs-on: x\n    steps: nope\n")]
#[case::empty_step(
    "scratch.yml",
    "jobs:\n  a:\n    runs-on: x\n    steps:\n      - name: n\n"
)]
#[case::mistyped_input(
    "scratch.yml",
    "jobs:\n  a:\n    runs-on: x\n    steps:\n      - uses: u\n        with:\n          k: [1]\n"
)]
// GitHub Actions runs a step either as an action or as a script, never both.
#[case::dual_mode_step(
    "scratch.yml",
    "jobs:\n  a:\n    runs-on: x\n    steps:\n      - uses: u\n        run: echo hi\n"
)]
fn a_malformed_workflow_is_an_error_not_a_default(#[case] file: &str, #[case] text: &str) {
    let outcome = parse_workflow(WorkflowSource { file, text });
    assert!(
        outcome.is_err(),
        "a workflow of unexpected shape must be rejected, not silently defaulted"
    );
}

/// Every `runs-on` shape GitHub Actions accepts must parse, not just the
/// scalar one: rejecting a label list or a runner group would fail a valid
/// workflow rather than the workflow a contract is meant to catch.
#[rstest]
#[case::single_label("runs-on: ubuntu-latest\n", &["ubuntu-latest"])]
#[case::label_list("runs-on: [self-hosted, linux]\n", &["self-hosted", "linux"])]
#[case::group_only("runs-on:\n      group: ubuntu-runners\n", &[])]
#[case::group_and_labels(
    "runs-on:\n      group: ubuntu-runners\n      labels: [ubuntu-20.04-16core]\n",
    &["ubuntu-20.04-16core"]
)]
fn every_valid_runs_on_shape_parses(#[case] runs_on: &str, #[case] expected: &[&str]) {
    let text = format!("on: push\njobs:\n  a:\n    {runs_on}    steps: []\n");
    let workflow = parse_workflow(WorkflowSource {
        file: "scratch.yml",
        text: &text,
    })
    .unwrap_or_else(|err| panic!("`{runs_on}` must parse: {err}"));
    let job = workflow
        .jobs
        .first()
        .unwrap_or_else(|| panic!("`{runs_on}` must yield a job"));
    assert_eq!(job.runs_on.labels(), expected);
    assert!(
        job.runs_on.names_a_runner(),
        "`{runs_on}` names a runner and must say so"
    );
}

#[rstest]
fn an_unreadable_workflow_directory_is_reported() {
    let missing = Utf8Path::new("this/directory/does/not/exist");
    let outcome = load_workflows_in(missing);
    assert!(
        outcome.is_err(),
        "an unreadable workflow directory must surface as an error"
    );
}
