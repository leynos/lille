//! Contracts over the `CodeScene` uploader's deprecated checksum inputs.
//!
//! At [`UPLOAD_CODESCENE_COVERAGE_SHA`] the shared uploader's committed
//! `cli-manifest.json` is the trust anchor for the `cs-coverage` archive, and
//! the action *rejects* a non-empty `installer-checksum` with a hard failure
//! rather than ignoring it. Both of this repository's callers passed
//! `${{ vars.CODESCENE_CLI_SHA256 }}`, so the coverage steps would have failed
//! outright the moment that variable held a value, and the variable itself
//! could only ever repeat the digest the manifest already pins.
//!
//! Four concerns are asserted, each in its own test so a failure names the
//! defect rather than a bundle: no workflow passes the input, none references
//! the variable that fed it, every uploader reference resolves to the approved
//! commit, and the dispatch workflow that refreshed the variable is gone.
//!
//! Each test asserts over a collection it first checks for content. A contract
//! that ranges over an empty collection is satisfied by deleting the thing it
//! guards, which is the failure mode these tests exist to avoid.

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use rstest::rstest;

use crate::workflow_assertions::workflows;
use crate::workflow_estate::{
    Workflow, WorkflowError, UPLOAD_CODESCENE_COVERAGE_SHA, WORKFLOW_DIR,
};
use crate::workflow_loader::all_steps;

/// The uploader's coordinate, without its pinned reference.
const UPLOADER: &str = "leynos/shared-actions/.github/actions/upload-codescene-coverage";

/// The deprecated input the uploader now rejects.
const DEPRECATED_INPUT: &str = "installer-checksum";

/// The repository variable that existed only to feed that input.
const DEPRECATED_VARIABLE: &str = "CODESCENE_CLI_SHA256";

/// The dispatch workflow that refreshed the variable.
const REFRESH_WORKFLOW: &str = "get-codescene-sha.yml";

/// Returns the workflow directory as an absolute path.
fn workflow_root() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(WORKFLOW_DIR)
}

/// Reads one directory entry as a workflow file name and its text.
///
/// Returns `Ok(None)` for an entry that is not a workflow, so a stray file
/// beside the workflows is skipped rather than failing the read.
///
/// # Errors
///
/// Returns an error when the entry, its name, or its contents cannot be read.
fn read_entry(
    dir: &Dir,
    entry: Result<cap_std::fs_utf8::DirEntry, std::io::Error>,
) -> Result<Option<(String, String)>, WorkflowError> {
    let read = |name: &str| {
        let owned = name.to_owned();
        move |err| WorkflowError::Read(owned.clone(), err)
    };
    let name = entry
        .map_err(read(WORKFLOW_DIR))?
        .file_name()
        .map_err(read(WORKFLOW_DIR))?;
    let extension = Utf8Path::new(&name).extension().unwrap_or_default();
    if !matches!(extension, "yml" | "yaml") {
        return Ok(None);
    }
    let text = dir.read_to_string(&name).map_err(read(&name))?;
    Ok(Some((name, text)))
}

/// Reads every workflow file name paired with its raw text.
///
/// The text is read rather than the parsed document so that a reference left
/// in a comment, or in a step commented out, is caught as well. Both of
/// GitHub's accepted extensions are matched: reading only one would let a
/// workflow escape every rule below without failing a test.
///
/// # Errors
///
/// Returns an error when the workflow directory cannot be opened or listed,
/// or when one of its files cannot be read.
fn read_workflow_texts() -> Result<Vec<(String, String)>, WorkflowError> {
    let root = workflow_root();
    let dir = Dir::open_ambient_dir(&root, ambient_authority())
        .map_err(|err| WorkflowError::Read(root.to_string(), err))?;
    let entries = dir
        .entries()
        .map_err(|err| WorkflowError::Read(root.to_string(), err))?;
    let mut found: Vec<(String, String)> = Vec::new();
    for entry in entries {
        if let Some(pair) = read_entry(&dir, entry)? {
            found.push(pair);
        }
    }
    found.sort();
    Ok(found)
}

/// Returns every workflow file name paired with its raw text.
///
/// # Panics
///
/// Panics when the estate cannot be read, or when it holds no workflow at
/// all: every rule below would then pass having read nothing.
fn workflow_texts() -> Vec<(String, String)> {
    let found = match read_workflow_texts() {
        Ok(found) => found,
        Err(err) => panic!("the workflow estate must be readable: {err}"),
    };
    assert!(
        !found.is_empty(),
        "no workflow was read from {}, so every rule below would pass having read nothing",
        workflow_root()
    );
    found
}

/// Returns the workflow files whose text contains `needle`.
fn workflows_containing(needle: &str) -> Vec<String> {
    workflow_texts()
        .into_iter()
        .filter(|(_, text)| text.contains(needle))
        .map(|(name, _)| name)
        .collect()
}

#[rstest]
fn no_workflow_passes_the_deprecated_installer_checksum() {
    let offenders = workflows_containing(DEPRECATED_INPUT);
    assert!(
        offenders.is_empty(),
        "{DEPRECATED_INPUT} is deprecated and rejected outright by the uploader at \
         {UPLOAD_CODESCENE_COVERAGE_SHA}; remove it from {offenders:?}"
    );
}

#[rstest]
fn no_workflow_references_the_deprecated_checksum_variable() {
    let offenders = workflows_containing(DEPRECATED_VARIABLE);
    assert!(
        offenders.is_empty(),
        "{DEPRECATED_VARIABLE} fed the deprecated installer checksum and has no remaining \
         consumer; remove it from {offenders:?}"
    );
}

#[rstest]
fn every_uploader_reference_is_pinned_to_the_approved_commit(workflows: Vec<Workflow>) {
    let references: Vec<(String, String, String)> = all_steps(&workflows)
        .into_iter()
        .filter(|(_, _, step)| step.uses.starts_with(UPLOADER))
        .map(|(file, job, step)| (file, job, step.uses))
        .collect();
    // The pin rule below is an allowlist, not a floor: SHAs cannot be ordered
    // from a checkout. Naming the approved commit keeps it hermetic and fails
    // closed on any other value, including a tag or a branch name. Asserting
    // the set is non-empty first is what stops deletion from satisfying it.
    assert!(
        !references.is_empty(),
        "no upload-codescene-coverage reference was found, so the pin rule below would pass \
         vacuously; this repository uploads coverage from main and checks it on pull requests"
    );
    let expected = format!("{UPLOADER}@{UPLOAD_CODESCENE_COVERAGE_SHA}");
    let wrong: Vec<String> = references
        .into_iter()
        .filter(|(_, _, uses)| *uses != expected)
        .map(|(file, job, uses)| format!("{file}:{job}: {uses}"))
        .collect();
    assert!(
        wrong.is_empty(),
        "every upload-codescene-coverage reference must pin \
         {UPLOAD_CODESCENE_COVERAGE_SHA}: {wrong:?}"
    );
}

#[rstest]
fn the_checksum_refresh_workflow_is_absent() {
    let present = workflow_texts()
        .into_iter()
        .any(|(name, _)| name == REFRESH_WORKFLOW);
    assert!(
        !present,
        "{REFRESH_WORKFLOW} recomputed {DEPRECATED_VARIABLE}, which no workflow reads any \
         more; delete it rather than leave a dispatch that refreshes an unused value"
    );
}
