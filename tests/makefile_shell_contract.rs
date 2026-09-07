//! Contract over the shell configuration this repository's Makefile uses.
//!
//! `.ONESHELL:` hands a whole recipe to one shell invocation, so only that
//! shell's final exit status reaches make. Without errexit a failing lint or
//! test on an earlier line of a multi-line recipe is invisible and a gate
//! reports success over a red run. Without pipefail the same is true of a
//! command at the head of a pipeline, which is the shape `make spelling` uses.
//! Wildside proved the first failure mode: run 33939820204 logged ruff's
//! "Found 3 errors." and the job passed.
//!
//! The contract drives GNU make rather than reading the Makefile for the flags
//! because an assertion that finds `.SHELLFLAGS` is satisfied by a value that
//! enables neither. It builds a scratch Makefile from the real file's own shell
//! prologue, gives it a recipe whose failure make must not swallow, and asserts
//! make reports that failure.
//!
//! Mutation record, measured 2026-09-07 with GNU Make 4.4.1. Deleting `-e` from
//! the repository's `.SHELLFLAGS` fails both cases of
//! `make_reports_a_masked_failure`; deleting `-o pipefail` fails its `pipeline`
//! case alone. Deleting the whole line fails both.

use std::process::{Command, Output};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use rstest::{fixture, rstest};
use tempfile::TempDir;

/// The make definitions that decide how a multi-line recipe is executed.
///
/// Every other line of the Makefile is irrelevant to the probe, and copying
/// only these keeps the scratch file independent of the repository's targets.
const PROLOGUE_NAMES: [&str; 3] = [".ONESHELL", "SHELL", ".SHELLFLAGS"];

/// Name of the generated Makefile inside the scratch directory.
const PROBE_MAKEFILE: &str = "Makefile";

/// A recipe whose first command fails and whose second succeeds.
///
/// This is the shape a gate takes when an early tool reports findings and a
/// later one is clean, which is what errexit has to catch.
const SEQUENTIAL_PROBE: &str = "\nprobe:\n\tfalse\n\ttrue\n";

/// A recipe whose pipeline fails at its head and succeeds at its tail.
///
/// `make spelling` pipes `git ls-files` into `xargs typos`, so a failure of the
/// producer is reported only when pipefail is set.
const PIPELINE_PROBE: &str = "\nprobe:\n\tfalse | cat\n\ttrue\n";

/// Reads this repository's Makefile through a directory capability.
///
/// # Errors
///
/// Returns a message when the repository root cannot be opened or the Makefile
/// cannot be read.
fn repository_makefile() -> Result<String, String> {
    let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = Dir::open_ambient_dir(&root, ambient_authority())
        .map_err(|err| format!("cannot open the repository root {root}: {err}"))?;
    dir.read_to_string(PROBE_MAKEFILE)
        .map_err(|err| format!("cannot read the repository's Makefile: {err}"))
}

/// Returns the name a make line defines, or `None` when it defines nothing.
///
/// The name is compared whole: a prefix test would let a future `SHELL_WRAPPER`
/// answer for `SHELL`, and a substring test would match a recipe body that
/// merely mentions one of these names.
fn declared_name(line: &str) -> Option<&str> {
    let end = line.find(|c: char| c == ':' || c == '=' || c.is_whitespace())?;
    let (name, rest) = line.split_at(end);
    let follows = rest.trim_start();
    let assigns = follows.starts_with([':', '=', '?', '+', '!']);
    assigns.then_some(name)
}

/// Returns the Makefile lines that configure the recipe shell, in file order.
fn prologue_lines(makefile: &str) -> Vec<String> {
    makefile
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter(|line| declared_name(line).is_some_and(|name| PROLOGUE_NAMES.contains(&name)))
        .map(ToOwned::to_owned)
        .collect()
}

/// The repository's own shell prologue, as the probe will copy it.
///
/// Fallible rather than panicking, so a test consumes it with `?` and an
/// unreadable Makefile is reported as the failure it is.
#[fixture]
fn prologue() -> Result<Vec<String>, String> {
    repository_makefile().map(|makefile| prologue_lines(&makefile))
}

/// Runs a probe recipe under the given prologue and returns make's result.
///
/// The child's `MAKEFLAGS` is cleared so that a run started from `make test`
/// cannot lend the probe its parent's flags, which would decide the outcome
/// instead of the prologue under test. Output is captured rather than
/// inherited, so make's own echo of the failing command does not read as a
/// failure of this test run. The scratch Makefile is written through a
/// directory capability rooted at the scratch directory, so the probe cannot
/// write outside it.
///
/// # Errors
///
/// Returns a message when the scratch directory cannot be created, the scratch
/// Makefile cannot be written, or `make` cannot be executed.
fn probe_outcome(prologue: &[String], recipe: &str) -> Result<Output, String> {
    let scratch =
        TempDir::new().map_err(|err| format!("cannot create a scratch directory: {err}"))?;
    let root = Utf8Path::from_path(scratch.path())
        .ok_or_else(|| "the scratch directory path is not UTF-8".to_owned())?;
    let dir = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|err| format!("cannot open the scratch directory {root}: {err}"))?;
    let text = format!("{}\n{recipe}", prologue.join("\n"));
    dir.write(PROBE_MAKEFILE, text)
        .map_err(|err| format!("cannot write the scratch Makefile: {err}"))?;
    Command::new("make")
        .arg("--file")
        .arg(root.join(PROBE_MAKEFILE).as_str())
        .arg("probe")
        .current_dir(root.as_std_path())
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .env_remove("MAKELEVEL")
        .output()
        .map_err(|err| format!("cannot run make on the scratch Makefile: {err}"))
}

/// The probes only test the flags while the prologue keeps `.ONESHELL:`.
///
/// Without it make runs each recipe line in its own shell and reports the first
/// failure regardless of any flag, so the behavioural contract below would pass
/// while proving nothing. This guard makes that silent weakening a test
/// failure.
///
/// # Errors
///
/// Fails when the Makefile cannot be read or no longer declares `.ONESHELL:`.
#[rstest]
fn the_prologue_the_probe_copies_still_batches_recipes(
    prologue: Result<Vec<String>, String>,
) -> Result<(), String> {
    let lines = prologue?;
    if lines
        .iter()
        .any(|line| declared_name(line) == Some(".ONESHELL"))
    {
        return Ok(());
    }
    Err(format!(
        "the Makefile prologue must declare .ONESHELL:, found {lines:?}"
    ))
}

/// A failure make could swallow must fail the whole recipe.
///
/// Driven through make on a scratch Makefile carrying the repository's own
/// shell prologue, so each case follows the real configuration rather than a
/// copy of it. The two cases are the two ways `.ONESHELL:` hides a failure:
/// an earlier line, and the head of a pipeline.
///
/// # Errors
///
/// Fails when the Makefile cannot be read, make cannot be run, or make reports
/// success over the failing probe.
#[rstest]
#[case::earlier_line(SEQUENTIAL_PROBE, "a failing command on an earlier line")]
#[case::pipeline(PIPELINE_PROBE, "a failing command at the head of a pipeline")]
fn make_reports_a_masked_failure(
    prologue: Result<Vec<String>, String>,
    #[case] recipe: &str,
    #[case] described: &str,
) -> Result<(), String> {
    let lines = prologue?;
    let outcome = probe_outcome(&lines, recipe)?;
    if outcome.status.success() {
        return Err(format!(
            "make must fail a recipe with {described}; \
             the prologue {lines:?} lets the failure pass unreported"
        ));
    }
    Ok(())
}
