//! The one `runs-on` expression this estate writes, and what counts as it.
//!
//! A pull request from a fork cannot obtain an Ubicloud runner, so a lane that
//! serves pull requests names two runners and chooses between them when the
//! workflow is evaluated. This module reads that declaration and nothing else.
//!
//! It is separate from `workflow_model` because the question differs. That
//! module states the shapes a `runs-on` can have; this one states what counts
//! as the expression shape, which is a grammar with its own cases and its own
//! reasons for refusing what it refuses.
//!
//! One spelling is read. A negated guard or a comparison says the same thing
//! with the arms the other way round, and reading either as the prescribed
//! form would let two spellings of the same placement drift apart while both
//! satisfied the contract.
//!
//! # Examples
//!
//! ```no_run
//! let read = placement_expression::read("${{ a.b && 'x' || 'y' }}");
//! assert!(read.is_some());
//! ```

/// Returns the text inside `${{` and `}}`, or `None` when it is not wrapped.
fn expression_body(text: &str) -> Option<&str> {
    text.strip_prefix("${{")?.strip_suffix("}}")
}

/// Returns a single-quoted literal's contents, or `None` for anything else.
///
/// GitHub's expression syntax has no escape inside a single-quoted literal
/// other than a doubled quote, so a value containing one is not the simple
/// literal this reader accepts and is refused rather than guessed at.
fn quoted_literal(text: &str) -> Option<&str> {
    let inner = text.trim().strip_prefix('\'')?.strip_suffix('\'')?;
    (!inner.contains('\'')).then_some(inner)
}

/// Reports whether the text is a bare context path such as `github.event.x`.
///
/// A guard has to be one field reference. Anything else, a call, a comparison
/// or a second operator, is a different question about the pull request and is
/// refused here so the assertion naming the expected field can report it.
fn is_context_path(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_' || character == '.')
}

/// Reads a placement expression as its guard and its two arms.
///
/// The fork's runner comes first and this repository's own second, matching
/// the order the expression writes them in.
///
/// A literal label, a matrix reference and a declaration carrying a line break
/// all read as `None`, so each is refused by the assertion written for it
/// rather than repaired here. The line break matters most: a folded scalar
/// whose continuation is indented deeper than its key keeps the break, GitHub
/// evaluates the value regardless, and a green run is therefore no evidence
/// that the declaration is well formed.
#[must_use]
pub fn read(text: &str) -> Option<(String, [String; 2])> {
    if text.contains('\n') {
        return None;
    }
    let body = expression_body(text)?;
    let (guard, arms) = body.split_once("&&")?;
    let (fork, owned) = arms.split_once("||")?;
    if !is_context_path(guard) {
        return None;
    }
    Some((
        guard.trim().to_owned(),
        [
            quoted_literal(fork)?.to_owned(),
            quoted_literal(owned)?.to_owned(),
        ],
    ))
}
