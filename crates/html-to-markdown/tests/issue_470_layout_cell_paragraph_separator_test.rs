// ~keep Rust inner attributes below are crate-level attributes, not a shell shebang.
#![allow(missing_docs)]

//! Regression tests for issue #470: adjacent `<p>` elements inside a table cell were joined
//! with zero bytes between them, merging words and running two emphasis spans together into
//! invalid Markdown (`**Alice Example***Customer Service*`).
//!
//! The cell is reached through the *layout*-table path: the reporter's third row has three
//! cells against the others' two, which makes `looks_like_layout` true, and a header-less,
//! caption-less layout table renders each row as a list item. `append_layout_cell_text` built
//! its cell context with `convert_as_inline: true` and nothing else, so `<p>`'s
//! table-continuation branch never fired — `br_in_tables` was not even consulted — while
//! `convert_as_inline` simultaneously suppressed the ordinary block separator. Neither
//! separator was emitted, so the paragraphs abutted.
//!
//! The fix marks the context as a layout cell, which routes `<p>` and `<div>` continuations
//! through the settled cell rule from issues #453/#454 (`emit_table_cell_break`): a literal
//! `<br>` under `br_in_tables`, otherwise a single space. Not a regression — this predates
//! 3.8.3, matching the report.

use html_to_markdown_rs::{ConversionOptions, NewlineStyle, convert};

fn reporter_options() -> ConversionOptions {
    ConversionOptions {
        extract_metadata: false,
        br_in_tables: true,
        bullets: "*+-".to_string(),
        compact_tables: true,
        keep_inline_images_in: vec!["a".to_string(), "td".to_string(), "th".to_string()],
        newline_style: NewlineStyle::Spaces,
        ..ConversionOptions::default()
    }
}

const REPORTED_HTML: &str = r"<table>
  <tr>
    <td><p><b>Alice Example</b></p><p><i>Customer Service</i></p><p><i>Example Group</i></p></td>
    <td>Logo</td>
  </tr>
  <tr>
    <td><p>Example</p><p>Contact details</p></td>
    <td>Other</td>
  </tr>
  <tr><td>A</td><td>B</td><td>C</td></tr>
</table>";

fn content(html: &str, options: ConversionOptions) -> String {
    convert(html, Some(options)).unwrap().content.unwrap_or_default()
}

/// The reported reproduction: no two paragraphs may abut.
#[test]
fn should_separate_adjacent_paragraphs_in_a_layout_cell() {
    let out = content(REPORTED_HTML, reporter_options());
    assert!(
        !out.contains("**Alice Example***Customer Service*"),
        "emphasis spans still run together: {out:?}"
    );
    assert!(
        !out.contains("ExampleContact details"),
        "paragraph text still merged: {out:?}"
    );
    for word in [
        "Alice Example",
        "Customer Service",
        "Example Group",
        "Contact details",
        "Logo",
    ] {
        assert!(out.contains(word), "{word:?} missing from output: {out:?}");
    }
}

/// `br_in_tables` selects how the boundary is represented, exactly as it does for `<br>`,
/// `<div>` and list items inside a cell (issues #453/#454).
#[test]
fn should_honour_br_in_tables_for_the_layout_cell_boundary() {
    let with_br = content(REPORTED_HTML, reporter_options());
    assert!(
        with_br.contains("**Alice Example**<br>*Customer Service*"),
        "br_in_tables must emit a literal <br>: {with_br:?}"
    );

    let without_br = content(
        REPORTED_HTML,
        ConversionOptions {
            br_in_tables: false,
            ..reporter_options()
        },
    );
    assert!(
        without_br.contains("**Alice Example** *Customer Service*"),
        "without br_in_tables the boundary must collapse to one space: {without_br:?}"
    );
    assert!(
        !without_br.contains("<br>"),
        "no <br> may be emitted when br_in_tables is off: {without_br:?}"
    );
}

/// A `<div>` continuation follows the same settled rule as `<p>`.
#[test]
fn should_separate_adjacent_divs_in_a_layout_cell() {
    let html =
        "<table><tr><td><div>A</div><div>B</div></td><td>x</td></tr><tr><td>1</td><td>2</td><td>3</td></tr></table>";
    let out = content(html, reporter_options());
    assert!(!out.contains("AB"), "divs still merged: {out:?}");
}

/// The layout row is a list item, not a table row, so its text must not pick up table-cell
/// pipe/emphasis escaping along with the continuation rule.
#[test]
fn should_not_escape_pipes_or_emphasis_markers_in_a_layout_row() {
    let html = "<table><tr><td><p>a|b</p><p>c*d</p></td><td>x</td></tr><tr><td>1</td><td>2</td><td>3</td></tr></table>";
    let out = content(html, reporter_options());
    assert!(!out.contains(r"\|"), "pipe was escaped in a layout row: {out:?}");
    assert!(!out.contains(r"\*"), "asterisk was escaped in a layout row: {out:?}");
}
