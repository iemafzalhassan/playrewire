//! Turns Go compiler/runtime errors into friendly cards. The raw message
//! always stays available on each card.

use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendlyError {
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub title: String,
    pub plain: String,
    pub raw: String,
}

/// `./main.go:9:2: msg` (gc) and `9:2: msg` (Yaegi).
static POS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^(?:\./)?(?:[\w./-]+\.go:)?(\d+):(\d+): (.+)$").unwrap());
/// The `main.go:N` frame of a goroutine trace.
static FRAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s+\S*main\.go:(\d+)").unwrap());
/// The runtime panic line holding the index message.
static PANIC_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^.*index out of range \[(\d+)\] with length (\d+).*$").unwrap()
});

type Explain = fn(&Captures) -> (&'static str, String);

static TABLE: LazyLock<Vec<(Regex, Explain)>> = LazyLock::new(|| {
    let r = |p: &str| Regex::new(p).unwrap();
    vec![
        (
            r(r"declared and not used: (\w+)"),
            (|c| {
                (
                    "Unused box",
                    format!(
                        "You made a box called `{}` but never used it. Use it, or delete it.",
                        &c[1]
                    ),
                )
            }) as Explain,
        ),
        (r(r"undefined: (\w+)"), |c| {
            (
                "Unknown name",
                format!(
                    "Go doesn't know what `{0}` is yet. Check the spelling, or create it first with `{0} := ...`.",
                    &c[1]
                ),
            )
        }),
        (r(r"syntax error: unexpected (.+)"), |c| {
            (
                "Something's out of place",
                format!(
                    "Go got confused near `{}`. Look for a missing `{{`, `}}` or `)` just before here.",
                    &c[1]
                ),
            )
        }),
        (r(r"missing return"), |_| {
            (
            "Missing answer",
            "This function promises to give back a value, but one path doesn't. Add a `return`."
                .to_string(),
        )
        }),
        (r(r"cannot use (\S+) .* as (\w+) value"), |c| {
            (
                "Wrong type",
                format!("You gave `{}` where Go expected a `{}`.", &c[1], &c[2]),
            )
        }),
        (r(r#""(\S+)" imported and not used"#), |c| {
            (
                "Unused import",
                format!(
                    "You brought in `{}` but didn't use it. Remove that import line.",
                    &c[1]
                ),
            )
        }),
        (
            r(r"index out of range \[(\d+)\] with length (\d+)"),
            index_card,
        ),
    ]
});

fn index_card(c: &Captures) -> (&'static str, String) {
    let plain = match c[2].parse::<u64>() {
        // "numbered 0 to {len-1}" makes no sense for an empty list.
        Ok(0) => format!("You asked for item {}, but there are no items.", &c[1]),
        Ok(n) => format!(
            "You asked for item {}, but there are only {} items (numbered 0 to {}).",
            &c[1],
            n,
            n - 1
        ),
        Err(_) => format!(
            "You asked for item {}, but the list is shorter than that.",
            &c[1]
        ),
    };
    ("Off the end", plain)
}

fn card(
    line: Option<u32>,
    col: Option<u32>,
    (title, plain): (&str, String),
    raw: &str,
) -> FriendlyError {
    FriendlyError {
        line,
        col,
        title: title.into(),
        plain,
        raw: raw.into(),
    }
}

fn fallback(line: Option<u32>, col: Option<u32>, raw: &str) -> FriendlyError {
    card(
        line,
        col,
        (
            "Go found a problem",
            "Here's what Go said. Tap to see the exact message.".into(),
        ),
        raw,
    )
}

/// Subtracts the wrapper offset; lines at or before the wrapper are `None`.
fn user_line(n: &str, offset: u32) -> Option<u32> {
    let n: i64 = n.parse().ok()?;
    let l = n - i64::from(offset);
    (l > 0).then(|| u32::try_from(l).ok()).flatten()
}

fn push_unique(v: &mut Vec<FriendlyError>, e: FriendlyError) {
    if !v.contains(&e) {
        v.push(e);
    }
}

fn explain_message(msg: &str) -> Option<(&'static str, String)> {
    TABLE
        .iter()
        .find_map(|(re, f)| re.captures(msg).map(|c| f(&c)))
}

pub fn explain_go_errors(
    stderr: &str,
    line_offset: u32,
    timed_out: bool,
    truncated: bool,
) -> Vec<FriendlyError> {
    let mut out = Vec::new();
    if timed_out {
        out.push(card(
            None,
            None,
            (
                "Never-ending loop",
                "Your program ran for 10 seconds without stopping. Check that your loop's condition eventually becomes false."
                    .into(),
            ),
            "",
        ));
    }
    if truncated {
        out.push(card(
            None,
            None,
            (
                "Too much output",
                "Your program printed a huge amount, so Rewire stopped it. Is something printing inside a loop that never ends?"
                    .into(),
            ),
            "",
        ));
    }

    let mut found: Vec<FriendlyError> = Vec::new();

    for c in POS.captures_iter(stderr) {
        let (line, col) = match user_line(&c[1], line_offset) {
            Some(l) => (Some(l), c[2].parse().ok()),
            None => (None, None),
        };
        let raw = c.get(0).unwrap().as_str();
        push_unique(
            &mut found,
            match explain_message(&c[3]) {
                Some(x) => card(line, col, x, raw),
                None => fallback(line, col, raw),
            },
        );
    }

    // Runtime panics carry no `LINE:COL:` prefix; the line comes from the trace.
    if found.is_empty()
        && let Some(m) = PANIC_LINE.captures(stderr)
    {
        let raw = m.get(0).unwrap().as_str();
        let line = FRAME
            .captures(stderr)
            .and_then(|f| user_line(&f[1], line_offset));
        if let Some(x) = explain_message(raw) {
            push_unique(&mut found, card(line, None, x, raw));
        }
    }

    if found.is_empty() && !stderr.trim().is_empty() {
        found.push(fallback(None, None, stderr));
    }
    out.extend(found);
    out
}
