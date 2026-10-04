//! Run the compiler-cache statistics step against a stand-in `sccache`.
//!
//! The rules in `compiler_cache` read the step's text, which cannot show what
//! a shell does with it: a guard that sits after the command it protects, or a
//! branch that exits before printing the backend, satisfies a text search. These
//! run the step's own script under `bash` with a stand-in `sccache` defined, and
//! read what it printed, what it wrote to the job summary and which arguments
//! the stand-in received.
//!
//! The stand-in is a shell function loaded through `BASH_ENV`, so it needs no
//! executable file and `command -v sccache` finds it as it would the binary.
//!
//! The server start and its fallback belong to the pinned `setup-rust` action,
//! whose own tests execute them in shared-actions. This repository owns the
//! report, so the report is what is run here.

use std::io::ErrorKind;
use std::process::Command;

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};
use rstest::rstest;
use tempfile::TempDir;

use crate::workflow_assertions::{job_named, workflows};
use crate::workflow_estate::{Workflow, BUILD_JOB_IDS};

/// What the stand-in `sccache` prints for `--show-stats`.
const FAKE_STATISTICS: &str = "Cache location                  ghac (fake)";

/// The stand-in: records its arguments, one call per line, and prints a fixed
/// statistics line. `CALLS_FILE` names where the calls go.
const FAKE_SCCACHE: &str = "sccache() {\n  printf '%s\\n' \"$*\" >> \"$CALLS_FILE\"\n  \
                            printf '%s\\n' 'Cache location                  ghac (fake)'\n}\n";

/// Fails with `message` when `condition` does not hold.
///
/// The tests return `Result`, so a failed expectation is an `Err` they
/// propagate rather than a panic.
fn ensure(condition: bool, message: String) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message)
    }
}

/// What one run of the report step produced.
struct Outcome {
    succeeded: bool,
    stdout: String,
    summary: String,
    /// Arguments the stand-in `sccache` received, one call per line.
    calls: String,
}

/// Reads a file the run may legitimately not have written.
///
/// # Errors
///
/// Returns the empty string only when the file is absent, which for the call
/// log means no call was recorded. Any other read error is returned, so an
/// environmental fault cannot pass for an expected empty result.
fn read_if_written(dir: &Dir, name: &str) -> Result<String, String> {
    match dir.read_to_string(name) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(format!("cannot read {name}: {err}")),
    }
}

/// Runs `script` with `status` as `SCCACHE_STATUS`, with or without the
/// stand-in `sccache` defined.
///
/// With the stand-in, `PATH` holds the system directories the script's own
/// commands need. Without it, `PATH` is an empty scratch directory, so command
/// lookup cannot find a host `sccache` and the "not installed" branch is the
/// one that runs whatever the host has installed.
///
/// # Errors
///
/// Returns a message when the scratch directory cannot be prepared, `bash`
/// cannot be started, or a file the run wrote cannot be read.
fn run_report(script: &str, status: &str, sccache_installed: bool) -> Result<Outcome, String> {
    let scratch = TempDir::new().map_err(|err| format!("a scratch directory: {err}"))?;
    let root = Utf8Path::from_path(scratch.path())
        .ok_or_else(|| "the scratch directory must be UTF-8".to_owned())?;
    let dir = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|err| format!("cannot open the scratch directory: {err}"))?;
    dir.write("summary", "")
        .map_err(|err| format!("a summary file: {err}"))?;
    dir.write("fake.sh", FAKE_SCCACHE)
        .map_err(|err| format!("the stand-in sccache: {err}"))?;
    let empty_path = root.join("empty-path");
    dir.create_dir("empty-path")
        .map_err(|err| format!("an empty PATH directory: {err}"))?;
    let path = if sccache_installed {
        "/usr/bin:/bin".to_owned()
    } else {
        empty_path.into_string()
    };
    // An absolute path: with `PATH` emptied, a bare `bash` would not be found.
    let bash = ["/bin/bash", "/usr/bin/bash"]
        .into_iter()
        .find(|candidate| Utf8Path::new(candidate).is_file())
        .ok_or_else(|| "no bash at /bin/bash or /usr/bin/bash".to_owned())?;
    let mut command = Command::new(bash);
    command
        .arg("-c")
        .arg(script)
        .env_clear()
        .env("PATH", path)
        .env("SCCACHE_STATUS", status)
        .env("SCCACHE_BACKEND", "ubicloud")
        .env("GITHUB_JOB", "scratch-job")
        .env("GITHUB_STEP_SUMMARY", root.join("summary"))
        .env("CALLS_FILE", root.join("calls"));
    if sccache_installed {
        command.env("BASH_ENV", root.join("fake.sh"));
    }
    let output = command
        .output()
        .map_err(|err| format!("bash must run the report step: {err}"))?;
    Ok(Outcome {
        succeeded: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        summary: dir
            .read_to_string("summary")
            .map_err(|err| format!("cannot read the job summary: {err}"))?,
        calls: read_if_written(&dir, "calls")?,
    })
}

/// Returns each build job's report script, with the job id.
fn report_scripts(workflows: &[Workflow]) -> Vec<(&'static str, String)> {
    BUILD_JOB_IDS
        .iter()
        .map(|id| {
            let Some((_, report)) =
                job_named(workflows, id).first_step_with("sccache --show-stats")
            else {
                panic!("`{id}` must report compiler-cache statistics");
            };
            (*id, report.run.clone())
        })
        .collect()
}

/// A started server is asked for its statistics, the backend is named, and the
/// numbers reach both the log and the job summary.
#[rstest]
fn a_started_server_is_reported_to_the_log_and_the_summary(
    workflows: Vec<Workflow>,
) -> Result<(), String> {
    for (id, script) in report_scripts(&workflows) {
        let outcome = run_report(&script, "started", true)?;
        ensure(
            outcome.succeeded,
            format!("`{id}` report must succeed when started"),
        )?;
        ensure(
            outcome.calls.trim() == "--show-stats",
            format!("`{id}` must ask the server for its statistics exactly once"),
        )?;
        ensure(
            outcome.stdout.contains("backend: ubicloud"),
            format!(
                "`{id}` must name the backend in the log; got {:?}",
                outcome.stdout
            ),
        )?;
        ensure(
            outcome.stdout.contains(FAKE_STATISTICS),
            format!(
                "`{id}` must print the statistics to the log; got {:?}",
                outcome.stdout
            ),
        )?;
        ensure(
            outcome
                .summary
                .contains("### sccache statistics (scratch-job)")
                && outcome.summary.contains("backend: ubicloud")
                && outcome.summary.contains(FAKE_STATISTICS),
            format!(
                "`{id}` must put the job, backend and statistics in the summary; got {:?}",
                outcome.summary
            ),
        )?;
    }
    Ok(())
}

/// A fallback, or no status at all, never asks for statistics, writes nothing
/// to the summary, and still succeeds, so the documented fail-open stays green.
#[rstest]
#[case::fallback("fallback")]
#[case::no_status("")]
fn a_server_that_did_not_start_is_not_asked_for_statistics(
    workflows: Vec<Workflow>,
    #[case] status: &str,
) -> Result<(), String> {
    for (id, script) in report_scripts(&workflows) {
        let outcome = run_report(&script, status, true)?;
        ensure(
            outcome.succeeded,
            format!("`{id}` report must not fail the job when sccache-status is {status:?}"),
        )?;
        ensure(
            outcome.calls.is_empty(),
            format!("`{id}` must not run `sccache --show-stats` when sccache-status is {status:?}"),
        )?;
        ensure(
            outcome.stdout.contains("sccache did not start"),
            format!(
                "`{id}` must say why there are no statistics; got {:?}",
                outcome.stdout
            ),
        )?;
        ensure(
            outcome.summary.is_empty(),
            format!("`{id}` must not write statistics to the summary after a fallback"),
        )?;
    }
    Ok(())
}

/// With no `sccache` available (an early step failed before installing it) the
/// report says so and succeeds, rather than burying the real failure.
#[rstest]
fn a_missing_sccache_is_reported_without_failing(workflows: Vec<Workflow>) -> Result<(), String> {
    for (id, script) in report_scripts(&workflows) {
        let outcome = run_report(&script, "started", false)?;
        ensure(
            outcome.succeeded,
            format!("`{id}` report must not fail when sccache is not installed"),
        )?;
        ensure(
            outcome.stdout.contains("sccache is not installed"),
            format!(
                "`{id}` must say sccache is not installed; got {:?}",
                outcome.stdout
            ),
        )?;
        ensure(
            outcome.summary.is_empty(),
            format!("`{id}` must write nothing to the summary"),
        )?;
    }
    Ok(())
}

/// The deterministic, human-facing output of the report, snapshotted.
///
/// The semantic assertions above stay; these pin the rendered text a person
/// reads in the log and the job summary, which a substring check lets drift. Each
/// run uses the stand-in's fixed statistics, the fixed backend and the fixed job
/// name, so nothing in a snapshot comes from the host or the clock.
#[rstest]
fn the_rendered_report_is_stable(workflows: Vec<Workflow>) -> Result<(), String> {
    let (_, script) = report_scripts(&workflows)
        .into_iter()
        .next()
        .ok_or_else(|| "a build job must report compiler-cache statistics".to_owned())?;
    let started = run_report(&script, "started", true)?;
    let fallback = run_report(&script, "fallback", true)?;
    let missing = run_report(&script, "started", false)?;

    insta::assert_snapshot!("started_log", started.stdout);
    insta::assert_snapshot!("started_summary", started.summary);
    insta::assert_snapshot!("fallback_log", fallback.stdout);
    insta::assert_snapshot!("missing_sccache_log", missing.stdout);
    Ok(())
}
