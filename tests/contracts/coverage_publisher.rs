//! CV-005 contracts on the one workflow allowed to publish coverage.
//!
//! Split from `coverage_reach` so each module stays under 400 lines. The
//! publisher's trigger is its trunk guard, held by equality; its upload is
//! gated on a credential check and passed the secret directly, with the token
//! bound in no `env`; and its runs never cancel and never overlap. Each reader
//! is driven over synthetic documents as well as the real file, because over
//! one file a reader that answered nothing would agree with a correct one.

use rstest::rstest;
use serde_norway::Value;

use crate::coverage_boundary::UPLOAD_COVERAGE_ACTION;
use crate::coverage_publisher::{
    cancelling_scopes, credential_bindings, credential_check_offences, push_branches,
    upload_condition_offences, CREDENTIAL_CHECK_COMMAND, TRUNK_BRANCH, TRUNK_REF_GUARD,
};
use crate::workflow_assertions::workflows;
use crate::workflow_estate::Workflow;
use crate::workflow_loader::repository_workflow_text;

/// The lane that owns the upload.
const PUBLISHER_WORKFLOW: &str = "coverage-main.yml";

/// Parses a document without the typed model, or panics naming the failure.
fn raw_document(text: &str) -> Value {
    match serde_norway::from_str(text) {
        Ok(document) => document,
        Err(err) => panic!("the document must parse: {err}"),
    }
}

/// Returns the publisher's raw document, parsed without the typed model.
fn publisher_document() -> Value {
    match repository_workflow_text(PUBLISHER_WORKFLOW) {
        Ok(text) => raw_document(&text),
        Err(err) => panic!("{PUBLISHER_WORKFLOW} must be readable: {err}"),
    }
}

/// The trigger is the trunk guard, so it is held by equality.
#[rstest]
fn the_publisher_answers_a_push_to_the_trunk_and_nothing_else(workflows: Vec<Workflow>) {
    let found = workflows
        .iter()
        .find(|workflow| workflow.file == PUBLISHER_WORKFLOW);
    let Some(publisher) = found else {
        panic!("the estate must define {PUBLISHER_WORKFLOW}")
    };
    assert_eq!(
        publisher.triggers,
        ["push"],
        "{PUBLISHER_WORKFLOW} carries no ref guard of its own, so any second \
         trigger, `workflow_dispatch` included, could upload another branch"
    );
    assert_eq!(
        push_branches(&publisher_document()),
        Some(vec![TRUNK_BRANCH.to_owned()]),
        "{PUBLISHER_WORKFLOW} must publish on a push to {TRUNK_BRANCH} alone"
    );
}

/// The upload is gated on a credential check and passed the secret directly,
/// with the token bound in no `env`: the composite upload action hands its
/// step's `env` to the nested `upload-artifact` and cache steps.
#[rstest]
fn the_publisher_checks_the_credential_and_binds_it_nowhere() {
    let document = publisher_document();
    let offences = credential_check_offences(&document, UPLOAD_COVERAGE_ACTION);
    assert!(
        offences.is_empty(),
        "{PUBLISHER_WORKFLOW}'s upload: {offences:?}"
    );
    let bound = credential_bindings(&document);
    assert!(
        bound.is_empty(),
        "{PUBLISHER_WORKFLOW} must bind CS_ACCESS_TOKEN in no env: {bound:?}"
    );
}

/// Each way the check can be missing or wrong, driven over synthetic steps.
#[rstest]
#[case::the_prescribed_shape("", true)]
#[case::no_check_step("drop", false)]
#[case::another_command("echo", false)]
#[case::a_check_with_a_condition("if", false)]
#[case::a_check_after_the_upload("after", false)]
#[case::an_upload_ignoring_the_check("ignore", false)]
#[case::the_token_from_env("env-input", false)]
fn the_credential_check_is_the_prescribed_step(#[case] variant: &str, #[case] accepted: bool) {
    let check = match variant {
        "echo" => "  - id: cred\n    run: echo available=true\n".to_owned(),
        "if" => format!(
            "  - id: cred\n    if: 'false'\n    run: '{}'\n",
            CREDENTIAL_CHECK_COMMAND.replace('\'', "''")
        ),
        _ => format!(
            "  - id: cred\n    run: '{}'\n",
            CREDENTIAL_CHECK_COMMAND.replace('\'', "''")
        ),
    };
    let condition = if variant == "ignore" {
        format!("success() && {TRUNK_REF_GUARD}")
    } else {
        format!("steps.cred.outputs.available == 'true' && {TRUNK_REF_GUARD}")
    };
    let input = if variant == "env-input" {
        "${{ env.CS_ACCESS_TOKEN }}"
    } else {
        "${{ secrets.CS_ACCESS_TOKEN }}"
    };
    let upload = format!(
        "  - if: {condition}\n    uses: {UPLOAD_COVERAGE_ACTION}@abc\n    with:\n      access-token: '{input}'\n"
    );
    let steps = match variant {
        "drop" => upload,
        "after" => format!("{upload}{check}"),
        _ => format!("{check}{upload}"),
    };
    let mut indented = String::new();
    for line in steps.lines() {
        indented.push_str("      ");
        indented.push_str(line);
        indented.push('\n');
    }
    let document = raw_document(&format!("jobs:\n  a:\n    steps:\n{indented}"));
    assert_eq!(
        credential_check_offences(&document, UPLOAD_COVERAGE_ACTION).is_empty(),
        accepted,
        "{variant:?}"
    );
}

#[rstest]
#[case::workflow("env:\n  CS_ACCESS_TOKEN: x\njobs: {}\n", 1)]
#[case::job_in_another_case("jobs:\n  a:\n    env:\n      cs_access_token: x\n", 1)]
#[case::another_name("jobs:\n  a:\n    steps:\n      - env:\n          OTHER: x\n", 0)]
fn a_credential_bound_in_any_env_is_found(#[case] text: &str, #[case] expected: usize) {
    assert_eq!(
        credential_bindings(&raw_document(text)).len(),
        expected,
        "{text}"
    );
}

/// Runs never cancel and never overlap: one group keyed on the ref alone,
/// so the newest trigger replaces a pending run and triggered runs upload in
/// commit order. A manual re-run of an older run republishes that commit until
/// the next push supersedes it, which is an operator's choice, not a race.
#[rstest]
fn the_publisher_never_cancels_and_is_keyed_on_the_ref() {
    let document = publisher_document();
    let scopes = cancelling_scopes(&document);
    assert!(
        scopes.is_empty(),
        "a cancelled publisher abandons its upload and the baseline the next pull \
         request reads; cancel-in-progress is set on {scopes:?}"
    );
    let concurrency = document.get("concurrency");
    assert_eq!(
        concurrency
            .and_then(|value| value.get("group"))
            .and_then(Value::as_str),
        Some("coverage-main-${{ github.ref }}"),
        "the group must be keyed on the ref alone"
    );
    assert_eq!(
        concurrency.and_then(|value| value.get("cancel-in-progress")),
        Some(&Value::Bool(false)),
        "cancel-in-progress must be declared false"
    );
}

#[rstest]
#[case::a_literal_true("cancel-in-progress: true", true)]
#[case::a_quoted_true("cancel-in-progress: 'true'", true)]
#[case::an_expression("cancel-in-progress: ${{ github.event_name == 'push' }}", true)]
#[case::a_literal_false("cancel-in-progress: false", false)]
#[case::no_setting("# no cancel-in-progress", false)]
fn a_cancelling_setting_is_found_at_either_scope(
    #[case] setting: &str,
    #[case] cancels: bool,
    #[values(true, false)] at_workflow_level: bool,
) {
    let text = if at_workflow_level {
        format!("concurrency:\n  group: g\n  {setting}\njobs:\n  a: {{}}\n")
    } else {
        format!("jobs:\n  a:\n    concurrency:\n      group: g\n      {setting}\n")
    };
    let document = raw_document(&text);
    assert_eq!(!cancelling_scopes(&document).is_empty(), cancels, "{text}");
}

#[rstest]
#[case::the_trunk("on:\n  push:\n    branches: [main]\n", Some(vec!["main"]))]
#[case::a_second_branch("on:\n  push:\n    branches: [main, 'release/*']\n", Some(vec!["main", "release/*"]))]
#[case::a_tag_workflow("on:\n  push:\n    tags: ['v*']\n", None)]
#[case::an_unfiltered_push("on: push\n", None)]
fn the_push_filter_is_read_as_declared(#[case] text: &str, #[case] expected: Option<Vec<&str>>) {
    let document = raw_document(text);
    let owned: Option<Vec<String>> =
        expected.map(|branches| branches.into_iter().map(ToOwned::to_owned).collect());
    assert_eq!(push_branches(&document), owned);
}

/// The trunk guard must be a whole conjunct, and no unquoted `||` may make it
/// optional. The last refused case is the one that proves the `||` refusal:
/// every required conjunct stays whole, so only that refusal catches it.
#[rstest]
#[case::the_prescribed_pair("steps.c.outputs.available == 'true' && GUARD", true)]
#[case::wrapped_and_spaced("${{ GUARD  &&  success() }}", true)]
#[case::an_or_inside_a_literal("GUARD && github.actor != 'a||b'", true)]
#[case::no_guard("steps.c.outputs.available == 'true'", false)]
#[case::the_negation("github.ref != 'refs/heads/main'", false)]
#[case::the_guard_inside_a_negation("success() && !(GUARD)", false)]
#[case::a_sibling_field("github.base_ref == 'refs/heads/main'", false)]
#[case::an_or_hidden_in_an_extra_conjunct(
    "GUARD && github.actor != 'x' || github.event_name == 'workflow_dispatch'",
    false
)]
fn the_upload_condition_requires_the_trunk_as_a_whole_conjunct(
    #[case] condition: &str,
    #[case] accepted: bool,
) {
    let expanded = condition.replace("GUARD", TRUNK_REF_GUARD);
    assert_eq!(
        upload_condition_offences(&expanded).is_empty(),
        accepted,
        "{expanded}"
    );
}
