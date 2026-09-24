//! What the one workflow allowed to publish coverage must itself hold to.
//!
//! `coverage-main.yml` owns the `CodeScene` upload and the ratchet baseline
//! every pull request compares against. Moving the upload there is only a
//! boundary if the publisher cannot be made to publish from anywhere else and
//! cannot be cancelled halfway through writing what the next pull request
//! will read.
//!
//! The trunk is guarded twice. The publisher answers a push to `main` and
//! nothing else, which the contract asserts by equality; and the upload's
//! condition carries `github.ref == 'refs/heads/main'` as a whole conjunct,
//! which `upload_condition_offences` reads by splitting on `&&` and refusing
//! any unquoted `||`, so the guard still holds should another trigger be added.
//!
//! These read the raw document, since the typed model carries neither the push
//! filter nor the concurrency settings.
//!
//! # Examples
//!
//! ```no_run
//! let document: serde_norway::Value =
//!     serde_norway::from_str("on:\n  push:\n    branches: [main]\njobs: {}\n")?;
//! assert_eq!(coverage_publisher::push_branches(&document), Some(vec!["main".to_owned()]));
//! # Ok::<(), serde_norway::Error>(())
//! ```

use serde_norway::Value;

/// The branch the publisher's push trigger must name, and name alone.
pub const TRUNK_BRANCH: &str = "main";

/// Returns the `on` values of a document under both keys it can arrive under.
fn trigger_values(document: &Value) -> impl Iterator<Item = &Value> {
    [document.get("on"), document.get(Value::Bool(true))]
        .into_iter()
        .flatten()
}

/// Returns the branch filter on a document's push trigger, if it declares one.
#[must_use]
pub fn push_branches(document: &Value) -> Option<Vec<String>> {
    trigger_values(document)
        .filter_map(|declared| declared.get("push")?.get("branches")?.as_sequence())
        .map(|branches| {
            branches
                .iter()
                .map(|branch| branch.as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .next()
}

/// Reports whether one concurrency setting can cancel a run in progress.
///
/// A group given as a bare string never cancels. Anything other than an
/// absent or false `cancel-in-progress` is refused, an expression included,
/// because an expression that reads false today is one edit from true.
fn cancels(concurrency: Option<&Value>) -> bool {
    let Some(setting) = concurrency.and_then(|value| value.get("cancel-in-progress")) else {
        return false;
    };
    !matches!(setting, Value::Bool(false)) && setting.as_str() != Some("false")
}

/// Returns every scope in a document whose concurrency cancels in progress.
#[must_use]
pub fn cancelling_scopes(document: &Value) -> Vec<String> {
    let mut scopes = Vec::new();
    if cancels(document.get("concurrency")) {
        scopes.push("the workflow".to_owned());
    }
    let jobs = document.get("jobs").and_then(Value::as_mapping);
    for (id, job) in jobs.into_iter().flatten() {
        if cancels(job.get("concurrency")) {
            scopes.push(format!("job {}", id.as_str().unwrap_or_default()));
        }
    }
    scopes
}

/// The one command the credential check may run.
///
/// The expression is evaluated before the shell starts, so the step writes a
/// literal `true` or `false`, holds no shell conditional, and puts the token
/// in no step's `env`.
pub const CREDENTIAL_CHECK_COMMAND: &str =
    "echo \"available=${{ secrets.CS_ACCESS_TOKEN != '' }}\" >> \"$GITHUB_OUTPUT\"";

/// The input the upload must pass, read from the secret directly.
pub const TOKEN_INPUT: &str = "${{ secrets.CS_ACCESS_TOKEN }}";

/// The conjunct that confines the upload to the trunk, compared exactly after
/// whitespace is normalised, because a looser match accepts a sibling field.
pub const TRUNK_REF_GUARD: &str = "github.ref == 'refs/heads/main'";

/// Returns a condition's body without the optional `${{ }}` wrapper.
fn condition_body(condition: &str) -> &str {
    let body = condition.trim();
    body.strip_prefix("${{")
        .and_then(|inner| inner.strip_suffix("}}"))
        .map_or(body, str::trim)
}

/// Returns the condition with every single-quoted literal emptied, so an
/// operator inside a literal is not read as one.
fn without_literals(body: &str) -> String {
    let mut kept = String::with_capacity(body.len());
    let mut in_literal = false;
    for character in body.chars() {
        if character == '\'' {
            in_literal = !in_literal;
            kept.push(character);
        } else if !in_literal {
            kept.push(character);
        }
    }
    kept
}

/// Returns why an upload condition fails to confine it to the trunk.
///
/// Split on `&&`, the trunk guard must be one whole conjunct, and an unquoted
/// `||` is refused outright: `guard && extra || dispatch` keeps the guard
/// whole while making every conjunct optional.
#[must_use]
pub fn upload_condition_offences(condition: &str) -> Vec<String> {
    let body = condition_body(condition);
    let mut offences = Vec::new();
    if without_literals(body).contains("||") {
        offences.push("the upload condition contains an unquoted `||`".to_owned());
    }
    let has_guard = body
        .split("&&")
        .any(|part| part.split_whitespace().collect::<Vec<_>>().join(" ") == TRUNK_REF_GUARD);
    if !has_guard {
        offences.push(format!(
            "the upload condition must require {TRUNK_REF_GUARD}"
        ));
    }
    offences
}

/// The credential's name, which no `env` on the publisher may bind.
const CREDENTIAL_NAME: &str = "CS_ACCESS_TOKEN";

/// Returns the id of the step whose `available` output a condition requires.
fn check_step_id(condition: &str) -> Option<&str> {
    condition_body(condition).split("&&").find_map(|part| {
        part.trim()
            .strip_prefix("steps.")?
            .strip_suffix(".outputs.available == 'true'")
    })
}

/// Returns why one credential check step is not the prescribed one.
fn check_step_offences(check: &Value) -> Vec<String> {
    let mut offences = Vec::new();
    if check.get("run").and_then(Value::as_str).map(str::trim) != Some(CREDENTIAL_CHECK_COMMAND) {
        offences.push(format!(
            "the check must run exactly {CREDENTIAL_CHECK_COMMAND:?}"
        ));
    }
    offences.extend(
        ["if", "env", "uses"]
            .into_iter()
            .filter(|key| check.get(*key).is_some())
            .map(|key| format!("the check must not declare `{key}`")),
    );
    offences
}

/// Returns why an upload is not gated on the prescribed check, given the
/// steps that run before it.
fn upload_offences(upload: &Value, earlier: &[Value]) -> Vec<String> {
    let mut offences = Vec::new();
    if upload
        .get("with")
        .and_then(|with| with.get("access-token"))
        .and_then(Value::as_str)
        != Some(TOKEN_INPUT)
    {
        offences.push(format!(
            "the upload must pass {TOKEN_INPUT} as access-token"
        ));
    }
    let condition = upload.get("if").and_then(Value::as_str).unwrap_or_default();
    offences.extend(upload_condition_offences(condition));
    let Some(id) = check_step_id(condition) else {
        offences.push("the upload's condition requires no credential check output".to_owned());
        return offences;
    };
    match earlier
        .iter()
        .rev()
        .find(|step| step.get("id").and_then(Value::as_str) == Some(id))
    {
        Some(check) => offences.extend(check_step_offences(check)),
        None => offences.push(format!("no step before the upload has the id {id:?}")),
    }
    offences
}

/// Returns why the uploads calling `action` are not checked and passed as
/// prescribed, or one offence when no step calls it at all.
///
/// Asserted positively: a guard on a binding that has been deleted passes, and
/// the upload then skips on every run without failing anything.
#[must_use]
pub fn credential_check_offences(document: &Value, action: &str) -> Vec<String> {
    let calls = |step: &Value| {
        step.get("uses")
            .and_then(Value::as_str)
            .is_some_and(|uses| uses.split('@').next() == Some(action))
    };
    let mut uploads = 0;
    let mut offences = Vec::new();
    for steps in job_steps(document) {
        for (index, step) in steps.iter().enumerate().filter(|(_, step)| calls(step)) {
            uploads += 1;
            let earlier = steps.get(..index).unwrap_or_default();
            offences.extend(upload_offences(step, earlier));
        }
    }
    if uploads == 0 {
        offences.push(format!("no step calls {action}"));
    }
    offences
}

/// Returns every job's steps as a slice.
fn job_steps(document: &Value) -> impl Iterator<Item = &[Value]> {
    document
        .get("jobs")
        .and_then(Value::as_mapping)
        .into_iter()
        .flat_map(|jobs| jobs.values())
        .filter_map(|job| job.get("steps").and_then(Value::as_sequence))
        .map(Vec::as_slice)
}

/// Reports whether one `env` mapping names the credential, in any case.
fn binds(env: Option<&Value>) -> bool {
    env.and_then(Value::as_mapping).is_some_and(|mapping| {
        mapping
            .keys()
            .filter_map(Value::as_str)
            .any(|key| key.eq_ignore_ascii_case(CREDENTIAL_NAME))
    })
}

/// Returns every `env` scope in a document that binds the credential.
///
/// The upload action is composite and hands its step's `env` to the
/// `upload-artifact` and cache steps nested inside it, so no scope may.
#[must_use]
pub fn credential_bindings(document: &Value) -> Vec<String> {
    let workflow = std::iter::once(("the workflow".to_owned(), document.get("env")));
    let jobs = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .into_iter()
        .flatten()
        .flat_map(|(key, job)| job_env_scopes(key.as_str().unwrap_or_default(), job));
    workflow
        .chain(jobs)
        .filter(|(_, env)| binds(*env))
        .map(|(scope, _)| scope)
        .collect()
}

/// Returns a job's own `env` scope followed by each of its steps' scopes.
fn job_env_scopes<'a>(
    name: &'a str,
    job: &'a Value,
) -> impl Iterator<Item = (String, Option<&'a Value>)> + 'a {
    let steps = job
        .get("steps")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
        .enumerate()
        .map(move |(index, step)| (format!("job {name} step {index}"), step.get("env")));
    std::iter::once((format!("job {name}"), job.get("env"))).chain(steps)
}
