//! The shapes a substring search accepts while nothing is sampled.
//!
//! The reading itself is `shell_reading`; this module is the question
//! asked of it. The samplers in `.github/workflows` are all written the
//! same way, so the tree exercises one shape and says nothing about the
//! rest.

use rstest::rstest;

use crate::shell_reading::{step_samples, Measure};

#[rstest]
#[case::a_bare_command("free -m", true)]
#[case::command_substitution("mem_used=\"$(free -m | awk '{print $3}')\"", true)]
#[case::a_pipeline("free -m | tee -a \"$log\"", true)]
#[case::single_quoted("echo 'free -m'", false)]
#[case::double_quoted("echo \"free -m\"", false)]
#[case::a_comment("# free -m", false)]
#[case::a_trailing_comment("uptime  # free -m", false)]
#[case::verdict_discarded("free -m || true", false)]
#[case::verdict_discarded_with_colon("free -m || :", false)]
#[case::verdict_discarded_with_wide_spacing("free -m ||    true", false)]
#[case::escaped_substitution("echo \"\\$(free -m)\"", false)]
#[case::quoted_bracket_in_a_substitution("value=\"$(printf '%s' ')'; free -m)\"", true)]
#[case::text_after_a_substitution("echo \"$(true) free -m\"", false)]
#[case::one_line_guard("if false; then free -m; fi", false)]
#[case::multi_line_guard("if false; then\n  free -m\nfi", false)]
#[case::multi_line_guard_with_test("if [ 1 -eq 0 ]; then\n  free -m\nfi", false)]
#[case::after_a_closed_guard("if false; then\n  true\nfi\nfree -m", true)]
#[case::guard_body_naming_find("if false; then\n  find . -name x\n  free -m\nfi", false)]
fn the_sampler_reading_judges_execution_not_text(#[case] run: &str, #[case] expected: bool) {
    assert_eq!(
        step_samples(run, Measure("free -m")),
        expected,
        "`{run}` must {} as sampling `free -m`",
        if expected { "read" } else { "not read" }
    );
}
