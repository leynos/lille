//! Contract over the shell configuration this repository's Makefile uses.
//!
//! Two independent defaults hide a failed command, and the Makefile turns off
//! both. `.ONESHELL:` hands a whole recipe to one shell invocation, so only
//! that shell's final exit status reaches make and `-e` is what stops an
//! earlier line's failure being overwritten by a later line's success.
//! Separately, and with or without `.ONESHELL:`, a shell reports a pipeline's
//! last command's status, so `-o pipefail` is what stops a failure at the head
//! of a pipeline being hidden by a healthy consumer. Wildside proved the first:
//! run 33939820204 logged ruff's "Found 3 errors." and the job passed.
//!
//! The contract drives GNU make rather than reading the Makefile for the flags
//! because an assertion that finds `.SHELLFLAGS` is satisfied by a value that
//! enables neither. It builds a scratch Makefile from the real file's own shell
//! prologue, gives it a recipe whose failure make must not swallow, and asserts
//! make reports that failure.
//!
//! Mutation record, measured 2026-09-07 with GNU Make 4.4.1, changing one
//! element of the repository's prologue at a time. Deleting `-o pipefail` fails
//! the `pipeline` case of `make_reports_a_masked_failure`; deleting `-e` fails
//! its `earlier_line` case; deleting the whole `.SHELLFLAGS` line fails both.
//! Weakening `.ONESHELL:` to `.ONESHELL = 1` fails
//! `the_prologue_the_probes_copy_still_batches_recipes` and nothing else, and
//! pointing `SHELL` at a path that cannot be started fails
//! `a_recipe_that_fails_nowhere_succeeds` and nothing else. Those last two are
//! why the guard and the control exist: under either, both failure probes still
//! pass while proving nothing.

use std::process::{Command, Output};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use rstest::{fixture, rstest};
use tempfile::TempDir;

/// Name of the generated Makefile inside the scratch directory.
const PROBE_MAKEFILE: &str = "Makefile";

/// A recipe whose first command fails and whose second succeeds.
///
/// This is the shape a gate takes when an early tool reports findings and a
/// later one is clean, which is what `-e` under `.ONESHELL:` has to catch.
const SEQUENTIAL_PROBE: &str = "\nprobe:\n\tfalse\n\ttrue\n";

/// A recipe whose pipeline fails at its head and succeeds at its tail.
///
/// `make spelling` pipes `git ls-files` into `xargs typos`, so a failure of the
/// producer is reported only when `pipefail` is set.
const PIPELINE_PROBE: &str = "\nprobe:\n\tfalse | cat\n\ttrue\n";

/// A recipe in which nothing fails, pipeline included.
///
/// The control for the two probes above: they would also "pass" under a
/// prologue that fails for the wrong reason, such as a `SHELL` that cannot be
/// started or a flag the shell rejects.
const CLEAN_PROBE: &str = "\nprobe:\n\ttrue | cat\n\ttrue\n";

/// What a line of a Makefile defines.
///
/// The two are kept apart because `.ONESHELL` means nothing as a variable:
/// `.ONESHELL = 1` defines one and leaves the special target unset, so a
/// contract that accepted either would pass with recipe batching switched off.
#[derive(Debug, PartialEq, Eq)]
enum Definition<'a> {
    /// A target, written `name:`, as in `.ONESHELL:`.
    Target(&'a str),
    /// A variable, written `name :=`, `name =`, `name ?=`, and so on.
    Variable(&'a str),
}

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

/// Returns what a make line defines, or `None` when it defines nothing.
///
/// The name is compared whole by the callers: a prefix test would let a future
/// `SHELL_WRAPPER` answer for `SHELL`, and a substring test would match a
/// recipe body that merely mentions one of these names.
fn definition(line: &str) -> Option<Definition<'_>> {
    let end = line.find(|c: char| c == ':' || c == '=' || c.is_whitespace())?;
    let (name, rest) = line.split_at(end);
    if name.is_empty() {
        return None;
    }
    let follows = rest.trim_start();
    if let Some(after_colon) = follows.strip_prefix(':') {
        // `:=`, `::=` and `:::=` assign a variable. A colon that opens no
        // assignment opens a rule, which is how a special target is written.
        return Some(if after_colon.trim_start_matches(':').starts_with('=') {
            Definition::Variable(name)
        } else {
            Definition::Target(name)
        });
    }
    let assigns = ["=", "?=", "+=", "!="]
        .into_iter()
        .any(|operator| follows.starts_with(operator));
    assigns.then_some(Definition::Variable(name))
}

/// Reports whether a definition configures how a recipe is executed.
///
/// Every other line of the Makefile is irrelevant to the probes, and copying
/// only these keeps the scratch file independent of the repository's targets.
fn configures_the_shell(definition: &Definition<'_>) -> bool {
    matches!(
        definition,
        Definition::Target(".ONESHELL") | Definition::Variable("SHELL" | ".SHELLFLAGS")
    )
}

/// Returns the Makefile lines that configure the recipe shell, in file order.
fn prologue_lines(makefile: &str) -> Vec<String> {
    makefile
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter(|line| definition(line).as_ref().is_some_and(configures_the_shell))
        .map(ToOwned::to_owned)
        .collect()
}

/// The repository's own shell prologue, as the probes will copy it.
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

/// The `earlier_line` probe only tests `-e` while `.ONESHELL:` is a target.
///
/// Without recipe batching make runs each line in its own shell and reports the
/// first failure whatever the flags, so that probe would pass while proving
/// nothing. `.ONESHELL = 1` is the trap this guards: it defines a variable of
/// that name and leaves batching off, and `definition` reports it as a variable
/// for exactly this reason.
///
/// # Errors
///
/// Fails when the Makefile cannot be read or no longer declares `.ONESHELL:`
/// as a target.
#[rstest]
fn the_prologue_the_probes_copy_still_batches_recipes(
    prologue: Result<Vec<String>, String>,
) -> Result<(), String> {
    let lines = prologue?;
    if lines
        .iter()
        .any(|line| definition(line) == Some(Definition::Target(".ONESHELL")))
    {
        return Ok(());
    }
    Err(format!(
        "the Makefile prologue must declare `.ONESHELL:` as a target, found {lines:?}"
    ))
}

/// The prologue must fail a recipe only for the reason under test.
///
/// A `SHELL` that cannot be started, or a flag the shell rejects, fails every
/// recipe and would satisfy both probes below without either flag being set.
/// Running a recipe in which nothing fails separates the two.
///
/// # Errors
///
/// Fails when the Makefile cannot be read, make cannot be run, or make reports
/// a failure over a recipe that has none.
#[rstest]
fn a_recipe_that_fails_nowhere_succeeds(
    prologue: Result<Vec<String>, String>,
) -> Result<(), String> {
    let lines = prologue?;
    let outcome = probe_outcome(&lines, CLEAN_PROBE)?;
    if outcome.status.success() {
        return Ok(());
    }
    Err(format!(
        "the prologue {lines:?} fails a recipe in which nothing fails, so the \
         probes below prove nothing: {}",
        String::from_utf8_lossy(&outcome.stderr).trim()
    ))
}

/// A failure the shell's defaults would hide must fail the whole recipe.
///
/// Driven through make on a scratch Makefile carrying the repository's own
/// shell prologue, so each case follows the real configuration rather than a
/// copy of it. The cases are the two independent defaults: `earlier_line` is
/// the `.ONESHELL:` and `-e` contract, where one shell runs the whole recipe
/// and its last command would otherwise decide the status; `pipeline` is the
/// `-o pipefail` contract, where a shell reports a pipeline's last command's
/// status whether the recipe is batched or not.
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
