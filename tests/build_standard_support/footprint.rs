//! The development-build footprint half of the build standard.
//!
//! A development build keeps debug information and the incremental cache off. The manifest's
//! `[profile.dev]` and `.cargo/config.toml` must both say so, or a clean build and a configured
//! one drift apart. The unstable Cargo key that controls how rustc metadata is embedded was renamed
//! across a window of nightlies, so the key must match the pinned date. Raw `metadata` rustc flags
//! in a `rustflags` line broke build scripts, so none may appear. Everything is read as text, so
//! the contract needs no parser dependency.

use super::config::{Pin, Problems};

/// The manifest, whose `[profile.dev]` must agree with the Cargo configuration.
pub const MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));

/// The footprint window a pinned nightly falls in, which decides the unstable key.
#[derive(Debug, PartialEq, Eq)]
enum Window {
    /// Predates every date the standard knows, so no setting is proven.
    Unknown,
    /// From 2025-06-26 through 2026-07-18: `no-embed-metadata = true`.
    NoEmbedMetadata,
    /// From 2026-07-19 through 2026-07-30: no setting is proven yet.
    Unproven,
    /// From 2026-07-31: `embed-metadata = false`.
    EmbedMetadata,
}

/// Returns the window a `YYYYMMDD` key falls in. The key is a number, so the bounds compare as
/// dates.
///
/// ```text
/// window(20_260_718) == Window::NoEmbedMetadata
/// window(20_260_719) == Window::Unproven
/// window(20_260_731) == Window::EmbedMetadata
/// ```
const fn window(key: u32) -> Window {
    match key {
        0..=20_250_625 => Window::Unknown,
        20_250_626..=20_260_718 => Window::NoEmbedMetadata,
        20_260_719..=20_260_730 => Window::Unproven,
        _ => Window::EmbedMetadata,
    }
}

/// Returns the text before a trailing comment, without surrounding space.
fn strip_comment(text: &str) -> &str {
    text.split(" #").next().unwrap_or_default().trim()
}

/// Returns the table named by a `[table]` header line, or `None` for any other line.
fn table_header(line: &str) -> Option<&str> {
    line.strip_prefix('[')
        .map(|name| name.trim_end_matches(']').trim())
}

/// Returns the `(key, value)` of a `key = value` line, without any trailing comment.
fn assignment(line: &str) -> Option<(&str, &str)> {
    line.split_once('=')
        .map(|(key, raw)| (key.trim(), strip_comment(raw)))
}

/// Returns the `(table, key, value)` triples of a TOML text, skipping blank and comment lines.
fn entries(text: &str) -> Vec<(String, String, String)> {
    let mut table = String::new();
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            if let Some(name) = table_header(line) {
                name.clone_into(&mut table);
                return None;
            }
            assignment(line).map(|(key, value)| (table.clone(), key.to_owned(), value.to_owned()))
        })
        .collect()
}

/// Returns whether a table sets a key to exactly this value.
///
/// ```text
/// holds("[profile.dev]\ndebug = 0\n", "profile.dev", "debug", "0") == true
/// ```
fn holds(text: &str, table: &str, key: &str, value: &str) -> bool {
    entries(text)
        .iter()
        .any(|(t, k, v)| t == table && k == key && v == value)
}

/// Returns whether any table sets the key, whatever its value.
fn sets(text: &str, key: &str) -> bool {
    entries(text).iter().any(|(_, k, _)| k == key)
}

/// Returns the complaints about one source's `[profile.dev]` debug and incremental settings.
fn dev_profile_problems(source: &str, text: &str) -> Problems {
    let mut problems = Vec::new();
    if !holds(text, "profile.dev", "debug", "0") {
        problems.push(format!("{source} does not set `debug = 0`"));
    }
    if !holds(text, "profile.dev", "incremental", "false") {
        problems.push(format!("{source} does not set `incremental = false`"));
    }
    problems
}

/// Returns whether a line opens a `rustflags` assignment.
fn opens_rustflags(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("rustflags") else {
        return false;
    };
    rest.trim_start().starts_with('=')
}

/// Returns whether the lines from a `rustflags` assignment to its closing bracket name `metadata`.
/// A value may span several lines, so the span ends at the first line holding `]`.
fn span_names_metadata<'a>(lines: impl Iterator<Item = &'a str>) -> bool {
    let mut closed = false;
    lines
        .take_while(|line| {
            let open = !closed;
            closed = line.contains(']');
            open
        })
        .any(|line| line.contains("metadata"))
}

/// Returns whether a `rustflags` assignment in the configuration names `metadata`.
fn names_metadata_in_rustflags(config: &str) -> bool {
    let lines: Vec<&str> = config
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .map(strip_comment)
        .collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| opens_rustflags(line))
        .any(|(start, _)| span_names_metadata(lines.iter().skip(start).copied()))
}

/// Returns the channel a toolchain file names under `[toolchain]`. `Pin::read` has already
/// checked that there is exactly one.
fn pinned_channel(toolchain: &str) -> Option<String> {
    entries(toolchain)
        .into_iter()
        .find(|(table, key, _)| table == "toolchain" && key == "channel")
        .map(|(_, _, value)| value.trim_matches('"').to_owned())
}

/// Returns whether the parts of a date are `YYYY`, `MM` and `DD` in that order.
fn is_date_parts(parts: &[&str]) -> bool {
    let [year, month, day] = parts else {
        return false;
    };
    is_digits(year, 4) && is_digits(month, 2) && is_digits(day, 2)
}

/// Returns whether a part is exactly `width` ASCII digits.
fn is_digits(part: &str, width: usize) -> bool {
    part.len() == width && part.chars().all(|c| c.is_ascii_digit())
}

/// Returns the `YYYYMMDD` key of a `nightly-YYYY-MM-DD` channel, so dates compare as numbers.
///
/// ```text
/// nightly_key("nightly-2026-10-14") == Some(20_261_014)
/// nightly_key("stable")             == None
/// ```
fn nightly_key(channel: &str) -> Option<u32> {
    let date = channel.strip_prefix("nightly-")?;
    let parts: Vec<&str> = date.split('-').collect();
    is_date_parts(&parts)
        .then(|| date.replace('-', "").parse().ok())
        .flatten()
}

/// Returns the complaints for a nightly from the window that takes `embed-metadata = false`.
fn embed_problems(config: &str) -> Problems {
    let mut problems = Vec::new();
    if !holds(config, "unstable", "embed-metadata", "false") {
        problems.push("`[unstable]` does not set `embed-metadata = false`".to_owned());
    }
    if sets(config, "no-embed-metadata") {
        problems.push("`no-embed-metadata` is set, which this nightly rejects".to_owned());
    }
    problems
}

/// Returns the complaints for a nightly from the window that takes `no-embed-metadata = true`.
fn no_embed_problems(config: &str) -> Problems {
    let mut problems = Vec::new();
    if !holds(config, "unstable", "no-embed-metadata", "true") {
        problems.push("`[unstable]` does not set `no-embed-metadata = true`".to_owned());
    }
    if sets(config, "embed-metadata") {
        problems.push("`embed-metadata` is set, which this nightly rejects".to_owned());
    }
    problems
}

/// Returns the complaints about the unstable key a dated nightly takes.
fn nightly_problems(config: &str, toolchain: &str) -> Problems {
    let Some(channel) = pinned_channel(toolchain) else {
        return vec!["rust-toolchain.toml names no readable channel".to_owned()];
    };
    let Some(key) = nightly_key(&channel) else {
        return vec![format!("`{channel}` is not a dated nightly")];
    };
    match window(key) {
        Window::Unknown => vec![format!("`{channel}` predates the footprint windows")],
        Window::Unproven => vec![format!("`{channel}` is in the unproven window")],
        Window::NoEmbedMetadata => no_embed_problems(config),
        Window::EmbedMetadata => embed_problems(config),
    }
}

/// Returns the complaints about the pin. A stable channel takes no unstable key.
fn pin_problems(config: &str, toolchain: &str) -> Problems {
    match Pin::read(toolchain) {
        Ok(Pin::Nightly) => nightly_problems(config, toolchain),
        Ok(Pin::Stable) => Vec::new(),
        Err(reason) => vec![format!("the pin cannot be read: {reason}")],
    }
}

/// Returns every complaint about the development-build footprint.
///
/// ```text
/// footprint_problems(CONFIG, MANIFEST, TOOLCHAIN) == []   // a compliant repository
/// ```
pub fn footprint_problems(config: &str, manifest: &str, toolchain: &str) -> Problems {
    let mut problems = dev_profile_problems("`.cargo/config.toml`", config);
    problems.extend(dev_profile_problems("`Cargo.toml`", manifest));
    problems.extend(pin_problems(config, toolchain));
    if names_metadata_in_rustflags(config) {
        problems.push("`rustflags` in `.cargo/config.toml` names `metadata`".to_owned());
    }
    problems
}

const CONFIG_OK: &str = concat!(
    "[build]\nrustflags = [\"-Zthreads=8\"]\n\n",
    "[profile.dev]\ndebug = 0\nincremental = false\n\n",
    "[unstable]\nembed-metadata = false\n",
);
const CONFIG_OLD: &str = concat!(
    "[profile.dev]\ndebug = 0\nincremental = false\n\n",
    "[unstable]\nno-embed-metadata = true\n",
);
const CONFIG_NO_DEBUG: &str = concat!(
    "[profile.dev]\nincremental = false\n\n",
    "[unstable]\nembed-metadata = false\n",
);
const CONFIG_INCR_ON: &str = concat!(
    "[profile.dev]\ndebug = 0\nincremental = true\n\n",
    "[unstable]\nembed-metadata = false\n",
);
const CONFIG_COMMENTED: &str = concat!(
    "[profile.dev]\n# debug = 0\n# incremental = false\n\n",
    "[unstable]\nembed-metadata = false\n",
);
const CONFIG_METADATA_FLAG: &str = concat!(
    "[build]\nrustflags = [\"-Zthreads=8\", \"-Cmetadata=abc\"]\n\n",
    "[profile.dev]\ndebug = 0\nincremental = false\n\n",
    "[unstable]\nembed-metadata = false\n",
);
const CONFIG_SPLIT: &str = concat!(
    "[build]\nrustflags = [\n    \"-Zthreads=8\",\n    \"-C\", \"metadata=abc\",\n]\n\n",
    "[profile.dev]\ndebug = 0\nincremental = false\n\n",
    "[unstable]\nembed-metadata = false\n",
);
const MANIFEST_OK: &str = "[profile.dev]\ndebug = 0\nincremental = false\n";
const MANIFEST_DEBUG_ON: &str = "[profile.dev]\ndebug = 2\nincremental = false\n";

/// Returns how many complaints a configuration, a manifest and a pinned channel draw.
fn problem_count(config: &str, manifest: &str, channel: &str) -> usize {
    let toolchain = format!("[toolchain]\nchannel = \"{channel}\"\n");
    footprint_problems(config, manifest, &toolchain).len()
}

/// Scenario: the dev profile in the Cargo configuration and in the manifest.
///
/// Invariant: both set `debug = 0` and `incremental = false` under `[profile.dev]`, and a
/// commented line sets nothing.
#[test]
fn the_dev_profile_wants_debug_off_and_incremental_off() {
    let nightly = "nightly-2026-10-14";
    assert_eq!(problem_count(CONFIG_OK, MANIFEST_OK, nightly), 0);
    assert_eq!(problem_count(CONFIG_NO_DEBUG, MANIFEST_OK, nightly), 1);
    assert_eq!(problem_count(CONFIG_INCR_ON, MANIFEST_OK, nightly), 1);
    assert_eq!(problem_count(CONFIG_COMMENTED, MANIFEST_OK, nightly), 2);
    assert_eq!(problem_count(CONFIG_OK, MANIFEST_DEBUG_ON, nightly), 1);
}

/// Scenario: a nightly from the embed-metadata window, with each unstable key.
///
/// Invariant: that window takes `embed-metadata = false` and rejects `no-embed-metadata`.
#[test]
fn the_unstable_key_follows_the_pinned_nightly() {
    let nightly = "nightly-2026-10-14";
    assert_eq!(problem_count(CONFIG_OK, MANIFEST_OK, nightly), 0);
    assert_eq!(problem_count(CONFIG_OLD, MANIFEST_OK, nightly), 2);
}

/// Scenario: the nightlies on either side of each window boundary.
///
/// Invariant: each row is a pinned channel, the count with `CONFIG_OK` and the count with
/// `CONFIG_OLD`. The window from 2026-07-19 through 2026-07-30 is unproven, so both count one.
#[test]
fn the_window_boundaries_pick_the_key() {
    let cases = [
        ("nightly-2025-06-25", 1, 1),
        ("nightly-2025-06-26", 2, 0),
        ("nightly-2026-07-18", 2, 0),
        ("nightly-2026-07-19", 1, 1),
        ("nightly-2026-07-30", 1, 1),
        ("nightly-2026-07-31", 0, 2),
    ];
    for (channel, ok_count, old_count) in cases {
        let ok = problem_count(CONFIG_OK, MANIFEST_OK, channel);
        let old = problem_count(CONFIG_OLD, MANIFEST_OK, channel);
        assert_eq!((ok, old), (ok_count, old_count), "{channel}");
    }
}

/// Scenario: stable channels, which pin no nightly key.
///
/// Invariant: neither key is required or rejected on a stable pin.
#[test]
fn a_stable_channel_imposes_no_unstable_key() {
    for channel in ["stable", "1.94.0"] {
        let old = problem_count(CONFIG_OLD, MANIFEST_OK, channel);
        let ok = problem_count(CONFIG_OK, MANIFEST_OK, channel);
        assert_eq!((old, ok), (0, 0), "{channel}");
    }
}

/// Scenario: `rustflags` lines with and without `metadata`, on one line and spread over several.
///
/// Invariant: no `rustflags` value may name `metadata`, however it is laid out.
#[test]
fn no_rustflags_line_names_metadata() {
    let nightly = "nightly-2026-10-14";
    assert_eq!(problem_count(CONFIG_OK, MANIFEST_OK, nightly), 0);
    assert_eq!(problem_count(CONFIG_METADATA_FLAG, MANIFEST_OK, nightly), 1);
    assert_eq!(problem_count(CONFIG_SPLIT, MANIFEST_OK, nightly), 1);
}

/// Scenario: a pin the reader cannot classify as a dated nightly or a release.
///
/// Invariant: an undated `nightly` is a complaint, not a silent pass.
#[test]
fn an_unreadable_pin_is_a_problem() {
    assert_eq!(problem_count(CONFIG_OK, MANIFEST_OK, "nightly"), 1);
}
