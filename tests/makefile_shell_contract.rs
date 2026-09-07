//! Contract over the shell configuration this repository's Makefile uses.
//!
//! `.ONESHELL:` hands a whole recipe to one shell invocation, so only that
//! shell's final exit status reaches make. Without an errexit flag a failing
//! lint or test on an earlier line of a multi-line recipe is invisible and a
//! gate reports success over a red run. Wildside proved the failure mode: run
//! 33939820204 logged ruff's "Found 3 errors." and the job passed.
//!
//! The contract drives GNU make rather than reading the Makefile for the flag,
//! because an assertion that finds `.SHELLFLAGS` is satisfied by a setting that
//! does not enable errexit. It builds a scratch Makefile from the real file's
//! own shell prologue and gives it a recipe whose first line fails and whose
//! second succeeds, then asserts make reports the failure.
//!
//! Mutation record: with the `.SHELLFLAGS := -ec` line deleted from the
//! repository's Makefile, `a_failing_line_fails_the_whole_recipe` fails because
//! make exits 0 on that probe. Measured 2026-09-07 with GNU Make 4.4.1.

use std::process::{Command, Output};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use tempfile::TempDir;

/// The make definitions that decide how a multi-line recipe is executed.
///
/// Every other line of the Makefile is irrelevant to the probe, and copying
/// only these keeps the scratch file independent of the repository's targets.
const PROLOGUE_NAMES: [&str; 3] = [".ONESHELL", "SHELL", ".SHELLFLAGS"];

/// Name of the generated Makefile inside the scratch directory.
const PROBE_MAKEFILE: &str = "Makefile";

/// The probe recipe appended to the copied prologue.
///
/// The first command fails and the second succeeds, which is the exact shape a
/// gate takes when an early tool reports findings and a later one is clean.
const PROBE_RECIPE: &str = "\nprobe:\n\tfalse\n\ttrue\n";

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
    dir.read_to_string("Makefile")
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

/// Runs the probe recipe under the given prologue and returns make's result.
///
/// The child's `MAKEFLAGS` is cleared so that a run started from `make test`
/// cannot lend the probe its parent's flags, which would decide the outcome
/// instead of the prologue under test. Output is captured rather than
/// inherited, so make's own echo of the failing command does not read as a
/// failure of this test run.
///
/// # Errors
///
/// Returns a message when the scratch directory cannot be created, the scratch
/// Makefile cannot be written, or `make` cannot be executed. The scratch file
/// is written through a directory capability rooted at the scratch directory,
/// so the probe cannot write outside it.
fn probe_outcome(prologue: &[String]) -> Result<Output, String> {
    let scratch =
        TempDir::new().map_err(|err| format!("cannot create a scratch directory: {err}"))?;
    let root = Utf8Path::from_path(scratch.path())
        .ok_or_else(|| "the scratch directory path is not UTF-8".to_owned())?;
    let dir = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|err| format!("cannot open the scratch directory {root}: {err}"))?;
    let text = format!("{}\n{PROBE_RECIPE}", prologue.join("\n"));
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

/// Reads the repository's shell prologue, or panics naming what went wrong.
fn repository_prologue() -> Vec<String> {
    match repository_makefile() {
        Ok(makefile) => prologue_lines(&makefile),
        Err(message) => panic!("{message}"),
    }
}

/// The probe only tests errexit while the prologue keeps `.ONESHELL:`.
///
/// Without it make runs each recipe line in its own shell and reports the first
/// failure regardless of any flag, so the behavioural contract below would pass
/// while proving nothing. This guard makes that silent weakening a test
/// failure.
#[test]
fn the_prologue_the_probe_copies_still_batches_recipes() {
    let prologue = repository_prologue();
    assert!(
        prologue
            .iter()
            .any(|line| declared_name(line) == Some(".ONESHELL")),
        "the Makefile prologue must declare .ONESHELL:, found {prologue:?}"
    );
}

/// A gate's earlier failing command must fail the whole recipe.
///
/// Driven through make on a scratch Makefile carrying the repository's own
/// shell prologue, so the assertion follows the real configuration rather than
/// a copy of it.
#[test]
fn a_failing_line_fails_the_whole_recipe() {
    let prologue = repository_prologue();
    match probe_outcome(&prologue) {
        Ok(outcome) => assert!(
            !outcome.status.success(),
            "make must fail a recipe whose first line fails; \
             the prologue {prologue:?} lets the failure pass unreported"
        ),
        Err(message) => panic!("{message}"),
    }
}
