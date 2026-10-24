use rewire_core::explain_go_errors;

fn fx(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn explains_unused_var_gc() {
    let out = explain_go_errors(&fx("gc_unused_var.txt"), 7, false, false);
    assert_eq!(out.len(), 1);
    let e = &out[0];
    assert_eq!((e.line, e.col), (Some(1), Some(2)));
    assert_eq!(e.title, "Unused box");
    assert_eq!(
        e.plain,
        "You made a box called `x` but never used it. Use it, or delete it."
    );
    assert_eq!(e.raw, "./main.go:8:2: declared and not used: x");
}

#[test]
fn explains_unused_var_yaegi() {
    let out = explain_go_errors(&fx("yaegi_unused_var.txt"), 0, false, false);
    assert_eq!(out.len(), 1);
    assert_eq!((out[0].line, out[0].col), (Some(4), Some(2)));
    assert_eq!(out[0].title, "Unused box");
    assert!(out[0].plain.contains("`x`"));
}

#[test]
fn explains_undefined() {
    for (f, off) in [("gc_undefined.txt", 7), ("yaegi_undefined.txt", 0)] {
        let out = explain_go_errors(&fx(f), off, false, false);
        assert_eq!(out.len(), 1, "{f}");
        assert_eq!(out[0].title, "Unknown name");
        assert_eq!(
            out[0].plain,
            "Go doesn't know what `score` is yet. Check the spelling, or create it first with `score := ...`."
        );
    }
}

#[test]
fn explains_syntax() {
    let out = explain_go_errors(&fx("gc_syntax.txt"), 7, false, false);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].title, "Something's out of place");
    assert_eq!(
        out[0].plain,
        "Go got confused near `newline in argument list; possibly missing comma or )`. Look for a missing `{`, `}` or `)` just before here."
    );
    assert_eq!(out[0].line, Some(1));
}

#[test]
fn explains_index_out_of_range() {
    let out = explain_go_errors(&fx("gc_index.txt"), 7, false, false);
    assert_eq!(out.len(), 1);
    let e = &out[0];
    assert_eq!(e.title, "Off the end");
    assert_eq!(
        e.plain,
        "You asked for item 5, but there are only 3 items (numbered 0 to 2)."
    );
    // goroutine frame is main.go:9 -> 9 - 7 = 2
    assert_eq!(e.line, Some(2));
    assert_eq!(e.col, None);
    assert_eq!(
        e.raw,
        "panic: runtime error: index out of range [5] with length 3"
    );
}

#[test]
fn index_out_of_range_empty_slice() {
    let out = explain_go_errors(
        "panic: runtime error: index out of range [0] with length 0\n",
        0,
        false,
        false,
    );
    assert_eq!(out[0].title, "Off the end");
    assert_eq!(
        out[0].plain,
        "You asked for item 0, but there are no items."
    );
    assert_eq!(out[0].line, None);
}

#[test]
fn explains_wrong_type_unused_import_missing_return() {
    let t = explain_go_errors(&fx("gc_type.txt"), 7, false, false);
    assert_eq!(t[0].title, "Wrong type");
    assert_eq!(
        t[0].plain,
        "You gave `\"hello\"` where Go expected a `int`."
    );
    let i = explain_go_errors(&fx("gc_unused_import.txt"), 0, false, false);
    assert_eq!(i[0].title, "Unused import");
    assert_eq!(
        i[0].plain,
        "You brought in `os` but didn't use it. Remove that import line."
    );
    let m = explain_go_errors(&fx("gc_missing_return.txt"), 0, false, false);
    assert_eq!(m[0].title, "Missing answer");
    assert_eq!(
        m[0].plain,
        "This function promises to give back a value, but one path doesn't. Add a `return`."
    );
}

#[test]
fn two_problems_two_cards_header_skipped() {
    let out = explain_go_errors(&fx("gc_two_unused.txt"), 7, false, false);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].line, Some(1));
    assert_eq!(out[1].line, Some(2));
    assert!(out[1].plain.contains("`b`"));
}

#[test]
fn duplicate_lines_are_one_card() {
    let s = "9:2: undefined: q\n9:2: undefined: q\n";
    assert_eq!(explain_go_errors(s, 0, false, false).len(), 1);
}

#[test]
fn timeout_card() {
    let out = explain_go_errors("", 0, true, false);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].title, "Never-ending loop");
    assert_eq!(
        out[0].plain,
        "Your program ran for 10 seconds without stopping. Check that your loop's condition eventually becomes false."
    );
    assert_eq!((out[0].line, out[0].col), (None, None));
}

#[test]
fn truncated_card() {
    let out = explain_go_errors("", 0, false, true);
    assert_eq!(out[0].title, "Too much output");
    assert_eq!(
        out[0].plain,
        "Your program printed a huge amount, so Rewire stopped it. Is something printing inside a loop that never ends?"
    );
}

#[test]
fn flag_cards_come_first() {
    let out = explain_go_errors(&fx("yaegi_undefined.txt"), 0, true, true);
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].title, "Never-ending loop");
    assert_eq!(out[1].title, "Too much output");
    assert_eq!(out[2].title, "Unknown name");
}

#[test]
fn unknown_falls_back_with_raw() {
    let s = "something odd happened\nand again\n";
    let out = explain_go_errors(s, 0, false, false);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].title, "Go found a problem");
    assert_eq!(
        out[0].plain,
        "Here's what Go said. Tap to see the exact message."
    );
    assert_eq!(out[0].raw, s);
    assert_eq!(out[0].line, None);
}

#[test]
fn unmatched_positioned_line_gets_fallback_card() {
    let out = explain_go_errors("6:18: something new\n", 0, false, false);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].title, "Go found a problem");
    assert_eq!(out[0].line, Some(6));
}

#[test]
fn explains_index_out_of_range_yaegi() {
    for f in ["yaegi_index.txt", "yaegi_index_var.txt"] {
        let out = explain_go_errors(&fx(f), 0, false, false);
        assert_eq!(out.len(), 1, "{f}");
        assert_eq!(out[0].title, "Off the end");
        assert_eq!(
            out[0].plain,
            "You asked for an item past the end of the list. Items are numbered from 0."
        );
        assert_eq!(out[0].line, Some(6));
        assert!(out[0].raw.contains("reflect: slice index out of range"));
    }
    let off = explain_go_errors(&fx("yaegi_index.txt"), 4, false, false);
    assert_eq!(off[0].line, Some(2));
}

#[test]
fn explains_syntax_yaegi() {
    let a = explain_go_errors(&fx("yaegi_syntax.txt"), 0, false, false);
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].title, "Something's out of place");
    assert_eq!(
        a[0].plain,
        "Go got confused near `missing ',' before newline in argument list`. Look for a missing `{`, `}` or `)` just before here."
    );
    assert_eq!(a[0].line, Some(6));
    let b = explain_go_errors(&fx("yaegi_syntax_brace.txt"), 0, false, false);
    assert_eq!(b[0].title, "Something's out of place");
    assert!(b[0].plain.contains("near `EOF`"));
    let c = explain_go_errors(&fx("yaegi_syntax_unexpected.txt"), 0, false, false);
    assert!(c[0].plain.contains("near `)`"));
}

#[test]
fn explains_wrong_type_yaegi() {
    let out = explain_go_errors(&fx("yaegi_type.txt"), 0, false, false);
    assert_eq!(out[0].title, "Wrong type");
    assert_eq!(
        out[0].plain,
        "You gave `\"hello\"` where Go expected a `int`."
    );
}

#[test]
fn line_offset_applied() {
    let out = explain_go_errors("./main.go:9:2: declared and not used: x\n", 7, false, false);
    assert_eq!((out[0].line, out[0].col), (Some(2), Some(2)));
}

#[test]
fn wrapper_line_becomes_none() {
    let out = explain_go_errors(
        "./main.go:5:2: \"os\" imported and not used\n",
        7,
        false,
        false,
    );
    assert_eq!((out[0].line, out[0].col), (None, None));
    assert_eq!(out[0].title, "Unused import");
}

#[test]
fn friendly_error_serializes_snake_case() {
    let out = explain_go_errors("3:1: missing return\n", 0, false, false);
    let v = serde_json::to_value(&out[0]).unwrap();
    assert_eq!(v["line"], 3);
    assert_eq!(v["title"], "Missing answer");
    assert!(v.get("raw").is_some() && v.get("col").is_some() && v.get("plain").is_some());
}

#[test]
fn out_of_memory_card() {
    let want = (
        "Too much memory",
        "Your program kept asking for more memory until Rewire stopped it. Is something growing inside a loop that never ends?",
    );
    for f in ["gc_out_of_memory.txt", "gc_out_of_memory_runtime.txt"] {
        let out = explain_go_errors(&fx(f), 7, false, false);
        assert_eq!(out.len(), 1, "{f}: {out:?}");
        assert_eq!((out[0].title.as_str(), out[0].plain.as_str()), want, "{f}");
        assert_eq!(out[0].line, None);
        assert!(out[0].raw.contains("out of memory"), "{f}");
    }
}

#[test]
fn labcore_gc_checks_map_to_cards() {
    let out = explain_go_errors(
        "9:1: missing return\n5:2: \"strings\" imported and not used\n",
        0,
        false,
        false,
    );
    let titles: Vec<_> = out.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, ["Missing answer", "Unused import"]);
    assert_eq!((out[0].line, out[0].col), (Some(9), Some(1)));
}

#[test]
fn slow_build_card() {
    let out = explain_go_errors("rewire: build took too long\n", 7, false, false);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].title, "Slow build");
    assert_eq!(
        out[0].plain,
        "Building took too long. Try again; the first build is the slowest."
    );
    assert_eq!(out[0].line, None);
}
