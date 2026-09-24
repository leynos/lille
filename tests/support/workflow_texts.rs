//! Reads workflow files as raw text, for questions the parser cannot answer.
//!
//! The parsed document is not enough for every contract: a credential named in
//! a comment, or a value in a shape the parser flattened away, is still in the
//! file. These readers return the text as written, through the same
//! `cap_std` directory capability and the same file listing the loader uses,
//! so a contract never reaches the filesystem itself and the raw and parsed
//! readings always cover the same files.
//!
//! Split from `workflow_loader`, which parses, to keep both under the 400-line
//! limit; any contract that needs raw text reads it through here.
//!
//! # Examples
//!
//! ```no_run
//! let texts = workflow_texts::repository_workflow_texts()?;
//! assert!(texts.iter().any(|(name, _)| name == "ci.yml"));
//! # Ok::<(), workflow_estate::WorkflowError>(())
//! ```

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};

use crate::workflow_estate::{WorkflowError, WORKFLOW_DIR};
use crate::workflow_loader::workflow_names;

/// Returns one workflow file's raw text, read through a directory capability.
///
/// The parsed document is not enough for every question: a credential named in
/// a comment, or in a shape the parser flattened away, is still a credential
/// the file carries. Reading it here keeps the ambient step in the one module
/// that already owns it rather than letting a contract reach the filesystem.
///
/// # Errors
///
/// Returns an error when the workflow directory cannot be opened or the file
/// cannot be read.
pub fn workflow_text(root: &Utf8Path, name: &str) -> Result<String, WorkflowError> {
    let dir = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|err| WorkflowError::Read(root.to_string(), err))?;
    dir.read_to_string(name)
        .map_err(|err| WorkflowError::Read(name.to_owned(), err))
}

/// Returns one workflow file's raw text from this repository.
///
/// # Errors
///
/// Returns the same errors as [`workflow_text`].
pub fn repository_workflow_text(name: &str) -> Result<String, WorkflowError> {
    let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(WORKFLOW_DIR);
    workflow_text(&root, name)
}

/// Returns every workflow file name beneath `root` paired with its raw text.
///
/// For contracts that must see what the parser drops, such as a reference in
/// a comment, across the whole estate. Both of GitHub's accepted extensions
/// are read, in name order, through the same directory capability the parser
/// uses.
///
/// # Errors
///
/// Returns an error when the directory cannot be opened or listed, or when a
/// file cannot be read.
pub fn workflow_texts_in(root: &Utf8Path) -> Result<Vec<(String, String)>, WorkflowError> {
    let dir = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|err| WorkflowError::Read(root.to_string(), err))?;
    workflow_names(&dir)?
        .into_iter()
        .map(|name| {
            let text = dir
                .read_to_string(&name)
                .map_err(|err| WorkflowError::Read(name.clone(), err))?;
            Ok((name, text))
        })
        .collect()
}

/// Returns every workflow in this repository paired with its raw text.
///
/// # Errors
///
/// Returns the same errors as [`workflow_texts_in`].
pub fn repository_workflow_texts() -> Result<Vec<(String, String)>, WorkflowError> {
    let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(WORKFLOW_DIR);
    workflow_texts_in(&root)
}
